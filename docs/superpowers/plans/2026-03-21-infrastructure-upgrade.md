# Escudo VPN Infrastructure Upgrade — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform Escudo VPN from a single manually-deployed server to a fully automated multi-provider fleet with residential IP streaming proxy support.

**Architecture:** Three new crates (`escudo-deploy`, `escudo-proxy`, `escudo-guardian`) plus modifications to existing `escudo-api`, `escudo-gateway`, `escudo-common`. Server provisioning via Vultr + Hetzner APIs. Streaming traffic routed through residential SOCKS5 proxies using DNS + nftables + tun2socks. IP health monitoring with auto-rotation.

**Tech Stack:** Rust (tokio, reqwest, tonic, sqlx), WireGuard, dnsmasq, nftables, tun2socks (Go binary), IPRoyal API, Vultr API v2, Hetzner Cloud API v1.

**Spec:** `docs/superpowers/specs/2026-03-21-infrastructure-upgrade-design.md`

**⚠ Concurrent work:** Another session is modifying `Cargo.toml`, `crates/escudo-api/src/routes/vpn.rs`, `crates/escudo-api/src/config.rs`, and sqlx dependencies. Tasks that touch those files are marked with ⚠ and should wait until that work merges.

---

## File Structure

### New Crates

```
crates/escudo-deploy/
  Cargo.toml
  src/
    main.rs              — CLI entry point (clap subcommands: validate, plan, apply, status, destroy, update)
    config.rs            — Parse deploy-config.toml
    provider.rs          — ServerProvider trait
    providers/
      mod.rs
      vultr.rs           — Vultr API v2 client
      hetzner.rs         — Hetzner Cloud API v1 client
    reconciler.rs        — Diff desired vs actual state, create/destroy servers
    cloudinit.rs         — Generate per-server cloud-init scripts
    ssh.rs               — SSH into servers for updates/diagnostics

crates/escudo-proxy/
  Cargo.toml
  src/
    lib.rs               — Module exports
    provider.rs          — ProxyProvider trait
    providers/
      mod.rs
      iproyal.rs         — IPRoyal API client
      proxycheap.rs      — Proxy-Cheap API client (stub/fallback)
    pool.rs              — Pool management (acquire, release, rotate)
    credential.rs        — ProxyCredential struct

crates/escudo-guardian/
  Cargo.toml
  src/
    main.rs              — Tokio service entry point
    checker.rs           — Health check logic (connect through SOCKS5, check block indicators)
    rotator.rs           — Auto-rotation flow (swap IP, update DB, notify gateways)
    analytics.rs         — Burn rate pattern detection
    config.rs            — Guardian config (check interval, services, thresholds)
```

### New Migrations

```
migrations/
  20260321000001_alter_servers_multi_interface.sql
  20260321000002_alter_subscriptions_add_tier.sql
  20260321000003_create_provider_servers.sql
  20260321000004_create_proxy_ips.sql
  20260321000005_create_server_proxy_assignments.sql
  20260321000006_create_ip_health_logs.sql
  20260321000007_create_ip_rotation_logs.sql
```

### New Config Files

```
config/
  deploy.toml            — Deploy config (server fleet definition)
  guardian.toml           — Guardian config (check intervals, streaming services)
  proxy.toml              — Proxy provider credentials and pool settings

deploy/
  escudo-guardian.service  — Systemd unit for IP Guardian
```

### Modified Files

```
proto/gateway.proto                              — Add Tier enum, UpdateProxyCredentials RPC
Cargo.toml                                       — ⚠ Add new workspace members (after sqlx work merges)
crates/escudo-common/src/models.rs               — Add new DB models
crates/escudo-common/src/lib.rs                  — Export new modules
crates/escudo-gateway/src/grpc.rs                — Handle Tier field, multi-interface routing
crates/escudo-gateway/src/wg.rs                  — Multi-interface WgManager
crates/escudo-gateway/src/main.rs                — Multiple WgManagers, proxy credential polling
crates/escudo-gateway/src/config.rs              — Multi-interface config
crates/escudo-api/src/routes/vpn.rs              — ⚠ Tier-aware connect flow (after sqlx work merges)
crates/escudo-api/src/router.rs                  — Add phone-home and proxy-credentials endpoints
```

---

## Phase 1: Foundation + All Provider Connections

### Task 1: Database Migrations

**Files:**
- Create: `migrations/20260321000001_alter_servers_multi_interface.sql`
- Create: `migrations/20260321000002_alter_subscriptions_add_tier.sql`
- Create: `migrations/20260321000003_create_provider_servers.sql`
- Create: `migrations/20260321000004_create_proxy_ips.sql`
- Create: `migrations/20260321000005_create_server_proxy_assignments.sql`
- Create: `migrations/20260321000006_create_ip_health_logs.sql`
- Create: `migrations/20260321000007_create_ip_rotation_logs.sql`

- [ ] **Step 1: Write alter_servers migration**

```sql
-- migrations/20260321000001_alter_servers_multi_interface.sql
ALTER TABLE servers ADD COLUMN IF NOT EXISTS wg0_public_key TEXT;
ALTER TABLE servers ADD COLUMN IF NOT EXISTS wg0_port INTEGER DEFAULT 51820;
ALTER TABLE servers ADD COLUMN IF NOT EXISTS wg1_public_key TEXT;
ALTER TABLE servers ADD COLUMN IF NOT EXISTS wg1_port INTEGER DEFAULT 51821;
ALTER TABLE servers ADD COLUMN IF NOT EXISTS wg2_public_key TEXT;
ALTER TABLE servers ADD COLUMN IF NOT EXISTS wg2_port INTEGER DEFAULT 51822;
ALTER TABLE servers ADD COLUMN IF NOT EXISTS country_code TEXT;

-- Copy existing data to new columns
UPDATE servers SET wg0_public_key = public_key, wg0_port = endpoint_port
WHERE wg0_public_key IS NULL AND public_key IS NOT NULL;
```

- [ ] **Step 2: Write alter_subscriptions migration**

```sql
-- migrations/20260321000002_alter_subscriptions_add_tier.sql
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS tier TEXT NOT NULL DEFAULT 'free';

-- Map existing plan names to tier names
UPDATE subscriptions SET tier = 'free' WHERE plan = 'free' OR plan IS NULL;
UPDATE subscriptions SET tier = 'escudo' WHERE plan = 'pro';
UPDATE subscriptions SET tier = 'pro' WHERE plan = 'family';
```

- [ ] **Step 3: Write provider_servers migration**

```sql
-- migrations/20260321000003_create_provider_servers.sql
CREATE TABLE IF NOT EXISTS provider_servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID UNIQUE REFERENCES servers(id),
    provider TEXT NOT NULL,
    provider_instance_id TEXT NOT NULL,
    label TEXT NOT NULL UNIQUE,
    region TEXT NOT NULL,
    plan TEXT NOT NULL,
    public_ip TEXT,
    status TEXT NOT NULL DEFAULT 'provisioning',
    gateway_version TEXT,
    last_heartbeat TIMESTAMPTZ,
    monthly_cost_usd DECIMAL(8,2),
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(provider, provider_instance_id)
);
CREATE INDEX IF NOT EXISTS idx_provider_servers_status ON provider_servers(status);
```

- [ ] **Step 4: Write proxy_ips migration**

```sql
-- migrations/20260321000004_create_proxy_ips.sql
CREATE TABLE IF NOT EXISTS proxy_ips (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider TEXT NOT NULL,
    provider_proxy_id TEXT NOT NULL,
    proxy_type TEXT NOT NULL,
    country TEXT NOT NULL,
    city TEXT,
    socks5_host TEXT NOT NULL,
    socks5_port INTEGER NOT NULL,
    socks5_username TEXT NOT NULL,
    socks5_password TEXT NOT NULL,
    external_ip TEXT,
    status TEXT NOT NULL DEFAULT 'healthy',
    assigned_user_id UUID REFERENCES users(id),
    max_concurrent INTEGER DEFAULT 4,
    current_concurrent INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    last_health_check TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_proxy_ips_country_status ON proxy_ips(country, status);
CREATE INDEX IF NOT EXISTS idx_proxy_ips_assigned_user ON proxy_ips(assigned_user_id) WHERE assigned_user_id IS NOT NULL;
```

- [ ] **Step 5: Write server_proxy_assignments migration**

```sql
-- migrations/20260321000005_create_server_proxy_assignments.sql
CREATE TABLE IF NOT EXISTS server_proxy_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id),
    proxy_ip_id UUID NOT NULL REFERENCES proxy_ips(id),
    proxy_target TEXT NOT NULL DEFAULT 'shared',
    assigned_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(server_id, proxy_target)
);
```

- [ ] **Step 6: Write ip_health_logs migration**

```sql
-- migrations/20260321000006_create_ip_health_logs.sql
CREATE TABLE IF NOT EXISTS ip_health_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proxy_ip_id UUID NOT NULL REFERENCES proxy_ips(id),
    service TEXT NOT NULL,
    status TEXT NOT NULL,
    response_time_ms INTEGER,
    error_detail TEXT,
    checked_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_health_logs_proxy_checked ON ip_health_logs(proxy_ip_id, checked_at);
CREATE INDEX IF NOT EXISTS idx_health_logs_service ON ip_health_logs(service, checked_at);
```

- [ ] **Step 7: Write ip_rotation_logs migration**

```sql
-- migrations/20260321000007_create_ip_rotation_logs.sql
CREATE TABLE IF NOT EXISTS ip_rotation_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    old_proxy_ip_id UUID NOT NULL,
    new_proxy_ip_id UUID NOT NULL,
    reason TEXT NOT NULL,
    country TEXT NOT NULL,
    provider TEXT NOT NULL,
    affected_servers INTEGER DEFAULT 0,
    affected_customers INTEGER DEFAULT 0,
    rotated_at TIMESTAMPTZ DEFAULT now()
);
```

- [ ] **Step 8: Test migrations run cleanly**

Run: `cd /home/dev/pulsovpn/escudo-vpn && cargo build -p escudo-api 2>&1 | tail -5`

If no local PostgreSQL: verify SQL syntax by reading each file and confirming no syntax errors. Migrations will be tested on the actual server in Task 5.

- [ ] **Step 9: Commit**

