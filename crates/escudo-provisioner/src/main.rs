use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::lookup_host;
use tokio::process::Command;
use tracing::{info, warn};
use uuid::Uuid;

const DATABASE_URL: &str = "postgresql://escudo:escudo_secret@localhost/escudo";
const MGMT_SERVER_IP: &str = "91.99.29.182";
const HETZNER_API: &str = "https://api.hetzner.cloud/v1";
const VULTR_API: &str = "https://api.vultr.com/v2";

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

#[derive(Debug, Clone)]
struct SshMaterial {
    private_key_path: PathBuf,
    public_key: String,
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

#[derive(Debug, Deserialize)]
struct VultrSshKeysResponse {
    ssh_keys: Vec<VultrSshKey>,
}

#[derive(Debug, Deserialize)]
struct VultrSshKey {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct VultrCreateInstanceRequest {
    region: String,
    plan: String,
    os_id: u32,
    label: String,
    hostname: String,
    sshkey_id: Vec<String>,
    enable_ipv6: bool,
    activation_email: bool,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct VultrCreateInstanceResponse {
    instance: VultrInstance,
}

#[derive(Debug, Deserialize)]
struct VultrGetInstanceResponse {
    instance: VultrInstance,
}

#[derive(Debug, Deserialize)]
struct VultrInstance {
    id: String,
    label: String,
    region: String,
    plan: String,
    main_ip: String,
    server_status: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct VultrCreateSshKeyRequest {
    name: String,
    ssh_key: String,
}

#[derive(Debug, Deserialize)]
struct VultrCreateSshKeyResponse {
    ssh_key: VultrSshKey,
}

#[derive(Debug, Serialize)]
struct StepTiming {
    step: String,
    seconds: f64,
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
    speed_mbps: Option<f64>,
    database_server_id: Option<Uuid>,
    step_timings: Vec<StepTiming>,
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

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await
        .context("connecting to PostgreSQL")?;

    let plan = allocate_next_tunnel_subnet(&pool).await?;
    let ssh = resolve_ssh_material().context("resolving management SSH key material")?;
    let timer = std::time::Instant::now();
    let mut timings = Vec::new();

    let location = match cli.provider.as_str() {
        "hetzner" => normalize_hetzner_location(&cli.location)?,
        "vultr" => normalize_vultr_location(&cli.location)?,
        other => bail!("unsupported provider: {other}"),
    };

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
            speed_mbps: None,
            database_server_id: None,
            step_timings: Vec::new(),
            dry_run: true,
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("building Hetzner HTTP client")?;

    let provider_server = match cli.provider.as_str() {
        "hetzner" => {
            let api_token = std::env::var("HETZNER_API_TOKEN")
                .context("HETZNER_API_TOKEN must be set for provisioning")?;
            let ssh_key_id = fetch_hetzner_ssh_key_id(&api_token, "escudo-deploy").await?;
            let server = create_hetzner_server(
                &client,
                &api_token,
                &cli.name,
                &location,
                &cli.server_type,
                ssh_key_id,
            )
            .await?;
            timings.push(step_done("provider_create", &timer));
            ProviderServer::Hetzner(
                wait_for_hetzner_server_ready(&client, &api_token, server.id).await?,
            )
        }
        "vultr" => {
            let api_token = std::env::var("VULTR_API_KEY")
                .or_else(|_| std::env::var("VULTR_API_TOKEN"))
                .context("VULTR_API_KEY or VULTR_API_TOKEN must be set for provisioning")?;
            let ssh_key_id =
                ensure_vultr_ssh_key(&client, &api_token, "escudo-deploy", &ssh).await?;
            let instance = create_vultr_server(
                &client,
                &api_token,
                &cli.name,
                &location,
                &cli.server_type,
                &ssh_key_id,
            )
            .await?;
            timings.push(step_done("provider_create", &timer));
            ProviderServer::Vultr(
                wait_for_vultr_server_ready(&client, &api_token, &instance.id).await?,
            )
        }
        _ => unreachable!(),
    };

    let public_ip = provider_server.public_ip()?;
    let provider_server_id = provider_server.provider_id();
    timings.push(step_done("provider_ready", &timer));

