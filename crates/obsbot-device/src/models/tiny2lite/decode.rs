// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! The Tiny 2 Lite's answers as leaf values (design spec 4.2, 4.3; field ledger
//! `docs/field-ledger.md`). Pure functions: bytes in, `leaf path → value` out.
//!
//! - Angles are converted to the units of 4.2 (pan positive = the camera's left, tilt positive
//!   = up; the camera's pitch is positive down) and every number is rounded to the precision of
//!   its setting where it is made (angles 0.1, zoom 0.01, fractions 0.01).
//! - A value the tables do not name is published as `raw:<n>` (4.2), never as a neighbouring
//!   value; a two-byte pair (the tracking mode) is `raw:<first byte * 256 + second byte>`.
//! - An answer too short for its layout, or a float that is not finite, is no answer
//!   (`None`): the group's read fails and its leaves go to `unread`.

use crate::contract::OnAir;
use crate::models::tiny2lite::presets::{self, Pose};
use obsbot_proto::Usb;
use obsbot_proto::Values;
use obsbot_proto::values::{number, raw, rounded};
use serde_json::{Map, Value, json};

/// The tracking modes and their (mode, sub-mode) pair: the short command `16 02 <m> <n>` and
/// status bytes 0x18 / 0x1c (capture 11).
pub const MODE_PAIRS: [(&str, [u8; 2]); 8] = [
    ("static", [0, 0]),
    ("normal", [2, 0]),
    ("upper_body", [2, 1]),
    ("close_up", [2, 2]),
    ("headless", [2, 3]),
    ("lower_body", [2, 4]),
    ("hand", [3, 0]),
    ("group", [1, 0]),
];
/// `tracking.profile`: framed `0x0cc4`, status 0x24.
pub const PROFILE: [(&str, u8); 3] = [("standard", 0), ("headroom", 1), ("sport", 2)];
/// `tracking.speed`: framed `0x0944`, query `0x0984`.
pub const SPEED: [(&str, u8); 6] = [
    ("lazy", 0),
    ("slow", 1),
    ("normal", 2),
    ("fast", 3),
    ("very_fast", 4),
    ("auto", 5),
];
/// `framing.size`: framed `0x0ac4`, query `0x0b04`.
pub const FRAMING_SIZE: [(&str, u8); 6] = [
    ("long_shot", 0),
    ("medium_long_shot", 1),
    ("medium_shot", 2),
    ("medium_close_up", 3),
    ("close_up", 4),
    ("big_close_up", 5),
];
/// `framing.zoom_speed`: framed `0x0b44`, query `0x0b84`.
pub const ZOOM_SPEED: [(&str, u8); 3] = [("slow", 0), ("medium", 1), ("fast", 2)];
/// `framing.part`: framed `0x1584`, query `0x15c4`.
pub const PART: [(&str, u8); 3] = [("normal", 0), ("head_removal", 1), ("lower_body", 2)];
/// `zoom.field_of_view`: short `04 01 <v>`, status 0x11 (`custom` only reads).
pub const FIELD_OF_VIEW: [(&str, u8); 4] =
    [("wide", 0), ("medium", 1), ("narrow", 2), ("custom", 3)];
/// `audio.noise_reduction`: short `0a 01 <v>`, status 0x08.
pub const NOISE_REDUCTION: [(&str, u8); 4] =
    [("off", 0), ("weak", 1), ("medium", 2), ("strong", 3)];
/// `exposure.anti_flicker`: UVC power line frequency 0–2, status 0x03 (`auto` only reads).
pub const ANTI_FLICKER: [(&str, u8); 4] = [("off", 0), ("50hz", 1), ("60hz", 2), ("auto", 3)];
/// `exposure.region` (short `03 01 <v>`, status 0x07) and `focus.region` (framed `0x3602`,
/// status 0x0d).
pub const REGION: [(&str, u8); 2] = [("global", 0), ("face", 1)];
/// `sleep.background`: short `0e 02 00 <v>`, status 0x1b (`own_image` only reads).
pub const SLEEP_BACKGROUND: [(&str, u8); 3] = [("black", 0), ("image", 1), ("own_image", 3)];
/// `hand.side`: framed `0x2244`, `0x0104` byte 2.
pub const HAND_SIDE: [(&str, u8); 2] = [("right", 0), ("left", 1)];
/// `focus.mode`: framed `0x3442` (u32), query `0x3642` word 0.
pub const FOCUS_MODE: [(&str, u8); 3] = [("continuous", 1), ("single", 2), ("manual", 3)];
/// `power_on.start.mode`: byte 0 of the initial-state block (`0x3e84` / `0x3e44`). `hand`
/// reads as 7 or 8 and is written as neither (U-18).
pub const START_MODE: [(&str, u8); 10] = [
    ("static", 0xff),
    ("normal", 0x00),
    ("upper_body", 0x01),
    ("close_up", 0x02),
    ("headless", 0x03),
    ("lower_body", 0x04),
    ("hand", 0x07),
    ("hand", 0x08),
    ("group", 0x09),
    ("unset", 0xfe),
];

/// The name of `v` in `table`, else `raw:<v>`.
pub fn named(table: &[(&str, u8)], v: u8) -> Value {
    table
        .iter()
        .find(|(_, b)| *b == v)
        .map_or_else(|| raw(i64::from(v)), |(name, _)| json!(name))
}

/// The byte of `name` in `table` (the first, where two bytes read as one name).
pub fn byte_of(table: &[(&str, u8)], name: &str) -> Option<u8> {
    table.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}

/// A flag byte: 0 false, 1 true, anything else `raw:<n>`.
pub fn flag(v: u8) -> Value {
    match v {
        0 => Value::Bool(false),
        1 => Value::Bool(true),
        other => raw(i64::from(other)),
    }
}

