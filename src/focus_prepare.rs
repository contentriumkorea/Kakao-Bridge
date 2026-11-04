trait FocusOps {
    fn foreground(&self)->isize;
    fn main_window(&self,pid:u32,target:isize)->Result<isize,&'static str>;
    fn activate(&self,main:isize,target:isize,pid:u32)->Result<(),&'static str>;
    fn wait(&self);
}
fn prepare_with<B:FocusOps>(ops:&B,pid:u32,target:isize)->Result<(),&'static str>{
    let before=ops.foreground();
    if before==0{return Err("FOCUS_UNKNOWN")}
    if before!=target{return Ok(())}
    let main=ops.main_window(pid,target)?;
    ops.activate(main,target,pid)?;
    for _ in 0..40 {
        let now=ops.foreground();
        if now==main{return Ok(())}
        if now!=target{return Err("INPUT_INTERRUPTED")}
        ops.wait();
    }
    Err("FOCUS_SWITCH_UNCONFIRMED")
}

use std::{ffi::c_void,thread,time::Duration};
type Hwnd=*mut c_void;
#[link(name="user32")]
extern "system" {
    fn GetForegroundWindow()->Hwnd;
    fn GetWindowThreadProcessId(h:Hwnd,pid:*mut u32)->u32;
    fn EnumWindows(cb:extern "system" fn(Hwnd,isize)->i32,param:isize)->i32;
    fn GetClassNameW(h:Hwnd,b:*mut u16,n:i32)->i32;
    fn GetWindowTextLengthW(h:Hwnd)->i32;
    fn GetWindowTextW(h:Hwnd,b:*mut u16,n:i32)->i32;
    fn IsWindow(h:Hwnd)->i32;
    fn IsWindowVisible(h:Hwnd)->i32;
    fn IsIconic(h:Hwnd)->i32;
    fn ShowWindowAsync(h:Hwnd,cmd:i32)->i32;
    fn SetForegroundWindow(h:Hwnd)->i32;
}
unsafe fn is_main(h:Hwnd,pid:u32)->bool {
    let mut owner=0;if IsWindow(h)==0||GetWindowThreadProcessId(h,&mut owner)==0||owner!=pid{return false}
    let mut c=[0u16;256];let n=GetClassNameW(h,c.as_mut_ptr(),256);
    if n<=0||String::from_utf16_lossy(&c[..n as usize])!="EVA_Window_Dblclk"{return false}
    let n=GetWindowTextLengthW(h);if n<=0||n>4096{return false}
    let mut b=vec![0u16;n as usize+1];let got=GetWindowTextW(h,b.as_mut_ptr(),b.len() as i32);
    got>0&&String::from_utf16_lossy(&b[..got as usize])=="카카오톡"
}
struct Search {pid:u32,target:isize,found:Vec<isize>}
extern "system" fn visit(h:Hwnd,param:isize)->i32 {unsafe{
    let s=&mut *(param as *mut Search);
    if h as isize!=s.target&&is_main(h,s.pid){s.found.push(h as isize)}
    1
}}
struct WindowsFocus;
impl FocusOps for WindowsFocus {
    fn foreground(&self)->isize {unsafe{GetForegroundWindow() as isize}}
    fn main_window(&self,pid:u32,target:isize)->Result<isize,&'static str>{unsafe{
        let mut s=Search{pid,target,found:Vec::new()};
        if EnumWindows(visit,&mut s as *mut _ as isize)==0{return Err("WINDOW_ENUMERATION_FAILED")}
        if s.found.len()!=1{return Err("MAIN_WINDOW_NOT_UNIQUE")}
        Ok(s.found[0])
    }}
    fn activate(&self,main:isize,target:isize,pid:u32)->Result<(),&'static str>{unsafe{
        let h=main as Hwnd;let target=target as Hwnd;
        if GetForegroundWindow()!=target{return Err("INPUT_INTERRUPTED")}
        if !is_main(h,pid){return Err("MAIN_WINDOW_CHANGED")}
        if IsWindowVisible(h)==0||IsIconic(h)!=0 {
            if ShowWindowAsync(h,9)==0{return Err("FOCUS_SWITCH_FAILED")}
        }
        let now=GetForegroundWindow();
        if now!=target&&now!=h{return Err("INPUT_INTERRUPTED")}
        if now==target&&SetForegroundWindow(h)==0{return Err("FOCUS_SWITCH_FAILED")}
        Ok(())
    }}
    fn wait(&self){thread::sleep(Duration::from_millis(50));}
}
pub fn prepare(pid:u32,target:isize)->Result<(),&'static str>{prepare_with(&WindowsFocus,pid,target)}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct Fake {foreground:Cell<isize>,main:Option<isize>,activate_calls:Cell<u32>,steal_on_activation:bool}
    impl FocusOps for Fake {
        fn foreground(&self)->isize {self.foreground.get()}
        fn main_window(&self,_:u32,_:isize)->Result<isize,&'static str>{self.main.ok_or("MAIN_WINDOW_NOT_UNIQUE")}
        fn activate(&self,main:isize,_:isize,_:u32)->Result<(),&'static str>{
            self.activate_calls.set(self.activate_calls.get()+1);
            self.foreground.set(if self.steal_on_activation {99} else {main});Ok(())
        }
        fn wait(&self){}
    }
    fn fake(foreground:isize,main:Option<isize>)->Fake {Fake{foreground:Cell::new(foreground),main,activate_calls:Cell::new(0),steal_on_activation:false}}
    #[test] fn background_target_never_moves_existing_foreground(){
        let x=fake(3,Some(2));assert_eq!(prepare_with(&x,1,4),Ok(()));assert_eq!(x.activate_calls.get(),0);assert_eq!(x.foreground.get(),3);
    }
    #[test] fn active_target_switches_only_to_unique_kakao_main(){
        let x=fake(4,Some(2));assert_eq!(prepare_with(&x,1,4),Ok(()));assert_eq!(x.activate_calls.get(),1);assert_eq!(x.foreground.get(),2);
    }
    #[test] fn missing_main_or_user_focus_change_blocks_submission(){
        let x=fake(4,None);assert_eq!(prepare_with(&x,1,4),Err("MAIN_WINDOW_NOT_UNIQUE"));assert_eq!(x.activate_calls.get(),0);
        let mut y=fake(4,Some(2));y.steal_on_activation=true;assert_eq!(prepare_with(&y,1,4),Err("INPUT_INTERRUPTED"));
    }
}
