// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite as data for the daemon (design spec 8.2, 8.8): its registry entry, its poll
//! plan, the groups each leaf and action touches, the apply order of a patch and the motion
//! marks of its actions. No I/O.

use crate::contract::{Group, IdentitySource, LiveTier, ModelEntry, Motion, Target, Tier, UsbId};
use obsbot_proto::Support;

/// The selector of framed commands, queries and their answers (8.8).
pub const FRAMED: u8 = 2;
/// The selector of short commands and the status block (8.8).
pub const SHORT: u8 = 6;
/// The selector of the preset list (count, slots), read and written back (8.8, CAM-19).
pub const PRESET_LIST: u8 = 0x0c;
/// The selector of the preset list's entries.
pub const PRESET_ENTRY: u8 = 0x0d;

/// Receivers of framed commands and queries (field ledger: receiver 4 unless named).
pub const TRACKING: u8 = 0x04;
pub const POWER: u8 = 0x02;
pub const GIMBAL: u8 = 0x03;
pub const IDENTITY: u8 = 0x0d;

/// The GUID of the extension unit; the USB descriptor recorded in capture 01 places it at
/// unit 2 with 19 controls.
pub const XU_GUID: &str = "9a1e7291-6843-4683-6d92-39bc7906ee49";

/// The registry entry (8.1, 8.2, 8.8). `support` is `untested` on `rebuild` until stage W3.
pub const ENTRY: ModelEntry = ModelEntry {
    key: "tiny2lite",
    display_name: "OBSBOT Tiny 2 Lite",
    usb: &[UsbId {
        vendor: 0x3564,
        product: 0xfef9,
    }],
    support: Support::Untested,
    identity: IdentitySource::Query,
    firmware_query: true,
    identity_tier: Tier::FiveSeconds,
    xu_guid: Some(XU_GUID),
    selectors: &[FRAMED, SHORT, PRESET_LIST, PRESET_ENTRY],
};

/// The leaves of the status block (group `status`), in block order.
pub const STATUS_LEAVES: [&str; 20] = [
    "asleep",
    "exposure.anti_flicker",
    "exposure.hdr",
    "exposure.region",
    "audio.noise_reduction",
    "sleep.auto.enabled",
    "sleep.auto.seconds",
    "focus.region",
    "audio.in_sleep",
    "zoom.field_of_view",
    "image.mirror",
    "sleep.wake_zoom",
    "tracking.mode",
    "audio.auto_gain",
    "sleep.background",
    "sleep.background_mirror",
    "image.frame_rate",
    "power_on.mode",
    "led.brightness",
    "tracking.profile",
];

/// The leaves of the `0x0104` answer (group `gestures`).
pub const GESTURE_LEAVES: [&str; 8] = [
    "hand.side",
    "gestures.target_lock",
    "gestures.zoom",
    "gestures.dynamic_zoom",
    "gestures.direction_change",
    "gestures.zoom_ratio",
    "zone.enabled",
    "gimbal.invert",
];

/// The leaves of the group `framing` and the query each reads back from.
pub const FRAMING_QUERIES: [(&str, u16); 7] = [
    ("tracking.speed", 0x0984),
    ("framing.auto_zoom", 0x0a84),
    ("framing.size", 0x0b04),
    ("framing.zoom_speed", 0x0b84),
    ("framing.offset.x", 0x0c04),
    ("framing.offset.y", 0x0c84),
    ("framing.part", 0x15c4),
];

fn group(name: &str, tier: Tier, asleep: bool, leaves: &[&str]) -> Group {
    Group {
        name: name.to_string(),
        tier,
        asleep,
        live: None,
        leaves: leaves.iter().map(|l| l.to_string()).collect(),
    }
}

