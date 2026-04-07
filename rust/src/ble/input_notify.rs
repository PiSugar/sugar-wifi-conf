use std::sync::{Arc, Mutex};
use std::time::Instant;

use bluer::gatt::local::{
    Characteristic, CharacteristicNotify, CharacteristicNotifyMethod, CharacteristicWrite,
    CharacteristicWriteMethod, CharacteristicWriteRequest,
};
use futures::FutureExt;
use tokio::sync::watch;
use tokio::time::{interval, Duration};

use crate::uuid as suuid;
use crate::wifi;

use super::{CONCAT_TAG, END_TAG};

/// Build the deprecated Input characteristic (write).
/// Format: key%&%ssid%&%password (single packet).
pub fn build_input(key: String, notify_tx: Arc<watch::Sender<String>>) -> Characteristic {
    Characteristic {
        uuid: suuid::parse_uuid(suuid::INPUT),
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |data: Vec<u8>, _req: CharacteristicWriteRequest| {
                let key = key.clone();
                let notify_tx = notify_tx.clone();
                async move {
                    let input = String::from_utf8_lossy(&data).to_string();
                    log::info!("Input write request: {}", input);

                    let parts: Vec<&str> = input.split(CONCAT_TAG).collect();
                    if parts.len() < 3 {
                        log::warn!("Wrong input syntax");
                        let _ = notify_tx.send("Wrong input syntax.".to_string());
                        return Ok(());
                    }
                    if parts[0] != key {
                        log::warn!("Wrong input key");
                        let _ = notify_tx.send("Wrong input key.".to_string());
                        return Ok(());
                    }

                    let ssid = parts[1].to_string();
                    let password = parts[2].to_string();

                    // Run wifi config in background
                    let tx = notify_tx.clone();
                    tokio::spawn(async move {
                        let msg = wifi::wpa::set_wifi_wpa(&ssid, &password).await;
                        let _ = tx.send(msg);
                    });

                    Ok(())
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Build the InputSep characteristic (write, chunked 20-byte packets).
/// Format: key%&%ssid%&%password&#& (split across multiple writes).
/// Buffer auto-clears after 5 seconds of inactivity.
pub fn build_input_sep(key: String, notify_tx: Arc<watch::Sender<String>>) -> Characteristic {
    let buffer = Arc::new(Mutex::new(String::new()));
    let last_change = Arc::new(Mutex::new(Instant::now()));

    // Spawn buffer cleanup task
    let buf_cleanup = buffer.clone();
    let lc_cleanup = last_change.clone();
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(1));
        loop {
            tick.tick().await;
            let mut buf = buf_cleanup.lock().unwrap();
            let lc = lc_cleanup.lock().unwrap();
            if !buf.is_empty() && lc.elapsed() > Duration::from_secs(5) {
                log::debug!("Clear separateInputString (timeout)");
                buf.clear();
            }
        }
    });

    Characteristic {
        uuid: suuid::parse_uuid(suuid::INPUT_SEP),
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |data: Vec<u8>, _req: CharacteristicWriteRequest| {
                let key = key.clone();
                let notify_tx = notify_tx.clone();
                let buffer = buffer.clone();
                let last_change = last_change.clone();
                async move {
                    let chunk = String::from_utf8_lossy(&data).to_string();
                    log::debug!("InputSep write: {}", chunk);

                    let mut buf = buffer.lock().unwrap();
                    buf.push_str(&chunk);
                    *last_change.lock().unwrap() = Instant::now();

                    let is_last = buf.contains(END_TAG);
                    if !is_last {
                        return Ok(());
                    }

                    // Complete message received
                    let full = buf.replace(END_TAG, "");
                    buf.clear();
                    drop(buf);

                    let parts: Vec<&str> = full.split(CONCAT_TAG).collect();
                    if parts.len() < 3 {
                        log::warn!("Invalid syntax");
                        let _ = notify_tx.send("Invalid syntax.".to_string());
                        return Ok(());
                    }
                    if parts[0] != key {
                        log::warn!("Invalid key");
                        let _ = notify_tx.send("Invalid key.".to_string());
                        return Ok(());
                    }

                    let ssid = parts[1].to_string();
                    let password = parts[2].to_string();

                    // Run wifi config in background (auto-detects NM vs wpa)
                    let tx = notify_tx.clone();
                    tokio::spawn(async move {
                        let msg = wifi::set_wifi(&ssid, &password).await;
                        let _ = tx.send(msg);
                    });

                    Ok(())
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Build the Notify Message characteristic (notify).
/// Polls the watch channel for new messages and sends them to subscribed clients.
pub fn build_notify_message(notify_rx: watch::Receiver<String>) -> Characteristic {
    Characteristic {
        uuid: suuid::parse_uuid(suuid::NOTIFY_MESSAGE),
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                let mut rx = notify_rx.clone();
                async move {
                    log::info!("NotifyMessage subscriber connected");
                    loop {
                        // Wait for value changes
                        if rx.changed().await.is_err() {
                            break;
                        }
                        let msg = rx.borrow_and_update().clone();
                        if msg.is_empty() {
                            continue;
                        }
                        log::info!("NotifyMessage sending: {}", msg);
                        if notifier.notify(msg.as_bytes().to_vec()).await.is_err() {
                            log::info!("NotifyMessage subscriber disconnected");
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
