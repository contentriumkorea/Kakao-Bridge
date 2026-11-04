//! File chooser workflow. The caller must first establish the room identity.
//! This workflow can activate windows and sends a guarded keyboard shortcut;
//! it never uses or modifies the clipboard.
use std::{collections::HashSet,ffi::c_void,path::Path,thread,time::Duration};
use crate::{shortcut,popup_ocr};
type Hwnd=*mut c_void;
#[repr(C)] #[derive(Default)] struct Rect {left:i32,top:i32,right:i32,bottom:i32}
#[repr(C)] struct Point {x:i32,y:i32}
#[link(name="user32")] extern "system" {
    fn EnumWindows(f:extern "system" fn(Hwnd,isize)->i32,l:isize)->i32;
    fn EnumChildWindows(p:Hwnd,f:extern "system" fn(Hwnd,isize)->i32,l:isize)->i32;
    fn GetWindowThreadProcessId(h:Hwnd,p:*mut u32)->u32;
    fn GetClassNameW(h:Hwnd,b:*mut u16,n:i32)->i32;
    fn GetWindowTextLengthW(h:Hwnd)->i32;
    fn GetWindowTextW(h:Hwnd,b:*mut u16,n:i32)->i32;
    fn GetDlgCtrlID(h:Hwnd)->i32;
    fn GetWindow(h:Hwnd,c:u32)->Hwnd;
    fn IsWindow(h:Hwnd)->i32;
    fn IsChild(parent:Hwnd,child:Hwnd)->i32;
    fn IsWindowVisible(h:Hwnd)->i32;
    fn GetWindowRect(h:Hwnd,r:*mut Rect)->i32;
    fn ScreenToClient(h:Hwnd,p:*mut Point)->i32;
    fn GetDpiForWindow(h:Hwnd)->u32;
    fn GetWindowDpiAwarenessContext(h:Hwnd)->isize;
    fn SetThreadDpiAwarenessContext(ctx:isize)->isize;
    fn LogicalToPhysicalPointForPerMonitorDPI(h:Hwnd,p:*mut Point)->i32;
    fn UpdateWindow(h:Hwnd)->i32;
    fn SendMessageTimeoutW(h:Hwnd,m:u32,w:usize,l:isize,flags:u32,ms:u32,res:*mut usize)->isize;
    fn PostMessageW(h:Hwnd,m:u32,w:usize,l:isize)->i32;
}

