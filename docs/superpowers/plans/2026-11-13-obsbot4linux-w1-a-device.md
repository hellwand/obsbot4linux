# obsbot4linux 1.0, track A (device) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the carried 0.13 camera code in `crates/obsbot-device` into the Tiny 2 Lite model behind `trait Camera`, on a transport behind `trait Transport` that sends only the model's send table: the USB transport (extension unit by GUID, named timeouts, per-operation node lock), the replay transport for tests, the send-table check, the family link (answer matching with the audit's fixes), the sysfs enumeration and bus, the registry entry, and `#[ignore]` live tests for camera session CS-1. Every request sends exactly the frames of its field-ledger row; the carried flows lose the switch to the static tracking mode and the invented preset name; the carried 0.13 modules (`legacy/`, the 0.13 flows, the old transport, detection and registry) are gone at the end.

**Architecture:** New code is written beside the carried code and the carried modules are deleted in the last task, so every pull request builds and every carried test runs until its replacement exists. `family::link::Link` owns the model's `Box<dyn Transport>` and does every exchange (context check, node lock, frame, answer poll). `models/tiny2lite` is split by concern: `decode` (answers → leaf values, pure), `plan` (registry entry, poll plan, touched groups, apply order, motion marks: data), `send_table` (data), `read`, `write`, `act`, and `model` (the `trait Camera` impl that delegates). The carried byte builders in `commands.rs` and `presets.rs` make every frame, so the stage-G byte guard keeps pinning the bytes; the guard's fixture changes only by deleted lines. Tests drive the model on `transport::replay::Replay` (recorded answers) behind `transport::checked::Checked`, so a frame outside the send table fails any test.

**Tech Stack:** Rust 2024 edition, MSRV 1.88 (CI toolchain 1.99.1), nix 0.31 (ioctl, flock), serde_json 1, thiserror 2, obsbot-proto (W0), hex (dev). No async runtime, no libudev.

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` (revision 2, approved 2026-11-13, with the changes after approval of its section 15.1), sections 2, 3.1, 3.2, 4.2 to 4.4, 5.3, 5.5, 5.7, 5.8, 8.1 to 8.3, 8.5, 8.8, 9.3, 10.3 (track A), 11, 14; field ledger `docs/field-ledger.md`; the W0 plan `docs/superpowers/plans/2026-11-13-obsbot4linux-w0-contracts.md` (its code is the contract this plan builds on).

## Global Constraints

- "A request sends exactly what it names: the frames of that setting or action, nothing before them and nothing after them" (spec 2). "the carried gimbal move and rate flows lose their switch to the static tracking mode" (3.2, 15.1 row 88). "`preset.save` without `name` sends the preset and no name: no rename frame follows, and obsbot4linux invents no name" (4.4, 15.1 row 90).
- The one exception: "the preset read writes to the camera to rewind its list … this rewind is the one known exception to section 2's 'no command sequence as a workaround' and stays only until CAM-19 decides" (8.8).
- "Each model declares its send table … The transport wrapper checks every outgoing frame against the active model's table in one place. Anything else is refused with `internal`, logged and never sent" (8.3); "every extension-unit selector outside the selectors the model's registry entry declares" and "on the receiver that answers identity queries (`0x0d` on the Tiny 2 Lite), every frame that is not one of the model's declared identity queries" are refused as classes (8.3).
- "No raw send path exists anywhere" (8.3). "No probing. … The extension unit is found by the GUID its registry entry declares, in the USB descriptor (read from sysfs), never by a request" (8.3).
- "A test drives every setting and action of each model at its boundary values on the replay transport and asserts that every frame is in the send table, and that each setting and action sends exactly the frames of its ledger row, in that order" (8.3).
- Timeouts (5.8): `ANSWER_POLL` "20 reads, 50 ms apart (about 1 s) per query"; `FRAME_GAP` "100 ms between two frames of one command, where a model needs it"; `NODE_LOCK_WAIT` "5 s for the per-operation lock on the video node".
- "A read that fails because the node is gone is reported at once, without waiting for the answer timeout" (5.3).
- Enumeration "reads, from sysfs only: vendor id, product id, product string, whether a serial string exists, bcdDevice, the device number on its bus, the interface classes, the extension units from the USB descriptors, and the node index. It never opens a video node during the scan" (5.3). "One USB device is one entry, whatever number of video nodes it has" (5.3).
- Registry: "The vendor table … today `0x3564`"; "Registered today: `tiny2lite`, `3564:fef9` (measured). Nothing else"; "A test fails if a pair belongs to two models" (8.1).
- Tiny 2 Lite (8.8): selectors "2 (framed commands and their answers), 6 (short commands and the status block), `0x0c` and `0x0d` (the preset list and its entries)"; identity "the serial query `0x18c8` to receiver `0x0d`" plus "the firmware query the ledger lists"; "Live pose: query `0x0043` to receiver 3, plus the zoom-factor query"; "Support: `untested` on `rebuild` until stage W3".
- Poll plan (8.8): "1 s: the status block …, the on-air probe, and the target while the tracking mode is not `static`. 1 s while a connection subscribes `pose`, else 5 s: the live pose. 5 s: the settings readbacks … 60 s: zone, hand, power-on, views. `touched`: presets … While asleep only the status block is read."
- Apply order (8.8): "a setting's `requires` paths before the setting …, `exposure.mode` before `exposure.gain`, and `tracking.mode` before `framing.*`". Touched (8.8): "`tracking.mode` touches tracking and framing; `zoom.field_of_view` and the gesture settings touch zoom; `preset.recall` touches zoom and the live pose; `gimbal.move`, `gimbal.turn`, `gimbal.stop`, `zoom.stop` and `zoom.move` touch nothing."
- Values (4.2): "Undecodable values … published as the string `raw:<n>` … never turned into `null` or into a neighbouring value"; precision "angles 0.1, zoom 0.01, fractions 0.01 … rounded to it where they are created"; "pan positive = the camera turns to its left; tilt positive = up".
- Contract-change rule (10.1): `trait Camera`, `trait Transport`, the proto types and the schema change only "first as its own small PR"; this plan's Task A-1 is that PR. Track A's owner files: "`crates/obsbot-device` (except the trait files), `descriptions/tiny2lite.json`, `docs/field-ledger.md`" (10.3).
- Fixtures (8.5): "single exchanges … each with its capture key. Recording files never enter the repository. Identity answers and descriptors that hold a serial string are never fixtures; tests use synthetic values."
- Public text (spec 8.5): comments describe this project's behaviour and what was recorded on the owner's device ("recorded on the owner's device (capture N)", "measured on the camera"); no names of methods or symbols of other software or firmware and no statement that knowledge comes from them; no third parties named as sources; no lists of ids that must not be sent; no serial numbers (the synthetic `EXAMPLE0000001` is allowed). The licence notes `// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.` stay and follow their code (3.3).
- Every new source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`; a file that receives code from a file with the EUPL note carries the note too (3.3).
- No camera I/O in any task's acceptance. The live tests are `#[ignore]` and need `OBSBOT_LIVE=1` (spec 10.3: "run only with an explicit environment variable and a connected camera and send only the model's send table").
- Builds: `export CARGO_TARGET_DIR=<a directory outside the repository and outside /tmp>`, `nice -n 19`, `-j 4`; every wait has a timeout. Gate of every task: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`. Commits are signed and end with the `Co-Authored-By:` line of the model that wrote them; no session link.

## Review Focus

The six inputs most likely to hit a user that no test of the carried code exercised, each now pinned in its owning task:

1. **The answer of another process's query taken for ours (#61).** A tool without the node lock queries the camera with our sequence number; 0.13 decoded its answer. Pinned by `the_answer_to_another_query_with_our_number_is_not_ours` (A-2, `family/link.rs`) and `an_answer_must_echo_sequence_and_command_and_carry_good_crcs` (A-2, `family/frame.rs`).
2. **An unplug during a query reported as "no answer" after a second (audit A-002).** Pinned by `a_failed_read_ends_the_poll_at_once` (A-2), `a_gone_camera_fails_a_read_at_once` (A-9), `a_node_that_is_gone_is_gone_at_once` (A-8).
3. **Auto-sleep: a patch that sets the time and switches it on, or the time alone, writes a stale or an invented word.** The two leaves are one camera word. Pinned by `auto_sleep_writes_one_word_with_the_other_half_as_the_camera_has_it`, `auto_sleep_with_0_s_stored_sends_nothing_for_the_switch_alone` (A-10) and `requires_paths_go_first_and_the_rest_keeps_its_order` (A-7).
4. **The start pose wiping the start mode and the start settings.** 0.13 wrote a fresh "not set" block after the pose; spec 4.4 says the actions "keep its tracking mode and `power_on.settings`". Pinned by `the_start_pose_is_set_and_the_block_written_back_unchanged` and `a_short_initial_state_block_stops_the_start_pose_before_any_frame` (A-11).
5. **A gimbal turn after a mirror toggle turning the wrong way.** The yaw sign follows mirror xor invert; with either flag unread the sign is unknown. Pinned by `a_gimbal_turn_sends_only_the_turn_with_the_yaw_sign_of_mirror_xor_invert` and `a_gimbal_turn_is_refused_until_mirror_and_invert_are_read` (A-11).
6. **Every exchange running without the node lock (5.8).** `Transport::hold` and `release` have empty defaults, so a wrapper that does not pass them on, or a model built on an unchecked transport, talks to the camera unlocked and no other test notices. Pinned by `the_lock_reaches_the_inner_transport` (A-6) and `a_query_of_the_model_holds_the_node_lock_over_the_frame_and_its_answer` (A-9, on the transport `Tiny2Lite::checked` builds, the only constructor of the model).

---

## What track A needs from W0 and from the other tracks

From W0 (its plan's code; if the tree on `rebuild` differs, the tree wins), by exact name:

- `obsbot_device::contract::{Camera, CameraError, Ctx, Group, Identity, LiveTier, Motion, Need, Target, Tier, Frame, SendEntry, SendTable, Transport, ControlRange, OnAir, Bus, Found, IdentitySource, Lookup, ModelEntry, Registry, UsbId}` and `obsbot_device::contract::camera::Result` (W0 Task 5); `obsbot_device::contract::conformance::check(&mut dyn Camera) -> Vec<String>` (W0 Task 7).
- `obsbot_device::models::tiny2lite::description::{description, MODES, SETTLE_MS, UNBACKED, UNCONFIRMED}` (W0 Task 8): 84 leaves, 27 actions, `limits.presets = 3`.
- `obsbot_proto::{Description, Setting, SettingType, Bound, Params, State, Values, Usb, ExtensionUnit, ErrorCode, is_valid_serial, short_id}`, `obsbot_proto::validate::Refusal` with `Refusal::new(ErrorCode, &str, &str)`, `obsbot_proto::values::{number, rounded, raw, same}` (W0 Task 1).
- W0 Task 6 has removed `legacy/full_status.rs` and the 0.13 command line; `legacy/state.rs` and `legacy/current_pose.rs` are still there (W0 decision 15).

From tracks B, C, D: nothing. Track A provides to track B (B-2): `obsbot_device::bus::SysfsBus` (`SysfsBus::new()` on `/sys`, `SysfsBus::with_root(root: impl Into<PathBuf>)` on another sysfs root for tests, both with `REGISTRY`; `Default`; `impl Bus`), `obsbot_device::registry::REGISTRY`, and the model behind `Bus::open`; track B logs a `CameraError::NotInSendTable` it receives (answered `internal`), which is how 8.3's "logged" is met.

## Decisions this plan takes

Each can be overruled by the maintainers; the reason follows the dash.

1. **Contract change first (A-1):** `Frame::Query { selector, receiver, command }` and `Frame::Block { selector }`, and `Transport::hold` / `Transport::release` with default bodies. — Without `Query` a send-table entry for a query would also allow a command with that id; the preset rewind writes a whole buffer that no W0 frame kind names; the node lock must cover a query and the reads of its answer (one answer buffer), which the transport cannot know from single calls.
2. **The node lock is held per exchange** (one command, one query and its answer poll, one buffer read or write), not per group read. — Spec 5.8 "takes turns with the daemon on single frames"; a whole group read under one lock would hold the firmware updater off for seconds.
3. **Every read error ends the answer poll at once** with its error (`Gone` stays `Gone`). — Audit A-002: no recording shows a read error before an answer is ready; spec 5.3 wants an unplug at once.
4. **An answer must echo the sequence number and the command id and carry a good header CRC; a value answer also a 12-byte header and a good data CRC** (#61). — Both CRC rules hold on the recorded answers in this plan's fixtures (captures 01, 04, 08, 11), which A-2 and A-3 pin byte for byte.
5. **New modules beside the carried ones; the carried ones are deleted in A-14.** — Each pull request stays green and the carried tests keep running until their replacement exists.
6. **The carried byte builders make every frame;** A-14 deletes only the builders no 1.0 path uses (`view_center`: PX-1; `pickup_distance`: merged into `audio.volume`; `auto_sleep_off`: it writes a time of its own; `startup_picture_settings`, `StartupAi`, `startup_ai_mode`, `StartupActions`: the start pose now writes back the block it read, and `decode` reads the block). — The stage-G byte guard keeps pinning every kept builder; its fixture loses exactly those builders' lines and gains none.
7. **Groups follow the queries** (`status`, `on_air`, `target`, `pose`, `gestures` = `0x0104`, `zoom`, `framing`, `uvc`, `focus`, `audio`, `zone`, `hand`, `hand_timer`, `power_on`, `views`, `presets`); `hand.return_after` has its own group. — A group read stops at its first unanswered query (5.7); the U-6 query must not leave the backed hand settings unread.
8. **The target group follows the trait doc of `Camera::read`:** the target query (`0x01c4`) is sent only while the tracking mode is read and not `static` (8.8); otherwise the group returns the values of its last read without a frame, and before its first read it fails with `NoAnswer` and sends nothing, so its leaves stay in `unread`. — W0's contract says "its last values, or `null` for a nullable leaf"; `tracking.target.detected` is not nullable, and a `false` the camera did not report would be a value of the model's own (spec 2).
9. **An identity query the camera does not answer, or refuses, spends the attempt** (`Ok` with `None`); `Gone`, `Cut`, `Locked` and I/O errors fail it. — 5.5 counts attempts; an unanswered serial query is an attempt.
10. **`sleep.auto.seconds` goes before `sleep.auto.enabled` in a patch, and the model keeps the auto-sleep word** of its last status read or write (a status read when it has none). — The two leaves are one camera word (4.3); the second leaf of a patch must build on the word the first one wrote, not on a readback that may lag.
11. **Switching auto-sleep on or off alone while the camera has 0 s stored sends nothing** and fails (`camera`). — 0.13 wrote 600 s of its own; spec 2 forbids a write the user did not ask for, and "on with 0 s" was never recorded (PX-7).
12. **`zoom.factor` is written as 0–100 for 1.0–4.0** (field ledger), with no range query. — The ledger's measured mapping; the description already holds the bounds.
13. **`exposure.time` is written in 100 µs units, rounded down; `exposure.mode=auto` is written as V4L2 0.** — Ledger "100 µs units … the camera takes the step at or below"; parity table 9.1 "0.13.2 writes 0 for automatic".
14. **`gimbal.turn` is refused with `conflict` while `image.mirror` or `gimbal.invert` is not a read bool.** — The yaw encoding needs both (4.4); a guessed sign turns the camera the wrong way.
15. **An empty name is refused with `out_of_range`** for `preset.save`, `preset.rename`, `view.save`, `view.rename`. — The carried rule is 1–32 printable characters; the W0 validator lets `""` through and what the camera does with an empty name is not recorded.
16. **The pose-taking actions read the live pose (`0x0043` on receiver 3) and the zoom (`0x6804`)**, angles to 0.1°, the zoom kept within 1.0–4.0. — Spec 4.4, audit A-022; carried from the 0.13 live-pose helper.
17. **`preset.save` picks create or update, `view.save` the lowest free id, and `power_on.use_preset` the slot's pose from the published state** the daemon passes to `act`. — The W0 contract's `act(…, state, …)`; the daemon's `check` already refused an unread list.
18. **The start-pose actions read the `0x3e84` block, build both frames, then send `0x3ec4` and, 100 ms later, the block unchanged with `0x3e44`.** — Field ledger rows `power_on.set_pose`, `use_current`, `use_preset`; spec 4.4.
19. **Frames take their sequence numbers in the order they are sent; a name is checked with a build before the first frame.** — The camera drops a repeated number; the order of numbers on the wire was never recorded out of order.
20. **The two framing offsets are sent `FRAME_GAP` apart** (`Link::send_spaced`). — 0.13.2 sent them 100 ms apart because back to back the camera applies neither; in 1.0 they are two leaves of one patch.
21. **Touched groups follow 8.8 literally:** `preset.recall` and `power_on.use_preset` also touch `presets`. — 8.8 "read … after `preset.*` and `power_on.use_preset`" (see question 2 for the cost).
22. **The send-table check sits in front of the transport of every model, in the bus and in every test:** `Tiny2Lite::checked` is the model's only public constructor. — A frame outside the table then fails whatever test sends it, and no caller can build a model that bypasses the check.
23. **sysfs attributes:** `class/video4linux/videoN/{index,device}`, and of the USB device `idVendor`, `idProduct`, `bcdDevice`, `product`, `serial` (existence), `devnum`, `descriptors`, `<dev>:<cfg>.<if>/{bInterfaceNumber,bInterfaceClass}`; the USB device directory for the transport is `<root>/bus/usb/devices/<usb path>`. — Spec 5.3 leaves the choice to track A; CAM-25 confirms it.
24. **`obsbot-device` drops `clap` and `serde`** (the setting enums get `ALL`). — The 1.0 CLI is generic; nothing in the device crate parses arguments or serialises its own types.
25. **The live tests cover every CS-1 item a machine can check with the send table's frames** (CAM-1, CAM-2, CAM-3, CAM-4, CAM-5, CAM-10, CAM-13, CAM-15, CAM-16, CAM-17, CAM-20, CAM-22, CAM-23 readback rows, CAM-25, CAM-35, the UVC ranges of the W0 plan's decision 12); each test that writes puts back what it wrote. CAM-6, CAM-7, CAM-8, CAM-14, CAM-18, CAM-19, CAM-33 and CAM-34 stay steps of the CS-1 session (reasons under "Camera items this track leaves open"). — A cargo test cannot watch the camera, run another program against it, re-plug the camera or send a frame outside the table.

### Questions for the coordinator (not built in)

1. **`zone.area.*` and `hand.area.*` with `min == max`.** The W0 validator accepts it (its decision 9); the carried builder refuses it, so the request fails with `camera` and nothing is sent. Recommendation: keep refusing until CS-1 (U-8, CAM-16) shows what the camera does; if it takes it, the builder follows, if not, a contract change adds a strict range.
2. **Preset reads after `preset.recall` and `power_on.use_preset`.** Built as 8.8 says, which adds a rewind write (the section 2 exception) after every recall; spec 8.2's definition of `touched` ("groups whose values the camera itself changes") would not touch `presets` there. Recommendation: change 8.8 to "after `preset.save`, `preset.rename`, `preset.delete`" in a spec edit, then drop the two entries.
3. **The evidence levels of 8.5 as constants of the model module** ("The model module records the levels as constants. A test in stage W3 fails …") are in no track's content. Recommendation: one task after CS-1 (W2), when the levels stop changing; the send table's evidence strings and the W0 marks `UNBACKED` and `UNCONFIRMED` are its inputs.

## Decisions taken after the plan review (2026-11-13)

The coordinator decided the product questions of the review and the seam points between the tracks. Where a line below differs from the text of this plan, the line wins; the code of the plan is otherwise unchanged. The product questions above are answered by these lines.

1. **a-10.** Question 1, zone and hand area with start equal to end: keep refusing. The request fails with `camera` and sends nothing until the camera session shows what the camera does (CAM-16, U-8). Task a-10 builds it as the plan says; nothing changes.
2. **a-7.** Question 2, preset reads: the preset list is re-read only after `preset.save`, `preset.rename` and `preset.delete`, not after `preset.recall` or `power_on.use_preset` (spec 8.8 now says so). Task a-7 builds `touched` with `preset.recall` giving `["zoom", "pose"]` and `power_on.use_preset` giving `["power_on"]`; decision 21, the doc comment of `touched` and the two assertions in `touched_groups_of_8_8` change to match; any later test that expects a preset list read after a recall or `power_on.use_preset` is changed to expect none.
3. **a-3.** Dependency: a-3 depends on a-1 (`impl Transport for Replay` defines `hold` and `release`, which exist only after A-1). A-1 is wave 0 (the contract-change pull request that lands before every other W1 task); a-3 stays in wave 1. The builder of a-3 branches from `rebuild` after a-1 has landed.

## File structure

| File                                                                                                                                                                | Responsibility                                                                 | Task |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ---- |
| `crates/obsbot-device/src/contract/camera.rs`, `contract/transport.rs`, `tests/contract.rs` (modify)                                                                | `Frame::Query`, `Frame::Block`; `Transport::hold`, `release` (contract change) | A-1  |
| `crates/obsbot-device/src/transport/replay.rs`, `transport/mod.rs`                                                                                                  | the replay transport and the recorded answer layout                            | A-3  |
| `crates/obsbot-device/src/models/tiny2lite/decode.rs`, `tiny2lite/mod.rs`                                                                                           | answers → leaf values                                                          | A-4  |
| `crates/obsbot-device/src/enumeration/sysfs.rs`, `enumeration.rs`, `tests/sysfs.rs`, `tests/sysfs_tree/mod.rs`                                                      | the sysfs scan                                                                 | A-5  |
| `crates/obsbot-device/src/family/frame.rs`, `family/link.rs`, `family/mod.rs`, `family/exchange.rs`, `scripted.rs` (helpers)                                        | answer checks; the link                                                        | A-2  |
| `crates/obsbot-device/src/transport/checked.rs`, `transport/mod.rs`                                                                                                 | the send-table check                                                           | A-6  |
| `crates/obsbot-device/src/models/tiny2lite/plan.rs`, `send_table.rs`, `tiny2lite/mod.rs`                                                                            | registry entry, poll plan, touched, order, motion; send table                  | A-7  |
| `crates/obsbot-device/src/transport/usb.rs`, `transport/mod.rs`                                                                                                     | the USB transport                                                              | A-8  |
| `crates/obsbot-device/src/models/tiny2lite/{model,read}.rs`, `write.rs` and `act.rs` (stubs), `tiny2lite/mod.rs`, `tests/support/mod.rs`, `tests/tiny2lite_read.rs` | the model, identity and reads                                                  | A-9  |
| `crates/obsbot-device/src/models/tiny2lite/write.rs`, `commands.rs` (area axis), `tests/tiny2lite_write.rs`                                                         | writes                                                                         | A-10 |
| `crates/obsbot-device/src/models/tiny2lite/act.rs`, `tests/tiny2lite_act.rs`                                                                                        | actions and checks                                                             | A-11 |
| `crates/obsbot-device/tests/tiny2lite_frames.rs`                                                                                                                    | the 8.3 send-table test and the conformance check                              | A-12 |
| `crates/obsbot-device/src/registry.rs`, `bus.rs`, `lib.rs`, `tests/bus.rs`                                                                                          | the registry and the sysfs bus                                                 | A-13 |
| `crates/obsbot-device/tests/live.rs`                                                                                                                                | `#[ignore]` live tests for CS-1                                                | A-15 |
| deletions and edits across `crates/obsbot-device`                                                                                                                   | the carried 0.13 modules removed; `clap`, `serde` dropped; public-text pass    | A-14 |

`descriptions/tiny2lite.json` and `docs/field-ledger.md` do not change in this plan: nothing it builds changes a described field, and every frame it sends is a ledger row.

## Tasks, waves and dependencies

Tasks of one wave run side by side and touch different files. A task branches from `rebuild` after its dependencies have landed; where one is still in review, the task is stacked on that branch (column "stacked on"). A-1 is the contract-change pull request of spec 10.1 and lands on its own.

| Task | Title                                                   | Wave | Depends on       | Stacked on | Effort |
| ---- | ------------------------------------------------------- | ---- | ---------------- | ---------- | ------ |
| A-1  | Contract change: query and buffer frames, the node lock | 0    | –                | –          | S      |
| A-3  | Replay transport                                        | 1    | A-1              | –          | M      |
| A-4  | Tiny 2 Lite decoders                                    | 1    | –                | –          | M      |
| A-5  | sysfs scan                                              | 1    | –                | –          | M      |
| A-2  | Family link and answer checks                           | 2    | A-1, A-3         | A-1        | M      |
| A-6  | Send-table check                                        | 2    | A-1, A-3         | A-1        | S      |
| A-7  | Tiny 2 Lite plan and send table                         | 2    | A-1, A-4         | A-1        | M      |
| A-8  | USB transport                                           | 3    | A-1, A-5         | –          | M      |
| A-9  | Tiny 2 Lite model: identity and reads                   | 3    | A-2, A-6, A-7    | A-7        | L      |
| A-10 | Tiny 2 Lite writes                                      | 4    | A-9              | A-9        | M      |
| A-11 | Tiny 2 Lite actions and checks                          | 4    | A-9              | A-9        | M      |
| A-13 | Registry and sysfs bus                                  | 4    | A-8, A-9         | A-9        | S      |
| A-12 | Send-table test and conformance                         | 5    | A-10, A-11       | A-11       | S      |
| A-15 | Live tests for CS-1                                     | 5    | A-10, A-13       | A-13       | M      |
| A-14 | Remove the carried 0.13 modules                         | 6    | A-12, A-13, A-15 | –          | M      |

Files touched by two tasks: `transport/mod.rs` (A-3 wave 1, A-6 wave 2, A-8 wave 3), `models/tiny2lite/mod.rs` (A-4 wave 1, A-7 wave 2, A-9 wave 3), `commands.rs` (A-10 wave 4, A-14 wave 6), `registry.rs` and `lib.rs` (A-13 wave 4, A-14 wave 6): always in different waves. `Cargo.lock` changes only in A-14 (two dependencies dropped); on a rebase conflict take `rebuild`'s version and run `cargo build --workspace`.

The code of this plan was built and run on 2026-11-13 (rustc 1.99.1), and rechecked on the landed W0 (`rebuild` at `3187b84`): A-1 to A-13 and A-15 applied as written, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`: 360 tests passed, the 13 live tests compiled and ignored; the state of A-9 with its stubs checked on its own (fmt, clippy, `tests/tiny2lite_read.rs` 15 passed). A-14 was run in the first copy (stage G `37471a0` with the W0 plan's code). The code in the steps is that code.

---

### Task A-1: Contract change: query and buffer frames, the node lock

**Contract change** (spec 10.1): this pull request changes `crates/obsbot-device/src/contract/camera.rs` and `contract/transport.rs` and nothing else but their test. Why: (1) a send-table entry `Frame::Framed` for a query id would also let a command (frame type `0x25`) with that id through, so queries need a kind of their own; (2) the preset list rewind (8.8) writes a selector's whole buffer, which no W0 frame kind names; (3) the node lock of 5.8 must cover a query and the reads of its answer, which a transport cannot tell from single calls. All three are additive: `Frame` gets two variants, `Transport` two methods with default bodies, so the fakes and every other implementor compile unchanged.

**Files:**

- Modify: `crates/obsbot-device/src/contract/camera.rs` (enum `Frame`), `crates/obsbot-device/src/contract/transport.rs` (trait `Transport`)
- Test: `crates/obsbot-device/tests/contract.rs` (two tests added)

**Interfaces:**

- Consumes: W0 Task 5's contract.
- Produces: `Frame::Query { selector: u8, receiver: u8, command: u16 }`, `Frame::Block { selector: u8 }`; `trait Transport { …; fn hold(&mut self) -> Result<()> { Ok(()) }; fn release(&mut self) {} }`.

- [ ] **Step 1: Write the failing tests**

In `crates/obsbot-device/tests/contract.rs`, replace the `use obsbot_device::contract::{…};` line with:

```rust
use obsbot_device::contract::camera::Result as CamResult;
use obsbot_device::contract::{
    CameraError, ControlRange, Ctx, Frame, IdentitySource, Lookup, ModelEntry, OnAir, Registry,
    SendEntry, SendTable, Tier, Transport, UsbId,
};
```

and append at the end of the file:

```rust
#[test]
fn queries_and_block_writes_are_frames_of_their_own() {
    // A query (frame type 0x01 or 0x21) and a command (type 0x25) with the same receiver and
    // id are two entries: allowing the query must not allow a write with that id (8.3).
    let query = Frame::Query {
        selector: 2,
        receiver: 0x0d,
        command: 0x0408,
    };
    let command = Frame::Framed {
        selector: 2,
        receiver: 0x0d,
        command: 0x0408,
    };
    let table = SendTable {
        entries: vec![
            SendEntry {
                frame: query,
                evidence: "capture 01",
                identity: true,
            },
            SendEntry {
                frame: Frame::Block { selector: 0x0c },
                evidence: "measured 2026-10-31 (#104)",
                identity: false,
            },
        ],
    };
    assert!(table.allows(&query));
    assert!(!table.allows(&command), "a query entry allows no command");
    assert!(table.allows(&Frame::Block { selector: 0x0c }));
    assert!(!table.allows(&Frame::Block { selector: 0x0d }));
}

/// A transport that does not override `hold` and `release`.
struct Plain;

impl Transport for Plain {
    fn xu_len(&mut self, _selector: u8) -> CamResult<u16> {
        Ok(60)
    }
    fn xu_get(&mut self, _selector: u8, _buf: &mut [u8]) -> CamResult<()> {
        Ok(())
    }
    fn xu_set(&mut self, _selector: u8, _data: &[u8]) -> CamResult<()> {
        Ok(())
    }
    fn ctrl_get(&mut self, _id: u32) -> CamResult<i32> {
        Ok(0)
    }
    fn ctrl_set(&mut self, _id: u32, _value: i32) -> CamResult<()> {
        Ok(())
    }
    fn ctrl_range(&mut self, _id: u32) -> CamResult<Option<ControlRange>> {
        Ok(None)
    }
    fn usb(&mut self) -> CamResult<Usb> {
        Err(CameraError::Gone)
    }
    fn usb_serial(&mut self) -> CamResult<Option<String>> {
        Ok(None)
    }
    fn on_air(&mut self) -> CamResult<OnAir> {
        Ok(OnAir::default())
    }
}

