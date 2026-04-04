use uuid::Uuid;

/// Parse a hex UUID string into a bluer-compatible 128-bit UUID.
/// The JS bleno library left-pads short UUIDs with zeros to 32 hex chars.
/// E.g., "FD2BCCCA0001" (12 chars) → "00000000000000000000FD2BCCCA0001" (32 chars)
/// Full 32-char UUIDs like "FD2B4448AA0F4A15A62FEB0BE77A0000" are used as-is.
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

// Service UUID (32 hex chars = full 128-bit)
pub const SERVICE_ID: &str = "FD2B4448AA0F4A15A62FEB0BE77A0000";

// Standard characteristic UUIDs (32 hex chars)
pub const SERVICE_NAME: &str = "FD2B4448AA0F4A15A62FEB0BE77A0001";
pub const DEVICE_MODEL: &str = "FD2B4448AA0F4A15A62FEB0BE77A0002";
pub const WIFI_NAME: &str = "FD2B4448AA0F4A15A62FEB0BE77A0003";
pub const IP_ADDRESS: &str = "FD2B4448AA0F4A15A62FEB0BE77A0004";
pub const INPUT: &str = "FD2B4448AA0F4A15A62FEB0BE77A0005";
pub const NOTIFY_MESSAGE: &str = "FD2B4448AA0F4A15A62FEB0BE77A0006";
pub const INPUT_SEP: &str = "FD2B4448AA0F4A15A62FEB0BE77A0007";
pub const CUSTOM_COMMAND_INPUT: &str = "FD2B4448AA0F4A15A62FEB0BE77A0008";
pub const CUSTOM_COMMAND_NOTIFY: &str = "FD2B4448AA0F4A15A62FEB0BE77A0009";

// SSH tunnel characteristic UUIDs
pub const SSH_CTRL: &str = "FD2B4448AA0F4A15A62FEB0BE77A000A";
pub const SSH_RX: &str = "FD2B4448AA0F4A15A62FEB0BE77A000B";
pub const SSH_TX: &str = "FD2B4448AA0F4A15A62FEB0BE77A000C";

// Custom info/command UUID prefixes (8 hex chars, appended with guid4(index) = 12 chars total)
pub const CUSTOM_INFO_LABEL_PREFIX: &str = "FD2BCCCA";
pub const CUSTOM_INFO_PREFIX: &str = "FD2BCCCB";
pub const CUSTOM_COMMAND_LABEL_PREFIX: &str = "FD2BCCCC";

// Custom info/command count UUIDs (12 hex chars, left-padded to 32 by bleno)
pub const CUSTOM_INFO_COUNT: &str = "FD2BCCAA0000";
pub const CUSTOM_COMMAND_COUNT: &str = "FD2BCCAC0000";

/// Generate a 4-character hex suffix for dynamic UUIDs.
/// Mirrors the JS `guid4(index)`: `(index + 1).toString(16)` zero-padded to 4 chars.
pub fn guid4(index: usize) -> String {
    format!("{:04x}", index + 1)
}

/// Build a 12-char custom UUID string from an 8-char prefix and a 4-char suffix.
/// E.g., prefix="FD2BCCCA", suffix="0001" => "FD2BCCCA0001"
/// Use `parse_uuid()` to convert to a full 128-bit UUID (left-padded to 32 chars).
pub fn custom_uuid_str(prefix: &str, suffix: &str) -> String {
    format!("{}{}", prefix, suffix)
}

/// Build and parse a custom UUID from prefix + index.
pub fn custom_uuid(prefix: &str, index: usize) -> Uuid {
    parse_uuid(&custom_uuid_str(prefix, &guid4(index)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_uuid() {
        let uuid = parse_uuid(SERVICE_ID);
        assert_eq!(uuid.to_string(), "fd2b4448-aa0f-4a15-a62f-eb0be77a0000");
    }

    #[test]
    fn test_parse_short_uuid() {
        // 12-char UUID gets left-padded with zeros
        let uuid = parse_uuid("FD2BCCCA0001");
        assert_eq!(uuid.to_string(), "00000000-0000-0000-0000-fd2bccca0001");
    }

    #[test]
    fn test_guid4() {
        assert_eq!(guid4(0), "0001");
        assert_eq!(guid4(1), "0002");
        assert_eq!(guid4(15), "0010");
        assert_eq!(guid4(255), "0100");
    }

    #[test]
    fn test_custom_uuid() {
        let uuid = custom_uuid("FD2BCCCA", 0);
        assert_eq!(uuid.to_string(), "00000000-0000-0000-0000-fd2bccca0001");
    }
}
