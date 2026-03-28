use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub gateway: GatewayConfig,
    pub jwt: JwtConfig,
    pub wireguard: WireguardConfig,
    pub stripe: Option<StripeConfig>,
    pub proxy: Option<ProxyConfig>,
    pub testing: Option<TestingConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub addr: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GatewayConfig {
    pub grpc_addr: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WireguardConfig {
    #[allow(dead_code)]
    pub server_public_key: String,
    #[allow(dead_code)]
    pub server_endpoint: String,
    pub dns: String,
    pub allowed_ips: String,
    pub encryption_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StripeConfig {
    pub secret_key: String,
    pub webhook_secret: String,
    pub price_id: String,
    pub app_url: String,
    #[allow(dead_code)]
    pub price_id_annual: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    pub iproyal_api_token: String,
    pub sticky_duration_mins: Option<u64>,
    pub default_country_code: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TestingConfig {
    #[serde(default)]
    pub open_server_access: bool,
    #[serde(default)]
    pub disable_device_limits: bool,
}

impl ApiConfig {
    pub fn validate_runtime(&self) -> anyhow::Result<()> {
        ensure_runtime_secret("database.url", &self.database.url)?;
        ensure_runtime_secret("jwt.secret", &self.jwt.secret)?;
        ensure_runtime_secret("wireguard.encryption_key", &self.wireguard.encryption_key)?;

        if let Some(stripe) = &self.stripe {
            ensure_runtime_secret("stripe.secret_key", &stripe.secret_key)?;
            ensure_runtime_secret("stripe.webhook_secret", &stripe.webhook_secret)?;
        }

        if let Some(proxy) = &self.proxy {
            ensure_runtime_secret("proxy.iproyal_api_token", &proxy.iproyal_api_token)?;
        }

        Ok(())
    }
}

fn ensure_runtime_secret(field: &str, value: &str) -> anyhow::Result<()> {
    let trimmed = value.trim();
    let is_placeholder = trimmed.is_empty()
        || trimmed.contains("CHANGE_ME")
        || trimmed.contains("REPLACE_ME")
        || trimmed.contains("example")
        || trimmed.contains("placeholder");

    anyhow::ensure!(
        !is_placeholder,
        "{field} must be supplied from a real runtime secret, not a placeholder value",
    );

    Ok(())
}
