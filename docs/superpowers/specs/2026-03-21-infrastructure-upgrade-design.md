# Escudo VPN — Infrastructure Upgrade Design Spec

> Version 2.1 — March 21, 2026. Server-side infrastructure upgrade.
> Reviewed and corrected after spec review (18 findings addressed).

**Requires:** PostgreSQL 16+ (uses `gen_random_uuid()` built-in, no `uuid-ossp` extension needed).

---

## Overview

Major server infrastructure upgrade to transform Escudo VPN from a single manually-deployed server to a fully automated multi-provider fleet with residential IP streaming proxy support. No app changes — pure server work.

**Scope:**
- Automated server provisioning across Vultr + Hetzner
- Residential IP integration via IPRoyal for streaming unlock
- IP health monitoring and auto-rotation (IP Guardian)
- Multi-interface WireGuard for per-tier routing
- DNS-based streaming traffic routing through SOCKS5 residential proxies

**Out of scope:**
- Billing/pricing changes (stays on Stripe as-is)
- PIX payment integration
- Android/iOS app changes
- Website changes

---

## Research Findings That Changed The Master Plan

| Master Plan Says | Reality (Researched 2026-03-21) |
|---|---|
| LightNode for 17 exotic servers via API | **No API exists.** Web console only. Dropped for Phase 1. |
| Vultr $6/mo plans | Actually **$5/mo** (vc2-1c-1gb) |
| Hetzner CX23 plan | **Doesn't exist.** Replaced by CX22 (€3.79/mo) |
| Hetzner CX in US/Singapore | **Intel CX only in EU.** Need CPX (AMD) for US/SG |
| Proxy-Cheap as primary dedicated IP provider | **Broken docs, weak API.** IPRoyal is primary |
| SNI proxy approach for streaming | **DNS + nftables + tun2socks is superior** — no TLS inspection, works with QUIC |

**Provider strategy (updated):**
- **Vultr** — primary (~75%), 32 regions, $5/mo, full REST API
- **Hetzner** — secondary (~25%), 6 locations, €3.79/mo, 20TB bandwidth, full REST API
- **IPRoyal** — sole residential IP provider (Proxy-Cheap as emergency fallback only)
- LightNode — deferred to Phase 2+ (manual provisioning for exotic locations)

---

## New Crates

| Crate | Type | Purpose |
|---|---|---|
| `escudo-deploy` | Binary (CLI) | Server provisioning — reads TOML config, diffs desired vs actual state, creates/destroys servers across Vultr + Hetzner |
| `escudo-proxy` | Library | Residential IP management — IPRoyal API client, ProxyProvider trait, pool management |
| `escudo-guardian` | Binary (service) | IP health monitoring — tests streaming IPs every 30 min, auto-rotates burned IPs |

## Modified Crates

| Crate | Changes |
|---|---|
| `escudo-common` | New models: ProxyIp, IpHealthLog, IpRotationLog, ProviderServer. Provider traits. |
| `escudo-api` | Tier-aware connect flow (wg0/wg1/wg2), server list filtering by tier, phone-home endpoint, proxy IP assignment |
| `escudo-gateway` | Multi-interface WireGuard, tun2socks credential management, phone-home on startup |

## Deprecated

| Crate | Reason |
|---|---|
| `escudo-sniproxy` | Replaced by DNS + nftables + tun2socks approach. Not deleted, just unused. |

---

## Database Migrations

### alter_servers (modify existing table)

Add multi-interface support and normalized country code to existing `servers` table.

```sql
-- Add columns for multi-interface WireGuard
ALTER TABLE servers ADD COLUMN wg1_public_key TEXT;
ALTER TABLE servers ADD COLUMN wg1_port INTEGER DEFAULT 51821;
ALTER TABLE servers ADD COLUMN wg2_public_key TEXT;
ALTER TABLE servers ADD COLUMN wg2_port INTEGER DEFAULT 51822;
ALTER TABLE servers ADD COLUMN country_code TEXT;  -- ISO 3166-1 alpha-2 (e.g., 'BR', 'US', 'DE')

-- Rename existing columns for clarity
-- public_key → wg0_public_key, endpoint_port → wg0_port
-- (done via new columns + data migration to avoid breaking existing queries during rollout)
ALTER TABLE servers ADD COLUMN wg0_public_key TEXT;
ALTER TABLE servers ADD COLUMN wg0_port INTEGER DEFAULT 51820;
UPDATE servers SET wg0_public_key = public_key, wg0_port = endpoint_port WHERE public_key IS NOT NULL;
```

