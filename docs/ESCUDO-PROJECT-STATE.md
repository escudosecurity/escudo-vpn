# Escudo VPN - Comprehensive Project State

**Generated:** 2026-03-24
**Hostname:** escudo-mgmt-01
**Public IP:** 91.99.29.182

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Technology Stack](#2-technology-stack)
3. [Source Code Layout](#3-source-code-layout)
4. [Services & Crates](#4-services--crates)
5. [API Endpoints](#5-api-endpoints)
6. [gRPC Gateway API](#6-grpc-gateway-api)
7. [Database Schema](#7-database-schema)
8. [Database Statistics](#8-database-statistics)
9. [VPN Server Fleet](#9-vpn-server-fleet)
10. [Cloud Provider Infrastructure](#10-cloud-provider-infrastructure)
11. [Proxy IP Pool](#11-proxy-ip-pool)
12. [WireGuard Configuration](#12-wireguard-configuration)
13. [Configuration Files](#13-configuration-files)
14. [Systemd Services](#14-systemd-services)
15. [Nginx Reverse Proxy](#15-nginx-reverse-proxy)
16. [Firewall & Networking](#16-firewall--networking)
17. [Deployment & Provisioning Scripts](#17-deployment--provisioning-scripts)
18. [Audit & Load Testing](#18-audit--load-testing)
19. [Client Applications](#19-client-applications)
20. [Website & Marketing](#20-website--marketing)
21. [Monitoring & Observability](#21-monitoring--observability)
22. [Current Runtime State](#22-current-runtime-state)
23. [Known Issues & What's Broken](#23-known-issues--whats-broken)
24. [What's Working](#24-whats-working)
25. [Environment & Secrets](#25-environment--secrets)

---

## 1. Architecture Overview

Escudo VPN is a Brazilian VPN service with a microservices architecture built primarily in Rust. The system consists of:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────────┐
│  Android App │     │  Windows App │     │   Website/Site   │
│  (Kotlin)    │     │  (Tauri/TS)  │     │   (Static HTML)  │
└──────┬───────┘     └──────┬───────┘     └────────┬─────────┘
       │                    │                      │
       └────────────┬───────┘──────────────────────┘
                    │ HTTPS (port 80/443)
                    ▼
              ┌───────────┐
              │   Nginx   │  Reverse proxy + rate limiting
              └─────┬─────┘
                    │ :3000
                    ▼
          ┌─────────────────┐         ┌──────────────────┐
          │   escudo-api    │◄───────►│  escudo-admin    │
          │  (Axum REST)    │         │  (Admin API)     │
          │  Port 3000      │         │  Port 3000       │
          └────────┬────────┘         └──────────────────┘
                   │ gRPC :9090
                   ▼
          ┌─────────────────┐    ┌──────────────────┐    ┌──────────────┐
          │ escudo-gateway  │    │   escudo-dns     │    │escudo-sniproxy│
          │ (WireGuard mgr) │    │  (DNS filter)    │    │(Streaming)   │
          │ gRPC :9090      │    │  10.0.0.1:53     │    │10.0.0.1:443  │
          │ Health :8080    │    │  Metrics :9153   │    └──────────────┘
          └────────┬────────┘    └──────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
    ┌───────┐  ┌───────┐  ┌───────┐
    │  wg0  │  │  wg1  │  │  wg2  │
    │ FREE  │  │ESCUDO │  │  PRO  │
    │:51820 │  │:51821 │  │:51822 │
    └───────┘  └───────┘  └───────┘

          ┌─────────────────┐    ┌──────────────────┐
          │ escudo-guardian  │    │ escudo-deploy    │
          │ (IP health)     │    │ (Infra mgmt)     │
          │ Port 3011       │    │ CLI tool         │
          └─────────────────┘    └──────────────────┘

                    ┌──────────────────┐
                    │   PostgreSQL 16  │
                    │   Port 5432      │
                    │   DB: escudo     │
                    └──────────────────┘
```

---

## 2. Technology Stack

| Layer | Technology |
|-------|-----------|
| **Backend** | Rust (Tokio async, Axum web framework) |
| **Database** | PostgreSQL 16 (via SQLx 0.8.6) |
| **VPN Protocol** | WireGuard (via BoringTun userspace) |
| **Inter-service** | gRPC (Tonic 0.12 + Prost 0.13) |
| **DNS** | Hickory DNS (server + resolver) |
| **Crypto** | X25519-Dalek, Argon2, AES-256-GCM, HMAC-SHA256 |
| **Quantum Resistance** | Rosenpass (PSK rotation) |
| **HTTP Client** | Reqwest (rustls-tls) |
| **Android** | Kotlin + Gradle + Hilt DI |
| **Windows** | Tauri (Rust + TypeScript/Svelte) |
| **Web Server** | Nginx (reverse proxy) |
| **Billing** | Stripe + PIX (Brazilian payment) |
| **Proxy Providers** | IPRoyal, ProxyCheap, Webshare |
| **Cloud Providers** | Hetzner, Vultr, DigitalOcean |
| **Observability** | Tracing, Prometheus, VictoriaMetrics, Grafana |

---

## 3. Source Code Layout

### Repository: `/home/dev/pulsovpn/escudo-vpn/`
### Deployed: `/opt/escudo/`

```
/opt/escudo/
├── Cargo.toml                  # Rust workspace manifest
├── Cargo.lock                  # Dependency lock
├── .env                        # Environment secrets
├── crates/                     # 13 Rust microservices
│   ├── escudo-api/             # REST API server
│   ├── escudo-gateway/         # WireGuard VPN gateway
│   ├── escudo-admin/           # Admin management API
│   ├── escudo-dns/             # DNS server + filtering
│   ├── escudo-deploy/          # Infrastructure provisioning CLI
│   ├── escudo-provisioner/     # Server bootstrap automation
│   ├── escudo-guardian/        # IP health monitoring & rotation
│   ├── escudo-ip-manager/      # Proxy IP pool management
│   ├── escudo-sniproxy/        # SNI-based streaming proxy
│   ├── escudo-client/          # Client tunnel library (FFI)
│   ├── escudo-proxy/           # Proxy provider abstraction
│   ├── escudo-common/          # Shared utilities (config, crypto, JWT)
│   └── escudo-db/              # SQLx database wrapper
├── config/                     # TOML config files
├── deploy/                     # Systemd services, nginx, setup scripts
├── migrations/                 # 20 PostgreSQL migrations
├── proto/                      # gRPC protobuf definitions
├── scripts/                    # Audit, load test, provisioning scripts
├── apps/                       # Client applications
│   ├── android/                # Android app (Kotlin)
│   └── windows/                # Windows app (Tauri)
├── site/                       # Marketing website (Portuguese)
├── docs/                       # Documentation & specs
├── audits/                     # 95+ test run results
├── generated-assets/           # Marketing images
├── seo-generator/              # Python SEO page generator
└── target/                     # Rust build artifacts (3.3GB)
```

---

## 4. Services & Crates

### escudo-api (Main REST API)
- **Port:** 3000
- **Binary:** `/opt/escudo/target/release/escudo-api`
- **Config:** `/etc/escudo/api.toml`
- **Source:** `crates/escudo-api/src/`
- **Key files:** `main.rs`, `router.rs` (215 lines, 50+ endpoints), `state.rs`, `middleware.rs`
- **Route modules:** `auth.rs`, `anon_auth.rs`, `account.rs`, `vpn.rs`, `billing.rs`, `billing_pix.rs`, `stats.rs`, `ws.rs`, `favorites.rs`, `settings.rs`, `profiles.rs`, `referral.rs`, `family.rs`, `security.rs`, `internal.rs`
- **Features:** JWT auth, WireGuard peer mgmt via gRPC, Stripe/PIX billing, HIBP breach checking, WebSocket stats

### escudo-gateway (VPN Gateway)
- **Ports:** 9090 (gRPC), 8080 (health)
- **Binary:** `/opt/escudo/target/release/escudo-gateway`
- **Config:** `/etc/escudo/gateway.toml`
- **Source:** `crates/escudo-gateway/src/`
- **Key files:** `main.rs`, `grpc.rs`, `wg.rs` (multi-interface), `proxy.rs`, `stats.rs`, `health.rs`
- **Features:** Multi-interface WireGuard (wg0/wg1/wg2), peer management, statistics collection, proxy integration

### escudo-admin (Admin Dashboard)
- **Port:** 3000 (shared with API, separate routes)
- **Source:** `crates/escudo-admin/src/`
- **Routes:** `/admin/v1/users`, `/admin/v1/servers`, `/admin/v1/tenants`, `/admin/v1/stats`, `/admin/v1/dashboard`

### escudo-dns (DNS Server)
- **Bind:** 10.0.0.1:53
- **Metrics:** Port 9153
- **Config:** `/etc/escudo/dns.toml`
- **Source:** `crates/escudo-dns/src/`
- **Key files:** `server.rs`, `handler.rs`, `blocklist.rs`, `stats.rs`
- **Upstream:** Cloudflare DoH (1.1.1.1)
- **Blocklists:** HaGeZi, URLhaus, phishing-filter, threat IPs (24h refresh)

### escudo-sniproxy (Streaming Proxy)
- **Bind:** 10.0.0.1:443
- **Config:** `/etc/escudo/sniproxy.toml`
- **Streaming targets:** Netflix, Globoplay, Disney+, HBO Max, Paramount+, Prime Video
- **Function:** SNI-based routing for geo-unblocking

### escudo-guardian (IP Health Monitor)
- **Port:** 3011
- **Binary:** Currently running **debug** build
- **Source:** `crates/escudo-guardian/src/`
- **Key files:** `checker.rs`, `rotator.rs`, `analytics.rs`
- **Monitors:** Netflix, BBC iPlayer, Disney+, Globoplay
- **Check interval:** 30 minutes

### escudo-deploy (Infrastructure CLI)
- **Subcommands:** validate, plan, apply, status, destroy
- **Providers:** Hetzner, Vultr
- **Source:** `crates/escudo-deploy/src/`
- **Key files:** `providers/hetzner.rs`, `providers/vultr.rs`, `ssh.rs`, `cloudinit.rs`, `reconciler.rs`

### escudo-provisioner (Server Bootstrap)
- **Source:** `crates/escudo-provisioner/src/main.rs` (20KB)
- **Function:** Provisions tunnel nodes, allocates WireGuard subnets, registers in DB

### escudo-ip-manager (Proxy Pool Manager)
- **Source:** `crates/escudo-ip-manager/src/main.rs`
- **Providers:** IPRoyal API integration

### escudo-client (VPN Client Library)
- **Source:** `crates/escudo-client/src/`
- **Key files:** `tunnel.rs`, `config.rs`, `killswitch.rs`
- **Function:** Cross-platform WireGuard tunnel (FFI for mobile/desktop)

### escudo-proxy (Proxy Abstraction)
- **Source:** `crates/escudo-proxy/src/`
- **Providers:** IPRoyal (`iproyal.rs`), ProxyCheap (`proxycheap.rs`)
- **Features:** Connection pooling, credential management

### escudo-common (Shared Library)
- **Source:** `crates/escudo-common/src/`
- **Modules:** `config.rs`, `crypto.rs` (X25519 + AES-256-GCM), `jwt.rs`, `error.rs`, `models.rs`, `types.rs`

### escudo-db (Database Layer)
- **Source:** `crates/escudo-db/src/lib.rs`
- **Function:** Re-exports SQLx with PostgreSQL support

---

## 5. API Endpoints

All under `/api/v1/`:

### Authentication
| Method | Path | Description |
|--------|------|-------------|
| POST | `/auth/register` | Email/password registration |
| POST | `/auth/login` | Email/password login |
| POST | `/auth/anon` | Anonymous account creation |
| POST | `/auth/qr/generate` | Generate QR login token |
| POST | `/auth/qr/redeem` | Redeem QR login token |

### Account Management
| Method | Path | Description |
|--------|------|-------------|
| GET | `/account` | Get current account |
| PUT | `/account` | Update account |
| DELETE | `/account` | Delete account |

### VPN
| Method | Path | Description |
|--------|------|-------------|
| GET | `/vpn/servers` | List VPN servers |
| POST | `/vpn/connect` | Connect to server (creates WG peer) |
| POST | `/vpn/disconnect` | Disconnect from server |
| GET | `/vpn/status` | Connection status |
| POST | `/vpn/multihop` | Multi-hop connection |

### Billing
| Method | Path | Description |
|--------|------|-------------|
| POST | `/billing/subscribe` | Create Stripe subscription |
| POST | `/billing/portal` | Stripe customer portal |
| POST | `/billing/webhook` | Stripe webhook handler |
| POST | `/billing/pix/create` | Create PIX payment |
| POST | `/billing/pix/webhook` | PIX payment webhook |

### Statistics & WebSocket
| Method | Path | Description |
|--------|------|-------------|
| GET | `/stats/dns` | DNS query statistics |
| GET | `/stats/usage` | Bandwidth usage |
| WS | `/ws/stats` | Real-time stats stream |

### Favorites, Settings, Profiles
| Method | Path | Description |
|--------|------|-------------|
| GET/POST/DELETE | `/favorites` | Server favorites |
| GET/PUT | `/settings` | User settings |
| GET/POST/PUT/DELETE | `/profiles` | Connection profiles |

### Security & Family
| Method | Path | Description |
|--------|------|-------------|
| POST | `/security/breach-check` | HIBP breach checking |
| GET | `/security/pastes` | Paste monitoring |
| GET/POST/PUT | `/family/profiles` | Family content filtering |

### Referral
| Method | Path | Description |
|--------|------|-------------|
| GET/POST | `/referral` | Referral code management |

### Internal
| Method | Path | Description |
|--------|------|-------------|
| POST | `/internal/deploy` | Deployment trigger (deploy-secret auth) |

---

## 6. gRPC Gateway API

**Proto:** `/opt/escudo/proto/gateway.proto`

```protobuf
service GatewayService {
  rpc AddPeer(AddPeerRequest) returns (AddPeerResponse);
  rpc RemovePeer(RemovePeerRequest) returns (RemovePeerResponse);
  rpc ListPeers(ListPeersRequest) returns (ListPeersResponse);
  rpc GetStats(GetStatsRequest) returns (GetStatsResponse);
  rpc AddMultihopPeer(AddMultihopPeerRequest) returns (AddMultihopPeerResponse);
  rpc UpdateProxyCredentials(UpdateProxyCredentialsRequest) returns (UpdateProxyCredentialsResponse);
}
```

**Tiers:** FREE (0), ESCUDO (1), PRO (2), DEDICATED (3)
**Proxy Targets:** SHARED (0), DEDICATED_PROXY (1)

---

## 7. Database Schema

**Database:** PostgreSQL 16 at `postgresql://escudo:escudo_secret@localhost/escudo`

### 23 Tables (22 application + 1 migration tracker)

#### users
| Column | Type | Nullable |
|--------|------|----------|
| id | uuid | NO |
| email | varchar | NO |
| password_hash | varchar | NO |
| role | varchar | NO |
| tenant_id | uuid | YES |
| is_active | boolean | NO |
| subscription_plan | varchar | NO |
| created_at / updated_at | timestamptz | NO |

#### accounts
| Column | Type | Nullable |
|--------|------|----------|
| account_number | varchar | NO (PK) |
| email | varchar | YES |
| tier | varchar | YES |
| status | varchar | YES |
| dedicated_ip_id | uuid | YES |
| devices_count | integer | YES |
| created_at / paid_until | timestamptz | YES |

#### servers
| Column | Type | Nullable |
|--------|------|----------|
| id | uuid | NO |
| name / location / city / country_name | varchar | varies |
| public_ip | varchar | NO |
| public_key | varchar | NO |
| endpoint_port | integer | NO |
| capacity_max | integer | NO |
| is_active / is_virtual | boolean | NO |
| gateway_grpc_addr | varchar | YES |
| wg0_public_key / wg0_port | text/int | YES |
| wg1_public_key / wg1_port | text/int | YES |
| wg2_public_key / wg2_port | text/int | YES |
| country_code | text | YES |
| latitude / longitude | float8 | YES |
| tunnel_ipv4_cidr / tunnel_ipv4_gateway | text | YES |

#### devices
| Column | Type | Nullable |
|--------|------|----------|
| id | uuid | NO |
| user_id / server_id | uuid | NO |
| name | varchar | NO |
| public_key / preshared_key | varchar | NO |
| assigned_ip | varchar | NO |
| private_key_encrypted | varchar | NO |
| is_active | boolean | NO |

#### subscriptions
| Column | Type | Nullable |
|--------|------|----------|
| id / user_id | uuid | NO |
| stripe_customer_id / stripe_subscription_id | varchar | NO |
| plan / status / tier | varchar | NO |
| period_start / period_end | timestamptz | NO |
| bandwidth_limit_bytes | bigint | NO |

#### proxy_ips
| Column | Type | Nullable |
|--------|------|----------|
| id | uuid | NO |
| provider / provider_proxy_id / proxy_type | text | NO |
| country / city | text | NO/YES |
| socks5_host / socks5_port / socks5_username / socks5_password | text/int | NO |
| external_ip | text | YES |
| status | text | NO |
| assigned_user_id | uuid | YES |
| max_concurrent / current_concurrent | integer | YES |
| last_health_check | timestamptz | YES |

#### Other Tables
- **dns_stats** - Per-client daily DNS query/block counts
- **blocked_domains** - User/system domain blocklist
- **favorites** - User favorite servers
- **user_settings** - Kill switch, auto-connect, split tunnel, protocol prefs
- **connection_profiles** - Named connection configs with split tunnel apps
- **qr_tokens** - QR-based auth tokens with expiration
- **referrals** - Referral codes + reward tracking
- **server_chains** - Multi-hop entry/exit server pairs
- **device_fingerprints** - Android device identification
- **usage_logs** - Per-device bandwidth (rx/tx bytes)
- **provider_servers** - Cloud instance metadata + costs
- **server_proxy_assignments** - Server-to-proxy IP mapping
- **ip_health_logs** - Per-service proxy health status
- **ip_health_checks** - Netflix/regional status per proxy
- **ip_rotation_logs** - IP rotation history with reason
- **tenants** - Multi-tenant organization support
- **family_profiles** - *(in migrations but may not be deployed yet)*

---

## 8. Database Statistics

| Table | Row Count | Size |
|-------|-----------|------|
| users | 98 | 48 KB |
| devices | 389 | 248 KB |
| servers | 32 | 48 KB |
| subscriptions | 58 | 48 KB |
| proxy_ips | 319 | 168 KB |
| provider_servers | 9 | 16 KB |
| ip_health_checks | ~many | 48 KB |

**User Breakdown:**
- 97 users: role=user, plan=free, active
- 1 user: role=user, plan=dedicated, active

---

## 9. VPN Server Fleet

32 servers across 24 countries, all active:

| Server | Location | Country | IP | Virtual |
|--------|----------|---------|----|---------|
| sp-01 | Sao Paulo | BR | 216.238.111.108 | No |
| escudo-sp-02 | Sao Paulo | BR | 38.60.242.127 | No |
| escudo-proof-vlt-01 | Sao Paulo | BR | 216.238.110.104 | Yes |
| escudo-clean-test-vlt | Sao Paulo | BR | 216.238.122.235 | Yes |
| nj-01 | New Jersey | US | 144.202.7.232 | No |
| escudo-ashburn | Ashburn | US | 178.156.140.98 | No |
| escudo-hillsboro | Hillsboro | US | 5.78.149.17 | No |
| ams-01 | Amsterdam | NL | 95.179.158.197 | No |
| escudo-nuremberg | Nuremberg | DE | 188.245.32.41 | No |
| escudo-falkenstein | Falkenstein | DE | 91.99.191.227 | No |
| escudo-auto-test-01 | Nuremberg | DE | 178.104.98.107 | Yes |
| escudo-clean-test-hzn | Nuremberg | DE | 178.104.62.115 | Yes |
| escudo-lon-01 | London | GB | 103.13.208.14 | No |
| 178.128.38.189 | London | GB | 178.128.38.189 | No |
| escudo-tor-01 | Toronto | CA | 103.54.59.199 | No |
| 138.197.133.115 | Toronto | CA | 138.197.133.115 | No |
| escudo-helsinki | Helsinki | FI | 204.168.145.177 | No |
| escudo-tyo-01 | Tokyo | JP | 130.94.117.205 | No |
| 104.248.145.138 | Singapore | SG | 104.248.145.138 | No |
| 159.65.149.0 | Bangalore | IN | 159.65.149.0 | No |
| escudo-dxb-01 | Dubai | AE | 130.94.45.199 | No |
| escudo-bue-01 | Buenos Aires | AR | 130.94.107.110 | No |
| escudo-bog-01 | Bogota | CO | 130.94.105.197 | No |
| escudo-lim-01 | Lima | PE | 149.104.66.155 | No |
| escudo-mad-01 | Madrid | ES | 103.45.245.67 | No |
| escudo-mil-01 | Milan | IT | 45.147.250.197 | No |
| escudo-ath-01 | Athens | GR | 38.54.29.167 | No |
| escudo-sto-01 | Stockholm | SE | 45.248.37.183 | No |
| escudo-bkk-01 | Bangkok | TH | 38.60.233.202 | No |
| escudo-jkt-01 | Jakarta | ID | 38.60.191.52 | No |
| escudo-sgn-01 | Ho Chi Minh | VN | 38.54.14.180 | No |
| 134.199.153.98 | Sydney | AU | 134.199.153.98 | No |

---

## 10. Cloud Provider Infrastructure

| Provider | Region | Plan | Status | IP | Label |
|----------|--------|------|--------|----|-------|
| DigitalOcean | Bangalore, India | custom | active | 159.65.149.0 | escudo-blr1 |
| DigitalOcean | London, UK | custom | active | 178.128.38.189 | escudo-lon1 |
| DigitalOcean | Singapore | custom | active | 104.248.145.138 | escudo-sgp1 |
| DigitalOcean | Sydney, Australia | custom | active | 134.199.153.98 | escudo-syd1 |
| DigitalOcean | Toronto, Canada | custom | active | 138.197.133.115 | escudo-tor1 |
| Hetzner | nbg1 | cx23 | running | 178.104.98.107 | escudo-auto-test-01 |
| Hetzner | nbg1 | cx23 | running | 178.104.62.115 | escudo-clean-test-hzn |
| Hetzner | sao | vc2-1c-1gb | running | 216.238.122.235 | escudo-clean-test-vlt |
| Hetzner | sao | vc2-1c-1gb | running | 216.238.110.104 | escudo-proof-vlt-01 |

Many servers (Vultr-hosted) are registered in `servers` table but not tracked in `provider_servers`.

---

## 11. Proxy IP Pool

**Total:** 319 proxy IPs across 22 countries

| Country | Healthy | Burned | Other | Type |
|---------|---------|--------|-------|------|
| BR | 186 shared + 6 dedicated | 5 shared + 1 dedicated | 2 backup, 1 unhealthy | Mixed |
| DE | 40 | 5 | - | Shared |
| US | 17 | 4 | - | Shared |
| CA | 8 | 2 | - | Shared |
| GB | 6 | 2 | - | Shared |
| FI | 3 | 1 | - | Shared |
| All others | 1 each | 1 each | - | Shared |

**Provider:** IPRoyal (primary)
**Status Summary:** ~280 healthy, ~30 burned, few backup/unhealthy

---

## 12. WireGuard Configuration

### Multi-Interface Setup (Tier-based)

| Interface | Subnet | Port | Tier |
|-----------|--------|------|------|
| wg0 | 10.0.0.1/18 (10.0.0.0 - 10.0.63.255) | 51820 | FREE |
| wg1 | 10.0.64.1/18 (10.0.64.0 - 10.0.127.255) | 51821 | ESCUDO |
| wg2 | 10.0.128.1/18 (10.0.128.0 - 10.0.191.255) | 51822 | PRO/DEDICATED |

**Config location:** `/etc/wireguard/wg{0,1,2}.conf`
**Peers:** Managed dynamically by escudo-gateway (not in config files)
**NAT:** nftables masquerade via PostUp rules
**Key encryption:** X25519 + AES-256-GCM (private keys encrypted at rest in DB)

### Kernel Tuning (`sysctl-wireguard.conf`)
```
net.core.rmem_max = 26214400
net.core.wmem_max = 26214400
net.core.netdev_max_backlog = 10000
net.ipv4.ip_forward = 1
net.ipv4.tcp_fastopen = 3
net.ipv4.tcp_mtu_probing = 1
net.netfilter.nf_conntrack_max = 131072
```

---

## 13. Configuration Files

### `/etc/escudo/api.toml`
```toml
[server]
host = "0.0.0.0"
port = 3000

[database]
url = "postgresql://escudo:escudo_secret@localhost/escudo"

[gateway]
grpc_addr = "http://127.0.0.1:9090"

[jwt]
secret = "..."
expiration_hours = 72

[wireguard]
server_public_key = "..."
endpoint = "api.escudovpn.com:51820"
dns = "10.0.0.1"

[stripe]
secret_key = "sk_live_..."
publishable_key = "pk_live_..."
webhook_secret = "whsec_..."

[testing]
open_server_access = true        # WARNING: open in current config
disable_device_limits = true     # WARNING: limits disabled
```

### `/etc/escudo/gateway.toml`
```toml
[grpc]
addr = "127.0.0.1:9090"

[health]
addr = "127.0.0.1:8080"

[wireguard]
subnet = "10.0.0.0/16"
ip_range_start = "10.0.0.2"
ip_range_end = "10.0.255.254"

[proxy]
enabled = true

[stats]
interval_secs = 30
```

### `/etc/escudo/dns.toml`
```toml
listen = "10.0.0.1:53"
upstream_doh = "https://1.1.1.1/dns-query"
blocklist_refresh_hours = 24
# Blocklist sources: HaGeZi, URLhaus, phishing-filter, threat IPs
```

### `/etc/escudo/guardian.env`
- Rotation enabled
- IPRoyal token configured
- Uses debug binary of escudo-ip-manager

### Other configs: `sniproxy.toml`, `rosenpass.toml`, `deploy.toml`, `proxy.toml`

---

## 14. Systemd Services

| Service | Status | Binary | User | Notes |
|---------|--------|--------|------|-------|
| escudo-api | **Running** | release | escudo | ESCUDO_SKIP_MIGRATIONS=true |
| escudo-gateway | **Running** | release | root | CAP_SYS_MODULE for WireGuard |
| escudo-guardian | **Running** | **debug** | root | RestartSec=30, DNS issues |
| wg-quick@wg0 | **Active (exited)** | - | root | FREE tier |
| wg-quick@wg1 | **Active (exited)** | - | root | ESCUDO tier |
| wg-quick@wg2 | **Active (exited)** | - | root | PRO tier |
| nginx | **Running** | - | root | 4 workers |
| postgresql@16-main | **Running** | - | postgres | - |
| escudo-dns | **Not installed** | - | - | Service file exists but not deployed here |
| escudo-sniproxy | **Not installed** | - | - | Service file exists but not deployed here |
| escudo-admin | **Not installed** | - | - | Service file exists but not deployed here |
| escudo-rosenpass | **Not installed** | - | - | Service file exists but not deployed here |
| escudo-victoriametrics | **Not installed** | - | - | Service file exists but not deployed here |

### Service file override
`/etc/systemd/system/escudo-api.service.d/deploy-secret.conf` injects the `DEPLOY_SECRET` env var.

---

## 15. Nginx Reverse Proxy

**Config:** `/etc/nginx/sites-enabled/escudo-api`

```nginx
server {
    listen 80;
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

**Note:** TLS/HTTPS is NOT configured at the nginx level on this management node. The deploy template (`nginx-escudo.conf`) includes Let's Encrypt TLS, rate limiting, WebSocket support, and security headers for production gateway nodes.

### Production nginx template features:
- Rate limiting: 3 req/s for auth, 10 req/s for API
- WebSocket upgrade for `/api/v1/ws/`
- Security headers: CSP, HSTS, X-Frame-Options, X-Content-Type-Options

---

## 16. Firewall & Networking

### UFW + iptables
- **Default INPUT:** DROP
- **Allowed inbound:** SSH (22), HTTP (80), HTTPS (443)
- **PostgreSQL (5432):** Allowed from ~28 specific IPs only
- **WireGuard (51820-51822):** Allowed via UFW

### NAT Rules
- Masquerade for debug/test namespaces: 172.31.201.0/24, 172.31.222-225.0/24
- WireGuard NAT via nftables `escudo-nat` table (PostUp/PostDown)

### Debug Network Namespaces
- `debugns` / `debugns2` / `debugns3` with veth pairs for testing

---

## 17. Deployment & Provisioning Scripts

### `deploy/setup-gateway.sh` - Initial tunnel node setup
- Installs WireGuard, generates keys
- Configures IP forwarding + kernel tuning
- Sets up iptables NAT + firewall

### `deploy/setup-rosenpass.sh` - Quantum-resistant PSK
- Downloads rosenpass v0.2.2 binary
- Generates server keypair
- Enables systemd service (PSK rotation every 120s)

### `scripts/provision/harden-tunnel-node.sh` - Full node hardening
- Disables IPv6 (leak prevention)
- Creates 3 WireGuard interfaces (wg0/wg1/wg2)
- Configures nftables NAT with interval sets
- Installs dnsmasq, jq, curl, nftables
- Disables swap, tunes kernel params

### `configs/selective-routing-sp01.sh` - Selective routing
- Pulls ASN prefixes from BGPview API
- Routes streaming IPs through VPN, others directly
- Supports fwmark 0x1 for selective routing

---

## 18. Audit & Load Testing

### Scripts (`scripts/audit/` and `scripts/load/`)

| Script | Purpose |
|--------|---------|
| `run_single_br_proof.sh` | End-to-end Brazil node proof (creates account, connects, tests Netflix/Globoplay) |
| `backend_control_plane_audit.sh` | API server audit |
| `backend_data_plane_sample.sh` | Gateway data plane stress test |
| `fleet_full_audit_v3.sh` | Full fleet audit across all servers |
| `lockdown_premium_ports.sh` | Security hardening for premium ports |
| `proof_client_harness.sh` | Client proof testing |
| `k6-api-smoke.js` | API smoke load testing (k6) |
| `k6-connect-cycle.js` | VPN connection cycle load testing |
| `wg-peer-load.sh` | WireGuard peer scale testing |
| `run_dns_stress.sh` | DNS query load testing |
| `run_launch_stress.sh` | Launch storm testing |
| `distributed_probe_batch.sh` | Multi-server connectivity probes |

### Audit Results (`audits/`)
95+ test run directories including:
- Fleet audits, peer scale tests (10/20 peers)
- DNS stress tests, launch stress tests
- Single Brazil node proofs
- Distributed probe batches
- Backend control/data plane audits
- Selective routing tests
- E2E automation tests

---

## 19. Client Applications

### Android (`apps/android/`)
- **Language:** Kotlin
- **Build:** Gradle 8.2.0, Kotlin 1.9.20
- **DI:** Hilt
- **Locale:** Portuguese (pt-BR)
- **Components:** TunnelManager, AuthInterceptor
- **APK:** `site/escudo-vpn.apk` (17.9 MB)
- **AAB:** `site/escudo-vpn.aab` (9.1 MB)
- **Signing:** `escudo-upload.jks` keystore

### Windows (`apps/windows/`)
- **Framework:** Tauri (Rust backend + TypeScript frontend)
- **App ID:** com.pulsovpn.escudo
- **Window:** 420x700 (min 380x600)
- **Dev port:** 1420

---

## 20. Website & Marketing

### Static Site (`site/`)
- **Language:** Portuguese (pt-BR)
- **Size:** ~306 MB total

**Content Pages:**
- Homepage, download, comparativo, bloqueador-de-anuncios
- o-que-e-vpn, servidores, sobre, ajuda
- teste-de-privacidade, teste-de-velocidade, vazamentos
- verificar-senha, verificar-link, meu-ip, scanner
- privacy policy, termos

**Sections:** blog/, casos-de-uso/, comparativo/, seguranca-digital/, recursos/

**SEO:**
- sitemaps (index, main, blog, SEO)
- `seo-generator/` - Python tool for automated landing pages per Brazilian state
- robots.txt configured

**Generated Assets:** ads, hero images, logos, OG images, social media graphics

---

## 21. Monitoring & Observability

| Component | Endpoint | Status |
|-----------|----------|--------|
| Gateway health | 127.0.0.1:8080/metrics | Active |
| DNS metrics | Port 9153 (Prometheus) | Not deployed on mgmt node |
| VictoriaMetrics | Service file exists | Not deployed on mgmt node |
| Grafana dashboard | `deploy/grafana-dashboard.json` | Template only |
| Logging | systemd journal (`RUST_LOG=info`) | Active |
| Metrics scrape config | `deploy/victoriametrics.yml` | Template only |

---

## 22. Current Runtime State

### Running on this node (escudo-mgmt-01):

| Component | PID | Build | Port | Status |
|-----------|-----|-------|------|--------|
| escudo-api | 111595 | release | 3000 | Healthy |
| escudo-gateway | 104996 | release | 9090/8080 | Healthy |
| escudo-guardian | 87124 | **debug** | 3011 | Running with errors |
| nginx | 26846 | - | 80 | Healthy |
| PostgreSQL 16 | - | - | 5432 | Healthy |
| WireGuard wg0 | - | - | 51820 | UP, 0 peers |
| WireGuard wg1 | - | - | 51821 | UP, 0 peers |
| WireGuard wg2 | - | - | 51822 | UP, 0 peers |

### Not running on this node:
- escudo-dns (service not installed)
- escudo-sniproxy (service not installed)
- escudo-admin (service not installed)
- escudo-rosenpass (service not installed)
- VictoriaMetrics (service not installed)

---

## 23. Known Issues & What's Broken

### Critical
1. **escudo-guardian DNS resolution failures** - `apid.iproyal.com` fails with "Temporary failure in name resolution". IPRoyal API calls fail, proxy IP rotations are broken. Proxies being "marked burned" and immediate rotation attempts fail.

2. **escudo-guardian running debug build** - `/opt/escudo/target/debug/escudo-guardian` instead of release. Performance impact for a long-running service.

### Warnings
3. **Testing flags enabled in production config** - `api.toml` has `open_server_access = true` and `disable_device_limits = true`. This bypasses access controls and device limits.

4. **No TLS on nginx** - Management node serves HTTP only (port 80). No Let's Encrypt certificates configured despite template existing.

5. **Zero WireGuard peers** - All 3 interfaces show 0 connected peers. Either no clients connected or this is purely a management/API node.

6. **PostgreSQL exposed on 0.0.0.0:5432** - While UFW restricts to ~28 IPs, binding to all interfaces increases attack surface.

7. **Secrets in .env file** - Production API keys (Vultr, Hetzner, IPRoyal, Stripe, etc.) stored in plaintext `.env` file in the repo.

8. **ESCUDO_SKIP_MIGRATIONS=true** - API service skips database migrations on startup. New migrations must be applied manually.

9. **Some server names are raw IPs** - Servers like `178.128.38.189`, `138.197.133.115`, `159.65.149.0`, `134.199.153.98` use IP addresses as names instead of proper labels.

10. **Missing country codes** - Servers `ams-01` and `nj-01` have NULL country_code.

### Not Deployed (service files exist but not active on this node)
- escudo-dns, escudo-sniproxy, escudo-admin, escudo-rosenpass, escudo-victoriametrics

---

## 24. What's Working

1. **Core API** - escudo-api running on release build, serving REST endpoints on port 3000
2. **VPN Gateway** - escudo-gateway managing 3 WireGuard interfaces via gRPC
3. **Database** - PostgreSQL 16 with 23 tables, 98 users, 32 servers, 319 proxy IPs
4. **WireGuard** - All 3 interfaces (wg0/wg1/wg2) are UP with correct subnets
5. **Nginx** - Reverse proxy routing traffic to API
6. **Firewall** - UFW active with proper rules
7. **Server fleet** - 32 servers across 24 countries registered and active
8. **Proxy pool** - 319 IPs managed, ~280 healthy across 22 countries
9. **Build system** - Rust workspace compiles all 13 crates
10. **Billing** - Stripe + PIX integration implemented
11. **Authentication** - Email, anonymous, and QR-based auth
12. **Client apps** - Android APK built, Windows Tauri app structured
13. **Website** - Full Portuguese marketing site with SEO optimization

---

## 25. Environment & Secrets

### Secret Locations
| Secret | Location |
|--------|----------|
| API keys (Vultr, Hetzner, IPRoyal, etc.) | `/opt/escudo/.env` |
| Stripe live keys | `/etc/escudo/api.toml` |
| JWT secret | `/etc/escudo/api.toml` |
| WireGuard encryption key | `/etc/escudo/api.toml` (env override) |
| Deploy secret | systemd drop-in `deploy-secret.conf` |
| DB password | `/etc/escudo/api.toml` |
| Android signing keystore | `apps/android/escudo-upload.jks` |

### Environment Variable Override Pattern
All config values can be overridden with `ESCUDO__<SECTION>__<KEY>` format:
- `ESCUDO__DATABASE__URL`
- `ESCUDO__JWT__SECRET`
- `ESCUDO__WIREGUARD__ENCRYPTION_KEY`
- `ESCUDO__STRIPE__SECRET_KEY`
- `ESCUDO_SKIP_MIGRATIONS`

---

*This document was generated by analyzing all source code, configs, running services, database state, and infrastructure at /opt/escudo/ and /home/dev/pulsovpn/escudo-vpn/.*
