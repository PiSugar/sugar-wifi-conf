pub mod server;
pub mod service_name;
pub mod device_model;
pub mod wifi_name;
pub mod ip_address;
pub mod input_notify;
pub mod custom_info;
pub mod custom_command;
pub mod ssh_tunnel;
pub mod ssh_username;

/// Protocol constants matching the JS client protocol.
pub const CONCAT_TAG: &str = "%&%";
pub const END_TAG: &str = "&#&";
pub const CHUNK_SIZE: usize = 20;

/// Split a string into chunks of CHUNK_SIZE bytes, appending END_TAG.
pub fn chunk_message(msg: &str) -> Vec<Vec<u8>> {
    let full = format!("{}{}", msg, END_TAG);
    full.as_bytes()
        .chunks(CHUNK_SIZE)
        .map(|c| c.to_vec())
        .collect()
}