### IP allocation migration

Existing devices use IPs from the full `10.0.0.0/16` range. New devices use per-tier subnets. Existing devices keep their IPs (all are Free/Pro under old scheme, mapped to wg0). The IP allocator is updated to allocate from the correct subnet based on tier.

```sql
-- No data migration needed for existing IPs — they all fall within 10.0.0.0/18 (wg0 range)
-- since existing usage is low and IPs are allocated sequentially from 10.0.0.2
-- The new allocator uses generate_series within the correct /18 range per tier:
--   wg0: generate_series(2, 16382)       → 10.0.0.2 - 10.0.63.254
--   wg1: generate_series(16385, 32766)   → 10.0.64.1 - 10.0.127.254
--   wg2: generate_series(32769, 49150)   → 10.0.128.1 - 10.0.191.254
```

### Subscription plan name migration

Map existing plan names to new tier system. Billing changes are out of scope, but the tier mapping must exist for the connect flow.

```sql
-- Map old plan names to new tier names
-- 'free' → 'free' (unchanged)
-- 'pro' → 'escudo' (the $8/mo tier, not the streaming tier)
-- 'family' → 'pro' (maps to streaming-capable tier)
-- New plans 'pro' and 'dedicated' will be created when billing spec is implemented
-- For now, add a tier column that the connect flow reads:
ALTER TABLE subscriptions ADD COLUMN tier TEXT NOT NULL DEFAULT 'free';
UPDATE subscriptions SET tier = 'free' WHERE plan = 'free' OR plan IS NULL;
UPDATE subscriptions SET tier = 'escudo' WHERE plan = 'pro';
UPDATE subscriptions SET tier = 'pro' WHERE plan = 'family';
```

### provider_servers

Tracks every VPS across Vultr/Hetzner with provider metadata. 1:1 relationship with `servers` table (one provider_server per server).

```sql
CREATE TABLE provider_servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID UNIQUE REFERENCES servers(id),  -- 1:1 with servers
    provider TEXT NOT NULL,              -- 'vultr' | 'hetzner'
    provider_instance_id TEXT NOT NULL,
    label TEXT NOT NULL UNIQUE,          -- 'br-sp-01'
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
```

### proxy_ips

Residential IP pool with provider credentials and assignment status.

```sql
CREATE TABLE proxy_ips (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider TEXT NOT NULL,
    provider_proxy_id TEXT NOT NULL,
    proxy_type TEXT NOT NULL,           -- 'shared' | 'dedicated'
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
CREATE INDEX idx_proxy_ips_country_status ON proxy_ips(country, status);
CREATE INDEX idx_proxy_ips_assigned_user ON proxy_ips(assigned_user_id) WHERE assigned_user_id IS NOT NULL;
```

### server_proxy_assignments

Which server uses which proxy for streaming routing. One shared proxy + optionally one dedicated proxy per server.

```sql
CREATE TABLE server_proxy_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id),
    proxy_ip_id UUID NOT NULL REFERENCES proxy_ips(id),
    proxy_target TEXT NOT NULL DEFAULT 'shared',  -- 'shared' | 'dedicated'
    assigned_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(server_id, proxy_target)  -- one proxy per target type per server
);
```

### ip_health_logs

Health check results per IP per streaming service.

```sql
CREATE TABLE ip_health_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proxy_ip_id UUID NOT NULL REFERENCES proxy_ips(id),
    service TEXT NOT NULL,
    status TEXT NOT NULL,
    response_time_ms INTEGER,
    error_detail TEXT,
    checked_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX idx_health_logs_proxy_checked ON ip_health_logs(proxy_ip_id, checked_at);
CREATE INDEX idx_health_logs_service ON ip_health_logs(service, checked_at);
```

### ip_rotation_logs

Audit trail of every IP swap.

```sql
CREATE TABLE ip_rotation_logs (
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

---

## escudo-deploy — Automated Server Provisioning

### Config File (deploy-config.toml)

```toml
[defaults]
ssh_key_name = "escudo-deploy"
firewall_rules = ["udp:51820", "udp:51821", "udp:51822", "tcp:22"]
image = "ubuntu-24.04"
gateway_binary_url = "https://deploy.escudovpn.com/gateway-linux-amd64"
tun2socks_binary_url = "https://deploy.escudovpn.com/tun2socks-linux-amd64"
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

