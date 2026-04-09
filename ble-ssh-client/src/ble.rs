use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;

use crate::speed::SpeedTracker;
use crate::uuid as suuid;

/// A device found during BLE scan.
pub struct ScannedDevice {
    pub peripheral: Peripheral,
    pub name: String,
    pub id: String,
}

/// Find the BLE adapter.
pub async fn get_adapter() -> Result<Adapter, String> {
    let manager = Manager::new()
        .await
        .map_err(|e| format!("BLE manager: {}", e))?;
    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("No adapters: {}", e))?;
    adapters
        .into_iter()
        .next()
        .ok_or_else(|| "No BLE adapter found".to_string())
}

/// Scan for PiSugar BLE devices for a given duration.
/// Sends each device through `device_tx` as soon as it's discovered.
pub async fn scan_devices(
    adapter: &Adapter,
    timeout: Duration,
    device_tx: mpsc::UnboundedSender<ScannedDevice>,
) -> Result<(), String> {
    let service_uuid = suuid::parse_uuid(suuid::SERVICE_ID);

    // Stop any stale scan from a previous run that didn't clean up
    let _ = adapter.stop_scan().await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    adapter
        .start_scan(ScanFilter {
            services: vec![service_uuid],
        })
        .await
        .map_err(|e| format!("Scan: {}", e))?;

    let mut events = adapter
        .events()
        .await
        .map_err(|e| format!("Events: {}", e))?;

    let mut seen = HashSet::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        match tokio::time::timeout(remaining.min(Duration::from_millis(200)), events.next()).await {
            // Handle both Discovered and Updated — on macOS, CoreBluetooth may
            // fire DeviceUpdated instead of DeviceDiscovered for cached devices.
            Ok(Some(CentralEvent::DeviceDiscovered(id)))
            | Ok(Some(CentralEvent::DeviceUpdated(id))) => {
                if let Ok(p) = adapter.peripheral(&id).await {
                    if let Ok(Some(props)) = p.properties().await {
                        if props.services.contains(&service_uuid) {
                            let id_str = format!("{:?}", id);
                            if seen.insert(id_str.clone()) {
                                let raw_name = props
                                    .local_name
                                    .unwrap_or_else(|| "Unknown".to_string());
                                // Strip trailing "[...]" suffix (e.g. "cm5 [pisugar]" → "cm5")
                                let name = raw_name
                                    .find(" [")
                                    .map(|i| raw_name[..i].to_string())
                                    .unwrap_or(raw_name);
                                log::info!("Scan: found {} ({})", name, id_str);
                                let _ = device_tx.send(ScannedDevice {
                                    peripheral: p,
                                    name,
                                    id: id_str,
                                });
                            }
                        }
                    }
                }
            }
            _ => continue,
        }
    }

    let _ = adapter.stop_scan().await;
    Ok(())
}

/// Connect to a BLE device and discover its services.
pub async fn ble_connect(peripheral: &Peripheral) -> Result<(), String> {
    // Disconnect stale first
    let _ = tokio::time::timeout(Duration::from_secs(3), peripheral.disconnect()).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    tokio::time::timeout(Duration::from_secs(15), peripheral.connect())
        .await
        .map_err(|_| "Connect timeout (15s)".to_string())?
        .map_err(|e| format!("Connect: {}", e))?;

    tokio::time::timeout(Duration::from_secs(15), peripheral.discover_services())
        .await
        .map_err(|_| "Discovery timeout (15s)".to_string())?
        .map_err(|e| format!("Discovery: {}", e))?;

    Ok(())
}

fn find_char(chars: &[Characteristic], uuid_hex: &str) -> Result<Characteristic, String> {
    let target = suuid::parse_uuid(uuid_hex);
    chars
        .iter()
        .find(|c| c.uuid == target)
        .cloned()
        .ok_or_else(|| format!("Characteristic {} not found", uuid_hex))
}

/// Read the SSH_USERNAME characteristic.
pub async fn read_ssh_username(peripheral: &Peripheral) -> Option<String> {
    let chars: Vec<_> = peripheral.characteristics().into_iter().collect();
    if let Ok(c) = find_char(&chars, suuid::SSH_USERNAME) {
        if let Ok(Ok(data)) =
            tokio::time::timeout(Duration::from_secs(5), peripheral.read(&c)).await
        {
            let user = String::from_utf8_lossy(&data).trim().to_string();
            if !user.is_empty() {
                return Some(user);
            }
        }
    }
    None
}

// ── BleTunnel ─────────────────────────────────────────

/// Manages the BLE SSH tunnel with a persistent notification pump.
pub struct BleTunnel {
    peripheral: Peripheral,
    ssh_ctrl: Characteristic,
    ssh_rx: Characteristic,
    speed: Arc<SpeedTracker>,
    ctrl_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    data_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
}

