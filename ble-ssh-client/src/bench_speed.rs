//! BLE SSH speed benchmark.
//! Connects via BLE tunnel, runs various throughput and latency tests,
//! and writes a Markdown report.

mod ble;
mod speed;
mod tunnel;
mod uuid;

use std::sync::Arc;
use std::time::{Duration, Instant};

use btleplug::api::{Central as _, Peripheral as _};
use tokio::io::AsyncWriteExt;

const PROXY_PORT: u16 = 12223;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    match run_bench().await {
        Ok(report) => {
            let path = std::path::Path::new("ble-ssh-speed-report.md");
            if let Err(e) = std::fs::write(path, &report) {
                log::error!("Failed to write report: {}", e);
                std::process::exit(1);
            }
            println!("\n{}", report);
            log::info!("Report saved to {}", path.display());
        }
        Err(e) => {
            log::error!("BENCHMARK FAILED: {}", e);
            std::process::exit(1);
        }
    }
}

struct BenchResult {
    name: String,
    detail: String,
}

async fn run_bench() -> Result<String, String> {
    // ── Connect ──────────────────────────────────────────
    log::info!("=== Connecting to PiSugar device ===");
    let adapter = ble::get_adapter().await?;
    let service_uuid = uuid::parse_uuid(uuid::SERVICE_ID);
    let dev = find_device(&adapter, service_uuid).await?;
    log::info!("Found: {} ({})", dev.name, dev.id);

    ble::ble_connect(&dev.peripheral).await?;
    let user = ble::read_ssh_username(&dev.peripheral)
        .await
        .unwrap_or_else(|| "pi".to_string());
    log::info!("SSH user: {}", user);

    let speed_tracker = speed::SpeedTracker::new();
    let tunnel_obj =
        ble::BleTunnel::from_connected(dev.peripheral.clone(), speed_tracker.clone()).await?;
    let tunnel_obj = Arc::new(tunnel_obj);
    log::info!("BleTunnel created");

    // Start TCP proxy
    let tun = tunnel_obj.clone();
    let spd = speed_tracker.clone();
    tokio::spawn(async move {
        if let Err(e) = tunnel::run_device_proxy(PROXY_PORT, tun, spd).await {
            log::error!("Proxy: {}", e);
        }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;

    let ssh_target = format!("{}@localhost", user);

    // ── Run benchmarks ───────────────────────────────────
    let mut results: Vec<BenchResult> = Vec::new();

    // 1) Latency: echo round-trip
    log::info!("=== Test 1: Latency (echo round-trip) ===");
    let latency_result = bench_latency(&ssh_target, 10).await;
    results.push(latency_result);

    // 2) Download: receive data from Pi
    log::info!("=== Test 2: Download throughput ===");
    for &size_kb in &[1, 10, 50] {
        let r = bench_download(&ssh_target, size_kb).await;
        results.push(r);
    }

    // 3) Upload: send data to Pi
    log::info!("=== Test 3: Upload throughput ===");
    for &size_kb in &[1, 10, 100] {
        let r = bench_upload(&ssh_target, size_kb).await;
        results.push(r);
    }

    // 4) Interactive command latency
    log::info!("=== Test 4: Command execution latency ===");
    let cmd_result = bench_commands(&ssh_target).await;
    results.push(cmd_result);

    // ── Cleanup ──────────────────────────────────────────
    log::info!("=== Cleanup ===");
    tunnel_obj.close_tunnel().await;
    let _ = adapter.stop_scan().await;
    let _ = dev.peripheral.disconnect().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // ── Generate report ──────────────────────────────────
    let total_tx = speed_tracker.total_tx();
    let total_rx = speed_tracker.total_rx();

    let mut md = String::new();
    md.push_str("# BLE SSH Speed Test Report\n\n");
    md.push_str(&format!(
        "- **Date**: {}\n",
        chrono_now()
    ));
    md.push_str(&format!("- **Device**: {} ({})\n", dev.name, dev.id));
    md.push_str(&format!("- **SSH User**: {}\n", user));
    md.push_str(&format!(
        "- **Total transferred**: TX {} / RX {}\n\n",
        speed::format_total(total_tx),
        speed::format_total(total_rx)
    ));

    md.push_str("## Results\n\n");
    md.push_str("| Test | Result |\n");
    md.push_str("|------|--------|\n");
    for r in &results {
        md.push_str(&format!("| {} | {} |\n", r.name, r.detail));
    }
    md.push_str("\n## Details\n\n");
    for r in &results {
        md.push_str(&format!("### {}\n\n{}\n\n", r.name, r.detail));
    }

    md.push_str("\n## Analysis\n\n");
    md.push_str("- **Latency**: Each SSH command includes full session setup (TCP connect → BLE CONNECT/OK → SSH handshake → command → response). The ~1s latency is dominated by SSH key exchange over the low-bandwidth BLE link.\n");
    md.push_str("- **Throughput**: BLE 4.x GATT notifications have a practical ceiling of ~5-10 KB/s due to MTU size (typically 20-512 bytes per notification) and connection interval. Upload (client→Pi) uses `WriteWithResponse` which is slower due to acknowledgment overhead.\n");
    md.push_str("- **Download vs Upload**: Download (Pi→client via BLE notifications) tends to be faster than upload (client→Pi via GATT writes) because notifications are fire-and-forget at the BLE level, while writes require per-packet ACK.\n");
    md.push_str("- **Large transfers**: 100KB+ transfers may fail due to SSH session timeout or BLE connection instability over extended periods. BLE is best suited for interactive SSH sessions, not bulk file transfers.\n");

    Ok(md)
}

// ── Benchmark helpers ────────────────────────────────────

/// Measure echo round-trip latency over SSH.
async fn bench_latency(ssh_target: &str, rounds: u32) -> BenchResult {
    let mut latencies = Vec::new();

    for i in 0..rounds {
        let start = Instant::now();
        let out = ssh_cmd(ssh_target, "echo pong").await;
        let elapsed = start.elapsed();

        match out {
            Ok(s) if s.trim() == "pong" => {
                let ms = elapsed.as_millis();
                log::info!("  Latency round {}: {} ms", i + 1, ms);
                latencies.push(elapsed);
            }
            Ok(s) => {
                log::warn!("  Round {}: unexpected output: {}", i + 1, s.trim());
            }
            Err(e) => {
                log::warn!("  Round {}: error: {}", i + 1, e);
            }
        }
    }

    if latencies.is_empty() {
        return BenchResult {
            name: "Echo Latency".to_string(),
            detail: "All rounds failed".to_string(),
        };
    }

    latencies.sort();
    let min = latencies.first().unwrap().as_millis();
    let max = latencies.last().unwrap().as_millis();
    let avg = latencies.iter().map(|d| d.as_millis()).sum::<u128>() / latencies.len() as u128;
    let median = latencies[latencies.len() / 2].as_millis();

    BenchResult {
        name: format!("Echo Latency ({} rounds)", rounds),
        detail: format!(
            "min {} ms / avg {} ms / median {} ms / max {} ms ({}/{} ok)",
            min,
            avg,
            median,
            max,
            latencies.len(),
            rounds
        ),
    }
}

/// Download: have the Pi generate N KB and measure receipt.
async fn bench_download(ssh_target: &str, size_kb: u32) -> BenchResult {
    let cmd = format!("dd if=/dev/urandom bs=1024 count={} 2>/dev/null | base64", size_kb);
    let start = Instant::now();
    let result = ssh_cmd(ssh_target, &cmd).await;
    let elapsed = start.elapsed();

    match result {
        Ok(data) => {
            let bytes = data.len() as f64;
            let secs = elapsed.as_secs_f64();
            let kbps = (bytes / 1024.0) / secs;
            let detail = format!(
                "{:.1} KB in {:.1}s = **{:.1} KB/s** ({:.1} kbit/s)",
                bytes / 1024.0,
                secs,
                kbps,
                kbps * 8.0
            );
            log::info!("  Download {}KB: {}", size_kb, detail);
            BenchResult {
                name: format!("Download {} KB", size_kb),
                detail,
            }
        }
        Err(e) => BenchResult {
            name: format!("Download {} KB", size_kb),
            detail: format!("FAILED: {}", e),
        },
    }
}

/// Upload: send N KB to the Pi via stdin and measure.
async fn bench_upload(ssh_target: &str, size_kb: u32) -> BenchResult {
    // Generate random payload locally
    let payload_size = size_kb as usize * 1024;
    let payload: Vec<u8> = (0..payload_size).map(|i| (i % 256) as u8).collect();

    let start = Instant::now();
    let result = ssh_cmd_with_stdin(
        ssh_target,
        &format!("wc -c"),
        &payload,
    )
    .await;
    let elapsed = start.elapsed();

    match result {
        Ok(out) => {
            let received: usize = out.trim().parse().unwrap_or(0);
            let secs = elapsed.as_secs_f64();
            let kbps = (payload_size as f64 / 1024.0) / secs;
            let detail = format!(
                "sent {} KB, Pi received {} bytes in {:.1}s = **{:.1} KB/s** ({:.1} kbit/s)",
                size_kb,
                received,
                secs,
                kbps,
                kbps * 8.0
            );
            log::info!("  Upload {}KB: {}", size_kb, detail);
            BenchResult {
                name: format!("Upload {} KB", size_kb),
                detail,
            }
        }
        Err(e) => BenchResult {
            name: format!("Upload {} KB", size_kb),
            detail: format!("FAILED: {}", e),
        },
    }
}

/// Measure execution time of several common commands.
async fn bench_commands(ssh_target: &str) -> BenchResult {
    let cmds = [
        ("uname -a", "Kernel info"),
        ("ls /", "List root"),
        ("cat /proc/cpuinfo | head -20", "CPU info"),
        ("df -h /", "Disk usage"),
        ("free -h", "Memory info"),
    ];

    let mut lines = Vec::new();
    for (cmd, label) in &cmds {
        let start = Instant::now();
        let result = ssh_cmd(ssh_target, cmd).await;
        let elapsed = start.elapsed();
        let ms = elapsed.as_millis();
        match result {
            Ok(_) => {
                log::info!("  {} ({}): {} ms", label, cmd, ms);
                lines.push(format!("{}: {} ms", label, ms));
            }
            Err(e) => {
                log::warn!("  {} ({}): FAILED: {}", label, cmd, e);
                lines.push(format!("{}: FAILED ({})", label, e));
            }
        }
    }

    BenchResult {
        name: "Command Execution".to_string(),
        detail: lines.join(" / "),
    }
}

// ── SSH execution helpers ────────────────────────────────

/// Run a command over SSH through the BLE tunnel, return stdout.
async fn ssh_cmd(ssh_target: &str, cmd: &str) -> Result<String, String> {
    let output = tokio::time::timeout(
        Duration::from_secs(60),
        tokio::process::Command::new("ssh")
            .args([
                "-o", "StrictHostKeyChecking=no",
                "-o", "UserKnownHostsFile=/dev/null",
                "-o", "ConnectTimeout=15",
                "-o", "LogLevel=ERROR",
                "-p", &PROXY_PORT.to_string(),
                ssh_target,
                cmd,
            ])
            .output(),
    )
    .await
    .map_err(|_| "SSH timeout (60s)".to_string())?
    .map_err(|e| format!("SSH spawn: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "exit {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a command over SSH, piping stdin data.
async fn ssh_cmd_with_stdin(
    ssh_target: &str,
    cmd: &str,
    stdin_data: &[u8],
) -> Result<String, String> {
    let mut child = tokio::process::Command::new("ssh")
        .args([
            "-o", "StrictHostKeyChecking=no",
            "-o", "UserKnownHostsFile=/dev/null",
            "-o", "ConnectTimeout=15",
            "-o", "LogLevel=ERROR",
            "-p", &PROXY_PORT.to_string(),
            ssh_target,
            cmd,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("SSH spawn: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        let data = stdin_data.to_vec();
        tokio::spawn(async move {
            let _ = stdin.write_all(&data).await;
            drop(stdin); // close stdin so remote cmd gets EOF
        });
    }

    let output = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())
        .await
        .map_err(|_| "SSH timeout (120s)".to_string())?
        .map_err(|e| format!("SSH wait: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "exit {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ── Device discovery (same as test binary) ───────────────

async fn find_device(
    adapter: &btleplug::platform::Adapter,
    service_uuid: ::uuid::Uuid,
) -> Result<ble::ScannedDevice, String> {
    use btleplug::api::Central;
    log::info!("Checking cached peripherals...");
    if let Ok(peripherals) = adapter.peripherals().await {
        for p in peripherals {
            if let Ok(Some(props)) = p.properties().await {
                if props.services.contains(&service_uuid) {
                    let name = props.local_name.unwrap_or_else(|| "Unknown".to_string());
                    let id = format!("{:?}", p.id());
                    return Ok(ble::ScannedDevice {
                        peripheral: p,
                        name,
                        id,
                    });
                }
            }
        }
    }

    log::info!("No cached device, scanning...");
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

/// Simple date/time string (no chrono dependency).
fn chrono_now() -> String {
    let output = std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S %Z")
        .output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => "unknown".to_string(),
    }
}
