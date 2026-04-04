mod ble;
mod speed;
mod tunnel;
mod uuid;

use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use crossterm::{cursor, execute, style, terminal};
use dialoguer::{theme::ColorfulTheme, Select};

#[derive(Parser, Debug)]
#[command(name = "ble-ssh", about = "SSH over BLE tunnel for sugar-wifi-conf")]
struct Args {
    /// Local port to listen on (connect via: ssh pi@localhost -p PORT)
    #[arg(short, long, default_value = "2222")]
    port: u16,

    /// Pi IP address (optional, auto-detected from BLE if omitted)
    #[arg(short, long)]
    ip: Option<String>,

    /// BLE scan timeout in seconds
    #[arg(long, default_value = "10")]
    scan_timeout: u64,

    /// Disable BLE (IP-only mode)
    #[arg(long)]
    no_ble: bool,

    /// Force BLE tunnel even when IP is reachable
    #[arg(long)]
    force_ble: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let speed = speed::SpeedTracker::new();

    println!("╔══════════════════════════════════════╗");
    println!("║       BLE-SSH Tunnel Client          ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    let mut pi_ip = args.ip.clone();
    let mut ble_tunnel: Option<Arc<ble::BleTunnel>> = None;
    let mut force_ble = args.force_ble;

    if !args.no_ble {
        println!("🔍 Scanning for PiSugar BLE devices ({} s)...", args.scan_timeout);
        let adapter = ble::get_adapter().await?;

        match ble::live_scan_and_select(&adapter, args.scan_timeout).await {
            Ok(device) => {
                // Use device's IP if we don't have one from CLI
                if pi_ip.is_none() {
                    pi_ip = device.ip.clone();
                }

                // If IP is available, ask for connection mode
                if pi_ip.is_some() && !force_ble {
                    let mode_items = &[
                        "Auto (prefer IP, BLE fallback)",
                        "Force BLE",
                    ];
                    let mode_sel = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Connection mode")
                        .items(mode_items)
                        .default(0)
                        .interact()?;
                    if mode_sel == 1 {
                        force_ble = true;
                    }
                }

                match ble::BleTunnel::from_connected(device.peripheral, speed.clone()).await {
                    Ok(tunnel) => {
                        println!("✅ BLE ready (tunnel opened per-connection)");
                        ble_tunnel = Some(Arc::new(tunnel));
                    }
                    Err(e) => {
                        println!("⚠️  BLE tunnel setup failed: {}", e);
                        if pi_ip.is_none() {
                            return Err(e.into());
                        }
                    }
                }
            }
            Err(e) => {
                println!("⚠️  BLE scan: {}", e);
                if pi_ip.is_none() {
                    eprintln!("❌ No IP and no BLE device — cannot connect.");
                    return Err(e.into());
                }
                println!("   Falling back to IP-only mode.");
            }
        }
    }

    // Determine connection mode
    let mode = if force_ble && ble_tunnel.is_some() {
        "Force BLE".to_string()
    } else if let Some(ref ip) = pi_ip {
        if ble_tunnel.is_some() {
            format!("Auto (IP: {} + BLE fallback)", ip)
        } else {
            format!("IP-only ({}:22)", ip)
        }
    } else if ble_tunnel.is_some() {
        "BLE-only".to_string()
    } else {
        eprintln!("❌ No connection method available");
        return Err("No IP and no BLE".into());
    };

    // When force_ble, don't pass IP to proxy so it always uses BLE
    let proxy_ip = if force_ble { None } else { pi_ip };

    // Try to bind, auto-increment port if in use
    let (actual_port, listener) = tunnel::try_bind(args.port, 10).await?;

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Mode:     {}", mode);
    println!("  Proxy:    localhost:{}", actual_port);
    println!("  Connect:  ssh pi@localhost -p {}", actual_port);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Spawn speed display task
    let speed_display = speed.clone();
    let display_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.tick().await;

        loop {
            interval.tick().await;
            let (tx_speed, rx_speed) = speed_display.speed();
            if tx_speed > 0.0 || rx_speed > 0.0 {
                let msg = format!(
                    "  ↑ {} | ↓ {} | Total: ↑ {} ↓ {}",
                    speed::format_bytes(tx_speed),
                    speed::format_bytes(rx_speed),
                    speed::format_total(speed_display.total_tx()),
                    speed::format_total(speed_display.total_rx()),
                );
                let mut stdout = std::io::stdout();
                let _ = execute!(
                    stdout,
                    cursor::SavePosition,
                    terminal::Clear(terminal::ClearType::CurrentLine),
                    style::Print(&msg),
                    cursor::RestorePosition,
                );
            }
        }
    });

    // Run the proxy with the pre-bound listener
    let result = tunnel::run_proxy_with_listener(listener, proxy_ip, ble_tunnel.clone(), speed.clone()).await;

    display_handle.abort();

    // Cleanup BLE
    if let Some(tunnel) = ble_tunnel {
        tunnel.disconnect().await;
    }

    result.map_err(|e| e.into())
}