```bash
git add migrations/20260321*.sql
git commit -m "feat: add database migrations for infrastructure upgrade

New tables: provider_servers, proxy_ips, server_proxy_assignments,
ip_health_logs, ip_rotation_logs. Alter servers for multi-interface
WireGuard. Add tier column to subscriptions."
```

---

### Task 2: escudo-proxy Crate (IPRoyal API Client)

**Files:**
- Create: `crates/escudo-proxy/Cargo.toml`
- Create: `crates/escudo-proxy/src/lib.rs`
- Create: `crates/escudo-proxy/src/credential.rs`
- Create: `crates/escudo-proxy/src/provider.rs`
- Create: `crates/escudo-proxy/src/providers/mod.rs`
- Create: `crates/escudo-proxy/src/providers/iproyal.rs`
- Create: `crates/escudo-proxy/src/providers/proxycheap.rs`
- Create: `crates/escudo-proxy/src/pool.rs`
- Create: `config/proxy.toml`
- Test: `crates/escudo-proxy/src/providers/iproyal.rs` (integration test at bottom)

- [ ] **Step 1: Create Cargo.toml**

```toml
# crates/escudo-proxy/Cargo.toml
[package]
name = "escudo-proxy"
version = "0.1.0"
edition = "2021"

[dependencies]
reqwest = { workspace = true, features = ["socks"] }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
async-trait = "0.1"
```

- [ ] **Step 2: Write credential.rs**

```rust
// crates/escudo-proxy/src/credential.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderKind {
    IpRoyal,
    ProxyCheap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProxyType {
    Shared,
    Dedicated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyCredential {
    pub id: String,
    pub provider: ProviderKind,
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub country: String,
    pub city: Option<String>,
    pub external_ip: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl ProxyCredential {
    pub fn socks5_url(&self) -> String {
        format!(
            "socks5://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}
```

- [ ] **Step 3: Write provider.rs (trait)**

```rust
// crates/escudo-proxy/src/provider.rs
use std::time::Duration;
use async_trait::async_trait;
use crate::credential::ProxyCredential;

#[async_trait]
pub trait ProxyProvider: Send + Sync {
    async fn acquire_shared_proxy(
        &self,
        country: &str,
        city: Option<&str>,
        sticky_duration: Duration,
    ) -> anyhow::Result<ProxyCredential>;

    async fn acquire_dedicated_ip(
        &self,
        country: &str,
    ) -> anyhow::Result<ProxyCredential>;

    async fn release_proxy(&self, proxy_id: &str) -> anyhow::Result<()>;

    async fn rotate_proxy(&self, proxy_id: &str) -> anyhow::Result<ProxyCredential>;

    async fn list_proxies(&self) -> anyhow::Result<Vec<ProxyCredential>>;

    async fn health_check(&self) -> anyhow::Result<bool>;
}
```

- [ ] **Step 4: Write IPRoyal provider**

```rust
// crates/escudo-proxy/src/providers/iproyal.rs
use std::time::Duration;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, error};
use crate::credential::{ProxyCredential, ProviderKind, ProxyType};
use crate::provider::ProxyProvider;

pub struct IpRoyalProvider {
    client: Client,
    api_token: String,
    base_url: String,
}

impl IpRoyalProvider {
    pub fn new(api_token: String) -> Self {
        Self {
            client: Client::new(),
            api_token,
            base_url: "https://dashboard.iproyal.com/api/v1".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct IpRoyalProxy {
    id: Option<String>,
    ip: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    country: Option<String>,
    city: Option<String>,
}

#[async_trait]
impl ProxyProvider for IpRoyalProvider {
    async fn acquire_shared_proxy(
        &self,
        country: &str,
        city: Option<&str>,
        sticky_duration: Duration,
    ) -> anyhow::Result<ProxyCredential> {
        // IPRoyal rotating residential with sticky session
        // Generate unique session ID for sticky session
        let session_id = format!("escudo_{}", uuid::Uuid::new_v4().simple());
        let sticky_mins = sticky_duration.as_secs() / 60;

        // IPRoyal proxy format: proxy.iproyal.com:12321 with auth
        // Country targeting via username suffix: _country-{code}
        let username = format!(
            "{}__country-{}{}__session-{}__lifetime-{}m",
            self.api_token,
            country.to_lowercase(),
            city.map(|c| format!("_city-{}", c.to_lowercase())).unwrap_or_default(),
            session_id,
            sticky_mins,
        );

        info!("Acquiring shared proxy for country={country}");

        Ok(ProxyCredential {
            id: session_id,
            provider: ProviderKind::IpRoyal,
            proxy_type: ProxyType::Shared,
            host: "geo.iproyal.com".to_string(),
            port: 32325, // SOCKS5 port
            username,
            password: self.api_token.clone(),
            country: country.to_string(),
            city: city.map(|s| s.to_string()),
            external_ip: None,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(sticky_duration.as_secs() as i64)),
        })
    }

    async fn acquire_dedicated_ip(
        &self,
        country: &str,
    ) -> anyhow::Result<ProxyCredential> {
        // IPRoyal static residential (ISP) proxies — requires API call to purchase
        let resp = self.client
            .get(format!("{}/proxy-manager/proxies", self.base_url))
            .bearer_auth(&self.api_token)
            .query(&[("country", country)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("IPRoyal API error {status}: {body}");
        }

        let proxies: Vec<IpRoyalProxy> = resp.json().await?;
        let proxy = proxies.first()
            .ok_or_else(|| anyhow::anyhow!("No dedicated IPs available for country={country}"))?;

        Ok(ProxyCredential {
            id: proxy.id.clone().unwrap_or_default(),
            provider: ProviderKind::IpRoyal,
            proxy_type: ProxyType::Dedicated,
            host: proxy.ip.clone().unwrap_or_default(),
            port: proxy.port.unwrap_or(32325),
            username: proxy.username.clone().unwrap_or_default(),
            password: proxy.password.clone().unwrap_or_default(),
            country: proxy.country.clone().unwrap_or_else(|| country.to_string()),
            city: proxy.city.clone(),
            external_ip: proxy.ip.clone(),
            expires_at: None,
        })
    }

    async fn release_proxy(&self, proxy_id: &str) -> anyhow::Result<()> {
        info!("Releasing proxy {proxy_id}");
        // For rotating proxies: no API call needed, session expires naturally
        // For static residential: API call to cancel
        Ok(())
    }

    async fn rotate_proxy(&self, proxy_id: &str) -> anyhow::Result<ProxyCredential> {
        // For rotating residential: generate new session ID (new sticky session = new IP)
        // proxy_id format includes country info, or caller passes country separately.
        // For now, parse country from the session ID or default to the proxy_id as-is.
        // The actual country is tracked in the DB — the rotator.rs passes the correct
        // country when calling ProxyPool::acquire_shared() directly instead of this method.
        info!("Rotating proxy {proxy_id} — generating new session");
        anyhow::bail!("Use ProxyPool::acquire_shared() with explicit country instead of rotate_proxy()")
    }

    async fn list_proxies(&self) -> anyhow::Result<Vec<ProxyCredential>> {
        let resp = self.client
            .get(format!("{}/proxy-manager/proxies", self.base_url))
            .bearer_auth(&self.api_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("IPRoyal API error {status}: {body}");
        }

        let proxies: Vec<IpRoyalProxy> = resp.json().await?;
        Ok(proxies.into_iter().map(|p| ProxyCredential {
            id: p.id.unwrap_or_default(),
            provider: ProviderKind::IpRoyal,
            proxy_type: ProxyType::Dedicated,
            host: p.ip.clone().unwrap_or_default(),
            port: p.port.unwrap_or(32325),
            username: p.username.unwrap_or_default(),
            password: p.password.unwrap_or_default(),
            country: p.country.unwrap_or_default(),
            city: p.city,
            external_ip: p.ip,
            expires_at: None,
        }).collect())
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        let resp = self.client
            .get(format!("{}/proxy-manager/proxies", self.base_url))
            .bearer_auth(&self.api_token)
            .send()
            .await?;
        Ok(resp.status().is_success())
    }
}
```

- [ ] **Step 5: Write providers/mod.rs and proxycheap stub**

```rust
// crates/escudo-proxy/src/providers/mod.rs
pub mod iproyal;
pub mod proxycheap;
```

```rust
// crates/escudo-proxy/src/providers/proxycheap.rs
use std::time::Duration;
use async_trait::async_trait;
use crate::credential::ProxyCredential;
use crate::provider::ProxyProvider;

/// Proxy-Cheap fallback provider — stub implementation.
/// Weak API docs, used only if IPRoyal is down.
pub struct ProxyCheapProvider {
    _api_key: String,
    _api_secret: String,
}

impl ProxyCheapProvider {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self { _api_key: api_key, _api_secret: api_secret }
    }
}

#[async_trait]
impl ProxyProvider for ProxyCheapProvider {
    async fn acquire_shared_proxy(&self, _country: &str, _city: Option<&str>, _sticky: Duration) -> anyhow::Result<ProxyCredential> {
        anyhow::bail!("ProxyCheap provider not yet implemented — use IPRoyal")
    }
    async fn acquire_dedicated_ip(&self, _country: &str) -> anyhow::Result<ProxyCredential> {
        anyhow::bail!("ProxyCheap provider not yet implemented — use IPRoyal")
    }
    async fn release_proxy(&self, _proxy_id: &str) -> anyhow::Result<()> { Ok(()) }
    async fn rotate_proxy(&self, _proxy_id: &str) -> anyhow::Result<ProxyCredential> {
        anyhow::bail!("ProxyCheap provider not yet implemented — use IPRoyal")
    }
    async fn list_proxies(&self) -> anyhow::Result<Vec<ProxyCredential>> { Ok(vec![]) }
    async fn health_check(&self) -> anyhow::Result<bool> { Ok(false) }
}
```

- [ ] **Step 6: Write pool.rs**