#[test]
fn a_transport_without_a_lock_holds_and_releases_without_effect() {
    let mut t = Plain;
    assert!(t.hold().is_ok());
    t.release();
    t.release();
    assert!(t.hold().is_ok(), "a second hold after release works too");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --test contract`
Expected: FAIL to compile: `no variant named Query found for enum Frame`, `no variant named Block`, `no method named hold`.

- [ ] **Step 3: Extend the contract**

In `crates/obsbot-device/src/contract/camera.rs`, the enum `Frame` ends with:

```rust
    /// A short command on an extension-unit selector.
    Short { selector: u8, command: u8 },
    /// A write of a standard UVC control (V4L2 control id).
    Uvc { control: u32 },
    /// A framed query (frame type `0x01`, or `0x21` with a request such as a view id) on an
    /// extension-unit selector. A query is a read, but it is a frame the model writes, so the
    /// send table names it like a command; `Framed` names commands (type `0x25`) only.
    Query {
        selector: u8,
        receiver: u8,
        command: u16,
    },
    /// The whole buffer of a selector written back as it was read: on the Tiny 2 Lite the
    /// preset list rewind on selector `0x0c` (spec 8.8, CAM-19).
    Block { selector: u8 },
}
```

In `crates/obsbot-device/src/contract/transport.rs`, the trait `Transport` ends with:

```rust
    /// One sample of the on-air probe on the video node with index 0.
    fn on_air(&mut self) -> Result<OnAir>;
    /// Takes the per-operation lock on the video node (5.8 `NODE_LOCK_WAIT`) and keeps it
    /// until [`Transport::release`], so that one exchange (a command, or a query and the
    /// reads of its answer) is not split by another process that talks to the same camera.
    /// `Err(Locked)` when the lock stayed taken for `NODE_LOCK_WAIT`. A transport without
    /// a lock (replay, fakes) keeps the default, which does nothing.
    fn hold(&mut self) -> Result<()> {
        Ok(())
    }
    /// Gives the lock of [`Transport::hold`] back. Calling it without a hold does nothing.
    fn release(&mut self) {}
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test contract`
Expected: PASS: `test result: ok. 7 passed`.

Run: `cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: every test result ok (the fakes compile unchanged); no clippy output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/contract/camera.rs crates/obsbot-device/src/contract/transport.rs crates/obsbot-device/tests/contract.rs
git commit -m "Contract change: query and buffer frames, Transport::hold and release"
```

### Task A-3: Replay transport

**Files:**

- Create: `crates/obsbot-device/src/transport/replay.rs`
- Modify: `crates/obsbot-device/src/transport/mod.rs` (one line)
- Test: unit tests inside `replay.rs`

**Interfaces:**

- Consumes: W0 `contract::{Transport, CameraError, ControlRange, OnAir}`, `contract::camera::Result`; stage G `family::frame::{FRAME_SIZE, crc16_usb}` (and in the tests `frame`, `query_frame`, `query_frame_with`, `parse_answer`, `Answer`).
- Produces (module `obsbot_device::transport::replay`): `enum Sent { Xu { selector: u8, data: Vec<u8> }, Ctrl { id: u32, value: i32 } }`; `enum Step { Hold, Release, Set(u8), Get(u8) }`; `struct Log` with `sent() -> Vec<Sent>`, `steps() -> Vec<Step>` (every hold and release of the node lock and every extension-unit write and read, in order, for the tests of the lock), `clear()`; `fn answer_frame(kind: u8, seq: u16, receiver: u8, command: u16, code: Option<i16>, data: &[u8]) -> [u8; 60]`; `struct Replay` with `new(framed: u8)`, builders `answer(receiver, command, data)`, `answer_to(receiver, command, request, data)`, `reject(receiver, command, code)`, `empty(receiver, command)`, `buffer(selector, [u8; 60])`, `reads(selector, &[[u8; 60]])`, `ctrl(id, value)`, `range(id, ControlRange)`, `usb(Usb)`, `serial(&str)`, `on_air(OnAir)`, `gone()`, `log() -> Log`, and `impl Transport` (its `hold` and `release` only log a step). A-2, A-6 and every model test use it.

- [ ] **Step 1: Add the module**

In `crates/obsbot-device/src/transport/mod.rs`, add `pub mod replay;` next to the existing module line.

- [ ] **Step 2: Write the failing tests**

Create `crates/obsbot-device/src/transport/replay.rs` with only its `#[cfg(test)] mod tests` from Step 4 (and the licence line above it).

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --lib transport::replay`
Expected: FAIL to compile: `cannot find type Replay`, `cannot find function answer_frame`.

- [ ] **Step 4: Write the replay transport**

Replace `crates/obsbot-device/src/transport/replay.rs` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The replay transport (design spec 3.1, 11): it answers from recorded exchanges, so that a
//! model's encoders, decoders and flows run in tests without a camera. It sends nothing
//! anywhere and opens no file.
//!
//! - A query written to the framed selector is answered from the table of recorded answers,
//!   by receiver, command and request: the next read of that selector returns the answer with
//!   the query's sequence number. A query without a recorded answer leaves the buffer as it
//!   was, so the model sees no answer, as from a camera that does not answer.
//! - A read of another selector returns its recorded buffer (the status block), or the next of
//!   its recorded reads (the entries of a list), in order.
//! - A UVC control reads its recorded value; a write stores the new value, as V4L2 keeps the
//!   last value written.
//! - Every write (extension-unit buffer or UVC control) is appended to the log, in order.
//! - Every hold and release of the node lock and every extension-unit read and write is
//!   appended to the log's steps, in order, for the tests of the lock (5.8).

use crate::contract::camera::Result;
use crate::contract::{CameraError, ControlRange, OnAir, Transport};
use crate::family::frame::{FRAME_SIZE, crc16_usb};
use obsbot_proto::Usb;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

/// One write the transport received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    /// An extension-unit SET_CUR: the selector and the bytes as given.
    Xu { selector: u8, data: Vec<u8> },
    /// A UVC control write.
    Ctrl { id: u32, value: i32 },
}

/// One step of the transport, for the tests of the node lock (5.8): a hold or a release of
/// the lock, or an extension-unit write or read on a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Hold,
    Release,
    Set(u8),
    Get(u8),
}

/// The log of a replay transport, kept by a test after the transport moved into a model.
#[derive(Debug, Clone, Default)]
pub struct Log {
    sent: Arc<Mutex<Vec<Sent>>>,
    steps: Arc<Mutex<Vec<Step>>>,
}

impl Log {
    /// Everything written so far, in order.
    pub fn sent(&self) -> Vec<Sent> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Every hold, release, write and read so far, in order.
    pub fn steps(&self) -> Vec<Step> {
        self.steps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Forgets what was written and done so far.
    pub fn clear(&self) {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.steps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    fn push(&self, sent: Sent) {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(sent);
    }

    fn step(&self, step: Step) {
        self.steps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(step);
    }
}

#[derive(Debug, Clone)]
enum Reply {
    Value(Vec<u8>),
    Rejected(i16),
    Empty,
}

/// An answer frame in the layout the camera's answers have in the recordings: `aa`, the type
/// (`0x29` value, `0x89` rejected, `0x09` empty), the query's sequence number, the header
/// length, the header CRC (CRC-16/USB over the header with the CRC field zero), the query's
/// receiver at 8 and `0x0a` at 9, the command echoed at 10..12. A value has a 12-byte header, its length at 12..14, the data CRC at 14..16
/// (CRC-16/USB over bytes 12 to the end of the data with that field zero) and the data from 16
/// (captures 04, 08, 11). An empty answer has a 12-byte header and nothing after it (capture
/// 11). A rejection has a 16-byte header whose extension `00 02` and i16 code fill 12..16
/// (capture 01).
///
/// # Panics
/// If `data` is longer than the 44 bytes after the headers.
pub fn answer_frame(
    kind: u8,
    seq: u16,
    receiver: u8,
    command: u16,
    code: Option<i16>,
    data: &[u8],
) -> [u8; FRAME_SIZE] {
    assert!(
        data.len() <= FRAME_SIZE - 16,
        "an answer holds at most 44 bytes"
    );
    let mut r = [0u8; FRAME_SIZE];
    r[0] = 0xaa;
    r[1] = kind;
    r[2..4].copy_from_slice(&seq.to_le_bytes());
    r[8] = receiver;
    r[9] = 0x0a;
    r[10..12].copy_from_slice(&command.to_le_bytes());
    let header = match code {
        Some(code) => {
            r[12..14].copy_from_slice(&[0x00, 0x02]);
            r[14..16].copy_from_slice(&code.to_le_bytes());
            16
        }
        None => {
            if kind != 0x09 {
                r[12..14].copy_from_slice(&(data.len() as u16).to_le_bytes());
                r[16..16 + data.len()].copy_from_slice(data);
                let crc = crc16_usb(&r[12..16 + data.len()]);
                r[14..16].copy_from_slice(&crc.to_le_bytes());
            }
            12
        }
    };
    r[4..6].copy_from_slice(&(header as u16).to_le_bytes());
    let crc = crc16_usb(&r[..header]);
    r[6..8].copy_from_slice(&crc.to_le_bytes());
    r
}

/// The replay transport. Build it with the recorded answers, then hand it to a model.
#[derive(Debug)]
pub struct Replay {
    framed: u8,
    answers: BTreeMap<(u8, u16, Vec<u8>), Reply>,
    buffers: BTreeMap<u8, [u8; FRAME_SIZE]>,
    reads: BTreeMap<u8, VecDeque<[u8; FRAME_SIZE]>>,
    ctrls: BTreeMap<u32, i32>,
    ranges: BTreeMap<u32, ControlRange>,
    usb: Option<Usb>,
    serial: Option<String>,
    on_air: OnAir,
    pending: [u8; FRAME_SIZE],
    gone: bool,
    log: Log,
}

impl Replay {
    /// A replay whose framed commands, queries and answers use `framed` (2 on the Tiny 2 Lite).
    pub fn new(framed: u8) -> Replay {
        Replay {
            framed,
            answers: BTreeMap::new(),
            buffers: BTreeMap::new(),
            reads: BTreeMap::new(),
            ctrls: BTreeMap::new(),
            ranges: BTreeMap::new(),
            usb: None,
            serial: None,
            on_air: OnAir::default(),
            pending: [0; FRAME_SIZE],
            gone: false,
            log: Log::default(),
        }
    }

    /// The recorded value answer to `command` on `receiver` (a query without a request).
    pub fn answer(self, receiver: u8, command: u16, data: &[u8]) -> Replay {
        self.answer_to(receiver, command, &[], data)
    }

    /// The recorded value answer to `command` on `receiver` for one request (a view id).
    pub fn answer_to(mut self, receiver: u8, command: u16, request: &[u8], data: &[u8]) -> Replay {
        self.answers.insert(
            (receiver, command, request.to_vec()),
            Reply::Value(data.to_vec()),
        );
        self
    }

    /// A recorded refusal (type `0x89`) with the camera's error code.
    pub fn reject(mut self, receiver: u8, command: u16, code: i16) -> Replay {
        self.answers
            .insert((receiver, command, Vec::new()), Reply::Rejected(code));
        self
    }

    /// A recorded answer without data (type `0x09`: nothing to list).
    pub fn empty(mut self, receiver: u8, command: u16) -> Replay {
        self.answers
            .insert((receiver, command, Vec::new()), Reply::Empty);
        self
    }

    /// What every read of `selector` returns (the status block).
    pub fn buffer(mut self, selector: u8, block: [u8; FRAME_SIZE]) -> Replay {
        self.buffers.insert(selector, block);
        self
    }

    /// Successive reads of `selector` (the entries of the preset list), before its buffer.
    pub fn reads(mut self, selector: u8, blocks: &[[u8; FRAME_SIZE]]) -> Replay {
        self.reads
            .entry(selector)
            .or_default()
            .extend(blocks.iter().copied());
        self
    }

    pub fn ctrl(mut self, id: u32, value: i32) -> Replay {
        self.ctrls.insert(id, value);
        self
    }

    pub fn range(mut self, id: u32, range: ControlRange) -> Replay {
        self.ranges.insert(id, range);
        self
    }

    pub fn usb(mut self, usb: Usb) -> Replay {
        self.usb = Some(usb);
        self
    }

    pub fn serial(mut self, serial: &str) -> Replay {
        self.serial = Some(serial.to_string());
        self
    }

    pub fn on_air(mut self, on_air: OnAir) -> Replay {
        self.on_air = on_air;
        self
    }

    /// A device that is gone: every call fails with `Gone`.
    pub fn gone(mut self) -> Replay {
        self.gone = true;
        self
    }

    /// The log of every write, shared with this transport.
    pub fn log(&self) -> Log {
        self.log.clone()
    }

    fn here(&self) -> Result<()> {
        if self.gone {
            Err(CameraError::Gone)
        } else {
            Ok(())
        }
    }

    /// A query frame (`aa 01` or `aa 21`) written to the framed selector: prepares its answer.
    fn query(&mut self, data: &[u8]) {
        if data.len() < 16 || data[0] != 0xaa || !matches!(data[1], 0x01 | 0x21) {
            return;
        }
        let seq = u16::from_le_bytes([data[2], data[3]]);
        let receiver = data[9];
        let command = u16::from_le_bytes([data[10], data[11]]);
        let request = if data[1] == 0x21 {
            let len = usize::from(u16::from_le_bytes([data[12], data[13]]));
            data.get(16..16 + len).unwrap_or(&[]).to_vec()
        } else {
            Vec::new()
        };
        if let Some(reply) = self.answers.get(&(receiver, command, request)) {
            self.pending = match reply {
                Reply::Value(d) => answer_frame(0x29, seq, receiver, command, None, d),
                Reply::Rejected(code) => {
                    answer_frame(0x89, seq, receiver, command, Some(*code), &[])
                }
                Reply::Empty => answer_frame(0x09, seq, receiver, command, None, &[]),
            };
        }
    }
}

impl Transport for Replay {
    fn xu_len(&mut self, _selector: u8) -> Result<u16> {
        self.here()?;
        Ok(FRAME_SIZE as u16)
    }

    fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()> {
        self.here()?;
        self.log.step(Step::Get(selector));
        let block = if selector == self.framed {
            self.pending
        } else if let Some(next) = self.reads.get_mut(&selector).and_then(VecDeque::pop_front) {
            next
        } else if let Some(block) = self.buffers.get(&selector) {
            *block
        } else {
            return Err(CameraError::Io(format!(
                "selector {selector:#04x} is not recorded"
            )));
        };
        let n = buf.len().min(FRAME_SIZE);
        buf[..n].copy_from_slice(&block[..n]);
        Ok(())
    }

    fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()> {
        self.here()?;
        self.log.step(Step::Set(selector));
        self.log.push(Sent::Xu {
            selector,
            data: data.to_vec(),
        });
        if selector == self.framed {
            self.query(data);
        }
        Ok(())
    }

    fn ctrl_get(&mut self, id: u32) -> Result<i32> {
        self.here()?;
        self.ctrls
            .get(&id)
            .copied()
            .ok_or_else(|| CameraError::Io(format!("control {id:#010x} is not recorded")))
    }

    fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()> {
        self.here()?;
        self.log.push(Sent::Ctrl { id, value });
        self.ctrls.insert(id, value);
        Ok(())
    }

    fn ctrl_range(&mut self, id: u32) -> Result<Option<ControlRange>> {
        self.here()?;
        Ok(self.ranges.get(&id).copied())
    }

    fn usb(&mut self) -> Result<Usb> {
        self.here()?;
        self.usb.clone().ok_or(CameraError::Gone)
    }

    fn usb_serial(&mut self) -> Result<Option<String>> {
        self.here()?;
        Ok(self.serial.clone())
    }

    fn on_air(&mut self) -> Result<OnAir> {
        self.here()?;
        Ok(self.on_air.clone())
    }

    fn hold(&mut self) -> Result<()> {
        self.log.step(Step::Hold);
        Ok(())
    }

    fn release(&mut self) {
        self.log.step(Step::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::family::frame::{Answer, frame, parse_answer, query_frame, query_frame_with};

    fn read(t: &mut Replay, selector: u8) -> [u8; FRAME_SIZE] {
        let mut buf = [0u8; FRAME_SIZE];
        t.xu_get(selector, &mut buf).unwrap();
        buf
    }

    #[test]
    fn a_recorded_query_is_answered_with_its_sequence_number() {
        let mut t = Replay::new(2).answer(0x04, 0x0104, &[1, 2, 3]);
        t.xu_set(2, &query_frame(0x04, 0x0104, 0x0123)).unwrap();
        let reply = read(&mut t, 2);
        assert_eq!(
            parse_answer(&reply, 0x0123, 0x0104),
            Some(Answer::Value(&[1, 2, 3]))
        );
    }

    #[test]
    fn answers_are_kept_apart_by_receiver_and_request() {
        let mut t = Replay::new(2)
            .answer(0x0d, 0x0408, &[1, 8, 2, 6])
            .answer_to(0x04, 0x1504, &0u32.to_le_bytes(), b"Desk")
            .answer_to(0x04, 0x1504, &1u32.to_le_bytes(), b"Door");
        t.xu_set(2, &query_frame_with(0x04, 0x1504, &1u32.to_le_bytes(), 9))
            .unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 9, 0x1504),
            Some(Answer::Value(b"Door"))
        );
        // the same command on another receiver has no recorded answer: the buffer stays
        t.xu_set(2, &query_frame(0x04, 0x0408, 10)).unwrap();
        assert_eq!(parse_answer(&read(&mut t, 2), 10, 0x0408), None);
        t.xu_set(2, &query_frame(0x0d, 0x0408, 11)).unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 11, 0x0408),
            Some(Answer::Value(&[1, 8, 2, 6]))
        );
    }

    #[test]
    fn refusals_and_empty_answers_are_replayed_as_recorded() {
        let mut t = Replay::new(2).reject(0x04, 0x01c4, -3).empty(0x04, 0x1344);
        t.xu_set(2, &query_frame(0x04, 0x01c4, 1)).unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 1, 0x01c4),
            Some(Answer::Rejected(-3))
        );
        t.xu_set(2, &query_frame(0x04, 0x1344, 2)).unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 2, 0x1344),
            Some(Answer::Empty)
        );
    }

    #[test]
    fn a_command_is_logged_and_leaves_the_answer_buffer_alone() {
        let mut t = Replay::new(2).answer(0x04, 0x0104, &[5]);
        let log = t.log();
        t.xu_set(2, &query_frame(0x04, 0x0104, 1)).unwrap();
        let command = frame(0x04, 0x3b84, &[1], 2);
        t.xu_set(2, &command).unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 1, 0x0104),
            Some(Answer::Value(&[5])),
            "still the answer to the query"
        );
        assert_eq!(
            log.sent()[1],
            Sent::Xu {
                selector: 2,
                data: command.to_vec()
            }
        );
        assert_eq!(log.sent().len(), 2, "the query is a write too");
    }

    #[test]
    fn buffers_and_successive_reads() {
        let entry = |slot: u8| {
            let mut b = [0u8; FRAME_SIZE];
            b[1] = slot;
            b
        };
        let mut t = Replay::new(2)
            .buffer(6, [7; FRAME_SIZE])
            .reads(0x0d, &[entry(0), entry(2)]);
        assert_eq!(read(&mut t, 6), [7; FRAME_SIZE]);
        assert_eq!(
            read(&mut t, 6),
            [7; FRAME_SIZE],
            "a buffer reads the same each time"
        );
        assert_eq!(read(&mut t, 0x0d)[1], 0);
        assert_eq!(read(&mut t, 0x0d)[1], 2);
        let mut buf = [0u8; FRAME_SIZE];
        assert!(
            matches!(t.xu_get(0x0d, &mut buf), Err(CameraError::Io(_))),
            "no third entry was recorded"
        );
    }

    #[test]
    fn controls_keep_the_last_value_written() {
        let mut t = Replay::new(2).ctrl(0x0098_0900, 50);
        let log = t.log();
        assert_eq!(t.ctrl_get(0x0098_0900).unwrap(), 50);
        t.ctrl_set(0x0098_0900, 20).unwrap();
        assert_eq!(t.ctrl_get(0x0098_0900).unwrap(), 20);
        assert_eq!(
            log.sent(),
            [Sent::Ctrl {
                id: 0x0098_0900,
                value: 20
            }]
        );
        assert!(matches!(t.ctrl_get(0x0098_0901), Err(CameraError::Io(_))));
        assert_eq!(t.ctrl_range(0x0098_0900).unwrap(), None);
    }

    #[test]
    fn a_gone_device_fails_every_call_with_gone() {
        let mut t = Replay::new(2).gone();
        let mut buf = [0u8; FRAME_SIZE];
        assert_eq!(t.xu_get(6, &mut buf), Err(CameraError::Gone));
        assert_eq!(t.xu_set(6, &[1]), Err(CameraError::Gone));
        assert_eq!(t.ctrl_set(1, 1), Err(CameraError::Gone));
        assert!(t.log().sent().is_empty(), "nothing reached the log");
    }

    #[test]
    fn answer_frames_have_the_recorded_layout() {
        // capture 04: the answer to 0x6804 (zoom 2.77) with sequence 0x2f, byte for byte
        let recorded = "aa292f000c00966b040a046804003a9aae473140";
        let built = answer_frame(
            0x29,
            0x2f,
            0x04,
            0x6804,
            None,
            &hex::decode("ae473140").unwrap(),
        );
        assert_eq!(hex::encode(&built[..20]), recorded);
        assert!(built[20..].iter().all(|b| *b == 0));
        // capture 11: 0x4e44 (zone follow off) with sequence 0x0175
        let built = answer_frame(0x29, 0x0175, 0x04, 0x4e44, None, &[0]);
        assert_eq!(
            hex::encode(&built[..17]),
            "aa2975010c000880040a444e0100e63f00"
        );
        // capture 11: the empty answer to 0x1344, sequence 0x0176 (its header only)
        let built = answer_frame(0x09, 0x0176, 0x04, 0x1344, None, &[]);
        assert_eq!(hex::encode(&built[..12]), "aa0976010c0092b6040a4413");
        // capture 01: query 0xc442 to receiver 2 refused with code -3, sequence 4
        let built = answer_frame(0x89, 4, 0x02, 0xc442, Some(-3), &[]);
        assert_eq!(
            hex::encode(&built[..16]),
            "aa890400100046b7020a42c40002fdff"
        );
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --lib transport::replay`
Expected: PASS: `test result: ok. 8 passed`. `answer_frames_have_the_recorded_layout` shows the builder reproduces four recorded answers byte for byte (captures 01, 04, 11).

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot-device/src/transport/replay.rs crates/obsbot-device/src/transport/mod.rs
git commit -m "obsbot-device: replay transport answering from recorded exchanges"
```

### Task A-4: Tiny 2 Lite decoders

**Files:**

- Create: `crates/obsbot-device/src/models/tiny2lite/decode.rs`
- Modify: `crates/obsbot-device/src/models/tiny2lite/mod.rs` (one line)
- Test: unit tests inside `decode.rs`

**Interfaces:**

- Consumes: W0 `contract::OnAir`, `obsbot_proto::{Usb, Values}`, `obsbot_proto::values::{number, raw, rounded}`; stage G `models::tiny2lite::presets::{Pose, parse_pose, parse_zone_return, parse_hand_return, parse_slots, parse_entry, parse_view_name}`.
- Produces (module `obsbot_device::models::tiny2lite::decode`): the tables `MODE_PAIRS`, `PROFILE`, `SPEED`, `FRAMING_SIZE`, `ZOOM_SPEED`, `PART`, `FIELD_OF_VIEW`, `NOISE_REDUCTION`, `ANTI_FLICKER`, `REGION`, `SLEEP_BACKGROUND`, `HAND_SIDE`, `FOCUS_MODE`, `START_MODE` (`[(&str, u8); n]`), `UVC: [(&str, u32); 11]`, `ZOOM_ABSOLUTE`, `FOCUS_ABSOLUTE`, `POWER_LINE`; `fn named(&[(&str, u8)], u8) -> Value`, `fn byte_of(&[(&str, u8)], &str) -> Option<u8>`, `fn flag(u8) -> Value`, `fn mode([u8; 2]) -> Value`, `fn mode_pair(&str) -> Option<[u8; 2]>`, `fn pose_value(&Pose) -> Value`, `fn pitch_to_tilt_range((f64, f64)) -> (f64, f64)`, `fn tilt_to_pitch_range((f32, f32)) -> (f32, f32)`, and per group `status(&[u8; 60]) -> Values`, `gestures(&[u8]) -> Option<Values>`, `pose(&[u8]) -> Option<Values>`, `zoom_factor(&[u8]) -> Option<f64>`, `zoom(&[u8]) -> Option<Values>`, `target(&[u8]) -> Values`, `no_target() -> Values`, `framing(&str, &[u8]) -> Option<Value>`, `focus(&[u8]) -> Option<Values>`, `volume(&[u8]) -> Option<Value>`, `audio_enabled(&Usb) -> Value`, `uvc(&str, i32) -> Value`, `zone(&[u8]; 6 args) -> Option<Values>`, `hand(&[u8], &[u8]) -> Option<Values>`, `return_after(&[u8]) -> Option<Value>`, `power_on(&[u8], &[u8]) -> Option<Values>`, `view(u32, &[u8], &[u8]) -> Option<Value>`, `presets(&[u8], &[[u8; 60]]) -> Option<Values>`, `on_air(&OnAir) -> Values`, `firmware(&[u8]) -> Option<String>`, `serial(&[u8]) -> Option<String>`.

The fixtures are recorded single exchanges (8.5): status blocks read in captures 02, 05, 06, 07 and 09 (one setting changed per block), answers from captures 01, 04, 08 and 11, and answers measured on the camera; the serial is synthetic. The status block `BLOCK` and its single-byte changes are the recorded blocks: each recorded change differs from `BLOCK` in exactly the byte the test sets.

- [ ] **Step 1: Add the module**

In `crates/obsbot-device/src/models/tiny2lite/mod.rs`, add `pub mod decode;`.

- [ ] **Step 2: Write the failing tests**

Create `decode.rs` with the licence lines and the `#[cfg(test)] mod tests` of Step 4.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --lib tiny2lite::decode`
Expected: FAIL to compile: `cannot find function status`, `cannot find value MODE_PAIRS`, …

- [ ] **Step 4: Write the decoders**

Replace `crates/obsbot-device/src/models/tiny2lite/decode.rs` with (the pose, area, volume and target decoders are carried from `camera.rs` and `status.rs` as their doc comments say; what changes is the output: leaf values with `raw:<n>` where 0.13 fell back to a default or `None`):

```rust
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
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --lib tiny2lite::decode`
Expected: PASS: `test result: ok. 22 passed`.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot-device/src/models/tiny2lite/decode.rs crates/obsbot-device/src/models/tiny2lite/mod.rs
git commit -m "tiny2lite: decoders from recorded answers to leaf values"
```

### Task A-5: sysfs scan

**Files:**

- Create: `crates/obsbot-device/src/enumeration/sysfs.rs`, `crates/obsbot-device/tests/sysfs_tree/mod.rs`
- Modify: `crates/obsbot-device/src/enumeration.rs` (one line)
- Test: `crates/obsbot-device/tests/sysfs.rs`; unit tests inside `sysfs.rs`

**Interfaces:**

- Consumes: W0 `obsbot_proto::{Usb, ExtensionUnit}`.
- Produces (module `obsbot_device::enumeration::sysfs`): `struct UsbDevice { usb: Usb, devnum: u32, video_node: Option<String>, dir: PathBuf }`; `fn scan(root: &Path) -> Vec<UsbDevice>`; `fn usb_device(dir: &Path) -> Option<(Usb, u32)>`; `fn extension_units(&[u8]) -> Vec<ExtensionUnit>`; `fn guid(&[u8; 16]) -> String`. The test module `tests/sysfs_tree` (`root`, `write`, `descriptors`, `Device`, `plug`, `LITE`) is shared with A-13.

The extension-unit fixture is the 29-byte unit descriptor recorded in capture 01 (unit 2, 19 controls); it holds no serial. The test trees live under `CARGO_TARGET_TMPDIR`, never under `/tmp`.

- [ ] **Step 1: Add the module**

In `crates/obsbot-device/src/enumeration.rs`, add `pub mod sysfs;` below the `use` lines (the file stays a module with a submodule in `src/enumeration/sysfs.rs`).

- [ ] **Step 2: Write the failing tests**

Create `crates/obsbot-device/tests/sysfs_tree/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! A sysfs tree written by a test, laid out as the kernel lays it out:
//! `class/video4linux/videoN` with `index` and a `device` link to the USB interface, whose
//! parent is the USB device; `bus/usb/devices/<path>` links to the USB device.

#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh, empty directory under cargo's temporary directory for tests.
pub fn root() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "sysfs-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// The extension unit descriptor recorded in capture 01, after a video control interface.
pub fn descriptors() -> Vec<u8> {
    let mut d = hex::decode("09040000010e010000").unwrap();
    d.extend(hex::decode("1d24060291721e9a436883466d9239bc7906ee4913010304ffff3f0000").unwrap());
    d
}

pub struct Device<'a> {
    pub path: &'a str,
    pub vendor: &'a str,
    pub product: &'a str,
    pub devnum: u32,
    pub serial: bool,
    /// (interface number, class)
    pub interfaces: &'a [(u8, &'a str)],
    /// (videoN, index, interface number)
    pub nodes: &'a [(u32, u32, u8)],
}

pub fn plug(root: &Path, d: &Device) {
    let dev = root.join("devices/pci0000:00/usb3").join(d.path);
    write(&dev.join("idVendor"), &format!("{}\n", d.vendor));
    write(&dev.join("idProduct"), &format!("{}\n", d.product));
    write(&dev.join("bcdDevice"), "0510\n");
    write(&dev.join("product"), "OBSBOT Tiny 2 Lite\n");
    write(&dev.join("devnum"), &format!("{}\n", d.devnum));
    if d.serial {
        write(&dev.join("serial"), "EXAMPLE0000001\n");
    }
    fs::write(dev.join("descriptors"), descriptors()).unwrap();
    let devices = root.join("bus/usb/devices");
    fs::create_dir_all(&devices).unwrap();
    symlink(&dev, devices.join(d.path)).unwrap();
    for (number, class) in d.interfaces {
        let iface = dev.join(format!("{}:1.{number}", d.path));
        write(&iface.join("bInterfaceNumber"), &format!("{number:02x}\n"));
        write(&iface.join("bInterfaceClass"), &format!("{class}\n"));
    }
    for (n, index, number) in d.nodes {
        let iface = dev.join(format!("{}:1.{number}", d.path));
        let node = iface.join(format!("video4linux/video{n}"));
        write(&node.join("index"), &format!("{index}\n"));
        symlink(&iface, node.join("device")).unwrap();
        let class = root.join("class/video4linux");
        fs::create_dir_all(&class).unwrap();
        symlink(&node, class.join(format!("video{n}"))).unwrap();
    }
}

pub const LITE: Device = Device {
    path: "3-2",
    vendor: "3564",
    product: "FEF9",
    devnum: 7,
    serial: false,
    interfaces: &[(0, "0e"), (1, "0e"), (2, "01"), (3, "01")],
    nodes: &[(2, 0, 0), (3, 1, 0)],
};
```

Create `crates/obsbot-device/tests/sysfs.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The sysfs scan (design spec 5.3) on a sysfs tree written by the test, laid out as the kernel
//! lays it out: `class/video4linux/videoN` with `index` and a `device` link to the USB
//! interface, whose parent is the USB device. No device node is opened.

mod sysfs_tree;

use obsbot_device::enumeration::sysfs::{UsbDevice, scan};
use std::fs;
use std::os::unix::fs::symlink;
use sysfs_tree::*;

#[test]
fn one_usb_device_with_two_nodes_is_one_entry_with_its_facts() {
    let root = root();
    plug(&root, &LITE);
    let found = scan(&root);
    assert_eq!(found.len(), 1);
    let UsbDevice {
        usb,
        devnum,
        video_node,
        ..
    } = &found[0];
    assert_eq!(*devnum, 7);
    assert_eq!(
        video_node.as_deref(),
        Some("/dev/video2"),
        "the node with index 0"
    );
    assert_eq!(
        (usb.vendor.as_str(), usb.product.as_str()),
        ("3564", "fef9"),
        "lower case"
    );
    assert_eq!(usb.bcd_device, "0510");
    assert_eq!(usb.product_name.as_deref(), Some("OBSBOT Tiny 2 Lite"));
    assert_eq!(usb.path, "3-2");
    assert!(!usb.serial_present);
    assert_eq!(usb.interfaces, ["0e", "0e", "01", "01"]);
    assert_eq!(usb.extension_units.len(), 1);
    assert_eq!(usb.extension_units[0].unit, 2);
    assert_eq!(
        usb.extension_units[0].guid,
        "9a1e7291-6843-4683-6d92-39bc7906ee49"
    );
}

#[test]
fn devices_are_sorted_by_usb_path_and_the_serial_is_only_a_flag() {
    let root = root();
    plug(
        &root,
        &Device {
            path: "3-2.1",
            serial: true,
            nodes: &[(4, 0, 0)],
            ..LITE
        },
    );
    plug(&root, &LITE);
    let found = scan(&root);
    let paths: Vec<&str> = found.iter().map(|d| d.usb.path.as_str()).collect();
    assert_eq!(paths, ["3-2", "3-2.1"]);
    assert!(found[1].usb.serial_present);
    let json = serde_json::to_string(&found[1].usb).unwrap();
    assert!(
        !json.contains("EXAMPLE"),
        "the serial string never enters usb (4.2)"
    );
}

#[test]
fn a_device_without_an_index_0_node_has_no_video_node() {
    let root = root();
    plug(
        &root,
        &Device {
            nodes: &[(5, 1, 0)],
            ..LITE
        },
    );
    assert_eq!(scan(&root)[0].video_node, None);
}

#[test]
fn a_node_whose_device_is_no_usb_device_is_skipped() {
    let root = root();
    plug(&root, &LITE);
    // a platform camera: its device has no idVendor
    let platform = root.join("devices/platform/cam0");
    let node = platform.join("video4linux/video9");
    write(&node.join("index"), "0\n");
    fs::create_dir_all(platform.join("port")).unwrap();
    symlink(platform.join("port"), node.join("device")).unwrap();
    symlink(&node, root.join("class/video4linux/video9")).unwrap();
    let found = scan(&root);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].usb.path, "3-2");
}

#[test]
fn no_video4linux_class_is_no_device() {
    assert!(scan(&root()).is_empty());
}
```

and create `src/enumeration/sysfs.rs` with only the licence line and the `#[cfg(test)] mod tests` of Step 4.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --test sysfs`
Expected: FAIL to compile: `unresolved import obsbot_device::enumeration::sysfs::UsbDevice`.

- [ ] **Step 4: Write the scan**

Replace `crates/obsbot-device/src/enumeration/sysfs.rs` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The USB facts of every video device, from sysfs only (design spec 4.2 `usb`, 5.3): no
//! device node is opened, nothing is sent. The attributes read are a choice of track A,
//! confirmed on the camera by CAM-25:
//!
//! - per video node `class/video4linux/videoN`: `index`, and the `device` link to its USB
//!   interface, whose parent directory is the USB device;
//! - per USB device: `idVendor`, `idProduct`, `bcdDevice`, `product` (optional), `serial`
//!   (whether it exists; its text is read only by the transport, for a model whose identity
//!   source it is), `devnum`, the interfaces' `bInterfaceNumber` and `bInterfaceClass`, and
//!   `descriptors` (the raw USB descriptors, for the extension units).

use obsbot_proto::{ExtensionUnit, Usb};
use std::fs;
use std::path::{Path, PathBuf};

/// One USB device with at least one video node.
#[derive(Debug, Clone, PartialEq)]
pub struct UsbDevice {
    pub usb: Usb,
    /// The device number on its bus (5.3: with the path it identifies one enumeration).
    pub devnum: u32,
    /// `/dev/videoN` of the lowest-numbered node with index 0, if any.
    pub video_node: Option<String>,
    /// The USB device's directory in sysfs.
    pub dir: PathBuf,
}

fn attr(dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir.join(name))
        .ok()
        .map(|s| s.trim().to_string())
}

/// A GUID in its usual text form (lower case, 8-4-4-4-12) from the 16 bytes of a USB
/// descriptor: the first three fields are little-endian.
pub fn guid(b: &[u8; 16]) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_le_bytes([b[4], b[5]]),
        u16::from_le_bytes([b[6], b[7]]),
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// The UVC extension units in raw USB descriptors (the sysfs file `descriptors`): every
/// class-specific interface descriptor (type `0x24`) of subtype `0x06` inside a video control
/// interface (class `0x0e`, subclass `0x01`), as unit id, GUID and control count. Subtype
/// `0x06` means something else in an audio interface, so the interface is tracked.
pub fn extension_units(d: &[u8]) -> Vec<ExtensionUnit> {
    let mut units = Vec::new();
    let mut video_control = false;
    let mut i = 0;
    while i + 2 <= d.len() {
        let len = usize::from(d[i]);
        if len < 2 || i + len > d.len() {
            break;
        }
        let desc = &d[i..i + len];
        match desc[1] {
            0x04 if len >= 7 => video_control = desc[5] == 0x0e && desc[6] == 0x01,
            0x24 if video_control && len >= 21 && desc[2] == 0x06 => {
                let g: [u8; 16] = desc[4..20].try_into().expect("16 bytes");
                units.push(ExtensionUnit {
                    unit: desc[3],
                    guid: guid(&g),
                    controls: u32::from(desc[20]),
                });
            }
            _ => {}
        }
        i += len;
    }
    units
}

/// The facts of the USB device at `dir`, read now. `None` when it is not a USB device (no
/// `idVendor`) or is gone.
pub fn usb_device(dir: &Path) -> Option<(Usb, u32)> {
    let name = dir.file_name()?.to_str()?.to_string();
    let hex4 = |a: &str| attr(dir, a).map(|v| v.to_lowercase());
    let vendor = hex4("idVendor")?;
    let product = hex4("idProduct")?;
    let bcd_device = hex4("bcdDevice").unwrap_or_default();
    let devnum = attr(dir, "devnum")?.parse().ok()?;
    let mut interfaces: Vec<(u32, String)> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with(&format!("{name}:")))
        })
        .filter_map(|e| {
            let p = e.path();
            let number = u32::from_str_radix(&attr(&p, "bInterfaceNumber")?, 16).ok()?;
            Some((number, attr(&p, "bInterfaceClass")?.to_lowercase()))
        })
        .collect();
    interfaces.sort();
    let descriptors = fs::read(dir.join("descriptors")).unwrap_or_default();
    Some((
        Usb {
            vendor,
            product,
            product_name: attr(dir, "product"),
            bcd_device,
            path: name,
            serial_present: dir.join("serial").exists(),
            interfaces: interfaces.into_iter().map(|(_, c)| c).collect(),
            extension_units: extension_units(&descriptors),
        },
        devnum,
    ))
}

