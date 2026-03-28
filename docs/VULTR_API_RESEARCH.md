# Vultr API v2 Research — Automated VPN Server Provisioning

Base URL: `https://api.vultr.com/v2`

---

## 1. Authentication

**Method:** Bearer token in the `Authorization` header.

```
Authorization: Bearer {VULTR_API_KEY}
Content-Type: application/json
```

API keys are generated in the Vultr customer portal under Account > API. Keys can be restricted by source IP (recommended for production).

### Rate Limits

- **30 requests per second** per originating IP address
- Exceeding returns **HTTP 429 Too Many Requests**
- No documented Retry-After header — implement exponential backoff
- Terraform provider defaults to 500ms delay between calls as a safe baseline

**Rust implementation note:** Use a token-bucket rate limiter capped at ~25 req/s to stay safely under the limit.

---

## 2. Create Instance

```
POST /v2/instances
```

### Request Body (JSON)

All confirmed field names from the official Go client (`govultr`):

| Field | JSON Key | Type | Required | Notes |
|-------|----------|------|----------|-------|
| Region | `region` | string | **Yes** | e.g. `"ewr"`, `"mia"`, `"gru"` |
| Plan | `plan` | string | **Yes** | e.g. `"vc2-1c-1gb"` |
| OS ID | `os_id` | int | One of os_id/iso_id/snapshot_id/app_id required | e.g. `1743` for Ubuntu 24.04 |
| ISO ID | `iso_id` | string | | Custom ISO |
| App ID | `app_id` | int | | One-click app |
| Image ID | `image_id` | string | | Marketplace image |
| Snapshot ID | `snapshot_id` | string | | Restore from snapshot |
| Label | `label` | string | | Display name |
| Tags | `tags` | []string | | Arbitrary tags |
| Hostname | `hostname` | string | | Server hostname |
| Script ID | `script_id` | string | | Startup script to run |
| SSH Keys | `ssh_keys` | []string | | Array of SSH key IDs |
| User Data | `user_data` | string | | Base64-encoded cloud-init |
| Backups | `backups` | string | | `"enabled"` or `"disabled"` |
| Enable IPv6 | `enable_ipv6` | bool | | |
| Disable Public IPv4 | `disable_public_ipv4` | bool | | |
| DDoS Protection | `ddos_protection` | string | | Extra charge |
| Firewall Group ID | `firewall_group_id` | string | | Attach firewall group |
| Reserved IPv4 | `reserved_ipv4` | string | | Floating IP to assign |
| Activation Email | `activation_email` | bool | | Send deploy notification |
| Enable VPC | `enable_vpc` | bool | | |
| Attach VPC | `attach_vpc` | string | | VPC ID to attach |
| VPC Only | `vpc_only` | bool | | No public IP — VPC only |
| User Scheme | `user_scheme` | string | | `"root"` or `"limited"` |
| IPXE Chain URL | `ipxe_chain_url` | string | | PXE boot URL |
| App Variables | `app_variables` | string | | App-specific config |

### Example: Minimal VPN Server

```json
{
  "region": "mia",
  "plan": "vc2-1c-1gb",
  "os_id": 1743,
  "label": "escudo-vpn-mia-001",
  "hostname": "vpn-mia-001",
  "script_id": "abc123-def456",
  "ssh_keys": ["key-id-1"],
  "firewall_group_id": "fw-group-id",
  "enable_ipv6": true,
  "backups": "disabled",
  "tags": ["escudo", "vpn", "production"]
}
```

### Response

Returns the full `Instance` object. Key fields:

```json
{
  "instance": {
    "id": "instance-uuid",
    "os": "Ubuntu 24.04 LTS x64",
    "ram": 1024,
    "disk": 25,
    "main_ip": "0.0.0.0",
    "vcpu_count": 1,
    "region": "mia",
    "plan": "vc2-1c-1gb",
    "default_password": "random-password",
    "date_created": "2024-01-01T00:00:00+00:00",
    "status": "pending",
    "power_status": "running",
    "server_status": "installingbooting",
    "v6_main_ip": "2001:...",
    "label": "escudo-vpn-mia-001",
    "firewall_group_id": "fw-group-id",
    "hostname": "vpn-mia-001",
    "tags": ["escudo", "vpn", "production"],
    "features": ["ipv6"],
    "user_scheme": "root"
  }
}
```

---

## 3. Startup Scripts

### Create Startup Script

```
POST /v2/startup-scripts
```

```json
{
  "name": "escudo-vpn-setup",
  "type": "boot",
  "script": "<base64-encoded script content>"
}
```

