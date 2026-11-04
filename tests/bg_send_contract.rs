use std::{ffi::c_void,io::{BufRead,BufReader},os::windows::process::CommandExt,process::{Child,Command,Stdio},sync::mpsc,thread,time::Duration};
#[allow(dead_code)] #[path="../src/send_background.rs"] mod send_background;
type Hwnd=*mut c_void;
#[link(name="user32")] extern "system"{
 fn GetForegroundWindow()->Hwnd;
 fn GetWindowThreadProcessId(h:Hwnd,pid:*mut u32)->u32;
}
fn attempt(h:usize,text:&str)->Result<(), &'static str>{let mut pid=0;unsafe{GetWindowThreadProcessId(h as Hwnd,&mut pid);};send_background::attempt(h as isize,h as isize,pid,text)}
fn fixture(initial:Option<&str>)->(Child,usize,mpsc::Receiver<std::io::Result<String>>){
 let mut cmd=Command::new(env!("CARGO_BIN_EXE_text_receiver"));if let Some(value)=initial {cmd.arg(value);}let mut child=cmd.creation_flags(0x0800_0000).stdout(Stdio::piped()).spawn().unwrap();
 let (tx,rx)=mpsc::channel();let stdout=child.stdout.take().unwrap();thread::spawn(move||{for line in BufReader::new(stdout).lines(){let _=tx.send(line);}});
 let handle=rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap().parse().unwrap();(child,handle,rx)
}
#[test] fn uncleared_text_after_enter_is_reported_as_uncertain(){
 let (mut child,h,_)=fixture(Some("stuck"));let result=attempt(h,"synthetic text");let _=child.kill();let _=child.wait();assert_eq!(result,Err("SUBMISSION_UNKNOWN"));
}
#[test] fn sends_to_hidden_fixture_without_changing_foreground(){
 let (mut child,h,rx)=fixture(None);let before=unsafe{GetForegroundWindow()};assert_eq!(attempt(h,"synthetic text"),Ok(()));
 let response=rx.recv_timeout(Duration::from_secs(5));if response.is_err(){let _=child.kill();}assert_eq!(response.unwrap().unwrap(),"synthetic text");assert!(child.wait().unwrap().success());assert_eq!(before,unsafe{GetForegroundWindow()});
}
#[test] fn sends_unicode_to_hidden_fixture_without_changing_foreground(){
 let (mut child,h,rx)=fixture(None);let before=unsafe{GetForegroundWindow()};let text="한글 확인 😀";
 assert_eq!(attempt(h,text),Ok(()));
 let response=rx.recv_timeout(Duration::from_secs(5));if response.is_err(){let _=child.kill();}
 assert_eq!(response.unwrap().unwrap(),text);assert!(child.wait().unwrap().success());assert_eq!(before,unsafe{GetForegroundWindow()});
}
#[test] fn refuses_existing_draft(){
 let (mut child,h,_)=fixture(Some("draft"));assert_eq!(attempt(h,"synthetic text"),Err("DRAFT_PRESENT"));let _=child.kill();let _=child.wait();
}
#[test] fn only_exact_authorized_hint_can_be_replaced(){
 let (mut child,h,rx)=fixture(Some("hint"));let mut pid=0;unsafe{GetWindowThreadProcessId(h as Hwnd,&mut pid);}
 assert_eq!(send_background::attempt_with_hint(h as isize,h as isize,pid,"synthetic text",Some("other-hint")),Err("DRAFT_PRESENT"));
 assert_eq!(send_background::attempt_with_hint(h as isize,h as isize,pid,"synthetic text",Some("synthetic-hint")),Ok(()));
 let response=rx.recv_timeout(Duration::from_secs(5));if response.is_err(){let _=child.kill();}assert_eq!(response.unwrap().unwrap(),"synthetic text");assert!(child.wait().unwrap().success());
}
#[test] fn replace_selection_reaches_hidden_richedit_without_focus_change(){
 let mut child=Command::new(env!("CARGO_BIN_EXE_rich_receiver")).creation_flags(0x0800_0000).stdout(Stdio::piped()).spawn().unwrap();
 let (tx,rx)=mpsc::channel();let stdout=child.stdout.take().unwrap();thread::spawn(move||{for line in BufReader::new(stdout).lines(){let _=tx.send(line);}});
 let h:usize=rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap().parse().unwrap();let mut pid=0;unsafe{GetWindowThreadProcessId(h as Hwnd,&mut pid);}let before=unsafe{GetForegroundWindow()};
 assert_eq!(send_background::attempt_with_replace(h as isize,h as isize,pid,"synthetic text",Some("synthetic-hint")),Ok(()));
 let response=rx.recv_timeout(Duration::from_secs(5));if response.is_err(){let _=child.kill();}assert_eq!(response.unwrap().unwrap(),"synthetic text");assert!(child.wait().unwrap().success());assert_eq!(before,unsafe{GetForegroundWindow()});
}
#[test] fn chat_submission_checks_parent_title_and_child_identity(){
 let mut child=Command::new(env!("CARGO_BIN_EXE_rich_receiver")).arg("child").creation_flags(0x0800_0000).stdout(Stdio::piped()).spawn().unwrap();
 let (tx,rx)=mpsc::channel();let stdout=child.stdout.take().unwrap();thread::spawn(move||{for line in BufReader::new(stdout).lines(){let _=tx.send(line);}});
 let ids=rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();let (p,e)=ids.split_once(',').unwrap();let parent:usize=p.parse().unwrap();let editor:usize=e.parse().unwrap();let mut pid=0;unsafe{GetWindowThreadProcessId(parent as Hwnd,&mut pid);}let before=unsafe{GetForegroundWindow()};
 assert_eq!(send_background::attempt_chat(parent as isize,editor as isize,pid,"wrong room","synthetic text",Some("synthetic-hint")),Err("TARGET_CHANGED"));
 assert_eq!(send_background::attempt_chat(parent as isize,editor as isize,pid,"Synthetic Parent","synthetic text",Some("synthetic-hint")),Ok(()));
 let response=rx.recv_timeout(Duration::from_secs(5));if response.is_err(){let _=child.kill();}assert_eq!(response.unwrap().unwrap(),"synthetic text");assert!(child.wait().unwrap().success());assert_eq!(before,unsafe{GetForegroundWindow()});
}
