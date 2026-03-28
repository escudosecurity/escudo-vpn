use anyhow::{Context, Result};
use axum::{extract::State, routing::get, Json, Router};
use clap::{Parser, Subcommand};
use reqwest::Proxy;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use uuid::Uuid;

const DATABASE_URL: &str = "postgresql://escudo:escudo_secret@localhost/escudo";
const DEFAULT_BIND: &str = "127.0.0.1:3011";

#[derive(Parser, Debug)]
#[command(name = "escudo-guardian", version, about = "Escudo VPN IP Guardian")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(long, default_value_t = 1800)]
    interval_secs: u64,
    #[arg(long, default_value_t = 300)]
    retest_delay_secs: u64,
    #[arg(long, default_value = DEFAULT_BIND)]
    bind: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    RunOnce,
    Serve,
}

#[derive(Clone)]
struct AppState {
    db: sqlx::postgres::PgPool,
}

#[derive(Debug, Clone)]
struct ManagedProxy {
    proxy_ip_id: Uuid,
    country: String,
    host: String,
    port: i32,
    username: String,
    password: String,
    server_name: String,
    server_ip: String,
}

#[derive(Debug, Serialize)]
struct ServiceProbe {
    service: String,
    status_code: Option<u16>,
    latency_ms: Option<i32>,
}

#[derive(Debug, Serialize)]
struct HealthSnapshot {
    proxy_ip_id: Uuid,
    server_name: String,
    server_ip: String,
    country: String,
    netflix_status: Option<u16>,
    regional_status: Option<u16>,
    regional_service: Option<String>,
    latency_ms: Option<i32>,
    status: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await
        .context("connecting to PostgreSQL")?;

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::RunOnce => {
            run_cycle(&db, cli.retest_delay_secs).await?;
        }
        Commands::Serve => {
            let state = AppState { db: db.clone() };
            let addr: SocketAddr = cli.bind.parse().context("parsing bind address")?;
            let app = Router::new()
                .route("/status", get(status_handler))
                .route("/api/v1/guardian/status", get(status_handler))
                .with_state(Arc::new(state));

            let mut tasks = JoinSet::new();
            let db_for_loop = db.clone();
            tasks.spawn(async move {
                loop {
                    if let Err(error) = run_cycle(&db_for_loop, cli.retest_delay_secs).await {
                        error!(%error, "guardian cycle failed");
                    }
                    tokio::time::sleep(Duration::from_secs(cli.interval_secs)).await;
                }
            });

            tasks.spawn(async move {
                info!(%addr, "starting guardian status server");
                let listener = tokio::net::TcpListener::bind(addr).await?;
                axum::serve(listener, app).await?;
                Result::<()>::Ok(())
            });

            while let Some(result) = tasks.join_next().await {
                result??;
            }
        }
    }

    Ok(())
}

async fn run_cycle(db: &sqlx::postgres::PgPool, retest_delay_secs: u64) -> Result<()> {
    let proxies = load_managed_proxies(db).await?;
    info!(count = proxies.len(), "guardian cycle started");

    for proxy in proxies {
        if proxy_is_burned(db, proxy.proxy_ip_id).await? {
            warn!(proxy_ip_id = %proxy.proxy_ip_id, server = %proxy.server_name, "proxy already marked burned; rotating immediately");
            mark_burned_and_rotate(db, &proxy).await?;
            continue;
        }

        let snapshot = evaluate_proxy(&proxy).await?;
        persist_snapshot(db, &proxy, &snapshot).await?;

        if snapshot.status == "degraded" {
            warn!(proxy_ip_id = %proxy.proxy_ip_id, "proxy degraded; scheduling retest");
            tokio::time::sleep(Duration::from_secs(retest_delay_secs)).await;
            let second = evaluate_proxy(&proxy).await?;
            persist_snapshot(db, &proxy, &second).await?;
            if second.status != "healthy" {
                mark_burned_and_rotate(db, &proxy).await?;
            }
        }
    }
    Ok(())
}

