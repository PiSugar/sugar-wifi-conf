pub mod nm;
pub mod wpa;

use std::process::Command;

/// Detect whether wlan0 is managed by NetworkManager.
pub async fn is_wlan0_managed_by_nm() -> bool {
    let output = Command::new("nmcli")
        .args(["dev", "status"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.contains("wlan0") {
                    if line.contains("unavailable") {
                        log::info!("wlan0 is unavailable, trying to unblock wifi");
                        let _ = Command::new("sudo")
                            .args(["rfkill", "unblock", "wifi"])
                            .output();
                    }
                    if line.contains("connected") || line.contains("disconnected") {
                        log::info!("wlan0 is managed by NetworkManager");
                        return true;
                    } else {
                        log::info!("wlan0 is not managed by NetworkManager");
                        return false;
                    }
                }
            }
            log::warn!("wlan0 device not found in nmcli output");
            false
        }
        Err(e) => {
            log::debug!("nmcli not available: {}", e);
            false
        }
    }
}

/// Configure WiFi by auto-detecting NetworkManager vs wpa_supplicant.
pub async fn set_wifi(ssid: &str, password: &str) -> String {
    if is_wlan0_managed_by_nm().await {
        nm::set_wifi_nm(ssid, password).await
    } else {
        wpa::set_wifi_wpa(ssid, password).await
    }
}