/// The poll plan of 8.8. 1 s: the status block, the on-air probe and the target (read only
/// while the tracking mode is not `static`); the live pose at 5 s, at 1 s while a connection
/// subscribes `pose`; 5 s: the settings readbacks; 60 s: zone, hand (its return timer in a
/// group of its own), power-on, views;
/// `touched`: presets. Only the status block is read while the camera sleeps.
pub fn groups() -> Vec<Group> {
    let framing: Vec<&str> = FRAMING_QUERIES.iter().map(|(leaf, _)| *leaf).collect();
    let uvc: Vec<&str> = crate::models::tiny2lite::decode::UVC
        .iter()
        .map(|(leaf, _)| *leaf)
        .collect();
    vec![
        group("status", Tier::Second, true, &STATUS_LEAVES),
        group(
            "on_air",
            Tier::Second,
            false,
            &["on_air.active", "on_air.apps"],
        ),
        group(
            "target",
            Tier::Second,
            false,
            &["tracking.target.detected", "tracking.target.box"],
        ),
        Group {
            live: Some(LiveTier {
                topic: "pose".into(),
                tier: Tier::Second,
            }),
            ..group(
                "pose",
                Tier::FiveSeconds,
                false,
                &["gimbal.pan", "gimbal.tilt"],
            )
        },
        group("gestures", Tier::FiveSeconds, false, &GESTURE_LEAVES),
        group("zoom", Tier::FiveSeconds, false, &["zoom.factor"]),
        group("framing", Tier::FiveSeconds, false, &framing),
        group("uvc", Tier::FiveSeconds, false, &uvc),
        group(
            "focus",
            Tier::FiveSeconds,
            false,
            &["focus.mode", "focus.position"],
        ),
        group(
            "audio",
            Tier::FiveSeconds,
            false,
            &["audio.volume", "audio.enabled"],
        ),
        group(
            "zone",
            Tier::Minute,
            false,
            &[
                "zone.area.pan",
                "zone.area.tilt",
                "zone.follow",
                "zone.return",
            ],
        ),
        group(
            "hand",
            Tier::Minute,
            false,
            &[
                "hand.area.pan",
                "hand.area.tilt",
                "hand.axes.pan",
                "hand.axes.tilt",
                "hand.return",
            ],
        ),
        // its own group: the query is U-6, and an answer it does not get must not leave the
        // rest of the hand settings unread
        group("hand_timer", Tier::Minute, false, &["hand.return_after"]),
        group(
            "power_on",
            Tier::Minute,
            false,
            &[
                "power_on.start.source",
                "power_on.start.pose",
                "power_on.start.mode",
                "power_on.settings.tracking_speed",
                "power_on.settings.autofocus",
                "power_on.settings.auto_exposure",
                "power_on.settings.exposure_compensation",
                "power_on.settings.white_balance_mode",
                "power_on.settings.white_balance_temperature",
                "power_on.settings.contrast",
                "power_on.settings.saturation",
                "power_on.settings.sharpness",
                "power_on.settings.hue",
            ],
        ),
        group("views", Tier::Minute, false, &["views"]),
        group("presets", Tier::Touched, false, &["presets"]),
    ]
}

/// The groups to re-read after a leaf was written or an action ran (8.2, 8.8): a leaf's own
/// group, plus the groups the camera itself changes in response. `tracking.mode` rewrites
/// `framing.*`; `zoom.field_of_view` and the gesture settings change the zoom; a recall or a
/// view moves the gimbal and the zoom; the preset list is read only after `preset.save`,
/// `preset.rename` and `preset.delete` (8.8). The moves, the stops and the go-to actions touch
/// nothing: the live pose shows the motion at its own tier.
pub fn touched(plan: &[Group], target: Target<'_>) -> Vec<String> {
    let own = |leaf: &str| -> Vec<String> {
        plan.iter()
            .filter(|g| g.leaves.iter().any(|l| l == leaf))
            .map(|g| g.name.clone())
            .collect()
    };
    let names = |list: &[&str]| list.iter().map(|g| g.to_string()).collect::<Vec<_>>();
    match target {
        Target::Leaf(leaf) => {
            let mut groups = own(leaf);
            let extra: &[&str] = match leaf {
                "tracking.mode" => &["framing"],
                "zoom.field_of_view" => &["zoom"],
                l if l.starts_with("gestures.") => &["zoom"],
                _ => &[],
            };
            groups.extend(names(extra));
            groups
        }
        Target::Action(action) => names(match action {
            "preset.save" | "preset.rename" | "preset.delete" => &["presets"],
            // 8.8: the preset list is read only after save, rename and delete
            "preset.recall" => &["zoom", "pose"],
            "view.go" => &["zoom", "pose"],
            "view.save" | "view.rename" | "view.delete" => &["views"],
            "zone.return_here" => &["zone"],
            "hand.return_here" | "hand.return_clear" | "hand.area_reset" => &["hand"],
            "focus.point" => &["focus"],
            "power_on.set_pose" | "power_on.use_current" => &["power_on"],
            "power_on.use_preset" => &["power_on"],
            "sleep" | "wake" => &["status"],
            _ => &[],
        }),
    }
}

