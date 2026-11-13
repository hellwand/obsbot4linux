// SPDX-License-Identifier: GPL-3.0-or-later

//! The description of the Tiny 2 Lite as data (design spec 4.3, 4.4, 8.8; field ledger
//! `docs/field-ledger.md`). `descriptions/tiny2lite.json` is generated from it.
//!
//! The fields, values and bounds of spec section 9.3 are 0.13.2 functions without backing
//! today. They are in the description, and [`UNBACKED`] marks each with its row; camera
//! session CS-1 keeps or removes them by a contract-change PR before the release candidate.

use obsbot_proto::{Action, Asleep, Description, Setting, SettingType, Unit, Warning};
use serde_json::json;

/// The tracking modes of `tracking.mode` and `power_on.mode`.
pub const MODES: [&str; 8] = [
    "static",
    "normal",
    "upper_body",
    "close_up",
    "headless",
    "lower_body",
    "hand",
    "group",
];

/// The one model-wide settle bound (8.8): 3000 ms for every writable leaf with a readback.
pub const SETTLE_MS: u32 = 3000;

/// Every mark of spec 9.3: (key, row). Keys: `<path>` a field; `<path>=<value>` an enum
/// value; `<path>@min` or `@max` a bound; `<path>#<aspect>` a property that is not a value or
/// a bound; `do:<action>`, `do:<action>.<param>=<value>`, `do:<action>.<param>@max` and
/// `do:<action>#<aspect>` the same for actions.
pub const UNBACKED: &[(&str, &str)] = &[
    ("framing.offset.x@min", "U-4"),
    ("framing.offset.x@max", "U-4"),
    ("framing.offset.y@min", "U-4"),
    ("framing.offset.y@max", "U-4"),
    ("framing.offset.x#direction", "U-4"),
    ("framing.offset.y#direction", "U-4"),
    ("hand.axes.pan", "U-5"),
    ("hand.axes.tilt", "U-5"),
    ("hand.return_after", "U-6"),
    ("do:hand.area_reset", "U-7"),
    ("hand.area.pan#order", "U-8"),
    ("hand.area.tilt#order", "U-8"),
    ("zone.area.pan#order", "U-8"),
    ("zone.area.tilt#order", "U-8"),
    ("do:gimbal.move.speed@max", "U-9"),
    ("do:gimbal.move#speed_near_tilt_limit", "U-9"),
    ("do:gimbal.move#speed_one_axis", "U-9"),
    ("do:zoom.move.speed=slow", "U-12"),
    ("do:zoom.move.speed=fast", "U-12"),
    ("do:focus.point", "U-16"),
    ("power_on.start.mode=static", "U-18"),
    ("power_on.start.mode=upper_body", "U-18"),
    ("power_on.start.mode=close_up", "U-18"),
    ("power_on.start.mode=headless", "U-18"),
    ("power_on.start.mode=lower_body", "U-18"),
    ("power_on.start.mode=hand", "U-18"),
    ("power_on.start.mode=group", "U-18"),
    ("power_on.settings.tracking_speed#values", "U-19"),
    ("power_on.settings.autofocus#values", "U-19"),
    ("power_on.settings.auto_exposure#values", "U-19"),
    ("power_on.settings.exposure_compensation#values", "U-19"),
    ("power_on.settings.white_balance_mode#values", "U-19"),
    ("power_on.settings.white_balance_temperature#values", "U-19"),
    ("power_on.settings.contrast#values", "U-19"),
    ("power_on.settings.saturation#values", "U-19"),
    ("power_on.settings.sharpness#values", "U-19"),
    ("power_on.settings.hue#values", "U-19"),
    ("exposure.compensation#ev_scale", "U-20"),
    ("audio.enabled#readback", "U-23"),
    ("exposure.anti_flicker=auto", "U-25"),
    ("power_on.start.source=preset_0", "U-26"),
    ("power_on.start.source=preset_1", "U-26"),
    ("power_on.start.source=preset_2", "U-26"),
];

