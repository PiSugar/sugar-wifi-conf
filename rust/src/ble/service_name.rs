use bluer::gatt::local::{Characteristic, CharacteristicRead, CharacteristicReadRequest};
use futures::FutureExt;

use crate::uuid as suuid;

/// Build the Service Name characteristic (read-only, static).
pub fn build() -> Characteristic {
    Characteristic {
        uuid: suuid::parse_uuid(suuid::SERVICE_NAME),
        read: Some(CharacteristicRead {
            read: true,
            fun: Box::new(move |_req: CharacteristicReadRequest| {
                async move {
                    log::debug!("ServiceName read request");
                    Ok(b"PiSugar BLE Wifi Config".to_vec())
                }
                .boxed()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}
