#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod api;
mod vpn;
mod wireguard;

use api::{LaunchStatusResponse, Server};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, PhysicalPosition, PhysicalSize, Position, Size, State};
use vpn::{AppState, VpnState};

#[derive(Debug, Serialize)]
struct StatusResponse {
    logged_in: bool,
    vpn: VpnState,
    tunnel_active: bool,
}

#[derive(Debug, Serialize)]
struct WindowsPrereqStatus {
    platform: String,
    wireguard_installed: bool,
    webview2_installed: bool,
    winget_available: bool,
}

#[tauri::command]
async fn login(
    email: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let result = state
        .api
        .login(&email, &password, None)
        .await
        .map_err(|e| e.to_string())?;
    vpn::save_token(&result.token);
    let mut token = state.token.lock().map_err(|e| e.to_string())?;
    *token = Some(result.token.clone());
    Ok(result.token)
}

#[tauri::command]
async fn register(
    email: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let result = state
        .api
        .register(&email, &password)
        .await
        .map_err(|e| e.to_string())?;
    vpn::save_token(&result.token);
    let mut token = state.token.lock().map_err(|e| e.to_string())?;
    *token = Some(result.token.clone());
    Ok(result.token)
}

#[tauri::command]
async fn login_number(
    account_number: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let result = state
        .api
        .login_number(&account_number)
        .await
        .map_err(|e| e.to_string())?;
    vpn::save_token(&result.token);
    let mut token = state.token.lock().map_err(|e| e.to_string())?;
    *token = Some(result.token.clone());
    Ok(result.token)
}

#[tauri::command]
async fn create_anonymous_account(state: State<'_, AppState>) -> Result<String, String> {
    let account = state
        .api
        .create_anonymous_account()
        .await
        .map_err(|e| e.to_string())?;
    Ok(account.account_number)
}

#[tauri::command]
async fn scan_qr(raw_value: String, state: State<'_, AppState>) -> Result<String, String> {
    let result = state
        .api
        .scan_qr_token(&raw_value)
        .await
        .map_err(|e| e.to_string())?;
    vpn::save_token(&result.token);
    let mut token = state.token.lock().map_err(|e| e.to_string())?;
    *token = Some(result.token.clone());
    Ok(result.token)
}

#[tauri::command]
async fn get_launch_status(state: State<'_, AppState>) -> Result<LaunchStatusResponse, String> {
    let token = {
        let token = state.token.lock().map_err(|e| e.to_string())?;
        token.as_ref().ok_or("Not logged in")?.clone()
    };
    state
        .api
        .get_launch_status(&token)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_servers(state: State<'_, AppState>) -> Result<Vec<Server>, String> {
    let token = {
        let token = state.token.lock().map_err(|e| e.to_string())?;
        token.as_ref().ok_or("Not logged in")?.clone()
    };
    let servers = state
        .api
        .get_servers(&token)
        .await
        .map_err(|e| e.to_string())?;
    Ok(servers)
}

#[tauri::command]
async fn connect(
    server_id: String,
    server_name: String,
    server_location: String,
    state: State<'_, AppState>,
) -> Result<VpnState, String> {
    let token = {
        let t = state.token.lock().map_err(|e| e.to_string())?;
        t.as_ref().ok_or("Not logged in")?.clone()
    };

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "escudo-device".to_string());

    let result = state
        .api
        .connect(
            &token,
            &server_id,
            &hostname,
            &vpn::get_or_create_install_id(),
        )
        .await
        .map_err(|e| e.to_string())?;

    wireguard::install_tunnel(&result.config).map_err(|e| e.to_string())?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let vpn_state = VpnState {
        connected: true,
        server_name,
        server_location,
        server_id,
        device_id: result.device_id,
        connected_at: now,
    };

    let mut vpn = state.vpn.lock().map_err(|e| e.to_string())?;
    *vpn = vpn_state.clone();

    Ok(vpn_state)
}

#[tauri::command]
async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    let (token, device_id) = {
        let t = state.token.lock().map_err(|e| e.to_string())?;
        let v = state.vpn.lock().map_err(|e| e.to_string())?;
        (
            t.as_ref().ok_or("Not logged in")?.clone(),
            v.device_id.clone(),
        )
    };

    wireguard::remove_tunnel().map_err(|e| e.to_string())?;

    if !device_id.is_empty() {
        let _ = state.api.disconnect(&token, &device_id).await;
    }

    let mut vpn = state.vpn.lock().map_err(|e| e.to_string())?;
    *vpn = VpnState::default();

    Ok(())
}

