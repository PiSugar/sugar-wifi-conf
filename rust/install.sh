#!/bin/bash
set -e

# ============================================================
# NOTE: For pre-built binaries (no Rust toolchain needed), use:
#   curl -sSL https://raw.githubusercontent.com/PiSugar/sugar-wifi-conf/master/install-bin.sh | sudo bash
#
# This script builds from source and requires Rust + build deps.
# ============================================================

REPO_URL="https://github.com/PiSugar/sugar-wifi-conf.git"
INSTALL_DIR="/opt/sugar-wifi-config"
SERVICE_NAME="sugar-wifi-config.service"
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME"

echo "=== Sugar WiFi Config (Rust) Installer ==="

# Check for Rust toolchain
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Install Rust if not present
if ! command_exists cargo; then
    echo "Rust is not installed. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Install build dependencies
echo "Installing build dependencies..."
sudo apt-get update -qq
sudo apt-get install -y -qq libdbus-1-dev pkg-config bluetooth bluez libbluetooth-dev

# Clone or update repo
if [ -d "$INSTALL_DIR" ]; then
    echo "$INSTALL_DIR exists. Updating..."
    cd "$INSTALL_DIR"
    git pull || true
else
    echo "Cloning $REPO_URL to $INSTALL_DIR..."
    sudo mkdir -p "$INSTALL_DIR"
    sudo chown "$USER:$USER" "$INSTALL_DIR"
    git clone "$REPO_URL" "$INSTALL_DIR" --depth 1
fi

# Build Rust binary
echo "Building Rust binary (release mode)..."
cd "$INSTALL_DIR/rust"
cargo build --release

# Copy binary to install directory
cp "$INSTALL_DIR/rust/target/release/sugar-wifi-conf" "$INSTALL_DIR/sugar-wifi-conf"
chmod +x "$INSTALL_DIR/sugar-wifi-conf"

# Create symlink so 'sugar-wifi-conf' is available system-wide
echo "Creating symlink in /usr/local/bin..."
sudo ln -sf "$INSTALL_DIR/sugar-wifi-conf" /usr/local/bin/sugar-wifi-conf

# Remove old rc.local configuration
echo "Removing old rc.local configuration..."
sudo sed -i '/sugar-wifi-conf/d' /etc/rc.local 2>/dev/null || true

# If systemd service exists, stop and remove it
if [ -f "$SERVICE_FILE" ]; then
    echo "Removing old systemd service..."
    sudo systemctl stop "$SERVICE_NAME" 2>/dev/null || true
    sudo systemctl disable "$SERVICE_NAME" 2>/dev/null || true
    sudo rm -f "$SERVICE_FILE"
fi

# Create systemd service file
echo "Creating systemd service file..."
sudo bash -c "cat > $SERVICE_FILE <<EOF
[Unit]
Description=Sugar WiFi Configuration Service (Rust)
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
EOF"

# Reload systemd
echo "Reloading systemd configuration..."
sudo systemctl daemon-reload

# Enable and start
echo "Enabling and starting $SERVICE_NAME..."
sudo systemctl enable "$SERVICE_NAME"
sudo systemctl start "$SERVICE_NAME"

echo ""
echo "Installation complete!"
echo "Check status: sudo systemctl status $SERVICE_NAME"
echo "View logs:    sudo journalctl -u $SERVICE_NAME -f"
