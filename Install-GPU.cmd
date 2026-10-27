@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\install-windows.ps1" -Device cuda %*
if errorlevel 1 echo GPU setup failed. Review the error above. The CPU installation is still available.
pause
