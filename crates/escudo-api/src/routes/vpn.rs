use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use escudo_common::crypto::{
    decrypt_private_key, encrypt_private_key, generate_keypair, generate_preshared_key,
};
use escudo_common::EscudoError;
use escudo_proxy::pool::ProxyPool;
use escudo_proxy::provider::SharedProxyRequest;
use escudo_proxy::providers::iproyal::IproyalClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::HashSet;
use uuid::Uuid;

use crate::backend_control;
use crate::middleware::AuthUser;
use crate::qr::generate_qr_base64;
use crate::state::gateway::{
    AddMultihopPeerRequest, AddPeerRequest, ProxyTarget, RemovePeerRequest, Tier,
    UpdateProxyCredentialsRequest,
};
use crate::state::AppState;
use crate::telemetry::{normalize_country, resolve_request_telemetry, ClientTelemetry};

fn get_master_key(state: &AppState) -> Result<[u8; 32], EscudoError> {
    let key_b64 = &state.config.wireguard.encryption_key;
    let key_bytes = BASE64
        .decode(key_b64)
        .map_err(|_| EscudoError::Internal("Invalid encryption key".into()))?;
    key_bytes
        .try_into()
        .map_err(|_| EscudoError::Internal("Encryption key must be 32 bytes".into()))
}

/// Fetch the user's active subscription tier. Defaults to "free".
async fn get_user_tier(state: &AppState, user_id: Uuid) -> Result<String, EscudoError> {
    backend_control::effective_user_tier(&state.db, user_id).await
}

async fn locked_server_for_user(
    _state: &AppState,
    _user_id: Uuid,
) -> Result<Option<Uuid>, EscudoError> {
    Ok(None)
}

async fn enforce_device_limit(state: &AppState, user_id: Uuid) -> Result<(), EscudoError> {
    if state
        .config
        .testing
        .as_ref()
        .map(|t| t.disable_device_limits)
        .unwrap_or(false)
    {
        return Ok(());
    }

    let active_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM devices WHERE user_id = $1 AND is_active = true")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;

    let tier = get_user_tier(state, user_id).await?;

    let max_devices: i64 = match tier.as_str() {
        "escudo" => 5,
        "pro" => 10,
        "dedicated" => 10,
        _ => 1, // free
    };

    if active_count >= max_devices {
        return Err(EscudoError::BadRequest(
            "Device limit reached. Disconnect a device first.".into(),
        ));
    }

    Ok(())
}

fn tier_str_to_proto(tier: &str) -> i32 {
    match tier {
        "escudo" => Tier::Escudo as i32,
        "pro" => Tier::Pro as i32,
        "dedicated" => Tier::Dedicated as i32,
        _ => Tier::Free as i32,
    }
}

fn normalize_device_name(raw: Option<String>, fallback: &str) -> Result<String, EscudoError> {
    let device_name = raw
        .unwrap_or_else(|| fallback.to_string())
        .trim()
        .to_string();

    if device_name.is_empty() {
        return Err(EscudoError::BadRequest(
            "Device name cannot be empty".into(),
        ));
    }

    if device_name.len() > 64 {
        return Err(EscudoError::BadRequest("Device name is too long".into()));
    }

    if device_name.chars().any(|c| c.is_control()) {
        return Err(EscudoError::BadRequest(
            "Device name contains invalid characters".into(),
        ));
    }

    Ok(device_name)
}

#[derive(Clone, Default)]
struct DeviceOpsMetadata {
    device_install_id: Option<String>,
    platform: Option<String>,
    usage_bucket: String,
    preferred_class: Option<String>,
    dedicated_required: bool,
    sensitive_route: bool,
}

fn sanitize_optional_text(
    raw: Option<String>,
    max_len: usize,
    field_name: &str,
) -> Result<Option<String>, EscudoError> {
    match raw {
        Some(value) => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(None);
            }
            if value.len() > max_len {
                return Err(EscudoError::BadRequest(format!("{field_name} is too long")));
            }
            if value.chars().any(|c| c.is_control()) {
                return Err(EscudoError::BadRequest(format!(
                    "{field_name} contains invalid characters"
                )));
            }
            Ok(Some(value.to_string()))
        }
        None => Ok(None),
    }
}

fn normalize_usage_bucket(raw: Option<String>) -> String {
    raw.unwrap_or_else(|| "normal".to_string())
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .trim()
        .to_string()
        .chars()
        .take(32)
        .collect::<String>()
}

fn normalize_preferred_class(raw: Option<String>) -> Result<Option<String>, EscudoError> {
    sanitize_optional_text(raw, 32, "preferred_class").map(|value| {
        value.map(|value| value.trim().to_ascii_lowercase())
    })
}

fn normalize_platform(
    explicit_platform: Option<String>,
    telemetry: &ClientTelemetry,
) -> Result<Option<String>, EscudoError> {
    let explicit = sanitize_optional_text(explicit_platform, 32, "platform")?;
    Ok(explicit.or_else(|| telemetry.inferred_platform.clone()))
}

fn normalize_device_metadata(
    device_install_id: Option<String>,
    platform: Option<String>,
    usage_bucket: Option<String>,
    preferred_class: Option<String>,
    dedicated_required: Option<bool>,
    sensitive_route: Option<bool>,
    telemetry: &ClientTelemetry,
) -> Result<DeviceOpsMetadata, EscudoError> {
    let usage_bucket = normalize_usage_bucket(usage_bucket);
    let usage_bucket = if usage_bucket.is_empty() {
        "normal".to_string()
    } else {
        usage_bucket
    };

    Ok(DeviceOpsMetadata {
        device_install_id: sanitize_optional_text(device_install_id, 128, "device_install_id")?,
        platform: normalize_platform(platform, telemetry)?,
        usage_bucket,
        preferred_class: normalize_preferred_class(preferred_class)?,
        dedicated_required: dedicated_required.unwrap_or(false),
        sensitive_route: sensitive_route.unwrap_or(false),
    })
}

fn device_limit_for_tier(tier: &str) -> i64 {
    match tier {
        "escudo" => 5,
        "pro" => 10,
        "dedicated" => 10,
        _ => 1,
    }
}

