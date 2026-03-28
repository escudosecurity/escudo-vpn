use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const TUNNEL_NAME: &str = "escudo-vpn";

fn get_config_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("{}.conf", TUNNEL_NAME));
    path
}

fn find_wireguard_exe() -> PathBuf {
    let program_files =
        std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
    let path = PathBuf::from(&program_files)
        .join("WireGuard")
        .join("wireguard.exe");
    if path.exists() {
        return path;
    }

    let program_files_x86 = std::env::var("ProgramFiles(x86)")
        .unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());
    let path = PathBuf::from(&program_files_x86)
        .join("WireGuard")
        .join("wireguard.exe");
    if path.exists() {
        return path;
    }

    PathBuf::from("wireguard.exe")
}

pub fn install_tunnel(config_content: &str) -> Result<()> {
    let config_path = get_config_path();
    fs::write(&config_path, config_content)
        .map_err(|e| anyhow!("Failed to write WireGuard config: {}", e))?;

    let wg_exe = find_wireguard_exe();

    let output = Command::new(&wg_exe)
        .arg("/installtunnelservice")
        .arg(&config_path)
        .output()
        .map_err(|e| {
            anyhow!(
                "Failed to run wireguard.exe at {:?}: {}. Make sure WireGuard is installed.",
                wg_exe,
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "wireguard.exe /installtunnelservice failed:\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        ));
    }

    Ok(())
}

pub fn remove_tunnel() -> Result<()> {
    let wg_exe = find_wireguard_exe();

    let output = Command::new(&wg_exe)
        .arg("/uninstalltunnelservice")
        .arg(TUNNEL_NAME)
        .output()
        .map_err(|e| anyhow!("Failed to run wireguard.exe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "wireguard.exe /uninstalltunnelservice failed:\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        ));
    }

    let config_path = get_config_path();
    let _ = fs::remove_file(&config_path);

    Ok(())
}

pub fn is_tunnel_active() -> bool {
    let output = Command::new("sc")
        .args(["query", &format!("WireGuardTunnel${}", TUNNEL_NAME)])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains("RUNNING")
        }
        Err(_) => false,
    }
}
