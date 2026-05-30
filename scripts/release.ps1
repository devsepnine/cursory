<#
.SYNOPSIS
    Build Cursory release MSI for distribution.

.DESCRIPTION
    Reads version from Cargo.toml, compiles in release mode, packages an MSI
    via cargo-wix, and copies the result into ./dist as Cursory-<version>.msi.

.PARAMETER SkipBuild
    Skip cargo build (assume target/release/cursory.exe already exists).

.EXAMPLE
    pwsh scripts/release.ps1
#>

[CmdletBinding()]
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Push-Location (Join-Path $PSScriptRoot "..")

try {
    # --- read Cargo.toml ---
    $manifest = Get-Content Cargo.toml -Raw
    # `[^\[]*` keeps the match inside the [package] section: it stops at the next
    # section header '[', so a `version =` under [dependencies] can't be captured.
    if ($manifest -notmatch '(?ms)^\[package\][^\[]*?^name\s*=\s*"([^"]+)"') {
        throw "could not parse 'name' from Cargo.toml"
    }
    $name = $matches[1]
    if ($manifest -notmatch '(?ms)^\[package\][^\[]*?^version\s*=\s*"([^"]+)"') {
        throw "could not parse 'version' from Cargo.toml"
    }
    $version = $matches[1]
    Write-Host "==> Building $name v$version" -ForegroundColor Cyan

    # --- ensure cargo-wix is installed ---
    $hasCargoWix = $false
    try {
        & cargo wix --version 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) { $hasCargoWix = $true }
    } catch {}
    if (-not $hasCargoWix) {
        Write-Host "==> Installing cargo-wix" -ForegroundColor Yellow
        cargo install cargo-wix
        if ($LASTEXITCODE -ne 0) { throw "cargo install cargo-wix failed" }
    }

    # --- ensure WiX Toolset v3 binaries are reachable ---
    $candle = Get-Command candle.exe -ErrorAction SilentlyContinue
    if (-not $candle) {
        $wixCandidates = @(
            "C:\Program Files (x86)\WiX Toolset v3.14\bin",
            "C:\Program Files (x86)\WiX Toolset v3.11\bin",
            "C:\Program Files (x86)\WiX Toolset v3.10\bin"
        )
        foreach ($p in $wixCandidates) {
            if (Test-Path (Join-Path $p "candle.exe")) {
                $env:PATH = "$p;$env:PATH"
                $candle = Get-Command candle.exe -ErrorAction SilentlyContinue
                break
            }
        }
    }
    if (-not $candle) {
        throw "WiX Toolset v3 not found. Install from https://github.com/wixtoolset/wix3/releases (v3.11+ recommended), then re-run."
    }

    # --- ensure wix/main.wxs exists ---
    if (-not (Test-Path "wix/main.wxs")) {
        Write-Host "==> Initializing wix template" -ForegroundColor Yellow
        cargo wix init
        if ($LASTEXITCODE -ne 0) { throw "cargo wix init failed" }
        Write-Host "    wix/main.wxs created — commit it to keep UpgradeCode stable across releases." -ForegroundColor Yellow
    }

    # --- compile release ---
    if (-not $SkipBuild) {
        Write-Host "==> cargo build --release" -ForegroundColor Cyan
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    }

    # --- package MSI ---
    Write-Host "==> cargo wix (packaging MSI)" -ForegroundColor Cyan
    cargo wix --no-build --nocapture
    if ($LASTEXITCODE -ne 0) { throw "cargo wix failed" }

    # --- collect outputs ---
    if (-not (Test-Path dist)) {
        New-Item -ItemType Directory dist | Out-Null
    }
    $msi = Get-ChildItem target/wix/*.msi -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $msi) { throw "MSI not found under target/wix/" }
    $destMsi = "dist/$name-$version.msi"
    Copy-Item $msi.FullName $destMsi -Force

    # also drop the bare exe for portable use
    $exe = "target/release/$name.exe"
    if (Test-Path $exe) {
        $destExe = "dist/$name-$version.exe"
        Copy-Item $exe $destExe -Force
    }

    Write-Host ""
    Write-Host "==> Done" -ForegroundColor Green
    Write-Host "    MSI : $destMsi"
    if (Test-Path "dist/$name-$version.exe") {
        Write-Host "    EXE : dist/$name-$version.exe"
    }
}
finally {
    Pop-Location
}
