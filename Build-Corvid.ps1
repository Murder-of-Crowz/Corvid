<#
Build the tested Corvid Windows application and NSIS installer.
The installer changes currently live in Documents\Corvid-installer-test.
This wrapper refreshes only maps, map IDs, and the icon from this repository.
Other source edits must be made in BuildRoot until the two checkouts are merged.
Usage: .\Build-Corvid.ps1
       .\Build-Corvid.ps1 -InstallDependencies -CreateZip
#>
[CmdletBinding()]
param(
    [string]$BuildRoot = (Join-Path $env:USERPROFILE 'Documents\Corvid-installer-test'),
    [switch]$InstallDependencies,
    [switch]$CreateZip
)
$ErrorActionPreference = 'Stop'
$originalPath = $env:Path
function Invoke-Checked([scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "Build command failed (exit $LASTEXITCODE)." }
}
$BuildRoot = (Resolve-Path -LiteralPath $BuildRoot).Path
if (-not (Test-Path -LiteralPath (Join-Path $BuildRoot 'desktop\src-tauri\src\windows_icon.rs'))) {
    throw "BuildRoot does not contain the tested installer changes: $BuildRoot"
}
$package = Get-Content -Raw -LiteralPath (Join-Path $BuildRoot 'desktop\package.json') | ConvertFrom-Json
$pinnedNode = Join-Path $env:LOCALAPPDATA "Author Software\nvm\installs\v$($package.engines.node)"
if (Test-Path -LiteralPath (Join-Path $pinnedNode 'node.exe')) { $env:Path = "$pinnedNode;$env:Path" }
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
Push-Location $BuildRoot
try {
    Write-Host "Building tested installer source: $BuildRoot"
    if ($PSScriptRoot -ne $BuildRoot) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'desktop\src\data\zones.ts') -Destination 'desktop\src\data\zones.ts'
        $mapSource = Join-Path $PSScriptRoot 'WTF\Zones'
        if (Test-Path -LiteralPath $mapSource) {
            New-Item -ItemType Directory -Path 'WTF\Zones' -Force | Out-Null
            Get-ChildItem -LiteralPath $mapSource -Filter 'Zone_*.png' -File | Copy-Item -Destination 'WTF\Zones'
        }
        $sourceIcon = Join-Path $PSScriptRoot 'corvid_icon.png'
        if (Test-Path -LiteralPath $sourceIcon) { Copy-Item -LiteralPath $sourceIcon -Destination 'corvid_icon.png' }
    }
    if ($InstallDependencies -or -not (Test-Path 'desktop\node_modules\.bin\tauri.cmd')) {
        Invoke-Checked { npm.cmd --prefix desktop ci }
    }
    Invoke-Checked { npm.cmd run check-version }
    if (Test-Path 'corvid_icon.png') {
        Invoke-Checked { npm.cmd --prefix desktop run tauri -- icon ../corvid_icon.png }
    }
    Push-Location desktop
    try {
        Invoke-Checked { npm.cmd exec -- vitest run }
        Invoke-Checked { npm.cmd run build }
        Invoke-Checked { cargo test --locked --manifest-path src-tauri/Cargo.toml }
        Invoke-Checked { npm.cmd run tauri -- build --bundles nsis }
    } finally { Pop-Location }
    $config = Get-Content -Raw 'desktop\src-tauri\tauri.conf.json' | ConvertFrom-Json
    $setup = Join-Path $BuildRoot "desktop\src-tauri\target\release\bundle\nsis\Corvid_$($config.version)_x64-setup.exe"
    if (-not (Test-Path -LiteralPath $setup)) { throw "Expected installer was not produced: $setup" }
    Write-Host "Application: $BuildRoot\desktop\src-tauri\target\release\Corvid.exe"
    Write-Host "Installer:   $setup"
    if ($CreateZip) {
        $zip = Join-Path $BuildRoot "Corvid_$($config.version)_Windows_Setup_$(Get-Date -Format 'yyyyMMdd-HHmmss').zip"
        Compress-Archive -LiteralPath $setup -DestinationPath $zip -CompressionLevel Optimal
        Write-Host "ZIP:         $zip"
    }
} finally {
    Pop-Location
    $env:Path = $originalPath
}
