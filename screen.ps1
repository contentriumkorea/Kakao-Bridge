param([string]$Command)
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
if ($Command -notin @('read','read-chat','read-friends','read-chats') -or $args.Count -ne 0) {
  [Console]::Out.WriteLine('{"error":"UNKNOWN_COMMAND"}'); exit 2
}
try {
  $env:TEMP=Join-Path $PSScriptRoot 'tmp'; $env:TMP=$env:TEMP
  New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
  Add-Type -AssemblyName System.Drawing
  Add-Type -AssemblyName System.Runtime.WindowsRuntime
  $title=$null
  if ($Command -eq 'read-chat') {
    [Console]::InputEncoding=[Text.UTF8Encoding]::new($false)
    $reader=[Console]::In; $buffer=New-Object char[] 4097; $total=0
    while ($total -lt $buffer.Length) { $n=$reader.Read($buffer,$total,$buffer.Length-$total); if($n -eq 0){break}; $total+=$n }
    if ($total -eq 0 -or $total -gt 4096) { throw 'INVALID_REQUEST' }
    try { $request=(-join $buffer[0..($total-1)]) | ConvertFrom-Json } catch { throw 'INVALID_REQUEST' }
    if ($null -eq $request -or @($request.PSObject.Properties).Count -ne 1 -or $request.PSObject.Properties.Name -cne 'chat' -or $request.chat -isnot [string]) { throw 'INVALID_REQUEST' }
    $title=$request.chat
    if ([string]::IsNullOrWhiteSpace($title) -or $title.Length -gt 1024 -or $title -match '[\x00-\x1f\x7f]') { throw 'INVALID_REQUEST' }
  }
  Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class KakaoScreenOcr {
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int left,top,right,bottom; }
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h,IntPtr dc,uint flags);
 public delegate bool Callback(IntPtr h,IntPtr l);
 [DllImport("user32.dll")] static extern bool EnumWindows(Callback f,IntPtr l);
 [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr h,Callback f,IntPtr l);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h,StringBuilder b,int n);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h,StringBuilder b,int n);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowTextLength(IntPtr h);
 [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr h);
 static string ClassOf(IntPtr h){var b=new StringBuilder(256);GetClassName(h,b,b.Capacity);return b.ToString();}
 public static bool MatchesMainWindow(IntPtr h,uint pid){
   uint owner;GetWindowThreadProcessId(h,out owner);if(owner!=pid||ClassOf(h)!="EVA_Window_Dblclk")return false;
   int n=GetWindowTextLength(h);if(n<=0||n>4096)return false;var b=new StringBuilder(n+1);
   return GetWindowText(h,b,b.Capacity)>0&&b.ToString()=="\uCE74\uCE74\uC624\uD1A1";
 }
 public static IntPtr[] FindMainWindows(uint pid){
   var found=new List<IntPtr>();EnumWindows((h,l)=>{if(MatchesMainWindow(h,pid))found.Add(h);return true;},IntPtr.Zero);return found.ToArray();
 }
 public static IntPtr[] FindChatWindows(uint pid,string title){
   var found=new List<IntPtr>();EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);if(p!=pid||ClassOf(h)!="EVA_Window_Dblclk")return true;
     int n=GetWindowTextLength(h);if(n<=0||n>4096)return true;var b=new StringBuilder(n+1);if(GetWindowText(h,b,b.Capacity)<=0||b.ToString()!=title)return true;
     found.Add(h);return true;},IntPtr.Zero);return found.ToArray();
 }
 public static IntPtr[] FindVisibleLists(IntPtr main,string prefix){
   var found=new List<IntPtr>();EnumChildWindows(main,(h,l)=>{if(!IsWindowVisible(h))return true;
     int n=GetWindowTextLength(h);if(n<=0||n>512)return true;var b=new StringBuilder(n+1);
     if(GetWindowText(h,b,b.Capacity)>0&&b.ToString().StartsWith(prefix,StringComparison.Ordinal))found.Add(h);
     return true;},IntPtr.Zero);return found.ToArray();
 }
 public static bool MatchesChatWindow(IntPtr h,uint pid,string title){
   uint owner;GetWindowThreadProcessId(h,out owner);if(owner!=pid || ClassOf(h)!="EVA_Window_Dblclk")return false;
   int n=GetWindowTextLength(h);if(n<=0||n>4096)return false;var b=new StringBuilder(n+1);if(GetWindowText(h,b,b.Capacity)<=0||b.ToString()!=title)return false;
   int editors=0;bool enumerated=EnumChildWindows(h,(c,x)=>{if(ClassOf(c)=="RICHEDIT50W"&&GetDlgCtrlID(c)==1006)editors++;return true;},IntPtr.Zero);return enumerated&&editors==1;
 }
}
'@
  $null=[Windows.Storage.Streams.InMemoryRandomAccessStream,Windows.Storage.Streams,ContentType=WindowsRuntime]
  $null=[Windows.Graphics.Imaging.BitmapDecoder,Windows.Graphics.Imaging,ContentType=WindowsRuntime]
  $null=[Windows.Graphics.Imaging.SoftwareBitmap,Windows.Graphics.Imaging,ContentType=WindowsRuntime]
  $null=[Windows.Media.Ocr.OcrEngine,Windows.Foundation,ContentType=WindowsRuntime]
  $null=[Windows.Media.Ocr.OcrResult,Windows.Foundation,ContentType=WindowsRuntime]
  $null=[Windows.Globalization.Language,Windows.Globalization,ContentType=WindowsRuntime]
  $asTask=[System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.IsGenericMethodDefinition -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
  if (-not $asTask) { throw 'ASYNC_BRIDGE_UNAVAILABLE' }
  function Wait-WinRt($operation,[Type]$type) {
    $task=$asTask.MakeGenericMethod($type).Invoke($null,@($operation)); $task.Wait(); return $task.Result
  }
  $processes=@(Get-Process -Name KakaoTalk -ErrorAction Stop)
  if ($processes.Count -ne 1) { throw 'KAKAO_PROCESS_NOT_UNIQUE' }
  $target=$processes[0]; $handle=[IntPtr]::Zero; [uint32]$owner=0
  if ($Command -in @('read','read-friends','read-chats')) {
    $mainWindows=@([KakaoScreenOcr]::FindMainWindows([uint32]$target.Id))
    if ($mainWindows.Count -ne 1) { throw 'MAIN_WINDOW_NOT_UNIQUE' }
    $handle=$mainWindows[0]
  }
  if ($Command -eq 'read-chat') {
    $matches=@([KakaoScreenOcr]::FindChatWindows([uint32]$target.Id,$title))
    if ($matches.Count -eq 0) { throw 'TARGET_NOT_FOUND' }
    if ($matches.Count -ne 1) { throw 'AMBIGUOUS_TARGET' }
    $handle=$matches[0]
  }
  if ($handle -eq [IntPtr]::Zero) { throw 'WINDOW_UNAVAILABLE' }
  if ($Command -ne 'read-chat' -and -not [KakaoScreenOcr]::MatchesMainWindow($handle,[uint32]$target.Id)) { throw 'WINDOW_CHANGED' }
  if ($Command -eq 'read-chat' -and -not [KakaoScreenOcr]::MatchesChatWindow($handle,[uint32]$target.Id,$title)) { throw 'WINDOW_CHANGED' }
  $null=[KakaoScreenOcr]::GetWindowThreadProcessId($handle,[ref]$owner)
  if ($owner -ne $target.Id) { throw 'WINDOW_CHANGED' }
  if ([KakaoScreenOcr]::IsIconic($handle)) { throw 'WINDOW_MINIMIZED' }
  if (-not [KakaoScreenOcr]::IsWindowVisible($handle)) { throw 'WINDOW_NOT_VISIBLE' }
  $rect=[KakaoScreenOcr+Rect]::new()
  if (-not [KakaoScreenOcr]::GetWindowRect($handle,[ref]$rect)) { throw 'WINDOW_UNAVAILABLE' }
  $width=$rect.right-$rect.left; $height=$rect.bottom-$rect.top
  if ($width -le 0 -or $height -le 0 -or $width -gt 4096 -or $height -gt 4096) { throw 'UNSUPPORTED_WINDOW_SIZE' }
  $listHandle=[IntPtr]::Zero;$crop=$null;$listPrefix=$null
  if ($Command -in @('read-friends','read-chats')) {
    . (Join-Path $PSScriptRoot 'ocr_region.ps1')
    $listPrefix=if($Command -eq 'read-friends'){'ContactListCtrl'}else{'ChatRoomListCtrl'}
    $lists=@([KakaoScreenOcr]::FindVisibleLists($handle,$listPrefix))
    if($lists.Count -eq 0){throw 'LIST_TAB_NOT_VISIBLE'}
    if($lists.Count -ne 1){throw 'LIST_CONTROL_NOT_UNIQUE'}
    $listHandle=$lists[0];$lr=[KakaoScreenOcr+Rect]::new()
    if(-not [KakaoScreenOcr]::GetWindowRect($listHandle,[ref]$lr)){throw 'LIST_GEOMETRY_UNAVAILABLE'}
    $crop=Get-KakaoListCrop @($rect.left,$rect.top,$rect.right,$rect.bottom) @($lr.left,$lr.top,$lr.right,$lr.bottom) $width $height
  }
  $bitmap=[System.Drawing.Bitmap]::new($width,$height)
  $graphics=[System.Drawing.Graphics]::FromImage($bitmap)
  $stream=[Windows.Storage.Streams.InMemoryRandomAccessStream]::new()
  try {
    $dc=$graphics.GetHdc()
    try { $ok=[KakaoScreenOcr]::PrintWindow($handle,$dc,0) } finally { $graphics.ReleaseHdc($dc) }
    if (-not $ok) { throw 'CAPTURE_FAILED' }
    [uint32]$ownerAfter=0
    $null=[KakaoScreenOcr]::GetWindowThreadProcessId($handle,[ref]$ownerAfter)
    if ($ownerAfter -ne $target.Id) { throw 'WINDOW_CHANGED' }
    if ($Command -ne 'read-chat' -and -not [KakaoScreenOcr]::MatchesMainWindow($handle,[uint32]$target.Id)) { throw 'WINDOW_CHANGED' }
    if ($Command -eq 'read-chat') {
      $current=@([KakaoScreenOcr]::FindChatWindows([uint32]$target.Id,$title))
      if ($current.Count -ne 1 -or $current[0] -ne $handle -or -not [KakaoScreenOcr]::MatchesChatWindow($handle,[uint32]$target.Id,$title)) { throw 'WINDOW_CHANGED' }
    }
    if($crop){
      $current=@([KakaoScreenOcr]::FindVisibleLists($handle,$listPrefix));$now=[KakaoScreenOcr+Rect]::new()
      if($current.Count -ne 1 -or $current[0] -ne $listHandle -or -not [KakaoScreenOcr]::GetWindowRect($listHandle,[ref]$now) -or $now.left -ne $lr.left -or $now.top -ne $lr.top -or $now.right -ne $lr.right -or $now.bottom -ne $lr.bottom){throw 'LIST_CHANGED'}
      $area=[System.Drawing.Rectangle]::new($crop.x,$crop.y,$crop.width,$crop.height)
      $ocrBitmap=$bitmap.Clone($area,$bitmap.PixelFormat)
    }else{$ocrBitmap=$bitmap}
    $writeStream=[System.IO.WindowsRuntimeStreamExtensions]::AsStreamForWrite($stream)
    $ocrBitmap.Save($writeStream,[System.Drawing.Imaging.ImageFormat]::Png); $writeStream.Flush(); $stream.Seek(0)
    $decoder=Wait-WinRt ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
    $softwareBitmap=Wait-WinRt ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
    $engine=[Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('ko-KR'))
    if ($null -eq $engine) { throw 'KOREAN_OCR_UNAVAILABLE' }
    $result=Wait-WinRt ($engine.RecognizeAsync($softwareBitmap)) ([Windows.Media.Ocr.OcrResult])
    $lines=@(foreach ($line in $result.Lines) {
      $words=@(foreach ($word in $line.Words) {
        $r=$word.BoundingRect
        $offsetX=if($crop){$crop.x}else{0};$offsetY=if($crop){$crop.y}else{0}
        [ordered]@{text=$word.Text;bounds=[ordered]@{x=($r.X+$offsetX);y=($r.Y+$offsetY);width=$r.Width;height=$r.Height}}
      })
      [ordered]@{text=$line.Text;words=$words}
    })
    if ($lines.Count -eq 0) { throw 'OCR_EMPTY_OR_CAPTURE_UNAVAILABLE' }
    if ($Command -ne 'read-chat' -and -not [KakaoScreenOcr]::MatchesMainWindow($handle,[uint32]$target.Id)) { throw 'WINDOW_CHANGED' }
    if ($Command -eq 'read-chat') {
      $current=@([KakaoScreenOcr]::FindChatWindows([uint32]$target.Id,$title))
      if ($current.Count -ne 1 -or $current[0] -ne $handle -or -not [KakaoScreenOcr]::MatchesChatWindow($handle,[uint32]$target.Id,$title)) { throw 'WINDOW_CHANGED' }
    }
    if($crop){
      $current=@([KakaoScreenOcr]::FindVisibleLists($handle,$listPrefix));$now=[KakaoScreenOcr+Rect]::new()
      if($current.Count -ne 1 -or $current[0] -ne $listHandle -or -not [KakaoScreenOcr]::GetWindowRect($listHandle,[ref]$now) -or $now.left -ne $lr.left -or $now.top -ne $lr.top -or $now.right -ne $lr.right -or $now.bottom -ne $lr.bottom){throw 'LIST_CHANGED'}
    }
    $scope=switch($Command){'read-chat'{'rendered_chat_window_ocr'}'read-friends'{'visible_friends_list_ocr'}'read-chats'{'visible_chat_list_ocr'}default{'rendered_window_ocr'}}
    $windowKind=if ($Command -eq 'read-chat') {'chat'}else{'main'}
    $response=[ordered]@{scope=$scope;window_kind=$windowKind;complete=$false;identity='unverified_title_only';coordinate_space='captured_window_pixels';width=$width;height=$height;note='OCR may be inaccurate or incomplete; never use as verified recipient identity.';lines=$lines}
    if($crop){
      $response.identity='unverified_ocr'
      $response.note='Visible list only; OCR, scrolling or active filters can omit or alter entries. Never use as verified recipient identity.'
      $response['list_region']=[ordered]@{x=$crop.x;y=$crop.y;width=$crop.width;height=$crop.height}
    }
    [Console]::Out.WriteLine(($response | ConvertTo-Json -Depth 8 -Compress))
  } finally {
    if ($softwareBitmap) { $softwareBitmap.Dispose() }
    if ($crop -and $ocrBitmap) { $ocrBitmap.Dispose() }
    if ($writeStream) { $writeStream.Dispose() }
    $stream.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
  }
} catch {
  $known=@('INVALID_REQUEST','TARGET_NOT_FOUND','AMBIGUOUS_TARGET','MAIN_WINDOW_NOT_UNIQUE','KAKAO_PROCESS_NOT_UNIQUE','WINDOW_UNAVAILABLE','WINDOW_CHANGED','WINDOW_MINIMIZED','WINDOW_NOT_VISIBLE','UNSUPPORTED_WINDOW_SIZE','CAPTURE_FAILED','KOREAN_OCR_UNAVAILABLE','OCR_EMPTY_OR_CAPTURE_UNAVAILABLE','ASYNC_BRIDGE_UNAVAILABLE','LIST_TAB_NOT_VISIBLE','LIST_CONTROL_NOT_UNIQUE','LIST_GEOMETRY_UNAVAILABLE','LIST_CHANGED')
  $errorCode=if ($_.Exception.Message -in $known) { $_.Exception.Message } else { 'SCREEN_READ_FAILED' }
  [Console]::Out.WriteLine((@{error=$errorCode}|ConvertTo-Json -Compress)); exit 1
}
