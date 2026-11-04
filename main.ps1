param([string]$Command)
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
if ($args.Count -ne 0) { [Console]::Out.WriteLine('{"error":"UNKNOWN_COMMAND"}'); exit 2 }
switch ($Command) {
  'capabilities' {
    $response=[ordered]@{
      schema='kakao-bridge-capabilities-v1';edition='public_preview'
      status=@{supported=$true;scope='running_process'}
      doctor=@{supported=$true;scope='read_only_environment'}
      screen=@{supported=$true;scope='visible_main_window_ocr';complete=$false}
      open_chats=@{supported=$true;scope='open_window_titles';complete=$false}
      show_main=@{supported=$true;input=@('allow_focus_change');focus_may_change=$true}
      friends=@{supported=$true;scope='visible_list_ocr';complete=$false;requires='friends_tab_visible'}
      chats=@{supported=$true;scope='visible_list_ocr';complete=$false;requires='chats_tab_visible'}
      show_list=@{supported=$true;input=@('list','allow_focus_change','allow_search_input_changes');focus_may_change=$true}
      messages=@{supported=$true;scope='visible_chat_ocr';complete=$false;identity='unverified_title_only'}
      send=@{supported=$true;scope='open_window';delivery='submitted_unconfirmed';focus_may_change=$true}
      send_file=@{supported=$true;scope='open_window';delivery='submitted_unconfirmed';focus_may_change=$true}
      open=@{supported=$true;identity='unverified_title_only';focus_may_change=$true}
      self_title=@{supported=$true;scope='user_bound_title_only';local_binding_required=$true}
      bind_self=@{supported=$true;scope='current_windows_user';no_overwrite=$true}
      download=@{supported=$false}
    }
    [Console]::Out.WriteLine(($response|ConvertTo-Json -Depth 6 -Compress))
  }
  'bind-self' {
    try {
      [Console]::InputEncoding=[Text.UTF8Encoding]::new($false)
      $reader=[Console]::In;$buffer=New-Object char[] 4097;$total=0
      while($total -lt $buffer.Length){$n=$reader.Read($buffer,$total,$buffer.Length-$total);if($n -eq 0){break};$total+=$n}
      if($total -eq 0 -or $total -gt 4096){throw 'INVALID_REQUEST'}
      try{$request=(-join $buffer[0..($total-1)])|ConvertFrom-Json}catch{throw 'INVALID_REQUEST'}
      if($null -eq $request -or @($request.PSObject.Properties).Count -ne 2 -or $request.chat -isnot [string] -or $request.acknowledge_title_only -isnot [bool] -or -not $request.acknowledge_title_only){throw 'INVALID_REQUEST'}
      if([string]::IsNullOrWhiteSpace($request.chat) -or $request.chat.Length -gt 1024 -or $request.chat -match '[\x00-\x1f\x7f]'){throw 'INVALID_REQUEST'}
      $binding=Join-Path $PSScriptRoot 'self-binding.dat'
      if(Test-Path -LiteralPath $binding){throw 'SELF_BINDING_EXISTS'}
      $null=[Reflection.Assembly]::LoadWithPartialName('System.Security')
      $plain=[Text.Encoding]::UTF8.GetBytes($request.chat)
      try{$protected=[Security.Cryptography.ProtectedData]::Protect($plain,$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)}finally{[Array]::Clear($plain,0,$plain.Length)}
      $cipher=[Convert]::ToBase64String($protected)
      $stream=[IO.File]::Open($binding,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
      try{$bytes=[Text.Encoding]::ASCII.GetBytes($cipher);$stream.Write($bytes,0,$bytes.Length)}finally{$stream.Dispose()}
      [Console]::Out.WriteLine('{"state":"self_binding_saved","identity":"user_bound_title_only"}')
    }catch{
      $known=@('INVALID_REQUEST','SELF_BINDING_EXISTS');$code=if($_.Exception.Message -in $known){$_.Exception.Message}else{'SELF_BINDING_WRITE_FAILED'}
      [Console]::Out.WriteLine((@{error=$code}|ConvertTo-Json -Compress));exit 1
    }
  }
  'self-title' {
    try {
      $binding=Join-Path $PSScriptRoot 'self-binding.dat'
      if(-not (Test-Path -LiteralPath $binding -PathType Leaf)){throw 'SELF_BINDING_NOT_CONFIGURED'}
      $item=Get-Item -LiteralPath $binding -Force
      if(($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $item.Length -gt 32768){throw 'SELF_BINDING_INVALID'}
      $cipher=[IO.File]::ReadAllText($binding,[Text.Encoding]::ASCII)
      $null=[Reflection.Assembly]::LoadWithPartialName('System.Security')
      $protected=[Convert]::FromBase64String($cipher)
      $plain=[Security.Cryptography.ProtectedData]::Unprotect($protected,$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
      try{$title=[Text.Encoding]::UTF8.GetString($plain)}finally{[Array]::Clear($plain,0,$plain.Length)}
      if([string]::IsNullOrWhiteSpace($title) -or $title.Length -gt 1024 -or $title -match '[\x00-\x1f\x7f]'){throw 'SELF_BINDING_INVALID'}
      [Console]::Out.WriteLine((@{chat=$title;identity='user_bound_title_only'}|ConvertTo-Json -Compress))
    }catch{
      $known=@('SELF_BINDING_NOT_CONFIGURED','SELF_BINDING_INVALID');$code=if($_.Exception.Message -in $known){$_.Exception.Message}else{'SELF_BINDING_INVALID'}
      [Console]::Out.WriteLine((@{error=$code}|ConvertTo-Json -Compress));exit 1
    }
  }
  'doctor' {
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'doctor.ps1')
    exit $LASTEXITCODE
  }
  'status' {
    $count=@(Get-Process -Name KakaoTalk -ErrorAction SilentlyContinue).Count
    [Console]::Out.WriteLine((@{edition='public_preview';kakao_running=($count -gt 0)}|ConvertTo-Json -Compress))
  }
  'screen' {
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'screen.ps1') read
    exit $LASTEXITCODE
  }
  'friends' {
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'screen.ps1') read-friends
    exit $LASTEXITCODE
  }
  'chats' {
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'screen.ps1') read-chats
    exit $LASTEXITCODE
  }
  'messages' {
    & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'screen.ps1') read-chat
    exit $LASTEXITCODE
  }
  'send' {
    $sender=Join-Path $PSScriptRoot 'bin\kakao-public-sender.exe'
    if(-not (Test-Path -LiteralPath $sender -PathType Leaf)) { [Console]::Out.WriteLine('{"error":"SENDER_UNAVAILABLE"}');exit 1 }
    & $sender
    exit $LASTEXITCODE
  }
  'send-file' {
    $sender=Join-Path $PSScriptRoot 'bin\kakao-public-sender.exe'
    if(-not (Test-Path -LiteralPath $sender -PathType Leaf)) { [Console]::Out.WriteLine('{"error":"SENDER_UNAVAILABLE"}');exit 1 }
    & $sender --file
    exit $LASTEXITCODE
  }
  'show-main' {
    $sender=Join-Path $PSScriptRoot 'bin\kakao-public-sender.exe'
    if(-not (Test-Path -LiteralPath $sender -PathType Leaf)) { [Console]::Out.WriteLine('{"error":"SENDER_UNAVAILABLE"}');exit 1 }
    & $sender --show-main
    exit $LASTEXITCODE
  }
  'show-list' {
    $sender=Join-Path $PSScriptRoot 'bin\kakao-public-sender.exe'
    if(-not (Test-Path -LiteralPath $sender -PathType Leaf)) { [Console]::Out.WriteLine('{"error":"SENDER_UNAVAILABLE"}');exit 1 }
    & $sender --show-list
    exit $LASTEXITCODE
  }
  'open' {
    $sender=Join-Path $PSScriptRoot 'bin\kakao-public-sender.exe'
    if(-not (Test-Path -LiteralPath $sender -PathType Leaf)) { [Console]::Out.WriteLine('{"error":"SENDER_UNAVAILABLE"}');exit 1 }
    & $sender --open
    exit $LASTEXITCODE
  }
  'open-chats' {
    try {
      $processes=@(Get-Process -Name KakaoTalk -ErrorAction SilentlyContinue)
      if($processes.Count -ne 1){throw 'KAKAO_PROCESS_NOT_UNIQUE'}
      $env:TEMP=Join-Path $PSScriptRoot 'tmp';$env:TMP=$env:TEMP
      New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
      Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class KakaoOpenWindows {
  public delegate bool Callback(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(Callback f,IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr h,Callback f,IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h,StringBuilder b,int n);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h,StringBuilder b,int n);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowTextLength(IntPtr h);
  [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr h);
  static string ClassOf(IntPtr h) { var b=new StringBuilder(256); GetClassName(h,b,b.Capacity); return b.ToString(); }
  public static string[] Find(uint pid) {
    var result=new List<string>();
    EnumWindows((h,l)=> {
      uint owner; GetWindowThreadProcessId(h,out owner);
      if(owner!=pid || ClassOf(h)!="EVA_Window_Dblclk") return true;
      int length=GetWindowTextLength(h); if(length<=0 || length>4096) return true;
      var title=new StringBuilder(length+1); if(GetWindowText(h,title,title.Capacity)<=0) return true;
      bool editor=false;
      EnumChildWindows(h,(c,x)=> {if(ClassOf(c)=="RICHEDIT50W" && GetDlgCtrlID(c)==1006) {editor=true;return false;} return true;},IntPtr.Zero);
      if(editor) result.Add(title.ToString()); return true;
    },IntPtr.Zero);
    return result.ToArray();
  }
}
'@
      $titles=@([KakaoOpenWindows]::Find([uint32]$processes[0].Id))
      [Console]::Out.WriteLine((@{scope='open_windows';chats=$titles}|ConvertTo-Json -Compress))
    } catch {
      $code=if($_.Exception.Message -eq 'KAKAO_PROCESS_NOT_UNIQUE'){'KAKAO_PROCESS_NOT_UNIQUE'}else{'WINDOW_ENUMERATION_FAILED'}
      [Console]::Out.WriteLine((@{error=$code}|ConvertTo-Json -Compress));exit 1
    }
  }
  'download' {
    [Console]::Out.WriteLine('{"error":"FEATURE_NOT_IMPLEMENTED"}'); exit 3
  }
  default { [Console]::Out.WriteLine('{"error":"UNKNOWN_COMMAND"}'); exit 2 }
}
