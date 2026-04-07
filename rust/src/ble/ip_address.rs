use bluer::gatt::local::{Characteristic, CharacteristicNotify, CharacteristicNotifyMethod};
use futures::FutureExt;
use std::net::IpAddr;
use tokio::time::{interval, Duration};


use crate::uuid as suuid;

fn get_ip_addresses() -> String {
    let mut addresses = Vec::new();
    if let Ok(ifaces) = nix_ifaddrs() {
        for addr in ifaces {
            if let IpAddr::V4(v4) = addr {
                if !v4.is_loopback() {
                    addresses.push(v4.to_string());
                }
            }
        }
    }
    if addresses.is_empty() {
        "--".to_string()
    } else {
        addresses.join(", ")
    }
}

/// Get IPv4 addresses from network interfaces using std::net and /proc/net or ip command.
fn nix_ifaddrs() -> Result<Vec<IpAddr>, std::io::Error> {
    // Use `ip -4 addr show` to list all IPv4 addresses
    let output = std::process::Command::new("ip")
        .args(["-4", "-o", "addr", "show"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let re = regex::Regex::new(r"inet (\d+\.\d+\.\d+\.\d+)").unwrap();
    let addrs: Vec<IpAddr> = re
        .captures_iter(&stdout)
        .filter_map(|cap| cap[1].parse().ok())
        .collect();
    Ok(addrs)
}

/// Build the IP Address characteristic (notify, polls every 5s).
pub fn build() -> Characteristic {
    Characteristic {
        uuid: suuid::parse_uuid(suuid::IP_ADDRESS),
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                async move {
                    log::info!("IpAddress subscriber connected");
                    let mut tick = interval(Duration::from_secs(5));
                    // Send initial value immediately
                    let ip = get_ip_addresses();
                    if notifier.notify(ip.as_bytes().to_vec()).await.is_err() {
                        return;
                    }
                    loop {
                        tick.tick().await;
                        let ip = get_ip_addresses();
                        log::debug!("IpAddress update: {}", ip);
                        if notifier.notify(ip.as_bytes().to_vec()).await.is_err() {
                            log::info!("IpAddress subscriber disconnected");
                            return;
                        }
                    }
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}
