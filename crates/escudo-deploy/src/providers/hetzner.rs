use crate::provider::{ProvisionedServer, ServerProvider};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

const BASE_URL: &str = "https://api.hetzner.cloud/v1";

pub struct HetznerProvider {
    client: Client,
    api_token: String,
}

impl HetznerProvider {
    pub fn new(api_token: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { client, api_token })
    }
}

fn plan_cost(server_type: &str) -> f64 {
    match server_type {
        "cx22" => 4.20,
        "cpx21" => 5.00,
        "cx32" => 7.50,
        _ => 5.00,
    }
}

#[derive(Debug, Deserialize)]
struct HetznerServer {
    id: u64,
    name: String,
    status: String,
    public_net: PublicNet,
    server_type: HetznerServerType,
    datacenter: Datacenter,
}

#[derive(Debug, Deserialize)]
struct PublicNet {
    ipv4: Option<Ipv4Info>,
}

#[derive(Debug, Deserialize)]
struct Ipv4Info {
    ip: String,
}

#[derive(Debug, Deserialize)]
struct HetznerServerType {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Datacenter {
    location: Location,
}

#[derive(Debug, Deserialize)]
struct Location {
    name: String,
}

impl HetznerServer {
    fn to_provisioned(&self) -> ProvisionedServer {
        let public_ip = self.public_net.ipv4.as_ref().map(|v| v.ip.clone());
        ProvisionedServer {
            provider: "hetzner".to_string(),
            provider_instance_id: self.id.to_string(),
            label: self.name.clone(),
            region: self.datacenter.location.name.clone(),
            plan: self.server_type.name.clone(),
            public_ip,
            status: self.status.clone(),
            monthly_cost_usd: plan_cost(&self.server_type.name),
        }
    }
}

#[derive(Debug, Serialize)]
struct CreateServerRequest {
    name: String,
    server_type: String,
    image: String,
    location: String,
    user_data: String,
    start_after_create: bool,
    labels: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct CreateServerResponse {
    server: HetznerServer,
}

#[derive(Debug, Deserialize)]
struct ListServersResponse {
    servers: Vec<HetznerServer>,
}

#[derive(Debug, Deserialize)]
struct GetServerResponse {
    server: HetznerServer,
}

#[async_trait]
impl ServerProvider for HetznerProvider {
    fn provider_name(&self) -> &str {
        "hetzner"
    }

    async fn validate(&self) -> Result<()> {
        let url = format!("{}/servers?per_page=1", BASE_URL);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .context("Failed to reach Hetzner API")?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            return Err(anyhow!("Hetzner API token is invalid (401 Unauthorized)"));
        }

        resp.error_for_status()
            .context("Hetzner validation request failed")?;

        info!("Hetzner Cloud API validated successfully");
        Ok(())
    }

    async fn create_server(
        &self,
        label: &str,
        region: &str,
        plan: &str,
        user_data: &str,
        _tags: &[String],
    ) -> Result<ProvisionedServer> {
        let mut labels = HashMap::new();
        labels.insert("managed-by".to_string(), "escudo-deploy".to_string());

        let payload = CreateServerRequest {
            name: label.to_string(),
            server_type: plan.to_string(),
            image: "ubuntu-24.04".to_string(),
            location: region.to_string(),
            user_data: user_data.to_string(),
            start_after_create: true,
            labels,
        };

        let url = format!("{}/servers", BASE_URL);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&payload)
            .send()
            .await
            .context("Failed to send create server request to Hetzner")?
            .error_for_status()
            .context("Hetzner create server request failed")?;

        let body: CreateServerResponse = resp
            .json()
            .await
            .context("Failed to parse Hetzner create server response")?;

        info!(
            id = body.server.id,
            name = %body.server.name,
            location = %body.server.datacenter.location.name,
            "Hetzner server created"
        );

        Ok(body.server.to_provisioned())
    }

    async fn list_servers(&self) -> Result<Vec<ProvisionedServer>> {
        let url = format!(
            "{}/servers?label_selector=managed-by=escudo-deploy&per_page=50",
            BASE_URL
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .context("Failed to list Hetzner servers")?
            .error_for_status()
            .context("Hetzner list servers request failed")?;

        let body: ListServersResponse = resp
            .json()
            .await
            .context("Failed to parse Hetzner list servers response")?;

        Ok(body.servers.iter().map(|s| s.to_provisioned()).collect())
    }

    async fn get_server(&self, instance_id: &str) -> Result<ProvisionedServer> {
        let url = format!("{}/servers/{}", BASE_URL, instance_id);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .context("Failed to get Hetzner server")?
            .error_for_status()
            .context("Hetzner get server request failed")?;

        let body: GetServerResponse = resp
            .json()
            .await
            .context("Failed to parse Hetzner get server response")?;

        let server = body.server.to_provisioned();

        if server.status != "running" {
            warn!(
                id = %instance_id,
                status = %server.status,
                "Hetzner server not yet running"
            );
        }

        Ok(server)
    }

    async fn destroy_server(&self, instance_id: &str) -> Result<()> {
        let url = format!("{}/servers/{}", BASE_URL, instance_id);
        self.client
            .delete(&url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .context("Failed to send destroy request to Hetzner")?
            .error_for_status()
            .context("Hetzner destroy server request failed")?;

        info!(id = %instance_id, "Hetzner server destroyed");
        Ok(())
    }
}
