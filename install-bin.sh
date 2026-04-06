#!/bin/bash
set -e

# Sugar WiFi Config — One-Click Installer (pre-built binary)
# Usage:
#   curl -sSL https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/install-bin.sh | sudo bash
#   curl -sSL https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/install-bin.sh | sudo bash -s -- v0.1.0

REPO="PiSugar/sugar-wifi-conf"
INSTALL_DIR="/opt/sugar-wifi-config"
SERVICE_NAME="sugar-wifi-config.service"
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME"

# --- Detect architecture ---
detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        aarch64|arm64)  echo "aarch64" ;;
        armv7*)         echo "armv7"   ;;
        armv6*)         echo "arm"     ;;
        *)
            echo "Error: unsupported architecture '$arch'" >&2
            echo "Supported: aarch64, armv7l (Pi 2/3/4), armv6l (Pi Zero/1)" >&2
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
        echo "https://github.com/${REPO}/releases/download/${version}/sugar-wifi-conf-${suffix}"
    else
        # Latest release — query GitHub API
        local latest
        latest="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
                  | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')"
        if [ -z "$latest" ]; then
            echo "Error: could not determine latest release version." >&2
            echo "Please specify a version: sudo bash install-bin.sh v0.1.0" >&2
            exit 1
        fi
        echo "https://github.com/${REPO}/releases/download/${latest}/sugar-wifi-conf-${suffix}"
    fi
}

# --- Main ---
echo "=== Sugar WiFi Config Installer ==="

VERSION="${1:-}"
SUFFIX="$(detect_arch)"
URL="$(resolve_url "$VERSION" "$SUFFIX")"

echo "Architecture : $SUFFIX"
echo "Download URL : $URL"

# Install runtime dependencies
echo ""
echo "Installing runtime dependencies..."
apt-get update -qq
apt-get install -y -qq bluez libdbus-1-3 rfkill

# Download binary
echo ""
echo "Downloading sugar-wifi-conf-${SUFFIX}..."
mkdir -p "$INSTALL_DIR"
curl -fSL "$URL" -o "$INSTALL_DIR/sugar-wifi-conf"
chmod +x "$INSTALL_DIR/sugar-wifi-conf"

# Download default config if not present
if [ ! -f "$INSTALL_DIR/custom_config.json" ]; then
    echo "Downloading default custom_config.json..."
    if [ -n "$VERSION" ]; then
        curl -fSL "https://github.com/${REPO}/releases/download/${VERSION}/custom_config.json" \
             -o "$INSTALL_DIR/custom_config.json"
    else
        curl -fSL "https://raw.githubusercontent.com/${REPO}/master/custom_config.json" \
             -o "$INSTALL_DIR/custom_config.json"
    fi
fi

# Symlink
ln -sf "$INSTALL_DIR/sugar-wifi-conf" /usr/local/bin/sugar-wifi-conf

# Clean up old rc.local entries
sed -i '/sugar-wifi-conf/d' /etc/rc.local 2>/dev/null || true

# Stop existing service if running
if [ -f "$SERVICE_FILE" ]; then
    echo "Stopping existing service..."
    systemctl stop "$SERVICE_NAME" 2>/dev/null || true
    systemctl disable "$SERVICE_NAME" 2>/dev/null || true
    rm -f "$SERVICE_FILE"
fi

# Create systemd service
echo "Creating systemd service..."
cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Sugar WiFi Configuration Service
After=network.target bluetooth.target
Wants=bluetooth.target

[Service]
ExecStartPre=/usr/sbin/rfkill unblock bluetooth
ExecStart=$INSTALL_DIR/sugar-wifi-conf --name pisugar --key pisugar --config $INSTALL_DIR/custom_config.json
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