unsafe fn title(h:Hwnd)->Option<String> {
    let n=GetWindowTextLengthW(h); if n<0 || n>4096{return None}
    if n==0{return Some(String::new())}
    let mut b=vec![0u16;n as usize+1];let n=GetWindowTextW(h,b.as_mut_ptr(),b.len() as i32);
    if n<=0{return None}String::from_utf16(&b[..n as usize]).ok()
}
unsafe fn class(h:Hwnd)->Option<String> {
    let mut b=[0u16;256];let n=GetClassNameW(h,b.as_mut_ptr(),b.len() as i32);
    if n<=0{return None}String::from_utf16(&b[..n as usize]).ok()
}
unsafe fn pid(h:Hwnd)->u32 {let mut p=0;GetWindowThreadProcessId(h,&mut p);p}
unsafe fn target_ok(window:Hwnd,editor:Hwnd,p:u32,expected:&str)->bool {
    IsWindow(window)!=0 && IsWindow(editor)!=0 && IsChild(window,editor)!=0
        && pid(window)==p && pid(editor)==p && title(window).as_deref()==Some(expected)
}
struct Enumerate {pid:u32, windows:Vec<isize>}
extern "system" fn visit(h:Hwnd,l:isize)->i32 {unsafe{
    let e=&mut *(l as *mut Enumerate);if pid(h)==e.pid {e.windows.push(h as isize);}1
}}
unsafe fn windows(p:u32)->Result<Vec<isize>, &'static str> {
    let mut e=Enumerate{pid:p,windows:Vec::new()};
    if EnumWindows(visit,&mut e as *mut _ as isize)==0 {return Err("WINDOW_ENUMERATION_FAILED")}Ok(e.windows)
}
struct Children {class:&'static str,id:i32,items:Vec<isize>}
extern "system" fn visit_child(h:Hwnd,l:isize)->i32 {unsafe{
    let e=&mut *(l as *mut Children);
    if GetDlgCtrlID(h)==e.id && class(h).as_deref()==Some(e.class){e.items.push(h as isize)}1
}}
unsafe fn child(parent:Hwnd, cls:&'static str,id:i32)->Result<Hwnd,&'static str> {
    let mut e=Children{class:cls,id,items:Vec::new()};
    if EnumChildWindows(parent,visit_child,&mut e as *mut _ as isize)==0 {return Err("DIALOG_UNAVAILABLE")}
    if e.items.len()!=1{return Err("DIALOG_CONTROL_NOT_UNIQUE")}Ok(e.items[0] as Hwnd)
}
unsafe fn msg(h:Hwnd,m:u32,w:usize,l:isize)->Result<usize,&'static str> {
    let mut out=0;if SendMessageTimeoutW(h,m,w,l,2,1500,&mut out)==0{return Err("WINDOW_MESSAGE_FAILED")}Ok(out)
}
unsafe fn control_text(h:Hwnd)->Result<String,&'static str> {
    let n=msg(h,0x000e,0,0)?;if n>32767{return Err("DIALOG_CONTENT_INVALID")}
    let mut b=vec![0u16;n+1];let got=msg(h,0x000d,b.len(),b.as_mut_ptr() as isize)?;
    if got!=n{return Err("DIALOG_CONTENT_CHANGED")}String::from_utf16(&b[..got]).map_err(|_|"DIALOG_CONTENT_INVALID")
}
fn scaled(v:i32,dpi:u32)->i32 {((v as i64*dpi as i64+48)/96) as i32}
fn mapped_rect_matches(physical:&Rect,a:&Point,b:&Point)->bool{
    a.x==physical.left && a.y==physical.top && b.x==physical.right && b.y==physical.bottom
}
/// Return the previous and active thread context. If Windows rejects a foreign
/// window's context, only use a 96-DPI fallback after verifying both rectangle
/// corners against the physical coordinate space. Fail closed on mismatches.
unsafe fn enter_window_dpi(window:Hwnd)->Result<(isize,isize),&'static str>{
    let target=GetWindowDpiAwarenessContext(window);
    if target==0{return Err("DPI_UNAVAILABLE")}
    let previous=SetThreadDpiAwarenessContext(target);
    if previous!=0{return Ok((previous,target))}
    if GetDpiForWindow(window)!=96{return Err("DPI_UNAVAILABLE")}
    let pm_previous=SetThreadDpiAwarenessContext(-4);
    if pm_previous==0{return Err("DPI_UNAVAILABLE")}
    let mut physical=Rect::default();let ok=GetWindowRect(window,&mut physical)!=0;
    SetThreadDpiAwarenessContext(pm_previous);
    if !ok{return Err("DPI_UNAVAILABLE")}
    let previous=SetThreadDpiAwarenessContext(-1);
    if previous==0{return Err("DPI_UNAVAILABLE")}
    let mut logical=Rect::default();
    let matched=(||{
        if GetWindowRect(window,&mut logical)==0 || GetDpiForWindow(window)!=96{return false}
        if logical.right<=logical.left || logical.bottom<=logical.top{return false}
        let mut a=Point{x:logical.left,y:logical.top};let mut b=Point{x:logical.right,y:logical.bottom};
        if LogicalToPhysicalPointForPerMonitorDPI(window,&mut a)==0 || LogicalToPhysicalPointForPerMonitorDPI(window,&mut b)==0{return false}
        mapped_rect_matches(&physical,&a,&b)
    })();
    if !matched{SetThreadDpiAwarenessContext(previous);return Err("DPI_UNAVAILABLE")}
    Ok((previous,-1))
}
fn popup_size(w:i32,h:i32,dpi:u32)->bool {
    if !(72..=480).contains(&dpi) || w<=0 || h<=0{return false}
    let min_w=scaled(220,dpi);let max_w=scaled(600,dpi);
    let min_h=scaled(130,dpi);let max_h=scaled(370,dpi);
    w>=min_w && w<=max_w && h>=min_h && h<=max_h
        && i64::from(w)*5>=i64::from(h)*6 && i64::from(w)*5<=i64::from(h)*12
}
unsafe fn geometry(h:Hwnd)->Option<(i32,i32,u32)> {
    let (old,_)=enter_window_dpi(h).ok()?;
    let mut rc=Rect::default();let dpi=GetDpiForWindow(h);
    let ok=GetWindowRect(h,&mut rc)!=0 && dpi!=0;
    SetThreadDpiAwarenessContext(old);
    if ok {Some((rc.right-rc.left,rc.bottom-rc.top,dpi))}else{None}
}
unsafe fn click(h:Hwnd,x:i32,y:i32)->Result<(),&'static str> {
    let (old,_)=enter_window_dpi(h)?;
    let r=(||{
        let mut rc=Rect::default();if GetWindowRect(h,&mut rc)==0{return Err("WINDOW_UNAVAILABLE")}
        let mut point=Point{x:rc.left+x,y:rc.top+y};if ScreenToClient(h,&mut point)==0{return Err("WINDOW_UNAVAILABLE")}
        let lp=(((point.y as u32 & 0xffff)<<16)|(point.x as u32 & 0xffff)) as isize;
        let _=msg(h,0,0,0)?;
        let _=msg(h,0x0200,0,lp)?;thread::sleep(Duration::from_millis(300));
        let _=msg(h,0x0201,1,lp)?;thread::sleep(Duration::from_millis(80));
        // Release may enter a modal loop. Regardless of timeout, inspect the next window.
        let _=msg(h,0x0202,0,lp);Ok(())
    })();
    SetThreadDpiAwarenessContext(old);r
}
unsafe fn wait_dialog(p:u32,window:Hwnd,before:&HashSet<isize>)->Result<Hwnd,&'static str> {
    for _ in 0..400 {
        let matches:Vec<_>=windows(p)?.into_iter().filter(|v|{
            let h=*v as Hwnd;!before.contains(v) && IsWindowVisible(h)!=0 && GetWindow(h,4)==window
                && class(h).as_deref()==Some("#32770")
        }).collect();
        if matches.len()>1{return Err("DIALOG_NOT_UNIQUE")}
        if let Some(v)=matches.first(){return Ok(*v as Hwnd)}
        thread::sleep(Duration::from_millis(50));
    }
    Err("FILE_DIALOG_NOT_FOUND")
}
unsafe fn wait_popup(p:u32,window:Hwnd,before:&HashSet<isize>)->Result<Hwnd,&'static str> {
    for _ in 0..300 {
        let matches:Vec<_>=windows(p)?.into_iter().filter(|v|{
            let h=*v as Hwnd;
            !before.contains(v) && IsWindowVisible(h)!=0 && GetWindow(h,4)==window
                && class(h).as_deref()==Some("EVA_Window_Dblclk")
                && title(h).as_deref()==Some("")
                && geometry(h).map(|(w,h,dpi)|popup_size(w,h,dpi)).unwrap_or(false)
        }).collect();
        if matches.len()>1{return Err("CONFIRMATION_NOT_UNIQUE")}
        if let Some(v)=matches.first(){return Ok(*v as Hwnd)}
        thread::sleep(Duration::from_millis(50));
    }
    Err("CONFIRMATION_NOT_IDENTIFIED")
}
/// Does not establish chat identity. The caller must verify unique room title,
/// the account/self-room relation, and that it is authorized to send first.
/// Success means only that the confirmation closed, not network delivery.
pub fn attempt(window:isize,editor:isize,p:u32,expected:&str,path:&str)->Result<(),&'static str>{unsafe{
    let path_obj=Path::new(path);
    if !path_obj.is_absolute() || path.contains('\0') || path.contains('"')
        || !path_obj.is_file() || std::fs::metadata(path_obj).map(|m|m.len()==0).unwrap_or(true) {
        return Err("INVALID_FILE");
    }
    let (window,editor)=(window as Hwnd,editor as Hwnd);
    if !target_ok(window,editor,p,expected){return Err("TARGET_CHANGED")}
    popup_ocr::preflight()?;
    if !target_ok(window,editor,p,expected){return Err("TARGET_CHANGED")}
    let before:HashSet<_>=windows(p)?.into_iter().collect();
    // An existing owned popup could be a pending transfer or a user's dialog.
    // Never click through or dismiss it on a new request.
    if before.iter().any(|v|IsWindowVisible(*v as Hwnd)!=0 && GetWindow(*v as Hwnd,4)==window) {
        return Err("TARGET_HAS_OPEN_DIALOG");
    }
    if !target_ok(window,editor,p,expected) || IsWindowVisible(window)==0{return Err("TARGET_UNAVAILABLE")}
    let opened=shortcut::request(window as isize,||target_ok(window,editor,p,expected))?;
    let _=UpdateWindow(window);
    let dialog=wait_dialog(p,window,&before)?;
    if !target_ok(window,editor,p,expected){return Err("TARGET_CHANGED_DIALOG_OPEN")}
    if shortcut::foreground()!=dialog as isize || shortcut::stamp()!=Some(opened.input_stamp){return Err("INPUT_INTERRUPTED_DIALOG_OPEN")}
    let edit=child(dialog,"Edit",1148)?;let open=child(dialog,"Button",1)?;
    if !control_text(edit)?.is_empty(){return Err("DIALOG_HAS_EXISTING_CONTENT")}
    if shortcut::foreground()!=dialog as isize || shortcut::stamp()!=Some(opened.input_stamp){return Err("INPUT_INTERRUPTED_DIALOG_OPEN")}
    let wide:Vec<u16>=path.encode_utf16().chain([0]).collect();
    msg(edit,0x000c,0,wide.as_ptr() as isize)?;
    if control_text(edit)?!=path{return Err("DIALOG_CONTENT_CHANGED")}
    if !target_ok(window,editor,p,expected) || GetWindow(dialog,4)!=window {return Err("TARGET_CHANGED_DIALOG_OPEN")}
    if shortcut::foreground()!=dialog as isize || shortcut::stamp()!=Some(opened.input_stamp){return Err("INPUT_INTERRUPTED_DIALOG_OPEN")}
    if PostMessageW(open,0x00f5,0,0)==0{return Err("OPEN_REQUEST_FAILED")}
    // Some app configurations may send directly when the chooser closes.
    // Any failure after pressing Open must therefore be treated as uncertain.
    let popup=wait_popup(p,window,&before).map_err(|_|"SUBMISSION_UNKNOWN")?;
    await_confirmation(popup as isize,window as isize,dialog as isize,||Ok((shortcut::foreground(),shortcut::stamp()==Some(opened.input_stamp),target_ok(window,editor,p,expected) && GetWindow(popup,4)==window)))?;
    let (w,h,_)=geometry(popup).ok_or("SUBMISSION_UNKNOWN")?;
    let (x,y)=popup_ocr::recognize(popup as isize,p,w,h).ok_or("SUBMISSION_UNKNOWN")?;
    if !target_ok(window,editor,p,expected) || GetWindow(popup,4)!=window || shortcut::foreground()!=popup as isize || shortcut::stamp()!=Some(opened.input_stamp){return Err("SUBMISSION_UNKNOWN")}
    click(popup,x,y).map_err(|_|"SUBMISSION_UNKNOWN")?;
    for _ in 0..100 {
        if IsWindow(popup)==0 || IsWindowVisible(popup)==0{shortcut::restore_if_safe(&opened,window as isize);return Ok(())}
        thread::sleep(Duration::from_millis(50));
    }
    Err("SUBMISSION_UNKNOWN")
}}