/// The tracking mode of a (mode, sub-mode) pair.
pub fn mode(pair: [u8; 2]) -> Value {
    MODE_PAIRS.iter().find(|(_, p)| *p == pair).map_or_else(
        || raw(i64::from(pair[0]) * 256 + i64::from(pair[1])),
        |(name, _)| json!(name),
    )
}

/// The (mode, sub-mode) pair of a tracking mode.
pub fn mode_pair(name: &str) -> Option<[u8; 2]> {
    MODE_PAIRS.iter().find(|(n, _)| *n == name).map(|(_, p)| *p)
}

fn angle(x: f64) -> Value {
    rounded(x, 0.1)
}

fn f32_at(d: &[u8], word: usize) -> Option<f64> {
    let bytes: [u8; 4] = d.get(word * 4..word * 4 + 4)?.try_into().ok()?;
    let v = f32::from_le_bytes(bytes);
    v.is_finite().then_some(f64::from(v))
}

/// A pose as published: `{pan, tilt, zoom}` rounded to 0.1, 0.1 and 0.01.
pub fn pose_value(p: &Pose) -> Value {
    json!({
        "pan": angle(f64::from(p.pan)),
        "tilt": angle(f64::from(p.tilt)),
        "zoom": rounded(f64::from(p.zoom), 0.01),
    })
}

fn range(min: f64, max: f64) -> Value {
    json!({ "min": angle(min), "max": angle(max) })
}

/// A pitch range (min, max) as the camera reports it (positive down) in the units of 4.2
/// (positive up): min and max swap and flip. Carried from `camera.rs` (`pitch_to_tilt_range`).
pub fn pitch_to_tilt_range(pitch: (f64, f64)) -> (f64, f64) {
    (-pitch.1 + 0.0, -pitch.0 + 0.0)
}

/// A tilt range in the units of 4.2 as the camera takes it (pitch): min and max swap and flip.
/// Carried from `camera.rs` (`tilt_to_pitch_range`).
pub fn tilt_to_pitch_range(tilt: (f32, f32)) -> (f32, f32) {
    (-tilt.1 + 0.0, -tilt.0 + 0.0)
}

/// The status block (selector 6), group `status`. Offsets are those of the field ledger.
pub fn status(b: &[u8; 60]) -> Values {
    let auto_sleep = i16::from_le_bytes([b[0x0a], b[0x0b]]);
    let wake_zoom = u16::from_le_bytes([b[0x16], b[0x17]]);
    let power_on = b[0x20];
    let mut v = Values::new();
    let mut put = |path: &str, value: Value| {
        v.insert(path.to_string(), value);
    };
    put("asleep", flag(b[0x02]));
    put("exposure.anti_flicker", named(&ANTI_FLICKER, b[0x03]));
    put("exposure.hdr", flag(b[0x06]));
    put("exposure.region", named(&REGION, b[0x07]));
    put("audio.noise_reduction", named(&NOISE_REDUCTION, b[0x08]));
    put("sleep.auto.enabled", Value::Bool(auto_sleep >= 0));
    put(
        "sleep.auto.seconds",
        number(f64::from(auto_sleep.unsigned_abs())),
    );
    put("focus.region", named(&REGION, b[0x0d]));
    put("audio.in_sleep", flag(b[0x10]));
    put("zoom.field_of_view", named(&FIELD_OF_VIEW, b[0x11]));
    put("image.mirror", flag(b[0x13]));
    put(
        "sleep.wake_zoom",
        rounded(1.0 + 3.0 * f64::from(wake_zoom) / 100.0, 0.01),
    );
    put("tracking.mode", mode([b[0x18], b[0x1c]]));
    put("audio.auto_gain", flag(b[0x19]));
    put("sleep.background", named(&SLEEP_BACKGROUND, b[0x1b]));
    put("sleep.background_mirror", flag(b[0x1d]));
    put("image.frame_rate", number(f64::from(b[0x1f])));
    put(
        "power_on.mode",
        match mode([power_on >> 4, power_on & 0x0f]) {
            Value::String(s) if s.starts_with("raw:") => raw(i64::from(power_on)),
            named => named,
        },
    );
    put("led.brightness", number(f64::from(b[0x21])));
    put("tracking.profile", named(&PROFILE, b[0x24]));
    v
}

/// The answer to `0x0104` (12 bytes), group `gestures`.
pub fn gestures(d: &[u8]) -> Option<Values> {
    if d.len() < 12 {
        return None;
    }
    let mut v = Values::new();
    v.insert("hand.side".into(), named(&HAND_SIDE, d[2]));
    v.insert("gestures.target_lock".into(), flag(d[3]));
    v.insert("gestures.zoom".into(), flag(d[4]));
    v.insert("gestures.dynamic_zoom".into(), flag(d[5]));
    v.insert("gestures.direction_change".into(), flag(d[6]));
    v.insert(
        "gestures.zoom_ratio".into(),
        rounded(f64::from(d[7]) / 10.0, 0.1),
    );
    v.insert("zone.enabled".into(), flag(d[10]));
    v.insert("gimbal.invert".into(), flag(d[11]));
    Some(v)
}

/// The answer to `0x0043` on receiver 3, group `pose`: bytes 8–9 the pitch (positive down) and
/// 10–11 the yaw (positive left), i16 in hundredths of a degree. Carried from `camera.rs`
/// (`LivePose::decode`).
pub fn pose(d: &[u8]) -> Option<Values> {
    let i16_at = |i: usize| d.get(i..i + 2).map(|b| i16::from_le_bytes([b[0], b[1]]));
    let pitch = f64::from(i16_at(8)?) / 100.0;
    let yaw = f64::from(i16_at(10)?) / 100.0;
    let mut v = Values::new();
    v.insert("gimbal.pan".into(), angle(yaw));
    v.insert("gimbal.tilt".into(), angle(-pitch + 0.0));
    Some(v)
}

/// The zoom factor of the answer to `0x6804` (f32).
pub fn zoom_factor(d: &[u8]) -> Option<f64> {
    f32_at(d, 0)
}

