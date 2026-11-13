// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! The status of 0.13 (`status --json` and the daemon's state): the full status, its
//! capability list and limits, and the readbacks the daemon keeps between full reads.

use crate::error::{Error, Result};
use crate::family::exchange::Camera;
use crate::family::streaming;
use crate::legacy::state::{Startup, State};
use crate::models::tiny2lite::camera::{
    Framing, Hand, Identity, ImageControl, LivePose, ZOOM_ABSOLUTE, Zone,
};
use crate::models::tiny2lite::commands::*;
use crate::models::tiny2lite::presets::{MAX_ZOOM, Preset};
use crate::models::tiny2lite::status::{Focus, ModelInfo, Status, Target, Tracking};
use serde::Serialize;
use std::collections::BTreeMap;

/// Commands obsbot4linux supports, for the `capabilities` list of `status --json`.
pub const CAPABILITIES: &[&str] = &[
    "sleep",
    "wake",
    "tracking",
    "speed",
    "hdr",
    "exposure",
    "zoom",
    "gimbal",
    "gimbal_speed",
    "gimbal_stop",
    "zoom_speed",
    "speed_mode",
    "speed_headroom",
    "framing",
    "fov",
    "led",
    "led_blink",
    "mirror",
    "invert",
    "auto_sleep",
    "sleep_background",
    "sleep_background_mirror",
    "audio",
    "audio_volume",
    "gesture",
    "hand",
    "zone",
    "zone_follow",
    "zone_return",
    "zone_views",
    "hand_return",
    "hand_area",
    "hand_axes",
    "hand_return_after",
    "hand_gimbal",
    "focus",
    "focus_af_mode",
    "focus_point",
    "wake_zoom",
    "wake_mode",
    "exposure_values",
    "startup",
    "startup_ai",
    "image",
    "preset",
    "preset_state",
    "factory_reset",
    "target",
    "live_pose_on_demand",
];

#[derive(Debug, Serialize)]
pub struct Zoom {
    /// Current zoom factor from query 0x6804 (follows gestures and preset recalls).
    pub factor: Option<f32>,
    /// UVC zoom_absolute, 0-100 = 1x-4x: the value last written over V4L2. It does not follow
    /// a field of view change, a gesture or a preset recall; `factor` does.
    pub absolute: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct FullStatus {
    /// Human name of the camera model, e.g. "Tiny 2 Lite"; null while no camera is connected.
    pub model: Option<&'static str>,
    /// V4L2 node the camera streams from (stable by-id path when available); null like `model`.
    pub device: Option<String>,
    /// Whether an application is streaming from the camera (kernel probe, see `streaming::is_streaming`).
    pub streaming: bool,
    /// `comm` names of those applications, sorted; empty when none.
    pub streaming_apps: Vec<String>,
    /// Always true here; the daemon reports false when the camera is gone.
    pub available: bool,
    /// Firmware version such as "6.2.8.1" (query 0x0408); null when the camera did not answer.
    pub firmware: Option<String>,
    /// Product and platform codes (query 0x1948); null when the camera did not answer.
    pub model_info: Option<ModelInfo>,
    pub status: Status,
    /// None when the camera did not answer query 0x0104.
    pub tracking: Option<Tracking>,
    pub zoom: Zoom,
    /// Live gimbal pose from query 0x0043 (gimbal controller), also while AI tracking moves it.
    pub pose: Option<LivePose>,
    /// Tracking target from query 0x01c4; None while Static or when the camera gave no usable answer.
    pub target: Option<Target>,
    pub image: BTreeMap<String, ImageControl>,
    /// Microphone volume 0-100 from query 0x80c2; null while asleep or when the camera did not
    /// answer.
    pub audio_volume: Option<u8>,
    /// Autofocus mode, manual position and focus window from query 0x3642; null like
    /// `audio_volume`.
    pub focus: Option<Focus>,
    pub state: StateView,
    /// Zone tracking area, follow and return position read from the camera; None when it did not
    /// answer the queries.
    pub zone: Option<Zone>,
    /// Hand tracking area and return position read from the camera; None when it did not answer.
    pub hand: Option<Hand>,
    /// Tracking speed mode and framing options read from the camera; None when it did not answer.
    pub framing: Option<Framing>,
    /// The presets stored in the camera, sorted by slot; null when the camera did not answer.
    pub presets: Option<Vec<Preset>>,
    pub capabilities: Vec<&'static str>,
    /// What `gimbal`, the poses and `zone area` accept, so clients need not hardcode it.
    pub limits: Limits,
}

/// `limits` in the status: [min, max] in obsbot4linux's angles (pan positive = camera left, tilt
/// positive = up) and the zoom factor.
#[derive(Debug, Serialize, PartialEq)]
pub struct Limits {
    pub pan: [f32; 2],
    pub tilt: [f32; 2],
    pub zoom: [f32; 2],
}

pub const LIMITS: Limits = Limits {
    pan: [-GIMBAL_PAN_LIMIT, GIMBAL_PAN_LIMIT],
    tilt: [GIMBAL_TILT_MIN, GIMBAL_TILT_MAX],
    zoom: [1.0, MAX_ZOOM],
};

/// `state` in the status: what obsbot4linux remembers (state file), plus `zone_follow` and
/// `startup`, which are read from the camera (null when it did not answer). Kept under `state`
/// so existing clients keep working.
#[derive(Debug, Serialize)]
pub struct StateView {
    #[serde(flatten)]
    pub remembered: State,
    pub zone_follow: Option<bool>,
    pub startup: Option<Startup>,
}

/// The rarely changing readbacks; the daemon keeps them between full reads.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneInfo {
    pub zone: Option<Zone>,
    pub hand: Option<Hand>,
    pub startup: Option<Startup>,
    pub framing: Option<Framing>,
    pub presets: Option<Vec<Preset>>,
}

