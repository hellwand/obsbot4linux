# SPDX-License-Identifier: GPL-3.0-or-later
"""Splits crates/obsbot-device/src/models/tiny2lite/camera.rs (the 0.13 camera.rs, moved there
with git mv) into four files. Every block is copied unchanged; the only edits are the visibility
widenings listed in VIS. Run from the repository root:
python3 -I scripts/groundwork/split-camera.py"""

import os

D = "crates/obsbot-device/src"
SRC = os.path.join(D, "models/tiny2lite/camera.rs")
L = open(SRC).read().split("\n")

# (line, text the line starts with): the boundaries of every block below, in camera.rs as on
# main 66287b5. If one does not match, main changed camera.rs: find the anchors with grep -n,
# fix the numbers here and in BLOCKS, and say so in the pull request.
ANCHORS = {
    20: "const REPLY_GAP",
    23: "const PAIR_GAP",
    25: "/// Commands obsbot4linux supports",
    76: "];",
    78: "/// UVC controls",
    100: "const ZOOM_ABSOLUTE",
    101: "const PAN_ABSOLUTE",
    102: "const TILT_ABSOLUTE",
    104: "#[derive(Debug, Clone, Serialize)]",
    109: "}",
    111: "#[derive(Debug, Serialize)]",
    118: "}",
    120: "/// Degrees",
    138: "}",
    140: "#[derive(Debug, Serialize)]",
    184: "}",
    186: "/// `limits`",
    199: "};",
    201: "/// `state`",
    210: "}",
    212: "/// Zone tracking",
    314: "}",
    316: "/// The rarely",
    324: "}",
    326: "/// What does not",
    331: "}",
    333: "/// The microphone",
    374: "}",
    376: "/// One sequence",
    381: "}",
    383: "pub struct Camera",
    386: "}",
    388: "impl Camera {",
    389: "    pub fn open",
    394: "    }",
    396: "    fn short",
    436: "    }",
    438: "    // --- reading ---",
    624: "    }",
    626: "    pub fn zone_info",
    634: "    }",
    636: "    pub fn full_status",
    638: "    }",
    640: "    /// Like [`Camera::full_status`]",
    707: "    }",
    709: "    // --- simple settings ---",
    1226: "    }",
    1228: "    /// The pose to save",
    1246: "    }",
    1248: "    /// Saves a preset",
    1297: "    }",
    1298: "}",
    1300: "#[cfg(test)]",
    1301: "mod full_status_tests {",
    1466: "}",
    1468: "#[cfg(test)]",
    1665: "}",
}
for n, start in ANCHORS.items():
    assert L[n - 1].startswith(start), f"camera.rs line {n}: {L[n - 1]!r}"
assert len(L) == 1666 and L[1665] == "", "camera.rs must have 1665 lines"

# The only edits: items another file now uses become pub(crate).
VIS = [
    ("const PAIR_GAP", "pub(crate) const PAIR_GAP"),
    ("    dev: Device,", "    pub(crate) dev: Device,"),
    ("    seq: &'static Sequence,", "    pub(crate) seq: &'static Sequence,"),
    ("    fn short(", "    pub(crate) fn short("),
    ("    fn framed(", "    pub(crate) fn framed("),
    ("    fn pair(", "    pub(crate) fn pair("),
    ("    fn query(", "    pub(crate) fn query("),
    ("    fn query_to(", "    pub(crate) fn query_to("),
    ("    fn answer(", "    pub(crate) fn answer("),
    ("const ZOOM_ABSOLUTE", "pub(crate) const ZOOM_ABSOLUTE"),
    ("    fn current_pose(", "    pub(crate) fn current_pose("),
]


def R(a, b):
    out = []
    for line in L[a - 1 : b]:
        for old, new in VIS:
            if line.startswith(old):
                line = new + line[len(old) :]
        out.append(line)
    return out


SPDX = "// SPDX-License-Identifier: GPL-3.0-or-later"
EUPL = "// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5."


def write(path, lines):
    with open(os.path.join(D, path), "w") as f:
        f.write("\n".join(lines) + "\n")


