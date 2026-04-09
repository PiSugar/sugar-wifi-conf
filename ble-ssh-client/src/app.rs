use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use btleplug::api::{Central as _, Peripheral as _};
use btleplug::platform::{Adapter, Peripheral};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, MenuId, PredefinedMenuItem, Submenu};
use tray_icon::TrayIconBuilder;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::aliases::DeviceAliases;
use crate::ble::{self, BleTunnel};
use crate::speed::SpeedTracker;
use crate::tunnel;

// ── Platform helpers ──────────────────────────────────

#[cfg(target_os = "macos")]
fn open_ssh_terminal(user: &str, port: u16) {
    let ssh_cmd = format!(
        "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -p {} {}@localhost",
        port, user
    );
    let script = format!(
        "tell application \"Terminal\"\n  activate\n  do script \"{}\"\nend tell",
        ssh_cmd
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn();
}

#[cfg(target_os = "windows")]
fn open_ssh_terminal(user: &str, port: u16) {
    let ssh_cmd = format!(
        "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -p {} {}@localhost",
        port, user
    );
    // Open Windows Terminal (wt) or fall back to cmd
    let wt = std::process::Command::new("wt")
        .args(["new-tab", "cmd", "/K", &ssh_cmd])
        .spawn();
    if wt.is_err() {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "cmd", "/K", &ssh_cmd])
            .spawn();
    }
}

#[cfg(target_os = "linux")]
fn open_ssh_terminal(user: &str, port: u16) {
    let ssh_cmd = format!(
        "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -p {} {}@localhost",
        port, user
    );
    // Try common terminal emulators in order of preference
    let terminals: &[(&str, &[&str])] = &[
        ("x-terminal-emulator", &["-e", &ssh_cmd]),
        ("gnome-terminal", &["--", "sh", "-c", &ssh_cmd]),
        ("konsole", &["-e", "sh", "-c", &ssh_cmd]),
        ("xfce4-terminal", &["-e", &ssh_cmd]),
        ("xterm", &["-e", &ssh_cmd]),
    ];
    for (term, args) in terminals {
        if std::process::Command::new(term).args(*args).spawn().is_ok() {
            return;
        }
    }
    log::warn!("No supported terminal emulator found");
}

#[cfg(target_os = "macos")]
fn copy_to_clipboard(text: &str) {
    let _ = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });
}

#[cfg(target_os = "windows")]
fn copy_to_clipboard(text: &str) {
    let _ = std::process::Command::new("clip")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        });
}

#[cfg(target_os = "linux")]
fn copy_to_clipboard(text: &str) {
    // Try xclip first, then xsel, then wl-copy (Wayland)
    let tools: &[(&str, &[&str])] = &[
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
        ("wl-copy", &[]),
    ];
    for (tool, args) in tools {
        if let Ok(mut child) = std::process::Command::new(tool)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return;
        }
    }
    log::warn!("No clipboard tool found (install xclip, xsel, or wl-copy)");
}

// ── Prompt for device alias ───────────────────────────

