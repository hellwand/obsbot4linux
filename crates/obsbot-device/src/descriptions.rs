// SPDX-License-Identifier: GPL-3.0-or-later

//! The generated description files (design spec 3): `descriptions/<model>.json` for every
//! real model and `descriptions/fake/<model>.json` for the fakes. `tests/generated.rs` fails
//! when a file differs from the code; `scripts/regenerate.sh` rewrites them. A new model adds
//! its line to [`all`].

use crate::models::{fake, tiny2lite};
use obsbot_proto::Description;

/// (file path relative to the repository root, description), in a fixed order.
pub fn all() -> Vec<(&'static str, Description)> {
    vec![
        (
            "descriptions/tiny2lite.json",
            tiny2lite::description::description(),
        ),
        ("descriptions/fake/fake-a.json", fake::a::description()),
        ("descriptions/fake/fake-b.json", fake::b::description()),
    ]
}

/// The file content of one description: pretty JSON with a final newline.
pub fn render(description: &Description) -> String {
    let mut text =
        serde_json::to_string_pretty(description).expect("a description always serialises");
    text.push('\n');
    text
}

/// `Ok` when `have` (the file content) equals the rendered description, else the reason.
pub fn compare(path: &str, description: &Description, have: &str) -> Result<(), String> {
    if have == render(description) {
        Ok(())
    } else {
        Err(format!(
            "{path} is stale: run scripts/regenerate.sh and commit the result"
        ))
    }
}