### CLI Commands

```
escudo-deploy validate      # Pre-flight: verify API keys, plan availability, SSH keys, estimate costs
escudo-deploy plan          # Show diff: what will be created/destroyed (no changes made)
escudo-deploy apply         # Execute the diff
escudo-deploy status        # Show all servers with health, IP, provider, uptime, cost
escudo-deploy destroy <id>  # Remove a specific server
escudo-deploy update --all  # Push new gateway binary to all servers via SSH
```

### Reconciliation Flow (apply command)

1. Read `deploy-config.toml` — desired state
2. Query Vultr + Hetzner APIs — actual state (match by label)
3. Diff: servers to create, servers to destroy
4. For each server to create:
   - Pre-flight: verify plan available in that region
   - Create server with cloud-init user_data
   - Poll until IP assigned and status ok (timeout 5 min)
   - Verify gateway phones home within 120 seconds
   - If phone-home fails: SSH in, check logs, mark degraded
5. For each server to destroy (in actual but not in desired):
   - Drain connections (30s grace)
   - Destroy via provider API
   - Remove from database
6. Report: created X, destroyed Y, Z healthy, W degraded, total monthly cost

### Pre-flight Validation (validate command)

- Test Vultr API key: `GET /v2/account`
- Test Hetzner API token: `GET /v1/servers`
- Test IPRoyal API token: `GET /api/v1/proxy-manager/proxies`
- Verify every region + plan combo available
- Check SSH keys exist in each provider
- Estimate total monthly cost
- Fail fast with clear errors

### Error Handling

- Provider API timeout: exponential backoff, max 3 retries
- Server stuck provisioning: timeout 5 min, destroy and retry once
- Phone-home never arrives: SSH in, check logs, alert
- Partial failure: report what succeeded and what failed, no rollback of successful servers

### Binary Distribution

Gateway binary + tun2socks binary hosted on Vultr Object Storage. Cloud-init curls them down on first boot. The `update` command uploads new binary to storage, then SSHs into each server to pull and restart.

### Cloud-Init Script Skeleton

Generated per-server by escudo-deploy. Order matters:

```yaml
#cloud-config
packages:
  - wireguard
  - wireguard-tools
  - dnsmasq
  - nftables

write_files:
  # 1. WireGuard configs (3 interfaces)
  - path: /etc/wireguard/wg0.conf    # Free/Escudo — generated keys, subnet 10.0.0.0/18
  - path: /etc/wireguard/wg1.conf    # Pro — generated keys, subnet 10.0.64.0/18
  - path: /etc/wireguard/wg2.conf    # Dedicated — generated keys, subnet 10.0.128.0/18

  # 2. dnsmasq streaming config
  - path: /etc/dnsmasq.d/streaming.conf  # nftset directives for Netflix/BBC/Disney+/Globoplay

  # 3. nftables rules
  - path: /etc/nftables.conf         # streaming_v4 set, fwmark rules, ct mark propagation

  # 4. Policy routing
  - path: /etc/networkd-dispatcher/routable.d/proxy-routes.sh  # ip rule + ip route for table 100

  # 5. Systemd services
  - path: /etc/systemd/system/escudo-gateway.service
  - path: /etc/systemd/system/tun2socks.service

runcmd:
  # Enable IP forwarding
  - sysctl -w net.ipv4.ip_forward=1
  - echo "net.ipv4.ip_forward = 1" >> /etc/sysctl.conf

  # Download binaries
  - curl -sSL ${gateway_binary_url} -o /usr/local/bin/escudo-gateway && chmod +x /usr/local/bin/escudo-gateway
  - curl -sSL ${tun2socks_binary_url} -o /usr/local/bin/tun2socks && chmod +x /usr/local/bin/tun2socks

  # Start WireGuard interfaces
  - systemctl enable --now wg-quick@wg0
  - systemctl enable --now wg-quick@wg1
  - systemctl enable --now wg-quick@wg2

  # Start dnsmasq and nftables
  - systemctl enable --now dnsmasq
  - nft -f /etc/nftables.conf

  # Apply policy routing
  - bash /etc/networkd-dispatcher/routable.d/proxy-routes.sh

  # Start services (tun2socks starts after gateway fetches proxy credentials)
  - systemctl enable --now escudo-gateway

  # Phone home to central API
  - |
    curl -X POST ${api_callback_url} \
      -H "Authorization: Bearer ${deploy_secret}" \
      -H "Content-Type: application/json" \
      -d '{"public_ip":"$(curl -s ifconfig.me)", ... }'
```