async fn update_user_login_telemetry(
    state: &AppState,
    user_id: Uuid,
    telemetry: &ClientTelemetry,
) -> Result<(), EscudoError> {
    sqlx::query(
        "UPDATE users
         SET latest_login_ip = COALESCE(CAST($2 AS inet), latest_login_ip),
             latest_login_country = COALESCE($3, latest_login_country),
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(user_id)
    .bind(telemetry.ip.map(|ip| ip.to_string()))
    .bind(normalize_country(
        telemetry
            .country_code
            .as_deref()
            .or(telemetry.country.as_deref()),
    ))
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn update_device_ops_metadata(
    state: &AppState,
    user_id: Uuid,
    device_id: Uuid,
    metadata: &DeviceOpsMetadata,
    telemetry: &ClientTelemetry,
) -> Result<(), EscudoError> {
    sqlx::query(
        "UPDATE devices
         SET device_install_id = COALESCE($2, device_install_id),
             platform = COALESCE($3, platform),
             current_active_sessions = 1,
             usage_bucket = $4,
             preferred_class = COALESCE($5, preferred_class),
             dedicated_required = $6,
             sensitive_route = $7,
             first_seen_at = COALESCE(first_seen_at, NOW()),
             last_seen_at = NOW(),
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(device_id)
    .bind(&metadata.device_install_id)
    .bind(&metadata.platform)
    .bind(&metadata.usage_bucket)
    .bind(&metadata.preferred_class)
    .bind(metadata.dedicated_required)
    .bind(metadata.sensitive_route)
    .execute(&state.db)
    .await?;

    update_user_login_telemetry(state, user_id, telemetry).await?;
    recompute_user_abuse_score(state, user_id).await?;
    Ok(())
}

async fn recompute_user_abuse_score(state: &AppState, user_id: Uuid) -> Result<(), EscudoError> {
    let tier = get_user_tier(state, user_id).await?;
    let device_limit = device_limit_for_tier(&tier);

    let row = sqlx::query(
        r#"
        WITH active_devices AS (
            SELECT COUNT(*)::BIGINT AS count
            FROM devices
            WHERE user_id = $1 AND is_active = true
        ),
        shared_install AS (
            SELECT COUNT(DISTINCT d2.user_id)::BIGINT AS count
            FROM devices d1
            JOIN devices d2
              ON d1.device_install_id IS NOT NULL
             AND d1.device_install_id <> ''
             AND d2.device_install_id = d1.device_install_id
             AND d2.user_id <> d1.user_id
            WHERE d1.user_id = $1
        ),
        geo_mismatch AS (
            SELECT CASE
                WHEN signup_country IS NOT NULL
                 AND latest_login_country IS NOT NULL
                 AND signup_country <> latest_login_country
                THEN 1 ELSE 0
            END AS mismatch
            FROM users
            WHERE id = $1
        )
        SELECT
            COALESCE((SELECT count FROM active_devices), 0) AS active_devices,
            COALESCE((SELECT count FROM shared_install), 0) AS shared_install_users,
            COALESCE((SELECT mismatch FROM geo_mismatch), 0) AS geo_mismatch
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    let active_devices: i64 = row.get("active_devices");
    let shared_install_users: i64 = row.get("shared_install_users");
    let geo_mismatch: i32 = row.get("geo_mismatch");

    let mut score = 0_i32;
    if active_devices > device_limit {
        score += ((active_devices - device_limit) as i32 * 10).min(40);
    }
    if shared_install_users > 0 {
        score += (shared_install_users as i32 * 35).min(70);
    }
    if geo_mismatch > 0 {
        score += 5;
    }
    score = score.min(100);

    sqlx::query("UPDATE users SET abuse_score = $2, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .bind(score)
        .execute(&state.db)
        .await?;

    Ok(())
}

async fn cleanup_failed_device_insert(state: &AppState, device_id: Uuid) {
    if let Err(e) = sqlx::query("DELETE FROM devices WHERE id = $1")
        .bind(device_id)
        .execute(&state.db)
        .await
    {
        tracing::error!("Failed to rollback device {device_id} after gateway error: {e}");
    }
}

fn require_gateway_addr(gateway_addr: Option<String>) -> Result<String, EscudoError> {
    gateway_addr
        .map(|addr| addr.trim().to_string())
        .filter(|addr| !addr.is_empty())
        .ok_or_else(|| EscudoError::Internal("Server gateway is not configured".into()))
}

async fn available_proxy_countries(
    state: &AppState,
    proxy_type: &str,
    user_id: Option<Uuid>,
) -> Result<HashSet<String>, EscudoError> {
    let rows: Vec<String> = if proxy_type == "dedicated" {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT UPPER(country)
            FROM proxy_ips
            WHERE status = 'healthy'
              AND proxy_type = 'dedicated'
              AND (assigned_user_id IS NULL OR assigned_user_id = $1)
            "#,
        )
        .bind(user_id)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT UPPER(country)
            FROM proxy_ips
            WHERE status = 'healthy'
              AND proxy_type = 'shared'
            "#,
        )
        .fetch_all(&state.db)
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|country| country.trim().to_uppercase())
        .filter(|country| !country.is_empty())
        .collect())
}

fn has_gateway(gateway_grpc_addr: &str) -> bool {
    let gateway_grpc_addr = gateway_grpc_addr.trim().to_ascii_lowercase();
    if gateway_grpc_addr.is_empty() {
        return false;
    }

    !gateway_grpc_addr.contains("127.0.0.1")
        && !gateway_grpc_addr.contains("localhost")
        && !gateway_grpc_addr.contains("0.0.0.0")
}

fn proxy_manager_ready_for_tier(tier: &str, server_ip: &str) -> bool {
    let _ = tier;
    let _ = server_ip;
    true
}

fn country_supported(country_code: Option<&str>, supported: &HashSet<String>) -> bool {
    country_code
        .map(|country| country.trim().to_uppercase())
        .filter(|country| !country.is_empty())
        .map(|country| supported.contains(&country))
        .unwrap_or(false)
}

fn server_supports_tier(
    tier: &str,
    server_ip: &str,
    gateway_grpc_addr: &str,
    country_code: Option<&str>,
    shared_countries: &HashSet<String>,
    dedicated_countries: &HashSet<String>,
) -> bool {
    if !has_gateway(gateway_grpc_addr) {
        return false;
    }

    if !proxy_manager_ready_for_tier(tier, server_ip) {
        return false;
    }

    match tier {
        "pro" => country_supported(country_code, shared_countries),
        "dedicated" => country_supported(country_code, dedicated_countries),
        _ => true,
    }
}

const CLIENT_DNS_SERVERS: &str = "1.1.1.1, 1.0.0.1";

fn dns_for_assigned_ip(_default_dns: &str, _assigned_ip: &str, _server_ip: &str) -> String {
    // The internal 10.0.0.1/10.0.64.1/10.0.128.1 resolvers are not yet
    // reliable across the current fleet. Serve public resolvers so Android
    // clients can resolve hostnames consistently while the in-tunnel DNS
    // plane is repaired.
    CLIENT_DNS_SERVERS.to_string()
}

fn resolve_proxy_country(
    state: &AppState,
    server_country_code: Option<String>,
) -> Result<String, EscudoError> {
    server_country_code
        .map(|c| c.trim().to_uppercase())
        .filter(|c| !c.is_empty())
        .or_else(|| {
            state
                .config
                .proxy
                .as_ref()
                .and_then(|p| p.default_country_code.clone())
                .map(|c| c.trim().to_uppercase())
                .filter(|c| !c.is_empty())
        })
        .ok_or_else(|| EscudoError::Internal("No proxy country configured for this server".into()))
}

async fn maybe_configure_proxy(
    state: &AppState,
    gateway: &mut crate::state::GatewayClient,
    server_id: Uuid,
    user_id: Uuid,
    tier: &str,
    server_country_code: Option<String>,
) -> Result<(), EscudoError> {
    if tier != "pro" && tier != "dedicated" {
        return Ok(());
    }

    let country = resolve_proxy_country(state, server_country_code)?;

    let credential = if tier == "dedicated" {
        if let Some((
            inventory_id,
            host,
            port,
            username,
            password,
            country_code,
            provider_proxy_id,
            external_ip,
        )) = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                i32,
                String,
                String,
                String,
                String,
                Option<String>,
            ),
        >(
            r#"
            WITH candidate AS (
                SELECT
                    id,
                    socks5_host,
                    socks5_port,
                    socks5_username,
                    socks5_password,
                    country,
                    provider_proxy_id,
                    external_ip
                FROM proxy_ips
                WHERE country = $1
                  AND proxy_type = 'dedicated'
                  AND status = 'healthy'
                  AND (assigned_user_id = $2 OR assigned_user_id IS NULL)
                ORDER BY
                    CASE WHEN assigned_user_id = $2 THEN 0 ELSE 1 END,
                    updated_at ASC,
                    created_at ASC
                LIMIT 1
                FOR UPDATE
            )
            UPDATE proxy_ips p
            SET assigned_user_id = COALESCE(p.assigned_user_id, $2),
                updated_at = NOW()
            FROM candidate c
            WHERE p.id = c.id
            RETURNING
                p.id,
                p.socks5_host,
                p.socks5_port,
                p.socks5_username,
                p.socks5_password,
                p.country,
                p.provider_proxy_id,
                p.external_ip
            "#,
        )
        .bind(&country)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            EscudoError::Internal(format!(
                "Failed to select dedicated proxy from inventory: {e}"
            ))
        })? {
            let _ = inventory_id;
            let _ = external_ip;
            let mut credential = escudo_proxy::credential::ProxyCredential::new(
                escudo_proxy::credential::ProviderKind::Iproyal,
                escudo_proxy::credential::ProxyType::Dedicated,
                country_code,
                host,
                u16::try_from(port).unwrap_or(32325),
                username,
                password,
                None,
            );
            credential.id = provider_proxy_id.parse().unwrap_or_else(|_| Uuid::new_v4());
            credential
        } else {
            return Err(EscudoError::Internal(format!(
                "No dedicated proxy inventory available for country {country}"
            )));
        }
    } else {
        if let Some((
            inventory_id,
            host,
            port,
            username,
            password,
            country_code,
            provider_proxy_id,
            external_ip,
        )) = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                i32,
                String,
                String,
                String,
                String,
                Option<String>,
            ),
        >(
            r#"
            SELECT
                p.id,
                p.socks5_host,
                p.socks5_port,
                p.socks5_username,
                p.socks5_password,
                p.country,
                p.provider_proxy_id,
                p.external_ip
            FROM server_proxy_assignments spa
            JOIN proxy_ips p ON p.id = spa.proxy_ip_id
            WHERE spa.server_id = $1
              AND spa.proxy_target = 'shared'
              AND p.status = 'healthy'
              AND p.proxy_type = 'shared'
            LIMIT 1
            "#,
        )
        .bind(server_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            EscudoError::Internal(format!(
                "Failed to load existing shared proxy assignment: {e}"
            ))
        })? {
            let _ = inventory_id;
            let _ = external_ip;
            let mut credential = escudo_proxy::credential::ProxyCredential::new(
                escudo_proxy::credential::ProviderKind::Iproyal,
                escudo_proxy::credential::ProxyType::Shared,
                country_code,
                host,
                u16::try_from(port).unwrap_or(32325),
                username,
                password,
                None,
            );
            credential.id = provider_proxy_id.parse().unwrap_or_else(|_| Uuid::new_v4());
            return finalize_proxy_assignment(
                state,
                gateway,
                server_id,
                credential,
                ProxyTarget::Shared as i32,
                "shared",
            )
            .await;
        }

        if let Some((
            inventory_id,
            host,
            port,
            username,
            password,
            country_code,
            provider_proxy_id,
            external_ip,
        )) = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                i32,
                String,
                String,
                String,
                String,
                Option<String>,
            ),
        >(
            r#"
            WITH candidate AS (
                SELECT
                    id,
                    socks5_host,
                    socks5_port,
                    socks5_username,
                    socks5_password,
                    country,
                    provider_proxy_id,
                    external_ip
                FROM proxy_ips
                WHERE country = $1
                  AND proxy_type = 'shared'
                  AND status = 'healthy'
                  AND external_ip IS NOT NULL
                ORDER BY
                    current_concurrent ASC,
                    updated_at ASC,
                    created_at ASC
                LIMIT 1
                FOR UPDATE
            )
            UPDATE proxy_ips p
            SET updated_at = NOW()
            FROM candidate c
            WHERE p.id = c.id
            RETURNING
                p.id,
                p.socks5_host,
                p.socks5_port,
                p.socks5_username,
                p.socks5_password,
                p.country,
                p.provider_proxy_id,
                p.external_ip
            "#,
        )
        .bind(&country)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            EscudoError::Internal(format!("Failed to select shared proxy from inventory: {e}"))
        })? {
            let _ = inventory_id;
            let _ = external_ip;
            let mut credential = escudo_proxy::credential::ProxyCredential::new(
                escudo_proxy::credential::ProviderKind::Iproyal,
                escudo_proxy::credential::ProxyType::Shared,
                country_code,
                host,
                u16::try_from(port).unwrap_or(32325),
                username,
                password,
                None,
            );
            credential.id = provider_proxy_id.parse().unwrap_or_else(|_| Uuid::new_v4());
            credential
        } else {
            let proxy_cfg = match state.config.proxy.as_ref() {
                Some(proxy_cfg) => proxy_cfg,
                None => {
                    let testing_open = state
                        .config
                        .testing
                        .as_ref()
                        .map(|t| t.open_server_access)
                        .unwrap_or(false);
                    if testing_open {
                        tracing::warn!(
                            "Proxy provider is not configured; skipping dynamic proxy assignment in testing mode for tier {tier}"
                        );
                        return Ok(());
                    }
                    return Err(EscudoError::Internal(format!(
                        "No shared proxy inventory available for country {country} and proxy provider is not configured"
                    )));
                }
            };
            let provider =
                IproyalClient::new(proxy_cfg.iproyal_api_token.clone()).map_err(|e| {
                    EscudoError::Internal(format!("Failed to initialize proxy provider: {e}"))
                })?;
            let pool = ProxyPool::new(provider);
            pool.acquire_shared(SharedProxyRequest {
                country: country.clone(),
                sticky_duration_mins: proxy_cfg.sticky_duration_mins,
            })
            .await
            .map_err(|e| EscudoError::Internal(format!("Failed to acquire shared proxy: {e}")))?
        }
    };
    let target = if tier == "dedicated" {
        ProxyTarget::DedicatedProxy as i32
    } else {
        ProxyTarget::Shared as i32
    };
    let proxy_target = if tier == "dedicated" {
        "dedicated"
    } else {
        "shared"
    };

    finalize_proxy_assignment(state, gateway, server_id, credential, target, proxy_target).await
}

