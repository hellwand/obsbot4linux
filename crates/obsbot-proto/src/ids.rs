// SPDX-License-Identifier: GPL-3.0-or-later

//! Request ids, device ids and the one formatter that shortens serials (4.2, 5.5).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The `id` of a request: an integer or a string, echoed verbatim in its reply.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RequestId {
    Int(i64),
    Str(String),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::Int(n) => write!(f, "{n}"),
            RequestId::Str(s) => write!(f, "{s:?}"),
        }
    }
}

/// Whether a serial is valid for an `sn:` id: `[A-Za-z0-9]{8,32}` (5.5). The pattern is part
/// of the id format of protocol 2.
pub fn is_valid_serial(serial: &str) -> bool {
    (8..=32).contains(&serial.len()) && serial.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `sn:<serial>`; the caller has checked the serial with [`is_valid_serial`].
pub fn sn_id(serial: &str) -> String {
    format!("sn:{serial}")
}

/// `port:<usb path>`, for a device without a valid serial.
pub fn port_id(usb_path: &str) -> String {
    format!("port:{usb_path}")
}

/// The id or serial as human output, logs and the journal show it (4.2): an `sn:` id or a bare
/// serial becomes `…` plus its last four characters; a `port:` id is shown as it is.
pub fn short_id(id_or_serial: &str) -> String {
    if id_or_serial.starts_with("port:") {
        return id_or_serial.to_string();
    }
    let serial = id_or_serial.strip_prefix("sn:").unwrap_or(id_or_serial);
    let chars: Vec<char> = serial.chars().collect();
    let tail: String = chars[chars.len().saturating_sub(4)..].iter().collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serials_follow_the_id_format_of_protocol_2() {
        assert!(is_valid_serial("EXAMPLE0000001"));
        assert!(is_valid_serial("ABCD1234"));
        assert!(!is_valid_serial("ABC1234"), "7 characters is too short");
        assert!(
            !is_valid_serial(&"A".repeat(33)),
            "33 characters is too long"
        );
        assert!(
            !is_valid_serial("EXAMPLE-000001"),
            "only letters and digits"
        );
        assert!(!is_valid_serial(""));
    }

    #[test]
    fn human_output_never_shows_more_than_four_serial_characters() {
        assert_eq!(short_id("sn:EXAMPLE0000001"), "…0001");
        assert_eq!(short_id("EXAMPLE0000001"), "…0001");
        assert_eq!(short_id("port:3-2.1"), "port:3-2.1");
        assert_eq!(sn_id("EXAMPLE0000001"), "sn:EXAMPLE0000001");
        assert_eq!(port_id("3-2.1"), "port:3-2.1");
    }
}
