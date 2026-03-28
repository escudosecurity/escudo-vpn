use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct AdminConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ServerConfig {
    pub addr: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

impl AdminConfig {
    pub fn validate_runtime(&self) -> anyhow::Result<()> {
        ensure_runtime_secret("database.url", &self.database.url)?;
        ensure_runtime_secret("jwt.secret", &self.jwt.secret)?;
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