#[cfg(target_os = "macos")]
fn prompt_alias(prompt: &str, default: &str) -> Option<String> {
    let script = format!(
        "set r to display dialog \"{}\" default answer \"{}\" buttons {{\"Cancel\", \"OK\"}} default button \"OK\"\n\
         return text returned of r",
        prompt.replace('"', "\\\""),
        default.replace('"', "\\\""),
    );
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn prompt_alias(prompt: &str, default: &str) -> Option<String> {
    let ps_script = format!(
        "Add-Type -AssemblyName Microsoft.VisualBasic; \
         [Microsoft.VisualBasic.Interaction]::InputBox('{}', 'Set Alias', '{}')",
        prompt.replace('\'', "''"),
        default.replace('\'', "''"),
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .ok()?;
    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if result.is_empty() && !default.is_empty() {
            // User pressed Cancel
            None
        } else {
            Some(result)
        }
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn prompt_alias(prompt: &str, default: &str) -> Option<String> {
    // Try zenity first, then kdialog
    let dialogs: &[(&str, Vec<String>)] = &[
        ("zenity", vec![
            "--entry".to_string(),
            format!("--text={}", prompt),
            format!("--entry-text={}", default),
            "--title=Set Alias".to_string(),
        ]),
        ("kdialog", vec![
            "--inputbox".to_string(),
            prompt.to_string(),
            default.to_string(),
        ]),
    ];
    for (cmd, args) in dialogs {
        if let Ok(output) = std::process::Command::new(cmd).args(args).output() {
            if output.status.success() {
                return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            } else {
                return None; // User cancelled
            }
        }
    }
    log::warn!("No dialog tool found (install zenity or kdialog)");
    None
}

// ── Messages ──────────────────────────────────────────

/// From async workers → UI thread.
#[derive(Debug, Clone)]
pub enum UiMsg {
    DeviceFound { id: String, name: String },
    SshUser { id: String, user: String },
    Connecting { id: String },
    TunnelUp { id: String, port: u16 },
    TunnelDown { id: String },
    ScanDone,
    Err(String),
}

/// From UI → async workers.
pub enum Cmd {
    Scan,
    Connect(String),
    Disconnect(String),
    Quit,
}

// ── App State ─────────────────────────────────────────

struct DeviceInfo {
    name: String,
    ssh_user: Option<String>,
    connecting: bool,
    tunnel_port: Option<u16>,
}

struct AppState {
    devices: Vec<(String, DeviceInfo)>,
    scanning: bool,
    aliases: DeviceAliases,
}

impl AppState {
    fn new() -> Self {
        Self {
            devices: Vec::new(),
            scanning: true,
            aliases: DeviceAliases::load(),
        }
    }

    fn display_name(&self, id: &str, info: &DeviceInfo) -> String {
        if let Some(alias) = self.aliases.get(id) {
            alias.to_string()
        } else {
            info.name.clone()
        }
    }
}

// ── Cleanup State (shared for signal/panic handlers) ──

struct CleanupState {
    adapter: Option<Adapter>,
    peripherals: Vec<Peripheral>,
    rt: Option<Arc<tokio::runtime::Runtime>>,
}

static CLEANUP: once_cell::sync::Lazy<Mutex<CleanupState>> =
    once_cell::sync::Lazy::new(|| {
        Mutex::new(CleanupState {
            adapter: None,
            peripherals: Vec::new(),
            rt: None,
        })
    });

/// Best-effort cleanup: stop scanning and disconnect all known peripherals.
/// Safe to call from inside or outside a tokio runtime.
fn do_cleanup() {
    let mut state = match CLEANUP.lock() {
        Ok(s) => s,
        Err(poisoned) => poisoned.into_inner(),
    };
    let rt = match state.rt.take() {
        Some(rt) => rt,
        None => return,
    };
    let adapter = state.adapter.take();
    let peripherals: Vec<Peripheral> = state.peripherals.drain(..).collect();
    drop(state); // release the lock before blocking

    // If we're already inside a tokio runtime (e.g. panic in an async task),
    // we can't call block_on directly — run cleanup on a dedicated thread.
    let cleanup = move || {
        if let Some(adapter) = adapter {
            let _ = rt.block_on(adapter.stop_scan());
            log::info!("[cleanup] stopped BLE scan");
        }
        for p in peripherals {
            if rt.block_on(async { p.is_connected().await.unwrap_or(false) }) {
                let _ = rt.block_on(p.disconnect());
                log::info!("[cleanup] disconnected peripheral");
            }
        }
    };

    if tokio::runtime::Handle::try_current().is_ok() {
        // We're inside a runtime — spawn a thread so block_on doesn't panic
        let handle = std::thread::spawn(cleanup);
        let _ = handle.join();
    } else {
        cleanup();
    }
}

// ── Entry Point ───────────────────────────────────────

pub fn run() {
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime"),
    );

    // Store runtime in cleanup state so signal/panic handlers can use it
    CLEANUP.lock().unwrap().rt = Some(rt.clone());

    // Register ctrlc (SIGINT/SIGTERM) handler
    ctrlc::set_handler(move || {
        log::info!("Signal received, cleaning up...");
        do_cleanup();
        std::process::exit(0);
    })
    .expect("Failed to set signal handler");

    // Register panic hook
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("Panic: {}", info);
        do_cleanup();
        default_hook(info);
    }));

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Cmd>();

    // Shared message queue: worker pushes, UI polls
    let ui_queue: Arc<Mutex<VecDeque<UiMsg>>> = Arc::new(Mutex::new(VecDeque::new()));

    // Spawn async worker
    let q = ui_queue.clone();
    rt.spawn(async move {
        worker(q, cmd_rx).await;
    });

    // Create tray icon with emoji title (no bitmap icon needed)
    let menu = build_menu(&AppState::new());

    // Need a minimal 1x1 transparent icon since tray-icon requires one
    let icon = tray_icon::Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).expect("icon");

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("PiSugar BLE-SSH")
        .with_icon(icon)
        .with_title(tray_title(&AppState::new()))
        .build()
        .expect("Failed to create tray icon");

    let mut state = AppState::new();

    // Start initial scan
    let _ = cmd_tx.send(Cmd::Scan);

    event_loop.run(move |_event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100));

        // Drain all messages from the shared queue
        let msgs: Vec<UiMsg> = {
            let mut q = ui_queue.lock().unwrap();
            q.drain(..).collect()
        };

        let mut menu_dirty = false;
        for msg in msgs {
            match msg {
                UiMsg::DeviceFound { id, name } => {
                    if !state.devices.iter().any(|(did, _)| *did == id) {
                        log::info!("[UI] Device found: {} ({})", name, id);
                        state.devices.push((
                            id,
                            DeviceInfo {
                                name,
                                ssh_user: None,
                                connecting: false,
                                tunnel_port: None,
                            },
                        ));
                        menu_dirty = true;
                    }
                }
                UiMsg::SshUser { id, user } => {
                    if let Some((_, info)) = state.devices.iter_mut().find(|(did, _)| *did == id) {
                        log::info!("SSH user for {}: {}", info.name, user);
                        info.ssh_user = Some(user);
                        menu_dirty = true;
                    }
                }
                UiMsg::Connecting { id } => {
                    if let Some((_, info)) = state.devices.iter_mut().find(|(did, _)| *did == id) {
                        log::info!("Connecting: {}", info.name);
                        info.connecting = true;
                        menu_dirty = true;
                    }
                }
                UiMsg::TunnelUp { id, port } => {
                    if let Some((_, info)) = state.devices.iter_mut().find(|(did, _)| *did == id) {
                        log::info!("Tunnel up: {} → localhost:{}", info.name, port);
                        info.connecting = false;
                        info.tunnel_port = Some(port);
                        menu_dirty = true;
                    }
                }
                UiMsg::TunnelDown { id } => {
                    if let Some((_, info)) = state.devices.iter_mut().find(|(did, _)| *did == id) {
                        log::info!("Tunnel down: {}", info.name);
                        info.connecting = false;
                        info.tunnel_port = None;
                        menu_dirty = true;
                    }
                }
                UiMsg::ScanDone => {
                    log::info!("[UI] Scan done, {} devices", state.devices.len());
                    state.scanning = false;
                    menu_dirty = true;
                }
                UiMsg::Err(e) => {
                    log::error!("{}", e);
                }
            }
        }
        if menu_dirty {
            let menu = build_menu(&state);
            tray.set_menu(Some(Box::new(menu)));
            tray.set_title(Some(tray_title(&state)));
        }

        // Handle menu clicks
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            let eid = &event.id;

            if *eid == MenuId::new("quit") {
                let _ = cmd_tx.send(Cmd::Quit);
                *control_flow = ControlFlow::Exit;
            } else if *eid == MenuId::new("rescan") {
                state.devices.retain(|(_, info)| info.tunnel_port.is_some());
                state.scanning = true;
                let menu = build_menu(&state);
                tray.set_menu(Some(Box::new(menu)));
                tray.set_title(Some(tray_title(&state)));
                let _ = cmd_tx.send(Cmd::Scan);
            } else {
                let eid_str = eid.0.as_str();
                if let Some(dev_id) = eid_str.strip_prefix("ssh:") {
                    // Open SSH in a terminal window
                    if let Some((_, info)) = state.devices.iter().find(|(did, _)| *did == dev_id) {
                        if let Some(port) = info.tunnel_port {
                            let user = info.ssh_user.as_deref().unwrap_or("pi");
                            open_ssh_terminal(user, port);
                        }
                    }
                } else if let Some(dev_id) = eid_str.strip_prefix("copy:") {
                    // Copy SSH command to clipboard
                    if let Some((_, info)) = state.devices.iter().find(|(did, _)| *did == dev_id) {
                        if let Some(port) = info.tunnel_port {
                            let user = info.ssh_user.as_deref().unwrap_or("pi");
                            let cmd = format!("ssh -p {} {}@localhost", port, user);
                            copy_to_clipboard(&cmd);
                        }
                    }
                } else if let Some(dev_id) = eid_str.strip_prefix("dc:") {
                    // Disconnect
                    let _ = cmd_tx.send(Cmd::Disconnect(dev_id.to_string()));
                } else if let Some(dev_id) = eid_str.strip_prefix("connect:") {
                    // Connect from submenu
                    let _ = cmd_tx.send(Cmd::Connect(dev_id.to_string()));
                } else if let Some(dev_id) = eid_str.strip_prefix("alias:") {
                    // Set alias via platform dialog
                    let dev_id = dev_id.to_string();
                    let current = state.aliases.get(&dev_id).unwrap_or("").to_string();
                    let dev_name = state.devices.iter()
                        .find(|(did, _)| *did == dev_id)
                        .map(|(_, info)| info.name.as_str())
                        .unwrap_or("device");
                    let prompt_msg = format!("Enter alias for {} (leave empty to clear):", dev_name);
                    if let Some(alias) = prompt_alias(&prompt_msg, &current) {
                        let alias = alias.trim().to_string();
                        if alias.is_empty() {
                            state.aliases.remove(&dev_id);
                        } else {
                            state.aliases.set(&dev_id, &alias);
                        }
                        let menu = build_menu(&state);
                        tray.set_menu(Some(Box::new(menu)));
                    }
                }
            }
        }
    });
}

