#!/usr/bin/env bash
set -euo pipefail

# Escudo VPN — Rosenpass setup script
# Downloads rosenpass binary and generates server keypair

ROSENPASS_VERSION="0.2.2"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/escudo"

echo "=== Escudo Rosenpass Setup ==="

# Create config directory
sudo mkdir -p "$CONFIG_DIR"

# Download rosenpass if not present
if ! command -v rosenpass &>/dev/null; then
    echo "Downloading rosenpass v${ROSENPASS_VERSION}..."
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64) ARCH="x86_64" ;;
        aarch64) ARCH="aarch64" ;;
        *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    curl -fsSL "https://github.com/rosenpass/rosenpass/releases/download/v${ROSENPASS_VERSION}/rosenpass-${ARCH}-linux" \
        -o /tmp/rosenpass
    chmod +x /tmp/rosenpass
    sudo mv /tmp/rosenpass "$INSTALL_DIR/rosenpass"
    echo "Installed rosenpass to $INSTALL_DIR/rosenpass"
fi

# Generate server keypair
if [ ! -f "$CONFIG_DIR/rp-secret" ]; then
    echo "Generating Rosenpass server keypair..."
    sudo rosenpass gen-keys \
        --secret-key "$CONFIG_DIR/rp-secret" \
        --public-key "$CONFIG_DIR/rp-public"
    sudo chmod 600 "$CONFIG_DIR/rp-secret"
    sudo chmod 644 "$CONFIG_DIR/rp-public"
    echo "Keypair generated."
else
    echo "Keypair already exists, skipping generation."
fi

# Copy config
sudo cp config/rosenpass.toml "$CONFIG_DIR/rosenpass.toml"

# Install and enable systemd service
sudo cp deploy/escudo-rosenpass.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable escudo-rosenpass

echo "=== Rosenpass setup complete ==="
echo "Start with: sudo systemctl start escudo-rosenpass"
