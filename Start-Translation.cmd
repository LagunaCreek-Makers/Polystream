@echo off
cd /d "%~dp0"
"C:\Espressif\tools\idf-python\3.11.2\python.exe" "%~dp0scripts\start_offline.py" --device cpu --app "%~dp0target\debug\live-translate-rs.exe" %*
pause
