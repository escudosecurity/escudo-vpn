use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeployConfig {
    pub defaults: Defaults,
    pub servers: Vec<ServerEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Defaults {
    pub ssh_key_ids: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerEntry {
    pub label: String,
    pub provider: String,
    pub region: String,
    pub plan: String,
}

impl DeployConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        let config: DeployConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;
        Ok(config)
    }
}
