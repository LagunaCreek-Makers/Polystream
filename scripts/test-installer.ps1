$ErrorActionPreference = 'Stop'
$ProjectRoot = Join-Path $PSScriptRoot 'test-fixture'
$work = Join-Path $ProjectRoot 'work'
New-Item -ItemType Directory -Force $work | Out-Null
try {
    $ast = [System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'install-windows.ps1'), [ref]$null, [ref]$null)
    $fn = $ast.FindAll({param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Install-Engine'}, $true)[0]
    Invoke-Expression $fn.Extent.Text
    $headers = @{}
    $package = Join-Path $work 'package'
    New-Item -ItemType Directory -Force (Join-Path $package 'nested\data') | Out-Null
    Set-Content (Join-Path $package 'nested\test.exe') 'fixture'
    Set-Content (Join-Path $package 'nested\dependency.dll') 'fixture'
    Set-Content (Join-Path $package 'nested\data\voice.txt') 'fixture'
    $archive = Join-Path $work 'fixture.zip'
    Compress-Archive -Path (Join-Path $package '*') -DestinationPath $archive
    $script:asset = [pscustomobject]@{name='test-bin.zip'; browser_download_url='fixture'; size=(Get-Item $archive).Length; digest=('sha256:' + (Get-FileHash $archive).Hash.ToLowerInvariant())}
    # Invoke-RestMethod returns a JSON array as a single pipeline object in PS 5.1.
    # Include multiple matching releases so accidental nested arrays cannot pass.
    function Invoke-RestMethod {
        param($Headers,$Uri)
        return ,@(
            [pscustomobject]@{draft=$false; tag_name='stable'; assets=@()},
            [pscustomobject]@{draft=$false; tag_name='test'; assets=@($script:asset)},
            [pscustomobject]@{draft=$false; tag_name='older'; assets=@($script:asset)}
        )
    }
    function Download($Url,$Path) { Copy-Item -LiteralPath $archive -Destination $Path }
    $installed = Install-Engine 'test' 'owner/repo' '' '^test-bin.zip$' 'test.exe'
    if (-not (Test-Path $installed)) { throw 'Executable was not installed' }
    $record = Get-Content (Join-Path $ProjectRoot 'engines\test\installation.json') -Raw | ConvertFrom-Json
    if ($record.release -ne 'test') { throw 'Wrong release selected' }
    if (-not (Test-Path (Join-Path (Split-Path $installed) 'dependency.dll'))) { throw 'DLL lost' }
    if (-not (Test-Path (Join-Path (Split-Path $installed) 'data\voice.txt'))) { throw 'Data lost' }
    if ((Install-Engine 'test' 'owner/repo' '' '^test-bin.zip$' 'test.exe') -ne $installed) { throw 'Reuse failed' }
    $script:asset.digest = 'sha256:invalid'
    $rejected = $false
    try { Install-Engine 'bad' 'owner/repo' '' '^test-bin.zip$' 'test.exe' } catch { $rejected = $_ -match 'checksum mismatch' }
    if (-not $rejected -or (Test-Path (Join-Path $ProjectRoot 'engines\bad'))) { throw 'Corrupt archive was not rejected' }
    Write-Host 'PASS: nested engine installation, DLL/data preservation, reuse, checksum rejection.'
} finally {
    $resolved = [IO.Path]::GetFullPath($ProjectRoot)
    if ($resolved -eq (Join-Path $PSScriptRoot 'test-fixture')) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
