@echo off
cd /d "%~dp0"
python scripts\diagnose_windows.py --device cpu --output "diagnostic-cpu-%RANDOM%-%RANDOM%" %*
pause