async fn finalize_proxy_assignment(
    state: &AppState,
    gateway: &mut crate::state::GatewayClient,
    server_id: Uuid,
    credential: escudo_proxy::credential::ProxyCredential,
    target: i32,
    proxy_target: &str,
) -> Result<(), EscudoError> {
    let provider = match credential.provider {
        escudo_proxy::credential::ProviderKind::Iproyal => "iproyal",
        escudo_proxy::credential::ProviderKind::Proxycheap => "proxycheap",
    };
    let provider_proxy_id = credential.id.to_string();
    let proxy_type = match credential.proxy_type {
        escudo_proxy::credential::ProxyType::Shared => "shared",
        escudo_proxy::credential::ProxyType::Dedicated => "dedicated",
    };
    let proxy_ip_id: Uuid = match sqlx::query_scalar(
        r#"
        SELECT id
        FROM proxy_ips
        WHERE provider = $1 AND provider_proxy_id = $2
        LIMIT 1
        "#,
    )
    .bind(provider)
    .bind(&provider_proxy_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| EscudoError::Internal(format!("Failed to look up proxy credential: {e}")))?
    {
        Some(existing) => {
            sqlx::query(
                r#"
                UPDATE proxy_ips
                SET proxy_type = $3,
                    country = $4,
                    socks5_host = $5,
                    socks5_port = $6,
                    socks5_username = $7,
                    socks5_password = $8,
                    external_ip = $9,
                    status = 'healthy',
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(existing)
            .bind(provider)
            .bind(proxy_type)
            .bind(&credential.country)
            .bind(&credential.host)
            .bind(i32::from(credential.port))
            .bind(&credential.username)
            .bind(&credential.password)
            .bind(Option::<String>::None)
            .execute(&state.db)
            .await
            .map_err(|e| {
                EscudoError::Internal(format!("Failed to update proxy credential: {e}"))
            })?;
            existing
        }
        None => sqlx::query_scalar(
            r#"
                INSERT INTO proxy_ips (
                    provider, provider_proxy_id, proxy_type, country,
                    socks5_host, socks5_port, socks5_username, socks5_password,
                    external_ip, status
                ) VALUES (
                    $1, $2, $3, $4,
                    $5, $6, $7, $8,
                    $9, 'healthy'
                )
                RETURNING id
                "#,
        )
        .bind(provider)
        .bind(&provider_proxy_id)
        .bind(proxy_type)
        .bind(&credential.country)
        .bind(&credential.host)
        .bind(i32::from(credential.port))
        .bind(&credential.username)
        .bind(&credential.password)
        .bind(Option::<String>::None)
        .fetch_one(&state.db)
        .await
        .map_err(|e| EscudoError::Internal(format!("Failed to store proxy credential: {e}")))?,
    };
    sqlx::query(
        r#"
        INSERT INTO server_proxy_assignments (server_id, proxy_ip_id, proxy_target)
        VALUES ($1, $2, $3)
        ON CONFLICT (server_id, proxy_target) DO UPDATE SET
            proxy_ip_id = EXCLUDED.proxy_ip_id,
            assigned_at = NOW()
        "#,
    )
    .bind(server_id)
    .bind(proxy_ip_id)
    .bind(proxy_target)
    .execute(&state.db)
    .await
    .map_err(|e| EscudoError::Internal(format!("Failed to assign proxy to server: {e}")))?;

    gateway
        .update_proxy_credentials(UpdateProxyCredentialsRequest {
            socks5_host: credential.host,
            socks5_port: credential.port as u32,
            socks5_username: credential.username,
            socks5_password: credential.password,
            target,
        })
        .await
        .map_err(|e| EscudoError::Internal(format!("Failed to push proxy credentials: {e}")))?;

    Ok(())
}

/// Returns the (ip_start, ip_end) range for generate_series based on tier.
fn tier_ip_range(tier: &str) -> (i32, i32) {
    match tier {
        "pro" => (16385i32, 32766i32),       // 10.0.64.1 – 10.0.127.254 (wg1)
        "dedicated" => (32769i32, 49150i32), // 10.0.128.1 – 10.0.191.254 (wg2)
        _ => (2i32, 16382i32),               // 10.0.0.2 – 10.0.63.254 (wg0)
    }
}

fn normalized_wg_key(key: &str) -> String {
    let trimmed = key.trim();
    let remainder = trimmed.len() % 4;
    if remainder == 0 {
        trimmed.to_string()
    } else {
        format!("{trimmed}{}", "=".repeat(4 - remainder))
    }
}

async fn allocate_ip_and_insert_device(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
    device_name: &str,
    public_key: &str,
    preshared_key: &str,
    encrypted_key: &str,
    tier: &str,
) -> Result<(Uuid, String), EscudoError> {
    let mut tx =
        state.db.begin().await.map_err(|e| {
            EscudoError::Internal(format!("Failed to begin device transaction: {e}"))
        })?;

    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(10_001_i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| EscudoError::Internal(format!("Failed to acquire IP allocation lock: {e}")))?;

    let (ip_start, ip_end) = tier_ip_range(tier);

    let assigned_ip: Option<String> = sqlx::query_scalar(
        r#"
        SELECT host(ip) FROM (
            SELECT ('10.0.' || (n / 256) || '.' || (n % 256))::inet AS ip
            FROM generate_series($1, $2) AS n
        ) candidates
        WHERE host(ip) NOT IN (
            SELECT assigned_ip
            FROM devices
            WHERE assigned_ip IS NOT NULL
        )
        LIMIT 1
        "#,
    )
    .bind(ip_start)
    .bind(ip_end)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| EscudoError::Internal(format!("Failed to allocate IP: {e}")))?;

    let assigned_ip =
        assigned_ip.ok_or_else(|| EscudoError::Internal("No IPs available".into()))?;

    let device_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO devices (user_id, server_id, name, public_key, preshared_key, assigned_ip, private_key_encrypted)
           VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"#,
    )
    .bind(user_id)
    .bind(server_id)
    .bind(device_name)
    .bind(public_key)
    .bind(preshared_key)
    .bind(&assigned_ip)
    .bind(encrypted_key)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| EscudoError::Internal(format!("Failed to persist device: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| EscudoError::Internal(format!("Failed to commit device transaction: {e}")))?;

    Ok((device_id, assigned_ip))
}

/// Pick the correct WireGuard public key and port for the given tier.
/// Falls back to the legacy `public_key` / `endpoint_port` columns via COALESCE.
fn pick_server_wg(
    tier: &str,
    wg0_key: Option<String>,
    wg0_port: Option<i32>,
    wg1_key: Option<String>,
    wg1_port: Option<i32>,
    wg2_key: Option<String>,
    wg2_port: Option<i32>,
    fallback_key: String,
    fallback_port: i32,
) -> (String, i32) {
    match tier {
        "pro" => (
            wg1_key.unwrap_or(fallback_key),
            wg1_port.unwrap_or(fallback_port),
        ),
        "dedicated" => (
            wg2_key.unwrap_or(fallback_key),
            wg2_port.unwrap_or(fallback_port),
        ),
        _ => (
            wg0_key.unwrap_or(fallback_key),
            wg0_port.unwrap_or(fallback_port),
        ),
    }
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub id: Uuid,
    pub name: String,
    pub location: String,
    pub country_code: Option<String>,
    pub load_percent: f64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub city: Option<String>,
    pub country_name: Option<String>,
    pub is_virtual: bool,
    pub service_class: String,
}

struct ServerListRow {
    id: Uuid,
    name: String,
    location: String,
    country_code: Option<String>,
    capacity_max: i32,
    active_count: i64,
    public_ip: String,
    gateway_grpc_addr: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    city: Option<String>,
    country_name: Option<String>,
    is_virtual: bool,
    node_class: Option<String>,
    server_tier: Option<String>,
    lifecycle_state: String,
    health_score: i32,
    assigned_user_cap: i32,
}

struct ConnectServerRow {
    id: Uuid,
    public_ip: String,
    fallback_key: String,
    fallback_port: i32,
    gateway_grpc_addr: Option<String>,
    country_code: Option<String>,
    wg0_public_key: Option<String>,
    wg0_port: Option<i32>,
    wg1_public_key: Option<String>,
    wg1_port: Option<i32>,
    wg2_public_key: Option<String>,
    wg2_port: Option<i32>,
    lifecycle_state: String,
    health_score: i32,
    assigned_user_cap: i32,
    active_session_hard_cap: i32,
}

pub async fn list_servers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<ServerInfo>>> {
    let launch_controls = backend_control::fetch_launch_controls(&state.db).await?;
    let tier = get_user_tier(&state, auth.0.sub).await?;
    let locked_server_id = locked_server_for_user(&state, auth.0.sub).await?;
    let open_server_access = state
        .config
        .testing
        .as_ref()
        .map(|t| t.open_server_access)
        .unwrap_or(false);
    let shared_countries = available_proxy_countries(&state, "shared", None).await?;
    let dedicated_countries =
        available_proxy_countries(&state, "dedicated", Some(auth.0.sub)).await?;

    let server_rows = if tier == "free" && !open_server_access {
        sqlx::query(
            r#"SELECT s.id, s.name, s.location, s.country_code, s.capacity_max, COUNT(d.id) as active_count,
                      s.public_ip,
                      COALESCE(s.gateway_grpc_addr, '') as gateway_grpc_addr,
                      s.latitude, s.longitude, s.city, s.country_name,
                      COALESCE(s.is_virtual, false) as is_virtual,
                      ps.node_class,
                      ps.tier,
                      s.lifecycle_state,
                      s.health_score,
                      s.assigned_user_cap
               FROM servers s
               LEFT JOIN provider_servers ps ON ps.server_id = s.id
               LEFT JOIN devices d ON d.server_id = s.id AND d.is_active = true
               WHERE s.is_active = true
                 AND s.gateway_grpc_addr IS NOT NULL
                 AND s.gateway_grpc_addr <> ''
                 AND (s.country_code IS NULL OR s.country_code IN ('BR', 'US', 'DE'))
               GROUP BY s.id, s.name, s.location, s.country_code, s.capacity_max, s.public_ip, s.gateway_grpc_addr,
                        s.latitude, s.longitude, s.city, s.country_name, s.is_virtual, ps.node_class, ps.tier,
                        s.lifecycle_state, s.health_score, s.assigned_user_cap"#,
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query(
            r#"SELECT s.id, s.name, s.location, s.country_code, s.capacity_max, COUNT(d.id) as active_count,
                      s.public_ip,
                      COALESCE(s.gateway_grpc_addr, '') as gateway_grpc_addr,
                      s.latitude, s.longitude, s.city, s.country_name,
                      COALESCE(s.is_virtual, false) as is_virtual,
                      ps.node_class,
                      ps.tier,
                      s.lifecycle_state,
                      s.health_score,
                      s.assigned_user_cap
               FROM servers s
               LEFT JOIN provider_servers ps ON ps.server_id = s.id
               LEFT JOIN devices d ON d.server_id = s.id AND d.is_active = true
               WHERE s.is_active = true
                 AND s.gateway_grpc_addr IS NOT NULL
                 AND s.gateway_grpc_addr <> ''
               GROUP BY s.id, s.name, s.location, s.country_code, s.capacity_max, s.public_ip, s.gateway_grpc_addr,
                        s.latitude, s.longitude, s.city, s.country_name, s.is_virtual, ps.node_class, ps.tier,
                        s.lifecycle_state, s.health_score, s.assigned_user_cap"#,
        )
        .fetch_all(&state.db)
        .await?
    };

    let servers: Vec<ServerListRow> = server_rows
        .into_iter()
        .map(|row| ServerListRow {
            id: row.get("id"),
            name: row.get("name"),
            location: row.get("location"),
            country_code: row.get("country_code"),
            capacity_max: row.get("capacity_max"),
            active_count: row.get("active_count"),
            public_ip: row.get("public_ip"),
            gateway_grpc_addr: row.get("gateway_grpc_addr"),
            latitude: row.get("latitude"),
            longitude: row.get("longitude"),
            city: row.get("city"),
            country_name: row.get("country_name"),
            is_virtual: row.get("is_virtual"),
            node_class: row.get("node_class"),
            server_tier: row.get("tier"),
            lifecycle_state: row.get("lifecycle_state"),
            health_score: row.get("health_score"),
            assigned_user_cap: row.get("assigned_user_cap"),
        })
        .collect();

    let result: Vec<ServerInfo> = servers
        .into_iter()
        .filter(|row| {
            locked_server_id
                .map(|locked| row.id == locked)
                .unwrap_or(true)
        })
        .filter(|row| {
            passes_launch_route_policy(
                launch_controls.healthy_only_routing,
                &row.lifecycle_state,
                row.health_score,
            )
                && server_supports_tier(
                    &tier,
                    &row.public_ip,
                    &row.gateway_grpc_addr,
                    row.country_code.as_deref(),
                    &shared_countries,
                    &dedicated_countries,
                )
        })
        .map(|row| {
            let (name, location) = if locked_server_id.is_some() {
                ("Brasil".to_string(), "Brasil".to_string())
            } else {
                (row.name, row.location)
            };
            let service_class = classify_service_class(
                &name,
                &location,
                row.node_class.as_deref(),
                row.server_tier.as_deref(),
            );
            ServerInfo {
                id: row.id,
                name,
                location,
                country_code: row.country_code,
                load_percent: (row.active_count as f64
                    / row.assigned_user_cap.max(row.capacity_max).max(1) as f64)
                    * 100.0,
                latitude: row.latitude,
                longitude: row.longitude,
                city: row.city,
                country_name: row.country_name,
                is_virtual: row.is_virtual,
                service_class,
            }
        })
        .collect();

    Ok(Json(result))
}

fn classify_service_class(
    name: &str,
    location: &str,
    node_class: Option<&str>,
    tier: Option<&str>,
) -> String {
    let name = name.to_lowercase();
    let location = location.to_lowercase();
    let node_class = node_class.unwrap_or_default().to_lowercase();
    let tier = tier.unwrap_or_default().to_lowercase();

    if tier == "pro"
        || tier == "dedicated"
        || node_class.contains("pro")
        || node_class.contains("dedicated")
    {
        return "Power".to_string();
    }

    if tier == "escudo"
        || node_class.contains("shared")
        || name.contains("shared")
        || location.contains("shared")
    {
        return "Medium".to_string();
    }

    "Free".to_string()
}

fn lifecycle_allows_new_assignments(state: &str) -> bool {
    !matches!(state, "blocked" | "draining" | "retiring" | "provisioning")
}

fn passes_launch_route_policy(
    healthy_only_routing: bool,
    lifecycle_state: &str,
    health_score: i32,
) -> bool {
    if !lifecycle_allows_new_assignments(lifecycle_state) {
        return false;
    }

    if healthy_only_routing {
        matches!(lifecycle_state, "healthy" | "warm") && health_score >= 70
    } else {
        health_score >= 50
    }
}

fn server_is_over_hard_cap(
    assigned_users: i64,
    active_sessions: i64,
    assigned_user_cap: i32,
    active_session_hard_cap: i32,
) -> bool {
    assigned_users >= i64::from(assigned_user_cap)
        || active_sessions >= i64::from(active_session_hard_cap)
}

fn compute_candidate_rank(
    health_score: i32,
    assigned_users: i64,
    active_sessions: i64,
    assigned_user_cap: i32,
    active_session_soft_cap: i32,
    routing_weight: f64,
) -> f64 {
    let assigned_ratio = if assigned_user_cap > 0 {
        assigned_users as f64 / assigned_user_cap as f64
    } else {
        1.0
    };
    let active_ratio = if active_session_soft_cap > 0 {
        active_sessions as f64 / active_session_soft_cap as f64
    } else {
        1.0
    };

    (health_score as f64 * routing_weight) - (assigned_ratio.max(active_ratio) * 100.0)
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub server_id: Option<Uuid>,
    pub device_name: Option<String>,
    pub device_install_id: Option<String>,
    pub platform: Option<String>,
    pub usage_bucket: Option<String>,
    pub preferred_class: Option<String>,
    pub dedicated_required: Option<bool>,
    pub sensitive_route: Option<bool>,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub device_id: Uuid,
    pub config: String,
    pub qr_code: String,
    pub public_ip: String,
}

pub async fn connect(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(req): Json<ConnectRequest>,
) -> escudo_common::Result<Json<ConnectResponse>> {
    let master_key = get_master_key(&state)?;
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    let launch_controls = backend_control::enforce_connect_allowed(&state.db).await?;
    let ops_metadata = normalize_device_metadata(
        req.device_install_id.clone(),
        req.platform.clone(),
        req.usage_bucket.clone(),
        req.preferred_class.clone(),
        req.dedicated_required,
        req.sensitive_route,
        &telemetry,
    )?;

    // Resolve tier before anything else so we can pick the right interface
    enforce_device_limit(&state, auth.0.sub).await?;
    let tier = get_user_tier(&state, auth.0.sub).await?;
    let locked_server_id = locked_server_for_user(&state, auth.0.sub).await?;
    let shared_countries = available_proxy_countries(&state, "shared", None).await?;
    let dedicated_countries =
        available_proxy_countries(&state, "dedicated", Some(auth.0.sub)).await?;

    // Get server; automatic selection must choose from routable, tier-eligible nodes.
    let requested_server_id = req.server_id.or(locked_server_id);
    let server_row: ConnectServerRow = if let Some(server_id) = requested_server_id {
        let selected = sqlx::query(
            "SELECT id, public_ip, COALESCE(wg0_public_key, public_key), COALESCE(wg0_port, endpoint_port),
                    gateway_grpc_addr,
                    country_code,
                    wg0_public_key, wg0_port,
                    wg1_public_key, wg1_port,
                    wg2_public_key, wg2_port,
                    lifecycle_state, health_score, assigned_user_cap, active_session_hard_cap
             FROM servers WHERE id = $1 AND is_active = true",
        )
        .bind(server_id)
        .fetch_optional(&state.db)
        .await?;

        selected.map(|row| ConnectServerRow {
            id: row.get("id"),
            public_ip: row.get("public_ip"),
            fallback_key: row.get(2),
            fallback_port: row.get(3),
            gateway_grpc_addr: row.get("gateway_grpc_addr"),
            country_code: row.get("country_code"),
            wg0_public_key: row.get("wg0_public_key"),
            wg0_port: row.get("wg0_port"),
            wg1_public_key: row.get("wg1_public_key"),
            wg1_port: row.get("wg1_port"),
            wg2_public_key: row.get("wg2_public_key"),
            wg2_port: row.get("wg2_port"),
            lifecycle_state: row.get("lifecycle_state"),
            health_score: row.get("health_score"),
            assigned_user_cap: row.get("assigned_user_cap"),
            active_session_hard_cap: row.get("active_session_hard_cap"),
        })
    } else {
        let candidate_rows = sqlx::query(
            "WITH latest_metrics AS (
                SELECT DISTINCT ON (nm.server_id)
                    nm.server_id,
                    nm.active_sessions
                FROM node_metrics nm
                ORDER BY nm.server_id, nm.collected_at DESC
             )
             SELECT s.id, s.public_ip, COALESCE(s.wg0_public_key, s.public_key), COALESCE(s.wg0_port, s.endpoint_port),
                    gateway_grpc_addr,
                    country_code,
                    s.wg0_public_key, s.wg0_port,
                    s.wg1_public_key, s.wg1_port,
                    s.wg2_public_key, s.wg2_port,
                    COUNT(d.id) AS active_count,
                    s.capacity_max,
                    s.lifecycle_state,
                    s.health_score,
                    s.assigned_user_cap,
                    s.active_session_soft_cap,
                    s.active_session_hard_cap,
                    s.routing_weight,
                    lm.active_sessions
             FROM servers s
             LEFT JOIN devices d ON d.server_id = s.id AND d.is_active = true
             LEFT JOIN latest_metrics lm ON lm.server_id = s.id
             WHERE s.is_active = true
             GROUP BY s.id, lm.active_sessions
             ORDER BY s.health_score DESC, s.routing_weight DESC, COUNT(d.id) ASC, s.capacity_max DESC, s.created_at DESC",
        )
        .fetch_all(&state.db)
        .await?;

        candidate_rows
            .into_iter()
            .filter_map(
                |row| {
                    let active_count: i64 = row.get("active_count");
                    let active_sessions = i64::from(row.get::<Option<i32>, _>("active_sessions").unwrap_or_default());
                    let lifecycle_state: String = row.get("lifecycle_state");
                    let health_score: i32 = row.get("health_score");
                    let assigned_user_cap: i32 = row.get("assigned_user_cap");
                    let active_session_soft_cap: i32 = row.get("active_session_soft_cap");
                    let active_session_hard_cap: i32 = row.get("active_session_hard_cap");
                    let routing_weight: f64 = row.get("routing_weight");
                    let public_ip: String = row.get("public_ip");
                    let gateway_grpc_addr: Option<String> = row.get("gateway_grpc_addr");
                    let country_code: Option<String> = row.get("country_code");
                    if passes_launch_route_policy(
                            launch_controls.healthy_only_routing,
                            &lifecycle_state,
                            health_score,
                        )
                        && !server_is_over_hard_cap(
                            active_count,
                            active_sessions,
                            assigned_user_cap,
                            active_session_hard_cap,
                        )
                        && server_supports_tier(
                        &tier,
                        public_ip.as_str(),
                        gateway_grpc_addr.as_deref().unwrap_or_default(),
                        country_code.as_deref(),
                        &shared_countries,
                        &dedicated_countries,
                    ) {
                        Some((
                            compute_candidate_rank(
                                health_score,
                                active_count,
                                active_sessions,
                                assigned_user_cap,
                                active_session_soft_cap,
                                routing_weight,
                            ),
                            ConnectServerRow {
                                id: row.get("id"),
                                public_ip,
                                fallback_key: row.get(2),
                                fallback_port: row.get(3),
                                gateway_grpc_addr,
                                country_code,
                                wg0_public_key: row.get("wg0_public_key"),
                                wg0_port: row.get("wg0_port"),
                                wg1_public_key: row.get("wg1_public_key"),
                                wg1_port: row.get("wg1_port"),
                                wg2_public_key: row.get("wg2_public_key"),
                                wg2_port: row.get("wg2_port"),
                                lifecycle_state,
                                health_score,
                                assigned_user_cap,
                                active_session_hard_cap,
                            },
                        ))
                    } else {
                        None
                    }
                },
            )
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, candidate)| candidate)
    }
    .ok_or_else(|| EscudoError::NotFound("No available server".into()))?;

    if let Some(locked) = locked_server_id {
        if server_row.id != locked {
            return Err(EscudoError::Forbidden(
                "This account is restricted to a dedicated Brazil route.".into(),
            ));
        }
    }

    let ConnectServerRow {
        id: server_id,
        public_ip: server_ip,
        fallback_key,
        fallback_port,
        gateway_grpc_addr: gateway_addr,
        country_code: server_country_code,
        wg0_public_key: wg0_key,
        wg0_port,
        wg1_public_key: wg1_key,
        wg1_port,
        wg2_public_key: wg2_key,
        wg2_port,
        lifecycle_state,
        health_score,
        assigned_user_cap,
        active_session_hard_cap,
    } = server_row;
    let requested_server_active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM devices WHERE server_id = $1 AND is_active = true",
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await?;
    if !passes_launch_route_policy(
        launch_controls.healthy_only_routing,
        &lifecycle_state,
        health_score,
    )
        || server_is_over_hard_cap(
            requested_server_active_count,
            requested_server_active_count,
            assigned_user_cap,
            active_session_hard_cap,
        )
    {
        return Err(EscudoError::BadRequest(
            "Selected server is not currently accepting new sessions.".into(),
        ));
    }
    if !server_supports_tier(
        &tier,
        &server_ip,
        gateway_addr.as_deref().unwrap_or_default(),
        server_country_code.as_deref(),
        &shared_countries,
        &dedicated_countries,
    ) {
        return Err(EscudoError::BadRequest(
            "Selected server is not ready for this plan yet.".into(),
        ));
    }
    let gateway_addr = require_gateway_addr(gateway_addr)?;

    let (server_pubkey, server_port) = pick_server_wg(
        &tier,
        wg0_key,
        wg0_port,
        wg1_key,
        wg1_port,
        wg2_key,
        wg2_port,
        fallback_key,
        fallback_port,
    );

    let tier_proto = tier_str_to_proto(&tier);

    // Generate keypair
    let keypair = generate_keypair();
    let psk = generate_preshared_key();

    let device_name = normalize_device_name(req.device_name, "default")?;

    // Encrypt private key before storing
    let encrypted_key = encrypt_private_key(&keypair.private_key, &master_key)
        .map_err(|e| EscudoError::Internal(format!("Encryption failed: {e}")))?;

    let (device_id, assigned_ip) = allocate_ip_and_insert_device(
        &state,
        auth.0.sub,
        server_id,
        &device_name,
        &keypair.public_key,
        &psk,
        &encrypted_key,
        &tier,
    )
    .await?;

    // Add peer to WireGuard via gRPC
    let mut gateway =
        match crate::state::gateway::gateway_service_client::GatewayServiceClient::connect(
            gateway_addr.clone(),
        )
        .await
        {
            Ok(gateway) => gateway,
            Err(e) => {
                cleanup_failed_device_insert(&state, device_id).await;
                return Err(EscudoError::Internal(format!(
                    "Gateway connection error: {e}"
                )));
            }
        };
    if let Err(e) = gateway
        .add_peer(AddPeerRequest {
            public_key: keypair.public_key.clone(),
            allowed_ip: assigned_ip.clone(),
            preshared_key: psk.clone(),
            tier: tier_proto,
        })
        .await
    {
        cleanup_failed_device_insert(&state, device_id).await;
        return Err(EscudoError::Internal(format!("Gateway error: {e}")));
    }

    if let Err(e) = maybe_configure_proxy(
        &state,
        &mut gateway,
        server_id,
        auth.0.sub,
        &tier,
        server_country_code,
    )
    .await
    {
        let _ = gateway
            .remove_peer(RemovePeerRequest {
                public_key: keypair.public_key.clone(),
            })
            .await;
        cleanup_failed_device_insert(&state, device_id).await;
        return Err(e);
    }

    update_device_ops_metadata(&state, auth.0.sub, device_id, &ops_metadata, &telemetry).await?;

    if let Err(error) = backend_control::open_vpn_session(
        &state.db,
        auth.0.sub,
        device_id,
        server_id,
        &tier,
        normalize_country(
            telemetry.country_code.as_deref().or(telemetry.country.as_deref()),
        ),
        json!({
            "mode": "standard",
            "server_ip": server_ip,
            "platform": ops_metadata.platform,
            "usage_bucket": ops_metadata.usage_bucket,
            "preferred_class": ops_metadata.preferred_class,
            "dedicated_required": ops_metadata.dedicated_required,
            "sensitive_route": ops_metadata.sensitive_route,
        }),
    )
    .await
    {
        tracing::warn!("Failed to open vpn session ledger for device {device_id}: {error}");
    }

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(auth.0.sub),
        Some(device_id),
        Some(server_id),
        "connect",
        "success",
        Some("Standard VPN connect completed".into()),
        json!({
            "mode": "standard",
            "tier": tier,
            "server_ip": server_ip,
            "platform": ops_metadata.platform,
            "usage_bucket": ops_metadata.usage_bucket,
        }),
    )
    .await
    {
        tracing::warn!("Failed to record connect journey event for device {device_id}: {error}");
    }

    // Build WireGuard config
    let dns_server = dns_for_assigned_ip(&state.config.wireguard.dns, &assigned_ip, &server_ip);
    let wg_config = format!(
        "[Interface]\n\
         PrivateKey = {}\n\
         Address = {}/32\n\
         DNS = {}\n\
         \n\
         [Peer]\n\
         PublicKey = {}\n\
         PresharedKey = {}\n\
         Endpoint = {}:{}\n\
         AllowedIPs = {}\n\
         PersistentKeepalive = 25\n",
        keypair.private_key,
        assigned_ip,
        dns_server,
        normalized_wg_key(&server_pubkey),
        psk,
        server_ip,
        server_port,
        state.config.wireguard.allowed_ips,
    );

    let qr_code = generate_qr_base64(&wg_config)
        .map_err(|e| EscudoError::Internal(format!("QR generation failed: {e}")))?;

    Ok(Json(ConnectResponse {
        device_id,
        config: wg_config,
        qr_code,
        public_ip: server_ip.clone(),
    }))
}

