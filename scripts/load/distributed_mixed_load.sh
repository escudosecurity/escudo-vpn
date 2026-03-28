#!/usr/bin/env bash
set -euo pipefail

SESSION_COUNT="${1:-40}"
OUT_ROOT="${OUT_ROOT:-/opt/escudo/audits}"
TARGET_SERVER_ID="${TARGET_SERVER_ID:?TARGET_SERVER_ID is required}"
AUTH_EMAIL="${AUTH_EMAIL:-loadfree@escudovpn.test}"
AUTH_PASSWORD="${AUTH_PASSWORD:-LoadPass123}"
HOST_SPECS_FILE="${HOST_SPECS_FILE:?HOST_SPECS_FILE is required}"
SESSION_SECONDS="${SESSION_SECONDS:-180}"
SP_IP="${SP_IP:-}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_ROOT/mixed-${SESSION_COUNT}-session-$TS"
mkdir -p "$OUT_DIR"
REMOTE_TIMEOUT_SECONDS="${REMOTE_TIMEOUT_SECONDS:-$((SESSION_SECONDS + 60))}"

LOGIN_PAYLOAD="$(jq -nc --arg email "$AUTH_EMAIL" --arg password "$AUTH_PASSWORD" '{email:$email,password:$password}')"
API_TOKEN="$(curl -sS -X POST https://api.escudovpn.com/api/v1/auth/login -H 'Content-Type: application/json' -d "$LOGIN_PAYLOAD" | jq -r '.token')"
echo "api_token_len=${#API_TOKEN}" > "$OUT_DIR/api-token.txt"

mapfile -t GOOD_HOSTS < "$HOST_SPECS_FILE"
if ! [[ "$SESSION_COUNT" =~ ^[0-9]+$ ]] || (( SESSION_COUNT < 1 )); then
  echo "usage: $0 <session-count>" >&2
  exit 2