/// Bounds, steps and defaults that this description holds but neither the spec nor the field
/// ledger backs yet (the W0 plan's decision 12): `<path>@min`, `<path>@max`, `<path>#step`,
/// `<path>#default`. They are not evidence. Track A confirms each in the ledger, from a capture
/// or a measurement on the camera, or changes it, before the evidence-level test of stage W3.
pub const UNCONFIRMED: &[&str] = &[
    "exposure.time@min",
    "exposure.time@max",
    "exposure.time#default",
    "exposure.gain#default",
    "focus.position#default",
    "image.brightness@min",
    "image.brightness@max",
    "image.brightness#default",
    "image.contrast@min",
    "image.contrast@max",
    "image.contrast#default",
    "image.saturation@min",
    "image.saturation@max",
    "image.saturation#default",
    "image.hue@min",
    "image.hue@max",
    "image.hue#default",
    "image.sharpness@min",
    "image.sharpness@max",
    "image.sharpness#default",
    "image.white_balance.auto#default",
    "image.white_balance.temperature@min",
    "image.white_balance.temperature@max",
    "image.white_balance.temperature#step",
    "image.white_balance.temperature#default",
];

/// A writable leaf with a readback: asleep `unknown` until CAM-2, the model-wide settle bound.
fn w(s: Setting) -> Setting {
    s.asleep(Asleep::Unknown).settle_ms(SETTLE_MS)
}

/// A writable leaf the camera cannot report (`readback: false`, 8.8).
fn w_no_readback(s: Setting) -> Setting {
    s.asleep(Asleep::Unknown).no_readback()
}

/// A pose `{pan, tilt, zoom}` in degrees and zoom factor, within the gimbal range.
pub fn pose(label: &str) -> Setting {
    Setting::composite(
        SettingType::Pose,
        label,
        &[("pan", -150.0), ("tilt", -70.0), ("zoom", 1.0)],
        &[("pan", 150.0), ("tilt", 90.0), ("zoom", 4.0)],
    )
    .units(&[
        ("pan", Unit::Deg),
        ("tilt", Unit::Deg),
        ("zoom", Unit::Factor),
    ])
    .precisions(&[("pan", 0.1), ("tilt", 0.1), ("zoom", 0.01)])
}

fn angle_range(label: &str, lo: f64, hi: f64) -> Setting {
    w(Setting::range(label, lo, hi).unit(Unit::Deg).precision(0.1))
}

fn uvc(label: &str, min: f64, max: f64, default: f64) -> Setting {
    w(Setting::number(label, min, max)
        .step(1.0)
        .precision(1.0)
        .default_value(json!(default)))
}

fn slot() -> Setting {
    Setting::number("Preset slot", 0.0, 2.0).step(1.0)
}

fn view_id() -> Setting {
    Setting::new(SettingType::Number, "View id")
        .min(0.0)
        .step(1.0)
}

fn name() -> Setting {
    Setting::string("Name", 32, true)
}

fn raw_block(label: &str) -> Setting {
    Setting::new(SettingType::Number, label).readonly()
}