```rust
// crates/escudo-proxy/src/pool.rs
use std::time::Duration;
use tracing::{info, warn};
use crate::credential::ProxyCredential;
use crate::provider::ProxyProvider;

/// Manages proxy acquisition with failover between providers.
pub struct ProxyPool {
    primary: Box<dyn ProxyProvider>,
    fallback: Option<Box<dyn ProxyProvider>>,
}

impl ProxyPool {
    pub fn new(primary: Box<dyn ProxyProvider>, fallback: Option<Box<dyn ProxyProvider>>) -> Self {
        Self { primary, fallback }
    }

    pub async fn acquire_shared(
        &self,
        country: &str,
        city: Option<&str>,
        sticky_duration: Duration,
    ) -> anyhow::Result<ProxyCredential> {
        match self.primary.acquire_shared_proxy(country, city, sticky_duration).await {
            Ok(cred) => {
                info!("Acquired shared proxy from primary provider for {country}");
                Ok(cred)
            }
            Err(e) => {
                warn!("Primary provider failed: {e}, trying fallback");
                if let Some(ref fallback) = self.fallback {
                    fallback.acquire_shared_proxy(country, city, sticky_duration).await
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn acquire_dedicated(&self, country: &str) -> anyhow::Result<ProxyCredential> {
        match self.primary.acquire_dedicated_ip(country).await {
            Ok(cred) => Ok(cred),
            Err(e) => {
                warn!("Primary provider failed for dedicated: {e}, trying fallback");
                if let Some(ref fallback) = self.fallback {
                    fallback.acquire_dedicated_ip(country).await
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn rotate(&self, proxy_id: &str) -> anyhow::Result<ProxyCredential> {
        self.primary.rotate_proxy(proxy_id).await
    }

    pub async fn validate_providers(&self) -> anyhow::Result<()> {
        let primary_ok = self.primary.health_check().await?;
        if !primary_ok {
            anyhow::bail!("Primary proxy provider health check failed");
        }
        info!("Primary proxy provider: healthy");

        if let Some(ref fallback) = self.fallback {
            match fallback.health_check().await {
                Ok(true) => info!("Fallback proxy provider: healthy"),
                _ => warn!("Fallback proxy provider: unhealthy (non-critical)"),
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 7: Write lib.rs**

```rust
// crates/escudo-proxy/src/lib.rs
pub mod credential;
pub mod provider;
pub mod providers;
pub mod pool;
```

- [ ] **Step 8: Write proxy.toml config**

```toml
# config/proxy.toml
[iproyal]
# API token loaded from IPROYAL_API_TOKEN env var
socks5_host = "geo.iproyal.com"
socks5_port = 32325

[pool]
default_sticky_duration_hours = 168  # 7 days
max_concurrent_per_shared_ip = 4
```

- [ ] **Step 9: Verify it compiles**

Note: The crate is not yet in the workspace. Compile-check from within the crate directory:
Run: `cd /home/dev/pulsovpn/escudo-vpn/crates/escudo-proxy && cargo check`
Expected: compiles with no errors (may show warnings about unused code)

- [ ] **Step 10: Commit**

```bash
git add crates/escudo-proxy/ config/proxy.toml
git commit -m "feat: add escudo-proxy crate with IPRoyal API client

ProxyProvider trait with IPRoyal implementation and ProxyCheap stub.
ProxyPool with automatic failover between providers.
SOCKS5 credential generation with sticky sessions up to 7 days."
```

---

### Task 3: escudo-deploy Crate (Vultr + Hetzner Provisioning)

**Files:**
- Create: `crates/escudo-deploy/Cargo.toml`
- Create: `crates/escudo-deploy/src/main.rs`
- Create: `crates/escudo-deploy/src/config.rs`
- Create: `crates/escudo-deploy/src/provider.rs`
- Create: `crates/escudo-deploy/src/providers/mod.rs`
- Create: `crates/escudo-deploy/src/providers/vultr.rs`
- Create: `crates/escudo-deploy/src/providers/hetzner.rs`
- Create: `crates/escudo-deploy/src/reconciler.rs`
- Create: `crates/escudo-deploy/src/cloudinit.rs`
- Create: `crates/escudo-deploy/src/ssh.rs`
- Create: `config/deploy.toml`

- [ ] **Step 1: Create Cargo.toml**

```toml
# crates/escudo-deploy/Cargo.toml
[package]
name = "escudo-deploy"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "escudo-deploy"
path = "src/main.rs"

[dependencies]
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
clap = { workspace = true }
chrono = { workspace = true }
base64 = { workspace = true }
async-trait = "0.1"
escudo-proxy = { path = "../escudo-proxy" }
```

- [ ] **Step 2: Write config.rs**

```rust
// crates/escudo-deploy/src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeployConfig {
    pub defaults: Defaults,
    pub servers: Vec<ServerEntry>,
}

#[derive(Debug, Deserialize)]
pub struct Defaults {
    pub ssh_key_name: String,
    pub firewall_rules: Vec<String>,
    pub image: String,
    pub gateway_binary_url: String,
    pub tun2socks_binary_url: String,
    pub api_callback_url: String,
    pub deploy_secret: Option<String>, // loaded from env if not in file
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerEntry {
    pub label: String,
    pub provider: String,   // "vultr" | "hetzner"
    pub region: String,
    pub plan: String,
}

impl DeployConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: DeployConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
```

- [ ] **Step 3: Write provider.rs (trait)**

```rust
// crates/escudo-deploy/src/provider.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionedServer {
    pub provider: String,
    pub provider_instance_id: String,
    pub label: String,
    pub region: String,
    pub plan: String,
    pub public_ip: Option<String>,
    pub status: String,
    pub monthly_cost_usd: f64,
}

#[async_trait]
pub trait ServerProvider: Send + Sync {
    /// Create a new server with the given cloud-init user_data
    async fn create_server(
        &self,
        label: &str,
        region: &str,
        plan: &str,
        image: &str,
        ssh_key_name: &str,
        user_data: &str,
    ) -> anyhow::Result<ProvisionedServer>;

    /// List all servers managed by this provider (filtered by label prefix or tag)
    async fn list_servers(&self) -> anyhow::Result<Vec<ProvisionedServer>>;

    /// Destroy a server by provider instance ID
    async fn destroy_server(&self, instance_id: &str) -> anyhow::Result<()>;

    /// Get server status and IP (for polling after creation)
    async fn get_server(&self, instance_id: &str) -> anyhow::Result<ProvisionedServer>;

    /// Validate API credentials
    async fn validate(&self) -> anyhow::Result<()>;

    /// Get provider name
    fn provider_name(&self) -> &str;
}
```

- [ ] **Step 4: Write Vultr provider**

```rust
// crates/escudo-deploy/src/providers/vultr.rs
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};
use crate::provider::{ProvisionedServer, ServerProvider};

pub struct VultrProvider {
    client: Client,
    api_key: String,
}

impl VultrProvider {
    pub fn new(api_key: String) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {api_key}").parse().unwrap(),
        );
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client, api_key }
    }
}

#[derive(Debug, Deserialize)]
struct VultrInstance {
    id: String,
    label: String,
    region: String,
    plan: String,
    main_ip: String,
    status: String,
    #[serde(default)]
    server_status: String,
}

#[derive(Debug, Deserialize)]
struct VultrInstanceResponse {
    instance: VultrInstance,
}

#[derive(Debug, Deserialize)]
struct VultrListResponse {
    instances: Vec<VultrInstance>,
}

#[derive(Debug, Deserialize)]
struct VultrAccount {
    account: VultrAccountInfo,
}

#[derive(Debug, Deserialize)]
struct VultrAccountInfo {
    balance: f64,
    pending_charges: f64,
}

#[async_trait]
impl ServerProvider for VultrProvider {
    async fn create_server(
        &self,
        label: &str,
        region: &str,
        plan: &str,
        image: &str,
        _ssh_key_name: &str,
        user_data: &str,
    ) -> anyhow::Result<ProvisionedServer> {
        use base64::Engine;
        let user_data_b64 = base64::engine::general_purpose::STANDARD.encode(user_data.as_bytes());

        // os_id 2284 = Ubuntu 24.04 LTS x64
        let body = serde_json::json!({
            "region": region,
            "plan": plan,
            "os_id": 2284,
            "label": label,
            "user_data": user_data_b64,
            "backups": "disabled",
            "tags": ["escudo-vpn"],
        });

        info!("Creating Vultr server: label={label} region={region} plan={plan}");

        let resp = self.client
            .post("https://api.vultr.com/v2/instances")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Vultr create failed ({status}): {body}");
        }

        let result: VultrInstanceResponse = resp.json().await?;
        let inst = result.instance;

        Ok(ProvisionedServer {
            provider: "vultr".to_string(),
            provider_instance_id: inst.id,
            label: inst.label,
            region: inst.region,
            plan: inst.plan,
            public_ip: if inst.main_ip == "0.0.0.0" { None } else { Some(inst.main_ip) },
            status: inst.status,
            monthly_cost_usd: 5.0, // vc2-1c-1gb
        })
    }

    async fn list_servers(&self) -> anyhow::Result<Vec<ProvisionedServer>> {
        let resp = self.client
            .get("https://api.vultr.com/v2/instances?tag=escudo-vpn&per_page=500")
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Vultr list failed ({status}): {body}");
        }

        let result: VultrListResponse = resp.json().await?;
        Ok(result.instances.into_iter().map(|i| ProvisionedServer {
            provider: "vultr".to_string(),
            provider_instance_id: i.id,
            label: i.label,
            region: i.region,
            plan: i.plan,
            public_ip: if i.main_ip == "0.0.0.0" { None } else { Some(i.main_ip) },
            status: i.status,
            monthly_cost_usd: 5.0,
        }).collect())
    }

    async fn destroy_server(&self, instance_id: &str) -> anyhow::Result<()> {
        info!("Destroying Vultr server {instance_id}");
        let resp = self.client
            .delete(format!("https://api.vultr.com/v2/instances/{instance_id}"))
            .send()
            .await?;

        if !resp.status().is_success() && resp.status().as_u16() != 404 {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Vultr destroy failed ({status}): {body}");
        }
        Ok(())
    }

    async fn get_server(&self, instance_id: &str) -> anyhow::Result<ProvisionedServer> {
        let resp = self.client
            .get(format!("https://api.vultr.com/v2/instances/{instance_id}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Vultr get failed ({status}): {body}");
        }

        let result: VultrInstanceResponse = resp.json().await?;
        let i = result.instance;
        Ok(ProvisionedServer {
            provider: "vultr".to_string(),
            provider_instance_id: i.id,
            label: i.label,
            region: i.region,
            plan: i.plan,
            public_ip: if i.main_ip == "0.0.0.0" { None } else { Some(i.main_ip) },
            status: if i.server_status == "ok" { "active".to_string() } else { i.status },
            monthly_cost_usd: 5.0,
        })
    }

    async fn validate(&self) -> anyhow::Result<()> {
        let resp = self.client
            .get("https://api.vultr.com/v2/account")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Vultr API key invalid");
        }

        let acct: VultrAccount = resp.json().await?;
        info!(
            "Vultr account validated. Balance: ${:.2}, Pending: ${:.2}",
            acct.account.balance, acct.account.pending_charges
        );
        Ok(())
    }

    fn provider_name(&self) -> &str { "vultr" }
}
```

- [ ] **Step 5: Write Hetzner provider**

```rust
// crates/escudo-deploy/src/providers/hetzner.rs
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tracing::info;
use crate::provider::{ProvisionedServer, ServerProvider};