impl BleTunnel {
    /// Create from an already-connected peripheral with services discovered.
    /// Subscribes to SSH characteristics and starts a persistent notification pump.
    pub async fn from_connected(
        peripheral: Peripheral,
        speed: Arc<SpeedTracker>,
    ) -> Result<Self, String> {
        let chars: Vec<_> = peripheral.characteristics().into_iter().collect();
        log::info!("Found {} characteristics", chars.len());
        let ssh_ctrl = find_char(&chars, suuid::SSH_CTRL)?;
        let ssh_rx = find_char(&chars, suuid::SSH_RX)?;
        let ssh_tx = find_char(&chars, suuid::SSH_TX)?;

        peripheral
            .subscribe(&ssh_ctrl)
            .await
            .map_err(|e| format!("Subscribe CTRL: {}", e))?;
        peripheral
            .subscribe(&ssh_tx)
            .await
            .map_err(|e| format!("Subscribe TX: {}", e))?;

        // Let BLE subscriptions settle before sending commands
        tokio::time::sleep(Duration::from_millis(300)).await;

        let (ctrl_tx, ctrl_rx) = mpsc::channel::<String>(16);
        let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>(512);

        let p = peripheral.clone();
        let ctrl_uuid = ssh_ctrl.uuid;
        let tx_uuid = ssh_tx.uuid;
        let spd = speed.clone();

        tokio::spawn(async move {
            let mut stream = match p.notifications().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Notification stream: {}", e);
                    return;
                }
            };
            log::info!("Notification pump started (CTRL={}, TX={})", ctrl_uuid, tx_uuid);
            while let Some(notif) = stream.next().await {
                if notif.uuid == tx_uuid {
                    log::debug!("BLE notif TX: {} bytes", notif.value.len());
                    spd.add_rx(notif.value.len() as u64);
                    if data_tx.send(notif.value).await.is_err() {
                        break;
                    }
                } else if notif.uuid == ctrl_uuid {
                    let msg = String::from_utf8_lossy(&notif.value).to_string();
                    log::info!("BLE notif CTRL: '{}'", msg);
                    if ctrl_tx.send(msg).await.is_err() {
                        break;
                    }
                } else {
                    log::debug!("BLE notif unknown UUID: {}", notif.uuid);
                }
            }
            log::info!("Notification pump ended");
        });

        Ok(Self {
            peripheral,
            ssh_ctrl,
            ssh_rx,
            speed,
            ctrl_rx: Arc::new(Mutex::new(ctrl_rx)),
            data_rx: Arc::new(Mutex::new(data_rx)),
        })
    }

    /// Open the SSH tunnel: send CONNECT, wait for server OK.
    pub async fn open_tunnel(&self) -> Result<(), String> {
        // Drain stale data
        {
            let mut d = self.data_rx.lock().await;
            while d.try_recv().is_ok() {}
        }
        {
            let mut c = self.ctrl_rx.lock().await;
            while c.try_recv().is_ok() {}
        }

        log::info!("Writing CONNECT to SSH_CTRL...");
        self.peripheral
            .write(&self.ssh_ctrl, b"CONNECT", WriteType::WithResponse)
            .await
            .map_err(|e| format!("Write CONNECT: {}", e))?;
        log::info!("CONNECT written, waiting for OK notification...");

        let mut ctrl = self.ctrl_rx.lock().await;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err("CONNECT timeout".to_string());
            }
            match tokio::time::timeout(remaining, ctrl.recv()).await {
                Ok(Some(msg)) if msg == "OK" => {
                    log::info!("Tunnel opened (server OK)");
                    return Ok(());
                }
                Ok(Some(msg)) if msg.starts_with("ERR:") => {
                    return Err(format!("Server: {}", msg));
                }
                Ok(Some(_)) => continue,
                Ok(None) => return Err("Control channel closed".to_string()),
                Err(_) => return Err("CONNECT timeout".to_string()),
            }
        }
    }

    /// Receive SSH data from the persistent pump.
    pub async fn recv_data(&self) -> Option<Vec<u8>> {
        self.data_rx.lock().await.recv().await
    }

    /// Send DISCONNECT and wait for CLOSED.
    pub async fn close_tunnel(&self) {
        let _ = self
            .peripheral
            .write(&self.ssh_ctrl, b"DISCONNECT", WriteType::WithResponse)
            .await;
        let mut ctrl = self.ctrl_rx.lock().await;
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(msg) = ctrl.recv().await {
                if msg == "CLOSED" {
                    log::info!("Tunnel closed (server CLOSED)");
                    break;
                }
            }
        })
        .await;
    }

    /// Write data to SSH_RX (client → Pi).
    pub async fn write_data(&self, data: &[u8]) -> Result<(), String> {
        for chunk in data.chunks(512) {
            self.peripheral
                .write(&self.ssh_rx, chunk, WriteType::WithResponse)
                .await
                .map_err(|e| format!("Write: {}", e))?;
            self.speed.add_tx(chunk.len() as u64);
        }
        Ok(())
    }

    pub async fn disconnect(&self) {
        let _ = self.peripheral.disconnect().await;
    }
}
