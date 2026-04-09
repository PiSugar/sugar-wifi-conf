#!/bin/bash
# build-pi-image.sh — Build a Raspberry Pi OS image with sugar-wifi-conf pre-installed
#
# Usage:
#   sudo bash build-pi-image.sh <arm64|armhf> <lite|desktop> <binary_path> <config_path> [output_name]
#
# Example:
#   sudo bash build-pi-image.sh arm64 lite ./sugar-wifi-conf-aarch64 ./custom_config.json
#   sudo bash build-pi-image.sh arm64 desktop ./sugar-wifi-conf-aarch64 ./custom_config.json

set -euo pipefail

ARCH="${1:?Usage: $0 <arm64|armhf> <lite|desktop> <binary_path> <config_path> [output_name]}"
VARIANT="${2:?Usage: $0 <arm64|armhf> <lite|desktop> <binary_path> <config_path> [output_name]}"
BINARY_PATH="${3:?Usage: $0 <arm64|armhf> <lite|desktop> <binary_path> <config_path> [output_name]}"
CONFIG_PATH="${4:?Usage: $0 <arm64|armhf> <lite|desktop> <binary_path> <config_path> [output_name]}"
OUTPUT_NAME="${5:-sugar-wifi-conf-raspios-${VARIANT}-${ARCH}}"

echo "=== Building Raspberry Pi OS image with sugar-wifi-conf ==="
echo "Architecture : ${ARCH}"
echo "Variant      : ${VARIANT}"
echo "Binary       : ${BINARY_PATH}"
echo "Config       : ${CONFIG_PATH}"
echo "Output       : ${OUTPUT_NAME}.img.xz"

# Validate inputs
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: binary not found: ${BINARY_PATH}" >&2
    exit 1
fi
if [ ! -f "$CONFIG_PATH" ]; then
    echo "Error: config not found: ${CONFIG_PATH}" >&2
    exit 1
fi

# Validate variant
case "$VARIANT" in
    lite|desktop) ;;
    *) echo "Error: unsupported variant '${VARIANT}' (use lite or desktop)" >&2; exit 1 ;;
esac

# Map arch + variant to Raspberry Pi OS download path
# Lite: raspios_lite_arm64, raspios_lite_armhf
# Desktop: raspios_arm64, raspios_armhf
case "${VARIANT}-${ARCH}" in
    lite-arm64)    IMAGE_PATH="raspios_lite_arm64" ;;
    lite-armhf)    IMAGE_PATH="raspios_lite_armhf" ;;
    desktop-arm64) IMAGE_PATH="raspios_arm64" ;;
    desktop-armhf) IMAGE_PATH="raspios_armhf" ;;
    *)             echo "Error: unsupported combination '${VARIANT}-${ARCH}'" >&2; exit 1 ;;
esac

# ── Download latest Raspberry Pi OS image ───────────────────────────────

WORK_DIR=$(mktemp -d)
echo ""
echo "Working directory: ${WORK_DIR}"

BASE_URL="https://downloads.raspberrypi.com/${IMAGE_PATH}/images"
echo "Finding latest image from ${BASE_URL} ..."

# Parse the Apache/nginx directory listing to find the newest release folder
LATEST_DIR=$(curl -sfL "${BASE_URL}/" \
    | grep -oE "href=\"${IMAGE_PATH}-[0-9]{4}-[0-9]{2}-[0-9]{2}/\"" \
    | sed 's/href="//;s/\/"//' | sort -V | tail -1)

if [ -z "$LATEST_DIR" ]; then
    echo "Error: could not determine latest image directory" >&2
    exit 1
fi
echo "Latest release : ${LATEST_DIR}"

# Find the .img.xz filename inside that folder
IMAGE_XZ_NAME=$(curl -sfL "${BASE_URL}/${LATEST_DIR}/" \
    | grep -oE "href=\"[^\"]+\.img\.xz\"" \
    | sed 's/href="//;s/"$//' | head -1)

if [ -z "$IMAGE_XZ_NAME" ]; then
    echo "Error: could not find .img.xz in ${LATEST_DIR}" >&2
    exit 1
fi