fi
if (( ${#GOOD_HOSTS[@]} == 0 )); then
  echo "no hosts found in $HOST_SPECS_FILE" >&2
  exit 2
fi

echo "out_dir=$OUT_DIR"
echo "session_count=$SESSION_COUNT"

profiles=(browse stream burst idle)
instances_per_host=$(( (SESSION_COUNT + ${#GOOD_HOSTS[@]} - 1) / ${#GOOD_HOSTS[@]} ))
seq_no=0
specs=()
for spec in "${GOOD_HOSTS[@]}"; do
  host="${spec%%|*}"
  mode="${spec##*|}"
  for ((i=1; i<=instances_per_host; i++)); do
    ((seq_no+=1))
    if (( seq_no > SESSION_COUNT )); then
      break 2
    fi
    instance_id="$(printf '%03d%02d' "$seq_no" "$i")"
    profile="${profiles[$(((i - 1) % ${#profiles[@]}))]}"
    specs+=("$host|$mode|$instance_id|$profile")
  done
done

printf '%s\n' "${specs[@]}" > "$OUT_DIR/hosts.txt"

monitor_ok=0
if [[ -n "$SP_IP" ]] && ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@"$SP_IP" "
  mkdir -p '$OUT_DIR/node-monitor'
  nohup mpstat 1 > '$OUT_DIR/node-monitor/cpu.log' 2>&1 < /dev/null & echo \$! > '$OUT_DIR/node-monitor/mpstat.pid'
  nohup free -m -s 1 > '$OUT_DIR/node-monitor/ram.log' 2>&1 < /dev/null & echo \$! > '$OUT_DIR/node-monitor/free.pid'
  nohup bash -lc 'while true; do date -u +%FT%TZ; ss -s; echo ---; sleep 1; done' > '$OUT_DIR/node-monitor/socket.log' 2>&1 < /dev/null & echo \$! > '$OUT_DIR/node-monitor/socket.pid'
  nohup bash -lc 'while true; do date -u +%FT%TZ; cat /proc/net/dev; echo ===; sleep 1; done' > '$OUT_DIR/node-monitor/netdev.log' 2>&1 < /dev/null & echo \$! > '$OUT_DIR/node-monitor/netdev.pid'
" >/dev/null 2>&1; then
  monitor_ok=1
fi

pids=()
run_spec() {
  local spec="$1"
  local host mode instance_id profile log
  host="$(printf '%s' "$spec" | cut -d'|' -f1)"
  mode="$(printf '%s' "$spec" | cut -d'|' -f2)"
  instance_id="$(printf '%s' "$spec" | cut -d'|' -f3)"
  profile="$(printf '%s' "$spec" | cut -d'|' -f4)"
  log="$OUT_DIR/${host//./-}-$instance_id-$profile.log"
  (
    API_TOKEN="$API_TOKEN" SSH_AUTH_MODE="$mode" INSTANCE_ID="$instance_id" TARGET_SERVER_ID="$TARGET_SERVER_ID" \
      SESSION_SECONDS="$SESSION_SECONDS" PROFILE="$profile" \
      REMOTE_TIMEOUT_SECONDS="$REMOTE_TIMEOUT_SECONDS" \
      bash /home/dev/pulsovpn/escudo-vpn/scripts/load/remote_ns_mixed_session.sh "$host"
  ) > "$log" 2>&1
}

for spec in "${specs[@]}"; do
  run_spec "$spec" &
  pids+=($!)
done

status=0
for pid in "${pids[@]}"; do
  wait "$pid" || status=1
done

# Retry sessions that died before producing a profile summary. These are harness
# launch failures, not meaningful traffic results.
retry_specs=()
for spec in "${specs[@]}"; do
  host="$(printf '%s' "$spec" | cut -d'|' -f1)"
  instance_id="$(printf '%s' "$spec" | cut -d'|' -f3)"
  profile="$(printf '%s' "$spec" | cut -d'|' -f4)"
  log="$OUT_DIR/${host//./-}-$instance_id-$profile.log"
  if [[ ! -s "$log" ]] || ! grep -q '^profile=.*requests=.*errors=.*bytes=' "$log"; then
    retry_specs+=("$spec")
  fi
done

if (( ${#retry_specs[@]} > 0 )); then
  printf '%s\n' "${retry_specs[@]}" > "$OUT_DIR/retry-specs.txt"
  retry_pids=()
  for spec in "${retry_specs[@]}"; do
    run_spec "$spec" &
    retry_pids+=($!)
  done
  for pid in "${retry_pids[@]}"; do
    wait "$pid" || status=1
  done
fi

if (( monitor_ok )); then
  ssh -o BatchMode=yes -o StrictHostKeyChecking=no root@"$SP_IP" "
    for f in mpstat free socket netdev; do
      pidfile='$OUT_DIR/node-monitor/'\"\$f\"'.pid'
      test -f \"\$pidfile\" && kill \$(cat \"\$pidfile\") >/dev/null 2>&1 || true
    done
  " >/dev/null 2>&1 || true
  scp -q -o BatchMode=yes -o StrictHostKeyChecking=no -r root@"$SP_IP":"$OUT_DIR/node-monitor" "$OUT_DIR/" >/dev/null 2>&1 || true
fi

{
  echo "host,mode,instance_id,profile,requests,errors,bytes,status"
  for spec in "${specs[@]}"; do
    host="$(printf '%s' "$spec" | cut -d'|' -f1)"
    mode="$(printf '%s' "$spec" | cut -d'|' -f2)"
    instance_id="$(printf '%s' "$spec" | cut -d'|' -f3)"
    profile="$(printf '%s' "$spec" | cut -d'|' -f4)"
    log="$OUT_DIR/${host//./-}-$instance_id-$profile.log"
    line="$(grep '^profile=' "$log" | tail -n1 || true)"
    requests="$(printf '%s' "$line" | sed -n 's/.*requests=\([0-9]*\).*/\1/p')"
    errors="$(printf '%s' "$line" | sed -n 's/.*errors=\([0-9]*\).*/\1/p')"
    bytes="$(printf '%s' "$line" | sed -n 's/.*bytes=\([0-9]*\).*/\1/p')"
    status_cell="FAIL"
    if [[ -n "$requests" && "$requests" =~ ^[0-9]+$ && "$requests" -gt 0 ]]; then
      status_cell="PASS"
    elif [[ -s "$log" ]]; then
      status_cell="PARTIAL"
    fi
    printf '%s,%s,%s,%s,%s,%s,%s,%s\n' \
      "$host" "$mode" "$instance_id" "$profile" "${requests:-}" "${errors:-}" "${bytes:-}" "$status_cell"
  done
} > "$OUT_DIR/summary.csv"

find "$OUT_DIR" -type f -print0 | sort -z | xargs -0 sha256sum > "$OUT_DIR/SHA256SUMS"
cat "$OUT_DIR/summary.csv"
if awk -F, 'NR > 1 && $8 != "PASS" {exit 1}' "$OUT_DIR/summary.csv"; then
  exit 0
fi

exit 1
