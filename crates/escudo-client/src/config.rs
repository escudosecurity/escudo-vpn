use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct WgConfig {
    pub private_key: Vec<u8>,
    pub address: Ipv4Addr,
    pub dns: Ipv4Addr,
    pub peer_public_key: Vec<u8>,
    pub preshared_key: Vec<u8>,
    pub endpoint: String,
    pub allowed_ips: String,
}

impl WgConfig {
    pub fn parse(config_str: &str) -> Result<Self, String> {
        let mut private_key = None;
        let mut address = None;
        let mut dns = None;
        let mut peer_public_key = None;
        let mut preshared_key = None;
        let mut endpoint = None;
        let mut allowed_ips = None;

        for line in config_str.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "PrivateKey" => private_key = Some(value.to_string()),
                    "Address" => {
                        let ip_str = value.split('/').next().unwrap_or(value);
                        address = ip_str.parse().ok();
                    }
                    "DNS" => dns = value.parse().ok(),
                    "PublicKey" => peer_public_key = Some(value.to_string()),
                    "PresharedKey" => preshared_key = Some(value.to_string()),
                    "Endpoint" => endpoint = Some(value.to_string()),
                    "AllowedIPs" => allowed_ips = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;

        Ok(WgConfig {
            private_key: b64
                .decode(private_key.ok_or("Missing PrivateKey")?)
                .map_err(|e| e.to_string())?,
            address: address.ok_or("Missing Address")?,
            dns: dns.ok_or("Missing DNS")?,
            peer_public_key: b64
                .decode(peer_public_key.ok_or("Missing PublicKey")?)
                .map_err(|e| e.to_string())?,
            preshared_key: b64
                .decode(preshared_key.ok_or("Missing PresharedKey")?)
                .map_err(|e| e.to_string())?,
            endpoint: endpoint.ok_or("Missing Endpoint")?.to_string(),
            allowed_ips: allowed_ips.ok_or("Missing AllowedIPs")?.to_string(),
        })
    }
}