async fn proxy_is_burned(db: &sqlx::postgres::PgPool, proxy_ip_id: Uuid) -> Result<bool> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM proxy_ips WHERE id = $1")
        .bind(proxy_ip_id)
        .fetch_optional(db)
        .await
        .context("loading current proxy status")?;
    Ok(matches!(status.as_deref(), Some("burned")))
}

async fn load_managed_proxies(db: &sqlx::postgres::PgPool) -> Result<Vec<ManagedProxy>> {
    let rows = sqlx::query(
        r#"
        SELECT p.id,
               p.country,
               p.socks5_host,
               p.socks5_port,
               p.socks5_username,
               p.socks5_password,
               s.name AS server_name,
               s.public_ip AS server_ip
        FROM proxy_ips p
        JOIN server_proxy_assignments spa ON spa.proxy_ip_id = p.id
        JOIN servers s ON s.id = spa.server_id
        WHERE p.status IN ('healthy', 'degraded', 'burned')
        "#,
    )
    .fetch_all(db)
    .await
    .context("loading managed proxy assignments")?;

    Ok(rows
        .into_iter()
        .map(|row| ManagedProxy {
            proxy_ip_id: row.get("id"),
            country: row.get("country"),
            host: row.get("socks5_host"),
            port: row.get("socks5_port"),
            username: row.get("socks5_username"),
            password: row.get("socks5_password"),
            server_name: row.get("server_name"),
            server_ip: row.get("server_ip"),
        })
        .collect())
}

async fn evaluate_proxy(proxy: &ManagedProxy) -> Result<HealthSnapshot> {
    let proxy_url = format!(
        "socks5://{}:{}@{}:{}",
        proxy.username, proxy.password, proxy.host, proxy.port
    );
    let client = reqwest::Client::builder()
        .proxy(Proxy::all(&proxy_url).context("configuring SOCKS5 proxy")?)
        .timeout(Duration::from_secs(20))
        .build()
        .context("building guardian proxy client")?;

    let netflix = probe(&client, "https://www.netflix.com").await?;
    let regional = regional_target(&proxy.country);
    let regional_probe = if let Some((_, url)) = regional {
        Some(probe(&client, url).await?)
    } else {
        None
    };

    let all_ok = netflix.status_code.unwrap_or_default() == 200
        && regional_probe
            .as_ref()
            .map(|probe| probe.status_code.unwrap_or_default() == 200)
            .unwrap_or(true);

    Ok(HealthSnapshot {
        proxy_ip_id: proxy.proxy_ip_id,
        server_name: proxy.server_name.clone(),
        server_ip: proxy.server_ip.clone(),
        country: proxy.country.clone(),
        netflix_status: netflix.status_code,
        regional_status: regional_probe.as_ref().and_then(|probe| probe.status_code),
        regional_service: regional.map(|(name, _)| name.to_string()),
        latency_ms: netflix.latency_ms,
        status: if all_ok { "healthy" } else { "degraded" }.to_string(),
    })
}

async fn probe(client: &reqwest::Client, url: &str) -> Result<ServiceProbe> {
    let start = std::time::Instant::now();
    let response = client.get(url).send().await;
    match response {
        Ok(response) => Ok(ServiceProbe {
            service: url.to_string(),
            status_code: Some(response.status().as_u16()),
            latency_ms: Some(start.elapsed().as_millis() as i32),
        }),
        Err(_) => Ok(ServiceProbe {
            service: url.to_string(),
            status_code: None,
            latency_ms: None,
        }),
    }
}

fn regional_target(country: &str) -> Option<(&'static str, &'static str)> {
    match country.to_uppercase().as_str() {
        "BR" => Some(("globoplay", "https://globoplay.globo.com")),
        "GB" | "UK" => Some(("bbc", "https://www.bbc.co.uk/iplayer")),
        "US" => Some(("peacock", "https://www.peacocktv.com")),
        _ => None,
    }
}

