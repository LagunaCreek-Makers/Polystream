#requires -Version 5.1
[CmdletBinding()]
param(
    [string]$ProjectRoot = (Split-Path $PSScriptRoot -Parent),
    [switch]$Plan,
    [switch]$SkipRuntime,
    [string]$WhisperTag = '',
    [string]$LlamaTag = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$ProjectRoot = (Resolve-Path -LiteralPath $ProjectRoot).Path
if (-not (Test-Path (Join-Path $ProjectRoot 'models-manifest.json'))) { throw 'Select the live-translate-rs project directory.' }
if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { throw 'Run 64-bit PowerShell on Windows x64.' }
if ($Plan) {
    Write-Host 'Install Python 3.12 if needed, Microsoft VC++ x64 runtime, whisper.cpp, llama.cpp CPU, native Piper, and the pinned Spanish/German model pack (~2.79 GB).'
    Write-Host "Destination: $ProjectRoot. Creates Start-Translation.cmd. Existing engine folders are reused; models are hash checked. No app build or GPU drivers are installed."
    return
}
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$headers = @{ 'User-Agent' = 'live-translate-rs-installer'; Accept = 'application/vnd.github+json' }
$work = Join-Path ([IO.Path]::GetTempPath()) ('live-translate-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
function Download($Url, $Path) {
    Write-Host "Downloading $Url"
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        try { Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $Path; return }
        catch { if ($attempt -eq 3) { throw }; Start-Sleep -Seconds 2 }
    }
}
function Find-Python {
    $candidates = @('python', (Join-Path $env:LOCALAPPDATA 'Programs\Python\Python312\python.exe'), (Join-Path $env:ProgramFiles 'Python312\python.exe'))
    foreach ($candidate in $candidates) {
        $command = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($command) {
            & $command.Source -c 'import sys; sys.exit(0 if sys.version_info >= (3,9) else 1)' 2>$null
            if ($LASTEXITCODE -eq 0) { return $command.Source }
        }
    }
    return $null
}
function Install-Engine($Name, $Repo, $Tag, $Pattern, $Exe) {
    $destination = Join-Path $ProjectRoot "engines\$Name"
    if (Test-Path -LiteralPath $destination) {
        $existing = @(Get-ChildItem -LiteralPath $destination -Filter $Exe -Recurse -File)
        if ($existing.Count -eq 1) { return $existing[0].FullName }
        throw "Existing $destination has no unique $Exe. Move the folder aside and retry."
    }
    if ($Tag) { $releases = @(Invoke-RestMethod -Headers $headers -Uri "https://api.github.com/repos/$Repo/releases/tags/$Tag") }
    else { $releases = @(Invoke-RestMethod -Headers $headers -Uri "https://api.github.com/repos/$Repo/releases?per_page=50") }
    $asset = $null
    foreach ($release in $releases) {
        if ($release.draft) { continue }
        $matches = @($release.assets | Where-Object { $_.name -match $Pattern })
        if ($matches.Count -eq 1) { $asset = $matches[0]; break }
    }
    if (-not $asset) { throw "No matching Windows CPU archive for $Repo. Specify a release tag or review upstream packaging." }
    $zip = Join-Path $work "$Name.zip"
    Download $asset.browser_download_url $zip
    if ((Get-Item $zip).Length -ne $asset.size) { throw "$Name archive size mismatch" }
    $hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($asset.PSObject.Properties['digest'] -and $asset.digest -and $asset.digest -ne "sha256:$hash") { throw "$Name archive checksum mismatch" }
    $stage = Join-Path $work $Name
    Expand-Archive -LiteralPath $zip -DestinationPath $stage
    $executables = @(Get-ChildItem -LiteralPath $stage -Filter $Exe -Recurse -File)
    if ($executables.Count -ne 1) { throw "$Name archive must contain exactly one $Exe" }
    # Keep the entire package, including sibling DLLs, licenses and phonemizer data.
    @{ repository=$Repo; release=$release.tag_name; url=$asset.browser_download_url; sha256=$hash } | ConvertTo-Json | Set-Content (Join-Path $stage 'installation.json') -Encoding UTF8
    $relative = $executables[0].FullName.Substring($stage.Length + 1)
    New-Item -ItemType Directory -Force (Split-Path $destination -Parent) | Out-Null
    Move-Item -LiteralPath $stage -Destination $destination
    return (Join-Path $destination $relative)
}
try {
    $python = Find-Python
    if (-not $python) {
        if (-not (Get-Command winget -ErrorAction SilentlyContinue)) { throw 'Install Windows App Installer (winget) or Python 3.9+ and retry.' }
        & winget install --id Python.Python.3.12 --exact --source winget --scope user --silent --accept-package-agreements --accept-source-agreements
        if ($LASTEXITCODE -ne 0) { throw "Python installation failed: $LASTEXITCODE" }
        $python = Find-Python
        if (-not $python) { throw 'Python was installed but could not be located. Reopen PowerShell and retry.' }
    }
    if (-not $SkipRuntime) {
        $runtime = Join-Path $work 'vc_redist.x64.exe'
        Download 'https://aka.ms/vc14/vc_redist.x64.exe' $runtime
        $signature = Get-AuthenticodeSignature -LiteralPath $runtime
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') { throw 'Microsoft runtime signature validation failed.' }
        $process = Start-Process -FilePath $runtime -ArgumentList '/install','/quiet','/norestart' -WindowStyle Hidden -Wait -PassThru
        if ($process.ExitCode -notin @(0,1638,3010)) { throw "VC++ installation failed: $($process.ExitCode)" }
        if ($process.ExitCode -eq 3010) { Write-Warning 'Windows requires a restart to finish runtime installation.' }
    }
    $whisper = Install-Engine 'whisper' 'ggml-org/whisper.cpp' $WhisperTag '^whisper-(bin-x64|.*bin-win-cpu-x64)\.zip$' 'whisper-server.exe'
    $llama = Install-Engine 'llama' 'ggml-org/llama.cpp' $LlamaTag '^llama-.*-bin-win-cpu-x64\.zip$' 'llama-server.exe'
    $piper = Install-Engine 'piper' 'rhasspy/piper' '2023.11.14-2' '^piper_windows_amd64\.zip$' 'piper.exe'
    foreach ($engine in @($whisper,$llama,$piper)) {
        & $engine --help | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Engine startup check failed: $engine (exit $LASTEXITCODE)" }
    }
    & $python (Join-Path $ProjectRoot 'scripts\fetch_models.py') --download
    if ($LASTEXITCODE -ne 0) { throw 'Model download or verification failed. Rerun setup to retry.' }
    $configPath = Join-Path $ProjectRoot 'offline.json'
    $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
    $config.whisper_server = $whisper.Substring($ProjectRoot.Length + 1).Replace('\','/')
    $config.llama_server = $llama.Substring($ProjectRoot.Length + 1).Replace('\','/')
    $config.piper_binary = $piper.Substring($ProjectRoot.Length + 1).Replace('\','/')
    $backup = "$configPath.before-installer"
    if (-not (Test-Path -LiteralPath $backup)) { Copy-Item -LiteralPath $configPath -Destination $backup }
    # Python's config reader expects UTF-8 without a BOM (PowerShell 5.1 adds one).
    [IO.File]::WriteAllText($configPath, ($config | ConvertTo-Json -Depth 10), (New-Object Text.UTF8Encoding $false))
    & $python (Join-Path $ProjectRoot 'scripts\check_offline.py')
    if ($LASTEXITCODE -ne 0) { throw 'Installation configuration check failed.' }
    $app = @('target\release\live-translate-rs.exe','target\debug\live-translate-rs.exe','live-translate-rs.exe') | Where-Object { Test-Path (Join-Path $ProjectRoot $_) } | Select-Object -First 1
    if (-not $app) { throw 'Dependencies installed, but no compiled application found. Build with cargo build --locked --release and rerun.' }
    $launcher = "@echo off`r`ncd /d `"%~dp0`"`r`n`"$python`" `"%~dp0scripts\start_offline.py`" --device cpu --app `"%~dp0$app`" %*`r`npause`r`n"
    [IO.File]::WriteAllText((Join-Path $ProjectRoot 'Start-Translation.cmd'), $launcher)
    Write-Host 'Setup complete. Open Start-Translation.cmd. Models and engines run locally; inference quality has not been tested by setup.'
} finally {
    # Delete only this invocation's randomly named temporary directory.
    if ($work.StartsWith([IO.Path]::GetTempPath()) -and (Split-Path $work -Leaf) -like 'live-translate-*') { Remove-Item -LiteralPath $work -Recurse -Force }
}