/// Every USB device with a video node under `root` (normally `/sys`), sorted by USB path. One
/// device is one entry, whatever number of video nodes it has (5.3).
pub fn scan(root: &Path) -> Vec<UsbDevice> {
    let Ok(nodes) = fs::read_dir(root.join("class/video4linux")) else {
        return Vec::new();
    };
    let mut nodes: Vec<(u32, PathBuf)> = nodes
        .flatten()
        .filter_map(|e| {
            let n = e
                .file_name()
                .to_str()?
                .strip_prefix("video")?
                .parse()
                .ok()?;
            Some((n, e.path()))
        })
        .collect();
    nodes.sort();
    let mut devices: Vec<UsbDevice> = Vec::new();
    for (n, node) in nodes {
        let Some(dir) = fs::canonicalize(node.join("device"))
            .ok()
            .and_then(|iface| iface.parent().map(Path::to_path_buf))
        else {
            continue;
        };
        let index0 = attr(&node, "index").as_deref() == Some("0");
        if let Some(known) = devices.iter_mut().find(|d| d.dir == dir) {
            if index0 && known.video_node.is_none() {
                known.video_node = Some(format!("/dev/video{n}"));
            }
            continue;
        }
        let Some((usb, devnum)) = usb_device(&dir) else {
            continue;
        };
        devices.push(UsbDevice {
            usb,
            devnum,
            video_node: index0.then(|| format!("/dev/video{n}")),
            dir,
        });
    }
    devices.sort_by(|a, b| a.usb.path.cmp(&b.usb.path));
    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The extension unit descriptor recorded in capture 01: unit 2, 19 controls.
    const XU: &str = "1d24060291721e9a436883466d9239bc7906ee4913010304ffff3f0000";

    #[test]
    fn the_recorded_extension_unit_is_found_by_its_guid() {
        // a video control interface (class 0e, subclass 01), then the unit
        let mut d = hex::decode("09040000010e010000").unwrap();
        d.extend(hex::decode(XU).unwrap());
        assert_eq!(
            extension_units(&d),
            [ExtensionUnit {
                unit: 2,
                guid: "9a1e7291-6843-4683-6d92-39bc7906ee49".into(),
                controls: 19,
            }]
        );
    }

    #[test]
    fn subtype_6_outside_video_control_is_no_extension_unit() {
        // the same bytes after an audio control interface (class 01, subclass 01)
        let mut d = hex::decode("090403000001010000").unwrap();
        d.extend(hex::decode(XU).unwrap());
        assert!(extension_units(&d).is_empty());
        // a truncated descriptor ends the walk without a panic
        let d = hex::decode("09040000010e0100001d2406").unwrap();
        assert!(extension_units(&d).is_empty());
        assert!(
            extension_units(&[0, 0, 0]).is_empty(),
            "a zero length ends the walk"
        );
    }

    #[test]
    fn guids_read_their_first_three_fields_little_endian() {
        let b: [u8; 16] = std::array::from_fn(|i| i as u8);
        assert_eq!(guid(&b), "03020100-0504-0706-0809-0a0b0c0d0e0f");
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test sysfs && cargo test -p obsbot-device --lib enumeration::sysfs`
Expected: PASS: `test result: ok. 5 passed` and `test result: ok. 3 passed`.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot-device/src/enumeration.rs crates/obsbot-device/src/enumeration/sysfs.rs crates/obsbot-device/tests/sysfs.rs crates/obsbot-device/tests/sysfs_tree
git commit -m "obsbot-device: sysfs scan of USB video devices, no node opened"
```

### Task A-2: Family link and answer checks

**Files:**

- Create: `crates/obsbot-device/src/family/link.rs`
- Modify: `crates/obsbot-device/src/family/frame.rs` (`parse_answer`, `Sequence::starting_at`, its test module), `family/mod.rs` (one line), `family/exchange.rs` (the test helper `reply`), `src/scripted.rs` (the helper `answer`)
- Test: unit tests inside `frame.rs` and `link.rs`

**Interfaces:**

- Consumes: A-1 `Transport::hold`, `release`; A-3 `transport::replay::answer_frame`; W0 `contract::{CameraError, Ctx, Transport}`, `contract::camera::Result`.
- Produces: `family::frame::parse_answer(reply, sequence, command)` with the checks of decision 4 (signature unchanged); `family::frame::Sequence::starting_at(first: u16) -> Sequence`; module `obsbot_device::family::link`: `const REPLY_GAP`, `REPLY_ATTEMPTS`, `ANSWER_POLL: (usize, Duration)` (the two under the name of 5.8), `FRAME_GAP`; `struct Timing { reply_gap, frame_gap }` with `Timing::CAMERA`, `Timing::instant()`; `fn process_sequence() -> &'static Sequence`; `fn poll(command: u16, seq: u16, gap: Duration, read: impl FnMut() -> Result<[u8; 60]>) -> Result<Vec<u8>>`; `struct Link` with `new(Box<dyn Transport>, &'static Sequence, Timing)`, `seq() -> u16`, `timing()`, `transport() -> &mut dyn Transport`, `short(selector, payload, ctx)`, `send(selector, frame, ctx)`, `send_spaced(selector, frame, ctx)`, `pair(selector, first, second, ctx)`, `query(selector, receiver, command, ctx) -> Result<Vec<u8>>`, `query_with(selector, receiver, command, request, ctx) -> Result<Vec<u8>>`, `read(selector, ctx) -> Result<[u8; 60]>`, `write_back(selector, block, ctx)`, `ctrl_get(id, ctx) -> Result<i32>`, `ctrl_set(id, value, ctx)`.

The carried `family::exchange` (the 0.13 `Camera` and its `poll_answer`) stays untouched until A-14; only its test helper builds answers in the recorded layout now, since `parse_answer` is shared.

- [ ] **Step 1: Write the failing tests**

In `crates/obsbot-device/src/family/frame.rs`, replace the whole `#[cfg(test)] mod tests` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_usb_check_value() {
        assert_eq!(crc16_usb(b"123456789"), 0xb4c8);
    }

    #[test]
    fn layout() {
        let f = frame(0x04, 0x3244, &[0x11, 0x22, 0x33], 0x1234);
        assert_eq!(f[0..2], [0xaa, 0x25]);
        assert_eq!(f[2..4], [0x34, 0x12], "sequence little endian");
        assert_eq!(f[10..12], [0x44, 0x32], "full 16 bit command id");
        assert_eq!(f[12..14], [0x03, 0x00]);
        assert_eq!(f[16..19], [0x11, 0x22, 0x33]);
        assert!(f[19..].iter().all(|b| *b == 0));
    }

    #[test]
    fn checksums_cover_header_and_data() {
        let f = frame(0x04, 0x30c4, &[0x01], 7);
        let mut h = f[0..12].to_vec();
        h[6..8].fill(0);
        assert_eq!(f[6..8], crc16_usb(&h).to_le_bytes());
        let mut d = f[12..17].to_vec();
        d[2..4].fill(0);
        assert_eq!(f[14..16], crc16_usb(&d).to_le_bytes());
    }

    #[test]
    #[should_panic]
    fn too_much_data_panics() {
        frame(0x04, 1, &[0u8; MAX_DATA + 1], 0);
    }

    #[test]
    fn query_matches_capture() {
        // Query 0x0104 recorded on the owner's device (capture 01, seq 7).
        let captured = hex::decode("aa0107000c00344e0a04040100000000").unwrap();
        let q = query_frame(0x04, 0x0104, 7);
        assert_eq!(q[..captured.len()], captured[..]);
        assert!(q[captured.len()..].iter().all(|b| *b == 0));
    }

    #[test]
    fn query_with_request_matches_capture() {
        // 0x1504 (name of zone view 0) recorded on the owner's device (capture 11, seq 450): type 0x21
        let captured = hex::decode("aa21c2010c00873d0a0404150400be0700000000").unwrap();
        let q = query_frame_with(0x04, 0x1504, &0u32.to_le_bytes(), 450);
        assert_eq!(q[..captured.len()], captured[..]);
        assert!(q[captured.len()..].iter().all(|b| *b == 0));
    }

    #[test]
    fn answer_without_data_section_is_empty_not_missing() {
        // Reply to the zone view list (0x1344, capture 11 seq 374) while no view is stored: type
        // 0x09, a 12-byte header and no data section; the bytes after the header are leftovers of
        // the previous read. It used to count as "no answer" and ran into the poll timeout.
        let mut r = [0u8; 60];
        let captured = hex::decode(concat!(
            "aa0976010c0092b6040a4413",
            "04006856",
            "0000c8c20101"
        ))
        .unwrap();
        r[..captured.len()].copy_from_slice(&captured);
        assert_eq!(parse_answer(&r, 374, 0x1344), Some(Answer::Empty));
        assert_eq!(parse_answer(&r, 375, 0x1344), None);
        // same sequence, but the reply is for another query: not an answer to this one
        assert_eq!(parse_answer(&r, 374, 0x1444), None);
    }

    use crate::transport::replay::answer_frame;

    /// A value answer in the recorded layout (12-byte header, data CRC).
    fn answer(seq: u16, command: u16, data: &[u8]) -> [u8; 60] {
        answer_frame(0x29, seq, 0x04, command, None, data)
    }

    #[test]
    fn recorded_value_answers_parse() {
        // capture 04 (0x6804, sequence 0x2f) and capture 11 (0x1504, sequence 0x01c2)
        let mut r = [0u8; 60];
        let rec = hex::decode("aa292f000c00966b040a046804003a9aae473140").unwrap();
        r[..rec.len()].copy_from_slice(&rec);
        assert_eq!(
            parse_answer(&r, 0x2f, 0x6804),
            Some(Answer::Value(&[0xae, 0x47, 0x31, 0x40]))
        );
        let mut r = [0u8; 60];
        let rec = hex::decode("aa29c2010c00ce76040a041508008f814265726569636831").unwrap();
        r[..rec.len()].copy_from_slice(&rec);
        assert_eq!(
            parse_answer(&r, 0x01c2, 0x1504),
            Some(Answer::Value(b"Bereich1"))
        );
    }

    #[test]
    fn an_answer_must_echo_sequence_and_command_and_carry_good_crcs() {
        let r = answer(7, 0x0104, &[1, 2, 3]);
        assert_eq!(parse_answer(&r, 7, 0x0104), Some(Answer::Value(&[1, 2, 3])));
        assert_eq!(parse_answer(&r, 8, 0x0104), None, "other sequence");
        assert_eq!(
            parse_answer(&r, 7, 0x0144),
            None,
            "the answer to another query with the same sequence number (#61)"
        );
        let mut bad = r;
        bad[6] ^= 0x01;
        assert_eq!(parse_answer(&bad, 7, 0x0104), None, "header CRC wrong");
        let mut bad = r;
        bad[9] ^= 0x01;
        assert_eq!(parse_answer(&bad, 7, 0x0104), None, "a header byte changed");
        let mut bad = r;
        bad[17] ^= 0x01;
        assert_eq!(parse_answer(&bad, 7, 0x0104), None, "a data byte changed");
        let mut bad = r;
        bad[14] ^= 0x01;
        assert_eq!(parse_answer(&bad, 7, 0x0104), None, "data CRC wrong");
        assert_eq!(parse_answer(&[0u8; 60], 7, 0), None, "not a frame");
    }

    #[test]
    fn a_value_answer_needs_the_12_byte_header_and_its_whole_data() {
        let mut r = answer(7, 0x0104, &[1, 2, 3]);
        r[4] = 16;
        r[6..8].fill(0);
        let crc = crc16_usb(&r[..16]);
        r[6..8].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(parse_answer(&r, 7, 0x0104), None, "16-byte header");
        let r = answer(7, 0x0104, &[0xab; 44]);
        assert!(
            parse_answer(&r, 7, 0x0104).is_some(),
            "44 bytes fill the frame"
        );
        assert_eq!(
            parse_answer(&r[..59], 7, 0x0104),
            None,
            "the data runs past the read"
        );
    }

    #[test]
    fn a_header_length_outside_12_to_16_is_no_answer() {
        for len in [0u16, 11, 17, 60] {
            let mut r = answer_frame(0x09, 7, 0x04, 0x1344, None, &[]);
            r[4..6].copy_from_slice(&len.to_le_bytes());
            r[6..8].fill(0);
            let crc = crc16_usb(&r[..usize::from(len).clamp(12, 16)]);
            r[6..8].copy_from_slice(&crc.to_le_bytes());
            assert_eq!(parse_answer(&r, 7, 0x1344), None, "header length {len}");
        }
    }

    #[test]
    fn rejected_answer_carries_the_code_from_the_header_extension() {
        // Captured rejection of query 0xc442 (capture 01): header length 16, extension
        // `00 02 fd ff` = -3. The bytes after it are not an error code: reading them from
        // offset 16 as before gave the wrong number.
        let mut r = [0u8; 60];
        let captured = hex::decode("aa890400100046b7020a42c40002fdff0000803f01010014").unwrap();
        r[..captured.len()].copy_from_slice(&captured);
        assert_eq!(parse_answer(&r, 4, 0xc442), Some(Answer::Rejected(-3)));
    }

    #[test]
    fn rejected_answer_without_extension_has_no_code() {
        // 12-byte header: nothing to read, and the data section of an error is ignored
        let mut r = answer(7, 0x0104, &[0x0a, 0x00]);
        r[1] = 0x89;
        r[6..8].fill(0);
        let crc = crc16_usb(&r[..12]);
        r[6..8].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(parse_answer(&r, 7, 0x0104), Some(Answer::Rejected(0)));
    }

    #[test]
    fn sequence_never_repeats_consecutively() {
        let s = Sequence::new();
        let mut prev = s.next();
        for _ in 0..70_000 {
            let cur = s.next();
            assert_eq!(cur, prev.wrapping_add(1));
            prev = cur;
        }
    }

    #[test]
    fn a_frame_holds_exactly_44_bytes_of_data() {
        // 60 bytes in all, 16 of them headers: 44 bytes of data fill the frame to its end
        assert_eq!(MAX_DATA, 44);
        let f = frame(0x04, 1, &[0xab; 44], 0);
        assert!(f[16..].iter().all(|b| *b == 0xab));
    }

    #[test]
    #[should_panic(expected = "must not exceed 44 bytes")]
    fn one_byte_past_the_data_section_is_refused() {
        frame(0x04, 1, &[0u8; 45], 0);
    }

    #[test]
    fn a_rejection_needs_a_16_byte_header_and_the_extension_tag() {
        // layout: header length at 4..6, the extension `00 02` at 12..14, the i16 code at 14..16
        let rejected = |header: u16, extension: [u8; 2]| {
            let mut r = [0u8; 60];
            r[0] = 0xaa;
            r[1] = 0x89;
            r[2..4].copy_from_slice(&7u16.to_le_bytes());
            r[4..6].copy_from_slice(&header.to_le_bytes());
            r[10..12].copy_from_slice(&0x0104u16.to_le_bytes());
            r[12..14].copy_from_slice(&extension);
            r[14..16].copy_from_slice(&(-3i16).to_le_bytes());
            let crc = crc16_usb(&r[..usize::from(header)]);
            r[6..8].copy_from_slice(&crc.to_le_bytes());
            parse_answer(&r, 7, 0x0104).map(|a| matches!(a, Answer::Rejected(-3)))
        };
        assert_eq!(rejected(16, [0x00, 0x02]), Some(true));
        assert_eq!(rejected(12, [0x00, 0x02]), Some(false), "12-byte header");
        assert_eq!(rejected(16, [0x01, 0x02]), Some(false), "another extension");
    }

    #[test]
    fn a_reply_must_hold_both_headers_to_be_an_answer() {
        let full = answer(7, 0x0104, &[]);
        assert_eq!(parse_answer(&full[..12], 7, 0x0104), None, "12 bytes");
        assert_eq!(parse_answer(&full[..15], 7, 0x0104), None, "one byte short");
        assert_eq!(
            parse_answer(&full[..16], 7, 0x0104),
            Some(Answer::Value(&[])),
            "both headers and no data"
        );
    }

    #[test]
    fn a_sequence_can_start_at_a_recorded_number() {
        let s = Sequence::starting_at(0x0186);
        assert_eq!(s.next(), 0x0186);
        assert_eq!(s.next(), 0x0187);
        let wrap = Sequence::starting_at(0);
        assert_eq!(wrap.next(), 0, "0 is a number like any other");
        assert_eq!(wrap.next(), 1);
    }
}
```

Add `pub mod link;` to `crates/obsbot-device/src/family/mod.rs` and create `crates/obsbot-device/src/family/link.rs` with the licence lines and the `#[cfg(test)] mod tests` of Step 3.

In `crates/obsbot-device/src/family/exchange.rs`, replace the test helper `reply` with:

```rust
    /// A selector-2 reply of the given type for `SEQ`, in the recorded layout.
    fn reply(kind: u8, command: u16, code: Option<i16>, data: &[u8]) -> [u8; XU_SIZE] {
        crate::transport::replay::answer_frame(kind, SEQ, 0x04, command, code, data)
    }
```

In `crates/obsbot-device/src/scripted.rs`, replace the function `answer` with:

```rust
/// An answer to the query frame `query`: same sequence number, receiver and command, in the
/// recorded layout.
fn answer(kind: u8, query: &[u8; XU_SIZE], code: Option<i16>, data: &[u8]) -> [u8; XU_SIZE] {
    let seq = u16::from_le_bytes([query[2], query[3]]);
    let command = u16::from_le_bytes([query[10], query[11]]);
    crate::transport::replay::answer_frame(kind, seq, query[9], command, code, data)
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --lib family`
Expected: FAIL to compile in `link.rs` (`cannot find type Link`, `Timing`, `process_sequence`) and `no function or associated item named starting_at found for struct Sequence`; once those exist, `an_answer_must_echo_sequence_and_command_and_carry_good_crcs` fails until Step 3's `parse_answer`.

- [ ] **Step 3: Write the answer checks and the link**

In `crates/obsbot-device/src/family/frame.rs`, replace `parse_answer` (and its doc comment) with:

```rust
/// Whether the header CRC of a reply is right: CRC-16/USB over the header (its length is the
/// word at bytes 4..6, 12 or 16 bytes) with the CRC field at bytes 6..8 taken as zero. The
/// recorded answers follow it (captures 01, 04, 08, 11).
fn header_crc_ok(reply: &[u8]) -> bool {
    let len = u16::from_le_bytes([reply[4], reply[5]]) as usize;
    if !(HEADER_SIZE..=DATA_OFFSET).contains(&len) || reply.len() < len {
        return false;
    }
    let mut header = [0u8; DATA_OFFSET];
    header[..len].copy_from_slice(&reply[..len]);
    header[6..8].fill(0);
    crc16_usb(&header[..len]) == u16::from_le_bytes([reply[6], reply[7]])
}

/// Whether the data CRC of a value answer is right: CRC-16/USB over bytes 12 to the end of the
/// data with the CRC field at 14..16 taken as zero, as in the frames this crate builds and in
/// the recorded value answers (captures 04, 08, 11), whose header is 12 bytes long.
fn data_crc_ok(reply: &[u8], len: usize) -> bool {
    let mut section = reply[HEADER_SIZE..DATA_OFFSET + len].to_vec();
    section[2..4].fill(0);
    crc16_usb(&section) == u16::from_le_bytes([reply[14], reply[15]])
}

/// Parses a selector-2 read; None when it is not the answer to the query with `sequence` and
/// `command` (yet). An answer must echo both and carry a good header CRC; a value answer also a
/// 12-byte header and a good data CRC (#61). So a reply to another process's query with the
/// same sequence number, or a damaged one, is not taken for ours.
pub fn parse_answer(reply: &[u8], sequence: u16, command: u16) -> Option<Answer<'_>> {
    if reply.len() < DATA_OFFSET
        || reply[0] != 0xaa
        || reply[2..4] != sequence.to_le_bytes()
        || reply[10..12] != command.to_le_bytes()
        || !header_crc_ok(reply)
    {
        return None;
    }
    match reply[1] {
        0x29 => {
            let len = u16::from_le_bytes([reply[12], reply[13]]) as usize;
            let header = u16::from_le_bytes([reply[4], reply[5]]) as usize;
            if header != HEADER_SIZE || DATA_OFFSET + len > reply.len() || !data_crc_ok(reply, len)
            {
                return None;
            }
            Some(Answer::Value(&reply[DATA_OFFSET..DATA_OFFSET + len]))
        }
        0x89 => Some(Answer::Rejected(error_code(reply))),
        0x09 => Some(Answer::Empty),
        _ => None,
    }
}
```

and add to `impl Sequence`, before `next`:

```rust
    /// A sequence whose first [`Sequence::next`] is `first`: for tests that compare frames with
    /// recorded ones at their recorded sequence numbers.
    pub fn starting_at(first: u16) -> Self {
        Self(AtomicU16::new(first.wrapping_sub(1)))
    }
```

Replace `crates/obsbot-device/src/family/link.rs` with (the poll is carried from `family/exchange.rs` `poll_answer`, with every read error ending it at once; the exchange methods are carried from `exchange::Camera`, with the selector passed in, the context checked and the lock held):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! One camera's link over its [`Transport`]: short commands, framed commands, queries and the
//! polling of their answers, reads and write-backs of whole selector buffers, UVC controls.
//! It knows no selector, receiver or command of a model: the model passes them in.
//!
//! Every exchange checks the operation's [`Ctx`] before its frame (a request stops at its
//! deadline, a group read stops when a stop arrives: 4.1 rule 9, 5.6) and holds the transport's
//! lock on the video node for the whole exchange, a query and the reads of its answer included
//! (5.8 `NODE_LOCK_WAIT`).

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx, Transport};
use crate::family::frame::{
    Answer, FRAME_SIZE, Sequence, parse_answer, query_frame, query_frame_with,
};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// `ANSWER_POLL` (5.8): the gap before each read of an answer.
pub const REPLY_GAP: Duration = Duration::from_millis(50);
/// `ANSWER_POLL` (5.8): reads of an answer before the query counts as unanswered.
pub const REPLY_ATTEMPTS: usize = 20;
/// `ANSWER_POLL` (5.8): [`REPLY_ATTEMPTS`] reads, [`REPLY_GAP`] apart, per query.
pub const ANSWER_POLL: (usize, Duration) = (REPLY_ATTEMPTS, REPLY_GAP);
/// `FRAME_GAP` (5.8): between two frames of one command where the model needs it (a pair).
pub const FRAME_GAP: Duration = Duration::from_millis(100);

/// The waits of a link. Tests use [`Timing::instant`]; the camera needs [`Timing::CAMERA`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    pub reply_gap: Duration,
    pub frame_gap: Duration,
}

impl Timing {
    pub const CAMERA: Timing = Timing {
        reply_gap: REPLY_GAP,
        frame_gap: FRAME_GAP,
    };

    /// No waiting: for tests on the replay transport.
    pub const fn instant() -> Timing {
        Timing {
            reply_gap: Duration::ZERO,
            frame_gap: Duration::ZERO,
        }
    }
}

/// The sequence counter of the process: every link of every camera takes its numbers from it,
/// so two frames in a row never repeat a number.
pub fn process_sequence() -> &'static Sequence {
    static SEQ: OnceLock<Sequence> = OnceLock::new();
    SEQ.get_or_init(Sequence::new)
}

/// Polls `read` for the answer to the query with `seq` and `command`: waits `gap` before each of
/// [`REPLY_ATTEMPTS`] reads. A read that fails ends the poll at once with its error (an unplug
/// is `Gone` at once, not "no answer" after a second); a read that is not the answer yet (a
/// stale buffer) is read again.
pub fn poll(
    command: u16,
    seq: u16,
    gap: Duration,
    mut read: impl FnMut() -> Result<[u8; FRAME_SIZE]>,
) -> Result<Vec<u8>> {
    for _ in 0..REPLY_ATTEMPTS {
        sleep(gap);
        let reply = read()?;
        match parse_answer(&reply, seq, command) {
            Some(Answer::Value(data)) => return Ok(data.to_vec()),
            Some(Answer::Empty) => return Ok(Vec::new()),
            Some(Answer::Rejected(code)) => {
                return Err(CameraError::Rejected(format!(
                    "query {command:#06x} (code {code})"
                )));
            }
            None => {}
        }
    }
    Err(CameraError::NoAnswer(format!("query {command:#06x}")))
}

/// The link of one camera. The model owns it; nothing else talks to the transport.
pub struct Link {
    transport: Box<dyn Transport>,
    seq: &'static Sequence,
    timing: Timing,
    /// When the last frame of [`Link::send_spaced`] went out.
    spaced: Option<Instant>,
}

impl Link {
    pub fn new(transport: Box<dyn Transport>, seq: &'static Sequence, timing: Timing) -> Link {
        Link {
            transport,
            seq,
            timing,
            spaced: None,
        }
    }

    /// The next sequence number, for a frame the model builds itself.
    pub fn seq(&self) -> u16 {
        self.seq.next()
    }

    pub fn timing(&self) -> Timing {
        self.timing
    }

    /// The transport, for the facts that are no exchange (`usb`, `usb_serial`, `on_air`).
    pub fn transport(&mut self) -> &mut dyn Transport {
        self.transport.as_mut()
    }

    /// Runs `f` under the transport's lock, after the context check.
    fn exchange<T>(
        &mut self,
        ctx: &Ctx,
        f: impl FnOnce(&mut dyn Transport, Timing) -> Result<T>,
    ) -> Result<T> {
        ctx.check()?;
        self.transport.hold()?;
        let result = f(self.transport.as_mut(), self.timing);
        self.transport.release();
        result
    }

    /// A short command: `payload` written to `selector`.
    pub fn short(&mut self, selector: u8, payload: &[u8], ctx: &Ctx) -> Result<()> {
        self.exchange(ctx, |t, _| t.xu_set(selector, payload))
    }

    /// One framed command written to `selector`.
    pub fn send(&mut self, selector: u8, frame: &[u8; FRAME_SIZE], ctx: &Ctx) -> Result<()> {
        self.exchange(ctx, |t, _| t.xu_set(selector, frame))
    }

    /// A framed command that keeps [`FRAME_GAP`] to the previous one sent this way: for commands
    /// that the camera drops when two follow each other back to back (the two framing offsets,
    /// which a patch may write one after the other).
    pub fn send_spaced(&mut self, selector: u8, frame: &[u8; FRAME_SIZE], ctx: &Ctx) -> Result<()> {
        if let Some(last) = self.spaced {
            let wait = self.timing.frame_gap.saturating_sub(last.elapsed());
            sleep(wait);
        }
        let sent = self.send(selector, frame, ctx);
        self.spaced = Some(Instant::now());
        sent
    }

    /// Two framed commands of one operation, [`FRAME_GAP`] apart: back to back the camera
    /// applies neither. The context is checked before each.
    pub fn pair(
        &mut self,
        selector: u8,
        first: &[u8; FRAME_SIZE],
        second: &[u8; FRAME_SIZE],
        ctx: &Ctx,
    ) -> Result<()> {
        self.send(selector, first, ctx)?;
        sleep(self.timing.frame_gap);
        self.send(selector, second, ctx)
    }

    /// A query without a request (frame type `0x01`) and its answer.
    pub fn query(
        &mut self,
        selector: u8,
        receiver: u8,
        command: u16,
        ctx: &Ctx,
    ) -> Result<Vec<u8>> {
        let seq = self.seq.next();
        let frame = query_frame(receiver, command, seq);
        self.ask(selector, &frame, command, seq, ctx)
    }

    /// A query with a request (frame type `0x21`, for example a view id) and its answer.
    pub fn query_with(
        &mut self,
        selector: u8,
        receiver: u8,
        command: u16,
        request: &[u8],
        ctx: &Ctx,
    ) -> Result<Vec<u8>> {
        let seq = self.seq.next();
        let frame = query_frame_with(receiver, command, request, seq);
        self.ask(selector, &frame, command, seq, ctx)
    }

    fn ask(
        &mut self,
        selector: u8,
        frame: &[u8; FRAME_SIZE],
        command: u16,
        seq: u16,
        ctx: &Ctx,
    ) -> Result<Vec<u8>> {
        self.exchange(ctx, |t, timing| {
            t.xu_set(selector, frame)?;
            poll(command, seq, timing.reply_gap, || {
                let mut buf = [0u8; FRAME_SIZE];
                t.xu_get(selector, &mut buf)?;
                Ok(buf)
            })
        })
    }

    /// One read of a selector's whole buffer (the status block, a preset list entry).
    pub fn read(&mut self, selector: u8, ctx: &Ctx) -> Result<[u8; FRAME_SIZE]> {
        self.exchange(ctx, |t, _| {
            let mut buf = [0u8; FRAME_SIZE];
            t.xu_get(selector, &mut buf)?;
            Ok(buf)
        })
    }

    /// A selector's buffer written back as it was read (the preset list rewind, 8.8).
    pub fn write_back(&mut self, selector: u8, block: &[u8; FRAME_SIZE], ctx: &Ctx) -> Result<()> {
        self.exchange(ctx, |t, _| t.xu_set(selector, block))
    }

    pub fn ctrl_get(&mut self, id: u32, ctx: &Ctx) -> Result<i32> {
        self.exchange(ctx, |t, _| t.ctrl_get(id))
    }

