#!/bin/bash
set -euo pipefail

# Escudo VPN — Gateway Server Setup Script
# Usage: ssh root@<server-ip> 'bash -s' < deploy/setup-gateway.sh

echo "=== Escudo VPN Gateway Setup ==="

# 1. Update system
echo "[1/7] Updating system..."
apt-get update -qq && apt-get upgrade -y -qq

# 2. Install WireGuard
echo "[2/7] Installing WireGuard..."
apt-get install -y -qq wireguard wireguard-tools

# 3. Generate server keys
echo "[3/7] Generating WireGuard keys..."
mkdir -p /etc/wireguard
cd /etc/wireguard
umask 077
wg genkey | tee server_private.key | wg pubkey > server_public.key
PRIVATE_KEY=$(cat server_private.key)
PUBLIC_KEY=$(cat server_public.key)
SERVER_IP=$(curl -s4 ifconfig.me)

# 4. Create WireGuard config
echo "[4/7] Creating WireGuard interface..."
cat > /etc/wireguard/wg0.conf <<EOF
[Interface]
Address = 10.0.0.1/16
ListenPort = 51820
PrivateKey = ${PRIVATE_KEY}
PostUp = iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE; iptables -A FORWARD -i wg0 -j ACCEPT; iptables -A FORWARD -o wg0 -j ACCEPT
PostDown = iptables -t nat -D POSTROUTING -o eth0 -j MASQUERADE; iptables -D FORWARD -i wg0 -j ACCEPT; iptables -D FORWARD -o wg0 -j ACCEPT
EOF

# Handle different interface names (some VPS use ens3, enp1s0, etc.)
DEFAULT_IFACE=$(ip route show default | awk '/default/ {print $5}' | head -1)
if [ "$DEFAULT_IFACE" != "eth0" ]; then
    echo "Detected network interface: $DEFAULT_IFACE (replacing eth0)"
    sed -i "s/eth0/${DEFAULT_IFACE}/g" /etc/wireguard/wg0.conf
fi

# 5. Enable IP forwarding and kernel tuning
echo "[5/7] Configuring kernel parameters..."
cat > /etc/sysctl.d/99-escudo.conf <<'EOF'
net.ipv4.ip_forward = 1
net.ipv6.conf.all.forwarding = 1
net.core.rmem_max = 26214400
net.core.wmem_max = 26214400
net.core.rmem_default = 1048576
net.core.wmem_default = 1048576
net.core.netdev_max_backlog = 10000
net.ipv4.tcp_fastopen = 3
net.ipv4.tcp_mtu_probing = 1
EOF
sysctl --system -q

# 6. Start WireGuard
echo "[6/7] Starting WireGuard..."
systemctl enable wg-quick@wg0
systemctl start wg-quick@wg0

# 7. Open firewall
echo "[7/7] Configuring firewall..."
if command -v ufw &>/dev/null; then
    ufw allow 51820/udp
    ufw allow 22/tcp
    ufw allow 9090/tcp   # gRPC (internal only — restrict in production)
    ufw --force enable
fi

echo ""
echo "========================================="
echo "  ESCUDO GATEWAY SETUP COMPLETE"
echo "========================================="
echo ""
echo "  Server IP:     ${SERVER_IP}"
echo "  Public Key:    ${PUBLIC_KEY}"
echo "  Listen Port:   51820"
echo ""
echo "  >>> SAVE THESE VALUES — you need them for the database <<<"
echo ""
echo "  Next steps:"
echo "  1. Upload escudo-gateway binary to /usr/local/bin/"
echo "  2. Upload escudo-dns binary to /usr/local/bin/"
echo "  3. Copy config files to /etc/escudo/"
echo "  4. Install systemd services"
echo "========================================="
