#!/usr/bin/env bash
set -euo pipefail

SERVER_NAME="${SERVER_NAME:-escudo-node}"
WG_PORT="${WG_PORT:-51820}"
WG_ADDRESS="${WG_ADDRESS:-10.10.1.1/24}"
WG_NETWORK_CIDR="${WG_NETWORK_CIDR:-10.10.1.0/24}"
MGMT_PUBLIC_KEY="${MGMT_PUBLIC_KEY:-}"
MGMT_ENDPOINT="${MGMT_ENDPOINT:-91.99.29.182}"
DNS_BINARY_SOURCE="${DNS_BINARY_SOURCE:-/tmp/escudo-dns}"
NFT_SOURCE="${NFT_SOURCE:-/tmp/selective-routing.nft}"

export DEBIAN_FRONTEND=noninteractive

apt-get update -y
apt-get upgrade -y
apt-get install -y \
  wireguard-tools \
  ufw \
  jq \
  curl \
  wget \
  unzip \
  nftables \
  ca-certificates \
  iproute2

swapoff -a || true
sed -i.bak '/\sswap\s/d' /etc/fstab || true

cat >/etc/sysctl.d/99-escudo-harden.conf <<'EOF'
net.ipv6.conf.all.disable_ipv6 = 1
net.ipv6.conf.default.disable_ipv6 = 1
net.ipv6.conf.lo.disable_ipv6 = 1
net.core.rmem_max = 8388608
net.core.wmem_max = 8388608
net.core.rmem_default = 1048576
net.core.wmem_default = 1048576
net.core.netdev_max_backlog = 10000
net.ipv4.ip_forward = 1
EOF
sysctl --system >/dev/null

cat >/etc/resolv.conf <<'EOF'
nameserver 1.1.1.1
nameserver 1.0.0.1
options timeout:2 attempts:2
EOF

mkdir -p /etc/wireguard
chmod 700 /etc/wireguard

for IFACE in wg0 wg1 wg2; do
  if [ ! -f "/etc/wireguard/${IFACE}.privkey" ]; then
    umask 077
    wg genkey | tee "/etc/wireguard/${IFACE}.privkey" | wg pubkey >"/etc/wireguard/${IFACE}.pubkey"
  fi
done

WG0_PRIVATE_KEY="$(cat /etc/wireguard/wg0.privkey)"
cat >/etc/wireguard/wg0.conf <<EOF
[Interface]
Address = 10.0.0.1/18
ListenPort = 51820
PrivateKey = ${WG0_PRIVATE_KEY}
SaveConfig = false

PostUp   = nft add rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
PostDown = nft delete rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
EOF

WG1_PRIVATE_KEY="$(cat /etc/wireguard/wg1.privkey)"
cat >/etc/wireguard/wg1.conf <<EOF
[Interface]
Address = 10.0.64.1/18
ListenPort = 51821
PrivateKey = ${WG1_PRIVATE_KEY}
SaveConfig = false

PostUp   = nft add rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
PostDown = nft delete rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
EOF

WG2_PRIVATE_KEY="$(cat /etc/wireguard/wg2.privkey)"
cat >/etc/wireguard/wg2.conf <<EOF
[Interface]
Address = 10.0.128.1/18
ListenPort = 51822
PrivateKey = ${WG2_PRIVATE_KEY}
SaveConfig = false

PostUp   = nft add rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
PostDown = nft delete rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
EOF

if [ -n "${MGMT_PUBLIC_KEY}" ]; then
  mkdir -p /root/.ssh
  chmod 700 /root/.ssh
  grep -qxF "${MGMT_PUBLIC_KEY}" /root/.ssh/authorized_keys 2>/dev/null || echo "${MGMT_PUBLIC_KEY}" >>/root/.ssh/authorized_keys
  chmod 600 /root/.ssh/authorized_keys
fi

if [ -x "${DNS_BINARY_SOURCE}" ]; then
  apt-get install -y dnsmasq
  install -m 0755 "${DNS_BINARY_SOURCE}" /usr/local/bin/escudo-dns
fi

if [ -f /tmp/escudo-gateway ]; then
  install -m 0755 /tmp/escudo-gateway /usr/local/bin/escudo-gateway
fi

ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
  BIN_ARCH="amd64"
