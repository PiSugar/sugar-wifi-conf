use std::sync::{Arc, Mutex};
use std::time::Instant;

use bluer::gatt::local::{
    Characteristic, CharacteristicNotify, CharacteristicNotifyMethod, CharacteristicRead,
    CharacteristicReadRequest, CharacteristicWrite, CharacteristicWriteMethod,
    CharacteristicWriteRequest,
};
use futures::FutureExt;
use tokio::sync::watch;
use tokio::time::{interval, Duration};

use crate::config::CommandItem;
use crate::uuid as suuid;

use super::{chunk_message, CONCAT_TAG, END_TAG};

/// Stored command info with its full UUID string for matching.
#[derive(Clone)]
struct CommandEntry {
    uuid_str: String,
    command: String,
}

/// Build all custom command characteristics from config.
/// For each command:
///   - A label characteristic (read): returns the command label
/// Plus:
///   - A count characteristic (read): total number of commands
///   - An input characteristic (write, chunked): receives `key%&%uuid_suffix&#&`
///   - A notify characteristic (notify): streams command output in 20-byte chunks
pub fn build(
    items: &[CommandItem],
    key: String,
    cmd_notify_tx: Arc<watch::Sender<String>>,
    cmd_notify_rx: watch::Receiver<String>,
) -> Vec<Characteristic> {
    let mut chars = Vec::new();
    let mut entries = Vec::new();

    for (index, item) in items.iter().enumerate() {
        let label = item.label.clone();
        let uuid_str = suuid::custom_uuid_str(suuid::CUSTOM_COMMAND_LABEL_PREFIX, &suuid::guid4(index));

        entries.push(CommandEntry {
            uuid_str: uuid_str.clone(),
            command: item.command.clone(),
        });

        // Label characteristic (read)
        let label_data = label.clone();
        chars.push(Characteristic {
            uuid: suuid::custom_uuid(suuid::CUSTOM_COMMAND_LABEL_PREFIX, index),
            read: Some(CharacteristicRead {
                read: true,
                fun: Box::new(move |_req: CharacteristicReadRequest| {
                    let data = label_data.clone();
                    async move {
                        log::debug!("CustomCommand label read");
                        Ok(data.into_bytes())
                    }
                    .boxed()
                }),
                ..Default::default()
            }),
            ..Default::default()
        });
    }

    // Count characteristic (read)
    let count = items.len();
    chars.push(Characteristic {
        uuid: suuid::parse_uuid(suuid::CUSTOM_COMMAND_COUNT),
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |_req: CharacteristicReadRequest| {
                async move {
                    log::debug!("CustomCommand count read");
                    Ok(format!("{}", count).into_bytes())
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    });

    // Input characteristic (write, chunked) for executing commands
    let buffer = Arc::new(Mutex::new(String::new()));
    let last_change = Arc::new(Mutex::new(Instant::now()));
    let entries = Arc::new(entries);

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
                log::debug!("Clear custom command input buffer (timeout)");
                buf.clear();
            }
        }
    });

    chars.push(Characteristic {
        uuid: suuid::parse_uuid(suuid::CUSTOM_COMMAND_INPUT),
        write: Some(CharacteristicWrite {
            write: true,
            write_without_response: true,
            method: CharacteristicWriteMethod::Fun(Box::new(move |data: Vec<u8>, _req: CharacteristicWriteRequest| {
                let key = key.clone();
                let buffer = buffer.clone();
                let last_change = last_change.clone();
                let entries = entries.clone();
                let tx = cmd_notify_tx.clone();
                async move {
                    let chunk = String::from_utf8_lossy(&data).to_string();
                    log::debug!("CustomCommand input write: {}", chunk);

                    let mut buf = buffer.lock().unwrap();
                    buf.push_str(&chunk);
                    *last_change.lock().unwrap() = Instant::now();

                    let is_last = buf.contains(END_TAG);
                    if !is_last {
                        return Ok(());
                    }

                    let full = buf.replace(END_TAG, "");
                    buf.clear();
                    drop(buf);

                    let parts: Vec<&str> = full.split(CONCAT_TAG).collect();
                    if parts.len() < 2 {
                        log::warn!("Invalid syntax");
                        let _ = tx.send("Invalid syntax.".to_string());
                        return Ok(());
                    }
                    if parts[0] != key {
                        log::warn!("Invalid key");
                        let _ = tx.send("Invalid key.".to_string());
                        return Ok(());
                    }

                    // Extract UUID suffix from input
                    let command_uuid = parts[1]
                        .split('-')
                        .last()
                        .unwrap_or("")
                        .to_uppercase();

                    // Find matching command
                    let command_to_execute = entries.iter().find(|e| {
                        let e_upper = e.uuid_str.to_uppercase();
                        e_upper == command_uuid || e_upper.get(8..).map_or(false, |s| s == command_uuid)
                    }).map(|e| e.command.clone());

                    let tx2 = tx.clone();
                    tokio::spawn(async move {
                        if let Some(cmd) = command_to_execute {
                            // Send "exec done." first like the JS version
                            let _ = tx2.send("exec done.\n".to_string());

                            match std::process::Command::new("sh")
                                .args(["-c", &cmd])
                                .output()
                            {
                                Ok(out) => {
                                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                                    // Send output in chunks with delays
                                    let chunks = chunk_message(&stdout);
                                    for chunk in chunks {
                                        let msg = String::from_utf8_lossy(&chunk).to_string();
                                        let _ = tx2.send(msg);
                                        tokio::time::sleep(Duration::from_millis(200)).await;
                                    }
                                }
                                Err(e) => {
                                    let msg = format!("exec error: {}", e);
                                    let _ = tx2.send(msg);
                                }
                            }
                        } else {
                            let _ = tx2.send("Command not found.".to_string());
                        }
                    });

                    Ok(())
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    });

    // Notify characteristic for command output
    chars.push(Characteristic {
        uuid: suuid::parse_uuid(suuid::CUSTOM_COMMAND_NOTIFY),
        notify: Some(CharacteristicNotify {
            notify: true,
            method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                let mut rx = cmd_notify_rx.clone();
                async move {
                    log::info!("CustomCommand notify subscriber connected");
                    loop {
                        if rx.changed().await.is_err() {
                            break;
                        }
                        let msg = rx.borrow_and_update().clone();
                        if msg.is_empty() {
                            continue;
                        }
                        log::debug!("CustomCommand notify: {}", msg);
                        if notifier.notify(msg.as_bytes().to_vec()).await.is_err() {
                            log::info!("CustomCommand notify subscriber disconnected");
                            return;
                        }
                    }
                }
                .boxed()
            })),
            ..Default::default()
        }),
        ..Default::default()
    });

    chars
}
