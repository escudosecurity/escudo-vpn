use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SniProxyConfig {
    pub listen: ListenConfig,
    pub streaming: StreamingConfig,
}

#[derive(Debug, Deserialize)]
pub struct ListenConfig {
    pub addr: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct StreamingConfig {
    pub domains: Vec<String>,
    pub vpn_bind_ip: String,
}
