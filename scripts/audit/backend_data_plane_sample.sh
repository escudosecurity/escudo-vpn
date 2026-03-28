#!/usr/bin/env bash
set -euo pipefail

API_BASE="${API_BASE:-https://api.escudovpn.com}"
OUT_ROOT="${OUT_ROOT:-/home/dev/pulsovpn/escudo-vpn/audits}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_ROOT/backend-data-plane-$TS"
mkdir -p "$OUT_DIR"
ONLY_SAMPLE="${ONLY_SAMPLE:-}"

ESCUDO_EMAIL="${ESCUDO_EMAIL:-audit-direct@escudovpn.test}"
ESCUDO_PASSWORD="${ESCUDO_PASSWORD:-AuditPass123!}"
PRO_EMAIL="${PRO_EMAIL:-test@escudovpn.test}"
PRO_PASSWORD="${PRO_PASSWORD:-Test123!}"
DEDICATED_EMAIL="${DEDICATED_EMAIL:-ritafiol@private.com}"
DEDICATED_PASSWORD="${DEDICATED_PASSWORD:-Private01}"

log() {
  printf '%s %s\n' "$(date -u +%FT%TZ)" "$*"
}

pad_b64() {
  local value="$1"
  local mod=$(( ${#value} % 4 ))
  if [[ $mod -eq 2 ]]; then
    printf '%s==' "$value"
  elif [[ $mod -eq 3 ]]; then
    printf '%s=' "$value"
  else
    printf '%s' "$value"
  fi
}

login_token() {
  local email="$1"
  local password="$2"
  curl -fsS -X POST "$API_BASE/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$email\",\"password\":\"$password\"}" | jq -r '.token'
}

find_server_id() {
  local token="$1"
  local name="$2"
  curl -fsS "$API_BASE/api/v1/servers" -H "Authorization: Bearer $token" |
    jq -r --arg name "$name" '.[] | select(.name == $name) | .id' | head -n1
}

extract_field() {
  local key="$1"
  sed -n "s/^$key = //p" | head -n1
}

resolve_host_in_ns() {
  local ns="$1"
  local dns="$2"
  local host="$3"
  sudo -n ip netns exec "$ns" dig +time=2 +tries=1 +short @"$dns" "$host" A | head -n1
}

https_head_via_ns() {
  local ns="$1"
  local dns="$2"
  local host="$3"
  local path="$4"
  local ip
  ip="$(resolve_host_in_ns "$ns" "$dns" "$host")"
  [[ -n "$ip" ]] || return 1
  sudo -n ip netns exec "$ns" curl -I -L --max-time 20 -sS \
    --resolve "$host:443:$ip" "https://$host$path"
}

wg_peer_public_key() {
  sed -n 's/^PublicKey = //p' | head -n1
}

json_escape() {
  jq -Rn --arg v "$1" '$v'
}

sample_tunnel() {
  local slug="$1"
  local email="$2"
  local password="$3"
  local server_name="$4"
  local url="$5"

  if [[ -n "$ONLY_SAMPLE" && "$slug" != "$ONLY_SAMPLE" ]]; then
    return
  fi

  local token server_id body resp cfg device_id ns conf ifname dns out peer_pub handshake transfer_rx transfer_tx
  token="$(login_token "$email" "$password")"
  server_id="$(find_server_id "$token" "$server_name")"
  if [[ -z "$server_id" ]]; then
    printf '{"slug":"%s","server":"%s","error":"server_not_found"}\n' "$slug" "$server_name" > "$OUT_DIR/$slug.json"
    return
  fi

  body="$(jq -nc --arg sid "$server_id" --arg dn "sample-$slug-$TS" '{server_id:$sid,device_name:$dn}')"
  resp="$(curl -fsS -X POST "$API_BASE/api/v1/connect" \
    -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' \
    -d "$body")"

  cfg="$(printf '%s' "$resp" | jq -r '.config' | awk '
    function pad(v) {
      mod = length(v) % 4
      if (mod == 2) return v "=="
      if (mod == 3) return v "="
      return v
    }
    /^PrivateKey = / { print "PrivateKey = " pad(substr($0, 14)); next }
    /^PublicKey = / { print "PublicKey = " pad(substr($0, 13)); next }
    /^PresharedKey = / { print "PresharedKey = " pad(substr($0, 16)); next }
    { print }
  ')"
  device_id="$(printf '%s' "$resp" | jq -r '.device_id')"
  peer_pub="$(printf '%s\n' "$cfg" | wg_peer_public_key)"
  ifname="wga$(printf '%s' "$slug" | tr -cd 'a-z0-9' | cut -c1-10)"
  conf="/etc/wireguard/$ifname.conf"
  printf '%s\n' "$cfg" | sudo -n tee "$conf" >/dev/null

  ns="escudo-${slug//[^a-zA-Z0-9]/-}"
  dns="$(printf '%s\n' "$cfg" | extract_field DNS)"

  sudo -n ip netns del "$ns" 2>/dev/null || true
  sudo -n mkdir -p "/etc/netns/$ns"
  printf 'nameserver %s\n' "$dns" | sudo -n tee "/etc/netns/$ns/resolv.conf" >/dev/null
  sudo -n ip netns add "$ns"
  sudo -n ip netns exec "$ns" ip link set lo up
  sudo -n ip netns exec "$ns" wg-quick up "$ifname" >/dev/null
  sleep 3
  handshake="$(sudo -n ip netns exec "$ns" wg show "$ifname" latest-handshakes 2>/dev/null | awk -v key="$peer_pub" '$1==key {print $2}' | head -n1)"
  transfer_rx="$(sudo -n ip netns exec "$ns" wg show "$ifname" transfer 2>/dev/null | awk -v key="$peer_pub" '$1==key {print $2}' | head -n1)"
  transfer_tx="$(sudo -n ip netns exec "$ns" wg show "$ifname" transfer 2>/dev/null | awk -v key="$peer_pub" '$1==key {print $3}' | head -n1)"

  {
    local target_host target_path
    target_host="${url#https://}"
    if [[ "$target_host" == */* ]]; then
      target_path="/${target_host#*/}"
      target_host="${target_host%%/*}"
    else
      target_path="/"
    fi
    echo "{"
    echo "  \"slug\": \"$slug\","
    echo "  \"server\": \"$server_name\","
    echo "  \"url\": \"$url\","
    echo "  \"dns_server\": $(json_escape "$dns"),"
    echo "  \"peer_public_key\": $(json_escape "$peer_pub"),"
    echo "  \"latest_handshake\": $(json_escape "${handshake:-}"),"
    echo "  \"transfer_rx\": $(json_escape "${transfer_rx:-}"),"
    echo "  \"transfer_tx\": $(json_escape "${transfer_tx:-}"),"
    echo "  \"ping_1_1_1_1\":"
    sudo -n ip netns exec "$ns" ping -c 2 -W 3 1.1.1.1 2>&1 | sed 's/^/    /' || true
    echo ","
    echo "  \"tcp_1_1_1_1_443\":"
    sudo -n ip netns exec "$ns" timeout 10 bash -lc 'cat < /dev/null > /dev/tcp/1.1.1.1/443' 2>&1 | sed 's/^/    /' || true
    echo ","
    if [[ -n "${handshake:-}" && "${handshake:-0}" != "0" ]]; then
      echo "  \"dns_ipinfo\":"
      resolve_host_in_ns "$ns" "$dns" "ipinfo.io" || true
      echo ","
      echo "  \"ipinfo_head\":"
      https_head_via_ns "$ns" "$dns" "ipinfo.io" "/json" | sed 's/^/    /' || true
      echo ","
      echo "  \"youtube_head\":"
      https_head_via_ns "$ns" "$dns" "www.youtube.com" "/" | sed 's/^/    /' || true
      echo ","
      echo "  \"target_head\":"
      https_head_via_ns "$ns" "$dns" "$target_host" "$target_path" | sed 's/^/    /' || true
      echo ","
      echo "  \"dns_doubleclick\":"
      sudo -n ip netns exec "$ns" dig +time=2 +tries=1 +short @"$dns" doubleclick.net || true
      echo ","
      echo "  \"dns_google_analytics\":"
      sudo -n ip netns exec "$ns" dig +time=2 +tries=1 +short @"$dns" google-analytics.com || true
    else
      echo "  \"dns_ipinfo\": \"skipped_no_handshake\","
      echo "  \"ipinfo_head\": \"skipped_no_handshake\","
      echo "  \"youtube_head\": \"skipped_no_handshake\","
      echo "  \"target_head\": \"skipped_no_handshake\","
      echo "  \"dns_doubleclick\": \"skipped_no_handshake\","
      echo "  \"dns_google_analytics\": \"skipped_no_handshake\""
    fi
    echo "}"
  } > "$OUT_DIR/$slug.json"

  sudo -n ip netns exec "$ns" wg-quick down "$ifname" >/dev/null || true
  sudo -n ip netns del "$ns" || true
  sudo -n rm -rf "/etc/netns/$ns"
  sudo -n rm -f "$conf"

  curl -sS -o /dev/null -X DELETE \
    "$API_BASE/api/v1/disconnect/$device_id" \
    -H "Authorization: Bearer $token" || true
}

log "running data-plane samples"
sample_tunnel "direct-london" "$ESCUDO_EMAIL" "$ESCUDO_PASSWORD" "escudo-lon-01" "https://www.youtube.com"
sample_tunnel "direct-singapore" "$ESCUDO_EMAIL" "$ESCUDO_PASSWORD" "104.248.145.138" "https://www.youtube.com"
sample_tunnel "pro-uk" "$PRO_EMAIL" "$PRO_PASSWORD" "escudo-falkenstein" "https://www.bbc.co.uk/iplayer"
sample_tunnel "pro-us" "$PRO_EMAIL" "$PRO_PASSWORD" "escudo-ashburn" "https://www.youtube.com"
sample_tunnel "pro-br" "$PRO_EMAIL" "$PRO_PASSWORD" "sp-01" "https://globoplay.globo.com/"
sample_tunnel "dedicated-br" "$DEDICATED_EMAIL" "$DEDICATED_PASSWORD" "sp-01" "https://globoplay.globo.com/"

(cd "$OUT_DIR" && sha256sum ./* > SHA256SUMS)
log "sample_complete out_dir=$OUT_DIR"
printf '%s\n' "$OUT_DIR"
