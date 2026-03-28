use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DnsConfig {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    pub blocklist: BlocklistConfig,
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub listen_port: u16,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UpstreamConfig {
    pub doh_url: String,
}

#[derive(Debug, Deserialize)]
pub struct BlocklistConfig {
    pub sources: Vec<BlocklistSource>,
    pub refresh_interval_hours: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlocklistSource {
    pub name: String,
    pub url: String,
}
