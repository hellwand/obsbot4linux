// SPDX-License-Identifier: GPL-3.0-or-later

//! The evidence levels of the Tiny 2 Lite (design spec 8.5), one row per field and action of
//! its description, with the evidence column of `docs/field-ledger.md` as it stands, and one
//! row per value, bound, step or default whose evidence differs from its field's. A value or
//! bound without a row of its own takes its field's level, except `none`: that is never
//! inherited, every value, bound or aspect without backing has a row of its own and a mark of
//! spec 9.3 in [`UNBACKED`], and the test checks the two lists against each other and the marks
//! against the ledger. `tests/evidence.rs` checks the rows against the description and the
//! ledger.

use super::description::{UNBACKED, UNCONFIRMED};
use crate::evidence::{Evidence, Level, Row};

const fn row(key: &'static str, level: Level, evidence: &'static str) -> Row {
    Row {
        key,
        level,
        evidence,
    }
}

/// The evidence of the Tiny 2 Lite for [`crate::evidence::of`].
pub const EVIDENCE: Evidence = Evidence {
    levels: LEVELS,
    unbacked: UNBACKED,
    unconfirmed: UNCONFIRMED,
};

/// Every field and action, in ledger order; then one row of level `none` per mark of spec 9.3
/// that names a value, bound or aspect (the order of `UNBACKED`); then the bounds, steps and
/// defaults of the UVC controls: the camera's answers to the UVC range requests (GET_MIN,
/// GET_MAX, GET_RES, GET_DEF) recorded in captures 01, 14, 16 and 18.
pub const LEVELS: &[Row] = &[
    row("asleep", Level::Captured, "capture 13; measured 2026-10-24"),
    row(
        "on_air.active",
        Level::Daemon,
        "daemon (the probe measured on this model 2026-10-25)",
    ),
    row(
        "on_air.apps",
        Level::Daemon,
        "daemon (the probe measured on this model 2026-10-25)",
    ),
    row("gimbal.pan", Level::Measured, "measured 2026-10-31 (#67)"),
    row("gimbal.tilt", Level::Measured, "measured 2026-10-31 (#67)"),
    row("gimbal.invert", Level::Captured, "capture 05, 17"),
    row(
        "zoom.factor",
        Level::Captured,
        "capture 04, 11; measured 2026-10-24",
    ),
    row(
        "zoom.field_of_view",
        Level::Captured,
        "capture 09; measured 2026-10-24",
    ),
    row("tracking.mode", Level::Captured, "capture 11"),
    row(
        "tracking.profile",
        Level::Captured,
        "capture 11 (0, 2); measured 2026-10-24 (0, 2), 2026-10-31 (#98: 1)",
    ),
    row(
        "tracking.speed",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "tracking.target.detected",
        Level::Measured,
        "measured 2026-10-25",
    ),
    row(
        "tracking.target.box",
        Level::Measured,
        "measured 2026-10-25; layout #71",
    ),
    row(
        "framing.auto_zoom",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "framing.size",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "framing.zoom_speed",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "framing.offset.x",
        Level::Measured,
        "measured 2026-10-31 (#98: read back); bounds and direction: measured 2026-11-25 (U-4)",
    ),
    row(
        "framing.offset.y",
        Level::Measured,
        "measured 2026-10-31 (#98: read back); bounds and direction: measured 2026-11-25 (U-4)",
    ),
    row(
        "framing.part",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "gestures.target_lock",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "gestures.zoom",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "gestures.zoom_ratio",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "gestures.dynamic_zoom",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "gestures.direction_change",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "zone.enabled",
        Level::Captured,
        "capture 11; measured 2026-10-31 (#81)",
    ),
    row(
        "zone.follow",
        Level::Captured,
        "capture 11, 18; measured 2026-10-25",
    ),
    row(
        "zone.area.pan",
        Level::Captured,
        "capture 11; measured 2026-10-25, 2026-10-27 (#58), 2026-11-25 (CAM-16)",
    ),
    row(
        "zone.area.tilt",
        Level::Captured,
        "capture 11; measured 2026-10-25, 2026-10-27 (#58), 2026-11-25 (CAM-16)",
    ),
    row(
        "zone.return",
        Level::Captured,
        "capture 11, 18; measured 2026-10-25",
    ),
    row("hand.side", Level::Captured, "capture 11, 17"),
    row(
        "hand.area.pan",
        Level::Captured,
        "capture 11; measured 2026-11-25 (CAM-16)",
    ),
    row(
        "hand.area.tilt",
        Level::Captured,
        "capture 11; measured 2026-11-25 (CAM-16)",
    ),
    row("hand.axes.tilt", Level::None, "none"),
    row(
        "hand.return_after",
        Level::Measured,
        "measured 2026-11-25 (U-6)",
    ),
    row("hand.move_gimbal", Level::Captured, "capture 11"),
    row(
        "hand.return",
        Level::Captured,
        "capture 11; measured 2026-10-25",
    ),
    row("presets", Level::Measured, "measured 2026-10-31 (#104)"),
    row(
        "views",
        Level::Captured,
        "capture 11, 18; measured 2026-10-31 (#100: empty list)",
    ),
    row(
        "exposure.mode",
        Level::Measured,
        "measured 2026-10-31 (#97)",
    ),
    row(
        "exposure.region",
        Level::Captured,
        "capture 12; measured 2026-10-24",
    ),
    row(
        "exposure.time",
        Level::Measured,
        "measured 2026-10-31 (#97); bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "exposure.gain",
        Level::Captured,
        "capture 12; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "exposure.compensation",
        Level::Captured,
        "capture 08b, 12; EV scale: measured 2026-11-25 (U-20)",
    ),
    row(
        "exposure.hdr",
        Level::Captured,
        "capture 12; measured 2026-10-24",
    ),
    row(
        "exposure.anti_flicker",
        Level::Captured,
        "capture 12; measured 2026-10-24 (0–2); order with `exposure.mode`: measured 2026-11-25 (CAM-13)",
    ),
    row(
        "image.brightness",
        Level::Measured,
        "measured 2026-10-24; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.contrast",
        Level::Captured,
        "capture 08b, 12; measured 2026-10-24; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.saturation",
        Level::Captured,
        "capture 08b, 12; measured 2026-10-24; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.hue",
        Level::Captured,
        "capture 08b, 12; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.sharpness",
        Level::Captured,
        "capture 08b, 12; measured 2026-10-24; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.white_balance.auto",
        Level::Captured,
        "capture 08b, 12; default: capture 14, 16, 18 (UVC range answers)",
    ),
    row(
        "image.white_balance.temperature",
        Level::Captured,
        "capture 08b, 12; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.mirror",
        Level::Captured,
        "capture 05; measured 2026-10-24",
    ),
    row("image.frame_rate", Level::Measured, "measured 2026-10-24"),
    row(
        "focus.mode",
        Level::Captured,
        "capture 12 (focus auto); measured 2026-10-31 (#97); UVC focus auto: measured 2026-11-25 (CAM-14)",
    ),
    row(
        "focus.region",
        Level::Captured,
        "capture 12; measured 2026-10-24",
    ),
    row(
        "focus.position",
        Level::Captured,
        "capture 12; measured 2026-10-31 (#97: read back; #99: no visible effect); default: capture 14, 16, 18 (UVC range answers); refused while the continuous autofocus is on: measured 2026-11-25 (CAM-15)",
    ),
    row(
        "audio.enabled",
        Level::Captured,
        "capture 14; measured 2026-10-24 (restart; the interface goes and returns); readback: measured 2026-11-25 (CAM-8: the interface stays away with the microphone off, also over a cold re-plug)",
    ),
    row(
        "audio.volume",
        Level::Measured,
        "measured 2026-10-31 (#97: 30, 55, 85 read back)",
    ),
    row(
        "audio.noise_reduction",
        Level::Captured,
        "capture 06; measured 2026-10-24",
    ),
    row(
        "audio.auto_gain",
        Level::Captured,
        "capture 06; measured 2026-10-24",
    ),
    row(
        "audio.in_sleep",
        Level::Captured,
        "capture 06; measured 2026-10-24",
    ),
    row(
        "led.brightness",
        Level::Captured,
        "capture 07; measured 2026-10-24",
    ),
    row(
        "led.blink",
        Level::Captured,
        "capture 11, 18; measured 2026-10-25 (the LED blinks)",
    ),
    row(
        "sleep.auto.enabled",
        Level::Captured,
        "capture 02; measured 2026-10-24",
    ),
    row(
        "sleep.auto.seconds",
        Level::Captured,
        "capture 02; measured 2026-10-24",
    ),
    row(
        "sleep.background",
        Level::Captured,
        "capture 03, 18 (`own_image`); measured 2026-10-24",
    ),
    row(
        "sleep.background_mirror",
        Level::Captured,
        "capture 03; measured 2026-10-24",
    ),
    row(
        "sleep.wake_zoom",
        Level::Measured,
        "measured 2026-10-31 (#97: read back; #102: no effect)",
    ),
    row(
        "power_on.mode",
        Level::Measured,
        "measured 2026-10-31 (#97: read back; #102: applied at power-on)",
    ),
    row(
        "power_on.start.source",
        Level::Captured,
        "capture 08; measured 2026-10-25 (`own`); `preset_<slot>`: measured 2026-11-25 (U-26)",
    ),
    row(
        "power_on.start.pose",
        Level::Captured,
        "capture 08; measured 2026-10-25",
    ),
    row(
        "power_on.start.mode",
        Level::Captured,
        "capture 08 (normal, unset); written values read back: measured 2026-11-25 (U-18); close_up: measured 2026-11-25 (U-19)",
    ),
    row(
        "power_on.settings.tracking_speed",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.autofocus",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.auto_exposure",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.exposure_compensation",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.white_balance_mode",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.white_balance_temperature",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.contrast",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.saturation",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.sharpness",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.hue",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "do:gimbal.move",
        Level::Captured,
        "capture 18 (`0x6444`); measured 2026-10-25; measured 2026-10-31 (#98: speed 1, 5, 100 °/s; tilt 80, pan 145); measured 2026-11-25 (CAM-17: 90 °/s, pan only and tilt only arrive; tilt 89 at 90 °/s and 180 °/s to tilt 0 do not move)",
    ),
    row(
        "do:gimbal.stop",
        Level::Measured,
        "measured 2026-10-31 (#98)",
    ),
    row(
        "do:gimbal.turn",
        Level::Captured,
        "capture 10; measured 2026-10-31 (#98, the sign flip included)",
    ),
    row(
        "do:zoom.move",
        Level::Captured,
        "capture 11 (30); measured 2026-10-31 (#98); 10 and 50: measured 2026-11-25 (U-12)",
    ),
    row("do:zoom.stop", Level::Measured, "measured 2026-10-31 (#98)"),
    row("do:preset.save", Level::Captured, "capture 10"),
    row("do:preset.recall", Level::Captured, "capture 10"),
    row("do:preset.rename", Level::Captured, "capture 10"),
    row("do:preset.delete", Level::Captured, "capture 10"),
    row("do:view.save", Level::Captured, "capture 11"),
    row("do:view.go", Level::Measured, "measured 2026-10-31 (#100)"),
    row("do:view.rename", Level::Captured, "capture 11"),
    row("do:view.delete", Level::Captured, "capture 11"),
    row(
        "do:zone.return_here",
        Level::Captured,
        "capture 11; measured 2026-10-25",
    ),
    row("do:zone.go_return", Level::Measured, "measured 2026-10-25"),
    row(
        "do:hand.return_here",
        Level::Captured,
        "capture 11; measured 2026-10-25",
    ),
    row("do:hand.go_return", Level::Measured, "measured 2026-10-25"),
    row(
        "do:hand.return_clear",
        Level::Measured,
        "measured 2026-10-25",
    ),
    row(
        "do:hand.area_reset",
        Level::Measured,
        "measured 2026-11-25 (U-7)",
    ),
    row(
        "do:power_on.set_pose",
        Level::Captured,
        "capture 08; measured 2026-10-25 (readback)",
    ),
    row("do:power_on.use_current", Level::Captured, "capture 08"),
    row("do:power_on.use_preset", Level::Captured, "capture 10"),
    row(
        "do:power_on.go",
        Level::Measured,
        "measured 2026-10-31 (#79)",
    ),
    row(
        "do:sleep",
        Level::Captured,
        "capture 13; measured 2026-10-24",
    ),
    row(
        "do:wake",
        Level::Captured,
        "capture 13; measured 2026-10-24",
    ),
    row(
        "do:factory_reset",
        Level::Captured,
        "capture 16; measured 2026-10-24",
    ),
    // marks of spec 9.3 on a value, a bound or an aspect: no backing
    row("hand.area.pan#order", Level::None, "none"),
    row("hand.area.tilt#order", Level::None, "none"),
    row("zone.area.pan#order", Level::None, "none"),
    row("zone.area.tilt#order", Level::None, "none"),
    row("power_on.start.mode#power_on_effect", Level::None, "none"),
    row(
        "power_on.settings.tracking_speed#values",
        Level::None,
        "none",
    ),
    row("power_on.settings.autofocus#values", Level::None, "none"),
    row(
        "power_on.settings.auto_exposure#values",
        Level::None,
        "none",
    ),
    row(
        "power_on.settings.exposure_compensation#values",
        Level::None,
        "none",
    ),
    row(
        "power_on.settings.white_balance_mode#values",
        Level::None,
        "none",
    ),
    row(
        "power_on.settings.white_balance_temperature#values",
        Level::None,
        "none",
    ),
    row("power_on.settings.contrast#values", Level::None, "none"),
    row("power_on.settings.saturation#values", Level::None, "none"),
    row("power_on.settings.sharpness#values", Level::None, "none"),
    row("power_on.settings.hue#values", Level::None, "none"),
    // the camera's UVC range answers
    row("exposure.time@min", Level::Captured, "capture 01"),
    row("exposure.time@max", Level::Captured, "capture 01"),
    row("exposure.time#step", Level::Captured, "capture 01"),
    row("exposure.time#default", Level::Captured, "capture 01"),
    row("exposure.gain#default", Level::Captured, "capture 01"),
    row(
        "focus.position#default",
        Level::Captured,
        "capture 14, 16, 18",
    ),
    row("image.brightness@min", Level::Captured, "capture 01"),
    row("image.brightness@max", Level::Captured, "capture 01"),
    row("image.brightness#default", Level::Captured, "capture 01"),
    row("image.contrast@min", Level::Captured, "capture 01"),
    row("image.contrast@max", Level::Captured, "capture 01"),
    row("image.contrast#default", Level::Captured, "capture 01"),
    row("image.saturation@min", Level::Captured, "capture 01"),
    row("image.saturation@max", Level::Captured, "capture 01"),
    row("image.saturation#default", Level::Captured, "capture 01"),
    row("image.hue@min", Level::Captured, "capture 01"),
    row("image.hue@max", Level::Captured, "capture 01"),
    row("image.hue#default", Level::Captured, "capture 01"),
    row("image.sharpness@min", Level::Captured, "capture 01"),
    row("image.sharpness@max", Level::Captured, "capture 01"),
    row("image.sharpness#default", Level::Captured, "capture 01"),
    row(
        "image.white_balance.auto#default",
        Level::Captured,
        "capture 14, 16, 18",
    ),
    row(
        "image.white_balance.temperature@min",
        Level::Captured,
        "capture 01",
    ),
    row(
        "image.white_balance.temperature@max",
        Level::Captured,
        "capture 01",
    ),
    row(
        "image.white_balance.temperature#step",
        Level::Captured,
        "capture 01",
    ),
    row(
        "image.white_balance.temperature#default",
        Level::Captured,
        "capture 01",
    ),
];
