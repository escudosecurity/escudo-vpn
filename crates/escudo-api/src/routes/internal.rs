use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use escudo_common::EscudoError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ServerRegisterRequest {
    pub public_ip: String,
    #[serde(alias = "wg0_pubkey")]
    pub wg0_public_key: Option<String>,
    #[serde(alias = "wg1_pubkey")]
    pub wg1_public_key: Option<String>,
    #[serde(alias = "wg2_pubkey")]
    pub wg2_public_key: Option<String>,
    pub wg0_port: Option<i32>,
    pub wg1_port: Option<i32>,
    pub wg2_port: Option<i32>,
    pub location: Option<String>,
    pub country_code: Option<String>,
    pub gateway_grpc_addr: Option<String>,
    pub provider: Option<String>,
    pub label: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize)]
pub struct ProxyCredentialView {
    pub proxy_ip_id: Uuid,
    pub proxy_target: String,
    pub provider: String,
    pub provider_proxy_id: String,
    pub proxy_type: String,
    pub country: String,
    pub socks5_host: String,
    pub socks5_port: i32,
    pub socks5_username: String,
    pub socks5_password: String,
    pub external_ip: Option<String>,
    pub assigned_at: String,
}

#[derive(Serialize)]
pub struct ServerProxyCredentialsResponse {
    pub label: String,
    pub shared: Option<ProxyCredentialView>,
    pub dedicated: Option<ProxyCredentialView>,
}

fn require_deploy_secret(headers: &HeaderMap) -> Result<(), EscudoError> {
    let deploy_secret = std::env::var("DEPLOY_SECRET").unwrap_or_default();
    if deploy_secret.is_empty() {
        return Err(EscudoError::Internal(
            "DEPLOY_SECRET is not configured".into(),
        ));
    }

    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = format!("Bearer {deploy_secret}");
    if auth_header != expected {
        return Err(EscudoError::Unauthorized("Invalid deploy secret".into()));
    }

    Ok(())
}

pub async fn register_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ServerRegisterRequest>,
) -> escudo_common::Result<StatusCode> {
    require_deploy_secret(&headers)?;

    let location = req.location.unwrap_or_else(|| "Unknown".to_string());
    let country_code = req.country_code.map(|c| c.trim().to_uppercase());
    let wg0_port = req.wg0_port.unwrap_or(51820);
    let wg1_port = req.wg1_port.unwrap_or(51821);
    let wg2_port = req.wg2_port.unwrap_or(51822);
    let gateway_grpc_addr = req
        .gateway_grpc_addr
        .unwrap_or_else(|| format!("http://{}:9090", req.public_ip));

    let existing_server_id = if let Some(label) = &req.label {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT ps.server_id
            FROM provider_servers ps
            WHERE ps.label = $1
            "#,
        )
        .bind(label)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            EscudoError::Internal(format!("Failed to look up existing provider server: {e}"))
        })?
    } else {
        None
    }
    .or_else(|| {
        // Fallback for older rows that may already exist in servers without provider_servers linkage.
        None
    });

    let server_id: uuid::Uuid = if let Some(server_id) = existing_server_id {
        sqlx::query_scalar(
            r#"
            UPDATE servers
            SET
                public_ip = $2,
                name = $2,
                location = $3,
                public_key = COALESCE($4, public_key, ''),
                endpoint_port = $5,
                wg0_public_key = $4,
                wg0_port = $5,
                wg1_public_key = $6,
                wg1_port = $7,
                wg2_public_key = $8,
                wg2_port = $9,
                gateway_grpc_addr = $10,
                country_code = COALESCE($11, country_code),
                is_active = true,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id
            "#,
        )
        .bind(server_id)
        .bind(&req.public_ip)
        .bind(&location)
        .bind(&req.wg0_public_key)
        .bind(wg0_port)
        .bind(&req.wg1_public_key)
        .bind(wg1_port)
        .bind(&req.wg2_public_key)
        .bind(wg2_port)
        .bind(&gateway_grpc_addr)
        .bind(&country_code)
        .fetch_one(&state.db)
        .await
        .map_err(|e| EscudoError::Internal(format!("Failed to update server: {e}")))?
    } else {
        sqlx::query_scalar(
            r#"
            INSERT INTO servers (
                public_ip, name, location, public_key, endpoint_port,
                wg0_public_key, wg0_port,
                wg1_public_key, wg1_port,
                wg2_public_key, wg2_port,
                gateway_grpc_addr, country_code,
                is_active
            ) VALUES (
                $1, $1, $2, COALESCE($3, ''), $4,
                $3, $4,
                $5, $6,
                $7, $8,
                $9, $10,
                true
            )
            RETURNING id
            "#,
        )
        .bind(&req.public_ip)
        .bind(&location)
        .bind(&req.wg0_public_key)
        .bind(wg0_port)
        .bind(&req.wg1_public_key)
        .bind(wg1_port)
        .bind(&req.wg2_public_key)
        .bind(wg2_port)
        .bind(&gateway_grpc_addr)
        .bind(&country_code)
        .fetch_one(&state.db)
        .await
        .map_err(|e| EscudoError::Internal(format!("Failed to insert server: {e}")))?
    };

    // Upsert into provider_servers if provider/label are given
    if let (Some(provider), Some(label)) = (&req.provider, &req.label) {
        sqlx::query(
            r#"
            INSERT INTO provider_servers (
                server_id, provider, provider_instance_id, label, region, plan,
                public_ip, status, gateway_version, last_heartbeat
            ) VALUES (
                $1, $2, $3, $3, $4, 'custom',
                $5, 'active', $6, NOW()
            )
            ON CONFLICT (label) DO UPDATE SET
                server_id       = EXCLUDED.server_id,
                public_ip       = EXCLUDED.public_ip,
                status          = 'active',
                gateway_version = EXCLUDED.gateway_version,
                last_heartbeat  = NOW(),
                updated_at      = NOW()
            "#,
        )
        .bind(server_id)
        .bind(provider)
        .bind(label)
        .bind(&location)
        .bind(&req.public_ip)
        .bind(&req.version)
        .execute(&state.db)
        .await
        .map_err(|e| EscudoError::Internal(format!("Failed to upsert provider_server: {e}")))?;
    }

    Ok(StatusCode::OK)
}

