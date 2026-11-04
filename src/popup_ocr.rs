use std::{io::Write,process::{Command,Stdio},os::windows::process::CommandExt,time::{Duration,Instant}};
pub fn availability(json:&str)->Result<(),&'static str>{
 let v:serde_json::Value=serde_json::from_str(json).map_err(|_|"OCR_PREFLIGHT_UNAVAILABLE")?;
 if v.get("ok").and_then(|x|x.as_bool())!=Some(true){return Err("OCR_PREFLIGHT_UNAVAILABLE")}
 match v.get("korean_ocr").and_then(|x|x.as_bool()){Some(true)=>Ok(()),Some(false)=>Err("KOREAN_OCR_UNAVAILABLE"),None=>Err("OCR_PREFLIGHT_UNAVAILABLE")}
}
pub fn point(json:&str,width:i32,height:i32)->Option<(i32,i32)>{
 let v:serde_json::Value=serde_json::from_str(json).ok()?;
 if v.get("ok")?.as_bool()?!=true || v.get("count")?.as_u64()?!=1 || width<=0 || height<=0{return None}
 let x=v.get("x")?.as_f64()?;let y=v.get("y")?.as_f64()?;
 if !x.is_finite() || !y.is_finite() || !(0.2..=0.8).contains(&x) || !(0.6..=0.95).contains(&y){return None}
 Some(((x*width as f64).round() as i32,(y*height as f64).round() as i32))
}
fn invoke(payload:&serde_json::Value)->Option<String>{
 let exe=std::env::current_exe().ok()?;
 let script=exe.ancestors().take(6).map(|p|p.join("popup-ocr.ps1")).find(|p|p.is_file())?;
 let shell=std::path::PathBuf::from(std::env::var_os("SystemRoot")?).join("System32/WindowsPowerShell/v1.0/powershell.exe");
 let mut child=Command::new(shell).args(["-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass","-File"]).arg(script)
  .creation_flags(0x0800_0000 | 0x0000_4000).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
 if child.stdin.take()?.write_all(payload.to_string().as_bytes()).is_err(){let _=child.kill();let _=child.wait();return None}
 let deadline=Instant::now()+Duration::from_secs(20);
 loop {match child.try_wait(){Ok(Some(_))=>break,Ok(None) if Instant::now()<deadline=>std::thread::sleep(Duration::from_millis(50)),_=>{let _=child.kill();let _=child.wait();return None}}}
 let out=child.wait_with_output().ok()?;if !out.status.success()||out.stdout.len()>1024{return None}
 String::from_utf8(out.stdout).ok()
}
pub fn preflight()->Result<(),&'static str>{let out=invoke(&serde_json::json!({"probe":true})).ok_or("OCR_PREFLIGHT_UNAVAILABLE")?;availability(&out)}
pub fn recognize(hwnd:isize,pid:u32,width:i32,height:i32)->Option<(i32,i32)>{
 let out=invoke(&serde_json::json!({"hwnd":hwnd.to_string(),"pid":pid}))?;point(&out,width,height)
}
#[cfg(test)] mod tests {use super::*;
 #[test]fn preflight_reports_missing_ocr_and_rejects_invalid_json(){
  assert_eq!(availability(r#"{"ok":true,"korean_ocr":true}"#),Ok(()));
  assert_eq!(availability(r#"{"ok":true,"korean_ocr":false}"#),Err("KOREAN_OCR_UNAVAILABLE"));
  assert_eq!(availability("{}"),Err("OCR_PREFLIGHT_UNAVAILABLE"));
 }
 #[test]fn accepts_only_unique_bottom_center_result(){
  assert_eq!(point(r#"{"ok":true,"count":1,"x":0.5,"y":0.77}"#,300,185),Some((150,142)));
  assert_eq!(point(r#"{"ok":true,"count":2,"x":0.5,"y":0.77}"#,300,185),None);
  assert_eq!(point(r#"{"ok":true,"count":1,"x":0.99,"y":0.1}"#,300,185),None);
 }
}
