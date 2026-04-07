# BLE SSH Client (System Tray)

A macOS system tray application that provides SSH access to PiSugar devices over BLE.

## Features

- **System tray / menu bar app** — runs in the macOS menu bar
- **Multi-device support** — connect to multiple PiSugar devices simultaneously, each on its own SSH proxy port
- **BLE-only tunnel** — all SSH traffic is tunneled through BLE, no WiFi or IP address required
- **Auto SSH username** — reads the device's SSH login username via BLE

## Usage

```bash
cargo build --release
./target/release/ble-ssh
```

The app appears in the menu bar with a 📡 icon. Click it to see:

- **device-name** — discovered device (click to connect)
- **✓ device-name → pi@localhost:2222** — active tunnel (submenu: Open SSH, Copy SSH Command, Disconnect)
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

## Environment

Set `RUST_LOG` for logging:

```bash
RUST_LOG=info ./target/release/ble-ssh
RUST_LOG=debug ./target/release/ble-ssh  # verbose BLE logs
```
