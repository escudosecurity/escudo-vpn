#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <host>" >&2
  exit 2
fi

HOST="$1"
API_BASE="${API_BASE:-https://api.escudovpn.com}"
AUTH_EMAIL="${AUTH_EMAIL:-loadpro@escudovpn.test}"
AUTH_PASSWORD="${AUTH_PASSWORD:-LoadPass123}"
TARGET_SERVER="${TARGET_SERVER:-sp-01}"
TARGET_SERVER_ID="${TARGET_SERVER_ID:-}"
DOWNLOAD_URL="${DOWNLOAD_URL:-https://speed.cloudflare.com/__down?bytes=10000000}"
DOWNLOAD_HOST="${DOWNLOAD_HOST:-speed.cloudflare.com}"
IPINFO_IP="${IPINFO_IP:-34.117.59.81}"
SPEED_IP="${SPEED_IP:-172.66.0.218}"
SSH_AUTH_MODE="${SSH_AUTH_MODE:-default}"
INSTANCE_ID="${INSTANCE_ID:-$(date +%s%N | tail -c 6)}"
API_TOKEN="${API_TOKEN:-}"

TOKEN="$API_TOKEN"
if [[ -z "$TOKEN" ]]; then
  TOKEN="$(curl -sS -X POST "$API_BASE/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$AUTH_EMAIL\",\"password\":\"$AUTH_PASSWORD\"}" | jq -r '.token')"
fi
SID="$TARGET_SERVER_ID"
if [[ -z "$SID" ]]; then
  SERVERS_RESP="$(curl -sS "$API_BASE/api/v1/servers" -H "Authorization: Bearer $TOKEN")"
  SID="$(printf '%s' "$SERVERS_RESP" |
    jq -r --arg name "$TARGET_SERVER" '.[] | select(.name==$name) | .id' | head -n1)"
fi
RESP="$(curl -sS -X POST "$API_BASE/api/v1/connect" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"server_id\":\"$SID\",\"device_name\":\"dist-ns-${HOST//[^a-zA-Z0-9]/-}-$(date +%s)\"}")"
DID="$(printf '%s' "$RESP" | jq -r '.device_id')"
CFG="$(printf '%s' "$RESP" | jq -r '.config')"
echo "===API==="
echo "host=$HOST instance=$INSTANCE_ID sid=$SID did=$DID"
if [[ -z "$TOKEN" || "$TOKEN" == "null" ]]; then
  echo "api_error=login_failed"
  exit 1
fi
if [[ -z "$SID" || "$SID" == "null" ]]; then
  echo "api_error=server_not_found"
  exit 1
fi
if [[ -z "$DID" || "$DID" == "null" || -z "$CFG" || "$CFG" == "null" ]]; then
  echo "api_error=connect_failed"
  printf '%s\n' "$RESP"
  exit 1
fi
CFG="$(printf '%s\n' "$CFG" | sed '/^DNS = /d')"
CFG_B64="$(printf '%s' "$CFG" | base64 -w0)"

cleanup_api() {
  curl -sS -o /dev/null -X DELETE \
    "$API_BASE/api/v1/disconnect/$DID" \
    -H "Authorization: Bearer $TOKEN" || true
}
trap cleanup_api EXIT

case "$SSH_AUTH_MODE" in
  lightnode)
    ssh_cmd=(ssh -i /home/dev/.ssh/lightnode_rsa -o BatchMode=yes -o StrictHostKeyChecking=no)
    ;;
  kamatera)
    ssh_cmd=(sshpass -p Escud0VPN2026x ssh -o StrictHostKeyChecking=no -o PreferredAuthentications=password -o PubkeyAuthentication=no)
    ;;
  *)
    ssh_cmd=(ssh -o BatchMode=yes)
    ;;
esac

