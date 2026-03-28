use sqlx::PgPool;
use std::sync::Arc;

use crate::config::ApiConfig;

pub mod gateway {
    tonic::include_proto!("gateway");
}

pub type GatewayClient =
    gateway::gateway_service_client::GatewayServiceClient<tonic::transport::Channel>;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub gateway: GatewayClient,
    pub config: Arc<ApiConfig>,
}
