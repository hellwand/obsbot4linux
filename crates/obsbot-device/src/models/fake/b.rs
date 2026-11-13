// SPDX-License-Identifier: GPL-3.0-or-later

//! `fake-b`: a model deliberately unlike the Tiny 2 Lite (5.12), so that the daemon, the CLI
//! and the widget are built against two shapes from the start.

use crate::contract::{IdentitySource, ModelEntry, Motion, Target, Tier, UsbId};
use crate::models::fake::camera::{Shape, initial_values, needs_item, plan_by_segment};
use crate::models::fake::{UNKNOWN_PRODUCT, VENDOR};
use obsbot_proto::validate::Refusal;
use obsbot_proto::{Action, Asleep, Description, Params, Setting, State, Support, Unit, Values};
use serde_json::{Value, json};
use std::sync::OnceLock;
use std::time::Duration;

pub const PRODUCT: u16 = 0xfa0b;
const _: () = assert!(PRODUCT != UNKNOWN_PRODUCT);

/// How long the readback of `led.brightness` lags a write.
pub const LAG: Duration = Duration::from_millis(250);
/// The settle bound of every writable leaf with a readback.
pub const SETTLE_MS: u32 = 1000;
/// The camera stores `exposure.time` rounded down to this step.
pub const TIME_STEP: f64 = 100.0;

pub const ENTRY: ModelEntry = ModelEntry {
    key: "fake-b",
    display_name: "Fake B",
    usb: &[UsbId {
        vendor: VENDOR,
        product: PRODUCT,
    }],
    support: Support::Untested,
    identity: IdentitySource::Query,
    firmware_query: true,
    identity_tier: Tier::FiveSeconds,
    xu_guid: None,
    selectors: &[],
};

fn w(s: Setting, asleep: Asleep) -> Setting {
    s.asleep(asleep).settle_ms(SETTLE_MS)
}

pub fn description() -> Description {
    Description::default()
        .setting("asleep", Setting::boolean("Asleep").readonly())
        .setting("on_air.active", Setting::boolean("On air").readonly())
        .setting(
            "on_air.apps",
            Setting::list(
                "Applications using the camera",
                &[("name", Setting::string("Process name", 255, false))],
            ),
        )
        .setting(
            "tracking.mode",
            w(
                Setting::enumeration("Tracking", &["off", "person", "desk"]),
                Asleep::Unknown,
            ),
        )
        .setting(
            "zoom.factor",
            w(
                Setting::number("Zoom", 1.0, 2.0)
                    .unit(Unit::Factor)
                    .precision(0.01),
                Asleep::Ignored,
            ),
        )
        .setting(
            "led.brightness",
            w(
                Setting::number("LED brightness", 0.0, 5.0)
                    .step(1.0)
                    .precision(1.0),
                Asleep::Unknown,
            ),
        )
        .setting(
            "led.blink",
            Setting::boolean("LED blink")
                .asleep(Asleep::Accepted)
                .no_readback(),
        )
        .setting(
            "exposure.mode",
            w(
                Setting::enumeration("Exposure", &["auto", "manual"]),
                Asleep::Unknown,
            ),
        )
        .setting(
            "exposure.time",
            w(
                Setting::number("Exposure time", 100.0, 10_000.0)
                    .unit(Unit::Microseconds)
                    .precision(1.0),
                Asleep::Unknown,
            )
            .requires("exposure.mode", json!("manual"))
            .live("auto"),
        )
        .setting(
            "beacon.color",
            w(
                Setting::enumeration("Beacon colour", &["red", "green", "blue"]),
                Asleep::Accepted,
            ),
        )
        .setting(
            "beacon.pulse",
            Setting::boolean("Beacon pulse")
                .asleep(Asleep::Unknown)
                .no_readback(),
        )
        .setting(
            "presets",
            Setting::list(
                "Presets",
                &[
                    ("slot", Setting::number("Preset slot", 0.0, 4.0).step(1.0)),
                    ("name", Setting::string("Name", 16, true)),
                ],
            ),
        )
        .action(
            "preset.save",
            Action::new("Save a preset", Asleep::Ignored)
                .param("slot", Setting::number("Preset slot", 0.0, 4.0).step(1.0))
                .param("name", Setting::string("Name", 16, true).optional()),
        )
        .action(
            "preset.recall",
            Action::new("Recall a preset", Asleep::Unknown)
                .param("slot", Setting::number("Preset slot", 0.0, 4.0).step(1.0)),
        )
        .action(
            "preset.delete",
            Action::new("Delete a preset", Asleep::Unknown)
                .param("slot", Setting::number("Preset slot", 0.0, 4.0).step(1.0)),
        )
        .action(
            "beacon.flash",
            Action::new("Flash the beacon", Asleep::Accepted)
                .param("times", Setting::number("Times", 1.0, 3.0).step(1.0)),
        )
        .action("sleep", Action::new("Sleep", Asleep::Unknown))
        .action("wake", Action::new("Wake", Asleep::Accepted))
        .limit("presets", 5)
}

fn touched(target: Target<'_>) -> Vec<&'static str> {
    match target {
        Target::Leaf(_) => vec![],
        Target::Action("preset.save" | "preset.delete") => vec!["presets"],
        Target::Action("sleep" | "wake") => vec!["status"],
        Target::Action(_) => vec![],
    }
}

fn act(values: &mut Values, action: &str, p: &Params) -> Option<Params> {
    let mut items: Vec<Value> = values
        .get("presets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    match action {
        "preset.save" => {
            let slot = p["slot"].clone();
            // The fake's own behaviour: an unnamed preset has an empty name (CAM-35).
            let name = p.get("name").cloned().unwrap_or_else(|| json!(""));
            items.retain(|i| i["slot"] != slot);
            items.push(json!({"slot": slot, "name": name}));
            items.sort_by_key(|i| i["slot"].as_u64());
        }
        "preset.delete" => items.retain(|i| i["slot"] != p["slot"]),
        _ => return None,
    }
    values.insert("presets".into(), Value::Array(items));
    None
}

fn check(action: &str, p: &Params, state: &State) -> Result<(), Refusal> {
    match action {
        "preset.save" => needs_item(state, "presets", "slot", None),
        "preset.recall" | "preset.delete" => needs_item(state, "presets", "slot", p.get("slot")),
        _ => Ok(()),
    }
}

pub(crate) fn shape() -> &'static Shape {
    static SHAPE: OnceLock<Shape> = OnceLock::new();
    SHAPE.get_or_init(|| {
        let description = description();
        let plan = plan_by_segment(
            &description,
            &[
                ("status", Tier::Second, true, None),
                ("on_air", Tier::Second, false, None),
                ("led", Tier::Second, false, None),
                ("exposure", Tier::Second, false, None),
                ("beacon", Tier::Minute, false, None),
                ("presets", Tier::Touched, false, None),
            ],
            Tier::FiveSeconds,
        );
        let initial = initial_values(&description);
        Shape {
            key: "fake-b",
            description,
            plan,
            initial,
            serial_prefix: "FAKEB",
            firmware: "0.9.1",
            motion: &[("preset.recall", Motion::Move)],
            touched,
            lag: &[("led.brightness", LAG)],
            floor: &[("exposure.time", TIME_STEP)],
            act,
            check,
        }
    })
}
