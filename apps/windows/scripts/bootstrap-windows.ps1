$ErrorActionPreference = "Stop"

function Assert-Command($name) {
    if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $name"
    }
}

Write-Host "Escudo Windows bootstrap starting..." -ForegroundColor Cyan

Assert-Command winget

Write-Host "Installing Node.js LTS..." -ForegroundColor Yellow
winget install --id OpenJS.NodeJS.LTS --exact --accept-package-agreements --accept-source-agreements --silent

Write-Host "Installing Rustup/MSVC toolchain..." -ForegroundColor Yellow
winget install --id Rustlang.Rustup --exact --accept-package-agreements --accept-source-agreements --silent

Write-Host "Installing Microsoft C++ Build Tools..." -ForegroundColor Yellow
winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --accept-package-agreements --accept-source-agreements --override "--quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

Write-Host "Installing WebView2 Runtime..." -ForegroundColor Yellow
winget install --id Microsoft.EdgeWebView2Runtime --exact --accept-package-agreements --accept-source-agreements --silent

Write-Host "Refreshing PATH for this session..." -ForegroundColor Yellow
$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

Assert-Command node
Assert-Command npm
Assert-Command rustup
Assert-Command cargo

Write-Host "Selecting stable MSVC Rust toolchain..." -ForegroundColor Yellow
rustup default stable-msvc

Write-Host "Installing npm dependencies..." -ForegroundColor Yellow
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot
npm install

Write-Host "Bootstrap complete." -ForegroundColor Green
Write-Host "Next step: run .\\scripts\\build-windows.ps1" -ForegroundColor Green
