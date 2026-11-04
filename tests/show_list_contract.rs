use std::{io::Write,process::{Command,Stdio}};
#[test] fn selecting_a_tab_requires_focus_authorization_before_ui_work(){
 let mut p=Command::new(env!("CARGO_BIN_EXE_kakao-public-sender")).arg("--show-list").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
 p.stdin.take().unwrap().write_all(br#"{"list":"chats"}"#).unwrap();
 let out=p.wait_with_output().unwrap();
 assert!(!out.status.success());
 let value:serde_json::Value=serde_json::from_slice(&out.stdout).unwrap();
 assert_eq!(value["error"],"FOCUS_CONSENT_REQUIRED");
}
