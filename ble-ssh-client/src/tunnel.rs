use std::net::SocketAddr;
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
    // BLE notifications can be silently dropped, so we retry if
    // the first data isn't a valid SSH banner.
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
                let hex: String = data.iter().take(16)
                    .map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
                log::warn!(
                    "Attempt {}: expected SSH banner, got {} bytes [{}], retrying...",
                    attempt, data.len(), hex
                );
                // Drain any remaining data from this failed attempt
                while let Ok(Some(_)) = tokio::time::timeout(
                    Duration::from_millis(100),
                    tunnel.recv_data(),
                ).await {}
                tunnel.close_tunnel().await;
                tokio::time::sleep(Duration::from_millis(300)).await;
                continue;
            }
            Ok(None) => {
                log::warn!("Attempt {}: data channel closed, retrying...", attempt);
                tunnel.close_tunnel().await;
                tokio::time::sleep(Duration::from_millis(300)).await;
                continue;
            }
            Err(_) => {
                log::warn!("Attempt {}: timeout waiting for SSH banner, retrying...", attempt);
                tunnel.close_tunnel().await;
                tokio::time::sleep(Duration::from_millis(300)).await;
                continue;
            }
        }
    }

    let banner = banner_data.ok_or_else(|| "Failed to receive SSH banner after 5 attempts".to_string())?;

    // Write the server's SSH banner to the local SSH client
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
                Ok(0) => {
                    log::debug!("SSH client closed TCP connection");
                    break;
                }
                Ok(n) => {
                    if tunnel_tx.write_data(&buf[..n]).await.is_err() {
                        log::debug!("BLE write failed");
                        break;
                    }
                }
                Err(e) => {
                    log::debug!("TCP read error: {}", e);
                    break;
                }
            }
        }
    };

    let tunnel_rx = tunnel.clone();
    let ble_to_client = async move {
        while let Some(bytes) = tunnel_rx.recv_data().await {
            if local_write.write_all(&bytes).await.is_err() {
                log::debug!("TCP write failed");
                break;
            }
        }
        log::debug!("BLE data channel ended");
    };

    tokio::select! {
        _ = client_to_ble => {}
        _ = ble_to_client => {}
    }

    tunnel.close_tunnel().await;
    log::info!("BLE tunnel closed for this connection");

    Ok(())
}

/// Check if a host:port is reachable via TCP with timeout.
pub async fn is_ip_reachable(addr: &str, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, TcpStream::connect(addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

/// Try to bind to the given port. If in use, try up to `max_retries` subsequent ports.
/// Returns (actual_port, listener).
pub async fn try_bind(port: u16, max_retries: u16) -> Result<(u16, TcpListener), String> {
    for offset in 0..max_retries {
        let try_port = port.wrapping_add(offset);
        let addr: SocketAddr = format!("127.0.0.1:{}", try_port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                if offset > 0 {
                    log::warn!("Port {} in use, using port {} instead", port, try_port);
                }
                return Ok((try_port, listener));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                log::warn!("Port {} in use, trying next...", try_port);
                continue;
            }
            Err(e) => return Err(format!("Bind failed: {}", e)),
        }
    }
    Err(format!(
        "All ports {}-{} are in use",
        port,
        port.wrapping_add(max_retries - 1)
    ))
}

/// Run the local TCP proxy server with a pre-bound listener.
pub async fn run_proxy_with_listener(
    listener: TcpListener,
    pi_ip: Option<String>,
    tunnel: Option<Arc<BleTunnel>>,
    speed: Arc<SpeedTracker>,
) -> Result<(), String> {
    log::info!("Proxy listening on {:?}", listener.local_addr());

    loop {
        let (stream, peer) = listener
            .accept()
            .await
            .map_err(|e| format!("Accept failed: {}", e))?;

        log::info!("New connection from {}", peer);

        let pi_ip = pi_ip.clone();
        let tunnel = tunnel.clone();
        let speed = speed.clone();

        tokio::spawn(async move {
            // Try IP first if available
            if let Some(ref ip) = pi_ip {
                let addr = format!("{}:22", ip);
                if is_ip_reachable(&addr, Duration::from_secs(2)).await {
                    log::info!("Using direct IP connection to {}", addr);
                    if let Err(e) = bridge_tcp_direct(stream, &addr, speed).await {
                        log::error!("Direct bridge error: {}", e);
                    }
                    return;
                }
                log::info!("IP {} not reachable, falling back to BLE", ip);
            }

            // Fall back to BLE tunnel
            if let Some(ref tunnel) = tunnel {
                log::info!("Using BLE tunnel");
                if let Err(e) = bridge_tcp_ble(stream, tunnel.clone(), speed).await {
                    log::error!("BLE bridge error: {}", e);
                }
            } else {
                log::error!("No BLE tunnel available and IP not reachable");
            }
        });
    }
}
