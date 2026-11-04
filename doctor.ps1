param()
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
try{
 Add-Type @"
using System;using System.Runtime.InteropServices;
public static class KbDoctorNative {
 [DllImport("user32.dll",SetLastError=true)] public static extern IntPtr OpenInputDesktop(uint f,bool inherit,uint access);
 [DllImport("user32.dll")] public static extern bool CloseDesktop(IntPtr d);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetThreadDesktop(uint id);
 [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern bool GetUserObjectInformation(IntPtr h,int index,System.Text.StringBuilder b,int n,out int needed);
 public static string DesktopName(IntPtr h){if(h==IntPtr.Zero)return null;int n;var b=new System.Text.StringBuilder(256);return GetUserObjectInformation(h,2,b,b.Capacity*2,out n)?b.ToString():null;}
}
"@
 $p=@(Get-Process -Name KakaoTalk -ErrorAction SilentlyContinue)
 $same=if($p.Count -eq 1){$p[0].SessionId -eq (Get-Process -Id $PID).SessionId}else{$false}
 $desk=[KbDoctorNative]::OpenInputDesktop(0,$false,1);$deskOk=$desk -ne [IntPtr]::Zero;$inputName=if($deskOk){[KbDoctorNative]::DesktopName($desk)}else{$null};$threadName=[KbDoctorNative]::DesktopName([KbDoctorNative]::GetThreadDesktop([KbDoctorNative]::GetCurrentThreadId()));$sameDesktop=if($null -ne $inputName -and $null -ne $threadName){$inputName -ceq $threadName}else{$null};if($deskOk){[void][KbDoctorNative]::CloseDesktop($desk)}
 $ocr=$false
 try{Add-Type -AssemblyName System.Runtime.WindowsRuntime;$null=[Windows.Media.Ocr.OcrEngine,Windows.Foundation,ContentType=WindowsRuntime];$null=[Windows.Globalization.Language,Windows.Globalization,ContentType=WindowsRuntime];$ocr=$null -ne [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('ko-KR'))}catch{}
 $dpi=$null;if($p.Count -eq 1 -and $p[0].MainWindowHandle -ne [IntPtr]::Zero){$v=[KbDoctorNative]::GetDpiForWindow($p[0].MainWindowHandle);if($v -gt 0){$dpi=$v}}
 $out=[ordered]@{schema='kakao-bridge-doctor-v1';kakao_process_count=$p.Count;same_session=$same;interactive_desktop_available=$deskOk;input_desktop_matches_thread=$sameDesktop;windows_is_64bit=[Environment]::Is64BitOperatingSystem;windows_version=[Environment]::OSVersion.Version.ToString();vc_runtime_available=(Test-Path -LiteralPath (Join-Path $env:SystemRoot 'System32\vcruntime140.dll'));foreground_window_available=([KbDoctorNative]::GetForegroundWindow() -ne [IntPtr]::Zero);korean_ocr_available=$ocr;main_window_dpi=$dpi;node_available=($null -ne (Get-Command node.exe -ErrorAction SilentlyContinue))}
 [Console]::Out.WriteLine(($out|ConvertTo-Json -Compress))
}catch{[Console]::Out.WriteLine('{"error":"DOCTOR_UNAVAILABLE"}');exit 1}
