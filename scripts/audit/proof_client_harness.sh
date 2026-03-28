#!/usr/bin/env bash
set -euo pipefail

API_BASE="${API_BASE:-http://127.0.0.1:3000/api/v1}"
DB_URL="${DB_URL:-postgresql://escudo:escudo_secret@localhost/escudo}"
OUTPUT_DIR="${OUTPUT_DIR:-/tmp/escudo-proof}"

mkdir -p "$OUTPUT_DIR"

require_bin() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required binary: $1" >&2
    exit 1
  }
}

require_bin curl
require_bin jq
require_bin psql
require_bin ip
require_bin iptables
require_bin wg-quick

RESOLV_DOWNSTR=$(resolvectl dns eth0 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+')
DNS_SERVERS="${DNS_SERVERS:-$RESOLV_DOWNSTR}"
if [[ -z "$DNS_SERVERS" ]]; then
  DNS_SERVERS="1.1.1.1 8.8.8.8"
fi

cleanup_ns() {
  local ns="$1"
  local peer="$2"
  ip netns del "$ns" 2>/dev/null || true
  ip link del "$peer" 2>/dev/null || true
  rm -rf "/etc/netns/$ns"
}

create_account() {
  local tier="$1"
  local json account login token user_id stripe_customer stripe_subscription

  json="$(curl -fsS -X POST "$API_BASE/auth/anonymous" -H "Content-Type: application/json")"
  account="$(echo "$json" | jq -r '.account_number')"
  login="$(curl -fsS -X POST "$API_BASE/auth/login-number" -H "Content-Type: application/json" -d "{\"account_number\":\"$account\"}")"
  token="$(echo "$login" | jq -r '.token')"
  user_id="$(echo "$login" | jq -r '.user_id')"
  stripe_customer="proof_cus_${user_id//-/}"
  stripe_subscription="proof_sub_${user_id//-/}"

  psql "$DB_URL" -c \
    "INSERT INTO subscriptions (
       user_id, stripe_customer_id, stripe_subscription_id, plan, status, period_start, period_end, tier
     ) VALUES (
       '$user_id', '$stripe_customer', '$stripe_subscription', '$tier', 'active', now(), now() + interval '30 days', '$tier'
     )" >/dev/null

  jq -n \
    --arg account_number "$account" \
    --arg token "$token" \
    --arg user_id "$user_id" \
    --arg tier "$tier" \
    '{account_number:$account_number, token:$token, user_id:$user_id, tier:$tier}'
}

connect_account() {
  local token="$1"
  local server_id="$2"
  local device_name="$3"

  curl -fsS -X POST "$API_BASE/connect" \
    -H "Authorization: Bearer $token" \
    -H "Content-Type: application/json" \
    -d "{\"server_id\":\"$server_id\",\"device_name\":\"$device_name\"}"
}

write_wg_config() {
  local raw_config="$1"
  local path="$2"
  printf "%s\n" "$raw_config" | sed '/^DNS = /d' >"$path"
}

start_namespace_client() {
  local ns="$1"
  local peer="$2"
  local veth="$3"
  local subnet_prefix="$4"
  local config_path="$5"

  cleanup_ns "$ns" "$peer"
  sysctl -w net.ipv4.ip_forward=1 >/dev/null
  iptables -t nat -C POSTROUTING -s "${subnet_prefix}.0/24" -j MASQUERADE 2>/dev/null || \
    iptables -t nat -A POSTROUTING -s "${subnet_prefix}.0/24" -j MASQUERADE

  ip netns add "$ns"
  mkdir -p "/etc/netns/$ns"
  local dns_servers="$DNS_SERVERS"
  {
    for ip in $dns_servers; do
      echo "nameserver $ip"
    done
  } >"/etc/netns/$ns/resolv.conf"
  cat /etc/netns/$ns/resolv.conf >&2
  ip link add "$veth" type veth peer name "$peer"
  ip link set "$veth" netns "$ns"
  ip addr add "${subnet_prefix}.1/24" dev "$peer"
  ip link set "$peer" up
  ip netns exec "$ns" ip addr add "${subnet_prefix}.2/24" dev "$veth"
  ip netns exec "$ns" ip link set lo up
  ip netns exec "$ns" ip link set "$veth" up
  ip netns exec "$ns" ip route add default via "${subnet_prefix}.1" dev "$veth"
  ip netns exec "$ns" wg-quick up "$config_path"
}

maybe_cleanup_trap() {
  if [[ -z "${KEEP_NS:-}" ]]; then
    trap 'cleanup_ns proofbr1 pbrpeer1 2>/dev/null' EXIT
  else
    trap - EXIT
  fi
}

probe_namespace() {
  local ns="$1"
  local output_json="$2"
  local netflix_txt="$3"
  local regional_txt="$4"
  local regional_url="$5"

  local ipinfo_ip="${IPINFO_IP:-34.117.59.81}"
  echo "ip netns exec $ns curl --connect-to ipinfo.io:443:${ipinfo_ip} -fsS --max-time 20 https://ipinfo.io/json" >&2
  ip netns exec "$ns" curl --connect-to ipinfo.io:443:${ipinfo_ip} -fsS --max-time 20 https://ipinfo.io/json >"$output_json"
  local netflix_ip="${NETFLIX_IP:-54.155.178.5}"
  echo "ip netns exec $ns curl --connect-to www.netflix.com:443:${netflix_ip} -s -o /dev/null -w \"%{http_code}\\n\" --max-time 20 https://www.netflix.com" >&2
  ip netns exec "$ns" curl --connect-to www.netflix.com:443:${netflix_ip} -s -o /dev/null -w "%{http_code}\n" --max-time 20 https://www.netflix.com >"$netflix_txt"
  local regional_ip="${REGIONAL_IP:-34.128.172.221}"
  local regional_host
  regional_host="$(echo "$regional_url" | awk -F/ '{print $3}')"
  echo "ip netns exec $ns curl --connect-to ${regional_host}:443:${regional_ip} -s -o /dev/null -w \"%{http_code}\\n\" --max-time 20 $regional_url" >&2
  ip netns exec "$ns" curl --connect-to "${regional_host}":443:"${regional_ip}" -s -o /dev/null -w "%{http_code}\n" --max-time 20 "$regional_url" >"$regional_txt"
}
