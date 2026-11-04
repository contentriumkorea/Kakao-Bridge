pub trait Ops {
 fn user_input(&self)->Option<(u32,u32)>;
 fn keys_busy(&self)->bool;
 fn target_valid(&self)->bool;
 fn foreground(&self)->isize;
 fn target(&self)->isize;
 fn activate(&self)->bool;
 fn inject(&self)->u32;
}
#[derive(Debug,PartialEq)]pub struct Opened {pub previous_foreground:isize,pub input_stamp:u32}
pub fn activate_and_shortcut<B:Ops>(ops:&B)->Result<Opened,&'static str>{
 let (stamp,now)=ops.user_input().ok_or("INPUT_STATE_UNAVAILABLE")?;
 if now.wrapping_sub(stamp)<1200 || ops.keys_busy(){return Err("INPUT_BUSY")}
 if !ops.target_valid(){return Err("TARGET_CHANGED")}
 let previous=ops.foreground();if previous==0{return Err("FOCUS_UNKNOWN")}
 if previous!=ops.target(){
  for _ in 0..3 {
   if !ops.target_valid(){return Err("TARGET_CHANGED")}
   if ops.keys_busy() || ops.user_input().map(|x|x.0)!=Some(stamp){return Err("INPUT_INTERRUPTED")}
   let fg=ops.foreground();if fg==ops.target(){break}if fg!=previous{return Err("INPUT_INTERRUPTED")}
   let _=ops.activate();
   if ops.foreground()!=ops.target(){std::thread::sleep(std::time::Duration::from_millis(150));}
  }
 }
 if !ops.target_valid(){return Err("TARGET_CHANGED")}
 if ops.keys_busy() || ops.user_input().map(|x|x.0)!=Some(stamp){return Err("INPUT_INTERRUPTED")}
 let fg=ops.foreground();if fg!=ops.target(){return if fg==previous{Err("FOCUS_SWITCH_FAILED")}else{Err("INPUT_INTERRUPTED")}}
 if ops.inject()!=4{return Err("SHORTCUT_INPUT_UNKNOWN")}
 let after=ops.user_input().ok_or("SHORTCUT_INPUT_UNKNOWN")?.0;
 Ok(Opened{previous_foreground:previous,input_stamp:after})
}
use std::{ffi::c_void,mem::size_of};
type Hwnd=*mut c_void;
#[repr(C)]struct LastInput{size:u32,time:u32}
#[repr(C)]#[derive(Copy,Clone)]struct Key{vk:u16,scan:u16,flags:u32,time:u32,extra:usize}
#[repr(C)]#[derive(Copy,Clone)]struct Mouse{dx:i32,dy:i32,data:u32,flags:u32,time:u32,extra:usize}
#[repr(C)]union EventData{k:Key,m:Mouse}
#[repr(C)]struct Event{kind:u32,data:EventData}
#[repr(C)]struct Point{x:i32,y:i32}
#[repr(C)]struct Msg{hwnd:Hwnd,msg:u32,wp:usize,lp:isize,time:u32,pt:Point,private:u32}
#[link(name="kernel32")]extern "system"{fn GetTickCount()->u32;fn GetCurrentThreadId()->u32;}
#[link(name="user32")]extern "system"{
 fn GetForegroundWindow()->Hwnd;fn SetForegroundWindow(h:Hwnd)->i32;
 fn IsWindow(h:Hwnd)->i32;fn IsIconic(h:Hwnd)->i32;fn ShowWindowAsync(h:Hwnd,c:i32)->i32;fn GetAsyncKeyState(k:i32)->i16;
 fn GetLastInputInfo(l:*mut LastInput)->i32;
 fn SendInput(n:u32,p:*const Event,sz:i32)->u32;
 fn GetWindowThreadProcessId(h:Hwnd,p:*mut u32)->u32;
 fn AttachThreadInput(a:u32,b:u32,attach:i32)->i32;
 fn PeekMessageW(m:*mut Msg,h:Hwnd,min:u32,max:u32,remove:u32)->i32;
 fn BringWindowToTop(h:Hwnd)->i32;fn SetActiveWindow(h:Hwnd)->Hwnd;
}
fn key(vk:u16,flags:u32)->Event{Event{kind:1,data:EventData{k:Key{vk,scan:0,flags,time:0,extra:0}}}}
fn relevant_key(key:i32)->bool{
 matches!(key,0x01|0x02|0x04|0x05|0x06|0x08|0x09|0x0c|0x0d|0x10..=0x1b|0x20..=0x2f|0x30..=0x39|0x41..=0x5d|0x60..=0x87|0x90|0x91|0xa0..=0xa5|0xba..=0xc0|0xdb..=0xdf|0xe2)
}
pub fn stamp()->Option<u32>{unsafe{let mut v=LastInput{size:size_of::<LastInput>() as u32,time:0};if GetLastInputInfo(&mut v)==0{None}else{Some(v.time)}}}
struct Native<F>{target:isize,valid:F}
impl<F:Fn()->bool> Ops for Native<F>{
 fn user_input(&self)->Option<(u32,u32)>{stamp().map(|s|(s,unsafe{GetTickCount()}))}
 fn keys_busy(&self)->bool{(1..=0xfe).filter(|k|relevant_key(*k)).any(|k|unsafe{GetAsyncKeyState(k)<0})}
 fn target_valid(&self)->bool{(self.valid)()}
 fn foreground(&self)->isize{unsafe{GetForegroundWindow() as isize}}
 fn target(&self)->isize{self.target}
 fn activate(&self)->bool{unsafe{
  let h=self.target as Hwnd;if IsIconic(h)!=0{if ShowWindowAsync(h,9)==0{return false}std::thread::sleep(std::time::Duration::from_millis(250));}
  if GetForegroundWindow()==h || SetForegroundWindow(h)!=0{return true}
  let before=GetForegroundWindow();let before_stamp=stamp();if before.is_null()||before_stamp.is_none(){return false}
  let own=GetCurrentThreadId();let foreground_thread=GetWindowThreadProcessId(before,std::ptr::null_mut());let target_thread=GetWindowThreadProcessId(h,std::ptr::null_mut());
  if foreground_thread==0||target_thread==0||foreground_thread==own||target_thread==own{return false}
  let mut msg:Msg=std::mem::zeroed();PeekMessageW(&mut msg,std::ptr::null_mut(),0,0,0);
  if AttachThreadInput(own,foreground_thread,1)==0{return false}
  let mut attached_target=false;
  if target_thread!=foreground_thread {
   if AttachThreadInput(own,target_thread,1)==0{AttachThreadInput(own,foreground_thread,0);return false}
   attached_target=true;
  }
  if GetForegroundWindow()==before && stamp()==before_stamp && !self.keys_busy() && (self.valid)(){
   let _=BringWindowToTop(h);let _=SetActiveWindow(h);let _=SetForegroundWindow(h);
  }
  let success=GetForegroundWindow()==h;
  if attached_target{AttachThreadInput(own,target_thread,0);}
  AttachThreadInput(own,foreground_thread,0);success
 }}
 fn inject(&self)->u32{unsafe{
  if GetForegroundWindow() as isize!=self.target || !(self.valid)() || self.keys_busy(){return 0}
  let keys=[key(0x11,0),key(0x54,0),key(0x54,2),key(0x11,2)];
  let sent=SendInput(4,keys.as_ptr(),size_of::<Event>() as i32);
  if sent>0 && sent<4 {let release=[key(0x54,2),key(0x11,2)];let _=SendInput(2,release.as_ptr(),size_of::<Event>() as i32);}
  sent
 }}
}
pub fn request<F:Fn()->bool>(target:isize,valid:F)->Result<Opened,&'static str>{activate_and_shortcut(&Native{target,valid})}
pub fn foreground()->isize{unsafe{GetForegroundWindow() as isize}}
pub fn restore_if_safe(opened:&Opened,target:isize){unsafe{
 if opened.previous_foreground!=0 && opened.previous_foreground!=target && foreground()==target
  && IsWindow(opened.previous_foreground as Hwnd)!=0 && stamp()==Some(opened.input_stamp){SetForegroundWindow(opened.previous_foreground as Hwnd);}
}}
#[cfg(test)]mod tests{
 use super::*;use std::cell::Cell;
 struct Fake{now:u32,stamp:Cell<u32>,busy:bool,valid:bool,fg:Cell<isize>,allow:bool,changed:bool,sent:u32,calls:Cell<u32>,attempts:Cell<u32>,allow_after:u32}
 impl Ops for Fake{
  fn user_input(&self)->Option<(u32,u32)>{Some((self.stamp.get(),self.now))}fn keys_busy(&self)->bool{self.busy}fn target_valid(&self)->bool{self.valid}
  fn foreground(&self)->isize{self.fg.get()}fn target(&self)->isize{2}
  fn activate(&self)->bool{let n=self.attempts.get()+1;self.attempts.set(n);if self.changed{self.stamp.set(self.stamp.get()+1)}if self.allow && n>=self.allow_after{self.fg.set(2);true}else{false}}
  fn inject(&self)->u32{self.calls.set(self.calls.get()+1);self.sent}
 }
 fn fake()->Fake{Fake{now:5000,stamp:Cell::new(1000),busy:false,valid:true,fg:Cell::new(1),allow:true,changed:false,sent:4,calls:Cell::new(0),attempts:Cell::new(0),allow_after:1}}
 #[test]fn never_injects_when_the_user_is_active(){let mut f=fake();f.busy=true;assert_eq!(activate_and_shortcut(&f),Err("INPUT_BUSY"));assert_eq!(f.calls.get(),0);}
 #[test]fn recent_activity_blocks_before_activation(){let mut f=fake();f.now=1500;assert_eq!(activate_and_shortcut(&f),Err("INPUT_BUSY"));assert_eq!(f.fg.get(),1);assert_eq!(f.calls.get(),0);}
 #[test]fn wrong_target_never_injects(){let mut f=fake();f.valid=false;assert_eq!(activate_and_shortcut(&f),Err("TARGET_CHANGED"));assert_eq!(f.calls.get(),0);}
 #[test]fn retries_transient_focus_refusal_before_any_shortcut(){let mut f=fake();f.allow_after=2;let r=activate_and_shortcut(&f).unwrap();assert_eq!(r.previous_foreground,1);assert_eq!(f.attempts.get(),2);assert_eq!(f.calls.get(),1);}
 #[test]fn never_injects_when_focus_was_not_obtained(){let mut f=fake();f.allow=false;assert_eq!(activate_and_shortcut(&f),Err("FOCUS_SWITCH_FAILED"));assert_eq!(f.calls.get(),0);}
 #[test]fn never_injects_if_user_input_changes_during_focus(){let mut f=fake();f.changed=true;assert_eq!(activate_and_shortcut(&f),Err("INPUT_INTERRUPTED"));assert_eq!(f.calls.get(),0);}
 #[test]fn partial_injection_is_uncertain_and_never_retried(){let mut f=fake();f.sent=2;assert_eq!(activate_and_shortcut(&f),Err("SHORTCUT_INPUT_UNKNOWN"));assert_eq!(f.calls.get(),1);}
 #[test]fn sends_one_shortcut_when_the_target_is_safe(){let f=fake();let r=activate_and_shortcut(&f).unwrap();assert_eq!(r.previous_foreground,1);assert_eq!(f.calls.get(),1);}
 #[test]fn excludes_undefined_key_codes_from_busy_detection(){
  for key in 0x3a..=0x40{assert!(!relevant_key(key))}assert!(relevant_key(0x41));assert!(relevant_key(0x11));assert!(relevant_key(0x01));
 }
}
