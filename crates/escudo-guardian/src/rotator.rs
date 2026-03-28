use anyhow::{Context, Result};
use sqlx::PgPool;
use sqlx::Row;
use tracing::{info, warn};
use uuid::Uuid;

use escudo_proxy::pool::ProxyPool;
use escudo_proxy::provider::SharedProxyRequest;

/// Handles rotating burned/blocked proxy IPs to fresh ones.
pub struct IpRotator<'a> {
    db: PgPool,
    pool: &'a ProxyPool,
}

impl<'a> IpRotator<'a> {
    pub fn new(db: PgPool, pool: &'a ProxyPool) -> Self {
        Self { db, pool }
    }

    /// Rotate a blocked proxy IP:
    /// 1. Fetch the country for the blocked IP
    /// 2. Mark the IP as blocked in proxy_ips
    /// 3. Acquire a fresh IP via ProxyPool
    /// 4. Insert the new IP into proxy_ips
    /// 5. Update server_proxy_assignments to point at the new IP
    /// 6. Log the rotation in ip_rotation_logs
    pub async fn rotate_blocked_ip(&self, proxy_ip_id: Uuid, reason: &str) -> Result<Uuid> {
        // Step 1: Get country from proxy_ips using query_scalar.
        let country: String = sqlx::query_scalar("SELECT country FROM proxy_ips WHERE id = $1")
            .bind(proxy_ip_id)
            .fetch_one(&self.db)
            .await
            .with_context(|| format!("failed to fetch country for proxy_ip {proxy_ip_id}"))?;

        // Step 2: Mark the IP as blocked.
        sqlx::query("UPDATE proxy_ips SET status = 'blocked', updated_at = now() WHERE id = $1")
            .bind(proxy_ip_id)
            .execute(&self.db)
            .await
            .with_context(|| format!("failed to mark proxy_ip {proxy_ip_id} as blocked"))?;

        info!(
            proxy_ip_id = %proxy_ip_id,
            country = %country,
            reason = %reason,
            "proxy IP marked as blocked, acquiring replacement"
        );

        // Step 3: Acquire a fresh IP via ProxyPool.
        let request = SharedProxyRequest {
            country: country.clone(),
            sticky_duration_mins: Some(60),
        };

        let new_cred = self
            .pool
            .acquire_shared(request)
            .await
            .with_context(|| format!("failed to acquire replacement proxy for country {country}"))?;

        // Step 4: Insert the new IP into proxy_ips.
        let new_proxy_ip_id = Uuid::new_v4();
        let provider_str = format!("{:?}", new_cred.provider).to_lowercase();

        sqlx::query(
            r#"
            INSERT INTO proxy_ips (
                id, provider, provider_proxy_id, proxy_type, country,
                socks5_host, socks5_port, socks5_username, socks5_password,
                status, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9,
                'healthy', now(), now()
            )
            "#,
        )
        .bind(new_proxy_ip_id)
        .bind(&provider_str)
        .bind(new_cred.id.to_string())
        .bind("shared")
        .bind(&new_cred.country)
        .bind(&new_cred.host)
        .bind(new_cred.port as i32)
        .bind(&new_cred.username)
        .bind(&new_cred.password)
        .execute(&self.db)
        .await
        .with_context(|| "failed to insert new proxy_ip record")?;

        info!(
            new_proxy_ip_id = %new_proxy_ip_id,
            host = %new_cred.host,
            country = %new_cred.country,
            "new proxy IP inserted"
        );

        // Step 5: Update server_proxy_assignments to point to the new IP.
        let rows_updated = sqlx::query(
            "UPDATE server_proxy_assignments SET proxy_ip_id = $1 WHERE proxy_ip_id = $2",
        )
        .bind(new_proxy_ip_id)
        .bind(proxy_ip_id)
        .execute(&self.db)
        .await
        .with_context(|| "failed to update server_proxy_assignments")?
        .rows_affected();

        info!(
            affected_servers = rows_updated,
            old_proxy_ip_id = %proxy_ip_id,
            new_proxy_ip_id = %new_proxy_ip_id,
            "server_proxy_assignments updated"
        );

        // Step 6: Log the rotation in ip_rotation_logs.
        sqlx::query(
            r#"
            INSERT INTO ip_rotation_logs (
                old_proxy_ip_id, new_proxy_ip_id, reason, country, provider,
                affected_servers, rotated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, now())
            "#,
        )
        .bind(proxy_ip_id)
        .bind(new_proxy_ip_id)
        .bind(reason)
        .bind(&country)
        .bind(&provider_str)
        .bind(rows_updated as i32)
        .execute(&self.db)
        .await
        .with_context(|| "failed to insert ip_rotation_log")?;

        Ok(new_proxy_ip_id)
    }
}

/// Fetch the SOCKS5 URL for a proxy IP record.
pub async fn get_socks5_url(db: &PgPool, proxy_ip_id: Uuid) -> Result<String> {
    let row = sqlx::query(
        "SELECT socks5_username, socks5_password, socks5_host, socks5_port FROM proxy_ips WHERE id = $1",
    )
    .bind(proxy_ip_id)
    .fetch_one(db)
    .await
    .with_context(|| format!("failed to fetch socks5 info for proxy_ip {proxy_ip_id}"))?;

    let username: String = row.get("socks5_username");
    let password: String = row.get("socks5_password");
    let host: String = row.get("socks5_host");
    let port: i32 = row.get("socks5_port");

    Ok(format!("socks5://{username}:{password}@{host}:{port}"))
}