impl Camera {
    pub fn zone_info(&self) -> ZoneInfo {
        ZoneInfo {
            zone: self.zone().ok(),
            hand: self.hand().ok(),
            startup: self.startup().ok(),
            framing: self.framing().ok(),
            presets: self.presets().ok(),
        }
    }

    pub fn full_status(&self) -> Result<FullStatus> {
        self.full_status_with(None, None)
    }

    /// Like [`Camera::full_status`], with the zone readbacks and the identity given instead of
    /// asked for again.
    pub fn full_status_with(
        &self,
        known: Option<ZoneInfo>,
        identity: Option<Identity>,
    ) -> Result<FullStatus> {
        let status = self.status()?;
        let tracking = match self.tracking() {
            Ok(t) => Some(t),
            Err(Error::NoAnswer(_) | Error::Rejected(..)) => None,
            Err(e) => return Err(e),
        };
        let factor = self.zoom_factor().ok();
        let absolute = self.dev.get_ctrl(ZOOM_ABSOLUTE).ok();
        let detected = self.dev.detected();
        let streaming_apps = streaming::apps_using(&detected.device);
        // an asleep camera does not answer these; asking would only wait out the polling
        let (audio_volume, focus) = if status.asleep {
            (None, None)
        } else {
            (self.audio_volume().ok(), self.focus().ok())
        };
        let target = if status.ai_mode == Some(AiMode::Static) {
            None
        } else {
            self.target().ok().flatten()
        };
        let ZoneInfo {
            zone,
            hand,
            startup,
            framing,
            presets,
        } = known.unwrap_or_else(|| self.zone_info());
        let Identity {
            firmware,
            model_info,
        } = identity.unwrap_or_else(|| self.identity());
        Ok(FullStatus {
            model: Some(detected.model.name),
            device: Some(detected.device.to_string_lossy().into_owned()),
            streaming: streaming::is_streaming(&detected.device),
            streaming_apps,
            available: true,
            firmware,
            model_info,
            status,
            tracking,
            zoom: Zoom { factor, absolute },
            pose: self.live_pose().ok(),
            target,
            image: self.image_controls()?,
            audio_volume,
            focus,
            state: StateView {
                remembered: State::load(),
                zone_follow: zone.as_ref().map(|z| z.follow),
                startup,
            },
            zone,
            hand,
            framing,
            presets,
            capabilities: CAPABILITIES.to_vec(),
            limits: LIMITS,
        })
    }
}

