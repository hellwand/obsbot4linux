// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! Writing one leaf of the Tiny 2 Lite (design spec 4.3, 8.8; field ledger, column "Write").
//! Each leaf sends the frames of its ledger row and nothing else (spec 2): no automatic
//! control is switched off first, nothing is re-sent. The value has passed the validator;
//! the byte builders of `commands` and `presets` (pinned by the byte guard) make the frames.

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx};
use crate::family::link::Link;
use crate::models::tiny2lite::commands::{
    self as cmd, AiMode, AutoZoomSpeed, AutofocusMode, ExposureMode, FieldOfView, FramingSize,
    Gesture, HandSide, NoiseReduction, PartMode, SleepBackground, SpeedMode, TrackingSpeed,
};
use crate::models::tiny2lite::decode::{self, byte_of};
use crate::models::tiny2lite::model::Memory;
use crate::models::tiny2lite::plan::{FRAMED, SHORT, TRACKING};
use crate::models::tiny2lite::presets;
use serde_json::Value;

/// A builder's refusal of a value the validator let through: description and builder
/// disagree. Nothing was sent.
fn built<T>(r: crate::error::Result<T>) -> Result<T> {
    r.map_err(|e| CameraError::Io(format!("not sent: {e}")))
}

fn wrong(leaf: &str, value: &Value) -> CameraError {
    CameraError::Io(format!("not sent: {value} is no value of {leaf}"))
}

fn boolean(leaf: &str, v: &Value) -> Result<bool> {
    v.as_bool().ok_or_else(|| wrong(leaf, v))
}

fn float(leaf: &str, v: &Value) -> Result<f64> {
    v.as_f64().ok_or_else(|| wrong(leaf, v))
}

fn int(leaf: &str, v: &Value) -> Result<i32> {
    let x = float(leaf, v)?;
    if x.fract() == 0.0 && x >= f64::from(i32::MIN) && x <= f64::from(i32::MAX) {
        Ok(x as i32)
    } else {
        Err(wrong(leaf, v))
    }
}

/// The byte of an enum value in a table of `decode`.
fn byte(table: &[(&str, u8)], leaf: &str, v: &Value) -> Result<u8> {
    v.as_str()
        .and_then(|name| byte_of(table, name))
        .ok_or_else(|| wrong(leaf, v))
}

/// `{min, max}` of a range.
fn range(leaf: &str, v: &Value) -> Result<(f32, f32)> {
    let min = v.get("min").and_then(Value::as_f64);
    let max = v.get("max").and_then(Value::as_f64);
    match (min, max) {
        (Some(min), Some(max)) => Ok((min as f32, max as f32)),
        _ => Err(wrong(leaf, v)),
    }
}

fn mode(leaf: &str, v: &Value) -> Result<AiMode> {
    v.as_str()
        .and_then(decode::mode_pair)
        .and_then(|[m, n]| AiMode::from_status(m, n))
        .ok_or_else(|| wrong(leaf, v))
}

/// The UVC control of a leaf of the group `uvc`.
fn uvc_id(leaf: &str) -> Option<u32> {
    decode::UVC
        .iter()
        .find(|(l, _)| *l == leaf)
        .map(|(_, id)| *id)
}

/// The auto-sleep word as the camera has it: from the model's last status read or write, else
/// from a status read now (a read, not a command).
fn sleep_word(link: &mut Link, memory: &mut Memory, ctx: &Ctx) -> Result<i16> {
    if let Some(w) = memory.sleep_word {
        return Ok(w);
    }
    let block = link.read(SHORT, ctx)?;
    let w = i16::from_le_bytes([block[0x0a], block[0x0b]]);
    memory.sleep_word = Some(w);
    Ok(w)
}