Variables like `${gateway_binary_url}`, `${deploy_secret}`, WireGuard keys, and server-specific config are interpolated by escudo-deploy before passing as `user_data` to the provider API.

### Destroy Safety

`escudo-deploy destroy <label>` checks for active connections before destroying:

```
$ escudo-deploy destroy br-sp-01
⚠  Server br-sp-01 has 47 active peers (23 on wg0, 18 on wg1, 6 on wg2)
   Destroying will disconnect all 47 users.
   Proceed? [y/N]
```

The `apply` command's reconciliation (removing servers not in config) also warns and requires `--force` to destroy servers with active peers.

### Provider API Details

**Vultr:**
- Auth: `Authorization: Bearer {VULTR_API_KEY}`
- Create: `POST /v2/instances` with `region`, `plan`, `os_id`, `script_id`, `user_data`
- Rate limit: 30 req/s
- Poll `main_ip` until not `0.0.0.0`
- Plan: `vc2-1c-1gb` at $5/mo

**Hetzner:**
- Auth: `Authorization: Bearer {HETZNER_API_TOKEN}`
- Create: `POST /v1/servers` with `name`, `server_type`, `image`, `location`, `ssh_keys`, `user_data`
- Rate limit: 3600 req/hr
- Poll action status until `success`
- Plan: `cx22` at €3.79/mo (EU), `cpx21` for US/SG
- CX23 does not exist — use CX22

---

## Streaming Proxy Architecture (DNS + nftables + tun2socks)

### How It Works

Pro/Dedicated users connect to WireGuard normally. On the server side, streaming traffic is transparently routed through a residential SOCKS5 proxy:

1. **dnsmasq** resolves DNS for Pro/Dedicated clients. When streaming domains are resolved, their IPs are automatically added to an nftables set via the `nftset` directive.
2. **nftables** marks packets destined for streaming IPs with fwmark `0x1`. Connection tracking persists the mark across the flow.
3. **Policy routing** sends marked packets to routing table 100, which routes through the `tun-proxy` interface.
4. **tun2socks** (xjasonlyu/tun2socks) creates the `tun-proxy` TUN interface and forwards all traffic entering it through SOCKS5 to the residential IP.
5. **Blackhole safety** — if the proxy tunnel goes down, streaming traffic is dropped rather than leaking through the datacenter IP.

### Why This Approach

- No TLS inspection — routing by IP from DNS resolution
- Works with QUIC/HTTP3 — nftables marks UDP too
- Dynamic IP tracking — streaming CDNs rotate IPs constantly, dnsmasq captures them as clients resolve
- Connection tracking — mid-stream CDN IP changes don't break the session

### Three WireGuard Interfaces Per Server

```
wg0 — Free/Escudo tier
  Subnet: 10.0.0.0/18 (16,382 clients)
  DNS: escudo-dns (ad-blocking only)
  Routing: direct internet, no proxy
  Speed cap: 10 Mbps (Free only, via tc)

wg1 — Pro tier
  Subnet: 10.0.64.0/18 (16,382 clients)
  DNS: dnsmasq (ad-blocking + nftset streaming rules)
  Routing: streaming → tun-proxy → residential IP, everything else direct

wg2 — Dedicated IP tier
  Subnet: 10.0.128.0/18 (16,382 clients)
  DNS: dnsmasq (ad-blocking + nftset streaming rules)
  Routing: ALL traffic → tun-proxy → customer's personal residential IP
```

### dnsmasq Streaming Config

