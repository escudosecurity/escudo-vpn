#!/usr/bin/env bash
set -euo pipefail

PEER_COUNT="${1:-10}"
OUT_ROOT="${OUT_ROOT:-/home/dev/pulsovpn/escudo-vpn/audits}"
TARGET_SERVER_ID="${TARGET_SERVER_ID:-9457881f-749c-4347-9fee-b1a5e30fc8e5}"
ENTRY_SERVER_ID="${ENTRY_SERVER_ID:-}"
EXIT_SERVER_ID="${EXIT_SERVER_ID:-}"
DOWNLOAD_URL="${DOWNLOAD_URL:-}"
DOWNLOAD_HOST="${DOWNLOAD_HOST:-}"
SPEED_IP="${SPEED_IP:-}"
AUTH_EMAIL="${AUTH_EMAIL:-loadpro@escudovpn.test}"
AUTH_PASSWORD="${AUTH_PASSWORD:-LoadPass123}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_ROOT/${PEER_COUNT}-peer-scale-br-$TS"
mkdir -p "$OUT_DIR"
LOGIN_PAYLOAD="$(jq -nc --arg email "$AUTH_EMAIL" --arg password "$AUTH_PASSWORD" '{email:$email,password:$password}')"
API_TOKEN="$(curl -sS -X POST https://api.escudovpn.com/api/v1/auth/login -H 'Content-Type: application/json' -d "$LOGIN_PAYLOAD" | jq -r '.token')"
echo "api_token_len=${#API_TOKEN}" > "$OUT_DIR/api-token.txt"

declare -a GOOD_HOSTS=(
  "91.99.191.227|default"
  "178.156.140.98|default"
  "204.168.145.177|default"
  "5.78.149.17|default"
  "188.245.32.41|default"
)

if ! [[ "$PEER_COUNT" =~ ^[0-9]+$ ]] || (( PEER_COUNT < 1 )); then
  echo "usage: $0 <peer-count>" >&2
  exit 2
fi

echo "out_dir=$OUT_DIR"
echo "peer_count=$PEER_COUNT"