    wait_for_ssh(&public_ip, &ssh).await?;
    timings.push(step_done("ssh_ready", &timer));
    run_remote_hardening(&public_ip, &cli.name, cli.endpoint_port, &plan, &ssh).await?;
    timings.push(step_done("hardening", &timer));
    let wg0_public_key =
        match fetch_remote_file(&public_ip, "/etc/wireguard/wg0.pubkey", &ssh).await {
            Ok(value) => value,
            Err(_) => fetch_remote_file(&public_ip, "/etc/wireguard/publickey", &ssh).await?,
        };
    let wg1_public_key = fetch_remote_file(&public_ip, "/etc/wireguard/wg1.pubkey", &ssh).await?;
    let wg2_public_key = fetch_remote_file(&public_ip, "/etc/wireguard/wg2.pubkey", &ssh).await?;

    let database_server_id = register_server(
        &pool,
        &cli,
        &location,
        &public_ip,
        &wg0_public_key,
        &wg1_public_key,
        &wg2_public_key,
        &plan,
        &provider_server_id,
    )
    .await?;
    timings.push(step_done("database_register", &timer));

    verify_wireguard(&public_ip, &ssh).await?;
    timings.push(step_done("verify_wireguard", &timer));
    let speed_mbps = run_speed_test(&public_ip, &ssh).await.ok();
    timings.push(step_done("speed_test", &timer));

    let summary = ProvisionSummary {
        provider: cli.provider.clone(),
        name: cli.name,
        location,
        server_type: cli.server_type,
        public_ip: Some(public_ip),
        tunnel_cidr: plan.cidr,
        tunnel_gateway: plan.gateway,
        wg_port: cli.endpoint_port,
        speed_mbps,
        database_server_id: Some(database_server_id),
        step_timings: timings,
        dry_run: false,
    };
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

#[derive(Debug)]
enum ProviderServer {
    Hetzner(HetznerServer),
    Vultr(VultrInstance),
}

impl ProviderServer {
    fn public_ip(&self) -> Result<String> {
        match self {
            Self::Hetzner(server) => server
                .public_net
                .ipv4
                .as_ref()
                .map(|ip| ip.ip.clone())
                .ok_or_else(|| anyhow!("server did not report an IPv4 address")),
            Self::Vultr(instance) => {
                if instance.main_ip == "0.0.0.0" || instance.main_ip.is_empty() {
                    bail!("Vultr instance does not yet have a public IP")
                }
                Ok(instance.main_ip.clone())
            }
        }
    }

