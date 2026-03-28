#!/usr/bin/env bash
set -euo pipefail

QUERY_FILE="${QUERY_FILE:-/home/dev/pulsovpn/escudo-vpn/scripts/load/dnsperf-queries.txt}"
OUT_ROOT="${OUT_ROOT:-/home/dev/pulsovpn/escudo-vpn/audits}"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="$OUT_ROOT/dns-stress-$TS"
mkdir -p "$OUT_DIR"

round() {
  local qps="$1"
  local conc="$2"
  local dur="${3:-30}"
  local log="$OUT_DIR/dnsperf-${qps}.log"
  dnsperf -s 10.0.0.1 -d "$QUERY_FILE" -c "$conc" -Q "$qps" -l "$dur" > "$log" 2>&1
}

nohup mpstat 1 > "$OUT_DIR/cpu.log" 2>&1 < /dev/null & echo $! > "$OUT_DIR/mpstat.pid"
nohup free -m -s 1 > "$OUT_DIR/ram.log" 2>&1 < /dev/null & echo $! > "$OUT_DIR/free.pid"

round 500 20 30
round 1000 40 30
round 2000 60 30
round 5000 80 30 || true

kill "$(cat "$OUT_DIR/mpstat.pid")" >/dev/null 2>&1 || true
kill "$(cat "$OUT_DIR/free.pid")" >/dev/null 2>&1 || true

{
  echo "qps_target,sent,completed,lost,avg_latency_ms,max_latency_ms,status"
  for qps in 500 1000 2000 5000; do
    log="$OUT_DIR/dnsperf-${qps}.log"
    [[ -f "$log" ]] || continue
    sent="$(awk '/Queries sent:/ {print $3}' "$log" | tail -n1)"
    completed="$(awk '/Queries completed:/ {print $3}' "$log" | tail -n1)"
    lost="$(awk '/Queries lost:/ {print $3}' "$log" | tail -n1)"
    avg="$(awk '/Average Latency \(s\):/ {printf "%.3f", $4*1000}' "$log" | tail -n1)"
    max="$(awk '/Maximum Latency \(s\):/ {printf "%.3f", $4*1000}' "$log" | tail -n1)"
    status="FAIL"
    if [[ "${lost:-}" == "0" ]] && awk "BEGIN {exit !(${avg:-999999} < 10)}"; then
      status="PASS"
    fi
    printf '%s,%s,%s,%s,%s,%s,%s\n' "$qps" "${sent:-}" "${completed:-}" "${lost:-}" "${avg:-}" "${max:-}" "$status"
  done
} > "$OUT_DIR/summary.csv"

find "$OUT_DIR" -type f -print0 | sort -z | xargs -0 sha256sum > "$OUT_DIR/SHA256SUMS"
echo "out_dir=$OUT_DIR"
cat "$OUT_DIR/summary.csv"
