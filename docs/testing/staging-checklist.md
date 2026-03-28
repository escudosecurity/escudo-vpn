# EscudoVPN Staging Checklist

Last updated: 2026-03-21
Owner: release engineering

This checklist is the pre-distribution gate for Android and the backend surfaces it depends on.
Do not distribute signed Android artifacts until every `Must Pass` item is green or has a written exception signed off in the release record.

## Inputs

- Staging API base URL, for example `https://api.escudovpn.com`
- At least 2 active VPN servers in staging
- A staging user account
- Android release signing env vars available on the build host
- `cargo`, `curl`, and `openssl` installed
- `k6` installed for synthetic API tests

## Must Pass

- `cargo check --workspace`
- `cargo audit`
- `curl -I $API_BASE/health` returns `200 OK`
- HTTP redirects to HTTPS for the public API host
- TLS certificate for `api.escudovpn.com` is valid and pin set matches release config
- Android release build succeeds with release signing env vars set
- Android release build fails if release signing env vars are missing
- Auth flow works: register or login returns a JWT
- `GET /api/v1/servers` works with JWT auth
- `POST /api/v1/connect` returns config, QR, and a `device_id`
- `DELETE /api/v1/disconnect/:id` succeeds and does not leave an active device behind
- `GET /api/v1/stats/dns` is auth-protected and user-scoped
- `GET /api/v1/ws/stats` is auth-protected
- Synthetic API smoke test passes
- Synthetic connect-cycle test passes
- 24-hour soak plan is scheduled or complete before broad distribution

## Local Verification Commands

Run from repo root unless noted otherwise.

```bash
cargo check --workspace
cargo audit
curl -I --max-time 10 "$API_BASE/health"
curl -I --max-time 10 "http://api.escudovpn.com/health"
curl -I --max-time 10 "https://api.escudovpn.com/health"
openssl s_client -connect api.escudovpn.com:443 -servername api.escudovpn.com </dev/null
```

Release-signing guard:

```bash
cd apps/android
env GRADLE_USER_HOME=/tmp/escudo-gradle ./gradlew :app:assembleRelease
```

Expected result without signing env vars:

```text
Release signing is required. Set ESCUDO_UPLOAD_STORE_FILE, ESCUDO_UPLOAD_STORE_PASSWORD, ESCUDO_UPLOAD_KEY_ALIAS, and ESCUDO_UPLOAD_KEY_PASSWORD.
```

Signed artifact build:

```bash
cd apps/android
set -a
. ./release-signing.env
set +a
env GRADLE_USER_HOME=/tmp/escudo-gradle ./gradlew :app:assembleRelease :app:bundleRelease
```

## Synthetic Test Commands

Smoke:

```bash
k6 run scripts/load/k6-api-smoke.js \
  -e API_BASE="$API_BASE" \
  -e TEST_EMAIL="$TEST_EMAIL" \
  -e TEST_PASSWORD="$TEST_PASSWORD"
```

Connect cycle:

```bash
k6 run scripts/load/k6-connect-cycle.js \
  -e API_BASE="$API_BASE" \
  -e TEST_EMAIL="$TEST_EMAIL" \
  -e TEST_PASSWORD="$TEST_PASSWORD" \
  -e DEVICE_PREFIX="staging-cycle"
```

## Android Runtime Matrix

- Pixel, current Android
- Samsung, current Android
- Xiaomi or other aggressive battery-managed OEM
- One older Android 8 or 9 device

For each device, verify:

- install succeeds
- login succeeds
- server list loads
- connect succeeds
- disconnect succeeds
- server switch succeeds
- app survives backgrounding and resume
- VPN permission flow is correct
- cert pinning succeeds against `api.escudovpn.com`
- no cleartext traffic fallback
- reconnect protection behavior matches UI wording

## Failure Injection Matrix

- API down for 60s
- gateway down for 60s
- DB down for 60s
- DNS stats DB unavailable
- TLS termination reload while clients are active
- webhook retry burst
- disk usage above 85%

Expected outcomes:

- API fails closed
- no orphan active devices after failed connect
- no plaintext secret leakage in client-visible errors
- reconnect protection does not misrepresent itself as a hard kill switch
- services recover without manual DB cleanup

## Soak Test

Run a 24-hour staging soak before broad release:

- 200 synthetic users
- periodic login, server list, connect, disconnect
- DNS traffic through the tunnel
- service restarts on one node every few hours

Collect:

- API p50, p95, p99 latency
- 4xx and 5xx rates
- DB connections
- memory and FD growth
- gateway peer counts
- DNS flush failures
- restart counts

## Release Decision

Only sign off if:

- all `Must Pass` checks are green
- unresolved risks are documented
- rollback plan exists
- artifacts are signed and checksummed
- release owner and security reviewer both approve
