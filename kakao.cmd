@echo off
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%~dp0main.ps1" %*
exit /b %errorlevel%