#[cfg(test)]
mod full_status_tests {
    use crate::models::tiny2lite::camera::{HandArea, Offset};
    use crate::models::tiny2lite::presets::{Pose, ZoneView};

    #[test]
    fn hand_serialises_area_and_a_null_return() {
        let mut h = Hand {
            area: HandArea {
                pan_min: -45.0,
                pan_max: 45.0,
                tilt_min: -30.0,
                tilt_max: 30.0,
            },
            return_pose: None,
            pan_enabled: Some(true),
            tilt_enabled: Some(false),
            return_after: Some(2.5),
        };
        let v = serde_json::to_value(&h).unwrap();
        assert_eq!(v["area"]["tilt_max"], 30.0);
        assert_eq!(
            (&v["pan_enabled"], &v["tilt_enabled"], &v["return_after"]),
            (&true.into(), &false.into(), &2.5.into())
        );
        assert!(v["return"].is_null(), "no return position set");
        h.return_pose = Some(Pose {
            pan: 20.0,
            tilt: 5.0,
            zoom: 1.0,
        });
        assert_eq!(
            serde_json::to_value(h).unwrap()["return"],
            serde_json::json!({"pan": 20.0, "tilt": 5.0, "zoom": 1.0})
        );
    }

    #[test]
    fn zone_serialises_with_return_as_key() {
        let z = Zone {
            pan_min: -10.0,
            pan_max: 10.0,
            tilt_min: -5.0,
            tilt_max: 5.0,
            follow: true,
            return_pose: Pose {
                pan: 0.0,
                tilt: 0.0,
                zoom: 1.0,
            },
            views: Some(vec![ZoneView {
                id: 0,
                name: "Bereich1".into(),
                pose: Pose {
                    pan: 0.5,
                    tilt: 0.0,
                    zoom: 1.5,
                },
            }]),
        };
        // the keys of 0.11 keep their place and shape; `views` is the only addition
        assert_eq!(
            serde_json::to_value(z).unwrap(),
            serde_json::json!({"pan_min": -10.0, "pan_max": 10.0, "tilt_min": -5.0, "tilt_max": 5.0,
                "follow": true, "return": {"pan": 0.0, "tilt": 0.0, "zoom": 1.0},
                "views": [{"id": 0, "name": "Bereich1", "pose": {"pan": 0.5, "tilt": 0.0, "zoom": 1.5}}]})
        );
    }

    use super::*;

