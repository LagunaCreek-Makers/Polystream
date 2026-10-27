@echo off
cd /d "%~dp0"
python scripts\install_modern_language.py ja --download
if errorlevel 1 echo Setup failed. Python with venv and pip is required; Python 3.12 was tested.
pause