```
nftset=/netflix.com/4#ip#filter#streaming_v4
nftset=/nflxvideo.net/4#ip#filter#streaming_v4
nftset=/nflxso.net/4#ip#filter#streaming_v4
nftset=/nflximg.net/4#ip#filter#streaming_v4
nftset=/nflxext.com/4#ip#filter#streaming_v4
nftset=/bbc.co.uk/4#ip#filter#streaming_v4
nftset=/bbci.co.uk/4#ip#filter#streaming_v4
nftset=/bbc.com/4#ip#filter#streaming_v4
nftset=/disneyplus.com/4#ip#filter#streaming_v4
nftset=/bamgrid.com/4#ip#filter#streaming_v4
nftset=/dssott.com/4#ip#filter#streaming_v4
nftset=/disney.io/4#ip#filter#streaming_v4
nftset=/globoplay.globo.com/4#ip#filter#streaming_v4
nftset=/video.globo.com/4#ip#filter#streaming_v4
```

Note: Do NOT use `globo.com` — it's too broad and would route all Globo news/sports/email traffic through the proxy, burning IPs faster. Only `globoplay.globo.com` and `video.globo.com` are needed for streaming.

### nftables + Policy Routing

```bash
# Create streaming IP set (populated dynamically by dnsmasq nftset directives)
nft add set ip filter streaming_v4 { type ipv4_addr\; flags timeout\; timeout 1h\; }

# Mark streaming packets from Pro interface (wg1) — only streaming destinations
nft add rule ip filter forward iifname "wg1" ip daddr @streaming_v4 ct mark set 0x1

# Mark ALL packets from Dedicated interface (wg2) — all traffic goes through personal proxy
# This is intentional: dedicated tier routes everything, not just streaming
nft add rule ip filter forward iifname "wg2" ct mark set 0x1

# Copy conntrack mark to packet mark for policy routing
# Both rules above set ct mark; this rule propagates it to meta mark for ip rule matching
# Rules are evaluated in order within the filter forward chain (type filter hook forward)
nft add rule ip filter forward ct mark 0x1 meta mark set 0x1

# Policy route marked packets to tun-proxy
ip rule add fwmark 0x1 table 100
ip route add default dev tun-proxy table 100

# Safety blackhole: if tun-proxy interface goes down, drop traffic rather than leaking
# through datacenter IP (which would get detected by Netflix etc.)
# metric 200 means this only activates when the tun-proxy route (metric 0) is unreachable
ip route add blackhole default table 100 metric 200
```

### tun2socks Crash Detection

If tun2socks crashes but the TUN interface stays up (zombie state), traffic would enter a dead tunnel. The gateway binary monitors tun2socks:

1. Gateway spawns tun2socks as a child process and watches the process handle
2. If tun2socks exits: immediately bring down `tun-proxy` interface (`ip link set tun-proxy down`)
3. This makes the route unreachable, blackhole catches the traffic
4. Attempt restart with same credentials, max 3 retries with 5s backoff
5. If restart fails: alert central API, mark server as `degraded` for proxy traffic
6. Non-streaming traffic (wg0, and wg1/wg2 non-streaming) continues unaffected

### tun2socks

```bash
tun2socks -device tun-proxy -proxy socks5://user:pass@proxy.iproyal.com:32325
```

Single Go binary, ~19.8 Gbps throughput with multi-threading. Handles thousands of concurrent sessions.

---

## escudo-proxy — Residential IP Pool Management

### Provider Trait

```rust
#[async_trait]
pub trait ProxyProvider: Send + Sync {
    async fn acquire_shared_proxy(&self, country: &str, city: Option<&str>,
                                   sticky_duration: Duration) -> Result<ProxyCredential>;
    async fn acquire_dedicated_ip(&self, country: &str) -> Result<ProxyCredential>;
    async fn release_proxy(&self, proxy_id: &str) -> Result<()>;
    async fn rotate_proxy(&self, proxy_id: &str) -> Result<ProxyCredential>;
    async fn list_proxies(&self) -> Result<Vec<ProxyCredential>>;
    async fn health_check(&self) -> Result<bool>;
}
```

### Implementations

- `IpRoyalProvider` — primary. Full API at `dashboard.iproyal.com/api/v1`. Bearer token auth. City-level targeting. Sticky sessions up to 7 days. SOCKS5 on port 32325.
- `ProxyCheapProvider` — emergency fallback only. Weak API, requires API key + secret.

### Failover

```
IPRoyal (primary) → ProxyCheap (fallback)
```

If IPRoyal API fails (timeout, error, out of stock), automatically retry on ProxyCheap. Calling code never knows which provider fulfilled the request.

### ProxyCredential

