// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! The actions of the Tiny 2 Lite (design spec 4.4, 8.8; field ledger, table "Actions") and the
//! model's checks before any frame (`Camera::check`). An action sends the frames of its ledger
//! row and nothing else (spec 2): the gimbal moves do not switch the tracking mode, a preset
//! saved without a name gets none.

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx};
use crate::family::frame::FRAME_SIZE;
use crate::family::link::Link;
use crate::models::tiny2lite::commands as cmd;
use crate::models::tiny2lite::decode;
use crate::models::tiny2lite::plan::{FRAMED, GIMBAL, SHORT, TRACKING};
use crate::models::tiny2lite::presets::{self, MAX_ZOOM, Pose};
use obsbot_proto::validate::Refusal;
use obsbot_proto::{ErrorCode, Params, State};
use serde_json::{Value, json};

/// The speeds of `zoom.move` (field ledger: slow 10, medium 30, fast 50).
pub const ZOOM_MOVE_SPEED: [(&str, i32); 3] = [("slow", 10), ("medium", 30), ("fast", 50)];

/// A tilt in the units of 4.2 (positive up) as the pose commands take it (pitch, positive down);
/// `+ 0.0` sends a tilt of 0 as +0.0. Carried from `camera.rs` (`tilt_to_pitch`).
pub fn tilt_to_pitch(tilt: Option<f32>) -> Option<f32> {
    tilt.map(|t| -t + 0.0)
}

fn refuse(code: ErrorCode, path: &str, message: &str) -> std::result::Result<(), Refusal> {
    Err(Refusal::new(code, path, message))
}

/// The list at `path` (`presets`, `views`) when it was read: `None` while it is in `unread`.
fn list<'a>(state: &'a State, path: &str) -> Option<&'a Vec<Value>> {
    if state.unread.iter().any(|p| p == path) {
        return None;
    }
    state.at(path)?.as_array()
}

fn item<'a>(items: &'a [Value], key: &str, id: &Value) -> Option<&'a Value> {
    items.iter().find(|i| i.get(key) == Some(id))
}

/// The model's checks of 4.4 that the description cannot express, before any frame.
pub fn check(action: &str, params: &Params, state: &State) -> std::result::Result<(), Refusal> {
    let has = |p: &str| params.contains_key(p);
    let name_given_empty = params.get("name").and_then(Value::as_str) == Some("");
    match action {
        "gimbal.move" if !has("pan") && !has("tilt") => {
            return refuse(ErrorCode::Invalid, action, "give pan, tilt or both");
        }
        "gimbal.turn" if !has("pan_speed") && !has("tilt_speed") => {
            return refuse(
                ErrorCode::Invalid,
                action,
                "give pan_speed, tilt_speed or both",
            );
        }
        "gimbal.turn" => {
            // the yaw of a turn is encoded from the mirror and invert flags (4.4)
            for flag in ["image.mirror", "gimbal.invert"] {
                if state.at(flag).and_then(Value::as_bool).is_none() {
                    return refuse(
                        ErrorCode::Conflict,
                        flag,
                        "the direction of a turn needs this flag, which is not read yet",
                    );
                }
            }
        }
        _ => {}
    }
    if name_given_empty
        && matches!(
            action,
            "preset.save" | "preset.rename" | "view.save" | "view.rename"
        )
    {
        return refuse(
            ErrorCode::OutOfRange,
            "name",
            "a name has 1 to 32 printable ASCII characters",
        );
    }
    match action {
        "preset.save"
        | "preset.recall"
        | "preset.rename"
        | "preset.delete"
        | "power_on.use_preset" => {
            let Some(presets) = list(state, "presets") else {
                return refuse(
                    ErrorCode::Conflict,
                    "presets",
                    "the presets are not read yet",
                );
            };
            let slot = params.get("slot").cloned().unwrap_or(Value::Null);
            if action != "preset.save" && item(presets, "slot", &slot).is_none() {
                return refuse(
                    ErrorCode::Conflict,
                    "slot",
                    &format!("preset slot {slot} is empty"),
                );
            }
        }
        "view.save" | "view.go" | "view.rename" | "view.delete" => {
            let Some(views) = list(state, "views") else {
                return refuse(ErrorCode::Conflict, "views", "the views are not read yet");
            };
            let id = params.get("id").cloned().unwrap_or(Value::Null);
            if action != "view.save" && item(views, "id", &id).is_none() {
                return refuse(ErrorCode::Conflict, "id", &format!("no view with id {id}"));
            }
        }
        _ => {}
    }
    Ok(())
}

