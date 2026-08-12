@echo off
setlocal
cd /d "%~dp0"
set "OMNIPHONY_PROFILE=%~1"
if "%OMNIPHONY_PROFILE%"=="" set "OMNIPHONY_PROFILE=all"
start "" "%~dp0Omniphony.exe"
exit /b 0