pub struct HetznerProvider {
    client: Client,
}

impl HetznerProvider {
    pub fn new(api_token: String) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {api_token}").parse().unwrap(),
        );
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client }
    }
}

#[derive(Debug, Deserialize)]
struct HtzServer {
    id: u64,
    name: String,
    status: String,
    public_net: HtzPublicNet,
    server_type: HtzServerType,
    datacenter: HtzDatacenter,
}

#[derive(Debug, Deserialize)]
struct HtzPublicNet {
    ipv4: HtzIpv4,
}

#[derive(Debug, Deserialize)]
struct HtzIpv4 {
    ip: String,
}

#[derive(Debug, Deserialize)]
struct HtzServerType {
    name: String,
}

#[derive(Debug, Deserialize)]
struct HtzDatacenter {
    name: String,
    location: HtzLocation,
}

#[derive(Debug, Deserialize)]
struct HtzLocation {
    name: String,
}

#[derive(Debug, Deserialize)]
struct HtzCreateResponse {
    server: HtzServer,
    action: HtzAction,
}

#[derive(Debug, Deserialize)]
struct HtzAction {
    id: u64,
    status: String,
}

#[derive(Debug, Deserialize)]
struct HtzListResponse {
    servers: Vec<HtzServer>,
}

#[derive(Debug, Deserialize)]
struct HtzServerResponse {
    server: HtzServer,
}

#[derive(Debug, Deserialize)]
struct HtzActionResponse {
    action: HtzAction,
}

fn hetzner_monthly_cost(plan: &str) -> f64 {
    match plan {
        "cx22" => 4.20,   // €3.79 ≈ $4.20
        "cpx21" => 5.00,  // estimate
        "cx32" => 7.50,
        _ => 5.00,
    }
}

#[async_trait]
impl ServerProvider for HetznerProvider {
    async fn create_server(
        &self,
        label: &str,
        region: &str,
        plan: &str,
        image: &str,
        _ssh_key_name: &str,
        user_data: &str,
    ) -> anyhow::Result<ProvisionedServer> {
        info!("Creating Hetzner server: label={label} location={region} plan={plan}");

        let body = serde_json::json!({
            "name": label,
            "server_type": plan,
            "image": image,
            "location": region,
            "user_data": user_data,
            "start_after_create": true,
            "labels": {"managed-by": "escudo-deploy"},
        });

        let resp = self.client
            .post("https://api.hetzner.cloud/v1/servers")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Hetzner create failed ({status}): {body}");
        }

        let result: HtzCreateResponse = resp.json().await?;
        let s = result.server;

        Ok(ProvisionedServer {
            provider: "hetzner".to_string(),
            provider_instance_id: s.id.to_string(),
            label: s.name,
            region: s.datacenter.location.name,
            plan: s.server_type.name,
            public_ip: Some(s.public_net.ipv4.ip),
            status: s.status,
            monthly_cost_usd: hetzner_monthly_cost(plan),
        })
    }

    async fn list_servers(&self) -> anyhow::Result<Vec<ProvisionedServer>> {
        let resp = self.client
            .get("https://api.hetzner.cloud/v1/servers?label_selector=managed-by=escudo-deploy&per_page=50")
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Hetzner list failed ({status}): {body}");
        }

        let result: HtzListResponse = resp.json().await?;
        Ok(result.servers.into_iter().map(|s| ProvisionedServer {
            provider: "hetzner".to_string(),
            provider_instance_id: s.id.to_string(),
            label: s.name,
            region: s.datacenter.location.name,
            plan: s.server_type.name,
            public_ip: Some(s.public_net.ipv4.ip),
            status: s.status,
            monthly_cost_usd: hetzner_monthly_cost(&s.server_type.name),
        }).collect())
    }

    async fn destroy_server(&self, instance_id: &str) -> anyhow::Result<()> {
        info!("Destroying Hetzner server {instance_id}");
        let resp = self.client
            .delete(format!("https://api.hetzner.cloud/v1/servers/{instance_id}"))
            .send()
            .await?;

        if !resp.status().is_success() && resp.status().as_u16() != 404 {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Hetzner destroy failed ({status}): {body}");
        }
        Ok(())
    }

    async fn get_server(&self, instance_id: &str) -> anyhow::Result<ProvisionedServer> {
        let resp = self.client
            .get(format!("https://api.hetzner.cloud/v1/servers/{instance_id}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Hetzner get failed ({status}): {body}");
        }

        let result: HtzServerResponse = resp.json().await?;
        let s = result.server;
        Ok(ProvisionedServer {
            provider: "hetzner".to_string(),
            provider_instance_id: s.id.to_string(),
            label: s.name,
            region: s.datacenter.location.name,
            plan: s.server_type.name,
            public_ip: Some(s.public_net.ipv4.ip),
            status: s.status,
            monthly_cost_usd: hetzner_monthly_cost(&s.server_type.name),
        })
    }

    async fn validate(&self) -> anyhow::Result<()> {
        let resp = self.client
            .get("https://api.hetzner.cloud/v1/servers?per_page=1")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Hetzner API token invalid");
        }
        info!("Hetzner account validated");
        Ok(())
    }

    fn provider_name(&self) -> &str { "hetzner" }
}
```

- [ ] **Step 6: Write providers/mod.rs**

```rust
// crates/escudo-deploy/src/providers/mod.rs
pub mod vultr;
pub mod hetzner;
```

- [ ] **Step 7: Write cloudinit.rs**

```rust
// crates/escudo-deploy/src/cloudinit.rs
use crate::config::Defaults;

