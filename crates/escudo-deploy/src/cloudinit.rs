/// Generate a cloud-init bash script for a new VPN server.
///
/// The script:
/// 1. Installs wireguard, dnsmasq, nftables
/// 2. Enables IP forwarding
/// 3. Generates WireGuard keys for 3 interfaces (wg0, wg1, wg2)
/// 4. Creates /etc/wireguard/wg{0,1,2}.conf
/// 5. Writes dnsmasq streaming nftset config
/// 6. Sets up nftables rules for streaming traffic marking
/// 7. Sets up policy routing (fwmark 0x1/0x2 to proxy route tables)
/// 8. Downloads gateway + tun2socks binaries
/// 9. Installs persistent systemd units for gateway + tun2socks
/// 10. Phones home to central API
pub fn generate_cloudinit(server_label: &str, deploy_secret: &str, home_url: &str) -> String {
    format!(
        r#"#!/bin/bash
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

SERVER_LABEL="{server_label}"
DEPLOY_SECRET="{deploy_secret}"
HOME_URL="{home_url}"

echo "[escudo] Starting cloud-init for $SERVER_LABEL"

# -------------------------------------------------------------------
# 1. Install packages
# -------------------------------------------------------------------
apt-get update -qq
apt-get install -y -qq wireguard dnsmasq nftables curl jq iproute2

# -------------------------------------------------------------------
# 2. Enable IP forwarding
# -------------------------------------------------------------------
cat > /etc/sysctl.d/99-escudo.conf <<'EOF'
net.ipv4.ip_forward = 1
net.ipv6.conf.all.forwarding = 1
EOF
sysctl --system

# -------------------------------------------------------------------
# 3. Generate WireGuard keys
# -------------------------------------------------------------------
for IFACE in wg0 wg1 wg2; do
    umask 077
    wg genkey | tee /etc/wireguard/${{IFACE}}.privkey | wg pubkey > /etc/wireguard/${{IFACE}}.pubkey
done

# -------------------------------------------------------------------
# 4. Create WireGuard configs
# -------------------------------------------------------------------
WG0_PRIVKEY=$(cat /etc/wireguard/wg0.privkey)
cat > /etc/wireguard/wg0.conf <<EOF
[Interface]
Address = 10.0.0.1/18
ListenPort = 51820
PrivateKey = $WG0_PRIVKEY

PostUp   = nft add rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
PostDown = nft delete rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
EOF

WG1_PRIVKEY=$(cat /etc/wireguard/wg1.privkey)
cat > /etc/wireguard/wg1.conf <<EOF
[Interface]
Address = 10.0.64.1/18
ListenPort = 51821
PrivateKey = $WG1_PRIVKEY

PostUp   = nft add rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
PostDown = nft delete rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
EOF

WG2_PRIVKEY=$(cat /etc/wireguard/wg2.privkey)
cat > /etc/wireguard/wg2.conf <<EOF
[Interface]
Address = 10.0.128.1/18
ListenPort = 51822
PrivateKey = $WG2_PRIVKEY

PostUp   = nft add rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
PostDown = nft delete rule ip escudo-nat POSTROUTING oifname != "wg*" masquerade
EOF

# Enable and start WireGuard interfaces
for IFACE in wg0 wg1 wg2; do
    systemctl enable wg-quick@${{IFACE}}
    systemctl start  wg-quick@${{IFACE}} || true
done

# -------------------------------------------------------------------
# 5. Persistent config files
# -------------------------------------------------------------------
mkdir -p /etc/escudo
touch /etc/escudo/proxy-shared.env /etc/escudo/proxy-dedicated.env

# Disable IPv6 when it is not explicitly tunneled to avoid leak paths.
cat > /etc/sysctl.d/99-escudo-disable-ipv6.conf <<'EOF'
net.ipv6.conf.all.disable_ipv6 = 1
net.ipv6.conf.default.disable_ipv6 = 1
EOF
sysctl --system || true

cat > /etc/escudo/gateway.toml <<EOF
[server]
grpc_addr = "127.0.0.1:9090"
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

[proxy.poll]
central_api_url = "$HOME_URL"
server_label = "$SERVER_LABEL"
deploy_secret = "$DEPLOY_SECRET"
poll_interval_secs = 60
EOF

cat > /etc/systemd/system/escudo-gateway.service <<'EOF'
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

cat > /etc/systemd/system/escudo-tun2socks-shared.service <<'EOF'
[Unit]
Description=Escudo tun2socks shared proxy
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=/etc/escudo/proxy-shared.env
ExecStartPre=/bin/sh -c 'test -n "$SOCKS5_HOST" && test -n "$SOCKS5_PORT" && test -n "$SOCKS5_USERNAME" && test -n "$SOCKS5_PASSWORD"'
ExecStart=/bin/sh -lc '/usr/local/bin/tun2socks -device tun-shared -proxy socks5://${{SOCKS5_USERNAME}}:${{SOCKS5_PASSWORD}}@${{SOCKS5_HOST}}:${{SOCKS5_PORT}}'
ExecStartPost=/bin/sh -lc 'for i in $(seq 1 20); do ip link show tun-shared >/dev/null 2>&1 && ip addr replace 198.18.0.1/15 dev tun-shared && ip link set tun-shared up && ip route replace default dev tun-shared table 100 && exit 0; sleep 1; done; exit 1'
ExecStopPost=/bin/sh -lc 'ip route replace blackhole default table 100'
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

cat > /etc/systemd/system/escudo-tun2socks-dedicated.service <<'EOF'
[Unit]
Description=Escudo tun2socks dedicated proxy
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=/etc/escudo/proxy-dedicated.env
ExecStartPre=/bin/sh -c 'test -n "$SOCKS5_HOST" && test -n "$SOCKS5_PORT" && test -n "$SOCKS5_USERNAME" && test -n "$SOCKS5_PASSWORD"'
ExecStart=/bin/sh -lc '/usr/local/bin/tun2socks -device tun-dedicated -proxy socks5://${{SOCKS5_USERNAME}}:${{SOCKS5_PASSWORD}}@${{SOCKS5_HOST}}:${{SOCKS5_PORT}}'
ExecStartPost=/bin/sh -lc 'for i in $(seq 1 20); do ip link show tun-dedicated >/dev/null 2>&1 && ip addr replace 198.19.0.1/15 dev tun-dedicated && ip link set tun-dedicated up && ip route replace default dev tun-dedicated table 101 && exit 0; sleep 1; done; exit 1'
ExecStopPost=/bin/sh -lc 'ip route replace blackhole default table 101'
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# -------------------------------------------------------------------
# 6. dnsmasq nftset config for streaming services
# -------------------------------------------------------------------
cat > /etc/dnsmasq.d/escudo-streaming.conf <<'EOF'
# Netflix
nftset=/netflix.com/4#ip#escudo#streaming_ips
nftset=/nflxso.net/4#ip#escudo#streaming_ips
nftset=/nflximg.net/4#ip#escudo#streaming_ips

# BBC iPlayer
nftset=/bbc.co.uk/4#ip#escudo#streaming_ips
nftset=/bbc.com/4#ip#escudo#streaming_ips
nftset=/bbci.co.uk/4#ip#escudo#streaming_ips

# Disney+
nftset=/disneyplus.com/4#ip#escudo#streaming_ips
nftset=/disney-plus.net/4#ip#escudo#streaming_ips
nftset=/dssott.com/4#ip#escudo#streaming_ips
nftset=/bamgrid.com/4#ip#escudo#streaming_ips

# Globoplay
nftset=/globo.com/4#ip#escudo#streaming_ips
nftset=/globoplay.globo.com/4#ip#escudo#streaming_ips
nftset=/g.globo/4#ip#escudo#streaming_ips
EOF

systemctl enable dnsmasq
systemctl restart dnsmasq || true

# -------------------------------------------------------------------
# 7. nftables rules for streaming traffic marking
# -------------------------------------------------------------------
cat > /etc/nftables.conf <<'EOF'
#!/usr/sbin/nft -f

flush ruleset

table ip escudo {{
    set streaming_ips {{
        type ipv4_addr
        flags dynamic, timeout
        timeout 5m
    }}

    chain forward {{
        type filter hook forward priority mangle; policy accept;
        iifname "wg1" ip daddr @streaming_ips ct mark set 0x00000001 meta mark set 0x00000001
        iifname "wg2" ct mark set 0x00000002 meta mark set 0x00000002
    }}

    chain output {{
        type route hook output priority mangle; policy accept;
        ct mark 0x00000001 meta mark set 0x00000001
        ct mark 0x00000002 meta mark set 0x00000002
    }}
}}

table ip escudo-nat {{
    chain POSTROUTING {{
        type nat hook postrouting priority srcnat; policy accept;
    }}
}}

table inet escudo_admin {{
    chain input {{
        type filter hook input priority -5; policy accept;
        tcp dport {{ 8080, 9090 }} ip saddr 216.238.111.108 accept
        tcp dport {{ 8080, 9090 }} drop
    }}
}}
EOF

systemctl enable nftables
systemctl restart nftables

# -------------------------------------------------------------------
# 8. Policy routing (fwmark 0x1/0x2 -> proxy tables)
# -------------------------------------------------------------------
if ! ip rule show | grep -q "fwmark 0x1"; then
    ip rule add fwmark 0x1 table 100
fi
if ! ip rule show | grep -q "fwmark 0x2"; then
    ip rule add fwmark 0x2 table 101
fi

ip route replace blackhole default table 100
ip route replace blackhole default table 101

cat >> /etc/rc.local <<'EOF'
#!/bin/bash
ip rule add fwmark 0x1 table 100 2>/dev/null || true
ip rule add fwmark 0x2 table 101 2>/dev/null || true
ip route replace blackhole default table 100 2>/dev/null || true
ip route replace blackhole default table 101 2>/dev/null || true
exit 0
EOF
chmod +x /etc/rc.local

# -------------------------------------------------------------------
# 9. Download gateway + tun2socks binaries
# -------------------------------------------------------------------
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    BIN_ARCH="amd64"
elif [ "$ARCH" = "aarch64" ]; then
    BIN_ARCH="arm64"
else
    BIN_ARCH="amd64"
fi

echo "[escudo] Downloading binaries for $BIN_ARCH"

# tun2socks
TUN2SOCKS_VERSION="v2.5.2"
curl -fsSL -o /usr/local/bin/tun2socks \
    "https://github.com/xjasonlyu/tun2socks/releases/download/${{TUN2SOCKS_VERSION}}/tun2socks-linux-${{BIN_ARCH}}" \
    || echo "[escudo] WARNING: tun2socks download failed, continuing"
chmod +x /usr/local/bin/tun2socks 2>/dev/null || true

# escudo-gateway (fetched from home URL if available)
curl -fsSL -o /usr/local/bin/escudo-gateway \
    "${{HOME_URL}}/releases/escudo-gateway-linux-${{BIN_ARCH}}" \
    || echo "[escudo] WARNING: escudo-gateway download failed, continuing"
chmod +x /usr/local/bin/escudo-gateway 2>/dev/null || true

# -------------------------------------------------------------------
# 10. Enable persistent services and phone home
# -------------------------------------------------------------------
systemctl daemon-reload
systemctl enable escudo-gateway
systemctl enable escudo-tun2socks-shared
systemctl enable escudo-tun2socks-dedicated
systemctl restart escudo-gateway || true

PUBLIC_IP=$(curl -fsSL https://api.ipify.org || echo "unknown")
WG0_PUBKEY=$(cat /etc/wireguard/wg0.pubkey 2>/dev/null || echo "")
WG1_PUBKEY=$(cat /etc/wireguard/wg1.pubkey 2>/dev/null || echo "")
WG2_PUBKEY=$(cat /etc/wireguard/wg2.pubkey 2>/dev/null || echo "")
COUNTRY_CODE=$(echo "$SERVER_LABEL" | cut -d'-' -f1 | tr '[:lower:]' '[:upper:]')
GATEWAY_GRPC_ADDR="http://${{PUBLIC_IP}}:9090"

curl -fsSL -X POST "${{HOME_URL}}/internal/servers/register" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${{DEPLOY_SECRET}}" \
    -d "{{
        \"label\": \"$SERVER_LABEL\",
        \"public_ip\": \"$PUBLIC_IP\",
        \"wg0_public_key\": \"$WG0_PUBKEY\",
        \"wg1_public_key\": \"$WG1_PUBKEY\",
        \"wg2_public_key\": \"$WG2_PUBKEY\",
        \"wg0_port\": 51820,
        \"wg1_port\": 51821,
        \"wg2_port\": 51822,
        \"country_code\": \"$COUNTRY_CODE\",
        \"gateway_grpc_addr\": \"$GATEWAY_GRPC_ADDR\",
        \"location\": \"$SERVER_LABEL\",
        \"provider\": \"auto\",
        \"version\": \"cloud-init\"
    }}" || echo "[escudo] WARNING: phone-home failed, server not registered"

echo "[escudo] Cloud-init complete for $SERVER_LABEL"
"#,
        server_label = server_label,
        deploy_secret = deploy_secret,
        home_url = home_url,
    )
}
