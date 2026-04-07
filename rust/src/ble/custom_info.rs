use bluer::gatt::local::{
    Characteristic, CharacteristicNotify, CharacteristicNotifyMethod, CharacteristicRead,
    CharacteristicReadRequest,
};
use futures::FutureExt;
use std::process::Command;
use tokio::time::{interval, Duration};

use crate::config::InfoItem;
use crate::uuid as suuid;

/// Build all custom info characteristics from config.
/// For each info item:
///   - A label characteristic (read): returns the label string
///   - A value characteristic (notify): polls shell command at specified interval
/// Plus a count characteristic (read): total number of info items.
pub fn build(items: &[InfoItem]) -> Vec<Characteristic> {
    let mut chars = Vec::new();

    for (index, item) in items.iter().enumerate() {
        let label = item.label.clone();
        let command = item.command.clone();
        let interval_secs = item.interval;

        // Label characteristic (read)
        let label_data = label.clone();
        chars.push(Characteristic {
            uuid: suuid::custom_uuid(suuid::CUSTOM_INFO_LABEL_PREFIX, index),
            read: Some(CharacteristicRead {
                read: true,
                fun: Box::new(move |_req: CharacteristicReadRequest| {
                    let data = label_data.clone();
                    async move {
                        log::debug!("CustomInfo label read");
                        Ok(data.into_bytes())
                    }
                    .boxed()
                }),
                ..Default::default()
            }),
            ..Default::default()
        });

        // Value characteristic (notify)
        chars.push(Characteristic {
            uuid: suuid::custom_uuid(suuid::CUSTOM_INFO_PREFIX, index),
            notify: Some(CharacteristicNotify {
                notify: true,
                method: CharacteristicNotifyMethod::Fun(Box::new(move |mut notifier| {
                    let cmd = command.clone();
                    let label_clone = label.clone();
                    let secs = interval_secs;
                    async move {
                        log::info!("CustomInfo '{}' subscriber connected", label_clone);
                        let mut tick = interval(Duration::from_secs(secs));

                        // Send initial value
                        let val = exec_shell_command(&cmd);
                        if notifier.notify(val.as_bytes().to_vec()).await.is_err() {
                            return;
                        }

                        loop {
                            tick.tick().await;
                            let val = exec_shell_command(&cmd);
                            if notifier.notify(val.as_bytes().to_vec()).await.is_err() {
                                log::info!("CustomInfo subscriber disconnected");
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
    }

    // Count characteristic (read)
    let count = items.len();
    chars.push(Characteristic {
        uuid: suuid::parse_uuid(suuid::CUSTOM_INFO_COUNT),
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |_req: CharacteristicReadRequest| {
                async move {
                    log::debug!("CustomInfo count read");
                    Ok(format!("{}", count).into_bytes())
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    });

    chars
}

/// Execute a shell command and return its stdout, or "cmd error" on failure.
fn exec_shell_command(command: &str) -> String {
    match Command::new("sh").args(["-c", command]).output() {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        Ok(out) => {
            log::warn!(
                "Command '{}' failed: {}",
                command,
                String::from_utf8_lossy(&out.stderr)
            );
            "cmd error".to_string()
        }
        Err(e) => {
            log::error!("Failed to execute '{}': {}", command, e);
            "cmd error".to_string()
        }
    }
}