fn await_confirmation<F:FnMut()->Result<(isize,bool,bool),&'static str>>(popup:isize,chat:isize,dialog:isize,mut sample:F)->Result<(),&'static str>{
 let deadline=std::time::Instant::now()+Duration::from_secs(3);
 loop {
  let (foreground,unchanged,target_valid)=sample().map_err(|_|"SUBMISSION_UNKNOWN")?;
  if !unchanged || !target_valid{return Err("SUBMISSION_UNKNOWN")}
  if foreground==popup{return Ok(())}
  if foreground!=chat && foreground!=dialog{return Err("SUBMISSION_UNKNOWN")}
  if std::time::Instant::now()>=deadline{return Err("SUBMISSION_UNKNOWN")}
  thread::sleep(Duration::from_millis(50));
 }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn waits_for_confirmation_focus_without_crossing_user_changes(){
        let mut samples=vec![(30,true,true),(20,true,true),(40,true,true)].into_iter();
        assert_eq!(await_confirmation(40,20,30,||Ok(samples.next().unwrap())),Ok(()));
        assert_eq!(await_confirmation(40,20,30,||Ok((99,true,true))),Err("SUBMISSION_UNKNOWN"));
        assert_eq!(await_confirmation(40,20,30,||Ok((30,false,true))),Err("SUBMISSION_UNKNOWN"));
        assert_eq!(await_confirmation(40,20,30,||Ok((30,true,false))),Err("SUBMISSION_UNKNOWN"));
    }
    #[test] fn dpi_fallback_rejects_mismatched_coordinate_spaces(){
        let physical=Rect{left:20,top:40,right:220,bottom:440};
        assert!(mapped_rect_matches(&physical,&Point{x:20,y:40},&Point{x:220,y:440}));
        assert!(!mapped_rect_matches(&physical,&Point{x:20,y:40},&Point{x:221,y:440}));
        assert!(!mapped_rect_matches(&physical,&Point{x:19,y:40},&Point{x:220,y:440}));
    }
    #[test] #[ignore] fn real_kakao_chat_geometry_is_available_without_touching_ui(){unsafe{
        let pid:u32=std::env::var("KAKAO_DPI_TEST_PID").unwrap().parse().unwrap();
        let h=windows(pid).unwrap().into_iter().map(|v|v as Hwnd).find(|h|IsWindowVisible(*h)!=0 && class(*h).as_deref()==Some("EVA_Window_Dblclk") && child(*h,"RICHEDIT50W",1006).is_ok()).expect("real chat window required");
        assert!(geometry(h).is_some(),"DPI_UNAVAILABLE for real chat window");
    }}
    #[test] fn allows_a_larger_accessible_text_layout_but_rejects_unrelated_shape(){
        assert!(popup_size(450,270,96));
        assert!(popup_size(900,540,192));
        assert!(!popup_size(500,185,96));
        assert!(!popup_size(300,500,96));
    }
    #[test] fn popup_geometry_scales_and_rejects_unrelated_windows(){
        assert!(popup_size(300,185,96));assert!(popup_size(450,278,144));
        assert!(!popup_size(500,185,96));assert!(!popup_size(300,300,96));
    }
}
