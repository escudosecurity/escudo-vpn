use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionedServer {
    pub provider: String,
    pub provider_instance_id: String,
    pub label: String,
    pub region: String,
    pub plan: String,
    pub public_ip: Option<String>,
    pub status: String,
    pub monthly_cost_usd: f64,
}

#[async_trait]
pub trait ServerProvider: Send + Sync {
    fn provider_name(&self) -> &str;

    async fn validate(&self) -> Result<()>;

    async fn create_server(
        &self,
        label: &str,
        region: &str,
        plan: &str,
        user_data: &str,
        tags: &[String],
    ) -> Result<ProvisionedServer>;

    async fn list_servers(&self) -> Result<Vec<ProvisionedServer>>;

    async fn get_server(&self, instance_id: &str) -> Result<ProvisionedServer>;

    async fn destroy_server(&self, instance_id: &str) -> Result<()>;
}