    pub fn ctrl_set(&mut self, id: u32, value: i32, ctx: &Ctx) -> Result<()> {
        self.exchange(ctx, |t, _| t.ctrl_set(id, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ControlRange, OnAir};
    use crate::family::frame::frame;
    use obsbot_proto::Usb;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    /// What the test transport saw, in order.
    #[derive(Debug, Clone, PartialEq)]
    enum Call {
        Hold,
        Release,
        Set(u8, Vec<u8>),
        Get(u8),
        CtrlGet(u32),
        CtrlSet(u32, i32),
    }

    /// A transport that logs every call and answers selector reads from a script: the n-th
    /// read gets `reads[n]` (the last entry repeats).
    struct Wire {
        log: Arc<Mutex<Vec<Call>>>,
        reads: Vec<Result<[u8; FRAME_SIZE]>>,
        n: usize,
        locked: bool,
    }

    impl Wire {
        fn new(reads: Vec<Result<[u8; FRAME_SIZE]>>) -> (Wire, Arc<Mutex<Vec<Call>>>) {
            let log = Arc::new(Mutex::new(Vec::new()));
            (
                Wire {
                    log: log.clone(),
                    reads,
                    n: 0,
                    locked: false,
                },
                log,
            )
        }
        fn push(&self, call: Call) {
            self.log.lock().unwrap().push(call);
        }
    }

    impl Transport for Wire {
        fn xu_len(&mut self, _selector: u8) -> Result<u16> {
            Ok(60)
        }
        fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()> {
            self.push(Call::Get(selector));
            let i = self.n.min(self.reads.len() - 1);
            self.n += 1;
            buf.copy_from_slice(&self.reads[i].clone()?);
            Ok(())
        }
        fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()> {
            assert!(self.locked, "every write happens under the lock");
            self.push(Call::Set(selector, data.to_vec()));
            Ok(())
        }
        fn ctrl_get(&mut self, id: u32) -> Result<i32> {
            self.push(Call::CtrlGet(id));
            Ok(7)
        }
        fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()> {
            self.push(Call::CtrlSet(id, value));
            Ok(())
        }
        fn ctrl_range(&mut self, _id: u32) -> Result<Option<ControlRange>> {
            Ok(None)
        }
        fn usb(&mut self) -> Result<Usb> {
            Err(CameraError::Gone)
        }
        fn usb_serial(&mut self) -> Result<Option<String>> {
            Ok(None)
        }
        fn on_air(&mut self) -> Result<OnAir> {
            Ok(OnAir::default())
        }
        fn hold(&mut self) -> Result<()> {
            self.locked = true;
            self.push(Call::Hold);
            Ok(())
        }
        fn release(&mut self) {
            self.locked = false;
            self.push(Call::Release);
        }
    }

    fn seq_at(first: u16) -> &'static Sequence {
        Box::leak(Box::new(Sequence::starting_at(first)))
    }

    /// An answer of `kind` to the query with `seq` and `command`, as the camera sends it.
    fn reply(kind: u8, seq: u16, command: u16, code: Option<i16>, data: &[u8]) -> [u8; 60] {
        crate::transport::replay::answer_frame(kind, seq, 0x04, command, code, data)
    }

    fn link(reads: Vec<Result<[u8; 60]>>, first: u16) -> (Link, Arc<Mutex<Vec<Call>>>) {
        let (wire, log) = Wire::new(reads);
        (
            Link::new(Box::new(wire), seq_at(first), Timing::instant()),
            log,
        )
    }

    fn gets(log: &Arc<Mutex<Vec<Call>>>) -> usize {
        log.lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c, Call::Get(_)))
            .count()
    }

    #[test]
    fn a_query_is_one_frame_and_the_reads_of_its_answer_under_one_hold() {
        let (mut link, log) = link(vec![Ok(reply(0x29, 7, 0x0104, None, &[1, 2, 3]))], 7);
        assert_eq!(
            link.query(2, 0x04, 0x0104, &Ctx::none()).unwrap(),
            [1, 2, 3]
        );
        assert_eq!(
            *log.lock().unwrap(),
            [
                Call::Hold,
                Call::Set(2, query_frame(0x04, 0x0104, 7).to_vec()),
                Call::Get(2),
                Call::Release
            ]
        );
    }

    #[test]
    fn a_stale_answer_is_read_again_until_the_answer_comes() {
        // the previous query's answer (sequence 6) is still in the buffer for two reads
        let stale = reply(0x29, 6, 0x0104, None, &[9]);
        let (mut link, log) = link(
            vec![Ok(stale), Ok(stale), Ok(reply(0x29, 7, 0x0104, None, &[4]))],
            7,
        );
        assert_eq!(link.query(2, 0x04, 0x0104, &Ctx::none()).unwrap(), [4]);
        assert_eq!(gets(&log), 3);
    }

    #[test]
    fn a_failed_read_ends_the_poll_at_once() {
        // an unplug while waiting for the answer is reported at the first failed read, not as
        // "no answer" after the whole poll (5.3, audit: the answer poll took every read error
        // for "not ready")
        let (mut link, log) = link(vec![Err(CameraError::Gone)], 7);
        assert_eq!(
            link.query(2, 0x04, 0x0104, &Ctx::none()),
            Err(CameraError::Gone)
        );
        assert_eq!(gets(&log), 1);
        assert_eq!(
            log.lock().unwrap().last(),
            Some(&Call::Release),
            "the lock is given back"
        );
    }

    #[test]
    fn a_rejected_query_returns_the_cameras_code_at_the_first_read() {
        let (mut link, log) = link(vec![Ok(reply(0x89, 7, 0x0104, Some(-3), &[]))], 7);
        assert_eq!(
            link.query(2, 0x04, 0x0104, &Ctx::none()),
            Err(CameraError::Rejected("query 0x0104 (code -3)".into()))
        );
        assert_eq!(gets(&log), 1);
    }

    #[test]
    fn an_empty_answer_is_an_empty_value() {
        let (mut link, _) = link(vec![Ok(reply(0x09, 7, 0x1344, None, &[]))], 7);
        assert_eq!(
            link.query(2, 0x04, 0x1344, &Ctx::none()).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn without_an_answer_the_query_gives_up_after_all_reads() {
        let (mut link, log) = link(vec![Ok([0u8; 60])], 7);
        assert_eq!(
            link.query(2, 0x04, 0x0104, &Ctx::none()),
            Err(CameraError::NoAnswer("query 0x0104".into()))
        );
        assert_eq!(gets(&log), REPLY_ATTEMPTS);
    }

    #[test]
    fn the_answer_to_another_query_with_our_number_is_not_ours() {
        // a process without the node lock asked 0x0144 with the same sequence number (#61)
        let (mut link, log) = link(vec![Ok(reply(0x29, 7, 0x0144, None, &[1]))], 7);
        assert!(matches!(
            link.query(2, 0x04, 0x0104, &Ctx::none()),
            Err(CameraError::NoAnswer(_))
        ));
        assert_eq!(gets(&log), REPLY_ATTEMPTS);
    }

    #[test]
    fn a_query_with_a_request_is_a_type_0x21_frame() {
        let (mut link, log) = link(vec![Ok(reply(0x29, 450, 0x1504, None, b"Desk"))], 450);
        assert_eq!(
            link.query_with(2, 0x04, 0x1504, &0u32.to_le_bytes(), &Ctx::none())
                .unwrap(),
            b"Desk"
        );
        assert_eq!(
            log.lock().unwrap()[1],
            Call::Set(
                2,
                query_frame_with(0x04, 0x1504, &[0, 0, 0, 0], 450).to_vec()
            )
        );
    }

    #[test]
    fn a_cut_context_sends_nothing_and_takes_no_lock() {
        let (mut link, log) = link(vec![Ok([0u8; 60])], 7);
        let stopped = Arc::new(AtomicBool::new(true));
        let ctx = Ctx::interruptible(stopped);
        assert_eq!(
            link.short(6, &[0x1a, 0x01, 0x02], &ctx),
            Err(CameraError::Cut)
        );
        assert_eq!(link.query(2, 0x04, 0x0104, &ctx), Err(CameraError::Cut));
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn a_stop_between_the_frames_of_a_pair_keeps_the_second_frame_back() {
        let (mut link, log) = link(vec![Ok([0u8; 60])], 7);
        let flag = Arc::new(AtomicBool::new(false));
        let ctx = Ctx::interruptible(flag.clone());
        let first = frame(0x04, 0x3984, &[1, 0, 0, 0], 1);
        let second = frame(0x04, 0x3d84, &[0], 2);
        link.send(2, &first, &ctx).unwrap();
        flag.store(true, Ordering::SeqCst);
        assert_eq!(link.send(2, &second, &ctx), Err(CameraError::Cut));
        let sets = log
            .lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c, Call::Set(..)))
            .count();
        assert_eq!(sets, 1);
    }

    #[test]
    fn the_two_frames_of_a_pair_are_a_frame_gap_apart() {
        let (wire, log) = Wire::new(vec![Ok([0u8; 60])]);
        let mut link = Link::new(
            Box::new(wire),
            seq_at(1),
            Timing {
                reply_gap: Duration::ZERO,
                frame_gap: Duration::from_millis(30),
            },
        );
        let a = frame(0x04, 0x3984, &[1, 0, 0, 0], 1);
        let b = frame(0x04, 0x3d84, &[0], 2);
        let start = Instant::now();
        link.pair(2, &a, &b, &Ctx::none()).unwrap();
        assert!(start.elapsed() >= Duration::from_millis(30));
        let sets: Vec<Call> = log
            .lock()
            .unwrap()
            .iter()
            .filter(|c| matches!(c, Call::Set(..)))
            .cloned()
            .collect();
        assert_eq!(sets, [Call::Set(2, a.to_vec()), Call::Set(2, b.to_vec())]);
    }

    #[test]
    fn spaced_frames_keep_the_frame_gap_between_them() {
        let (wire, log) = Wire::new(vec![Ok([0u8; 60])]);
        let mut link = Link::new(
            Box::new(wire),
            seq_at(1),
            Timing {
                reply_gap: Duration::ZERO,
                frame_gap: Duration::from_millis(30),
            },
        );
        let x = frame(0x04, 0x0bc4, &0.1f32.to_le_bytes(), 1);
        let y = frame(0x04, 0x0c44, &0.1f32.to_le_bytes(), 2);
        let start = Instant::now();
        link.send_spaced(2, &x, &Ctx::none()).unwrap();
        assert!(
            start.elapsed() < Duration::from_millis(30),
            "the first one does not wait"
        );
        link.send_spaced(2, &y, &Ctx::none()).unwrap();
        assert!(start.elapsed() >= Duration::from_millis(30));
        assert_eq!(
            log.lock()
                .unwrap()
                .iter()
                .filter(|c| matches!(c, Call::Set(..)))
                .count(),
            2
        );
    }

    #[test]
    fn controls_and_buffers_go_through_the_lock_too() {
        let block = [3u8; 60];
        let (mut link, log) = link(vec![Ok(block)], 7);
        assert_eq!(link.read(6, &Ctx::none()).unwrap(), block);
        link.write_back(0x0c, &block, &Ctx::none()).unwrap();
        assert_eq!(link.ctrl_get(0x0098_0900, &Ctx::none()).unwrap(), 7);
        link.ctrl_set(0x0098_0900, 5, &Ctx::none()).unwrap();
        assert_eq!(
            *log.lock().unwrap(),
            [
                Call::Hold,
                Call::Get(6),
                Call::Release,
                Call::Hold,
                Call::Set(0x0c, block.to_vec()),
                Call::Release,
                Call::Hold,
                Call::CtrlGet(0x0098_0900),
                Call::Release,
                Call::Hold,
                Call::CtrlSet(0x0098_0900, 5),
                Call::Release,
            ]
        );
    }

    #[test]
    fn one_counter_numbers_the_frames_of_the_process() {
        assert!(std::ptr::eq(process_sequence(), process_sequence()));
        let s = process_sequence();
        let a = s.next();
        assert_ne!(s.next(), a);
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --lib family`
Expected: PASS; `family::frame` 19 tests and `family::link` 14 tests among them, and the carried `family::exchange` tests.

Run: `cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: every test result ok (the byte guard unchanged); no clippy output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/family crates/obsbot-device/src/scripted.rs
git commit -m "family: link over Transport; answers checked by command and CRCs (#61); a read error ends the poll"
```

### Task A-6: Send-table check

**Files:**

- Create: `crates/obsbot-device/src/transport/checked.rs`
- Modify: `crates/obsbot-device/src/transport/mod.rs` (one line)
- Test: unit tests inside `checked.rs`

**Interfaces:**

- Consumes: A-1 `Frame::Query`, `Frame::Block`, `Transport::hold`, `release`; A-3 `Replay`, `Sent`, `Step` (tests); W0 `contract::{SendTable, SendEntry, Frame, Transport, CameraError}`.
- Produces (module `obsbot_device::transport::checked`): `struct Selectors { framed: u8, short: u8, all: &'static [u8] }`; `fn classify(&Selectors, selector: u8, data: &[u8]) -> Result<Frame, String>`; `fn describe(&Frame) -> String`; `struct Checked<T: Transport>` with `new(inner: T, table: SendTable, selectors: Selectors)`, `allows(&Frame) -> Result<(), String>`, and `impl<T: Transport> Transport for Checked<T>`.

- [ ] **Step 1: Add the module and write the failing tests**

Add `pub mod checked;` to `crates/obsbot-device/src/transport/mod.rs`; create `checked.rs` with the licence line and the `#[cfg(test)] mod tests` of Step 3.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --lib transport::checked`
Expected: FAIL to compile: `cannot find type Selectors`, `Checked`, `classify`.

- [ ] **Step 3: Write the check**

Replace `crates/obsbot-device/src/transport/checked.rs` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The send-table check (design spec 8.3): one wrapper around a model's transport that looks at
//! every outgoing write before it is sent. A write that is not in the model's send table is
//! refused with `CameraError::NotInSendTable` (the daemon answers `internal` and logs it) and
//! never reaches the camera. Independently of the table it refuses two whole classes: any
//! extension-unit selector the model's registry entry does not declare (reads included: no
//! probing), and on a receiver that answers identity queries every frame that is not one of the
//! model's identity queries.

use crate::contract::camera::Result;
use crate::contract::{CameraError, ControlRange, Frame, OnAir, SendTable, Transport};
use obsbot_proto::Usb;
use std::collections::BTreeSet;

/// Where a model's frames go: the selector of its framed commands and queries, the selector
/// of its short commands, and every selector it uses (its registry entry). A write to any other
/// declared selector is a whole-buffer write ([`Frame::Block`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selectors {
    pub framed: u8,
    pub short: u8,
    pub all: &'static [u8],
}

/// The frame a write is, by the selector it goes to and its bytes; `Err` with the reason when it
/// is no frame of a known kind.
pub fn classify(
    selectors: &Selectors,
    selector: u8,
    data: &[u8],
) -> std::result::Result<Frame, String> {
    if !selectors.all.contains(&selector) {
        return Err(format!(
            "selector {selector:#04x} is not a selector of this model"
        ));
    }
    if selector == selectors.framed {
        if data.len() < 12 || data[0] != 0xaa {
            return Err(format!(
                "{} bytes on selector {selector:#04x} that are no frame",
                data.len()
            ));
        }
        let receiver = data[9];
        let command = u16::from_le_bytes([data[10], data[11]]);
        return match data[1] {
            0x25 => Ok(Frame::Framed {
                selector,
                receiver,
                command,
            }),
            0x01 | 0x21 => Ok(Frame::Query {
                selector,
                receiver,
                command,
            }),
            kind => Err(format!(
                "frame type {kind:#04x} on selector {selector:#04x}"
            )),
        };
    }
    if selector == selectors.short {
        return match data.first() {
            Some(command) => Ok(Frame::Short {
                selector,
                command: *command,
            }),
            None => Err(format!(
                "an empty short command on selector {selector:#04x}"
            )),
        };
    }
    Ok(Frame::Block { selector })
}

/// A short description of a frame for error messages and logs.
pub fn describe(frame: &Frame) -> String {
    match frame {
        Frame::Framed {
            selector,
            receiver,
            command,
        } => {
            format!("framed {command:#06x} to receiver {receiver:#04x} on selector {selector:#04x}")
        }
        Frame::Query {
            selector,
            receiver,
            command,
        } => {
            format!("query {command:#06x} to receiver {receiver:#04x} on selector {selector:#04x}")
        }
        Frame::Short { selector, command } => {
            format!("short {command:#04x} on selector {selector:#04x}")
        }
        Frame::Uvc { control } => format!("UVC control {control:#010x}"),
        Frame::Block { selector } => format!("buffer write on selector {selector:#04x}"),
    }
}

/// A transport that sends only what the model's send table allows.
pub struct Checked<T: Transport> {
    inner: T,
    table: SendTable,
    selectors: Selectors,
    identity_receivers: BTreeSet<u8>,
}

impl<T: Transport> Checked<T> {
    pub fn new(inner: T, table: SendTable, selectors: Selectors) -> Checked<T> {
        let identity_receivers = table
            .entries
            .iter()
            .filter(|e| e.identity)
            .filter_map(|e| match e.frame {
                Frame::Query { receiver, .. } | Frame::Framed { receiver, .. } => Some(receiver),
                _ => None,
            })
            .collect();
        Checked {
            inner,
            table,
            selectors,
            identity_receivers,
        }
    }

    /// Whether `frame` may be sent: in the table, and not on an identity receiver unless it is
    /// one of the identity queries.
    pub fn allows(&self, frame: &Frame) -> std::result::Result<(), String> {
        let identity_receiver = match frame {
            Frame::Framed { receiver, .. } | Frame::Query { receiver, .. } => {
                self.identity_receivers.contains(receiver)
            }
            _ => false,
        };
        let entry = self.table.entries.iter().find(|e| e.frame == *frame);
        match entry {
            Some(e) if identity_receiver && !e.identity => Err(format!(
                "{}: the receiver answers identity queries and this is none",
                describe(frame)
            )),
            Some(_) => Ok(()),
            None if identity_receiver => Err(format!(
                "{}: the receiver answers identity queries and this is none",
                describe(frame)
            )),
            None => Err(format!("{} is not in the send table", describe(frame))),
        }
    }

    fn selector_declared(&self, selector: u8) -> Result<()> {
        if self.selectors.all.contains(&selector) {
            Ok(())
        } else {
            Err(CameraError::NotInSendTable(format!(
                "selector {selector:#04x} is not a selector of this model"
            )))
        }
    }
}

impl<T: Transport> Transport for Checked<T> {
    fn xu_len(&mut self, selector: u8) -> Result<u16> {
        self.selector_declared(selector)?;
        self.inner.xu_len(selector)
    }

    fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()> {
        self.selector_declared(selector)?;
        self.inner.xu_get(selector, buf)
    }

    fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()> {
        let frame =
            classify(&self.selectors, selector, data).map_err(CameraError::NotInSendTable)?;
        self.allows(&frame).map_err(CameraError::NotInSendTable)?;
        self.inner.xu_set(selector, data)
    }

    fn ctrl_get(&mut self, id: u32) -> Result<i32> {
        self.inner.ctrl_get(id)
    }

    fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()> {
        self.allows(&Frame::Uvc { control: id })
            .map_err(CameraError::NotInSendTable)?;
        self.inner.ctrl_set(id, value)
    }

    fn ctrl_range(&mut self, id: u32) -> Result<Option<ControlRange>> {
        self.inner.ctrl_range(id)
    }

    fn usb(&mut self) -> Result<Usb> {
        self.inner.usb()
    }

    fn usb_serial(&mut self) -> Result<Option<String>> {
        self.inner.usb_serial()
    }

    fn on_air(&mut self) -> Result<OnAir> {
        self.inner.on_air()
    }

    fn hold(&mut self) -> Result<()> {
        self.inner.hold()
    }

    fn release(&mut self) {
        self.inner.release()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::SendEntry;
    use crate::family::frame::{frame, query_frame};
    use crate::transport::replay::{Replay, Sent, Step};

    const SELECTORS: Selectors = Selectors {
        framed: 2,
        short: 6,
        all: &[2, 6, 0x0c, 0x0d],
    };

    fn entry(frame: Frame, identity: bool) -> SendEntry {
        SendEntry {
            frame,
            evidence: "capture 01",
            identity,
        }
    }

    fn table() -> SendTable {
        SendTable {
            entries: vec![
                entry(
                    Frame::Framed {
                        selector: 2,
                        receiver: 0x04,
                        command: 0x3b84,
                    },
                    false,
                ),
                entry(
                    Frame::Query {
                        selector: 2,
                        receiver: 0x04,
                        command: 0x0104,
                    },
                    false,
                ),
                entry(
                    Frame::Query {
                        selector: 2,
                        receiver: 0x0d,
                        command: 0x0408,
                    },
                    true,
                ),
                // a broken table entry: a command on the identity receiver
                entry(
                    Frame::Framed {
                        selector: 2,
                        receiver: 0x0d,
                        command: 0x0408,
                    },
                    false,
                ),
                entry(
                    Frame::Short {
                        selector: 6,
                        command: 0x1a,
                    },
                    false,
                ),
                entry(
                    Frame::Uvc {
                        control: 0x0098_0900,
                    },
                    false,
                ),
                entry(Frame::Block { selector: 0x0c }, false),
            ],
        }
    }

    fn checked() -> (Checked<Replay>, crate::transport::replay::Log) {
        let replay = Replay::new(2).buffer(0x0c, [1; 60]).buffer(0x07, [0; 60]);
        let log = replay.log();
        (Checked::new(replay, table(), SELECTORS), log)
    }

    #[test]
    fn what_the_table_names_goes_through() {
        let (mut t, log) = checked();
        t.xu_set(2, &frame(0x04, 0x3b84, &[1], 1)).unwrap();
        t.xu_set(2, &query_frame(0x04, 0x0104, 2)).unwrap();
        t.xu_set(2, &query_frame(0x0d, 0x0408, 3)).unwrap();
        t.xu_set(6, &[0x1a, 0x01, 0x02]).unwrap();
        t.xu_set(0x0c, &[1; 60]).unwrap();
        t.ctrl_set(0x0098_0900, 5).unwrap();
        assert_eq!(log.sent().len(), 6);
    }

    #[test]
    fn the_lock_reaches_the_inner_transport() {
        // the per-operation node lock (5.8) is the inner transport's: without these two the
        // trait's empty defaults would run every exchange of a model without the lock
        let (mut t, log) = checked();
        t.hold().unwrap();
        t.xu_set(2, &query_frame(0x04, 0x0104, 1)).unwrap();
        t.release();
        assert_eq!(log.steps(), [Step::Hold, Step::Set(2), Step::Release]);
    }

    #[test]
    fn anything_else_is_refused_and_never_sent() {
        let (mut t, log) = checked();
        let refused = |r: Result<()>| matches!(r, Err(CameraError::NotInSendTable(_)));
        assert!(
            refused(t.xu_set(2, &frame(0x04, 0x3b44, &[1], 1))),
            "another command id"
        );
        assert!(
            refused(t.xu_set(2, &frame(0x02, 0x3b84, &[1], 1))),
            "another receiver"
        );
        assert!(
            refused(t.xu_set(2, &frame(0x04, 0x0104, &[], 1))),
            "a command with the id of an allowed query"
        );
        assert!(
            refused(t.xu_set(2, &query_frame(0x04, 0x3b84, 1))),
            "a query with the id of an allowed command"
        );
        assert!(
            refused(t.xu_set(6, &[0x1b, 0x01, 0x00])),
            "another short command"
        );
        assert!(
            refused(t.xu_set(0x0d, &[0; 60])),
            "a buffer write on another selector"
        );
        assert!(refused(t.ctrl_set(0x0098_0901, 1)), "another control");
        assert!(
            refused(t.xu_set(2, &[0xaa, 0x99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
            "unknown type"
        );
        assert!(refused(t.xu_set(2, &[1, 2, 3])), "no frame");
        assert!(refused(t.xu_set(6, &[])), "an empty short command");
        assert!(log.sent().is_empty(), "nothing reached the transport");
    }

    #[test]
    fn an_undeclared_selector_is_refused_for_reads_too() {
        let (mut t, log) = checked();
        let mut buf = [0u8; 60];
        assert!(matches!(
            t.xu_get(0x07, &mut buf),
            Err(CameraError::NotInSendTable(_))
        ));
        assert!(matches!(
            t.xu_len(0x07),
            Err(CameraError::NotInSendTable(_))
        ));
        assert!(matches!(
            t.xu_set(0x07, &[0; 60]),
            Err(CameraError::NotInSendTable(_))
        ));
        assert!(
            t.xu_get(0x0c, &mut buf).is_ok(),
            "a declared selector reads"
        );
        assert!(log.sent().is_empty());
    }

    #[test]
    fn the_identity_receiver_takes_only_identity_queries() {
        let (mut t, log) = checked();
        // in the table, but a command on the receiver that answers identity queries (8.3)
        let r = t.xu_set(2, &frame(0x0d, 0x0408, &[0], 1));
        assert!(
            matches!(&r, Err(CameraError::NotInSendTable(m)) if m.contains("identity")),
            "{r:?}"
        );
        // not in the table and on that receiver: the identity rule names it too
        let r = t.xu_set(2, &query_frame(0x0d, 0x1948, 2));
        assert!(
            matches!(&r, Err(CameraError::NotInSendTable(m)) if m.contains("identity")),
            "{r:?}"
        );
        assert!(log.sent().is_empty());
        t.xu_set(2, &query_frame(0x0d, 0x0408, 3)).unwrap();
        assert_eq!(
            log.sent(),
            [Sent::Xu {
                selector: 2,
                data: query_frame(0x0d, 0x0408, 3).to_vec()
            }]
        );
    }

    #[test]
    fn classify_names_each_kind() {
        assert_eq!(
            classify(&SELECTORS, 2, &frame(0x04, 0x6444, &[0; 12], 1)),
            Ok(Frame::Framed {
                selector: 2,
                receiver: 0x04,
                command: 0x6444
            })
        );
        assert_eq!(
            classify(
                &SELECTORS,
                2,
                &crate::family::frame::query_frame_with(0x04, 0x1504, &[0; 4], 1)
            ),
            Ok(Frame::Query {
                selector: 2,
                receiver: 0x04,
                command: 0x1504
            })
        );
        assert_eq!(
            classify(&SELECTORS, 6, &[0x16, 0x02, 0x00, 0x00]),
            Ok(Frame::Short {
                selector: 6,
                command: 0x16
            })
        );
        assert_eq!(
            classify(&SELECTORS, 0x0c, &[0; 60]),
            Ok(Frame::Block { selector: 0x0c })
        );
        assert!(classify(&SELECTORS, 0x09, &[0; 60]).is_err());
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --lib transport::checked`
Expected: PASS: `test result: ok. 6 passed`. `the_lock_reaches_the_inner_transport` fails when `hold` or `release` is left out of `impl Transport for Checked` (the trait's empty defaults would then run every exchange without the lock).

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/transport/checked.rs crates/obsbot-device/src/transport/mod.rs
git commit -m "transport: the send-table check in front of a model's transport (8.3)"
```

### Task A-7: Tiny 2 Lite plan and send table

**Files:**

- Create: `crates/obsbot-device/src/models/tiny2lite/plan.rs`, `crates/obsbot-device/src/models/tiny2lite/send_table.rs`
- Modify: `crates/obsbot-device/src/models/tiny2lite/mod.rs` (two lines)
- Test: unit tests inside both files

**Interfaces:**

- Consumes: A-1 `Frame::Query`, `Frame::Block`; A-4 `decode::UVC`; W0 `contract::{Group, IdentitySource, LiveTier, ModelEntry, Motion, Target, Tier, UsbId, Frame, SendEntry, SendTable}`, `tiny2lite::description::description` (tests).
- Produces (module `obsbot_device::models::tiny2lite::plan`): `const FRAMED: u8 = 2`, `SHORT: u8 = 6`, `PRESET_LIST: u8 = 0x0c`, `PRESET_ENTRY: u8 = 0x0d`, `TRACKING: u8 = 0x04`, `POWER: u8 = 0x02`, `GIMBAL: u8 = 0x03`, `IDENTITY: u8 = 0x0d`, `XU_GUID: &str`, `ENTRY: ModelEntry`, `STATUS_LEAVES: [&str; 20]`, `GESTURE_LEAVES: [&str; 8]`, `FRAMING_QUERIES: [(&str, u16); 7]`, `FIRST: [&str; 5]`; `fn groups() -> Vec<Group>`, `fn touched(&[Group], Target) -> Vec<String>`, `fn order(&[&str]) -> Vec<String>`, `fn motion(&str) -> Motion`. Module `send_table`: `const ENTRIES: &[SendEntry]`, `fn table() -> SendTable`.

The registry entry's GUID is the extension unit of the USB descriptor recorded in capture 01. `support` stays `untested` (8.8: `tested` is W3's change).

- [ ] **Step 1: Add the modules and write the failing tests**

Add `pub mod plan;` and `pub mod send_table;` to `crates/obsbot-device/src/models/tiny2lite/mod.rs`; create both files with the licence line and the `#[cfg(test)] mod tests` of Step 3.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --lib tiny2lite::plan tiny2lite::send_table`
Expected: FAIL to compile: `cannot find function groups`, `cannot find value ENTRIES`.

- [ ] **Step 3: Write the plan and the send table**

Replace `crates/obsbot-device/src/models/tiny2lite/plan.rs` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite as data for the daemon (design spec 8.2, 8.8): its registry entry, its poll
//! plan, the groups each leaf and action touches, the apply order of a patch and the motion
//! marks of its actions. No I/O.

use crate::contract::{Group, IdentitySource, LiveTier, ModelEntry, Motion, Target, Tier, UsbId};
use obsbot_proto::Support;

/// The selector of framed commands, queries and their answers (8.8).
pub const FRAMED: u8 = 2;
/// The selector of short commands and the status block (8.8).
pub const SHORT: u8 = 6;
/// The selector of the preset list (count, slots), read and written back (8.8, CAM-19).
pub const PRESET_LIST: u8 = 0x0c;
/// The selector of the preset list's entries.
pub const PRESET_ENTRY: u8 = 0x0d;

/// Receivers of framed commands and queries (field ledger: receiver 4 unless named).
pub const TRACKING: u8 = 0x04;
pub const POWER: u8 = 0x02;
pub const GIMBAL: u8 = 0x03;
pub const IDENTITY: u8 = 0x0d;

/// The GUID of the extension unit; the USB descriptor recorded in capture 01 places it at
/// unit 2 with 19 controls.
pub const XU_GUID: &str = "9a1e7291-6843-4683-6d92-39bc7906ee49";

/// The registry entry (8.1, 8.2, 8.8). `support` is `untested` on `rebuild` until stage W3.
pub const ENTRY: ModelEntry = ModelEntry {
    key: "tiny2lite",
    display_name: "OBSBOT Tiny 2 Lite",
    usb: &[UsbId {
        vendor: 0x3564,
        product: 0xfef9,
    }],
    support: Support::Untested,
    identity: IdentitySource::Query,
    firmware_query: true,
    identity_tier: Tier::FiveSeconds,
    xu_guid: Some(XU_GUID),
    selectors: &[FRAMED, SHORT, PRESET_LIST, PRESET_ENTRY],
};

/// The leaves of the status block (group `status`), in block order.
pub const STATUS_LEAVES: [&str; 20] = [
    "asleep",
    "exposure.anti_flicker",
    "exposure.hdr",
    "exposure.region",
    "audio.noise_reduction",
    "sleep.auto.enabled",
    "sleep.auto.seconds",
    "focus.region",
    "audio.in_sleep",
    "zoom.field_of_view",
    "image.mirror",
    "sleep.wake_zoom",
    "tracking.mode",
    "audio.auto_gain",
    "sleep.background",
    "sleep.background_mirror",
    "image.frame_rate",
    "power_on.mode",
    "led.brightness",
    "tracking.profile",
];

/// The leaves of the `0x0104` answer (group `gestures`).
pub const GESTURE_LEAVES: [&str; 8] = [
    "hand.side",
    "gestures.target_lock",
    "gestures.zoom",
    "gestures.dynamic_zoom",
    "gestures.direction_change",
    "gestures.zoom_ratio",
    "zone.enabled",
    "gimbal.invert",
];

/// The leaves of the group `framing` and the query each reads back from.
pub const FRAMING_QUERIES: [(&str, u16); 7] = [
    ("tracking.speed", 0x0984),
    ("framing.auto_zoom", 0x0a84),
    ("framing.size", 0x0b04),
    ("framing.zoom_speed", 0x0b84),
    ("framing.offset.x", 0x0c04),
    ("framing.offset.y", 0x0c84),
    ("framing.part", 0x15c4),
];

fn group(name: &str, tier: Tier, asleep: bool, leaves: &[&str]) -> Group {
    Group {
        name: name.to_string(),
        tier,
        asleep,
        live: None,
        leaves: leaves.iter().map(|l| l.to_string()).collect(),
    }
}

/// The poll plan of 8.8. 1 s: the status block, the on-air probe and the target (read only
/// while the tracking mode is not `static`); the live pose at 5 s, at 1 s while a connection
/// subscribes `pose`; 5 s: the settings readbacks; 60 s: zone, hand (its return timer in a
/// group of its own), power-on, views;
/// `touched`: presets. Only the status block is read while the camera sleeps.
pub fn groups() -> Vec<Group> {
    let framing: Vec<&str> = FRAMING_QUERIES.iter().map(|(leaf, _)| *leaf).collect();
    let uvc: Vec<&str> = crate::models::tiny2lite::decode::UVC
        .iter()
        .map(|(leaf, _)| *leaf)
        .collect();
    vec![
        group("status", Tier::Second, true, &STATUS_LEAVES),
        group(
            "on_air",
            Tier::Second,
            false,
            &["on_air.active", "on_air.apps"],
        ),
        group(
            "target",
            Tier::Second,
            false,
            &["tracking.target.detected", "tracking.target.box"],
        ),
        Group {
            live: Some(LiveTier {
                topic: "pose".into(),
                tier: Tier::Second,
            }),
            ..group(
                "pose",
                Tier::FiveSeconds,
                false,
                &["gimbal.pan", "gimbal.tilt"],
            )
        },
        group("gestures", Tier::FiveSeconds, false, &GESTURE_LEAVES),
        group("zoom", Tier::FiveSeconds, false, &["zoom.factor"]),
        group("framing", Tier::FiveSeconds, false, &framing),
        group("uvc", Tier::FiveSeconds, false, &uvc),
        group(
            "focus",
            Tier::FiveSeconds,
            false,
            &["focus.mode", "focus.position"],
        ),
        group(
            "audio",
            Tier::FiveSeconds,
            false,
            &["audio.volume", "audio.enabled"],
        ),
        group(
            "zone",
            Tier::Minute,
            false,
            &[
                "zone.area.pan",
                "zone.area.tilt",
                "zone.follow",
                "zone.return",
            ],
        ),
        group(
            "hand",
            Tier::Minute,
            false,
            &[
                "hand.area.pan",
                "hand.area.tilt",
                "hand.axes.pan",
                "hand.axes.tilt",
                "hand.return",
            ],
        ),
        // its own group: the query is U-6, and an answer it does not get must not leave the
        // rest of the hand settings unread
        group("hand_timer", Tier::Minute, false, &["hand.return_after"]),
        group(
            "power_on",
            Tier::Minute,
            false,
            &[
                "power_on.start.source",
                "power_on.start.pose",
                "power_on.start.mode",
                "power_on.settings.tracking_speed",
                "power_on.settings.autofocus",
                "power_on.settings.auto_exposure",
                "power_on.settings.exposure_compensation",
                "power_on.settings.white_balance_mode",
                "power_on.settings.white_balance_temperature",
                "power_on.settings.contrast",
                "power_on.settings.saturation",
                "power_on.settings.sharpness",
                "power_on.settings.hue",
            ],
        ),
        group("views", Tier::Minute, false, &["views"]),
        group("presets", Tier::Touched, false, &["presets"]),
    ]
}

/// The groups to re-read after a leaf was written or an action ran (8.2, 8.8): a leaf's own
/// group, plus the groups the camera itself changes in response. `tracking.mode` rewrites
/// `framing.*`; `zoom.field_of_view` and the gesture settings change the zoom; a recall or a
/// view moves the gimbal and the zoom; the presets are read after every `preset.*` action and
/// `power_on.use_preset` (8.8). The moves, the stops and the go-to actions touch nothing: the
/// live pose shows the motion at its own tier.
pub fn touched(plan: &[Group], target: Target<'_>) -> Vec<String> {
    let own = |leaf: &str| -> Vec<String> {
        plan.iter()
            .filter(|g| g.leaves.iter().any(|l| l == leaf))
            .map(|g| g.name.clone())
            .collect()
    };
    let names = |list: &[&str]| list.iter().map(|g| g.to_string()).collect::<Vec<_>>();
    match target {
        Target::Leaf(leaf) => {
            let mut groups = own(leaf);
            let extra: &[&str] = match leaf {
                "tracking.mode" => &["framing"],
                "zoom.field_of_view" => &["zoom"],
                l if l.starts_with("gestures.") => &["zoom"],
                _ => &[],
            };
            groups.extend(names(extra));
            groups
        }
        Target::Action(action) => names(match action {
            "preset.save" | "preset.rename" | "preset.delete" => &["presets"],
            // 8.8: presets are read after every `preset.*` action and `power_on.use_preset`
            "preset.recall" => &["zoom", "pose", "presets"],
            "view.go" => &["zoom", "pose"],
            "view.save" | "view.rename" | "view.delete" => &["views"],
            "zone.return_here" => &["zone"],
            "hand.return_here" | "hand.return_clear" | "hand.area_reset" => &["hand"],
            "focus.point" => &["focus"],
            "power_on.set_pose" | "power_on.use_current" => &["power_on"],
            "power_on.use_preset" => &["power_on", "presets"],
            "sleep" | "wake" => &["status"],
            _ => &[],
        }),
    }
}

/// The leaves that go first in a patch (8.8): the `requires` paths of the description
/// (`exposure.mode`, `image.white_balance.auto`, `focus.mode`), `exposure.mode` also before
/// `exposure.gain`, `tracking.mode` before `framing.*`, and `sleep.auto.seconds` before
/// `sleep.auto.enabled`, so that switching auto-sleep on in the same patch uses the new time.
pub const FIRST: [&str; 5] = [
    "exposure.mode",
    "image.white_balance.auto",
    "focus.mode",
    "tracking.mode",
    "sleep.auto.seconds",
];

/// The apply order of a patch: the leaves of [`FIRST`], then the others, each part in the order
/// given.
pub fn order(leaves: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = leaves.iter().map(|l| l.to_string()).collect();
    out.sort_by_key(|l| !FIRST.contains(&l.as_str()));
    out
}

/// What an action is to the queue (4.1 rules 2 and 6, 5.6, 8.8).
pub fn motion(action: &str) -> Motion {
    match action {
        "gimbal.turn" => Motion::Continuing {
            stop: "gimbal.stop",
        },
        "gimbal.stop" | "zoom.stop" => Motion::Stop,
        "gimbal.move" | "zoom.move" | "preset.recall" | "view.go" | "zone.go_return"
        | "hand.go_return" | "power_on.go" => Motion::Move,
        _ => Motion::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tiny2lite::description::description;
    use std::collections::BTreeMap;

    #[test]
    fn every_leaf_with_a_readback_is_in_exactly_one_group() {
        let d = description();
        let mut seen: BTreeMap<String, String> = BTreeMap::new();
        for g in groups() {
            for leaf in &g.leaves {
                assert!(d.settings.contains_key(leaf), "{leaf} is not described");
                assert!(
                    seen.insert(leaf.clone(), g.name.clone()).is_none(),
                    "{leaf} twice"
                );
            }
        }
        for (path, s) in &d.settings {
            assert_eq!(seen.contains_key(path), s.readback, "{path}");
        }
        assert_eq!(seen.len(), 82);
    }

    #[test]
    fn the_tiers_of_8_8() {
        let tier = |name: &str| groups().into_iter().find(|g| g.name == name).unwrap();
        for name in ["status", "on_air", "target"] {
            assert_eq!(tier(name).tier, Tier::Second, "{name}");
        }
        for name in [
            "pose", "gestures", "zoom", "framing", "uvc", "focus", "audio",
        ] {
            assert_eq!(tier(name).tier, Tier::FiveSeconds, "{name}");
        }
        for name in ["zone", "hand", "hand_timer", "power_on", "views"] {
            assert_eq!(tier(name).tier, Tier::Minute, "{name}");
        }
        assert_eq!(tier("presets").tier, Tier::Touched);
        assert_eq!(
            tier("pose").live,
            Some(LiveTier {
                topic: "pose".into(),
                tier: Tier::Second
            })
        );
        let asleep: Vec<String> = groups()
            .into_iter()
            .filter(|g| g.asleep)
            .map(|g| g.name)
            .collect();
        assert_eq!(
            asleep,
            ["status"],
            "while asleep only the status block is read"
        );
    }

    #[test]
    fn touched_groups_of_8_8() {
        let plan = groups();
        let t = |target| touched(&plan, target);
        assert_eq!(t(Target::Leaf("tracking.mode")), ["status", "framing"]);
        assert_eq!(t(Target::Leaf("zoom.field_of_view")), ["status", "zoom"]);
        assert_eq!(t(Target::Leaf("gestures.zoom_ratio")), ["gestures", "zoom"]);
        assert_eq!(t(Target::Leaf("led.brightness")), ["status"]);
        assert_eq!(t(Target::Leaf("focus.position")), ["focus"]);
        assert_eq!(
            t(Target::Leaf("led.blink")),
            Vec::<String>::new(),
            "no readback"
        );
        assert_eq!(
            t(Target::Action("preset.recall")),
            ["zoom", "pose", "presets"]
        );
        assert_eq!(
            t(Target::Action("power_on.use_preset")),
            ["power_on", "presets"]
        );
        assert_eq!(t(Target::Action("view.go")), ["zoom", "pose"]);
        for quiet in [
            "gimbal.move",
            "gimbal.turn",
            "gimbal.stop",
            "zoom.move",
            "zoom.stop",
            "zone.go_return",
            "hand.go_return",
            "power_on.go",
            "factory_reset",
        ] {
            assert_eq!(
                t(Target::Action(quiet)),
                Vec::<String>::new(),
                "{quiet} touches nothing (8.8; spec 2: no frame of our own)"
            );
        }
    }

    #[test]
    fn requires_paths_go_first_and_the_rest_keeps_its_order() {
        assert_eq!(
            order(&["exposure.time", "led.brightness", "exposure.mode"]),
            ["exposure.mode", "exposure.time", "led.brightness"]
        );
        assert_eq!(
            order(&["framing.size", "tracking.mode"]),
            ["tracking.mode", "framing.size"]
        );
        assert_eq!(
            order(&["sleep.auto.enabled", "sleep.auto.seconds"]),
            ["sleep.auto.seconds", "sleep.auto.enabled"]
        );
        assert_eq!(
            order(&[
                "image.white_balance.temperature",
                "image.white_balance.auto"
            ]),
            [
                "image.white_balance.auto",
                "image.white_balance.temperature"
            ]
        );
        assert_eq!(
            order(&["focus.position", "focus.mode"]),
            ["focus.mode", "focus.position"]
        );
        assert_eq!(
            order(&["exposure.gain", "exposure.mode"]),
            ["exposure.mode", "exposure.gain"]
        );
    }

    #[test]
    fn motion_marks() {
        assert_eq!(
            motion("gimbal.turn"),
            Motion::Continuing {
                stop: "gimbal.stop"
            }
        );
        assert_eq!(motion("gimbal.stop"), Motion::Stop);
        assert_eq!(motion("zoom.stop"), Motion::Stop);
        for m in [
            "gimbal.move",
            "zoom.move",
            "preset.recall",
            "view.go",
            "zone.go_return",
            "hand.go_return",
            "power_on.go",
        ] {
            assert_eq!(motion(m), Motion::Move, "{m} (4.1 rule 6)");
        }
        assert_eq!(motion("preset.save"), Motion::None);
        assert_eq!(motion("sleep"), Motion::None);
    }

    #[test]
    fn the_registry_entry() {
        assert_eq!(ENTRY.key, "tiny2lite");
        assert_eq!(
            ENTRY.usb,
            [UsbId {
                vendor: 0x3564,
                product: 0xfef9
            }]
        );
        assert_eq!(
            ENTRY.support,
            Support::Untested,
            "tested only from W3 on (8.8)"
        );
        assert_eq!(ENTRY.identity, IdentitySource::Query);
        let firmware_query = ENTRY.firmware_query;
        assert!(firmware_query, "the firmware query of 8.8");
        assert_eq!(ENTRY.identity_tier, Tier::FiveSeconds);
        assert_eq!(ENTRY.selectors, [2, 6, 0x0c, 0x0d]);
    }
}
```

Replace `crates/obsbot-device/src/models/tiny2lite/send_table.rs` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The send table of the Tiny 2 Lite (design spec 8.3): every frame the model may write, each
//! with its evidence from the field ledger. Entries that only 0.13.2 sends (the U-rows of spec
//! 9.3 without any recording) say so until camera session CS-1 measures or removes them.

use crate::contract::{Frame, SendEntry, SendTable};
use crate::models::tiny2lite::plan::{
    FRAMED, GIMBAL, IDENTITY, POWER, PRESET_LIST, SHORT, TRACKING,
};

const fn short(command: u8, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Short {
            selector: SHORT,
            command,
        },
        evidence,
        identity: false,
    }
}

const fn framed(receiver: u8, command: u16, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Framed {
            selector: FRAMED,
            receiver,
            command,
        },
        evidence,
        identity: false,
    }
}

const fn query(receiver: u8, command: u16, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Query {
            selector: FRAMED,
            receiver,
            command,
        },
        evidence,
        identity: false,
    }
}

const fn identity(command: u16, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Query {
            selector: FRAMED,
            receiver: IDENTITY,
            command,
        },
        evidence,
        identity: true,
    }
}

const fn uvc(control: u32, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Uvc { control },
        evidence,
        identity: false,
    }
}

/// Every entry, grouped as the field ledger lists them.
pub const ENTRIES: &[SendEntry] = &[
    // identity (5.5, 8.8): the serial and the firmware query, nothing else on receiver 0x0d
    identity(0x18c8, "capture 01"),
    identity(0x0408, "capture 01"),
    // short commands (selector 6)
    short(0x04, "capture 09"),
    short(0x16, "capture 11"),
    short(0x03, "capture 12"),
    short(0x01, "capture 12"),
    short(0x14, "capture 05"),
    short(0x1c, "capture 14"),
    short(0x0a, "capture 06"),
    short(0x17, "capture 06"),
    short(0x13, "capture 06"),
    short(0x1a, "capture 07"),
    short(0x18, "capture 11, 18"),
    short(0x0b, "capture 02"),
    short(0x0e, "capture 03"),
    short(0x15, "measured 2026-10-31 (#97)"),
    short(0x19, "measured 2026-10-31 (#97)"),
    short(0x00, "measured 2026-10-31 (#79)"),
    // framed settings
    framed(TRACKING, 0x3b84, "capture 05, 17"),
    framed(TRACKING, 0x0cc4, "capture 11"),
    framed(TRACKING, 0x0944, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0a44, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0ac4, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0b44, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0bc4, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0c44, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x1584, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x30c4, "capture 04"),
    framed(TRACKING, 0x3144, "capture 04"),
    framed(TRACKING, 0x3244, "capture 04"),
    framed(TRACKING, 0x3344, "capture 04"),
    framed(TRACKING, 0x33c4, "capture 04"),
    framed(TRACKING, 0x08c4, "capture 11"),
    framed(TRACKING, 0x4e04, "capture 11, 18"),
    framed(TRACKING, 0x4c04, "capture 11"),
    framed(TRACKING, 0x4c84, "capture 11"),
    framed(TRACKING, 0x4d04, "capture 11"),
    framed(TRACKING, 0x4d84, "capture 11"),
    framed(TRACKING, 0x2244, "capture 11, 17"),
    framed(TRACKING, 0x2044, "capture 11"),
    framed(TRACKING, 0x20c4, "capture 11"),
    framed(TRACKING, 0x2144, "capture 11"),
    framed(TRACKING, 0x21c4, "capture 11"),
    framed(TRACKING, 0x2344, "0.13.2 only, U-5"),
    framed(TRACKING, 0x23c4, "0.13.2 only, U-5"),
    framed(TRACKING, 0x2444, "0.13.2 only, U-6"),
    framed(TRACKING, 0x26c4, "capture 11"),
    framed(POWER, 0x3442, "capture 12; measured 2026-10-31 (#97)"),
    framed(POWER, 0x3602, "capture 12"),
    framed(POWER, 0x8102, "measured 2026-10-31 (#97)"),
    // framed actions
    framed(TRACKING, 0x6444, "capture 18"),
    framed(TRACKING, 0x6544, "measured 2026-10-31 (#98); limits U-9"),
    framed(TRACKING, 0x6704, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x6484, "capture 10"),
    framed(TRACKING, 0x6884, "capture 11; speeds slow and fast U-12"),
    framed(TRACKING, 0x68c4, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x3944, "capture 10"),
    framed(TRACKING, 0x3e04, "capture 10"),
    framed(TRACKING, 0x3a84, "capture 10"),
    framed(TRACKING, 0x39c4, "capture 10"),
    framed(TRACKING, 0x3984, "capture 10"),
    framed(TRACKING, 0x3d84, "capture 10"),
    framed(TRACKING, 0x1384, "capture 11"),
    framed(TRACKING, 0x1544, "capture 11"),
    framed(TRACKING, 0x1604, "measured 2026-10-31 (#100)"),
    framed(TRACKING, 0x13c4, "capture 11"),
    framed(TRACKING, 0x4e84, "capture 11"),
    framed(TRACKING, 0x4f44, "measured 2026-10-25"),
    framed(TRACKING, 0x24c4, "capture 11"),
    framed(TRACKING, 0x2584, "measured 2026-10-25"),
    framed(TRACKING, 0x2544, "measured 2026-10-25"),
    framed(TRACKING, 0x25c4, "0.13.2 only, U-7"),
    framed(TRACKING, 0x2604, "0.13.2 only, U-7"),
    framed(TRACKING, 0x2644, "0.13.2 only, U-7"),
    framed(TRACKING, 0x2684, "0.13.2 only, U-7"),
    framed(POWER, 0x3582, "measured 2026-10-31 (#97); U-16"),
    framed(TRACKING, 0x3ec4, "capture 08"),
    framed(TRACKING, 0x3e44, "capture 08"),
    framed(POWER, 0xa0c2, "capture 13"),
    framed(POWER, 0xa802, "capture 16"),
    // queries (reads, but frames the model writes)
    query(GIMBAL, 0x0043, "measured 2026-10-31 (#67)"),
    query(TRACKING, 0x0104, "capture 01"),
    query(TRACKING, 0x6804, "capture 04, 11"),
    query(TRACKING, 0x01c4, "measured 2026-10-25"),
    query(TRACKING, 0x0984, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0a84, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0b04, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0b84, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0c04, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0c84, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x15c4, "measured 2026-10-31 (#98)"),
    query(POWER, 0x3642, "measured 2026-10-31 (#97)"),
    query(POWER, 0x80c2, "measured 2026-10-31 (#97)"),
    query(TRACKING, 0x4c44, "capture 11"),
    query(TRACKING, 0x4cc4, "capture 11"),
    query(TRACKING, 0x4d44, "capture 11"),
    query(TRACKING, 0x4dc4, "capture 11"),
    query(TRACKING, 0x4e44, "capture 11, 18"),
    query(TRACKING, 0x4ec4, "capture 11, 18"),
    query(TRACKING, 0x2004, "capture 11"),
    query(TRACKING, 0x2504, "capture 11"),
    query(TRACKING, 0x2484, "0.13.2 only, U-6"),
    query(TRACKING, 0x3884, "capture 08"),
    query(TRACKING, 0x3e84, "capture 08"),
    query(TRACKING, 0x1344, "capture 11, 18"),
    query(TRACKING, 0x1504, "capture 11"),
    query(TRACKING, 0x1444, "capture 11"),
    // UVC controls
    uvc(0x009a_090d, "capture 04, 11; measured 2026-10-24"),
    uvc(0x009a_0901, "measured 2026-10-31 (#97)"),
    uvc(0x009a_0902, "measured 2026-10-31 (#97)"),
    uvc(0x0098_0913, "capture 12"),
    uvc(0x0098_091c, "capture 08b, 12"),
    uvc(0x0098_0918, "capture 12"),
    uvc(0x0098_0900, "measured 2026-10-24"),
    uvc(0x0098_0901, "capture 08b, 12"),
    uvc(0x0098_0902, "capture 08b, 12"),
    uvc(0x0098_0903, "capture 08b, 12"),
    uvc(0x0098_091b, "capture 08b, 12"),
    uvc(0x0098_090c, "capture 08b, 12"),
    uvc(0x0098_091a, "capture 08b, 12"),
    uvc(0x009a_090a, "capture 12"),
    // the preset list rewind (8.8): the one known exception of section 2 until CAM-19
    SendEntry {
        frame: Frame::Block {
            selector: PRESET_LIST,
        },
        evidence: "measured 2026-10-31 (#104); CAM-19",
        identity: false,
    },
];

/// The send table.
pub fn table() -> SendTable {
    SendTable {
        entries: ENTRIES.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_frame_is_named_once_with_its_evidence() {
        let mut seen = BTreeSet::new();
        for e in ENTRIES {
            assert!(seen.insert(format!("{:?}", e.frame)), "{:?} twice", e.frame);
            assert!(
                e.evidence.starts_with("capture ")
                    || e.evidence.starts_with("measured ")
                    || e.evidence.starts_with("0.13.2 only, U-"),
                "{:?}: {}",
                e.frame,
                e.evidence
            );
        }
    }

    #[test]
    fn only_the_two_identity_queries_go_to_the_identity_receiver() {
        let on_identity: Vec<&SendEntry> = ENTRIES
            .iter()
            .filter(|e| {
                matches!(e.frame, Frame::Framed { receiver, .. } | Frame::Query { receiver, .. } if receiver == IDENTITY)
            })
            .collect();
        assert_eq!(on_identity.len(), 2);
        assert!(on_identity.iter().all(|e| e.identity));
        assert!(ENTRIES.iter().filter(|e| e.identity).count() == 2);
    }

    #[test]
    fn the_frames_only_0_13_2_sends_are_the_u_rows() {
        let only: BTreeSet<String> = ENTRIES
            .iter()
            .filter(|e| e.evidence.starts_with("0.13.2 only"))
            .map(|e| format!("{:?}", e.frame))
            .collect();
        let want: BTreeSet<String> = [
            (FRAMED, TRACKING, 0x2344u16, false),
            (FRAMED, TRACKING, 0x23c4, false),
            (FRAMED, TRACKING, 0x2444, false),
            (FRAMED, TRACKING, 0x2484, true),
            (FRAMED, TRACKING, 0x25c4, false),
            (FRAMED, TRACKING, 0x2604, false),
            (FRAMED, TRACKING, 0x2644, false),
            (FRAMED, TRACKING, 0x2684, false),
        ]
        .into_iter()
        .map(|(selector, receiver, command, is_query)| {
            format!(
                "{:?}",
                if is_query {
                    Frame::Query {
                        selector,
                        receiver,
                        command,
                    }
                } else {
                    Frame::Framed {
                        selector,
                        receiver,
                        command,
                    }
                }
            )
        })
        .collect();
        assert_eq!(only, want);
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --lib tiny2lite::plan && cargo test -p obsbot-device --lib tiny2lite::send_table`
Expected: PASS: `test result: ok. 6 passed` and `test result: ok. 3 passed`.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/models/tiny2lite/plan.rs crates/obsbot-device/src/models/tiny2lite/send_table.rs crates/obsbot-device/src/models/tiny2lite/mod.rs
git commit -m "tiny2lite: registry entry, poll plan, touched groups, apply order, send table"
```

### Task A-8: USB transport

**Files:**

- Create: `crates/obsbot-device/src/transport/usb.rs`
- Modify: `crates/obsbot-device/src/transport/mod.rs` (one line)
- Test: unit tests inside `usb.rs`

**Interfaces:**

- Consumes: A-1 `Transport::hold`, `release`; A-5 `enumeration::sysfs::usb_device`; stage G `family::streaming::{is_streaming, apps_using}`; W0 `contract::{Transport, CameraError, ControlRange, OnAir}`.
- Produces (module `obsbot_device::transport::usb`): `const NODE_LOCK_WAIT: Duration` (5 s); `fn from_errno(Errno) -> CameraError`; `fn lock_exclusive(File, Duration) -> Result<Flock<File>>`; `fn padded(selector: u8, len: usize, data: &[u8]) -> Result<Vec<u8>>`; `struct UsbTransport` with `new(node: impl Into<PathBuf>, unit: u8, usb_dir: impl Into<PathBuf>)` and `impl Transport` (`hold` takes the lock and keeps the node open until `release`).

The ioctl definitions and `#[repr(C)]` layouts are copied from `transport/uvc.rs` with their numbers unchanged; `uvc.rs` is deleted in A-14, after which each exists once again (the review checks the numbers against `uvc.rs` in this pull request's diff). What changes against `uvc::Device`: the unit is a parameter (the bus finds it by GUID), the node is given (no detection), errors are `CameraError` (`ENODEV`, `ENOENT`, `ENXIO`, `ESHUTDOWN` → `Gone`), a write shorter than the selector is padded and a longer one refused, and `on_air` takes `active` and `apps` in one sample (audit A-003).

- [ ] **Step 1: Add the module and write the failing tests**

Add `pub mod usb;` to `crates/obsbot-device/src/transport/mod.rs`; create `usb.rs` with the licence lines and the `#[cfg(test)] mod tests` of Step 3.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --lib transport::usb`
Expected: FAIL to compile: `cannot find type UsbTransport`, `from_errno`, `lock_exclusive`, `padded`.

- [ ] **Step 3: Write the transport**

Replace `crates/obsbot-device/src/transport/usb.rs` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! The USB transport (design spec 3.1, 5.8): UVC extension-unit queries and V4L2 controls on
//! one camera's video node, and its facts from sysfs. Carried from `transport/uvc.rs`
//! (`Device`), with three changes: the extension unit is the one the registry entry's GUID
//! names in the USB descriptor (no fixed unit), the node is given (no detection by name), and
//! errors are `CameraError`s (a node that is gone is `Gone` at once).
//!
//! The node is opened per operation and closed after it, so streaming applications keep
//! working: control ioctls need no exclusive access. Each operation holds an exclusive `flock`
//! on the node (`NODE_LOCK_WAIT`, 5 s), so that another process that talks to the same camera
//! (a second daemon, the firmware updater) takes turns with this one; [`Transport::hold`] keeps
//! it over a whole exchange. flock is advisory: streaming applications do not take it.

use crate::contract::camera::Result;
use crate::contract::{CameraError, ControlRange, OnAir, Transport};
use crate::enumeration::sysfs;
use crate::family::streaming;
use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg};
use nix::ioctl_readwrite;
use obsbot_proto::Usb;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// `NODE_LOCK_WAIT` (5.8): how long an operation waits for another process's lock on the node.
pub const NODE_LOCK_WAIT: Duration = Duration::from_secs(5);

const UVC_SET_CUR: u8 = 0x01;
const UVC_GET_CUR: u8 = 0x81;
const UVC_GET_LEN: u8 = 0x85;
const V4L2_CTRL_FLAG_DISABLED: u32 = 0x0001;

#[repr(C)]
struct UvcXuControlQuery {
    unit: u8,
    selector: u8,
    query: u8,
    size: u16,
    data: *mut u8,
}

#[repr(C)]
#[derive(Default)]
struct V4l2Control {
    id: u32,
    value: i32,
}

#[repr(C)]
#[derive(Default)]
struct V4l2QueryCtrl {
    id: u32,
    kind: u32,
    name: [u8; 32],
    minimum: i32,
    maximum: i32,
    step: i32,
    default_value: i32,
    flags: u32,
    reserved: [u32; 2],
}

ioctl_readwrite!(uvcioc_ctrl_query, b'u', 0x21, UvcXuControlQuery);
ioctl_readwrite!(vidioc_g_ctrl, b'V', 27, V4l2Control);
ioctl_readwrite!(vidioc_s_ctrl, b'V', 28, V4l2Control);
ioctl_readwrite!(vidioc_queryctrl, b'V', 36, V4l2QueryCtrl);

/// The error of a failed system call: a node or device that is gone is `Gone` (5.3: reported
/// at once), anything else `Io` with the errno's text.
pub fn from_errno(e: Errno) -> CameraError {
    match e {
        Errno::ENODEV | Errno::ENOENT | Errno::ENXIO | Errno::ESHUTDOWN => CameraError::Gone,
        other => CameraError::Io(other.desc().to_string()),
    }
}

fn from_io(e: std::io::Error) -> CameraError {
    from_errno(Errno::from_raw(e.raw_os_error().unwrap_or(0)))
}

/// Takes an exclusive flock on `file`, retrying every 20 ms until `wait` has passed; `Locked`
/// then. Carried from `transport/uvc.rs` (`lock_exclusive`).
pub fn lock_exclusive(mut file: File, wait: Duration) -> Result<Flock<File>> {
    let deadline = Instant::now() + wait;
    loop {
        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(lock) => return Ok(lock),
            Err((f, Errno::EWOULDBLOCK)) if Instant::now() < deadline => {
                file = f;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err((_, Errno::EWOULDBLOCK)) => return Err(CameraError::Locked),
            Err((_, e)) => return Err(from_errno(e)),
        }
    }
}

/// The transport of one camera.
pub struct UsbTransport {
    node: PathBuf,
    unit: u8,
    usb_dir: PathBuf,
    held: Option<Flock<File>>,
}

impl UsbTransport {
    /// The camera at `node` (`/dev/videoN`), its extension unit `unit` (found by GUID in the
    /// USB descriptor by the bus) and its USB device directory in sysfs. Opens nothing.
    pub fn new(node: impl Into<PathBuf>, unit: u8, usb_dir: impl Into<PathBuf>) -> UsbTransport {
        UsbTransport {
            node: node.into(),
            unit,
            usb_dir: usb_dir.into(),
            held: None,
        }
    }

    fn open(&self) -> Result<Flock<File>> {
        let file = File::open(&self.node).map_err(from_io)?;
        lock_exclusive(file, NODE_LOCK_WAIT)
    }

    /// Runs `f` on the node under the lock: the held one, or one taken for this call.
    fn with_node<T>(&mut self, f: impl FnOnce(&File) -> Result<T>) -> Result<T> {
        match &self.held {
            Some(file) => f(file),
            None => {
                let file = self.open()?;
                f(&file)
            }
        }
    }

    fn xu(file: &File, unit: u8, selector: u8, query: u8, data: &mut [u8]) -> Result<()> {
        let mut q = UvcXuControlQuery {
            unit,
            selector,
            query,
            size: data.len() as u16,
            data: data.as_mut_ptr(),
        };
        unsafe { uvcioc_ctrl_query(file.as_raw_fd(), &mut q) }.map_err(from_errno)?;
        Ok(())
    }

    fn len(file: &File, unit: u8, selector: u8) -> Result<usize> {
        let mut len = [0u8; 2];
        Self::xu(file, unit, selector, UVC_GET_LEN, &mut len)?;
        Ok(usize::from(u16::from_le_bytes(len)))
    }
}

/// The kernel fails GET_CUR and SET_CUR with ENOBUFS unless the buffer is exactly the
/// selector's length; a write is padded with zeros to it. Carried from `transport/uvc.rs`
/// (`check_len`, `Device::xu_set`).
pub fn padded(selector: u8, len: usize, data: &[u8]) -> Result<Vec<u8>> {
    if data.len() > len {
        return Err(CameraError::Io(format!(
            "{} bytes for selector {selector:#04x}, which takes {len}",
            data.len()
        )));
    }
    let mut buf = vec![0u8; len];
    buf[..data.len()].copy_from_slice(data);
    Ok(buf)
}

impl Transport for UsbTransport {
    fn xu_len(&mut self, selector: u8) -> Result<u16> {
        let unit = self.unit;
        self.with_node(|f| Self::len(f, unit, selector).map(|l| l as u16))
    }

    fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()> {
        let unit = self.unit;
        self.with_node(|f| {
            let len = Self::len(f, unit, selector)?;
            if len != buf.len() {
                return Err(CameraError::Io(format!(
                    "selector {selector:#04x} is {len} bytes, not {}",
                    buf.len()
                )));
            }
            Self::xu(f, unit, selector, UVC_GET_CUR, buf)
        })
    }

    fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()> {
        let unit = self.unit;
        self.with_node(|f| {
            let len = Self::len(f, unit, selector)?;
            let mut buf = padded(selector, len, data)?;
            Self::xu(f, unit, selector, UVC_SET_CUR, &mut buf)
        })
    }

    fn ctrl_get(&mut self, id: u32) -> Result<i32> {
        self.with_node(|f| {
            let mut c = V4l2Control { id, value: 0 };
            unsafe { vidioc_g_ctrl(f.as_raw_fd(), &mut c) }.map_err(from_errno)?;
            Ok(c.value)
        })
    }

    fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()> {
        self.with_node(|f| {
            let mut c = V4l2Control { id, value };
            unsafe { vidioc_s_ctrl(f.as_raw_fd(), &mut c) }.map_err(from_errno)?;
            Ok(())
        })
    }

    fn ctrl_range(&mut self, id: u32) -> Result<Option<ControlRange>> {
        self.with_node(|f| {
            let mut q = V4l2QueryCtrl {
                id,
                ..Default::default()
            };
            match unsafe { vidioc_queryctrl(f.as_raw_fd(), &mut q) } {
                Ok(_) if q.flags & V4L2_CTRL_FLAG_DISABLED == 0 => Ok(Some(ControlRange {
                    min: q.minimum,
                    max: q.maximum,
                    step: q.step,
                    default: q.default_value,
                })),
                Ok(_) | Err(Errno::EINVAL) => Ok(None),
                Err(e) => Err(from_errno(e)),
            }
        })
    }

    fn usb(&mut self) -> Result<Usb> {
        sysfs::usb_device(&self.usb_dir)
            .map(|(usb, _)| usb)
            .ok_or(CameraError::Gone)
    }

    fn usb_serial(&mut self) -> Result<Option<String>> {
        match fs::read_to_string(self.usb_dir.join("serial")) {
            Ok(s) => Ok(Some(s.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if self.usb_dir.exists() {
                    Ok(None)
                } else {
                    Err(CameraError::Gone)
                }
            }
            Err(e) => Err(from_io(e)),
        }
    }

    fn on_air(&mut self) -> Result<OnAir> {
        if !Path::new(&self.node).exists() {
            return Err(CameraError::Gone);
        }
        // one sample: both taken at the same moment (4.3, audit)
        let active = streaming::is_streaming(&self.node);
        let apps = streaming::apps_using(&self.node);
        Ok(OnAir { active, apps })
    }

    fn hold(&mut self) -> Result<()> {
        if self.held.is_none() {
            self.held = Some(self.open()?);
        }
        Ok(())
    }

    fn release(&mut self) {
        self.held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp(name: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        std::env::temp_dir().join(format!(
            "o4l-usb-{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn a_node_that_is_gone_is_gone_at_once() {
        for e in [Errno::ENODEV, Errno::ENOENT, Errno::ENXIO, Errno::ESHUTDOWN] {
            assert_eq!(from_errno(e), CameraError::Gone, "{e}");
        }
        assert!(matches!(from_errno(Errno::EIO), CameraError::Io(_)));
        assert!(matches!(from_errno(Errno::EACCES), CameraError::Io(_)));
        let mut t = UsbTransport::new("/nonexistent/obsbot-video0", 2, "/nonexistent/usb");
        let start = Instant::now();
        assert_eq!(t.xu_len(2), Err(CameraError::Gone));
        assert_eq!(t.hold(), Err(CameraError::Gone));
        assert_eq!(t.usb(), Err(CameraError::Gone));
        assert_eq!(t.usb_serial(), Err(CameraError::Gone));
        assert_eq!(t.on_air(), Err(CameraError::Gone));
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "no waiting for a gone node"
        );
    }

    #[test]
    fn a_second_holder_waits_and_then_gets_locked() {
        let path = temp("lock");
        File::create(&path).unwrap();
        let mut first = UsbTransport::new(&path, 2, "/nonexistent");
        first.hold().unwrap();
        let busy = lock_exclusive(File::open(&path).unwrap(), Duration::from_millis(100));
        assert!(
            matches!(busy, Err(CameraError::Locked)),
            "a held node is not shared"
        );
        let waiting = {
            let path = path.clone();
            std::thread::spawn(move || lock_exclusive(File::open(&path).unwrap(), NODE_LOCK_WAIT))
        };
        std::thread::sleep(Duration::from_millis(100));
        first.release();
        assert!(
            waiting.join().unwrap().is_ok(),
            "released, the next one gets it"
        );
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_refused_ioctl_is_an_error_not_a_value() {
        // a plain file: the kernel refuses every UVC and V4L2 ioctl on it
        let path = temp("plain");
        File::create(&path).unwrap();
        let mut t = UsbTransport::new(&path, 2, "/nonexistent");
        let mut buf = [0u8; 60];
        assert!(matches!(t.xu_get(2, &mut buf), Err(CameraError::Io(_))));
        assert!(matches!(t.xu_set(2, &[0; 60]), Err(CameraError::Io(_))));
        assert!(matches!(t.ctrl_get(1), Err(CameraError::Io(_))));
        assert!(matches!(t.ctrl_set(1, 0), Err(CameraError::Io(_))));
        assert!(matches!(t.ctrl_range(1), Err(CameraError::Io(_))));
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_write_is_padded_to_the_selector_length_and_never_cut() {
        assert_eq!(
            padded(6, 60, &[0x1a, 0x01, 0x02]).unwrap()[..4],
            [0x1a, 0x01, 0x02, 0]
        );
        assert_eq!(padded(6, 60, &[0x1a, 0x01, 0x02]).unwrap().len(), 60);
        assert_eq!(padded(2, 60, &[7; 60]).unwrap(), vec![7; 60]);
        assert!(matches!(padded(2, 60, &[0; 61]), Err(CameraError::Io(_))));
    }

    #[test]
    fn the_serial_string_is_read_from_sysfs_only_where_it_exists() {
        let dir = temp("usbdir");
        fs::create_dir_all(&dir).unwrap();
        let mut t = UsbTransport::new("/nonexistent", 2, &dir);
        assert_eq!(t.usb_serial(), Ok(None), "no serial string");
        fs::write(dir.join("serial"), "EXAMPLE0000001\n").unwrap();
        assert_eq!(t.usb_serial(), Ok(Some("EXAMPLE0000001".into())));
        fs::remove_dir_all(&dir).unwrap();
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --lib transport::usb`
Expected: PASS: `test result: ok. 5 passed`. No test opens a device node: the paths are `/nonexistent/…` or plain temporary files, which the kernel answers with `ENOENT` or a refused ioctl.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/transport/usb.rs crates/obsbot-device/src/transport/mod.rs
git commit -m "transport: USB transport with the unit found by GUID, the node lock per exchange, Gone at once"
```

### Task A-9: Tiny 2 Lite model: identity and reads

**Files:**

- Create: `crates/obsbot-device/src/models/tiny2lite/model.rs`, `read.rs`; `write.rs` and `act.rs` as stubs (A-10 and A-11 replace them)
- Modify: `crates/obsbot-device/src/models/tiny2lite/mod.rs` (four lines)
- Test: `crates/obsbot-device/tests/support/mod.rs` (shared by A-10, A-11, A-12), `crates/obsbot-device/tests/tiny2lite_read.rs`

**Interfaces:**

- Consumes: A-2 `Link`, `Timing`, `Sequence::starting_at`; A-3 `Replay`, `Log`, `Sent`, `Step`; A-4 `decode::*`; A-6 `Checked`, `Selectors`, `classify`; A-7 `plan::*`, `send_table::table`; W0 `contract::{Camera, …}`, `description::description`.
- Produces (module `obsbot_device::models::tiny2lite::model`): `const SELECTORS: Selectors`, `SERIAL_QUERY: u16 = 0x18c8`, `FIRMWARE_QUERY: u16 = 0x0408`; `struct Memory { mode: Option<Value>, target: Option<Values>, sleep_word: Option<i16> }` (`Default`); `struct Tiny2Lite` with `checked<T: Transport + 'static>(T, &'static Sequence, Timing)` (its only public constructor: every model sends through the send-table check), `memory() -> &Memory`, and `impl Camera`. Module `read`: `fn read(&mut Link, &mut Memory, group: &str, &Ctx) -> Result<Values>`. Stubs with their final signatures: `write::apply(&mut Link, &mut Memory, leaf: &str, value: &Value, &Ctx) -> Result<()>`, `act::check(action: &str, &Params, &State) -> Result<(), Refusal>`, `act::act(&mut Link, action: &str, &Params, &State, &Ctx) -> Result<Option<Params>>`.

The recorded camera of `tests/support` answers every query of the model from a recording (capture named at each answer) or a value measured on the camera; the serial is the synthetic `EXAMPLE0000001`.

- [ ] **Step 1: Add the modules and the stubs**

Add `pub mod act;`, `pub mod model;`, `pub mod read;`, `pub mod write;` to `crates/obsbot-device/src/models/tiny2lite/mod.rs` (alphabetical with the others). Create the stub `write.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing one leaf of the Tiny 2 Lite. Written in track A task A-10; until then nothing is
//! written.

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx};
use crate::family::link::Link;
use crate::models::tiny2lite::model::Memory;
use serde_json::Value;

/// Writes `leaf` = `value`: the frames of its ledger row (A-10).
pub fn apply(
    _link: &mut Link,
    _memory: &mut Memory,
    leaf: &str,
    _value: &Value,
    _ctx: &Ctx,
) -> Result<()> {
    Err(CameraError::Io(format!("not sent: {leaf} is not writable")))
}
```

and the stub `act.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The actions of the Tiny 2 Lite and the model's checks. Written in track A task A-11; until
//! then every action is refused before any frame.

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx};
use crate::family::link::Link;
use obsbot_proto::validate::Refusal;
use obsbot_proto::{ErrorCode, Params, State};

/// The model's checks before any frame (A-11).
pub fn check(action: &str, _params: &Params, _state: &State) -> std::result::Result<(), Refusal> {
    Err(Refusal::new(
        ErrorCode::Internal,
        action,
        "actions are not built yet",
    ))
}

/// Runs `action` (A-11).
pub fn act(
    _link: &mut Link,
    action: &str,
    _params: &Params,
    _state: &State,
    _ctx: &Ctx,
) -> Result<Option<Params>> {
    Err(CameraError::Io(format!("not sent: no action {action}")))
}
```

- [ ] **Step 2: Write the failing tests**

Create `crates/obsbot-device/tests/support/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! A recorded Tiny 2 Lite on the replay transport, for the model's tests. Every answer is a
//! recorded one where a recording exists (the capture is named), or a value measured on the
//! camera; the serial is synthetic (identity answers are never fixtures, spec 8.5).

#![allow(dead_code)]

use obsbot_device::contract::{Frame, OnAir};
use obsbot_device::family::frame::Sequence;
use obsbot_device::family::link::Timing;
use obsbot_device::models::tiny2lite::model::{SELECTORS, Tiny2Lite};
use obsbot_device::transport::checked::classify;
use obsbot_device::transport::replay::{Log, Replay, Sent};
use obsbot_proto::Usb;

/// The status block read in captures 05, 06 and 07 before any change: awake, static,
/// mirror off, auto-sleep off with 120 s kept.
pub const STATUS: &str = "2701000121000101010188ff000101000003000001002100000103000000001e00031000020000000000000000000000000000000000000000000000";

pub fn hex(s: &str) -> Vec<u8> {
    hex::decode(s).unwrap()
}

pub fn block(s: &str) -> [u8; 60] {
    let mut b = [0u8; 60];
    let d = hex(s);
    b[..d.len()].copy_from_slice(&d);
    b
}

/// The status block `STATUS` with the tracking mode `normal`.
pub fn status_tracking() -> [u8; 60] {
    let mut b = block(STATUS);
    b[0x18] = 2;
    b
}

/// An answer to the target query `0x01c4` with a target (layout measured 2026-10-25, #71):
/// detected, box 0.25, 0.5, 0.75, 1.0.
pub fn target_answer() -> Vec<u8> {
    let mut d = vec![1, 1, 1, 1, 0, 0];
    d.extend(floats(&[0.25, 0.5, 0.75, 1.0]));
    d
}

/// The recorded camera with the tracking on (`normal`) and a target in view.
pub fn tracking() -> Replay {
    recorded()
        .buffer(6, status_tracking())
        .answer(0x04, 0x01c4, &target_answer())
}

/// A preset list entry read from a Tiny 2 Lite (pan 12.5, pitch -3.25, zoom 1.5, "t4ltest").
pub fn preset_entry(slot: u8) -> [u8; 60] {
    let mut b = block("00000000bbfee2049600644452736447567a64413d3d");
    b[1] = slot;
    b
}

/// The USB facts of the camera: the descriptor recorded in capture 01 (USB 3564:fef9,
/// bcdDevice 0510, no serial string, video and audio interfaces, extension unit 2).
pub fn usb() -> Usb {
    Usb {
        vendor: "3564".into(),
        product: "fef9".into(),
        product_name: Some("OBSBOT Tiny 2 Lite".into()),
        bcd_device: "0510".into(),
        path: "3-2".into(),
        serial_present: false,
        interfaces: vec!["0e".into(), "0e".into(), "01".into(), "01".into()],
        extension_units: vec![obsbot_proto::ExtensionUnit {
            unit: 2,
            guid: "9a1e7291-6843-4683-6d92-39bc7906ee49".into(),
            controls: 19,
        }],
    }
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// The recorded camera: one preset in slot 0, one view, no target (static).
pub fn recorded() -> Replay {
    let mut focus = 1u32.to_le_bytes().to_vec();
    focus.extend(0u32.to_le_bytes());
    focus.extend(floats(&[0.5, 0.5]));
    let mut pose = vec![0u8; 12];
    pose[8..10].copy_from_slice(&(-1000i16).to_le_bytes());
    pose[10..12].copy_from_slice(&2500i16.to_le_bytes());
    Replay::new(2)
        .buffer(6, block(STATUS))
        // identity: synthetic serial; firmware 6.2.7.4 (capture 01)
        .answer(0x0d, 0x18c8, b"EXAMPLE0000001")
        .answer(0x0d, 0x0408, &hex("0407020600000000"))
        // capture 01
        .answer(0x04, 0x0104, &hex("000000010101001400020000"))
        // live pose: pitch -10.00 (up), yaw 25.00
        .answer(0x03, 0x0043, &pose)
        // capture 04: zoom 1.6
        .answer(0x04, 0x6804, &hex("cdcccc3f"))
        // framing readbacks (measured 2026-10-31, #98): speed normal, auto zoom off, medium
        // shot, zoom speed never set (30), offsets 0, part normal
        .answer(0x04, 0x0984, &[2])
        .answer(0x04, 0x0a84, &[0])
        .answer(0x04, 0x0b04, &[2])
        .answer(0x04, 0x0b84, &[30])
        .answer(0x04, 0x0c04, &floats(&[0.0]))
        .answer(0x04, 0x0c84, &floats(&[0.0]))
        .answer(0x04, 0x15c4, &[0])
        // focus continuous, position 0 (measured 2026-10-31, #97); volume 55 (#97)
        .answer(0x02, 0x3642, &focus)
        .answer(0x02, 0x80c2, &[0x37, 0x00])
        // capture 11: zone pan -100..100, pitch -30..30, follow off, return 0/0/1
        .answer(0x04, 0x4c44, &hex("0000c8c2"))
        .answer(0x04, 0x4cc4, &hex("0000c842"))
        .answer(0x04, 0x4d44, &hex("0000f0c1"))
        .answer(0x04, 0x4dc4, &hex("0000f041"))
        .answer(0x04, 0x4e44, &[0])
        .answer(
            0x04,
            0x4ec4,
            &hex("000000000000000000000000000000000000803f00000000"),
        )
        // capture 11: hand area -45..45 / -30..30, axes on, no return pose
        .answer(
            0x04,
            0x2004,
            &hex("000034c2000034420000f0c10000f041ffffffff00050101"),
        )
        .answer(
            0x04,
            0x2504,
            &hex("ffffffff0000000000000000000000000000803f00000000"),
        )
        .answer(0x04, 0x2484, &floats(&[2.5]))
        // capture 08: own start pose, start mode normal, no settings
        .answer(
            0x04,
            0x3884,
            &hex("03000000676666bfcdcc88c1000000000000c03f00000000"),
        )
        .answer(
            0x04,
            0x3e84,
            &hex(
                "00ffffffffffffff80ffffffffffffff00000000ffffffff00000000000000000000000000000000",
            ),
        )
        // capture 11: one view, id 0, "Bereich1"
        .answer(0x04, 0x1344, &hex("00000000"))
        .answer_to(0x04, 0x1504, &[0, 0, 0, 0], &hex("4265726569636831"))
        .answer_to(
            0x04,
            0x1444,
            &[0, 0, 0, 0],
            &hex("00000000cdcc4c3e0000000000000000a370bd3f00000000"),
        )
        // one preset in slot 0 (measured 2026-10-31, #104)
        .buffer(0x0c, block("0100"))
        .buffer(0x0d, preset_entry(0))
        // UVC controls
        .ctrl(0x009a_0901, 3)
        .ctrl(0x009a_0902, 330)
        .ctrl(0x0098_0913, 1)
        .ctrl(0x0098_091c, 9)
        .ctrl(0x0098_0900, 50)
        .ctrl(0x0098_0901, 50)
        .ctrl(0x0098_0902, 50)
        .ctrl(0x0098_0903, 50)
        .ctrl(0x0098_091b, 50)
        .ctrl(0x0098_090c, 1)
        .ctrl(0x0098_091a, 5000)
        .ctrl(0x009a_090d, 20)
        .ctrl(0x009a_090a, 0)
        .ctrl(0x0098_0918, 1)
        .usb(usb())
        .on_air(OnAir::default())
}

/// A sequence for one test, starting at `first`.
pub fn seq(first: u16) -> &'static Sequence {
    Box::leak(Box::new(Sequence::starting_at(first)))
}

/// The model on `replay`, behind the send-table check, without waits; and the replay's log.
pub fn model(replay: Replay) -> (Tiny2Lite, Log) {
    let log = replay.log();
    (Tiny2Lite::checked(replay, seq(1), Timing::instant()), log)
}

/// What was sent, as frames: queries left out (they are reads), commands, buffer writes and
/// UVC writes in order.
pub fn commands(log: &Log) -> Vec<Frame> {
    log.sent()
        .into_iter()
        .filter_map(|s| match s {
            Sent::Xu { selector, data } => {
                let f = classify(&SELECTORS, selector, &data).expect("only frames are sent");
                (!matches!(f, Frame::Query { .. })).then_some(f)
            }
            Sent::Ctrl { id, .. } => Some(Frame::Uvc { control: id }),
        })
        .collect()
}

/// The data of every command sent on selector 2 (bytes 16 and on, as long as the frame says),
/// and the payload of every short command, in order; queries left out.
pub fn payloads(log: &Log) -> Vec<Vec<u8>> {
    log.sent()
        .into_iter()
        .filter_map(|s| match s {
            Sent::Xu { selector: 2, data } if data[1] == 0x25 => {
                let len = usize::from(u16::from_le_bytes([data[12], data[13]]));
                Some(data[16..16 + len].to_vec())
            }
            Sent::Xu { selector: 6, data } => Some(data),
            Sent::Ctrl { value, .. } => Some(value.to_le_bytes().to_vec()),
            _ => None,
        })
        .collect()
}

pub fn framed(command: u16) -> Frame {
    Frame::Framed {
        selector: 2,
        receiver: 0x04,
        command,
    }
}

pub fn framed_to(receiver: u8, command: u16) -> Frame {
    Frame::Framed {
        selector: 2,
        receiver,
        command,
    }
}

pub fn short(command: u8) -> Frame {
    Frame::Short {
        selector: 6,
        command,
    }
}

pub fn uvc(control: u32) -> Frame {
    Frame::Uvc { control }
}
```

Create `crates/obsbot-device/tests/tiny2lite_read.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite model reading its groups and its identity on a recorded camera (replay
//! transport): what each read sends, what it decodes, and how a read fails.

mod support;

use obsbot_device::contract::{Camera, CameraError, Ctx, Frame, Need};
use obsbot_device::transport::replay::{Sent, Step};
use serde_json::json;
use support::*;

fn queries(log: &obsbot_device::transport::replay::Log) -> Vec<(u8, u16)> {
    log.sent()
        .into_iter()
        .filter_map(|s| match s {
            Sent::Xu { selector: 2, data } if matches!(data[1], 0x01 | 0x21) => {
                Some((data[9], u16::from_le_bytes([data[10], data[11]])))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn an_identity_attempt_sends_only_the_missing_queries() {
    let (mut cam, log) = model(recorded());
    let both = cam
        .identity(
            Need {
                serial: true,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(both.serial.as_deref(), Some("EXAMPLE0000001"));
    assert_eq!(both.firmware.as_deref(), Some("6.2.7.4"));
    assert_eq!(queries(&log), [(0x0d, 0x18c8), (0x0d, 0x0408)]);

    log.clear();
    let firmware_only = cam
        .identity(
            Need {
                serial: false,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(firmware_only.serial, None);
    assert_eq!(queries(&log), [(0x0d, 0x0408)]);

    log.clear();
    cam.identity(
        Need {
            serial: false,
            firmware: false,
        },
        &Ctx::none(),
    )
    .unwrap();
    assert!(log.sent().is_empty(), "nothing asked, nothing sent (5.5)");
}

#[test]
fn an_unanswered_identity_query_spends_the_attempt_without_failing_it() {
    // a camera that does not answer the serial query (CAM-1): no serial, the firmware still
    let (mut cam, _) = model(obsbot_device::transport::replay::Replay::new(2).answer(
        0x0d,
        0x0408,
        &hex("0407020600000000"),
    ));
    let id = cam
        .identity(
            Need {
                serial: true,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!((id.serial, id.firmware.as_deref()), (None, Some("6.2.7.4")));
    // a refused query is spent the same way
    let (mut cam, _) =
        model(obsbot_device::transport::replay::Replay::new(2).reject(0x0d, 0x18c8, -3));
    let id = cam
        .identity(
            Need {
                serial: true,
                firmware: false,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(id.serial, None);
}

#[test]
fn an_identity_attempt_on_a_gone_camera_fails() {
    let (mut cam, _) = model(recorded().gone());
    assert_eq!(
        cam.identity(
            Need {
                serial: true,
                firmware: true
            },
            &Ctx::none()
        ),
        Err(CameraError::Gone)
    );
}

#[test]
fn every_group_reads_exactly_its_leaves_and_sends_no_command() {
    // the tracking on, so that the target group is asked too
    let (mut cam, log) = model(tracking());
    let plan = cam.poll_plan().to_vec();
    for group in &plan {
        let values = cam.read(&group.name, &Ctx::none()).unwrap();
        let mut got: Vec<&String> = values.keys().collect();
        let mut want: Vec<&String> = group.leaves.iter().collect();
        got.sort();
        want.sort();
        assert_eq!(got, want, "group {}", group.name);
    }
    // reads write queries and the preset list rewind, and nothing else (spec 2)
    assert_eq!(commands(&log), [Frame::Block { selector: 0x0c }]);
}

#[test]
fn the_status_read_decodes_the_recorded_block_and_remembers_the_mode() {
    let (mut cam, log) = model(recorded());
    let v = cam.read("status", &Ctx::none()).unwrap();
    assert_eq!(v["tracking.mode"], json!("static"));
    assert_eq!(v["led.brightness"], json!(3));
    assert_eq!(v["sleep.auto.seconds"], json!(120));
    assert_eq!(cam.memory().mode, Some(json!("static")));
    assert_eq!(cam.memory().sleep_word, Some(-120));
    assert!(
        log.sent().is_empty(),
        "a status read is a read of selector 6, no write"
    );
}

#[test]
fn the_target_is_read_only_while_the_tracking_mode_is_known_and_not_static() {
    // the mode not read yet: no query and no value the camera did not report, so the leaves
    // stay in `unread` (`detected` is not nullable)
    let (mut cam, log) = model(tracking());
    assert!(matches!(
        cam.read("target", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(log.sent().is_empty());
    // static and never read: the same
    let (mut cam, log) = model(recorded());
    cam.read("status", &Ctx::none()).unwrap();
    assert!(matches!(
        cam.read("target", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(log.sent().is_empty());

    // tracking: the query and the camera's answer; then static: the values of the last read,
    // without a query (trait `Camera::read`)
    let replay = recorded()
        .reads(6, &[status_tracking()])
        .answer(0x04, 0x01c4, &target_answer());
    let (mut cam, log) = model(replay);
    cam.read("status", &Ctx::none()).unwrap();
    let v = cam.read("target", &Ctx::none()).unwrap();
    assert_eq!(queries(&log), [(0x04, 0x01c4)]);
    assert_eq!(v["tracking.target.detected"], json!(true));
    assert_eq!(
        v["tracking.target.box"],
        json!({"left": 0.25, "top": 0.5, "right": 0.75, "bottom": 1})
    );
    cam.read("status", &Ctx::none()).unwrap();
    assert_eq!(cam.memory().mode, Some(json!("static")));
    assert_eq!(cam.read("target", &Ctx::none()).unwrap(), v);
    assert_eq!(queries(&log), [(0x04, 0x01c4)]);
}

#[test]
fn a_query_of_the_model_holds_the_node_lock_over_the_frame_and_its_answer() {
    // 5.8: one hold covers the query and the reads of its answer, on the transport a bus opens
    // (the model behind the send-table check)
    let (mut cam, log) = model(recorded());
    cam.read("zoom", &Ctx::none()).unwrap();
    assert_eq!(
        log.steps(),
        [Step::Hold, Step::Set(2), Step::Get(2), Step::Release]
    );
}

#[test]
fn the_live_pose_comes_from_its_own_query_in_the_units_of_4_2() {
    let (mut cam, log) = model(recorded());
    let v = cam.read("pose", &Ctx::none()).unwrap();
    assert_eq!(
        (&v["gimbal.pan"], &v["gimbal.tilt"]),
        (&json!(25), &json!(10))
    );
    assert_eq!(
        queries(&log),
        [(0x03, 0x0043)],
        "receiver 3, query 0x0043 only (8.8)"
    );
}

#[test]
fn the_preset_list_is_rewound_once_and_read_whole() {
    let (mut cam, log) = model(recorded());
    let v = cam.read("presets", &Ctx::none()).unwrap();
    assert_eq!(
        v["presets"],
        json!([{"slot": 0, "name": "t4ltest", "pose": {"pan": 12.5, "tilt": 3.3, "zoom": 1.5}}])
    );
    assert_eq!(
        log.sent(),
        [Sent::Xu {
            selector: 0x0c,
            data: block("0100").to_vec()
        }],
        "the list is written back as it was read"
    );
    // an empty list: nothing to rewind
    let (mut cam, log) = model(recorded().buffer(0x0c, block("00")));
    assert_eq!(
        cam.read("presets", &Ctx::none()).unwrap()["presets"],
        json!([])
    );
    assert!(log.sent().is_empty());
    // the count says two, one entry decodes: a failed read, not a short list
    let mut end = preset_entry(1);
    end[0] = 2;
    let (mut cam, _) = model(
        recorded()
            .buffer(0x0c, block("020001"))
            .reads(0x0d, &[preset_entry(0), end]),
    );
    assert!(matches!(
        cam.read("presets", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
}

#[test]
fn the_views_are_listed_with_their_names_and_poses() {
    let (mut cam, log) = model(recorded());
    let v = cam.read("views", &Ctx::none()).unwrap();
    assert_eq!(
        v["views"],
        json!([{"id": 0, "name": "Bereich1", "pose": {"pan": 0.2, "tilt": 0, "zoom": 1.48}}])
    );
    assert_eq!(
        queries(&log),
        [(0x04, 0x1344), (0x04, 0x1504), (0x04, 0x1444)]
    );
    // no view stored: the empty answer of capture 11
    let (mut cam, _) = model(recorded().empty(0x04, 0x1344));
    assert_eq!(cam.read("views", &Ctx::none()).unwrap()["views"], json!([]));
}

#[test]
fn a_failed_query_fails_its_group_only() {
    // the hand return timer (U-6) unanswered: the timer group fails, the hand group reads
    let (mut cam, _) = model(recorded().reject(0x04, 0x2484, -1));
    assert!(matches!(
        cam.read("hand_timer", &Ctx::none()),
        Err(CameraError::Rejected(_))
    ));
    assert!(cam.read("hand", &Ctx::none()).is_ok());
    // an unanswered query of a group with several: the group fails at it
    let (mut cam, log) = model(obsbot_device::transport::replay::Replay::new(2).answer(
        0x04,
        0x4c44,
        &hex("0000c8c2"),
    ));
    assert!(matches!(
        cam.read("zone", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert_eq!(
        queries(&log),
        [(0x04, 0x4c44), (0x04, 0x4cc4)],
        "the read stops at its first unanswered query (5.7)"
    );
}

#[test]
fn a_gone_camera_fails_a_read_at_once() {
    let (mut cam, _) = model(recorded().gone());
    assert_eq!(cam.read("status", &Ctx::none()), Err(CameraError::Gone));
    assert_eq!(cam.read("zoom", &Ctx::none()), Err(CameraError::Gone));
}

#[test]
fn a_stop_cuts_a_group_read_before_its_next_query() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let (mut cam, log) = model(recorded());
    let ctx = Ctx::interruptible(Arc::new(AtomicBool::new(true)));
    assert_eq!(cam.read("framing", &ctx), Err(CameraError::Cut));
    assert!(log.sent().is_empty());
}

#[test]
fn the_audio_group_takes_the_microphone_from_the_usb_interfaces() {
    let mut usb = usb();
    usb.interfaces.truncate(2);
    let (mut cam, _) = model(recorded().usb(usb));
    let v = cam.read("audio", &Ctx::none()).unwrap();
    assert_eq!(
        (&v["audio.enabled"], &v["audio.volume"]),
        (&json!(false), &json!(55))
    );
}

#[test]
fn an_unknown_group_is_an_error_and_sends_nothing() {
    let (mut cam, log) = model(recorded());
    assert!(matches!(
        cam.read("gimbal", &Ctx::none()),
        Err(CameraError::Io(_))
    ));
    assert!(log.sent().is_empty());
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --test tiny2lite_read`
Expected: FAIL to compile: `unresolved import obsbot_device::models::tiny2lite::model` (`SELECTORS`, `Tiny2Lite`).

- [ ] **Step 4: Write the model and the reads**

Create `crates/obsbot-device/src/models/tiny2lite/model.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite behind `trait Camera` (design spec 8.2, 8.8). The model owns its link to
//! the camera; reading is in `read`, writing in `write`, actions in `act`, the plan data in
//! `plan` and the send table in `send_table`.

use crate::contract::camera::Result;
use crate::contract::{
    Camera, CameraError, Ctx, Group, Identity, Motion, Need, SendTable, Target, Transport,
};
use crate::family::frame::Sequence;
use crate::family::link::{Link, Timing};
use crate::models::tiny2lite::description::description;
use crate::models::tiny2lite::plan::{self, ENTRY, FRAMED, IDENTITY, SHORT};
use crate::models::tiny2lite::{act, decode, read, send_table, write};
use crate::transport::checked::{Checked, Selectors};
use obsbot_proto::validate::Refusal;
use obsbot_proto::{Description, Params, State, Values};
use serde_json::Value;

/// Where the Tiny 2 Lite's frames go (its registry entry, 8.8).
pub const SELECTORS: Selectors = Selectors {
    framed: FRAMED,
    short: SHORT,
    all: ENTRY.selectors,
};

/// The identity queries (receiver `0x0d`, 8.8): serial and firmware.
pub const SERIAL_QUERY: u16 = 0x18c8;
pub const FIRMWARE_QUERY: u16 = 0x0408;

/// What the model keeps from its own last reads and writes: the tracking mode (the target is
/// read only while it is not `static`, 8.8), the values of the last target read (the group's
/// values while it is not read, trait `Camera::read`) and the auto-sleep word
/// (`sleep.auto.enabled` and `sleep.auto.seconds` are one word on the camera, 4.3).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Memory {
    pub mode: Option<Value>,
    pub target: Option<Values>,
    pub sleep_word: Option<i16>,
}

/// One Tiny 2 Lite.
pub struct Tiny2Lite {
    link: Link,
    description: Description,
    plan: Vec<Group>,
    table: SendTable,
    memory: Memory,
}

impl Tiny2Lite {
    /// The model on a transport that is already checked against its send table; private, so
    /// that [`Tiny2Lite::checked`] is the only way to build one.
    fn new(transport: Box<dyn Transport>, seq: &'static Sequence, timing: Timing) -> Tiny2Lite {
        Tiny2Lite {
            link: Link::new(transport, seq, timing),
            description: description(),
            plan: plan::groups(),
            table: send_table::table(),
            memory: Memory::default(),
        }
    }

    /// The model on `transport`, wrapped in the send-table check (8.3): what a bus opens, and
    /// what every test uses, so that a frame outside the table fails the test.
    pub fn checked<T: Transport + 'static>(
        transport: T,
        seq: &'static Sequence,
        timing: Timing,
    ) -> Tiny2Lite {
        Tiny2Lite::new(
            Box::new(Checked::new(transport, send_table::table(), SELECTORS)),
            seq,
            timing,
        )
    }

    /// What the model keeps from its reads and writes (tests).
    pub fn memory(&self) -> &Memory {
        &self.memory
    }
}

/// One identity query: its answer, `None` when the camera did not answer or refused it (the
/// attempt is spent), an error for anything else (gone, cut, locked).
fn ask(link: &mut Link, command: u16, ctx: &Ctx) -> Result<Option<Vec<u8>>> {
    match link.query(FRAMED, IDENTITY, command, ctx) {
        Ok(d) => Ok(Some(d)),
        Err(CameraError::NoAnswer(_) | CameraError::Rejected(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

impl Camera for Tiny2Lite {
    fn description(&self) -> &Description {
        &self.description
    }

    fn identity(&mut self, need: Need, ctx: &Ctx) -> Result<Identity> {
        let mut id = Identity::default();
        if need.serial {
            id.serial = ask(&mut self.link, SERIAL_QUERY, ctx)?.and_then(|d| decode::serial(&d));
        }
        if need.firmware {
            id.firmware =
                ask(&mut self.link, FIRMWARE_QUERY, ctx)?.and_then(|d| decode::firmware(&d));
        }
        Ok(id)
    }

    fn poll_plan(&self) -> &[Group] {
        &self.plan
    }

    fn read(&mut self, group: &str, ctx: &Ctx) -> Result<Values> {
        read::read(&mut self.link, &mut self.memory, group, ctx)
    }

    fn order(&self, leaves: &[&str]) -> Vec<String> {
        plan::order(leaves)
    }

    fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<()> {
        write::apply(&mut self.link, &mut self.memory, leaf, value, ctx)
    }

    fn check(
        &self,
        action: &str,
        params: &Params,
        state: &State,
    ) -> std::result::Result<(), Refusal> {
        act::check(action, params, state)
    }

    fn act(
        &mut self,
        action: &str,
        params: &Params,
        state: &State,
        ctx: &Ctx,
    ) -> Result<Option<Params>> {
        act::act(&mut self.link, action, params, state, ctx)
    }

    fn touched(&self, target: Target<'_>) -> Vec<String> {
        plan::touched(&self.plan, target)
    }

    fn motion(&self, action: &str) -> Motion {
        plan::motion(action)
    }

    fn send_table(&self) -> &SendTable {
        &self.table
    }
}
```

Create `crates/obsbot-device/src/models/tiny2lite/read.rs` (the query sequences are carried from the read methods of `camera.rs`: `status`, `tracking`, `zoom_factor`, `live_pose`, `target`, `audio_volume`, `focus`, `zone`, `hand`, `zone_views`, `startup`, `startup_actions`, `framing`, `presets`; the 0.13 cache, the remembered state and the "tolerated" errors are gone, and the preset list fails as a whole when it does not read whole):

```rust
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
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test tiny2lite_read`
Expected: PASS: `test result: ok. 15 passed`.

Run: `cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: every test result ok; no clippy output.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot-device/src/models/tiny2lite crates/obsbot-device/tests/support crates/obsbot-device/tests/tiny2lite_read.rs
git commit -m "tiny2lite: the model behind trait Camera: identity, poll plan, reads"
```

### Task A-10: Tiny 2 Lite writes

**Files:**

- Modify: `crates/obsbot-device/src/models/tiny2lite/write.rs` (replace the stub), `crates/obsbot-device/src/models/tiny2lite/commands.rs` (the per-axis area builder and its test)
- Test: `crates/obsbot-device/tests/tiny2lite_write.rs`

**Interfaces:**

- Consumes: A-9 `model::{Memory, Tiny2Lite}`, `tests/support`; A-2 `Link::{send, send_spaced, short, query, read, ctrl_set, seq}`; A-4 `decode::{byte_of, mode_pair, tilt_to_pitch_range, UVC, ZOOM_ABSOLUTE, FOCUS_ABSOLUTE, POWER_LINE, the enum tables}`; A-7 `plan::{FRAMED, SHORT, TRACKING}`; stage G builders in `commands.rs` and `presets::startup_with_ai`.
- Produces: `write::apply(&mut Link, &mut Memory, leaf: &str, value: &Value, &Ctx) -> Result<()>` for all 60 writable leaves; in `commands`: `const ZONE_PAN`, `ZONE_PITCH`, `HAND_PAN`, `HAND_PITCH: [u16; 2]`, `YAW_LIMITS`, `PITCH_LIMITS: (f32, f32)`, `fn area_axis(ids: [u16; 2], range: (f32, f32), limits: (f32, f32), seqs: [u16; 2]) -> Result<[[u8; 60]; 2]>`.

No leaf writes an automatic control first (0.13's `set_focus` wrote autofocus off, `set_exposure_mode` wrote automatic exposure, `set_manual_exposure` wrote manual first): a patch names both leaves, and the apply order of A-7 puts the `requires` path first. No leaf touches the remembered state (it is gone with `legacy/` in A-14).

- [ ] **Step 1: Write the failing tests**

Create `crates/obsbot-device/tests/tiny2lite_write.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite model writing leaves on a recorded camera (replay transport): each leaf
//! sends the frames of its ledger row and nothing before or after them (spec 2).

mod support;

use obsbot_device::contract::{Camera, CameraError, Ctx};
use serde_json::{Value, json};
use support::*;

fn write(leaf: &str, value: Value) -> (Vec<obsbot_device::contract::Frame>, Vec<Vec<u8>>) {
    let (mut cam, log) = model(recorded());
    cam.apply(leaf, &value, &Ctx::none()).unwrap();
    (commands(&log), payloads(&log))
}

#[test]
fn short_command_leaves_send_their_short_command() {
    let rows: &[(&str, Value, &[u8])] = &[
        ("zoom.field_of_view", json!("narrow"), &[0x04, 0x01, 0x02]),
        (
            "tracking.mode",
            json!("close_up"),
            &[0x16, 0x02, 0x02, 0x02],
        ),
        ("tracking.mode", json!("group"), &[0x16, 0x02, 0x01, 0x00]),
        ("exposure.region", json!("face"), &[0x03, 0x01, 0x01]),
        ("exposure.hdr", json!(false), &[0x01, 0x01, 0x00]),
        ("image.mirror", json!(true), &[0x14, 0x01, 0x01]),
        ("audio.enabled", json!(false), &[0x1c, 0x01, 0x00]),
        (
            "audio.noise_reduction",
            json!("strong"),
            &[0x0a, 0x01, 0x03],
        ),
        ("audio.auto_gain", json!(true), &[0x17, 0x01, 0x01]),
        ("audio.in_sleep", json!(false), &[0x13, 0x01, 0x00]),
        ("led.brightness", json!(2), &[0x1a, 0x01, 0x02]),
        ("led.blink", json!(true), &[0x18, 0x01, 0x01]),
        (
            "sleep.background",
            json!("image"),
            &[0x0e, 0x02, 0x00, 0x01],
        ),
        (
            "sleep.background_mirror",
            json!(true),
            &[0x0e, 0x02, 0x02, 0x01],
        ),
        ("sleep.wake_zoom", json!(2.0), &[0x15, 0x02, 0x64, 33]),
        ("power_on.mode", json!("upper_body"), &[0x19, 0x01, 0x21]),
    ];
    for (leaf, value, bytes) in rows {
        let (frames, data) = write(leaf, value.clone());
        assert_eq!(frames, [short(bytes[0])], "{leaf}");
        assert_eq!(data, [bytes.to_vec()], "{leaf} = {value}");
    }
}

#[test]
fn framed_leaves_send_one_frame_with_their_value() {
    let rows: &[(&str, Value, u8, u16, Vec<u8>)] = &[
        ("gimbal.invert", json!(true), 0x04, 0x3b84, vec![1]),
        ("tracking.profile", json!("headroom"), 0x04, 0x0cc4, vec![1]),
        ("tracking.speed", json!("very_fast"), 0x04, 0x0944, vec![4]),
        ("framing.auto_zoom", json!(true), 0x04, 0x0a44, vec![1]),
        ("framing.size", json!("big_close_up"), 0x04, 0x0ac4, vec![5]),
        ("framing.zoom_speed", json!("fast"), 0x04, 0x0b44, vec![2]),
        (
            "framing.offset.x",
            json!(0.1),
            0x04,
            0x0bc4,
            0.1f32.to_le_bytes().to_vec(),
        ),
        (
            "framing.offset.y",
            json!(-0.16),
            0x04,
            0x0c44,
            (-0.16f32).to_le_bytes().to_vec(),
        ),
        ("framing.part", json!("lower_body"), 0x04, 0x1584, vec![2]),
        ("gestures.target_lock", json!(false), 0x04, 0x30c4, vec![0]),
        ("gestures.zoom", json!(true), 0x04, 0x3144, vec![1]),
        (
            "gestures.zoom_ratio",
            json!(2.0),
            0x04,
            0x3244,
            2.0f32.to_le_bytes().to_vec(),
        ),
        ("gestures.dynamic_zoom", json!(false), 0x04, 0x3344, vec![0]),
        (
            "gestures.direction_change",
            json!(true),
            0x04,
            0x33c4,
            vec![1],
        ),
        ("zone.enabled", json!(true), 0x04, 0x08c4, vec![1]),
        ("zone.follow", json!(false), 0x04, 0x4e04, vec![0]),
        ("hand.side", json!("left"), 0x04, 0x2244, vec![1]),
        ("hand.axes.pan", json!(false), 0x04, 0x2344, vec![0]),
        ("hand.axes.tilt", json!(true), 0x04, 0x23c4, vec![1]),
        (
            "hand.return_after",
            json!(2.5),
            0x04,
            0x2444,
            2.5f32.to_le_bytes().to_vec(),
        ),
        ("hand.move_gimbal", json!(false), 0x04, 0x26c4, vec![0]),
        (
            "focus.mode",
            json!("manual"),
            0x02,
            0x3442,
            3u32.to_le_bytes().to_vec(),
        ),
        (
            "focus.region",
            json!("face"),
            0x02,
            0x3602,
            1u32.to_le_bytes().to_vec(),
        ),
        (
            "audio.volume",
            json!(55),
            0x02,
            0x8102,
            55i16.to_le_bytes().to_vec(),
        ),
    ];
    for (leaf, value, receiver, command, data) in rows {
        let (frames, sent) = write(leaf, value.clone());
        assert_eq!(frames, [framed_to(*receiver, *command)], "{leaf}");
        assert_eq!(sent, std::slice::from_ref(data), "{leaf} = {value}");
    }
}

#[test]
fn uvc_leaves_write_their_control_and_no_other() {
    // no automatic control is switched off first (spec 2): exposure.time, the white balance
    // temperature and focus.position each write their own control; the patch names the mode
    let rows: &[(&str, Value, u32, i32)] = &[
        ("zoom.factor", json!(1.0), 0x009a_090d, 0),
        ("zoom.factor", json!(2.5), 0x009a_090d, 50),
        ("zoom.factor", json!(4.0), 0x009a_090d, 100),
        ("exposure.mode", json!("manual"), 0x009a_0901, 1),
        ("exposure.mode", json!("auto"), 0x009a_0901, 0),
        ("exposure.time", json!(100), 0x009a_0902, 1),
        ("exposure.time", json!(33050), 0x009a_0902, 330),
        ("exposure.time", json!(250000), 0x009a_0902, 2500),
        ("exposure.gain", json!(64), 0x0098_0913, 64),
        ("exposure.compensation", json!(18), 0x0098_091c, 18),
        ("exposure.anti_flicker", json!("60hz"), 0x0098_0918, 2),
        ("image.brightness", json!(0), 0x0098_0900, 0),
        ("image.contrast", json!(100), 0x0098_0901, 100),
        ("image.saturation", json!(10), 0x0098_0902, 10),
        ("image.hue", json!(20), 0x0098_0903, 20),
        ("image.sharpness", json!(30), 0x0098_091b, 30),
        ("image.white_balance.auto", json!(false), 0x0098_090c, 0),
        (
            "image.white_balance.temperature",
            json!(3800),
            0x0098_091a,
            3800,
        ),
        ("focus.position", json!(40), 0x009a_090a, 40),
    ];
    for (leaf, value, control, raw) in rows {
        let (frames, sent) = write(leaf, value.clone());
        assert_eq!(frames, [uvc(*control)], "{leaf}");
        assert_eq!(sent, [raw.to_le_bytes().to_vec()], "{leaf} = {value}");
    }
}

#[test]
fn an_area_axis_sends_min_then_max_in_the_cameras_angles() {
    // capture 11: zone pan -100..100 and the asymmetric tilt -30..10 (pitch -10..30)
    let (frames, data) = write("zone.area.pan", json!({"min": -100, "max": 100}));
    assert_eq!(frames, [framed(0x4c04), framed(0x4c84)]);
    assert_eq!(
        data,
        [
            (-100f32).to_le_bytes().to_vec(),
            100f32.to_le_bytes().to_vec()
        ]
    );
    let (frames, data) = write("zone.area.tilt", json!({"min": -30, "max": 10}));
    assert_eq!(frames, [framed(0x4d04), framed(0x4d84)]);
    assert_eq!(
        data,
        [
            (-10f32).to_le_bytes().to_vec(),
            30f32.to_le_bytes().to_vec()
        ]
    );
    let (frames, _) = write("hand.area.pan", json!({"min": -45, "max": 45}));
    assert_eq!(frames, [framed(0x2044), framed(0x20c4)]);
    let (frames, data) = write("hand.area.tilt", json!({"min": -30, "max": 10}));
    assert_eq!(frames, [framed(0x2144), framed(0x21c4)]);
    assert_eq!(
        data,
        [
            (-10f32).to_le_bytes().to_vec(),
            30f32.to_le_bytes().to_vec()
        ]
    );
}

#[test]
fn an_area_with_equal_bounds_is_not_sent() {
    // the W0 validator accepts min == max; the carried builder refuses it (U-8, CAM-16)
    let (mut cam, log) = model(recorded());
    let r = cam.apply(
        "zone.area.pan",
        &json!({"min": 10, "max": 10}),
        &Ctx::none(),
    );
    assert!(matches!(r, Err(CameraError::Io(_))), "{r:?}");
    assert!(log.sent().is_empty());
}

#[test]
fn auto_sleep_writes_one_word_with_the_other_half_as_the_camera_has_it() {
    // recorded status: off with 120 s (word -120)
    let (mut cam, log) = model(recorded());
    cam.apply("sleep.auto.enabled", &json!(true), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log),
        [vec![0x0b, 0x02, 0x78, 0x00]],
        "on with the 120 s kept"
    );
    // the word the model wrote is the one the next leaf of a patch starts from
    cam.apply("sleep.auto.seconds", &json!(600), &Ctx::none())
        .unwrap();
    assert_eq!(payloads(&log)[1], [0x0b, 0x02, 0x58, 0x02], "on, 600 s");
    cam.apply("sleep.auto.enabled", &json!(false), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log)[2],
        [0x0b, 0x02, 0xa8, 0xfd],
        "off, 600 s kept (-600)"
    );
    // seconds alone while off keeps it off (4.3)
    cam.apply("sleep.auto.seconds", &json!(30), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log)[3],
        [0x0b, 0x02, 0xe2, 0xff],
        "off, 30 s (-30)"
    );
    assert_eq!(commands(&log), vec![short(0x0b); 4]);
}

#[test]
fn auto_sleep_reads_the_status_block_when_it_has_no_word_yet_and_after_a_status_read() {
    let (mut cam, log) = model(recorded());
    assert_eq!(cam.memory().sleep_word, None);
    cam.apply("sleep.auto.seconds", &json!(300), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log),
        [vec![0x0b, 0x02, 0xd4, 0xfe]],
        "off with 300 s (-300)"
    );
    // a status read replaces what the model wrote with what the camera reports
    let mut on = block(STATUS);
    on[0x0a] = 0x1e;
    on[0x0b] = 0x00;
    let (mut cam, log) = model(recorded().buffer(6, on));
    cam.read("status", &Ctx::none()).unwrap();
    cam.apply("sleep.auto.enabled", &json!(false), &Ctx::none())
        .unwrap();
    assert_eq!(payloads(&log), [vec![0x0b, 0x02, 0xe2, 0xff]]);
}

#[test]
fn auto_sleep_with_0_s_stored_sends_nothing_for_the_switch_alone() {
    // the camera cannot store "off" with 0 s, and "on with 0 s" was never recorded (PX-7);
    // obsbot4linux writes no time of its own (spec 2)
    let mut zero = block(STATUS);
    zero[0x0a] = 0;
    zero[0x0b] = 0;
    for on in [true, false] {
        let (mut cam, log) = model(recorded().buffer(6, zero));
        let r = cam.apply("sleep.auto.enabled", &json!(on), &Ctx::none());
        assert!(matches!(r, Err(CameraError::Io(_))), "{r:?}");
        assert!(commands(&log).is_empty(), "{on}");
    }
}

#[test]
fn the_start_mode_writes_back_the_block_it_read_with_byte_0_changed() {
    let (mut cam, log) = model(recorded());
    cam.apply("power_on.start.mode", &json!("unset"), &Ctx::none())
        .unwrap();
    assert_eq!(commands(&log), [framed(0x3e44)]);
    // capture 08: the block of 0x3e84 with start mode normal, written with "not set"
    assert_eq!(
        payloads(&log),
        [hex(
            "feffffffffffffff80ffffffffffffff00000000ffffffff00000000000000000000000000000000"
        )]
    );
    // a short block is not written back
    let (mut cam, log) = model(recorded().answer(0x04, 0x3e84, &[0xfe; 39]));
    assert!(matches!(
        cam.apply("power_on.start.mode", &json!("normal"), &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(commands(&log).is_empty());
}

#[test]
fn a_value_the_builder_refuses_sends_nothing() {
    // the validator would refuse these; the model still sends nothing for them
    let (mut cam, log) = model(recorded());
    for (leaf, value) in [
        ("led.brightness", json!(4)),
        ("framing.offset.x", json!(0.5)),
        ("tracking.mode", json!("off")),
        ("gimbal.pan", json!(1)),
    ] {
        assert!(
            cam.apply(leaf, &value, &Ctx::none()).is_err(),
            "{leaf} = {value}"
        );
    }
    assert!(log.sent().is_empty());
}

#[test]
fn a_write_on_a_gone_camera_fails_with_gone() {
    let (mut cam, _) = model(recorded().gone());
    assert_eq!(
        cam.apply("led.brightness", &json!(1), &Ctx::none()),
        Err(CameraError::Gone)
    );
}
```

In `crates/obsbot-device/src/models/tiny2lite/commands.rs`, add to its test module, before `hand_gimbal_matches_capture`:

```rust
    #[test]
    fn one_area_axis_is_the_two_frames_of_that_axis() {
        // capture 11, seq 430-433 (zone) and 397-400 (hand): the same bytes as the whole area
        let zone = zone_area((-100.0, 100.0), (-30.0, 30.0), [430, 431, 432, 433]).unwrap();
        assert_eq!(
            area_axis(ZONE_PAN, (-100.0, 100.0), YAW_LIMITS, [430, 431]).unwrap(),
            [zone[0], zone[1]]
        );
        assert_eq!(
            area_axis(ZONE_PITCH, (-30.0, 30.0), PITCH_LIMITS, [432, 433]).unwrap(),
            [zone[2], zone[3]]
        );
        let hand = hand_area((-45.0, 45.0), (-30.0, 30.0), [397, 398, 399, 400]).unwrap();
        assert_eq!(
            area_axis(HAND_PAN, (-45.0, 45.0), YAW_LIMITS, [397, 398]).unwrap(),
            [hand[0], hand[1]]
        );
        assert_eq!(
            area_axis(HAND_PITCH, (-30.0, 30.0), PITCH_LIMITS, [399, 400]).unwrap(),
            [hand[2], hand[3]]
        );
        // the checks of the whole area, per axis
        assert!(
            area_axis(ZONE_PAN, (10.0, 10.0), YAW_LIMITS, [0, 1]).is_err(),
            "equal"
        );
        assert!(
            area_axis(ZONE_PAN, (10.0, -10.0), YAW_LIMITS, [0, 1]).is_err(),
            "inverted"
        );
        assert!(area_axis(ZONE_PAN, (-150.5, 0.0), YAW_LIMITS, [0, 1]).is_err());
        assert!(area_axis(ZONE_PITCH, (-90.0, 70.5), PITCH_LIMITS, [0, 1]).is_err());
        assert!(area_axis(ZONE_PITCH, (-90.0, 70.0), PITCH_LIMITS, [0, 1]).is_ok());
        assert!(area_axis(ZONE_PAN, (f32::NAN, 1.0), YAW_LIMITS, [0, 1]).is_err());
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --test tiny2lite_write`
Expected: FAIL: every test panics with `not sent: <leaf> is not writable` (the stub of A-9); `cargo test -p obsbot-device --lib commands` fails to compile with `cannot find function area_axis`.

- [ ] **Step 3: Write the area builder and the writes**

In `crates/obsbot-device/src/models/tiny2lite/commands.rs`, insert before `hand_area_reset` (the checks and bytes are those of the carried `area`, for one axis; `area`, `zone_area` and `hand_area` stay unchanged, so the byte guard does not move):

```rust
/// The frame ids of one axis of an area: the minimum, then the maximum (capture 11).
pub const ZONE_PAN: [u16; 2] = [0x4c04, 0x4c84];
pub const ZONE_PITCH: [u16; 2] = [0x4d04, 0x4d84];
pub const HAND_PAN: [u16; 2] = [0x2044, 0x20c4];
pub const HAND_PITCH: [u16; 2] = [0x2144, 0x21c4];
/// The limits of an area's yaw and pitch (the gimbal range in the camera's angles).
pub const YAW_LIMITS: (f32, f32) = (-GIMBAL_PAN_LIMIT, GIMBAL_PAN_LIMIT);
pub const PITCH_LIMITS: (f32, f32) = (-GIMBAL_TILT_MAX, -GIMBAL_TILT_MIN);

/// One axis of the zone or hand area: its two frames, minimum then maximum (the order for a
/// new range wholly beyond the stored one is U-8). The check and the bytes are those of the
/// axis in [`zone_area`] and [`hand_area`]: finite, minimum below maximum, within `limits`.
pub fn area_axis(
    ids: [u16; 2],
    range: (f32, f32),
    limits: (f32, f32),
    seqs: [u16; 2],
) -> Result<[[u8; FRAME_SIZE]; 2]> {
    let (lo, hi) = range;
    if !(lo.is_finite() && hi.is_finite() && lo < hi && lo >= limits.0 && hi <= limits.1) {
        return Err(invalid(&format!(
            "area: min must be below max and within {}..{}",
            limits.0, limits.1
        )));
    }
    Ok([
        frame(TRACKING, ids[0], &lo.to_le_bytes(), seqs[0]),
        frame(TRACKING, ids[1], &hi.to_le_bytes(), seqs[1]),
    ])
}
```

Replace `crates/obsbot-device/src/models/tiny2lite/write.rs` with (each arm carried from the setter of `camera.rs` named in the field ledger's write column, minus the remembered state and the automatic controls 0.13 switched first):

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test tiny2lite_write && cargo test -p obsbot-device --lib commands::tests::one_area_axis`
Expected: PASS: `test result: ok. 11 passed` and `1 passed`.

Run: `cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: every test result ok; the byte guard unchanged; no clippy output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/models/tiny2lite/write.rs crates/obsbot-device/src/models/tiny2lite/commands.rs crates/obsbot-device/tests/tiny2lite_write.rs
git commit -m "tiny2lite: every writable leaf sends the frames of its ledger row and nothing else"
```

### Task A-11: Tiny 2 Lite actions and checks

**Files:**

- Modify: `crates/obsbot-device/src/models/tiny2lite/act.rs` (replace the stub)
- Test: `crates/obsbot-device/tests/tiny2lite_act.rs`

**Interfaces:**

- Consumes: A-9 `model`, `tests/support`; A-2 `Link`; A-4 `decode::zoom_factor`; A-7 `plan::{FRAMED, GIMBAL, SHORT, TRACKING}`; stage G builders (`commands`: `gimbal_pose`, `gimbal_pose_speed`, `gimbal_rate`, `gimbal_stop`, `zoom_to`, `zoom_stop`, `zone_return_go`, `hand_return_go`, `hand_return_reset`, `hand_area_reset`, `focus_point`, `startup_go`, `sleep`, `factory_reset`; `presets`: `create`, `update`, `rename`, `delete`, `recall`, `view_add`, `view_rename`, `view_remove`, `view_go`, `free_view_id`, `zone_return_set`, `hand_return_set`, `startup_pose`, `startup_with_ai`, `MAX_ZOOM`, `Pose`).
- Produces: `act::check(action, &Params, &State) -> Result<(), Refusal>`, `act::act(&mut Link, action, &Params, &State, &Ctx) -> Result<Option<Params>>` for all 27 actions; `act::ZOOM_MOVE_SPEED: [(&str, i32); 3]`; `act::tilt_to_pitch(Option<f32>) -> Option<f32>` (carried from `camera.rs`); `act::live_pose(&mut Link, &Ctx) -> Result<Pose>`.

What changes against the 0.13 flows of `camera.rs`: `gimbal`, `gimbal_rate` lose the short command `16 02 00 00` before the move (spec 2, 15.1 row 88) and the remembered pose; `save_preset` sends no default name (15.1 row 90), checks the name before the first frame (audit A-027) and takes the live pose (audit A-022); `set_startup` writes back the block it read instead of a fresh one (spec 4.4); `recall_preset`, `zone_view_go`, `factory_reset` lose the remembered state; `gimbal_center` is not carried (PX-1).

- [ ] **Step 1: Write the failing tests**

Create `crates/obsbot-device/tests/tiny2lite_act.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite model's actions and checks on a recorded camera (replay transport): each
//! action sends the frames of its ledger row and nothing else (spec 2, 15.1 rows 88 and 90).

mod support;

use obsbot_device::contract::{Camera, CameraError, Ctx};
use obsbot_proto::{ErrorCode, Params, State};
use serde_json::{Value, json};
use support::*;

fn params(v: Value) -> Params {
    v.as_object().unwrap().clone()
}

/// The published state the actions see: one preset in slot 0, one view with id 0, mirror and
/// invert off.
fn state() -> State {
    let mut s = State::new(true);
    s.set(
        "presets",
        json!([{"slot": 0, "name": "Desk", "pose": {"pan": 12.5, "tilt": 3.3, "zoom": 1.5}}]),
    );
    s.set(
        "views",
        json!([{"id": 0, "name": "Bereich1", "pose": {"pan": 0.2, "tilt": 0, "zoom": 1.48}}]),
    );
    s.set("image.mirror", json!(false));
    s.set("gimbal.invert", json!(false));
    s
}

fn run(
    action: &str,
    p: Value,
    st: &State,
) -> (
    Option<Params>,
    Vec<obsbot_device::contract::Frame>,
    Vec<Vec<u8>>,
) {
    let (mut cam, log) = model(recorded());
    let p = params(p);
    cam.check(action, &p, st).unwrap();
    let result = cam.act(action, &p, st, &Ctx::none()).unwrap();
    (result, commands(&log), payloads(&log))
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn a_gimbal_move_sends_only_the_move() {
    // spec 2 and 15.1 row 88: no short 16 02 (tracking static) before the move
    let (_, frames, data) = run("gimbal.move", json!({"pan": 100, "tilt": -30}), &state());
    assert_eq!(frames, [framed(0x6444)]);
    // roll kept (-1000), pitch +30 (tilt -30 is down), yaw 100
    assert_eq!(data, [floats(&[-1000.0, 30.0, 100.0])]);
    // a missing axis keeps its position (4.4): -1000 in its place
    let (_, _, data) = run("gimbal.move", json!({"tilt": 20}), &state());
    assert_eq!(data, [floats(&[-1000.0, -20.0, -1000.0])]);
    // with speed: 0x6544, three speeds then the targets
    let (_, frames, data) = run(
        "gimbal.move",
        json!({"pan": 20, "tilt": 10, "speed": 5}),
        &state(),
    );
    assert_eq!(frames, [framed(0x6544)]);
    assert_eq!(data, [floats(&[5.0, 5.0, 5.0, -1000.0, -10.0, 20.0])]);
}

#[test]
fn a_gimbal_move_needs_an_axis() {
    let (cam, _) = model(recorded());
    let refused = cam
        .check("gimbal.move", &params(json!({"speed": 5})), &state())
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::Invalid);
}

#[test]
fn a_gimbal_turn_sends_only_the_turn_with_the_yaw_sign_of_mirror_xor_invert() {
    let (_, frames, data) = run(
        "gimbal.turn",
        json!({"pan_speed": 5, "keep": true}),
        &state(),
    );
    assert_eq!(
        frames,
        [framed(0x6484)],
        "no tracking switch before it (15.1 row 88)"
    );
    // mirror and invert off: a positive pan speed is a negative yaw on the wire (#98)
    assert_eq!(data, [floats(&[0.0, 0.0, -5.0])]);
    let mut mirrored = state();
    mirrored.set("image.mirror", json!(true));
    let (_, _, data) = run(
        "gimbal.turn",
        json!({"pan_speed": 5, "tilt_speed": 3}),
        &mirrored,
    );
    assert_eq!(data, [floats(&[0.0, -3.0, 5.0])]);
    let mut both = mirrored.clone();
    both.set("gimbal.invert", json!(true));
    let (_, _, data) = run("gimbal.turn", json!({"pan_speed": 5}), &both);
    assert_eq!(data, [floats(&[0.0, 0.0, -5.0])]);
}

#[test]
fn a_gimbal_turn_is_refused_until_mirror_and_invert_are_read() {
    let (cam, _) = model(recorded());
    let mut unread = state();
    unread.set("gimbal.invert", Value::Null);
    let refused = cam
        .check("gimbal.turn", &params(json!({"pan_speed": 5})), &unread)
        .unwrap_err();
    assert_eq!(
        (refused.code, refused.path.as_deref()),
        (ErrorCode::Conflict, Some("gimbal.invert"))
    );
    let refused = cam
        .check("gimbal.turn", &params(json!({"keep": true})), &state())
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::Invalid, "no speed at all");
}

#[test]
fn stops_and_zoom_moves() {
    let (_, frames, _) = run("gimbal.stop", json!({}), &state());
    assert_eq!(frames, [framed(0x6704)]);
    let (_, frames, _) = run("zoom.stop", json!({}), &state());
    assert_eq!(frames, [framed(0x68c4)]);
    for (speed, raw) in [("slow", 10i32), ("medium", 30), ("fast", 50)] {
        let (_, frames, data) = run(
            "zoom.move",
            json!({"factor": 2.5, "speed": speed}),
            &state(),
        );
        assert_eq!(frames, [framed(0x6884)]);
        let mut want = raw.to_le_bytes().to_vec();
        want.extend(2.5f32.to_le_bytes());
        assert_eq!(data, [want], "{speed}");
    }
}

#[test]
fn a_preset_saved_without_a_name_gets_none() {
    // 15.1 row 90: one frame, no rename; slot 1 is empty, so it is created at the live pose
    let (_, frames, data) = run("preset.save", json!({"slot": 1}), &state());
    assert_eq!(frames, [framed(0x3944)]);
    let mut want = 1u32.to_le_bytes().to_vec();
    // live pose of the recording: pan 25, tilt 10 up (pitch -10), zoom 1.6
    want.extend(floats(&[25.0, -10.0, 0.0, 1.6, -1000.0]));
    assert_eq!(data, [want]);
}

#[test]
fn a_preset_saved_with_a_name_is_saved_then_renamed() {
    // slot 0 holds a preset: the update frame, then the name
    let (_, frames, data) = run(
        "preset.save",
        json!({"slot": 0, "name": "Door", "pose": {"pan": 1, "tilt": 2, "zoom": 1}}),
        &state(),
    );
    assert_eq!(frames, [framed(0x3e04), framed(0x3a84)]);
    let mut name = 0u32.to_le_bytes().to_vec();
    name.extend(b"Door");
    assert_eq!(data[1], name);
}

#[test]
fn a_name_the_camera_cannot_take_stops_the_save_before_any_frame() {
    // audit: the pose went out before the name was checked
    let (mut cam, log) = model(recorded());
    let p = params(json!({"slot": 0, "name": "Küche"}));
    assert!(cam.act("preset.save", &p, &state(), &Ctx::none()).is_err());
    assert!(log.sent().is_empty(), "not even the live pose was read");
    let refused = cam
        .check(
            "preset.save",
            &params(json!({"slot": 0, "name": ""})),
            &state(),
        )
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::OutOfRange);
}

#[test]
fn slot_and_id_actions_are_refused_for_an_empty_slot_an_unknown_id_or_an_unread_list() {
    let (cam, _) = model(recorded());
    let conflict =
        |action: &str, p: Value, st: &State| cam.check(action, &params(p), st).unwrap_err().code;
    for action in ["preset.recall", "preset.delete", "power_on.use_preset"] {
        assert_eq!(
            conflict(action, json!({"slot": 2}), &state()),
            ErrorCode::Conflict,
            "{action}"
        );
    }
    assert_eq!(
        conflict("preset.rename", json!({"slot": 2, "name": "x"}), &state()),
        ErrorCode::Conflict
    );
    for action in ["view.go", "view.delete"] {
        assert_eq!(
            conflict(action, json!({"id": 5}), &state()),
            ErrorCode::Conflict
        );
    }
    let mut unread = state();
    unread.unread.push("presets".into());
    unread.unread.push("views".into());
    assert_eq!(
        conflict("preset.save", json!({"slot": 0}), &unread),
        ErrorCode::Conflict
    );
    assert_eq!(
        conflict("preset.recall", json!({"slot": 0}), &unread),
        ErrorCode::Conflict
    );
    assert_eq!(
        conflict("view.save", json!({"name": "x"}), &unread),
        ErrorCode::Conflict
    );
    // a used slot and a known id pass
    assert!(
        cam.check("preset.recall", &params(json!({"slot": 0})), &state())
            .is_ok()
    );
    assert!(
        cam.check("view.go", &params(json!({"id": 0})), &state())
            .is_ok()
    );
}

#[test]
fn preset_actions() {
    let (_, frames, data) = run("preset.recall", json!({"slot": 0}), &state());
    assert_eq!(frames, [framed(0x39c4)]);
    let mut want = 0u32.to_le_bytes().to_vec();
    want.extend(floats(&[1.0, 1.0, 1.0, 1.0]));
    assert_eq!(data, [want]);
    let (_, frames, _) = run(
        "preset.rename",
        json!({"slot": 0, "name": "Desk"}),
        &state(),
    );
    assert_eq!(frames, [framed(0x3a84)]);
    let (_, frames, data) = run("preset.delete", json!({"slot": 0}), &state());
    assert_eq!(frames, [framed(0x3984), framed(0x3d84)]);
    assert_eq!(data[0], 0u32.to_le_bytes());
}

#[test]
fn a_view_is_saved_under_the_lowest_free_id_and_named() {
    let (result, frames, data) = run("view.save", json!({"name": "Door"}), &state());
    assert_eq!(result, Some(params(json!({"id": 1}))));
    assert_eq!(frames, [framed(0x1384), framed(0x1544)]);
    assert_eq!(data[0][..4], 1u32.to_le_bytes());
    let mut name = 1u32.to_le_bytes().to_vec();
    name.extend(b"Door");
    assert_eq!(data[1], name);
    let (_, frames, _) = run("view.go", json!({"id": 0}), &state());
    assert_eq!(frames, [framed(0x1604)]);
    let (_, frames, _) = run("view.rename", json!({"id": 0, "name": "Desk"}), &state());
    assert_eq!(frames, [framed(0x1544)]);
    let (_, frames, _) = run("view.delete", json!({"id": 0}), &state());
    assert_eq!(frames, [framed(0x13c4)]);
}

#[test]
fn return_poses_go_returns_and_the_hand_area_reset() {
    let (_, frames, data) = run("zone.return_here", json!({}), &state());
    assert_eq!(frames, [framed(0x4e84)]);
    let mut want = 0u32.to_le_bytes().to_vec();
    want.extend(floats(&[25.0, -10.0, 0.0, 1.6, 0.0]));
    assert_eq!(data, [want.clone()]);
    let (_, frames, data) = run("hand.return_here", json!({}), &state());
    assert_eq!(frames, [framed(0x24c4)]);
    assert_eq!(data, [want]);
    for (action, command) in [
        ("zone.go_return", 0x4f44),
        ("hand.go_return", 0x2584),
        ("hand.return_clear", 0x2544),
    ] {
        let (_, frames, data) = run(action, json!({}), &state());
        assert_eq!(frames, [framed(command)], "{action}");
        assert_eq!(data, [Vec::<u8>::new()], "{action}: no data");
    }
    let (_, frames, _) = run("hand.area_reset", json!({}), &state());
    assert_eq!(
        frames,
        [
            framed(0x25c4),
            framed(0x2604),
            framed(0x2644),
            framed(0x2684)
        ]
    );
}

#[test]
fn the_start_pose_is_set_and_the_block_written_back_unchanged() {
    let block =
        hex("00ffffffffffffff80ffffffffffffff00000000ffffffff00000000000000000000000000000000");
    let (_, frames, data) = run(
        "power_on.set_pose",
        json!({"pose": {"pan": -3, "tilt": 17.1, "zoom": 1}}),
        &state(),
    );
    assert_eq!(frames, [framed(0x3ec4), framed(0x3e44)]);
    let mut pose = 3u32.to_le_bytes().to_vec();
    pose.extend(floats(&[-3.0, -17.1, 0.0, 1.0, 0.0]));
    assert_eq!(
        data,
        [pose, block.clone()],
        "the start mode and the settings stay (4.4)"
    );
    let (_, frames, data) = run("power_on.use_current", json!({}), &state());
    assert_eq!(frames, [framed(0x3ec4), framed(0x3e44)]);
    assert_eq!(data[1], block);
    let (_, frames, data) = run("power_on.use_preset", json!({"slot": 0}), &state());
    assert_eq!(frames, [framed(0x3ec4), framed(0x3e44)]);
    let mut pose = 0u32.to_le_bytes().to_vec();
    pose.extend(floats(&[12.5, -3.3, 0.0, 1.5, 0.0]));
    assert_eq!(
        data[0], pose,
        "the slot's pose from the presets list, lead = slot"
    );
    let (_, frames, data) = run("power_on.go", json!({}), &state());
    assert_eq!(
        (frames, data),
        (vec![short(0x00)], vec![vec![0x00, 0x01, 0x01]])
    );
}

#[test]
fn a_short_initial_state_block_stops_the_start_pose_before_any_frame() {
    let (mut cam, log) = model(recorded().answer(0x04, 0x3e84, &[0; 10]));
    let p = params(json!({"pose": {"pan": 0, "tilt": 0, "zoom": 1}}));
    assert!(matches!(
        cam.act("power_on.set_pose", &p, &state(), &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(commands(&log).is_empty());
}

#[test]
fn sleep_wake_focus_point_and_factory_reset() {
    let (_, frames, data) = run("sleep", json!({}), &state());
    assert_eq!(
        (frames, data),
        (
            vec![framed_to(0x02, 0xa0c2)],
            vec![1u32.to_le_bytes().to_vec()]
        )
    );
    let (_, _, data) = run("wake", json!({}), &state());
    assert_eq!(data, [0u32.to_le_bytes().to_vec()]);
    let (_, frames, data) = run("focus.point", json!({"x": 0.3, "y": 0.4}), &state());
    assert_eq!(frames, [framed_to(0x02, 0x3582)]);
    let mut want = floats(&[0.3, 0.4]);
    for v in [1u32, 100, 100] {
        want.extend(v.to_le_bytes());
    }
    assert_eq!(data, [want]);
    let (_, frames, data) = run("factory_reset", json!({"confirm": true}), &state());
    assert_eq!(
        (frames, data),
        (vec![framed_to(0x02, 0xa802)], vec![vec![1]])
    );
}

#[test]
fn every_action_of_the_description_is_known_to_act() {
    let (mut cam, _) = model(recorded());
    let names: Vec<String> = cam.description().actions.keys().cloned().collect();
    for name in names {
        let r = cam.act(&name, &Params::new(), &state(), &Ctx::none());
        assert!(
            !matches!(&r, Err(CameraError::Io(m)) if m.contains("no action")),
            "{name}: {r:?}"
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --test tiny2lite_act`
Expected: FAIL: the tests panic on `actions are not built yet` (the stub's `check`) or `not sent: no action …`.

- [ ] **Step 3: Write the actions**

Replace `crates/obsbot-device/src/models/tiny2lite/act.rs` with:

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test tiny2lite_act`
Expected: PASS: `test result: ok. 16 passed`.

Mutation check of the open item of stage G (the call site of the tilt negation in the gimbal move): replace `let pitch = tilt_to_pitch(f32_param(params, "tilt"));` by `let pitch = f32_param(params, "tilt");`, run the test file: `a_gimbal_move_sends_only_the_move` fails. Revert; `git diff` shows only this task's files.

Run: `cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: every test result ok; no clippy output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/models/tiny2lite/act.rs crates/obsbot-device/tests/tiny2lite_act.rs
git commit -m "tiny2lite: actions send only their frames: no tracking switch, no invented preset name"
```

### Task A-13: Registry and sysfs bus

**Files:**

- Create: `crates/obsbot-device/src/bus.rs`
- Modify: `crates/obsbot-device/src/registry.rs` (the registry added; the 0.13 detection tables stay until A-14), `crates/obsbot-device/src/lib.rs` (one line)
- Test: `crates/obsbot-device/tests/bus.rs`

**Interfaces:**

- Consumes: A-5 `sysfs::scan`, `tests/sysfs_tree`; A-7 `plan::ENTRY`; A-8 `UsbTransport`; A-9 `Tiny2Lite::checked`; A-2 `process_sequence`, `Timing::CAMERA`; W0 `contract::{Bus, Camera, Found, Lookup, Registry, UsbId}`.
- Produces: `obsbot_device::registry::REGISTRY: Registry` (vendors `[0x3564]`, models `[tiny2lite::plan::ENTRY]`); `obsbot_device::bus::SysfsBus` with `new()` (root `/sys`), `with_root(root: impl Into<PathBuf>)` (another sysfs root, for tests), both with `REGISTRY`; `Default`; and `impl Bus`. Track B's B-2 uses `SysfsBus::new()` and, in its test, `SysfsBus::with_root`.

- [ ] **Step 1: Write the failing tests**

Create `crates/obsbot-device/tests/bus.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The registry (design spec 8.1) and the real bus on a sysfs tree written by the test
//! (5.3, 5.4, 8.4): a registered device is supported, a known vendor with an unknown product is
//! unsupported, any other vendor is not listed; opening sends nothing.

mod sysfs_tree;

use obsbot_device::bus::SysfsBus;
use obsbot_device::contract::{Bus, CameraError, Lookup, UsbId};
use obsbot_device::registry::REGISTRY;
use sysfs_tree::*;

#[test]
fn the_registry_holds_the_vendor_table_and_the_tiny_2_lite_only() {
    assert_eq!(REGISTRY.vendors, [0x3564]);
    let keys: Vec<&str> = REGISTRY.models.iter().map(|m| m.key).collect();
    assert_eq!(keys, ["tiny2lite"]);
    assert!(
        REGISTRY.shared_pairs().is_empty(),
        "no pair belongs to two models (8.1)"
    );
    let at = |vendor, product| REGISTRY.lookup(UsbId { vendor, product });
    assert!(matches!(at(0x3564, 0xfef9), Lookup::Model(m) if m.key == "tiny2lite"));
    assert!(matches!(at(0x3564, 0xfa02), Lookup::Unsupported));
    assert!(matches!(at(0xffff, 0xfef9), Lookup::NotListed));
}

#[test]
fn a_scan_sorts_devices_into_supported_unsupported_and_not_listed() {
    let root = root();
    plug(&root, &LITE);
    plug(
        &root,
        &Device {
            path: "3-3",
            product: "fa02",
            nodes: &[(6, 0, 0)],
            ..LITE
        },
    );
    plug(
        &root,
        &Device {
            path: "1-1",
            vendor: "ffff",
            product: "0001",
            nodes: &[(0, 0, 0)],
            ..LITE
        },
    );
    let bus = SysfsBus::with_root(&root);
    let found = bus.scan();
    let seen: Vec<(&str, Option<&str>)> = found
        .iter()
        .map(|f| (f.usb.path.as_str(), f.model.map(|m| m.key)))
        .collect();
    assert_eq!(seen, [("3-2", Some("tiny2lite")), ("3-3", None)]);
    assert_eq!(found[0].devnum, 7);
    assert_eq!(found[0].video_node.as_deref(), Some("/dev/video2"));
}

#[test]
fn opening_builds_the_model_and_sends_nothing() {
    let root = root();
    plug(&root, &LITE);
    let bus = SysfsBus::with_root(&root);
    let found = bus.scan();
    // /dev/video2 does not exist here: opening must not touch it
    let camera = bus.open(&found[0]).unwrap();
    assert_eq!(camera.description().settings.len(), 84);
    assert!(!camera.send_table().entries.is_empty());
}

#[test]
fn an_unsupported_device_or_one_without_the_extension_unit_is_not_opened() {
    let unknown = root();
    plug(
        &unknown,
        &Device {
            product: "fa02",
            ..LITE
        },
    );
    let bus = SysfsBus::with_root(&unknown);
    let found = bus.scan();
    assert!(matches!(bus.open(&found[0]), Err(CameraError::Io(_))));
    let root2 = root();
    plug(&root2, &LITE);
    let lite = SysfsBus::with_root(&root2);
    let mut f = lite.scan().remove(0);
    f.usb.extension_units.clear();
    let r = lite.open(&f);
    assert!(
        matches!(&r, Err(CameraError::Io(m)) if m.contains("9a1e7291")),
        "{:?}",
        r.err()
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-device --test bus`
Expected: FAIL to compile: `unresolved import obsbot_device::bus`, `cannot find REGISTRY`.

- [ ] **Step 3: Write the registry and the bus**

In `crates/obsbot-device/src/registry.rs`, insert after the module doc comment:

```rust
use crate::contract::Registry;
use crate::models::tiny2lite;

/// The vendor table and the registered models (design spec 8.1): the OBSBOT vendor id read
/// from a unit in hand (`0x3564`, measured on the Tiny 2 Lite) and the Tiny 2 Lite,
/// `3564:fef9`. Nothing else.
pub static REGISTRY: Registry = Registry {
    vendors: &[0x3564],
    models: &[tiny2lite::plan::ENTRY],
};
```

Add `pub mod bus;` to `crates/obsbot-device/src/lib.rs` and create `crates/obsbot-device/src/bus.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The real bus (design spec 5.3, 8.1): one sysfs scan per call, each device looked up in the
//! registry by its USB (vendor id, product id), and the constructor of each registered model.
//! It opens no node while it scans; `open` builds the camera and sends nothing either.

use crate::contract::camera::Result;
use crate::contract::{Bus, Camera, CameraError, Found, Lookup, UsbId};
use crate::enumeration::sysfs;
use crate::family::link::{Timing, process_sequence};
use crate::models::tiny2lite::model::Tiny2Lite;
use crate::registry::REGISTRY;
use crate::transport::usb::UsbTransport;
use std::path::PathBuf;

/// The bus of the real devices, with the registry of the registered models ([`REGISTRY`]).
pub struct SysfsBus {
    root: PathBuf,
}

impl SysfsBus {
    /// The bus on `/sys`.
    pub fn new() -> SysfsBus {
        SysfsBus::with_root("/sys")
    }

    /// A bus on another sysfs root (tests).
    pub fn with_root(root: impl Into<PathBuf>) -> SysfsBus {
        SysfsBus { root: root.into() }
    }
}

impl Default for SysfsBus {
    fn default() -> Self {
        SysfsBus::new()
    }
}

impl Bus for SysfsBus {
    fn scan(&self) -> Vec<Found> {
        sysfs::scan(&self.root)
            .into_iter()
            .filter_map(|d| {
                let model = match REGISTRY.lookup(UsbId::from_usb(&d.usb)?) {
                    Lookup::NotListed => return None,
                    Lookup::Unsupported => None,
                    Lookup::Model(entry) => Some(entry),
                };
                Some(Found {
                    usb: d.usb,
                    devnum: d.devnum,
                    video_node: d.video_node,
                    model,
                })
            })
            .collect()
    }

    fn open(&self, found: &Found) -> Result<Box<dyn Camera>> {
        let entry = found
            .model
            .ok_or_else(|| CameraError::Io("an unsupported device is never opened".into()))?;
        let node = found
            .video_node
            .clone()
            .ok_or_else(|| CameraError::Io("the device has no video node with index 0".into()))?;
        let guid = entry.xu_guid.unwrap_or_default();
        let unit = found
            .usb
            .extension_units
            .iter()
            .find(|u| u.guid == guid)
            .map(|u| u.unit)
            .ok_or_else(|| {
                CameraError::Io(format!("no extension unit {guid} in the USB descriptor"))
            })?;
        let usb_dir = self.root.join("bus/usb/devices").join(&found.usb.path);
        let transport = UsbTransport::new(node, unit, usb_dir);
        match entry.key {
            "tiny2lite" => Ok(Box::new(Tiny2Lite::checked(
                transport,
                process_sequence(),
                Timing::CAMERA,
            ))),
            other => Err(CameraError::Io(format!(
                "no constructor for the model {other}"
            ))),
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test bus`
Expected: PASS: `test result: ok. 4 passed`. `opening_builds_the_model_and_sends_nothing` opens a camera whose `/dev/video2` does not exist: the bus touches no node.

Run: `cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: every test result ok; no clippy output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/registry.rs crates/obsbot-device/src/bus.rs crates/obsbot-device/src/lib.rs crates/obsbot-device/tests/bus.rs
git commit -m "obsbot-device: registry with the vendor table and the sysfs bus"
```

### Task A-12: Send-table test and conformance

**Files:**

- Test: `crates/obsbot-device/tests/tiny2lite_frames.rs`

**Interfaces:**

- Consumes: A-10 `write`, A-11 `act`, A-9 `model`, `tests/support`, A-6 `classify`, A-7 `send_table::ENTRIES`; W0 `conformance::check`, `obsbot_proto::{Bound, Setting, SettingType}`.
- Produces: the test of spec 8.3 and the conformance run of 8.7 step 6 for the Tiny 2 Lite.

This test is what kills the stage-G mutation pass's 86 "needs the device" mutants in their 1.0 form: a model arm whose body sends nothing (the 1.0 counterpart of `replace Camera::set_* with Ok(())`) leaves its ledger row unsent and fails `every_setting_at_its_boundaries_sends_exactly_its_ledger_row` or `every_action_at_its_boundaries_sends_exactly_its_ledger_row`. The enumeration, on-air probe and `ctrl_range` mutants of that pass stay "needs the device" (I/O on a real node); their code is replaced by `sysfs.rs` (tested on a written tree) and `usb.rs`.

- [ ] **Step 1: Write the test**

Create `crates/obsbot-device/tests/tiny2lite_frames.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The send-table test of design spec 8.3 and the trait conformance check (8.7 step 6) for the
//! Tiny 2 Lite, on a recorded camera (replay transport, behind the send-table check):
//!
//! - every setting at its boundary values and every action sends exactly the frames of its
//!   field-ledger row, in that order, and nothing before or after them (spec 2);
//! - every frame the model writes is in its send table (the check refuses anything else, so a
//!   frame outside it fails here), and every entry of the table is used;
//! - the model passes `obsbot_device::contract::conformance::check`.

mod support;

use obsbot_device::contract::conformance;
use obsbot_device::contract::{Camera, Ctx, Frame, Need};
use obsbot_device::models::tiny2lite::model::SELECTORS;
use obsbot_device::models::tiny2lite::send_table;
use obsbot_device::transport::checked::classify;
use obsbot_device::transport::replay::Sent;
use obsbot_proto::{Bound, Params, Setting, SettingType, State};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use support::*;

/// The frames of each writable leaf's ledger row (`docs/field-ledger.md`, column "Write").
fn ledger_row(leaf: &str) -> Vec<Frame> {
    let one = |c: u16| vec![framed(c)];
    match leaf {
        "gimbal.invert" => one(0x3b84),
        "zoom.factor" => vec![uvc(0x009a_090d)],
        "zoom.field_of_view" => vec![short(0x04)],
        "tracking.mode" => vec![short(0x16)],
        "tracking.profile" => one(0x0cc4),
        "tracking.speed" => one(0x0944),
        "framing.auto_zoom" => one(0x0a44),
        "framing.size" => one(0x0ac4),
        "framing.zoom_speed" => one(0x0b44),
        "framing.offset.x" => one(0x0bc4),
        "framing.offset.y" => one(0x0c44),
        "framing.part" => one(0x1584),
        "gestures.target_lock" => one(0x30c4),
        "gestures.zoom" => one(0x3144),
        "gestures.zoom_ratio" => one(0x3244),
        "gestures.dynamic_zoom" => one(0x3344),
        "gestures.direction_change" => one(0x33c4),
        "zone.enabled" => one(0x08c4),
        "zone.follow" => one(0x4e04),
        "zone.area.pan" => vec![framed(0x4c04), framed(0x4c84)],
        "zone.area.tilt" => vec![framed(0x4d04), framed(0x4d84)],
        "hand.side" => one(0x2244),
        "hand.area.pan" => vec![framed(0x2044), framed(0x20c4)],
        "hand.area.tilt" => vec![framed(0x2144), framed(0x21c4)],
        "hand.axes.pan" => one(0x2344),
        "hand.axes.tilt" => one(0x23c4),
        "hand.return_after" => one(0x2444),
        "hand.move_gimbal" => one(0x26c4),
        "exposure.mode" => vec![uvc(0x009a_0901)],
        "exposure.region" => vec![short(0x03)],
        "exposure.time" => vec![uvc(0x009a_0902)],
        "exposure.gain" => vec![uvc(0x0098_0913)],
        "exposure.compensation" => vec![uvc(0x0098_091c)],
        "exposure.hdr" => vec![short(0x01)],
        "exposure.anti_flicker" => vec![uvc(0x0098_0918)],
        "image.brightness" => vec![uvc(0x0098_0900)],
        "image.contrast" => vec![uvc(0x0098_0901)],
        "image.saturation" => vec![uvc(0x0098_0902)],
        "image.hue" => vec![uvc(0x0098_0903)],
        "image.sharpness" => vec![uvc(0x0098_091b)],
        "image.white_balance.auto" => vec![uvc(0x0098_090c)],
        "image.white_balance.temperature" => vec![uvc(0x0098_091a)],
        "image.mirror" => vec![short(0x14)],
        "focus.mode" => vec![framed_to(0x02, 0x3442)],
        "focus.region" => vec![framed_to(0x02, 0x3602)],
        "focus.position" => vec![uvc(0x009a_090a)],
        "audio.enabled" => vec![short(0x1c)],
        "audio.volume" => vec![framed_to(0x02, 0x8102)],
        "audio.noise_reduction" => vec![short(0x0a)],
        "audio.auto_gain" => vec![short(0x17)],
        "audio.in_sleep" => vec![short(0x13)],
        "led.brightness" => vec![short(0x1a)],
        "led.blink" => vec![short(0x18)],
        "sleep.auto.enabled" | "sleep.auto.seconds" => vec![short(0x0b)],
        "sleep.background" => vec![short(0x0e)],
        "sleep.background_mirror" => vec![short(0x0e)],
        "sleep.wake_zoom" => vec![short(0x15)],
        "power_on.mode" => vec![short(0x19)],
        "power_on.start.mode" => one(0x3e44),
        other => panic!("{other}: no ledger row in this test"),
    }
}

fn bound(b: &Option<Bound>, component: Option<&str>) -> Option<f64> {
    b.as_ref().and_then(|b| b.get(component))
}

/// The boundary values of a writable leaf: both bools, every writable enum value, the minimum
/// and the maximum of a number (the minimum and one more where it has no maximum), the widest
/// range.
fn boundaries(s: &Setting) -> Vec<Value> {
    match &s.kind {
        SettingType::Bool => vec![json!(true), json!(false)],
        SettingType::Enum => s.writable.iter().map(|v| json!(v)).collect(),
        SettingType::Number => {
            let lo = bound(&s.min, None).unwrap();
            let hi = bound(&s.max, None).unwrap_or(lo + 1.0);
            vec![json!(lo), json!(hi)]
        }
        SettingType::Range => vec![json!({
            "min": bound(&s.min, Some("min")).unwrap(),
            "max": bound(&s.max, Some("max")).unwrap(),
        })],
        other => panic!("no boundaries for a writable {other:?}"),
    }
}

/// A published state with presets in slots 0 and 2 (slot 1 empty) and view 0.
fn full_state() -> State {
    let mut s = State::new(true);
    let preset =
        |slot: u8| json!({"slot": slot, "name": "x", "pose": {"pan": 0, "tilt": 0, "zoom": 1}});
    s.set("presets", json!([preset(0), preset(2)]));
    s.set(
        "views",
        json!([{"id": 0, "name": "x", "pose": {"pan": 0, "tilt": 0, "zoom": 1}}]),
    );
    s.set("image.mirror", json!(false));
    s.set("gimbal.invert", json!(true));
    s
}

/// Every action at its boundary values, with the frames of its ledger row (table "Actions").
fn action_rows() -> Vec<(&'static str, Value, Vec<Frame>)> {
    let name32 = "N".repeat(32);
    let pose_lo = json!({"pan": -150, "tilt": -70, "zoom": 1});
    let pose_hi = json!({"pan": 150, "tilt": 90, "zoom": 4});
    let mut rows = vec![
        (
            "gimbal.move",
            json!({"pan": -150, "tilt": -70}),
            vec![framed(0x6444)],
        ),
        (
            "gimbal.move",
            json!({"pan": 150, "tilt": 90}),
            vec![framed(0x6444)],
        ),
        (
            "gimbal.move",
            json!({"pan": 0, "speed": 1}),
            vec![framed(0x6544)],
        ),
        (
            "gimbal.move",
            json!({"tilt": 90, "speed": 180}),
            vec![framed(0x6544)],
        ),
        ("gimbal.stop", json!({}), vec![framed(0x6704)]),
        (
            "gimbal.turn",
            json!({"pan_speed": -178, "tilt_speed": 178}),
            vec![framed(0x6484)],
        ),
        (
            "gimbal.turn",
            json!({"pan_speed": 178, "keep": true}),
            vec![framed(0x6484)],
        ),
        ("zoom.stop", json!({}), vec![framed(0x68c4)]),
        ("preset.save", json!({"slot": 0}), vec![framed(0x3e04)]),
        ("preset.save", json!({"slot": 1}), vec![framed(0x3944)]),
        (
            "preset.save",
            json!({"slot": 2, "name": name32, "pose": pose_hi}),
            vec![framed(0x3e04), framed(0x3a84)],
        ),
        ("preset.recall", json!({"slot": 2}), vec![framed(0x39c4)]),
        (
            "preset.rename",
            json!({"slot": 0, "name": name32}),
            vec![framed(0x3a84)],
        ),
        (
            "preset.delete",
            json!({"slot": 2}),
            vec![framed(0x3984), framed(0x3d84)],
        ),
        (
            "view.save",
            json!({"name": "a", "pose": pose_lo}),
            vec![framed(0x1384), framed(0x1544)],
        ),
        ("view.go", json!({"id": 0}), vec![framed(0x1604)]),
        (
            "view.rename",
            json!({"id": 0, "name": name32}),
            vec![framed(0x1544)],
        ),
        ("view.delete", json!({"id": 0}), vec![framed(0x13c4)]),
        ("zone.return_here", json!({}), vec![framed(0x4e84)]),
        ("zone.go_return", json!({}), vec![framed(0x4f44)]),
        ("hand.return_here", json!({}), vec![framed(0x24c4)]),
        ("hand.go_return", json!({}), vec![framed(0x2584)]),
        ("hand.return_clear", json!({}), vec![framed(0x2544)]),
        (
            "hand.area_reset",
            json!({}),
            vec![
                framed(0x25c4),
                framed(0x2604),
                framed(0x2644),
                framed(0x2684),
            ],
        ),
        (
            "focus.point",
            json!({"x": 0, "y": 1}),
            vec![framed_to(0x02, 0x3582)],
        ),
        (
            "power_on.set_pose",
            json!({"pose": pose_lo}),
            vec![framed(0x3ec4), framed(0x3e44)],
        ),
        (
            "power_on.set_pose",
            json!({"pose": pose_hi}),
            vec![framed(0x3ec4), framed(0x3e44)],
        ),
        (
            "power_on.use_current",
            json!({}),
            vec![framed(0x3ec4), framed(0x3e44)],
        ),
        (
            "power_on.use_preset",
            json!({"slot": 2}),
            vec![framed(0x3ec4), framed(0x3e44)],
        ),
        ("power_on.go", json!({}), vec![short(0x00)]),
        ("sleep", json!({}), vec![framed_to(0x02, 0xa0c2)]),
        ("wake", json!({}), vec![framed_to(0x02, 0xa0c2)]),
        (
            "factory_reset",
            json!({"confirm": true}),
            vec![framed_to(0x02, 0xa802)],
        ),
    ];
    for factor in [1.0, 4.0] {
        for speed in ["slow", "medium", "fast"] {
            rows.push((
                "zoom.move",
                json!({"factor": factor, "speed": speed}),
                vec![framed(0x6884)],
            ));
        }
    }
    rows
}

#[test]
fn every_setting_at_its_boundaries_sends_exactly_its_ledger_row() {
    let (cam, _) = model(recorded());
    let settings: Vec<(String, Setting)> = cam
        .description()
        .settings
        .iter()
        .filter(|(_, s)| !s.readonly)
        .map(|(p, s)| (p.clone(), s.clone()))
        .collect();
    assert_eq!(settings.len(), 60, "every writable leaf of 4.3 is driven");
    for (leaf, setting) in settings {
        for value in boundaries(&setting) {
            let (mut cam, log) = model(recorded());
            cam.apply(&leaf, &value, &Ctx::none())
                .unwrap_or_else(|e| panic!("{leaf} = {value}: {e}"));
            assert_eq!(commands(&log), ledger_row(&leaf), "{leaf} = {value}");
        }
    }
}

#[test]
fn every_action_at_its_boundaries_sends_exactly_its_ledger_row() {
    let (cam, _) = model(recorded());
    let described: BTreeSet<String> = cam.description().actions.keys().cloned().collect();
    let rows = action_rows();
    let driven: BTreeSet<String> = rows.iter().map(|(a, _, _)| a.to_string()).collect();
    assert_eq!(driven, described, "every action of 4.4 is driven");
    for (action, params, frames) in rows {
        let (mut cam, log) = model(recorded());
        let p: Params = params.as_object().unwrap().clone();
        let state = full_state();
        cam.check(action, &p, &state)
            .unwrap_or_else(|r| panic!("{action} {params}: {r:?}"));
        cam.act(action, &p, &state, &Ctx::none())
            .unwrap_or_else(|e| panic!("{action} {params}: {e}"));
        assert_eq!(commands(&log), frames, "{action} {params}");
    }
}

#[test]
fn the_send_table_names_exactly_what_the_model_sends() {
    let mut sent: BTreeSet<String> = BTreeSet::new();
    let mut note = |log: &obsbot_device::transport::replay::Log| {
        for s in log.sent() {
            let f = match s {
                Sent::Xu { selector, data } => classify(&SELECTORS, selector, &data).unwrap(),
                Sent::Ctrl { id, .. } => Frame::Uvc { control: id },
            };
            sent.insert(format!("{f:?}"));
        }
    };
    // identity and every group, with the tracking on so that the target is read
    let mut tracking = block(STATUS);
    tracking[0x18] = 2;
    let mut target = vec![1, 1, 1, 1, 0, 0];
    target.extend([0u8; 16]);
    let (mut cam, log) = model(recorded().buffer(6, tracking).answer(0x04, 0x01c4, &target));
    cam.identity(
        Need {
            serial: true,
            firmware: true,
        },
        &Ctx::none(),
    )
    .unwrap();
    for g in cam.poll_plan().to_vec() {
        cam.read(&g.name, &Ctx::none()).unwrap();
    }
    note(&log);
    // every setting and every action
    let (cam, _) = model(recorded());
    let desc = cam.description().clone();
    for (leaf, setting) in desc.settings.iter().filter(|(_, s)| !s.readonly) {
        for value in boundaries(setting) {
            let (mut cam, log) = model(recorded());
            cam.apply(leaf, &value, &Ctx::none()).unwrap();
            note(&log);
        }
    }
    for (action, params, _) in action_rows() {
        let (mut cam, log) = model(recorded());
        cam.act(
            action,
            params.as_object().unwrap(),
            &full_state(),
            &Ctx::none(),
        )
        .unwrap();
        note(&log);
    }
    let table: BTreeSet<String> = send_table::ENTRIES
        .iter()
        .map(|e| format!("{:?}", e.frame))
        .collect();
    let unused: Vec<&String> = table.difference(&sent).collect();
    assert!(
        unused.is_empty(),
        "entries the model never sends: {unused:?}"
    );
    // the check would have refused anything else; this makes it explicit
    assert!(sent.is_subset(&table));
}

#[test]
fn the_model_passes_the_trait_conformance_check() {
    // the tracking on: on a static camera the target group has no read to return before its
    // first one (decision 8), which the check counts as a failed read
    let (mut cam, _) = model(tracking());
    assert_eq!(conformance::check(&mut cam), Vec::<String>::new());
}
```

- [ ] **Step 2: Run it and see it pass; then see it fail on a wrong frame**

Run: `cargo test -p obsbot-device --test tiny2lite_frames`
Expected: PASS: `test result: ok. 4 passed`.

To see the test can fail: in `write.rs` change `"exposure.hdr" => link.short(SHORT, &cmd::hdr(boolean(leaf, v)?), ctx),` to send `cmd::mirror` instead of `cmd::hdr`; the first test fails with `exposure.hdr = true` and the frames `[Short { selector: 6, command: 20 }]`. In `act.rs` add `link.short(SHORT, &cmd::ai_mode(cmd::AiMode::Static), ctx)?;` as the first line of the `"gimbal.move"` arm (0.13's switch); `every_action_at_its_boundaries_sends_exactly_its_ledger_row` fails on `gimbal.move`. Revert both; `git status` shows only the new test file.

Optional, not a gate (spec 11: "Mutation tests are not a 1.0 gate"): `cargo mutants --package obsbot-device --file crates/obsbot-device/src/models/tiny2lite/write.rs --file crates/obsbot-device/src/models/tiny2lite/act.rs --test-workspace=true` with the target and output directories outside the repository; record the result in the pull request.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add crates/obsbot-device/tests/tiny2lite_frames.rs
git commit -m "tiny2lite: send-table test of every setting and action, conformance check"
```

### Task A-15: Live tests for CS-1

**Files:**

- Test: `crates/obsbot-device/tests/live.rs`

**Interfaces:**

- Consumes: A-13 `SysfsBus`; A-8 `UsbTransport`; A-6 `Checked`; A-2 `Link`, `Timing::CAMERA`, `process_sequence`; A-7 `plan::XU_GUID`, `send_table::table`; A-9 `model::SELECTORS`; A-4 `decode::UVC`; A-10 `Camera::apply`; A-11 `Camera::check`, `Camera::act`; W0 `obsbot_proto::{Description, Setting, SettingType, State, Params, is_valid_serial, short_id, values::{same, number, rounded}}`.
- Produces: thirteen `#[ignore]` tests run in CS-1: `live_cam_25_the_sysfs_facts`, `live_cam_1_the_identity_queries_answer`, `live_cam_10_cam_20_a_cold_read_of_every_group`, `live_the_uvc_ranges_the_description_holds` (the W0 plan's decision 12, `UNCONFIRMED`), and with `OBSBOT_LIVE_WRITE=1` `live_cam_23_the_u_rows_that_read_back` (U-4, U-5, U-6, U-18), `live_cam_16_an_area_wholly_beyond_the_stored_one` (U-8), `live_cam_5_every_writable_leaf_reads_back_and_its_lag` (`settle_ms`), `live_cam_2_cam_3_cam_22_asleep` (groups, writes and actions asleep, kept after the wake, the time to awake, the live pose asleep), `live_cam_4_the_zoom_readback_during_a_slow_zoom_move`, `live_cam_13_anti_flicker_and_exposure_mode_in_both_orders`, `live_cam_15_a_manual_value_while_its_automatic_control_is_on`, `live_cam_17_gimbal_moves_with_speed` (U-9) and `live_cam_35_a_preset_saved_without_a_name`. Every frame goes through the model or the checked link, so only the send table is sent; the link finds the extension unit by the registry entry's GUID. A test that writes puts back the value each leaf read before; a value it cannot write back (a `raw:<n>` or `null` read, or a write that fails) is left with a printed warning, never a panic. A preset or view a test saves goes into a free slot or a new id and is deleted again; with no free preset slot the preset tests write nothing. The serial is printed only shortened.

- [ ] **Step 1: Write the tests**

Create `crates/obsbot-device/tests/live.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Live tests on a connected Tiny 2 Lite (design spec 10.3 track A, 11, 14: camera session
//! CS-1). Every test is `#[ignore]` and does nothing unless `OBSBOT_LIVE=1` is set; the tests
//! that write also need `OBSBOT_LIVE_WRITE=1` and put back what they wrote. They talk to the
//! camera only through the model, behind its send-table check, so they send only the model's
//! send table (8.3). Stop `obsbot4linux.socket` and `obsbot4linux.service` first. Run one at a
//! time:
//!
//! `OBSBOT_LIVE=1 cargo test -p obsbot-device --test live -- --ignored --nocapture --test-threads=1`
//!
//! The serial is never printed (4.2): only `short_id` of it.

use obsbot_device::bus::SysfsBus;
use obsbot_device::contract::{Bus, Camera, CameraError, Ctx, Found, Need};
use obsbot_device::family::link::{Link, Timing, process_sequence};
use obsbot_device::models::tiny2lite::model::SELECTORS;
use obsbot_device::models::tiny2lite::{decode, plan, send_table};
use obsbot_device::transport::checked::Checked;
use obsbot_device::transport::usb::UsbTransport;
use obsbot_proto::{
    Bound, Description, Params, Setting, SettingType, State, is_valid_serial, short_id, values,
};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn enabled(var: &str) -> bool {
    std::env::var(var).as_deref() == Ok("1")
}

/// The first Tiny 2 Lite the bus finds, opened; `None` (and a line saying why) when the live
/// tests are not enabled or no camera is there.
fn camera(write: bool) -> Option<(Found, Box<dyn Camera>)> {
    if !enabled("OBSBOT_LIVE") || (write && !enabled("OBSBOT_LIVE_WRITE")) {
        eprintln!("skipped: set OBSBOT_LIVE=1 (and OBSBOT_LIVE_WRITE=1 for writes)");
        return None;
    }
    let bus = SysfsBus::new();
    let found = bus
        .scan()
        .into_iter()
        .find(|f| f.model.is_some_and(|m| m.key == "tiny2lite"))
        .expect("a connected Tiny 2 Lite (USB 3564:fef9)");
    let camera = bus.open(&found).expect("open");
    Some((found, camera))
}

/// A raw link to the camera behind the send-table check, for reads the model does not expose.
/// The extension unit is the one with the registry entry's GUID (8.3).
fn link(found: &Found) -> Link {
    let unit = found
        .usb
        .extension_units
        .iter()
        .find(|u| u.guid == plan::XU_GUID)
        .expect("the extension unit of the registry entry")
        .unit;
    let usb_dir = std::path::Path::new("/sys/bus/usb/devices").join(&found.usb.path);
    let t = UsbTransport::new(found.video_node.clone().unwrap(), unit, usb_dir);
    Link::new(
        Box::new(Checked::new(t, send_table::table(), SELECTORS)),
        process_sequence(),
        Timing::CAMERA,
    )
}

/// The group of `leaf` in the model's poll plan.
fn group_of(cam: &dyn Camera, leaf: &str) -> String {
    cam.poll_plan()
        .iter()
        .find(|g| g.leaves.iter().any(|l| l == leaf))
        .unwrap_or_else(|| panic!("{leaf} is in no group"))
        .name
        .clone()
}

/// `leaf` as one read of its group shows it.
fn read_leaf(cam: &mut dyn Camera, leaf: &str) -> Result<Value, CameraError> {
    let group = group_of(cam, leaf);
    Ok(cam.read(&group, &Ctx::none())?[leaf].clone())
}

/// The state after one read of every group, as the daemon publishes it (a group that fails
/// puts its leaves in `unread`): what the checks and the actions see.
fn state(cam: &mut dyn Camera) -> State {
    let mut s = State::new(true);
    for g in cam.poll_plan().to_vec() {
        match cam.read(&g.name, &Ctx::none()) {
            Ok(values) => values.into_iter().for_each(|(leaf, v)| s.set(&leaf, v)),
            Err(_) => s.unread.extend(g.leaves.iter().cloned()),
        }
    }
    s
}

/// `action` as the daemon runs it: the model's check, then the action; the error as text.
fn act(cam: &mut dyn Camera, action: &str, p: Value, st: &State) -> Result<Option<Params>, String> {
    let p = p.as_object().cloned().unwrap_or_default();
    cam.check(action, &p, st).map_err(|r| r.message)?;
    cam.act(action, &p, st, &Ctx::none())
        .map_err(|e| e.to_string())
}

/// The bound, step or default `key` of a setting (`min`, `max`, `step`, `default`).
fn number(s: &Setting, key: &str) -> Option<f64> {
    let b = |b: &Option<Bound>| b.as_ref().and_then(|b| b.get(None));
    match key {
        "min" => b(&s.min),
        "max" => b(&s.max),
        "step" => b(&s.step),
        _ => s.default.as_ref().and_then(Value::as_f64),
    }
}

/// A valid value of a writable leaf next to `before`, from its description: the other bool,
/// another writable enum value, a number one step (or a tenth of its range) up, or down at the
/// top. `None` for the other types (the area ranges are CAM-16's) and for an undecoded value.
fn other_value(s: &Setting, before: &Value) -> Option<Value> {
    match s.kind {
        SettingType::Bool => Some(json!(!before.as_bool()?)),
        SettingType::Enum => s
            .writable
            .iter()
            .find(|w| before.as_str() != Some(w.as_str()))
            .map(|w| json!(w)),
        SettingType::Number => {
            let b = before.as_f64()?;
            let (min, max) = (number(s, "min"), number(s, "max"));
            let step = match (number(s, "step"), min, max) {
                (Some(step), _, _) => step,
                (None, Some(lo), Some(hi)) => (hi - lo) / 10.0,
                _ => 1.0,
            };
            let up = b + step;
            let v = if max.is_none_or(|hi| up <= hi) {
                up
            } else {
                b - step
            };
            Some(match s.precision.as_ref().and_then(|p| p.get(None)) {
                Some(p) => values::rounded(v, p),
                None => values::number(v),
            })
        }
        _ => None,
    }
}

/// Leaves the sweeps leave alone: switching the microphone off can take the USB audio
/// interface away (CAM-8, a step of the session).
const NOT_SWEPT: [&str; 1] = ["audio.enabled"];

/// The writable bool, enum and number leaves with a readback, in the model's apply order.
fn swept(cam: &dyn Camera) -> Vec<String> {
    let desc = cam.description();
    let leaves: Vec<&str> = desc
        .settings
        .iter()
        .filter(|(path, s)| {
            !s.readonly
                && s.readback
                && !NOT_SWEPT.contains(&path.as_str())
                && matches!(
                    s.kind,
                    SettingType::Bool | SettingType::Enum | SettingType::Number
                )
        })
        .map(|(path, _)| path.as_str())
        .collect();
    cam.order(&leaves)
}

/// Puts `leaf` back to `before`. A value that cannot be written (a `raw:<n>` or a `null` read)
/// or a write that fails leaves the leaf as the test wrote it, with a warning.
fn restore(cam: &mut dyn Camera, leaf: &str, before: &Value) {
    let raw = before.as_str().is_some_and(|s| s.starts_with("raw:"));
    let result = if before.is_null() || raw {
        Err(format!("it read {before} before the test"))
    } else {
        cam.apply(leaf, before, &Ctx::none())
            .map_err(|e| e.to_string())
    };
    if let Err(why) = result {
        eprintln!("WARNING: {leaf} not put back: {why}");
    }
}

/// Writes the `requires` values of `s` and returns what those leaves read before.
fn meet(cam: &mut dyn Camera, s: &Setting) -> Vec<(String, Value)> {
    let mut before = Vec::new();
    for r in &s.requires {
        if let Ok(v) = read_leaf(cam, &r.path) {
            before.push((r.path.clone(), v));
        }
        if let Err(e) = cam.apply(&r.path, &r.equals, &Ctx::none()) {
            eprintln!("WARNING: {} = {} not written: {e}", r.path, r.equals);
        }
    }
    before
}

/// The preset in `slot` of a state, when it is listed.
fn preset(st: &State, slot: u64) -> Option<&Value> {
    st.at("presets")?
        .as_array()?
        .iter()
        .find(|p| p.get("slot") == Some(&json!(slot)))
}

/// The lowest preset slot that holds no preset; `None` when all are taken or the list is not
/// read (no preset of the owner is overwritten).
fn free_preset_slot(desc: &Description, st: &State) -> Option<u64> {
    st.at("presets")?;
    let slots = u64::from(*desc.limits.get("presets")?);
    (0..slots).find(|slot| preset(st, *slot).is_none())
}

/// Reads the status group until `asleep` is `want`, every 500 ms for at most 20 s; how long it
/// took.
fn until_asleep_is(cam: &mut dyn Camera, want: bool) -> Option<Duration> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        if read_leaf(cam, "asleep").ok() == Some(json!(want)) {
            return Some(start.elapsed());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    None
}

#[test]
#[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
fn live_cam_25_the_sysfs_facts() {
    let Some((found, _)) = camera(false) else {
        return;
    };
    println!("{}", serde_json::to_string_pretty(&found.usb).unwrap());
    println!("devnum {} video node {:?}", found.devnum, found.video_node);
    assert_eq!(
        (found.usb.vendor.as_str(), found.usb.product.as_str()),
        ("3564", "fef9")
    );
    assert!(found.video_node.is_some(), "a node with index 0");
    assert!(
        found
            .usb
            .extension_units
            .iter()
            .any(|u| u.guid == plan::XU_GUID),
        "the extension unit of the registry entry"
    );
}

#[test]
#[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
fn live_cam_1_the_identity_queries_answer() {
    let Some((_, mut cam)) = camera(false) else {
        return;
    };
    let start = Instant::now();
    let id = cam
        .identity(
            Need {
                serial: true,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    println!("identity attempt took {:?}", start.elapsed());
    let serial = id.serial.expect("a serial answer");
    println!("serial {} firmware {:?}", short_id(&serial), id.firmware);
    assert!(is_valid_serial(&serial), "the serial format of 5.5");
    assert_eq!(serial.len(), 14, "14 ASCII characters (8.8)");
}

#[test]
#[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
fn live_cam_10_cam_20_a_cold_read_of_every_group() {
    let Some((found, mut cam)) = camera(false) else {
        return;
    };
    // CAM-20: the first byte of the status block (0 while the camera boots?)
    let block = link(&found).read(6, &Ctx::none()).unwrap();
    println!("status block byte 0: {:#04x}", block[0]);
    let mut total = Duration::ZERO;
    for g in cam.poll_plan().to_vec() {
        let start = Instant::now();
        let r = cam.read(&g.name, &Ctx::none());
        let took = start.elapsed();
        total += took;
        match r {
            Ok(v) => println!("{:<12} {:>6} ms  {}", g.name, took.as_millis(), json!(v)),
            Err(e) => println!("{:<12} {:>6} ms  FAILED: {e}", g.name, took.as_millis()),
        }
    }
    println!(
        "a cold read of every group: {} ms (CAM-10)",
        total.as_millis()
    );
}

#[test]
#[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
fn live_the_uvc_ranges_the_description_holds() {
    // the W0 plan's decision 12: these bounds, steps and defaults are not backed yet; the camera
    // reports its own (a read). A mismatch is fixed by a contract-change PR of the description.
    let Some((found, cam)) = camera(false) else {
        return;
    };
    let desc = cam.description().clone();
    let mut link = link(&found);
    let mut mismatches = Vec::new();
    for (leaf, id) in decode::UVC {
        let range = link.transport().ctrl_range(id).unwrap();
        let s = &desc.settings[leaf];
        println!("{leaf:<34} camera {range:?}");
        let Some(r) = range else {
            mismatches.push(format!("{leaf}: the camera has no control {id:#010x}"));
            continue;
        };
        // exposure.time is in 100 µs units on the camera, µs in the description
        let scale = if leaf == "exposure.time" { 100.0 } else { 1.0 };
        for (key, camera) in [
            ("min", r.min),
            ("max", r.max),
            ("step", r.step),
            ("default", r.default),
        ] {
            if let Some(ours) = number(s, key)
                && (ours - f64::from(camera) * scale).abs() > 1e-9
            {
                mismatches.push(format!(
                    "{leaf}#{key}: description {ours}, camera {}",
                    f64::from(camera) * scale
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Writes `value` to `leaf`, then reads the leaf's group once a second for up to 10 s until it
/// shows `value`; puts the value read before back. Returns the lag, or the last value read (or
/// why the write failed).
fn write_and_watch(cam: &mut dyn Camera, leaf: &str, value: &Value) -> Result<Duration, Value> {
    let before = read_leaf(cam, leaf).unwrap();
    if let Err(e) = cam.apply(leaf, value, &Ctx::none()) {
        return Err(json!(format!("not written: {e}")));
    }
    let start = Instant::now();
    let outcome = loop {
        let last = read_leaf(cam, leaf).unwrap();
        if values::same(&last, value) {
            break Ok(start.elapsed());
        }
        if start.elapsed() > Duration::from_secs(10) {
            break Err(last);
        }
        std::thread::sleep(Duration::from_secs(1));
    };
    restore(cam, leaf, &before);
    outcome
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_23_the_u_rows_that_read_back() {
    // U-4 (offset bounds), U-5 (hand axes), U-6 (hand return time), U-18 (start modes);
    // each written, read back, restored. The lags are CAM-5 data.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let rows: &[(&str, Value)] = &[
        ("framing.offset.x", json!(-0.16)),
        ("framing.offset.x", json!(0.16)),
        ("framing.offset.y", json!(-0.16)),
        ("framing.offset.y", json!(0.16)),
        ("hand.axes.pan", json!(false)),
        ("hand.axes.tilt", json!(false)),
        ("hand.return_after", json!(3.0)),
        ("power_on.start.mode", json!("static")),
        ("power_on.start.mode", json!("upper_body")),
        ("power_on.start.mode", json!("headless")),
        ("power_on.start.mode", json!("group")),
    ];
    let mut failed = Vec::new();
    for (leaf, value) in rows {
        match write_and_watch(cam.as_mut(), leaf, value) {
            Ok(lag) => println!("pass {leaf} = {value} after {} ms", lag.as_millis()),
            Err(last) => {
                println!("FAIL {leaf} = {value}: reads {last}");
                failed.push(format!("{leaf} = {value}"));
            }
        }
    }
    assert!(
        failed.is_empty(),
        "rows that failed (their U-row becomes PX-U-n): {failed:?}"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_16_an_area_wholly_beyond_the_stored_one() {
    // U-8: the bounds go min then max; a new range wholly above or below the stored one
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let mut failed = Vec::new();
    for leaf in [
        "zone.area.pan",
        "zone.area.tilt",
        "hand.area.pan",
        "hand.area.tilt",
    ] {
        let before = read_leaf(cam.as_mut(), leaf).unwrap();
        for (lo, hi) in [(-20.0, 20.0), (40.0, 60.0), (-60.0, -40.0)] {
            let v = json!({"min": lo, "max": hi});
            match write_and_watch(cam.as_mut(), leaf, &v) {
                Ok(_) => println!("pass {leaf} = {v}"),
                Err(last) => {
                    println!("FAIL {leaf} = {v}: reads {last}");
                    failed.push(format!("{leaf} = {v}"));
                }
            }
        }
        restore(cam.as_mut(), leaf, &before);
    }
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_5_every_writable_leaf_reads_back_and_its_lag() {
    // CAM-5: each writable leaf with a readback written to a value next to its own (its
    // `requires` met first), read back once a second, put back; the lags are `settle_ms` data
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let mut failed = Vec::new();
    for leaf in swept(cam) {
        let s = &desc.settings[&leaf];
        let met = meet(cam, s);
        let before = read_leaf(cam, &leaf);
        match before.as_ref().ok().and_then(|b| other_value(s, b)) {
            None => println!("skip {leaf}: it reads {before:?}"),
            Some(value) => match write_and_watch(cam, &leaf, &value) {
                Ok(lag) => println!("pass {leaf} = {value} after {} ms", lag.as_millis()),
                Err(last) => {
                    println!("FAIL {leaf} = {value}: reads {last}");
                    failed.push(format!("{leaf} = {value}"));
                }
            },
        }
        for (path, v) in met.iter().rev() {
            restore(cam, path, v);
        }
    }
    assert!(
        failed.is_empty(),
        "leaves that did not read back within 10 s: {failed:?}"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_2_cam_3_cam_22_asleep() {
    // CAM-2: which groups read and which writes and actions the camera takes while asleep,
    // and whether it keeps them after the wake; CAM-3: `zoom.factor` asleep, and the time from
    // `wake` until the status reports awake; CAM-22: the live pose asleep, the go-return
    // actions and the pose-taking actions (a save into a free preset slot, a new view, both
    // deleted again). Everything written is put back.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let awake = state(cam);
    assert_eq!(awake.at("asleep"), Some(&json!(false)), "start awake");
    act(cam, "sleep", json!({}), &awake).unwrap();
    let slept = until_asleep_is(cam, true).expect("asleep within 20 s");
    println!("asleep {} ms after `sleep`", slept.as_millis());

    for g in cam.poll_plan().to_vec() {
        match cam.read(&g.name, &Ctx::none()) {
            Ok(v) => println!("asleep read {:<12} {}", g.name, json!(v)),
            Err(e) => println!("asleep read {:<12} FAILED: {e}", g.name),
        }
    }
    let mut written = Vec::new();
    for leaf in swept(cam) {
        let Some(before) = awake.at(&leaf).cloned() else {
            continue;
        };
        let Some(value) = other_value(&desc.settings[&leaf], &before) else {
            continue;
        };
        match cam.apply(&leaf, &value, &Ctx::none()) {
            Ok(()) => {
                println!("asleep write {leaf} = {value}: sent");
                written.push((leaf, before, value));
            }
            Err(e) => println!("asleep write {leaf} = {value}: {e}"),
        }
    }
    let slot = free_preset_slot(&desc, &awake);
    let mut actions = vec![
        ("gimbal.move", json!({"pan": 10, "tilt": 5})),
        ("zoom.move", json!({"speed": "fast", "factor": 2})),
        ("zone.go_return", json!({})),
        ("hand.go_return", json!({})),
    ];
    if let Some(slot) = slot {
        actions.push(("preset.save", json!({ "slot": slot })));
    }
    for (action, p) in &actions {
        println!(
            "asleep do {action} {p}: {:?}",
            act(cam, action, p.clone(), &awake)
        );
    }
    let view = act(cam, "view.save", json!({"name": "cam22"}), &awake);
    println!("asleep do view.save: {view:?}");

    act(cam, "wake", json!({}), &awake).unwrap();
    let woke = until_asleep_is(cam, false).expect("awake within 20 s");
    println!("awake {} ms after `wake` (CAM-3)", woke.as_millis());
    for (leaf, _, value) in &written {
        let now = read_leaf(cam, leaf);
        let kept = now.as_ref().is_ok_and(|v| values::same(v, value));
        println!(
            "after the wake {leaf}: wrote {value}, reads {now:?}: {}",
            if kept { "kept" } else { "not kept" }
        );
    }
    let after = state(cam);
    println!(
        "after the wake: pan {:?} tilt {:?} zoom {:?}",
        after.at("gimbal.pan"),
        after.at("gimbal.tilt"),
        after.at("zoom.factor")
    );

    // put back: the saved preset and view, the writes, the pose
    if let Some(slot) = slot
        && let Some(p) = preset(&after, slot)
    {
        println!("the preset saved asleep is there: {p}");
        act(cam, "preset.delete", json!({ "slot": slot }), &after).unwrap();
    }
    if let Ok(Some(result)) = &view {
        let id = result["id"].clone();
        println!(
            "view.save asleep gave id {id}; views now {:?}",
            after.at("views")
        );
        if let Err(e) = act(cam, "view.delete", json!({ "id": id }), &after) {
            eprintln!("WARNING: view {id} not deleted: {e}");
        }
    }
    for (leaf, before, _) in written.iter().rev() {
        restore(cam, leaf, before);
    }
    if let (Some(pan), Some(tilt)) = (awake.at("gimbal.pan"), awake.at("gimbal.tilt")) {
        let back = json!({"pan": pan, "tilt": tilt});
        if let Err(e) = act(cam, "gimbal.move", back, &awake) {
            eprintln!("WARNING: the gimbal not moved back: {e}");
        }
    }
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_4_the_zoom_readback_during_a_slow_zoom_move() {
    // CAM-4: during a speed-limited `zoom.move`, does the zoom readback show the target or the
    // factor on the way? The readings every 200 ms are the answer. The zoom is put back.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let before = read_leaf(cam, "zoom.factor").unwrap();
    let target = if before.as_f64().unwrap_or(1.0) < 2.5 {
        4.0
    } else {
        1.0
    };
    let st = State::new(true);
    act(
        cam,
        "zoom.move",
        json!({"speed": "slow", "factor": target}),
        &st,
    )
    .unwrap();
    let start = Instant::now();
    let mut readings = Vec::new();
    while start.elapsed() < Duration::from_secs(6) {
        let v = read_leaf(cam, "zoom.factor");
        println!("{:>5} ms  zoom.factor {v:?}", start.elapsed().as_millis());
        readings.push(v);
        std::thread::sleep(Duration::from_millis(200));
    }
    if let Err(e) = act(
        cam,
        "zoom.move",
        json!({"speed": "fast", "factor": before}),
        &st,
    ) {
        eprintln!("WARNING: the zoom not put back: {e}");
    }
    let first = readings.first().and_then(|r| r.as_ref().ok());
    let shows_target = first.is_some_and(|v| values::same(v, &json!(target)));
    println!(
        "the first reading {first:?} shows {}",
        if shows_target {
            "the target"
        } else {
            "the factor on the way"
        }
    );
    assert!(
        readings.iter().all(Result::is_ok),
        "every zoom read answered"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_13_anti_flicker_and_exposure_mode_in_both_orders() {
    // CAM-13: `exposure.anti_flicker` and `exposure.mode` written one right after the other, in
    // both orders; both must read back for the order not to matter (8.8 apply order)
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let leaves = ["exposure.anti_flicker", "exposure.mode"];
    let before: Vec<Value> = leaves.iter().map(|l| read_leaf(cam, l).unwrap()).collect();
    let wanted: Vec<Value> = leaves
        .iter()
        .zip(&before)
        .map(|(l, b)| other_value(&desc.settings[*l], b).unwrap())
        .collect();
    let mut failed = Vec::new();
    for order in [[0, 1], [1, 0]] {
        for i in order {
            cam.apply(leaves[i], &wanted[i], &Ctx::none()).unwrap();
        }
        std::thread::sleep(Duration::from_secs(3));
        let name = format!("{} then {}", leaves[order[0]], leaves[order[1]]);
        for (leaf, value) in leaves.iter().zip(&wanted) {
            let now = read_leaf(cam, leaf).unwrap();
            let ok = values::same(&now, value);
            println!(
                "{name}: {leaf} wrote {value}, reads {now}: {}",
                if ok { "pass" } else { "FAIL" }
            );
            if !ok {
                failed.push(format!("{name}: {leaf}"));
            }
        }
        for (leaf, value) in leaves.iter().zip(&before).rev() {
            restore(cam, leaf, value);
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    assert!(
        failed.is_empty(),
        "an order that loses a write becomes an apply-order rule (8.8): {failed:?}"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_15_a_manual_value_while_its_automatic_control_is_on() {
    // CAM-15: the white-balance temperature and the absolute focus written while their
    // automatic control is on (the model sends what it is given; the daemon's `requires` check
    // refuses it before). The outcome printed is the answer; both are put back.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    for (auto, on, leaf) in [
        (
            "image.white_balance.auto",
            json!(true),
            "image.white_balance.temperature",
        ),
        ("focus.mode", json!("continuous"), "focus.position"),
    ] {
        let auto_before = read_leaf(cam, auto).unwrap();
        let before = read_leaf(cam, leaf).unwrap();
        cam.apply(auto, &on, &Ctx::none()).unwrap();
        let value = other_value(&desc.settings[leaf], &before).unwrap();
        let sent = cam.apply(leaf, &value, &Ctx::none());
        std::thread::sleep(Duration::from_secs(2));
        println!(
            "{auto} = {on}, then {leaf} = {value}: {sent:?}; it reads {:?}",
            read_leaf(cam, leaf)
        );
        meet(cam, &desc.settings[leaf]);
        restore(cam, leaf, &before);
        restore(cam, auto, &auto_before);
    }
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_17_gimbal_moves_with_speed() {
    // CAM-17 (U-9): `gimbal.move` with a speed near the tilt limit and at the speed ceiling, and
    // with one axis only; each must arrive within 1°. The pose is put back.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let p = &desc.actions["gimbal.move"].params;
    let tilt_max = number(&p["tilt"], "max").unwrap();
    let speed_max = number(&p["speed"], "max").unwrap();
    let st = State::new(true);
    let pose = |cam: &mut dyn Camera| {
        let v = cam.read("pose", &Ctx::none()).unwrap();
        (
            v["gimbal.pan"].as_f64().unwrap(),
            v["gimbal.tilt"].as_f64().unwrap(),
        )
    };
    let start = pose(cam);
    let mut failed = Vec::new();
    for (params, want) in [
        (
            json!({"pan": 0, "tilt": tilt_max - 1.0, "speed": speed_max}),
            (Some(0.0), Some(tilt_max - 1.0)),
        ),
        (
            json!({"pan": 30, "speed": speed_max / 2.0}),
            (Some(30.0), None),
        ),
        (
            json!({"tilt": -10, "speed": speed_max / 2.0}),
            (None, Some(-10.0)),
        ),
    ] {
        act(cam, "gimbal.move", params.clone(), &st).unwrap();
        std::thread::sleep(Duration::from_secs(4));
        let (pan, tilt) = pose(cam);
        let near = |got: f64, want: Option<f64>| want.is_none_or(|w| (got - w).abs() <= 1.0);
        let ok = near(pan, want.0) && near(tilt, want.1);
        println!(
            "gimbal.move {params}: pan {pan} tilt {tilt}: {}",
            if ok { "pass" } else { "FAIL" }
        );
        if !ok {
            failed.push(params.to_string());
        }
    }
    let back = json!({"pan": start.0, "tilt": start.1});
    if let Err(e) = act(cam, "gimbal.move", back, &st) {
        eprintln!("WARNING: the gimbal not moved back: {e}");
    }
    assert!(
        failed.is_empty(),
        "moves that did not arrive (U-9): {failed:?}"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_35_a_preset_saved_without_a_name() {
    // CAM-35: what the camera reports as the name of a preset saved without one (obsbot4linux
    // sends none). The preset goes into a free slot and is deleted again; with no free slot
    // the test writes nothing.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let st = state(cam);
    let Some(slot) = free_preset_slot(&desc, &st) else {
        eprintln!("skipped: no free preset slot, or the presets did not read");
        return;
    };
    act(cam, "preset.save", json!({ "slot": slot }), &st).unwrap();
    std::thread::sleep(Duration::from_secs(1));
    let st = state(cam);
    let saved = preset(&st, slot).cloned();
    println!("preset {slot}, saved without a name, reads {saved:?}");
    if let Err(e) = act(cam, "preset.delete", json!({ "slot": slot }), &st) {
        eprintln!("WARNING: preset {slot} not deleted: {e}");
    }
    let saved = saved.expect("the saved preset is listed");
    assert!(
        saved.get("name").is_some(),
        "the item has its name field: a string, or null"
    );
}
```

- [ ] **Step 2: Compile them; see them skip**

Run: `cargo test -p obsbot-device --test live`
Expected: `test result: ok. 0 passed; 0 failed; 13 ignored`.

Run: `cargo test -p obsbot-device --test live -- --ignored`
Expected (no `OBSBOT_LIVE`): thirteen tests pass, each printing `skipped: set OBSBOT_LIVE=1 …`; nothing is opened.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add crates/obsbot-device/tests/live.rs
git commit -m "obsbot-device: ignored live tests for camera session CS-1"
```

### Task A-14: Remove the carried 0.13 modules

**Files:**

- Delete: `crates/obsbot-device/src/legacy/` (`mod.rs`, `state.rs`, `current_pose.rs`), `src/models/tiny2lite/camera.rs`, `src/models/tiny2lite/status.rs`, `src/family/exchange.rs`, `src/scripted.rs`, `src/transport/uvc.rs`
- Modify: `crates/obsbot-device/src/lib.rs`, `src/enumeration.rs`, `src/registry.rs`, `src/error.rs`, `src/family/mod.rs`, `src/transport/mod.rs`, `src/models/tiny2lite/mod.rs`, `src/models/tiny2lite/commands.rs`, `src/models/tiny2lite/presets.rs`, `crates/obsbot-device/Cargo.toml`, `Cargo.lock`, `crates/obsbot-device/tests/byte_guard.rs`, `crates/obsbot-device/tests/fixtures/byte-guard.txt` (deleted lines only), and the comments named in Step 6
- Test: the whole workspace; the byte guard

**Interfaces:**

- Consumes: everything above; nothing outside `obsbot-device` uses a deleted item (W0 Task 6 removed the 0.13 command line; track B uses `bus`, `registry`, `models`).
- Produces: `obsbot-device` without the 0.13 modules, `clap` and `serde`; `crate::error::Error` with one variant `Invalid(String)`; `commands::{AiMode, Gesture}::ALL` and `ALL` on every enum of the `setting!` macro.

- [ ] **Step 1: See that nothing uses the carried modules any more**

Run: `git grep -nE "legacy::|tiny2lite::camera|tiny2lite::status|family::exchange|transport::uvc|scripted::|registry::(MODELS|Model|from_by_id|from_card)|enumeration::(detect|Detected)" -- crates`
Expected: hits only inside the files this task deletes.

- [ ] **Step 2: Delete the modules**

```bash
git rm -r crates/obsbot-device/src/legacy crates/obsbot-device/src/models/tiny2lite/camera.rs crates/obsbot-device/src/models/tiny2lite/status.rs crates/obsbot-device/src/family/exchange.rs crates/obsbot-device/src/scripted.rs crates/obsbot-device/src/transport/uvc.rs
```

Remove their module lines: `pub mod legacy;` and `#[cfg(test)] pub(crate) mod scripted;` from `lib.rs`; `pub mod camera;` and `pub mod status;` from `models/tiny2lite/mod.rs`; `pub mod exchange;` from `family/mod.rs`; `pub mod uvc;` from `transport/mod.rs`. Write these files whole:

`crates/obsbot-device/src/lib.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Camera access for obsbot4linux (design spec 3.1): the device contract, the transports,
//! the family building blocks, the models and their registry, and the sysfs enumeration.

pub mod bus;
pub mod contract;
pub mod descriptions;
pub mod enumeration;
pub mod error;
pub mod family;
pub mod models;
pub mod registry;
pub mod transport;
```

`crates/obsbot-device/src/enumeration.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Enumeration (design spec 3.1, 5.3): the USB facts of every video device, read from sysfs.

pub mod sysfs;
```

`crates/obsbot-device/src/registry.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The registry of the real models (design spec 8.1). The types are in `contract::bus`.

use crate::contract::Registry;
use crate::models::tiny2lite;

/// The vendor table and the registered models (design spec 8.1): the OBSBOT vendor id read
/// from a unit in hand (`0x3564`, measured on the Tiny 2 Lite) and the Tiny 2 Lite,
/// `3564:fef9`. Nothing else.
pub static REGISTRY: Registry = Registry {
    vendors: &[0x3564],
    models: &[tiny2lite::plan::ENTRY],
};
```

`crates/obsbot-device/src/error.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The error of the byte builders: a value a builder refuses. The models turn it into a
//! `CameraError` before anything is sent.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid value: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

and in `family/mod.rs` and `transport/mod.rs` the module doc comments:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Building blocks that are not tied to one camera model: the frame codec, the link over a
//! transport (answer matching, short commands, queries) and the on-air probe.

pub mod frame;
pub mod link;
pub mod streaming;
```

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The transports (design spec 3.1): the USB transport, the replay transport for tests, and
//! the send-table check that wraps a model's transport.

pub mod checked;
pub mod replay;
pub mod usb;
```

- [ ] **Step 3: Drop clap and serde from the builders; delete the builders no 1.0 path uses**

In `crates/obsbot-device/src/models/tiny2lite/commands.rs`:

1. Remove `use clap::ValueEnum;` and `use serde::{Deserialize, Serialize};`.
2. In the `setting!` macro, the derive becomes `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` (no `#[serde]` line), and the `impl` gets `ALL` as its first item:

```rust
macro_rules! setting {
    ($(#[$m:meta])* $name:ident { $($v:ident = $n:expr),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($v),+ }
        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$v),+];
            pub fn from_u8(value: u8) -> Option<Self> {
                match value { $($n => Some(Self::$v),)+ _ => None }
            }
            pub fn to_u8(self) -> u8 {
                match self { $(Self::$v => $n),+ }
            }
        }
    };
}
```

3. `AiMode` and `Gesture`: the derive as in 2; add to `impl AiMode`, first item:

```rust
    /// Every mode, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::Static,
        Self::Normal,
        Self::UpperBody,
        Self::CloseUp,
        Self::Headless,
        Self::LowerBody,
        Self::Hand,
        Self::Group,
    ];
```

and after the enum `Gesture`:

```rust
impl Gesture {
    /// Every gesture, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::TargetLock,
        Self::Zoom,
        Self::DynamicZoom,
        Self::DirectionChange,
    ];
}
```

4. Delete `setting!(PickupDistance …)`, the functions `auto_sleep_off`, `pickup_distance` and `view_center` with their doc comments, and in the tests: the line `assert_eq!(auto_sleep_off(120).unwrap(), auto_sleep(-120));`, the pickup-distance block of `display_and_audio_short_commands_match_capture` (from `// capture 06: pickup distance far` to the line before `// capture 14`), the `view_center` assertions of `stop_and_centre_frames` (rename that test `stop_and_zoom_frames`), and the test `auto_sleep_off_is_always_negative`. In `ai_mode_status_pairs_round_trip` write `for mode in AiMode::ALL {`.

In `crates/obsbot-device/src/models/tiny2lite/presets.rs`:

1. Replace the three `use` lines of `commands`, `clap` and `serde` by `use crate::models::tiny2lite::commands::gimbal_angles_ok;`; `Pose` derives `Debug, Clone, Copy, PartialEq`, `Preset` and `ZoneView` derive `Debug, Clone, PartialEq`.
2. Delete `StartupAi` with its `impl`, `STARTUP_AI_NOT_SET`, `startup_ai_mode`, `startup_picture_settings`, and `StartupActions` with its `impl`; keep `const STARTUP_BLOCK: usize = 40;` just above `startup_with_ai`.
3. In the tests: delete `startup_actions_nothing_set_as_read_live`, `startup_actions_set_groups`, `startup_block_with_an_ai_mode_matches_capture_08`, `startup_ai_bytes_decode_and_only_known_ones_are_written`, `startup_ai_mode_names_the_documented_bytes`, `startup_actions_need_the_24_bytes_up_to_the_hue`, and the `startup_picture_settings` assertion of `startup_matches_capture`; in `ai_alone_rewrites_only_byte_0_of_what_the_camera_reports` write `read[0] = 0xfe;` and `startup_with_ai(&read, 0x00, 7)`. (`decode::power_on` and its tests in A-4 read the same recorded blocks; `write.rs` and `act.rs` write them.)

In `crates/obsbot-device/Cargo.toml`, `[dependencies]`: remove `clap.workspace = true` and `serde.workspace = true`. Run `cargo build --workspace` (updates `Cargo.lock`).

- [ ] **Step 4: Move the byte guard off clap; drop the deleted builders' lines**

In `crates/obsbot-device/tests/byte_guard.rs`: remove `use clap::ValueEnum;`; import `presets::{self, Pose}` (no `StartupAi`); replace every `::value_variants()` by `::ALL`; delete the loops that record `cmd::auto_sleep_off`, `cmd::pickup_distance`, `cmd::view_center` and `presets::startup_picture_settings`. Every other label and call stays as it is.

Remove exactly those builders' lines from the fixture:

```bash
cp crates/obsbot-device/tests/fixtures/byte-guard.txt "$SCRATCH/byte-guard.before"
grep -vE '^(cmd::auto_sleep_off|cmd::pickup_distance|cmd::view_center|presets::startup_picture_settings)\(' "$SCRATCH/byte-guard.before" > crates/obsbot-device/tests/fixtures/byte-guard.txt
diff "$SCRATCH/byte-guard.before" crates/obsbot-device/tests/fixtures/byte-guard.txt | grep -c '^<'
diff "$SCRATCH/byte-guard.before" crates/obsbot-device/tests/fixtures/byte-guard.txt | grep -c '^>'
```

Expected: `40` and `0`: 6 `auto_sleep_off`, 3 `pickup_distance`, 25 `view_center` and 6 `startup_picture_settings` lines removed, none added. `BYTE_GUARD_BLESS` is not used: every kept builder emits the bytes it emitted at stage G.

- [ ] **Step 5: Run the workspace**

Run: `cargo test --workspace --locked && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: every test result ok, the 13 live tests ignored; `byte_guard` passes; no clippy output.

Run: `cargo tree -p obsbot-device -e normal --depth 1`
Expected: `nix`, `obsbot-proto`, `serde_json`, `thiserror`; no `clap`, no `serde`.

- [ ] **Step 6: Public-text pass over the carried comments**

The public-text rule allows two statements of where knowledge comes from: "recorded on the owner's device (capture N)" and "measured on the camera" (with a date or an issue number). Read every comment of `commands.rs` and `presets.rs`, code and tests, and of the other files this task keeps in `crates/obsbot-device/src`, and change comments only, by these rules:

- reword every comment whose statement of origin is not one of those two forms into one of them, where a capture or a measurement backs it; delete it where none does;
- delete statements about how a frame or a value was found;
- delete a sentence that names an id this project does not send; the field ledger states what is sent;
- keep the licence notes of 3.3 (`// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.`) and the firmware version a measurement was made on ("firmware 6.2.8.1").

After the pass, a comment cites as its source only capture keys, measurement dates and issue numbers. The brief of this task lists the lines the plan's check found when it was built (35 in `commands.rs` and `presets.rs`; A-2 already rewrote the two in `family/frame.rs`); the reviewer runs that check again and it finds nothing.
Expected: `git diff --stat` shows comment changes only in those files (review: `git diff -U0 | grep '^[-+]' | grep -vE '^[-+]\s*(//|///|\*)' ` lists no code line beyond Steps 2–4).

- [ ] **Step 7: Commit**

```bash
git add -A crates/obsbot-device Cargo.lock
git commit -m "obsbot-device: remove the carried 0.13 modules, clap and serde; public-text pass"
```

---

## Track A acceptance without camera (spec 10.3), mapped

| Acceptance                                                                              | Where                                                                                                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| every encoder against recorded request bytes                                            | the carried builder tests of `commands.rs` and `presets.rs` (captures 03–18) and the byte guard, unchanged except A-14's deletions; A-10 and A-11 assert every leaf's and action's payload; A-12 every frame and its order                                                                                                                                              |
| every decoder against recorded answers                                                  | A-4 (answers of captures 01, 04, 08, 11 and measured answers), A-3 `answer_frames_have_the_recorded_layout`, A-2 `recorded_value_answers_parse`                                                                                                                                                                                                                         |
| the status decoder against recorded blocks                                              | A-4 `a_recorded_status_block_decodes_leaf_by_leaf`, `each_recorded_change_shows_in_its_own_leaf_only`, `auto_sleep_is_one_word_with_the_sign_as_the_switch` (captures 02, 05, 06, 07, 09)                                                                                                                                                                               |
| angle conversions as pure functions with a recorded answer and one asymmetric case      | A-4 `pose`, `pitch_to_tilt_range`, `tilt_to_pitch_range` (`the_live_pose_reads_in_the_units_of_4_2`, `the_recorded_zone_answers_decode_in_the_units_of_4_2`, `the_recorded_hand_answers_decode`, `tilt_ranges_swap_and_flip_both_ways`); A-11 `tilt_to_pitch` through `a_gimbal_move_sends_only_the_move`; A-10 `an_area_axis_sends_min_then_max_in_the_cameras_angles` |
| the send-table test                                                                     | A-12 (A-6 the check, A-7 the table)                                                                                                                                                                                                                                                                                                                                     |
| the registry test                                                                       | A-13 `the_registry_holds_the_vendor_table_and_the_tiny_2_lite_only`                                                                                                                                                                                                                                                                                                     |
| the serial-format test                                                                  | A-4 `identity_answers`, A-9 `an_identity_attempt_sends_only_the_missing_queries`; the format itself is W0's `is_valid_serial` and its tests; live: A-15 `live_cam_1_the_identity_queries_answer`                                                                                                                                                                        |
| each audit defect in carried code fixed or named in section 14                          | table below                                                                                                                                                                                                                                                                                                                                                             |
| the carried gimbal move and rate flows without their switch to the static tracking mode | A-11 `a_gimbal_move_sends_only_the_move`, `a_gimbal_turn_sends_only_the_turn_with_the_yaw_sign_of_mirror_xor_invert`; A-12 the action rows                                                                                                                                                                                                                              |

Audit defects in carried code (`2026-11-12-audit/findings.json`, device files) and #61:

| Finding                                                                                        | Fate                                                                                                                                 |
| ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| A-002 the answer poll takes every read error for "not ready"                                   | fixed: A-2 decision 3                                                                                                                |
| A-088 the answer poll's mapping untested                                                       | fixed: A-2 `link` tests (read error, stale, rejected, empty, no answer)                                                              |
| #61 answers matched by sequence only                                                           | fixed: A-2 decision 4                                                                                                                |
| A-001 white balance temperature and absolute focus written while their automatic control is on | fixed by the description's `requires` (W0) and A-10 (no automatic control written first); the camera's refusal is CAM-15             |
| A-003 on-air state and applications at two moments                                             | fixed: A-8 `on_air`, one sample                                                                                                      |
| A-004 angle conversions without tests                                                          | fixed: A-4, A-10, A-11                                                                                                               |
| A-005 a short preset list counted as a read                                                    | fixed: A-4 `decode::presets`, A-9 `the_preset_list_is_rewound_once_and_read_whole`                                                   |
| A-006, A-007, A-009 builders and status fields without tests                                   | the carried tests (stage G) and A-4's recorded blocks, A-10's payload rows                                                           |
| A-008 preset lead tests that cannot fail                                                       | A-4 `the_recorded_hand_answers_decode` and `the_recorded_zone_answers_decode_in_the_units_of_4_2` test the lead through the decoders |
| A-022 "the current pose" is remembered or V4L2                                                 | fixed: A-11 `live_pose`                                                                                                              |
| A-027 the preset pose sent before the name is checked                                          | fixed: A-11 `a_name_the_camera_cannot_take_stops_the_save_before_any_frame`                                                          |
| A-018, A-053 detection by product name; other nodes opened                                     | fixed: A-5 (USB ids from sysfs, no node opened), A-13                                                                                |
| A-101 names of other software and firmware in comments                                         | fixed: A-14 Step 6                                                                                                                   |
| A-010 the probe's exclusive privilege on kernels up to 6.16                                    | documented in the carried `streaming.rs`; no code change (spec 4.3)                                                                  |
| A-011 area bounds order                                                                        | named: U-8, CAM-16 (A-15 `live_cam_16…`)                                                                                             |
| A-012 unbacked start-mode values                                                               | named: U-18, CAM-23 (A-15 `live_cam_23…`)                                                                                            |
| A-014 a silent query channel                                                                   | named: CAM-21                                                                                                                        |
| A-015 the preset list rewind                                                                   | named: CAM-19                                                                                                                        |
| A-016 gimbal speed near the tilt limit                                                         | named: U-9, CAM-17                                                                                                                   |
| A-019 the second frame of a preset delete                                                      | named: CAM-18                                                                                                                        |
| A-020 the status block while booting                                                           | named: CAM-20 (A-15 prints byte 0)                                                                                                   |
| A-021 a status bit for the microphone                                                          | named: U-23, CAM-8                                                                                                                   |

## Camera items this track leaves open

CAM-1 to CAM-8, CAM-10, CAM-13 to CAM-20, CAM-22, CAM-23, CAM-25, CAM-33 to CAM-35 (CS-1; CAM-21 recorded if it happens), with the U-rows they settle: U-4, U-5, U-6, U-7, U-8, U-9, U-12, U-16, U-18, U-19, U-20, U-23, U-25, U-26. None is in a task's acceptance.

A-15 runs, as `#[ignore]` tests with send-table frames only: CAM-1, CAM-2, CAM-3, CAM-4, CAM-5, CAM-10, CAM-13, CAM-15, CAM-16, CAM-17, CAM-20 (prints the byte), CAM-22, CAM-23 (U-4, U-5, U-6, U-18), CAM-25, CAM-35 and the UVC ranges of the W0 plan's decision 12 (`UNCONFIRMED`). The session runs CAM-1 again asleep, right after plug-in, after the microphone restart and after a factory reset, and CAM-20 right after plug-in.

Steps of the CS-1 session, with the reason a cargo test cannot do them:

| Item   | Reason                                                                                                                               |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| CAM-3  | the part "is the camera awake at plug-in" needs a re-plug (the rest is in `live_cam_2_cam_3_cam_22_asleep`)                          |
| CAM-6  | `hand.move_gimbal` and `led.blink` have no readback: a person watches them, across a re-plug and a factory reset                     |
| CAM-7  | whether a running turn stops at sleep and resumes after the wake is seen on the camera; the live pose may not answer asleep (CAM-22) |
| CAM-8  | a cold plug with the microphone off; `audio.enabled` is left out of the sweeps because it can take the USB audio interface away      |
| CAM-14 | it asks about the UVC focus-auto control, which no 1.0 leaf writes and the send table does not hold                                  |
| CAM-18 | needs a second writer on the camera                                                                                                  |
| CAM-19 | compares the rewind with a read path of one slot list plus one pose query per slot                                                   |
| CAM-33 | needs a person in view                                                                                                               |
| CAM-34 | needs a person in view                                                                                                               |

CAM-2, CAM-6 and CAM-22 asleep rows also run through `scripts/check-model.sh --asleep` of W2. The results arrive as contract-change PRs of the description and ledger edits (spec 10.3).

## Self-review

1. **Spec coverage (10.3 track A content).** USB transport with the extension unit by GUID (A-8, A-13), named timeouts (A-2 `ANSWER_POLL` = `REPLY_ATTEMPTS` × `REPLY_GAP`, `FRAME_GAP`; A-8 `NODE_LOCK_WAIT`), per-operation node lock (A-1, A-2, A-8; pinned on the production path by A-6 and A-9), send-table check (A-6); replay transport (A-3); family code with scripted answer-poll tests for read error, stale sequence, rejected, empty, no answer (A-2); `models/tiny2lite` behind the trait: identity, poll plan, read (A-9), apply, order (A-7, A-10), act (A-11), touched, stops, continuing (A-7 `motion`), send table (A-7); registry with the vendor table (A-13); enumeration from sysfs (A-5); `#[ignore]` live tests behind an environment variable, sending only the send table, for every CS-1 item of 10.3 track A a machine can check (A-15; the rest under "Camera items this track leaves open"). Section 2: A-10, A-11, A-12; 3.2 carry-over and removals: A-11, A-14; 5.3: A-5, A-8; 5.5: A-9 identity; 5.7: A-7, A-9; 8.1–8.3: A-6, A-7, A-12, A-13; 8.5 public text and fixtures: every fixture names its capture or measurement, the serial is synthetic, A-14 Step 6; 8.8: A-7. Not in this plan: the evidence-level constants of 8.5 (question 3), anything of section 5 that is daemon behaviour (track B), `descriptions/tiny2lite.json` and `docs/field-ledger.md` (unchanged).
2. **Placeholder scan.** Every test is in full; every new file is in full; the edits to carried files are given as the code to insert or as named deletions with a count check (A-14 Step 4: 40 lines). A-14 Step 6 is a comment rewrite by stated rules, checked by the check in its brief and a diff that must hold comment lines only.
3. **Type consistency.** The names each task's "Produces" lists are those the later tasks' "Consumes" and code use: the plan's code was built in that order of files in one copy and rechecked on the landed W0 (with the stubs of A-9 checked separately: the model and `tests/tiny2lite_read.rs` compile and pass with them). `Link`'s methods, `Memory`, `Tiny2Lite::checked` (the only public constructor), `Log::steps`, `SysfsBus::with_root`, `Selectors`, `classify`, `answer_frame`, `Replay`'s builders and `decode`'s functions have one signature each across the tasks.
4. **Section 2 in tests, not only in code.** `a_gimbal_move_sends_only_the_move`, `a_preset_saved_without_a_name_gets_none`, `uvc_leaves_write_their_control_and_no_other`, `auto_sleep_with_0_s_stored_sends_nothing_for_the_switch_alone`, `every_group_reads_exactly_its_leaves_and_sends_no_command` and A-12's two row tests fail when a frame of the model's own is added; A-12 Step 2 shows it for the 0.13 switch.

## Notes for the coordinator

- A-2 changes the shared `parse_answer`, so the carried `family::exchange` tests and `scripted.rs` build answers in the recorded layout from A-2 on (their helpers call `transport::replay::answer_frame`).
- A-14 Step 6: the brief of the A-14 builder carries the check that lists the comment lines to reword (kept out of this plan, which is public text).
