use std::sync::Arc;

use bluer::gatt::local::{
    Characteristic, CharacteristicNotify, CharacteristicNotifyMethod, CharacteristicWrite,
    CharacteristicWriteMethod, CharacteristicWriteRequest,
};
use futures::FutureExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, watch, Mutex};

use crate::uuid as suuid;

/// Shared state for the SSH tunnel connection.
struct TunnelState {
    /// TCP write half to sshd.
    tcp_tx: Option<tokio::io::WriteHalf<TcpStream>>,
    /// Sender to stop the TCP read task.
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Sender to stop the TCP write task.
    write_shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Build all 3 SSH tunnel characteristics.
/// Returns (ssh_ctrl, ssh_rx, ssh_tx).
pub fn build() -> Vec<Characteristic> {
    // Channel: TCP read data → BLE TX notify (broadcast so each subscriber
    // gets its own independent stream — no lock contention between old/new subscribers)
    let (ble_tx_sender, _) = broadcast::channel::<Vec<u8>>(512);
    let ble_tx_sender = Arc::new(ble_tx_sender);

    // Channel: BLE RX write → TCP write
    let (tcp_write_tx, tcp_write_rx) = mpsc::channel::<Vec<u8>>(256);
    let tcp_write_rx = Arc::new(Mutex::new(tcp_write_rx));

    // Shared tunnel state
    let tunnel = Arc::new(Mutex::new(TunnelState {
        tcp_tx: None,
        shutdown_tx: None,
        write_shutdown_tx: None,
    }));

    // Channel for control responses
    let (ctrl_notify_tx, ctrl_notify_rx) = watch::channel::<String>(String::new());
    let ctrl_notify_tx = Arc::new(ctrl_notify_tx);

    vec![
        build_ctrl(
            tunnel.clone(),
            ble_tx_sender.clone(),
            tcp_write_rx,
            ctrl_notify_tx.clone(),
            ctrl_notify_rx,
        ),
        build_rx(tcp_write_tx),
        build_tx(ble_tx_sender.clone()),
    ]
}

/// SSH_CTRL characteristic: write+notify.
/// Write "CONNECT" to open tunnel, "DISCONNECT" to close.
/// Notifies "OK", "ERR:reason", "CLOSED".
fn build_ctrl(
    tunnel: Arc<Mutex<TunnelState>>,
    ble_tx_sender: Arc<broadcast::Sender<Vec<u8>>>,
    tcp_write_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    ctrl_notify_tx: Arc<watch::Sender<String>>,
    ctrl_notify_rx: watch::Receiver<String>,
) -> Characteristic {
    let tunnel_write = tunnel.clone();
    let ctrl_tx_write = ctrl_notify_tx.clone();

    Characteristic {
        uuid: suuid::parse_uuid(suuid::SSH_CTRL),
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(
                move |data: Vec<u8>, _req: CharacteristicWriteRequest| {
                    let tunnel = tunnel_write.clone();
                    let ble_tx_sender = ble_tx_sender.clone();
                    let tcp_write_rx = tcp_write_rx.clone();
                    let ctrl_tx = ctrl_tx_write.clone();
                    async move {
                        let cmd = String::from_utf8_lossy(&data).to_string();
                        let cmd = cmd.trim();
                        log::info!("SSH_CTRL command: {}", cmd);

                        match cmd {
                            "CONNECT" => {
                                // Close existing tunnel if any
                                {
                                    let mut state = tunnel.lock().await;
                                    if let Some(shutdown) = state.shutdown_tx.take() {
                                        let _ = shutdown.send(());
                                    }
                                    if let Some(shutdown) = state.write_shutdown_tx.take() {
                                        let _ = shutdown.send(());
                                    }
                                    state.tcp_tx = None;
                                }

                                // Connect to local sshd
                                match TcpStream::connect("127.0.0.1:22").await {
                                    Ok(stream) => {
                                        log::info!("SSH tunnel: connected to sshd");
                                        let (tcp_read, tcp_write) =
                                            tokio::io::split(stream);

                                        let (shutdown_tx, shutdown_rx) =
                                            tokio::sync::oneshot::channel();
                                        let (write_shutdown_tx, write_shutdown_rx) =
                                            tokio::sync::oneshot::channel();

                                        {
                                            let mut state = tunnel.lock().await;
                                            state.tcp_tx = Some(tcp_write);
                                            state.shutdown_tx = Some(shutdown_tx);
                                            state.write_shutdown_tx = Some(write_shutdown_tx);
                                        }

                                        // Spawn: TCP read → BLE TX
                                        let tx = ble_tx_sender.clone();
                                        let ctrl_tx2 = ctrl_tx.clone();
                                        tokio::spawn(tcp_read_task(
                                            tcp_read,
                                            tx,
                                            ctrl_tx2,
                                            shutdown_rx,
                                        ));

                                        // Spawn: BLE RX → TCP write
                                        let tunnel2 = tunnel.clone();
                                        let ctrl_tx3 = ctrl_tx.clone();
                                        let rx = tcp_write_rx.clone();
                                        tokio::spawn(tcp_write_task(
                                            tunnel2, rx, ctrl_tx3, write_shutdown_rx,
                                        ));

                                        log::info!("SSH tunnel: sending OK");
                                        let _ = ctrl_tx.send("OK".to_string());
                                    }
                                    Err(e) => {
                                        log::error!("SSH tunnel: failed to connect: {}", e);
                                        let _ = ctrl_tx
                                            .send(format!("ERR:{}", e));
                                    }
                                }
                            }
                            "DISCONNECT" => {
                                let mut state = tunnel.lock().await;
                                if let Some(shutdown) = state.shutdown_tx.take() {
                                    let _ = shutdown.send(());
                                }
                                if let Some(shutdown) = state.write_shutdown_tx.take() {
                                    let _ = shutdown.send(());
                                }
                                state.tcp_tx = None;
                                let _ = ctrl_tx.send("CLOSED".to_string());
                                log::info!("SSH tunnel: disconnected");
                            }
                            _ => {
                                log::warn!("SSH_CTRL: unknown command: {}", cmd);
                            }
                        }
                        Ok(())
                    }
                    .boxed()
                },
            )),
            ..Default::default()
        }),
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                let mut rx = ctrl_notify_rx.clone();
                async move {
                    log::info!("SSH_CTRL notify subscriber connected");
                    // Mark current value as seen to skip stale values from
                    // previous tunnel sessions (e.g. leftover "CLOSED").
                    rx.borrow_and_update();
                    loop {
                        if rx.changed().await.is_err() {
                            log::info!("SSH_CTRL: watch sender dropped");
                            break;
                        }
                        let msg = rx.borrow_and_update().clone();
                        if msg.is_empty() {
                            continue;
                        }
                        log::info!("SSH_CTRL: notifying '{}'", msg);
                        if let Err(e) = notifier.notify(msg.as_bytes().to_vec()).await {
                            log::warn!("SSH_CTRL notify error: {:?}", e);
                            break;
                        }
                    }
                    log::info!("SSH_CTRL notify subscriber disconnected");
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// SSH_RX characteristic: write-without-response.
/// Client sends raw SSH bytes here → forwarded to TCP.
fn build_rx(tcp_write_tx: mpsc::Sender<Vec<u8>>) -> Characteristic {
    Characteristic {
        uuid: suuid::parse_uuid(suuid::SSH_RX),
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(
                move |data: Vec<u8>, _req: CharacteristicWriteRequest| {
                    let tx = tcp_write_tx.clone();
                    async move {
                        log::info!("SSH_RX: received {} bytes", data.len());
                        if tx.send(data).await.is_err() {
                            log::warn!("SSH_RX: tunnel not connected, data dropped");
                        }
                        Ok(())
                    }
                    .boxed()
                },
            )),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// SSH_TX characteristic: notify.
/// Sends raw SSH data from sshd → BLE client.
/// Uses broadcast so each subscriber gets its own independent stream,
/// avoiding lock contention between old and new BLE connections.
fn build_tx(ble_tx_sender: Arc<broadcast::Sender<Vec<u8>>>) -> Characteristic {
    Characteristic {
        uuid: suuid::parse_uuid(suuid::SSH_TX),
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                let mut rx = ble_tx_sender.subscribe();
                async move {
                    log::info!("SSH_TX notify subscriber connected");
                    loop {
                        match rx.recv().await {
                            Ok(data) if !data.is_empty() => {
                                // BLE notifications cannot exceed ATT_MTU - 3.
                                // macOS CoreBluetooth typically negotiates MTU 185,
                                // so max notification payload is 182 bytes.
                                for chunk in data.chunks(182) {
                                    if let Err(e) = notifier.notify(chunk.to_vec()).await {
                                        log::info!("SSH_TX notify error: {:?}, retrying once", e);
                                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                                        if notifier.notify(chunk.to_vec()).await.is_err() {
                                            log::info!("SSH_TX subscriber disconnected");
                                            return;
                                        }
                                    }
                                }
                            }
                            Ok(_) => continue,
                            Err(broadcast::error::RecvError::Closed) => {
                                log::info!("SSH_TX: broadcast channel closed");
                                break;
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                log::warn!("SSH_TX: subscriber lagged {} messages", n);
                                continue;
                            }
                        }
                    }
                    log::info!("SSH_TX notify loop ended");
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Task: read from TCP (sshd) and send to BLE TX notifications.
async fn tcp_read_task(
    mut tcp_read: tokio::io::ReadHalf<TcpStream>,
    ble_tx: Arc<broadcast::Sender<Vec<u8>>>,
    ctrl_tx: Arc<watch::Sender<String>>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) {
    let mut buf = [0u8; 512];
    loop {
        tokio::select! {
            _ = &mut shutdown => {
                log::debug!("SSH tunnel read task: shutdown");
                return;
            }
            result = tcp_read.read(&mut buf) => {
                match result {
                    Ok(0) => {
                        log::info!("SSH tunnel: sshd closed connection");
                        let _ = ctrl_tx.send("CLOSED".to_string());
                        return;
                    }
                    Ok(n) => {
                        log::info!("SSH tunnel: read {} bytes from sshd", n);
                        // broadcast::send() returns Err if no active receivers — that's OK
                        let _ = ble_tx.send(buf[..n].to_vec());
                    }
                    Err(e) => {
                        log::error!("SSH tunnel read error: {}", e);
                        let _ = ctrl_tx.send(format!("ERR:{}", e));
                        return;
                    }
                }
            }
        }
    }
}

/// Task: receive from BLE RX channel and write to TCP (sshd).
async fn tcp_write_task(
    tunnel: Arc<Mutex<TunnelState>>,
    rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    ctrl_tx: Arc<watch::Sender<String>>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) {
    let mut rx = rx.lock().await;
    loop {
        tokio::select! {
            _ = &mut shutdown => {
                log::debug!("SSH tunnel write task: shutdown");
                return;
            }
            data = rx.recv() => {
                match data {
                    Some(data) => {
                        log::info!("SSH tunnel: writing {} bytes to sshd", data.len());
                        let mut state = tunnel.lock().await;
                        if let Some(ref mut tcp_tx) = state.tcp_tx {
                            if let Err(e) = tcp_tx.write_all(&data).await {
                                log::error!("SSH tunnel write error: {}", e);
                                let _ = ctrl_tx.send(format!("ERR:{}", e));
                                state.tcp_tx = None;
                                return;
                            }
                        }
                    }
                    None => return,
                }
            }
        }
    }
}