write(
    "family/exchange.rs",
    [
        SPDX,
        EUPL,
        "",
        "//! The exchange with the camera's extension unit: short commands, framed commands and the",
        "//! polling for their answers, with one sequence number counter for the whole process.",
        "",
        "use crate::error::{Error, Result};",
        "use crate::family::frame::{Answer, Sequence, parse_answer, query_frame};",
        "use crate::transport::uvc::Device;",
        "use std::sync::OnceLock;",
        "use std::thread::sleep;",
        "use std::time::Duration;",
        "",
    ]
    + R(20, 23)
    + [""]
    + R(376, 381)
    + [""]
    + R(383, 386)
    + ["", "impl Camera {"]
    + R(389, 394)
    + [""]
    + R(396, 436)
    + ["}"],
)

write(
    "legacy/full_status.rs",
    [
        SPDX,
        EUPL,
        "",
        "//! The status of 0.13 (`status --json` and the daemon's state): the full status, its",
        "//! capability list and limits, and the readbacks the daemon keeps between full reads.",
        "",
        "use crate::error::{Error, Result};",
        "use crate::family::exchange::Camera;",
        "use crate::family::streaming;",
        "use crate::legacy::state::{Startup, State};",
        "use crate::models::tiny2lite::camera::{",
        "    Framing, Hand, Identity, ImageControl, LivePose, ZOOM_ABSOLUTE, Zone,",
        "};",
        "use crate::models::tiny2lite::commands::*;",
        "use crate::models::tiny2lite::presets::{MAX_ZOOM, Preset};",
        "use crate::models::tiny2lite::status::{Focus, ModelInfo, Status, Target, Tracking};",
        "use serde::Serialize;",
        "use std::collections::BTreeMap;",
        "",
    ]
    + R(25, 76)
    + [""]
    + R(111, 118)
    + [""]
    + R(140, 184)
    + [""]
    + R(186, 199)
    + [""]
    + R(201, 210)
    + [""]
    + R(316, 324)
    + ["", "impl Camera {"]
    + R(626, 634)
    + [""]
    + R(636, 638)
    + [""]
    + R(640, 707)
    + ["}", ""]
    + R(1300, 1301)
    + [
        "    use crate::models::tiny2lite::camera::{HandArea, Offset};",
        "    use crate::models::tiny2lite::presets::{Pose, ZoneView};",
        "",
    ]
    + R(1302, 1466),
)

write(
    "legacy/current_pose.rs",
    [
        SPDX,
        EUPL,
        "",
        "//! The pose 0.13 saves when none is given: the remembered gimbal pose, else the one V4L2",
        "//! last set, with the live zoom factor.",
        "",
        "use crate::error::Result;",
        "use crate::family::exchange::Camera;",
        "use crate::legacy::state::State;",
        "use crate::models::tiny2lite::presets::{MAX_ZOOM, Pose};",
        "",
    ]
    + R(101, 102)
    + ["", "impl Camera {"]
    + R(1228, 1246)
    + ["}"],
)

write(
    "models/tiny2lite/camera.rs",
    L[0:4]
    + [
        "",
        "use crate::error::{Error, Result};",
        "use crate::family::exchange::{Camera, PAIR_GAP};",
        "use crate::legacy::state::{Startup, State};",
        "use crate::models::tiny2lite::commands::{self as cmd, *};",
        "use crate::models::tiny2lite::presets::{",
        "    self, MAX_ZOOM, Pose, Preset, StartupActions, StartupAi, ZoneView,",
        "};",
        "use crate::models::tiny2lite::status::{",
        "    Focus, ModelInfo, Status, Target, Tracking, firmware_version,",
        "};",
        "use crate::transport::uvc::CtrlRange;",
        "use serde::Serialize;",
        "use std::collections::BTreeMap;",
        "use std::thread::sleep;",
        "",
    ]
    + R(78, 100)
    + [""]
    + R(104, 109)
    + [""]
    + R(120, 138)
    + [""]
    + R(212, 314)
    + [""]
    + R(326, 331)
    + [""]
    + R(333, 374)
    + ["", "impl Camera {"]
    + R(438, 624)
    + [""]
    + R(709, 1226)
    + [""]
    + R(1248, 1297)
    + ["}", ""]
    + R(1468, 1665),
)