async fn persist_snapshot(
    db: &sqlx::postgres::PgPool,
    proxy: &ManagedProxy,
    snapshot: &HealthSnapshot,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO ip_health_checks (
            proxy_ip_id, netflix_status, regional_status, regional_service, latency_ms, status
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(proxy.proxy_ip_id)
    .bind(snapshot.netflix_status.map(|value| value as i32))
    .bind(snapshot.regional_status.map(|value| value as i32))
    .bind(snapshot.regional_service.as_deref())
    .bind(snapshot.latency_ms)
    .bind(&snapshot.status)
    .execute(db)
    .await
    .context("inserting ip_health_checks row")?;

    sqlx::query(
        "UPDATE proxy_ips SET status = $2, last_health_check = now(), updated_at = now() WHERE id = $1",
    )
    .bind(proxy.proxy_ip_id)
    .bind(&snapshot.status)
    .execute(db)
    .await
    .context("updating proxy status")?;
    Ok(())
}

async fn mark_burned_and_rotate(db: &sqlx::postgres::PgPool, proxy: &ManagedProxy) -> Result<()> {
    sqlx::query("UPDATE proxy_ips SET status = 'burned', updated_at = now() WHERE id = $1")
        .bind(proxy.proxy_ip_id)
        .execute(db)
        .await
        .context("marking proxy burned")?;

    if !rotation_enabled() {
        warn!(
            server = %proxy.server_name,
            proxy_ip_id = %proxy.proxy_ip_id,
            "rotation skipped because ESCUDO_GUARDIAN_ALLOW_ROTATION is not enabled"
        );
        return Ok(());
    }

    let status = Command::new(ip_manager_binary())
        .args(["rotate", "--server", &proxy.server_name])
        .status()
        .await
        .context("spawning escudo-ip-manager rotate")?;
    if !status.success() {
        warn!(server = %proxy.server_name, "ip-manager rotate exited non-zero");
    }
    Ok(())
}

fn rotation_enabled() -> bool {
    matches!(
        std::env::var("ESCUDO_GUARDIAN_ALLOW_ROTATION")
            .ok()
            .as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn ip_manager_binary() -> String {
    std::env::var("ESCUDO_IP_MANAGER_BIN").unwrap_or_else(|_| {
        "/home/dev/pulsovpn/escudo-vpn/target/debug/escudo-ip-manager".to_string()
    })
}

async fn status_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<HealthSnapshot>>, axum::http::StatusCode> {
    guardian_status(&state.db)
        .await
        .map(Json)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

async fn guardian_status(db: &sqlx::postgres::PgPool) -> Result<Vec<HealthSnapshot>> {
    let rows = sqlx::query(
        r#"
        SELECT p.id AS proxy_ip_id,
               s.name AS server_name,
               s.public_ip AS server_ip,
               p.country,
               hc.netflix_status,
               hc.regional_status,
               hc.regional_service,
               hc.latency_ms,
               hc.status
        FROM proxy_ips p
        JOIN server_proxy_assignments spa ON spa.proxy_ip_id = p.id
        JOIN servers s ON s.id = spa.server_id
        LEFT JOIN LATERAL (
            SELECT netflix_status, regional_status, regional_service, latency_ms, status
            FROM ip_health_checks
            WHERE proxy_ip_id = p.id
            ORDER BY checked_at DESC
            LIMIT 1
        ) hc ON true
        ORDER BY s.name
        "#,
    )
    .fetch_all(db)
    .await
    .context("loading guardian status")?;

    Ok(rows
        .into_iter()
        .map(|row| HealthSnapshot {
            proxy_ip_id: row.get("proxy_ip_id"),
            server_name: row.get("server_name"),
            server_ip: row.get("server_ip"),
            country: row.get("country"),
            netflix_status: row
                .try_get::<Option<i32>, _>("netflix_status")
                .ok()
                .flatten()
                .map(|v| v as u16),
            regional_status: row
                .try_get::<Option<i32>, _>("regional_status")
                .ok()
                .flatten()
                .map(|v| v as u16),
            regional_service: row.try_get("regional_service").ok(),
            latency_ms: row.try_get("latency_ms").ok(),
            status: row
                .try_get::<Option<String>, _>("status")
                .ok()
                .flatten()
                .unwrap_or_else(|| "unknown".to_string()),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regional_target_matches_expected() {
        assert_eq!(regional_target("BR").unwrap().0, "globoplay");
        assert_eq!(regional_target("US").unwrap().0, "peacock");
        assert!(regional_target("DE").is_none());
    }
}
