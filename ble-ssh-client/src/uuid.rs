use uuid::Uuid;

/// Parse a hex UUID string into a standard 128-bit UUID.
/// Matches server-side parse_uuid: left-pads short UUIDs with zeros to 32 hex chars.
pub fn parse_uuid(hex: &str) -> Uuid {
    let padded = format!("{:0>32}", hex);
    let formatted = format!(
        "{}-{}-{}-{}-{}",
        &padded[0..8],
        &padded[8..12],
        &padded[12..16],
        &padded[16..20],
        &padded[20..32],
    );
    Uuid::parse_str(&formatted).expect("invalid UUID hex string")
}

pub const SERVICE_ID: &str = "FD2B4448AA0F4A15A62FEB0BE77A0000";
pub const SSH_CTRL: &str = "FD2B4448AA0F4A15A62FEB0BE77A000A";
pub const SSH_RX: &str = "FD2B4448AA0F4A15A62FEB0BE77A000B";
pub const SSH_TX: &str = "FD2B4448AA0F4A15A62FEB0BE77A000C";
pub const SSH_USERNAME: &str = "FD2B4448AA0F4A15A62FEB0BE77A000D";
