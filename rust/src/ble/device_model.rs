use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicReadRequest};
use futures::FutureExt;
use std::process::Command;

use crate::uuid as suuid;

/// Build the Device Model characteristic (read-only).
/// Reads from /proc/device-tree/model on Raspberry Pi.
pub fn build() -> Characteristic {
    Characteristic {
        uuid: suuid::parse_uuid(suuid::DEVICE_MODEL),
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |_req: CharacteristicReadRequest| {
                async move {
                    let model = match std::fs::read_to_string("/proc/device-tree/model") {
                        Ok(m) => m.trim_end_matches('\0').to_string(),
                        Err(_) => {
                            // Fallback: try cat command
                            match Command::new("cat")
                                .arg("/proc/device-tree/model")
                                .output()
                            {
                                Ok(out) => String::from_utf8_lossy(&out.stdout)
                                    .trim_end_matches('\0')
                                    .to_string(),
                                Err(_) => "Unknown".to_string(),
                            }
                        }
                    };
                    log::debug!("DeviceModel read: {}", model);
                    Ok(model.into_bytes())
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}
