#!/bin/bash
set -e

# Sugar WiFi Config — One-Click Installer (pre-built binary)
# Usage:
#   curl -sSL https://repo.pisugar.uk/PiSugar/sugar-wifi-conf/raw/master/install-bin.sh | sudo bash
#   curl -sSL https://repo.pisugar.uk/PiSugar/sugar-wifi-conf/raw/master/install-bin.sh | sudo bash -s -- v0.1.0

REPO="PiSugar/sugar-wifi-conf"
PROXY="https://repo.pisugar.uk"
INSTALL_DIR="/opt/sugar-wifi-config"
SERVICE_NAME="sugar-wifi-config.service"
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME"
TMP_BINARY="$INSTALL_DIR/sugar-wifi-conf.tmp"

# --- Detect architecture ---
detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        aarch64|arm64)  echo "aarch64" ;;
        armv7*)         echo "armv7"   ;;
        armv6*)         echo "armv6"   ;;
        *)
            echo "Error: unsupported architecture '$arch'" >&2
            echo "Supported: aarch64 (Pi 3/4/5 64-bit), armv7l (Pi 2/3/4 32-bit), armv6l (Pi Zero/1)" >&2
            exit 1
            ;;
    esac
}

# --- Resolve download URL ---
resolve_url() {
    local version="$1"
    local suffix="$2"

    if [ -n "$version" ]; then
        # Explicit version
        echo "${PROXY}/${REPO}/releases/download/${version}/sugar-wifi-conf-${suffix}"
    else
        # Latest release — use the latest/download redirect to avoid API rate limits
        echo "${PROXY}/${REPO}/releases/latest/download/sugar-wifi-conf-${suffix}"
    fi
}

# --- Main ---
echo "=== Sugar WiFi Config Installer ==="

VERSION="${1:-}"
SUFFIX="$(detect_arch)"
URL="$(resolve_url "$VERSION" "$SUFFIX")"

echo "Architecture : $SUFFIX"
echo "Download URL : $URL"

# Stop existing service before replacing the binary to avoid "Text file busy"
if [ -f "$SERVICE_FILE" ]; then
    echo "Stopping existing service..."
    systemctl stop "$SERVICE_NAME" 2>/dev/null || true
    systemctl disable "$SERVICE_NAME" 2>/dev/null || true
    rm -f "$SERVICE_FILE"
fi

# Install runtime dependencies
echo ""
echo "Installing runtime dependencies..."
export DEBIAN_FRONTEND=noninteractive
export NEEDRESTART_MODE=a
export APT_LISTCHANGES_FRONTEND=none
export UCF_FORCE_CONFOLD=1
apt-get update -qq
apt-get install -y -qq -o Dpkg::Options::=--force-confold bluez libdbus-1-3 rfkill

# Download binary
echo ""
echo "Downloading sugar-wifi-conf-${SUFFIX}..."
mkdir -p "$INSTALL_DIR"
curl -fSL "$URL" -o "$TMP_BINARY"
mv -f "$TMP_BINARY" "$INSTALL_DIR/sugar-wifi-conf"
chmod +x "$INSTALL_DIR/sugar-wifi-conf"

# Download default config if not present
if [ ! -f "$INSTALL_DIR/custom_config.json" ]; then
    echo "Downloading default custom_config.json..."
    if [ -n "$VERSION" ]; then
        curl -fSL "${PROXY}/${REPO}/releases/download/${VERSION}/custom_config.json" \
             -o "$INSTALL_DIR/custom_config.json"
    else
        curl -fSL "${PROXY}/${REPO}/releases/latest/download/custom_config.json" \
             -o "$INSTALL_DIR/custom_config.json"
    fi
fi

# Symlink
ln -sf "$INSTALL_DIR/sugar-wifi-conf" /usr/local/bin/sugar-wifi-conf

# Clean up old rc.local entries
sed -i '/sugar-wifi-conf/d' /etc/rc.local 2>/dev/null || true

# Create systemd service
echo "Creating systemd service..."
cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Sugar WiFi Configuration Service
After=network.target bluetooth.target
Wants=bluetooth.target

[Service]
ExecStartPre=/usr/sbin/rfkill unblock bluetooth
ExecStart=$INSTALL_DIR/sugar-wifi-conf --name raspberrypi --key pisugar --config $INSTALL_DIR/custom_config.json
WorkingDirectory=$INSTALL_DIR
Restart=always
RestartSec=5
User=root
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl start "$SERVICE_NAME"

echo ""
echo "=== Installation complete! ==="
echo ""
echo "Binary  : $INSTALL_DIR/sugar-wifi-conf"
echo "Config  : $INSTALL_DIR/custom_config.json"
echo "Service : $SERVICE_NAME"
echo ""
echo "Useful commands:"
echo "  sudo systemctl status  $SERVICE_NAME"
echo "  sudo systemctl restart $SERVICE_NAME"
echo "  sudo journalctl -u $SERVICE_NAME -f"
echo ""
echo "To edit configuration interactively:"
echo "  sugar-wifi-conf config --config $INSTALL_DIR/custom_config.json"