/// The leaves that go first in a patch (8.8): the `requires` paths of the description
/// (`exposure.mode`, `image.white_balance.auto`, `focus.mode`), `exposure.mode` also before
/// `exposure.gain`, `tracking.mode` before `framing.*`, and `sleep.auto.seconds` before
/// `sleep.auto.enabled`, so that switching auto-sleep on in the same patch uses the new time.
pub const FIRST: [&str; 5] = [
    "exposure.mode",
    "image.white_balance.auto",
    "focus.mode",
    "tracking.mode",
    "sleep.auto.seconds",
];

/// The apply order of a patch: the leaves of [`FIRST`], then the others, each part in the order
/// given.
pub fn order(leaves: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = leaves.iter().map(|l| l.to_string()).collect();
    out.sort_by_key(|l| !FIRST.contains(&l.as_str()));
    out
}

/// What an action is to the queue (4.1 rules 2 and 6, 5.6, 8.8).
pub fn motion(action: &str) -> Motion {
    match action {
        "gimbal.turn" => Motion::Continuing {
            stop: "gimbal.stop",
        },
        "gimbal.stop" | "zoom.stop" => Motion::Stop,
        "gimbal.move" | "zoom.move" | "preset.recall" | "view.go" | "zone.go_return"
        | "hand.go_return" | "power_on.go" => Motion::Move,
        _ => Motion::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tiny2lite::description::description;
    use std::collections::BTreeMap;

    #[test]
    fn every_leaf_with_a_readback_is_in_exactly_one_group() {
        let d = description();
        let mut seen: BTreeMap<String, String> = BTreeMap::new();
        for g in groups() {
            for leaf in &g.leaves {
                assert!(d.settings.contains_key(leaf), "{leaf} is not described");
                assert!(
                    seen.insert(leaf.clone(), g.name.clone()).is_none(),
                    "{leaf} twice"
                );
            }
        }
        for (path, s) in &d.settings {
            assert_eq!(seen.contains_key(path), s.readback, "{path}");
        }
        assert_eq!(seen.len(), 82);
    }

    #[test]
    fn the_tiers_of_8_8() {
        let tier = |name: &str| groups().into_iter().find(|g| g.name == name).unwrap();
        for name in ["status", "on_air", "target"] {
            assert_eq!(tier(name).tier, Tier::Second, "{name}");
        }
        for name in [
            "pose", "gestures", "zoom", "framing", "uvc", "focus", "audio",
        ] {
            assert_eq!(tier(name).tier, Tier::FiveSeconds, "{name}");
        }
        for name in ["zone", "hand", "hand_timer", "power_on", "views"] {
            assert_eq!(tier(name).tier, Tier::Minute, "{name}");
        }
        assert_eq!(tier("presets").tier, Tier::Touched);
        assert_eq!(
            tier("pose").live,
            Some(LiveTier {
                topic: "pose".into(),
                tier: Tier::Second
            })
        );
        let asleep: Vec<String> = groups()
            .into_iter()
            .filter(|g| g.asleep)
            .map(|g| g.name)
            .collect();
        assert_eq!(
            asleep,
            ["status"],
            "while asleep only the status block is read"
        );
    }

    #[test]
    fn touched_groups_of_8_8() {
        let plan = groups();
        let t = |target| touched(&plan, target);
        assert_eq!(t(Target::Leaf("tracking.mode")), ["status", "framing"]);
        assert_eq!(t(Target::Leaf("zoom.field_of_view")), ["status", "zoom"]);
        assert_eq!(t(Target::Leaf("gestures.zoom_ratio")), ["gestures", "zoom"]);
        assert_eq!(t(Target::Leaf("led.brightness")), ["status"]);
        assert_eq!(t(Target::Leaf("focus.position")), ["focus"]);
        assert_eq!(
            t(Target::Leaf("led.blink")),
            Vec::<String>::new(),
            "no readback"
        );
        assert_eq!(t(Target::Action("preset.recall")), ["zoom", "pose"]);
        assert_eq!(t(Target::Action("power_on.use_preset")), ["power_on"]);
        assert_eq!(t(Target::Action("view.go")), ["zoom", "pose"]);
        for quiet in [
            "gimbal.move",
            "gimbal.turn",
            "gimbal.stop",
            "zoom.move",
            "zoom.stop",
            "zone.go_return",
            "hand.go_return",
            "power_on.go",
            "factory_reset",
        ] {
            assert_eq!(
                t(Target::Action(quiet)),
                Vec::<String>::new(),
                "{quiet} touches nothing (8.8; spec 2: no frame of our own)"
            );
        }
    }

    #[test]
    fn requires_paths_go_first_and_the_rest_keeps_its_order() {
        assert_eq!(
            order(&["exposure.time", "led.brightness", "exposure.mode"]),
            ["exposure.mode", "exposure.time", "led.brightness"]
        );
        assert_eq!(
            order(&["framing.size", "tracking.mode"]),
            ["tracking.mode", "framing.size"]
        );
        assert_eq!(
            order(&["sleep.auto.enabled", "sleep.auto.seconds"]),
            ["sleep.auto.seconds", "sleep.auto.enabled"]
        );
        assert_eq!(
            order(&[
                "image.white_balance.temperature",
                "image.white_balance.auto"
            ]),
            [
                "image.white_balance.auto",
                "image.white_balance.temperature"
            ]
        );
        assert_eq!(
            order(&["focus.position", "focus.mode"]),
            ["focus.mode", "focus.position"]
        );
        assert_eq!(
            order(&["exposure.gain", "exposure.mode"]),
            ["exposure.mode", "exposure.gain"]
        );
    }

    #[test]
    fn motion_marks() {
        assert_eq!(
            motion("gimbal.turn"),
            Motion::Continuing {
                stop: "gimbal.stop"
            }
        );
        assert_eq!(motion("gimbal.stop"), Motion::Stop);
        assert_eq!(motion("zoom.stop"), Motion::Stop);
        for m in [
            "gimbal.move",
            "zoom.move",
            "preset.recall",
            "view.go",
            "zone.go_return",
            "hand.go_return",
            "power_on.go",
        ] {
            assert_eq!(motion(m), Motion::Move, "{m} (4.1 rule 6)");
        }
        assert_eq!(motion("preset.save"), Motion::None);
        assert_eq!(motion("sleep"), Motion::None);
    }

    #[test]
    fn the_registry_entry() {
        assert_eq!(ENTRY.key, "tiny2lite");
        assert_eq!(
            ENTRY.usb,
            [UsbId {
                vendor: 0x3564,
                product: 0xfef9
            }]
        );
        assert_eq!(
            ENTRY.support,
            Support::Untested,
            "tested only from W3 on (8.8)"
        );
        assert_eq!(ENTRY.identity, IdentitySource::Query);
        let firmware_query = ENTRY.firmware_query;
        assert!(firmware_query, "the firmware query of 8.8");
        assert_eq!(ENTRY.identity_tier, Tier::FiveSeconds);
        assert_eq!(ENTRY.selectors, [2, 6, 0x0c, 0x0d]);
    }
}
