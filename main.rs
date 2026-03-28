use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{info, warn};

const DATABASE_URL: &str = "postgresql://escudo:escudo_secret@localhost/escudo";
const MGMT_SERVER_IP: &str = "91.99.29.182";
const HETZNER_API: &str = "https://api.hetzner.cloud/v1";

#[derive(Parser, Debug)]
#[command(
    name = "escudo-provisioner",
    version,
    about = "Escudo VPN tunnel node provisioner"
)]
struct Cli {
    #[arg(long)]
    provider: String,
    #[arg(long)]
    location: String,
    #[arg(long)]
    name: String,
    #[arg(long = "type", default_value = "cx23")]
    server_type: String,
    #[arg(long, default_value_t = 51820)]
    endpoint_port: i32,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Clone)]
struct TunnelPlan {
    cidr: String,
    gateway: String,
    host_address: String,
    subnet_octet: u8,
}

#[derive(Debug, Deserialize)]
struct HetznerSshKeysResponse {
    ssh_keys: Vec<HetznerSshKey>,
}

#[derive(Debug, Deserialize)]
struct HetznerSshKey {
    id: u64,
    name: String,
}

#[derive(Debug, Serialize)]
struct CreateServerRequest {
    name: String,
    server_type: String,
    image: String,
    location: String,
    ssh_keys: Vec<u64>,
    labels: std::collections::BTreeMap<String, String>,
    start_after_create: bool,
}

#[derive(Debug, Deserialize)]
struct CreateServerResponse {
    server: HetznerServer,
}

#[derive(Debug, Deserialize)]
struct GetServerResponse {
    server: HetznerServer,
}

#[derive(Debug, Deserialize)]
struct HetznerServer {
    id: u64,
    name: String,
    status: String,
    public_net: HetznerPublicNet,
}

#[derive(Debug, Deserialize)]
struct HetznerPublicNet {
    ipv4: Option<HetznerIpv4>,
}

#[derive(Debug, Deserialize)]
struct HetznerIpv4 {
    ip: String,
}

#[derive(Debug, Serialize)]
struct ProvisionSummary {
    provider: String,
    name: String,
    location: String,
    server_type: String,
    public_ip: Option<String>,
    tunnel_cidr: String,
    tunnel_gateway: String,
    wg_port: i32,
    dry_run: bool,
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
    if cli.provider != "hetzner" {
        bail!("only --provider hetzner is implemented in this phase");
    }

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await
        .context("connecting to PostgreSQL")?;

    let plan = allocate_next_tunnel_subnet(&pool).await?;
    let location = normalize_hetzner_location(&cli.location)?;