pub async fn disconnect(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(device_id): Path<Uuid>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let device = sqlx::query_as::<_, (String, Option<String>, Uuid)>(
        r#"SELECT d.public_key, s.gateway_grpc_addr, d.server_id
           FROM devices d
           JOIN servers s ON s.id = d.server_id
           WHERE d.id = $1 AND d.user_id = $2 AND d.is_active = true"#,
    )
    .bind(device_id)
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::NotFound("Device not found".into()))?;

    let (public_key, gateway_addr, server_id) = device;
    let gateway_addr = require_gateway_addr(gateway_addr)?;

    let mut gateway = crate::state::gateway::gateway_service_client::GatewayServiceClient::connect(
        gateway_addr.clone(),
    )
    .await
    .map_err(|e| EscudoError::Internal(format!("Gateway connection error: {e}")))?;
    gateway
        .remove_peer(RemovePeerRequest {
            public_key: public_key.clone(),
        })
        .await
        .map_err(|e| EscudoError::Internal(format!("Gateway error: {e}")))?;

    sqlx::query(
        "UPDATE devices
         SET is_active = false,
             current_active_sessions = 0,
             last_seen_at = NOW(),
             updated_at = NOW()
         WHERE id = $1",
    )
        .bind(device_id)
        .execute(&state.db)
        .await?;

    recompute_user_abuse_score(&state, auth.0.sub).await?;

    if let Err(error) =
        backend_control::close_vpn_session(&state.db, device_id, "user_disconnect").await
    {
        tracing::warn!("Failed to close vpn session ledger for device {device_id}: {error}");
    }

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(auth.0.sub),
        Some(device_id),
        Some(server_id),
        "disconnect",
        "success",
        Some("Device disconnected by user".into()),
        json!({ "reason": "user_disconnect" }),
    )
    .await
    {
        tracing::warn!("Failed to record disconnect journey event for device {device_id}: {error}");
    }

    Ok(Json(serde_json::json!({ "message": "Disconnected" })))
}