"${ssh_cmd[@]}" "root@$HOST" "CFG_B64='$CFG_B64' DOWNLOAD_URL='$DOWNLOAD_URL' DOWNLOAD_HOST='$DOWNLOAD_HOST' IPINFO_IP='$IPINFO_IP' SPEED_IP='$SPEED_IP' INSTANCE_ID='$INSTANCE_ID' bash -s" <<'EOF'
set -euo pipefail
NS="ltns${INSTANCE_ID}"
IF="ltwg${INSTANCE_ID}"
V0="ltv${INSTANCE_ID}a"
V1="ltv${INSTANCE_ID}b"
DEV="$(ip route | awk '/default/ {print $5; exit}')"
NET_ID="$((10#${INSTANCE_ID} % 200 + 20))"
HOST_VETH_IP="169.254.${NET_ID}.1/30"
NS_VETH_IP="169.254.${NET_ID}.2/30"
NS_GW_IP="169.254.${NET_ID}.1"

iptables -t nat -D POSTROUTING -s "${NS_VETH_IP%/30}/32" -o "$DEV" -j MASQUERADE >/dev/null 2>&1 || true
ip netns del "$NS" >/dev/null 2>&1 || true
ip link del "$V0" >/dev/null 2>&1 || true
ip link del "$V1" >/dev/null 2>&1 || true
rm -f "/etc/wireguard/$IF.conf"

ip netns add "$NS"
ip link add "$V0" type veth peer name "$V1"
ip addr add "$HOST_VETH_IP" dev "$V0"
ip link set "$V0" up
ip link set "$V1" netns "$NS"
ip netns exec "$NS" ip addr add "$NS_VETH_IP" dev "$V1"
ip netns exec "$NS" ip link set lo up
ip netns exec "$NS" ip link set "$V1" up
ip netns exec "$NS" ip route add default via "$NS_GW_IP"
sysctl -w net.ipv4.ip_forward=1 >/dev/null
iptables -t nat -A POSTROUTING -s "${NS_VETH_IP%/30}/32" -o "$DEV" -j MASQUERADE

echo "$CFG_B64" | base64 -d > "/etc/wireguard/$IF.conf"
ip netns exec "$NS" wg-quick up "/etc/wireguard/$IF.conf" >/tmp/"$IF"-up.log 2>&1
sleep 3
echo "===IPINFO==="
ip netns exec "$NS" curl -sS --max-time 15 --resolve "ipinfo.io:443:$IPINFO_IP" https://ipinfo.io/json | head -c 400 || true
echo
echo "===ROUTES==="
ip netns exec "$NS" ip route || true
echo "===DOWNLOAD==="
if [[ -n "${DOWNLOAD_HOST}" && -n "${SPEED_IP}" ]]; then
  ip netns exec "$NS" curl -L -sS -o /dev/null \
    --max-time 30 \
    --resolve "${DOWNLOAD_HOST}:443:${SPEED_IP}" \
    -w 'code=%{http_code} time=%{time_total} speed=%{speed_download} remote=%{remote_ip}\n' \
    "$DOWNLOAD_URL" || true
else
  ip netns exec "$NS" curl -L -sS -o /dev/null \
    --max-time 30 \
    -w 'code=%{http_code} time=%{time_total} speed=%{speed_download} remote=%{remote_ip}\n' \
    "$DOWNLOAD_URL" || true
fi
echo "===WG==="
ip netns exec "$NS" wg show "$IF"
echo "===UPLOG==="
cat /tmp/"$IF"-up.log || true
ip netns exec "$NS" wg-quick down "/etc/wireguard/$IF.conf" >/tmp/"$IF"-down.log 2>&1 || true
ip netns del "$NS" >/dev/null 2>&1 || true
iptables -t nat -D POSTROUTING -s "${NS_VETH_IP%/30}/32" -o "$DEV" -j MASQUERADE >/dev/null 2>&1 || true
ip link del "$V0" >/dev/null 2>&1 || true
ip link del "$V1" >/dev/null 2>&1 || true
rm -f "/etc/wireguard/$IF.conf"
EOF
