#!/usr/bin/env bash
set -euo pipefail

API_BASE="${API_BASE:-https://api.escudovpn.com}"
TARGET_SERVER="${TARGET_SERVER:-sp-01}"
PEERS="${PEERS:-5}"
DOWNLOAD_URL="${DOWNLOAD_URL:-https://speed.cloudflare.com/__down?bytes=5000000}"
AUTH_EMAIL="${AUTH_EMAIL:-}"
AUTH_PASSWORD="${AUTH_PASSWORD:-}"
OUT_ROOT="${OUT_ROOT:-/home/dev/pulsovpn/escudo-vpn/audits}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_ROOT/wg-peer-load-$TS"
mkdir -p "$OUT_DIR"

declare -a NAMESPACES=()
declare -a IFACES=()
declare -a DEVICE_IDS=()
declare -a TOKENS=()

json_headers() {
  local token="${1:-}"
  if [[ -n "$token" ]]; then
    printf 'Content-Type: application/json\nAuthorization: Bearer %s\n' "$token"
  else
    printf 'Content-Type: application/json\n'
  fi
}

cleanup() {
  local i
  for (( i=0; i<${#NAMESPACES[@]}; i++ )); do
    local ns="${NAMESPACES[$i]}"
    local iface="${IFACES[$i]:-}"
    local device_id="${DEVICE_IDS[$i]:-}"
    local token="${TOKENS[$i]:-}"
    sudo -n ip netns exec "$ns" wg-quick down "$iface" >/dev/null 2>&1 || true
    sudo -n ip netns del "$ns" >/dev/null 2>&1 || true
    sudo -n rm -rf "/etc/netns/$ns" >/dev/null 2>&1 || true
    sudo -n rm -f "/etc/wireguard/$iface.conf" >/dev/null 2>&1 || true
    if [[ -n "$device_id" && -n "$token" ]]; then
      curl -sS -o /dev/null -X DELETE \
        "$API_BASE/api/v1/disconnect/$device_id" \
        -H "Authorization: Bearer $token" || true
    fi
  done
}
trap cleanup EXIT

register_or_login() {
  local email="$1"
  local password="$2"
  local payload
  payload="$(jq -nc --arg email "$email" --arg password "$password" '{email:$email,password:$password}')"

  local register_res register_status
  register_res="$(mktemp)"
  register_status="$(curl -sS -o "$register_res" -w '%{http_code}' -X POST \
    "$API_BASE/api/v1/auth/register" \
    -H 'Content-Type: application/json' \
    -d "$payload")"
  if [[ "$register_status" == "200" ]]; then
    jq -r '.token' < "$register_res"
    rm -f "$register_res"
    return
  fi
  rm -f "$register_res"

  curl -fsS -X POST "$API_BASE/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    -d "$payload" | jq -r '.token'
}

server_id_for() {
  local token="$1"
  curl -fsS "$API_BASE/api/v1/servers" -H "Authorization: Bearer $token" |
    jq -r --arg target "$TARGET_SERVER" '.[] | select(.name == $target) | .id' | head -n1
}

wg_peer_public_key() {
  sed -n 's/^PublicKey = //p' | head -n1
}

extract_field() {
  local key="$1"
  sed -n "s/^$key = //p" | head -n1
}

setup_peer() {
  local idx="$1"
  local email password
  local token server_id payload resp cfg device_id iface ns dns peer_pub

  if [[ -n "$AUTH_EMAIL" && -n "$AUTH_PASSWORD" ]]; then
    email="$AUTH_EMAIL"
    password="$AUTH_PASSWORD"
  else
    email="load-${TS}-${idx}@escudovpn.test"
    password="LoadPass123!"
  fi

  token="$(register_or_login "$email" "$password")"
  server_id="$(server_id_for "$token")"
  if [[ -z "$server_id" ]]; then
    echo "peer $idx: target server '$TARGET_SERVER' not visible" >&2
    return 1
  fi

  payload="$(jq -nc --arg sid "$server_id" --arg dn "wg-load-$idx-$TS" '{server_id:$sid,device_name:$dn}')"
  resp="$(curl -fsS -X POST "$API_BASE/api/v1/connect" \
    -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' \
    -d "$payload")"
  cfg="$(printf '%s' "$resp" | jq -r '.config')"
  device_id="$(printf '%s' "$resp" | jq -r '.device_id')"
  iface="wgl$(printf '%03d' "$idx")"
  ns="wgload-$idx"
  dns="$(printf '%s\n' "$cfg" | extract_field DNS)"
  peer_pub="$(printf '%s\n' "$cfg" | wg_peer_public_key)"

  printf '%s\n' "$cfg" | sudo -n tee "/etc/wireguard/$iface.conf" >/dev/null
  sudo -n mkdir -p "/etc/netns/$ns"
  printf 'nameserver %s\n' "$dns" | sudo -n tee "/etc/netns/$ns/resolv.conf" >/dev/null
  sudo -n ip netns add "$ns"
  sudo -n ip netns exec "$ns" ip link set lo up
  sudo -n ip netns exec "$ns" wg-quick up "$iface" >/dev/null

  NAMESPACES+=("$ns")
  IFACES+=("$iface")
  DEVICE_IDS+=("$device_id")
  TOKENS+=("$token")

  jq -nc \
    --arg peer "$idx" \
    --arg email "$email" \
    --arg ns "$ns" \
    --arg iface "$iface" \
    --arg dns "$dns" \
    --arg peer_pub "$peer_pub" \
    --arg device_id "$device_id" \
    '{peer:$peer,email:$email,namespace:$ns,iface:$iface,dns:$dns,peer_public_key:$peer_pub,device_id:$device_id}' \
    > "$OUT_DIR/peer-$idx.setup.json"
}

run_peer_download() {
  local idx="$1"
  local ns="wgload-$idx"
  local iface="wgl$(printf '%03d' "$idx")"
  local dns handshake transfer

  dns="$(jq -r '.dns' < "$OUT_DIR/peer-$idx.setup.json")"
  handshake="$(sudo -n ip netns exec "$ns" wg show "$iface" latest-handshakes | awk 'NR==1 {print $2}')"
  {
    echo "{"
    echo "  \"peer\": $idx,"
    echo "  \"dns\": $(jq -Rn --arg v "$dns" '$v'),"
    echo "  \"handshake_before\": $(jq -Rn --arg v "${handshake:-}" '$v'),"
    echo "  \"dig_ipinfo\": $(sudo -n ip netns exec "$ns" dig +time=2 +tries=1 +short @"$dns" ipinfo.io A | jq -Rsc 'split("\n")[:-1]'),"
    echo "  \"download\":"
    sudo -n ip netns exec "$ns" curl -L -sS -o /dev/null \
      -w '{"http_code":%{http_code},"time_total":%{time_total},"speed_download":%{speed_download},"remote_ip":"%{remote_ip}"}' \
      "$DOWNLOAD_URL" || echo '{"error":"download_failed"}'
    echo ","
    echo "  \"ping_1_1_1_1\":"
    sudo -n ip netns exec "$ns" ping -c 2 -W 3 1.1.1.1 2>&1 | jq -Rsc '.'
    echo "}"
  } > "$OUT_DIR/peer-$idx.result.json"

  transfer="$(sudo -n ip netns exec "$ns" wg show "$iface" transfer | awk 'NR==1 {print $2" "$3}')"
  printf '%s\n' "$transfer" > "$OUT_DIR/peer-$idx.transfer.txt"
}

echo "out_dir=$OUT_DIR"
echo "target_server=$TARGET_SERVER"
echo "peers=$PEERS"

for idx in $(seq 1 "$PEERS"); do
  setup_peer "$idx"
done

sleep 3

for idx in $(seq 1 "$PEERS"); do
  run_peer_download "$idx" &
done
wait

jq -s '
  {
    peers: length,
    ok: map(select(.download.http_code == 200)) | length,
    p50_speed_download: (map(.download.speed_download // 0) | sort | .[(length/2|floor)]),
    p95_speed_download: (map(.download.speed_download // 0) | sort | .[((length*0.95)|floor)]),
    avg_time_total: (map(.download.time_total // 0) | add / (length|if .==0 then 1 else . end))
  }
' "$OUT_DIR"/peer-*.result.json > "$OUT_DIR/summary.json"

(cd "$OUT_DIR" && sha256sum ./* > SHA256SUMS)
cat "$OUT_DIR/summary.json"