/// The Tiny 2 Lite's settings (4.3), actions (4.4) and limits.
pub fn description() -> Description {
    let mut start_modes: Vec<&str> = MODES.to_vec();
    start_modes.push("unset");
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
            "gimbal.pan",
            Setting::new(SettingType::Number, "Pan")
                .unit(Unit::Deg)
                .precision(0.1)
                .readonly()
                .live("pose"),
        )
        .setting(
            "gimbal.tilt",
            Setting::new(SettingType::Number, "Tilt")
                .unit(Unit::Deg)
                .precision(0.1)
                .readonly()
                .live("pose"),
        )
        .setting(
            "gimbal.invert",
            w(Setting::boolean("Reverse the pan direction")),
        )
        .setting(
            "zoom.factor",
            w(Setting::number("Zoom", 1.0, 4.0)
                .unit(Unit::Factor)
                .precision(0.01)),
        )
        .setting(
            "zoom.field_of_view",
            Setting::enumeration("Field of view", &["wide", "medium", "narrow", "custom"])
                .writable_only(&["wide", "medium", "narrow"])
                .asleep(Asleep::Ignored)
                .settle_ms(SETTLE_MS),
        )
        .setting(
            "tracking.mode",
            w(Setting::enumeration("Tracking mode", &MODES)),
        )
        .setting(
            "tracking.profile",
            w(Setting::enumeration(
                "Tracking profile",
                &["standard", "headroom", "sport"],
            )),
        )
        .setting(
            "tracking.speed",
            w(Setting::enumeration(
                "Tracking speed",
                &["lazy", "slow", "normal", "fast", "very_fast", "auto"],
            )),
        )
        .setting(
            "tracking.target.detected",
            Setting::boolean("Target detected").readonly(),
        )
        .setting(
            "tracking.target.box",
            Setting::composite(
                SettingType::Box,
                "Target box",
                &[("left", 0.0), ("top", 0.0), ("right", 0.0), ("bottom", 0.0)],
                &[("left", 1.0), ("top", 1.0), ("right", 1.0), ("bottom", 1.0)],
            )
            .unit(Unit::Fraction)
            .precision(0.01)
            .readonly()
            .nullable()
            .live("target"),
        )
        .setting(
            "framing.auto_zoom",
            w(Setting::boolean("Automatic framing zoom")),
        )
        .setting(
            "framing.size",
            w(Setting::enumeration(
                "Framing size",
                &[
                    "long_shot",
                    "medium_long_shot",
                    "medium_shot",
                    "medium_close_up",
                    "close_up",
                    "big_close_up",
                ],
            )),
        )
        .setting(
            "framing.zoom_speed",
            w(Setting::enumeration(
                "Framing zoom speed",
                &["slow", "medium", "fast"],
            )),
        )
        .setting(
            "framing.offset.x",
            w(Setting::number("Framing offset x", -0.16, 0.16)
                .unit(Unit::Fraction)
                .precision(0.01)),
        )
        .setting(
            "framing.offset.y",
            w(Setting::number("Framing offset y", -0.16, 0.16)
                .unit(Unit::Fraction)
                .precision(0.01)),
        )
        .setting(
            "framing.part",
            w(Setting::enumeration(
                "Framed part",
                &["normal", "head_removal", "lower_body"],
            )),
        )
        .setting(
            "gestures.target_lock",
            w(Setting::boolean("Gesture: target lock")),
        )
        .setting("gestures.zoom", w(Setting::boolean("Gesture: zoom")))
        .setting(
            "gestures.zoom_ratio",
            w(Setting::number("Gesture zoom ratio", 1.0, 4.0)
                .unit(Unit::Factor)
                .step(0.1)
                .precision(0.1)),
        )
        .setting(
            "gestures.dynamic_zoom",
            w(Setting::boolean("Gesture: dynamic zoom")),
        )
        .setting(
            "gestures.direction_change",
            w(Setting::boolean("Gesture: reverse the dynamic zoom")),
        )
        .setting("zone.enabled", w(Setting::boolean("Zone tracking")))
        .setting(
            "zone.follow",
            w(Setting::boolean("Zone: follow the target")),
        )
        .setting(
            "zone.area.pan",
            angle_range("Zone pan range", -150.0, 150.0),
        )
        .setting(
            "zone.area.tilt",
            angle_range("Zone tilt range", -70.0, 90.0),
        )
        .setting("zone.return", pose("Zone return pose").readonly())
        .setting(
            "hand.side",
            w(Setting::enumeration("Hand", &["left", "right"])),
        )
        .setting(
            "hand.area.pan",
            angle_range("Hand tracking pan range", -150.0, 150.0),
        )
        .setting(
            "hand.area.tilt",
            angle_range("Hand tracking tilt range", -70.0, 90.0),
        )
        .setting(
            "hand.axes.pan",
            w(Setting::boolean("Hand tracking may pan")),
        )
        .setting(
            "hand.axes.tilt",
            w(Setting::boolean("Hand tracking may tilt")),
        )
        .setting(
            "hand.return_after",
            w(Setting::new(SettingType::Number, "Hand return after")
                .min(0.0)
                .unit(Unit::Seconds)
                .precision(0.1)),
        )
        .setting(
            "hand.move_gimbal",
            w_no_readback(Setting::boolean("Hand tracking moves the gimbal")),
        )
        .setting(
            "hand.return",
            pose("Hand return pose").readonly().nullable(),
        )
        .setting(
            "presets",
            Setting::list(
                "Presets",
                &[("slot", slot()), ("name", name()), ("pose", pose("Pose"))],
            ),
        )
        .setting(
            "views",
            Setting::list(
                "Views",
                &[("id", view_id()), ("name", name()), ("pose", pose("Pose"))],
            ),
        )
        .setting(
            "exposure.mode",
            w(Setting::enumeration("Exposure", &["auto", "manual"])),
        )
        .setting(
            "exposure.region",
            w(Setting::enumeration(
                "Exposure metering",
                &["face", "global"],
            )),
        )
        .setting(
            "exposure.time",
            w(Setting::number("Exposure time", 100.0, 250_000.0)
                .unit(Unit::Microseconds)
                .precision(1.0))
            .requires("exposure.mode", json!("manual"))
            .live("auto")
            .default_value(json!(33_000)),
        )
        .setting("exposure.gain", uvc("Gain", 1.0, 64.0, 1.0).live("auto"))
        .setting(
            "exposure.compensation",
            uvc("Exposure compensation", 0.0, 18.0, 9.0),
        )
        .setting("exposure.hdr", w(Setting::boolean("HDR")))
        .setting(
            "exposure.anti_flicker",
            w(
                Setting::enumeration("Anti-flicker", &["off", "50hz", "60hz", "auto"])
                    .writable_only(&["off", "50hz", "60hz"]),
            ),
        )
        .setting("image.brightness", uvc("Brightness", 0.0, 100.0, 50.0))
        .setting("image.contrast", uvc("Contrast", 0.0, 100.0, 50.0))
        .setting("image.saturation", uvc("Saturation", 0.0, 100.0, 50.0))
        .setting("image.hue", uvc("Hue", 0.0, 100.0, 50.0))
        .setting("image.sharpness", uvc("Sharpness", 0.0, 100.0, 50.0))
        .setting(
            "image.white_balance.auto",
            w(Setting::boolean("Automatic white balance")).default_value(json!(true)),
        )
        .setting(
            "image.white_balance.temperature",
            w(
                Setting::number("White balance temperature", 2000.0, 10_000.0)
                    .step(100.0)
                    .unit(Unit::Kelvin)
                    .precision(1.0),
            )
            .requires("image.white_balance.auto", json!(false))
            .live("auto")
            .default_value(json!(5000)),
        )
        .setting("image.mirror", w(Setting::boolean("Mirror")))
        .setting(
            "image.frame_rate",
            Setting::number("Frame rate", 30.0, 60.0)
                .unit(Unit::Fps)
                .readonly(),
        )
        .setting(
            "focus.mode",
            w(Setting::enumeration(
                "Focus",
                &["continuous", "single", "manual"],
            )),
        )
        .setting(
            "focus.region",
            w(Setting::enumeration("Focus region", &["face", "global"])),
        )
        .setting(
            "focus.position",
            uvc("Focus position", 0.0, 100.0, 0.0)
                .requires("focus.mode", json!("manual"))
                .no_effect(),
        )
        .setting(
            "audio.enabled",
            w(Setting::boolean("Microphone")).warning(Warning::Restart),
        )
        .setting(
            "audio.volume",
            w(Setting::number("Microphone volume", 0.0, 100.0)
                .step(1.0)
                .unit(Unit::Percent)
                .precision(1.0)),
        )
        .setting(
            "audio.noise_reduction",
            w(Setting::enumeration(
                "Noise reduction",
                &["off", "weak", "medium", "strong"],
            )),
        )
        .setting(
            "audio.auto_gain",
            w(Setting::boolean("Automatic microphone gain")),
        )
        .setting(
            "audio.in_sleep",
            w(Setting::boolean("Microphone while asleep")),
        )
        .setting(
            "led.brightness",
            w(Setting::number("LED brightness", 0.0, 3.0)
                .step(1.0)
                .precision(1.0)),
        )
        .setting("led.blink", w_no_readback(Setting::boolean("LED blink")))
        .setting("sleep.auto.enabled", w(Setting::boolean("Auto-sleep")))
        .setting(
            "sleep.auto.seconds",
            w(Setting::number("Auto-sleep after", 1.0, 32_767.0)
                .step(1.0)
                .unit(Unit::Seconds)
                .precision(1.0)),
        )
        .setting(
            "sleep.background",
            w(
                Setting::enumeration("Sleep background", &["black", "image", "own_image"])
                    .writable_only(&["black", "image"]),
            ),
        )
        .setting(
            "sleep.background_mirror",
            w(Setting::boolean("Mirror the sleep background")),
        )
        .setting(
            "sleep.wake_zoom",
            w(Setting::number("Zoom at wake", 1.0, 4.0)
                .unit(Unit::Factor)
                .precision(0.01))
            .no_effect(),
        )
        .setting(
            "power_on.mode",
            w(Setting::enumeration("Tracking mode at power-on", &MODES)),
        )
        .setting(
            "power_on.start.source",
            Setting::enumeration(
                "Initial state source",
                &["own", "preset_0", "preset_1", "preset_2"],
            )
            .readonly(),
        )
        .setting("power_on.start.pose", pose("Initial pose").readonly())
        .setting(
            "power_on.start.mode",
            w(
                Setting::enumeration("Initial tracking mode", &start_modes).writable_only(&[
                    "static",
                    "normal",
                    "upper_body",
                    "headless",
                    "group",
                    "unset",
                ]),
            ),
        )
        .setting(
            "power_on.settings.tracking_speed",
            raw_block("Initial tracking speed (raw)").nullable(),
        )
        .setting(
            "power_on.settings.autofocus",
            raw_block("Initial autofocus (raw)").nullable(),
        )
        .setting(
            "power_on.settings.auto_exposure",
            raw_block("Initial automatic exposure (raw)").nullable(),
        )
        .setting(
            "power_on.settings.exposure_compensation",
            raw_block("Initial exposure compensation (raw)"),
        )
        .setting(
            "power_on.settings.white_balance_mode",
            raw_block("Initial white balance mode (raw)").nullable(),
        )
        .setting(
            "power_on.settings.white_balance_temperature",
            raw_block("Initial white balance temperature (raw)").nullable(),
        )
        .setting(
            "power_on.settings.contrast",
            raw_block("Initial contrast (raw)").nullable(),
        )
        .setting(
            "power_on.settings.saturation",
            raw_block("Initial saturation (raw)").nullable(),
        )
        .setting(
            "power_on.settings.sharpness",
            raw_block("Initial sharpness (raw)").nullable(),
        )
        .setting(
            "power_on.settings.hue",
            raw_block("Initial hue (raw)").nullable(),
        )
        .action(
            "gimbal.move",
            Action::new("Move the gimbal", Asleep::Unknown)
                .param(
                    "pan",
                    Setting::number("Pan", -150.0, 150.0)
                        .unit(Unit::Deg)
                        .precision(0.1)
                        .optional(),
                )
                .param(
                    "tilt",
                    Setting::number("Tilt", -70.0, 90.0)
                        .unit(Unit::Deg)
                        .precision(0.1)
                        .optional(),
                )
                .param(
                    "speed",
                    Setting::number("Speed", 1.0, 180.0)
                        .unit(Unit::DegPerSecond)
                        .precision(0.1)
                        .optional(),
                ),
        )
        .action(
            "gimbal.stop",
            Action::new("Stop the gimbal", Asleep::Unknown),
        )
        .action(
            "gimbal.turn",
            Action::new("Turn the gimbal", Asleep::Unknown)
                .param(
                    "pan_speed",
                    Setting::number("Pan speed", -178.0, 178.0)
                        .unit(Unit::DegPerSecond)
                        .precision(0.1)
                        .optional(),
                )
                .param(
                    "tilt_speed",
                    Setting::number("Tilt speed", -178.0, 178.0)
                        .unit(Unit::DegPerSecond)
                        .precision(0.1)
                        .optional(),
                )
                .param(
                    "keep",
                    Setting::boolean("Keep turning after the connection closes").optional(),
                ),
        )
        .action(
            "zoom.move",
            Action::new("Zoom at a speed", Asleep::Unknown)
                .param(
                    "factor",
                    Setting::number("Zoom", 1.0, 4.0)
                        .unit(Unit::Factor)
                        .precision(0.01),
                )
                .param(
                    "speed",
                    Setting::enumeration("Speed", &["slow", "medium", "fast"]),
                ),
        )
        .action("zoom.stop", Action::new("Stop zooming", Asleep::Unknown))
        .action(
            "preset.save",
            Action::new("Save a preset", Asleep::Ignored)
                .param("slot", slot())
                .param("name", name().optional())
                .param("pose", pose("Pose").optional()),
        )
        .action(
            "preset.recall",
            Action::new("Recall a preset", Asleep::Unknown).param("slot", slot()),
        )
        .action(
            "preset.rename",
            Action::new("Rename a preset", Asleep::Unknown)
                .param("slot", slot())
                .param("name", name()),
        )
        .action(
            "preset.delete",
            Action::new("Delete a preset", Asleep::Unknown).param("slot", slot()),
        )
        .action(
            "view.save",
            Action::new("Save a view", Asleep::Ignored)
                .param("name", name())
                .param("pose", pose("Pose").optional())
                .result("id", view_id()),
        )
        .action(
            "view.go",
            Action::new("Go to a view", Asleep::Unknown).param("id", view_id()),
        )
        .action(
            "view.rename",
            Action::new("Rename a view", Asleep::Unknown)
                .param("id", view_id())
                .param("name", name()),
        )
        .action(
            "view.delete",
            Action::new("Delete a view", Asleep::Unknown).param("id", view_id()),
        )
        .action(
            "zone.return_here",
            Action::new("Set the zone return pose here", Asleep::Ignored),
        )
        .action(
            "zone.go_return",
            Action::new("Go to the zone return pose", Asleep::Unknown),
        )
        .action(
            "hand.return_here",
            Action::new("Set the hand return pose here", Asleep::Ignored),
        )
        .action(
            "hand.go_return",
            Action::new("Go to the hand return pose", Asleep::Unknown),
        )
        .action(
            "hand.return_clear",
            Action::new("Clear the hand return pose", Asleep::Unknown),
        )
        .action(
            "hand.area_reset",
            Action::new("Reset the hand tracking area", Asleep::Unknown),
        )
        .action(
            "focus.point",
            Action::new("Focus on a point", Asleep::Unknown)
                .param(
                    "x",
                    Setting::number("x", 0.0, 1.0)
                        .unit(Unit::Fraction)
                        .precision(0.01),
                )
                .param(
                    "y",
                    Setting::number("y", 0.0, 1.0)
                        .unit(Unit::Fraction)
                        .precision(0.01),
                ),
        )
        .action(
            "power_on.set_pose",
            Action::new("Set the initial pose", Asleep::Unknown).param("pose", pose("Pose")),
        )
        .action(
            "power_on.use_current",
            Action::new("Use the current pose as the initial pose", Asleep::Ignored),
        )
        .action(
            "power_on.use_preset",
            Action::new("Use a preset as the initial pose", Asleep::Unknown).param("slot", slot()),
        )
        .action(
            "power_on.go",
            Action::new("Go to the initial pose", Asleep::Ignored),
        )
        .action("sleep", Action::new("Sleep", Asleep::Unknown))
        .action("wake", Action::new("Wake", Asleep::Accepted))
        .action(
            "factory_reset",
            Action::new("Factory reset", Asleep::Unknown)
                .param("confirm", Setting::boolean("Confirm"))
                .warning(Warning::Destructive)
                .warning(Warning::Restart),
        )
        .limit("presets", 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::validate::check_description;
    use std::collections::BTreeSet;

    #[test]
    fn the_description_is_consistent() {
        assert_eq!(check_description(&description()), Vec::<String>::new());
    }

    /// Every leaf of the state model of 4.3 (the rows of the field ledger without the daemon's
    /// `available`, `pending` and `unread`) and every action of 4.4, and nothing else.
    #[test]
    fn it_has_exactly_the_paths_and_actions_of_the_spec() {
        const LEAVES: [&str; 84] = [
            "asleep",
            "audio.auto_gain",
            "audio.enabled",
            "audio.in_sleep",
            "audio.noise_reduction",
            "audio.volume",
            "exposure.anti_flicker",
            "exposure.compensation",
            "exposure.gain",
            "exposure.hdr",
            "exposure.mode",
            "exposure.region",
            "exposure.time",
            "focus.mode",
            "focus.position",
            "focus.region",
            "framing.auto_zoom",
            "framing.offset.x",
            "framing.offset.y",
            "framing.part",
            "framing.size",
            "framing.zoom_speed",
            "gestures.direction_change",
            "gestures.dynamic_zoom",
            "gestures.target_lock",
            "gestures.zoom",
            "gestures.zoom_ratio",
            "gimbal.invert",
            "gimbal.pan",
            "gimbal.tilt",
            "hand.area.pan",
            "hand.area.tilt",
            "hand.axes.pan",
            "hand.axes.tilt",
            "hand.move_gimbal",
            "hand.return",
            "hand.return_after",
            "hand.side",
            "image.brightness",
            "image.contrast",
            "image.frame_rate",
            "image.hue",
            "image.mirror",
            "image.saturation",
            "image.sharpness",
            "image.white_balance.auto",
            "image.white_balance.temperature",
            "led.blink",
            "led.brightness",
            "on_air.active",
            "on_air.apps",
            "power_on.mode",
            "power_on.settings.auto_exposure",
            "power_on.settings.autofocus",
            "power_on.settings.contrast",
            "power_on.settings.exposure_compensation",
            "power_on.settings.hue",
            "power_on.settings.saturation",
            "power_on.settings.sharpness",
            "power_on.settings.tracking_speed",
            "power_on.settings.white_balance_mode",
            "power_on.settings.white_balance_temperature",
            "power_on.start.mode",
            "power_on.start.pose",
            "power_on.start.source",
            "presets",
            "sleep.auto.enabled",
            "sleep.auto.seconds",
            "sleep.background",
            "sleep.background_mirror",
            "sleep.wake_zoom",
            "tracking.mode",
            "tracking.profile",
            "tracking.speed",
            "tracking.target.box",
            "tracking.target.detected",
            "views",
            "zone.area.pan",
            "zone.area.tilt",
            "zone.enabled",
            "zone.follow",
            "zone.return",
            "zoom.factor",
            "zoom.field_of_view",
        ];
        const ACTIONS: [&str; 27] = [
            "factory_reset",
            "focus.point",
            "gimbal.move",
            "gimbal.stop",
            "gimbal.turn",
            "hand.area_reset",
            "hand.go_return",
            "hand.return_clear",
            "hand.return_here",
            "power_on.go",
            "power_on.set_pose",
            "power_on.use_current",
            "power_on.use_preset",
            "preset.delete",
            "preset.recall",
            "preset.rename",
            "preset.save",
            "sleep",
            "view.delete",
            "view.go",
            "view.rename",
            "view.save",
            "wake",
            "zone.go_return",
            "zone.return_here",
            "zoom.move",
            "zoom.stop",
        ];
        let d = description();
        assert_eq!(
            d.settings.keys().map(String::as_str).collect::<Vec<_>>(),
            LEAVES
        );
        assert_eq!(
            d.actions.keys().map(String::as_str).collect::<Vec<_>>(),
            ACTIONS
        );
        assert_eq!(d.limits.get("presets"), Some(&3));
    }

    #[test]
    fn readback_false_and_asleep_values_follow_section_8_8() {
        let d = description();
        let no_readback: Vec<&String> = d
            .settings
            .iter()
            .filter(|(_, s)| !s.readback)
            .map(|(k, _)| k)
            .collect();
        assert_eq!(no_readback, ["hand.move_gimbal", "led.blink"]);
        assert_eq!(
            d.settings["zoom.field_of_view"].asleep,
            Some(Asleep::Ignored)
        );
        assert_eq!(d.actions["power_on.go"].asleep, Asleep::Ignored);
        assert_eq!(d.actions["wake"].asleep, Asleep::Accepted);
        for pose_taking in [
            "preset.save",
            "view.save",
            "power_on.use_current",
            "zone.return_here",
            "hand.return_here",
        ] {
            assert_eq!(
                d.actions[pose_taking].asleep,
                Asleep::Ignored,
                "{pose_taking} (CAM-22)"
            );
        }
        for (path, s) in &d.settings {
            if !s.readonly && s.readback {
                assert_eq!(s.settle_ms, Some(SETTLE_MS), "{path}");
            }
        }
    }

    /// Every mark names something the description has, and every row of 9.3 is marked.
    #[test]
    fn every_unbacked_mark_resolves_and_every_row_of_9_3_is_marked() {
        let d = description();
        for (key, row) in UNBACKED {
            let (base, aspect) = key
                .split_once('#')
                .map_or((*key, None), |(b, a)| (b, Some(a)));
            let (base, value) = base
                .split_once('=')
                .map_or((base, None), |(b, v)| (b, Some(v)));
            let (base, bound) = base
                .split_once('@')
                .map_or((base, None), |(b, x)| (b, Some(x)));
            let setting = match base.strip_prefix("do:") {
                Some(rest) => {
                    let (action, param) = match d.actions.get(rest) {
                        Some(_) => (rest, None),
                        None => rest
                            .rsplit_once('.')
                            .map(|(a, p)| (a, Some(p)))
                            .expect("an action or action.param"),
                    };
                    let a = d
                        .actions
                        .get(action)
                        .unwrap_or_else(|| panic!("{key}: no action {action}"));
                    param.map(|p| {
                        a.params
                            .get(p)
                            .unwrap_or_else(|| panic!("{key}: no parameter {p}"))
                    })
                }
                None => Some(
                    d.settings
                        .get(base)
                        .unwrap_or_else(|| panic!("{key}: no setting {base}")),
                ),
            };
            if let Some(v) = value {
                assert!(
                    setting.is_some_and(|s| s.values.iter().any(|x| x == v)),
                    "{key}: no value {v}"
                );
            }
            if let Some(b) = bound {
                let s = setting.expect("a bound belongs to a setting or parameter");
                let present = if b == "min" {
                    s.min.is_some()
                } else {
                    s.max.is_some()
                };
                assert!(present, "{key}: no bound {b}");
            }
            assert!(aspect.is_none_or(|a| !a.is_empty()), "{key}");
            assert!(row.starts_with("U-"), "{key}: {row}");
        }
        let rows: BTreeSet<&str> = UNBACKED.iter().map(|(_, r)| *r).collect();
        let want: BTreeSet<&str> = [
            "U-4", "U-5", "U-6", "U-7", "U-8", "U-9", "U-12", "U-16", "U-18", "U-19", "U-20",
            "U-23", "U-25", "U-26",
        ]
        .into();
        assert_eq!(rows, want);
    }

    /// Every unconfirmed number names a bound, step or default the description has, so that
    /// track A's ledger work and the W3 evidence-level test find each one.
    #[test]
    fn every_unconfirmed_number_is_in_the_description() {
        let d = description();
        for key in UNCONFIRMED {
            let (path, what) = key
                .split_once(['@', '#'])
                .unwrap_or_else(|| panic!("{key}: path@bound or path#aspect"));
            let s = d
                .settings
                .get(path)
                .unwrap_or_else(|| panic!("{key}: no setting {path}"));
            let present = match what {
                "min" => s.min.is_some(),
                "max" => s.max.is_some(),
                "step" => s.step.is_some(),
                "default" => s.default.is_some(),
                _ => false,
            };
            assert!(present, "{key}: the setting has no {what}");
        }
    }
}
