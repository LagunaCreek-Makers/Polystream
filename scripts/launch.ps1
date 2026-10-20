$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path $PSScriptRoot -Parent
python (Join-Path $PSScriptRoot 'start_offline.py') --app (Join-Path $projectDirectory 'target\debug\live-translate-rs.exe') @args