    if cli.dry_run {
        let summary = ProvisionSummary {
            provider: cli.provider,
            name: cli.name,
            location,
            server_type: cli.server_type,
            public_ip: None,
            tunnel_cidr: plan.cidr,
            tunnel_gateway: plan.gateway,
            wg_port: cli.endpoint_port,
            dry_run: true,
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    let api_token = std::env::var("HETZNER_API_TOKEN")
        .context("HETZNER_API_TOKEN must be set for provisioning")?;
    let ssh_key_id = fetch_hetzner_ssh_key_id(&api_token, "escudo-deploy").await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("building Hetzner HTTP client")?;

    let server = create_hetzner_server(
        &client,
        &api_token,
        &cli.name,
        &location,
        &cli.server_type,
        ssh_key_id,
    )
    .await?;

    let public_ip = wait_for_hetzner_server_ready(&client, &api_token, server.id)
        .await?
        .public_net
        .ipv4
        .map(|ip| ip.ip)
        .ok_or_else(|| anyhow!("server did not report an IPv4 address"))?;

    wait_for_ssh(&public_ip).await?;
    run_remote_hardening(&public_ip, &cli.name, cli.endpoint_port, &plan).await?;
    let wg0_public_key = match fetch_remote_file(&public_ip, "/etc/wireguard/wg0.pubkey").await {
        Ok(value) => value,
        Err(_) => fetch_remote_file(&public_ip, "/etc/wireguard/publickey").await?,
    };
    let wg1_public_key = fetch_remote_file(&public_ip, "/etc/wireguard/wg1.pubkey").await?;
    let wg2_public_key = fetch_remote_file(&public_ip, "/etc/wireguard/wg2.pubkey").await?;

    register_server(
        &pool,
        &cli,
        &location,
        &public_ip,
        &wg0_public_key,
        &wg1_public_key,
        &wg2_public_key,
        &plan,
        server.id,
    )
    .await?;

    verify_wireguard(&public_ip).await?;

    let summary = ProvisionSummary {
        provider: "hetzner".to_string(),
        name: cli.name,
        location,
        server_type: cli.server_type,
        public_ip: Some(public_ip),
        tunnel_cidr: plan.cidr,
        tunnel_gateway: plan.gateway,
        wg_port: cli.endpoint_port,
        dry_run: false,
    };
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

async fn allocate_next_tunnel_subnet(pool: &sqlx::postgres::PgPool) -> Result<TunnelPlan> {
    let rows =
        sqlx::query("SELECT tunnel_ipv4_cidr FROM servers WHERE tunnel_ipv4_cidr IS NOT NULL")
            .fetch_all(pool)
            .await
            .context("querying existing tunnel subnets")?;

    let highest = rows
        .iter()
        .filter_map(|row| row.try_get::<String, _>("tunnel_ipv4_cidr").ok())
        .filter_map(|cidr| parse_subnet_octet(&cidr))
        .max()
        .unwrap_or(0);

    let next = highest
        .checked_add(1)
        .ok_or_else(|| anyhow!("subnet allocator overflow"))?;
    if next == 0 || next > 254 {
        bail!("no free 10.10.x.0/24 subnet remains");
    }

    Ok(TunnelPlan {
        cidr: format!("10.10.{next}.0/24"),
        gateway: format!("10.10.{next}.1"),
        host_address: format!("10.10.{next}.1/24"),
        subnet_octet: next,
    })
}

fn parse_subnet_octet(cidr: &str) -> Option<u8> {
    let third = cidr.split('.').nth(2)?;
    let octet = third.split('/').next().unwrap_or(third);
    octet.parse().ok()
}

fn normalize_hetzner_location(raw: &str) -> Result<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    let mapped = match normalized.as_str() {
        "ash" | "us" | "usa" | "mia" | "miami" => "ash",
        "fsn" | "fsn1" | "de" | "germany" => "fsn1",
        "nbg" | "nbg1" => "nbg1",
        "hel" | "hel1" | "fi" | "finland" => "hel1",
        other if ["ash", "fsn1", "nbg1", "hel1"].contains(&other) => other,
        other => bail!("unsupported Hetzner location mapping: {other}"),
    };
    Ok(mapped.to_string())
}

async fn fetch_hetzner_ssh_key_id(api_token: &str, key_name: &str) -> Result<u64> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("building HTTP client")?;
    let resp = client
        .get(format!("{HETZNER_API}/ssh_keys"))
        .bearer_auth(api_token)
        .send()
        .await
        .context("listing Hetzner SSH keys")?
        .error_for_status()
        .context("Hetzner SSH key list failed")?;

    let body: HetznerSshKeysResponse = resp.json().await.context("parsing SSH key list")?;
    body.ssh_keys
        .into_iter()
        .find(|key| key.name == key_name)
        .map(|key| key.id)
        .ok_or_else(|| anyhow!("Hetzner SSH key '{key_name}' was not found"))
}

async fn create_hetzner_server(
    client: &reqwest::Client,
    api_token: &str,
    name: &str,
    location: &str,
    server_type: &str,
    ssh_key_id: u64,
) -> Result<HetznerServer> {
    for candidate in [server_type] {
        let mut labels = std::collections::BTreeMap::new();
        labels.insert("managed-by".to_string(), "escudo-provisioner".to_string());
        labels.insert("role".to_string(), "tunnel".to_string());

        let payload = CreateServerRequest {
            name: name.to_string(),
            server_type: candidate.to_string(),
            image: "ubuntu-24.04".to_string(),
            location: location.to_string(),
            ssh_keys: vec![ssh_key_id],
            labels,
            start_after_create: true,
        };

        let resp = client
            .post(format!("{HETZNER_API}/servers"))
            .bearer_auth(api_token)
            .json(&payload)
            .send()
            .await
            .context("creating Hetzner server")?;

        if resp.status() == StatusCode::UNPROCESSABLE_ENTITY {
            let body = resp.text().await.unwrap_or_default();
            bail!("Hetzner rejected server creation: {body}");
        }

        let resp = resp.error_for_status().context("Hetzner create failed")?;
        let body: CreateServerResponse = resp.json().await.context("parsing create response")?;
        info!(server_id = body.server.id, server = %body.server.name, server_type = candidate, "server created");
        return Ok(body.server);
    }

    bail!("Hetzner server creation did not succeed for {server_type}")
}

async fn wait_for_hetzner_server_ready(
    client: &reqwest::Client,
    api_token: &str,
    server_id: u64,
) -> Result<HetznerServer> {
    for _ in 0..60 {
        let resp = client
            .get(format!("{HETZNER_API}/servers/{server_id}"))
            .bearer_auth(api_token)
            .send()
            .await
            .context("polling Hetzner server")?
            .error_for_status()
            .context("Hetzner polling failed")?;
        let body: GetServerResponse = resp.json().await.context("parsing poll response")?;
        let ready = body.server.status == "running" && body.server.public_net.ipv4.is_some();
        if ready {
            return Ok(body.server);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    bail!("server {server_id} did not become ready in time")
}

async fn wait_for_ssh(public_ip: &str) -> Result<()> {
    for _ in 0..60 {
        let status = Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=10",
                &format!("root@{public_ip}"),
                "true",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .context("probing SSH availability")?;
        if status.success() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    bail!("SSH did not become available on {public_ip}")
}

async fn run_remote_hardening(
    public_ip: &str,
    server_name: &str,
    wg_port: i32,
    plan: &TunnelPlan,
) -> Result<()> {
    let repo_root = PathBuf::from("/home/dev/pulsovpn/escudo-vpn");
    let script = repo_root.join("scripts/provision/harden-tunnel-node.sh");
    copy_file(public_ip, &script, "/tmp/harden-tunnel-node.sh").await?;

    let dns_binary = repo_root.join("target/release/escudo-dns");
    if dns_binary.exists() {
        copy_file(public_ip, &dns_binary, "/tmp/escudo-dns").await?;
    } else {
        warn!(
            "target/release/escudo-dns missing locally; continuing without escudo-dns deployment"
        );
    }

    let gateway_binary = repo_root.join("target/release/escudo-gateway");
    let fallback_gateway_binary = PathBuf::from("/opt/escudo/target/release/escudo-gateway");
    if gateway_binary.exists() {
        copy_file(public_ip, &gateway_binary, "/tmp/escudo-gateway").await?;
    } else if fallback_gateway_binary.exists() {
        copy_file(public_ip, &fallback_gateway_binary, "/tmp/escudo-gateway").await?;
    } else {
        warn!("escudo-gateway binary missing locally; continuing without gateway deployment");
    }

    let mgmt_pubkey = read_local_file("/home/dev/.ssh/id_ed25519.pub")
        .or_else(|| read_local_file("/root/.ssh/id_ed25519.pub"))
        .context("reading management SSH public key")?;

    run_ssh(
        public_ip,
        &format!(
            "chmod +x /tmp/harden-tunnel-node.sh && \
             SERVER_NAME={server_name} WG_PORT={wg_port} WG_ADDRESS={} WG_NETWORK_CIDR={} \
             MGMT_ENDPOINT={MGMT_SERVER_IP} MGMT_PUBLIC_KEY={} bash /tmp/harden-tunnel-node.sh",
            shell_quote(&plan.host_address),
            shell_quote(&plan.cidr),
            shell_quote(mgmt_pubkey.trim()),
        ),
    )
    .await?;

    Ok(())
}

async fn register_server(
    pool: &sqlx::postgres::PgPool,
    cli: &Cli,
    location: &str,
    public_ip: &str,
    wg0_public_key: &str,
    wg1_public_key: &str,
    wg2_public_key: &str,
    plan: &TunnelPlan,
    provider_server_id: u64,
) -> Result<()> {
    let mut tx = pool
        .begin()
        .await
        .context("starting registration transaction")?;

    let server_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO servers (
            name, location, public_ip, public_key, endpoint_port,
            gateway_grpc_addr, wg0_public_key, wg0_port,
            wg1_public_key, wg1_port, wg2_public_key, wg2_port,
            country_code, country_name, is_virtual,
            tunnel_ipv4_cidr, tunnel_ipv4_gateway
        ) VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8,
            $9, $10, $11, $12,
            $13, $14, true,
            $15, $16
        )
        RETURNING id
        "#,
    )
    .bind(&cli.name)
    .bind(location)
    .bind(public_ip)
    .bind(wg0_public_key.trim())
    .bind(51820)
    .bind(format!("http://{public_ip}:9090"))
    .bind(wg0_public_key.trim())
    .bind(51820)
    .bind(wg1_public_key.trim())
    .bind(51821)
    .bind(wg2_public_key.trim())
    .bind(51822)
    .bind(location_to_country_code(location))
    .bind(location_to_country_name(location))
    .bind(&plan.cidr)
    .bind(&plan.gateway)
    .fetch_one(&mut *tx)
    .await
    .context("inserting server row")?;

    sqlx::query(
        r#"
        INSERT INTO provider_servers (
            server_id, provider, provider_instance_id, label, region, plan, public_ip, status, monthly_cost_usd
        ) VALUES ($1, 'hetzner', $2, $3, $4, $5, $6, 'running', $7)
        "#,
    )
    .bind(server_id)
    .bind(provider_server_id.to_string())
    .bind(&cli.name)
    .bind(location)
    .bind(&cli.server_type)
    .bind(public_ip)
    .bind(server_plan_monthly_cost(&cli.server_type))
    .execute(&mut *tx)
    .await
    .context("inserting provider_servers row")?;

    tx.commit()
        .await
        .context("committing registration transaction")?;
    info!(server = %cli.name, public_ip = %public_ip, subnet = %plan.cidr, octet = plan.subnet_octet, "server registered in database");
    Ok(())
}

async fn verify_wireguard(public_ip: &str) -> Result<()> {
    run_ssh(public_ip, "wg show wg0 >/dev/null && wg show wg1 >/dev/null && wg show wg2 >/dev/null && systemctl is-active escudo-gateway >/dev/null").await
}

async fn copy_file(public_ip: &str, local_path: &Path, remote_path: &str) -> Result<()> {
    let status = Command::new("scp")
        .args([
            "-o",
            "StrictHostKeyChecking=no",
            local_path.to_string_lossy().as_ref(),
            &format!("root@{public_ip}:{remote_path}"),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .with_context(|| format!("copying {} to {}", local_path.display(), public_ip))?;
    if !status.success() {
        bail!("scp to {public_ip}:{remote_path} failed");
    }
    Ok(())
}

async fn fetch_remote_file(public_ip: &str, remote_path: &str) -> Result<String> {
    let output = Command::new("ssh")
        .args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            &format!("root@{public_ip}"),
            &format!("cat {remote_path}"),
        ])
        .output()
        .await
        .with_context(|| format!("fetching {remote_path} from {public_ip}"))?;
    if !output.status.success() {
        bail!("failed to fetch remote file {remote_path}");
    }
    Ok(String::from_utf8(output.stdout).context("remote file was not UTF-8")?)
}

async fn run_ssh(public_ip: &str, remote_command: &str) -> Result<()> {
    let status = Command::new("ssh")
        .args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            &format!("root@{public_ip}"),
            remote_command,
        ])
        .status()
        .await
        .with_context(|| format!("running remote command on {public_ip}"))?;
    if !status.success() {
        bail!("remote command failed on {public_ip}");
    }
    Ok(())
}

fn server_plan_monthly_cost(server_type: &str) -> f64 {
    match server_type {
        "cx23" => 3.59,
        "cx32" => 7.14,
        "cx33" => 5.99,
        _ => 0.0,
    }
}

fn location_to_country_code(location: &str) -> &'static str {
    match location {
        "ash" => "US",
        "hel1" => "FI",
        "nbg1" | "fsn1" => "DE",
        _ => "XX",
    }
}

fn location_to_country_name(location: &str) -> &'static str {
    match location {
        "ash" => "United States",
        "hel1" => "Finland",
        "nbg1" | "fsn1" => "Germany",
        _ => "Unknown",
    }
}

fn read_local_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_subnet_octet() {
        assert_eq!(parse_subnet_octet("10.10.17.0/24"), Some(17));
        assert_eq!(parse_subnet_octet("not-a-cidr"), None);
    }

    #[test]
    fn maps_locations() {
        assert_eq!(normalize_hetzner_location("ash").unwrap(), "ash");
        assert_eq!(normalize_hetzner_location("DE").unwrap(), "fsn1");
        assert!(normalize_hetzner_location("bogus").is_err());
    }
}
