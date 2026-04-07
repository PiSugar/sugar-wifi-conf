use bluer::gatt::local::{Characteristic, CharacteristicNotify, CharacteristicNotifyMethod};
use futures::FutureExt;
use std::process::Command;
use tokio::time::{interval, Duration};

use crate::uuid as suuid;

fn get_wifi_name() -> String {
    let output = Command::new("iwconfig").arg("wlan0").output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let re = regex::Regex::new(r#"ESSID:"([^"]*)""#).unwrap();
            match re.captures(&stdout) {
                Some(caps) => caps[1].to_string(),
                None => "Not available".to_string(),
            }
        }
        Err(_) => "Not available".to_string(),
    }
}

/// Build the WiFi Name characteristic (notify, polls every 5s).
pub fn build() -> Characteristic {
    Characteristic {
        uuid: suuid::parse_uuid(suuid::WIFI_NAME),
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                async move {
                    log::info!("WifiName subscriber connected");
                    let mut tick = interval(Duration::from_secs(5));
                    // Send initial value immediately
                    let name = get_wifi_name();
                    if notifier.notify(name.as_bytes().to_vec()).await.is_err() {
                        return;
                    }
                    loop {
                        tick.tick().await;
                        let name = get_wifi_name();
                        log::debug!("WifiName update: {}", name);
                        if notifier.notify(name.as_bytes().to_vec()).await.is_err() {
                            log::info!("WifiName subscriber disconnected");
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