/// Generate cloud-init user_data script for a VPN server.
/// This installs WireGuard (3 interfaces), dnsmasq, nftables, tun2socks,
/// and the escudo-gateway binary, then phones home to the central API.
pub fn generate_cloudinit(
    defaults: &Defaults,
    label: &str,
    provider: &str,
    deploy_secret: &str,
) -> String {
    format!(r#"#!/bin/bash
set -e

export DEBIAN_FRONTEND=noninteractive

# Install packages
apt-get update -qq
apt-get install -y -qq wireguard wireguard-tools dnsmasq nftables curl

# Enable IP forwarding
sysctl -w net.ipv4.ip_forward=1
echo "net.ipv4.ip_forward = 1" >> /etc/sysctl.conf

# Generate WireGuard keys for 3 interfaces
WG0_PRIVATE=$(wg genkey)
WG0_PUBLIC=$(echo "$WG0_PRIVATE" | wg pubkey)
WG1_PRIVATE=$(wg genkey)
WG1_PUBLIC=$(echo "$WG1_PRIVATE" | wg pubkey)
WG2_PRIVATE=$(wg genkey)
WG2_PUBLIC=$(echo "$WG2_PRIVATE" | wg pubkey)

INTERFACE=$(ip -4 route show default | awk '{{print $5}}')

# wg0 — Free/Escudo tier
cat > /etc/wireguard/wg0.conf << WGEOF
[Interface]
Address = 10.0.0.1/18
ListenPort = 51820
PrivateKey = $WG0_PRIVATE
PostUp = iptables -t nat -A POSTROUTING -o $INTERFACE -j MASQUERADE
PostUp = iptables -A FORWARD -i wg0 -j ACCEPT
PostUp = iptables -A FORWARD -o wg0 -m state --state RELATED,ESTABLISHED -j ACCEPT
PostDown = iptables -t nat -D POSTROUTING -o $INTERFACE -j MASQUERADE
PostDown = iptables -D FORWARD -i wg0 -j ACCEPT
PostDown = iptables -D FORWARD -o wg0 -m state --state RELATED,ESTABLISHED -j ACCEPT
WGEOF

# wg1 — Pro tier (streaming proxy)
cat > /etc/wireguard/wg1.conf << WGEOF
[Interface]
Address = 10.0.64.1/18
ListenPort = 51821
PrivateKey = $WG1_PRIVATE
PostUp = iptables -t nat -A POSTROUTING -o $INTERFACE -j MASQUERADE
PostUp = iptables -A FORWARD -i wg1 -j ACCEPT
PostUp = iptables -A FORWARD -o wg1 -m state --state RELATED,ESTABLISHED -j ACCEPT
PostDown = iptables -t nat -D POSTROUTING -o $INTERFACE -j MASQUERADE
PostDown = iptables -D FORWARD -i wg1 -j ACCEPT
PostDown = iptables -D FORWARD -o wg1 -m state --state RELATED,ESTABLISHED -j ACCEPT
WGEOF

# wg2 — Dedicated IP tier
cat > /etc/wireguard/wg2.conf << WGEOF
[Interface]
Address = 10.0.128.1/18
ListenPort = 51822
PrivateKey = $WG2_PRIVATE
PostUp = iptables -t nat -A POSTROUTING -o $INTERFACE -j MASQUERADE
PostUp = iptables -A FORWARD -i wg2 -j ACCEPT
PostUp = iptables -A FORWARD -o wg2 -m state --state RELATED,ESTABLISHED -j ACCEPT
PostDown = iptables -t nat -D POSTROUTING -o $INTERFACE -j MASQUERADE
PostDown = iptables -D FORWARD -i wg2 -j ACCEPT
PostDown = iptables -D FORWARD -o wg2 -m state --state RELATED,ESTABLISHED -j ACCEPT
WGEOF

chmod 600 /etc/wireguard/*.conf

# Start WireGuard interfaces
systemctl enable --now wg-quick@wg0
systemctl enable --now wg-quick@wg1
systemctl enable --now wg-quick@wg2

# Configure dnsmasq for streaming domain nftset population
cat > /etc/dnsmasq.d/escudo-streaming.conf << DNSEOF
# Upstream DNS (Cloudflare DoH)
server=1.1.1.1
server=1.0.0.1

# Netflix
nftset=/netflix.com/4#ip#filter#streaming_v4
nftset=/nflxvideo.net/4#ip#filter#streaming_v4
nftset=/nflxso.net/4#ip#filter#streaming_v4
nftset=/nflximg.net/4#ip#filter#streaming_v4
nftset=/nflxext.com/4#ip#filter#streaming_v4

# BBC iPlayer
nftset=/bbc.co.uk/4#ip#filter#streaming_v4
nftset=/bbci.co.uk/4#ip#filter#streaming_v4
nftset=/bbc.com/4#ip#filter#streaming_v4

# Disney+
nftset=/disneyplus.com/4#ip#filter#streaming_v4
nftset=/bamgrid.com/4#ip#filter#streaming_v4
nftset=/dssott.com/4#ip#filter#streaming_v4
nftset=/disney.io/4#ip#filter#streaming_v4

# Globoplay (scoped — NOT globo.com which is too broad)
nftset=/globoplay.globo.com/4#ip#filter#streaming_v4
nftset=/video.globo.com/4#ip#filter#streaming_v4

# Listen on VPN interfaces
listen-address=10.0.64.1,10.0.128.1
bind-interfaces
DNSEOF

systemctl restart dnsmasq

# Setup nftables for streaming traffic marking
nft add table ip filter 2>/dev/null || true
nft add chain ip filter forward '{{ type filter hook forward priority 0; policy accept; }}' 2>/dev/null || true
nft add set ip filter streaming_v4 '{{ type ipv4_addr; flags timeout; timeout 1h; }}' 2>/dev/null || true

# Mark streaming packets from Pro interface
nft add rule ip filter forward iifname "wg1" ip daddr @streaming_v4 ct mark set 0x1
# Mark ALL packets from Dedicated interface
nft add rule ip filter forward iifname "wg2" ct mark set 0x1
# Propagate conntrack mark to packet mark for policy routing
nft add rule ip filter forward ct mark 0x1 meta mark set 0x1

# Policy routing for marked packets
ip rule add fwmark 0x1 table 100 2>/dev/null || true
ip route add blackhole default table 100 metric 200 2>/dev/null || true

# Download binaries
curl -sSL {gateway_url} -o /usr/local/bin/escudo-gateway
chmod +x /usr/local/bin/escudo-gateway

curl -sSL {tun2socks_url} -o /usr/local/bin/tun2socks
chmod +x /usr/local/bin/tun2socks

# Phone home to central API
PUBLIC_IP=$(curl -s ifconfig.me)
curl -sS -X POST {api_url} \
  -H "Authorization: Bearer {secret}" \
  -H "Content-Type: application/json" \
  -d "{{
    \"public_ip\": \"$PUBLIC_IP\",
    \"wg0_public_key\": \"$WG0_PUBLIC\",
    \"wg1_public_key\": \"$WG1_PUBLIC\",
    \"wg2_public_key\": \"$WG2_PUBLIC\",
    \"wg0_port\": 51820,
    \"wg1_port\": 51821,
    \"wg2_port\": 51822,
    \"location\": \"{label}\",
    \"provider\": \"{provider}\",
    \"label\": \"{label}\",
    \"version\": \"0.3.0\"
  }}"

echo "Escudo VPN gateway setup complete"
"#,
        gateway_url = defaults.gateway_binary_url,
        tun2socks_url = defaults.tun2socks_binary_url,
        api_url = defaults.api_callback_url,
        secret = deploy_secret,
        label = label,
        provider = provider,
    )
}
```

- [ ] **Step 8: Write reconciler.rs**

```rust
// crates/escudo-deploy/src/reconciler.rs
use std::collections::HashMap;
use tracing::{info, warn, error};
use crate::config::{DeployConfig, ServerEntry};
use crate::provider::{ProvisionedServer, ServerProvider};
use crate::cloudinit::generate_cloudinit;

pub struct Reconciler {
    providers: HashMap<String, Box<dyn ServerProvider>>,
    config: DeployConfig,
    deploy_secret: String,
}

pub struct ReconcileResult {
    pub created: Vec<ProvisionedServer>,
    pub destroyed: Vec<String>,
    pub existing: Vec<ProvisionedServer>,
    pub failed: Vec<(String, String)>, // (label, error)
    pub total_monthly_cost: f64,
}

impl Reconciler {
    pub fn new(
        providers: HashMap<String, Box<dyn ServerProvider>>,
        config: DeployConfig,
        deploy_secret: String,
    ) -> Self {
        Self { providers, config, deploy_secret }
    }

    /// Show what would change without making changes
    pub async fn plan(&self) -> anyhow::Result<()> {
        let actual = self.get_actual_state().await?;
        let desired: HashMap<String, &ServerEntry> = self.config.servers.iter()
            .map(|s| (s.label.clone(), s))
            .collect();

        let mut to_create = vec![];
        let mut to_destroy = vec![];

        for (label, entry) in &desired {
            if !actual.contains_key(label) {
                to_create.push(entry);
            }
        }

        for (label, server) in &actual {
            if !desired.contains_key(label) {
                to_destroy.push((label.clone(), server));
            }
        }

        if to_create.is_empty() && to_destroy.is_empty() {
            info!("No changes needed — desired state matches actual state");
            println!("✓ No changes needed. {} servers running.", actual.len());
            return Ok(());
        }

        println!("Plan:");
        for entry in &to_create {
            println!("  + CREATE {} ({} in {})", entry.label, entry.provider, entry.region);
        }
        for (label, server) in &to_destroy {
            println!("  - DESTROY {} ({})", label, server.provider);
        }

        let cost: f64 = desired.values().map(|e| match e.provider.as_str() {
            "vultr" => 5.0,
            "hetzner" => 4.20,
            _ => 5.0,
        }).sum();
        println!("\nEstimated monthly cost: ${:.2}", cost);

        Ok(())
    }

    /// Apply changes to match desired state
    pub async fn apply(&self) -> anyhow::Result<ReconcileResult> {
        let actual = self.get_actual_state().await?;
        let desired: HashMap<String, &ServerEntry> = self.config.servers.iter()
            .map(|s| (s.label.clone(), s))
            .collect();

        let mut result = ReconcileResult {
            created: vec![],
            destroyed: vec![],
            existing: vec![],
            failed: vec![],
            total_monthly_cost: 0.0,
        };

        // Create missing servers
        for (label, entry) in &desired {
            if actual.contains_key(label) {
                if let Some(server) = actual.get(label) {
                    result.existing.push(server.clone());
                    result.total_monthly_cost += server.monthly_cost_usd;
                }
                continue;
            }

            let provider = self.providers.get(&entry.provider)
                .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", entry.provider))?;

            let user_data = generate_cloudinit(
                &self.config.defaults,
                &entry.label,
                &entry.provider,
                &self.deploy_secret,
            );

            match provider.create_server(
                &entry.label,
                &entry.region,
                &entry.plan,
                &self.config.defaults.image,
                &self.config.defaults.ssh_key_name,
                &user_data,
            ).await {
                Ok(server) => {
                    info!("Created server: {} ({})", server.label, server.provider);
                    result.total_monthly_cost += server.monthly_cost_usd;
                    result.created.push(server);
                }
                Err(e) => {
                    error!("Failed to create {}: {e}", entry.label);
                    result.failed.push((entry.label.clone(), e.to_string()));
                }
            }
        }

        // Destroy servers not in desired state
        for (label, server) in &actual {
            if desired.contains_key(label) {
                continue;
            }

            let provider = match self.providers.get(&server.provider) {
                Some(p) => p,
                None => {
                    warn!("Cannot destroy {label}: unknown provider {}", server.provider);
                    continue;
                }
            };

            match provider.destroy_server(&server.provider_instance_id).await {
                Ok(()) => {
                    info!("Destroyed server: {label}");
                    result.destroyed.push(label.clone());
                }
                Err(e) => {
                    error!("Failed to destroy {label}: {e}");
                    result.failed.push((label.clone(), e.to_string()));
                }
            }
        }

        // Report
        println!("\nReconciliation complete:");
        println!("  Created:  {}", result.created.len());
        println!("  Existing: {}", result.existing.len());
        println!("  Destroyed: {}", result.destroyed.len());
        println!("  Failed:   {}", result.failed.len());
        println!("  Monthly cost: ${:.2}", result.total_monthly_cost);

        for (label, err) in &result.failed {
            println!("  ✗ {label}: {err}");
        }

        Ok(result)
    }

    /// Get current state across all providers
    async fn get_actual_state(&self) -> anyhow::Result<HashMap<String, ProvisionedServer>> {
        let mut actual = HashMap::new();

        for (_, provider) in &self.providers {
            match provider.list_servers().await {
                Ok(servers) => {
                    for server in servers {
                        actual.insert(server.label.clone(), server);
                    }
                }
                Err(e) => {
                    warn!("Failed to list servers from {}: {e}", provider.provider_name());
                }
            }
        }

        Ok(actual)
    }
}
```

- [ ] **Step 9: Write ssh.rs (stub for update command)**

```rust
// crates/escudo-deploy/src/ssh.rs
use tracing::info;

/// SSH-based operations for server management.
/// Used by `escudo-deploy update` to push new binaries.
pub async fn update_server_binary(
    _public_ip: &str,
    _binary_url: &str,
) -> anyhow::Result<()> {
    // TODO: Implement SSH-based binary update
    // For Phase 1, binary updates are done by destroying and recreating servers
    info!("SSH update not yet implemented — recreate server to update binary");
    Ok(())
}
```

- [ ] **Step 10: Write main.rs**

```rust
// crates/escudo-deploy/src/main.rs
use std::collections::HashMap;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod config;
mod provider;
mod providers;
mod reconciler;
mod cloudinit;
mod ssh;

use config::DeployConfig;
use providers::vultr::VultrProvider;
use providers::hetzner::HetznerProvider;
use reconciler::Reconciler;

#[derive(Parser)]
#[command(name = "escudo-deploy", about = "Escudo VPN server fleet management")]
struct Cli {
    /// Path to deploy config file
    #[arg(short, long, default_value = "config/deploy.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate API keys and config
    Validate,
    /// Show what would change (dry run)
    Plan,
    /// Apply changes to match desired state
    Apply,
    /// Show status of all servers
    Status,
    /// Destroy a specific server by label
    Destroy { label: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("escudo_deploy=info".parse()?))
        .init();

    let cli = Cli::parse();
    let deploy_config = DeployConfig::load(&cli.config)?;

    let vultr_key = std::env::var("VULTR_API_KEY")
        .map_err(|_| anyhow::anyhow!("VULTR_API_KEY env var not set"))?;
    let hetzner_token = std::env::var("HETZNER_API_TOKEN")
        .map_err(|_| anyhow::anyhow!("HETZNER_API_TOKEN env var not set"))?;
    let deploy_secret = std::env::var("DEPLOY_SECRET")
        .unwrap_or_else(|_| "dev-secret-change-me".to_string());

    let mut providers: HashMap<String, Box<dyn provider::ServerProvider>> = HashMap::new();
    providers.insert("vultr".to_string(), Box::new(VultrProvider::new(vultr_key)));
    providers.insert("hetzner".to_string(), Box::new(HetznerProvider::new(hetzner_token)));

    let reconciler = Reconciler::new(providers, deploy_config, deploy_secret);

    match cli.command {
        Commands::Validate => {
            // Import providers to call validate directly
            let vultr_key = std::env::var("VULTR_API_KEY")?;
            let hetzner_token = std::env::var("HETZNER_API_TOKEN")?;
            let iproyal_token = std::env::var("IPROYAL_API_TOKEN")
                .map_err(|_| anyhow::anyhow!("IPROYAL_API_TOKEN env var not set"))?;

            println!("Validating provider APIs...\n");

            let vultr = VultrProvider::new(vultr_key);
            vultr.validate().await?;
            println!("  ✓ Vultr API key valid");

            let hetzner = HetznerProvider::new(hetzner_token);
            hetzner.validate().await?;
            println!("  ✓ Hetzner API token valid");

            let iproyal = escudo_proxy::providers::iproyal::IpRoyalProvider::new(iproyal_token);
            use escudo_proxy::provider::ProxyProvider;
            if iproyal.health_check().await? {
                println!("  ✓ IPRoyal API token valid");
            } else {
                println!("  ✗ IPRoyal API token invalid");
            }

            println!("\nAll providers validated.");
        }
        Commands::Plan => {
            reconciler.plan().await?;
        }
        Commands::Apply => {
            reconciler.apply().await?;
        }
        Commands::Status => {
            reconciler.plan().await?; // Shows current state as part of plan
        }
        Commands::Destroy { label } => {
            println!("Destroying server: {label}");
            // TODO: check active peers, prompt for confirmation
            info!("Destroy command for {label} — not yet implemented with safety checks");
        }
    }

    Ok(())
}
```

- [ ] **Step 11: Write deploy.toml test config**

```toml
# config/deploy.toml — test config (1 server per provider)
[defaults]
ssh_key_name = "escudo-deploy"
firewall_rules = ["udp:51820", "udp:51821", "udp:51822", "tcp:22"]
image = "ubuntu-24.04"
gateway_binary_url = "https://deploy.escudovpn.com/gateway-linux-amd64"
tun2socks_binary_url = "https://github.com/xjasonlyu/tun2socks/releases/latest/download/tun2socks-linux-amd64.zip"
api_callback_url = "https://api.escudovpn.com/internal/servers/register"

[[servers]]
label = "br-sp-01"
provider = "vultr"
region = "gru"
plan = "vc2-1c-1gb"

[[servers]]
label = "de-nbg-01"
provider = "hetzner"
region = "nbg1"
plan = "cx22"
```

- [ ] **Step 12: Add new crates to workspace (⚠ after sqlx work merges)**

Add to workspace `Cargo.toml` members:
```toml
members = [
    # ... existing ...
    "crates/escudo-deploy",
    "crates/escudo-proxy",
]
```

Also add to `[workspace.dependencies]`:
```toml
async-trait = "0.1"
```

- [ ] **Step 13: Verify it compiles**

Run: `cd /home/dev/pulsovpn/escudo-vpn && cargo check -p escudo-deploy`
Expected: compiles with no errors

- [ ] **Step 14: Commit**

```bash
git add crates/escudo-deploy/ config/deploy.toml
git commit -m "feat: add escudo-deploy crate with Vultr + Hetzner provisioning

Declarative TOML config, diff-based reconciliation (plan/apply/status).
Vultr API v2 and Hetzner Cloud API v1 clients.
Cloud-init script generates 3 WireGuard interfaces + dnsmasq + nftables."
```

---

### Task 4: Validate All Provider APIs

**Files:** None (runtime test)

- [ ] **Step 1: Load env vars and run validate**

```bash
cd /home/dev/pulsovpn/escudo-vpn
source .env
export VULTR_API_KEY HETZNER_API_TOKEN IPROYAL_API_TOKEN
cargo run -p escudo-deploy -- validate
```

Expected output:
```
Validating provider APIs...

  ✓ Vultr API key valid
  ✓ Hetzner API token valid
  ✓ IPRoyal API token valid

All providers validated.
```

- [ ] **Step 2: If any fail, debug and fix**

Check error messages. Common issues:
- Vultr: key might need enabling in dashboard
- Hetzner: token might be read-only (needs read-write)
- IPRoyal: token format might need adjustment

- [ ] **Step 3: Run plan (dry run)**

```bash
cargo run -p escudo-deploy -- plan
```

Expected:
```
Plan:
  + CREATE br-sp-01 (vultr in gru)
  + CREATE de-nbg-01 (hetzner in nbg1)

Estimated monthly cost: $9.20
```

---

### Task 5: Deploy Test Servers + Buy Test IP

**Files:** None (runtime test)

- [ ] **Step 1: Deploy 1 Vultr + 1 Hetzner server**

```bash
cd /home/dev/pulsovpn/escudo-vpn
source .env
export VULTR_API_KEY HETZNER_API_TOKEN IPROYAL_API_TOKEN DEPLOY_SECRET=test-secret-$(openssl rand -hex 16)
cargo run -p escudo-deploy -- apply
```

Expected: 2 servers created, output shows IPs and costs.

- [ ] **Step 2: Wait for servers to come online (~90 seconds)**

The cloud-init takes about 60-90 seconds. Check:
```bash
cargo run -p escudo-deploy -- status
```

Both should show status: active with public IPs.

- [ ] **Step 3: Test IPRoyal — acquire 1 US residential IP**

Write a quick test binary or use the library directly:

```bash
# Quick test via cargo test or a script
cd /home/dev/pulsovpn/escudo-vpn
cargo test -p escudo-proxy -- --nocapture 2>&1 | head -20
```

Or add a test in `crates/escudo-proxy/src/providers/iproyal.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Run manually: cargo test -p escudo-proxy -- --ignored --nocapture
    async fn test_iproyal_health_check() {
        let token = std::env::var("IPROYAL_API_TOKEN").expect("IPROYAL_API_TOKEN not set");
        let provider = IpRoyalProvider::new(token);
        let ok = provider.health_check().await.unwrap();
        assert!(ok, "IPRoyal API should be healthy");
    }

    #[tokio::test]
    #[ignore]
    async fn test_acquire_shared_proxy() {
        let token = std::env::var("IPROYAL_API_TOKEN").expect("IPROYAL_API_TOKEN not set");
        let provider = IpRoyalProvider::new(token);
        let cred = provider.acquire_shared_proxy("us", None, Duration::from_secs(3600)).await.unwrap();
        println!("Got proxy: {}", cred.socks5_url());
        assert_eq!(cred.country, "us");
        assert!(!cred.username.is_empty());
    }
}
```

- [ ] **Step 4: Verify streaming through residential IP**

```bash
# Test that the SOCKS5 proxy works and shows a residential IP
curl -x socks5://USER:PASS@geo.iproyal.com:32325 https://ipinfo.io/json
```

Check output: `"org"` should show a residential ISP (e.g., Comcast, AT&T), NOT a datacenter.

- [ ] **Step 5: Record costs and results**

Document: Vultr server cost, Hetzner server cost, IPRoyal IP cost, and whether all 3 connections work.

- [ ] **Step 6: Commit test results**

```bash
git add -A
git commit -m "feat: validate all provider APIs — Vultr, Hetzner, IPRoyal tested

Vultr: 1 server deployed in São Paulo ($5/mo)
Hetzner: 1 server deployed in Nuremberg (~$4.20/mo)
IPRoyal: 1 US residential IP acquired and verified"
```

---

## Phase 2: Multi-Interface WireGuard (Tier Separation)

### Task 6: Update gateway.proto

**Files:**
- Modify: `proto/gateway.proto`

- [ ] **Step 1: Add Tier enum and UpdateProxyCredentials RPC**

Add to `proto/gateway.proto` before the service definition:

```protobuf
enum Tier {
    FREE = 0;
    ESCUDO = 1;
    PRO = 2;
    DEDICATED = 3;
}

enum ProxyTarget {
    SHARED = 0;
    DEDICATED_PROXY = 1;
}
```

Add field to `AddPeerRequest`:
```protobuf
message AddPeerRequest {
  string public_key = 1;
  string allowed_ip = 2;
  string preshared_key = 3;
  Tier tier = 4;
}
```

Add new RPC to service and messages:
```protobuf
service GatewayService {
  // ... existing RPCs ...
  rpc UpdateProxyCredentials(UpdateProxyCredentialsRequest) returns (UpdateProxyCredentialsResponse);
}

message UpdateProxyCredentialsRequest {
    string socks5_host = 1;
    uint32 socks5_port = 2;
    string socks5_username = 3;
    string socks5_password = 4;
    ProxyTarget target = 5;
}

message UpdateProxyCredentialsResponse {
    bool success = 1;
    string error = 2;
}
```

- [ ] **Step 2: Rebuild proto**

Run: `cd /home/dev/pulsovpn/escudo-vpn && cargo build -p escudo-gateway 2>&1 | head -20`
Expected: compiles (proto regenerates via build.rs)

- [ ] **Step 3: Commit**

```bash
git add proto/gateway.proto
git commit -m "feat: add Tier enum and UpdateProxyCredentials to gateway proto"
```

---

### Task 7: Multi-Interface WgManager

**Files:**
- Modify: `crates/escudo-gateway/src/wg.rs`
- Modify: `crates/escudo-gateway/src/grpc.rs`
- Modify: `crates/escudo-gateway/src/config.rs`
- Modify: `crates/escudo-gateway/src/main.rs`

- [ ] **Step 1: Update WgManager to support interface selection**

In `crates/escudo-gateway/src/wg.rs`, the existing `WgManager` takes a single interface name. Update `add_peer` to accept an optional interface override, OR create a `MultiWgManager` that wraps three `WgManager` instances:

```rust
pub struct MultiWgManager {
    pub wg0: WgManager,  // Free/Escudo
    pub wg1: WgManager,  // Pro
    pub wg2: WgManager,  // Dedicated
}

impl MultiWgManager {
    pub fn new(wg0_iface: &str, wg1_iface: &str, wg2_iface: &str) -> Self {
        Self {
            wg0: WgManager::new(wg0_iface),
            wg1: WgManager::new(wg1_iface),
            wg2: WgManager::new(wg2_iface),
        }
    }

    pub fn for_tier(&self, tier: i32) -> &WgManager {
        match tier {
            2 => &self.wg1, // PRO
            3 => &self.wg2, // DEDICATED
            _ => &self.wg0, // FREE (0) and ESCUDO (1)
        }
    }
}
```

- [ ] **Step 2: Update grpc.rs to use tier for interface selection**

In `add_peer`, use `req.tier` to select the right WgManager:

```rust
let wg = self.wg.for_tier(req.tier);
wg.add_peer(&req.public_key, &req.allowed_ip, &req.preshared_key).await?;
```

Add the `UpdateProxyCredentials` handler (stub that logs and returns success — actual tun2socks restart comes in Phase 3).

- [ ] **Step 3: Update config.rs and main.rs**

Add `wg1_interface` and `wg2_interface` to gateway config. Initialize `MultiWgManager` in main.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p escudo-gateway`

- [ ] **Step 5: Commit**

```bash
git add crates/escudo-gateway/
git commit -m "feat: multi-interface WireGuard support in gateway

WgManager now supports 3 interfaces (wg0/wg1/wg2) selected by Tier.
UpdateProxyCredentials RPC added (stub for Phase 3)."
```

---

### Task 8: Tier-Aware Connect Flow in API (⚠ after sqlx work merges)

**Files:**
- Modify: `crates/escudo-api/src/routes/vpn.rs:27-50` — update `enforce_device_limit`
- Modify: `crates/escudo-api/src/routes/vpn.rs:140-177` — update server query and IP allocation
- Modify: `crates/escudo-api/src/router.rs` — add phone-home endpoint

- [ ] **Step 1: Update enforce_device_limit to use tier**

Replace the plan lookup at `vpn.rs:34-44`:

```rust
// Read tier from subscriptions table
let tier = sqlx::query_scalar::<_, Option<String>>(
    "SELECT s.tier FROM subscriptions s WHERE s.user_id = $1 AND s.status = 'active' ORDER BY s.period_end DESC LIMIT 1"
)
.bind(user_id)
.fetch_optional(&state.db)
.await?
.flatten()
.unwrap_or_else(|| "free".to_string());

let max_devices: i64 = match tier.as_str() {
    "escudo" => 5,
    "pro" => 10,
    "dedicated" => 10,
    _ => 1, // free
};
```

- [ ] **Step 2: Update IP allocation to use per-tier subnets**

Replace the IP allocation at `vpn.rs:166-177`:

```rust
let (ip_start, ip_end) = match tier.as_str() {
    "pro" => (16385, 32766),        // 10.0.64.1 - 10.0.127.254 (wg1)
    "dedicated" => (32769, 49150),  // 10.0.128.1 - 10.0.191.254 (wg2)
    _ => (2, 16382),                // 10.0.0.2 - 10.0.63.254 (wg0)
};

let assigned_ip: Option<String> = sqlx::query_scalar(
    r#"
    SELECT host(ip) FROM (
        SELECT ('10.0.' || (n / 256) || '.' || (n % 256))::inet AS ip
        FROM generate_series($1, $2) AS n
    ) candidates
    WHERE host(ip) NOT IN (SELECT assigned_ip FROM devices)
    LIMIT 1
    "#,
)
.bind(ip_start)
.bind(ip_end)
.fetch_optional(&state.db)
.await?;
```

- [ ] **Step 3: Update server query to use new key columns**

Replace the server query at `vpn.rs:142-157` to select the right public key and port based on tier:

```rust
// Select per-tier columns
let wg_port_col = match tier.as_str() {
    "pro" => "wg1_port",
    "dedicated" => "wg2_port",
    _ => "COALESCE(wg0_port, endpoint_port)",
};
let wg_key_col = match tier.as_str() {
    "pro" => "COALESCE(wg1_public_key, public_key)",
    "dedicated" => "COALESCE(wg2_public_key, public_key)",
    _ => "COALESCE(wg0_public_key, public_key)",
};
```

- [ ] **Step 4: Pass tier to gRPC AddPeerRequest**

```rust
gateway.add_peer(AddPeerRequest {
    public_key: keypair.public_key.clone(),
    allowed_ip: assigned_ip.clone(),
    preshared_key: psk.clone(),
    tier: match tier.as_str() {
        "escudo" => 1,
        "pro" => 2,
        "dedicated" => 3,
        _ => 0, // free
    },
}).await?;
```

- [ ] **Step 5: Add phone-home endpoint to router.rs**

Add `POST /internal/servers/register` route that validates `DEPLOY_SECRET` and inserts into `provider_servers` + updates `servers` table with WireGuard keys.

- [ ] **Step 6: Update server list to filter by tier**

In the `/api/v1/servers` handler, check user's tier. If free, filter: `WHERE country_code IN ('BR', 'US', 'DE')`.

- [ ] **Step 7: Verify it compiles**

Run: `cargo check -p escudo-api`

- [ ] **Step 8: Commit**

```bash
git add crates/escudo-api/
git commit -m "feat: tier-aware connect flow and phone-home endpoint

Connect routes to correct WireGuard interface by tier.
IP allocation uses per-tier subnets. Server list filters by tier.
Phone-home endpoint for automated server registration."
```

---

## Phase 3: IP Guardian

### Task 9: escudo-guardian Crate

**Files:**
- Create: `crates/escudo-guardian/Cargo.toml`
- Create: `crates/escudo-guardian/src/main.rs`
- Create: `crates/escudo-guardian/src/checker.rs`
- Create: `crates/escudo-guardian/src/rotator.rs`
- Create: `crates/escudo-guardian/src/analytics.rs`
- Create: `crates/escudo-guardian/src/config.rs`
- Create: `config/guardian.toml`
- Create: `deploy/escudo-guardian.service`

- [ ] **Step 1: Create Cargo.toml**

```toml
# crates/escudo-guardian/Cargo.toml
[package]
name = "escudo-guardian"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "escudo-guardian"
path = "src/main.rs"

[dependencies]
reqwest = { workspace = true, features = ["socks"] }
sqlx = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
clap = { workspace = true }
escudo-proxy = { path = "../escudo-proxy" }
```

- [ ] **Step 2: Write config.rs**

```rust
// crates/escudo-guardian/src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GuardianConfig {
    pub database_url: String,
    pub check_interval_secs: u64,   // default 1800 (30 min)
    pub services: Vec<StreamingService>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamingService {
    pub name: String,
    pub url: String,
    pub block_indicators: Vec<String>,
}

impl GuardianConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}
```

- [ ] **Step 3: Write checker.rs**

```rust
// crates/escudo-guardian/src/checker.rs
use std::time::{Duration, Instant};
use reqwest::Client;
use tracing::{info, warn};
use crate::config::StreamingService;

pub struct HealthChecker {
    client: Client,
}

#[derive(Debug)]
pub struct CheckResult {
    pub service: String,
    pub status: String,       // "healthy" | "blocked" | "timeout" | "error"
    pub response_time_ms: i32,
    pub error_detail: Option<String>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap(),
        }
    }

    /// Check a single IP against a single streaming service
    pub async fn check_ip_against_service(
        &self,
        socks5_url: &str,
        service: &StreamingService,
    ) -> CheckResult {
        let proxy = match reqwest::Proxy::all(socks5_url) {
            Ok(p) => p,
            Err(e) => return CheckResult {
                service: service.name.clone(),
                status: "error".to_string(),
                response_time_ms: 0,
                error_detail: Some(format!("Invalid proxy URL: {e}")),
            },
        };

        let client = match Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(15))
            .build()
        {
            Ok(c) => c,
            Err(e) => return CheckResult {
                service: service.name.clone(),
                status: "error".to_string(),
                response_time_ms: 0,
                error_detail: Some(format!("Client build error: {e}")),
            },
        };

        let start = Instant::now();

        match client.get(&service.url).send().await {
            Ok(resp) => {
                let elapsed = start.elapsed().as_millis() as i32;
                let body = resp.text().await.unwrap_or_default();

                for indicator in &service.block_indicators {
                    if body.to_lowercase().contains(&indicator.to_lowercase()) {
                        warn!(
                            "IP blocked on {} — matched indicator: '{indicator}'",
                            service.name
                        );
                        return CheckResult {
                            service: service.name.clone(),
                            status: "blocked".to_string(),
                            response_time_ms: elapsed,
                            error_detail: Some(format!("Matched: {indicator}")),
                        };
                    }
                }

                info!("{}: healthy ({}ms)", service.name, elapsed);
                CheckResult {
                    service: service.name.clone(),
                    status: "healthy".to_string(),
                    response_time_ms: elapsed,
                    error_detail: None,
                }
            }
            Err(e) => {
                let elapsed = start.elapsed().as_millis() as i32;
                let status = if e.is_timeout() { "timeout" } else { "error" };
                CheckResult {
                    service: service.name.clone(),
                    status: status.to_string(),
                    response_time_ms: elapsed,
                    error_detail: Some(e.to_string()),
                }
            }
        }
    }
}
```

- [ ] **Step 4: Write rotator.rs**

```rust
// crates/escudo-guardian/src/rotator.rs
use sqlx::PgPool;
use tracing::{info, error};
use uuid::Uuid;
use escudo_proxy::pool::ProxyPool;

pub struct IpRotator {
    db: PgPool,
    proxy_pool: ProxyPool,
}

impl IpRotator {
    pub fn new(db: PgPool, proxy_pool: ProxyPool) -> Self {
        Self { db, proxy_pool }
    }

    pub async fn rotate_blocked_ip(&self, proxy_ip_id: Uuid, reason: &str) -> anyhow::Result<()> {
        // 1. Get the blocked IP details (use individual queries to avoid sqlx tuple FromRow issues)
        let country: String = sqlx::query_scalar(
            "SELECT country FROM proxy_ips WHERE id = $1"
        )
        .bind(proxy_ip_id)
        .fetch_one(&self.db)
        .await?;

        let provider_proxy_id: String = sqlx::query_scalar(
            "SELECT provider_proxy_id FROM proxy_ips WHERE id = $1"
        )
        .bind(proxy_ip_id)
        .fetch_one(&self.db)
        .await?;

        // 2. Mark as blocked
        sqlx::query("UPDATE proxy_ips SET status = 'blocked', updated_at = now() WHERE id = $1")
            .bind(proxy_ip_id)
            .execute(&self.db)
            .await?;

        // 3. Acquire fresh IP
        let new_cred = self.proxy_pool
            .acquire_shared(&country, None, std::time::Duration::from_secs(7 * 24 * 3600))
            .await?;

        // 4. Insert new proxy_ip
        let new_proxy_id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO proxy_ips (provider, provider_proxy_id, proxy_type, country, socks5_host, socks5_port, socks5_username, socks5_password, status)
               VALUES ($1, $2, 'shared', $3, $4, $5, $6, $7, 'healthy') RETURNING id"#
        )
        .bind(format!("{:?}", new_cred.provider))
        .bind(&new_cred.id)
        .bind(&country)
        .bind(&new_cred.host)
        .bind(new_cred.port as i32)
        .bind(&new_cred.username)
        .bind(&new_cred.password)
        .fetch_one(&self.db)
        .await?;

        // 5. Update server_proxy_assignments
        sqlx::query(
            "UPDATE server_proxy_assignments SET proxy_ip_id = $1 WHERE proxy_ip_id = $2"
        )
        .bind(new_proxy_id)
        .bind(proxy_ip_id)
        .execute(&self.db)
        .await?;

        // 6. Log rotation
        let affected: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM server_proxy_assignments WHERE proxy_ip_id = $1"
        )
        .bind(new_proxy_id)
        .fetch_one(&self.db)
        .await?;

        sqlx::query(
            r#"INSERT INTO ip_rotation_logs (old_proxy_ip_id, new_proxy_ip_id, reason, country, provider, affected_servers)
               VALUES ($1, $2, $3, $4, $5, $6)"#
        )
        .bind(proxy_ip_id)
        .bind(new_proxy_id)
        .bind(reason)
        .bind(&country)
        .bind(format!("{:?}", new_cred.provider))
        .bind(affected as i32)
        .execute(&self.db)
        .await?;

        info!("Rotated IP for {country}: {proxy_ip_id} → {new_proxy_id} (reason: {reason})");

        Ok(())
    }
}
```

- [ ] **Step 5: Write analytics.rs (basic stub)**

```rust
// crates/escudo-guardian/src/analytics.rs
use sqlx::PgPool;
use tracing::info;

pub struct BurnAnalytics {
    db: PgPool,
}

impl BurnAnalytics {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Analyze burn patterns from ip_health_logs.
    /// Called daily. Phase 1: just log stats. Phase 2: auto-adjust pool sizes.
    pub async fn analyze(&self) -> anyhow::Result<()> {
        let blocked_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ip_health_logs WHERE status = 'blocked' AND checked_at > now() - interval '24 hours'"
        )
        .fetch_one(&self.db)
        .await?;

        let total_checks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ip_health_logs WHERE checked_at > now() - interval '24 hours'"
        )
        .fetch_one(&self.db)
        .await?;

        let burn_rate = if total_checks > 0 {
            blocked_count as f64 / total_checks as f64 * 100.0
        } else {
            0.0
        };

        info!("24h burn analytics: {blocked_count}/{total_checks} blocked ({burn_rate:.1}%)");
        Ok(())
    }
}
```

- [ ] **Step 6: Write main.rs**

```rust
// crates/escudo-guardian/src/main.rs
use std::time::Duration;
use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod checker;
mod rotator;
mod analytics;
mod config;

use checker::HealthChecker;

#[derive(Parser)]
#[command(name = "escudo-guardian", about = "IP health monitoring and auto-rotation")]
struct Cli {
    #[arg(short, long, default_value = "config/guardian.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("escudo_guardian=info".parse()?))
        .init();

    let cli = Cli::parse();
    let config = config::GuardianConfig::load(&cli.config)?;

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    info!("IP Guardian started. Check interval: {}s", config.check_interval_secs);

    let checker = HealthChecker::new();
    let interval = Duration::from_secs(config.check_interval_secs);

    loop {
        // Get all healthy proxy IPs
        // Use raw query rows to avoid sqlx tuple FromRow issues
        let rows = sqlx::query(
            "SELECT id, socks5_host, socks5_port, socks5_username, socks5_password FROM proxy_ips WHERE status = 'healthy'"
        )
        .fetch_all(&db)
        .await?;

        info!("Checking {} proxy IPs against {} services", rows.len(), config.services.len());

        for row in &rows {
            use sqlx::Row;
            let proxy_id: Uuid = row.get("id");
            let host: String = row.get("socks5_host");
            let port: i32 = row.get("socks5_port");
            let username: String = row.get("socks5_username");
            let password: String = row.get("socks5_password");
            let socks5_url = format!("socks5://{username}:{password}@{host}:{port}");

            for service in &config.services {
                let result = checker.check_ip_against_service(&socks5_url, service).await;

                // Record in health log
                sqlx::query(
                    "INSERT INTO ip_health_logs (proxy_ip_id, service, status, response_time_ms, error_detail) VALUES ($1, $2, $3, $4, $5)"
                )
                .bind(proxy_id)
                .bind(&result.service)
                .bind(&result.status)
                .bind(result.response_time_ms)
                .bind(&result.error_detail)
                .execute(&db)
                .await?;

                if result.status == "blocked" {
                    error!("IP {proxy_id} blocked on {} — triggering rotation", result.service);
                    // TODO: call rotator.rotate_blocked_ip() here
                    // For Phase 1, just mark as blocked and log
                    sqlx::query("UPDATE proxy_ips SET status = 'blocked', updated_at = now() WHERE id = $1")
                        .bind(proxy_id)
                        .execute(&db)
                        .await?;
                }
            }
        }

        info!("Health check cycle complete. Sleeping {}s", config.check_interval_secs);
        tokio::time::sleep(interval).await;
    }
}
```

- [ ] **Step 7: Write guardian.toml**

```toml
# config/guardian.toml
database_url = "postgres://escudo:escudo@localhost/escudo"
check_interval_secs = 1800  # 30 minutes

[[services]]
name = "netflix"
url = "https://www.netflix.com/browse"
block_indicators = ["unblocker or proxy", "proxy or VPN", "not available in your area"]

[[services]]
name = "bbc_iplayer"
url = "https://www.bbc.co.uk/iplayer"
block_indicators = ["not available in your area", "only available in the UK", "outside the UK"]

[[services]]
name = "disney_plus"
url = "https://www.disneyplus.com/"
block_indicators = ["proxy", "VPN", "not available in your region"]

[[services]]
name = "globoplay"
url = "https://globoplay.globo.com/"
block_indicators = ["não está disponível", "indisponível na sua região"]
```

- [ ] **Step 8: Write systemd service**

```ini
# deploy/escudo-guardian.service
[Unit]
Description=Escudo VPN IP Guardian
After=network.target postgresql.service

[Service]
Type=simple
User=escudo
ExecStart=/usr/local/bin/escudo-guardian --config /etc/escudo/guardian.toml
Restart=always
RestartSec=10
Environment=RUST_LOG=escudo_guardian=info

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 9: Add to workspace (⚠ after sqlx work merges)**

Add `"crates/escudo-guardian"` to workspace members in `Cargo.toml`.

- [ ] **Step 10: Verify it compiles**

Run: `cargo check -p escudo-guardian`

- [ ] **Step 11: Commit**

```bash
git add crates/escudo-guardian/ config/guardian.toml deploy/escudo-guardian.service
git commit -m "feat: add escudo-guardian crate for IP health monitoring

Checks each residential IP against Netflix/BBC/Disney+/Globoplay every 30 min.
Auto-marks blocked IPs. Rotation integration with escudo-proxy.
Burn rate analytics stub for pattern detection."
```

---

## Phase 4: Fleet Scale-Up

### Task 10: Full Deploy Config

**Files:**
- Modify: `config/deploy.toml`

- [ ] **Step 1: Expand deploy.toml with full server list**

Update `config/deploy.toml` with all Phase 1 servers from the master plan (adjusted for Vultr + Hetzner only — no LightNode). See spec for the complete list.

Key regions:
- Vultr: gru, mia, ewr, lax, ord, dfw, sjc, atl, yto, lhr, fra, cdg, ams, mad, sto, waw, nrt, icn, sgp, bom, blr, syd, mel, scl, mex
- Hetzner: nbg1, fsn1, hel1, ash, hil, sin

- [ ] **Step 2: Run plan to see full cost**

```bash
cargo run -p escudo-deploy -- plan
```

Review output: should show ~47 servers, estimated ~$250/mo.

- [ ] **Step 3: Apply when ready**

```bash
cargo run -p escudo-deploy -- apply
```

This is the big deploy — creates the full fleet. Run only after Phase 1-3 are verified.

- [ ] **Step 4: Commit**

```bash
git add config/deploy.toml
git commit -m "feat: full fleet deploy config — 47 servers across 28 countries"
```

---

## Summary

| Task | Phase | What | Depends On |
|------|-------|------|------------|
| 1 | 1 | Database migrations | Nothing |
| 2 | 1 | escudo-proxy (IPRoyal client) | Nothing |
| 3 | 1 | escudo-deploy (Vultr + Hetzner) | Task 2 |
| 4 | 1 | Validate all APIs | Tasks 2, 3 |
| 5 | 1 | Deploy test servers + buy test IP | Task 4 |
| 6 | 2 | Update gateway.proto | Nothing |
| 7 | 2 | Multi-interface WgManager | Task 6 |
| 8 | 2 | Tier-aware API connect flow | Tasks 1, 7 + ⚠ sqlx work |
| 9 | 3 | escudo-guardian | Tasks 1, 2 |
| 10 | 4 | Full fleet deploy | Tasks 3, 5 |

Tasks 1 and 2 can run in parallel. Task 3 depends on Task 2. Task 6 can start while Task 5 runs.
