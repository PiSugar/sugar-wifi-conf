#!/bin/bash

# Installation paths (current: support/node & master)
INSTALL_DIR="/opt/sugar-wifi-config"
SERVICE_NAME="sugar-wifi-config.service"
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME"

# Old installation path (develop branch, tarball-based)
OLD_INSTALL_DIR="/home/pi/sugar-wifi-conf"

echo "Uninstalling Sugar WiFi Configuration..."

# --- Clean up systemd service (support/node & master) ---
if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
    echo "Stopping $SERVICE_NAME..."
    sudo systemctl stop "$SERVICE_NAME"
fi

if systemctl is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
    echo "Disabling $SERVICE_NAME..."
    sudo systemctl disable "$SERVICE_NAME"
fi

if [ -f "$SERVICE_FILE" ]; then
    echo "Removing systemd service file..."
    sudo rm -f "$SERVICE_FILE"
    sudo systemctl daemon-reload
fi

# --- Clean up rc.local entries (old develop branch) ---
if [ -f /etc/rc.local ]; then
    if grep -q "sugar-wifi-conf" /etc/rc.local; then
        echo "Removing old rc.local configuration..."
        sudo sed -i '/sugar-wifi-conf/d' /etc/rc.local
    fi
fi

# --- Remove installation directories ---
if [ -d "$INSTALL_DIR" ]; then
    echo "Removing $INSTALL_DIR..."
    sudo rm -rf "$INSTALL_DIR"
fi

if [ -d "$OLD_INSTALL_DIR" ]; then
    echo "Removing old installation $OLD_INSTALL_DIR..."
    sudo rm -rf "$OLD_INSTALL_DIR"
fi

echo ""
echo "Sugar WiFi Configuration has been uninstalled."
echo "Note: Node.js, nvm, yarn, and git were not removed."
echo "To remove them manually:"
echo "  - nvm: rm -rf \$HOME/.nvm and remove nvm lines from ~/.bashrc"
echo "  - yarn: npm uninstall -g yarn"
