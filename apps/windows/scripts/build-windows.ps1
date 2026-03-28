$ErrorActionPreference = "Stop"

function Assert-Command($name) {
    if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $name"
    }
}

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

Write-Host "Escudo Windows build starting..." -ForegroundColor Cyan

$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

Assert-Command node
Assert-Command npm
Assert-Command cargo

if (-not (Test-Path ".\\node_modules")) {
    Write-Host "Installing npm dependencies..." -ForegroundColor Yellow
    npm install
}

Write-Host "Building Tauri Windows app..." -ForegroundColor Yellow
npm run tauri build

$bundleDir = Join-Path $repoRoot "src-tauri\\target\\release\\bundle"
if (Test-Path $bundleDir) {
    Write-Host "Build complete. Bundles are under $bundleDir" -ForegroundColor Green
} else {
    throw "Build finished but bundle directory was not found: $bundleDir"
}
