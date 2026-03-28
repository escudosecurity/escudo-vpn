# LightNode API Research — VPS Provisioning for Exotic Locations

**Date:** 2026-03-21
**Purpose:** Evaluate LightNode and alternatives for automated VPS provisioning in exotic locations

---

## 1. LightNode — Does It Have a Public REST API?

**Short answer: No usable public REST API exists.**

After exhaustive searching of:
- LightNode Documentation Center (doc.lightnode.com)
- LightNode main site (lightnode.com)
- LightNode tech blog (go.lightnode.com/tech)
- Third-party reviews and GitHub repos

**Findings:**
- The documentation center contains only GUI guides: how to sign up, recharge, manage instances via web console
- No API reference, no endpoint documentation, no authentication docs
- No Terraform provider, no CLI tool, no SDK
- One GitHub repo mentions "API access is available for automation, though documentation and integration support may be less comprehensive than larger providers offer" — but provides zero specifics
- The web console (console.lightnode.com) likely has internal APIs, but they are undocumented and not intended for public use
- No OpenAPI/Swagger spec found anywhere

**LightNode Pricing (for reference):**
- Cheapest visible plan: $8.70/month (1 vCPU, 2GB RAM, 50GB SSD, 2TB bandwidth)
- The $2.57/month plan referenced in some articles is not visible on the current pricing page — may have been a promotional or discontinued tier
- Hourly billing available ($0.012/hr)

**LightNode Locations (40+ total):**
USA, Germany, Turkey, Greece, Bulgaria, UK, France, Brazil, Argentina, Saudi Arabia, Dubai/UAE, Bahrain, Oman, Kuwait, Japan, Singapore, South Korea, Hong Kong, Vietnam, Cambodia, Philippines, Taiwan, Thailand, Bangladesh, Malaysia, Pakistan, South Africa, Egypt, Nepal, Russia, Mexico, Chile, and more.

**Verdict: LightNode cannot be used for automated provisioning. Manual-only via web console.**

---

## 2. Recommended Alternative: Vultr

Vultr is the strongest alternative — it has a mature REST API AND covers many of the exotic locations we need.

### 2.1 API Overview

| Item | Detail |
|------|--------|
| **Base URL** | `https://api.vultr.com/v2/` |
| **Auth** | Bearer token: `Authorization: Bearer {VULTR_API_KEY}` |
| **API Key** | Generated in Vultr dashboard under Account > API |
| **Format** | JSON request/response |
| **Rate Limit** | 30 requests/second per IP; returns HTTP 429 if exceeded |
| **Docs** | https://www.vultr.com/api/ |

### 2.2 Create Instance

```
POST /v2/instances
```

**Required fields:**
- `region` (string) — e.g. `"sao"` for São Paulo
- `plan` (string) — e.g. `"vc2-1c-1gb"`

**Deployment source (pick one):**
- `os_id` (integer) — OS image ID
- `snapshot_id` — from a saved snapshot
- `app_id` — one-click app
- `iso_id` — custom ISO

**Optional fields:**
- `label` — instance name
- `hostname` — system hostname
- `script_id` — startup script ID (created separately)
- `user_data` — base64-encoded cloud-init data
- `sshkey_id` — array of SSH key IDs
- `backups` — "enabled" or "disabled"
- `enable_ipv6` — boolean
- `ddos_protection` — boolean
- `firewall_group_id` — firewall group
- `tag` — deprecated, use `tags` array

**Example:**
```bash
curl "https://api.vultr.com/v2/instances" \
  -X POST \
  -H "Authorization: Bearer ${VULTR_API_KEY}" \
  -H "Content-Type: application/json" \
  --data '{
    "region": "sao",
    "plan": "vc2-1c-1gb",
    "os_id": 387,
    "label": "vpn-brazil-01",
    "script_id": "abc-123",
    "sshkey_id": ["ssh-key-id"],
    "enable_ipv6": true,
    "tags": ["vpn", "brazil"]
  }'
```

### 2.3 Startup Scripts

```
POST /v2/startup-scripts
```

```json
{
  "name": "vpn-setup",
  "type": "boot",
  "script": "<base64-encoded-script>"
}
```

- Script must be base64-encoded
- Types: `boot` (runs on first boot) or `pxe` (PXE provisioning)
- Returns a `script_id` to pass when creating instances

### 2.4 List Regions

```
GET /v2/regions
```

Returns region IDs, city names, countries, continents, and available plan types per region.

### 2.5 List Plans

```
GET /v2/plans
```

Returns plan IDs, vCPU count, RAM, disk, bandwidth, monthly cost, and which regions each plan is available in.

**Cheapest plans (as of March 2026):**

| Plan | vCPU | RAM | Disk | BW | Price |
|------|------|-----|------|----|-------|
| IPv6-only | 1 | 0.5GB | 10GB | 0.5TB | **$2.50/mo** |
| With IPv4 | 1 | 0.5GB | 10GB | 0.5TB | **$3.50/mo** |
| vc2-1c-1gb | 1 | 1GB | 25GB | 1TB | **$5.00/mo** |
| vc2-1c-2gb | 1 | 2GB | 55GB | 2TB | **$10.00/mo** |
| High Perf AMD | 1 | 1GB | 25GB | 2TB | **$6.00/mo** |

### 2.6 VPS Lifecycle