    #[test]
    fn model_and_device_are_top_level_keys() {
        let full = FullStatus {
            model: Some("Tiny 2 Lite"),
            device: Some("/dev/v4l/by-id/x-video-index0".into()),
            streaming: true,
            streaming_apps: vec!["ffmpeg".into()],
            available: true,
            firmware: Some("6.2.8.1".into()),
            model_info: Some(ModelInfo {
                product: 8,
                platform: 8,
            }),
            status: Status::decode(&[0; 60]),
            tracking: None,
            zoom: Zoom {
                factor: None,
                absolute: None,
            },
            pose: None,
            target: None,
            image: BTreeMap::new(),
            audio_volume: Some(10),
            focus: None,
            state: StateView {
                remembered: State::default(),
                zone_follow: Some(true),
                startup: None,
            },
            zone: None,
            hand: None,
            presets: Some(vec![Preset {
                slot: 1,
                name: "Desk".into(),
                pose: Pose {
                    pan: 10.0,
                    tilt: -5.0,
                    zoom: 2.0,
                },
            }]),
            framing: Some(Framing {
                speed_mode: SpeedMode::VeryFast,
                auto_zoom: true,
                size: FramingSize::MediumCloseUp,
                zoom_speed: AutoZoomSpeed::Fast,
                offset: Offset {
                    x: 0.125,
                    y: -0.0625,
                },
                part_mode: PartMode::HeadRemoval,
            }),
            capabilities: CAPABILITIES.to_vec(),
            limits: LIMITS,
        };
        let v = serde_json::to_value(full).unwrap();
        // the widget reads these names; they are the snake_case of the CLI names
        assert_eq!(
            v["framing"],
            serde_json::json!({"speed_mode": "very_fast", "auto_zoom": true,
                "size": "medium_close_up", "zoom_speed": "fast",
                "offset": {"x": 0.125, "y": -0.0625}, "part_mode": "head_removal"})
        );
        // the widget reads `presets` at the top level; a camera that did not answer gives null
        assert_eq!(
            v["presets"],
            serde_json::json!([{"slot": 1, "name": "Desk", "pose": {"pan": 10.0, "tilt": -5.0, "zoom": 2.0}}])
        );
        assert!(CAPABILITIES.contains(&"preset_state"));
        assert_eq!(v["model"], "Tiny 2 Lite");
        assert_eq!(v["device"], "/dev/v4l/by-id/x-video-index0");
        assert_eq!(v["streaming"], true);
        assert_eq!(v["streaming_apps"], serde_json::json!(["ffmpeg"]));
        assert!(v["target"].is_null());
        assert_eq!(v["firmware"], "6.2.8.1");
        assert_eq!(
            v["model_info"],
            serde_json::json!({"product": 8, "platform": 8})
        );
        // zone_follow stays under state, next to the remembered values
        assert_eq!(v["state"]["zone_follow"], true);
        assert!(v["state"]["startup"].is_null() && v["zone"].is_null() && v["hand"].is_null());
        // the widget takes its d-pad and zone slider ranges from here
        assert_eq!(
            v["limits"],
            serde_json::json!({"pan": [-150.0, 150.0], "tilt": [-70.0, 90.0], "zoom": [1.0, 4.0]})
        );
        let t = Target {
            detected: true,
            coords: [0.25, 0.5, 0.75, 1.0],
        };
        assert_eq!(
            serde_json::to_value(t).unwrap(),
            serde_json::json!({"detected": true, "box": [0.25, 0.5, 0.75, 1.0]})
        );
        // the keys the widget already reads stay where they were
        assert_eq!(v["available"], true);
        assert!(v["status"].is_object() && v["capabilities"].is_array());
    }
}

/// `full_status_with` on a scripted camera (no device is opened).
#[cfg(test)]
mod scripted_camera_tests {
    use super::*;
    use crate::family::exchange::Camera;
    use crate::family::frame::Sequence;
    use crate::scripted::{Answer, Scripted};
    use crate::transport::uvc::Device;
    use std::sync::OnceLock;

    /// A camera that is awake, with HDR on and the LED in mode 2; it answers the zoom factor
    /// (2.0), the live pose (pan 25, tilt 10), the microphone volume (55) and the absolute zoom
    /// control (37). The tracking query gets `tracking`; every other query is rejected.
    fn camera(tracking: Answer) -> Camera {
        camera_in_mode(tracking, (0, 0), Vec::new())
    }