/// The answer to `0x6804`, group `zoom`.
pub fn zoom(d: &[u8]) -> Option<Values> {
    let mut v = Values::new();
    v.insert("zoom.factor".into(), rounded(zoom_factor(d)?, 0.01));
    Some(v)
}

/// The answer to `0x01c4`, group `target`: `01 01 <detected> 01 00 00` and four f32 (left, top,
/// right, bottom as picture fractions). An answer that is not that (the single `00` of a camera
/// without a target) is no target: `detected` false, `box` null. Carried from `status.rs`
/// (`Target::decode`).
pub fn target(d: &[u8]) -> Values {
    let decoded = (|| {
        if d.len() < 22 || d[0] != 1 || d[2] > 1 {
            return None;
        }
        let mut b = [0f64; 4];
        for (i, c) in b.iter_mut().enumerate() {
            *c = f32_at(&d[6..], i)?;
        }
        Some((d[2] == 1, b))
    })();
    match decoded {
        Some((detected, [left, top, right, bottom])) => {
            let f = |x: f64| rounded(x, 0.01);
            let mut v = Values::new();
            v.insert("tracking.target.detected".into(), Value::Bool(detected));
            v.insert(
                "tracking.target.box".into(),
                json!({"left": f(left), "top": f(top), "right": f(right), "bottom": f(bottom)}),
            );
            v
        }
        None => no_target(),
    }
}

/// The target group when the camera answers that it has no target (a single `00`).
pub fn no_target() -> Values {
    let mut v = Values::new();
    v.insert("tracking.target.detected".into(), Value::Bool(false));
    v.insert("tracking.target.box".into(), Value::Null);
    v
}

/// The readback of one leaf of the group `framing` from the answer to its query.
pub fn framing(leaf: &str, d: &[u8]) -> Option<Value> {
    let byte = d.first().copied();
    Some(match leaf {
        "tracking.speed" => named(&SPEED, byte?),
        "framing.auto_zoom" => flag(byte?),
        "framing.size" => named(&FRAMING_SIZE, byte?),
        // a camera that was never set reports the speed itself: 10, 30 or 50 (#98); carried
        // from `camera.rs` (`auto_zoom_speed_reading`)
        "framing.zoom_speed" => match byte? {
            10 => json!("slow"),
            30 => json!("medium"),
            50 => json!("fast"),
            other => named(&ZOOM_SPEED, other),
        },
        "framing.offset.x" | "framing.offset.y" => rounded(f32_at(d, 0)?, 0.01),
        "framing.part" => named(&PART, byte?),
        _ => return None,
    })
}

/// The answer to `0x3642` on receiver 2, group `focus`: u32 mode, u32 manual position.
pub fn focus(d: &[u8]) -> Option<Values> {
    let word = |i: usize| -> Option<u32> {
        Some(u32::from_le_bytes(
            d.get(i * 4..i * 4 + 4)?.try_into().ok()?,
        ))
    };
    let mode = word(0)?;
    let position = word(1)?;
    let mut v = Values::new();
    v.insert(
        "focus.mode".into(),
        match u8::try_from(mode) {
            Ok(m) => named(&FOCUS_MODE, m),
            Err(_) => raw(i64::from(mode)),
        },
    );
    v.insert("focus.position".into(), number(f64::from(position)));
    Some(v)
}

/// The answer to `0x80c2` on receiver 2: `audio.volume`, i16.
pub fn volume(d: &[u8]) -> Option<Value> {
    let v = i16::from_le_bytes(d.get(..2)?.try_into().ok()?);
    Some(number(f64::from(v)))
}

/// `audio.enabled`: whether the camera's USB audio interface (class `01`) is there (U-23).
pub fn audio_enabled(usb: &Usb) -> Value {
    Value::Bool(usb.interfaces.iter().any(|c| c == "01"))
}

/// The UVC controls of the Tiny 2 Lite: leaf, V4L2 control id.
pub const UVC: [(&str, u32); 11] = [
    ("exposure.mode", 0x009a_0901),
    ("exposure.time", 0x009a_0902),
    ("exposure.gain", 0x0098_0913),
    ("exposure.compensation", 0x0098_091c),
    ("image.brightness", 0x0098_0900),
    ("image.contrast", 0x0098_0901),
    ("image.saturation", 0x0098_0902),
    ("image.hue", 0x0098_0903),
    ("image.sharpness", 0x0098_091b),
    ("image.white_balance.auto", 0x0098_090c),
    ("image.white_balance.temperature", 0x0098_091a),
];
/// UVC zoom absolute (`zoom.factor` is written here, 0–100 for 1.0–4.0).
pub const ZOOM_ABSOLUTE: u32 = 0x009a_090d;
/// UVC focus absolute (`focus.position` is written here).
pub const FOCUS_ABSOLUTE: u32 = 0x009a_090a;
/// UVC power line frequency (`exposure.anti_flicker` is written here).
pub const POWER_LINE: u32 = 0x0098_0918;

/// The value of a leaf of the group `uvc` from the V4L2 value of its control.
pub fn uvc(leaf: &str, v: i32) -> Value {
    match leaf {
        // V4L2 1 is manual; 0 (written by 0.13 for automatic) and 3 read as automatic (#97)
        "exposure.mode" => match v {
            1 => json!("manual"),
            0 | 3 => json!("auto"),
            other => raw(i64::from(other)),
        },
        // 100 µs units
        "exposure.time" => number(f64::from(v) * 100.0),
        "image.white_balance.auto" => match v {
            0 => Value::Bool(false),
            1 => Value::Bool(true),
            other => raw(i64::from(other)),
        },
        _ => number(f64::from(v)),
    }
}

