use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::config::{ProxyConfig, ProxyPollConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyTargetKind {
    Shared,
    Dedicated,
}

impl ProxyTargetKind {
    pub fn from_proto(target: i32) -> Self {
        if target == 1 {
            Self::Dedicated
        } else {
            Self::Shared
        }
    }

    fn env_file_name(self) -> &'static str {
        match self {
            Self::Shared => "proxy-shared.env",
            Self::Dedicated => "proxy-dedicated.env",
        }
    }

    fn service_name<'a>(self, cfg: &'a ProxyConfig) -> &'a str {
        match self {
            Self::Shared => &cfg.shared_service,
            Self::Dedicated => &cfg.dedicated_service,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyCredential {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Default)]
struct ProxyState {
    shared: Option<ProxyCredential>,
    dedicated: Option<ProxyCredential>,
}

#[derive(Debug, Deserialize)]
struct ServerProxyCredentialsResponse {
    shared: Option<ProxyCredentialView>,
    dedicated: Option<ProxyCredentialView>,
}

#[derive(Debug, Deserialize)]
struct ProxyCredentialView {
    socks5_host: String,
    socks5_port: i32,
    socks5_username: String,
    socks5_password: String,
}

impl TryFrom<ProxyCredentialView> for ProxyCredential {
    type Error = anyhow::Error;

    fn try_from(value: ProxyCredentialView) -> Result<Self> {
        let port = u16::try_from(value.socks5_port).context("proxy port out of range")?;
        Ok(Self {
            host: value.socks5_host,
            port,
            username: value.socks5_username,
            password: value.socks5_password,
        })
    }
}

#[derive(Clone)]
pub struct ProxyManager {
    cfg: ProxyConfig,
    client: reqwest::Client,
    state: Arc<RwLock<ProxyState>>,
}

impl ProxyManager {
    pub fn new(cfg: ProxyConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to build proxy poll client")?;

        Ok(Self {
            cfg,
            client,
            state: Arc::new(RwLock::new(ProxyState::default())),
        })
    }

    pub async fn update_credentials(
        &self,
        target: ProxyTargetKind,
        credential: ProxyCredential,
    ) -> Result<()> {
        let changed = {
            let mut state = self.state.write().await;
            let slot = match target {
                ProxyTargetKind::Shared => &mut state.shared,
                ProxyTargetKind::Dedicated => &mut state.dedicated,
            };
            if slot.as_ref() == Some(&credential) {
                false
            } else {
                *slot = Some(credential.clone());
                true
            }
        };

        if !changed {
            return Ok(());
        }

        self.persist_env(target, &credential).await?;
        self.restart_service(target).await?;
        info!(target = ?target, "updated proxy credentials");
        Ok(())
    }

    async fn persist_env(
        &self,
        target: ProxyTargetKind,
        credential: &ProxyCredential,
    ) -> Result<()> {
        let env_dir = Path::new(&self.cfg.env_dir);
        fs::create_dir_all(env_dir)
            .await
            .with_context(|| format!("failed to create proxy env dir {}", env_dir.display()))?;

        let env_path = env_dir.join(target.env_file_name());
        let body = format!(
            "SOCKS5_HOST={}\nSOCKS5_PORT={}\nSOCKS5_USERNAME={}\nSOCKS5_PASSWORD={}\n",
            credential.host, credential.port, credential.username, credential.password
        );
        fs::write(&env_path, body)
            .await
            .with_context(|| format!("failed to write {}", env_path.display()))?;
        Ok(())
    }

    async fn restart_service(&self, target: ProxyTargetKind) -> Result<()> {
        let service = target.service_name(&self.cfg);
        let output = Command::new("systemctl")
            .args(["restart", service])
            .output()
            .await
            .with_context(|| format!("failed to restart {}", service))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("systemctl restart {} failed: {}", service, stderr.trim());
        }

        Ok(())
    }

    pub fn spawn_poller(self: Arc<Self>) -> Option<tokio::task::JoinHandle<()>> {
        let poll = self.cfg.poll.clone()?;
        if !self.cfg.enabled {
            return None;
        }

        Some(tokio::spawn(async move {
            let interval = Duration::from_secs(poll.poll_interval_secs.max(15));
            loop {
                if let Err(err) = self.poll_once(&poll).await {
                    warn!(error = %err, "proxy credential poll failed");
                }
                tokio::time::sleep(interval).await;
            }
        }))
    }

    async fn poll_once(&self, poll: &ProxyPollConfig) -> Result<()> {
        let base = poll.central_api_url.trim_end_matches('/');
        let url = format!(
            "{}/internal/servers/{}/proxy-credentials",
            base, poll.server_label
        );
        let response = self
            .client
            .get(&url)
            .bearer_auth(&poll.deploy_secret)
            .send()
            .await
            .with_context(|| format!("failed to reach {}", url))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }

        let response = response
            .error_for_status()
            .with_context(|| format!("proxy credential poll returned error for {}", url))?;
        let body: ServerProxyCredentialsResponse = response
            .json()
            .await
            .context("failed to decode proxy credential response")?;

        if let Some(shared) = body.shared {
            match ProxyCredential::try_from(shared) {
                Ok(cred) => {
                    if let Err(err) = self.update_credentials(ProxyTargetKind::Shared, cred).await {
                        error!(error = %err, "failed to apply shared proxy credentials");
                    }
                }
                Err(err) => warn!(error = %err, "invalid shared proxy credentials from API"),
            }
        }

        if let Some(dedicated) = body.dedicated {
            match ProxyCredential::try_from(dedicated) {
                Ok(cred) => {
                    if let Err(err) = self
                        .update_credentials(ProxyTargetKind::Dedicated, cred)
                        .await
                    {
                        error!(error = %err, "failed to apply dedicated proxy credentials");
                    }
                }
                Err(err) => warn!(error = %err, "invalid dedicated proxy credentials from API"),
            }
        }

        Ok(())
    }

    pub fn env_path(&self, target: ProxyTargetKind) -> PathBuf {
        Path::new(&self.cfg.env_dir).join(target.env_file_name())
    }
}
