#!/usr/bin/env bash
set -euo pipefail

API_BASE="${API_BASE:-https://api.escudovpn.com}"
OUT_ROOT="${OUT_ROOT:-/home/dev/pulsovpn/escudo-vpn/audits}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_ROOT/launch-stress-$TS"
mkdir -p "$OUT_DIR"

TEST_EMAIL="${TEST_EMAIL:-test@escudovpn.test}"
TEST_PASSWORD="${TEST_PASSWORD:-Test123!}"
DNS_SERVER="${DNS_SERVER:-10.0.0.1}"
DNS_QUERIES="${DNS_QUERIES:-/home/dev/pulsovpn/escudo-vpn/scripts/load/dnsperf-queries.txt}"
DNS_QPS="${DNS_QPS:-200}"
DNS_CLIENTS="${DNS_CLIENTS:-10}"
K6_VUS="${K6_VUS:-20}"
K6_ITERATIONS="${K6_ITERATIONS:-60}"
WG_PEERS="${WG_PEERS:-5}"
WG_TARGET_SERVER="${WG_TARGET_SERVER:-sp-01}"
WG_AUTH_EMAIL="${WG_AUTH_EMAIL:-$TEST_EMAIL}"
WG_AUTH_PASSWORD="${WG_AUTH_PASSWORD:-$TEST_PASSWORD}"

echo "out_dir=$OUT_DIR"

{
  echo "api_base=$API_BASE"
  echo "dns_server=$DNS_SERVER"
  echo "dns_qps=$DNS_QPS"
  echo "dns_clients=$DNS_CLIENTS"
  echo "k6_vus=$K6_VUS"
  echo "k6_iterations=$K6_ITERATIONS"
  echo "wg_peers=$WG_PEERS"
  echo "wg_target_server=$WG_TARGET_SERVER"
  echo "wg_auth_email=$WG_AUTH_EMAIL"
} > "$OUT_DIR/config.env"

k6 run \
  --vus "$K6_VUS" \
  --iterations "$K6_ITERATIONS" \
  --summary-export "$OUT_DIR/k6-connect-summary.json" \
  /home/dev/pulsovpn/escudo-vpn/scripts/load/k6-connect-cycle.js \
  -e API_BASE="$API_BASE" \
  -e TEST_EMAIL="$TEST_EMAIL" \
  -e TEST_PASSWORD="$TEST_PASSWORD" \
  -e DEVICE_PREFIX="launch-stress" \
  > "$OUT_DIR/k6-connect.log" 2>&1 || true

dnsperf \
  -s "$DNS_SERVER" \
  -d "$DNS_QUERIES" \
  -Q "$DNS_QPS" \
  -c "$DNS_CLIENTS" \
  -l 15 \
  > "$OUT_DIR/dnsperf.log" 2>&1 || true

PEERS="$WG_PEERS" \
TARGET_SERVER="$WG_TARGET_SERVER" \
AUTH_EMAIL="$WG_AUTH_EMAIL" \
AUTH_PASSWORD="$WG_AUTH_PASSWORD" \
OUT_ROOT="$OUT_DIR" \
bash /home/dev/pulsovpn/escudo-vpn/scripts/load/wg-peer-load.sh \
  > "$OUT_DIR/wg-peer-load.log" 2>&1 || true

find "$OUT_DIR" -maxdepth 1 -type f -print0 | sort -z | xargs -0 sha256sum > "$OUT_DIR/SHA256SUMS"
echo "$OUT_DIR"
