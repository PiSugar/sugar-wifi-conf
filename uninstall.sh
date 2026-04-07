#!/bin/bash
INSTALL_DIR="/opt/sugar-wifi-config"
SERVICE_NAME="sugar-wifi-config.service"
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME"

echo "Uninstalling Sugar WiFi Configuration..."

# Stop and disable systemd service
if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
    echo "Stopping $SERVICE_NAME..."
    sudo systemctl stop "$SERVICE_NAME"
fi

if systemctl is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
    echo "Disabling $SERVICE_NAME..."
    sudo systemctl disable "$SERVICE_NAME"
fi

# Remove systemd service file
if [ -f "$SERVICE_FILE" ]; then
    echo "Removing systemd service file..."
    sudo rm -f "$SERVICE_FILE"
    sudo systemctl daemon-reload
fi

# Remove old rc.local configuration
if [ -f /etc/rc.local ]; then
    echo "Removing old rc.local configuration..."
    sudo sed -i '/sugar-wifi-conf/d' /etc/rc.local
fi

# Remove installation directory
if [ -d "$INSTALL_DIR" ]; then
    echo "Removing $INSTALL_DIR..."
    sudo rm -rf "$INSTALL_DIR"
fi

echo ""
echo "Sugar WiFi Configuration has been uninstalled."
echo "Note: Node.js, nvm, yarn, and git were not removed."
echo "To remove them manually:"
echo "  - nvm: rm -rf \$HOME/.nvm and remove nvm lines from ~/.bashrc"
echo "  - yarn: npm uninstall -g yarn"
