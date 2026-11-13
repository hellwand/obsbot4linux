// SPDX-License-Identifier: GPL-3.0-or-later

//! The JSON schema of protocol 2, `schema/protocol-2.json` (design spec, sections 3 and 4).
//! It covers every line on the socket (requests, replies, events) and, through `Device`, the
//! description format. It is generated from the types of this crate and checked for staleness
//! by `tests/schema.rs`.

use crate::description::Description;
use crate::message::{Request, ServerMessage};
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};

/// The schema as a JSON value.
pub fn protocol_schema() -> Value {
    let mut generator = SchemaSettings::draft2020_12().into_generator();
    let request = generator.subschema_for::<Request>();
    let server = generator.subschema_for::<ServerMessage>();
    generator.subschema_for::<Description>();
    let defs = generator.take_definitions(true);
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "obsbot4linux protocol 2",
        "description": "One JSON object per line on the socket: a Request from a client, or a ServerMessage (a Reply or an Event) from the daemon. Description is the content of descriptions/<model>.json.",
        "oneOf": [request, server],
        "$defs": defs,
    })
}

/// The file content: pretty JSON with a final newline.
pub fn schema_text() -> String {
    let mut text =
        serde_json::to_string_pretty(&protocol_schema()).expect("a JSON value always serialises");
    text.push('\n');
    text
}
