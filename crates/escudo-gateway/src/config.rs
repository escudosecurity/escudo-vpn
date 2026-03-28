use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    pub wireguard: WireguardConfig,
    pub stats: StatsConfig,
    pub proxy: Option<ProxyConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub grpc_addr: String,
    pub health_addr: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WireguardConfig {
    pub interface: String,
    pub subnet: String,
    pub ip_start: String,
    pub ip_end: String,
    #[serde(default = "default_wg1_interface")]
    pub wg1_interface: String,
    #[serde(default = "default_wg2_interface")]
    pub wg2_interface: String,
}

fn default_wg1_interface() -> String {
    "wg1".to_string()
}

fn default_wg2_interface() -> String {
    "wg2".to_string()
}

#[derive(Debug, Deserialize)]
pub struct StatsConfig {
    pub collection_interval_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_proxy_env_dir")]
    pub env_dir: String,
    #[serde(default = "default_shared_service")]
    pub shared_service: String,
    #[serde(default = "default_dedicated_service")]
    pub dedicated_service: String,
    pub poll: Option<ProxyPollConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyPollConfig {
    pub central_api_url: String,
    pub server_label: String,
    pub deploy_secret: String,
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,
}

fn default_proxy_env_dir() -> String {
    "/etc/escudo".to_string()
}

fn default_shared_service() -> String {
    "escudo-tun2socks-shared.service".to_string()
}

fn default_dedicated_service() -> String {
    "escudo-tun2socks-dedicated.service".to_string()
}

fn default_poll_interval_secs() -> u64 {
    60
}