- **type**: `"boot"` (runs on first boot) or `"pxe"` (PXE provisioning)
- **script**: Must be **base64-encoded**
- No documented size limit, but keep scripts reasonable (the control panel has a practical limit)

### List Startup Scripts

```
GET /v2/startup-scripts
```

### Get Single Script

```
GET /v2/startup-scripts/{startup-script-id}
```

### Update Script

```
PATCH /v2/startup-scripts/{startup-script-id}
```

### Delete Script

```
DELETE /v2/startup-scripts/{startup-script-id}
```

### Startup Script vs. user_data

| Feature | `script_id` (Startup Script) | `user_data` (Cloud-init) |
|---------|------------------------------|--------------------------|
| Storage | Pre-stored via API | Inline in create request |
| Format | Base64 bash script | Base64 cloud-init (bash or YAML) |
| Reuse | Reference by ID across instances | Must re-send each time |
| Platform | All OS images | Linux only (cloud-init) |
| Best for | Static setup scripts | Dynamic per-instance config |

### Passing Environment Variables

There is no built-in env var injection. Strategies:

1. **user_data with embedded vars** — Base64-encode a script that has the vars baked in at generation time
2. **Metadata service** — Instance queries `http://169.254.169.254/v1/` for its own metadata; your script can fetch config from an external source using the instance ID
3. **Hybrid** — Use a stored `script_id` for the base setup, plus `user_data` for per-instance secrets/config

**For Escudo VPN:** Use `user_data` with dynamically generated cloud-init scripts that embed WireGuard keys, server config, and API callback URLs.

---

## 4. List Regions

```
GET /v2/regions
```

Response (paginated):

```json
{
  "regions": [
    {
      "id": "ewr",
      "city": "New Jersey",
      "country": "US",
      "continent": "North America",
      "options": ["ddos_protection", "block_storage_high_perf", "block_storage_storage_opt"]
    },
    {
      "id": "mia",
      "city": "Miami",
      "country": "US",
      "continent": "North America",
      "options": [...]
    },
    {
      "id": "gru",
      "city": "Sao Paulo",
      "country": "BR",
      "continent": "South America",
      "options": [...]
    }
  ],
  "meta": {
    "total": 32,
    "links": { "next": "", "prev": "" }
  }
}
```

### Key Region IDs (relevant for Brazil-focused VPN)

| ID | City | Country |
|----|------|---------|
| `gru` | Sao Paulo | BR |
| `mia` | Miami | US |
| `ewr` | New Jersey | US |
| `ord` | Chicago | US |
| `dfw` | Dallas | US |
| `lax` | Los Angeles | US |
| `atl` | Atlanta | US |
| `ams` | Amsterdam | NL |
| `fra` | Frankfurt | DE |
| `lhr` | London | GB |
| `nrt` | Tokyo | JP |
| `sgp` | Singapore | SG |

### List Available Plans Per Region

```
GET /v2/regions/{region-id}/availability
```

Returns plan IDs available in that specific region.

---

## 5. List Plans

```
GET /v2/plans
```

Supports query parameter `?type=vc2` to filter by plan type.

Response:

```json
{
  "plans": [
    {
      "id": "vc2-1c-1gb",
      "vcpu_count": 1,
      "ram": 1024,
      "disk": 25,
      "bandwidth": 1024,
      "monthly_cost": 5,
      "type": "vc2",
      "locations": ["ewr", "mia", "gru", ...]
    }
  ],
  "meta": { "total": 50, "links": { "next": "", "prev": "" } }
}
```

**Note:** The `vc2-1c-1gb` plan is listed at **$5/month** (not $6). The `locations` array tells you which regions support this plan. Always verify with `/v2/regions/{id}/availability` before provisioning.

---

## 6. Instance Lifecycle

### Get Instance

```
GET /v2/instances/{instance-id}
```

### List All Instances

```
GET /v2/instances
```

Supports filtering with query params: `?label=`, `?tag=`, `?region=`, `?main_ip=`.

### Start (Power On)

```
POST /v2/instances/{instance-id}/start
```

### Stop (Halt)

```
POST /v2/instances/{instance-id}/halt
```

**Also bulk halt:**
```
POST /v2/instances/halt
Body: { "instance_ids": ["id1", "id2"] }
```

### Reboot

```
POST /v2/instances/{instance-id}/reboot
```

**Also bulk reboot:**
```
POST /v2/instances/reboot
Body: { "instance_ids": ["id1", "id2"] }
```

### Reinstall