elif [ "$ARCH" = "aarch64" ]; then
  BIN_ARCH="arm64"
else
  BIN_ARCH="amd64"
fi

if [ ! -x /usr/local/bin/tun2socks ]; then
  TUN2SOCKS_VERSION="v2.5.2"
  TUN2SOCKS_ZIP="$(mktemp)"
  curl -fsSL -o "${TUN2SOCKS_ZIP}" \
    "https://github.com/xjasonlyu/tun2socks/releases/download/${TUN2SOCKS_VERSION}/tun2socks-linux-${BIN_ARCH}.zip" \
    || curl -fsSL -o "${TUN2SOCKS_ZIP}" \
      "https://github.com/xjasonlyu/tun2socks/releases/download/${TUN2SOCKS_VERSION}/tun2socks-linux-${BIN_ARCH}-v3.zip" \
    || true
  if [ -s "${TUN2SOCKS_ZIP}" ]; then
    unzip -p "${TUN2SOCKS_ZIP}" tun2socks >/usr/local/bin/tun2socks 2>/dev/null || true
    chmod +x /usr/local/bin/tun2socks 2>/dev/null || true
  fi
  rm -f "${TUN2SOCKS_ZIP}"
fi

mkdir -p /etc/escudo
cat >/etc/escudo/tunnel-node.env <<EOF
SERVER_NAME=${SERVER_NAME}
WG_ADDRESS=${WG_ADDRESS}
WG_NETWORK_CIDR=${WG_NETWORK_CIDR}
WG_PORT=${WG_PORT}
MGMT_ENDPOINT=${MGMT_ENDPOINT}
EOF

touch /etc/escudo/proxy-shared.env /etc/escudo/proxy-dedicated.env

cat >/etc/escudo/gateway.toml <<EOF
[server]
grpc_addr = "0.0.0.0:9090"
health_addr = "127.0.0.1:8080"

[wireguard]
interface = "wg0"
subnet = "10.0.0.0/16"
ip_start = "10.0.0.2"
ip_end = "10.0.255.254"
wg1_interface = "wg1"
wg2_interface = "wg2"

[stats]
collection_interval_secs = 30

[proxy]
enabled = true
env_dir = "/etc/escudo"
shared_service = "escudo-tun2socks-shared.service"
dedicated_service = "escudo-tun2socks-dedicated.service"
EOF

cat >/etc/systemd/system/escudo-gateway.service <<'EOF'
[Unit]
Description=Escudo VPN Gateway
After=network-online.target wg-quick@wg0.service wg-quick@wg1.service wg-quick@wg2.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/escudo-gateway --config /etc/escudo/gateway.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

cat >/etc/systemd/system/escudo-tun2socks-shared.service <<'EOF'
[Unit]
Description=Escudo tun2socks shared proxy
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=/etc/escudo/proxy-shared.env
ExecStartPre=/bin/sh -c 'test -n "$SOCKS5_HOST" && test -n "$SOCKS5_PORT" && test -n "$SOCKS5_USERNAME" && test -n "$SOCKS5_PASSWORD"'
ExecStart=/bin/sh -lc '/usr/local/bin/tun2socks -device tun-shared -proxy socks5://${SOCKS5_USERNAME}:${SOCKS5_PASSWORD}@${SOCKS5_HOST}:${SOCKS5_PORT}'
ExecStartPost=/bin/sh -lc 'for i in $(seq 1 20); do ip link show tun-shared >/dev/null 2>&1 && ip addr replace 198.18.0.1/15 dev tun-shared && ip link set tun-shared up && ip route replace default dev tun-shared table 100 && exit 0; sleep 1; done; exit 1'
ExecStopPost=/bin/sh -lc 'ip route replace blackhole default table 100'
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

