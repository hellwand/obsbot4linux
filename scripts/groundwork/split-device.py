# SPDX-License-Identifier: GPL-3.0-or-later
"""Splits crates/obsbot-device/src/transport/uvc.rs (the 0.13 device.rs, moved there with git
mv) into uvc.rs (the device and its ioctls) and enumeration.rs (finding the camera). Every
block is copied unchanged. Run from the repository root:
python3 -I scripts/groundwork/split-device.py"""

D = "crates/obsbot-device/src"
UVC = D + "/transport/uvc.rs"
L = open(UVC).read().split("\n")

# (line, text the line starts with) in device.rs as on main 66287b5
ANCHORS = {
    12: "use crate::error::{Error, Result};",
    13: "use crate::registry::{self, Model};",
    16: "use nix::{ioctl_read, ioctl_readwrite};",
    17: "use std::fs::{self, File};",
    19: "use std::path::PathBuf;",
    32: "const V4L2_CAP_META_CAPTURE",
    44: "#[repr(C)]",
    46: "struct V4l2Capability",
    54: "}",
    78: "ioctl_read!(vidioc_querycap",
    92: "/// A camera found by enumeration",
    99: "}",
    122: "fn card_name",
    134: "}",
    136: "/// Finds a supported camera",
    172: "}",
    174: "/// The first supported camera",
    182: "}",
    276: "#[cfg(test)]",
    278: "    use super::*;",
    312: "    #[test]",
    313: "    fn by_id_detection_does_not_depend_on_directory_order",
    325: "    }",
    326: "}",
}
for n, start in ANCHORS.items():
    assert L[n - 1].startswith(start), f"device.rs line {n}: {L[n - 1]!r}"
assert len(L) == 327 and L[326] == "", "device.rs must have 326 lines"

MOVED = [
    (32, 32),
    (44, 54),
    (78, 78),
    (92, 99),
    (122, 134),
    (136, 172),
    (174, 182),
    (312, 325),
]


def R(a, b):
    return L[a - 1 : b]


enumeration = [
    "// SPDX-License-Identifier: GPL-3.0-or-later",
    "// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.",
    "",
    "//! Finding a supported camera without talking to it: `/dev/v4l/by-id` links first, the V4L2",
    "//! card name as fallback.",
    "",
    "use crate::registry::{self, Model};",
    "use nix::ioctl_read;",
    "use std::fs::{self, File};",
    "use std::os::fd::AsRawFd;",
    "use std::path::PathBuf;",
]
for a, b in MOVED[:-1]:
    enumeration += [""] + R(a, b)
enumeration += (
    ["", "#[cfg(test)]", "mod tests {", "    use super::*;", ""]
    + R(312, 325)
    + ["}", ""]
)
open(D + "/enumeration.rs", "w").write("\n".join(enumeration))

drop = set()
for a, b in MOVED:
    drop.update(range(a, b + 1))
drop.add(311)  # the blank line before the moved test
uvc = [line for n, line in enumerate(L, 1) if n not in drop]
text = "\n".join(uvc)
old_uses = (
    "use crate::error::{Error, Result};\nuse crate::registry::{self, Model};\n"
    "use nix::errno::Errno;\nuse nix::fcntl::{Flock, FlockArg};\n"
    "use nix::{ioctl_read, ioctl_readwrite};\nuse std::fs::{self, File};\n"
    "use std::os::fd::AsRawFd;\nuse std::path::PathBuf;\n"
)
new_uses = (
    "use crate::enumeration::{Detected, detect};\nuse crate::error::{Error, Result};\n"
    "use nix::errno::Errno;\nuse nix::fcntl::{Flock, FlockArg};\nuse nix::ioctl_readwrite;\n"
    "use std::fs::File;\nuse std::os::fd::AsRawFd;\n"
)
assert text.count(old_uses) == 1
text = text.replace(old_uses, new_uses)
# the remaining tests of uvc.rs remove their lock file with fs::remove_file
assert text.count("mod tests {\n    use super::*;\n") == 1
text = text.replace(
    "mod tests {\n    use super::*;\n",
    "mod tests {\n    use super::*;\n    use std::fs;\n",
)
open(UVC, "w").write(text)
