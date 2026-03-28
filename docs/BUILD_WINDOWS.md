# Escudo Windows Build

This repo contains the real Windows companion app under [`/opt/escudo/apps/windows`](/opt/escudo/apps/windows).

## Goal

End users should only download an installer and open the app.

They should not:
- install Node
- install Rust
- run `npm`
- understand WireGuard

Those steps belong to the build environment only.

## Build Environment

Use either:
- a Windows GitHub Actions runner
- a real Windows build machine

Recommended:
- Windows 11 or Windows Server runner
- Rust MSVC toolchain
- Node.js LTS
- Microsoft C++ Build Tools

Official Tauri references used:
- GitHub Actions with `tauri-action` and `projectPath`: https://v2.tauri.app/ja/distribute/pipelines/github/
- Windows installer guidance: https://v2.tauri.app/distribute/windows-installer/
- Windows prerequisites: https://v2.tauri.app/start/prerequisites/

## Local Windows Build

From PowerShell on a Windows machine:

```powershell
cd apps/windows
.\scripts\bootstrap-windows.ps1
.\scripts\build-windows.ps1
```

Expected output location:

```text
apps/windows/src-tauri/target/release/bundle/
```

Typical bundles:
- `.msi`
- `-setup.exe`

## GitHub Actions Build

Workflow file:
- [`/opt/escudo/.github/workflows/windows-tauri-build.yml`](/opt/escudo/.github/workflows/windows-tauri-build.yml)

Trigger:
- `workflow_dispatch`
- pushes touching the Windows app or workflow

Artifact:
- `escudo-windows-bundle`

## Runtime/Test Notes

The Windows app itself now supports:
- QR pairing scan
- pairing-link paste fallback
- `16-digit` code login
- email login/register
- residential and standard server listing
- connect/disconnect flow through WireGuard
- first-run checks for:
  - WireGuard installed
  - WebView2 installed

For real end-user release quality:
- produce installer on Windows
- test Android -> Windows QR pairing
- test WireGuard installation/detection
- test connect/disconnect on a real Windows machine
- add code signing later
