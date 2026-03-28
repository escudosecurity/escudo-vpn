use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VpnState {
    pub connected: bool,
    pub server_name: String,
    pub server_location: String,
    pub server_id: String,
    pub device_id: String,
    pub connected_at: u64,
}

impl Default for VpnState {
    fn default() -> Self {
        Self {
            connected: false,
            server_name: String::new(),
            server_location: String::new(),
            server_id: String::new(),
            device_id: String::new(),
            connected_at: 0,
        }
    }
}

pub struct AppState {
    pub token: Mutex<Option<String>>,
    pub vpn: Mutex<VpnState>,
    pub api: crate::api::ApiClient,
}

impl AppState {
    pub fn new() -> Self {
        let token = load_token();
        Self {
            token: Mutex::new(token),
            vpn: Mutex::new(VpnState::default()),
            api: crate::api::ApiClient::new(),
        }
    }
}

fn get_data_dir() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("EscudoVPN");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn get_token_path() -> PathBuf {
    get_data_dir().join("session.dat")
}

fn get_install_id_path() -> PathBuf {
    get_data_dir().join("install_id.dat")
}

pub fn save_token(token: &str) {
    let path = get_token_path();
    let encoded = simple_encode(token);
    let _ = fs::write(path, encoded);
}

pub fn load_token() -> Option<String> {
    let path = get_token_path();
    let data = fs::read_to_string(path).ok()?;
    simple_decode(&data)
}

pub fn clear_token() {
    let path = get_token_path();
    let _ = fs::remove_file(path);
}

pub fn get_or_create_install_id() -> String {
    let path = get_install_id_path();
    if let Ok(value) = fs::read_to_string(&path) {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "windows".to_string())
        .replace(' ', "-")
        .to_lowercase();
    let stamp = format!(
        "win-{}-{}",
        hostname,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    let _ = fs::write(&path, &stamp);
    stamp
}

fn simple_encode(input: &str) -> String {
    let key: u8 = 0x5A;
    let encoded: Vec<u8> = input.as_bytes().iter().map(|b| b ^ key).collect();
    base64_encode(&encoded)
}

fn simple_decode(input: &str) -> Option<String> {
    let key: u8 = 0x5A;
    let decoded_bytes = base64_decode(input)?;
    let original: Vec<u8> = decoded_bytes.iter().map(|b| b ^ key).collect();
    String::from_utf8(original).ok()
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let chunks = data.chunks(3);
    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let mut result = Vec::new();
    let chars: Vec<u8> = input.bytes().collect();
    let chunks = chars.chunks(4);
    for chunk in chunks {
        if chunk.len() < 4 {
            return None;
        }
        let vals: Vec<Option<u8>> = chunk
            .iter()
            .map(|&c| {
                if c == b'=' {
                    Some(0)
                } else {
                    CHARS.iter().position(|&x| x == c).map(|p| p as u8)
                }
            })
            .collect();
        if vals.iter().any(|v| v.is_none()) {
            return None;
        }
        let v: Vec<u8> = vals.into_iter().map(|v| v.unwrap()).collect();
        let triple =
            ((v[0] as u32) << 18) | ((v[1] as u32) << 12) | ((v[2] as u32) << 6) | (v[3] as u32);
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            result.push((triple & 0xFF) as u8);
        }
    }
    Some(result)
}