fn wrong(action: &str, param: &str) -> CameraError {
    CameraError::Io(format!("not sent: {action} has no valid {param}"))
}

fn built<T>(r: crate::error::Result<T>) -> Result<T> {
    r.map_err(|e| CameraError::Io(format!("not sent: {e}")))
}

fn f32_param(params: &Params, name: &str) -> Option<f32> {
    params.get(name).and_then(Value::as_f64).map(|x| x as f32)
}

fn u8_param(action: &str, params: &Params, name: &str) -> Result<u8> {
    params
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|x| u8::try_from(x).ok())
        .ok_or_else(|| wrong(action, name))
}

fn u32_param(action: &str, params: &Params, name: &str) -> Result<u32> {
    params
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|x| u32::try_from(x).ok())
        .ok_or_else(|| wrong(action, name))
}

fn str_param<'a>(action: &str, params: &'a Params, name: &str) -> Result<&'a str> {
    params
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| wrong(action, name))
}

/// A pose object `{pan, tilt, zoom}` of a parameter or a list item.
fn pose_of(v: &Value) -> Option<Pose> {
    let f = |k: &str| v.get(k).and_then(Value::as_f64).map(|x| x as f32);
    Some(Pose {
        pan: f("pan")?,
        tilt: f("tilt")?,
        zoom: f("zoom")?,
    })
}

/// The live pose (query `0x0043` on receiver 3) and the zoom factor (query `0x6804`), as the
/// pose-taking actions use it (4.4). Angles to a tenth of a degree, the zoom kept within
/// 1.0–4.0. Carried from `camera.rs` (`live_pose_and_zoom`).
pub fn live_pose(link: &mut Link, ctx: &Ctx) -> Result<Pose> {
    let d = link.query(FRAMED, GIMBAL, 0x0043, ctx)?;
    let i16_at = |i: usize| d.get(i..i + 2).map(|b| i16::from_le_bytes([b[0], b[1]]));
    let tenths = |v: i16| (f64::from(v) / 10.0).round() / 10.0;
    let (Some(pitch), Some(yaw)) = (i16_at(8), i16_at(10)) else {
        return Err(CameraError::NoAnswer("query 0x0043: a short answer".into()));
    };
    let zoom = decode::zoom_factor(&link.query(FRAMED, TRACKING, 0x6804, ctx)?)
        .ok_or_else(|| CameraError::NoAnswer("query 0x6804: no zoom factor".into()))?;
    Ok(Pose {
        pan: tenths(yaw) as f32,
        tilt: (-tenths(pitch)) as f32 + 0.0,
        zoom: (zoom as f32).clamp(1.0, MAX_ZOOM),
    })
}

/// The pose of a `pose` parameter, else the live pose.
fn pose_or_live(link: &mut Link, params: &Params, ctx: &Ctx) -> Result<Pose> {
    match params.get("pose") {
        Some(p) => pose_of(p).ok_or_else(|| wrong("a pose-taking action", "pose")),
        None => live_pose(link, ctx),
    }
}

/// The start pose of the initial state: `0x3ec4` with the pose (lead `source`: 3 own, 0–2 a
/// preset slot), then 100 ms later `0x3e44` with the block read by `0x3e84`, unchanged, so the
/// start mode and the settings stay (4.4; captures 08, 10). The block is read and both frames
/// are built before the first is sent.
fn start_pose(link: &mut Link, source: u32, pose: Pose, ctx: &Ctx) -> Result<()> {
    let block = link.query(FRAMED, TRACKING, 0x3e84, ctx)?;
    let first: [u8; FRAME_SIZE] = built(presets::startup_pose(source, pose, link.seq()))?;
    let unchanged = *block.first().ok_or_else(|| {
        CameraError::NoAnswer("query 0x3e84: an empty initial-state block".into())
    })?;
    let second = presets::startup_with_ai(&block, unchanged, link.seq()).ok_or_else(|| {
        CameraError::NoAnswer("query 0x3e84: the initial-state block is short".into())
    })?;
    link.pair(FRAMED, &first, &second, ctx)
}