/// Writes `leaf` = `value`: the frames of its ledger row, in that order.
pub fn apply(
    link: &mut Link,
    memory: &mut Memory,
    leaf: &str,
    value: &Value,
    ctx: &Ctx,
) -> Result<()> {
    let v = value;
    match leaf {
        "gimbal.invert" => {
            let f = cmd::invert(boolean(leaf, v)?, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "zoom.factor" => {
            // UVC zoom absolute, 0–100 for 1.0–4.0 (field ledger)
            let percent = ((float(leaf, v)? - 1.0) * 100.0 / 3.0).round() as i32;
            link.ctrl_set(decode::ZOOM_ABSOLUTE, percent, ctx)
        }
        "zoom.field_of_view" => {
            let fov = FieldOfView::from_u8(byte(&decode::FIELD_OF_VIEW, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            link.short(SHORT, &cmd::field_of_view(fov), ctx)
        }
        "tracking.mode" => link.short(SHORT, &cmd::ai_mode(mode(leaf, v)?), ctx),
        "tracking.profile" => {
            let p = TrackingSpeed::from_u8(byte(&decode::PROFILE, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            let f = cmd::tracking_speed(p, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "tracking.speed" => {
            let s =
                SpeedMode::from_u8(byte(&decode::SPEED, leaf, v)?).ok_or_else(|| wrong(leaf, v))?;
            let f = cmd::speed_mode(s, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "framing.auto_zoom" => {
            let f = cmd::auto_zoom(boolean(leaf, v)?, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "framing.size" => {
            let s = FramingSize::from_u8(byte(&decode::FRAMING_SIZE, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            let f = cmd::framing_size(s, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "framing.zoom_speed" => {
            let s = AutoZoomSpeed::from_u8(byte(&decode::ZOOM_SPEED, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            let f = cmd::auto_zoom_speed(s, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "framing.offset.x" | "framing.offset.y" => {
            let f = built(cmd::framing_offset(
                leaf == "framing.offset.y",
                float(leaf, v)? as f32,
                link.seq(),
            ))?;
            // back to back the camera applies neither offset (0.13.2)
            link.send_spaced(FRAMED, &f, ctx)
        }
        "framing.part" => {
            let p =
                PartMode::from_u8(byte(&decode::PART, leaf, v)?).ok_or_else(|| wrong(leaf, v))?;
            let f = cmd::part_mode(p, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "gestures.target_lock"
        | "gestures.zoom"
        | "gestures.dynamic_zoom"
        | "gestures.direction_change" => {
            let g = match leaf {
                "gestures.target_lock" => Gesture::TargetLock,
                "gestures.zoom" => Gesture::Zoom,
                "gestures.dynamic_zoom" => Gesture::DynamicZoom,
                _ => Gesture::DirectionChange,
            };
            let f = cmd::gesture(g, boolean(leaf, v)?, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "gestures.zoom_ratio" => {
            let f = built(cmd::gesture_zoom_ratio(float(leaf, v)? as f32, link.seq()))?;
            link.send(FRAMED, &f, ctx)
        }
        "zone.enabled" => {
            let f = cmd::zone_tracking(boolean(leaf, v)?, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "zone.follow" => {
            let f = cmd::zone_follow(boolean(leaf, v)?, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "zone.area.pan" | "zone.area.tilt" | "hand.area.pan" | "hand.area.tilt" => {
            let (lo, hi) = range(leaf, v)?;
            let (ids, wire, limits) = match leaf {
                "zone.area.pan" => (cmd::ZONE_PAN, (lo, hi), cmd::YAW_LIMITS),
                "zone.area.tilt" => (
                    cmd::ZONE_PITCH,
                    decode::tilt_to_pitch_range((lo, hi)),
                    cmd::PITCH_LIMITS,
                ),
                "hand.area.pan" => (cmd::HAND_PAN, (lo, hi), cmd::YAW_LIMITS),
                _ => (
                    cmd::HAND_PITCH,
                    decode::tilt_to_pitch_range((lo, hi)),
                    cmd::PITCH_LIMITS,
                ),
            };
            let [first, second] =
                built(cmd::area_axis(ids, wire, limits, [link.seq(), link.seq()]))?;
            link.send(FRAMED, &first, ctx)?;
            link.send(FRAMED, &second, ctx)
        }
        "hand.side" => {
            let s = HandSide::from_u8(byte(&decode::HAND_SIDE, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            let f = cmd::hand_side(s, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "hand.axes.pan" => {
            let f = cmd::hand_pan(boolean(leaf, v)?, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "hand.axes.tilt" => {
            let f = cmd::hand_tilt(boolean(leaf, v)?, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "hand.return_after" => {
            let f = built(cmd::hand_return_after(float(leaf, v)? as f32, link.seq()))?;
            link.send(FRAMED, &f, ctx)
        }
        "hand.move_gimbal" => {
            let f = cmd::hand_gimbal(boolean(leaf, v)?, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "exposure.mode" => {
            // UVC auto-exposure: 1 manual; automatic is written as 0, as 0.13.2 does (#97)
            let raw = match v.as_str() {
                Some("manual") => 1,
                Some("auto") => 0,
                _ => return Err(wrong(leaf, v)),
            };
            link.ctrl_set(0x009a_0901, raw, ctx)
        }
        "exposure.region" => {
            let r = ExposureMode::from_u8(byte(&decode::REGION, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            link.short(SHORT, &cmd::exposure_mode_select(r), ctx)
        }
        "exposure.time" => {
            // 100 µs units; the camera takes the step at or below (#97)
            let units = (float(leaf, v)? / 100.0).floor() as i32;
            link.ctrl_set(0x009a_0902, units, ctx)
        }
        "exposure.hdr" => link.short(SHORT, &cmd::hdr(boolean(leaf, v)?), ctx),
        "exposure.anti_flicker" => {
            let b = byte(&decode::ANTI_FLICKER, leaf, v)?;
            link.ctrl_set(decode::POWER_LINE, i32::from(b), ctx)
        }
        "image.white_balance.auto" => link.ctrl_set(0x0098_090c, i32::from(boolean(leaf, v)?), ctx),
        "exposure.gain"
        | "exposure.compensation"
        | "image.brightness"
        | "image.contrast"
        | "image.saturation"
        | "image.hue"
        | "image.sharpness"
        | "image.white_balance.temperature" => {
            let id = uvc_id(leaf).ok_or_else(|| wrong(leaf, v))?;
            link.ctrl_set(id, int(leaf, v)?, ctx)
        }
        "image.mirror" => link.short(SHORT, &cmd::mirror(boolean(leaf, v)?), ctx),
        "focus.mode" => {
            let m = AutofocusMode::from_u8(byte(&decode::FOCUS_MODE, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            let f = cmd::autofocus_mode_select(m, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "focus.region" => {
            let face = byte(&decode::REGION, leaf, v)? == 1;
            let f = cmd::autofocus_mode(face, link.seq());
            link.send(FRAMED, &f, ctx)
        }
        "focus.position" => link.ctrl_set(decode::FOCUS_ABSOLUTE, int(leaf, v)?, ctx),
        "audio.enabled" => link.short(SHORT, &cmd::microphone(boolean(leaf, v)?), ctx),
        "audio.volume" => {
            let volume = u8::try_from(int(leaf, v)?).map_err(|_| wrong(leaf, v))?;
            let f = built(cmd::audio_volume(volume, link.seq()))?;
            link.send(FRAMED, &f, ctx)
        }
        "audio.noise_reduction" => {
            let n = NoiseReduction::from_u8(byte(&decode::NOISE_REDUCTION, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            link.short(SHORT, &cmd::noise_reduction(n), ctx)
        }
        "audio.auto_gain" => link.short(SHORT, &cmd::auto_gain(boolean(leaf, v)?), ctx),
        "audio.in_sleep" => link.short(SHORT, &cmd::mic_in_sleep(boolean(leaf, v)?), ctx),
        "led.brightness" => {
            let level = u8::try_from(int(leaf, v)?).map_err(|_| wrong(leaf, v))?;
            link.short(SHORT, &built(cmd::led(level))?, ctx)
        }
        "led.blink" => link.short(SHORT, &cmd::led_blink(boolean(leaf, v)?), ctx),
        "sleep.auto.enabled" | "sleep.auto.seconds" => {
            let word = sleep_word(link, memory, ctx)?;
            let (on, seconds) = match leaf {
                "sleep.auto.enabled" => (boolean(leaf, v)?, i32::from(word.unsigned_abs())),
                _ => (word >= 0, int(leaf, v)?),
            };
            // the camera cannot store "off" together with 0 s, and "on with 0 s" was never
            // recorded (4.3, PX-7): with 0 s stored, switching it on or off alone sends nothing
            let seconds = i16::try_from(seconds)
                .ok()
                .filter(|s| *s > 0)
                .ok_or_else(|| {
                    CameraError::Io(format!(
                        "not sent: auto-sleep has {seconds} s stored; set sleep.auto.seconds with it"
                    ))
                })?;
            let word = if on { seconds } else { -seconds };
            link.short(SHORT, &cmd::auto_sleep(word), ctx)?;
            memory.sleep_word = Some(word);
            Ok(())
        }
        "sleep.background" => {
            let b = SleepBackground::from_u8(byte(&decode::SLEEP_BACKGROUND, leaf, v)?)
                .ok_or_else(|| wrong(leaf, v))?;
            link.short(SHORT, &cmd::sleep_background(b), ctx)
        }
        "sleep.background_mirror" => {
            link.short(SHORT, &cmd::sleep_background_mirror(boolean(leaf, v)?), ctx)
        }
        "sleep.wake_zoom" => {
            let percent = built(cmd::wake_zoom_percent(float(leaf, v)? as f32))?;
            link.short(SHORT, &built(cmd::wake_zoom(percent))?, ctx)
        }
        "power_on.mode" => link.short(SHORT, &cmd::default_work_mode(mode(leaf, v)?), ctx),
        "power_on.start.mode" => {
            // the block read by 0x3e84, written back by 0x3e44 with byte 0 changed (capture 08)
            let b = byte(&decode::START_MODE, leaf, v)?;
            let block = link.query(FRAMED, TRACKING, 0x3e84, ctx)?;
            let f = presets::startup_with_ai(&block, b, link.seq()).ok_or_else(|| {
                CameraError::NoAnswer("query 0x3e84: the initial-state block is short".into())
            })?;
            link.send(FRAMED, &f, ctx)
        }
        _ => Err(CameraError::Io(format!("not sent: {leaf} is not writable"))),
    }
}