    /// The same camera in the AI mode of the status bytes 0x18 and 0x1c (0, 0 is Static), with
    /// `target` as the answer to the target query when it is not empty.
    fn camera_in_mode(tracking: Answer, ai_mode: (u8, u8), target: Vec<u8>) -> Camera {
        let mut status = [0u8; 60];
        status[0x06] = 1;
        status[0x18] = ai_mode.0;
        status[0x1c] = ai_mode.1;
        status[0x21] = 2;
        let mut script = Scripted::new(status);
        script.answers.insert(0x0104, tracking);
        if !target.is_empty() {
            script.answers.insert(0x01c4, Answer::Value(target));
        }
        script
            .answers
            .insert(0x6804, Answer::Value(2.0f32.to_le_bytes().to_vec()));
        let mut pose = vec![0u8; 12];
        pose[8..10].copy_from_slice(&(-1000i16).to_le_bytes());
        pose[10..12].copy_from_slice(&2500i16.to_le_bytes());
        script.answers.insert(0x0043, Answer::Value(pose));
        script
            .answers
            .insert(0x80c2, Answer::Value(55i16.to_le_bytes().to_vec()));
        script.ctrls.insert(ZOOM_ABSOLUTE, 37);
        static SEQ: OnceLock<Sequence> = OnceLock::new();
        Camera {
            dev: Device::scripted(script),
            seq: SEQ.get_or_init(Sequence::new),
        }
    }

    /// The zone readbacks and the identity the daemon keeps, so the test asks only for the rest.
    fn known() -> (Option<ZoneInfo>, Option<Identity>) {
        (
            Some(ZoneInfo {
                zone: None,
                hand: None,
                startup: None,
                framing: None,
                presets: None,
            }),
            Some(Identity {
                firmware: Some("6.2.8.1".into()),
                model_info: None,
            }),
        )
    }

    #[test]
    fn a_rejected_tracking_query_gives_tracking_null_and_the_rest_of_the_status() {
        // the camera refuses 0x0104: status and the daemon's broadcast must still show
        // everything else, with `tracking: null`, not fail as a whole
        let (zones, identity) = known();
        let full = camera(Answer::Rejected(-3))
            .full_status_with(zones, identity)
            .expect("a rejected tracking query does not fail the read");
        assert!(full.tracking.is_none());
        assert!(full.status.hdr);
        assert_eq!(full.status.led, 2);
        assert_eq!(full.zoom.factor, Some(2.0));
        assert_eq!(full.zoom.absolute, Some(37));
        let pose = full
            .pose
            .expect("the live pose is read after the tracking query");
        assert_eq!((pose.pan, pose.tilt), (25.0, 10.0));
        assert_eq!(full.audio_volume, Some(55));
        assert_eq!(full.firmware.as_deref(), Some("6.2.8.1"));
        assert_eq!(full.model, Some("Tiny 2 Lite"));
    }

    #[test]
    fn an_answered_tracking_query_is_in_the_status() {
        // the same script with a real answer: tracking is filled, so the test above sees the
        // arm and not a camera that never reports tracking
        let mut answer = vec![0u8; 12];
        answer[0] = 1; // group
        answer[10] = 1; // zone tracking
        let (zones, identity) = known();
        let full = camera(Answer::Value(answer))
            .full_status_with(zones, identity)
            .unwrap();
        assert!(full.tracking.is_some_and(|t| t.zone_tracking));
    }

    #[test]
    fn any_other_tracking_error_fails_the_whole_read() {
        // only "no answer" and "rejected" are tolerated; a failed write is a broken camera
        let (zones, identity) = known();
        let result = camera(Answer::SendFails).full_status_with(zones, identity);
        assert!(matches!(result, Err(Error::Io(_))), "{result:?}");
    }

    #[test]
    fn the_target_is_asked_only_while_ai_tracking_is_on() {
        // `01 01 <detected> 01 00 00` and four f32; a camera in Static would answer a single 00,
        // so it is not asked at all
        let mut answer = vec![1, 1, 1, 1, 0, 0];
        for v in [0.25f32, 0.5, 0.75, 1.0] {
            answer.extend(v.to_le_bytes());
        }
        let target = |ai_mode| {
            let (zones, identity) = known();
            camera_in_mode(Answer::Value(vec![0; 12]), ai_mode, answer.clone())
                .full_status_with(zones, identity)
                .unwrap()
                .target
        };
        assert_eq!(target((0, 0)), None, "Static");
        let followed = target((2, 0)).expect("Normal tracking asks for the target");
        assert!(followed.detected);
        assert_eq!(followed.coords, [0.25, 0.5, 0.75, 1.0]);
    }
}