| Action | Method | Endpoint |
|--------|--------|----------|
| **List instances** | GET | `/v2/instances` |
| **Get instance** | GET | `/v2/instances/{instance-id}` |
| **Create** | POST | `/v2/instances` |
| **Delete/Destroy** | DELETE | `/v2/instances/{instance-id}` |
| **Halt (stop)** | POST | `/v2/instances/halt` with `{"instance_ids": [...]}` |
| **Reboot** | POST | `/v2/instances/{instance-id}/reboot` |
| **Reinstall** | POST | `/v2/instances/{instance-id}/reinstall` |
| **List OS** | GET | `/v2/os` |
| **List SSH keys** | GET | `/v2/ssh-keys` |

### 2.7 Vultr Data Center Locations (32 regions)

**Exotic locations we need (COVERED):**
- **Brazil** — São Paulo (`sao`)
- **South Africa** — Johannesburg (`jnb`)
- **Chile** — Santiago (`scl`)
- **Mexico** — Mexico City (`mex`)
- **South Korea** — Seoul (`icn`)
- **India** — Mumbai, Delhi NCR, Bangalore
- **Israel** — Tel Aviv

**Exotic locations we need (NOT COVERED by Vultr):**
- Turkey / Istanbul
- UAE / Dubai
- Vietnam
- Philippines
- Argentina
- Egypt

**Full Vultr region list:**
North America: Toronto, Mexico City, Atlanta, Honolulu, Chicago, Dallas, LA, Miami, NYC, SF Bay, Seattle
South America: São Paulo, Santiago
Europe: Amsterdam, London, Frankfurt, Paris, Madrid, Stockholm, Warsaw, Manchester
Asia: Tokyo, Osaka, Seoul, Singapore, Mumbai, Delhi NCR, Bangalore, Tel Aviv
Australia: Sydney, Melbourne
Africa: Johannesburg

### 2.8 Vultr Limitations & Gotchas

- Rate limit: 30 req/s — use exponential backoff on 429
- $2.50/mo plan is IPv6-only (no IPv4 address)
- Startup scripts run only on first boot (not on reboot)
- Instance creation is async — poll GET /v2/instances/{id} for status
- API key is account-wide, no per-project scoping
- Some plans not available in all regions — check plan.locations array

---

## 3. Other Alternatives Evaluated

### 3.1 Kamatera

**API:** Full REST API at `https://console.kamatera.com/service/`
- Auth: clientId + secret → 1-hour token
- Full CRUD for servers, snapshots, disks, networking
- Well-documented

**Locations (24 total):**
US (8), Canada, UK, Germany, Netherlands, Sweden, Spain, Italy, Israel (5), Hong Kong, Singapore, Tokyo, Sydney

**Verdict:** Great API, but **no exotic locations** (no Brazil, no Africa, no Turkey, no UAE, no Vietnam, no Philippines). Primarily US/Europe/Israel.

### 3.2 UpCloud

**API:** Full REST API at `https://api.upcloud.com/1.3/`
- Well-documented at developers.upcloud.com
- Terraform provider available

**Locations (~15 total):**
Helsinki, London, Frankfurt, Amsterdam, Madrid, Warsaw, Chicago, NYC, San Jose, Singapore, Sydney, Copenhagen, Stavanger (Norway), plus Nordic expansion

**Verdict:** Good API, but **even fewer exotic locations** than Kamatera. Focused on Europe and Nordics.

### 3.3 OVHcloud

Has API and Brazil location, but complex authentication (application key + consumer key + timestamp signing). Limited exotic coverage.

---

## 4. Recommended Strategy

Given the findings, a **multi-provider approach** is recommended:

### Tier 1 — Vultr (primary, API-driven)
Use for: Brazil, South Africa, Chile, Mexico, US, Europe, Asia-Pacific, Australia
- Covers ~70% of target locations
- Best API, best docs, cheapest plans ($5/mo for usable specs)
- Terraform provider available

### Tier 2 — LightNode (manual or semi-automated)
Use for locations Vultr doesn't cover: Turkey, UAE/Dubai, Vietnam, Philippines, Argentina, Egypt, Cambodia, Bangladesh, Pakistan, Nepal
- Must provision via web console (console.lightnode.com) or potentially reverse-engineer their console API
- Could use browser automation (Playwright/Puppeteer) as a last resort
- 40+ locations including all the exotic ones we need

### Tier 3 — Kamatera (backup API provider)
Use if Vultr has capacity/availability issues in a given region.

### Coverage Matrix

| Location | Vultr | LightNode | Kamatera |
|----------|-------|-----------|----------|
| Brazil (São Paulo) | YES | YES | no |
| Argentina | no | YES | no |
| South Africa (Johannesburg) | YES | YES | no |
| Turkey (Istanbul) | no | YES | no |
| UAE (Dubai) | no | YES | no |
| Vietnam | no | YES | no |
| Philippines | no | YES | no |
| Egypt | no | YES | no |
| Chile (Santiago) | YES | YES | no |
| Mexico | YES | YES | no |
| India (Mumbai/Delhi/Bangalore) | YES | no | no |
| Singapore | YES | YES | YES |
| Japan (Tokyo) | YES | no | YES |
| South Korea (Seoul) | YES | no | no |

---

## 5. Key Takeaway

**Vultr should be the primary provider** — it has the best API, covers 32 regions including Brazil and South Africa, and starts at $5/mo for usable specs. For the truly exotic locations (Turkey, UAE, Vietnam, Philippines, Argentina), LightNode is the only option but requires manual provisioning or creative automation since they have no public API.