/// Runs `action`; its parameters passed the validator and [`check`].
pub fn act(
    link: &mut Link,
    action: &str,
    params: &Params,
    state: &State,
    ctx: &Ctx,
) -> Result<Option<Params>> {
    let send = |link: &mut Link, f: [u8; FRAME_SIZE]| link.send(FRAMED, &f, ctx);
    match action {
        "gimbal.move" => {
            // only the move (spec 2, 15.1 row 88): no switch of the tracking mode before it
            let pan = f32_param(params, "pan");
            let pitch = tilt_to_pitch(f32_param(params, "tilt"));
            let seq = link.seq();
            let f = match f32_param(params, "speed") {
                Some(speed) => built(cmd::gimbal_pose_speed(pan, pitch, speed, seq))?,
                None => built(cmd::gimbal_pose(pan, pitch, seq))?,
            };
            send(link, f)?;
        }
        "gimbal.stop" => {
            let f = cmd::gimbal_stop(link.seq());
            send(link, f)?;
        }
        "gimbal.turn" => {
            // only the turn (spec 2, 15.1 row 88); the yaw sign follows mirror xor invert (4.4)
            let flag = |p: &str| state.at(p).and_then(Value::as_bool);
            let (Some(mirror), Some(invert)) = (flag("image.mirror"), flag("gimbal.invert")) else {
                return Err(wrong(action, "mirror and invert state"));
            };
            let pan = f32_param(params, "pan_speed").unwrap_or(0.0);
            let tilt = f32_param(params, "tilt_speed").unwrap_or(0.0);
            let f = built(cmd::gimbal_rate(pan, tilt, mirror, invert, link.seq()))?;
            send(link, f)?;
        }
        "zoom.move" => {
            let speed = str_param(action, params, "speed")?;
            let speed = ZOOM_MOVE_SPEED
                .iter()
                .find(|(n, _)| *n == speed)
                .map(|(_, s)| *s)
                .ok_or_else(|| wrong(action, "speed"))?;
            let factor = f32_param(params, "factor").ok_or_else(|| wrong(action, "factor"))?;
            let f = built(cmd::zoom_to(speed, factor, link.seq()))?;
            send(link, f)?;
        }
        "zoom.stop" => {
            let f = cmd::zoom_stop(link.seq());
            send(link, f)?;
        }
        "preset.save" => {
            let slot = u8_param(action, params, "slot")?;
            let name = params.get("name").and_then(Value::as_str);
            // the name is checked before the first frame (audit: the pose went out first);
            // the frames take their sequence numbers in the order they are sent
            if let Some(name) = name {
                built(presets::rename(slot, name, 0))?;
            }
            let pose = pose_or_live(link, params, ctx)?;
            let exists =
                list(state, "presets").is_some_and(|p| item(p, "slot", &json!(slot)).is_some());
            let seq = link.seq();
            let save = if exists {
                built(presets::update(slot, pose, seq))?
            } else {
                built(presets::create(slot, pose, seq))?
            };
            // without a name no rename frame follows: obsbot4linux makes none up (15.1 row 90)
            match name {
                Some(name) => {
                    let rename = built(presets::rename(slot, name, link.seq()))?;
                    link.pair(FRAMED, &save, &rename, ctx)?;
                }
                None => send(link, save)?,
            }
        }
        "preset.recall" => {
            let f = built(presets::recall(
                u8_param(action, params, "slot")?,
                link.seq(),
            ))?;
            send(link, f)?;
        }
        "preset.rename" => {
            let slot = u8_param(action, params, "slot")?;
            let f = built(presets::rename(
                slot,
                str_param(action, params, "name")?,
                link.seq(),
            ))?;
            send(link, f)?;
        }
        "preset.delete" => {
            // the delete, then the cleared settings block 100 ms later (capture 10); which
            // preset the second frame addresses is CAM-18
            let slot = u8_param(action, params, "slot")?;
            let (a, b) = (link.seq(), link.seq());
            let [delete, cleared] = built(presets::delete(slot, a, b))?;
            link.pair(FRAMED, &delete, &cleared, ctx)?;
        }
        "view.save" => {
            let name = str_param(action, params, "name")?;
            let taken: Vec<u32> = list(state, "views")
                .map(|views| {
                    views
                        .iter()
                        .filter_map(|v| v.get("id").and_then(Value::as_u64))
                        .filter_map(|id| u32::try_from(id).ok())
                        .collect()
                })
                .unwrap_or_default();
            let id = presets::free_view_id(&taken);
            // the name is checked before anything is sent
            built(presets::view_rename(id, name, 0))?;
            let pose = pose_or_live(link, params, ctx)?;
            let add = built(presets::view_add(id, pose, link.seq()))?;
            let rename = built(presets::view_rename(id, name, link.seq()))?;
            link.pair(FRAMED, &add, &rename, ctx)?;
            let mut result = Params::new();
            result.insert("id".into(), json!(id));
            return Ok(Some(result));
        }
        "view.go" => {
            let f = presets::view_go(u32_param(action, params, "id")?, link.seq());
            send(link, f)?;
        }
        "view.rename" => {
            let id = u32_param(action, params, "id")?;
            let f = built(presets::view_rename(
                id,
                str_param(action, params, "name")?,
                link.seq(),
            ))?;
            send(link, f)?;
        }
        "view.delete" => {
            let f = presets::view_remove(u32_param(action, params, "id")?, link.seq());
            send(link, f)?;
        }
        "zone.return_here" => {
            let pose = live_pose(link, ctx)?;
            let f = built(presets::zone_return_set(pose, link.seq()))?;
            send(link, f)?;
        }
        "zone.go_return" => {
            let f = cmd::zone_return_go(link.seq());
            send(link, f)?;
        }
        "hand.return_here" => {
            let pose = live_pose(link, ctx)?;
            let f = built(presets::hand_return_set(pose, link.seq()))?;
            send(link, f)?;
        }
        "hand.go_return" => {
            let f = cmd::hand_return_go(link.seq());
            send(link, f)?;
        }
        "hand.return_clear" => {
            let f = cmd::hand_return_reset(link.seq());
            send(link, f)?;
        }
        "hand.area_reset" => {
            // four frames, one per bound, no data (U-7)
            let seqs = [link.seq(), link.seq(), link.seq(), link.seq()];
            for f in cmd::hand_area_reset(seqs) {
                send(link, f)?;
            }
        }
        "focus.point" => {
            let x = f32_param(params, "x").ok_or_else(|| wrong(action, "x"))?;
            let y = f32_param(params, "y").ok_or_else(|| wrong(action, "y"))?;
            let f = built(cmd::focus_point(x, y, link.seq()))?;
            send(link, f)?;
        }
        "power_on.set_pose" => {
            let pose = params
                .get("pose")
                .and_then(pose_of)
                .ok_or_else(|| wrong(action, "pose"))?;
            start_pose(link, 3, pose, ctx)?;
        }
        "power_on.use_current" => {
            let pose = live_pose(link, ctx)?;
            start_pose(link, 3, pose, ctx)?;
        }
        "power_on.use_preset" => {
            let slot = u8_param(action, params, "slot")?;
            let pose = list(state, "presets")
                .and_then(|p| item(p, "slot", &json!(slot)))
                .and_then(|p| p.get("pose"))
                .and_then(pose_of)
                .ok_or_else(|| wrong(action, "slot"))?;
            start_pose(link, u32::from(slot), pose, ctx)?;
        }
        "power_on.go" => link.short(SHORT, &cmd::startup_go(), ctx)?,
        "sleep" | "wake" => {
            let f = cmd::sleep(action == "sleep", link.seq());
            send(link, f)?;
        }
        "factory_reset" => {
            let f = cmd::factory_reset(link.seq());
            send(link, f)?;
        }
        other => return Err(CameraError::Io(format!("not sent: no action {other}"))),
    }
    Ok(None)
}