```rust
pub struct ProxyCredential {
    pub id: String,
    pub provider: ProviderKind,
    pub proxy_type: ProxyType,      // Shared | Dedicated
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub country: String,
    pub city: Option<String>,
    pub external_ip: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}
```

### Pool Management

**Shared streaming IPs (Pro tier):**
- Pre-warm small pool per country (Phase 1: just 1 US IP for testing)
- 3-5 Pro users share one IP
- IPRoyal sticky sessions keep same IP for up to 7 days
- When session ends, slot opens for another user

**Dedicated IPs:**
- Purchased on-demand when customer subscribes
- One customer, one IP, never shared
- Stored in DB, assigned permanently
- If burned, IP Guardian rotates automatically

---

## escudo-guardian — IP Health Monitoring

### Health Check Flow

Every 30 minutes, for each proxy_ip where status = 'healthy':

1. Connect through SOCKS5 proxy to each streaming service
2. HTTP GET homepage, check response for block indicators
3. Record result in ip_health_logs
4. If blocked: trigger auto-rotation

### Block Detection

```rust
const CHECKS: &[StreamingCheck] = &[
    StreamingCheck {
        service: "netflix",
        url: "https://www.netflix.com/browse",
        block_indicators: &["unblocker or proxy", "proxy or VPN", "not available in your area"],
    },
    StreamingCheck {
        service: "bbc_iplayer",
        url: "https://www.bbc.co.uk/iplayer",
        block_indicators: &["not available in your area", "only available in the UK", "outside the UK"],
    },
    StreamingCheck {
        service: "disney_plus",
        url: "https://www.disneyplus.com/",
        block_indicators: &["proxy", "VPN", "not available in your region"],
    },
    StreamingCheck {
        service: "globoplay",
        url: "https://globoplay.globo.com/",
        block_indicators: &["não está disponível", "indisponível na sua região"],
    },
];
```

### Auto-Rotation Flow

1. Mark IP as `blocked` in DB
2. Find affected servers (server_proxy_assignments) and customers (dedicated)
3. Call `rotate_proxy()` on provider → fresh IP, same country
4. Insert new proxy_ip record
5. Update server_proxy_assignments
6. Push new SOCKS5 credentials to affected gateway servers (gRPC UpdateProxyCredentials)
7. Gateway restarts tun2socks with new proxy (~2 seconds)
8. Release burned IP via provider API
9. Log in ip_rotation_logs
10. Dedicated IP customers get notification

Total time: 30-60 seconds. Customer impact: zero.

### Pattern Detection (learns over time)

Tracks burn rates by country, provider, service, and time-of-day. Actions:
- Country burn rate > 2x average → double pool size for that country
- Service burn rate > 2x average → increase check frequency to 15 min
- Provider burn rate > 3x other → shift acquisitions to other provider
- Hour burn rate spike → check every 10 min during those hours

### Deployment

Single binary, runs on central API server. Connects to PostgreSQL. Systemd service.

---

## escudo-gateway Upgrades

### New Responsibilities

- Manage 3 WireGuard interfaces (wg0, wg1, wg2)
- Fetch SOCKS5 credentials from central API on startup
- Manage tun2socks process lifecycle
- Accept credential updates via gRPC (pushed by Guardian)
- Report per-interface metrics

### Updated gRPC Proto

Full updated service definition (existing RPCs preserved, new ones added):

