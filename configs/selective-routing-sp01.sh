#!/usr/bin/env bash
set -euo pipefail

TABLE="ip escudo_proxy"
CHAIN="prerouting"
SET="streaming_ips"
BACKUP_DIR="${BACKUP_DIR:-/home/dev/pulsovpn/escudo-vpn/audits/selective-routing-backups}"
mkdir -p "$BACKUP_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
BACKUP_FILE="$BACKUP_DIR/nft-escudo_proxy-$STAMP.nft"

if sudo nft list table $TABLE > "$BACKUP_FILE" 2>/dev/null; then
  :
else
  : > "$BACKUP_FILE"
fi

tmp_elements="$(mktemp)"
tmp_nft="$(mktemp)"
trap 'rm -f "$tmp_elements" "$tmp_nft"' EXIT

append_prefixes() {
  local asn="$1"
  local url="https://api.bgpview.io/asn/${asn}/prefixes"
  curl -fsSL --max-time 20 "$url" \
    | jq -r '.data.ipv4_prefixes[].prefix' \
    | sed '/^null$/d' >> "$tmp_elements" || true
}

append_hosts() {
  local host
  for host in "$@"; do
    dig +short "$host" A \
      | grep -E '^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$' \
      | sed 's/$/\/32/' >> "$tmp_elements" || true
  done
}

append_prefixes 2906
append_prefixes 40027

cat >> "$tmp_elements" <<'EOF'
23.246.0.0/18
37.77.184.0/21
38.72.126.0/24
45.57.0.0/17
64.120.128.0/17
66.197.128.0/17
69.53.224.0/19
108.175.32.0/20
185.2.220.0/22
185.9.188.0/22
192.173.64.0/18
198.38.96.0/19
198.45.48.0/20
208.75.76.0/22
EOF

append_hosts \
  globoplay.globo.com globo.com g1.globo.com \
  bbc.co.uk bbci.co.uk bbc.com \
  disneyplus.com bamgrid.com \
  hulu.com peacocktv.com paramountplus.com \
  www.netflix.com api-global.netflix.com www.nflxvideo.net

sort -u "$tmp_elements" | sed '/^$/d' > "${tmp_elements}.sorted"
mv "${tmp_elements}.sorted" "$tmp_elements"

{
  echo "table $TABLE {"
  echo "  set $SET {"
  echo "    type ipv4_addr"
  echo "    flags interval"
  echo "    elements = {"
  sed 's/^/      /; s/$/,/' "$tmp_elements"
  echo "    }"
  echo "  }"
  echo "  chain $CHAIN {"
  echo "    type filter hook prerouting priority mangle; policy accept;"
  echo "  }"
  echo "}"
} > "$tmp_nft"

sudo nft -f "$tmp_nft"
sudo nft flush chain $TABLE $CHAIN
sudo ip rule del fwmark 0x1 table 100 2>/dev/null || true
sudo ip rule add fwmark 0x1 table 100 priority 100
sudo nft add rule $TABLE $CHAIN 'iifname "wg0" ip saddr 10.0.64.0/18 ip daddr @streaming_ips counter meta mark set 0x00000001 comment "escudo-selective-pro-streaming"'
sudo nft add rule $TABLE $CHAIN 'iifname "wg0" ip saddr 10.0.128.0/18 counter meta mark set 0x00000002 comment "escudo-preroute-dedicated-wg0"'

echo "backup=$BACKUP_FILE"
echo "elements=$(wc -l < "$tmp_elements")"
sudo nft list table $TABLE