fn tray_title(state: &AppState) -> String {
    let has_connecting = state.devices.iter().any(|(_, i)| i.connecting);
    let suffix = if state.scanning {
        " Scanning…"
    } else if has_connecting {
        " Connecting…"
    } else {
        ""
    };
    if state.devices.is_empty() {
        format!("📡{}", suffix)
    } else {
        let connected = state.devices.iter().filter(|(_, i)| i.tunnel_port.is_some()).count();
        let total = state.devices.len();
        format!("📡 {}/{}{}", connected, total, suffix)
    }
}

// ── Menu Builder ──────────────────────────────────────

fn build_menu(state: &AppState) -> Menu {
    let menu = Menu::new();

    if state.scanning && state.devices.is_empty() {
        let _ = menu.append(&MenuItem::with_id(
            MenuId::new("_scanning"),
            "🔍 Scanning...",
            false,
            None,
        ));
    }

    if !state.devices.is_empty() {
        let _ = menu.append(&PredefinedMenuItem::separator());
    }

    // Connected devices first, then connecting, then disconnected
    let sorted: Vec<_> = {
        let mut connected: Vec<_> = state.devices.iter().filter(|(_, i)| i.tunnel_port.is_some()).collect();
        let mut connecting: Vec<_> = state.devices.iter().filter(|(_, i)| i.connecting && i.tunnel_port.is_none()).collect();
        let mut disconnected: Vec<_> = state.devices.iter().filter(|(_, i)| !i.connecting && i.tunnel_port.is_none()).collect();
        connected.append(&mut connecting);
        connected.append(&mut disconnected);
        connected
    };

    for (id, info) in &sorted {
        let display = state.display_name(id, info);
        if let Some(port) = info.tunnel_port {
            // Connected: submenu with actions
            let user = info.ssh_user.as_deref().unwrap_or("pi");
            let text = format!("✓ {} → {}@localhost:{}", display, user, port);
            let ssh_label = format!("Open SSH ({}@localhost -p {})", user, port);
            let alias_label = if state.aliases.get(id).is_some() {
                "Rename"
            } else {
                "Set Alias"
            };
            let sub = Submenu::with_id_and_items(
                MenuId::new(&format!("sub:{}", id)),
                &text,
                true,
                &[
                    &MenuItem::with_id(
                        MenuId::new(&format!("ssh:{}", id)),
                        &ssh_label,
                        true,
                        None,
                    ),
                    &MenuItem::with_id(
                        MenuId::new(&format!("copy:{}", id)),
                        "Copy SSH Command",
                        true,
                        None,
                    ),
                    &PredefinedMenuItem::separator(),
                    &MenuItem::with_id(
                        MenuId::new(&format!("alias:{}", id)),
                        alias_label,
                        true,
                        None,
                    ),
                    &PredefinedMenuItem::separator(),
                    &MenuItem::with_id(
                        MenuId::new(&format!("dc:{}", id)),
                        "Disconnect",
                        true,
                        None,
                    ),
                ],
            )
            .unwrap();
            let _ = menu.append(&sub);
        } else if info.connecting {
            // Connecting: disabled submenu
            let text = format!("{} ⏳", display);
            let sub = Submenu::with_id_and_items(
                MenuId::new(&format!("sub:{}", id)),
                &text,
                true,
                &[
                    &MenuItem::with_id(
                        MenuId::new("_info"),
                        "Connecting...",
                        false,
                        None,
                    ),
                ],
            )
            .unwrap();
            let _ = menu.append(&sub);
        } else {
            // Disconnected: submenu with Connect + Set Alias
            let alias_label = if state.aliases.get(id).is_some() {
                "Rename"
            } else {
                "Set Alias"
            };
            let sub = Submenu::with_id_and_items(
                MenuId::new(&format!("sub:{}", id)),
                &display,
                true,
                &[
                    &MenuItem::with_id(
                        MenuId::new(&format!("connect:{}", id)),
                        "Connect",
                        true,
                        None,
                    ),
                    &PredefinedMenuItem::separator(),
                    &MenuItem::with_id(
                        MenuId::new(&format!("alias:{}", id)),
                        alias_label,
                        true,
                        None,
                    ),
                ],
            )
            .unwrap();
            let _ = menu.append(&sub);
        }
    }

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(
        MenuId::new("rescan"),
        "🔄 Rescan",
        !state.scanning,
        None,
    ));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(
        MenuId::new("quit"),
        "Quit",
        true,
        None,
    ));

    menu
}