instances_per_host=$(( (PEER_COUNT + ${#GOOD_HOSTS[@]} - 1) / ${#GOOD_HOSTS[@]} ))
seq_no=0
specs=()
for spec in "${GOOD_HOSTS[@]}"; do
  host="${spec%%|*}"
  mode="${spec##*|}"
  for ((i=1; i<=instances_per_host; i++)); do
    ((seq_no+=1))
    if (( seq_no > PEER_COUNT )); then
      break 2
    fi
    instance_id="$(printf '%02d%02d' "$seq_no" "$i")"
    specs+=("$host|$mode|$instance_id")
  done
done

printf '%s\n' "${specs[@]}" > "$OUT_DIR/hosts.txt"

SP_IP="${SP_IP:-216.238.111.108}"
MON_DIR="$OUT_DIR/sp-monitor"
mkdir -p "$MON_DIR"
monitor_ok=0

if ssh -o BatchMode=yes root@"$SP_IP" "
  mkdir -p '$MON_DIR'
  nohup mpstat 1 > '$MON_DIR/cpu.log' 2>&1 < /dev/null & echo \$! > '$MON_DIR/mpstat.pid'
  nohup free -m -s 1 > '$MON_DIR/ram.log' 2>&1 < /dev/null & echo \$! > '$MON_DIR/free.pid'
  nohup bash -lc 'while true; do date -u +%FT%TZ; ss -s; echo ---; sleep 1; done' > '$MON_DIR/socket.log' 2>&1 < /dev/null & echo \$! > '$MON_DIR/socket.pid'
  nohup bash -lc 'while true; do date -u +%FT%TZ; wg show wg1; echo ===; sleep 1; done' > '$MON_DIR/wg1.log' 2>&1 < /dev/null & echo \$! > '$MON_DIR/wg1.pid'
  nohup bash -lc 'while true; do date -u +%FT%TZ; cat /proc/net/dev; echo ===; sleep 1; done' > '$MON_DIR/netdev.log' 2>&1 < /dev/null & echo \$! > '$MON_DIR/netdev.pid'
  cat /proc/softirqs > '$MON_DIR/softirq-before.log'
" >/dev/null; then
  monitor_ok=1
else
  echo "monitor=unavailable" > "$OUT_DIR/monitor-status.txt"
fi

pids=()
for spec in "${specs[@]}"; do
  host="${spec%%|*}"
  rest="${spec#*|}"
  mode="${rest%%|*}"
  instance_id="${spec##*|}"
  (
    API_TOKEN="$API_TOKEN" SSH_AUTH_MODE="$mode" INSTANCE_ID="$instance_id" TARGET_SERVER_ID="$TARGET_SERVER_ID" \
      ENTRY_SERVER_ID="$ENTRY_SERVER_ID" EXIT_SERVER_ID="$EXIT_SERVER_ID" \
      DOWNLOAD_URL="$DOWNLOAD_URL" DOWNLOAD_HOST="$DOWNLOAD_HOST" SPEED_IP="$SPEED_IP" \
      bash /home/dev/pulsovpn/escudo-vpn/scripts/load/remote_ns_probe.sh "$host"
  ) > "$OUT_DIR/${host//./-}-$instance_id.log" 2>&1 &
  pids+=($!)
done

status=0
for pid in "${pids[@]}"; do
  wait "$pid" || status=1
done

if (( monitor_ok )); then
  ssh -o BatchMode=yes root@"$SP_IP" "
    cat /proc/softirqs > '$MON_DIR/softirq-after.log'
    for f in mpstat free socket wg1 netdev; do
      pidfile='$MON_DIR/'\"\$f\"'.pid'
      test -f \"\$pidfile\" && kill \$(cat \"\$pidfile\") >/dev/null 2>&1 || true
    done
  " >/dev/null || true

  scp -q -o BatchMode=yes -r root@"$SP_IP":"$MON_DIR" "$OUT_DIR/" >/dev/null 2>&1 || true
fi

{
  echo "host,mode,instance_id,code,time_total,speed_download,remote_ip,egress_ip,egress_country,egress_org,status"
  for spec in "${specs[@]}"; do
    host="${spec%%|*}"
    rest="${spec#*|}"
    mode="${rest%%|*}"
    instance_id="${spec##*|}"
    log="$OUT_DIR/${host//./-}-$instance_id.log"
    line="$(grep '^code=' "$log" | tail -n1 || true)"
    ipinfo_json="$(awk '/^===IPINFO===/{flag=1;next}/^===ROUTES===/{flag=0}flag' "$log" | tr -d '\n' || true)"
    code="$(printf '%s' "$line" | sed -n 's/.*code=\([^ ]*\).*/\1/p')"
    time_total="$(printf '%s' "$line" | sed -n 's/.*time=\([^ ]*\).*/\1/p')"
    speed_download="$(printf '%s' "$line" | sed -n 's/.*speed=\([^ ]*\).*/\1/p')"
    remote_ip="$(printf '%s' "$line" | sed -n 's/.*remote=\([^ ]*\).*/\1/p')"
    egress_ip="$(printf '%s' "$ipinfo_json" | jq -r '.ip // empty' 2>/dev/null || true)"
    egress_country="$(printf '%s' "$ipinfo_json" | jq -r '.country // empty' 2>/dev/null || true)"
    egress_org="$(printf '%s' "$ipinfo_json" | jq -r '.org // empty' 2>/dev/null || true)"
    status_cell="FAIL"
    if [[ "$code" == "200" ]]; then
      status_cell="PASS"
    elif [[ -s "$log" ]]; then
      status_cell="PARTIAL"
    fi
    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
      "$host" "$mode" "$instance_id" "$code" "$time_total" "$speed_download" "$remote_ip" "$egress_ip" "$egress_country" "$egress_org" "$status_cell"
  done
} > "$OUT_DIR/summary.csv"

find "$OUT_DIR" -type f -print0 | sort -z | xargs -0 sha256sum > "$OUT_DIR/SHA256SUMS"
cat "$OUT_DIR/summary.csv"
exit "$status"
