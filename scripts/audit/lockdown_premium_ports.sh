#!/usr/bin/env bash
set -euo pipefail

MGMT_IP="216.238.111.108"
NODES=(
  "91.99.191.227"
  "178.156.140.98"
  "204.168.145.177"
  "5.78.149.17"
  "188.245.32.41"
)

for h in "${NODES[@]}"; do
  echo "=== $h ==="
  ssh -o BatchMode=yes "root@$h" 'bash -s' <<EOF
python3 - <<'PY'
from pathlib import Path
p = Path('/etc/nftables.conf')
s = p.read_text()
allow = '        tcp dport { 8080, 9090 } ip saddr '"$MGMT_IP"' accept\\n'
drop = '        tcp dport { 8080, 9090 } drop\\n'
if allow not in s or drop not in s:
    needle = 'chain input {\\n'
    block = needle + '\\t\\ttype filter hook input priority filter; policy accept;\\n\\t\\tiif \"lo\" accept\\n\\t\\tct state established,related accept\\n' + allow.replace('        ', '\\t\\t') + drop.replace('        ', '\\t\\t')
    s = s.replace('chain input {\\n\\t\\ttype filter hook input priority filter;\\n\\t}', block + '\\t}', 1)
    p.write_text(s)
PY
nft -f /etc/nftables.conf
nft list table inet filter | sed -n '1,120p'
EOF
done