#[derive(Serialize)]
pub struct PeerInfo {
    pub id: Uuid,
    pub name: String,
    pub assigned_ip: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_peers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<PeerInfo>>> {
    let devices = sqlx::query_as::<_, (Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, assigned_ip, created_at FROM devices WHERE user_id = $1 AND is_active = true",
    )
    .bind(auth.0.sub)
    .fetch_all(&state.db)
    .await?;

    let peers: Vec<PeerInfo> = devices
        .into_iter()
        .map(|(id, name, assigned_ip, created_at)| PeerInfo {
            id,
            name,
            assigned_ip,
            created_at,
        })
        .collect();

    Ok(Json(peers))
}

pub async fn get_config_qr(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(device_id): Path<Uuid>,
) -> escudo_common::Result<Json<serde_json::Value>> {
    let master_key = get_master_key(&state)?;

    let device = sqlx::query_as::<_, (String, String, String, Uuid)>(
        "SELECT private_key_encrypted, assigned_ip, preshared_key, server_id FROM devices WHERE id = $1 AND user_id = $2 AND is_active = true",
    )
    .bind(device_id)
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::NotFound("Device not found".into()))?;

    let (encrypted_key, assigned_ip, psk, server_id) = device;

    // Decrypt private key
    let private_key = decrypt_private_key(&encrypted_key, &master_key)
        .map_err(|e| EscudoError::Internal(format!("Decryption failed: {e}")))?;

