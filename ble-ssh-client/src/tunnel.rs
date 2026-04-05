use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::ble::BleTunnel;
use crate::speed::SpeedTracker;

/// Bridge a local TCP connection directly to a remote IP:22.
pub async fn bridge_tcp_direct(
    mut local: TcpStream,
    remote_addr: &str,
    speed: Arc<SpeedTracker>,
) -> Result<(), String> {
    let mut remote = TcpStream::connect(remote_addr)
        .await
        .map_err(|e| format!("TCP connect to {} failed: {}", remote_addr, e))?;

    let (mut local_read, mut local_write) = local.split();
    let (mut remote_read, mut remote_write) = remote.split();

    let speed_tx = speed.clone();
    let speed_rx = speed.clone();

    let client_to_remote = async move {
        let mut buf = [0u8; 8192];
        loop {
            match local_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    speed_tx.add_tx(n as u64);
                    if remote_write.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    let remote_to_client = async move {
        let mut buf = [0u8; 8192];
        loop {
            match remote_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    speed_rx.add_rx(n as u64);
                    if local_write.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    tokio::select! {
        _ = client_to_remote => {}
        _ = remote_to_client => {}
    }
    Ok(())
}

/// Bridge a local TCP connection through BLE tunnel.
/// Opens the tunnel per-connection with retry logic to handle
/// dropped BLE notifications (the SSH banner might be lost).
pub async fn bridge_tcp_ble(
    mut local: TcpStream,
    tunnel: Arc<BleTunnel>,
    _speed: Arc<SpeedTracker>,
) -> Result<(), String> {
    let (mut local_read, mut local_write) = local.split();

    // Phase 1: Open tunnel and verify SSH banner arrives.
    let mut banner_data: Option<Vec<u8>> = None;
    for attempt in 0..5u32 {
        tunnel.open_tunnel().await?;

        match tokio::time::timeout(Duration::from_secs(3), tunnel.recv_data()).await {
            Ok(Some(data)) if data.len() >= 4 && data.starts_with(b"SSH-") => {
                log::info!("Got SSH banner ({} bytes) on attempt {}", data.len(), attempt);
                banner_data = Some(data);
                break;
            }
            Ok(Some(data)) => {
                let hex: String = data
                    .iter()
                    .take(16)
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                log::warn!(
                    "Attempt {}: expected SSH banner, got {} bytes [{}], retrying...",
                    attempt,
                    data.len(),
                    hex
                );
                while let Ok(Some(_)) =
                    tokio::time::timeout(Duration::from_millis(100), tunnel.recv_data()).await
                {
                }
                tunnel.close_tunnel().await;
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            Ok(None) => {
                log::warn!("Attempt {}: data channel closed, retrying...", attempt);
                tunnel.close_tunnel().await;
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            Err(_) => {
                log::warn!(
                    "Attempt {}: timeout waiting for SSH banner, retrying...",
                    attempt
                );
                tunnel.close_tunnel().await;
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }

    let banner =
        banner_data.ok_or_else(|| "Failed to receive SSH banner after 5 attempts".to_string())?;

    local_write
        .write_all(&banner)
        .await
        .map_err(|e| format!("Write banner: {}", e))?;

    log::info!("BLE tunnel opened, banner delivered to SSH client");

    // Phase 2: Bidirectional bridge
    let tunnel_tx = tunnel.clone();
    let client_to_ble = async move {
        let mut buf = [0u8; 512];
        loop {
            match local_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if tunnel_tx.write_data(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    let tunnel_rx = tunnel.clone();
    let ble_to_client = async move {
        while let Some(bytes) = tunnel_rx.recv_data().await {
            if local_write.write_all(&bytes).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = client_to_ble => {}
        _ = ble_to_client => {}
    }

    tunnel.close_tunnel().await;
    Ok(())
}

/// Check if a host:port is reachable via TCP with timeout.
pub async fn is_ip_reachable(addr: &str, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, TcpStream::connect(addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

/// Run a TCP proxy for a single device on the given port.
/// Accepts SSH connections and bridges them (tries direct IP first, falls back to BLE).
pub async fn run_device_proxy(
    port: u16,
    pi_ip: Option<String>,
    tunnel: Option<Arc<BleTunnel>>,
    speed: Arc<SpeedTracker>,
) -> Result<(), String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .map_err(|e| format!("Bind port {}: {}", port, e))?;

    log::info!("Device proxy listening on port {}", port);

    loop {
        let (stream, peer) = listener
            .accept()
            .await
            .map_err(|e| format!("Accept: {}", e))?;

        log::info!("Connection from {} on port {}", peer, port);

        let ip = pi_ip.clone();
        let tun = tunnel.clone();
        let spd = speed.clone();

        tokio::spawn(async move {
            if let Some(ref ip) = ip {
                let addr = format!("{}:22", ip);
                if is_ip_reachable(&addr, Duration::from_secs(2)).await {
                    log::info!("Direct IP to {}", addr);
                    if let Err(e) = bridge_tcp_direct(stream, &addr, spd).await {
                        log::error!("Direct bridge: {}", e);
                    }
                    return;
                }
                log::info!("IP {} not reachable, falling back to BLE", ip);
            }

            if let Some(ref tun) = tun {
                if let Err(e) = bridge_tcp_ble(stream, tun.clone(), spd).await {
                    log::error!("BLE bridge: {}", e);
                }
            } else {
                log::error!("No tunnel and IP not reachable");
            }
        });
    }
}