    fn provider_id(&self) -> String {
        match self {
            Self::Hetzner(server) => server.id.to_string(),
            Self::Vultr(instance) => instance.id.clone(),
        }
    }
}

fn step_done(name: &str, start: &std::time::Instant) -> StepTiming {
    StepTiming {
        step: name.to_string(),
        seconds: start.elapsed().as_secs_f64(),
    }
}

fn resolve_ssh_material() -> Result<SshMaterial> {
    let candidates = [
        ("/root/.ssh/id_ed25519", "/root/.ssh/id_ed25519.pub"),
        ("/home/dev/.ssh/id_ed25519", "/home/dev/.ssh/id_ed25519.pub"),
    ];

    for (private_key, public_key) in candidates {
        let private_path = PathBuf::from(private_key);
        let public_path = PathBuf::from(public_key);
        if private_path.exists() && public_path.exists() {
            return Ok(SshMaterial {
                private_key_path: private_path,
                public_key: std::fs::read_to_string(&public_path)
                    .with_context(|| format!("reading {}", public_path.display()))?,
            });
        }
    }

    bail!("no ed25519 SSH keypair found for management automation")
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

fn normalize_vultr_location(raw: &str) -> Result<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    let mapped = match normalized.as_str() {
        "gru" | "sao" | "sao-paulo" | "saopaulo" | "br" | "sao paulo" => "sao",
        "ewr" | "nj" | "newark" | "us" => "ewr",
        other => other,
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

async fn ensure_vultr_ssh_key(
    client: &reqwest::Client,
    api_token: &str,
    key_name: &str,
    ssh: &SshMaterial,
) -> Result<String> {
    let resp = client
        .get(format!("{VULTR_API}/ssh-keys"))
        .bearer_auth(api_token)
        .send()
        .await
        .context("listing Vultr SSH keys")?
        .error_for_status()
        .context("Vultr SSH key list failed")?;
    let body: VultrSshKeysResponse = resp.json().await.context("parsing Vultr SSH key list")?;
    if let Some(key) = body.ssh_keys.into_iter().find(|key| key.name == key_name) {
        return Ok(key.id);
    }

    let payload = VultrCreateSshKeyRequest {
        name: key_name.to_string(),
        ssh_key: ssh.public_key.trim().to_string(),
    };
    let resp = client
        .post(format!("{VULTR_API}/ssh-keys"))
        .bearer_auth(api_token)
        .json(&payload)
        .send()
        .await
        .context("creating Vultr SSH key")?
        .error_for_status()
        .context("Vultr SSH key create failed")?;
    let body: VultrCreateSshKeyResponse = resp
        .json()
        .await
        .context("parsing Vultr SSH key create response")?;
    Ok(body.ssh_key.id)
}

async fn create_vultr_server(
    client: &reqwest::Client,
    api_token: &str,
    name: &str,
    location: &str,
    server_type: &str,
    ssh_key_id: &str,
) -> Result<VultrInstance> {
    let payload = VultrCreateInstanceRequest {
        region: location.to_string(),
        plan: server_type.to_string(),
        os_id: 2284,
        label: name.to_string(),
        hostname: name.to_string(),
        sshkey_id: vec![ssh_key_id.to_string()],
        enable_ipv6: false,
        activation_email: false,
        tags: vec!["escudo-vpn".to_string(), "escudo-provisioner".to_string()],
    };

    let resp = client
        .post(format!("{VULTR_API}/instances"))
        .bearer_auth(api_token)
        .json(&payload)
        .send()
        .await
        .context("creating Vultr instance")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read response body>".to_string());
        bail!("Vultr create instance failed: {status} {body}");
    }
    let body: VultrCreateInstanceResponse = resp
        .json()
        .await
        .context("parsing Vultr create instance response")?;
    Ok(body.instance)
}

async fn wait_for_vultr_server_ready(
    client: &reqwest::Client,
    api_token: &str,
    instance_id: &str,
) -> Result<VultrInstance> {
    for _ in 0..60 {
        let resp = client
            .get(format!("{VULTR_API}/instances/{instance_id}"))
            .bearer_auth(api_token)
            .send()
            .await
            .context("polling Vultr instance")?
            .error_for_status()
            .context("Vultr polling failed")?;
        let body: VultrGetInstanceResponse =
            resp.json().await.context("parsing Vultr poll response")?;
        let instance = body.instance;
        let ready = instance.status == "active"
            && instance.server_status.eq_ignore_ascii_case("ok")
            && instance.main_ip != "0.0.0.0"
            && !instance.main_ip.is_empty();
        if ready {
            return Ok(instance);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    bail!("Vultr instance {instance_id} did not become ready in time")
}

async fn wait_for_ssh(public_ip: &str, ssh: &SshMaterial) -> Result<()> {
    for _ in 0..60 {
        let status = ssh_command(
            public_ip,
            ssh,
            "cloud-init status --wait >/dev/null 2>&1 || true; test -w /root",
        )
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
    ssh: &SshMaterial,
) -> Result<()> {
    let repo_root = resolve_repo_root();
    let script = repo_root.join("scripts/provision/harden-tunnel-node.sh");
    copy_file(public_ip, &script, "/tmp/harden-tunnel-node.sh", ssh).await?;

    let dns_binary = repo_root.join("target/release/escudo-dns");
    if dns_binary.exists() {
        copy_file(public_ip, &dns_binary, "/tmp/escudo-dns", ssh).await?;
    } else {
        warn!(
            "target/release/escudo-dns missing locally; continuing without escudo-dns deployment"
        );
    }

    let gateway_binary = repo_root.join("target/release/escudo-gateway");
    let fallback_gateway_binary = PathBuf::from("/opt/escudo/target/release/escudo-gateway");
    if gateway_binary.exists() {
        copy_file(public_ip, &gateway_binary, "/tmp/escudo-gateway", ssh).await?;
    } else if fallback_gateway_binary.exists() {
        copy_file(
            public_ip,
            &fallback_gateway_binary,
            "/tmp/escudo-gateway",
            ssh,
        )
        .await?;
    } else {
        warn!("escudo-gateway binary missing locally; continuing without gateway deployment");
    }

    let mgmt_pubkey = ssh.public_key.clone();

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
        ssh,
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
    provider_server_id: &str,
) -> Result<Uuid> {
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
    Ok(server_id)
}

async fn verify_wireguard(public_ip: &str, ssh: &SshMaterial) -> Result<()> {
    run_ssh(public_ip, "wg show wg0 >/dev/null && wg show wg1 >/dev/null && wg show wg2 >/dev/null && systemctl is-active nftables >/dev/null && systemctl is-active escudo-gateway >/dev/null", ssh).await
}

async fn copy_file(
    public_ip: &str,
    local_path: &Path,
    remote_path: &str,
    ssh: &SshMaterial,
) -> Result<()> {
    let bytes = tokio::fs::read(local_path)
        .await
        .with_context(|| format!("reading {}", local_path.display()))?;
    let mut child = ssh_command(public_ip, ssh, &format!("cat > {remote_path}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("copying {} to {}", local_path.display(), public_ip))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to open SSH stdin for file copy"))?;
    stdin
        .write_all(&bytes)
        .await
        .with_context(|| format!("streaming {} to {}", local_path.display(), public_ip))?;
    drop(stdin);

    let status = child.wait().await.with_context(|| {
        format!(
            "finishing file copy {} to {}",
            local_path.display(),
            public_ip
        )
    })?;
    if !status.success() {
        bail!("ssh file copy to {public_ip}:{remote_path} failed");
    }
    Ok(())
}

async fn fetch_remote_file(
    public_ip: &str,
    remote_path: &str,
    ssh: &SshMaterial,
) -> Result<String> {
    let output = ssh_command(public_ip, ssh, &format!("cat {remote_path}"))
        .output()
        .await
        .with_context(|| format!("fetching {remote_path} from {public_ip}"))?;
    if !output.status.success() {
        bail!("failed to fetch remote file {remote_path}");
    }
    Ok(String::from_utf8(output.stdout).context("remote file was not UTF-8")?)
}

async fn run_ssh(public_ip: &str, remote_command: &str, ssh: &SshMaterial) -> Result<()> {
    let status = ssh_command(public_ip, ssh, remote_command)
        .status()
        .await
        .with_context(|| format!("running remote command on {public_ip}"))?;
    if !status.success() {
        bail!("remote command failed on {public_ip}");
    }
    Ok(())
}

async fn run_speed_test(public_ip: &str, ssh: &SshMaterial) -> Result<f64> {
    let speed_host = "releases.ubuntu.com";
    let resolved_ip = lookup_host((speed_host, 443))
        .await
        .context("resolving speed test host")?
        .find(|addr| addr.is_ipv4())
        .map(|addr| addr.ip().to_string())
        .ok_or_else(|| anyhow!("no ipv4 address found for speed test host"))?;
    let output = ssh_command(
        public_ip,
        ssh,
        &format!(
            "DL=$(curl -4 -L -s -o /dev/null -w \"%{{speed_download}}\" --resolve {speed_host}:443:{resolved_ip} --max-time 20 https://{speed_host}/24.04/ubuntu-24.04.3-live-server-amd64.iso 2>/dev/null || echo 0); awk -v dl=\"$DL\" 'BEGIN {{ printf \"%.1f\\n\", (dl*8)/1000000 }}'"
        ),
    )
    .output()
    .await
    .context("running remote speed test")?;
    if !output.status.success() {
        bail!("remote speed test failed");
    }
    let text = String::from_utf8(output.stdout).context("speed test output was not utf-8")?;
    text.trim()
        .parse::<f64>()
        .context("parsing speed test result")
}

fn resolve_repo_root() -> PathBuf {
    for candidate in ["/home/dev/pulsovpn/escudo-vpn", "/opt/escudo"] {
        let path = PathBuf::from(candidate);
        if path
            .join("scripts/provision/harden-tunnel-node.sh")
            .exists()
        {
            return path;
        }
    }
    PathBuf::from("/opt/escudo")
}

fn ssh_command(public_ip: &str, ssh: &SshMaterial, remote_command: &str) -> Command {
    let mut command = Command::new("ssh");
    command.args([
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/dev/null",
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=10",
        "-i",
        ssh.private_key_path.to_string_lossy().as_ref(),
        &format!("root@{public_ip}"),
        remote_command,
    ]);
    command
}

fn server_plan_monthly_cost(server_type: &str) -> f64 {
    match server_type {
        "cx23" => 3.59,
        "cpx11" => 6.99,
        "cpx21" => 13.99,
        "ccx13" => 13.99,
        "vc2-1c-1gb" => 5.0,
        "vc2-1c-2gb" => 10.0,
        "cx32" => 7.14,
        "cx33" => 5.99,
        "vc2-2c-4gb" | "vhf-2c-4gb" => 20.0,
        _ => 0.0,
    }
}

fn location_to_country_code(location: &str) -> &'static str {
    match location {
        "ash" => "US",
        "hel1" => "FI",
        "gru" | "sao" => "BR",
        "nbg1" | "fsn1" => "DE",
        _ => "XX",
    }
}

fn location_to_country_name(location: &str) -> &'static str {
    match location {
        "ash" => "United States",
        "hel1" => "Finland",
        "gru" | "sao" => "Brazil",
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