    let server = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT public_ip, COALESCE(wg0_public_key, public_key), COALESCE(wg0_port, endpoint_port) FROM servers WHERE id = $1",
    )
    .bind(server_id)
    .fetch_one(&state.db)
    .await?;

    let (server_ip, server_pubkey, server_port) = server;

    let dns_server = dns_for_assigned_ip(&state.config.wireguard.dns, &assigned_ip, &server_ip);
    let wg_config = format!(
        "[Interface]\n\
         PrivateKey = {}\n\
         Address = {}/32\n\
         DNS = {}\n\
         \n\
         [Peer]\n\
         PublicKey = {}\n\
         PresharedKey = {}\n\
         Endpoint = {}:{}\n\
         AllowedIPs = {}\n\
         PersistentKeepalive = 25\n",
        private_key,
        assigned_ip,
        dns_server,
        normalized_wg_key(&server_pubkey),
        psk,
        server_ip,
        server_port,
        state.config.wireguard.allowed_ips,
    );

    let qr_code = generate_qr_base64(&wg_config)
        .map_err(|e| EscudoError::Internal(format!("QR generation failed: {e}")))?;

    Ok(Json(serde_json::json!({
        "config": wg_config,
        "qr_code": qr_code,
    })))
}

#[derive(Serialize)]
pub struct UsageInfo {
    pub device_id: Uuid,
    pub device_name: String,
    pub total_rx_bytes: i64,
    pub total_tx_bytes: i64,
}

