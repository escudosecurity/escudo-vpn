use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use escudo_proxy::credential::{ProviderKind, ProxyCredential, ProxyType};
use escudo_proxy::pool::ProxyPool;
use escudo_proxy::provider::{DedicatedProxyRequest, SharedProxyRequest};
use escudo_proxy::providers::iproyal::IproyalClient;
use reqwest::Proxy;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{info, warn};
use uuid::Uuid;

const DATABASE_URL: &str = "postgresql://escudo:escudo_secret@localhost/escudo";

#[derive(Parser, Debug)]
#[command(
    name = "escudo-ip-manager",
    version,
    about = "Escudo VPN residential IP manager"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Buy {
        #[arg(long, default_value = "iproyal")]
        provider: String,
        #[arg(long)]
        country: String,
        #[arg(long = "type", value_enum)]
        proxy_type: ProxyKindArg,
        #[arg(long)]
        assign_to: String,
        #[arg(long, default_value_t = 60)]
        duration_mins: u64,
    },
    Test {
        #[arg(long)]
        ip_id: Option<Uuid>,
        #[arg(long)]
        all: bool,
    },
    Rotate {
        #[arg(long)]
        server: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProxyKindArg {
    Shared,
    Dedicated,
}

#[derive(Debug, Clone)]
struct ServerRecord {
    id: Uuid,
    name: String,
    public_ip: String,
}

#[derive(Debug, Clone)]
struct ProxyRecord {
    id: Uuid,
    country: String,
    proxy_type: String,
    socks5_host: String,
    socks5_port: i32,
    socks5_username: String,
    socks5_password: String,
}

#[derive(Debug, Serialize)]
struct StreamingCheck {
    service: String,
    status_code: Option<u16>,
    ok: bool,
}

#[derive(Debug, Serialize)]
struct ProxyTestReport {
    proxy_ip_id: Uuid,
    country: String,
    external_ip: Option<String>,
    checks: Vec<StreamingCheck>,
    overall_ok: bool,
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
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await
        .context("connecting to PostgreSQL")?;
    let iproyal = IproyalClient::new(
        std::env::var("IPROYAL_API_TOKEN").context("IPROYAL_API_TOKEN must be set")?,
    )?;
    let proxy_pool = ProxyPool::new(iproyal);

    match cli.command {
        Commands::Buy {
            provider,
            country,
            proxy_type,
            assign_to,
            duration_mins,
        } => {
            if provider != "iproyal" {
                bail!("only --provider iproyal is implemented in this phase");
            }
            let server = load_server(&pool, &assign_to).await?;
            let report = buy_and_assign(
                &pool,
                &proxy_pool,
                &server,
                &country,
                proxy_type,
                duration_mins,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Commands::Test { ip_id, all } => {
            let reports = if all {
                test_all(&pool).await?
            } else if let Some(ip_id) = ip_id {
                vec![test_by_id(&pool, ip_id).await?]
            } else {
                bail!("use --all or --ip-id");
            };
            println!("{}", serde_json::to_string_pretty(&reports)?);
        }
        Commands::Rotate { server } => {
            let report = rotate_server_proxy(&pool, &proxy_pool, &server).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

async fn buy_and_assign(
    pool: &sqlx::postgres::PgPool,
    proxy_pool: &ProxyPool,
    server: &ServerRecord,
    country: &str,
    proxy_type: ProxyKindArg,
    duration_mins: u64,
) -> Result<ProxyTestReport> {
    for attempt in 1..=3 {
        let credential = purchase_proxy(proxy_pool, country, proxy_type, duration_mins).await?;
        let proxy_id = insert_proxy(pool, &credential).await?;
        let report = test_proxy_credential(proxy_id, country, &credential).await?;
        if report.overall_ok {
            deploy_proxy_chain(server, &credential).await?;
            upsert_assignment(pool, server.id, proxy_id, proxy_type).await?;
            sqlx::query("UPDATE proxy_ips SET status = 'healthy', external_ip = $2, updated_at = now() WHERE id = $1")
                .bind(proxy_id)
                .bind(report.external_ip.clone())
                .execute(pool)
                .await
                .context("updating purchased proxy status")?;
            info!(server = %server.name, country = %country, attempt, "proxy assigned successfully");
            return Ok(report);
        }

        warn!(proxy_ip_id = %proxy_id, attempt, "streaming validation failed; marking proxy burned");
        sqlx::query("UPDATE proxy_ips SET status = 'burned', updated_at = now() WHERE id = $1")
            .bind(proxy_id)
            .execute(pool)
            .await
            .context("marking burned proxy")?;
    }

    bail!("failed to buy a healthy proxy for {country} after 3 attempts")
}

async fn purchase_proxy(
    proxy_pool: &ProxyPool,
    country: &str,
    proxy_type: ProxyKindArg,
    duration_mins: u64,
) -> Result<ProxyCredential> {
    match proxy_type {
        ProxyKindArg::Shared => {
            proxy_pool
                .acquire_shared(SharedProxyRequest {
                    country: country.to_string(),
                    sticky_duration_mins: Some(duration_mins),
                })
                .await
        }
        ProxyKindArg::Dedicated => {
            proxy_pool
                .acquire_dedicated(DedicatedProxyRequest {
                    country: country.to_string(),
                })
                .await
        }
    }
}

async fn insert_proxy(pool: &sqlx::postgres::PgPool, credential: &ProxyCredential) -> Result<Uuid> {
    let external_ip = resolve_external_ip(credential).await.ok();
    sqlx::query_scalar(
        r#"
        INSERT INTO proxy_ips (
            provider, provider_proxy_id, proxy_type, country,
            socks5_host, socks5_port, socks5_username, socks5_password,
            external_ip, status, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6, $7, $8,
            $9, 'provisioning', now(), now()
        )
        RETURNING id
        "#,
    )
    .bind(provider_name(credential.provider.clone()))
    .bind(credential.id.to_string())
    .bind(proxy_kind_name(credential.proxy_type.clone()))
    .bind(credential.country.to_uppercase())
    .bind(&credential.host)
    .bind(credential.port as i32)
    .bind(&credential.username)
    .bind(&credential.password)
    .bind(external_ip)
    .fetch_one(pool)
    .await
    .context("inserting proxy_ips row")
}

async fn load_server(pool: &sqlx::postgres::PgPool, server_name: &str) -> Result<ServerRecord> {
    let row =
        sqlx::query("SELECT id, name, public_ip FROM servers WHERE name = $1 OR public_ip = $1")
            .bind(server_name)
            .fetch_one(pool)
            .await
            .with_context(|| format!("server {server_name} was not found"))?;
    Ok(ServerRecord {
        id: row.get("id"),
        name: row.get("name"),
        public_ip: row.get("public_ip"),
    })
}

async fn deploy_proxy_chain(server: &ServerRecord, credential: &ProxyCredential) -> Result<()> {
    let env_blob = [
        format!("PROXY_HOST={}", credential.host),
        format!("PROXY_PORT={}", credential.port),
        format!("PROXY_USER={}", credential.username),
        format!("PROXY_PASS={}", credential.password),
        format!("PROXY_COUNTRY={}", credential.country),
        format!(
            "PROXY_TYPE={}",
            proxy_kind_name(credential.proxy_type.clone())
        ),
    ]
    .join("\n");

    let proxy_cfg = format!(
        "daemon\nmaxconn 128\nnserver 1.1.1.1\nnserver 8.8.8.8\nnscache 65536\ntimeouts 1 5 30 60 180 1800 15 60\nauth none\nallow *\nparent 1000 socks5 {} {} {} {}\nsocks -p1080 -i127.0.0.1\nflush\n",
        credential.host,
        credential.port,
        credential.username,
        credential.password
    );

    let remote = format!(
        "export DEBIAN_FRONTEND=noninteractive; \
         apt-get update -y >/dev/null && apt-get install -y 3proxy >/dev/null; \
         mkdir -p /etc/escudo /etc/3proxy; \
         cat > /etc/escudo/proxy-chain.env <<'EOF1'\n{}\nEOF1\n\
         cat > /etc/3proxy/3proxy.cfg <<'EOF2'\n{}\nEOF2\n\
         systemctl enable 3proxy >/dev/null 2>&1 || true; \
         systemctl restart 3proxy",
        env_blob, proxy_cfg
    );

    ssh_any_key(&server.public_ip, &remote).await?;
    Ok(())
}

async fn upsert_assignment(
    pool: &sqlx::postgres::PgPool,
    server_id: Uuid,
    proxy_ip_id: Uuid,
    proxy_type: ProxyKindArg,
) -> Result<()> {
    let target = match proxy_type {
        ProxyKindArg::Shared => "shared",
        ProxyKindArg::Dedicated => "dedicated",
    };

    sqlx::query(
        r#"
        INSERT INTO server_proxy_assignments (server_id, proxy_ip_id, proxy_target)
        VALUES ($1, $2, $3)
        ON CONFLICT (server_id, proxy_target)
        DO UPDATE SET proxy_ip_id = EXCLUDED.proxy_ip_id, assigned_at = now()
        "#,
    )
    .bind(server_id)
    .bind(proxy_ip_id)
    .bind(target)
    .execute(pool)
    .await
    .context("upserting server_proxy_assignments")?;
    Ok(())
}

async fn rotate_server_proxy(
    pool: &sqlx::postgres::PgPool,
    proxy_pool: &ProxyPool,
    server_name: &str,
) -> Result<ProxyTestReport> {
    let row = sqlx::query(
        r#"
        SELECT s.id AS server_id, s.name AS server_name, s.public_ip,
               p.id AS proxy_ip_id, p.country, p.proxy_type
        FROM server_proxy_assignments spa
        JOIN servers s ON s.id = spa.server_id
        JOIN proxy_ips p ON p.id = spa.proxy_ip_id
        WHERE s.name = $1
        ORDER BY spa.assigned_at DESC
        LIMIT 1
        "#,
    )
    .bind(server_name)
    .fetch_one(pool)
    .await
    .with_context(|| format!("no proxy assignment found for server {server_name}"))?;

    let server = ServerRecord {
        id: row.get("server_id"),
        name: row.get("server_name"),
        public_ip: row.get("public_ip"),
    };
    let old_proxy_id: Uuid = row.get("proxy_ip_id");
    let country: String = row.get("country");
    let proxy_type = match row.get::<String, _>("proxy_type").as_str() {
        "dedicated" => ProxyKindArg::Dedicated,
        _ => ProxyKindArg::Shared,
    };

    let report = buy_and_assign(pool, proxy_pool, &server, &country, proxy_type, 60).await?;
    sqlx::query("UPDATE proxy_ips SET status = 'rotated', updated_at = now() WHERE id = $1")
        .bind(old_proxy_id)
        .execute(pool)
        .await
        .context("marking rotated-out proxy")?;
    Ok(report)
}

async fn test_all(pool: &sqlx::postgres::PgPool) -> Result<Vec<ProxyTestReport>> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT p.id, p.country, p.proxy_type, p.socks5_host, p.socks5_port, p.socks5_username, p.socks5_password
        FROM proxy_ips p
        JOIN server_proxy_assignments spa ON spa.proxy_ip_id = p.id
        WHERE p.status IN ('healthy', 'degraded', 'provisioning')
        ORDER BY p.id
        "#,
    )
    .fetch_all(pool)
    .await
    .context("loading proxy records")?;

    let mut reports = Vec::with_capacity(rows.len());
    for row in rows {
        let record = ProxyRecord {
            id: row.get("id"),
            country: row.get("country"),
            proxy_type: row.get("proxy_type"),
            socks5_host: row.get("socks5_host"),
            socks5_port: row.get("socks5_port"),
            socks5_username: row.get("socks5_username"),
            socks5_password: row.get("socks5_password"),
        };
        reports.push(test_proxy_record(&record).await?);
    }
    Ok(reports)
}

async fn test_by_id(pool: &sqlx::postgres::PgPool, ip_id: Uuid) -> Result<ProxyTestReport> {
    let row = sqlx::query(
        "SELECT id, country, proxy_type, socks5_host, socks5_port, socks5_username, socks5_password FROM proxy_ips WHERE id = $1",
    )
    .bind(ip_id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("proxy IP {ip_id} not found"))?;
    let record = ProxyRecord {
        id: row.get("id"),
        country: row.get("country"),
        proxy_type: row.get("proxy_type"),
        socks5_host: row.get("socks5_host"),
        socks5_port: row.get("socks5_port"),
        socks5_username: row.get("socks5_username"),
        socks5_password: row.get("socks5_password"),
    };
    test_proxy_record(&record).await
}

async fn test_proxy_record(record: &ProxyRecord) -> Result<ProxyTestReport> {
    let credential = ProxyCredential {
        id: record.id,
        provider: ProviderKind::Iproyal,
        proxy_type: if record.proxy_type == "dedicated" {
            ProxyType::Dedicated
        } else {
            ProxyType::Shared
        },
        country: record.country.clone(),
        host: record.socks5_host.clone(),
        port: record.socks5_port as u16,
        username: record.socks5_username.clone(),
        password: record.socks5_password.clone(),
        issued_at: chrono::Utc::now(),
        expires_at: None,
    };
    test_proxy_credential(record.id, &record.country, &credential).await
}

async fn test_proxy_credential(
    proxy_ip_id: Uuid,
    country: &str,
    credential: &ProxyCredential,
) -> Result<ProxyTestReport> {
    let external_ip = resolve_external_ip(credential).await.ok();
    let services = service_matrix(country);
    let mut checks = Vec::new();
    for (name, url) in services {
        let (status_code, ok) = fetch_status_via_proxy(credential, url).await?;
        checks.push(StreamingCheck {
            service: name.to_string(),
            status_code,
            ok,
        });
    }
    let overall_ok = checks.iter().all(|check| check.ok);
    Ok(ProxyTestReport {
        proxy_ip_id,
        country: country.to_uppercase(),
        external_ip,
        checks,
        overall_ok,
    })
}

fn service_matrix(country: &str) -> Vec<(&'static str, &'static str)> {
    let mut services = vec![("netflix", "https://www.netflix.com")];
    match country.to_uppercase().as_str() {
        "BR" => services.push(("globoplay", "https://globoplay.globo.com")),
        "UK" | "GB" => services.push(("bbc", "https://www.bbc.co.uk/iplayer")),
        "US" => services.push(("peacock", "https://www.peacocktv.com")),
        _ => {}
    }
    services
}

async fn resolve_external_ip(credential: &ProxyCredential) -> Result<String> {
    let client = proxy_client(credential).await?;
    let response = client
        .get("https://ifconfig.me")
        .send()
        .await
        .context("requesting external IP")?
        .error_for_status()
        .context("external IP request failed")?;
    Ok(response.text().await?.trim().to_string())
}

async fn fetch_status_via_proxy(
    credential: &ProxyCredential,
    url: &str,
) -> Result<(Option<u16>, bool)> {
    let client = proxy_client(credential).await?;
    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(proxy_ip_id = %credential.id, url, %error, "streaming health request failed");
            return Ok((None, false));
        }
    };
    let status = response.status().as_u16();
    Ok((Some(status), response.status().is_success()))
}

async fn proxy_client(credential: &ProxyCredential) -> Result<reqwest::Client> {
    let proxy =
        Proxy::all(credential.socks5_url()).context("building SOCKS5 proxy configuration")?;
    reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(8))
        .build()
        .context("building proxied HTTP client")
}

async fn ssh_any_key(host: &str, remote_command: &str) -> Result<()> {
    for key in [
        "/root/.ssh/id_ed25519",
        "/root/.ssh/lightnode_rsa",
        "/home/dev/.ssh/id_ed25519",
        "/home/dev/.ssh/lightnode_rsa",
    ] {
        if std::path::Path::new(key).exists() {
            let status = Command::new("ssh")
                .args([
                    "-o",
                    "StrictHostKeyChecking=no",
                    "-o",
                    "ConnectTimeout=10",
                    "-i",
                    key,
                    &format!("root@{host}"),
                    remote_command,
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .with_context(|| format!("ssh to {host} with key {key}"))?;
            if status.success() {
                return Ok(());
            }
        }
    }
    bail!("failed to SSH to {host} with available fleet keys")
}

fn provider_name(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Iproyal => "iproyal",
        ProviderKind::Proxycheap => "proxycheap",
    }
}

fn proxy_kind_name(kind: ProxyType) -> &'static str {
    match kind {
        ProxyType::Shared => "shared",
        ProxyType::Dedicated => "dedicated",
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_matrix_has_regional_service() {
        assert_eq!(service_matrix("BR").len(), 2);
        assert_eq!(service_matrix("US").len(), 2);
        assert_eq!(service_matrix("DE").len(), 1);
    }

    #[test]
    fn shell_quote_wraps_value() {
        assert_eq!(shell_quote("abc"), "'abc'");
    }
}
