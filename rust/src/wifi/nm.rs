use std::process::Command;

/// Configure WiFi using NetworkManager (nmcli).
/// Uses Command::new() with separate args to prevent command injection.
pub async fn set_wifi_nm(ssid: &str, password: &str) -> String {
    // Delete existing connection (ignore errors)
    let _ = Command::new("nmcli")
        .args(["connection", "delete", "id", ssid])
        .output();

    log::info!("Adding and connecting to Wi-Fi SSID: {}", ssid);

    // Add new connection
    let add_result = Command::new("nmcli")
        .args([
            "connection", "add",
            "type", "wifi",
            "ifname", "wlan0",
            "con-name", ssid,
            "ssid", ssid,
            "wifi-sec.key-mgmt", "wpa-psk",
            "wifi-sec.psk", password,
        ])
        .output();

    if let Err(e) = &add_result {
        let msg = format!("Failed to add WiFi connection: {}", e);
        log::error!("{}", msg);
        return msg;
    }

    let add_out = add_result.unwrap();
    if !add_out.status.success() {
        let msg = format!(
            "nmcli add failed: {}",
            String::from_utf8_lossy(&add_out.stderr)
        );
        log::error!("{}", msg);
        return msg;
    }

    // Connect to new connection
    let up_result = Command::new("nmcli")
        .args(["connection", "up", ssid])
        .output();

    match up_result {
        Ok(out) if out.status.success() => {
            let msg = format!(
                "Successfully connected to Wi-Fi: {}",
                String::from_utf8_lossy(&out.stdout).trim()
            );
            log::info!("{}", msg);
            msg
        }
        Ok(out) => {
            let msg = format!(
                "Failed to connect: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            log::error!("{}", msg);
            msg
        }
        Err(e) => {
            let msg = format!("Failed to execute nmcli up: {}", e);
            log::error!("{}", msg);
            msg
        }
    }
}
