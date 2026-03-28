#!/usr/bin/env bash
set -euo pipefail

OUT_ROOT="${OUT_ROOT:-/home/dev/pulsovpn/escudo-vpn/audits}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_ROOT/10-peer-batch-$TS"
mkdir -p "$OUT_DIR"

declare -a HOST_SPECS=(
  "91.99.191.227|default"
  "178.156.140.98|default"
  "204.168.145.177|default"
  "5.78.149.17|default"
  "188.245.32.41|default"
  "38.54.29.167|lightnode"
  "38.60.233.202|lightnode"
  "130.94.105.197|lightnode"
  "103.13.208.14|kamatera"
  "103.45.245.67|kamatera"
)

echo "out_dir=$OUT_DIR"
printf '%s\n' "${HOST_SPECS[@]}" > "$OUT_DIR/hosts.txt"

pids=()
for spec in "${HOST_SPECS[@]}"; do
  host="${spec%%|*}"
  mode="${spec##*|}"
  (
    SSH_AUTH_MODE="$mode" bash /home/dev/pulsovpn/escudo-vpn/scripts/load/remote_ns_probe.sh "$host"
  ) > "$OUT_DIR/$host.log" 2>&1 &
  pids+=($!)
done

status=0
for pid in "${pids[@]}"; do
  wait "$pid" || status=1
done

{
  echo "host,mode,code,time_total,speed_download,remote_ip,egress_ip,egress_country,egress_org,status"
  for spec in "${HOST_SPECS[@]}"; do
    host="${spec%%|*}"
    mode="${spec##*|}"
    line="$(grep '^code=' "$OUT_DIR/$host.log" | tail -n1 || true)"
    ipinfo_json="$(awk '/^===IPINFO===/{flag=1;next}/^===ROUTES===/{flag=0}flag' "$OUT_DIR/$host.log" | tr -d '\n' || true)"
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
    elif [[ -s "$OUT_DIR/$host.log" ]]; then
      status_cell="PARTIAL"
    fi
    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
      "$host" "$mode" "$code" "$time_total" "$speed_download" "$remote_ip" "$egress_ip" "$egress_country" "$egress_org" "$status_cell"
  done
} > "$OUT_DIR/summary.csv"

find "$OUT_DIR" -maxdepth 1 -type f -print0 | sort -z | xargs -0 sha256sum > "$OUT_DIR/SHA256SUMS"
cat "$OUT_DIR/summary.csv"
exit "$status"