#[tauri::command]
async fn get_status(state: State<'_, AppState>) -> Result<StatusResponse, String> {
    let logged_in = {
        let t = state.token.lock().map_err(|e| e.to_string())?;
        t.is_some()
    };
    let vpn = {
        let v = state.vpn.lock().map_err(|e| e.to_string())?;
        v.clone()
    };
    let tunnel_active = wireguard::is_tunnel_active();

    Ok(StatusResponse {
        logged_in,
        vpn,
        tunnel_active,
    })
}

#[tauri::command]
async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    vpn::clear_token();
    let mut token = state.token.lock().map_err(|e| e.to_string())?;
    *token = None;
    let mut vpn = state.vpn.lock().map_err(|e| e.to_string())?;
    *vpn = VpnState::default();
    Ok(())
}

#[tauri::command]
async fn check_windows_prereqs() -> Result<WindowsPrereqStatus, String> {
    Ok(WindowsPrereqStatus {
        platform: std::env::consts::OS.to_string(),
        wireguard_installed: is_wireguard_installed(),
        webview2_installed: is_webview2_installed(),
        winget_available: command_exists("winget"),
    })
}

#[tauri::command]
async fn install_wireguard() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if command_exists("winget") {
            let status = Command::new("winget")
                .args([
                    "install",
                    "--exact",
                    "--id",
                    "WireGuard.WireGuard",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ])
                .status()
                .map_err(|e| e.to_string())?;

            if status.success() {
                return Ok("WireGuard installed.".to_string());
            }
        }

        run_powershell(
            r#"$out = Join-Path $env:TEMP 'EscudoWireGuardInstaller.exe'; Invoke-WebRequest -Uri 'https://download.wireguard.com/windows-client/wireguard-installer.exe' -OutFile $out; Start-Process -FilePath $out -Verb RunAs -Wait"#,
        )?;
        Ok("WireGuard installer started.".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Windows installer actions are only available on Windows.".to_string())
    }
}

#[tauri::command]
async fn install_webview2() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if command_exists("winget") {
            let status = Command::new("winget")
                .args([
                    "install",
                    "--exact",
                    "--id",
                    "Microsoft.EdgeWebView2Runtime",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ])
                .status()
                .map_err(|e| e.to_string())?;

            if status.success() {
                return Ok("WebView2 installed.".to_string());
            }
        }

        run_powershell(
            r#"$out = Join-Path $env:TEMP 'EscudoWebView2Bootstrapper.exe'; Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $out; Start-Process -FilePath $out -Verb RunAs -Wait"#,
        )?;
        Ok("WebView2 installer started.".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Windows installer actions are only available on Windows.".to_string())
    }
}

fn command_exists(command: &str) -> bool {
    Command::new(command).arg("--version").output().is_ok()
}

fn is_wireguard_installed() -> bool {
    candidate_paths(&[
        ("ProgramFiles", &["WireGuard", "wireguard.exe"]),
        ("ProgramFiles(x86)", &["WireGuard", "wireguard.exe"]),
    ])
    .into_iter()
    .any(|path| path.exists())
}

fn is_webview2_installed() -> bool {
    candidate_paths(&[
        ("ProgramFiles", &["Microsoft", "EdgeWebView", "Application"]),
        (
            "ProgramFiles(x86)",
            &["Microsoft", "EdgeWebView", "Application"],
        ),
    ])
    .into_iter()
    .any(|path| {
        path.exists()
            && std::fs::read_dir(path)
                .ok()
                .map(|entries| {
                    entries
                        .flatten()
                        .any(|entry| entry.path().join("msedgewebview2.exe").exists())
                })
                .unwrap_or(false)
    })
}

fn candidate_paths(items: &[(&str, &[&str])]) -> Vec<PathBuf> {
    items
        .iter()
        .filter_map(|(env_key, parts)| {
            std::env::var(env_key).ok().map(|base| {
                let mut path = PathBuf::from(base);
                for part in *parts {
                    path.push(part);
                }
                path
            })
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn run_powershell(script: &str) -> Result<(), String> {
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Installer exited with status {}", status))
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let width = 470u32;
                let height = 860u32;
                let _ = window.set_size(Size::Physical(PhysicalSize::new(width, height)));

                if let Ok(Some(monitor)) = window.current_monitor() {
                    let monitor_size = monitor.size();
                    let x = monitor_size.width.saturating_sub(width + 24) as i32;
                    let y = monitor_size.height.saturating_sub(height + 56) as i32;
                    let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
            register,
            login_number,
            create_anonymous_account,
            scan_qr,
            get_launch_status,
            get_servers,
            connect,
            disconnect,
            get_status,
            logout,
            check_windows_prereqs,
            install_wireguard,
            install_webview2,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Escudo VPN");
}
