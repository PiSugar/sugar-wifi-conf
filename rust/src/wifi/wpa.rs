use regex::Regex;
use std::fs;
use std::process::Command;

const CONF_PATH: &str = "/etc/wpa_supplicant/wpa_supplicant.conf";
const IFACE_PATH: &str = "/etc/network/interfaces";

/// Configure WiFi using wpa_supplicant directly.
/// Parses and modifies wpa_supplicant.conf, then restarts wpa_supplicant.
pub async fn set_wifi_wpa(ssid: &str, password: &str) -> String {
    // Read existing config
    let data = match fs::read_to_string(CONF_PATH) {
        Ok(d) => d,
        Err(e) => return format!("Failed to read {}: {}", CONF_PATH, e),
    };

    let wifi_regex = Regex::new(r"(?s)(network=\{[^}]+\})").unwrap();
    let ssid_regex = Regex::new(r#"ssid="([^"]*)""#).unwrap();
    let priority_regex = Regex::new(r"priority=(\d+)").unwrap();

    let mut wifi_blocks: Vec<String> = Vec::new();
    let mut max_priority: u32 = 0;

    // Strip out all network blocks from the prefix
    let mut prefix = data.clone();
    for cap in wifi_regex.captures_iter(&data) {
        let block = cap[1].to_string();

        // Extract SSID from this block
        let block_ssid = ssid_regex
            .captures(&block)
            .map(|c| c[1].to_string())
            .unwrap_or_default();

        // Extract priority
        if let Some(p) = priority_regex.captures(&block) {
            if let Ok(pri) = p[1].parse::<u32>() {
                max_priority = max_priority.max(pri);
            }
        }

        // Remove block from prefix
        prefix = prefix.replace(&block, "");

        // Keep non-duplicate SSIDs
        if block_ssid != ssid {
            wifi_blocks.push(block);
        }
    }

    // Normalize: Country= -> country=
    let prefix = prefix.replace("Country=", "country=");
    let prefix = prefix.trim_end();

    // Add new network block
    wifi_blocks.push(format!(
        "network={{\n\t\tssid=\"{}\"\n\t\tscan_ssid=1\n\t\tpsk=\"{}\"\n\t\tpriority={}\n\t}}",
        ssid,
        password,
        max_priority + 1
    ));

    let content = format!("{}\n\t{}", prefix, wifi_blocks.join("\n\t"));

    // Write config
    if let Err(e) = fs::write(CONF_PATH, &content) {
        return format!("Failed to write {}: {}", CONF_PATH, e);
    }

    // Check if wlan0 is OK (no nohook wpa_supplicant)
    if !is_wlan0_ok() {
        return "OK. Please reboot.".to_string();
    }

    // Kill existing wpa_supplicant
    let _ = Command::new("killall").arg("wpa_supplicant").output();

    // Retry launching wpa_supplicant
    let mut res_msg = String::new();
    let mut tries_left = 10;
    while tries_left > 0 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let result = Command::new("wpa_supplicant")
            .args(["-B", "-iwlan0", &format!("-c{}", CONF_PATH)])
            .output();

        match result {
            Ok(out) if out.status.success() => {
                res_msg = String::from_utf8_lossy(&out.stdout).to_string();
                break;
            }
            Ok(out) => {
                res_msg = format!(
                    "wpa_supplicant error: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                log::warn!("{}", res_msg);
            }
            Err(e) => {
                res_msg = format!("Command failed: {}", e);
                log::error!("{}", res_msg);
            }
        }
        tries_left -= 1;
    }

    format!("{} {}", tries_left, res_msg)
}

/// Check /etc/network/interfaces for wlan0 nohook wpa_supplicant issue.
fn is_wlan0_ok() -> bool {
    let data = match fs::read_to_string(IFACE_PATH) {
        Ok(d) => d,
        Err(_) => return true, // If file doesn't exist, assume OK
    };

    let mut found_wlan0 = false;
    let mut is_ok = true;

    for line in data.lines() {
        let trimmed = line.trim();

        if found_wlan0 && trimmed.contains("interface ") && !trimmed.starts_with('#') {
            found_wlan0 = false;
        }
        if trimmed.contains("interface wlan0") && !trimmed.starts_with('#') {
            found_wlan0 = true;
        }
        if found_wlan0 && trimmed.contains("nohook wpa_supplicant") && !trimmed.starts_with('#') {
            is_ok = false;
        }
    }

    log::info!("Is wlan0 Ok? {}", is_ok);
    is_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wpa_supplicant_conf() {
        let conf = r#"country=CN
ctrl_interface=DIR=/var/run/wpa_supplicant GROUP=netdev
update_config=1

network={
    ssid="ExampleSSID1"
    psk="example_password"
    priority=2
}
network={
    ssid="ExampleSSID2"
    psk="example_password"
}"#;

        let wifi_regex = Regex::new(r"(?s)(network=\{[^}]+\})").unwrap();
        let ssid_regex = Regex::new(r#"ssid="([^"]*)""#).unwrap();
        let priority_regex = Regex::new(r"priority=(\d+)").unwrap();

        let matches: Vec<_> = wifi_regex.captures_iter(conf).collect();
        assert_eq!(matches.len(), 2);

        let block0 = &matches[0][1];
        let ssid0 = ssid_regex.captures(block0).unwrap()[1].to_string();
        assert_eq!(ssid0, "ExampleSSID1");

        let pri0 = priority_regex.captures(block0).unwrap()[1]
            .parse::<u32>()
            .unwrap();
        assert_eq!(pri0, 2);

        let block1 = &matches[1][1];
        let ssid1 = ssid_regex.captures(block1).unwrap()[1].to_string();
        assert_eq!(ssid1, "ExampleSSID2");
        assert!(priority_regex.captures(block1).is_none());
    }
}
