use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for a streaming service to health-check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingService {
    /// Human-readable service name (e.g. "netflix").
    pub name: String,
    /// URL to GET for the health check.
    pub url: String,
    /// Substrings in the response body that indicate the IP is blocked.
    pub block_indicators: Vec<String>,
}

/// Top-level guardian configuration loaded from `guardian.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianConfig {
    /// PostgreSQL connection URL.
    pub database_url: String,
    /// How often to run the health-check cycle, in seconds. Default: 1800 (30 min).
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,
    /// Streaming services to check against each proxy IP.
    #[serde(default)]
    pub services: Vec<StreamingService>,
}

fn default_check_interval() -> u64 {
    1800
}

impl GuardianConfig {
    /// Load configuration from a TOML file at the given path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: GuardianConfig = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(config)
    }
}
