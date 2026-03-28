pub mod config;
pub mod killswitch;
pub mod tunnel;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

use crate::config::WgConfig;
use crate::tunnel::{TunnelConfig, VpnTunnel};

static TUNNEL: Mutex<Option<VpnTunnel>> = Mutex::new(None);
static DAITA_ENABLED: Mutex<bool> = Mutex::new(false);

/// Connect to VPN using a WireGuard config string.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn escudo_connect(config_str: *const c_char) -> i32 {
    let c_str = unsafe {
        if config_str.is_null() {
            return -1;
        }
        CStr::from_ptr(config_str)
    };

    let config_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let config = match WgConfig::parse(config_str) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let private_key: [u8; 32] = match config.private_key.try_into() {
        Ok(k) => k,
        Err(_) => return -1,
    };

    let peer_key: [u8; 32] = match config.peer_public_key.try_into() {
        Ok(k) => k,
        Err(_) => return -1,
    };

    let psk: Option<[u8; 32]> = config.preshared_key.try_into().ok();

    let daita = DAITA_ENABLED.lock().map(|g| *g).unwrap_or(false);
    let tc = TunnelConfig {
        daita_enabled: daita,
        ..TunnelConfig::default()
    };

    let mut tunnel = VpnTunnel::with_config(&private_key, &peer_key, psk, tc);
    tunnel.set_server_ip(config.endpoint.clone());

    if let Ok(mut guard) = TUNNEL.lock() {
        *guard = Some(tunnel);
        0
    } else {
        -1
    }
}

/// Connect to VPN in multihop mode using two WireGuard config strings.
/// The entry config is the first hop; the exit config is the second hop.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn escudo_connect_multihop(
    config_entry: *const c_char,
    config_exit: *const c_char,
) -> i32 {
    let parse_config = |ptr: *const c_char| -> Option<WgConfig> {
        let c_str = unsafe {
            if ptr.is_null() {
                return None;
            }
            CStr::from_ptr(ptr)
        };
        c_str.to_str().ok().and_then(|s| WgConfig::parse(s).ok())
    };

    let entry = match parse_config(config_entry) {
        Some(c) => c,
        None => return -1,
    };
    let exit = match parse_config(config_exit) {
        Some(c) => c,
        None => return -1,
    };

    let entry_pk: [u8; 32] = match entry.private_key.try_into() {
        Ok(k) => k,
        Err(_) => return -1,
    };
    let entry_peer: [u8; 32] = match entry.peer_public_key.try_into() {
        Ok(k) => k,
        Err(_) => return -1,
    };
    let entry_psk: Option<[u8; 32]> = entry.preshared_key.try_into().ok();

    let exit_pk: [u8; 32] = match exit.private_key.try_into() {
        Ok(k) => k,
        Err(_) => return -1,
    };
    let exit_peer: [u8; 32] = match exit.peer_public_key.try_into() {
        Ok(k) => k,
        Err(_) => return -1,
    };
    let exit_psk: Option<[u8; 32]> = exit.preshared_key.try_into().ok();

    let daita = DAITA_ENABLED.lock().map(|g| *g).unwrap_or(false);
    let tc = TunnelConfig {
        daita_enabled: daita,
        multihop_enabled: true,
        ..TunnelConfig::default()
    };

    let mut tunnel = VpnTunnel::new_multihop(
        &entry_pk,
        &entry_peer,
        entry_psk,
        &exit_pk,
        &exit_peer,
        exit_psk,
        tc,
    );
    tunnel.set_server_ip(exit.endpoint.clone());

    if let Ok(mut guard) = TUNNEL.lock() {
        *guard = Some(tunnel);
        0
    } else {
        -1
    }
}

/// Disconnect from VPN.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn escudo_disconnect() -> i32 {
    if let Ok(mut guard) = TUNNEL.lock() {
        *guard = None;
        0
    } else {
        -1
    }
}

/// Get connection status.
/// Returns 1 if connected, 0 if disconnected, -1 on error.
#[no_mangle]
pub extern "C" fn escudo_get_status() -> i32 {
    if let Ok(guard) = TUNNEL.lock() {
        if guard.is_some() {
            1
        } else {
            0
        }
    } else {
        -1
    }
}

/// Enable or disable DAITA (Defence Against AI-guided Traffic Analysis).
/// Takes effect on the next connection and also updates the active tunnel if one exists.
/// Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn escudo_set_daita_enabled(enabled: bool) -> i32 {
    if let Ok(mut guard) = DAITA_ENABLED.lock() {
        *guard = enabled;
    } else {
        return -1;
    }

    if let Ok(mut guard) = TUNNEL.lock() {
        if let Some(ref mut tunnel) = *guard {
            tunnel.set_daita_enabled(enabled);
        }
    }

    0
}

/// Get connection info as a JSON string.
/// Returns a pointer to a null-terminated UTF-8 JSON string, or null on error.
/// The caller must free the returned string with `escudo_free_string`.
///
/// JSON format:
/// {
///   "connected": bool,
///   "server_ip": string|null,
///   "protocol": string,
///   "daita_enabled": bool,
///   "multihop_enabled": bool,
///   "uptime_secs": u64,
///   "bytes_rx": u64,
///   "bytes_tx": u64
/// }
#[no_mangle]
pub extern "C" fn escudo_get_connection_info() -> *const c_char {
    let json = if let Ok(guard) = TUNNEL.lock() {
        if let Some(ref tunnel) = *guard {
            let info = tunnel.connection_info();
            format!(
                r#"{{"connected":true,"server_ip":{},"protocol":"{}","daita_enabled":{},"multihop_enabled":{},"uptime_secs":{},"bytes_rx":{},"bytes_tx":{}}}"#,
                match &info.server_ip {
                    Some(ip) => format!("\"{}\"", ip),
                    None => "null".to_string(),
                },
                info.protocol,
                info.daita_enabled,
                info.multihop_enabled,
                info.uptime_secs,
                info.bytes_rx,
                info.bytes_tx,
            )
        } else {
            r#"{"connected":false,"server_ip":null,"protocol":"WireGuard","daita_enabled":false,"multihop_enabled":false,"uptime_secs":0,"bytes_rx":0,"bytes_tx":0}"#.to_string()
        }
    } else {
        return std::ptr::null();
    };

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null(),
    }
}

/// Free a string previously returned by `escudo_get_connection_info`.
#[no_mangle]
pub unsafe extern "C" fn escudo_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}
