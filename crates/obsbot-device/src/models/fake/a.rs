// SPDX-License-Identifier: GPL-3.0-or-later

//! `fake-a`: the Tiny 2 Lite's description in memory, for client work (5.12).

use crate::contract::{IdentitySource, ModelEntry, Motion, Target, Tier, UsbId};
use crate::models::fake::camera::{Shape, initial_values, live_pose, needs_item, plan_by_segment};
use crate::models::fake::{UNKNOWN_PRODUCT, VENDOR};
use crate::models::tiny2lite;
use obsbot_proto::validate::Refusal;
use obsbot_proto::{Description, ErrorCode, Params, State, Support, Values};
use serde_json::{Value, json};
use std::sync::OnceLock;

pub const PRODUCT: u16 = 0xfa0a;
const _: () = assert!(PRODUCT != UNKNOWN_PRODUCT);

pub const ENTRY: ModelEntry = ModelEntry {
    key: "fake-a",
    display_name: "Fake A",
    usb: &[UsbId {
        vendor: VENDOR,
        product: PRODUCT,
    }],
    support: Support::Tested,
    identity: IdentitySource::Query,
    firmware_query: true,
    identity_tier: Tier::FiveSeconds,
    xu_guid: None,
    selectors: &[],
};

pub fn description() -> Description {
    tiny2lite::description::description()
}

