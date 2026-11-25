// SPDX-License-Identifier: GPL-3.0-or-later

//! Status blocks recorded on the owner's device around sleep, wake, the microphone restart and
//! plug-in, run through the Tiny 2 Lite's status decoder. They answer parts of camera items of
//! design spec 14 from recordings (CAM-3, CAM-8 and U-23, CAM-20); the camera session CS-1
//! still runs every item. Two tests pin what the model does today with a block whose meaning
//! CS-1 settles: the pull request that follows the item's answer changes them.

mod support;

use obsbot_device::models::tiny2lite::decode;
use serde_json::json;
use support::block;

/// Capture 13, the last status block before the camera was sent to sleep (22.4 s) and the
/// first after it was woken (36.2 s): awake.
const AWAKE_13: &str = "2701000110000101010188ff0001010a0003000001002100000103000000001e00031000";
/// Capture 13, the first status block after the sleep command (28.5 s): asleep. Byte 0x09
/// reads 3 instead of 1 while asleep.
const ASLEEP_13: &str = "2701010110000101010388ff0001010a0003000001002100000103000000001e00031000";
/// Capture 14, after the restart that switched the microphone off (72.5 s).
const MIC_OFF_14: &str = "2701000100000101010188ff0001010a0000000001002100000103000000001e00030000";
/// Capture 14, right after the command that switches the microphone on again, before the
/// camera restarts (114.9 s). It differs from `MIC_OFF_14` in byte 0x22 alone.
const MIC_ON_14: &str = "2701000100000101010188ff0001010a0000000001002100000103000000001e00031000";

/// CAM-3 in part: the decoder reads `asleep` from the blocks recorded around a sleep and a
/// wake (capture 13). The recording shows the asleep block at the first read after the sleep
/// command (5.2 s later) and the awake block at the first read after the wake command (3.3 s
/// later); how long the camera itself takes is for CS-1.
#[test]
fn asleep_follows_the_blocks_recorded_around_sleep_and_wake() {
    assert_eq!(decode::status(&block(AWAKE_13))["asleep"], json!(false));
    assert_eq!(decode::status(&block(ASLEEP_13))["asleep"], json!(true));
}

/// CAM-8 and U-23 in part: in capture 14 the status byte 0x22 reads 0x10 while the microphone
/// is on and 0x00 while it is off, and it changes with the command, before the restart. The
/// model reads `audio.enabled` from the USB audio interface (spec 4.3), not from this byte;
/// whether the byte follows the microphone on a cold plug is CS-1's (CAM-8). Until then the
/// decoder reads no leaf from it.
#[test]
fn no_leaf_is_read_from_the_byte_that_follows_the_microphone_until_u_23() {
    let (off, on) = (block(MIC_OFF_14), block(MIC_ON_14));
    let differ: Vec<usize> = (0..60).filter(|&i| off[i] != on[i]).collect();
    assert_eq!(
        differ,
        [0x22],
        "the recorded blocks differ in byte 0x22 only"
    );
    assert_eq!((off[0x22], on[0x22]), (0x00, 0x10));
    assert_eq!(decode::status(&off), decode::status(&on));
}

/// CAM-20, what the decoder does with a zero block. After every enumeration recorded on the
/// owner's device (capture 01 at plug-in, capture 14 after both microphone restarts, capture 16
/// at plug-in and after the factory reset) the host's minimum, maximum, resolution and default
/// queries of the status selector were answered with 60 zero bytes; every status read after an
/// enumeration returned a full block, and no recorded status read has 0 in its first field.
/// So the recordings do not show whether a status read during boot returns such a block. The
/// decoder turns a zero block into values that look real (`sleep.auto.seconds` 0,
/// `image.frame_rate` 0). CS-1 answered CAM-20 with no on 2026-11-25 (spec 14, field ledger):
/// the first status read after a cold re-plug and after a factory reset returned a full block
/// with byte 0 `0x27`, so the model has no not-ready block to recognise, and a zero block stays
/// decoded as values. This test pins that decision (w2-7 closed without a change); the test's
/// name predates the answer.
#[test]
fn a_boot_block_decodes_as_values_until_cam_20_is_settled() {
    let values = decode::status(&[0u8; 60]);
    assert_eq!(values["asleep"], json!(false));
    assert_eq!(values["sleep.auto.seconds"], json!(0));
    assert_eq!(values["image.frame_rate"], json!(0));
    assert_eq!(values["zoom.field_of_view"], json!("wide"));
}