IMAGE_URL="${BASE_URL}/${LATEST_DIR}/${IMAGE_XZ_NAME}"
echo "Downloading    : ${IMAGE_URL}"
curl -fSL --progress-bar -o "${WORK_DIR}/image.img.xz" "${IMAGE_URL}"

# ── Decompress ──────────────────────────────────────────────────────────

echo ""
echo "Decompressing image ..."
xz -d "${WORK_DIR}/image.img.xz"
IMAGE_FILE="${WORK_DIR}/image.img"

# ── Grow image to make room for the binary ──────────────────────────────

echo "Growing image by 64 MB ..."
truncate -s +64M "$IMAGE_FILE"

# ── Set up loop device ──────────────────────────────────────────────────

LOOP_DEV=""
MOUNT_DIR=""
BOOT_DIR=""

cleanup() {
    echo "Cleaning up ..."
    if [ -n "$BOOT_DIR" ] && mountpoint -q "$BOOT_DIR" 2>/dev/null; then
        umount "$BOOT_DIR" || true
    fi
    [ -n "$BOOT_DIR" ] && rm -rf "$BOOT_DIR"
    if [ -n "$MOUNT_DIR" ] && mountpoint -q "$MOUNT_DIR" 2>/dev/null; then
        umount "$MOUNT_DIR" || true
    fi
    [ -n "$MOUNT_DIR" ] && rm -rf "$MOUNT_DIR"
    if [ -n "$LOOP_DEV" ]; then
        losetup -d "$LOOP_DEV" 2>/dev/null || true
    fi
}
trap cleanup EXIT

LOOP_DEV=$(losetup -fP --show "$IMAGE_FILE")
echo "Loop device    : ${LOOP_DEV}"

# Grow partition 2 (root) to fill the extra space
growpart "$LOOP_DEV" 2 || true
e2fsck -fy "${LOOP_DEV}p2" || true
resize2fs "${LOOP_DEV}p2"

# ── Mount root filesystem ───────────────────────────────────────────────

MOUNT_DIR=$(mktemp -d)
mount "${LOOP_DEV}p2" "$MOUNT_DIR"
echo "Mounted rootfs : ${MOUNT_DIR}"

# ── Install sugar-wifi-conf ─────────────────────────────────────────────

INSTALL_DIR="$MOUNT_DIR/opt/sugar-wifi-config"
mkdir -p "$INSTALL_DIR"

echo "Installing binary ..."
cp "$BINARY_PATH" "$INSTALL_DIR/sugar-wifi-conf"
chmod +x "$INSTALL_DIR/sugar-wifi-conf"

echo "Installing config ..."
cp "$CONFIG_PATH" "$INSTALL_DIR/custom_config.json"

echo "Creating symlink /usr/local/bin/sugar-wifi-conf ..."
ln -sf /opt/sugar-wifi-config/sugar-wifi-conf "$MOUNT_DIR/usr/local/bin/sugar-wifi-conf"

echo "Installing systemd service ..."
cat > "$MOUNT_DIR/etc/systemd/system/sugar-wifi-config.service" <<'SERVICE'
[Unit]
Description=Sugar WiFi Configuration Service
After=network.target bluetooth.target
Wants=bluetooth.target

[Service]
ExecStartPre=/usr/sbin/rfkill unblock bluetooth
ExecStart=/opt/sugar-wifi-config/sugar-wifi-conf --name raspberrypi --key pisugar --config /opt/sugar-wifi-config/custom_config.json
WorkingDirectory=/opt/sugar-wifi-config
Restart=always
RestartSec=5
User=root
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
SERVICE

echo "Enabling service ..."
mkdir -p "$MOUNT_DIR/etc/systemd/system/multi-user.target.wants"
ln -sf /etc/systemd/system/sugar-wifi-config.service \
    "$MOUNT_DIR/etc/systemd/system/multi-user.target.wants/sugar-wifi-config.service"

# ── Enable SSH ──────────────────────────────────────────────────────────

echo "Enabling SSH ..."
mkdir -p "$MOUNT_DIR/etc/systemd/system/sshd.service.d"
mkdir -p "$MOUNT_DIR/etc/systemd/system/multi-user.target.wants"
ln -sf /usr/lib/systemd/system/ssh.service \
    "$MOUNT_DIR/etc/systemd/system/multi-user.target.wants/ssh.service" 2>/dev/null || true
