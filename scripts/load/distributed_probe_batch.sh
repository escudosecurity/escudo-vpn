#!/usr/bin/env bash
set -euo pipefail

OUT_ROOT="${OUT_ROOT:-/home/dev/pulsovpn/escudo-vpn/audits}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_ROOT/distributed-probe-batch-$TS"
mkdir -p "$OUT_DIR"

HOSTS="${HOSTS:-91.99.191.227 178.156.140.98 204.168.145.177 5.78.149.17 188.245.32.41}"

echo "out_dir=$OUT_DIR"
printf '%s\n' "$HOSTS" > "$OUT_DIR/hosts.txt"

pids=()
for host in $HOSTS; do
  (
    bash /home/dev/pulsovpn/escudo-vpn/scripts/load/remote_ns_probe.sh "$host"
  ) > "$OUT_DIR/$host.log" 2>&1 &
  pids+=($!)
done

status=0
for pid in "${pids[@]}"; do
  wait "$pid" || status=1
done

{
  echo "host,code,time_total,speed_download,remote_ip,egress_ip,egress_country,egress_org"
  for host in $HOSTS; do
    line="$(grep '^code=' "$OUT_DIR/$host.log" | tail -n1 || true)"
    ipinfo_json="$(awk '/^===IPINFO===/{flag=1;next}/^===ROUTES===/{flag=0}flag' "$OUT_DIR/$host.log" | tr -d '\n' || true)"
    code="$(printf '%s' "$line" | sed -n 's/.*code=\([^ ]*\).*/\1/p')"
    time_total="$(printf '%s' "$line" | sed -n 's/.*time=\([^ ]*\).*/\1/p')"
    speed_download="$(printf '%s' "$line" | sed -n 's/.*speed=\([^ ]*\).*/\1/p')"
    remote_ip="$(printf '%s' "$line" | sed -n 's/.*remote=\([^ ]*\).*/\1/p')"
    egress_ip="$(printf '%s' "$ipinfo_json" | jq -r '.ip // empty' 2>/dev/null || true)"
    egress_country="$(printf '%s' "$ipinfo_json" | jq -r '.country // empty' 2>/dev/null || true)"
    egress_org="$(printf '%s' "$ipinfo_json" | jq -r '.org // empty' 2>/dev/null || true)"
    printf '%s,%s,%s,%s,%s,%s,%s,%s\n' \
      "$host" "$code" "$time_total" "$speed_download" "$remote_ip" "$egress_ip" "$egress_country" "$egress_org"
  done
} > "$OUT_DIR/summary.csv"

find "$OUT_DIR" -maxdepth 1 -type f -print0 | sort -z | xargs -0 sha256sum > "$OUT_DIR/SHA256SUMS"
cat "$OUT_DIR/summary.csv"
exit "$status"