```protobuf
syntax = "proto3";
package gateway;

enum Tier {
    FREE = 0;      // Default — old clients without tier field get FREE (proto3 default)
    ESCUDO = 1;    // Same interface as FREE (wg0) but no speed cap
    PRO = 2;       // wg1 — streaming proxy
    DEDICATED = 3; // wg2 — personal proxy for all traffic
}

enum ProxyTarget {
    SHARED = 0;    // wg1 tun2socks instance
    DEDICATED_PROXY = 1; // wg2 tun2socks instance
}

service GatewayService {
    // Existing RPCs (unchanged signatures, AddPeer gets new field)
    rpc AddPeer(AddPeerRequest) returns (AddPeerResponse);
    rpc RemovePeer(RemovePeerRequest) returns (RemovePeerResponse);
    rpc ListPeers(ListPeersRequest) returns (ListPeersResponse);
    rpc GetStats(GetStatsRequest) returns (GetStatsResponse);
    rpc AddMultihopPeer(AddMultihopPeerRequest) returns (AddMultihopPeerResponse);

    // New RPCs
    rpc UpdateProxyCredentials(UpdateProxyCredentialsRequest) returns (UpdateProxyCredentialsResponse);
}

// Updated: added tier field at position 4 (backward compatible — old clients omit it, defaults to FREE)
message AddPeerRequest {
    string public_key = 1;
    string allowed_ip = 2;
    string preshared_key = 3;
    Tier tier = 4;
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

**Backward compatibility:** `Tier tier = 4` defaults to `FREE = 0` when omitted (proto3 default value behavior). During rolling upgrades, old API servers send AddPeerRequest without tier → gateway treats as FREE → peer goes to wg0. This is correct and safe.

### gRPC Authentication

The gRPC server on each gateway listens on `127.0.0.1:9090` (localhost only). For `UpdateProxyCredentials` pushed from the central API/Guardian:

- The Guardian does NOT call gRPC directly (gateways are remote, gRPC is localhost-bound)
- Instead, Guardian updates the `proxy_ips` and `server_proxy_assignments` tables in PostgreSQL
- Each gateway polls the central API every 60 seconds: `GET /internal/servers/{label}/proxy-credentials`
- If credentials changed, gateway restarts tun2socks with new ones
- The poll endpoint uses the same `Bearer $DEPLOY_SECRET` as phone-home

This avoids opening gRPC to the internet entirely. The `UpdateProxyCredentials` gRPC method is kept for local admin use (e.g., `escudo-admin` on the same box).

### Phone-Home Registration

```
POST /internal/servers/register
Authorization: Bearer $DEPLOY_SECRET
{
    "public_ip": "1.2.3.4",
    "wg0_public_key": "abc...",
    "wg1_public_key": "def...",
    "wg2_public_key": "ghi...",
    "wg0_port": 51820,
    "wg1_port": 51821,
    "wg2_port": 51822,
    "location": "gru",
    "provider": "vultr",
    "provider_instance_id": "abc-123",
    "label": "br-sp-01",
    "version": "0.3.0"
}
```

---

## escudo-api Changes

### Updated Connect Flow

```
POST /api/v1/connect { "server_id": "uuid", "device_name": "My Phone" }

1. Check subscription → tier via subscriptions.tier column:
   - No subscription / tier='free' → FREE
   - tier='escudo' → ESCUDO
   - tier='pro' → PRO
   - tier='dedicated' → DEDICATED

2. Server access control (uses servers.country_code):
   Free: reject if server country_code not in ('BR', 'US', 'DE')
   All others: user chose server from app, any server allowed

3. Check device limit:
   Free=1, Escudo=5, Pro=10, Dedicated=10

4. Pick WireGuard interface by tier:
   FREE/ESCUDO → wg0 (port from servers.wg0_port)
   PRO → wg1 (port from servers.wg1_port)
   DEDICATED → wg2 (port from servers.wg2_port)

5. Allocate IP from correct subnet:
   wg0 → 10.0.0.2 - 10.0.63.254  (generate_series 2..16382)
   wg1 → 10.0.64.1 - 10.0.127.254  (generate_series 16385..32766)
   wg2 → 10.0.128.1 - 10.0.191.254  (generate_series 32769..49150)

6. Generate keypair, create peer via gRPC:
   AddPeer { public_key, allowed_ip, preshared_key, tier: FREE/ESCUDO/PRO/DEDICATED }

7. Gateway uses tier to decide interface:
   FREE/ESCUDO → wg set wg0 peer ...
   PRO → wg set wg1 peer ...
   DEDICATED → wg set wg2 peer ...

8. Pro: ensure server has shared streaming proxy assigned
   Dedicated: verify customer has dedicated IP in proxy_ips table

9. Return WireGuard config + QR (with correct endpoint port per tier)
```

### Speed Cap: Free vs Escudo on wg0

Both Free and Escudo use wg0, but Free has a 10 Mbps cap. This is handled per-peer using `tc` with per-IP filtering:

```bash
# On wg0, apply per-peer speed shaping
# Gateway tracks which peers are FREE tier
# For each FREE peer added: tc filter add ... flowid for 10mbit class
# For ESCUDO peers: no filter added, full speed
```

The gateway's `AddPeer` handler checks the `Tier` field:
- `FREE` → add tc filter for that peer's allowed_ip, capped at 10 Mbps
- `ESCUDO` → no tc filter, full speed on wg0

When a peer is removed, the corresponding tc filter is also removed.

### Updated Server List

```
GET /api/v1/servers