// ── Async Worker ──────────────────────────────────────

#[allow(dead_code)]
struct ConnectedDevice {
    tunnel: Arc<BleTunnel>,
    proxy_handle: JoinHandle<()>,
    port: u16,
}

/// Helper to push a message onto the shared UI queue
fn send_ui(q: &Arc<Mutex<VecDeque<UiMsg>>>, msg: UiMsg) {
    q.lock().unwrap().push_back(msg);
}

async fn worker(ui_q: Arc<Mutex<VecDeque<UiMsg>>>, mut cmd_rx: mpsc::UnboundedReceiver<Cmd>) {
    let adapter = match ble::get_adapter().await {
        Ok(a) => a,
        Err(e) => {
            send_ui(&ui_q, UiMsg::Err(format!("BLE: {}", e)));
            // Keep running so Quit still works
            while let Some(cmd) = cmd_rx.recv().await {
                if matches!(cmd, Cmd::Quit) {
                    break;
                }
            }
            return;
        }
    };

    // Register adapter in cleanup state for signal/panic handlers
    CLEANUP.lock().unwrap().adapter = Some(adapter.clone());

    let mut peripherals: HashMap<String, Peripheral> = HashMap::new();
    let mut connections: HashMap<String, ConnectedDevice> = HashMap::new();
    let mut next_port: u16 = 2222;

    // Channel for scan results (non-blocking scan)
    let (scan_tx, mut scan_rx) = mpsc::unbounded_channel::<ScanResult>();

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    Cmd::Scan => {
                        let q = ui_q.clone();
                        let stx = scan_tx.clone();
                        let scan_adapter = adapter.clone();
                        tokio::spawn(async move {
                            do_scan(scan_adapter, stx, q).await;
                        });
                    }
                    Cmd::Disconnect(id) => {
                        if let Some(conn) = connections.remove(&id) {
                            conn.proxy_handle.abort();
                            conn.tunnel.close_tunnel().await;
                            conn.tunnel.disconnect().await;
                            send_ui(&ui_q, UiMsg::TunnelDown { id });
                        }
                    }
                    Cmd::Connect(id) => {
                        if let Some(peripheral) = peripherals.get(&id).cloned() {
                            // Connect — send Connecting state first
                            send_ui(&ui_q, UiMsg::Connecting { id: id.clone() });

                            let speed = SpeedTracker::new();
                            let q = ui_q.clone();
                            let id2 = id.clone();

                            // Spawn in a separate task so btleplug panics
                            // (known CoreBluetooth race) don't crash the app.
                            // Always do fresh ble_connect right before from_connected
                            // in one shot — btleplug on macOS only delivers notifications
                            // when subscribe+pump happen in the same task context
                            // as the connection.
                            let conn_result = tokio::spawn(async move {
                                ble::ble_connect(&peripheral).await?;

                                // Read SSH username while connected
                                if let Some(user) = ble::read_ssh_username(&peripheral).await {
                                    send_ui(&q, UiMsg::SshUser { id: id2, user });
                                }

                                let tunnel = BleTunnel::from_connected(peripheral.clone(), speed.clone()).await?;
                                let tunnel = Arc::new(tunnel);
                                Ok::<_, String>((tunnel, speed))
                            }).await;

                            match conn_result {
                                Ok(Ok((tunnel, speed))) => {
                                    let port = next_port;
                                    next_port += 1;

                                    let tun = tunnel.clone();
                                    let spd = speed.clone();
                                    let handle = tokio::spawn(async move {
                                        if let Err(e) = tunnel::run_device_proxy(port, tun, spd).await {
                                            log::error!("Proxy on port {}: {}", port, e);
                                        }
                                    });

                                    connections.insert(id.clone(), ConnectedDevice {
                                        tunnel,
                                        proxy_handle: handle,
                                        port,
                                    });
                                    send_ui(&ui_q, UiMsg::TunnelUp { id, port });
                                }
                                Ok(Err(e)) => {
                                    send_ui(&ui_q, UiMsg::Err(format!("Connect: {}", e)));
                                    send_ui(&ui_q, UiMsg::TunnelDown { id });
                                }
                                Err(join_err) => {
                                    // Task panicked (btleplug CoreBluetooth race)
                                    log::error!("Connect task panicked: {}", join_err);
                                    send_ui(&ui_q, UiMsg::Err(format!("Connect: internal BLE error")));
                                    send_ui(&ui_q, UiMsg::TunnelDown { id });
                                }
                            }
                        }
                    }
                    Cmd::Quit => {
                        log::info!("Quit: cleaning up...");
                        let _ = adapter.stop_scan().await;
                        for (_, conn) in connections.drain() {
                            conn.proxy_handle.abort();
                            conn.tunnel.close_tunnel().await;
                            conn.tunnel.disconnect().await;
                        }
                        for (_, p) in peripherals.drain() {
                            if p.is_connected().await.unwrap_or(false) {
                                let _ = p.disconnect().await;
                            }
                        }
                        break;
                    }
                }
            }
            Some(result) = scan_rx.recv() => {
                match result {
                    ScanResult::Device { id, name: _, peripheral } => {
                        // Register peripheral in cleanup state
                        CLEANUP.lock().unwrap().peripherals.push(peripheral.clone());
                        peripherals.insert(id.clone(), peripheral);
                    }
                    ScanResult::Done => {}
                }
            }
        }
    }
}

