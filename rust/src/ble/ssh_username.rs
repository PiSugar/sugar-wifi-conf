use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicReadRequest};
use futures::FutureExt;

use crate::uuid as suuid;

/// Find the first normal user (UID >= 1000) with a real login shell.
/// Parses /etc/passwd to find users suitable for SSH login.
/// Falls back to "pi" if no suitable user is found.
fn detect_ssh_user() -> String {
    if let Ok(contents) = std::fs::read_to_string("/etc/passwd") {
        for line in contents.lines() {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() < 7 {
                continue;
            }
            let uid: u32 = match fields[2].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let shell = fields[6];
            // Normal user: UID >= 1000 and < 65534 (nobody), with a real shell
            if uid >= 1000
                && uid < 65534
                && !shell.ends_with("/nologin")
                && !shell.ends_with("/false")
            {
                return fields[0].to_string();
            }
        }
    }
    "pi".to_string()
}

/// Build the SSH Username characteristic (read-only).
/// Returns the detected SSH login username for this device.
pub fn build() -> Characteristic {
    let username = detect_ssh_user();
    log::info!("SSH username: {}", username);

    Characteristic {
        uuid: suuid::parse_uuid(suuid::SSH_USERNAME),
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |_req: CharacteristicReadRequest| {
                let user = username.clone();
                async move {
                    log::debug!("SSH_USERNAME read: {}", user);
                    Ok(user.into_bytes())
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}