fn touched(target: Target<'_>) -> Vec<&'static str> {
    match target {
        Target::Leaf("tracking.mode") => vec!["framing"],
        Target::Leaf(l) if l.starts_with("gestures.") => vec!["zoom"],
        Target::Leaf(_) => vec![],
        Target::Action(a) => match a {
            "preset.recall" | "view.go" => vec!["zoom", "gimbal"],
            "preset.save" | "preset.rename" | "preset.delete" => vec!["presets"],
            "power_on.use_preset" => vec!["presets", "power_on"],
            "power_on.set_pose" | "power_on.use_current" => vec!["power_on"],
            "view.save" | "view.rename" | "view.delete" => vec!["views"],
            "zone.return_here" => vec!["zone"],
            "hand.return_here" | "hand.return_clear" | "hand.area_reset" => vec!["hand"],
            "focus.point" => vec!["focus"],
            "sleep" | "wake" => vec!["status"],
            _ => vec![],
        },
    }
}

/// The Tiny 2 Lite's motion actions (4.1 rule 6) and stops (4.1 rule 2).
const MOTION: &[(&str, Motion)] = &[
    ("gimbal.move", Motion::Move),
    (
        "gimbal.turn",
        Motion::Continuing {
            stop: "gimbal.stop",
        },
    ),
    ("zoom.move", Motion::Move),
    ("preset.recall", Motion::Move),
    ("view.go", Motion::Move),
    ("zone.go_return", Motion::Move),
    ("hand.go_return", Motion::Move),
    ("power_on.go", Motion::Move),
    ("gimbal.stop", Motion::Stop),
    ("zoom.stop", Motion::Stop),
];

fn set_pose(values: &mut Values, pose: &Value) {
    values.insert("gimbal.pan".into(), pose["pan"].clone());
    values.insert("gimbal.tilt".into(), pose["tilt"].clone());
    values.insert("zoom.factor".into(), pose["zoom"].clone());
}

fn list(values: &mut Values, path: &str) -> Vec<Value> {
    values
        .get(path)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn act(values: &mut Values, action: &str, p: &Params) -> Option<Params> {
    let pose = |values: &Values| p.get("pose").cloned().unwrap_or_else(|| live_pose(values));
    match action {
        // A move sends only the move (spec 2): the fake sets the pose and leaves
        // `tracking.mode` as it is, whatever the mode. That is the fake's own behaviour, not
        // the camera's; what the camera does with a move while it tracks is CAM-34.
        // `gimbal.turn` changes nothing the fake stores (it has no turning pose).
        "gimbal.move" => {
            for (param, leaf) in [("pan", "gimbal.pan"), ("tilt", "gimbal.tilt")] {
                if let Some(v) = p.get(param) {
                    values.insert(leaf.into(), v.clone());
                }
            }
        }
        "zoom.move" => {
            values.insert("zoom.factor".into(), p["factor"].clone());
        }
        "preset.save" => {
            let slot = p["slot"].as_u64().unwrap_or(0);
            // The fake's own behaviour: an unnamed preset has an empty name. What the
            // camera reports for one is CAM-35; the daemon never invents a name (spec 2).
            let name = p.get("name").cloned().unwrap_or_else(|| json!(""));
            let item = json!({"slot": slot, "name": name, "pose": pose(values)});
            let mut items: Vec<Value> = list(values, "presets")
                .into_iter()
                .filter(|i| i["slot"] != json!(slot))
                .collect();
            items.push(item);
            items.sort_by_key(|i| i["slot"].as_u64());
            values.insert("presets".into(), Value::Array(items));
        }
        "preset.recall" | "view.go" => {
            let (path, key, wanted) = if action == "view.go" {
                ("views", "id", &p["id"])
            } else {
                ("presets", "slot", &p["slot"])
            };
            if let Some(item) = list(values, path).into_iter().find(|i| &i[key] == wanted) {
                set_pose(values, &item["pose"]);
            }
        }
        "preset.rename" | "view.rename" | "preset.delete" | "view.delete" => {
            let (path, key) = if action.starts_with("view.") {
                ("views", "id")
            } else {
                ("presets", "slot")
            };
            let wanted = &p[key];
            let mut items = list(values, path);
            if action.ends_with(".delete") {
                items.retain(|i| &i[key] != wanted);
            } else if let Some(item) = items.iter_mut().find(|i| &i[key] == wanted) {
                item["name"] = p["name"].clone();
            }
            values.insert(path.into(), Value::Array(items));
        }
        "view.save" => {
            let mut items = list(values, "views");
            let id = (0u64..)
                .find(|n| !items.iter().any(|i| i["id"] == json!(n)))
                .unwrap_or(0);
            items.push(json!({"id": id, "name": p["name"], "pose": pose(values)}));
            values.insert("views".into(), Value::Array(items));
            return Some(json!({"id": id}).as_object().cloned().unwrap_or_default());
        }
        "zone.return_here" => {
            values.insert("zone.return".into(), live_pose(values));
        }
        "hand.return_here" => {
            values.insert("hand.return".into(), live_pose(values));
        }
        "hand.return_clear" => {
            values.insert("hand.return".into(), Value::Null);
        }
        "hand.area_reset" => {
            values.insert("hand.area.pan".into(), json!({"min": -45, "max": 45}));
            values.insert("hand.area.tilt".into(), json!({"min": -30, "max": 30}));
        }
        "focus.point" => {
            values.insert("focus.mode".into(), json!("single"));
        }
        "power_on.set_pose" | "power_on.use_current" => {
            values.insert("power_on.start.pose".into(), pose(values));
            values.insert("power_on.start.source".into(), json!("own"));
        }
        "power_on.use_preset" => {
            let slot = p["slot"].clone();
            if let Some(item) = list(values, "presets")
                .into_iter()
                .find(|i| i["slot"] == slot)
            {
                values.insert("power_on.start.pose".into(), item["pose"].clone());
                values.insert(
                    "power_on.start.source".into(),
                    json!(format!("preset_{slot}")),
                );
            }
        }
        _ => {}
    }
    None
}

fn check(action: &str, p: &Params, state: &State) -> Result<(), Refusal> {
    match action {
        "gimbal.move" if !p.contains_key("pan") && !p.contains_key("tilt") => Err(Refusal::new(
            ErrorCode::Invalid,
            "gimbal.move",
            "gimbal.move needs pan, tilt or both",
        )),
        "preset.save" => needs_item(state, "presets", "slot", None),
        "preset.recall" | "preset.rename" | "preset.delete" | "power_on.use_preset" => {
            needs_item(state, "presets", "slot", p.get("slot"))
        }
        "view.save" => needs_item(state, "views", "id", None),
        "view.go" | "view.rename" | "view.delete" => needs_item(state, "views", "id", p.get("id")),
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
                ("tracking", Tier::Second, false, None),
                (
                    "gimbal",
                    Tier::FiveSeconds,
                    false,
                    Some(("pose", Tier::Second)),
                ),
                ("presets", Tier::Touched, false, None),
                ("views", Tier::Minute, false, None),
                ("zone", Tier::Minute, false, None),
                ("hand", Tier::Minute, false, None),
                ("power_on", Tier::Minute, false, None),
            ],
            Tier::FiveSeconds,
        );
        let initial = initial_values(&description);
        Shape {
            key: "fake-a",
            description,
            plan,
            initial,
            serial_prefix: "FAKEA",
            firmware: "1.0.0.0",
            motion: MOTION,
            touched,
            lag: &[],
            floor: &[],
            act,
            check,
        }
    })
}
