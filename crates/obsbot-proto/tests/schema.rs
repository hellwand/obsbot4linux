// SPDX-License-Identifier: GPL-3.0-or-later

//! Stale test of `schema/protocol-2.json`. `scripts/regenerate.sh` rewrites the file
//! (`OBSBOT_REGENERATE=1`); without it, any difference fails.

use obsbot_proto::schema::{protocol_schema, schema_text};
use std::path::PathBuf;

fn schema_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schema/protocol-2.json")
}

#[test]
fn schema_file_is_current() {
    let path = schema_file();
    let want = schema_text();
    if std::env::var_os("OBSBOT_REGENERATE").is_some() {
        std::fs::write(&path, &want).expect("write schema/protocol-2.json");
        return;
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        have == want,
        "schema/protocol-2.json is stale: run scripts/regenerate.sh and commit the result"
    );
}

#[test]
fn schema_names_every_envelope_and_the_description_format() {
    let schema = protocol_schema();
    let defs = schema["$defs"].as_object().expect("$defs");
    for name in [
        "Request",
        "Reply",
        "Event",
        "ErrorBody",
        "ErrorCode",
        "Device",
        "Description",
        "Setting",
        "Action",
        "State",
    ] {
        assert!(defs.contains_key(name), "the schema lacks {name}");
    }
    let codes = serde_json::to_string(&defs["ErrorCode"]).unwrap();
    for code in obsbot_proto::ErrorCode::ALL {
        assert!(
            codes.contains(&format!("\"{}\"", code.as_str())),
            "ErrorCode lacks {}",
            code.as_str()
        );
    }
}
