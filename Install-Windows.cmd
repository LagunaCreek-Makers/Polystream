@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\install-windows.ps1" %*
if errorlevel 1 echo Setup failed. Review the error above, then run this installer again.
pause
