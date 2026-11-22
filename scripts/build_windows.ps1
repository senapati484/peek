<#
.SYNOPSIS
    Builds Peek on Windows and creates a release ZIP archive.
#>

param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
Set-Location $RootDir

$Version = "0.1.0"
$DistDir = Join-Path $RootDir "dist"
if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

if (-not $SkipBuild) {
    Write-Host "==> 1. Building Svelte UI bundle..."
    Push-Location (Join-Path $RootDir "ui")
    npm run build
    Pop-Location

    Write-Host "==> 2. Compiling Rust release binary..."
    cargo build --release -p peek-app
}

$BinSrc = Join-Path $RootDir "target\release\Peek.exe"
if (-not (Test-Path $BinSrc)) {
    Write-Error "Binary not found at $BinSrc"
    exit 1
}

Write-Host "==> 3. Creating Windows release archive..."
$StageDir = Join-Path $RootDir "target\Peek-windows-x86_64"
if (Test-Path $StageDir) {
    Remove-Item -Recurse -Force $StageDir
}
New-Item -ItemType Directory -Path $StageDir | Out-Null

Copy-Item $BinSrc -Destination (Join-Path $StageDir "Peek.exe")
if (Test-Path (Join-Path $RootDir "README.md")) {
    Copy-Item (Join-Path $RootDir "README.md") -Destination (Join-Path $StageDir "README.md")
}
if (Test-Path (Join-Path $RootDir "LICENSE")) {
    Copy-Item (Join-Path $RootDir "LICENSE") -Destination (Join-Path $StageDir "LICENSE")
}
if (Test-Path (Join-Path $RootDir "assets\app_icon.ico")) {
    Copy-Item (Join-Path $RootDir "assets\app_icon.ico") -Destination (Join-Path $StageDir "app_icon.ico")
}

$ZipFile = Join-Path $DistDir "Peek-windows-x86_64.zip"
if (Test-Path $ZipFile) {
    Remove-Item -Force $ZipFile
}
Compress-Archive -Path "$StageDir\*" -DestinationPath $ZipFile
Write-Host "==> Created: $ZipFile"
Write-Host "==> Windows build and packaging complete!"