Free: only servers WHERE country_code IN ('BR', 'US', 'DE')
Escudo/Pro/Dedicated: all active servers with load %, location, country_code
```

---

## Implementation Phases

### Phase 1: Foundation + All Provider Connections

All 3 provider APIs validated with real transactions.

- All new database migrations
- Vultr API client in escudo-deploy → deploy 1 test server
- Hetzner API client in escudo-deploy → deploy 1 test server
- IPRoyal API client in escudo-proxy → buy 1 US residential IP
- Cloud-init with full stack (3 WireGuard interfaces + dnsmasq + nftables + tun2socks)
- Phone-home registration endpoint in escudo-api
- `validate`, `plan`, `apply`, `status` commands working
- **Exit gate:** 1 Vultr server + 1 Hetzner server running, 1 residential IP active, can connect as Pro and test Netflix through residential IP. All 3 provider APIs validated with real money. Output: server cost, IP cost, connection verified.

### Phase 2: Multi-Interface WireGuard (Tier Separation)

- Tier detection from subscription in connect flow
- Route to correct WireGuard interface (wg0/wg1/wg2)
- Server list filtering by tier (Free = BR/US/DE only)
- Speed cap on wg0 Free tier (10 Mbps via tc)
- Device limits per tier (1/5/10)
- **Exit gate:** Free user connects wg0, Pro user connects wg1, Dedicated user connects wg2. Verified on test servers.

### Phase 3: IP Guardian

- Health check service testing each IP against Netflix/BBC/Disney+/Globoplay
- Auto-rotation: detect block → swap IP → push credentials → zero downtime
- Rotation logging and audit trail
- Basic pattern detection (burn rate by country/service)
- **Exit gate:** Manually block a test IP, Guardian detects within 30 min, rotates automatically, verified zero customer impact.

### Phase 4: Fleet Scale-Up

- Full `deploy-config.toml` with all servers (Vultr + Hetzner)
- `escudo-deploy apply` → spin up fleet
- Full IP pool pre-warm (per master plan numbers)
- Monitor costs, verify all servers healthy
- `escudo-deploy status` dashboard
- **Exit gate:** Full fleet running, all servers phoning home, total monthly cost matches projections.

### Dependencies

```
Phase 1 → Phase 2 → Phase 3
Phase 1 ──────────→ Phase 4 (parallel with 2/3)
```

---

## Environment Variables

```
VULTR_API_KEY=<stored in .env>
HETZNER_API_TOKEN=<stored in .env>
IPROYAL_API_TOKEN=<stored in .env>
DEPLOY_SECRET=<generated, used for server phone-home auth>
```

---

## Cost Projections (Updated)

### Phase 1 (Testing)

| Item | Cost |
|---|---|
| 1 Vultr server (vc2-1c-1gb, São Paulo) | $5/mo |
| 1 Hetzner server (cx22, Nuremberg) | €3.79/mo (~$4.20) |
| 1 IPRoyal US residential IP | ~$2.70/mo |
| **Total test infrastructure** | **~$12/mo** |

### Phase 4 (Full Fleet)

| Provider | Servers | Cost |
|---|---|---|
| Vultr | ~35 servers × $5 | $175/mo |
| Hetzner | ~12 servers × €3.79 | ~$50/mo |
| IPRoyal shared IPs (36) | ~$97/mo |
| **Total Phase 4** | **~$322/mo** |

Note: Vultr plan is $5/mo (not $6 as in master plan). Hetzner CX22 is €3.79 (not €3.99-4.99 as in master plan for non-existent CX23). Total is lower than originally projected.

---

## What's NOT In This Spec

- Billing tier changes (Free/Escudo/Pro/Dedicated pricing) — separate spec
- PIX payment integration — separate spec
- Regional pricing — separate spec
- Android/iOS app changes — separate spec
- Desktop (Tauri) app — separate spec
- Post-quantum crypto (ML-KEM-768) — Phase 2 of master plan
- QUIC obfuscation (VLESS+REALITY) — Phase 2 of master plan
- RAM-only server architecture — Phase 3 of master plan
- Home Shield / TV box support — Phase 3 of master plan
