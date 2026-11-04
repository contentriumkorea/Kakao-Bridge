use std::ffi::c_void;
type Hwnd=*mut c_void;
#[repr(C)]#[derive(Default)]struct Rect{left:i32,top:i32,right:i32,bottom:i32}
#[repr(C)]struct Point{x:i32,y:i32}
#[link(name="user32")]extern "system"{
 fn GetWindowDpiAwarenessContext(h:Hwnd)->isize;fn SetThreadDpiAwarenessContext(v:isize)->isize;
 fn GetDpiForWindow(h:Hwnd)->u32;fn GetWindowRect(h:Hwnd,r:*mut Rect)->i32;
 fn LogicalToPhysicalPointForPerMonitorDPI(h:Hwnd,p:*mut Point)->i32;
}
pub struct ContextGuard(isize);
impl Drop for ContextGuard{fn drop(&mut self){unsafe{SetThreadDpiAwarenessContext(self.0);}}}
pub fn enter(hwnd:isize)->Result<ContextGuard,&'static str>{unsafe{
 let h=hwnd as Hwnd;let target=GetWindowDpiAwarenessContext(h);if target==0{return Err("DPI_UNAVAILABLE")}
 let previous=SetThreadDpiAwarenessContext(target);if previous!=0{return Ok(ContextGuard(previous))}
 if GetDpiForWindow(h)!=96{return Err("DPI_UNAVAILABLE")}
 let pm_previous=SetThreadDpiAwarenessContext(-4);if pm_previous==0{return Err("DPI_UNAVAILABLE")}
 let mut physical=Rect::default();let ok=GetWindowRect(h,&mut physical)!=0;SetThreadDpiAwarenessContext(pm_previous);if !ok{return Err("DPI_UNAVAILABLE")}
 let previous=SetThreadDpiAwarenessContext(-1);if previous==0{return Err("DPI_UNAVAILABLE")}
 let mut logical=Rect::default();
 let matched=if GetWindowRect(h,&mut logical)==0 || GetDpiForWindow(h)!=96 || logical.right<=logical.left || logical.bottom<=logical.top{false}else{
  let mut a=Point{x:logical.left,y:logical.top};let mut b=Point{x:logical.right,y:logical.bottom};
  LogicalToPhysicalPointForPerMonitorDPI(h,&mut a)!=0 && LogicalToPhysicalPointForPerMonitorDPI(h,&mut b)!=0 && a.x==physical.left && a.y==physical.top && b.x==physical.right && b.y==physical.bottom
 };
 if !matched{SetThreadDpiAwarenessContext(previous);return Err("DPI_UNAVAILABLE")}Ok(ContextGuard(previous))
}}
pub fn client_point(dpi:u32,width:i32,height:i32,x:i32,y:i32)->Option<(i32,i32)>{
 if !(72..=480).contains(&dpi) || width<=0 || height<=0 || x<0 || y<0{return None}
 let sx=((i64::from(x)*i64::from(dpi)+48)/96).try_into().ok()?;
 let sy=((i64::from(y)*i64::from(dpi)+48)/96).try_into().ok()?;
 if sx>=width || sy>=height{return None}Some((sx,sy))
}
pub fn pack_signed(x:i32,y:i32)->Option<isize>{
 if !(i16::MIN as i32..=i16::MAX as i32).contains(&x) || !(i16::MIN as i32..=i16::MAX as i32).contains(&y){return None}
 Some((((y as u32 & 0xffff)<<16)|(x as u32 & 0xffff)) as isize)
}
#[cfg(test)] mod tests {use super::*;
 #[test]fn scales_points_and_rejects_out_of_bounds(){
  assert_eq!(client_point(96,500,400,134,29),Some((134,29)));
  assert_eq!(client_point(120,500,400,134,29),Some((168,36)));
  assert_eq!(client_point(144,500,400,134,29),Some((201,44)));
  assert_eq!(client_point(192,500,400,134,29),Some((268,58)));
  assert_eq!(client_point(240,500,400,134,29),Some((335,73)));
  assert_eq!(client_point(0,500,400,134,29),None);assert_eq!(client_point(192,200,400,134,29),None);
 }
 #[test]fn real_window_context_matches_dpi_aware_client_coordinates(){unsafe{
  #[link(name="user32")]extern "system"{fn CreateWindowExW(ex:u32,c:*const u16,n:*const u16,style:u32,x:i32,y:i32,w:i32,h:i32,parent:Hwnd,menu:Hwnd,instance:Hwnd,param:Hwnd)->Hwnd;fn DestroyWindow(h:Hwnd)->i32;fn GetClientRect(h:Hwnd,r:*mut Rect)->i32;}
  let previous=SetThreadDpiAwarenessContext(-4);assert_ne!(previous,0);
  let class:Vec<u16>="STATIC".encode_utf16().chain([0]).collect();let title:Vec<u16>="dpi test".encode_utf16().chain([0]).collect();
  let h=CreateWindowExW(0,class.as_ptr(),title.as_ptr(),0x80000000,10,10,600,400,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null_mut());assert!(!h.is_null());
  let dpi=GetDpiForWindow(h);eprintln!("fixture_dpi={dpi}");let mut expected=Rect::default();assert_ne!(GetClientRect(h,&mut expected),0);
  let unaware=SetThreadDpiAwarenessContext(-1);assert_ne!(unaware,0);
  {let _guard=enter(h as isize).unwrap();let mut actual=Rect::default();assert_ne!(GetClientRect(h,&mut actual),0);assert_eq!((actual.right,actual.bottom),(expected.right,expected.bottom));let pt=client_point(dpi,actual.right,actual.bottom,134,29).unwrap();assert_eq!(pt.0,((134u64*dpi as u64+48)/96) as i32);}
  SetThreadDpiAwarenessContext(unaware);DestroyWindow(h);SetThreadDpiAwarenessContext(previous);
 }}
 #[test]fn preserves_negative_screen_coordinates_and_rejects_overflow(){
  assert_eq!(pack_signed(-5,-3),Some(0xfffdfffb_u32 as isize));
  assert_eq!(pack_signed(-32769,0),None);assert_eq!(pack_signed(0,32768),None);
 }
}
