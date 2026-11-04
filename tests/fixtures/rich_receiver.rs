use std::{ffi::c_void,io::{self,Write},ptr};type Hwnd=*mut c_void;
#[repr(C)]struct Point{x:i32,y:i32}
#[repr(C)]struct Msg{hwnd:Hwnd,msg:u32,wp:usize,lp:isize,time:u32,pt:Point,private:u32}
#[repr(C)]struct WndClass{style:u32,proc:Option<extern "system" fn(Hwnd,u32,usize,isize)->isize>,class_extra:i32,win_extra:i32,instance:Hwnd,icon:Hwnd,cursor:Hwnd,background:Hwnd,menu:*const u16,class:*const u16}
#[link(name="kernel32")]extern "system"{fn LoadLibraryW(s:*const u16)->Hwnd;fn GetModuleHandleW(s:*const u16)->Hwnd;}
#[link(name="user32")]extern "system"{
 fn CreateWindowExW(ex:u32,class:*const u16,title:*const u16,style:u32,x:i32,y:i32,w:i32,h:i32,parent:Hwnd,menu:Hwnd,instance:Hwnd,param:*mut c_void)->Hwnd;
 fn RegisterClassW(c:*const WndClass)->u16;
 fn DefWindowProcW(h:Hwnd,m:u32,w:usize,l:isize)->isize;
 fn SetWindowLongPtrW(h:Hwnd,index:i32,value:isize)->isize;
 fn CallWindowProcW(old:isize,h:Hwnd,m:u32,w:usize,l:isize)->isize;
 fn GetWindowTextW(h:Hwnd,b:*mut u16,n:i32)->i32;
 fn SetWindowTextW(h:Hwnd,text:*const u16)->i32;
 fn PostMessageW(h:Hwnd,m:u32,w:usize,l:isize)->i32;
 fn GetMessageW(m:*mut Msg,h:Hwnd,min:u32,max:u32)->i32;
 fn DispatchMessageW(m:*const Msg)->isize;
 fn PostQuitMessage(code:i32);
}
static mut OLD:isize=0;
extern "system" fn parent_wnd(h:Hwnd,m:u32,w:usize,l:isize)->isize {unsafe{DefWindowProcW(h,m,w,l)}}
extern "system" fn wnd(h:Hwnd,m:u32,w:usize,l:isize)->isize {unsafe{
 if m==0x100 && w==13 {let mut b=[0u16;256];let n=GetWindowTextW(h,b.as_mut_ptr(),256);println!("{}",String::from_utf16_lossy(&b[..n as usize]));let _=io::stdout().flush();let empty=[0u16];SetWindowTextW(h,empty.as_ptr());let id=h as usize;std::thread::spawn(move||{std::thread::sleep(std::time::Duration::from_millis(500));PostMessageW(id as Hwnd,0x8001,0,0);});return 0;}if m==0x8001{PostQuitMessage(0);return 0;}CallWindowProcW(OLD,h,m,w,l)
}}
fn main(){unsafe{
 let dll:Vec<u16>="Msftedit.dll\0".encode_utf16().collect();if LoadLibraryW(dll.as_ptr()).is_null(){std::process::exit(1)}
 let class:Vec<u16>="RICHEDIT50W\0".encode_utf16().collect();let title:Vec<u16>="synthetic-hint\0".encode_utf16().collect();let instance=GetModuleHandleW(ptr::null());
 let child=std::env::args().nth(1).as_deref()==Some("child");let mut parent=ptr::null_mut();
 if child {let parent_class:Vec<u16>="SyntheticParentClass\0".encode_utf16().collect();let parent_title:Vec<u16>="Synthetic Parent\0".encode_utf16().collect();let wc=WndClass{style:0,proc:Some(parent_wnd),class_extra:0,win_extra:0,instance,icon:ptr::null_mut(),cursor:ptr::null_mut(),background:ptr::null_mut(),menu:ptr::null(),class:parent_class.as_ptr()};if RegisterClassW(&wc)==0{std::process::exit(4)}parent=CreateWindowExW(0,parent_class.as_ptr(),parent_title.as_ptr(),0,0,0,100,100,ptr::null_mut(),ptr::null_mut(),instance,ptr::null_mut());if parent.is_null(){std::process::exit(5)}}
 let h=CreateWindowExW(0,class.as_ptr(),title.as_ptr(),if child{0x40000004}else{4},0,0,100,100,parent,ptr::null_mut(),instance,ptr::null_mut());if h.is_null(){std::process::exit(2)}
 OLD=SetWindowLongPtrW(h,-4,wnd as *const () as isize);if OLD==0{std::process::exit(3)}if child{println!("{},{}",parent as usize,h as usize)}else{println!("{}",h as usize)}io::stdout().flush().unwrap();
 let mut msg:Msg=std::mem::zeroed();while GetMessageW(&mut msg,ptr::null_mut(),0,0)>0{DispatchMessageW(&msg);}
}}
