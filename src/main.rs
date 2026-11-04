mod dpi_geometry;
mod popup_ocr;
mod gui_lock;
#[allow(dead_code)] mod send_background;
mod file_send;
mod shortcut;
mod focus_prepare;
mod restore_main;
mod open_room;
mod hint_guard;
use std::{ffi::c_void, io::{self, Read}, mem::size_of, process::ExitCode};

#[repr(C)]
struct ProcessEntry32W {
    size:u32, usage:u32, process_id:u32, default_heap_id:usize,
    module_id:u32, threads:u32, parent_process_id:u32,
    priority_base:i32, flags:u32, exe_file:[u16;260],
}
#[link(name="kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags:u32,pid:u32)->*mut c_void;
    fn Process32FirstW(snapshot:*mut c_void,entry:*mut ProcessEntry32W)->i32;
    fn Process32NextW(snapshot:*mut c_void,entry:*mut ProcessEntry32W)->i32;
    fn CloseHandle(handle:*mut c_void)->i32;
}
#[link(name="user32")]
extern "system" {
    fn EnumWindows(callback:extern "system" fn(*mut c_void,isize)->i32,param:isize)->i32;
    fn EnumChildWindows(parent:*mut c_void,callback:extern "system" fn(*mut c_void,isize)->i32,param:isize)->i32;
    fn GetWindowThreadProcessId(hwnd:*mut c_void,pid:*mut u32)->u32;
    fn GetClassNameW(hwnd:*mut c_void,buffer:*mut u16,max:i32)->i32;
    fn GetWindowTextLengthW(hwnd:*mut c_void)->i32;
    fn GetWindowTextW(hwnd:*mut c_void,buffer:*mut u16,max:i32)->i32;
    fn GetDlgCtrlID(hwnd:*mut c_void)->i32;
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    chat:String,
    message:String,
    #[serde(default)] allow_placeholder:bool,
    #[serde(default)] expected_existing_text:Option<String>,
    #[serde(default)] allow_focus_change:bool,
    #[serde(default)] allow_unverified_title:bool,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRequest {
    chat:String,
    file:String,
    #[serde(default)] allow_focus_change:bool,
    #[serde(default)] allow_unverified_title:bool,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenRequest {
    chat:String,
    #[serde(default, rename="self")] self_chat:bool,
    #[serde(default)] allow_focus_change:bool,
    #[serde(default)] allow_unverified_title:bool,
    #[serde(default)] allow_search_input_changes:bool,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ShowRequest { #[serde(default)] allow_focus_change:bool }
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ListRequest { list:String, #[serde(default)] allow_focus_change:bool, #[serde(default)] allow_search_input_changes:bool }
fn parse_list_request(bytes:&[u8])->Result<ListRequest,&'static str>{
 let r:ListRequest=serde_json::from_slice(bytes).map_err(|_|"INVALID_REQUEST")?;
 if r.list!="friends" && r.list!="chats" {return Err("INVALID_REQUEST")}
 if !r.allow_focus_change{return Err("FOCUS_CONSENT_REQUIRED")}
 if !r.allow_search_input_changes{return Err("SEARCH_INPUT_APPROVAL_REQUIRED")}
 Ok(r)
}
fn parse_show_request(bytes:&[u8])->Result<ShowRequest,&'static str>{
 let r:ShowRequest=serde_json::from_slice(bytes).map_err(|_|"INVALID_REQUEST")?;
 if !r.allow_focus_change{return Err("FOCUS_CONSENT_REQUIRED")}
 Ok(r)
}
fn parse_open_request(bytes:&[u8])->Result<OpenRequest,&'static str>{
    let r:OpenRequest=serde_json::from_slice(bytes).map_err(|_|"INVALID_REQUEST")?;
    if r.chat.trim().is_empty()||r.chat.len()>1024||r.chat.chars().any(char::is_control){return Err("INVALID_REQUEST")}
    if !r.allow_focus_change{return Err("FOCUS_CONSENT_REQUIRED")}
    if !r.allow_unverified_title{return Err("IDENTITY_CONSENT_REQUIRED")}
    Ok(r)
}
enum Action { Text(Request), File(FileRequest), Open(OpenRequest), Show, ShowList(ListRequest) }
enum Outcome { Text, File, Open(&'static str), Show, ShowList(String) }

fn parse_request(bytes:&[u8])->Result<Request,&'static str> {
    let r:Request=serde_json::from_slice(bytes).map_err(|_|"INVALID_REQUEST")?;
    if r.chat.trim().is_empty() || r.chat.len()>1024 || r.chat.chars().any(char::is_control)
        || r.message.trim().is_empty() || r.message.len()>16000 || r.message.chars().any(char::is_control) {
        return Err("INVALID_REQUEST");
    }
    if !r.allow_focus_change {return Err("FOCUS_CONSENT_REQUIRED")}
    if !r.allow_unverified_title {return Err("IDENTITY_CONSENT_REQUIRED")}
    Ok(r)
}
fn parse_file_request(bytes:&[u8])->Result<FileRequest,&'static str> {
    let r:FileRequest=serde_json::from_slice(bytes).map_err(|_|"INVALID_REQUEST")?;
    if r.chat.trim().is_empty() || r.chat.len()>1024 || r.chat.chars().any(char::is_control)
        || r.file.is_empty() || r.file.contains('\0') || r.file.contains('"') || r.file.encode_utf16().count()>32760
        || !std::path::Path::new(&r.file).is_absolute() {return Err("INVALID_REQUEST")}
    if !r.allow_focus_change {return Err("FOCUS_CONSENT_REQUIRED")}
    if !r.allow_unverified_title {return Err("IDENTITY_CONSENT_REQUIRED")}
    Ok(r)
}
fn pids()->Result<Vec<u32>,&'static str>{unsafe{
    let h=CreateToolhelp32Snapshot(2,0);if h as isize == -1{return Err("PROCESS_CHECK_FAILED")}
    let mut e:ProcessEntry32W=std::mem::zeroed();e.size=size_of::<ProcessEntry32W>() as u32;
    if Process32FirstW(h,&mut e)==0 {CloseHandle(h);return Err("PROCESS_CHECK_FAILED")}
    let mut result=Vec::new();loop {
        let end=e.exe_file.iter().position(|c|*c==0).unwrap_or(e.exe_file.len());
        if String::from_utf16_lossy(&e.exe_file[..end]).eq_ignore_ascii_case("KakaoTalk.exe"){result.push(e.process_id)}
        if Process32NextW(h,&mut e)==0 {break}
    }
    CloseHandle(h);Ok(result)
}}
unsafe fn class(h:*mut c_void)->String {
    let mut b=[0u16;256];let n=GetClassNameW(h,b.as_mut_ptr(),256);
    if n<=0 {String::new()} else {String::from_utf16_lossy(&b[..n as usize])}
}
struct Editors { items:Vec<isize> }
extern "system" fn find_editor(h:*mut c_void,p:isize)->i32 {unsafe{
    let ctx=&mut *(p as *mut Editors);
    if class(h)=="RICHEDIT50W" && GetDlgCtrlID(h)==1006 {ctx.items.push(h as isize)}
    1
}}
struct Windows<'a> { pid:u32, title:&'a str, matches:Vec<(isize,isize)>, invalid:bool }
fn select_window(ctx:&Windows<'_>)->Result<(isize,isize),&'static str>{
    if ctx.invalid || ctx.matches.len()>1{return Err("AMBIGUOUS_TARGET")}
    ctx.matches.first().copied().ok_or("TARGET_NOT_FOUND")
}
extern "system" fn visit(h:*mut c_void,p:isize)->i32 {unsafe{
    let ctx=&mut *(p as *mut Windows<'_>);let mut pid=0;GetWindowThreadProcessId(h,&mut pid);
    if pid!=ctx.pid || class(h)!="EVA_Window_Dblclk" {return 1}
    let n=GetWindowTextLengthW(h);if n<=0 || n>4096{return 1}
    let mut b=vec![0u16;n as usize+1];let got=GetWindowTextW(h,b.as_mut_ptr(),b.len() as i32);
    if got<=0 || String::from_utf16_lossy(&b[..got as usize])!=ctx.title{return 1}
    let mut editors=Editors{items:Vec::new()};
    if EnumChildWindows(h,find_editor,&mut editors as *mut _ as isize)!=0 && editors.items.len()==1 {
        ctx.matches.push((h as isize,editors.items[0]));
    } else { ctx.invalid=true; }
    1
}}
fn run()->Result<Outcome,&'static str>{
    let args:Vec<_>=std::env::args_os().collect();
    let mode=if args.len()==1 {"text"} else if args.len()==2 && args[1]=="--file" {"file"} else if args.len()==2 && args[1]=="--open" {"open"} else if args.len()==2 && args[1]=="--show-main" {"show"} else if args.len()==2 && args[1]=="--show-list" {"show-list"} else {return Err("INVALID_REQUEST")};
    let mut input=Vec::new();io::stdin().take(65537).read_to_end(&mut input).map_err(|_|"INVALID_REQUEST")?;
    if input.len()>65536 {return Err("INVALID_REQUEST")}
    let action=match mode {"file"=>Action::File(parse_file_request(&input)?),"open"=>Action::Open(parse_open_request(&input)?),"show"=>{parse_show_request(&input)?;Action::Show},"show-list"=>Action::ShowList(parse_list_request(&input)?),_=>Action::Text(parse_request(&input)?)};
    if let Action::File(r)=&action {
        if !std::path::Path::new(&r.file).is_file() || std::fs::metadata(&r.file).map(|m|m.len()==0).unwrap_or(true) {return Err("FILE_UNAVAILABLE")}
    }
    let pids=pids()?;if pids.len()!=1{return Err("KAKAO_PROCESS_NOT_UNIQUE")}
    let _gui_guard=gui_lock::acquire()?;
    if let Action::Show=action {restore_main::show(pids[0])?;return Ok(Outcome::Show)}
    if let Action::ShowList(r)=&action {restore_main::show(pids[0])?;open_room::select_list(pids[0],&r.list)?;return Ok(Outcome::ShowList(r.list.clone()))}
    if let Action::Open(r)=&action {
        restore_main::show(pids[0])?;
        let state=open_room::attempt(pids[0],&r.chat,r.self_chat,r.allow_search_input_changes)?;
        return Ok(Outcome::Open(state))
    }
    let title=match &action {Action::Text(r)=>&r.chat,Action::File(r)=>&r.chat,_=>unreachable!()};
    let mut ctx=Windows{pid:pids[0],title,matches:Vec::new(),invalid:false};
    if unsafe{EnumWindows(visit,&mut ctx as *mut _ as isize)}==0{return Err("WINDOW_ENUMERATION_FAILED")}
    let (window,editor)=select_window(&ctx)?;
    match &action {
        Action::Text(request)=>{
            let hint=if request.allow_placeholder {hint_guard::authorize(send_background::known_hint(editor)?,request.expected_existing_text.as_deref())?} else {None};
            focus_prepare::prepare(ctx.pid,window)?;
            send_background::attempt_chat(window,editor,ctx.pid,&request.chat,&request.message,hint.as_deref())?;
            Ok(Outcome::Text)
        }
        Action::File(request)=>{
            file_send::attempt(window,editor,ctx.pid,&request.chat,&request.file)?;
            Ok(Outcome::File)
        }
        _=>unreachable!()
    }
}
fn main()->ExitCode {
    match run() {
        Ok(Outcome::Text)=>{println!("{{\"state\":\"submitted_unconfirmed\",\"identity\":\"unverified_title_only\",\"focus_may_change\":true}}");ExitCode::SUCCESS}
        Ok(Outcome::File)=>{println!("{{\"state\":\"submitted_unconfirmed\",\"identity\":\"unverified_title_only\",\"focus_may_change\":true,\"cursor_may_move\":true}}");ExitCode::SUCCESS}
        Ok(Outcome::Open(state))=>{println!("{}",serde_json::json!({"state":state,"identity":"unverified_title_only","focus_may_change":true}));ExitCode::SUCCESS}
        Ok(Outcome::Show)=>{println!("{{\"state\":\"main_window_visible\",\"focus_may_change\":true}}");ExitCode::SUCCESS}
        Ok(Outcome::ShowList(list))=>{println!("{}",serde_json::json!({"state":"list_visible","list":list,"focus_may_change":true}));ExitCode::SUCCESS}
        Err(code)=>{println!("{}",serde_json::json!({"error":code}));ExitCode::FAILURE}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn list_switch_requires_search_risk_consent_and_known_tab(){
        assert_eq!(parse_list_request(br#"{"list":"chats","allow_focus_change":true}"#).err(),Some("SEARCH_INPUT_APPROVAL_REQUIRED"));
        assert_eq!(parse_list_request(br#"{"list":"other","allow_focus_change":true,"allow_search_input_changes":true}"#).err(),Some("INVALID_REQUEST"));
        assert!(parse_list_request(br#"{"list":"friends","allow_focus_change":true,"allow_search_input_changes":true}"#).is_ok());
    }
    #[test] fn malformed_same_title_window_blocks_an_otherwise_unique_match(){
        let ctx=Windows{pid:1,title:"synthetic",matches:vec![(1,2)],invalid:true};
        assert_eq!(select_window(&ctx),Err("AMBIGUOUS_TARGET"));
    }
    #[test] fn explicit_existing_text_can_be_provided_for_hint_replacement(){
        let input=br#"{"chat":"room","message":"x","allow_placeholder":true,"expected_existing_text":"message hint","allow_focus_change":true,"allow_unverified_title":true}"#;
        assert!(parse_request(input).is_ok());
    }
    #[test] fn showing_main_requires_focus_acknowledgement(){
        assert_eq!(parse_show_request(br#"{}"#).err(),Some("FOCUS_CONSENT_REQUIRED"));
        assert!(parse_show_request(br#"{"allow_focus_change":true}"#).is_ok());
        assert!(parse_show_request(br#"{"allow_focus_change":true,"unknown":1}"#).is_err());
    }
    #[test] fn opening_a_room_requires_focus_and_identity_acknowledgement(){
        assert_eq!(parse_open_request(br#"{"chat":"room"}"#).err(),Some("FOCUS_CONSENT_REQUIRED"));
        assert_eq!(parse_open_request(br#"{"chat":"room","allow_focus_change":true}"#).err(),Some("IDENTITY_CONSENT_REQUIRED"));
        let r=parse_open_request(br#"{"chat":"room","self":true,"allow_focus_change":true,"allow_unverified_title":true}"#).unwrap();
        assert!(r.self_chat);
    }
    #[test] fn opening_rejects_empty_control_and_unexpected_fields(){
        assert!(parse_open_request(br#"{"chat":"","allow_focus_change":true,"allow_unverified_title":true}"#).is_err());
        assert!(parse_open_request(br#"{"chat":"a\nb","allow_focus_change":true,"allow_unverified_title":true}"#).is_err());
        assert!(parse_open_request(br#"{"chat":"room","other":true,"allow_focus_change":true,"allow_unverified_title":true}"#).is_err());
    }
    #[test] fn request_rejects_empty_controls_and_unknown_fields(){
        assert!(parse_request(br#"{"chat":"","message":"x"}"#).is_err());
        assert!(parse_request(br#"{"chat":"room","message":"a\nb"}"#).is_err());
        assert!(parse_request(br#"{"chat":"room","message":"x","unknown":1}"#).is_err());
        assert_eq!(parse_request(br#"{"chat":"room","message":"x"}"#).err(),Some("FOCUS_CONSENT_REQUIRED"));
        assert_eq!(parse_request(br#"{"chat":"room","message":"x","allow_focus_change":true}"#).err(),Some("IDENTITY_CONSENT_REQUIRED"));
        assert!(parse_request(br#"{"chat":"room","message":"x","allow_focus_change":true,"allow_unverified_title":true}"#).is_ok());
    }
    #[test] fn file_request_requires_explicit_focus_and_identity_consent(){
        let no_focus=br#"{"chat":"room","file":"C:\\synthetic\\file.txt"}"#;
        assert_eq!(parse_file_request(no_focus).err(),Some("FOCUS_CONSENT_REQUIRED"));
        let no_identity=br#"{"chat":"room","file":"C:\\synthetic\\file.txt","allow_focus_change":true}"#;
        assert_eq!(parse_file_request(no_identity).err(),Some("IDENTITY_CONSENT_REQUIRED"));
        let allowed=br#"{"chat":"room","file":"C:\\synthetic\\file.txt","allow_focus_change":true,"allow_unverified_title":true}"#;
        assert!(parse_file_request(allowed).is_ok());
    }
}
