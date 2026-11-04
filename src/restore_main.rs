trait RestoreOps {
    fn main_window(&self,pid:u32)->Result<isize,&'static str>;
    fn is_visible(&self,window:isize)->bool;
    fn restore(&self,window:isize,pid:u32)->Result<(),&'static str>;
    fn wait(&self);
}
fn restore_with<B:RestoreOps>(ops:&B,pid:u32)->Result<isize,&'static str>{
 let main=ops.main_window(pid)?;
 if ops.is_visible(main){return Ok(main)}
 ops.restore(main,pid)?;
 for _ in 0..60 {
  if ops.main_window(pid)?!=main{return Err("WINDOW_CHANGED")}
  if ops.is_visible(main){return Ok(main)}
  ops.wait();
 }
 Err("MAIN_WINDOW_RESTORE_UNCONFIRMED")
}

use std::{ffi::c_void,thread,time::Duration};
type Hwnd=*mut c_void;
#[link(name="user32")]
extern "system" {
 fn EnumWindows(cb:extern "system" fn(Hwnd,isize)->i32,param:isize)->i32;
 fn GetWindowThreadProcessId(h:Hwnd,pid:*mut u32)->u32;
 fn GetClassNameW(h:Hwnd,b:*mut u16,n:i32)->i32;
 fn GetWindowTextLengthW(h:Hwnd)->i32;
 fn GetWindowTextW(h:Hwnd,b:*mut u16,n:i32)->i32;
 fn IsWindow(h:Hwnd)->i32;
 fn IsWindowVisible(h:Hwnd)->i32;
 fn IsIconic(h:Hwnd)->i32;
 fn ShowWindowAsync(h:Hwnd,cmd:i32)->i32;
}
unsafe fn is_main(h:Hwnd,pid:u32)->bool {
 let mut owner=0;if IsWindow(h)==0||GetWindowThreadProcessId(h,&mut owner)==0||owner!=pid{return false}
 let mut c=[0u16;256];let n=GetClassNameW(h,c.as_mut_ptr(),256);
 if n<=0||String::from_utf16_lossy(&c[..n as usize])!="EVA_Window_Dblclk"{return false}
 let n=GetWindowTextLengthW(h);if n<=0||n>4096{return false}
 let mut b=vec![0u16;n as usize+1];let got=GetWindowTextW(h,b.as_mut_ptr(),b.len() as i32);
 got>0&&String::from_utf16_lossy(&b[..got as usize])=="카카오톡"
}
struct Search{pid:u32,found:Vec<isize>}
extern "system" fn visit(h:Hwnd,p:isize)->i32{unsafe{let s=&mut *(p as *mut Search);if is_main(h,s.pid){s.found.push(h as isize)}1}}
struct WindowsRestore;
impl RestoreOps for WindowsRestore {
 fn main_window(&self,pid:u32)->Result<isize,&'static str>{unsafe{
  let mut s=Search{pid,found:Vec::new()};
  if EnumWindows(visit,&mut s as *mut _ as isize)==0{return Err("WINDOW_ENUMERATION_FAILED")}
  if s.found.len()!=1{return Err("MAIN_WINDOW_NOT_UNIQUE")}
  Ok(s.found[0])
 }}
 fn is_visible(&self,window:isize)->bool{unsafe{IsWindowVisible(window as Hwnd)!=0&&IsIconic(window as Hwnd)==0}}
 fn restore(&self,window:isize,pid:u32)->Result<(),&'static str>{unsafe{
  if !is_main(window as Hwnd,pid){return Err("WINDOW_CHANGED")}
  if ShowWindowAsync(window as Hwnd,9)==0{return Err("MAIN_WINDOW_RESTORE_FAILED")}
  Ok(())
 }}
 fn wait(&self){thread::sleep(Duration::from_millis(50))}
}
pub fn show(pid:u32)->Result<isize,&'static str>{restore_with(&WindowsRestore,pid)}

#[cfg(test)]
mod tests {
 use super::*;use std::cell::Cell;
 struct Fake {main:Result<isize,&'static str>,visible:Cell<bool>,calls:Cell<u32>,fail:bool}
 impl RestoreOps for Fake {
  fn main_window(&self,_:u32)->Result<isize,&'static str>{self.main}
  fn is_visible(&self,_:isize)->bool{self.visible.get()}
  fn restore(&self,_:isize,_:u32)->Result<(),&'static str>{self.calls.set(self.calls.get()+1);if self.fail {return Err("WINDOW_CHANGED")};self.visible.set(true);Ok(())}
  fn wait(&self){}
 }
 #[test] fn restores_hidden_unique_main_window(){let x=Fake{main:Ok(1),visible:Cell::new(false),calls:Cell::new(0),fail:false};assert_eq!(restore_with(&x,1),Ok(1));assert_eq!(x.calls.get(),1);}
 #[test] fn leaves_visible_main_alone(){let x=Fake{main:Ok(1),visible:Cell::new(true),calls:Cell::new(0),fail:false};assert_eq!(restore_with(&x,1),Ok(1));assert_eq!(x.calls.get(),0);}
 #[test] fn refuses_ambiguous_or_changed_window(){let x=Fake{main:Err("MAIN_WINDOW_NOT_UNIQUE"),visible:Cell::new(false),calls:Cell::new(0),fail:false};assert_eq!(restore_with(&x,1),Err("MAIN_WINDOW_NOT_UNIQUE"));assert_eq!(x.calls.get(),0);let y=Fake{main:Ok(1),visible:Cell::new(false),calls:Cell::new(0),fail:true};assert_eq!(restore_with(&y,1),Err("WINDOW_CHANGED"));}
}
