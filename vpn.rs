use axum::extract::{Path, State};
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
use std::collections::HashSet;
use uuid::Uuid;

use crate::middleware::AuthUser;
use crate::qr::generate_qr_base64;
use crate::state::gateway::{
    AddMultihopPeerRequest, AddPeerRequest, ProxyTarget, RemovePeerRequest, Tier,
    UpdateProxyCredentialsRequest,
};
use crate::state::AppState;

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
    let tier: String = sqlx::query_scalar(
        "SELECT COALESCE(
            (SELECT s.tier FROM subscriptions s WHERE s.user_id = $1 AND s.status = 'active' ORDER BY s.period_end DESC LIMIT 1),
            'free'
        )",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    Ok(tier)
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
    !gateway_grpc_addr.trim().is_empty()
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
            return Err(EscudoError::Internal(
                "Proxy provider is not configured".into(),
            ));
        }
    };

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
    let proxy_target = if tier == "dedicated" {
        "dedicated"
    } else {
        "shared"
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
}

pub async fn list_servers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> escudo_common::Result<Json<Vec<ServerInfo>>> {
    let tier = get_user_tier(&state, auth.0.sub).await?;
    let open_server_access = state
        .config
        .testing
        .as_ref()
        .map(|t| t.open_server_access)
        .unwrap_or(false);
    let shared_countries = available_proxy_countries(&state, "shared", None).await?;
    let dedicated_countries =
        available_proxy_countries(&state, "dedicated", Some(auth.0.sub)).await?;

    let servers = if tier == "free" && !open_server_access {
        sqlx::query_as::<_, (Uuid, String, String, Option<String>, i32, i64, String, String, Option<f64>, Option<f64>, Option<String>, Option<String>, bool)>(
            r#"SELECT s.id, s.name, s.location, s.country_code, s.capacity_max, COUNT(d.id) as active_count,
                      s.public_ip,
                      COALESCE(s.gateway_grpc_addr, '') as gateway_grpc_addr,
                      s.latitude, s.longitude, s.city, s.country_name,
                      COALESCE(s.is_virtual, false) as is_virtual
               FROM servers s
               LEFT JOIN devices d ON d.server_id = s.id AND d.is_active = true
               WHERE s.is_active = true
                 AND s.gateway_grpc_addr IS NOT NULL
                 AND s.gateway_grpc_addr <> ''
                 AND (s.country_code IS NULL OR s.country_code IN ('BR', 'US', 'DE'))
               GROUP BY s.id, s.name, s.location, s.country_code, s.capacity_max, s.public_ip, s.gateway_grpc_addr,
                        s.latitude, s.longitude, s.city, s.country_name, s.is_virtual"#,
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, (Uuid, String, String, Option<String>, i32, i64, String, String, Option<f64>, Option<f64>, Option<String>, Option<String>, bool)>(
            r#"SELECT s.id, s.name, s.location, s.country_code, s.capacity_max, COUNT(d.id) as active_count,
                      s.public_ip,
                      COALESCE(s.gateway_grpc_addr, '') as gateway_grpc_addr,
                      s.latitude, s.longitude, s.city, s.country_name,
                      COALESCE(s.is_virtual, false) as is_virtual
               FROM servers s
               LEFT JOIN devices d ON d.server_id = s.id AND d.is_active = true
               WHERE s.is_active = true
                 AND s.gateway_grpc_addr IS NOT NULL
                 AND s.gateway_grpc_addr <> ''
               GROUP BY s.id, s.name, s.location, s.country_code, s.capacity_max, s.public_ip, s.gateway_grpc_addr,
                        s.latitude, s.longitude, s.city, s.country_name, s.is_virtual"#,
        )
        .fetch_all(&state.db)
        .await?
    };

    let result: Vec<ServerInfo> = servers
        .into_iter()
        .filter(
            |(_, _, _, country_code, _, _, public_ip, gateway_grpc_addr, _, _, _, _, _)| {
                server_supports_tier(
                    &tier,
                    public_ip,
                    gateway_grpc_addr,
                    country_code.as_deref(),
                    &shared_countries,
                    &dedicated_countries,
                )
            },
        )
        .map(
            |(
                id,
                name,
                location,
                country_code,
                capacity,
                active_count,
                _public_ip,
                _gateway_grpc_addr,
                latitude,
                longitude,
                city,
                country_name,
                is_virtual,
            )| {
                ServerInfo {
                    id,
                    name,
                    location,
                    country_code,
                    load_percent: (active_count as f64 / capacity as f64) * 100.0,
                    latitude,
                    longitude,
                    city,
                    country_name,
                    is_virtual,
                }
            },
        )
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub server_id: Option<Uuid>,
    pub device_name: Option<String>,
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
    Json(req): Json<ConnectRequest>,
) -> escudo_common::Result<Json<ConnectResponse>> {
    let master_key = get_master_key(&state)?;

    // Resolve tier before anything else so we can pick the right interface
    enforce_device_limit(&state, auth.0.sub).await?;
    let tier = get_user_tier(&state, auth.0.sub).await?;
    let shared_countries = available_proxy_countries(&state, "shared", None).await?;
    let dedicated_countries =
        available_proxy_countries(&state, "dedicated", Some(auth.0.sub)).await?;

    // Get server (use first active if not specified); select all wg key/port columns
    let server_row = if let Some(server_id) = req.server_id {
        sqlx::query_as::<_, (Uuid, String, String, i32, Option<String>, Option<String>, Option<String>, Option<i32>, Option<String>, Option<i32>, Option<String>, Option<i32>)>(
            "SELECT id, public_ip, COALESCE(wg0_public_key, public_key), COALESCE(wg0_port, endpoint_port),
                    gateway_grpc_addr,
                    country_code,
                    wg0_public_key, wg0_port,
                    wg1_public_key, wg1_port,
                    wg2_public_key, wg2_port
             FROM servers WHERE id = $1 AND is_active = true",
        )
        .bind(server_id)
        .fetch_optional(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, (Uuid, String, String, i32, Option<String>, Option<String>, Option<String>, Option<i32>, Option<String>, Option<i32>, Option<String>, Option<i32>)>(
            "SELECT id, public_ip, COALESCE(wg0_public_key, public_key), COALESCE(wg0_port, endpoint_port),
                    gateway_grpc_addr,
                    country_code,
                    wg0_public_key, wg0_port,
                    wg1_public_key, wg1_port,
                    wg2_public_key, wg2_port
             FROM servers WHERE is_active = true LIMIT 1",
        )
        .fetch_optional(&state.db)
        .await?
    }
    .ok_or_else(|| EscudoError::NotFound("No available server".into()))?;

    let (
        server_id,
        server_ip,
        fallback_key,
        fallback_port,
        gateway_addr,
        server_country_code,
        wg0_key,
        wg0_port,
        wg1_key,
        wg1_port,
        wg2_key,
        wg2_port,
    ) = server_row;
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
    let device = sqlx::query_as::<_, (String, Option<String>)>(
        r#"SELECT d.public_key, s.gateway_grpc_addr
           FROM devices d
           JOIN servers s ON s.id = d.server_id
           WHERE d.id = $1 AND d.user_id = $2 AND d.is_active = true"#,
    )
    .bind(device_id)
    .bind(auth.0.sub)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::NotFound("Device not found".into()))?;

    let (public_key, gateway_addr) = device;
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

    sqlx::query("UPDATE devices SET is_active = false, updated_at = NOW() WHERE id = $1")
        .bind(device_id)
        .execute(&state.db)
        .await?;

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
    Json(req): Json<PrivateModeRequest>,
) -> escudo_common::Result<Json<ConnectResponse>> {
    let master_key = get_master_key(&state)?;

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
}

pub async fn connect_multihop(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ConnectMultihopRequest>,
) -> escudo_common::Result<Json<ConnectResponse>> {
    let master_key = get_master_key(&state)?;

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
    let gateway_addr = require_gateway_addr(gateway_addr)?;

    // Get exit server
    let exit = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT public_ip, COALESCE(wg0_public_key, public_key), COALESCE(wg0_port, endpoint_port) FROM servers WHERE id = $1 AND is_active = true",
    )
    .bind(req.exit_server_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| EscudoError::NotFound("Exit server not found".into()))?;

    let (exit_ip, exit_pubkey, exit_port) = exit;

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

    // Add multi-hop peer via gRPC
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
