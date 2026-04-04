use std::sync::Arc;
use std::time::Duration;

use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use crossterm::{
    cursor, event, execute, style,
    terminal::{self, ClearType},
};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;

use crate::speed::SpeedTracker;
use crate::uuid as suuid;

/// Info about a discovered BLE device.
pub struct DeviceInfo {
    pub peripheral: Peripheral,
    pub name: String,
    pub ip: Option<String>,
}

/// Find the BLE adapter.
pub async fn get_adapter() -> Result<Adapter, String> {
    let manager = Manager::new()
        .await
        .map_err(|e| format!("BLE manager init failed: {}", e))?;
    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("No BLE adapters: {}", e))?;
    adapters
        .into_iter()
        .next()
        .ok_or_else(|| "No BLE adapter found".to_string())
}

/// Live scan-and-select: shows devices as they appear, user picks with arrow keys + Enter.
/// Returns the selected DeviceInfo (already connected with services discovered).
pub async fn live_scan_and_select(
    adapter: &Adapter,
    timeout_secs: u64,
) -> Result<DeviceInfo, String> {
    let service_uuid = suuid::parse_uuid(suuid::SERVICE_ID);

    adapter
        .start_scan(ScanFilter {
            services: vec![service_uuid],
        })
        .await
        .map_err(|e| format!("Scan failed: {}", e))?;

    let mut events = adapter
        .events()
        .await
        .map_err(|e| format!("Events failed: {}", e))?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let timeout = Duration::from_secs(timeout_secs);
    let start = tokio::time::Instant::now();

    let mut devices: Vec<(Peripheral, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut selected: usize = 0;
    let mut done = false;

    // Enable raw mode for keyboard input
    terminal::enable_raw_mode().map_err(|e| format!("Raw mode failed: {}", e))?;

    let mut stdout = std::io::stdout();
    let _ = execute!(
        stdout,
        style::Print("  Scanning... (↑↓ select, Enter confirm, q quit)\r\n")
    );

    let mut last_drawn_count: usize = 0;

    while !done {
        if start.elapsed() >= timeout {
            if devices.is_empty() {
                terminal::disable_raw_mode().ok();
                let _ = execute!(stdout, style::Print("\n"));
                let _ = adapter.stop_scan().await;
                return Err("No devices found within timeout".to_string());
            }
            // Auto-select on timeout
            break;
        }

        // Poll for BLE events (non-blocking, 50ms)
        let remaining = deadline - tokio::time::Instant::now();
        let poll_time = remaining.min(Duration::from_millis(50));
        if let Ok(Some(ev)) = tokio::time::timeout(poll_time, events.next()).await {
            if let CentralEvent::DeviceDiscovered(id) = ev {
                if let Ok(p) = adapter.peripheral(&id).await {
                    if let Ok(Some(props)) = p.properties().await {
                        if props.services.contains(&service_uuid) {
                            let id_str = format!("{:?}", id);
                            if seen.insert(id_str) {
                                let name = props
                                    .local_name
                                    .unwrap_or_else(|| "Unknown".to_string());
                                devices.push((p, name));
                                if devices.len() == 1 {
                                    selected = 0;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Poll for keyboard input (non-blocking)
        if event::poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(ev) = event::read() {
                if let event::Event::Key(key) = ev {
                    match key.code {
                        event::KeyCode::Up => {
                            if selected > 0 {
                                selected -= 1;
                            }
                        }
                        event::KeyCode::Down => {
                            if !devices.is_empty() && selected < devices.len() - 1 {
                                selected += 1;
                            }
                        }
                        event::KeyCode::Enter => {
                            if !devices.is_empty() {
                                done = true;
                            }
                        }
                        event::KeyCode::Char('q') | event::KeyCode::Esc => {
                            terminal::disable_raw_mode().ok();
                            let _ = execute!(stdout, style::Print("\n"));
                            let _ = adapter.stop_scan().await;
                            return Err("Cancelled".to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Redraw device list
        redraw_device_list(&mut stdout, &devices, selected, start.elapsed(), timeout, &mut last_drawn_count);
    }

    // Restore terminal
    terminal::disable_raw_mode().ok();
    let _ = execute!(stdout, style::Print("\n"));
    let _ = adapter.stop_scan().await;

    let (peripheral, name) = devices.remove(selected);

    // Disconnect any stale connection first to avoid btleplug SendError
    let _ = tokio::time::timeout(Duration::from_secs(3), peripheral.disconnect()).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    println!("Connecting to {}...", name);

    tokio::time::timeout(Duration::from_secs(15), peripheral.connect())
        .await
        .map_err(|_| format!("Connect to {} timed out (15s)", name))?
        .map_err(|e| format!("Connect failed: {}", e))?;

    if !peripheral.is_connected().await.unwrap_or(false) {
        return Err("Not connected after connect()".to_string());
    }

    println!("Discovering services...");

    tokio::time::timeout(Duration::from_secs(15), peripheral.discover_services())
        .await
        .map_err(|_| "Service discovery timed out (15s)".to_string())?
        .map_err(|e| format!("Service discovery failed: {}", e))?;

    let ip = read_ip(&peripheral).await;
    if let Some(ref ip) = ip {
        println!("📡 Pi IP: {}", ip);
    }

    Ok(DeviceInfo {
        peripheral,
        name,
        ip,
    })
}

fn redraw_device_list(
    stdout: &mut std::io::Stdout,
    devices: &[(Peripheral, String)],
    selected: usize,
    elapsed: Duration,
    timeout: Duration,
    last_drawn_count: &mut usize,
) {
    let remaining = timeout.saturating_sub(elapsed).as_secs();

    // Move cursor up to overwrite previous list (header + previous device lines)
    let lines_to_clear = *last_drawn_count + 1;
    for _ in 0..lines_to_clear {
        let _ = execute!(
            stdout,
            cursor::MoveUp(1),
            terminal::Clear(ClearType::CurrentLine),
        );
    }

    // Header
    let _ = execute!(
        stdout,
        style::Print(format!(
            "  Scanning... {}s remaining (↑↓ select, Enter confirm, q quit)\r\n",
            remaining
        ))
    );

    // Device list
    for (i, (_p, name)) in devices.iter().enumerate() {
        let marker = if i == selected { "▸" } else { " " };
        let highlight = if i == selected { " ◀" } else { "" };
        let _ = execute!(
            stdout,
            style::Print(format!("  {} 📱 {}{}\r\n", marker, name, highlight))
        );
    }

    *last_drawn_count = devices.len();
}

/// Find a characteristic by UUID on a connected peripheral.
fn find_char(chars: &[Characteristic], uuid_hex: &str) -> Result<Characteristic, String> {
    let target = suuid::parse_uuid(uuid_hex);
    chars
        .iter()
        .find(|c| c.uuid == target)
        .cloned()
        .ok_or_else(|| format!("Characteristic {} not found", uuid_hex))
}

/// Read the IP_ADDRESS characteristic to get the Pi's IP.
pub async fn read_ip(peripheral: &Peripheral) -> Option<String> {
    let chars = peripheral.characteristics();
    let chars_vec: Vec<_> = chars.into_iter().collect();
    if let Ok(c) = find_char(&chars_vec, suuid::IP_ADDRESS) {
        if let Ok(Ok(data)) =
            tokio::time::timeout(Duration::from_secs(5), peripheral.read(&c)).await
        {
            let ip = String::from_utf8_lossy(&data).trim().to_string();
            if !ip.is_empty() {
                return Some(ip);
            }
        }
    }
    None
}

/// Manages the BLE SSH tunnel connection with a persistent notification pump.
/// The notification stream lives entirely inside a background task to avoid
/// btleplug stream issues when crossing tokio task boundaries on macOS.
pub struct BleTunnel {
    peripheral: Peripheral,
    ssh_ctrl: Characteristic,
    ssh_rx: Characteristic,
    speed: Arc<SpeedTracker>,
    /// Control messages (OK, CLOSED, ERR:...) from the persistent pump.
    ctrl_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    /// SSH data from the persistent pump.
    data_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
}

impl BleTunnel {
    /// Create from an already-connected peripheral with services discovered.
    /// Subscribes to SSH characteristics and starts a persistent notification pump.
    pub async fn from_connected(peripheral: Peripheral, speed: Arc<SpeedTracker>) -> Result<Self, String> {
        let chars: Vec<_> = peripheral.characteristics().into_iter().collect();
        log::info!("Found {} characteristics", chars.len());
        let ssh_ctrl = find_char(&chars, suuid::SSH_CTRL)?;
        let ssh_rx = find_char(&chars, suuid::SSH_RX)?;
        let ssh_tx = find_char(&chars, suuid::SSH_TX)?;

        // Subscribe once (persistent for the lifetime of the BLE connection)
        peripheral
            .subscribe(&ssh_ctrl)
            .await
            .map_err(|e| format!("Subscribe CTRL: {}", e))?;
        peripheral
            .subscribe(&ssh_tx)
            .await
            .map_err(|e| format!("Subscribe TX: {}", e))?;

        let (ctrl_tx, ctrl_rx) = mpsc::channel::<String>(16);
        let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>(512);

        // Clone peripheral for the pump task — notification stream is created
        // AND consumed entirely inside this task (never crosses task boundaries).
        let p = peripheral.clone();
        let ctrl_uuid = ssh_ctrl.uuid;
        let tx_uuid = ssh_tx.uuid;
        let spd = speed.clone();

        tokio::spawn(async move {
            let mut stream = match p.notifications().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Notification stream failed: {}", e);
                    return;
                }
            };
            log::info!("Persistent BLE notification pump started");
            let mut data_count: u64 = 0;
            let mut ctrl_count: u64 = 0;
            let mut other_count: u64 = 0;
            while let Some(notif) = stream.next().await {
                if notif.uuid == tx_uuid {
                    data_count += 1;
                    let hex_preview: String = notif.value.iter().take(20)
                        .map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
                    log::debug!(
                        "PUMP TX #{}: {} bytes, hex: [{}]",
                        data_count, notif.value.len(), hex_preview
                    );
                    if data_count <= 3 {
                        // Log text preview for the first few messages (might be SSH banner)
                        let text = String::from_utf8_lossy(&notif.value[..notif.value.len().min(80)]);
                        log::info!("PUMP TX #{}: {} bytes, text: {:?}", data_count, notif.value.len(), text);
                    }
                    spd.add_rx(notif.value.len() as u64);
                    if data_tx.send(notif.value).await.is_err() {
                        log::info!("Data channel closed, pump ending");
                        break;
                    }
                } else if notif.uuid == ctrl_uuid {
                    ctrl_count += 1;
                    let msg = String::from_utf8_lossy(&notif.value).to_string();
                    log::info!("PUMP CTRL #{}: {:?}", ctrl_count, msg);
                    if ctrl_tx.send(msg).await.is_err() {
                        log::info!("Ctrl channel closed, pump ending");
                        break;
                    }
                } else {
                    other_count += 1;
                    if other_count <= 5 {
                        log::debug!("PUMP OTHER: uuid={}, {} bytes", notif.uuid, notif.value.len());
                    }
                }
            }
            log::info!("Persistent BLE notification pump ended (tx={}, ctrl={}, other={})", data_count, ctrl_count, other_count);
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
    /// Drains any stale data in the channels first.
    pub async fn open_tunnel(&self) -> Result<(), String> {
        // Drain stale data from previous connections
        {
            let mut data = self.data_rx.lock().await;
            let mut drained = 0;
            while data.try_recv().is_ok() {
                drained += 1;
            }
            if drained > 0 {
                log::warn!("Drained {} stale data messages before CONNECT", drained);
            }
        }
        {
            let mut ctrl = self.ctrl_rx.lock().await;
            let mut drained = 0;
            while ctrl.try_recv().is_ok() {
                drained += 1;
            }
            if drained > 0 {
                log::warn!("Drained {} stale ctrl messages before CONNECT", drained);
            }
        }

        self.peripheral
            .write(&self.ssh_ctrl, b"CONNECT", WriteType::WithResponse)
            .await
            .map_err(|e| format!("Write CONNECT: {}", e))?;

        let mut ctrl = self.ctrl_rx.lock().await;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err("Timeout waiting for CONNECT response".to_string());
            }
            match tokio::time::timeout(remaining, ctrl.recv()).await {
                Ok(Some(msg)) if msg == "OK" => {
                    log::info!("Tunnel opened (server OK)");
                    return Ok(());
                }
                Ok(Some(msg)) if msg.starts_with("ERR:") => {
                    return Err(format!("Server error: {}", msg));
                }
                Ok(Some(msg)) => {
                    log::debug!("Ignoring ctrl message during handshake: {}", msg);
                    continue;
                }
                Ok(None) => return Err("Control channel closed".to_string()),
                Err(_) => return Err("Timeout waiting for CONNECT response".to_string()),
            }
        }
    }

    /// Receive SSH data from the persistent pump.
    pub async fn recv_data(&self) -> Option<Vec<u8>> {
        self.data_rx.lock().await.recv().await
    }

    /// Check for a control message (non-blocking).
    pub async fn try_recv_ctrl(&self) -> Option<String> {
        self.ctrl_rx.lock().await.try_recv().ok()
    }

    /// Send DISCONNECT and wait for CLOSED response.
    pub async fn close_tunnel(&self) {
        let _ = self
            .peripheral
            .write(&self.ssh_ctrl, b"DISCONNECT", WriteType::WithResponse)
            .await;
        // Wait briefly for CLOSED acknowledgment
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
        let mtu = 512;
        for chunk in data.chunks(mtu) {
            self.peripheral
                .write(&self.ssh_rx, chunk, WriteType::WithResponse)
                .await
                .map_err(|e| format!("Write failed: {}", e))?;
            self.speed.add_tx(chunk.len() as u64);
        }
        Ok(())
    }

    pub async fn disconnect(&self) {
        let _ = self.peripheral.disconnect().await;
    }
}
