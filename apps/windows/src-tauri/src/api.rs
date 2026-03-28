use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://api.escudovpn.com";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthResponse {
    pub token: String,
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnonymousAccountResponse {
    pub account_number: String,
    pub tier: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LaunchControls {
    pub free_beta_label: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LaunchStatusResponse {
    pub controls: LaunchControls,
    pub effective_tier: String,
    pub active_invites: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub location: String,
    pub load_percent: u8,
    #[serde(default)]
    pub country_code: Option<String>,
    #[serde(default)]
    pub service_class: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectResponse {
    pub device_id: String,
    pub config: String,
    pub qr_code: String,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .danger_accept_invalid_certs(false)
                .build()
                .unwrap_or_default(),
            base_url: API_BASE.to_string(),
        }
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        token: Option<&str>,
    ) -> Result<AuthResponse> {
        let url = format!("{}/api/v1/auth/login", self.base_url);
        let mut req = self.client.post(&url).json(&serde_json::json!({
            "email": email,
            "password": password
        }));

        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Login failed ({}): {}", status, body));
        }

        let auth: AuthResponse = resp.json().await?;
        Ok(auth)
    }

    pub async fn login_number(&self, account_number: &str) -> Result<AuthResponse> {
        let url = format!("{}/api/v1/auth/login-number", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "account_number": account_number
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Code login failed ({}): {}", status, body));
        }

        let auth: AuthResponse = resp.json().await?;
        Ok(auth)
    }

    pub async fn register(&self, email: &str, password: &str) -> Result<AuthResponse> {
        let url = format!("{}/api/v1/auth/register", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Registration failed ({}): {}", status, body));
        }

        let auth: AuthResponse = resp.json().await?;
        Ok(auth)
    }

    pub async fn create_anonymous_account(&self) -> Result<AnonymousAccountResponse> {
        let url = format!("{}/api/v1/auth/anonymous", self.base_url);
        let resp = self.client.post(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Anonymous signup failed ({}): {}", status, body));
        }

        let account: AnonymousAccountResponse = resp.json().await?;
        Ok(account)
    }

    pub async fn scan_qr_token(&self, raw_value: &str) -> Result<AuthResponse> {
        let token = extract_qr_token(raw_value)?;
        let url = format!("{}/api/v1/auth/qr/scan", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "qr_token": token
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("QR scan failed ({}): {}", status, body));
        }

        let auth: AuthResponse = resp.json().await?;
        Ok(auth)
    }

    pub async fn get_launch_status(&self, token: &str) -> Result<LaunchStatusResponse> {
        let url = format!("{}/api/v1/launch/status", self.base_url);
        let resp = self.client.get(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to get launch status ({}): {}",
                status,
                body
            ));
        }

        let status: LaunchStatusResponse = resp.json().await?;
        Ok(status)
    }

    pub async fn get_servers(&self, token: &str) -> Result<Vec<Server>> {
        let url = format!("{}/api/v1/servers", self.base_url);
        let resp = self.client.get(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to get servers ({}): {}", status, body));
        }

        let servers: Vec<Server> = resp.json().await?;
        Ok(servers)
    }

    pub async fn connect(
        &self,
        token: &str,
        server_id: &str,
        device_name: &str,
        device_install_id: &str,
    ) -> Result<ConnectResponse> {
        let url = format!("{}/api/v1/connect", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&serde_json::json!({
                "server_id": server_id,
                "device_name": device_name,
                "device_install_id": device_install_id,
                "platform": "windows",
                "usage_bucket": "normal",
                "preferred_class": "free"
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Connect failed ({}): {}", status, body));
        }

        let connect: ConnectResponse = resp.json().await?;
        Ok(connect)
    }

    pub async fn disconnect(&self, token: &str, device_id: &str) -> Result<()> {
        let url = format!("{}/api/v1/disconnect/{}", self.base_url, device_id);
        let resp = self.client.delete(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Disconnect failed ({}): {}", status, body));
        }

        Ok(())
    }
}

fn extract_qr_token(raw_value: &str) -> Result<String> {
    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("QR token is empty"));
    }

    if let Some(token) = trimmed.split("token=").nth(1) {
        let cleaned = token.split('&').next().unwrap_or(token).trim();
        if !cleaned.is_empty() {
            return Ok(cleaned.to_string());
        }
    }

    Ok(trimmed.to_string())
}