```
POST /v2/instances/{instance-id}/reinstall
```

Wipes the filesystem, reinstalls OS. **Destructive.** Optionally accepts `hostname` in body.

### Destroy

```
DELETE /v2/instances/{instance-id}
```

Returns **204 No Content** on success. **Irreversible.**

### Instance Status Model

```
status:        "pending" | "active" | "suspended" | "closed"
power_status:  "running" | "stopped"
server_status: "none" | "locked" | "installingbooting" | "isomounting" | "ok"
```

**Polling strategy for provisioning:**
1. Create instance -> returns `status: "pending"`
2. Poll `GET /v2/instances/{id}` every 5-10 seconds
3. Wait for `status: "active"` AND `server_status: "ok"` AND `power_status: "running"`
4. At that point, `main_ip` is assigned and the startup script has begun executing
5. SSH into instance or wait for a callback from the startup script to confirm VPN is ready

---

## 7. SSH Keys

### Create SSH Key

```
POST /v2/ssh-keys
```

```json
{
  "name": "escudo-management-key",
  "ssh_key": "ssh-ed25519 AAAA... admin@escudo"
}
```

Response:

```json
{
  "ssh_key": {
    "id": "key-uuid",
    "name": "escudo-management-key",
    "ssh_key": "ssh-ed25519 AAAA...",
    "date_created": "2024-01-01T00:00:00+00:00"
  }
}
```

### List SSH Keys

```
GET /v2/ssh-keys
```

### Get SSH Key

```
GET /v2/ssh-keys/{ssh-key-id}
```

### Update SSH Key

```
PATCH /v2/ssh-keys/{ssh-key-id}
```

### Delete SSH Key

```
DELETE /v2/ssh-keys/{ssh-key-id}
```

**Usage:** Pass the SSH key ID in the `ssh_keys` array when creating an instance. Multiple keys can be added.

---

## 8. Additional IPs / Reserved IPs

### Add IPv4 to Instance

```
POST /v2/instances/{instance-id}/ipv4
```

```json
{
  "reboot": true
}
```

The `reboot` parameter controls whether the instance restarts to apply the new IP. The new IP is returned in the response.

### List Instance IPv4

```
GET /v2/instances/{instance-id}/ipv4
```

### Set Reverse DNS

```
POST /v2/instances/{instance-id}/ipv4/reverse
```

```json
{
  "ip": "203.0.113.50",
  "reverse": "vpn-mia-001.escudovpn.com"
}
```

### Reserved IPs (Floating IPs)

**Create Reserved IP:**

```
POST /v2/reserved-ips
```

```json
{
  "region": "mia",
  "ip_type": "v4",
  "label": "escudo-mia-floating"
}
```

Cost: **$3/month** ($0.004/hr) per reserved IP.

**Attach to Instance:**

```
POST /v2/reserved-ips/{reserved-ip}/attach
```

```json
{
  "instance_id": "instance-uuid"
}
```

**Detach:**

```
POST /v2/reserved-ips/{reserved-ip}/detach
```

**Key constraint:** A reserved IP can only be attached to one instance at a time, and only within the same region.

---

## 9. Firewalls

### Create Firewall Group

```
POST /v2/firewalls
```

```json
{
  "description": "escudo-vpn-firewall"
}
```

Response returns the `firewall_group` object with an `id`.

### List Firewall Groups

```
GET /v2/firewalls
```

### Delete Firewall Group

```
DELETE /v2/firewalls/{firewall-group-id}
```

### Create Firewall Rule

```
POST /v2/firewalls/{firewall-group-id}/rules
```

```json
{
  "ip_type": "v4",
  "protocol": "udp",
  "port": "51820",
  "subnet": "0.0.0.0",
  "subnet_size": 0,
  "notes": "WireGuard VPN"
}
```

| Field | Values |
|-------|--------|
| `ip_type` | `"v4"`, `"v6"` |
| `protocol` | `"tcp"`, `"udp"`, `"icmp"`, `"gre"` |
| `port` | Single port `"443"`, range `"8000:9000"`, or empty for ICMP |
| `subnet` | Source IP/network, `"0.0.0.0"` for any |
| `subnet_size` | CIDR prefix, `0` for any |
| `source` | Alternative: `""` (custom), `"cloudflare"` |
| `notes` | Description label |

### List Firewall Rules

```
GET /v2/firewalls/{firewall-group-id}/rules
```

### Delete Firewall Rule

```
DELETE /v2/firewalls/{firewall-group-id}/rules/{firewall-rule-id}
```

### Recommended VPN Firewall Rules