cat >/etc/systemd/system/escudo-tun2socks-dedicated.service <<'EOF'
[Unit]
Description=Escudo tun2socks dedicated proxy
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=/etc/escudo/proxy-dedicated.env
ExecStartPre=/bin/sh -c 'test -n "$SOCKS5_HOST" && test -n "$SOCKS5_PORT" && test -n "$SOCKS5_USERNAME" && test -n "$SOCKS5_PASSWORD"'
ExecStart=/bin/sh -lc '/usr/local/bin/tun2socks -device tun-dedicated -proxy socks5://${SOCKS5_USERNAME}:${SOCKS5_PASSWORD}@${SOCKS5_HOST}:${SOCKS5_PORT}'
ExecStartPost=/bin/sh -lc 'for i in $(seq 1 20); do ip link show tun-dedicated >/dev/null 2>&1 && ip addr replace 198.19.0.1/15 dev tun-dedicated && ip link set tun-dedicated up && ip route replace default dev tun-dedicated table 101 && exit 0; sleep 1; done; exit 1'
ExecStopPost=/bin/sh -lc 'ip route replace blackhole default table 101'
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

if [ -x "${DNS_BINARY_SOURCE}" ]; then
  cat >/etc/dnsmasq.d/escudo-streaming.conf <<'EOF'
nftset=/netflix.com/4#ip#escudo#streaming_ips
nftset=/nflxso.net/4#ip#escudo#streaming_ips
nftset=/nflximg.net/4#ip#escudo#streaming_ips
nftset=/bbc.co.uk/4#ip#escudo#streaming_ips
nftset=/bbc.com/4#ip#escudo#streaming_ips
nftset=/bbci.co.uk/4#ip#escudo#streaming_ips
nftset=/globo.com/4#ip#escudo#streaming_ips
nftset=/globoplay.globo.com/4#ip#escudo#streaming_ips
nftset=/g.globo/4#ip#escudo#streaming_ips
nftset=/peacocktv.com/4#ip#escudo#streaming_ips
EOF
fi

cat >/etc/nftables.conf <<EOF
#!/usr/sbin/nft -f

flush ruleset

table ip escudo {
    set streaming_ips {
        type ipv4_addr
        flags dynamic, timeout
        timeout 5m
    }

    chain forward {
        type filter hook forward priority mangle; policy accept;
        iifname "wg1" ip daddr @streaming_ips ct mark set 0x00000001 meta mark set 0x00000001
        iifname "wg2" ct mark set 0x00000002 meta mark set 0x00000002
    }

    chain output {
        type route hook output priority mangle; policy accept;
        ct mark 0x00000001 meta mark set 0x00000001
        ct mark 0x00000002 meta mark set 0x00000002
    }
}

table ip escudo-nat {
    chain POSTROUTING {
        type nat hook postrouting priority srcnat; policy accept;
    }
}
EOF

if ! ip rule show | grep -q "fwmark 0x1"; then
  ip rule add fwmark 0x1 table 100
fi
if ! ip rule show | grep -q "fwmark 0x2"; then
  ip rule add fwmark 0x2 table 101
fi
ip route replace blackhole default table 100
ip route replace blackhole default table 101

if [ -f "${NFT_SOURCE}" ]; then
  install -m 0644 "${NFT_SOURCE}" /etc/nftables.d/escudo-selective-routing.nft
  mkdir -p /etc/nftables.d
  touch /etc/nftables.conf
  grep -q 'escudo-selective-routing.nft' /etc/nftables.conf 2>/dev/null || \
    printf '\ninclude "/etc/nftables.d/escudo-selective-routing.nft"\n' >>/etc/nftables.conf
fi

systemctl daemon-reload
systemctl enable nftables || true
systemctl restart nftables || true
if [ -x "${DNS_BINARY_SOURCE}" ]; then
  systemctl enable dnsmasq || true
  systemctl restart dnsmasq || true
fi
for IFACE in wg0 wg1 wg2; do
  systemctl enable "wg-quick@${IFACE}"
  systemctl restart "wg-quick@${IFACE}"
done
systemctl enable escudo-gateway || true
systemctl restart escudo-gateway || true

ufw --force reset
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp
ufw allow 51820/udp
ufw allow 51821/udp
ufw allow 51822/udp
ufw allow from "${MGMT_ENDPOINT}" to any port 9090 proto tcp
ufw --force enable

echo "SERVER_NAME=${SERVER_NAME}"
echo "WG0_PUBLIC_KEY=$(cat /etc/wireguard/wg0.pubkey)"
echo "WG1_PUBLIC_KEY=$(cat /etc/wireguard/wg1.pubkey)"
echo "WG2_PUBLIC_KEY=$(cat /etc/wireguard/wg2.pubkey)"
echo "WG_NETWORK_CIDR=${WG_NETWORK_CIDR}"
