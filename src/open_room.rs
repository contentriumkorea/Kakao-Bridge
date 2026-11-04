//! Room opening through the user's running client. This can activate KakaoTalk.
//! A title match is not proof of the underlying Kakao chat ID.
use std::{collections::HashSet, ffi::c_void, thread, time::{Duration, Instant}};
use crate::dpi_geometry;

type Hwnd = *mut c_void;
#[derive(Clone,Debug)]
struct Window { handle: isize, title: String }

trait Backend {
    fn windows(&self) -> Result<Vec<Window>, &'static str>;
    fn self_list(&self) -> Result<isize, &'static str>;
    fn ensure_friends_tab(&self) -> Result<(), &'static str>;
    fn ensure_chat_tab(&self) -> Result<(), &'static str>;
    fn search_box(&self) -> Result<isize, &'static str>;
    fn read_text(&self, hwnd: isize) -> Result<String, &'static str>;
    fn write_text(&self, hwnd: isize, text: &str) -> Result<(), &'static str>;
    fn results(&self) -> Result<Option<isize>, &'static str>;
    fn click_first(&self, hwnd: isize, scroll_first: bool) -> Result<(), &'static str>;
    fn sleep(&self);
}

fn find_new_window<B: Backend>(backend: &B, before: &HashSet<isize>, expected: &str) -> Result<(), &'static str> {
    let deadline=Instant::now()+Duration::from_secs(8);
    loop {
        let windows=backend.windows()?;
        let new:Vec<_>=windows.iter().filter(|w|!before.contains(&w.handle)).collect();
        let matches:Vec<_>=new.iter().filter(|w|w.title==expected).collect();
        if new.len()==1 && matches.len()==1 {return Ok(());}
        if !new.is_empty() {return Err("OPENED_OTHER_OR_AMBIGUOUS_ROOM");}
        if Instant::now()>=deadline {return Err("OPEN_RESULT_NOT_OBSERVED");}
        backend.sleep();
    }
}

fn drive<B: Backend>(backend: &B, title: &str, is_self: bool,allow_search_input_changes:bool) -> Result<&'static str, &'static str> {
    if title.trim().is_empty() || title.len()>1024 || title.chars().any(char::is_control) {return Err("CHAT_TITLE_UNAVAILABLE");}
    let initial=backend.windows()?;
    match initial.iter().filter(|w|w.title==title).count() {
        1=>return Ok("already_open_title_match"),
        n if n>1=>return Err("AMBIGUOUS_WINDOW_TITLE"),
        _=>{}
    }
    let before:HashSet<_>=initial.iter().map(|w|w.handle).collect();
    if is_self {
        backend.ensure_friends_tab()?;
        backend.click_first(backend.self_list()?,true)?;
        find_new_window(backend,&before,title)?;
        return Ok("opened_title_match");
    }
    if !allow_search_input_changes{return Err("SEARCH_INPUT_APPROVAL_REQUIRED");}
    backend.ensure_chat_tab()?;
    let edit=backend.search_box()?;
    let existing=backend.read_text(edit)?;
    if existing.is_empty(){backend.write_text(edit,title)?;}else if existing!=title{return Err("SEARCH_DRAFT_PRESENT");}
    let result=(|| {
        if backend.read_text(edit)?!=title {return Err("SEARCH_INPUT_CHANGED");}
        let deadline=Instant::now()+Duration::from_secs(8);
        let results=loop {
            if backend.read_text(edit)?!=title {return Err("SEARCH_INPUT_CHANGED");}
            if let Some(list)=backend.results()? {break list;}
            if Instant::now()>=deadline {return Err("SEARCH_RESULTS_UNAVAILABLE");}
            backend.sleep();
        };
        // Allow asynchronous filtering to settle, then ensure the user's input did not change.
        backend.sleep();backend.sleep();
        if backend.read_text(edit)?!=title {return Err("SEARCH_INPUT_CHANGED");}
        backend.click_first(results,false)?;
        find_new_window(backend,&before,title)?;
        if backend.read_text(edit)?!=title {return Err("SEARCH_INPUT_CHANGED_AFTER_OPEN");}
        Ok("opened_title_match")
    })();
    // Do not clear search text: user input can race with a read/clear sequence.
    result
}