```
# WireGuard UDP
{ "ip_type": "v4", "protocol": "udp", "port": "51820", "subnet": "0.0.0.0", "subnet_size": 0, "notes": "WireGuard" }

# SSH management (restrict to your IP)
{ "ip_type": "v4", "protocol": "tcp", "port": "22", "subnet": "YOUR_MGMT_IP", "subnet_size": 32, "notes": "SSH Management" }

# ICMP for diagnostics
{ "ip_type": "v4", "protocol": "icmp", "subnet": "0.0.0.0", "subnet_size": 0, "notes": "ICMP" }

# IPv6 WireGuard
{ "ip_type": "v6", "protocol": "udp", "port": "51820", "subnet": "::", "subnet_size": 0, "notes": "WireGuard v6" }
```

---

## 10. Error Handling

### HTTP Status Codes

| Code | Meaning | Action |
|------|---------|--------|
| 200 | Success | Process response |
| 201 | Created | Resource created successfully |
| 204 | No Content | Successful delete |
| 400 | Bad Request | Fix request body/params — log and alert |
| 401 | Unauthorized | Invalid/expired API key — do not retry |
| 403 | Forbidden | Insufficient permissions — do not retry |
| 404 | Not Found | Resource doesn't exist — may be deleted already |
| 422 | Unprocessable | Validation error (e.g., plan not available in region) |
| 429 | Rate Limited | **Retry with exponential backoff** |
| 500 | Server Error | Retry with backoff, max 3 attempts |

### Error Response Format

All 4xx/5xx responses return JSON:

```json
{
  "error": "Description of what went wrong",
  "status": 400
}
```

### Retry Strategy for Rust Implementation

```
Retry policy:
  - 429: Backoff starting at 1s, doubling each retry, max 5 retries (1s, 2s, 4s, 8s, 16s)
  - 500/502/503/504: Backoff starting at 2s, max 3 retries
  - 400/401/403: Never retry (client error)
  - 404 on GET: May retry once (eventual consistency after creation)
  - Add jitter (0-500ms random) to prevent thundering herd

Rate limiter:
  - Token bucket: 25 tokens/sec, burst of 5
  - Queue requests when bucket empty rather than failing
```

### Common Gotchas

1. **Instance IP not immediately available:** After `POST /v2/instances`, the `main_ip` may be `"0.0.0.0"` initially. Poll until `status: "active"`.

2. **Startup script execution is fire-and-forget:** No API to check if a startup script completed. Implement a callback mechanism (script POSTs to your API when done).

3. **script_id vs user_data:** You can use both simultaneously. `script_id` runs the stored startup script; `user_data` provides cloud-init data. For Linux images with cloud-init, `user_data` is preferred.

4. **Base64 encoding:** Both `script` (in startup scripts) and `user_data` (in instance creation) must be base64-encoded. Use standard base64 (not URL-safe).

5. **Region availability:** Not all plans are available in all regions. Always check `/v2/regions/{id}/availability` before attempting to create an instance.

6. **Pagination:** All list endpoints use cursor-based pagination with `meta.links.next` and `meta.links.prev`. Loop until `next` is empty.

7. **Firewall default-deny:** Vultr firewall groups default to deny-all. You must explicitly add rules for any traffic you want to allow (including SSH).

8. **Destroy is immediate:** `DELETE /v2/instances/{id}` is irreversible with no confirmation. The instance is gone and billing stops immediately.

9. **SSH keys are install-time only:** SSH keys specified at instance creation are injected into `authorized_keys` during provisioning. Changing the SSH key resource later does not affect running instances.

10. **Metadata service:** Instances can query `http://169.254.169.254/v1/` for their own metadata (IP, region, plan, etc.). Useful for self-configuration scripts.

---

## Quick Reference: Full Provisioning Flow

```
1. POST /v2/ssh-keys                              -> ssh_key_id
2. POST /v2/startup-scripts                        -> script_id
3. POST /v2/firewalls                              -> firewall_group_id
4. POST /v2/firewalls/{fw_id}/rules                (add WireGuard + SSH rules)
5. POST /v2/instances                              -> instance_id
   {region, plan, os_id, script_id, ssh_keys, firewall_group_id, user_data, ...}
6. Poll GET /v2/instances/{id}                     until status=active, server_status=ok
7. (Optional) POST /v2/instances/{id}/ipv4         add extra IPs
8. (Optional) POST /v2/reserved-ips + attach       floating IP
9. Wait for startup script callback or SSH verify
10. Instance ready — VPN operational
```