/// The group `zone` from the answers to `0x4c44`, `0x4cc4` (pan min, max), `0x4d44`, `0x4dc4`
/// (pitch min, max), `0x4e44` (follow) and `0x4ec4` (return pose; its lead is always 0, so it
/// is never null).
pub fn zone(
    pan_min: &[u8],
    pan_max: &[u8],
    pitch_min: &[u8],
    pitch_max: &[u8],
    follow: &[u8],
    ret: &[u8],
) -> Option<Values> {
    let (tilt_min, tilt_max) = pitch_to_tilt_range((f32_at(pitch_min, 0)?, f32_at(pitch_max, 0)?));
    let mut v = Values::new();
    v.insert(
        "zone.area.pan".into(),
        range(f32_at(pan_min, 0)?, f32_at(pan_max, 0)?),
    );
    v.insert("zone.area.tilt".into(), range(tilt_min, tilt_max));
    v.insert("zone.follow".into(), flag(*follow.first()?));
    v.insert(
        "zone.return".into(),
        pose_value(&presets::parse_zone_return(ret)?),
    );
    Some(v)
}

/// The group `hand` from the answers to `0x2004` (yaw min, max, pitch min, max as f32; the
/// pan and tilt flags in bytes 22 and 23) and `0x2504` (return pose, lead −1 = none). Carried
/// from `camera.rs` (`HandArea::decode`, `hand_enabled`).
pub fn hand(area: &[u8], ret: &[u8]) -> Option<Values> {
    let (tilt_min, tilt_max) = pitch_to_tilt_range((f32_at(area, 2)?, f32_at(area, 3)?));
    let mut v = Values::new();
    v.insert(
        "hand.area.pan".into(),
        range(f32_at(area, 0)?, f32_at(area, 1)?),
    );
    v.insert("hand.area.tilt".into(), range(tilt_min, tilt_max));
    v.insert("hand.axes.pan".into(), flag(*area.get(22)?));
    v.insert("hand.axes.tilt".into(), flag(*area.get(23)?));
    v.insert(
        "hand.return".into(),
        presets::parse_hand_return(ret)?.map_or(Value::Null, |p| pose_value(&p)),
    );
    Some(v)
}

/// `hand.return_after` from the answer to `0x2484` (f32 seconds; U-6).
pub fn return_after(d: &[u8]) -> Option<Value> {
    Some(rounded(f32_at(d, 0)?, 0.1))
}

/// The group `power_on` from the answers to `0x3884` (lead: 3 own, 0–2 a preset slot; then
/// the pose) and `0x3e84` (the initial-state block: byte 0 the start mode, the settings bytes,
/// `ff` = not set).
pub fn power_on(source: &[u8], block: &[u8]) -> Option<Values> {
    let (lead, pose) = presets::parse_pose(source)?;
    if block.len() < 24 {
        return None;
    }
    let set = |i: usize| {
        if block[i] == 0xff {
            Value::Null
        } else {
            number(f64::from(block[i]))
        }
    };
    let mut v = Values::new();
    v.insert(
        "power_on.start.source".into(),
        match lead {
            3 => json!("own"),
            0..=2 => json!(format!("preset_{lead}")),
            other => raw(i64::from(other)),
        },
    );
    v.insert("power_on.start.pose".into(), pose_value(&pose));
    v.insert("power_on.start.mode".into(), named(&START_MODE, block[0]));
    v.insert("power_on.settings.tracking_speed".into(), set(1));
    v.insert("power_on.settings.autofocus".into(), set(2));
    v.insert("power_on.settings.auto_exposure".into(), set(6));
    v.insert(
        "power_on.settings.exposure_compensation".into(),
        number(f64::from(block[8])),
    );
    v.insert("power_on.settings.white_balance_mode".into(), set(13));
    v.insert(
        "power_on.settings.white_balance_temperature".into(),
        if block[13] == 5 {
            number(f64::from(u16::from_le_bytes([block[14], block[15]])))
        } else {
            Value::Null
        },
    );
    v.insert("power_on.settings.contrast".into(), set(20));
    v.insert("power_on.settings.saturation".into(), set(21));
    v.insert("power_on.settings.sharpness".into(), set(22));
    v.insert("power_on.settings.hue".into(), set(23));
    Some(v)
}

/// One item of `views` from the answers to `0x1504` (name) and `0x1444` (pose) for `id`.
pub fn view(id: u32, name: &[u8], pose: &[u8]) -> Option<Value> {
    let (_, pose) = presets::parse_pose(pose)?;
    Some(json!({
        "id": id,
        "name": presets::parse_view_name(name),
        "pose": pose_value(&pose),
    }))
}

/// The group `presets` from a selector-`0x0c` read (count, then the slots) and the
/// selector-`0x0d` reads that followed it. A list is read only whole: an entry that does not
/// decode, fewer entries than the count, or an entry whose slot the count's list does not name
/// is a failed read (audit: a short list counted as a successful read).
pub fn presets(list: &[u8], entries: &[[u8; 60]]) -> Option<Values> {
    let slots = presets::parse_slots(list);
    if entries.len() != slots.len() {
        return None;
    }
    let mut items = Vec::new();
    for e in entries {
        let p = presets::parse_entry(e)?;
        if !slots.contains(&p.slot) {
            return None;
        }
        items.push(p);
    }
    items.sort_by_key(|p| p.slot);
    let list = items
        .iter()
        .map(|p| json!({"slot": p.slot, "name": p.name, "pose": pose_value(&p.pose)}))
        .collect();
    let mut v = Values::new();
    v.insert("presets".into(), Value::Array(list));
    Some(v)
}

/// The group `on_air` from one sample of the on-air probe.
pub fn on_air(o: &OnAir) -> Values {
    let apps = o
        .apps
        .iter()
        .map(|name| {
            let mut m = Map::new();
            m.insert("name".into(), json!(name));
            Value::Object(m)
        })
        .collect();
    let mut v = Values::new();
    v.insert("on_air.active".into(), Value::Bool(o.active));
    v.insert("on_air.apps".into(), Value::Array(apps));
    v
}

