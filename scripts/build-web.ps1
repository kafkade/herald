#!/usr/bin/env pwsh
# Build herald-web with Trunk and copy output to web-dist for serving.
param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$WebCrate = Join-Path $ProjectRoot "crates\herald-web"
$DistDir = Join-Path $ProjectRoot "web-dist"

if (-not (Test-Path (Join-Path $WebCrate "Cargo.toml"))) {
    Write-Error "herald-web crate not found at $WebCrate"
    exit 1
}

Write-Host "Building herald-web..." -ForegroundColor Cyan

Push-Location $WebCrate
try {
    if ($Release) {
        trunk build --release
    } else {
        trunk build
    }
} finally {
    Pop-Location
}

# Copy Trunk dist output to web-dist
$TrunkDist = Join-Path $WebCrate "dist"
if (-not (Test-Path $TrunkDist)) {
    Write-Error "Trunk build output not found at $TrunkDist"
    exit 1
}

if (Test-Path $DistDir) {
    Remove-Item -Recurse -Force $DistDir
}
Copy-Item -Recurse $TrunkDist $DistDir

Write-Host "Web assets copied to $DistDir" -ForegroundColor Green
Get-ChildItem $DistDir | Format-Table Name, Length
