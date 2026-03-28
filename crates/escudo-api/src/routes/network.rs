use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::Json;
use escudo_common::EscudoError;
use serde::Serialize;

use crate::middleware::AuthUser;
use crate::state::AppState;
use crate::telemetry::resolve_request_telemetry;

#[derive(Debug, Serialize)]
pub struct NetworkInfoResponse {
    pub ip: String,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub city: Option<String>,
    pub connected: bool,
    pub active_server_name: Option<String>,
    pub active_server_country_code: Option<String>,
}

pub async fn get_network_info(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
) -> escudo_common::Result<Json<NetworkInfoResponse>> {
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    let client_ip = telemetry
        .ip
        .ok_or_else(|| EscudoError::Internal("Could not determine client IP".into()))?;

    let active_server = sqlx::query_as::<_, (String, Option<String>)>(
        r#"
        SELECT s.name, s.country_code
        FROM devices d
        JOIN servers s ON s.id = d.server_id
        WHERE d.user_id = $1
          AND d.is_active = true
        ORDER BY d.updated_at DESC, d.created_at DESC
        LIMIT 1
        "#,
    )
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| EscudoError::Internal(format!("Failed to load active server: {e}")))?;

    Ok(Json(NetworkInfoResponse {
        ip: client_ip.to_string(),
        country: telemetry.country,
        country_code: telemetry.country_code,
        city: None,
        connected: active_server.is_some(),
        active_server_name: active_server.as_ref().map(|(name, _)| name.clone()),
        active_server_country_code: active_server.and_then(|(_, country_code)| country_code),
    }))
}
