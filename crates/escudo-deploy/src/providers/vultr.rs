use crate::provider::{ProvisionedServer, ServerProvider};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::Engine;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

const BASE_URL: &str = "https://api.vultr.com/v2";
const UBUNTU_24_04_OS_ID: u32 = 2284;

pub struct VultrProvider {
    client: Client,
    api_key: String,
}

impl VultrProvider {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { client, api_key })
    }
}

#[derive(Debug, Deserialize)]
struct VultrInstance {
    id: String,
    label: String,
    region: String,
    plan: String,
    main_ip: String,
    server_status: String,
    status: String,
    tag: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

impl VultrInstance {
    fn to_provisioned(&self) -> ProvisionedServer {
        let public_ip = if self.main_ip == "0.0.0.0" {
            None
        } else {
            Some(self.main_ip.clone())
        };

        ProvisionedServer {
            provider: "vultr".to_string(),
            provider_instance_id: self.id.clone(),
            label: self.label.clone(),
            region: self.region.clone(),
            plan: self.plan.clone(),
            public_ip,
            status: self.server_status.clone(),
            monthly_cost_usd: plan_cost(&self.plan),
        }
    }
}

fn plan_cost(plan: &str) -> f64 {
    match plan {
        "vc2-1c-1gb" => 5.0,
        "vc2-1c-2gb" => 10.0,
        "vc2-2c-4gb" => 20.0,
        _ => 5.0,
    }
}

#[derive(Debug, Serialize)]
struct CreateInstanceRequest {
    region: String,
    plan: String,
    os_id: u32,
    label: String,
    user_data: String,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreateInstanceResponse {
    instance: VultrInstance,
}

#[derive(Debug, Deserialize)]
struct ListInstancesResponse {
    instances: Vec<VultrInstance>,
}

#[derive(Debug, Deserialize)]
struct GetInstanceResponse {
    instance: VultrInstance,
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    account: AccountInfo,
}

#[derive(Debug, Deserialize)]
struct AccountInfo {
    balance: f64,
    pending_charges: f64,
    email: String,
}

#[async_trait]
impl ServerProvider for VultrProvider {
    fn provider_name(&self) -> &str {
        "vultr"
    }

    async fn validate(&self) -> Result<()> {
        let url = format!("{}/account", BASE_URL);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .context("Failed to reach Vultr API")?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            return Err(anyhow!("Vultr API key is invalid (401 Unauthorized)"));
        }

        let body: AccountResponse = resp
            .error_for_status()
            .context("Vultr account request failed")?
            .json()
            .await
            .context("Failed to parse Vultr account response")?;

        info!(
            email = %body.account.email,
            balance = body.account.balance,
            pending_charges = body.account.pending_charges,
            "Vultr account validated"
        );

        Ok(())
    }

    async fn create_server(
        &self,
        label: &str,
        region: &str,
        plan: &str,
        user_data: &str,
        tags: &[String],
    ) -> Result<ProvisionedServer> {
        let encoded_user_data = base64::engine::general_purpose::STANDARD.encode(user_data);

        let payload = CreateInstanceRequest {
            region: region.to_string(),
            plan: plan.to_string(),
            os_id: UBUNTU_24_04_OS_ID,
            label: label.to_string(),
            user_data: encoded_user_data,
            tags: tags.to_vec(),
        };

        let url = format!("{}/instances", BASE_URL);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .context("Failed to send create instance request to Vultr")?
            .error_for_status()
            .context("Vultr create instance request failed")?;

        let body: CreateInstanceResponse = resp
            .json()
            .await
            .context("Failed to parse Vultr create instance response")?;

        info!(
            id = %body.instance.id,
            label = %body.instance.label,
            region = %body.instance.region,
            "Vultr instance created"
        );

        Ok(body.instance.to_provisioned())
    }

    async fn list_servers(&self) -> Result<Vec<ProvisionedServer>> {
        let url = format!("{}/instances?tag=escudo-vpn&per_page=500", BASE_URL);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .context("Failed to list Vultr instances")?
            .error_for_status()
            .context("Vultr list instances request failed")?;

        let body: ListInstancesResponse = resp
            .json()
            .await
            .context("Failed to parse Vultr list instances response")?;

        Ok(body.instances.iter().map(|i| i.to_provisioned()).collect())
    }

    async fn get_server(&self, instance_id: &str) -> Result<ProvisionedServer> {
        let url = format!("{}/instances/{}", BASE_URL, instance_id);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .context("Failed to get Vultr instance")?
            .error_for_status()
            .context("Vultr get instance request failed")?;

        let body: GetInstanceResponse = resp
            .json()
            .await
            .context("Failed to parse Vultr get instance response")?;

        let server = body.instance.to_provisioned();

        if server.status != "ok" {
            warn!(
                id = %instance_id,
                status = %server.status,
                "Vultr instance not yet ready"
            );
        }

        Ok(server)
    }

    async fn destroy_server(&self, instance_id: &str) -> Result<()> {
        let url = format!("{}/instances/{}", BASE_URL, instance_id);
        self.client
            .delete(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .context("Failed to send destroy request to Vultr")?
            .error_for_status()
            .context("Vultr destroy instance request failed")?;

        info!(id = %instance_id, "Vultr instance destroyed");
        Ok(())
    }
}
