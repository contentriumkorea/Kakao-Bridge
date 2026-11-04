//! Background window messages: no global keyboard injection or focus changes.
use std::ffi::c_void;
type Hwnd=*mut c_void;
#[repr(C)] struct LastInputInfo{size:u32,time:u32}
#[link(name="user32")]
extern "system" {
    fn GetForegroundWindow()->Hwnd;
    fn GetWindowThreadProcessId(hwnd:Hwnd,pid:*mut u32)->u32;
    fn IsWindow(hwnd:Hwnd)->i32;
    fn IsChild(parent:Hwnd,child:Hwnd)->i32;
    fn GetWindowTextLengthW(hwnd:Hwnd)->i32;
    fn GetWindowTextW(hwnd:Hwnd,buffer:*mut u16,max:i32)->i32;
    fn SendMessageTimeoutW(hwnd:Hwnd,msg:u32,wp:usize,lp:isize,flags:u32,timeout:u32,result:*mut usize)->isize;
    fn PostMessageW(hwnd:Hwnd,msg:u32,wp:usize,lp:isize)->i32;
    fn GetLastInputInfo(info:*mut LastInputInfo)->i32;
}
fn input_stamp()->Result<u32,&'static str>{unsafe{
    let mut info=LastInputInfo{size:std::mem::size_of::<LastInputInfo>() as u32,time:0};
    if GetLastInputInfo(&mut info)==0{return Err("INPUT_STATE_UNAVAILABLE")}
    Ok(info.time)
}}
fn post_foreground_safe(before:usize,now:usize,chat:usize)->bool {now==before || now==chat}
fn uncertain(_reason:&'static str)->Result<(),&'static str>{
    #[cfg(debug_assertions)] eprintln!("uncertain_reason={_reason}");
    Err("SUBMISSION_UNKNOWN")
}
fn target_valid(window:Hwnd,editor:Hwnd,pid:u32)->bool {unsafe{
    let mut wp=0;let mut ep=0;
    IsWindow(window)!=0 && IsWindow(editor)!=0 && (window==editor || IsChild(window,editor)!=0)
        && GetWindowThreadProcessId(window,&mut wp)!=0 && GetWindowThreadProcessId(editor,&mut ep)!=0 && wp==pid && ep==pid
}}
fn editor_length(editor:Hwnd)->Result<usize,&'static str>{unsafe{
    let mut result=0; if SendMessageTimeoutW(editor,0x000e,0,0,2,2000,&mut result)==0 {Err("EDITOR_UNAVAILABLE")} else {Ok(result)}
}}
fn editor_matches(editor:Hwnd,message:&str)->bool{unsafe{
    let expected:Vec<u16>=message.encode_utf16().collect();
    if editor_length(editor).ok()!=Some(expected.len()){return false}
    let mut actual=vec![0u16;expected.len()+1];let mut got=0;
    SendMessageTimeoutW(editor,0x000d,actual.len(),actual.as_mut_ptr() as isize,2,2000,&mut got)!=0
        && got==expected.len() && actual[..got]==expected && editor_length(editor).ok()==Some(expected.len())
}}
fn title_matches(window:Hwnd,expected:Option<&str>)->bool{unsafe{
    let Some(expected)=expected else{return true};
    let n=GetWindowTextLengthW(window);if n<=0 || n>4096{return false;}
    let mut data=vec![0u16;n as usize+1];let got=GetWindowTextW(window,data.as_mut_ptr(),data.len() as i32);
    got>0 && String::from_utf16_lossy(&data[..got as usize])==expected
}}
/// Only use when the caller explicitly allows replacing a known empty-field hint.
pub fn known_hint(editor:isize)->Result<Option<String>, &'static str>{unsafe{
    let editor=editor as Hwnd;let length=editor_length(editor)?;
    if length==0 {return Ok(None);}if length>128 {return Err("DRAFT_PRESENT");}
    let mut text=vec![0u16;length+1];let mut got=0;
    if SendMessageTimeoutW(editor,0x000d,text.len(),text.as_mut_ptr() as isize,2,2000,&mut got)==0 || got!=length {return Err("EDITOR_UNAVAILABLE");}
    let value=String::from_utf16(&text[..got]).map_err(|_|"EDITOR_UNAVAILABLE")?;
    if !["메시지 입력","메세지 입력","메시지입력","메세지입력","대화내용 입력","메시지를 입력하세요.","메시지를 입력하세요","메시지를 입력해주세요."].contains(&value.as_str()) {return Err("DRAFT_PRESENT");}
    Ok(Some(value))
}}
/// Attempt a submission without any system keyboard injection or focus change.
/// A successful post does not establish delivery and must not be retried blindly.
pub fn attempt(window:isize,editor:isize,pid:u32,message:&str)->Result<(), &'static str>{
    attempt_with_hint(window,editor,pid,message,None)
}
/// `hint` is an explicitly authorized, exact existing value, not a draft guess.
pub fn attempt_with_hint(window:isize,editor:isize,pid:u32,message:&str,hint:Option<&str>)->Result<(), &'static str>{unsafe{
    attempt_impl(window,editor,pid,None,message,hint,false)
}}
pub fn attempt_with_replace(window:isize,editor:isize,pid:u32,message:&str,hint:Option<&str>)->Result<(), &'static str>{unsafe{
    attempt_impl(window,editor,pid,None,message,hint,true)
}}
pub fn attempt_chat(window:isize,editor:isize,pid:u32,title:&str,message:&str,hint:Option<&str>)->Result<(), &'static str>{unsafe{
    attempt_impl(window,editor,pid,Some(title),message,hint,true)
}}
unsafe fn attempt_impl(window:isize,editor:isize,pid:u32,title:Option<&str>,message:&str,hint:Option<&str>,replace:bool)->Result<(), &'static str>{
    let window=window as Hwnd;let editor=editor as Hwnd;
    let before=GetForegroundWindow();
    if before.is_null() || before==window {return Err("TARGET_ACTIVE_OR_FOCUS_UNKNOWN");}
    let input_before=input_stamp()?;
    if !target_valid(window,editor,pid) || !title_matches(window,title) {return Err("TARGET_CHANGED");}
    let length=editor_length(editor)?;
    if length!=0 && !hint.map(|s|editor_matches(editor,s)).unwrap_or(false) {return Err("DRAFT_PRESENT");}
    if GetForegroundWindow()!=before || input_stamp()?!=input_before || !target_valid(window,editor,pid) || !title_matches(window,title) {return Err("INPUT_INTERRUPTED");}
    if length!=0 && !hint.map(|s|editor_matches(editor,s)).unwrap_or(false) {return Err("DRAFT_PRESENT");}
    if length==0 && editor_length(editor)?!=0 {return Err("DRAFT_PRESENT");}
    let wide:Vec<u16>=message.encode_utf16().chain([0]).collect();let mut result=0;
    if replace {
        if SendMessageTimeoutW(editor,0x00b1,0,-1,2,2000,&mut result)==0 {return Err("DRAFT_STATE_UNKNOWN");}
        if GetForegroundWindow()!=before || input_stamp()?!=input_before || !target_valid(window,editor,pid) || !title_matches(window,title) {return Err("INPUT_INTERRUPTED");}
        if SendMessageTimeoutW(editor,0x00c2,0,wide.as_ptr() as isize,2,2000,&mut result)==0 {return Err("DRAFT_STATE_UNKNOWN");}
    } else if SendMessageTimeoutW(editor,0x000c,0,wide.as_ptr() as isize,2,2000,&mut result)==0 || result==0 {return Err("DRAFT_STATE_UNKNOWN");}
    if GetForegroundWindow()!=before || input_stamp().ok()!=Some(input_before) || !target_valid(window,editor,pid) || !title_matches(window,title) || !editor_matches(editor,message) {return Err("DRAFT_PREPARED_UNSENT");}
    // KakaoTalk processes the queued keyboard message through its normal message loop.
    // Once posted, failure is always uncertain: do not retry automatically.
    if PostMessageW(editor,0x0100,13,0)==0 {return Err("DRAFT_PREPARED_UNSENT");}
    if PostMessageW(editor,0x0101,13,0)==0 {return Err("SUBMISSION_UNKNOWN");}
    for _ in 0..40 {
        if !post_foreground_safe(before as usize,GetForegroundWindow() as usize,window as usize) {return uncertain("foreground_changed")}
        if input_stamp().ok()!=Some(input_before) {return uncertain("user_input_changed")}
        if !target_valid(window,editor,pid) || !title_matches(window,title) {return uncertain("target_changed")}
        let length=editor_length(editor).map_err(|_|"SUBMISSION_UNKNOWN")?;
        if length==0 {return Ok(())}
        if !editor_matches(editor,message) {
            // The receiver can clear the editor between the length and text reads.
            if editor_length(editor).map_err(|_|"SUBMISSION_UNKNOWN")?==0 {return Ok(())}
            if let Ok(Some(_))=known_hint(editor as isize){return Ok(())}
            // Rich-edit contents may briefly change while Kakao handles Enter.
            // Wait for a stable empty editor; any interaction still fails above.
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    uncertain("editor_did_not_clear")
}

#[cfg(test)] mod exact_match_tests {
 use super::*;
 #[test] fn post_allows_app_activated_chat_but_not_unrelated_window(){
  assert!(post_foreground_safe(1,1,2));assert!(post_foreground_safe(1,2,2));assert!(!post_foreground_safe(1,3,2));
 }
 #[test] fn prefix_with_extra_text_is_not_an_exact_match(){unsafe{
  #[link(name="user32")] extern "system" {fn CreateWindowExW(ex:u32,class:*const u16,name:*const u16,style:u32,x:i32,y:i32,w:i32,h:i32,parent:Hwnd,menu:Hwnd,instance:Hwnd,param:Hwnd)->Hwnd;fn DestroyWindow(h:Hwnd)->i32;}
  let class:Vec<u16>="EDIT".encode_utf16().chain([0]).collect();let value:Vec<u16>="prefix-extra".encode_utf16().chain([0]).collect();
  let h=CreateWindowExW(0,class.as_ptr(),value.as_ptr(),0x80000000,0,0,200,30,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut());assert!(!h.is_null());
  assert!(editor_matches(h,"prefix-extra"));assert!(!editor_matches(h,"prefix"));assert_ne!(DestroyWindow(h),0);
 }}
}