#[allow(dead_code)]
enum ScanResult {
    Device {
        id: String,
        name: String,
        peripheral: Peripheral,
    },
    Done,
}

async fn do_scan(
    adapter: btleplug::platform::Adapter,
    result_tx: mpsc::UnboundedSender<ScanResult>,
    ui_q: Arc<Mutex<VecDeque<UiMsg>>>,
) {
    let (dev_tx, mut dev_rx) = mpsc::unbounded_channel::<ble::ScannedDevice>();

    // Spawn the BLE scan; it sends devices through dev_tx as they're found
    let scan_adapter = adapter.clone();
    let scan_ui_q = ui_q.clone();
    tokio::spawn(async move {
        if let Err(e) = ble::scan_devices(&scan_adapter, Duration::from_secs(30), dev_tx).await {
            send_ui(&scan_ui_q, UiMsg::Err(format!("Scan: {}", e)));
        }
    });

    // Process each device immediately as it streams in.
    // Do NOT connect during scan — btleplug on macOS cannot reliably
    // reconnect to a peripheral after disconnect, and notifications
    // only work when connect+subscribe happen in the same task.
    // SSH username will be read during Cmd::Connect instead.
    while let Some(dev) = dev_rx.recv().await {
        let id = dev.id.clone();
        let name = dev.name.clone();
        send_ui(&ui_q, UiMsg::DeviceFound {
            id: id.clone(),
            name: name.clone(),
        });
        let _ = result_tx.send(ScanResult::Device {
            id: id.clone(),
            name,
            peripheral: dev.peripheral.clone(),
        });
    }

    let _ = result_tx.send(ScanResult::Done);
    send_ui(&ui_q, UiMsg::ScanDone);
}
