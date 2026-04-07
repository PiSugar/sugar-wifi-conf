#!/bin/bash
set -e

echo "Starting sugar-wifi-conf (Rust)..."

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BINARY="$SCRIPT_DIR/target/release/sugar-wifi-conf"

# If no release binary, try debug
if [ ! -f "$BINARY" ]; then
    BINARY="$SCRIPT_DIR/target/debug/sugar-wifi-conf"
fi

# If still no binary, check /opt install location
if [ ! -f "$BINARY" ]; then
    BINARY="/opt/sugar-wifi-config/sugar-wifi-conf"
fi

if [ ! -f "$BINARY" ]; then
    echo "Error: sugar-wifi-conf binary not found"
    exit 1
fi

echo "Unblocking bluetooth..."
rfkill list
rfkill unblock bluetooth
sudo hciconfig hci0 up 2>/dev/null || true

# Parse arguments: first arg is BLE name/key, second is config path
# Default BLE name: use device model from /proc/device-tree/model
DEFAULT_NAME=$(tr -d '\0' < /proc/device-tree/model 2>/dev/null || echo "pisugar")
BLE_NAME="${1:-$DEFAULT_NAME}"
CONFIG_PATH="${2:-$SCRIPT_DIR/../custom_config.json}"

echo "Running: $BINARY --name $BLE_NAME --key $BLE_NAME --config $CONFIG_PATH"
exec "$BINARY" --name "$BLE_NAME" --key "$BLE_NAME" --config "$CONFIG_PATH"
