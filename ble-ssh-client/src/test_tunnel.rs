//! Automated BLE SSH tunnel test.
//! Scans for a PiSugar device, connects, opens tunnel, verifies SSH banner.
//! Exit code 0 = success, 1 = failure.

mod ble;
mod speed;
mod tunnel;
mod uuid;

use std::sync::Arc;
use std::time::Duration;

use btleplug::api::{Central as _, Peripheral as _};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(e) = run_test().await {
        log::error!("TEST FAILED: {}", e);
        std::process::exit(1);
    }
    log::info!("TEST PASSED: SSH over BLE tunnel works!");
    std::process::exit(0);
}

async fn run_test() -> Result<(), String> {
    // Step 1: Get adapter
    log::info!("=== Step 1: Get BLE adapter ===");
    let adapter = ble::get_adapter().await?;

    // Step 2: Find device — scan only, don't connect yet
    log::info!("=== Step 2: Find PiSugar device (scan only) ===");
    let service_uuid = uuid::parse_uuid(uuid::SERVICE_ID);
    let dev = find_device(&adapter, service_uuid).await?;
    log::info!("Found: {} ({})", dev.name, dev.id);

    // Step 3: Connect, read username, create tunnel — all in one shot
    // This matches how the tray app's Cmd::Connect handler works.
    log::info!("=== Step 3: Connect + create tunnel (one shot) ===");
    ble::ble_connect(&dev.peripheral).await?;
    log::info!("Connected and services discovered");

    let user = ble::read_ssh_username(&dev.peripheral)
        .await
        .unwrap_or_else(|| "pi".to_string());
    log::info!("SSH user: {}", user);

    let speed_tracker = speed::SpeedTracker::new();
    let tunnel = ble::BleTunnel::from_connected(dev.peripheral.clone(), speed_tracker.clone()).await?;
    let tunnel = Arc::new(tunnel);
    log::info!("BleTunnel created");

    // Step 4: Start TCP proxy
    log::info!("=== Step 4: Start TCP proxy on port 12222 ===");
    let tun = tunnel.clone();
    let spd = speed_tracker.clone();
    tokio::spawn(async move {
        if let Err(e) = tunnel::run_device_proxy(12222, tun, spd).await {
            log::error!("Proxy error: {}", e);
        }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Step 5: Test actual SSH
    log::info!("=== Step 5: Testing actual SSH connection via localhost:12222 ===");

    let ssh_result = tokio::process::Command::new("ssh")
        .args([
            "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=15",
            "-p", "12222",
            &format!("{}@localhost", user),
            "echo BLE_SSH_OK && uname -a",
        ])
        .output()
        .await
        .map_err(|e| format!("SSH spawn: {}", e))?;

    let stdout = String::from_utf8_lossy(&ssh_result.stdout);
    let stderr = String::from_utf8_lossy(&ssh_result.stderr);
    log::info!("SSH stdout: {}", stdout.trim());
    if !stderr.is_empty() {
        log::info!("SSH stderr: {}", stderr.trim());
    }
    log::info!("SSH exit code: {}", ssh_result.status.code().unwrap_or(-1));

    if !stdout.contains("BLE_SSH_OK") {
        return Err(format!(
            "SSH did not produce expected output. stdout={}, stderr={}",
            stdout.trim(),
            stderr.trim()
        ));
    }

    // Cleanup
    log::info!("=== Cleanup ===");
    tunnel.close_tunnel().await;
    let _ = adapter.stop_scan().await;
    let _ = dev.peripheral.disconnect().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}

/// Find a PiSugar device: check cached peripherals first, then scan.
async fn find_device(
    adapter: &btleplug::platform::Adapter,
    service_uuid: ::uuid::Uuid,
) -> Result<ble::ScannedDevice, String> {
    // Check if we already know about it (cached from previous connection)
    log::info!("Checking cached peripherals...");
    if let Ok(peripherals) = adapter.peripherals().await {
        for p in peripherals {
            if let Ok(Some(props)) = p.properties().await {
                if props.services.contains(&service_uuid) {
                    let name = props.local_name.unwrap_or_else(|| "Unknown".to_string());
                    let id = format!("{:?}", p.id());
                    log::info!("Found cached device: {} ({})", name, id);
                    return Ok(ble::ScannedDevice {
                        peripheral: p,
                        name,
                        id,
                    });
                }
            }
        }
    }

    // Not cached — do a scan
    log::info!("No cached device, scanning (15s)...");
    let (dev_tx, mut dev_rx) = tokio::sync::mpsc::unbounded_channel();
    let scan_adapter = adapter.clone();
    tokio::spawn(async move {
        if let Err(e) = ble::scan_devices(&scan_adapter, Duration::from_secs(15), dev_tx).await {
            log::error!("Scan error: {}", e);
        }
    });

    tokio::time::timeout(Duration::from_secs(20), dev_rx.recv())
        .await
        .map_err(|_| "No device found within 20s".to_string())?
        .ok_or_else(|| "Scan returned no devices".to_string())
}