#[repr(C)] struct Rect { left:i32,top:i32,right:i32,bottom:i32 }
#[repr(C)] struct Point { x:i32,y:i32 }
#[link(name="user32")] extern "system" {
    fn EnumWindows(f:extern "system" fn(Hwnd,isize)->i32,l:isize)->i32;
    fn EnumChildWindows(h:Hwnd,f:extern "system" fn(Hwnd,isize)->i32,l:isize)->i32;
    fn GetWindowThreadProcessId(h:Hwnd,p:*mut u32)->u32;
    fn GetClassNameW(h:Hwnd,b:*mut u16,n:i32)->i32;
    fn GetWindowTextLengthW(h:Hwnd)->i32;
    fn GetWindowTextW(h:Hwnd,b:*mut u16,n:i32)->i32;
    fn GetDlgCtrlID(h:Hwnd)->i32;
    fn GetParent(h:Hwnd)->Hwnd;
    fn IsWindowVisible(h:Hwnd)->i32;
    fn IsWindow(h:Hwnd)->i32;
    fn GetClientRect(h:Hwnd,r:*mut Rect)->i32;
    fn ClientToScreen(h:Hwnd,p:*mut Point)->i32;
    fn GetDpiForWindow(h:Hwnd)->u32;
    fn PostMessageW(h:Hwnd,msg:u32,w:usize,l:isize)->i32;
    fn SendMessageTimeoutW(h:Hwnd,msg:u32,w:usize,l:isize,flags:u32,timeout:u32,result:*mut usize)->isize;
}
fn text(h:Hwnd)->String { unsafe {
    let n=GetWindowTextLengthW(h); if n<=0 || n>4096{return String::new();}
    let mut b=vec![0u16;n as usize+1];let got=GetWindowTextW(h,b.as_mut_ptr(),b.len() as i32);
    if got<=0 {String::new()} else {String::from_utf16_lossy(&b[..got as usize])}
}}
fn class(h:Hwnd)->String {unsafe{let mut b=[0u16;256];let n=GetClassNameW(h,b.as_mut_ptr(),256);if n<=0{String::new()}else{String::from_utf16_lossy(&b[..n as usize])}}}
fn belongs(h:Hwnd,pid:u32)->bool {unsafe{let mut found=0;GetWindowThreadProcessId(h,&mut found);IsWindow(h)!=0 && found==pid}}
fn message(h:Hwnd,msg:u32,w:usize,l:isize)->Result<usize,&'static str>{unsafe{let mut out=0;if SendMessageTimeoutW(h,msg,w,l,0x0002,3000,&mut out)==0{Err("KAKAO_CONTROL_UNRESPONSIVE")}else{Ok(out)}}}
struct Collect { pid:u32, handles:Vec<isize> }
extern "system" fn top(h:Hwnd,p:isize)->i32 {unsafe{let c=&mut *(p as *mut Collect);if belongs(h,c.pid){c.handles.push(h as isize);}1}}
extern "system" fn child(h:Hwnd,p:isize)->i32 {unsafe{let c=&mut *(p as *mut Collect);if belongs(h,c.pid){c.handles.push(h as isize);}1}}
fn top_windows(pid:u32)->Result<Vec<isize>,&'static str>{let mut c=Collect{pid,handles:Vec::new()};if unsafe{EnumWindows(top,&mut c as *mut _ as isize)}==0{return Err("WINDOW_ENUMERATION_FAILED");}Ok(c.handles)}
fn children(pid:u32,h:isize)->Result<Vec<isize>,&'static str>{let mut c=Collect{pid,handles:Vec::new()};if unsafe{EnumChildWindows(h as Hwnd,child,&mut c as *mut _ as isize)}==0{return Err("WINDOW_ENUMERATION_FAILED");}Ok(c.handles)}
struct WindowsBackend {pid:u32}
impl WindowsBackend {
    fn main(&self)->Result<isize,&'static str>{
        let list:Vec<_>=top_windows(self.pid)?.into_iter().filter(|h|class(*h as Hwnd)=="EVA_Window_Dblclk" && text(*h as Hwnd)=="카카오톡").collect();
        if list.len()!=1{return Err("KAKAO_MAIN_NOT_UNIQUE");}Ok(list[0])
    }
    fn named_control(&self,prefix:&str)->Result<isize,&'static str>{
        let list:Vec<_>=children(self.pid,self.main()?)?.into_iter().filter(|h|text(*h as Hwnd).starts_with(prefix)).collect();
        if list.len()!=1{return Err("KAKAO_CONTROL_NOT_UNIQUE");}Ok(list[0])
    }
    fn click_navigation(&self,main:isize,y:u32)->Result<(),&'static str>{
        let navigation:Vec<_>=children(self.pid,main)?.into_iter().filter(|h|unsafe{GetParent(*h as Hwnd)}==main as Hwnd && class(*h as Hwnd)=="EVA_ChildWindow" && unsafe{GetDlgCtrlID(*h as Hwnd)}==132 && unsafe{IsWindowVisible(*h as Hwnd)}!=0).collect();
        if navigation.len()!=1{return Err("NAVIGATION_CONTROL_NOT_UNIQUE");}
        let h=navigation[0] as Hwnd;if !belongs(h,self.pid){return Err("TARGET_CHANGED");}
        let _guard=dpi_geometry::enter(h as isize)?;let dpi=unsafe{GetDpiForWindow(h)};
        let mut r=Rect{left:0,top:0,right:0,bottom:0};if unsafe{GetClientRect(h,&mut r)}==0{return Err("NAVIGATION_GEOMETRY_UNAVAILABLE");}
        let (x,y)=dpi_geometry::client_point(dpi,r.right-r.left,r.bottom-r.top,32,y as i32).ok_or("NAVIGATION_GEOMETRY_UNAVAILABLE")?;
        let position=dpi_geometry::pack_signed(x,y).ok_or("NAVIGATION_GEOMETRY_UNAVAILABLE")?;
        if !belongs(h,self.pid) || unsafe{GetDpiForWindow(h)}!=dpi{return Err("TARGET_CHANGED");}
        for (msg,w) in [(0x0200,0usize),(0x0201,1),(0x0202,0)] {
            if unsafe{PostMessageW(h,msg,w,position)}==0{return Err("NAVIGATION_INPUT_FAILED");}
        }
        Ok(())
    }
}
impl Backend for WindowsBackend {
    fn windows(&self)->Result<Vec<Window>,&'static str>{
        let mut out=Vec::new();
        for h in top_windows(self.pid)? {
            if class(h as Hwnd)!="EVA_Window_Dblclk" {continue;}
            let title=text(h as Hwnd);if title.is_empty() || title=="카카오톡" {continue;}
            if children(self.pid,h)?.into_iter().any(|e|class(e as Hwnd)=="RICHEDIT50W" && unsafe{GetDlgCtrlID(e as Hwnd)}==1006){out.push(Window{handle:h,title});}
        }Ok(out)
    }
    fn self_list(&self)->Result<isize,&'static str>{
        let view=self.named_control("ContactListView")?;
        if unsafe{IsWindowVisible(view as Hwnd)}==0 {return Err("FRIENDS_TAB_NOT_VISIBLE");}
        let controls=children(self.pid,view)?;
        if controls.iter().any(|h|text(*h as Hwnd).starts_with("SearchListCtrl") && unsafe{IsWindowVisible(*h as Hwnd)}!=0){return Err("CONTACT_SEARCH_ACTIVE");}
        let edits:Vec<_>=controls.iter().filter(|h|class(**h as Hwnd)=="Edit").collect();
        if edits.is_empty(){return Err("CONTACT_SEARCH_CONTROL_UNAVAILABLE");}
        for edit in edits {
            if !self.read_text(*edit)?.is_empty(){return Err("CONTACT_SEARCH_DRAFT_PRESENT");}
        }
        let list=self.named_control("ContactListCtrl")?;
        if unsafe{IsWindowVisible(list as Hwnd)}==0 {return Err("FRIENDS_TAB_NOT_VISIBLE");}
        Ok(list)
    }
    fn ensure_friends_tab(&self)->Result<(),&'static str>{
        let main=self.main()?;let view=self.named_control("ContactListView")?;
        if unsafe{IsWindowVisible(view as Hwnd)}!=0{return Ok(());}
        let chat_view=self.named_control("ChatRoomListView")?;
        if unsafe{IsWindowVisible(chat_view as Hwnd)}==0{return Err("CHAT_LIST_TAB_NOT_VISIBLE");}
        let edit=self.search_box()?;
        if !self.read_text(edit)?.is_empty(){return Err("SEARCH_DRAFT_PRESENT");}
        if children(self.pid,chat_view)?.iter().any(|h|text(*h as Hwnd).starts_with("SearchListCtrl") && unsafe{IsWindowVisible(*h as Hwnd)}!=0){return Err("CHAT_SEARCH_ACTIVE");}
        let list=self.named_control("ContactListCtrl")?;
        // At 96 DPI, y=24 is an unverified symmetric candidate above the chat tab.
        self.click_navigation(main,24)?;
        let deadline=Instant::now()+Duration::from_secs(8);
        loop {
            if !belongs(view as Hwnd,self.pid) || !belongs(list as Hwnd,self.pid){return Err("TARGET_CHANGED");}
            if unsafe{IsWindowVisible(view as Hwnd)}!=0 && unsafe{IsWindowVisible(list as Hwnd)}!=0{return Ok(());}
            if Instant::now()>=deadline{return Err("FRIENDS_TAB_SWITCH_UNCONFIRMED");}
            self.sleep();
        }
    }
    fn ensure_chat_tab(&self)->Result<(),&'static str>{
        let main=self.main()?;let view=self.named_control("ChatRoomListView")?;
        if unsafe{IsWindowVisible(view as Hwnd)}!=0{return Ok(());}
        // Switch only from a verified, unfiltered friends tab; never scan guesses.
        self.self_list()?;
        self.click_navigation(main,72)?;
        let room_list=self.named_control("ChatRoomListCtrl")?;
        let deadline=Instant::now()+Duration::from_secs(8);
        loop {
            if !belongs(view as Hwnd,self.pid) || !belongs(room_list as Hwnd,self.pid){return Err("TARGET_CHANGED");}
            if unsafe{IsWindowVisible(view as Hwnd)}!=0 && unsafe{IsWindowVisible(room_list as Hwnd)}!=0{return Ok(());}
            if Instant::now()>=deadline{return Err("CHAT_TAB_SWITCH_UNCONFIRMED");}
            self.sleep();
        }
    }
    fn search_box(&self)->Result<isize,&'static str>{
        let view=self.named_control("ChatRoomListView")?;
        if unsafe{IsWindowVisible(view as Hwnd)}==0{return Err("CHAT_LIST_TAB_NOT_VISIBLE");}
        let list:Vec<_>=children(self.pid,view)?.into_iter().filter(|h|{
            if class(*h as Hwnd)!="Edit" {return false;}
            let mut r=Rect{left:0,top:0,right:0,bottom:0};(unsafe{GetClientRect(*h as Hwnd,&mut r)})!=0 && r.right-r.left>=100 && r.bottom-r.top>=15
        }).collect();
        if list.len()!=1{return Err("SEARCH_CONTROL_NOT_UNIQUE");}Ok(list[0])
    }
    fn read_text(&self,hwnd:isize)->Result<String,&'static str>{
        if !belongs(hwnd as Hwnd,self.pid){return Err("TARGET_CHANGED");}
        let n=message(hwnd as Hwnd,0x000e,0,0)?;if n>4096{return Err("SEARCH_INPUT_TOO_LONG");}
        let mut b=vec![0u16;n+1];let got=message(hwnd as Hwnd,0x000d,b.len(),b.as_mut_ptr() as isize)?;
        if got>n{return Err("SEARCH_INPUT_CHANGED");}Ok(String::from_utf16_lossy(&b[..got]))
    }
    fn write_text(&self,hwnd:isize,s:&str)->Result<(),&'static str>{
        if !belongs(hwnd as Hwnd,self.pid){return Err("TARGET_CHANGED");}
        let b:Vec<u16>=s.encode_utf16().chain([0]).collect();
        // Append instead of replacing selected text so concurrent user input is not erased.
        message(hwnd as Hwnd,0x00b1,0x7fff_ffff,0x7fff_ffff)?;message(hwnd as Hwnd,0x00c2,1,b.as_ptr() as isize)?;Ok(())
    }
    fn results(&self)->Result<Option<isize>,&'static str>{
        let list:Vec<_>=children(self.pid,self.main()?)?.into_iter().filter(|h|text(*h as Hwnd).starts_with("SearchListCtrl") && unsafe{IsWindowVisible(*h as Hwnd)}!=0).collect();
        match list.len(){0=>Ok(None),1=>Ok(Some(list[0])),_=>Err("SEARCH_RESULTS_NOT_UNIQUE")}
    }
    fn click_first(&self,hwnd:isize,scroll:bool)->Result<(),&'static str>{unsafe{
        let h=hwnd as Hwnd;if !belongs(h,self.pid){return Err("TARGET_CHANGED");}
        if scroll && self.self_list()?!=hwnd{return Err("TARGET_CHANGED");}
        let _guard=dpi_geometry::enter(h as isize)?;
        let mut r=Rect{left:0,top:0,right:0,bottom:0};if GetClientRect(h,&mut r)==0{return Err("LIST_GEOMETRY_UNAVAILABLE");}
        let dpi=GetDpiForWindow(h);let (x,y)=dpi_geometry::client_point(dpi,r.right-r.left,r.bottom-r.top,134,29).ok_or("LIST_GEOMETRY_UNAVAILABLE")?;
        if !belongs(h,self.pid) || GetDpiForWindow(h)!=dpi{return Err("TARGET_CHANGED");}
        if scroll {
            let mut p=Point{x,y};if ClientToScreen(h,&mut p)==0{return Err("LIST_GEOMETRY_UNAVAILABLE");}
            let screen=dpi_geometry::pack_signed(p.x,p.y).ok_or("LIST_GEOMETRY_UNAVAILABLE")?;
            for _ in 0..32 {if PostMessageW(h,0x020a,120usize<<16,screen)==0{return Err("LIST_INPUT_FAILED");}}
        }
        let pos=dpi_geometry::pack_signed(x,y).ok_or("LIST_GEOMETRY_UNAVAILABLE")?;
        for (m,w) in [(0x0201,1usize),(0x0202,0),(0x0203,1),(0x0202,0)] {if PostMessageW(h,m,w,pos)==0{return Err("LIST_INPUT_FAILED");}}
        Ok(())
    }}
    fn sleep(&self){thread::sleep(Duration::from_millis(50));}
}

