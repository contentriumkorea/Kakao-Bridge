param()
$ErrorActionPreference='Stop'
[Console]::InputEncoding=[Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
try {
 $raw=[Console]::In.ReadToEnd();if($raw.Length -gt 1024){throw 'INVALID'}
 $req=$raw|ConvertFrom-Json
 if($req.probe -eq $true){
  Add-Type -AssemblyName System.Runtime.WindowsRuntime
  $null=[Windows.Media.Ocr.OcrEngine,Windows.Foundation,ContentType=WindowsRuntime]
  $null=[Windows.Globalization.Language,Windows.Globalization,ContentType=WindowsRuntime]
  $engine=[Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('ko-KR'))
  [Console]::Out.WriteLine((@{ok=$true;korean_ocr=($null -ne $engine)}|ConvertTo-Json -Compress));exit 0
 }
 $h=[IntPtr]::new([Int64]::Parse([string]$req.hwnd));$pidExpected=[uint32]$req.pid
 Add-Type -AssemblyName System.Drawing
 Add-Type -AssemblyName System.Runtime.WindowsRuntime
 Add-Type @"
using System;using System.Runtime.InteropServices;
public static class KbPopupOcr {
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int left,top,right,bottom;}
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h,IntPtr dc,uint flags);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
}
"@
 [uint32]$owner=0;[void][KbPopupOcr]::GetWindowThreadProcessId($h,[ref]$owner)
 if($owner -ne $pidExpected -or -not [KbPopupOcr]::IsWindowVisible($h)){throw 'WINDOW_CHANGED'}
 $rect=[KbPopupOcr+Rect]::new();if(-not [KbPopupOcr]::GetWindowRect($h,[ref]$rect)){throw 'NO_RECT'}
 $w=$rect.right-$rect.left;$hh=$rect.bottom-$rect.top;if($w -lt 100 -or $hh -lt 80 -or $w -gt 1600 -or $hh -gt 1200){throw 'BAD_SIZE'}
 $bitmap=[Drawing.Bitmap]::new($w,$hh);$graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{$dc=$graphics.GetHdc();try{$ok=[KbPopupOcr]::PrintWindow($h,$dc,0)}finally{$graphics.ReleaseHdc($dc)};if(-not $ok){throw 'CAPTURE_FAILED'}}finally{$graphics.Dispose()}
 $scaled=$bitmap
 if($w -lt 500){$scaled=[Drawing.Bitmap]::new($bitmap,$w*2,$hh*2)}
 try{
  $null=[Windows.Storage.Streams.InMemoryRandomAccessStream,Windows.Storage.Streams,ContentType=WindowsRuntime]
  $null=[Windows.Graphics.Imaging.BitmapDecoder,Windows.Graphics.Imaging,ContentType=WindowsRuntime]
  $null=[Windows.Media.Ocr.OcrEngine,Windows.Foundation,ContentType=WindowsRuntime]
  $null=[Windows.Globalization.Language,Windows.Globalization,ContentType=WindowsRuntime]
  $asTask=[System.WindowsRuntimeSystemExtensions].GetMethods()|Where-Object {$_.Name -eq 'AsTask' -and $_.IsGenericMethodDefinition -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'}|Select-Object -First 1
  if(-not $asTask){throw 'NO_ASYNC'}
  function Wait-WinRt($op,[Type]$type){$task=$asTask.MakeGenericMethod($type).Invoke($null,@($op));$task.Wait();return $task.Result}
  $stream=[Windows.Storage.Streams.InMemoryRandomAccessStream]::new()
  try{
   $writer=[IO.WindowsRuntimeStreamExtensions]::AsStreamForWrite($stream);$scaled.Save($writer,[Drawing.Imaging.ImageFormat]::Png);$writer.Flush();$stream.Seek(0)
   $decoder=Wait-WinRt ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
   $software=Wait-WinRt ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
   $engine=[Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('ko-KR'));if(-not $engine){throw 'OCR_UNAVAILABLE'}
   $result=Wait-WinRt ($engine.RecognizeAsync($software)) ([Windows.Media.Ocr.OcrResult])
   $matches=@(foreach($line in $result.Lines){
     if(($line.Text -replace '\s','') -ne '1개전송'){continue}
     $words=@($line.Words);if($words.Count -eq 0){continue}
     $minX=($words|ForEach-Object {$_.BoundingRect.X}|Measure-Object -Minimum).Minimum
     $minY=($words|ForEach-Object {$_.BoundingRect.Y}|Measure-Object -Minimum).Minimum
     $maxX=($words|ForEach-Object {$_.BoundingRect.X+$_.BoundingRect.Width}|Measure-Object -Maximum).Maximum
     $maxY=($words|ForEach-Object {$_.BoundingRect.Y+$_.BoundingRect.Height}|Measure-Object -Maximum).Maximum
     $x=(($minX+$maxX)/2)/$scaled.Width;$y=(($minY+$maxY)/2)/$scaled.Height
     if($x -ge 0.2 -and $x -le 0.8 -and $y -ge 0.6 -and $y -le 0.95){@{x=$x;y=$y}}
   })
   [uint32]$after=0;[void][KbPopupOcr]::GetWindowThreadProcessId($h,[ref]$after);if($after -ne $pidExpected -or -not [KbPopupOcr]::IsWindowVisible($h)){throw 'WINDOW_CHANGED'}
   $out=@{ok=$true;count=$matches.Count;x=0;y=0};if($matches.Count -eq 1){$out.x=$matches[0].x;$out.y=$matches[0].y}
   [Console]::Out.WriteLine(($out|ConvertTo-Json -Compress))
  }finally{$stream.Dispose()}
 }finally{if($scaled -ne $bitmap){$scaled.Dispose()};$bitmap.Dispose()}
}catch{[Console]::Out.WriteLine('{"ok":false}');exit 1}
