// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! Reading one group of the Tiny 2 Lite's poll plan (design spec 5.7, 8.8): the frames of the
//! group's queries and reads, decoded by `decode`. A group read stops at its first failed
//! query; the daemon then lists the group's leaves in `unread`.

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx};
use crate::family::link::Link;
use crate::models::tiny2lite::decode;
use crate::models::tiny2lite::model::Memory;
use crate::models::tiny2lite::plan::{
    FRAMED, FRAMING_QUERIES, GIMBAL, POWER, PRESET_ENTRY, PRESET_LIST, SHORT, TRACKING,
};
use crate::models::tiny2lite::presets;
use obsbot_proto::Values;
use serde_json::{Value, json};

fn short_answer(command: u16) -> CameraError {
    CameraError::NoAnswer(format!(
        "query {command:#06x}: an answer that does not decode"
    ))
}

fn ask(link: &mut Link, receiver: u8, command: u16, ctx: &Ctx) -> Result<Vec<u8>> {
    link.query(FRAMED, receiver, command, ctx)
}

/// Reads `group`. `memory` takes the tracking mode and the auto-sleep word of a status read.
pub fn read(link: &mut Link, memory: &mut Memory, group: &str, ctx: &Ctx) -> Result<Values> {
    match group {
        "status" => {
            let block = link.read(SHORT, ctx)?;
            let values = decode::status(&block);
            memory.mode = values.get("tracking.mode").cloned();
            memory.sleep_word = Some(i16::from_le_bytes([block[0x0a], block[0x0b]]));
            Ok(values)
        }
        "on_air" => {
            ctx.check()?;
            Ok(decode::on_air(&link.transport().on_air()?))
        }
        "target" => {
            // the target is asked only while the tracking mode is known and not static (8.8);
            // else the group keeps the values of its last read without a frame, and before its
            // first read it is not read at all, so its leaves stay in `unread` (trait
            // `Camera::read`: `detected` is not nullable, and `false` is not what the camera said)
            let tracking = matches!(&memory.mode, Some(m) if m != &json!("static"));
            if !tracking {
                ctx.check()?;
                return memory.target.clone().ok_or_else(|| {
                    CameraError::NoAnswer(
                        "the target, which is asked only while the tracking mode is known \
                         and not static (8.8)"
                            .into(),
                    )
                });
            }
            let values = decode::target(&ask(link, TRACKING, 0x01c4, ctx)?);
            memory.target = Some(values.clone());
            Ok(values)
        }
        "pose" => decode::pose(&ask(link, GIMBAL, 0x0043, ctx)?).ok_or(short_answer(0x0043)),
        "gestures" => {
            decode::gestures(&ask(link, TRACKING, 0x0104, ctx)?).ok_or(short_answer(0x0104))
        }
        "zoom" => decode::zoom(&ask(link, TRACKING, 0x6804, ctx)?).ok_or(short_answer(0x6804)),
        "framing" => {
            let mut values = Values::new();
            for (leaf, command) in FRAMING_QUERIES {
                let d = ask(link, TRACKING, command, ctx)?;
                let v = decode::framing(leaf, &d).ok_or(short_answer(command))?;
                values.insert(leaf.to_string(), v);
            }
            Ok(values)
        }
        "uvc" => {
            let mut values = Values::new();
            for (leaf, id) in decode::UVC {
                let v = link.ctrl_get(id, ctx)?;
                values.insert(leaf.to_string(), decode::uvc(leaf, v));
            }
            Ok(values)
        }
        "focus" => decode::focus(&ask(link, POWER, 0x3642, ctx)?).ok_or(short_answer(0x3642)),
        "audio" => {
            ctx.check()?;
            let enabled = decode::audio_enabled(&link.transport().usb()?);
            let volume =
                decode::volume(&ask(link, POWER, 0x80c2, ctx)?).ok_or(short_answer(0x80c2))?;
            let mut values = Values::new();
            values.insert("audio.enabled".into(), enabled);
            values.insert("audio.volume".into(), volume);
            Ok(values)
        }
        "zone" => {
            let pan_min = ask(link, TRACKING, 0x4c44, ctx)?;
            let pan_max = ask(link, TRACKING, 0x4cc4, ctx)?;
            let pitch_min = ask(link, TRACKING, 0x4d44, ctx)?;
            let pitch_max = ask(link, TRACKING, 0x4dc4, ctx)?;
            let follow = ask(link, TRACKING, 0x4e44, ctx)?;
            let ret = ask(link, TRACKING, 0x4ec4, ctx)?;
            decode::zone(&pan_min, &pan_max, &pitch_min, &pitch_max, &follow, &ret)
                .ok_or(short_answer(0x4ec4))
        }
        "hand" => {
            let area = ask(link, TRACKING, 0x2004, ctx)?;
            let ret = ask(link, TRACKING, 0x2504, ctx)?;
            decode::hand(&area, &ret).ok_or(short_answer(0x2004))
        }
        "hand_timer" => {
            let v = decode::return_after(&ask(link, TRACKING, 0x2484, ctx)?)
                .ok_or(short_answer(0x2484))?;
            Ok(Values::from([("hand.return_after".to_string(), v)]))
        }
        "power_on" => {
            let source = ask(link, TRACKING, 0x3884, ctx)?;
            let block = ask(link, TRACKING, 0x3e84, ctx)?;
            decode::power_on(&source, &block).ok_or(short_answer(0x3e84))
        }
        "views" => {
            let ids = presets::parse_view_ids(&ask(link, TRACKING, 0x1344, ctx)?);
            let mut items = Vec::new();
            for id in ids {
                let request = id.to_le_bytes();
                let name = link.query_with(FRAMED, TRACKING, 0x1504, &request, ctx)?;
                let pose = link.query_with(FRAMED, TRACKING, 0x1444, &request, ctx)?;
                items.push(decode::view(id, &name, &pose).ok_or(short_answer(0x1444))?);
            }
            Ok(Values::from([("views".to_string(), Value::Array(items))]))
        }
        "presets" => {
            let list = link.read(PRESET_LIST, ctx)?;
            let slots = presets::parse_slots(&list);
            let mut entries = Vec::new();
            if !slots.is_empty() {
                // writing the list back moves the entry cursor to the first preset: the one
                // known exception of spec section 2, until CAM-19
                link.write_back(PRESET_LIST, &list, ctx)?;
                for _ in 0..slots.len() {
                    entries.push(link.read(PRESET_ENTRY, ctx)?);
                }
            }
            decode::presets(&list, &entries)
                .ok_or_else(|| CameraError::NoAnswer("the preset list did not read whole".into()))
        }
        other => Err(CameraError::Io(format!(
            "the Tiny 2 Lite has no group {other}"
        ))),
    }
}