/// Caller must first prove a unique ID and title in a complete local cache.
/// This function opens at most one row and never sends or closes a room.
pub fn attempt(pid:u32,title:&str,is_self:bool,allow_search_input_changes:bool)->Result<&'static str,&'static str>{drive(&WindowsBackend{pid},title,is_self,allow_search_input_changes)}

/// Select only a known list tab; existing search content is checked by the backend.
pub fn select_list(pid:u32,list:&str)->Result<(),&'static str>{
    let backend=WindowsBackend{pid};
    match list {"friends"=>backend.ensure_friends_tab(),"chats"=>backend.ensure_chat_tab(),_=>Err("INVALID_REQUEST")}
}

#[cfg(test)] mod tests {
    use super::*;use std::cell::{Cell,RefCell};
    struct Fake{windows:RefCell<Vec<Window>>,edit:RefCell<String>,clicks:Cell<u32>,opened:&'static str,change_on_click:bool,change_on_write:bool,self_err:Option<&'static str>,tab_err:Option<&'static str>}
    impl Backend for Fake{
        fn windows(&self)->Result<Vec<Window>,&'static str>{Ok(self.windows.borrow().clone())}
        fn self_list(&self)->Result<isize,&'static str>{if let Some(err)=self.self_err{Err(err)}else{Ok(1)}}
        fn ensure_friends_tab(&self)->Result<(),&'static str>{if let Some(err)=self.tab_err{Err(err)}else{Ok(())}}
        fn ensure_chat_tab(&self)->Result<(),&'static str>{if let Some(err)=self.tab_err{Err(err)}else{Ok(())}}
        fn search_box(&self)->Result<isize,&'static str>{Ok(2)}
        fn read_text(&self,_:isize)->Result<String,&'static str>{Ok(self.edit.borrow().clone())}
        fn write_text(&self,_:isize,s:&str)->Result<(),&'static str>{if self.change_on_write{self.edit.borrow_mut().push_str("user input");}self.edit.borrow_mut().push_str(s);Ok(())}
        fn results(&self)->Result<Option<isize>,&'static str>{Ok(Some(3))}
        fn click_first(&self,_:isize,_:bool)->Result<(),&'static str>{self.clicks.set(self.clicks.get()+1);self.windows.borrow_mut().push(Window{handle:10,title:self.opened.into()});if self.change_on_click{*self.edit.borrow_mut()="user update".into();}Ok(())}
        fn sleep(&self){}
    }
    fn fake(opened:&'static str)->Fake{Fake{windows:RefCell::new(vec![]),edit:RefCell::new(String::new()),clicks:Cell::new(0),opened,change_on_click:false,change_on_write:false,self_err:None,tab_err:None}}
    #[test] fn nonself_search_requires_explicit_input_change_permission(){let b=fake("target");assert_eq!(drive(&b,"target",false,false),Err("SEARCH_INPUT_APPROVAL_REQUIRED"));assert_eq!(b.clicks.get(),0);assert!(b.edit.borrow().is_empty());}
    #[test] fn successful_search_is_not_cleared_after_opening(){let b=fake("target");assert_eq!(drive(&b,"target",false,true),Ok("opened_title_match"));assert_eq!(*b.edit.borrow(),"target");}
    #[test] fn target_title_match_returns_unverified_success_and_retains_search(){let b=fake("target");assert_eq!(drive(&b,"target",false,true),Ok("opened_title_match"));assert_eq!(b.clicks.get(),1);assert_eq!(*b.edit.borrow(),"target");}
    #[test] fn wrong_room_is_reported_without_closing_or_retrying(){let b=fake("other");assert_eq!(drive(&b,"target",false,true),Err("OPENED_OTHER_OR_AMBIGUOUS_ROOM"));assert_eq!(b.clicks.get(),1);assert_eq!(b.windows.borrow().len(),1);assert_eq!(*b.edit.borrow(),"target");}
    #[test] fn matching_existing_search_is_reused_without_modification(){let b=fake("target");*b.edit.borrow_mut()="target".into();assert_eq!(drive(&b,"target",false,true),Ok("opened_title_match"));assert_eq!(*b.edit.borrow(),"target");assert_eq!(b.clicks.get(),1);}
    #[test] fn existing_search_is_never_overwritten(){let b=fake("target");*b.edit.borrow_mut()="draft".into();assert_eq!(drive(&b,"target",false,true),Err("SEARCH_DRAFT_PRESENT"));assert_eq!(*b.edit.borrow(),"draft");assert_eq!(b.clicks.get(),0);}
    #[test] fn existing_room_does_not_click(){let b=fake("target");b.windows.borrow_mut().push(Window{handle:1,title:"target".into()});assert_eq!(drive(&b,"target",true,true),Ok("already_open_title_match"));assert_eq!(b.clicks.get(),0);}
    #[test] fn later_user_search_input_is_preserved_and_open_is_unconfirmed(){let mut b=fake("target");b.change_on_click=true;assert_eq!(drive(&b,"target",false,true),Err("SEARCH_INPUT_CHANGED_AFTER_OPEN"));assert_eq!(*b.edit.borrow(),"user update");}
    #[test] fn concurrent_user_input_is_not_erased_or_opened(){let mut b=fake("target");b.change_on_write=true;assert_eq!(drive(&b,"target",false,true),Err("SEARCH_INPUT_CHANGED"));assert!(b.edit.borrow().contains("user input"));assert_eq!(b.clicks.get(),0);}
    #[test] fn self_search_guard_refuses_click(){for err in ["FRIENDS_TAB_NOT_VISIBLE","CONTACT_SEARCH_ACTIVE","CONTACT_SEARCH_DRAFT_PRESENT","CONTACT_SEARCH_CONTROL_UNAVAILABLE"]{let mut b=fake("target");b.self_err=Some(err);assert_eq!(drive(&b,"target",true,true),Err(err));assert_eq!(b.clicks.get(),0);}}
    #[test] fn failed_tab_switch_does_not_search_or_click_room(){let mut b=fake("target");b.tab_err=Some("CHAT_TAB_SWITCH_UNCONFIRMED");assert_eq!(drive(&b,"target",false,true),Err("CHAT_TAB_SWITCH_UNCONFIRMED"));assert_eq!(b.clicks.get(),0);assert_eq!(*b.edit.borrow(),"");}
    #[test] fn failed_friend_tab_switch_never_opens_any_room(){let mut b=fake("target");b.tab_err=Some("FRIENDS_TAB_SWITCH_UNCONFIRMED");*b.edit.borrow_mut()="preserved".into();assert_eq!(drive(&b,"target",true,true),Err("FRIENDS_TAB_SWITCH_UNCONFIRMED"));assert_eq!(b.clicks.get(),0);assert_eq!(*b.edit.borrow(),"preserved");}
    #[test] fn delayed_window_response_is_allowed_without_retrying_input(){
        use std::sync::mpsc;use std::thread;
        let (tx,rx)=mpsc::channel::<isize>();
        let child=thread::spawn(move || unsafe {
            #[repr(C)]struct WC{style:u32,proc:Option<unsafe extern "system" fn(Hwnd,u32,usize,isize)->isize>,extra:i32,wextra:i32,instance:Hwnd,icon:Hwnd,cursor:Hwnd,background:Hwnd,menu:*const u16,class:*const u16}
            #[repr(C)]struct P{x:i32,y:i32}#[repr(C)]struct M{hwnd:Hwnd,msg:u32,wp:usize,lp:isize,time:u32,pt:P,private:u32}
            #[link(name="kernel32")]extern "system"{fn GetCurrentProcessId()->u32;fn GetModuleHandleW(n:*const u16)->Hwnd;}
            #[link(name="user32")]extern "system"{fn RegisterClassW(w:*const WC)->u16;fn UnregisterClassW(c:*const u16,h:Hwnd)->i32;fn CreateWindowExW(ex:u32,c:*const u16,n:*const u16,style:u32,x:i32,y:i32,w:i32,h:i32,parent:Hwnd,menu:Hwnd,instance:Hwnd,param:Hwnd)->Hwnd;fn DestroyWindow(h:Hwnd)->i32;fn DefWindowProcW(h:Hwnd,m:u32,w:usize,l:isize)->isize;fn GetMessageW(m:*mut M,h:Hwnd,min:u32,max:u32)->i32;fn DispatchMessageW(m:*const M)->isize;fn PostQuitMessage(code:i32);}
            unsafe extern "system" fn wnd(h:Hwnd,m:u32,w:usize,l:isize)->isize{if m==0x8001{thread::sleep(Duration::from_millis(1600));return 42}if m==0x0010{DestroyWindow(h);PostQuitMessage(0);return 0}DefWindowProcW(h,m,w,l)}
            let instance=GetModuleHandleW(std::ptr::null());let class:Vec<u16>=format!("KbSlowFixture{}",GetCurrentProcessId()).encode_utf16().chain([0]).collect();let title:Vec<u16>="test".encode_utf16().chain([0]).collect();
            let wc=WC{style:0,proc:Some(wnd),extra:0,wextra:0,instance,icon:std::ptr::null_mut(),cursor:std::ptr::null_mut(),background:std::ptr::null_mut(),menu:std::ptr::null(),class:class.as_ptr()};assert_ne!(RegisterClassW(&wc),0);
            let h=CreateWindowExW(0,class.as_ptr(),title.as_ptr(),0x80000000,0,0,200,100,std::ptr::null_mut(),std::ptr::null_mut(),instance,std::ptr::null_mut());assert!(!h.is_null());tx.send(h as isize).unwrap();
            let mut msg:M=std::mem::zeroed();while GetMessageW(&mut msg,std::ptr::null_mut(),0,0)>0{DispatchMessageW(&msg);}UnregisterClassW(class.as_ptr(),instance);
        });
        let hwnd=rx.recv().unwrap();let result=message(hwnd as Hwnd,0x8001,0,0);unsafe{PostMessageW(hwnd as Hwnd,0x0010,0,0);};child.join().unwrap();assert_eq!(result,Ok(42));
    }
    #[test] fn real_dpi_aware_window_receives_correct_client_coordinates(){unsafe{
        use std::sync::atomic::{AtomicU32,Ordering};
        static CLICK:AtomicU32=AtomicU32::new(0);
        #[repr(C)]struct WC{style:u32,proc:Option<unsafe extern "system" fn(Hwnd,u32,usize,isize)->isize>,extra:i32,wextra:i32,instance:Hwnd,icon:Hwnd,cursor:Hwnd,background:Hwnd,menu:*const u16,class:*const u16}
        #[repr(C)]struct P{x:i32,y:i32}
        #[repr(C)]struct M{hwnd:Hwnd,msg:u32,wp:usize,lp:isize,time:u32,pt:P,private:u32}
        #[link(name="kernel32")]extern "system"{fn GetCurrentProcessId()->u32;fn GetModuleHandleW(name:*const u16)->Hwnd;}
        #[link(name="user32")]extern "system"{fn RegisterClassW(w:*const WC)->u16;fn UnregisterClassW(c:*const u16,h:Hwnd)->i32;fn CreateWindowExW(ex:u32,c:*const u16,n:*const u16,style:u32,x:i32,y:i32,w:i32,h:i32,parent:Hwnd,menu:Hwnd,instance:Hwnd,param:Hwnd)->Hwnd;fn DestroyWindow(h:Hwnd)->i32;fn DefWindowProcW(h:Hwnd,m:u32,w:usize,l:isize)->isize;fn PeekMessageW(m:*mut M,h:Hwnd,min:u32,max:u32,remove:u32)->i32;fn DispatchMessageW(m:*const M)->isize;fn SetThreadDpiAwarenessContext(v:isize)->isize;}
        unsafe extern "system" fn wnd(h:Hwnd,m:u32,w:usize,l:isize)->isize{if m==0x0201{CLICK.store(l as u32,Ordering::SeqCst);}DefWindowProcW(h,m,w,l)}
        let old=SetThreadDpiAwarenessContext(-4);assert_ne!(old,0);let instance=GetModuleHandleW(std::ptr::null());let class:Vec<u16>=format!("KbDpiFixture{}",GetCurrentProcessId()).encode_utf16().chain([0]).collect();let title:Vec<u16>="test".encode_utf16().chain([0]).collect();
        let wc=WC{style:0,proc:Some(wnd),extra:0,wextra:0,instance,icon:std::ptr::null_mut(),cursor:std::ptr::null_mut(),background:std::ptr::null_mut(),menu:std::ptr::null(),class:class.as_ptr()};assert_ne!(RegisterClassW(&wc),0);
        let h=CreateWindowExW(0,class.as_ptr(),title.as_ptr(),0x80000000,10,10,600,400,std::ptr::null_mut(),std::ptr::null_mut(),instance,std::ptr::null_mut());assert!(!h.is_null());let dpi=GetDpiForWindow(h);eprintln!("click_fixture_dpi={dpi}");
        let previous=SetThreadDpiAwarenessContext(-1);assert_ne!(previous,0);let backend=WindowsBackend{pid:GetCurrentProcessId()};assert_eq!(backend.click_first(h as isize,false),Ok(()));
        let mut m:M=std::mem::zeroed();while PeekMessageW(&mut m,h,0,0,1)!=0{DispatchMessageW(&m);}
        let (x,y)=dpi_geometry::client_point(dpi,600,400,134,29).unwrap();assert_eq!(CLICK.load(Ordering::SeqCst),dpi_geometry::pack_signed(x,y).unwrap() as u32);
        SetThreadDpiAwarenessContext(previous);DestroyWindow(h);UnregisterClassW(class.as_ptr(),instance);SetThreadDpiAwarenessContext(old);
    }}
    #[test] fn native_edit_append_does_not_replace_existing_input(){unsafe{
        #[link(name="user32")] extern "system" {fn CreateWindowExW(ex:u32,class:*const u16,name:*const u16,style:u32,x:i32,y:i32,w:i32,h:i32,parent:Hwnd,menu:Hwnd,instance:Hwnd,param:Hwnd)->Hwnd;fn DestroyWindow(h:Hwnd)->i32;}
        #[link(name="kernel32")] extern "system" {fn GetCurrentProcessId()->u32;}
        let class:Vec<u16>="EDIT".encode_utf16().chain([0]).collect();let value:Vec<u16>="user input".encode_utf16().chain([0]).collect();
        let h=CreateWindowExW(0,class.as_ptr(),value.as_ptr(),0x80000000,0,0,200,30,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut());
        assert!(!h.is_null());let b=WindowsBackend{pid:GetCurrentProcessId()};
        assert_eq!(b.write_text(h as isize,"target"),Ok(()));assert_eq!(b.read_text(h as isize).unwrap(),"user inputtarget");
        assert_ne!(DestroyWindow(h),0);
    }}
}
