#!/usr/bin/env bash
set -euo pipefail

source /opt/escudo/scripts/audit/proof_client_harness.sh

OUT="${1:-/opt/escudo/audits/single-br-$(date -u +%Y%m%dT%H%M%SZ)}"
BR_SERVER_ID="${BR_SERVER_ID:-043e8ffc-a5f5-4351-9bf9-2258214fab49}"
BR_SERVER_NAME="${BR_SERVER_NAME:-escudo-proof-vlt-01}"
BR_PROXY_ID="${BR_PROXY_ID:-}"

mkdir -p "$OUT"

if [[ -z "$BR_PROXY_ID" ]]; then
  BR_PROXY_ID="$(psql "$DB_URL" -At -c \
    "SELECT id FROM proxy_ips
     WHERE country = 'BR'
       AND status = 'healthy'
       AND proxy_type = 'shared'
       AND socks5_port = 12324
     ORDER BY created_at ASC
     LIMIT 1")"
fi

psql "$DB_URL" -c \
  "INSERT INTO server_proxy_assignments (server_id, proxy_ip_id, proxy_target)
   VALUES ('$BR_SERVER_ID', '$BR_PROXY_ID', 'shared')
   ON CONFLICT (server_id, proxy_target)
   DO UPDATE SET proxy_ip_id = EXCLUDED.proxy_ip_id, assigned_at = now()" >/dev/null

account_json="$(create_account pro)"
echo "$account_json" | jq . >"$OUT/account.json"
token="$(echo "$account_json" | jq -r '.token')"

echo "Connecting to server $BR_SERVER_ID"
connect_json="$(connect_account "$token" "$BR_SERVER_ID" "proof-br-1")"
echo "$connect_json" | jq . >"$OUT/connect.json"

config_path="$OUT/wg-proof-br-1.conf"
write_wg_config "$(echo "$connect_json" | jq -r '.config')" "$config_path"

start_namespace_client "proofbr1" "pbrpeer1" "pbrveth1" "172.31.201" "$config_path"
maybe_cleanup_trap

probe_namespace \
  "proofbr1" \
  "$OUT/ipinfo.json" \
  "$OUT/netflix.txt" \
  "$OUT/globoplay.txt" \
  "https://globoplay.globo.com"

{
  echo "server_name=$BR_SERVER_NAME"
  echo "server_id=$BR_SERVER_ID"
  echo "proxy_ip_id=$BR_PROXY_ID"
  echo "exit_ip=$(jq -r '.ip // empty' "$OUT/ipinfo.json")"
  echo "exit_org=$(jq -r '.org // empty' "$OUT/ipinfo.json")"
  echo "exit_country=$(jq -r '.country // empty' "$OUT/ipinfo.json")"
  echo "netflix=$(cat "$OUT/netflix.txt")"
  echo "globoplay=$(cat "$OUT/globoplay.txt")"
} >"$OUT/summary.env"

cat "$OUT/summary.env"
