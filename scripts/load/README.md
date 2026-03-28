# EscudoVPN Synthetic Load Tests

These scripts exercise the staging API contract that the Android client depends on.

## Prerequisites

- `k6` installed
- a reachable staging API
- a test user account

## Environment

- `API_BASE` example: `https://api.escudovpn.com`
- `TEST_EMAIL`
- `TEST_PASSWORD`
- `DEVICE_PREFIX` optional, default: `k6-device`

## Scripts

- `k6-api-smoke.js`
  - health
  - login or register
  - authenticated server list
  - authenticated DNS stats
  - authenticated websocket handshake check

- `k6-connect-cycle.js`
  - login or register
  - fetch server list
  - connect
  - verify config and QR presence
  - disconnect

## Example

```bash
k6 run scripts/load/k6-api-smoke.js \
  -e API_BASE="https://api.escudovpn.com" \
  -e TEST_EMAIL="staging@example.com" \
  -e TEST_PASSWORD="change-me-please"
```

```bash
k6 run scripts/load/k6-connect-cycle.js \
  -e API_BASE="https://api.escudovpn.com" \
  -e TEST_EMAIL="staging@example.com" \
  -e TEST_PASSWORD="change-me-please" \
  -e DEVICE_PREFIX="staging-cycle"
```

## Notes

- These scripts are meant for staging and controlled synthetic load, not anonymous public execution.
- `connect` allocates VPN devices, so keep VU counts realistic until staging cleanup automation is in place.
- If register returns `409 Conflict`, the scripts fall back to login.