/// The firmware version from the answer to `0x0408` on receiver `0x0d`: a u32 shown as
/// byte 3.2.1.0. Carried from `status.rs` (`firmware_version`).
pub fn firmware(d: &[u8]) -> Option<String> {
    let v: [u8; 4] = d.get(..4)?.try_into().ok()?;
    Some(format!("{}.{}.{}.{}", v[3], v[2], v[1], v[0]))
}

/// The serial from the answer to `0x18c8` on receiver `0x0d`: ASCII, trailing NULs dropped.
/// The daemon checks its format (5.5); it is never logged.
pub fn serial(d: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(d).ok()?;
    let text = text.trim_end_matches('\0');
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex60(s: &str) -> [u8; 60] {
        let mut b = [0u8; 60];
        let d = hex::decode(s).unwrap();
        b[..d.len()].copy_from_slice(&d);
        b
    }

    /// The status block of a camera read in captures 05, 06 and 07 before any change.
    const BLOCK: &str = "2701000121000101010188ff000101000003000001002100000103000000001e00031000020000000000000000000000000000000000000000000000";

    /// The leaves whose value differs between two decoded blocks.
    fn changed(a: &Values, b: &Values) -> Vec<(String, Value)> {
        b.iter()
            .filter(|(k, v)| a.get(*k) != Some(v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    #[test]
    fn a_recorded_status_block_decodes_leaf_by_leaf() {
        let v = status(&hex60(BLOCK));
        let want = json!({
            "asleep": false,
            "audio.auto_gain": true,
            "audio.in_sleep": false,
            "audio.noise_reduction": "weak",
            "exposure.anti_flicker": "50hz",
            "exposure.hdr": true,
            "exposure.region": "face",
            "focus.region": "face",
            "image.frame_rate": 30,
            "image.mirror": false,
            "led.brightness": 3,
            "power_on.mode": "static",
            "sleep.auto.enabled": false,
            "sleep.auto.seconds": 120,
            "sleep.background": "black",
            "sleep.background_mirror": false,
            "sleep.wake_zoom": 1.99,
            "tracking.mode": "static",
            "tracking.profile": "sport",
            "zoom.field_of_view": "custom",
        });
        assert_eq!(serde_json::to_value(&v).unwrap(), want);
    }

    #[test]
    fn each_recorded_change_shows_in_its_own_leaf_only() {
        let base = status(&hex60(BLOCK));
        let with = |at: usize, byte: u8| {
            let mut b = hex60(BLOCK);
            b[at] = byte;
            status(&b)
        };
        // capture 05: mirror on (short 14 01 01), status 0x13
        assert_eq!(
            changed(&base, &with(0x13, 1)),
            [("image.mirror".to_string(), json!(true))]
        );
        // capture 07: LED 2, 1, 0 (short 1a 01 <v>), status 0x21
        for led in [2u8, 1, 0] {
            assert_eq!(
                changed(&base, &with(0x21, led)),
                [("led.brightness".to_string(), json!(led))]
            );
        }
        // capture 06: microphone while asleep on (0x10), noise reduction medium, strong, off
        // (0x08), automatic gain off (0x19)
        assert_eq!(
            changed(&base, &with(0x10, 1)),
            [("audio.in_sleep".to_string(), json!(true))]
        );
        for (byte, name) in [(2u8, "medium"), (3, "strong"), (0, "off")] {
            assert_eq!(
                changed(&base, &with(0x08, byte)),
                [("audio.noise_reduction".to_string(), json!(name))]
            );
        }
        assert_eq!(
            changed(&base, &with(0x19, 0)),
            [("audio.auto_gain".to_string(), json!(false))]
        );
        // capture 09: field of view wide, medium, narrow (status 0x11)
        for (byte, name) in [(0u8, "wide"), (1, "medium"), (2, "narrow")] {
            assert_eq!(
                changed(&base, &with(0x11, byte)),
                [("zoom.field_of_view".to_string(), json!(name))]
            );
        }
    }

    #[test]
    fn auto_sleep_is_one_word_with_the_sign_as_the_switch() {
        // capture 02: 600 s on (short 0b 02 58 02), 30 s on, then off with 120 s kept (88 ff)
        let word = |lo: u8, hi: u8| {
            let mut b = hex60(BLOCK);
            b[0x0a] = lo;
            b[0x0b] = hi;
            let v = status(&b);
            (
                v["sleep.auto.enabled"].clone(),
                v["sleep.auto.seconds"].clone(),
            )
        };
        assert_eq!(word(0x58, 0x02), (json!(true), json!(600)));
        assert_eq!(word(0x1e, 0x00), (json!(true), json!(30)));
        assert_eq!(word(0x88, 0xff), (json!(false), json!(120)));
    }

    #[test]
    fn unknown_status_values_are_raw_never_a_neighbour() {
        let mut b = hex60(BLOCK);
        b[0x02] = 2; // asleep
        b[0x11] = 4; // field of view
        b[0x18] = 2; // tracking mode 2 / 9
        b[0x1c] = 9;
        b[0x1b] = 2; // sleep background
        b[0x20] = 0x29; // power-on mode 2 / 9
        b[0x24] = 7; // profile
        let v = status(&b);
        assert_eq!(v["asleep"], json!("raw:2"));
        assert_eq!(v["zoom.field_of_view"], json!("raw:4"));
        assert_eq!(v["tracking.mode"], json!("raw:521"));
        assert_eq!(v["sleep.background"], json!("raw:2"));
        assert_eq!(v["power_on.mode"], json!("raw:41"));
        assert_eq!(v["tracking.profile"], json!("raw:7"));
    }

    #[test]
    fn every_tracking_mode_reads_from_its_status_pair() {
        for (name, [m, n]) in MODE_PAIRS {
            let mut b = hex60(BLOCK);
            b[0x18] = m;
            b[0x1c] = n;
            assert_eq!(status(&b)["tracking.mode"], json!(name));
            let mut b = hex60(BLOCK);
            b[0x20] = m << 4 | n;
            assert_eq!(status(&b)["power_on.mode"], json!(name), "{name}");
        }
    }

    #[test]
    fn the_recorded_0x0104_answer_decodes() {
        // capture 01: gestures target lock, zoom and dynamic zoom on, ratio 2.0, hand right
        let v = gestures(&hex::decode("000000010101001400020000").unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&v).unwrap(),
            json!({
                "gestures.direction_change": false,
                "gestures.dynamic_zoom": true,
                "gestures.target_lock": true,
                "gestures.zoom": true,
                "gestures.zoom_ratio": 2,
                "gimbal.invert": false,
                "hand.side": "right",
                "zone.enabled": false,
            })
        );
        assert!(gestures(&[0; 11]).is_none(), "short");
        let mut d = hex::decode("000000010101001400020000").unwrap();
        d[2] = 1;
        d[10] = 1;
        d[11] = 1;
        let v = gestures(&d).unwrap();
        assert_eq!(
            (&v["hand.side"], &v["zone.enabled"], &v["gimbal.invert"]),
            (&json!("left"), &json!(true), &json!(true))
        );
    }

    #[test]
    fn the_live_pose_reads_in_the_units_of_4_2() {
        // a 0x0043 answer of a Tiny 2 Lite in its sleep pose (measured): pan -0.3, tilt -85
        let sleep_pose =
            hex::decode("38004e24043000003321e1ff0000cf00fafffd20010000000101110000000000")
                .unwrap();
        let v = pose(&sleep_pose).unwrap();
        assert_eq!(
            (&v["gimbal.pan"], &v["gimbal.tilt"]),
            (&json!(-0.3), &json!(-85))
        );
        // asymmetric: pitch +15.00 (down) and yaw +20.00 (left) read as tilt -15, pan +20
        let mut d = [0u8; 12];
        d[8..10].copy_from_slice(&1500i16.to_le_bytes());
        d[10..12].copy_from_slice(&2000i16.to_le_bytes());
        let v = pose(&d).unwrap();
        assert_eq!(
            (&v["gimbal.pan"], &v["gimbal.tilt"]),
            (&json!(20), &json!(-15))
        );
        // pitch 0 reads as tilt 0, not -0
        let v = pose(&[0u8; 12]).unwrap();
        assert_eq!(serde_json::to_string(&v["gimbal.tilt"]).unwrap(), "0");
        assert!(pose(&[0u8; 11]).is_none());
    }

    #[test]
    fn the_recorded_zoom_answers_decode() {
        // capture 04: ae473140 is 2.77, cdcccc3f is 1.6
        assert_eq!(
            zoom(&hex::decode("ae473140").unwrap()).unwrap()["zoom.factor"],
            json!(2.77)
        );
        assert_eq!(
            zoom(&hex::decode("cdcccc3f").unwrap()).unwrap()["zoom.factor"],
            json!(1.6)
        );
        assert!(zoom(&[0, 0, 0xc0, 0x7f]).is_none(), "NaN");
        assert!(zoom(&[1, 2]).is_none());
    }

    #[test]
    fn the_target_box_and_a_camera_without_a_target() {
        let mut d = vec![1, 1, 1, 1, 0, 0];
        for x in [0.25f32, 0.5, 0.75, 1.0] {
            d.extend(x.to_le_bytes());
        }
        let v = target(&d);
        assert_eq!(v["tracking.target.detected"], json!(true));
        assert_eq!(
            v["tracking.target.box"],
            json!({"left": 0.25, "top": 0.5, "right": 0.75, "bottom": 1})
        );
        d[2] = 0;
        assert_eq!(target(&d)["tracking.target.detected"], json!(false));
        assert_eq!(target(&[0]), no_target(), "the single 00 of no target");
        assert_eq!(no_target()["tracking.target.box"], Value::Null);
    }

    #[test]
    fn framing_readbacks() {
        assert_eq!(framing("tracking.speed", &[4]), Some(json!("very_fast")));
        assert_eq!(framing("tracking.speed", &[6]), Some(json!("raw:6")));
        assert_eq!(framing("framing.auto_zoom", &[1]), Some(json!(true)));
        assert_eq!(framing("framing.size", &[5]), Some(json!("big_close_up")));
        // a camera nobody set reports 10, 30 or 50 (#98); after a write the index
        for (byte, name) in [
            (10u8, "slow"),
            (0, "slow"),
            (30, "medium"),
            (1, "medium"),
            (50, "fast"),
            (2, "fast"),
        ] {
            assert_eq!(framing("framing.zoom_speed", &[byte]), Some(json!(name)));
        }
        assert_eq!(framing("framing.zoom_speed", &[7]), Some(json!("raw:7")));
        assert_eq!(
            framing("framing.offset.x", &0.1f32.to_le_bytes()),
            Some(json!(0.1))
        );
        assert_eq!(
            framing("framing.offset.y", &(-0.05f32).to_le_bytes()),
            Some(json!(-0.05))
        );
        assert_eq!(framing("framing.part", &[2]), Some(json!("lower_body")));
        assert_eq!(framing("framing.part", &[]), None, "empty answer");
        assert_eq!(framing("zoom.factor", &[1]), None, "not a framing leaf");
    }

    #[test]
    fn focus_volume_and_the_audio_interface() {
        let mut d = 3u32.to_le_bytes().to_vec();
        d.extend(40u32.to_le_bytes());
        let v = focus(&d).unwrap();
        assert_eq!(
            (&v["focus.mode"], &v["focus.position"]),
            (&json!("manual"), &json!(40))
        );
        let mut d = 9u32.to_le_bytes().to_vec();
        d.extend(0u32.to_le_bytes());
        assert_eq!(focus(&d).unwrap()["focus.mode"], json!("raw:9"));
        assert!(focus(&[1, 0, 0, 0]).is_none());
        assert_eq!(volume(&[0x37, 0x00]), Some(json!(55)));
        assert_eq!(volume(&[0x37]), None);
        let mut usb = Usb {
            vendor: "3564".into(),
            product: "fef9".into(),
            product_name: None,
            bcd_device: "0510".into(),
            path: "3-2".into(),
            serial_present: false,
            interfaces: vec!["0e".into(), "0e".into(), "01".into(), "01".into()],
            extension_units: vec![],
        };
        assert_eq!(audio_enabled(&usb), json!(true));
        usb.interfaces.truncate(2);
        assert_eq!(audio_enabled(&usb), json!(false));
    }

    #[test]
    fn uvc_values() {
        assert_eq!(uvc("exposure.mode", 1), json!("manual"));
        assert_eq!(uvc("exposure.mode", 3), json!("auto"));
        assert_eq!(uvc("exposure.mode", 0), json!("auto"));
        assert_eq!(uvc("exposure.mode", 2), json!("raw:2"));
        assert_eq!(uvc("exposure.time", 330), json!(33000));
        assert_eq!(uvc("image.white_balance.auto", 1), json!(true));
        assert_eq!(uvc("image.white_balance.auto", 5), json!("raw:5"));
        assert_eq!(uvc("image.brightness", 42), json!(42));
    }

    #[test]
    fn the_recorded_zone_answers_decode_in_the_units_of_4_2() {
        // capture 11: pan -100..100, pitch -30..30, follow off, return pose 0/0/1
        let v = zone(
            &hex::decode("0000c8c2").unwrap(),
            &hex::decode("0000c842").unwrap(),
            &hex::decode("0000f0c1").unwrap(),
            &hex::decode("0000f041").unwrap(),
            &[0],
            &hex::decode("000000000000000000000000000000000000803f00000000").unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&v).unwrap(),
            json!({
                "zone.area.pan": {"max": 100, "min": -100},
                "zone.area.tilt": {"max": 30, "min": -30},
                "zone.follow": false,
                "zone.return": {"pan": 0, "tilt": 0, "zoom": 1},
            })
        );
        // asymmetric: pitch -10..30 (positive down) is tilt -30..10
        let v = zone(
            &(-45f32).to_le_bytes(),
            &45f32.to_le_bytes(),
            &(-10f32).to_le_bytes(),
            &30f32.to_le_bytes(),
            &[1],
            &hex::decode("000000000000a0410000a0c0000000000000803f00000000").unwrap(),
        )
        .unwrap();
        assert_eq!(v["zone.area.tilt"], json!({"min": -30, "max": 10}));
        assert_eq!(v["zone.return"], json!({"pan": 20, "tilt": 5, "zoom": 1}));
        assert_eq!(v["zone.follow"], json!(true));
    }

    #[test]
    fn the_recorded_hand_answers_decode() {
        // capture 11: area yaw -45..45, pitch -30..30, both axes on; no return pose (lead -1)
        let area = hex::decode("000034c2000034420000f0c10000f041ffffffff00050101").unwrap();
        let none = hex::decode("ffffffff0000000000000000000000000000803f00000000").unwrap();
        let v = hand(&area, &none).unwrap();
        assert_eq!(
            serde_json::to_value(&v).unwrap(),
            json!({
                "hand.area.pan": {"max": 45, "min": -45},
                "hand.area.tilt": {"max": 30, "min": -30},
                "hand.axes.pan": true,
                "hand.axes.tilt": true,
                "hand.return": null,
            })
        );
        // asymmetric pitch -10..30 is tilt -30..10; a set return pose
        let mut area2 = area.clone();
        area2[8..12].copy_from_slice(&(-10f32).to_le_bytes());
        area2[22] = 0;
        let set = hex::decode("000000000000a0410000a0c0000000000000803f00000000").unwrap();
        let v = hand(&area2, &set).unwrap();
        assert_eq!(v["hand.area.tilt"], json!({"min": -30, "max": 10}));
        assert_eq!(v["hand.axes.pan"], json!(false));
        assert_eq!(v["hand.return"], json!({"pan": 20, "tilt": 5, "zoom": 1}));
        assert!(
            hand(&area[..23], &none).is_none(),
            "the flags need 24 bytes"
        );
        assert_eq!(return_after(&2.5f32.to_le_bytes()), Some(json!(2.5)));
        assert_eq!(return_after(&[0, 0]), None);
    }

    #[test]
    fn the_recorded_initial_state_decodes() {
        // capture 08: source 3 (own pose) pan -0.9, pitch -17.1 (tilt up 17.1), zoom 1.5; the
        // block with start mode normal and nothing else set
        let v = power_on(
            &hex::decode("03000000676666bfcdcc88c1000000000000c03f00000000").unwrap(),
            &hex::decode(
                "00ffffffffffffff80ffffffffffffff00000000ffffffff00000000000000000000000000000000",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&v).unwrap(),
            json!({
                "power_on.settings.auto_exposure": null,
                "power_on.settings.autofocus": null,
                "power_on.settings.contrast": null,
                "power_on.settings.exposure_compensation": 128,
                "power_on.settings.hue": null,
                "power_on.settings.saturation": null,
                "power_on.settings.sharpness": null,
                "power_on.settings.tracking_speed": null,
                "power_on.settings.white_balance_mode": null,
                "power_on.settings.white_balance_temperature": null,
                "power_on.start.mode": "normal",
                "power_on.start.pose": {"pan": -0.9, "tilt": 17.1, "zoom": 1.5},
                "power_on.start.source": "own",
            })
        );
    }

    #[test]
    fn start_modes_and_sources_by_byte() {
        let block = |b0: u8| {
            let mut b = vec![0xffu8; 40];
            b[0] = b0;
            b
        };
        let src = |lead: u32| {
            let mut d = lead.to_le_bytes().to_vec();
            for x in [0f32, 0.0, 0.0, 1.0, 0.0] {
                d.extend(x.to_le_bytes());
            }
            d
        };
        for (byte, name) in [
            (0xffu8, "static"),
            (0xfe, "unset"),
            (7, "hand"),
            (8, "hand"),
            (9, "group"),
            (2, "close_up"),
        ] {
            assert_eq!(
                power_on(&src(3), &block(byte)).unwrap()["power_on.start.mode"],
                json!(name)
            );
        }
        assert_eq!(
            power_on(&src(3), &block(5)).unwrap()["power_on.start.mode"],
            json!("raw:5")
        );
        assert_eq!(
            power_on(&src(1), &block(0)).unwrap()["power_on.start.source"],
            json!("preset_1")
        );
        assert_eq!(
            power_on(&src(7), &block(0)).unwrap()["power_on.start.source"],
            json!("raw:7")
        );
        // white balance temperature only with mode 5 (capture 08b layout: d8 0e = 3800 K)
        let mut b = block(0);
        b[13] = 5;
        b[14..16].copy_from_slice(&[0xd8, 0x0e]);
        let v = power_on(&src(3), &b).unwrap();
        assert_eq!(
            v["power_on.settings.white_balance_temperature"],
            json!(3800)
        );
        assert_eq!(v["power_on.settings.white_balance_mode"], json!(5));
        assert!(power_on(&src(3), &block(0)[..23]).is_none());
    }

    #[test]
    fn a_recorded_view_decodes() {
        // capture 11: view 0 "Bereich1" at pan 0.2, tilt 0, zoom 1.48
        let v = view(
            0,
            &hex::decode("4265726569636831").unwrap(),
            &hex::decode("00000000cdcc4c3e0000000000000000a370bd3f00000000").unwrap(),
        )
        .unwrap();
        assert_eq!(
            v,
            json!({"id": 0, "name": "Bereich1", "pose": {"pan": 0.2, "tilt": 0, "zoom": 1.48}})
        );
    }

    fn entry(slot: u8) -> [u8; 60] {
        // read from a Tiny 2 Lite after writing pan 12.5, pitch -3.25, zoom 1.5, "t4ltest"
        let mut block = [0u8; 60];
        let read = hex::decode("00000000bbfee2049600644452736447567a64413d3d").unwrap();
        block[..read.len()].copy_from_slice(&read);
        block[1] = slot;
        block
    }

    #[test]
    fn the_preset_list_reads_only_whole() {
        let v = presets(&[2, 2, 0], &[entry(2), entry(0)]).unwrap();
        assert_eq!(
            v["presets"],
            json!([
                {"slot": 0, "name": "t4ltest", "pose": {"pan": 12.5, "tilt": 3.3, "zoom": 1.5}},
                {"slot": 2, "name": "t4ltest", "pose": {"pan": 12.5, "tilt": 3.3, "zoom": 1.5}},
            ])
        );
        assert_eq!(presets(&[0], &[]).unwrap()["presets"], json!([]));
        assert!(
            presets(&[2, 0, 2], &[entry(0)]).is_none(),
            "one entry short"
        );
        let mut end = entry(2);
        end[0] = 2;
        assert!(
            presets(&[2, 0, 2], &[entry(0), end]).is_none(),
            "an entry that does not decode"
        );
        assert!(
            presets(&[2, 0, 2], &[entry(0), entry(1)]).is_none(),
            "a slot the list does not name"
        );
    }

    #[test]
    fn on_air_is_one_sample() {
        let v = on_air(&OnAir {
            active: true,
            apps: vec!["firefox".into()],
        });
        assert_eq!(v["on_air.active"], json!(true));
        assert_eq!(v["on_air.apps"], json!([{"name": "firefox"}]));
    }

    #[test]
    fn identity_answers() {
        // capture 01: 0x0408 answers 04 07 02 06, shown as 6.2.7.4
        assert_eq!(
            firmware(&hex::decode("0407020600000000").unwrap()).as_deref(),
            Some("6.2.7.4")
        );
        assert_eq!(firmware(&[1, 2]), None);
        // synthetic serial (identity answers are never fixtures)
        assert_eq!(
            serial(b"EXAMPLE0000001\0\0").as_deref(),
            Some("EXAMPLE0000001")
        );
        assert_eq!(serial(b"\0\0"), None);
        assert_eq!(serial(&[0xff, 0xfe]), None);
    }

    #[test]
    fn tilt_ranges_swap_and_flip_both_ways() {
        assert_eq!(tilt_to_pitch_range((-30.0, 10.0)), (-10.0, 30.0));
        assert_eq!(pitch_to_tilt_range((-10.0, 30.0)), (-30.0, 10.0));
        assert_eq!(
            tilt_to_pitch_range((0.0, 30.0)).1.to_bits(),
            0,
            "no -0 on the wire"
        );
        assert_eq!(pitch_to_tilt_range((-30.0, 0.0)).0.to_bits(), 0);
    }

    #[test]
    fn tables_name_each_byte_once_except_the_two_hand_bytes() {
        for table in [
            &PROFILE[..],
            &SPEED,
            &FRAMING_SIZE,
            &ZOOM_SPEED,
            &PART,
            &FIELD_OF_VIEW,
            &NOISE_REDUCTION,
            &ANTI_FLICKER,
            &REGION,
            &SLEEP_BACKGROUND,
            &HAND_SIDE,
            &FOCUS_MODE,
            &START_MODE,
        ] {
            for (name, byte) in table {
                assert_eq!(named(table, *byte), json!(name));
            }
        }
        assert_eq!(byte_of(&START_MODE, "hand"), Some(7));
        assert_eq!(mode_pair("close_up"), Some([2, 2]));
        assert_eq!(mode_pair("off"), None);
    }
}
