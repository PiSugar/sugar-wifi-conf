# BLE SSH Client (System Tray)

A cross-platform system tray application that provides SSH access to PiSugar devices over BLE.

## Features

- **System tray / menu bar app** — runs in the macOS menu bar, Windows system tray, or Linux indicator area
- **Multi-device support** — connect to multiple PiSugar devices simultaneously, each on its own SSH proxy port
- **Auto IP discovery** — reads each device's LAN IP during scan and displays it in the menu
- **Smart routing** — tries direct TCP connection first (if device has LAN IP), falls back to BLE tunnel
- **Cross-platform** — macOS, Windows, Linux (via btleplug + tray-icon + tao)

## Usage

```bash
cargo build --release
./target/release/ble-ssh
```

The app appears in your system tray / menu bar with a dot icon. Click it to see:

- **📡 device-name (192.168.x.x)** — discovered device with IP (click to connect)
- **🔗 device-name → localhost:2222** — active tunnel (click to disconnect)
- **🔄 Rescan** — trigger a new BLE scan
- **Quit** — exit the app

After connecting, SSH into the device:

```bash
ssh pi@localhost -p 2222
```

Each connected device gets a unique port starting from 2222.

## Platform Requirements

### macOS

No extra dependencies. Requires Bluetooth permission (grant when prompted).

### Linux

Install the app indicator library:

```bash
# Ubuntu / Debian
sudo apt install libayatana-appindicator3-dev

# Fedora
sudo dnf install libayatana-appindicator-gtk3-devel
```

Requires BlueZ 5.x.

### Windows

Requires Windows 10+ with Bluetooth support.

## Environment

Set `RUST_LOG` for logging:

```bash
RUST_LOG=info ./target/release/ble-ssh
RUST_LOG=debug ./target/release/ble-ssh  # verbose BLE logs
```