pub async fn get_server_proxy_credentials(
    State(state): State<AppState>,
    Path(label): Path<String>,
    headers: HeaderMap,
) -> escudo_common::Result<Json<ServerProxyCredentialsResponse>> {
    require_deploy_secret(&headers)?;

    let rows = sqlx::query_as::<
        _,
        (
            String,
            Uuid,
            String,
            String,
            String,
            String,
            String,
            String,
            i32,
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT
            spa.proxy_target,
            pi.id,
            pi.provider,
            pi.provider_proxy_id,
            pi.proxy_type,
            pi.country,
            pi.socks5_host,
            pi.socks5_username,
            pi.socks5_port,
            pi.socks5_password,
            COALESCE(pi.external_ip, ''),
            pi.external_ip,
            spa.assigned_at
        FROM provider_servers ps
        JOIN server_proxy_assignments spa ON spa.server_id = ps.server_id
        JOIN proxy_ips pi ON pi.id = spa.proxy_ip_id
        WHERE ps.label = $1
        ORDER BY spa.assigned_at DESC
        "#,
    )
    .bind(&label)
    .fetch_all(&state.db)
    .await
    .map_err(|e| EscudoError::Internal(format!("Failed to fetch proxy credentials: {e}")))?;

    let mut shared = None;
    let mut dedicated = None;

    for (
        proxy_target,
        proxy_ip_id,
        provider,
        provider_proxy_id,
        proxy_type,
        country,
        socks5_host,
        socks5_username,
        socks5_port,
        socks5_password,
        _external_ip_unused,
        external_ip,
        assigned_at,
    ) in rows
    {
        let view = ProxyCredentialView {
            proxy_ip_id,
            proxy_target: proxy_target.clone(),
            provider,
            provider_proxy_id,
            proxy_type,
            country,
            socks5_host,
            socks5_port,
            socks5_username,
            socks5_password,
            external_ip,
            assigned_at: assigned_at.to_rfc3339(),
        };

        match proxy_target.as_str() {
            "dedicated" => dedicated = Some(view),
            _ => shared = Some(view),
        }
    }

    Ok(Json(ServerProxyCredentialsResponse {
        label,
        shared,
        dedicated,
    }))
}