ln -sf /lib/systemd/system/ssh.service \
    "$MOUNT_DIR/etc/systemd/system/multi-user.target.wants/ssh.service" 2>/dev/null || true

# Also place marker on boot partition (legacy method)
BOOT_DIR=$(mktemp -d)
mount "${LOOP_DEV}p1" "$BOOT_DIR"
touch "$BOOT_DIR/ssh"
echo "SSH marker placed on boot partition"
umount "$BOOT_DIR"
rm -rf "$BOOT_DIR"
BOOT_DIR=""

# ── Configure default user pi:raspberry ─────────────────────────────────

echo "Setting up user pi with default password ..."
# Generate password hash for 'raspberry'
PASS_HASH=$(openssl passwd -6 raspberry)

# userconf.txt: used by Raspberry Pi OS firstboot to create the user
echo "pi:${PASS_HASH}" > "$MOUNT_DIR/boot/userconf.txt" 2>/dev/null || true
echo "pi:${PASS_HASH}" > "$MOUNT_DIR/etc/userconf.txt" 2>/dev/null || true

# Also pre-populate /etc/shadow in case firstboot is skipped
if [ -f "$MOUNT_DIR/etc/shadow" ]; then
    # If pi user already exists in shadow, update its hash
    if grep -q '^pi:' "$MOUNT_DIR/etc/shadow"; then
        sed -i "s|^pi:[^:]*:|pi:${PASS_HASH}:|" "$MOUNT_DIR/etc/shadow"
    fi
fi

# ── Set WiFi regulatory country & unlock WiFi ───────────────────────────

echo "Creating WiFi unblock service ..."
cat > "$MOUNT_DIR/etc/systemd/system/wifi-unblock.service" <<'WIFISERVICE'
[Unit]
Description=Set WiFi regulatory domain and unblock WiFi
Before=network-pre.target
Wants=network-pre.target

[Service]
Type=oneshot
ExecStart=/usr/bin/raspi-config nonint do_wifi_country GB
ExecStart=/usr/sbin/rfkill unblock wifi
ExecStart=/usr/bin/nmcli radio wifi on
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
WIFISERVICE

ln -sf /etc/systemd/system/wifi-unblock.service \
    "$MOUNT_DIR/etc/systemd/system/multi-user.target.wants/wifi-unblock.service"

# ── Disable Bluetooth panel plugin (desktop only) ───────────────────────
# The wf-panel-pi bluetooth widget triggers continuous BLE scanning, which
# conflicts with BLE advertising on the BCM43430A1 (single-radio chip).
# HCI opcode 0x2005 (LE Set Advertising Parameters) fails with EBUSY (-16)
# whenever scanning and advertising run simultaneously.

if [ "$VARIANT" = "desktop" ]; then
    echo "Removing bluetooth widget from wf-panel-pi config ..."
    PANEL_CFG="$MOUNT_DIR/etc/xdg/wf-panel-pi/wf-panel-pi.ini"
    if [ -f "$PANEL_CFG" ]; then
        sed -i 's/ bluetooth / /g' "$PANEL_CFG"
        echo "  bluetooth widget removed from panel config"
    else
        echo "  wf-panel-pi.ini not found, skipping"
    fi
fi

# ── Unmount & detach ────────────────────────────────────────────────────

echo ""
echo "Unmounting ..."
umount "$MOUNT_DIR"
rm -rf "$MOUNT_DIR"
MOUNT_DIR=""
BOOT_DIR=""

losetup -d "$LOOP_DEV"
LOOP_DEV=""
trap - EXIT

# ── Compress output ─────────────────────────────────────────────────────

echo "Compressing image (xz, multi-threaded) ..."
xz -T0 -3 "$IMAGE_FILE"
mv "${IMAGE_FILE}.xz" "${OUTPUT_NAME}.img.xz"

echo ""
echo "=== Done! ==="
ls -lh "${OUTPUT_NAME}.img.xz"

# Write base image version info for CI release notes
echo "${IMAGE_XZ_NAME}" > "${OUTPUT_NAME}.base-image.txt"
echo "Base image info written to ${OUTPUT_NAME}.base-image.txt"