pub async fn get_usage(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<UsageInfo>>> {
    let usage = sqlx::query_as::<_, (Uuid, String, i64, i64)>(
        r#"SELECT d.id, d.name,
           COALESCE(SUM(u.rx_bytes), 0)::BIGINT as total_rx,
           COALESCE(SUM(u.tx_bytes), 0)::BIGINT as total_tx
           FROM devices d
           LEFT JOIN usage_logs u ON d.id = u.device_id
           WHERE d.user_id = $1 AND d.is_active = true
           GROUP BY d.id, d.name"#,
    )
    .bind(auth.0.sub)
    .fetch_all(&state.db)
    .await?;

    let result: Vec<UsageInfo> = usage
        .into_iter()
        .map(
            |(device_id, device_name, total_rx_bytes, total_tx_bytes)| UsageInfo {
                device_id,
                device_name,
                total_rx_bytes,
                total_tx_bytes,
            },
        )
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct PrivateModeRequest {
    pub device_name: Option<String>,
    pub device_install_id: Option<String>,
    pub platform: Option<String>,
    pub usage_bucket: Option<String>,
    pub preferred_class: Option<String>,
    pub dedicated_required: Option<bool>,
    pub sensitive_route: Option<bool>,
}

/// Brazil-related location substrings used to filter out domestic servers.
const BRAZIL_LOCATIONS: &[&str] = &[
    "São Paulo",
    "Sao Paulo",
    "Fortaleza",
    "Brasil",
    "Brazil",
    "Rio de Janeiro",
    "Brasília",
    "Brasilia",
    "Curitiba",
    "Salvador",
    "Recife",
    "Porto Alegre",
    "Belo Horizonte",
];

pub async fn connect_private_mode(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(req): Json<PrivateModeRequest>,
) -> escudo_common::Result<Json<ConnectResponse>> {
    let master_key = get_master_key(&state)?;
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    let launch_controls = backend_control::enforce_connect_allowed(&state.db).await?;
    let ops_metadata = normalize_device_metadata(
        req.device_install_id.clone(),
        req.platform.clone(),
        req.usage_bucket.clone(),
        req.preferred_class.clone(),
        req.dedicated_required.or(Some(false)),
        Some(true),
        &telemetry,
    )?;

    enforce_device_limit(&state, auth.0.sub).await?;
    let tier = get_user_tier(&state, auth.0.sub).await?;
    let shared_countries = available_proxy_countries(&state, "shared", None).await?;
    let dedicated_countries =
        available_proxy_countries(&state, "dedicated", Some(auth.0.sub)).await?;

    // Fetch all active servers with their location info and all wg key/port columns
    let servers = sqlx::query_as::<_, (Uuid, String, String, String, i32, Option<String>, Option<String>, Option<String>, Option<i32>, Option<String>, Option<i32>, Option<String>, Option<i32>)>(
        "SELECT id, public_ip, COALESCE(wg0_public_key, public_key), location, COALESCE(wg0_port, endpoint_port),
                gateway_grpc_addr,
                country_code,
                wg0_public_key, wg0_port,
                wg1_public_key, wg1_port,
                wg2_public_key, wg2_port
         FROM servers WHERE is_active = true",
    )
    .fetch_all(&state.db)
    .await?;

    if servers.is_empty() {
        return Err(EscudoError::NotFound("No available server".into()));
    }

    // Prefer an international (non-Brazil) server
    let international = servers.iter().find(
        |(_, server_ip, _, location, _, gateway_addr, country_code, _, _, _, _, _, _)| {
            let loc_lower = location.to_lowercase();
            !BRAZIL_LOCATIONS
                .iter()
                .any(|br| loc_lower.contains(&br.to_lowercase()))
                && server_supports_tier(
                    &tier,
                    server_ip,
                    gateway_addr.as_deref().unwrap_or_default(),
                    country_code.as_deref(),
                    &shared_countries,
                    &dedicated_countries,
                )
        },
    );

    let chosen = international
        .or_else(|| {
            servers.iter().find(
                |(_, server_ip, _, _, _, gateway_addr, country_code, _, _, _, _, _, _)| {
                    server_supports_tier(
                        &tier,
                        server_ip,
                        gateway_addr.as_deref().unwrap_or_default(),
                        country_code.as_deref(),
                        &shared_countries,
                        &dedicated_countries,
                    )
                },
            )
        })
        .cloned()
        .ok_or_else(|| EscudoError::NotFound("No available server for this plan".into()))?;
    let (
        server_id,
        server_ip,
        fallback_key,
        _,
        fallback_port,
        gateway_addr,
        server_country_code,
        wg0_key,
        wg0_port,
        wg1_key,
        wg1_port,
        wg2_key,
        wg2_port,
    ) = chosen;
    let selected_health: Option<(String, i32)> = sqlx::query_as(
        "SELECT lifecycle_state, health_score FROM servers WHERE id = $1",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;
    if let Some((lifecycle_state, health_score)) = selected_health {
        if !passes_launch_route_policy(
            launch_controls.healthy_only_routing,
            &lifecycle_state,
            health_score,
        ) {
            return Err(EscudoError::BadRequest(
                "Selected private-mode server is not currently accepting new sessions.".into(),
            ));
        }
    }
    let gateway_addr = require_gateway_addr(gateway_addr)?;

    let (server_pubkey, server_port) = pick_server_wg(
        &tier,
        wg0_key,
        wg0_port,
        wg1_key,
        wg1_port,
        wg2_key,
        wg2_port,
        fallback_key,
        fallback_port,
    );

    let tier_proto = tier_str_to_proto(&tier);

    // Generate keypair
    let keypair = generate_keypair();
    let psk = generate_preshared_key();

    let device_name = normalize_device_name(req.device_name, "private-mode")?;

    // Encrypt private key before storing
    let encrypted_key = encrypt_private_key(&keypair.private_key, &master_key)
        .map_err(|e| EscudoError::Internal(format!("Encryption failed: {e}")))?;

    let (device_id, assigned_ip) = allocate_ip_and_insert_device(
        &state,
        auth.0.sub,
        server_id,
        &device_name,
        &keypair.public_key,
        &psk,
        &encrypted_key,
        &tier,
    )
    .await?;

    // Add peer to WireGuard via gRPC
    let mut gateway =
        match crate::state::gateway::gateway_service_client::GatewayServiceClient::connect(
            gateway_addr.clone(),
        )
        .await
        {
            Ok(gateway) => gateway,
            Err(e) => {
                cleanup_failed_device_insert(&state, device_id).await;
                return Err(EscudoError::Internal(format!(
                    "Gateway connection error: {e}"
                )));
            }
        };
    if let Err(e) = gateway
        .add_peer(AddPeerRequest {
            public_key: keypair.public_key.clone(),
            allowed_ip: assigned_ip.clone(),
            preshared_key: psk.clone(),
            tier: tier_proto,
        })
        .await
    {
        cleanup_failed_device_insert(&state, device_id).await;
        return Err(EscudoError::Internal(format!("Gateway error: {e}")));
    }

    if let Err(e) = maybe_configure_proxy(
        &state,
        &mut gateway,
        server_id,
        auth.0.sub,
        &tier,
        server_country_code,
    )
    .await
    {
        let _ = gateway
            .remove_peer(RemovePeerRequest {
                public_key: keypair.public_key.clone(),
            })
            .await;
        cleanup_failed_device_insert(&state, device_id).await;
        return Err(e);
    }

    update_device_ops_metadata(&state, auth.0.sub, device_id, &ops_metadata, &telemetry).await?;

    if let Err(error) = backend_control::open_vpn_session(
        &state.db,
        auth.0.sub,
        device_id,
        server_id,
        &tier,
        normalize_country(
            telemetry.country_code.as_deref().or(telemetry.country.as_deref()),
        ),
        json!({
            "mode": "private_mode",
            "server_ip": server_ip,
            "platform": ops_metadata.platform,
            "usage_bucket": ops_metadata.usage_bucket,
            "preferred_class": ops_metadata.preferred_class,
        }),
    )
    .await
    {
        tracing::warn!("Failed to open private-mode session ledger for device {device_id}: {error}");
    }

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(auth.0.sub),
        Some(device_id),
        Some(server_id),
        "connect_private_mode",
        "success",
        Some("Private mode connect completed".into()),
        json!({
            "mode": "private_mode",
            "tier": tier,
            "server_ip": server_ip,
            "platform": ops_metadata.platform,
        }),
    )
    .await
    {
        tracing::warn!(
            "Failed to record private-mode journey event for device {device_id}: {error}"
        );
    }

    // Build WireGuard config
    let dns_server = dns_for_assigned_ip(&state.config.wireguard.dns, &assigned_ip, &server_ip);
    let wg_config = format!(
        "[Interface]\n\
         PrivateKey = {}\n\
         Address = {}/32\n\
         DNS = {}\n\
         \n\
         [Peer]\n\
         PublicKey = {}\n\
         PresharedKey = {}\n\
         Endpoint = {}:{}\n\
         AllowedIPs = {}\n\
         PersistentKeepalive = 25\n",
        keypair.private_key,
        assigned_ip,
        dns_server,
        server_pubkey,
        psk,
        server_ip,
        server_port,
        state.config.wireguard.allowed_ips,
    );

    let qr_code = generate_qr_base64(&wg_config)
        .map_err(|e| EscudoError::Internal(format!("QR generation failed: {e}")))?;

    Ok(Json(ConnectResponse {
        device_id,
        config: wg_config,
        qr_code,
        public_ip: server_ip.clone(),
    }))
}

#[derive(Deserialize)]
pub struct ConnectMultihopRequest {
    pub entry_server_id: Uuid,
    pub exit_server_id: Uuid,
    pub device_name: Option<String>,
    pub device_install_id: Option<String>,
    pub platform: Option<String>,
    pub usage_bucket: Option<String>,
    pub preferred_class: Option<String>,
    pub dedicated_required: Option<bool>,
    pub sensitive_route: Option<bool>,
}

pub async fn connect_multihop(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(req): Json<ConnectMultihopRequest>,
) -> escudo_common::Result<Json<ConnectResponse>> {
    let master_key = get_master_key(&state)?;
    let telemetry = resolve_request_telemetry(&headers, connect_info).await;
    let launch_controls = backend_control::enforce_connect_allowed(&state.db).await?;
    let ops_metadata = normalize_device_metadata(
        req.device_install_id.clone(),
        req.platform.clone(),
        req.usage_bucket.clone(),
        req.preferred_class.clone(),
        req.dedicated_required,
        req.sensitive_route.or(Some(true)),
        &telemetry,
    )?;

    if req.entry_server_id == req.exit_server_id {
        return Err(EscudoError::BadRequest(
            "Entry and exit servers must be different".into(),
        ));
    }

    // Get entry server
    let entry = sqlx::query_as::<_, (Uuid, String, String, i32, Option<String>)>(
        "SELECT id, public_ip, COALESCE(wg0_public_key, public_key), COALESCE(wg0_port, endpoint_port), gateway_grpc_addr FROM servers WHERE id = $1 AND is_active = true",
    )
    .bind(req.entry_server_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::NotFound("Entry server not found".into()))?;

    let (entry_id, entry_ip, entry_pubkey, entry_port, gateway_addr) = entry;
    let entry_health: Option<(String, i32)> = sqlx::query_as(
        "SELECT lifecycle_state, health_score FROM servers WHERE id = $1",
    )
    .bind(entry_id)
    .fetch_optional(&state.db)
    .await?;
    if let Some((lifecycle_state, health_score)) = entry_health {
        if !passes_launch_route_policy(
            launch_controls.healthy_only_routing,
            &lifecycle_state,
            health_score,
        ) {
            return Err(EscudoError::BadRequest(
                "Entry server is not currently accepting new sessions.".into(),
            ));
        }
    }
    let gateway_addr = require_gateway_addr(gateway_addr)?;

    // Get exit server
    let exit = sqlx::query_as::<_, (String, String, i32, Option<String>)>(
        "SELECT public_ip, COALESCE(wg0_public_key, public_key), COALESCE(wg0_port, endpoint_port), gateway_grpc_addr FROM servers WHERE id = $1 AND is_active = true",
    )
    .bind(req.exit_server_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::NotFound("Exit server not found".into()))?;

    let (exit_ip, exit_pubkey, exit_port, exit_gateway_addr) = exit;
    let exit_gateway_addr = require_gateway_addr(exit_gateway_addr)?;

    enforce_device_limit(&state, auth.0.sub).await?;
    let tier = get_user_tier(&state, auth.0.sub).await?;

    // Generate keypair
    let keypair = generate_keypair();
    let psk = generate_preshared_key();

    let device_name = normalize_device_name(req.device_name, "multihop")?;

    let encrypted_key = encrypt_private_key(&keypair.private_key, &master_key)
        .map_err(|e| EscudoError::Internal(format!("Encryption failed: {e}")))?;

    let (device_id, assigned_ip) = allocate_ip_and_insert_device(
        &state,
        auth.0.sub,
        entry_id,
        &device_name,
        &keypair.public_key,
        &psk,
        &encrypted_key,
        &tier,
    )
    .await?;

    let mut gateway =
        match crate::state::gateway::gateway_service_client::GatewayServiceClient::connect(
            gateway_addr.clone(),
        )
        .await
        {
            Ok(gateway) => gateway,
            Err(e) => {
                cleanup_failed_device_insert(&state, device_id).await;
                return Err(EscudoError::Internal(format!(
                    "Gateway connection error: {e}"
                )));
            }
        };
    let mut exit_gateway =
        match crate::state::gateway::gateway_service_client::GatewayServiceClient::connect(
            exit_gateway_addr.clone(),
        )
        .await
        {
            Ok(gateway) => gateway,
            Err(e) => {
                cleanup_failed_device_insert(&state, device_id).await;
                return Err(EscudoError::Internal(format!(
                    "Exit gateway connection error: {e}"
                )));
            }
        };

    if let Err(e) = exit_gateway
        .add_exit_peer(crate::state::gateway::AddExitPeerRequest {
            entry_server_public_key: entry_pubkey.clone(),
            allowed_ip: assigned_ip.clone(),
        })
        .await
    {
        cleanup_failed_device_insert(&state, device_id).await;
        return Err(EscudoError::Internal(format!("Exit gateway error: {e}")));
    }

    if let Err(e) = gateway
        .add_multihop_peer(AddMultihopPeerRequest {
            public_key: keypair.public_key.clone(),
            allowed_ip: assigned_ip.clone(),
            preshared_key: psk.clone(),
            exit_server_endpoint: format!("{exit_ip}:{exit_port}"),
            exit_server_public_key: exit_pubkey.clone(),
        })
        .await
    {
        cleanup_failed_device_insert(&state, device_id).await;
        return Err(EscudoError::Internal(format!("Gateway error: {e}")));
    }

    update_device_ops_metadata(&state, auth.0.sub, device_id, &ops_metadata, &telemetry).await?;

    if let Err(error) = backend_control::open_vpn_session(
        &state.db,
        auth.0.sub,
        device_id,
        entry_id,
        &tier,
        normalize_country(
            telemetry.country_code.as_deref().or(telemetry.country.as_deref()),
        ),
        json!({
            "mode": "multihop",
            "entry_ip": entry_ip,
            "exit_ip": exit_ip,
            "platform": ops_metadata.platform,
            "usage_bucket": ops_metadata.usage_bucket,
            "preferred_class": ops_metadata.preferred_class,
        }),
    )
    .await
    {
        tracing::warn!("Failed to open multihop session ledger for device {device_id}: {error}");
    }

    if let Err(error) = backend_control::record_journey_event(
        &state.db,
        Some(auth.0.sub),
        Some(device_id),
        Some(entry_id),
        "connect_multihop",
        "success",
        Some("Multihop connect completed".into()),
        json!({
            "mode": "multihop",
            "tier": tier,
            "entry_ip": entry_ip,
            "exit_ip": exit_ip,
            "platform": ops_metadata.platform,
        }),
    )
    .await
    {
        tracing::warn!("Failed to record multihop journey event for device {device_id}: {error}");
    }

    let dns_server = dns_for_assigned_ip(&state.config.wireguard.dns, &assigned_ip, &entry_ip);
    let wg_config = format!(
        "[Interface]\n\
         PrivateKey = {}\n\
         Address = {}/32\n\
         DNS = {}\n\
         \n\
         [Peer]\n\
         PublicKey = {}\n\
         PresharedKey = {}\n\
         Endpoint = {}:{}\n\
         AllowedIPs = {}\n\
         PersistentKeepalive = 25\n",
        keypair.private_key,
        assigned_ip,
        dns_server,
        normalized_wg_key(&entry_pubkey),
        psk,
        entry_ip,
        entry_port,
        state.config.wireguard.allowed_ips,
    );

    let qr_code = generate_qr_base64(&wg_config)
        .map_err(|e| EscudoError::Internal(format!("QR generation failed: {e}")))?;

    Ok(Json(ConnectResponse {
        device_id,
        config: wg_config,
        qr_code,
        public_ip: exit_ip.clone(),
    }))
}

#[derive(Serialize)]
pub struct RecentServer {
    pub server_id: Uuid,
    pub server_name: String,
    pub location: String,
    pub country_code: Option<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_recents(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<RecentServer>>> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT DISTINCT ON (ul.server_id)
            ul.server_id,
            s.name,
            s.location,
            s.country_code,
            ul.created_at
        FROM usage_logs ul
        JOIN servers s ON s.id = ul.server_id
        WHERE ul.user_id = $1
        ORDER BY ul.server_id, ul.created_at DESC
        "#,
    )
    .bind(auth.0.sub)
    .fetch_all(&state.db)
    .await?;

    let mut recents: Vec<RecentServer> = rows
        .into_iter()
        .map(
            |(server_id, server_name, location, country_code, connected_at)| RecentServer {
                server_id,
                server_name,
                location,
                country_code,
                connected_at,
            },
        )
        .collect();

    // Re-sort by most recent connection and limit to 10
    recents.sort_by(|a, b| b.connected_at.cmp(&a.connected_at));
    recents.truncate(10);

    Ok(Json(recents))
}
