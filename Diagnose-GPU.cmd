@echo off
cd /d "%~dp0"
python scripts\diagnose_windows.py --config offline.cuda.json --device cuda --output "diagnostic-gpu-%RANDOM%-%RANDOM%" %*
pause
