use std::ffi::c_void;
type Handle=*mut c_void;
#[link(name="kernel32")] extern "system" {
 fn CreateMutexW(attrs:*mut c_void,owner:i32,name:*const u16)->Handle;
 fn WaitForSingleObject(handle:Handle,millis:u32)->u32;
 fn ReleaseMutex(handle:Handle)->i32;
 fn CloseHandle(handle:Handle)->i32;
}
pub struct Guard(Handle);
impl Drop for Guard { fn drop(&mut self){unsafe{ReleaseMutex(self.0);CloseHandle(self.0);}} }
pub fn acquire()->Result<Guard,&'static str>{acquire_named("Local\\KakaoBridge.GuiAutomation.v1")}
pub fn acquire_named(name:&str)->Result<Guard,&'static str>{unsafe{
 let n:Vec<u16>=name.encode_utf16().chain([0]).collect();
 let h=CreateMutexW(std::ptr::null_mut(),0,n.as_ptr());if h.is_null(){return Err("AUTOMATION_LOCK_UNAVAILABLE")}
 match WaitForSingleObject(h,0){
  0=>Ok(Guard(h)),
  0x80=>{ReleaseMutex(h);CloseHandle(h);Err("AUTOMATION_RECOVERY_REQUIRED")},
  0x102=>{CloseHandle(h);Err("AUTOMATION_BUSY")},
  _=>{CloseHandle(h);Err("AUTOMATION_LOCK_UNAVAILABLE")}
 }
}}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn second_actor_is_blocked_until_first_releases() {
  let name=format!("Local\\KakaoBridge.Test.{}",std::process::id());
  let first=acquire_named(&name).unwrap();
  let n=name.clone();assert!(std::thread::spawn(move || acquire_named(&n).is_err()).join().unwrap());
  drop(first);assert!(std::thread::spawn(move || acquire_named(&name).is_ok()).join().unwrap());
 }
}
