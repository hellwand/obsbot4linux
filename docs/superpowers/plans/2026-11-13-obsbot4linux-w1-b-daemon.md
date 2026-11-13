# obsbot4linux 1.0, stage W1 track B (daemon) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `obsbot4linuxd` behind the entry point W0 fixed (`Config`, `run`): the socket and its protocol-2 framing, connections and subscriptions, the device table with its events, the per-device worker with the write path of spec 5.6, the poll tiers, hotplug through `trait Bus`, the identity series and hand-over of 5.5, the connection-bound stop of a continuing action, fake mode with the test-control socket, systemd socket activation and the logs of 5.11. Milestone B-1 (task b-10, wave 4) is the daemon core in fake mode with the plain worker, so that tracks C and D can run their live tests against `obsbot4linuxd --fake fake-a,fake-b,unknown`; a chain of four tasks (b-11 to b-14) then adds the worker's remaining features, each with the spec's named tests that pin it; B-2 (task b-15) adds the real device path once track A's sysfs bus exists.

**Architecture:** Synchronous threads, no async runtime (spec 5.1): an acceptor; per connection a reader (on the connection's thread) and a writer that drains a 64-line outbound queue; a registry thread that scans through `trait Bus` once a second; one worker per supported device, the only code that touches it; a sweeper that answers requests not started within 15 s. A device table under one lock holds every entry (id, `Device`, rev, last state) and the subscribed connections, so every event leaves in one order for every connection and a `hello` snapshot is atomic with the start of its events. The daemon's own logic is split into pure modules with their own tests (bounded line reader, `pending`, poll schedule, identity series, queue and batch planning, continuing-motion record, scan diff, names and log lines); the worker, the connections and the registry wire them to the camera and the socket. The daemon knows no command id, offset or selector: everything model-specific comes through `trait Camera`.

**Tech Stack:** Rust 2024 edition, MSRV 1.88 (CI toolchain 1.99.1), the workspace's `nix` 0.31 (features `fs` for `flock`, `signal` for `sigwait`), `serde_json` 1, `obsbot-proto` and `obsbot-device` of W0. No new crate.

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` (revision 2, approved 2026-11-13, with the changes after approval of section 15.1, rows 88 to 90), sections 2, 4.1, 4.2, 5 (all), 8.2, 8.3, 10.1, 10.3 (track B), 11, 14; the W0 plan `docs/superpowers/plans/2026-11-13-obsbot4linux-w0-contracts.md` (its code is the contract this plan builds on).

## Global Constraints

- Owner files of track B: `crates/obsbot4linuxd` only (spec 10.3). No task edits a W0 contract file: `crates/obsbot-proto/src/*` except `client*`, `crates/obsbot-device/src/contract/*`, `schema/protocol-2.json` (W0 plan, Global Constraints; spec 10.1). This plan needs no contract change.
- "A request sends exactly what it names: the frames of that setting or action, nothing before them and nothing after them." The only stop the daemon sends of its own is "the end the request itself set: a `gimbal.turn` without `keep:true` lasts as long as its connection, so its stop is sent when that connection closes, a daemon shutdown included" (spec 2, 15.1 rows 88 and 89).
- "The worker sends no other stop of its own: not when `asleep` turns true, and not for a `keep:true` turn" (spec 5.6).
- "Synchronous threads, no async runtime" (spec 5.1).
- Socket: "`$XDG_RUNTIME_DIR/obsbot4linux.sock` (mode 0600; the environment variable `OBSBOT4LINUX_SOCKET` overrides the path)"; "A request line longer than 64 KiB closes the connection"; "The daemon accepts at most 64 connections; a further connection is closed at once" (spec 4.1).
- "the first message on a connection must be `hello`. Requests that follow `hello` before its reply are kept and handled after the reply. A second `hello` is answered `invalid`" (spec 4.1).
- Outbound queue "64 lines; when it is full, the connection is closed and a log line names it by its connection number and the `client` of its `hello`" (spec 5.1).
- Device queue "FIFO, at most 32 requests; more is `busy`" (spec 5.6); "A stop … is taken before every queued request and answered when the camera accepted it" (4.1 rule 2).
- `REQUEST_DEADLINE` 15 s: "Every device request (`get`, `set`, `do`) is answered within 15 s of its arrival. A request not started by then is dropped and answered `camera`. A started request checks the deadline before each frame" (4.1 rule 9, 5.8).
- `ACCEPT_BACKOFF` "100 ms after a failed accept" (5.8).
- `hello` timing: "after its first enumeration and the first read attempt of every supported device that enumeration found …, and at the latest 15 s (`REQUEST_DEADLINE`) after the `hello` arrived" (4.1).
- Rev: "1 for the first state of a new entry, only goes up while the entry exists"; "An unchanged read produces no new rev and no event" (4.1 rule 1, 5.7).
- Tiers "1 s, 5 s, 60 s, or `touched`"; "Intervals count from the end of the previous read of that tier" (5.7).
- Enumeration: "Once a second the registry thread lists …"; "When the device number under a path changes between two scans, the daemon handles it as an unplug followed by a plug-in" (5.3).
- Identity: "A serial is valid only if it matches `[A-Za-z0-9]{8,32}`"; "no more than one attempt is ever sent to a sleeping camera"; "The id can change at most once per plug-in" (5.5).
- Single instance: "Every daemon, socket-activated or not, takes an exclusive `flock` on `<socket path>.lock` first. If the lock is held, it exits non-zero with a message that names the lock file" (5.9).
- "On SIGTERM or SIGINT the daemon closes its connections … and exits 0"; "A panic in a worker, the registry thread or the acceptor ends the process with a non-zero code … A panic in a connection thread closes only that connection. Locks are taken poison-tolerant" (5.9).
- "The daemon reads and writes no file except its socket and its lock file" (5.10); fake mode adds `<socket path>.test` (5.12).
- Logs: "Logs go to stderr"; serials and `sn:` ids "shortened by the one formatter of 4.2" (`obsbot_proto::short_id`); "No log line prints an identity answer" (5.11).
- Fake mode: "`obsbot4linuxd --fake <model>[,<model>…]` … never touches a real device"; test control on `<socket path>.test`, "not part of protocol 2" (5.12). "CI runs `--fake fake-a,fake-b,unknown` in every daemon … test that needs a daemon" (5.12, 11).
- Tests: "Tests run `obsbot4linuxd::run(Config)` in-process on a temporary socket, in parallel, without systemd and without `XDG_RUNTIME_DIR`. The fakes are deterministic" (11). No test in this track talks to a camera; no test sends anything to the unsupported fake.
- Every source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`. Public text: this project's behaviour and what was recorded on the owner's device; no statement of where knowledge comes from beyond "recorded on the owner's device" and "measured on the camera"; no serials except the synthetic `FAKEA…`, `FAKEB…`, `EXAMPLE…` (spec 8.5).
- Commits on `rebuild` are signed; every task is one PR with the `required` gate (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`) (spec 10.1, W0 plan).
- Builds and test runs: `nice -n 19`, at most 4 jobs (`-j 4`); `CARGO_TARGET_DIR` and `TMPDIR` under the builder's own scratch directory, never `/tmp`.

## Timing in tests

`Config` and `run` are fixed by W0, and the tier periods (`Tier::period`) and `REQUEST_DEADLINE` are contract constants. The daemon reads every interval from one struct, `Timing` (`crates/obsbot4linuxd/src/lib.rs`): `run(config)` is `run_with(config, Timing::SPEC)`, and a unit test pins `Timing::SPEC` to the contract values (scan 1 s, deadline 15 s, tiers 1 s, 5 s, 60 s). In-process tests call the doc-hidden `run_with` with `tests/common::fast()`: scan 100 ms, tiers 100 ms, 500 ms and 3 s, **the deadline stays 15 s**. So the 60 s retry and the 5 s identity attempts happen within seconds, and the three tests that need the 15 s deadline (`every_request_is_answered_within_the_deadline`, `hello_is_answered_by_its_deadline_while_a_first_read_is_slow`, and the busy test's bound) run in real time, in parallel with the rest. The fakes' own times are fixed (`fake::RESTART` 1 s, `fake::b::LAG` 250 ms, settle 1000 ms on fake-b and 3000 ms on fake-a); the tests are written for them. The test-only knob is the only addition to the daemon's public surface beyond W0's; it is `#[doc(hidden)]` and named in decision 1.

## Review Focus

Inputs and failure modes a user will hit that the spec's acceptance list does not name; each has its test in the owning task:

1. **Float noise from a client** (a QML slider, JavaScript arithmetic: `2.9999999999999996` for `led.brightness`). The W0 validator accepts a value within 1e-6 steps of a step. A request sends the value it names (section 2), so nothing is rounded to a precision before it is sent; the only change is that such noise is put on its step (`pending::sent`, decision 9), because an encoder of whole numbers refuses a fraction (track A's encoders do, which would answer the request `camera`). The state and `pending` hold the _echo_ (4.2: rounded where it is created), so the readback 3 shows the write at once instead of after the settle bound. Tests: `pending::tests::sent_keeps_the_value_and_only_puts_float_noise_on_its_step`, `pending::tests::echo_snaps_float_noise_to_the_step_when_there_is_no_precision` (b-03), `requests::a_value_is_sent_as_the_request_names_it` (b-11).
2. **A requirement written by the previous request of the same batch** (`set exposure.mode=manual`, then pipelined `set exposure.time=20000`). Validated against the not-yet-re-read published state it would be a `conflict`. The batch validates each request against the published state with the earlier requests' written leaves applied (decision 6). Test: `requests::a_pipelined_requirement_counts_within_one_batch` (b-11).
3. **Coalescing edge cases**: the same leaf written dotted once and nested once must coalesce; a later value the setting cannot take must not make an earlier valid one disappear unsent; another connection's write never coalesces; a write of another leaf or an action between the two keeps both (it may make the later one a `conflict` after the earlier one was answered `ok`). Tests: `queue::tests::single_leaf_sets_of_one_connection_coalesce`, `queue::tests::a_write_in_between_keeps_both_sets` (b-05), `requests::a_single_leaf_set_is_coalesced_with_a_later_one` (b-11).
4. **A stop while a slow camera answers a group read**: the read must end at its next frame boundary and the stop be sent at once, not after the read. Test: `motion::a_stop_cuts_a_group_read_in_progress` (b-13).
5. **Shutdown or unplug while a camera operation hangs**: polls and identity attempts must be cut so that `run` returns; a started request is cut when its device goes. Tests: `process::a_stop_returns_while_a_camera_operation_hangs` (b-14), `deadline::the_queue_of_a_device_that_goes_is_answered_and_a_replug_starts_afresh` (b-13), `worker::tests::the_end_answers_the_queue` (b-07).
6. **A connection that closes while its requests are in a batch**: requests queued behind a slow poll are taken together; they are no longer in the queue when the connection closes, so dropping the queued jobs is not enough. The connection's close marks its outbound closed first, and the batch skips every request whose connection is closed (4.1 rule 9). Tests: `conn::tests::a_closing_connection_marks_requests_already_taken_closed` (b-08), `outbound::tests::mark_closed_keeps_the_queued_lines_and_refuses_new_ones` (b-01), `deadline::a_batch_skips_the_requests_of_a_connection_that_closed` (b-13).

## What track B needs from W0

Exactly these names (W0 plan, Tasks 1, 2, 5, 9, 11), all compiled together in this plan's scratch build:

- `obsbot_proto`: `PROTOCOL`, `MAX_REQUEST_LINE`, `REQUEST_DEADLINE`, `socket_path()`, `sn_id`, `port_id`, `short_id`, `is_valid_serial`; `Device`, `Usb`, `Description`, `Setting`, `SettingType`, `Bound`, `Support`, `Warning`; `State` (`new`, `at`, `set`, `select`, `asleep`, fields `available`, `pending`, `unread`, `values`), `Patch`, `Params`, `Values`; `Request::from_line`, `RequestError { id, message }`, `Verb::{Hello, Get, Set, Do, Live}`, `Hello`, `Reply`, `ReplyBody`, `OkBody::{Hello, State, Rev, Empty}`, `HelloOk`, `RevState`, `RevOk`, `Empty`, `ErrorBody::new`, `ErrorCode`, `Event::{State, Devices}`, `RequestId`; `values::{number, rounded, same}`; `validate::{validate_set, validate_do, validate_get, leaves, check_value, Fit, Refusal}`.
- `obsbot_device::contract`: `trait Camera` (`description`, `identity(Need, &Ctx)`, `poll_plan`, `read(&str, &Ctx)`, `order`, `apply(&str, &Value, &Ctx)`, `check(&str, &Params, &State)`, `act(&str, &Params, &State, &Ctx)`, `touched(Target)`, `motion(&str)`, `send_table`), `CameraError::{NoAnswer, Rejected, Gone, Locked, Cut, NotInSendTable, Io}`, `Ctx::new(Option<Instant>, Option<Arc<AtomicBool>>)`, `Group { name, tier, asleep, live, leaves }`, `LiveTier`, `Tier::{Second, FiveSeconds, Minute, Touched}`, `Need`, `Identity`, `Target::{Leaf, Action}`, `Motion::{None, Move, Continuing { stop }, Stop}`; `trait Bus { scan, open }`, `Found { usb, devnum, video_node, model }`, `ModelEntry { key, display_name, support, identity, firmware_query, identity_tier, .. }`, `IdentitySource`.
- `obsbot_device::models::fake`: `FakeBus` (`new`, `plug`, `unplug`, `sleep`, `wake`, `fail_next`, `set_delay`, `ops`, `handle_line`, `impl Bus`), `Op::{Open, Identity, Read, Apply, Act}`, `KEYS`, `description(key)`; the fake behaviour W0 pins in `tests/fakes.rs` (serials `FAKEA0000001` …, `unknown` as `port:9-3`, paths `9-<n>`, a re-plug keeps serial and path with a new device number, `fail_next` checked after the operation's delay and surviving an unplug, a read of a group not readable asleep fails, `fake-b`'s lag and rounding, `RESTART` re-enumeration).
- `obsbot4linuxd::Config { socket, fake, stop }`, `Config::new`, `run(Config) -> io::Result<()>` (W0 Task 11): kept unchanged.

What the fakes cannot produce, and where it is tested instead: a duplicate serial (`identity::tests` and `table::tests::a_duplicate_serial_stays_on_its_port_id`), an invalid serial (`identity::tests`), a camera without a firmware query (`identity::tests`), a panic (`daemon::tests::a_panic_in_a_critical_thread_is_fatal`, `conn` guard), an unreadable camera node (`worker` open failure path, reviewed).

## What track B needs from the other tracks

- **Track A (only for B-2, task b-15):** its task `a-13-registry-and-bus`, with the names of track A's plan: `obsbot_device::bus::SysfsBus` (`impl Bus`), `SysfsBus::new() -> SysfsBus` (root `/sys`, the real registry) and `SysfsBus::with_root(root: impl Into<PathBuf>) -> SysfsBus` (another root, for the test; always the real registry), and `obsbot_device::registry::REGISTRY`. They go into exactly one line of `crates/obsbot4linuxd/src/daemon.rs` and the one test of b-15; if track A's landed code names them differently, the landed names win there. Nothing else of track A is used: the daemon never names `tiny2lite`. Track A's plan expects the daemon to log a `CameraError::NotInSendTable` (8.3 "refused with `internal`, logged and never sent"): b-07 does, through `names::log_refused_frame` (b-01). Track A's encoders refuse a value with a fraction for a leaf of whole numbers; the daemon therefore puts float noise on its step before it sends (decision 9).
- **Tracks C and D:** nothing. They consume B-1: the in-process entry `obsbot4linuxd::run(Config)` (through the CLI crate's `[dev-dependencies]`) and the binary `obsbot4linuxd --fake fake-a,fake-b,unknown` with its test-control socket `<socket path>.test`.

## Milestone B-1 (what tracks C and D wait for)

B-1 is landed when task **b-10-daemon** is on `rebuild` (wave 4: the skeleton, five pure modules and the table, then the plain worker, the connections and the registry, then the daemon). It is the daemon core of spec 10.3 B-1 with what tracks C and D use: all five verbs; every error code the daemon sends (`internal` for a refused frame, logged); `hello` with its start-up wait and deadline; events before replies; `pending` with the settle bound; `rev`; live-topic filtering; the plain write path (whole-patch validation against the current state of the batch, apply order, `applied`/`failed`, touched re-reads shared by a batch, stops as batches of their own that cut a group read); polls by tier with the asleep filter and the retry of a failed `touched` read; the 15 s deadline with the sweeper; `busy`; requests of a closed connection dropped, also inside a batch; the queue of a device that goes; hotplug through the fake bus with identity attempt 1; the unsupported entry; the test-control socket; `--fake`; socket activation; the lock; signals; logs. Its acceptance is `tests/core.rs` (9 tests) and the unit tests of b-01 to b-10.

What follows B-1, one task each, in a stacked chain that adds the feature together with the spec's named tests that pin it (spec 10.3 B "Acceptance without camera"; every other named test in those files pins B-1 behaviour and runs there for the first time):

| Task | Worker feature added                                                                     | Spec items it proves (with the B-1 items in the same files)                                                                                                                        |
| ---- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| b-11 | single-leaf coalescing                                                                   | single-leaf coalescing; pipelined requests; at most two re-reads; whole-patch validation, apply order, `applied`/`failed`; `requires`; live-topic filtering; section 2; the limits |
| b-12 | the restart quiet time; the wake read and the `wake()` step                              | `restart` write answered on acceptance; `asleep: ignored`/`unknown`; the wake read and the `touched` retry; the settle bound; `hello` timing                                       |
| b-13 | the continuing-motion record and the daemon's own stops at shutdown                      | `gimbal.turn` stopped on close unless `keep`; no other stop; stops overtake the queue; the deadline; `busy`; closed connections; the queue of a device that goes                   |
| b-14 | identity attempts 2 and 3 and the wake series; the skeleton's crate-wide `allow` removed | the `port:` → `sn:` hand-over and its events; the identity series ending at the first complete answer; device-number change; unavailable devices; lock, activation, binary         |

Not in B-1 and not in the chain: the real device path (`fake: None` returns an error naming B-2), which is B-2, task b-15; it waits for track A.

For the coordinator: tracks C and D can start their live tests once the key `b-10-daemon` has landed; nothing earlier serves a socket. Track C's restart rows (its task C-13) need no quiet time: on B-1 a `restart` write is answered after its touched re-read fails (the fake is gone during `fake::RESTART`, and an operation on it records nothing), and the device comes back as ordinary hotplug. A turn's stop at connection close arrives with b-13; track C's parity row sends its turn with `keep:true`, and track D's end-to-end test sends no turn.

## Decisions this plan takes

Engineering choices the spec leaves open; each can be overruled in review.

1. **`Timing` and `run_with`.** Every interval comes from `Timing`; `run` passes `Timing::SPEC`, pinned by a test to the contract constants; in-process tests pass shorter tiers and scans to the doc-hidden `run_with`. Reason: the 60 s and 5 s tiers would otherwise make the named tests take minutes, and `Config` is fixed by W0.
2. **B-1 without the real bus.** `run` with `fake: None` returns `Err(Unsupported)` naming B-2 until task b-15. Reason: the real bus is track A's; guessing its name would put a contract into this track.
3. **Hidden entries.** A supported device's entry exists from its scan on but is listed (and routable) only when its worker announces it after the first read attempt, or when a `hello` reaches its deadline (then with every leaf `null` and in `unread`). Reason: 5.2 step 4 and 4.1 `hello` timing in one mechanism.
4. **Plugs.** The registry numbers every enumeration (`PlugId`); workers address the table by plug, so a merge into an unplugged `sn:` entry or a hand-over never changes what a worker holds. Reason: ids change, enumerations do not.
5. **Per-tier ticks.** A tick reads every group currently in that tier (live tier while its topic is subscribed by a connection with `subscribe:true`); its next tick counts from the tick's end. Identity attempts 2 and 3 ride the first two ticks of the model's identity tier after the first read attempt (the first read attempt itself is not a tick). Reason: "Intervals count from the end of the previous read of that tier" and "with the next two reads of the tier" (5.5, 5.7).
6. **The current state inside a batch** is the published state with the earlier requests' written leaves (their echoes) applied; each `set` and `do` of the batch is validated against it. Reason: 4.1 rule 5 "against the current state with the patch applied"; review focus 2. Provisional until product question 3 is answered: the same two requests in two batches are refused `conflict` while the mode's readback lags, because `pending` does not count as the current state.
7. **Coalescing.** Within one batch, a `set` whose patch has exactly one leaf is answered `ok` (the batch's rev) without being sent when a later `set` of the same connection in the batch has the same single leaf (normalised by `validate::leaves`, so dotted and nested match) and a value the setting can take (`check_value` with `Fit::Write`, not read-only), and every request between the two, of any connection, is a `get` or a single-leaf `set` of the same leaf (a write of another leaf or an action in between could make the later one a `conflict` after the earlier one was answered `ok` unsent). The earlier request is validated first; a refusal is answered as a refusal. Coalescing is added after B-1, in b-11 (decision 24). Reason: 4.1 rule 9; review focus 3.
8. **Batches.** A stop runs as a batch of its own, before everything queued; a request with a `restart` warning (a leaf's or the action's) runs alone and ends the batch before it (5.6); otherwise a batch is every queued request up to the first such request. Requests after a camera failure go back to the front of the queue (4.1 rule 9).
9. **Sent value and echo.** `apply` and `act` get the value the request names (section 2), with one change: a stepped number, which the validator accepted within 1e-6 steps of a step, is put on that step (`pending::sent`); nothing is rounded to a precision before it is sent (fake-a's `gimbal.move pan 10.04` is sent as 10.04, a zoom factor of 2.345 as 2.345). The _echo_ (`pending::echo`: rounded to the precision, or put on the step without one) is what `pending` holds, what a `readback:false` leaf shows once accepted, and what later requests of the batch are validated against; readbacks are compared with it after rounding (`pending::shows`). Reason: 4.2 "Precision" puts rounding where values are created (decode and echo), not on what is sent; the step is kept because an encoder of whole numbers refuses a fraction (track A's encoders do); review focus 1.
10. **Touched re-read while asleep.** A touched group that cannot be read asleep is not read (no query the camera is known not to answer); a `touched`-tier group skipped this way has its leaves put in `unread` and waits for the wake read; a periodic group waits for its next tick after wake, and a written leaf of it stays `pending` with its bound unstarted (5.7). Reason: 5.7 and 4.2 `unread`.
11. **A failed status read at plug-in counts as awake**: the first read attempt then reads every group. Reason: 5.2 step 3 names only the two outcomes "awake" and "asleep"; reading groups that cannot answer only puts them in `unread`.
12. **Unread excludes `readback:false` leaves.** They are never read, so they are never in `unread`; they are `null` until written and again after unplug (4.2 lifetime). Reason: 4.2 "Absent vs null".
13. **Refusals carry no `rev`**; only `camera` errors do, with the rev of the batch's re-read state (none when the device went). Reason: 4.1 "`rev` is present whenever the daemon re-read the device before answering", read as "re-read for this request".
14. **`keep`.** The worker reads the parameter `keep` of a `Motion::Continuing` action as `params["keep"] == true`; it is still passed to `act` with the other parameters. Reason: 4.4 and 5.6 name `keep:true` as the protocol's way to say so.
15. **A stop counts toward the 32.** `busy` applies to every request, stops included. Reason: 4.1 `busy` "32 requests are already queued"; no exception is written.
16. **Duplicate resolution order.** "The device whose serial was read later" is the one whose answer reaches the device table later; two serials of one scan are not reordered by scan position. Reason: workers read in parallel; the fakes cannot produce a duplicate, and two cameras with the same serial need a second unit (CAM-26). This departs from 5.5's "for two serials read in one enumeration pass, the one later in the order of the scan"; provisional until product question 4 is answered.
17. **Open failure.** When `Bus::open` fails, the worker logs it, lists the device under its `port:` id with every leaf `null` and in `unread`, answers `get` from that state and `set`/`do` with `camera` ("the camera could not be opened: …"), sends nothing and waits for the unplug. Reason: "A failed read never sets `available:false`" (5.3).
18. **Own sockets removed at exit.** A socket file the daemon bound, and the test-control socket, are removed at a clean exit; an inherited socket file is left to systemd. Reason: 5.9 removes a stale socket at start anyway; this only avoids leaving one.
19. **Exit codes of the binary:** 0 after a clean stop (SIGTERM, SIGINT), 2 for a bad command line, 1 for every other error (lock held, socket path unset, a fatal panic). Reason: 5.9 "exits non-zero", "exits 0"; 2 follows the CLI's usage code.
20. **Signals** are blocked in `main` before any thread starts and taken by one `sigwait` thread that sets `Config::stop`. Reason: no handler code runs in signal context.
21. **Undecodable values** (`raw:<n>` from a model) are logged once per device, leaf and value by the worker. Reason: 4.2 "logged once per device and value".
22. **Refused frames are logged.** Every `CameraError::NotInSendTable` the worker gets from a camera operation (identity attempt, read, `apply`, `act`) is logged through `names::log_refused_frame`; a request it fails is answered `internal`, a read leaves its group's leaves in `unread`. Reason: 8.3 "refused with `internal`, logged and never sent".
23. **B-1 and B-2 are milestones of several PRs.** Spec 10.3 says track B comes "in two PRs"; this plan delivers B-1 as ten PRs (b-01 to b-10) and the rest of track B as five more (b-11 to b-15). Reason: the coordinator's process is one task per PR with one review each, and tasks of one wave never share a file; B-1 is still one milestone that tracks C and D wait for, B-2 one task.
24. **The worker's features after B-1 come with their tests.** The plain worker (b-07) leaves out coalescing, the restart quiet time, the wake read, the continuing-motion record with the own stops at shutdown, and identity attempts 2 and 3; the chain b-11 to b-14 adds one each, in the task that writes the named tests pinning it. The chain is stacked (one task after the other), so its tasks may fix any module of the crate in the same PR without two PRs of one wave touching one file. Reason: a small, early B-1 for tracks C and D, and no feature without a failing test first.

## Product questions (for the coordinator; not built)

1. Should a stop (`gimbal.stop`, `zoom.stop`) be accepted when 32 requests are queued, instead of `busy`? Recommendation: yes, as an additive exception to `busy` in 4.1 rule 2, because a user who floods a device and then presses stop gets nothing; this plan follows the spec text (decision 15) until the spec says otherwise.
2. Should the daemon log anything when a `keep:true` turn outlives its connection or the daemon? Recommendation: no; 15.1 row 89 makes it ordinary camera state.
3. Does a `pending` value count as the current state for `requires` and `asleep` checks? Recommendation: yes, so that `set exposure.mode=manual` followed by `set exposure.time=…` is accepted whether or not both land in one batch; decision 6 then becomes the general rule instead of a batch-only one. Built today: decision 6 (within one batch only), provisional.
4. Should 5.5's duplicate rule read "the serial that reaches the device table later" instead of "the one later in the order of the scan" for two serials of one enumeration pass? Recommendation: yes; workers read in parallel, and the scan order cannot be tested without two cameras with one serial (CAM-26). Built today: decision 16, provisional.

## Decisions taken after the plan review (2026-11-13)

The coordinator decided the product questions of the review and the seam points between the tracks. Where a line below differs from the text of this plan, the line wins; the code of the plan is otherwise unchanged. The product questions above are answered by these lines.

1. **b-05.** Question 3, stops when the queue is full: a stop (`gimbal.stop`, `zoom.stop`) is accepted even when 32 requests are queued; every other request still gets `busy` at 32 (spec 4.1 rule 2). Task b-05 changes `DeviceQueue::push` so that a stop job is inserted whatever the length; decision 15 is replaced by this line, and the last assertion of `the_33rd_queued_request_is_busy` ("a stop counts too") becomes one that pins the stop being accepted and placed before the queued requests. Check `busy_is_answered_by_the_reader` (b-08) and `the_request_after_32_queued_ones_is_busy` (b-13) still hold; they send no stop.
2. **b-13.** Question 4, a `keep:true` turn that outlives its connection or the daemon: log nothing; it is ordinary camera state (15.1 row 89). Task b-13 builds it as the plan says; nothing changes.
3. **b-07.** Question 5, a pending value counts as current for `requires` and `asleep` checks: decision 6 is the general rule, not a batch-only one. Task b-07 builds `Worker::view()` so that the published state's `pending` values are laid over the leaves (the pending value is the leaf's value), and `batch` starts from it (`working = self.view()`), so `set exposure.mode=manual` followed, in a later batch, by `set exposure.time=…` is accepted while the mode is still pending. Add one unit test of `view` for this; `a_pipelined_requirement_counts_within_one_batch` (b-11) stays. The coalescing guard of b-11 (nothing in between can change what the later request is validated against) is unchanged.
4. **b-06.** Question 6, two cameras with one serial: the duplicate is the serial that reaches the device table later, not the one later in the scan order. Decision 16 and `table::tests::a_duplicate_serial_stays_on_its_port_id` already build this; spec 5.5 now says it; nothing changes.
5. **b-01.** Socket paths in tests (seam point of the cross-check): a Unix socket path holds at most 107 bytes, and the daemon adds `.lock` and `.test` to the socket path. `tests/common::temp_dir` asserts that `dir.join("s.sock")` is at most 100 bytes, with the message `socket path too long for a Unix socket: <path>`, as track C's `common::socket` does; the tests of b-02, b-10 and b-15 that build their own socket directory assert the same, so that a deep `TMPDIR` fails loudly.
6. **b-10.** Milestone B-1 is the task b-10 landed on `rebuild`, built in ten pull requests (b-01 to b-10); every gate of the other tracks names b-10 (spec 10.3 B now says so). The integration test file of this task is `crates/obsbot4linuxd/tests/core.rs` (this plan says `core` everywhere; it was `live`), so that `cargo test --test core -- --ignored` without `-p` cannot pick up track A's `tests/live.rs`.
7. **b-15.** Seam to track A: `SysfsBus::with_root` takes one argument (`with_root(root: impl Into<PathBuf>)`, always the real registry); the test calls `SysfsBus::with_root(dir.join("sys"))`. The task starts only after a-13 and b-14 have landed. If track A's landed names differ, the landed names win in the one `daemon.rs` line and the one test.

## File structure

| File                                                           | Responsibility                                                                                                                                                                               | Task                                    |
| -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| `crates/obsbot4linuxd/Cargo.toml` (modify)                     | `nix` (`fs`, `signal`) and `serde_json`                                                                                                                                                      | b-01                                    |
| `crates/obsbot4linuxd/src/lib.rs` (replace)                    | `Config`, `run` (unchanged API), `Timing`, `run_with`, limits, module list                                                                                                                   | b-01; b-14 drops the skeleton's `allow` |
| `src/outbound.rs`                                              | connection id, bounded outbound queue, `mark_closed`, reply lines                                                                                                                            | b-01                                    |
| `src/names.rs`                                                 | device entries, first states, log lines                                                                                                                                                      | b-01                                    |
| `src/hotplug.rs`                                               | diff of two scans by path and device number                                                                                                                                                  | b-01                                    |
| `src/daemon.rs`                                                | `Fatal`, `critical` (b-01); start-up, threads, sweeper, shutdown (b-10); the bus choice (b-15)                                                                                               | b-01, b-10, b-15                        |
| `src/worker.rs`                                                | `Plugged`, `spawn` (b-01); the plain worker loop, first read, ticks, end (b-07); coalescing plan (b-11); quiet time and wake read (b-12); own stops (b-13); identity attempts 2 and 3 (b-14) | b-01, b-07, b-11 to b-14                |
| `src/identity.rs`                                              | `PortReason`, `choose_id` (b-01); identity series (b-04)                                                                                                                                     | b-01, b-04                              |
| `src/queue.rs`                                                 | job types (b-01); device queue, batch plan, motion record (b-05)                                                                                                                             | b-01, b-05                              |
| `src/table.rs`                                                 | device table types (b-01, stub); the table (b-06)                                                                                                                                            | b-01, b-06                              |
| `src/lines.rs`, `src/socket.rs`, `src/args.rs`                 | bounded line reader; lock, bind, activation; command line                                                                                                                                    | b-02                                    |
| `src/pending.rs`, `src/schedule.rs`                            | `pending`, the sent value and the echo; poll schedule                                                                                                                                        | b-03                                    |
| `src/batch.rs`                                                 | the plain write path (b-07); coalesced answers (b-11); restart answers (b-12); the motion record (b-13)                                                                                      | b-07, b-11 to b-13                      |
| `src/conn.rs`                                                  | acceptor, connections, routing                                                                                                                                                               | b-08                                    |
| `src/registry.rs`                                              | registry thread                                                                                                                                                                              | b-09                                    |
| `src/control.rs`, `src/main.rs`                                | test-control socket; the binary                                                                                                                                                              | b-10                                    |
| `crates/obsbot4linuxd/tests/common/mod.rs`, `tests/harness.rs` | in-process daemon, line client, control client                                                                                                                                               | b-01                                    |
| `tests/core.rs`                                                | B-1 live acceptance                                                                                                                                                                          | b-10                                    |
| `tests/requests.rs`, `tests/limits.rs`                         | write path, coalescing, queue rules, limits                                                                                                                                                  | b-11                                    |
| `tests/asleep.rs`, `tests/startup.rs`                          | asleep, wake, retries, restart, start-up                                                                                                                                                     | b-12                                    |
| `tests/deadline.rs`, `tests/motion.rs`                         | deadline, busy, dropped requests, unplug; stops and turns                                                                                                                                    | b-13                                    |
| `tests/hotplug.rs`, `tests/process.rs`                         | hotplug, hand-over, identity; lock, activation, binary                                                                                                                                       | b-14                                    |

## Tasks, waves and dependencies

A task branches from `rebuild` after its dependencies have landed; where its one dependency is still in review it is stacked on that branch ("stacked on"). A task with several dependencies starts once all have landed. No two tasks of one wave edit the same file. The skeleton (b-01) writes every module file, so tasks up to b-10 never edit `lib.rs` or another task's file; the stub bodies (`todo!("task …")`) keep the crate compiling meanwhile. After B-1 (b-10), the tasks b-11 to b-14 form a stacked chain, one per wave: each adds one worker feature to `worker.rs` and `batch.rs` together with the named tests that pin it, and may fix any module of the crate that one of its tests shows to be wrong (a defect of B-1), saying so in its PR. b-15 comes last because b-14 may touch `daemon.rs` and `lib.rs`.

| Task                       | Title                                                                       | Wave | Depends on                              | Stacked on | Effort |
| -------------------------- | --------------------------------------------------------------------------- | ---- | --------------------------------------- | ---------- | ------ |
| b-01-skeleton              | Crate skeleton, outbound queue, names, scan diff, test harness              | 1    | –                                       | –          | M      |
| b-02-lines-socket          | Bounded line reader, lock and socket, command line                          | 2    | b-01                                    | b-01       | S      |
| b-03-pending-schedule      | `pending`, sent value and echo, poll schedule                               | 2    | b-01                                    | b-01       | M      |
| b-04-identity              | Identity series                                                             | 2    | b-01                                    | b-01       | S      |
| b-05-queue                 | Device queue, batch plan, continuing-motion record                          | 2    | b-01                                    | b-01       | M      |
| b-06-table                 | Device table                                                                | 2    | b-01                                    | b-01       | M      |
| b-07-worker                | Plain worker and write path                                                 | 3    | b-03, b-04, b-05, b-06                  | –          | M      |
| b-08-conn                  | Acceptor and connections                                                    | 3    | b-02, b-05, b-06                        | –          | M      |
| b-09-registry              | Registry thread                                                             | 3    | b-05, b-06                              | –          | S      |
| b-10-daemon                | Daemon, test-control socket, binary (B-1)                                   | 4    | b-07, b-08, b-09                        | –          | M      |
| b-11-tests-requests        | Coalescing; tests: write path, queue rules, limits                          | 5    | b-10                                    | b-10       | M      |
| b-12-tests-asleep-startup  | Restart quiet time and wake read; tests: asleep, wake, restart, start-up    | 6    | b-11                                    | b-11       | M      |
| b-13-tests-deadline-motion | Motion record and own stops; tests: deadline, busy, unplug, stops and turns | 7    | b-12                                    | b-12       | M      |
| b-14-tests-hotplug-process | Identity attempts 2 and 3; tests: hotplug, hand-over, identity, process     | 8    | b-13                                    | b-13       | M      |
| b-15-real-bus              | Real device path (B-2)                                                      | 9    | b-14; track A's `a-13-registry-and-bus` | –          | S      |

`Cargo.lock` changes only in b-01 (`nix` gains the `signal` feature; no new package). On a rebase conflict in it, take `rebuild`'s version and run `cargo build --workspace`.

The code of this plan was built in a copy of the workspace with the W0 plan's code (see "Proof" at the end).

---

### Task b-01 (key `b-01-skeleton`): Crate skeleton, outbound queue, names, scan diff, test harness

**Files:**

- Modify: `crates/obsbot4linuxd/Cargo.toml`
- Replace: `crates/obsbot4linuxd/src/lib.rs`, `crates/obsbot4linuxd/src/main.rs`
- Create (complete): `src/outbound.rs`, `src/names.rs`, `src/hotplug.rs`, `src/daemon.rs` (its `run` a stub), `src/worker.rs` (its `run` a stub)
- Create (stubs that fix the signatures later tasks fill): `src/identity.rs`, `src/queue.rs`, `src/table.rs`
- Create (one doc line each, filled by later tasks): `src/lines.rs`, `src/socket.rs`, `src/args.rs`, `src/pending.rs`, `src/schedule.rs`, `src/batch.rs`, `src/conn.rs`, `src/registry.rs`, `src/control.rs`
- Test: `crates/obsbot4linuxd/tests/common/mod.rs`, `tests/harness.rs`; unit tests inside `lib.rs`, `outbound.rs`, `names.rs`, `hotplug.rs`, `daemon.rs`

**Interfaces:**

- Consumes (W0): `obsbot4linuxd::Config`, `run` (W0 Task 11, kept as they are); `obsbot_proto::{Device, Description, State, Support, Usb, ErrorBody, ErrorCode, OkBody, Reply, ReplyBody, RequestId, short_id, port_id, sn_id, is_valid_serial, REQUEST_DEADLINE}`; `obsbot_device::contract::{Found, ModelEntry, Tier, Bus, Identity, IdentitySource, Need, Motion}`; `obsbot_device::models::fake::{FakeBus, KEYS, Op, description}`.
- Produces (complete in this task): `obsbot4linuxd::{Timing { scan, request_deadline, second, five_seconds, minute }, Timing::SPEC, Timing::period(Tier) -> Option<Duration>, run_with(Config, Timing) -> io::Result<()>, MAX_CONNECTIONS = 64, MAX_QUEUED = 32, OUTBOUND_LINES = 64, ACCEPT_BACKOFF = 100 ms, SWEEP_EVERY = 50 ms}`; `outbound::{ConnId = u64, Outbound::{new(ConnId, Option<UnixStream>, usize) -> (Outbound, Receiver<String>), conn, set_client, send(String) -> bool, close, mark_closed, is_closed}, ok_line(&RequestId, OkBody) -> String, error_line(Option<&RequestId>, ErrorBody) -> String, error(ErrorCode, &str) -> ErrorBody, log_full(ConnId, &str) -> String}`; `names::{name_of, supported_device, unsupported_device, initial_state, log_appeared, log_handover, log_gone, log_open_failed, log_raw, log_refused_frame}`; `hotplug::{Seen { path, devnum }, Seen::of, Diff { gone, new }, diff}`; `daemon::{Fatal::{new, set, get}, critical(String, Arc<Fatal>, F) -> io::Result<JoinHandle<()>>}`; `worker::{Plugged { plug, found, model, bus, table, queue, timing }, spawn(Plugged, Arc<Fatal>) -> io::Result<JoinHandle<()>>}`; `identity::{PortReason::{NoSerial, InvalidSerial, Duplicate}, PortReason::text, choose_id}`; `queue::{Work, Job { id, out, conn, device, work, arrived, stop }, Job::{own_stop, answer, fail}, DeviceQueue::new, DeviceQueue { cut, halt, ended }}`.
- Produces (signatures only; bodies `todo!("task …")`, filled by b-04, b-05, b-06): every other item of `identity.rs`, `queue.rs` and `table.rs` as written below.
- Test harness (`tests/common/mod.rs`, used by every integration test of this track): `fast() -> Timing`; `A`, `B`, `UNKNOWN` (`sn:FAKEA0000001`, `sn:FAKEB0000001`, `port:9-3`); `temp_dir()`; `Daemon::{start, start_with(&[&str], Timing, impl FnOnce(&FakeBus)), connect, hello(bool, &[&str]) -> (Client, Value), control(&str) -> Value, ops(&str) -> Vec<Op>, stop() -> io::Result<()>, ended}` with fields `socket`, `bus`, `stop`; `Client::{connect, id, send, send_raw, line(Duration) -> Option<Value>, reply(i64) -> Value, hello, get, set, act, event(Duration, pred) -> Option<Value>, closed_within(Duration) -> bool, close}` with field `events`; `state_of(&'static str)`, `devices(&Value)`, `ids(&Value) -> Vec<String>`, `at(&Value, &str)`, `writes_after(&Daemon, &str, usize) -> Vec<Op>`, `eventually(Duration, impl Fn() -> bool) -> bool`.

The skeleton allows `dead_code`, `unused_variables` and `unused_imports` crate-wide, because its stubs leave items unused; b-14 removes that line, after the chain b-11 to b-14 has wired the last worker feature (until then the pure modules hold items only those tasks call).

- [ ] **Step 1: Write the failing tests**

In `crates/obsbot4linuxd/Cargo.toml`, replace the `[dependencies]` table with:

```toml
[dependencies]
nix = { workspace = true, features = ["fs", "signal"] }
obsbot-device.workspace = true
obsbot-proto.workspace = true
serde_json.workspace = true
```

Create `crates/obsbot4linuxd/tests/common/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The harness of the daemon's integration tests (design spec 11): an in-process daemon on a
//! temporary socket with the fake bus `fake-a,fake-b,unknown`, a line client that speaks
//! protocol 2 by hand (the client library is track C's), and the test-control socket.
//! Each test file uses the parts it needs.

#![allow(dead_code)]

use obsbot_device::models::fake::{FakeBus, KEYS, Op};
use obsbot4linuxd::{Config, Timing, run_with};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Short tier periods and scans, so that the 5 s and 60 s tiers and hotplug happen within a
/// test; the request deadline stays the spec's 15 s.
pub fn fast() -> Timing {
    Timing {
        scan: Duration::from_millis(100),
        request_deadline: obsbot_proto::REQUEST_DEADLINE,
        second: Duration::from_millis(100),
        five_seconds: Duration::from_millis(500),
        minute: Duration::from_secs(3),
    }
}

/// The ids the fake bus gives the three devices of `KEYS`.
pub const A: &str = "sn:FAKEA0000001";
pub const B: &str = "sn:FAKEB0000001";
pub const UNKNOWN: &str = "port:9-3";

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A fresh, short directory for one daemon's sockets (a Unix socket path has at most 107
/// bytes): `$TMPDIR/o4ld-<pid>-<n>`.
pub fn temp_dir() -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("o4ld-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One in-process daemon in fake mode.
pub struct Daemon {
    pub socket: PathBuf,
    pub bus: Arc<FakeBus>,
    pub stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<std::io::Result<()>>>,
    dir: PathBuf,
}

impl Daemon {
    /// `fake-a,fake-b,unknown` with [`fast`] timing.
    pub fn start() -> Daemon {
        Daemon::start_with(&KEYS, fast(), |_| {})
    }

    /// `models` with `timing`; `setup` runs on the fake bus before the daemon starts (a delay,
    /// a `fail_next`).
    pub fn start_with(models: &[&str], timing: Timing, setup: impl FnOnce(&FakeBus)) -> Daemon {
        let dir = temp_dir();
        let socket = dir.join("s.sock");
        let bus = Arc::new(FakeBus::new(models).expect("fake models"));
        setup(&bus);
        let stop = Arc::new(AtomicBool::new(false));
        let config = Config {
            socket: socket.clone(),
            fake: Some(bus.clone()),
            stop: stop.clone(),
        };
        let handle = std::thread::spawn(move || run_with(config, timing));
        let until = Instant::now() + Duration::from_secs(5);
        while UnixStream::connect(&socket).is_err() {
            assert!(
                Instant::now() < until,
                "the daemon did not bind {}",
                socket.display()
            );
            assert!(!handle.is_finished(), "the daemon ended at start-up");
            std::thread::sleep(Duration::from_millis(10));
        }
        Daemon {
            socket,
            bus,
            stop,
            handle: Some(handle),
            dir,
        }
    }

    /// A raw connection.
    pub fn connect(&self) -> Client {
        Client::connect(&self.socket)
    }

    /// A connection after `hello`; returns it and the `ok` payload of the reply.
    pub fn hello(&self, subscribe: bool, live: &[&str]) -> (Client, Value) {
        let mut c = self.connect();
        let ok = c.hello(subscribe, live);
        (c, ok)
    }

    /// One line on the test-control socket (5.12); returns the reply.
    pub fn control(&self, line: &str) -> Value {
        let mut path = self.socket.as_os_str().to_owned();
        path.push(".test");
        let mut s = UnixStream::connect(PathBuf::from(path)).expect("the test-control socket");
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        writeln!(s, "{line}").unwrap();
        let mut reply = String::new();
        BufReader::new(s).read_line(&mut reply).unwrap();
        serde_json::from_str(&reply).expect("a JSON reply")
    }

    /// The operations the fake device `id` received, oldest first.
    pub fn ops(&self, id: &str) -> Vec<Op> {
        self.bus.ops(id)
    }

    /// Stops the daemon as SIGTERM would and returns what `run` returned.
    pub fn stop(mut self) -> std::io::Result<()> {
        self.finish().expect("the daemon was running")
    }

    /// Whether `run` has returned (a fatal error ends it on its own).
    pub fn ended(&self) -> bool {
        self.handle.as_ref().is_none_or(|h| h.is_finished())
    }

    fn finish(&mut self) -> Option<std::io::Result<()>> {
        let handle = self.handle.take()?;
        self.stop.store(true, Ordering::SeqCst);
        let until = Instant::now() + Duration::from_secs(20);
        while !handle.is_finished() {
            assert!(
                Instant::now() < until,
                "run did not return within 20 s of stop"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        Some(handle.join().expect("run panicked"))
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = self.finish();
        } else {
            self.stop.store(true, Ordering::SeqCst);
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A protocol-2 connection that keeps the events it reads while it waits for a reply.
pub struct Client {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    /// Events read and not yet taken, oldest first.
    pub events: Vec<Value>,
    next: i64,
}

impl Client {
    pub fn connect(path: &std::path::Path) -> Client {
        let stream = UnixStream::connect(path).expect("connect");
        let reader = BufReader::new(stream.try_clone().unwrap());
        Client {
            stream,
            reader,
            events: Vec::new(),
            next: 100,
        }
    }

    /// A fresh request id.
    pub fn id(&mut self) -> i64 {
        self.next += 1;
        self.next
    }

    pub fn send(&mut self, v: &Value) {
        self.send_raw(format!("{v}\n").as_bytes());
    }

    pub fn send_raw(&mut self, bytes: &[u8]) {
        self.stream.write_all(bytes).expect("write");
    }

    /// The next line from the daemon; `None` at the end of the stream or after `timeout`.
    pub fn line(&mut self, timeout: Duration) -> Option<Value> {
        self.stream.set_read_timeout(Some(timeout)).unwrap();
        let mut text = String::new();
        match self.reader.read_line(&mut text) {
            Ok(0) => None,
            Ok(_) => Some(serde_json::from_str(&text).expect("the daemon sends JSON lines")),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => None,
            Err(e) if e.kind() == ErrorKind::ConnectionReset => None,
            Err(e) => panic!("read: {e}"),
        }
    }

    /// Reads until the reply with `id`; events on the way go to `events`. Panics after 20 s.
    pub fn reply(&mut self, id: i64) -> Value {
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            let left = until.saturating_duration_since(Instant::now());
            let line = self
                .line(left.max(Duration::from_millis(1)))
                .unwrap_or_else(|| panic!("no reply to {id}"));
            if line.get("event").is_some() {
                self.events.push(line);
            } else if line["id"] == json!(id) {
                return line;
            } else {
                panic!("a reply to another id: {line}");
            }
        }
    }

    /// Sends `hello` and returns the `ok` payload.
    pub fn hello(&mut self, subscribe: bool, live: &[&str]) -> Value {
        let id = self.id();
        self.send(
            &json!({"id": id, "hello": {"protocol": 2, "client": "test 1.0",
            "subscribe": subscribe, "live": live}}),
        );
        let r = self.reply(id);
        r.get("ok")
            .cloned()
            .unwrap_or_else(|| panic!("hello refused: {r}"))
    }

    pub fn get(&mut self, device: &str, paths: Option<&[&str]>) -> Value {
        let id = self.id();
        self.send(&json!({"id": id, "device": device, "get": paths}));
        self.reply(id)
    }

    pub fn set(&mut self, device: &str, patch: Value) -> Value {
        let id = self.id();
        self.send(&json!({"id": id, "device": device, "set": patch}));
        self.reply(id)
    }

    pub fn act(&mut self, device: &str, action: &str, with: Value) -> Value {
        let id = self.id();
        self.send(&json!({"id": id, "device": device, "do": action, "with": with}));
        self.reply(id)
    }

    /// The first event (already read or read now) for which `pred` holds; the events before it
    /// are dropped. `None` after `timeout`.
    pub fn event(&mut self, timeout: Duration, pred: impl Fn(&Value) -> bool) -> Option<Value> {
        if let Some(i) = self.events.iter().position(&pred) {
            let ev = self.events.remove(i);
            self.events.drain(..i);
            return Some(ev);
        }
        self.events.clear();
        let until = Instant::now() + timeout;
        loop {
            let left = until.checked_duration_since(Instant::now())?;
            let line = self.line(left)?;
            if line.get("event").is_some() && pred(&line) {
                return Some(line);
            }
        }
    }

    /// Whether the daemon closes the connection within `timeout` (lines before the end are
    /// read and dropped).
    pub fn closed_within(&mut self, timeout: Duration) -> bool {
        let until = Instant::now() + timeout;
        self.stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let mut buf = Vec::new();
        loop {
            if Instant::now() >= until {
                return false;
            }
            buf.clear();
            match self.reader.read_until(b'\n', &mut buf) {
                Ok(0) => return true,
                Ok(_) => continue,
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(_) => return true,
            }
        }
    }

    /// Closes this end.
    pub fn close(self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}

/// A state event of `device`.
pub fn state_of(device: &'static str) -> impl Fn(&Value) -> bool {
    move |e: &Value| e["event"] == "state" && e["device"] == device
}

/// A devices event.
pub fn devices(e: &Value) -> bool {
    e["event"] == "devices"
}

/// The ids of a devices list, with `!` for an unavailable entry.
pub fn ids(devices: &Value) -> Vec<String> {
    devices
        .as_array()
        .expect("a device list")
        .iter()
        .map(|d| {
            let mut id = d["id"].as_str().unwrap().to_string();
            if d["available"] == json!(false) {
                id.push('!');
            }
            id
        })
        .collect()
}

/// The value at a dotted path of a state object.
pub fn at<'a>(state: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(state, |v, k| &v[k])
}

/// The ops of `id` after the first `skip`, without reads.
pub fn writes_after(daemon: &Daemon, id: &str, skip: usize) -> Vec<Op> {
    daemon.ops(id)[skip..]
        .iter()
        .filter(|op| !matches!(op, Op::Read(_)))
        .cloned()
        .collect()
}

/// Polls `cond` every 20 ms until it holds or `timeout` passes.
pub fn eventually(timeout: Duration, cond: impl Fn() -> bool) -> bool {
    let until = Instant::now() + timeout;
    while Instant::now() < until {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}
```

Create `crates/obsbot4linuxd/tests/harness.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The test harness compiles from the skeleton on, and its timing keeps the spec's request
//! deadline: only tier periods and scans are shortened for tests (spec 11).

mod common;

#[test]
fn the_test_timing_shortens_only_tiers_and_scans() {
    let t = common::fast();
    assert_eq!(t.request_deadline, obsbot_proto::REQUEST_DEADLINE);
    assert!(t.second < t.five_seconds && t.five_seconds < t.minute);
    assert_eq!(
        common::temp_dir().parent(),
        Some(std::env::temp_dir().as_path())
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `nice -n 19 cargo test -p obsbot4linuxd --test harness -j 4`
Expected: FAIL to compile: `unresolved imports obsbot4linuxd::Timing, obsbot4linuxd::run_with` (W0's stub has neither).

- [ ] **Step 3: Write the skeleton**

Replace `crates/obsbot4linuxd/src/lib.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The obsbot4linux daemon (design spec 5) as a library, so that tests run it in-process on a
//! temporary socket (section 11). `Config` and `run` are the entry point fixed in stage W0.
//!
//! The daemon knows devices, groups, tiers, leaves, actions and live topics. It knows no
//! command id, no offset and no selector of any model: everything model-specific comes through
//! `trait Camera` and `trait Bus` (section 5, 8.2).

// Removed by task b-14-tests-hotplug-process, the last task that wires a worker feature;
// until then the stubs of the skeleton and the features still to come leave items unused.
#![allow(dead_code, unused_variables, unused_imports)]

mod batch;
mod conn;
mod control;
mod daemon;
mod hotplug;
mod identity;
mod lines;
mod names;
mod outbound;
mod pending;
mod queue;
mod registry;
mod schedule;
mod socket;
mod table;
mod worker;

#[doc(hidden)]
pub mod args;

use obsbot_device::contract::Tier;
use obsbot_device::models::fake::FakeBus;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// At most this many connections at once; a further one is closed at once (4.1).
pub const MAX_CONNECTIONS: usize = 64;
/// At most this many requests queued per device; one more is answered `busy` (4.1, 5.6).
pub const MAX_QUEUED: usize = 32;
/// The outbound queue of one connection; when it is full the connection is closed (5.1).
pub const OUTBOUND_LINES: usize = 64;
/// The pause after a failed `accept` (5.8 `ACCEPT_BACKOFF`).
pub const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);
/// How often queued requests are checked against their deadline (4.1 rule 9).
pub const SWEEP_EVERY: Duration = Duration::from_millis(50);

/// How one daemon runs.
pub struct Config {
    /// The socket to bind when systemd passes none (`LISTEN_FDS`). The lock file is
    /// `<socket>.lock` (5.9); in fake mode the test-control socket is `<socket>.test` (5.12).
    pub socket: PathBuf,
    /// `None`: the real devices (the sysfs scan). `Some`: fake mode with these fake devices
    /// (`obsbot4linuxd --fake fake-a,fake-b,unknown` builds it with `FakeBus::new`).
    pub fake: Option<Arc<FakeBus>>,
    /// Set to `true` to end `run` as on SIGTERM (5.9).
    pub stop: Arc<AtomicBool>,
}

impl Config {
    /// Real devices on `socket`, not stopped.
    pub fn new(socket: PathBuf) -> Config {
        Config {
            socket,
            fake: None,
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// The intervals the daemon runs on. `run` always uses [`Timing::SPEC`]; in-process tests
/// may pass shorter tier periods and scan intervals to [`run_with`], so that a 60 s tier or a
/// 5 s identity attempt is seen within a test's time. Not part of the daemon's interface.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// The sysfs (or fake bus) scan interval (5.3: once a second).
    pub scan: Duration,
    /// `REQUEST_DEADLINE` (4.1 rule 9, 5.8); also the `hello` bound (4.1) and the quiet time
    /// after a `restart` write (5.6).
    pub request_deadline: Duration,
    /// The periods of the tiers `Second`, `FiveSeconds` and `Minute` (5.7).
    pub second: Duration,
    pub five_seconds: Duration,
    pub minute: Duration,
}

impl Timing {
    /// The values of the design spec: scan 1 s, deadline 15 s, tiers 1 s, 5 s and 60 s.
    pub const SPEC: Timing = Timing {
        scan: Duration::from_secs(1),
        request_deadline: obsbot_proto::REQUEST_DEADLINE,
        second: Duration::from_secs(1),
        five_seconds: Duration::from_secs(5),
        minute: Duration::from_secs(60),
    };

    /// The period of a tier under this timing; `None` for `Touched`.
    pub fn period(&self, tier: Tier) -> Option<Duration> {
        match tier {
            Tier::Second => Some(self.second),
            Tier::FiveSeconds => Some(self.five_seconds),
            Tier::Minute => Some(self.minute),
            Tier::Touched => None,
        }
    }
}

/// Runs the daemon until `config.stop` is set or a signal ends it. Returns once the socket,
/// the lock and every worker are released.
pub fn run(config: Config) -> std::io::Result<()> {
    run_with(config, Timing::SPEC)
}

/// [`run`] with other intervals; for in-process tests only.
#[doc(hidden)]
pub fn run_with(config: Config, timing: Timing) -> std::io::Result<()> {
    daemon::run(config, timing)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `run` must use the spec's intervals: the tier periods of the contract, the 15 s
    /// deadline of `obsbot-proto` and one scan a second (5.3, 5.7, 5.8).
    #[test]
    fn run_uses_the_intervals_of_the_spec() {
        let t = Timing::SPEC;
        for tier in [Tier::Second, Tier::FiveSeconds, Tier::Minute, Tier::Touched] {
            assert_eq!(t.period(tier), tier.period(), "{tier:?}");
        }
        assert_eq!(t.request_deadline, obsbot_proto::REQUEST_DEADLINE);
        assert_eq!(t.scan, Duration::from_secs(1));
    }

    #[test]
    fn the_daemon_limits_are_the_spec_values() {
        assert_eq!(MAX_CONNECTIONS, 64);
        assert_eq!(MAX_QUEUED, 32);
        assert_eq!(OUTBOUND_LINES, 64);
        assert_eq!(ACCEPT_BACKOFF, Duration::from_millis(100));
    }
}
```

Create `crates/obsbot4linuxd/src/outbound.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The outbound queue of one connection (5.1): at most `OUTBOUND_LINES` lines wait for the
//! connection's writer thread. When the queue is full, the connection is closed and one log
//! line names it by its connection number and the `client` of its `hello`.

use obsbot_proto::{ErrorBody, ErrorCode, OkBody, Reply, ReplyBody, RequestId};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, PoisonError};

/// The number of one connection, counted from 1 in accept order. 0 is never a connection: it
/// marks the daemon's own jobs (the stop of a continuing action, 5.6).
pub type ConnId = u64;

struct Shared {
    conn: ConnId,
    client: Mutex<String>,
    closed: AtomicBool,
    stream: Option<UnixStream>,
}

/// The sending end of a connection's outbound queue. Clones share the queue.
#[derive(Clone)]
pub struct Outbound {
    tx: SyncSender<String>,
    shared: Arc<Shared>,
}

impl Outbound {
    /// A queue of `capacity` lines for connection `conn`; `stream` (a clone of the
    /// connection's socket) is shut down when the queue overflows or [`Outbound::close`] runs.
    /// The receiver goes to the writer thread.
    pub fn new(
        conn: ConnId,
        stream: Option<UnixStream>,
        capacity: usize,
    ) -> (Outbound, Receiver<String>) {
        let (tx, rx) = sync_channel(capacity);
        let shared = Arc::new(Shared {
            conn,
            client: Mutex::new(String::new()),
            closed: AtomicBool::new(false),
            stream,
        });
        (Outbound { tx, shared }, rx)
    }

    pub fn conn(&self) -> ConnId {
        self.shared.conn
    }

    /// Records the `client` of the connection's `hello`, for the overflow log line.
    pub fn set_client(&self, client: &str) {
        *self
            .shared
            .client
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = client.to_string();
    }

    /// Queues one line. `false` when the connection is closed or the queue was full; a full
    /// queue closes the connection and logs it once.
    pub fn send(&self, line: String) -> bool {
        if self.is_closed() {
            return false;
        }
        match self.tx.try_send(line) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                let client = self
                    .shared
                    .client
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                if !self.shared.closed.swap(true, Ordering::SeqCst) {
                    eprintln!("{}", log_full(self.shared.conn, &client));
                    self.shutdown();
                }
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                self.shared.closed.store(true, Ordering::SeqCst);
                false
            }
        }
    }

    /// Closes the connection: no further line is queued, and the socket is shut down in both
    /// directions, which ends its reader and writer threads.
    pub fn close(&self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        self.shutdown();
    }

    /// Marks the connection closed without touching its socket: no further line is queued,
    /// and a worker skips every request of this connection it has not started, also one it
    /// has already taken into a batch (4.1 rule 9). The writer still sends what is queued, so
    /// a last error line reaches the client.
    pub fn mark_closed(&self) {
        self.shared.closed.store(true, Ordering::SeqCst);
    }

    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::SeqCst)
    }

    fn shutdown(&self) {
        if let Some(stream) = &self.shared.stream {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

/// The line of an `ok` reply.
pub fn ok_line(id: &RequestId, body: OkBody) -> String {
    let reply = Reply {
        id: Some(id.clone()),
        body: ReplyBody::Ok(body),
    };
    serde_json::to_string(&reply).expect("a reply always serialises")
}

/// The line of an `error` reply; `id: null` when the request had no usable id.
pub fn error_line(id: Option<&RequestId>, body: ErrorBody) -> String {
    let reply = Reply {
        id: id.cloned(),
        body: ReplyBody::Error(body),
    };
    serde_json::to_string(&reply).expect("a reply always serialises")
}

/// An error body with only a code and a message.
pub fn error(code: ErrorCode, message: &str) -> ErrorBody {
    ErrorBody::new(code, message)
}

/// The log line of 5.11 for a connection closed for a full outbound queue.
pub fn log_full(conn: ConnId, client: &str) -> String {
    let client = if client.is_empty() {
        "no hello"
    } else {
        client
    };
    format!("obsbot4linuxd: closed connection {conn} ({client}): its outbound queue was full")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_arrive_in_order() {
        let (out, rx) = Outbound::new(7, None, 4);
        assert!(out.send("a".into()));
        assert!(out.send("b".into()));
        assert_eq!(rx.try_recv().unwrap(), "a");
        assert_eq!(rx.try_recv().unwrap(), "b");
        assert_eq!(out.conn(), 7);
    }

    /// 5.1: a subscriber that does not read is disconnected when its queue is full; it never
    /// blocks the sender (a worker or the device table).
    #[test]
    fn a_full_queue_closes_the_connection_instead_of_blocking() {
        let (out, _rx) = Outbound::new(1, None, 2);
        out.set_client("widget 1.0");
        assert!(out.send("1".into()));
        assert!(out.send("2".into()));
        assert!(!out.send("3".into()), "full: refused at once");
        assert!(out.is_closed());
        assert!(!out.send("4".into()), "closed stays closed");
    }

    #[test]
    fn a_full_queue_shuts_the_socket_down() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let (out, _rx) = Outbound::new(1, Some(ours), 1);
        assert!(out.send("1".into()));
        assert!(!out.send("2".into()));
        let mut buf = [0u8; 1];
        use std::io::Read;
        assert_eq!(
            (&theirs).read(&mut buf).unwrap(),
            0,
            "the peer sees the end"
        );
    }

    #[test]
    fn the_log_line_names_the_connection_and_its_client() {
        assert_eq!(
            log_full(3, "widget 1.0"),
            "obsbot4linuxd: closed connection 3 (widget 1.0): its outbound queue was full"
        );
        assert_eq!(
            log_full(4, ""),
            "obsbot4linuxd: closed connection 4 (no hello): its outbound queue was full"
        );
    }

    /// The golden lines of `errors.jsonl` and `session.jsonl` come out byte for byte.
    #[test]
    fn reply_lines_are_canonical() {
        use obsbot_proto::{Empty, RevOk};
        assert_eq!(
            ok_line(
                &RequestId::Int(4),
                OkBody::Rev(RevOk {
                    rev: 6,
                    result: None
                })
            ),
            r#"{"id":4,"ok":{"rev":6}}"#
        );
        assert_eq!(
            ok_line(&RequestId::Int(5), OkBody::Empty(Empty {})),
            r#"{"id":5,"ok":{}}"#
        );
        let mut protocol = error(ErrorCode::Protocol, "the first message must be hello");
        protocol.supported = Some(vec![2]);
        assert_eq!(
            error_line(None, protocol),
            r#"{"id":null,"error":{"code":"protocol","message":"the first message must be hello","supported":[2]}}"#
        );
        assert_eq!(
            error_line(
                Some(&RequestId::Int(3)),
                error(ErrorCode::UnknownDevice, "no device sn:EXAMPLE0000009")
            ),
            r#"{"id":3,"error":{"code":"unknown_device","message":"no device sn:EXAMPLE0000009"}}"#
        );
    }

    #[test]
    fn a_dropped_receiver_counts_as_closed() {
        let (out, rx) = Outbound::new(1, None, 2);
        drop(rx);
        assert!(!out.send("1".into()));
        assert!(out.is_closed());
    }

    /// 4.1 rule 9: a closed connection's clones (held by queued and batched jobs) see the
    /// close at once; the lines queued before it still reach the writer.
    #[test]
    fn mark_closed_keeps_the_queued_lines_and_refuses_new_ones() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let (out, rx) = Outbound::new(1, Some(ours), 4);
        let job = out.clone();
        assert!(out.send("last".into()));
        out.mark_closed();
        assert!(job.is_closed(), "every clone sees it");
        assert!(!job.send("late".into()));
        assert_eq!(rx.try_recv().unwrap(), "last");
        assert!(rx.try_recv().is_err());
        theirs.set_nonblocking(true).unwrap();
        let mut buf = [0u8; 1];
        use std::io::Read;
        assert!(
            (&theirs).read(&mut buf).is_err(),
            "the socket is left open for the writer"
        );
    }
}
```

Create `crates/obsbot4linuxd/src/names.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Device entries, first states and log lines (4.2, 5.4, 5.11). Human-facing text shows a
//! serial or an `sn:` id only through `obsbot_proto::short_id`; no log line prints an
//! identity answer.

use crate::identity::PortReason;
use obsbot_device::contract::{Found, ModelEntry};
use obsbot_proto::{Description, Device, State, Support, short_id};
use serde_json::Value;

/// The `name` of a supported device (4.2): the display name plus the last four characters of
/// the serial for an `sn:` id, else plus the USB path.
pub fn name_of(display: &str, id: &str, usb_path: &str) -> String {
    match id.strip_prefix("sn:") {
        Some(serial) => {
            let chars: Vec<char> = serial.chars().collect();
            let tail: String = chars[chars.len().saturating_sub(4)..].iter().collect();
            format!("{display} {tail}")
        }
        None => format!("{display} {usb_path}"),
    }
}

/// The entry of a supported device. `serial` is a valid serial or `None`.
pub fn supported_device(
    found: &Found,
    model: &ModelEntry,
    description: Description,
    id: &str,
    serial: Option<&str>,
    firmware: Option<&str>,
) -> Device {
    Device {
        id: id.to_string(),
        name: name_of(model.display_name, id, &found.usb.path),
        model: model.key.to_string(),
        support: model.support.clone(),
        serial: serial.map(String::from),
        firmware: firmware.map(String::from),
        video_node: found.video_node.clone(),
        available: true,
        replaced_by: None,
        usb: found.usb.clone(),
        description,
    }
}

/// The entry of an unsupported device (5.4): `port:` id, model `unknown`, no serial, no
/// firmware, empty settings, actions and limits; its name is the USB product string plus the
/// path, or `USB <vendor>:<product>` plus the path without a product string (4.2).
pub fn unsupported_device(found: &Found) -> Device {
    let usb = &found.usb;
    let name = match &usb.product_name {
        Some(product) => format!("{product} {}", usb.path),
        None => format!("USB {}:{} {}", usb.vendor, usb.product, usb.path),
    };
    Device {
        id: obsbot_proto::port_id(&usb.path),
        name,
        model: "unknown".into(),
        support: Support::Unsupported,
        serial: None,
        firmware: None,
        video_node: found.video_node.clone(),
        available: true,
        replaced_by: None,
        usb: usb.clone(),
        description: Description::default(),
    }
}

/// The state of a device before its first read: every leaf `null`, every leaf with a readback
/// in `unread` (4.2 "Absent vs null"); a `readback: false` leaf is `null` until written and is
/// never in `unread`, since it is never read.
pub fn initial_state(desc: &Description) -> State {
    let mut state = State::new(true);
    for (path, setting) in &desc.settings {
        state.set(path, Value::Null);
        if setting.readback {
            state.unread.push(path.clone());
        }
    }
    state
}

fn support_text(support: &Support) -> String {
    serde_json::to_value(support)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

/// 5.11: one line for every device that appears: id, model, USB ids, USB path, support level,
/// and for a `port:` id of a supported model the reason.
pub fn log_appeared(device: &Device, reason: Option<PortReason>) -> String {
    let mut line = format!(
        "obsbot4linuxd: device {} appeared: model {}, USB {}:{}, path {}, support {}",
        short_id(&device.id),
        device.model,
        device.usb.vendor,
        device.usb.product,
        device.usb.path,
        support_text(&device.support)
    );
    if let Some(r) = reason {
        line.push_str(&format!(", port id: {}", r.text()));
    }
    line
}

/// 5.11: one line for a hand-over, with the old and the new id.
pub fn log_handover(old: &str, new: &str) -> String {
    format!(
        "obsbot4linuxd: device {} is now {}",
        short_id(old),
        short_id(new)
    )
}

/// 5.11: one line for a device that goes.
pub fn log_gone(id: &str) -> String {
    format!("obsbot4linuxd: device {} went away", short_id(id))
}

/// A device whose camera could not be opened (it stays listed, every leaf unread).
pub fn log_open_failed(id: &str, error: &str) -> String {
    format!(
        "obsbot4linuxd: device {} could not be opened: {error}",
        short_id(id)
    )
}

/// 4.2: a camera value the model cannot decode, logged once per device and value.
pub fn log_raw(id: &str, leaf: &str, value: &str) -> String {
    format!(
        "obsbot4linuxd: device {} reported {leaf} = {value}, which its model cannot decode",
        short_id(id)
    )
}

/// 8.3: a frame outside the model's send table was refused before it was sent
/// (`CameraError::NotInSendTable`); the request is answered `internal`.
pub fn log_refused_frame(id: &str, message: &str) -> String {
    format!(
        "obsbot4linuxd: device {}: a frame outside the send table was refused and not sent: {message}",
        short_id(id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::models::fake;
    use obsbot_proto::Setting;
    use serde_json::json;

    fn scan() -> Vec<Found> {
        use obsbot_device::contract::Bus;
        fake::FakeBus::new(&fake::KEYS).unwrap().scan()
    }

    #[test]
    fn names_follow_4_2() {
        assert_eq!(
            name_of("OBSBOT Tiny 2 Lite", "sn:EXAMPLE0000001", "3-2.1"),
            "OBSBOT Tiny 2 Lite 0001"
        );
        assert_eq!(name_of("Fake A", "port:9-1", "9-1"), "Fake A 9-1");
        let mut unknown = scan().remove(2);
        assert_eq!(unsupported_device(&unknown).name, "Unknown Camera 9-3");
        unknown.usb.product_name = None;
        assert_eq!(unsupported_device(&unknown).name, "USB 3564:fa00 9-3");
    }

    /// 5.4: the unsupported entry holds no serial, no firmware and nothing to set or do.
    #[test]
    fn the_unsupported_entry_is_empty() {
        let d = unsupported_device(&scan()[2]);
        assert_eq!(d.id, "port:9-3");
        assert_eq!(d.model, "unknown");
        assert_eq!(d.support, Support::Unsupported);
        assert_eq!((d.serial, d.firmware), (None, None));
        assert!(d.description.settings.is_empty() && d.description.actions.is_empty());
        assert!(d.available);
        assert_eq!(d.video_node.as_deref(), Some("/dev/video106"));
    }

    #[test]
    fn a_supported_entry_takes_the_model_and_the_answers() {
        let found = scan().remove(0);
        let model = found.model.unwrap();
        let d = supported_device(
            &found,
            model,
            fake::description("fake-a").unwrap(),
            "sn:FAKEA0000001",
            Some("FAKEA0000001"),
            Some("1.0.0.0"),
        );
        assert_eq!(d.name, "Fake A 0001");
        assert_eq!(d.model, "fake-a");
        assert_eq!(d.support, Support::Tested);
        assert_eq!(d.usb.path, "9-1");
        assert!(d.description.settings.contains_key("led.brightness"));
    }

    #[test]
    fn the_first_state_is_all_null_and_unread_except_readback_false() {
        let desc = Description::default()
            .setting("asleep", Setting::boolean("Asleep").readonly())
            .setting("led.blink", Setting::boolean("Blink").no_readback())
            .setting("zoom.factor", Setting::number("Zoom", 1.0, 4.0));
        let s = initial_state(&desc);
        assert_eq!(s.at("led.blink"), Some(&Value::Null));
        assert_eq!(s.at("zoom.factor"), Some(&Value::Null));
        assert_eq!(s.unread, ["asleep", "zoom.factor"]);
        assert!(s.pending.is_empty() && s.available);
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            json!({"available": true, "pending": {}, "unread": ["asleep", "zoom.factor"],
                   "asleep": null, "led": {"blink": null}, "zoom": {"factor": null}})
        );
    }

    /// 4.2, 5.11: logs show `…` and the last four characters, never a whole serial.
    #[test]
    fn log_lines_never_show_a_whole_serial() {
        let found = scan().remove(0);
        let d = supported_device(
            &found,
            found.model.unwrap(),
            Description::default(),
            "sn:FAKEA0000001",
            Some("FAKEA0000001"),
            None,
        );
        let lines = [
            log_appeared(&d, None),
            log_handover("port:9-1", "sn:FAKEA0000001"),
            log_gone("sn:FAKEA0000001"),
            log_open_failed("sn:FAKEA0000001", "gone"),
            log_raw("sn:FAKEA0000001", "tracking.mode", "raw:9"),
            log_refused_frame("sn:FAKEA0000001", "an example frame"),
        ];
        for line in &lines {
            assert!(!line.contains("FAKEA0000001"), "{line}");
        }
        assert_eq!(
            lines[0],
            "obsbot4linuxd: device …0001 appeared: model fake-a, USB 3564:fa0a, path 9-1, support tested"
        );
        assert_eq!(lines[1], "obsbot4linuxd: device port:9-1 is now …0001");
        assert_eq!(lines[2], "obsbot4linuxd: device …0001 went away");
        assert_eq!(
            lines[4],
            "obsbot4linuxd: device …0001 reported tracking.mode = raw:9, which its model cannot decode"
        );
        assert_eq!(
            lines[5],
            "obsbot4linuxd: device …0001: a frame outside the send table was refused and not sent: an example frame"
        );
        let port = Device {
            id: "port:9-1".into(),
            ..d
        };
        assert_eq!(
            log_appeared(&port, Some(PortReason::NoSerial)),
            "obsbot4linuxd: device port:9-1 appeared: model fake-a, USB 3564:fa0a, path 9-1, support tested, port id: no serial answered"
        );
        let unknown = unsupported_device(&scan()[2]);
        assert_eq!(
            log_appeared(&unknown, None),
            "obsbot4linuxd: device port:9-3 appeared: model unknown, USB 3564:fa00, path 9-3, support unsupported"
        );
    }
}
```

Create `crates/obsbot4linuxd/src/hotplug.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The difference between two scans (5.3). An enumeration is identified by the USB path
//! together with the device number: when the device number under a path changes between two
//! scans, the old enumeration went away and a new one came, even if no scan saw the path
//! missing.

use obsbot_device::contract::Found;

/// One enumeration the registry knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub path: String,
    pub devnum: u32,
}

impl Seen {
    pub fn of(found: &Found) -> Seen {
        Seen {
            path: found.usb.path.clone(),
            devnum: found.devnum,
        }
    }
}

/// The result of comparing a scan with the known enumerations: indexes into `known` of the
/// enumerations that went away, and indexes into `found` of the new ones, in scan order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    pub gone: Vec<usize>,
    pub new: Vec<usize>,
}

pub fn diff(known: &[Seen], found: &[Found]) -> Diff {
    let now: Vec<Seen> = found.iter().map(Seen::of).collect();
    Diff {
        gone: (0..known.len())
            .filter(|i| !now.contains(&known[*i]))
            .collect(),
        new: (0..now.len())
            .filter(|i| !known.contains(&now[*i]))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::contract::Bus;
    use obsbot_device::models::fake::{FakeBus, KEYS};

    #[test]
    fn a_first_scan_finds_everything_new() {
        let found = FakeBus::new(&KEYS).unwrap().scan();
        assert_eq!(
            diff(&[], &found),
            Diff {
                gone: vec![],
                new: vec![0, 1, 2]
            }
        );
        let known: Vec<Seen> = found.iter().map(Seen::of).collect();
        assert_eq!(
            diff(&known, &found),
            Diff {
                gone: vec![],
                new: vec![]
            },
            "an unchanged scan changes nothing"
        );
    }

    #[test]
    fn a_missing_path_is_gone() {
        let bus = FakeBus::new(&KEYS).unwrap();
        let known: Vec<Seen> = bus.scan().iter().map(Seen::of).collect();
        bus.unplug("sn:FAKEB0000001").unwrap();
        assert_eq!(
            diff(&known, &bus.scan()),
            Diff {
                gone: vec![1],
                new: vec![]
            }
        );
    }

    /// 5.3: a device-number change under the same path is an unplug followed by a plug-in.
    #[test]
    fn a_new_device_number_under_a_known_path_is_gone_and_new() {
        let bus = FakeBus::new(&KEYS).unwrap();
        let known: Vec<Seen> = bus.scan().iter().map(Seen::of).collect();
        bus.unplug("sn:FAKEA0000001").unwrap();
        bus.plug("fake-a").unwrap();
        let found = bus.scan();
        let d = diff(&known, &found);
        assert_eq!(d.gone, vec![0]);
        assert_eq!(d.new.len(), 1);
        assert_eq!(found[d.new[0]].usb.path, known[0].path, "same path");
        assert_ne!(found[d.new[0]].devnum, known[0].devnum);
    }
}
```

Create `crates/obsbot4linuxd/src/daemon.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Start-up, the daemon's threads and shutdown (5.1, 5.2, 5.9), and the panic policy: a panic
//! in a worker, the registry thread, the acceptor or the sweeper ends the daemon with an error
//! (`run` returns it, `main` exits non-zero and systemd restarts the service); a panic in a
//! connection thread closes only that connection (`conn`).

use crate::{Config, Timing};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

/// The first panic of a thread the daemon cannot run without.
#[derive(Default)]
pub struct Fatal(Mutex<Option<String>>);

impl Fatal {
    pub fn new() -> Arc<Fatal> {
        Arc::new(Fatal::default())
    }

    /// Records that the thread `name` panicked; the first record wins.
    pub fn set(&self, name: &str) {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some(name.to_string());
        }
    }

    pub fn get(&self) -> Option<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Spawns a thread the daemon cannot run without: a panic in it is recorded in `fatal`.
pub fn critical<F>(name: String, fatal: Arc<Fatal>, f: F) -> io::Result<JoinHandle<()>>
where
    F: FnOnce() + Send + 'static,
{
    std::thread::Builder::new()
        .name(name.clone())
        .spawn(move || {
            if catch_unwind(AssertUnwindSafe(f)).is_err() {
                fatal.set(&name);
            }
        })
}

/// The daemon: see `crate::run`.
pub fn run(config: Config, timing: Timing) -> io::Result<()> {
    let _ = (config, timing);
    todo!("task b-10-daemon")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 5.9: a panic in a critical thread is recorded, so that `run` ends with an error.
    #[test]
    fn a_panic_in_a_critical_thread_is_fatal() {
        let fatal = Fatal::new();
        let h = critical("worker 9-1".into(), fatal.clone(), || {
            panic!("a daemon bug")
        })
        .unwrap();
        h.join().unwrap();
        assert_eq!(fatal.get().as_deref(), Some("worker 9-1"));
        critical("registry".into(), fatal.clone(), || panic!("later"))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            fatal.get().as_deref(),
            Some("worker 9-1"),
            "the first one is reported"
        );
        let calm = Fatal::new();
        critical("acceptor".into(), calm.clone(), || ())
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(calm.get(), None);
    }
}
```

Create `crates/obsbot4linuxd/src/worker.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The worker of one supported device (5.1, 5.2, 5.5 to 5.7): the only code that touches the
//! device. Written in task b-07-worker.

use crate::Timing;
use crate::daemon::Fatal;
use crate::queue::DeviceQueue;
use crate::table::{PlugId, Table};
use obsbot_device::contract::{Bus, Found, ModelEntry};
use std::io;
use std::sync::Arc;
use std::thread::JoinHandle;

/// Everything a worker gets from the registry thread when its device is plugged in.
pub struct Plugged {
    pub plug: PlugId,
    pub found: Found,
    pub model: &'static ModelEntry,
    pub bus: Arc<dyn Bus>,
    pub table: Arc<Table>,
    pub queue: Arc<DeviceQueue>,
    pub timing: Timing,
}

/// Starts the worker thread `worker <usb path>`; a panic in it is fatal (5.9).
pub fn spawn(p: Plugged, fatal: Arc<Fatal>) -> io::Result<JoinHandle<()>> {
    let name = format!("worker {}", p.found.usb.path);
    crate::daemon::critical(name, fatal, move || run(p))
}

/// The worker's whole life: open, first identity attempt and first read, announce, then the
/// loop of batches and ticks until the device becomes unavailable or the daemon stops.
pub fn run(p: Plugged) {
    let _ = p;
    todo!("task b-07-worker")
}
```

Create `crates/obsbot4linuxd/src/identity.rs` (stub of b-04; `PortReason` and `choose_id` complete, because the table uses them):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The identity series of one plug-in (5.5) as a state machine, and the choice of the id.
//!
//! - The first attempt is sent at plug-in, asleep or awake. Attempts 2 and 3 ride the next two
//!   ticks of the tier the model names (`ModelEntry::identity_tier`) while the camera is awake.
//! - If the first attempt gave no valid serial and the first read of the group holding `asleep`
//!   reported the camera asleep, the first series ends there; one further series of three
//!   attempts runs from the camera's first wake: at the wake, then with the next two ticks.
//! - A series ends at the first attempt after which serial and firmware are both answered; an
//!   attempt asks only for what is missing. A model with neither query sends no attempt.
//!
//! No more than one attempt is ever sent to a sleeping camera.

use obsbot_device::contract::{Identity, IdentitySource, Need};
use obsbot_proto::{is_valid_serial, port_id, sn_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Before the plug-in attempt.
    Plug,
    /// Attempt 1 was sent; waiting for the first status read.
    First,
    /// `left` attempts ride the next ticks of the identity tier.
    Ticks {
        left: u8,
    },
    /// The first series ended asleep; the wake series starts at the first wake.
    AwaitWake,
    Done,
}

/// Why a supported device got a `port:` id (5.11 log line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortReason {
    NoSerial,
    InvalidSerial,
    Duplicate,
}

impl PortReason {
    pub fn text(self) -> &'static str {
        match self {
            PortReason::NoSerial => "no serial answered",
            PortReason::InvalidSerial => "an invalid serial",
            PortReason::Duplicate => "a duplicate serial",
        }
    }
}

/// The identity series of one plug-in, and what it has learnt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    source: IdentitySource,
    firmware_query: bool,
    phase: Phase,
    woke: bool,
    /// A valid serial, once answered. Never logged (4.2).
    pub serial: Option<String>,
    /// The camera answered a serial that is not valid (and no valid one yet).
    pub invalid: bool,
    pub firmware: Option<String>,
}

impl Series {
    pub fn new(source: IdentitySource, firmware_query: bool) -> Series {
        todo!("b-04-identity")
    }

    fn need(&self, plug_in: bool) -> Need {
        todo!("b-04-identity")
    }

    fn complete(&self) -> bool {
        todo!("b-04-identity")
    }

    /// Attempt 1, at plug-in; `None` when the model has nothing to ask.
    pub fn at_plug_in(&mut self) -> Option<Need> {
        todo!("b-04-identity")
    }

    /// The outcome of an attempt: `None` when it failed. Answers are kept only once given.
    pub fn answered(&mut self, got: Option<&Identity>) {
        todo!("b-04-identity")
    }

    /// The first read of the group holding `asleep` (5.2 step 3) reported `asleep`; `false`
    /// as well for a model without an `asleep` leaf.
    pub fn after_first_status(&mut self, asleep: bool) {
        todo!("b-04-identity")
    }

    /// A tick of the identity tier: the attempt that rides it, if any (only while awake).
    pub fn at_tick(&mut self, awake: bool) -> Option<Need> {
        todo!("b-04-identity")
    }

    /// `asleep` turned false: the first attempt of the wake series, if one is due.
    pub fn at_wake(&mut self) -> Option<Need> {
        todo!("b-04-identity")
    }

    /// No further attempt will be sent for this plug-in.
    pub fn done(&self) -> bool {
        todo!("b-04-identity")
    }

    /// Why the id is a `port:` id when no valid serial is known.
    pub fn reason(&self) -> PortReason {
        todo!("b-04-identity")
    }
}

/// The id of a supported device (5.5): `sn:<serial>` for a valid serial that no available
/// device holds, else `port:<usb path>` with the reason.
pub fn choose_id(
    serial: Option<&str>,
    reason: PortReason,
    usb_path: &str,
    held_by_an_available_device: impl Fn(&str) -> bool,
) -> (String, Option<PortReason>) {
    match serial {
        Some(s) => {
            let id = sn_id(s);
            if held_by_an_available_device(&id) {
                (port_id(usb_path), Some(PortReason::Duplicate))
            } else {
                (id, None)
            }
        }
        None => (port_id(usb_path), Some(reason)),
    }
}
```

Create `crates/obsbot4linuxd/src/queue.rs` (stub of b-05; the types, `DeviceQueue::new` and the `Job` methods complete, because the table and the worker stub use them):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The queue of one supported device (4.1 rules 2 and 9, 5.6): at most `MAX_QUEUED` requests,
//! FIFO, except that a stop is taken before every queued request. Connection readers push,
//! the device's worker takes batches, the deadline sweeper removes requests not started within
//! `REQUEST_DEADLINE`. The queue also holds the continuing-motion record (5.6) and the flags
//! that cut a camera operation short.

use crate::MAX_QUEUED;
use crate::outbound::{ConnId, Outbound, error, error_line};
use obsbot_device::contract::Motion;
use obsbot_proto::{ErrorCode, Params, Patch, RequestId};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// What a device request asks.
#[derive(Debug, Clone, PartialEq)]
pub enum Work {
    /// `paths: None` for the full state.
    Get(Option<Vec<String>>),
    Set(Patch),
    Do {
        action: String,
        params: Params,
    },
}

/// One queued device request.
#[derive(Clone)]
pub struct Job {
    pub id: RequestId,
    /// Where the reply goes; `None` for the daemon's own stop of a continuing action.
    pub out: Option<Outbound>,
    /// The connection that sent it; 0 for the daemon's own stop.
    pub conn: ConnId,
    /// The device id the request named.
    pub device: String,
    pub work: Work,
    pub arrived: Instant,
    /// A stop action (`Motion::Stop`): taken before every queued request.
    pub stop: bool,
}

impl Job {
    /// The daemon's own stop of a continuing action whose connection closed (5.6).
    pub fn own_stop(action: &str, now: Instant) -> Job {
        Job {
            id: RequestId::Int(0),
            out: None,
            conn: 0,
            device: String::new(),
            work: Work::Do {
                action: action.to_string(),
                params: Params::new(),
            },
            arrived: now,
            stop: true,
        }
    }

    /// Sends a reply line to the job's connection, if it has one.
    pub fn answer(&self, line: String) {
        if let Some(out) = &self.out {
            out.send(line);
        }
    }

    /// Answers the job with an error.
    pub fn fail(&self, code: ErrorCode, message: &str) {
        self.answer(error_line(Some(&self.id), error(code, message)));
    }
}

/// Which connection's continuing action runs on a device (5.6), and the stop that ends it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    turn: Option<(ConnId, String)>,
}

impl Record {
    /// An action ran (the camera accepted it). A continuing action without `keep: true`
    /// records its connection; with `keep: true` nothing is recorded, since closing the
    /// connection must not stop it. Any other motion action clears the record; a stop clears it
    /// only when it is the stop the record names.
    pub fn after_act(&mut self, action: &str, motion: Motion, conn: ConnId, keep: bool) {
        todo!("b-05-queue")
    }

    /// The connection `conn` closed: the stop to queue for it, if its continuing action is the
    /// recorded one. The record is cleared.
    pub fn on_close(&mut self, conn: ConnId) -> Option<String> {
        todo!("b-05-queue")
    }
}

/// Why the queue did not take a job.
pub enum Refused {
    /// `MAX_QUEUED` requests wait already: answer `busy`.
    Busy(Box<Job>),
    /// The device became unavailable and its worker ended: answer as for an unavailable device.
    Closed(Box<Job>),
}

struct Inner {
    jobs: VecDeque<Job>,
    closed: bool,
    stops: Vec<String>,
    record: Record,
}

/// The queue of one device, shared by the readers, the worker and the sweeper.
pub struct DeviceQueue {
    inner: Mutex<Inner>,
    cv: Condvar,
    /// Cuts a group read at the next frame boundary: a stop arrived, the device went, or the
    /// daemon stops (5.6 "Stops").
    pub cut: Arc<AtomicBool>,
    /// Cuts an identity attempt or a poll: the device went or the daemon stops.
    pub halt: Arc<AtomicBool>,
    /// Cuts a started request: the device went (5.6 "Unavailable").
    pub ended: Arc<AtomicBool>,
    shutdown: AtomicBool,
}

impl DeviceQueue {
    pub fn new() -> Arc<DeviceQueue> {
        Arc::new(DeviceQueue {
            inner: Mutex::new(Inner {
                jobs: VecDeque::new(),
                closed: false,
                stops: Vec::new(),
                record: Record::default(),
            }),
            cv: Condvar::new(),
            cut: Arc::new(AtomicBool::new(false)),
            halt: Arc::new(AtomicBool::new(false)),
            ended: Arc::new(AtomicBool::new(false)),
            shutdown: AtomicBool::new(false),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The stop actions of the device's model (`Motion::Stop`), set by the worker once it has
    /// opened the camera, before the device is announced.
    pub fn set_stops(&self, stops: Vec<String>) {
        todo!("b-05-queue")
    }

    pub fn is_stop(&self, action: &str) -> bool {
        todo!("b-05-queue")
    }

    /// Queues a job. A stop goes before every queued request that is not a stop, and sets
    /// `cut` so that a group read in progress ends at its next frame boundary.
    pub fn push(&self, job: Job) -> Result<(), Refused> {
        todo!("b-05-queue")
    }

    /// Takes every queued job; the worker plans its batch from them and puts the rest back.
    pub fn take_all(&self) -> Vec<Job> {
        todo!("b-05-queue")
    }

    /// Puts jobs back at the front, in their order (the requests after a failed one, 4.1 rule
    /// 9, or the rest of a batch plan). A stop pushed meanwhile stays in front of them.
    pub fn put_back(&self, jobs: Vec<Job>) {
        todo!("b-05-queue")
    }

    pub fn len(&self) -> usize {
        todo!("b-05-queue")
    }

    /// The connection `conn` closed: its queued requests are dropped (4.1 rule 9), and if its
    /// continuing action is recorded, the model's stop is queued for it (5.6). Returns the stop.
    pub fn close_conn(&self, conn: ConnId, now: Instant) -> Option<String> {
        todo!("b-05-queue")
    }

    /// Updates the continuing-motion record after the camera accepted an action.
    pub fn record_act(&self, action: &str, motion: Motion, conn: ConnId, keep: bool) {
        todo!("b-05-queue")
    }

    /// Removes the jobs not started within `deadline` of their arrival (4.1 rule 9).
    pub fn expire(&self, now: Instant, deadline: Duration) -> Vec<Job> {
        todo!("b-05-queue")
    }

    /// The device became unavailable: every camera operation is cut and the worker ends.
    pub fn end(&self) {
        todo!("b-05-queue")
    }

    /// The daemon stops: polls and identity attempts are cut; the worker sends the stops still
    /// queued and ends (5.9).
    pub fn shutdown(&self) {
        todo!("b-05-queue")
    }

    pub fn is_ended(&self) -> bool {
        todo!("b-05-queue")
    }

    pub fn is_shutdown(&self) -> bool {
        todo!("b-05-queue")
    }

    /// The worker is about to end: no job is taken any more, and the queued ones are returned
    /// to be answered as for an unavailable device.
    pub fn close(&self) -> Vec<Job> {
        todo!("b-05-queue")
    }

    /// Waits until a job is queued, the device ends or the daemon stops, or `until` passes.
    pub fn wait(&self, until: Instant) {
        todo!("b-05-queue")
    }
}

/// Answers the requests the sweeper removed: `camera`, not started within the deadline.
pub fn answer_late(jobs: Vec<Job>) {
    todo!("b-05-queue")
}

/// How the worker takes a batch (4.1 rule 9, 5.6): `take` jobs from the front of the queue;
/// `coalesced[i]` marks a single-leaf `set` that a later `set` of the same connection in the
/// batch, with the same single leaf, makes unnecessary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub take: usize,
    pub coalesced: Vec<bool>,
}

/// Plans one batch. A stop runs alone; a request with a `restart` warning runs alone and ends
/// the batch before it; otherwise the batch runs up to the first such request. `restarts`
/// tells a request with a `restart` warning; `single_leaf` gives the one leaf of a `set`
/// whose patch has exactly one leaf with a value the setting can take, else `None`.
pub fn plan(
    jobs: &[Job],
    restarts: impl Fn(&Job) -> bool,
    single_leaf: impl Fn(&Patch) -> Option<String>,
) -> Plan {
    todo!("b-05-queue")
}
```

Create `crates/obsbot4linuxd/src/table.rs` (stub of b-06):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The device table (5.1): every entry's id, description, rev and last state, and the
//! connections that receive events, under one lock. Every change of an entry goes through
//! here, so the events of 4.1 leave in one order for every connection, and a `hello` snapshot
//! and the start of its event delivery are atomic (4.1 `hello` timing).
//!
//! An enumeration of a supported device is a plug (`PlugId`); its worker addresses the table
//! by it. A plug's entry stays hidden until the worker announces it after its first read
//! attempt (5.2), or until a `hello` reaches its deadline (4.1).

use crate::identity::{PortReason, choose_id};
use crate::names;
use crate::outbound::{ConnId, Outbound, ok_line};
use crate::outbound::{error, error_line};
use crate::queue::{DeviceQueue, Job, Work};
use obsbot_device::contract::ModelEntry;
use obsbot_proto::ErrorCode;
use obsbot_proto::validate::validate_get;
use obsbot_proto::values::same;
use obsbot_proto::{
    Description, Device, Event, Hello, HelloOk, OkBody, RequestId, RevState, State,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// One enumeration of a device, numbered by the registry from 1.
pub type PlugId = u64;

/// Who changed a state (4.1 rule 8): a request (its batch's re-read), or a periodic read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Request,
    Poll,
}

/// Where a reader sends a device request (4.1 rule 2).
pub enum Routed {
    /// No listed device has this id.
    Unknown,
    /// An unsupported device: `get` from its fixed state, `set` and `do` refused.
    Unsupported(RevState, Description),
    /// An unplugged `sn:` device: `get` from its published state, `set` and `do` refused.
    Unavailable(RevState, Description),
    /// A supported, available device: its queue.
    Worker(Arc<DeviceQueue>),
}

impl Routed {
    /// Answers a request the reader (or a worker that ended) answers at once (4.1 rule 2):
    /// `unknown_device`; for an unsupported or unplugged device a `get` from its state (after
    /// `validate_get`), a `set` or `do` with `unsupported_device` or `unavailable`.
    pub fn answer(&self, job: &Job) {
        todo!("b-06-table")
    }
}

enum Route {
    Unsupported,
    Worker(Arc<DeviceQueue>),
    Unplugged,
}

struct Entry {
    device: Device,
    rev: u64,
    state: State,
    announced: bool,
    plug: Option<PlugId>,
    model: Option<&'static ModelEntry>,
    route: Route,
}

struct Sub {
    out: Outbound,
    subscribe: bool,
    live: BTreeSet<String>,
}

struct Inner {
    entries: Vec<Entry>,
    subs: BTreeMap<ConnId, Sub>,
    startup: BTreeSet<PlugId>,
}

pub struct Table {
    inner: Mutex<Inner>,
    startup_done: Condvar,
    daemon: String,
}

/// The live topics of a state change that touches only volatile leaves (4.1 rule 8);
/// `None` when it touches anything else (a leaf without a live topic, `available`, `pending`
/// or `unread`).
pub fn volatile_topics(desc: &Description, old: &State, new: &State) -> Option<BTreeSet<String>> {
    todo!("b-06-table")
}

fn same_state(a: &State, b: &State) -> bool {
    todo!("b-06-table")
}

fn line(event: &Event) -> String {
    todo!("b-06-table")
}

impl Inner {
    fn listed(&self) -> Vec<Device> {
        todo!("b-06-table")
    }

    fn by_plug(&self, plug: PlugId) -> Option<usize> {
        todo!("b-06-table")
    }

    fn by_id(&self, id: &str) -> Option<usize> {
        todo!("b-06-table")
    }

    /// Sends one line to the subscribed connections; `only` limits it to connections whose
    /// live list holds one of these topics.
    fn send(&self, text: &str, only: Option<&BTreeSet<String>>) {
        todo!("b-06-table")
    }

    fn send_devices(&self) {
        todo!("b-06-table")
    }

    fn send_state(&self, i: usize, only: Option<&BTreeSet<String>>) {
        todo!("b-06-table")
    }

    /// Lists a hidden entry: `devices`, then its first `state` (4.1 Events).
    fn list(&mut self, i: usize) {
        todo!("b-06-table")
    }

    fn held(&self, id: &str, except: usize) -> bool {
        todo!("b-06-table")
    }

    /// The hand-over of 5.5 from the listed `port:` entry `i` to `sn_id`: three events in a
    /// row, the rev above both entries, `pending` and `unread` carried over.
    fn hand_over(&mut self, i: usize, sn_id: &str, serial: &str) {
        todo!("b-06-table")
    }
}

impl Table {
    /// An empty table; `daemon` is the version the `hello` reply names.
    pub fn new(daemon: &str) -> Table {
        todo!("b-06-table")
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        todo!("b-06-table")
    }

    /// The plugs of the first enumeration whose first read attempt every `hello` waits for.
    pub fn startup(&self, plugs: &[PlugId]) {
        todo!("b-06-table")
    }

    /// Answers a `hello` that arrived at `arrived` (4.1 `hello` timing): waits until the first
    /// read attempt of every device of the first enumeration has ended, at most until
    /// `arrived + deadline`; a device still in its first read is then listed with its leaves
    /// `null` and in `unread`. The reply and the connection's registration for events happen
    /// under one lock. Returns `false` when the reply could not be queued.
    pub fn hello(
        &self,
        out: &Outbound,
        id: &RequestId,
        hello: &Hello,
        arrived: Instant,
        deadline: std::time::Duration,
    ) -> bool {
        todo!("b-06-table")
    }

    /// The `live` verb: replaces the connection's live list.
    pub fn set_live(&self, conn: ConnId, topics: &[String]) {
        todo!("b-06-table")
    }

    /// The connection closed.
    pub fn remove_conn(&self, conn: ConnId) {
        todo!("b-06-table")
    }

    /// The live topics at least one subscribed connection holds (5.7 live tiers).
    pub fn topics(&self) -> BTreeSet<String> {
        todo!("b-06-table")
    }

    /// Where a request for `id` goes (4.1 rule 2).
    pub fn route(&self, id: &str) -> Routed {
        todo!("b-06-table")
    }

    /// An unsupported device appeared (5.4): listed at once with its fixed state, rev 1.
    pub fn plug_unsupported(&self, plug: PlugId, device: Device) {
        todo!("b-06-table")
    }

    /// A supported device appeared: a hidden entry under its `port:` id until its worker
    /// announces it.
    pub fn plug_hidden(
        &self,
        plug: PlugId,
        model: &'static ModelEntry,
        device: Device,
        queue: Arc<DeviceQueue>,
    ) {
        todo!("b-06-table")
    }

    /// The worker opened the camera: the hidden entry takes the model's description, and its
    /// state every described leaf, `null` and in `unread` (what a `hello` at its deadline
    /// shows, 4.1).
    pub fn describe(&self, plug: PlugId, description: Description) {
        todo!("b-06-table")
    }

    /// The worker's first read attempt ended (5.2): the entry is listed under `sn:<serial>`
    /// (taking an unplugged entry with that id, 5.3) or under its `port:` id. An entry a
    /// `hello` already listed at its deadline is handed over instead. Returns the id.
    pub fn announce(
        &self,
        plug: PlugId,
        serial: Option<&str>,
        reason: PortReason,
        firmware: Option<&str>,
        state: State,
    ) -> Option<String> {
        todo!("b-06-table")
    }

    /// A later identity attempt answered (5.5): a valid serial that no available device holds
    /// hands a `port:` entry over to `sn:`; a firmware answered for the first time changes the
    /// entry (a `devices` event). Returns the new id after a hand-over.
    pub fn identity(
        &self,
        plug: PlugId,
        serial: Option<&str>,
        firmware: Option<&str>,
    ) -> Option<String> {
        todo!("b-06-table")
    }

    /// Publishes a worker's view of its device: values, `pending` and `unread`. A change gets
    /// a new rev and a `state` event; a change caused by no request that touches only volatile
    /// leaves goes only to the connections that subscribe one of their live topics (4.1 rule
    /// 8). An unchanged state keeps its rev and sends nothing. Returns the rev that reflects
    /// the state. A plug that no longer serves its entry (unplugged) changes nothing.
    pub fn publish(&self, plug: PlugId, state: &State, cause: Cause) -> u64 {
        todo!("b-06-table")
    }

    /// The rev and the published state of a plug's entry.
    pub fn current(&self, plug: PlugId) -> Option<(u64, State)> {
        todo!("b-06-table")
    }

    /// The id a plug's entry is listed under now.
    pub fn id_of(&self, plug: PlugId) -> Option<String> {
        todo!("b-06-table")
    }

    /// The enumeration went away (5.3): an unsupported or `port:` entry is removed (one
    /// `devices` event); an `sn:` entry stays listed with `available: false`, no video node,
    /// `pending` emptied and its `readback: false` leaves `null` (4.2), then one `state`
    /// event. A hidden entry goes without an event.
    pub fn unplug(&self, plug: PlugId) {
        todo!("b-06-table")
    }

    /// The queues of every available supported device: for the deadline sweeper, and for a
    /// closing connection (its queued requests are dropped, 4.1 rule 9; its continuing action
    /// stopped, 5.6).
    pub fn queues(&self) -> Vec<Arc<DeviceQueue>> {
        todo!("b-06-table")
    }

    /// The listed devices, for tests and logs.
    pub fn devices(&self) -> Vec<Device> {
        todo!("b-06-table")
    }
}
```

Create the nine files that later tasks write, each with its licence line and one doc line:

Create `crates/obsbot4linuxd/src/lines.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The bounded line reader of a connection (4.1). Written in task b-02-lines-socket.
```

Create `crates/obsbot4linuxd/src/socket.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The single-instance lock and the listening socket (5.9, 5.10, 11). Written in task b-02-lines-socket.
```

Create `crates/obsbot4linuxd/src/args.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line of `obsbot4linuxd` (5.12). Written in task b-02-lines-socket.
```

Create `crates/obsbot4linuxd/src/pending.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `pending` and the echo of written values (4.1 rule 4, 4.2, 5.6, 5.7). Written in task b-03-pending-schedule.
```

Create `crates/obsbot4linuxd/src/schedule.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The poll schedule of one device (5.7). Written in task b-03-pending-schedule.
```

Create `crates/obsbot4linuxd/src/batch.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The write path of a worker (4.1 rules 3 to 6 and 9, 5.6). Written in task b-07-worker.
```

Create `crates/obsbot4linuxd/src/conn.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The acceptor and the connections (4.1, 5.1). Written in task b-08-conn.
```

Create `crates/obsbot4linuxd/src/registry.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The registry thread (5.3). Written in task b-09-registry.
```

Create `crates/obsbot4linuxd/src/control.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The test-control socket of fake mode (5.12). Written in task b-10-daemon.
```

Replace `crates/obsbot4linuxd/src/main.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linuxd`: the daemon binary (design spec 5). Written in task b-10-daemon.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("obsbot4linuxd: not built yet (track B of the 1.0 rebuild)");
    ExitCode::FAILURE
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4`
Expected: PASS: `test result: ok. 18 passed` (unit tests of `lib`, `outbound`, `names`, `hotplug`, `daemon`) and `test result: ok. 1 passed` (`tests/harness.rs`).

Run: `cargo fmt --all --check && nice -n 19 cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add Cargo.lock crates/obsbot4linuxd
git commit -m "obsbot4linuxd: skeleton with Timing, outbound queue, names, scan diff and test harness"
```

### Task b-02 (key `b-02-lines-socket`): Bounded line reader, lock and socket, command line

**Files:**

- Replace: `crates/obsbot4linuxd/src/lines.rs`, `src/socket.rs`, `src/args.rs`
- Test: unit tests inside the three files

**Interfaces:**

- Consumes: `obsbot_proto::MAX_REQUEST_LINE`; `obsbot_device::models::fake::KEYS`; `nix::fcntl::{Flock, FlockArg}`, `nix::sys::stat::{umask, Mode}`.
- Produces: `lines::{Line::{Text(String), TooLong, NotUtf8, Eof}, read_line(&mut impl BufRead, cap: usize) -> io::Result<Line>}`; `socket::{Bound { listener: UnixListener, owned: Option<PathBuf>, .. }, lock_path(&Path) -> PathBuf, take_lock(&Path) -> io::Result<Flock<File>>, activated(Option<&str>, Option<&str>, u32) -> bool, bind_at(&Path) -> io::Result<UnixListener>, open(&Path) -> io::Result<Bound>}`; `args::{USAGE, Args::{Run { fake: Option<Vec<String>> }, Help, Version}, parse(&[String]) -> Result<Args, String>}`.

`socket.rs` carries `inherited_listener` and `bind_at` and their two tests from the 0.13 daemon (`crates/obsbot4linux/src/daemon.rs` of stage G, deleted by W0 Task 6). What changes: spec 5.9 takes the lock for an inherited socket too, so `open` locks first and then inherits or binds; the `LISTEN_PID`/`LISTEN_FDS` check is the pure function `activated`; errors are `io::Error` (the 0.13 error type is gone); a third test checks that a socket something listens on is never replaced.

- [ ] **Step 1: Write the failing tests**

Add the `#[cfg(test)] mod tests` block at the end of each file of Step 3 to the doc-only file of the same name (`lines.rs`, `socket.rs`, `args.rs`).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 -- lines:: socket:: args::`
Expected: FAIL to compile: `cannot find function read_line`, `cannot find function open`, `cannot find function parse` (and the types).

- [ ] **Step 3: Write the code**

Replace `crates/obsbot4linuxd/src/lines.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The bounded line reader of a connection (4.1): one JSON object per line; a request line
//! longer than `MAX_REQUEST_LINE` (64 KiB) closes the connection. Never `BufRead::lines`,
//! which would buffer a line of any length.

use std::io::{self, BufRead};

/// One line from a client.
#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    /// A line without its `\n`. A last line without `\n` before the end of the stream counts
    /// as a line.
    Text(String),
    /// The line is longer than the cap: the caller closes the connection. The rest of the
    /// line is not read.
    TooLong,
    /// The line is not UTF-8: answered like a line that is not JSON.
    NotUtf8,
    /// The client closed its end.
    Eof,
}

/// Reads one line of at most `cap` bytes (the `\n` not counted).
pub fn read_line(reader: &mut impl BufRead, cap: usize) -> io::Result<Line> {
    let mut line: Vec<u8> = Vec::new();
    loop {
        let chunk = match reader.fill_buf() {
            Ok(chunk) => chunk,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if chunk.is_empty() {
            if line.is_empty() {
                return Ok(Line::Eof);
            }
            return Ok(text(line));
        }
        match chunk.iter().position(|b| *b == b'\n') {
            Some(at) => {
                if line.len() + at > cap {
                    return Ok(Line::TooLong);
                }
                line.extend_from_slice(&chunk[..at]);
                reader.consume(at + 1);
                return Ok(text(line));
            }
            None => {
                let n = chunk.len();
                if line.len() + n > cap {
                    return Ok(Line::TooLong);
                }
                line.extend_from_slice(chunk);
                reader.consume(n);
            }
        }
    }
}

fn text(bytes: Vec<u8>) -> Line {
    match String::from_utf8(bytes) {
        Ok(s) => Line::Text(s),
        Err(_) => Line::NotUtf8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    fn all(input: &[u8], cap: usize) -> Vec<Line> {
        // A small buffer, so that lines cross buffer boundaries.
        let mut r = BufReader::with_capacity(7, Cursor::new(input.to_vec()));
        let mut out = Vec::new();
        loop {
            let line = read_line(&mut r, cap).unwrap();
            let end = matches!(line, Line::Eof | Line::TooLong);
            out.push(line);
            if end {
                return out;
            }
        }
    }

    #[test]
    fn lines_are_split_at_newlines_and_a_last_line_needs_none() {
        assert_eq!(
            all(b"{\"a\":1}\n\n{\"b\":2}", 100),
            [
                Line::Text("{\"a\":1}".into()),
                Line::Text(String::new()),
                Line::Text("{\"b\":2}".into()),
                Line::Eof
            ]
        );
    }

    /// 4.1: a request line longer than 64 KiB closes the connection; exactly 64 KiB is a line.
    #[test]
    fn the_cap_allows_exactly_its_length_and_refuses_one_byte_more() {
        let cap = obsbot_proto::MAX_REQUEST_LINE;
        let mut at = vec![b'x'; cap];
        at.push(b'\n');
        assert_eq!(all(&at, cap)[0], Line::Text("x".repeat(cap)));
        let mut over = vec![b'x'; cap + 1];
        over.push(b'\n');
        assert_eq!(all(&over, cap), [Line::TooLong]);
        let no_newline = vec![b'x'; cap + 1];
        assert_eq!(
            all(&no_newline, cap),
            [Line::TooLong],
            "also without a newline"
        );
    }

    #[test]
    fn a_line_that_is_not_utf8_is_reported_and_the_next_line_still_reads() {
        assert_eq!(
            all(b"\xff\xfe\n{}\n", 100),
            [Line::NotUtf8, Line::Text("{}".into()), Line::Eof]
        );
    }

    #[test]
    fn an_empty_stream_is_the_end() {
        assert_eq!(all(b"", 100), [Line::Eof]);
    }
}
```

Replace `crates/obsbot4linuxd/src/socket.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The single-instance lock and the listening socket (5.9, 5.10, 11). Every daemon,
//! socket-activated or not, takes an exclusive `flock` on `<socket path>.lock` first. Then it
//! inherits the socket from systemd (`LISTEN_FDS`), or binds it: after checking that nothing
//! accepts connections on the path, it removes a stale socket file and binds under umask 077.
//!
//! Carried from the 0.13 daemon (`crates/obsbot4linux/src/daemon.rs`: `inherited_listener`,
//! `bind_at` and their tests). Changed: the lock is taken for an inherited socket too (5.9),
//! and the checks of `LISTEN_PID`/`LISTEN_FDS` are a pure function.

use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg};
use nix::sys::stat::{Mode, umask};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::FromRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

/// The first file descriptor systemd passes (`SD_LISTEN_FDS_START`).
const LISTEN_FD: i32 = 3;

/// The socket the daemon serves and the lock it holds while it runs.
pub struct Bound {
    pub listener: UnixListener,
    /// The socket file this daemon bound, removed at exit; `None` for an inherited socket.
    pub owned: Option<PathBuf>,
    _lock: Flock<File>,
}

/// `<socket path>.lock`.
pub fn lock_path(socket: &Path) -> PathBuf {
    let mut p = socket.as_os_str().to_owned();
    p.push(".lock");
    PathBuf::from(p)
}

/// Takes the single-instance lock; an error names the lock file (5.9).
pub fn take_lock(socket: &Path) -> io::Result<Flock<File>> {
    let path = lock_path(socket);
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)?;
    match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
        Ok(lock) => Ok(lock),
        Err((_, Errno::EWOULDBLOCK)) => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("another daemon holds {}", path.display()),
        )),
        Err((_, e)) => Err(io::Error::from(e)),
    }
}

/// Whether systemd passed this process a socket: `LISTEN_PID` is our pid and `LISTEN_FDS` is
/// at least 1 (sd_listen_fds(3)).
pub fn activated(listen_pid: Option<&str>, listen_fds: Option<&str>, pid: u32) -> bool {
    let pid_ok = listen_pid.and_then(|p| p.parse::<u32>().ok()) == Some(pid);
    let fds_ok = listen_fds
        .and_then(|n| n.parse::<i32>().ok())
        .is_some_and(|n| n >= 1);
    pid_ok && fds_ok
}

/// Binds `path` (the lock is already held): refuses when a daemon accepts connections there,
/// replaces a stale socket file, binds under umask 077 and sets mode 0600.
pub fn bind_at(path: &Path) -> io::Result<UnixListener> {
    if UnixStream::connect(path).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("a daemon is already listening on {}", path.display()),
        ));
    }
    let _ = std::fs::remove_file(path);
    let old = umask(Mode::from_bits_truncate(0o077));
    let listener = UnixListener::bind(path);
    umask(old);
    let listener = listener?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Lock first, then the inherited socket or a bound one (5.9, 11).
pub fn open(socket: &Path) -> io::Result<Bound> {
    let lock = take_lock(socket)?;
    let env = |k: &str| std::env::var(k).ok();
    if activated(
        env("LISTEN_PID").as_deref(),
        env("LISTEN_FDS").as_deref(),
        std::process::id(),
    ) {
        // SAFETY: systemd passed fd 3 to this process for exactly this use; nothing else in
        // the process owns it, and `activated` is true at most for the process systemd started.
        let listener = unsafe { UnixListener::from_raw_fd(LISTEN_FD) };
        return Ok(Bound {
            listener,
            owned: None,
            _lock: lock,
        });
    }
    let listener = bind_at(socket)?;
    Ok(Bound {
        listener,
        owned: Some(socket.to_path_buf()),
        _lock: lock,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn socket_in(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("o4ld-sock-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("obsbot4linux.sock")
    }

    /// Carried from 0.13 (`a_second_daemon_does_not_take_over_the_socket`), with the lock
    /// taken by `open`.
    #[test]
    fn a_second_daemon_does_not_take_over_the_socket() {
        let path = socket_in("second");
        let starting = take_lock(&path).unwrap();
        let err = open(&path).err().unwrap().to_string();
        assert!(err.contains("another daemon holds"), "{err}");
        assert!(
            err.contains("obsbot4linux.sock.lock"),
            "names the lock file: {err}"
        );
        assert!(!path.exists(), "the loser creates no socket");
        drop(starting);
        let bound = open(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(bound.owned.as_deref(), Some(path.as_path()));
        UnixStream::connect(&path).unwrap();
        bound.listener.accept().unwrap();
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Carried from 0.13 (`a_stale_socket_is_replaced`).
    #[test]
    fn a_stale_socket_is_replaced() {
        let path = socket_in("stale");
        drop(open(&path).unwrap());
        assert!(path.exists() && UnixStream::connect(&path).is_err());
        let bound = open(&path).unwrap();
        UnixStream::connect(&path).unwrap();
        bound.listener.accept().unwrap();
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// 5.9: a daemon that binds the socket itself first checks whether something already
    /// accepts connections on that path (a daemon without the lock, such as a
    /// socket-activated 0.13 daemon).
    #[test]
    fn a_listening_socket_is_never_replaced() {
        let path = socket_in("listening");
        let _other = UnixListener::bind(&path).unwrap();
        let err = open(&path).err().unwrap().to_string();
        assert!(err.contains("already listening"), "{err}");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn activation_needs_our_pid_and_at_least_one_fd() {
        assert!(activated(Some("42"), Some("1"), 42));
        assert!(activated(Some("42"), Some("2"), 42));
        assert!(
            !activated(Some("41"), Some("1"), 42),
            "another process's sockets"
        );
        assert!(!activated(Some("42"), Some("0"), 42));
        assert!(!activated(None, Some("1"), 42));
        assert!(!activated(Some("42"), None, 42));
        assert!(!activated(Some("x"), Some("1"), 42));
    }
}
```

Replace `crates/obsbot4linuxd/src/args.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line of `obsbot4linuxd` (5.12): `--fake <model>[,<model>…]` runs the daemon with
//! fake devices instead of the sysfs scan.

use obsbot_device::models::fake::KEYS;

pub const USAGE: &str = "usage: obsbot4linuxd [--fake <model>[,<model>...]]

The obsbot4linux daemon. It is started by the systemd user socket obsbot4linux.socket.

  --fake <models>  run with fake devices instead of the cameras (fake-a, fake-b, unknown);
                   the test-control socket is <socket>.test
  --help           print this help
  --version        print the version";

/// What the command line asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Args {
    /// Run the daemon; `fake` holds the fake models, in plug order.
    Run {
        fake: Option<Vec<String>>,
    },
    Help,
    Version,
}

/// Parses the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut fake: Option<Vec<String>> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let list = match arg.as_str() {
            "--help" | "-h" => return Ok(Args::Help),
            "--version" | "-V" => return Ok(Args::Version),
            "--fake" => it
                .next()
                .ok_or_else(|| "--fake needs a list of models".to_string())?
                .clone(),
            other => match other.strip_prefix("--fake=") {
                Some(list) => list.to_string(),
                None => return Err(format!("unknown argument {other}")),
            },
        };
        if fake.is_some() {
            return Err("--fake given twice".into());
        }
        let models: Vec<String> = list.split(',').map(|m| m.trim().to_string()).collect();
        if let Some(bad) = models.iter().find(|m| !KEYS.contains(&m.as_str())) {
            return Err(format!(
                "unknown fake model {bad:?}; known: {}",
                KEYS.join(", ")
            ));
        }
        fake = Some(models);
    }
    Ok(Args::Run { fake })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Args, String> {
        parse(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn no_arguments_run_the_real_devices() {
        assert_eq!(p(&[]), Ok(Args::Run { fake: None }));
    }

    /// 5.12 and 10.3: CI runs `--fake fake-a,fake-b,unknown`.
    #[test]
    fn fake_takes_a_comma_list_in_plug_order() {
        let want = Ok(Args::Run {
            fake: Some(vec!["fake-a".into(), "fake-b".into(), "unknown".into()]),
        });
        assert_eq!(p(&["--fake", "fake-a,fake-b,unknown"]), want);
        assert_eq!(p(&["--fake=fake-a,fake-b,unknown"]), want);
        assert_eq!(
            p(&["--fake", "fake-b,fake-b"]),
            Ok(Args::Run {
                fake: Some(vec!["fake-b".into(), "fake-b".into()])
            }),
            "a model may repeat"
        );
    }

    #[test]
    fn bad_command_lines_are_refused_with_a_reason() {
        assert!(p(&["--fake"]).unwrap_err().contains("needs a list"));
        assert!(
            p(&["--fake", "tiny2lite"])
                .unwrap_err()
                .contains("unknown fake model")
        );
        assert!(
            p(&["--fake", ""])
                .unwrap_err()
                .contains("unknown fake model")
        );
        assert!(
            p(&["--fake", "fake-a", "--fake", "fake-b"])
                .unwrap_err()
                .contains("twice")
        );
        assert!(p(&["daemon"]).unwrap_err().contains("unknown argument"));
        assert_eq!(p(&["--help"]), Ok(Args::Help));
        assert_eq!(p(&["--version"]), Ok(Args::Version));
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4`
Expected: PASS; the new tests: `lines` 4, `socket` 4, `args` 3.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy -p obsbot4linuxd --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/lines.rs crates/obsbot4linuxd/src/socket.rs crates/obsbot4linuxd/src/args.rs
git commit -m "obsbot4linuxd: bounded line reader, single-instance lock and socket, command line"
```

### Task b-03 (key `b-03-pending-schedule`): `pending`, echo and the poll schedule

**Files:**

- Replace: `crates/obsbot4linuxd/src/pending.rs`, `src/schedule.rs`
- Test: unit tests inside both files

**Interfaces:**

- Consumes: `obsbot_proto::{Setting, SettingType, Bound, values::{number, rounded, same}}`; `obsbot_device::contract::{Group, LiveTier, Tier}`; `crate::Timing` (b-01).
- Produces: `pending::{sent(&Setting, &Value) -> Value, echo(&Setting, &Value) -> Value, shows(&Setting, &Value, &Value) -> bool, Pending::{new, map, is_empty, contains, written(&mut, &Setting, &str, Value, Option<&Value>, Option<Instant>), group_read(&mut, &BTreeMap<String, Setting>, &BTreeMap<String, Value>, Instant) -> bool, clear}}`; `schedule::{PERIODIC, effective(&Group, &BTreeSet<String>) -> Tier, Schedule::{new(Instant, &Timing), due(Instant) -> Option<Tier>, next_due() -> Instant, groups(&[Group], Tier, &BTreeSet<String>, bool) -> Vec<String>, ticked(Tier, Instant, &Timing), touched_read(&str, bool), wake_groups(&[Group], &[String]) -> Vec<String>, woke(&[Group], &[String], &BTreeSet<String>, Instant, &Timing)}}`.

Both modules are pure: they take `now` as a parameter and never sleep.

- [ ] **Step 1: Write the failing tests**

Add the `#[cfg(test)] mod tests` block at the end of each file of Step 3 to the doc-only file of the same name.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 -- pending:: schedule::`
Expected: FAIL to compile: `cannot find type Pending`, `cannot find type Schedule`, `cannot find function echo`, `cannot find function sent`.

- [ ] **Step 3: Write the code**

Replace `crates/obsbot4linuxd/src/pending.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `pending` (4.1 rule 4, 5.6, 5.7): leaf path → the value written, while the camera accepted
//! the write and its readback does not show it yet. A path leaves `pending` when a read of its
//! group shows the written value (after rounding to the setting's precision), or when its
//! `settle_ms` has passed at a read of its group; then the camera's value stands. Nothing here
//! reads the camera: the checks run at the reads the poll plan and the write path make anyway.

use obsbot_proto::values::{number, rounded, same};
use obsbot_proto::{Setting, SettingType};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// A written value as the daemon publishes and compares it: in `pending`, as the state of a
/// `readback: false` leaf, and in the state a batch validates against (4.2 "Precision":
/// rounded where it is created, decode and echo). What is sent is [`sent`], never the echo. A
/// number is rounded to the setting's precision; without a
/// precision, a stepped number is snapped to its step counted from the minimum (the validator
/// accepts float noise such as `2.9999999999999996` on a stepped leaf); otherwise it is only
/// put in canonical form. `range`, `pose`, `point` and `box` go component by component.
/// Every other value is returned as it is.
pub fn echo(setting: &Setting, value: &Value) -> Value {
    let round = |x: &Value, component: Option<&str>| -> Value {
        let Some(f) = x.as_f64() else {
            return x.clone();
        };
        let get = |b: &Option<obsbot_proto::Bound>| b.as_ref().and_then(|b| b.get(component));
        if let Some(p) = get(&setting.precision) {
            return rounded(f, p);
        }
        if let Some(step) = get(&setting.step).filter(|st| *st > 0.0) {
            let base = get(&setting.min).unwrap_or(0.0);
            let snapped = base + ((f - base) / step).round() * step;
            return rounded(snapped, decimal_unit(step));
        }
        number(f)
    };
    match (&setting.kind, value) {
        (SettingType::Number, Value::Number(_)) => round(value, None),
        (kind, Value::Object(map)) if kind.components().is_some() => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), round(v, Some(k)));
            }
            Value::Object(out)
        }
        _ => value.clone(),
    }
}

/// The unit of the last decimal of a step as written (`100` → 1, `0.1` → 0.1, `0.25` → 0.01).
fn decimal_unit(step: f64) -> f64 {
    let text = format!("{step}");
    let decimals = text.split_once('.').map_or(0, |(_, d)| d.len());
    10f64.powi(-(decimals as i32))
}

/// The value a request sends (section 2: exactly the value it names). The one change: a
/// stepped number the validator accepted lies within 1e-6 steps of a step
/// (`validate::check_number`), and it is sent on that step, so float noise such as
/// `2.9999999999999996` reaches the model as `3`, which an encoder of whole numbers takes.
/// Nothing is rounded to a precision here (that happens where values are created for the
/// state: decode and [`echo`], 4.2); a number without a step and every other value are sent
/// as they are. `range`, `pose`, `point` and `box` go component by component.
pub fn sent(setting: &Setting, value: &Value) -> Value {
    let snap = |x: &Value, component: Option<&str>| -> Value {
        let get = |b: &Option<obsbot_proto::Bound>| b.as_ref().and_then(|b| b.get(component));
        match (x.as_f64(), get(&setting.step).filter(|st| *st > 0.0)) {
            (Some(f), Some(step)) => {
                let base = get(&setting.min).unwrap_or(0.0);
                rounded(
                    base + ((f - base) / step).round() * step,
                    decimal_unit(step),
                )
            }
            _ => x.clone(),
        }
    };
    match (&setting.kind, value) {
        (SettingType::Number, Value::Number(_)) => snap(value, None),
        (kind, Value::Object(map)) if kind.components().is_some() => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), snap(v, Some(k))))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// Whether a read value shows the written one, both rounded to the setting's precision.
pub fn shows(setting: &Setting, read: &Value, written: &Value) -> bool {
    same(&echo(setting, read), &echo(setting, written))
}

#[derive(Debug, Clone, PartialEq)]
struct Wait {
    value: Value,
    /// When the settle bound started: the write's acceptance, or (for a write whose group
    /// cannot be read while asleep) the first read of the group after it; `None` until then.
    since: Option<Instant>,
    settle: Duration,
}

/// The pending paths of one device.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pending {
    waits: BTreeMap<String, Wait>,
}

impl Pending {
    pub fn new() -> Pending {
        Pending::default()
    }

    /// The `pending` map of the published state.
    pub fn map(&self) -> BTreeMap<String, Value> {
        self.waits
            .iter()
            .map(|(k, w)| (k.clone(), w.value.clone()))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.waits.is_empty()
    }

    pub fn contains(&self, leaf: &str) -> bool {
        self.waits.contains_key(leaf)
    }

    /// After the touched re-read of a write (5.6): `read` is the leaf's value from that re-read,
    /// `None` when its group was not read or the read failed. A value that shows the write
    /// leaves (or never enters) `pending`; otherwise the leaf enters it with `echo`, replacing an
    /// earlier pending value of the same path and restarting the bound. `settle_from` is the
    /// write's acceptance, or `None` when the group cannot be read now (asleep, 5.7).
    pub fn written(
        &mut self,
        setting: &Setting,
        leaf: &str,
        echo: Value,
        read: Option<&Value>,
        settle_from: Option<Instant>,
    ) {
        if read.is_some_and(|r| shows(setting, r, &echo)) {
            self.waits.remove(leaf);
            return;
        }
        let settle = Duration::from_millis(u64::from(setting.settle_ms.unwrap_or(0)));
        self.waits.insert(
            leaf.to_string(),
            Wait {
                value: echo,
                since: settle_from,
                settle,
            },
        );
    }

    /// A successful read of one group at `now`: `values` holds every leaf of the group. Each
    /// pending leaf of the group leaves when the read shows its value, or when its bound has
    /// passed; a bound that had not started starts with this read (5.7). Returns whether
    /// `pending` changed.
    pub fn group_read(
        &mut self,
        settings: &BTreeMap<String, Setting>,
        values: &BTreeMap<String, Value>,
        now: Instant,
    ) -> bool {
        let before = self.waits.len();
        self.waits.retain(|leaf, wait| {
            let Some(read) = values.get(leaf) else {
                return true;
            };
            let Some(setting) = settings.get(leaf) else {
                return false;
            };
            let since = *wait.since.get_or_insert(now);
            if shows(setting, read, &wait.value) {
                return false;
            }
            now.duration_since(since) < wait.settle
        });
        before != self.waits.len()
    }

    /// The device became unavailable: `pending` is emptied (4.1 rule 4).
    pub fn clear(&mut self) {
        self.waits.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::SettingType;
    use serde_json::json;

    fn led() -> Setting {
        Setting::number("LED", 0.0, 3.0).step(1.0).settle_ms(1000)
    }

    fn zoom() -> Setting {
        Setting::number("Zoom", 1.0, 4.0)
            .precision(0.01)
            .settle_ms(3000)
    }

    fn settings() -> BTreeMap<String, Setting> {
        [
            ("led.brightness".to_string(), led()),
            ("zoom.factor".to_string(), zoom()),
        ]
        .into()
    }

    fn read(leaf: &str, v: Value) -> BTreeMap<String, Value> {
        [(leaf.to_string(), v)].into()
    }

    #[test]
    fn echo_rounds_to_the_precision_and_writes_integral_numbers_without_a_fraction() {
        assert_eq!(echo(&zoom(), &json!(1.5000000001)), json!(1.5));
        assert_eq!(echo(&zoom(), &json!(2.0)).to_string(), "2");
        assert_eq!(
            echo(&led(), &json!(2.0)).to_string(),
            "2",
            "no precision: canonical"
        );
        let pose = Setting::composite(
            SettingType::Pose,
            "Pose",
            &[("pan", -150.0), ("tilt", -70.0), ("zoom", 1.0)],
            &[("pan", 150.0), ("tilt", 90.0), ("zoom", 4.0)],
        )
        .precisions(&[("pan", 0.1), ("tilt", 0.1), ("zoom", 0.01)]);
        assert_eq!(
            echo(
                &pose,
                &json!({"pan": 10.04, "tilt": -0.3000001, "zoom": 1.234})
            ),
            json!({"pan": 10, "tilt": -0.3, "zoom": 1.23})
        );
        assert_eq!(echo(&Setting::boolean("B"), &json!(true)), json!(true));
    }

    /// Section 2 and review focus 1: a request sends the value it names; only float noise
    /// within the validator's step tolerance is put on the step, and nothing is rounded to a
    /// precision before it is sent.
    #[test]
    fn sent_keeps_the_value_and_only_puts_float_noise_on_its_step() {
        assert_eq!(sent(&led(), &json!(2.9999999999999996)).to_string(), "3");
        assert_eq!(sent(&led(), &json!(2)), json!(2), "on the step: unchanged");
        let tenths = Setting::number("Ratio", 1.0, 4.0).step(0.1);
        assert_eq!(sent(&tenths, &json!(0.1 + 0.2 + 1.0)).to_string(), "1.3");
        assert_eq!(
            sent(&zoom(), &json!(2.345)),
            json!(2.345),
            "no step: not rounded to the precision"
        );
        let pose = Setting::composite(
            SettingType::Pose,
            "Pose",
            &[("pan", -150.0), ("tilt", -70.0), ("zoom", 1.0)],
            &[("pan", 150.0), ("tilt", 90.0), ("zoom", 4.0)],
        )
        .precisions(&[("pan", 0.1), ("tilt", 0.1), ("zoom", 0.01)]);
        let named = json!({"pan": 10.04, "tilt": -0.3000001, "zoom": 1.234});
        assert_eq!(sent(&pose, &named), named);
        assert_eq!(sent(&Setting::boolean("B"), &json!(true)), json!(true));
    }

    /// Review focus 1: the echo of a slider's float noise on a stepped leaf without a
    /// precision is on the step, so the readback 3 ends `pending` at once instead of after the
    /// settle bound.
    #[test]
    fn echo_snaps_float_noise_to_the_step_when_there_is_no_precision() {
        let stepped = Setting::number("Times", 1.0, 3.0).step(1.0).settle_ms(1000);
        assert_eq!(echo(&stepped, &json!(2.9999999999999996)).to_string(), "3");
        let hundreds = Setting::number("Temperature", 2000.0, 10000.0).step(100.0);
        assert_eq!(echo(&hundreds, &json!(5000.000000001)).to_string(), "5000");
        let tenths = Setting::number("Ratio", 1.0, 4.0).step(0.1);
        assert_eq!(echo(&tenths, &json!(0.1 + 0.2 + 1.0)).to_string(), "1.3");
        let quarters = Setting::number("Q", 0.0, 1.0).step(0.25);
        assert_eq!(echo(&quarters, &json!(0.7500000001)).to_string(), "0.75");
        let mut p = Pending::new();
        p.written(
            &stepped,
            "beacon.times",
            echo(&stepped, &json!(2.9999999999999996)),
            Some(&json!(3)),
            Some(Instant::now()),
        );
        assert!(p.is_empty(), "the readback shows it");
    }

    /// 4.1 rules 3 and 4: the re-read shows the write: nothing pending.
    #[test]
    fn a_write_the_reread_shows_is_not_pending() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(2), Some(&json!(2)), Some(t));
        assert!(p.is_empty());
        p.written(
            &zoom(),
            "zoom.factor",
            json!(1.5),
            Some(&json!(1.504)),
            Some(t),
        );
        assert!(p.is_empty(), "equal after rounding to 0.01");
    }

    /// The golden session: the re-read does not show it yet, the next read of the group does.
    #[test]
    fn a_lagging_readback_is_pending_until_a_read_shows_it() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(2), Some(&json!(1)), Some(t));
        assert_eq!(p.map(), [("led.brightness".to_string(), json!(2))].into());
        assert!(!p.group_read(&settings(), &read("led.brightness", json!(1)), t));
        assert!(p.group_read(
            &settings(),
            &read("led.brightness", json!(2)),
            t + Duration::from_millis(300)
        ));
        assert!(p.is_empty());
    }

    /// A camera that rounds the value (fake-b's exposure time) never shows it: the bound ends it,
    /// at a read of its group, and the camera's value stands.
    #[test]
    fn the_settle_bound_ends_pending_at_a_read_of_the_group() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(3), None, Some(t));
        let other = read("led.brightness", json!(1));
        assert!(!p.group_read(&settings(), &other, t + Duration::from_millis(999)));
        assert!(
            !p.group_read(
                &settings(),
                &read("zoom.factor", json!(1)),
                t + Duration::from_secs(5)
            ),
            "a read of another group does not end it"
        );
        assert!(p.contains("led.brightness"));
        assert!(p.group_read(&settings(), &other, t + Duration::from_millis(1000)));
        assert!(p.is_empty());
    }

    /// 5.7: written while asleep, its group unreadable asleep: the bound starts at the first
    /// read after wake, however long the camera slept.
    #[test]
    fn a_bound_that_had_not_started_starts_at_the_first_read() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(3), None, None);
        let wake = t + Duration::from_secs(600);
        assert!(!p.group_read(&settings(), &read("led.brightness", json!(0)), wake));
        assert!(
            p.contains("led.brightness"),
            "the bound starts now, it has not passed"
        );
        assert!(p.group_read(
            &settings(),
            &read("led.brightness", json!(0)),
            wake + Duration::from_millis(1000)
        ));
    }

    /// 4.1 rule 4: a new write of the same path replaces its pending value and restarts the bound.
    #[test]
    fn a_new_write_replaces_the_value_and_restarts_the_bound() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(2), None, Some(t));
        let later = t + Duration::from_millis(900);
        p.written(&led(), "led.brightness", json!(3), None, Some(later));
        assert_eq!(p.map()["led.brightness"], json!(3));
        assert!(!p.group_read(
            &settings(),
            &read("led.brightness", json!(0)),
            t + Duration::from_millis(1500)
        ));
        assert!(
            p.contains("led.brightness"),
            "1000 ms count from the second write"
        );
    }

    #[test]
    fn a_write_the_reread_shows_removes_an_older_pending_value() {
        let mut p = Pending::new();
        let t = Instant::now();
        p.written(&led(), "led.brightness", json!(2), None, Some(t));
        p.written(&led(), "led.brightness", json!(1), Some(&json!(1)), Some(t));
        assert!(p.is_empty());
    }

    #[test]
    fn unavailable_empties_pending() {
        let mut p = Pending::new();
        p.written(&led(), "led.brightness", json!(2), None, None);
        p.clear();
        assert!(p.is_empty());
    }
}
```

Replace `crates/obsbot4linuxd/src/schedule.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The poll schedule of one device (5.7). Each periodic tier ticks on its own: its interval
//! counts from the end of its previous tick, so a slow read never starves the 1 s tier. A
//! tick reads the groups whose tier is that tier, or whose live tier is that tier while a
//! connection subscribes the group's live topic. While the camera is asleep a tick reads only
//! the groups readable asleep. A `touched` group is never read by a tick, except a `touched`
//! group whose last read failed or was cut short: it is read with the next tick of the 60 s
//! tier, or at the next wake, whichever comes first.

use crate::Timing;
use obsbot_device::contract::{Group, Tier};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

/// The periodic tiers, fastest first.
pub const PERIODIC: [Tier; 3] = [Tier::Second, Tier::FiveSeconds, Tier::Minute];

/// The tier a group is read in now: its live tier while a connection subscribes its topic.
pub fn effective(group: &Group, topics: &BTreeSet<String>) -> Tier {
    match &group.live {
        Some(live) if topics.contains(&live.topic) => live.tier,
        _ => group.tier,
    }
}

/// When each periodic tier is due next, and which `touched` groups wait for a retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    next: BTreeMap<Tier, Instant>,
    retry: BTreeSet<String>,
}

impl Schedule {
    /// The schedule after the first read attempt, which ended at `end`: every periodic tier
    /// ticks one period later.
    pub fn new(end: Instant, timing: &Timing) -> Schedule {
        let next = PERIODIC
            .iter()
            .map(|t| (*t, end + timing.period(*t).expect("periodic")))
            .collect();
        Schedule {
            next,
            retry: BTreeSet::new(),
        }
    }

    /// The tier to tick now: the one due longest ago, the faster one on a tie; `None` when no
    /// tier is due.
    pub fn due(&self, now: Instant) -> Option<Tier> {
        PERIODIC
            .iter()
            .filter(|t| self.next[t] <= now)
            .min_by_key(|t| self.next[t])
            .copied()
    }

    /// When the next tier is due.
    pub fn next_due(&self) -> Instant {
        *self.next.values().min().expect("three tiers")
    }

    /// The groups one tick of `tier` reads, in poll-plan order.
    pub fn groups(
        &self,
        plan: &[Group],
        tier: Tier,
        topics: &BTreeSet<String>,
        asleep: bool,
    ) -> Vec<String> {
        plan.iter()
            .filter(|g| !asleep || g.asleep)
            .filter(|g| {
                let t = effective(g, topics);
                t == tier
                    || (tier == Tier::Minute && t == Tier::Touched && self.retry.contains(&g.name))
            })
            .map(|g| g.name.clone())
            .collect()
    }

    /// The tick of `tier` ended at `end`: its next tick is one period later.
    pub fn ticked(&mut self, tier: Tier, end: Instant, timing: &Timing) {
        if let Some(period) = timing.period(tier) {
            self.next.insert(tier, end + period);
        }
    }

    /// The outcome of a read of a `touched` group (at plug-in, after a request, at a retry):
    /// a failed or cut read is retried (5.7 "Wake and retries"), a good read ends the retry.
    pub fn touched_read(&mut self, group: &str, ok: bool) {
        if ok {
            self.retry.remove(group);
        } else {
            self.retry.insert(group.to_string());
        }
    }

    /// The groups the wake read reads (5.7): every group with a leaf in `unread`, `touched`
    /// groups included, in poll-plan order.
    pub fn wake_groups(plan: &[Group], unread: &[String]) -> Vec<String> {
        plan.iter()
            .filter(|g| g.leaves.iter().any(|l| unread.contains(l)))
            .map(|g| g.name.clone())
            .collect()
    }

    /// The wake read of `groups` ended at `end`: the next tick of each tier one of them is read
    /// in counts from this read.
    pub fn woke(
        &mut self,
        plan: &[Group],
        groups: &[String],
        topics: &BTreeSet<String>,
        end: Instant,
        timing: &Timing,
    ) {
        for g in plan.iter().filter(|g| groups.contains(&g.name)) {
            let tier = effective(g, topics);
            self.ticked(tier, end, timing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::contract::LiveTier;
    use std::time::Duration;

    fn group(name: &str, tier: Tier, asleep: bool, live: Option<(&str, Tier)>) -> Group {
        Group {
            name: name.into(),
            tier,
            asleep,
            live: live.map(|(t, tier)| LiveTier {
                topic: t.into(),
                tier,
            }),
            leaves: vec![format!("{name}.x")],
        }
    }

    fn plan() -> Vec<Group> {
        vec![
            group("status", Tier::Second, true, None),
            group(
                "gimbal",
                Tier::FiveSeconds,
                false,
                Some(("pose", Tier::Second)),
            ),
            group("led", Tier::FiveSeconds, false, None),
            group("zone", Tier::Minute, false, None),
            group("presets", Tier::Touched, false, None),
        ]
    }

    fn none() -> BTreeSet<String> {
        BTreeSet::new()
    }

    #[test]
    fn every_tier_ticks_one_period_after_the_first_read() {
        let t = Instant::now();
        let s = Schedule::new(t, &Timing::SPEC);
        assert_eq!(s.due(t), None);
        assert_eq!(s.due(t + Duration::from_secs(1)), Some(Tier::Second));
        assert_eq!(s.next_due(), t + Duration::from_secs(1));
        assert_eq!(
            s.due(t + Duration::from_secs(60)),
            Some(Tier::Second),
            "the one due longest ago first"
        );
    }

    /// 5.7: intervals count from the end of the previous tick of that tier.
    #[test]
    fn a_tick_counts_from_its_end() {
        let t = Instant::now();
        let mut s = Schedule::new(t, &Timing::SPEC);
        let end = t + Duration::from_millis(1700);
        s.ticked(Tier::Second, end, &Timing::SPEC);
        assert_eq!(s.due(t + Duration::from_millis(2600)), None);
        assert_eq!(s.due(end + Duration::from_secs(1)), Some(Tier::Second));
    }

    #[test]
    fn a_tick_reads_its_tier_and_asleep_only_the_groups_readable_asleep() {
        let s = Schedule::new(Instant::now(), &Timing::SPEC);
        assert_eq!(s.groups(&plan(), Tier::Second, &none(), false), ["status"]);
        assert_eq!(
            s.groups(&plan(), Tier::FiveSeconds, &none(), false),
            ["gimbal", "led"]
        );
        assert_eq!(
            s.groups(&plan(), Tier::FiveSeconds, &none(), true),
            Vec::<String>::new()
        );
        assert_eq!(s.groups(&plan(), Tier::Second, &none(), true), ["status"]);
        assert_eq!(
            s.groups(&plan(), Tier::Minute, &none(), false),
            ["zone"],
            "touched: never"
        );
    }

    /// 5.7: a group with a live topic is read in its faster tier while the topic is subscribed.
    #[test]
    fn a_subscribed_live_topic_moves_its_group_to_the_live_tier() {
        let s = Schedule::new(Instant::now(), &Timing::SPEC);
        let pose: BTreeSet<String> = ["pose".to_string()].into();
        assert_eq!(
            s.groups(&plan(), Tier::Second, &pose, false),
            ["status", "gimbal"]
        );
        assert_eq!(s.groups(&plan(), Tier::FiveSeconds, &pose, false), ["led"]);
        let other: BTreeSet<String> = ["target".to_string()].into();
        assert_eq!(s.groups(&plan(), Tier::Second, &other, false), ["status"]);
    }

    /// 5.7: a failed `touched` read is read again with the next 60 s tick until a read succeeds.
    #[test]
    fn a_failed_touched_read_rides_the_minute_tier_until_it_succeeds() {
        let mut s = Schedule::new(Instant::now(), &Timing::SPEC);
        s.touched_read("presets", false);
        assert_eq!(
            s.groups(&plan(), Tier::Minute, &none(), false),
            ["zone", "presets"]
        );
        assert_eq!(
            s.groups(&plan(), Tier::FiveSeconds, &none(), false),
            ["gimbal", "led"]
        );
        assert_eq!(
            s.groups(&plan(), Tier::Minute, &none(), true),
            Vec::<String>::new(),
            "not while asleep: the wake read takes it"
        );
        s.touched_read("presets", true);
        assert_eq!(s.groups(&plan(), Tier::Minute, &none(), false), ["zone"]);
    }

    /// 5.7: at wake every group with a leaf in `unread` is read once, `touched` groups
    /// included, and the next tick of their tiers counts from that read.
    #[test]
    fn the_wake_read_takes_every_group_with_an_unread_leaf() {
        let unread = vec!["led.x".to_string(), "presets.x".to_string()];
        assert_eq!(Schedule::wake_groups(&plan(), &unread), ["led", "presets"]);
        let t = Instant::now();
        let mut s = Schedule::new(t, &Timing::SPEC);
        let end = t + Duration::from_secs(4);
        s.woke(
            &plan(),
            &["led".into(), "presets".into()],
            &none(),
            end,
            &Timing::SPEC,
        );
        assert_eq!(s.due(t + Duration::from_secs(5)), Some(Tier::Second));
        assert_eq!(s.due(t + Duration::from_millis(8500)), Some(Tier::Second));
        let mut later = s.clone();
        later.ticked(Tier::Second, t + Duration::from_secs(9), &Timing::SPEC);
        assert_eq!(
            later.due(t + Duration::from_secs(9)),
            Some(Tier::FiveSeconds),
            "4 + 5 s"
        );
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 -- pending:: schedule::`
Expected: PASS: `pending` 10, `schedule` 6.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy -p obsbot4linuxd --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/pending.rs crates/obsbot4linuxd/src/schedule.rs
git commit -m "obsbot4linuxd: pending with the settle bound, echo of written values, poll schedule"
```

### Task b-04 (key `b-04-identity`): Identity series

**Files:**

- Replace: `crates/obsbot4linuxd/src/identity.rs` (the stub of b-01; `PortReason` and `choose_id` stay as they are)
- Test: unit tests inside the file

**Interfaces:**

- Consumes: `obsbot_device::contract::{Identity, IdentitySource, Need}`; `obsbot_proto::{is_valid_serial, port_id, sn_id}`.
- Produces: `identity::Series::{new(IdentitySource, bool), at_plug_in() -> Option<Need>, answered(Option<&Identity>), after_first_status(bool), at_tick(bool) -> Option<Need>, at_wake() -> Option<Need>, done() -> bool, reason() -> PortReason}` with pub fields `serial: Option<String>` (valid only), `invalid: bool`, `firmware: Option<String>`; unchanged `PortReason`, `choose_id`.

- [ ] **Step 1: Write the failing tests**

Add the `#[cfg(test)] mod tests` block at the end of the file of Step 3 to the stub.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 identity::`
Expected: FAIL: 7 tests panic with `not yet implemented: task b-04-identity`; `the_id_is_the_serial_unless_an_available_device_holds_it` passes.

- [ ] **Step 3: Write the code**

Replace `crates/obsbot4linuxd/src/identity.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The identity series of one plug-in (5.5) as a state machine, and the choice of the id.
//!
//! - The first attempt is sent at plug-in, asleep or awake. Attempts 2 and 3 ride the next two
//!   ticks of the tier the model names (`ModelEntry::identity_tier`) while the camera is awake.
//! - If the first attempt gave no valid serial and the first read of the group holding `asleep`
//!   reported the camera asleep, the first series ends there; one further series of three
//!   attempts runs from the camera's first wake: at the wake, then with the next two ticks.
//! - A series ends at the first attempt after which serial and firmware are both answered; an
//!   attempt asks only for what is missing. A model with neither query sends no attempt.
//!
//! No more than one attempt is ever sent to a sleeping camera.

use obsbot_device::contract::{Identity, IdentitySource, Need};
use obsbot_proto::{is_valid_serial, port_id, sn_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Before the plug-in attempt.
    Plug,
    /// Attempt 1 was sent; waiting for the first status read.
    First,
    /// `left` attempts ride the next ticks of the identity tier.
    Ticks {
        left: u8,
    },
    /// The first series ended asleep; the wake series starts at the first wake.
    AwaitWake,
    Done,
}

/// Why a supported device got a `port:` id (5.11 log line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortReason {
    NoSerial,
    InvalidSerial,
    Duplicate,
}

impl PortReason {
    pub fn text(self) -> &'static str {
        match self {
            PortReason::NoSerial => "no serial answered",
            PortReason::InvalidSerial => "an invalid serial",
            PortReason::Duplicate => "a duplicate serial",
        }
    }
}

/// The identity series of one plug-in, and what it has learnt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    source: IdentitySource,
    firmware_query: bool,
    phase: Phase,
    woke: bool,
    /// A valid serial, once answered. Never logged (4.2).
    pub serial: Option<String>,
    /// The camera answered a serial that is not valid (and no valid one yet).
    pub invalid: bool,
    pub firmware: Option<String>,
}

impl Series {
    pub fn new(source: IdentitySource, firmware_query: bool) -> Series {
        Series {
            source,
            firmware_query,
            phase: Phase::Plug,
            woke: false,
            serial: None,
            invalid: false,
            firmware: None,
        }
    }

    fn need(&self, plug_in: bool) -> Need {
        Need {
            serial: match self.source {
                IdentitySource::Query => self.serial.is_none(),
                IdentitySource::UsbSerial => plug_in,
                IdentitySource::Absent => false,
            },
            firmware: self.firmware_query && self.firmware.is_none(),
        }
    }

    fn complete(&self) -> bool {
        let n = self.need(false);
        !n.serial && !n.firmware
    }

    /// Attempt 1, at plug-in; `None` when the model has nothing to ask.
    pub fn at_plug_in(&mut self) -> Option<Need> {
        if self.phase != Phase::Plug {
            return None;
        }
        let need = self.need(true);
        if !need.serial && !need.firmware {
            self.phase = Phase::Done;
            return None;
        }
        self.phase = Phase::First;
        Some(need)
    }

    /// The outcome of an attempt: `None` when it failed. Answers are kept only once given.
    pub fn answered(&mut self, got: Option<&Identity>) {
        if let Some(id) = got {
            if self.serial.is_none()
                && let Some(s) = &id.serial
            {
                if is_valid_serial(s) {
                    self.serial = Some(s.clone());
                    self.invalid = false;
                } else {
                    self.invalid = true;
                }
            }
            if self.firmware.is_none() {
                self.firmware.clone_from(&id.firmware);
            }
        }
        if self.complete() || self.phase == (Phase::Ticks { left: 0 }) {
            self.phase = Phase::Done;
        }
    }

    /// The first read of the group holding `asleep` (5.2 step 3) reported `asleep`; `false`
    /// as well for a model without an `asleep` leaf.
    pub fn after_first_status(&mut self, asleep: bool) {
        if self.phase != Phase::First {
            return;
        }
        self.phase = if self.serial.is_none() && asleep {
            Phase::AwaitWake
        } else {
            Phase::Ticks { left: 2 }
        };
    }

    /// A tick of the identity tier: the attempt that rides it, if any (only while awake).
    pub fn at_tick(&mut self, awake: bool) -> Option<Need> {
        let Phase::Ticks { left } = self.phase else {
            return None;
        };
        if !awake || left == 0 {
            return None;
        }
        self.phase = Phase::Ticks { left: left - 1 };
        Some(self.need(false))
    }

    /// `asleep` turned false: the first attempt of the wake series, if one is due.
    pub fn at_wake(&mut self) -> Option<Need> {
        if self.phase != Phase::AwaitWake || self.woke {
            return None;
        }
        self.woke = true;
        self.phase = Phase::Ticks { left: 2 };
        Some(self.need(false))
    }

    /// No further attempt will be sent for this plug-in.
    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }

    /// Why the id is a `port:` id when no valid serial is known.
    pub fn reason(&self) -> PortReason {
        if self.invalid {
            PortReason::InvalidSerial
        } else {
            PortReason::NoSerial
        }
    }
}

/// The id of a supported device (5.5): `sn:<serial>` for a valid serial that no available
/// device holds, else `port:<usb path>` with the reason.
pub fn choose_id(
    serial: Option<&str>,
    reason: PortReason,
    usb_path: &str,
    held_by_an_available_device: impl Fn(&str) -> bool,
) -> (String, Option<PortReason>) {
    match serial {
        Some(s) => {
            let id = sn_id(s);
            if held_by_an_available_device(&id) {
                (port_id(usb_path), Some(PortReason::Duplicate))
            } else {
                (id, None)
            }
        }
        None => (port_id(usb_path), Some(reason)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn both() -> Need {
        Need {
            serial: true,
            firmware: true,
        }
    }

    fn answer(serial: Option<&str>, firmware: Option<&str>) -> Identity {
        Identity {
            serial: serial.map(String::from),
            firmware: firmware.map(String::from),
        }
    }

    /// 10.3 B: "an identity series ending at the first complete answer".
    #[test]
    fn a_complete_first_answer_ends_the_series() {
        let mut s = Series::new(IdentitySource::Query, true);
        assert_eq!(s.at_plug_in(), Some(both()));
        s.answered(Some(&answer(Some("FAKEA0000001"), Some("1.0"))));
        assert!(s.done());
        s.after_first_status(false);
        assert_eq!(s.at_tick(true), None, "nothing more is sent");
        assert_eq!(s.serial.as_deref(), Some("FAKEA0000001"));
    }

    /// Attempts 2 and 3 ride the next two ticks of the identity tier and ask only what is missing.
    #[test]
    fn missing_answers_are_asked_again_twice_and_only_what_is_missing() {
        let mut s = Series::new(IdentitySource::Query, true);
        s.at_plug_in();
        s.answered(Some(&answer(None, Some("1.0"))));
        s.after_first_status(false);
        assert_eq!(
            s.at_tick(true),
            Some(Need {
                serial: true,
                firmware: false
            })
        );
        s.answered(None);
        assert!(!s.done());
        assert_eq!(
            s.at_tick(false),
            None,
            "the tier is not read asleep: the attempt waits"
        );
        assert!(s.at_tick(true).is_some(), "attempt 3");
        s.answered(None);
        assert!(s.done(), "three attempts at most");
        assert_eq!(s.at_tick(true), None);
        assert_eq!(
            s.at_wake(),
            None,
            "the wake series is only for a camera asleep at plug-in"
        );
    }

    #[test]
    fn a_later_attempt_that_completes_ends_the_series() {
        let mut s = Series::new(IdentitySource::Query, true);
        s.at_plug_in();
        s.answered(None);
        s.after_first_status(false);
        assert_eq!(s.at_tick(true), Some(both()));
        s.answered(Some(&answer(Some("FAKEA0000001"), Some("1.0"))));
        assert!(s.done());
        assert_eq!(s.at_tick(true), None);
    }

    /// 5.5 wake series: asleep at plug-in without a serial, one attempt only, then three from
    /// the first wake: so no more than one attempt reaches a sleeping camera.
    #[test]
    fn asleep_at_plug_in_the_series_waits_for_the_first_wake() {
        let mut s = Series::new(IdentitySource::Query, true);
        assert!(s.at_plug_in().is_some());
        s.answered(None);
        s.after_first_status(true);
        assert_eq!(s.at_tick(true), None, "the first series has ended");
        assert_eq!(s.at_wake(), Some(both()), "attempt 1 of the wake series");
        s.answered(None);
        assert!(s.at_tick(true).is_some());
        s.answered(None);
        assert!(s.at_tick(true).is_some());
        s.answered(None);
        assert!(s.done());
        assert_eq!(s.at_wake(), None, "one wake series only");
    }

    #[test]
    fn asleep_with_a_serial_answered_needs_no_wake_series() {
        let mut s = Series::new(IdentitySource::Query, true);
        s.at_plug_in();
        s.answered(Some(&answer(Some("FAKEA0000001"), None)));
        s.after_first_status(true);
        assert_eq!(s.at_wake(), None);
        assert_eq!(
            s.at_tick(true),
            Some(Need {
                serial: false,
                firmware: true
            }),
            "the firmware is still asked, with the ticks"
        );
    }

    /// 5.5: the USB serial is read once at plug-in; later attempts are the firmware query alone;
    /// a model with neither query sends nothing.
    #[test]
    fn usb_serial_models_and_models_without_queries() {
        let mut s = Series::new(IdentitySource::UsbSerial, true);
        assert_eq!(s.at_plug_in(), Some(both()));
        s.answered(Some(&answer(None, None)));
        s.after_first_status(false);
        assert_eq!(
            s.at_tick(true),
            Some(Need {
                serial: false,
                firmware: true
            })
        );
        let mut none = Series::new(IdentitySource::Absent, false);
        assert_eq!(none.at_plug_in(), None);
        assert!(none.done());
        let mut fw = Series::new(IdentitySource::Absent, true);
        assert_eq!(
            fw.at_plug_in(),
            Some(Need {
                serial: false,
                firmware: true
            })
        );
    }

    #[test]
    fn an_invalid_serial_is_not_kept_and_is_the_reason_for_the_port_id() {
        let mut s = Series::new(IdentitySource::Query, false);
        s.at_plug_in();
        s.answered(Some(&answer(Some("BAD-1"), None)));
        assert_eq!(s.serial, None);
        assert_eq!(s.reason(), PortReason::InvalidSerial);
        assert!(!s.done(), "asked again");
    }

    /// 5.5 duplicates: a serial an available device already holds gives a `port:` id.
    #[test]
    fn the_id_is_the_serial_unless_an_available_device_holds_it() {
        assert_eq!(
            choose_id(Some("FAKEA0000001"), PortReason::NoSerial, "9-1", |_| false),
            ("sn:FAKEA0000001".to_string(), None)
        );
        assert_eq!(
            choose_id(Some("FAKEA0000001"), PortReason::NoSerial, "9-4", |id| id
                == "sn:FAKEA0000001"),
            ("port:9-4".to_string(), Some(PortReason::Duplicate))
        );
        assert_eq!(
            choose_id(None, PortReason::InvalidSerial, "9-1", |_| false),
            ("port:9-1".to_string(), Some(PortReason::InvalidSerial))
        );
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 identity::`
Expected: PASS: 8 tests.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy -p obsbot4linuxd --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/identity.rs
git commit -m "obsbot4linuxd: identity series of 5.5 as a state machine"
```

### Task b-05 (key `b-05-queue`): Device queue, batch plan, continuing-motion record

**Files:**

- Replace: `crates/obsbot4linuxd/src/queue.rs` (the stub of b-01; the types and the methods b-01 wrote stay as they are)
- Test: unit tests inside the file

**Interfaces:**

- Consumes: `crate::MAX_QUEUED`, `crate::outbound::{ConnId, Outbound, error, error_line}` (b-01); `obsbot_device::contract::Motion`; `obsbot_proto::{ErrorCode, Params, Patch, RequestId}`.
- Produces: `queue::{Record::{after_act(&str, Motion, ConnId, bool), on_close(ConnId) -> Option<String>}, Refused::{Busy(Box<Job>), Closed(Box<Job>)}, DeviceQueue::{set_stops, is_stop, push(Job) -> Result<(), Refused>, take_all() -> Vec<Job>, put_back(Vec<Job>), len, close_conn(ConnId, Instant) -> Option<String>, record_act, expire(Instant, Duration) -> Vec<Job>, end, shutdown, is_ended, is_shutdown, close() -> Vec<Job>, wait(Instant)}, answer_late(Vec<Job>), Plan { take, coalesced }, plan(&[Job], impl Fn(&Job) -> bool, impl Fn(&Patch) -> Option<String>) -> Plan}`.

- [ ] **Step 1: Write the failing tests**

Add the `#[cfg(test)] mod tests` block at the end of the file of Step 3 to the stub.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 queue::`
Expected: FAIL: the tests panic with `not yet implemented: task b-05-queue`.

- [ ] **Step 3: Write the code**

Replace `crates/obsbot4linuxd/src/queue.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The queue of one supported device (4.1 rules 2 and 9, 5.6): at most `MAX_QUEUED` requests,
//! FIFO, except that a stop is taken before every queued request. Connection readers push,
//! the device's worker takes batches, the deadline sweeper removes requests not started within
//! `REQUEST_DEADLINE`. The queue also holds the continuing-motion record (5.6) and the flags
//! that cut a camera operation short.

use crate::MAX_QUEUED;
use crate::outbound::{ConnId, Outbound, error, error_line};
use obsbot_device::contract::Motion;
use obsbot_proto::{ErrorCode, Params, Patch, RequestId};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// What a device request asks.
#[derive(Debug, Clone, PartialEq)]
pub enum Work {
    /// `paths: None` for the full state.
    Get(Option<Vec<String>>),
    Set(Patch),
    Do {
        action: String,
        params: Params,
    },
}

/// One queued device request.
#[derive(Clone)]
pub struct Job {
    pub id: RequestId,
    /// Where the reply goes; `None` for the daemon's own stop of a continuing action.
    pub out: Option<Outbound>,
    /// The connection that sent it; 0 for the daemon's own stop.
    pub conn: ConnId,
    /// The device id the request named.
    pub device: String,
    pub work: Work,
    pub arrived: Instant,
    /// A stop action (`Motion::Stop`): taken before every queued request.
    pub stop: bool,
}

impl Job {
    /// The daemon's own stop of a continuing action whose connection closed (5.6).
    pub fn own_stop(action: &str, now: Instant) -> Job {
        Job {
            id: RequestId::Int(0),
            out: None,
            conn: 0,
            device: String::new(),
            work: Work::Do {
                action: action.to_string(),
                params: Params::new(),
            },
            arrived: now,
            stop: true,
        }
    }

    /// Sends a reply line to the job's connection, if it has one.
    pub fn answer(&self, line: String) {
        if let Some(out) = &self.out {
            out.send(line);
        }
    }

    /// Answers the job with an error.
    pub fn fail(&self, code: ErrorCode, message: &str) {
        self.answer(error_line(Some(&self.id), error(code, message)));
    }
}

/// Which connection's continuing action runs on a device (5.6), and the stop that ends it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    turn: Option<(ConnId, String)>,
}

impl Record {
    /// An action ran (the camera accepted it). A continuing action without `keep: true`
    /// records its connection; with `keep: true` nothing is recorded, since closing the
    /// connection must not stop it. Any other motion action clears the record; a stop clears it
    /// only when it is the stop the record names.
    pub fn after_act(&mut self, action: &str, motion: Motion, conn: ConnId, keep: bool) {
        match motion {
            Motion::None => {}
            Motion::Move => self.turn = None,
            Motion::Continuing { stop } => {
                self.turn = (!keep && conn != 0).then(|| (conn, stop.to_string()));
            }
            Motion::Stop => {
                if self.turn.as_ref().is_some_and(|(_, s)| s == action) {
                    self.turn = None;
                }
            }
        }
    }

    /// The connection `conn` closed: the stop to queue for it, if its continuing action is the
    /// recorded one. The record is cleared.
    pub fn on_close(&mut self, conn: ConnId) -> Option<String> {
        match &self.turn {
            Some((c, _)) if *c == conn => self.turn.take().map(|(_, stop)| stop),
            _ => None,
        }
    }
}

/// Why the queue did not take a job.
pub enum Refused {
    /// `MAX_QUEUED` requests wait already: answer `busy`.
    Busy(Box<Job>),
    /// The device became unavailable and its worker ended: answer as for an unavailable device.
    Closed(Box<Job>),
}

struct Inner {
    jobs: VecDeque<Job>,
    closed: bool,
    stops: Vec<String>,
    record: Record,
}

/// The queue of one device, shared by the readers, the worker and the sweeper.
pub struct DeviceQueue {
    inner: Mutex<Inner>,
    cv: Condvar,
    /// Cuts a group read at the next frame boundary: a stop arrived, the device went, or the
    /// daemon stops (5.6 "Stops").
    pub cut: Arc<AtomicBool>,
    /// Cuts an identity attempt or a poll: the device went or the daemon stops.
    pub halt: Arc<AtomicBool>,
    /// Cuts a started request: the device went (5.6 "Unavailable").
    pub ended: Arc<AtomicBool>,
    shutdown: AtomicBool,
}

impl DeviceQueue {
    pub fn new() -> Arc<DeviceQueue> {
        Arc::new(DeviceQueue {
            inner: Mutex::new(Inner {
                jobs: VecDeque::new(),
                closed: false,
                stops: Vec::new(),
                record: Record::default(),
            }),
            cv: Condvar::new(),
            cut: Arc::new(AtomicBool::new(false)),
            halt: Arc::new(AtomicBool::new(false)),
            ended: Arc::new(AtomicBool::new(false)),
            shutdown: AtomicBool::new(false),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The stop actions of the device's model (`Motion::Stop`), set by the worker once it has
    /// opened the camera, before the device is announced.
    pub fn set_stops(&self, stops: Vec<String>) {
        self.lock().stops = stops;
    }

    pub fn is_stop(&self, action: &str) -> bool {
        self.lock().stops.iter().any(|s| s == action)
    }

    /// Queues a job. A stop goes before every queued request that is not a stop, and sets
    /// `cut` so that a group read in progress ends at its next frame boundary.
    pub fn push(&self, job: Job) -> Result<(), Refused> {
        let mut inner = self.lock();
        if inner.closed {
            return Err(Refused::Closed(Box::new(job)));
        }
        if inner.jobs.len() >= MAX_QUEUED {
            return Err(Refused::Busy(Box::new(job)));
        }
        if job.stop {
            let at = inner.jobs.iter().take_while(|j| j.stop).count();
            inner.jobs.insert(at, job);
            self.cut.store(true, Ordering::SeqCst);
        } else {
            inner.jobs.push_back(job);
        }
        drop(inner);
        self.cv.notify_all();
        Ok(())
    }

    /// Takes every queued job; the worker plans its batch from them and puts the rest back.
    pub fn take_all(&self) -> Vec<Job> {
        let mut inner = self.lock();
        let jobs: Vec<Job> = inner.jobs.drain(..).collect();
        if !self.halt.load(Ordering::SeqCst) {
            self.cut.store(false, Ordering::SeqCst);
        }
        jobs
    }

    /// Puts jobs back at the front, in their order (the requests after a failed one, 4.1 rule
    /// 9, or the rest of a batch plan). A stop pushed meanwhile stays in front of them.
    pub fn put_back(&self, jobs: Vec<Job>) {
        let mut inner = self.lock();
        let stops = inner.jobs.iter().take_while(|j| j.stop).count();
        for (i, job) in jobs.into_iter().enumerate() {
            inner.jobs.insert(stops + i, job);
        }
    }

    pub fn len(&self) -> usize {
        self.lock().jobs.len()
    }

    /// The connection `conn` closed: its queued requests are dropped (4.1 rule 9), and if its
    /// continuing action is recorded, the model's stop is queued for it (5.6). Returns the stop.
    pub fn close_conn(&self, conn: ConnId, now: Instant) -> Option<String> {
        let mut inner = self.lock();
        inner.jobs.retain(|j| j.conn != conn);
        let stop = inner.record.on_close(conn)?;
        if inner.closed {
            return None;
        }
        let at = inner.jobs.iter().take_while(|j| j.stop).count();
        inner.jobs.insert(at, Job::own_stop(&stop, now));
        drop(inner);
        self.cut.store(true, Ordering::SeqCst);
        self.cv.notify_all();
        Some(stop)
    }

    /// Updates the continuing-motion record after the camera accepted an action.
    pub fn record_act(&self, action: &str, motion: Motion, conn: ConnId, keep: bool) {
        self.lock().record.after_act(action, motion, conn, keep);
    }

    /// Removes the jobs not started within `deadline` of their arrival (4.1 rule 9).
    pub fn expire(&self, now: Instant, deadline: Duration) -> Vec<Job> {
        let mut inner = self.lock();
        let (late, keep): (Vec<Job>, Vec<Job>) = inner
            .jobs
            .drain(..)
            .partition(|j| now.duration_since(j.arrived) >= deadline);
        inner.jobs = keep.into();
        late
    }

    /// The device became unavailable: every camera operation is cut and the worker ends.
    pub fn end(&self) {
        self.ended.store(true, Ordering::SeqCst);
        self.halt.store(true, Ordering::SeqCst);
        self.cut.store(true, Ordering::SeqCst);
        self.cv.notify_all();
    }

    /// The daemon stops: polls and identity attempts are cut; the worker sends the stops still
    /// queued and ends (5.9).
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.halt.store(true, Ordering::SeqCst);
        self.cut.store(true, Ordering::SeqCst);
        self.cv.notify_all();
    }

    pub fn is_ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// The worker is about to end: no job is taken any more, and the queued ones are returned
    /// to be answered as for an unavailable device.
    pub fn close(&self) -> Vec<Job> {
        let mut inner = self.lock();
        inner.closed = true;
        inner.jobs.drain(..).collect()
    }

    /// Waits until a job is queued, the device ends or the daemon stops, or `until` passes.
    pub fn wait(&self, until: Instant) {
        let mut inner = self.lock();
        loop {
            if !inner.jobs.is_empty() || self.is_ended() || self.is_shutdown() {
                return;
            }
            let now = Instant::now();
            if now >= until {
                return;
            }
            inner = self
                .cv
                .wait_timeout(inner, until - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// Answers the requests the sweeper removed: `camera`, not started within the deadline.
pub fn answer_late(jobs: Vec<Job>) {
    for job in jobs {
        job.fail(ErrorCode::Camera, "the request was not started within 15 s");
    }
}

/// How the worker takes a batch (4.1 rule 9, 5.6): `take` jobs from the front of the queue;
/// `coalesced[i]` marks a single-leaf `set` that a later `set` of the same connection in the
/// batch, with the same single leaf, makes unnecessary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub take: usize,
    pub coalesced: Vec<bool>,
}

/// Plans one batch. A stop runs alone; a request with a `restart` warning runs alone and ends
/// the batch before it; otherwise the batch runs up to the first such request. `restarts`
/// tells a request with a `restart` warning; `single_leaf` gives the one leaf of a `set`
/// whose patch has exactly one leaf with a value the setting can take, else `None`.
pub fn plan(
    jobs: &[Job],
    restarts: impl Fn(&Job) -> bool,
    single_leaf: impl Fn(&Patch) -> Option<String>,
) -> Plan {
    let take = match jobs.first() {
        None => 0,
        Some(first) if first.stop || restarts(first) => 1,
        Some(_) => jobs
            .iter()
            .position(|j| j.stop || restarts(j))
            .unwrap_or(jobs.len()),
    };
    let leaves: Vec<Option<String>> = jobs[..take]
        .iter()
        .map(|j| match &j.work {
            Work::Set(patch) => single_leaf(patch),
            _ => None,
        })
        .collect();
    // Only when nothing between them can change what the later one is validated against
    // (`requires`, `asleep`): every request in between is a `get` or a single-leaf `set` of
    // the same leaf. Otherwise the later one may be refused after the earlier one was
    // answered `ok` without being sent.
    let same_leaf_or_get = |j: usize, leaf: &String| {
        matches!(jobs[j].work, Work::Get(_)) || leaves[j].as_ref() == Some(leaf)
    };
    let coalesced = (0..take)
        .map(|i| {
            leaves[i].as_ref().is_some_and(|leaf| {
                (i + 1..take).any(|k| {
                    jobs[k].conn == jobs[i].conn
                        && leaves[k].as_ref() == Some(leaf)
                        && (i + 1..k).all(|j| same_leaf_or_get(j, leaf))
                })
            })
        })
        .collect();
    Plan { take, coalesced }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn job(conn: ConnId, work: Work) -> Job {
        Job {
            id: RequestId::Int(conn as i64),
            out: None,
            conn,
            device: "sn:FAKEA0000001".into(),
            work,
            arrived: Instant::now(),
            stop: false,
        }
    }

    fn set(conn: ConnId, leaf: &str, v: i64) -> Job {
        job(conn, Work::Set(Patch::new().leaf(leaf, v)))
    }

    fn act(conn: ConnId, action: &str) -> Job {
        job(
            conn,
            Work::Do {
                action: action.into(),
                params: Params::new(),
            },
        )
    }

    fn stop(conn: ConnId) -> Job {
        Job {
            stop: true,
            ..act(conn, "gimbal.stop")
        }
    }

    fn names(jobs: &[Job]) -> Vec<String> {
        jobs.iter()
            .map(|j| match &j.work {
                Work::Get(_) => "get".to_string(),
                Work::Set(p) => format!("set {}", p.0.keys().next().unwrap()),
                Work::Do { action, .. } => action.clone(),
            })
            .collect()
    }

    fn single(p: &Patch) -> Option<String> {
        (p.0.len() == 1).then(|| p.0.keys().next().unwrap().clone())
    }

    #[test]
    fn requests_queue_in_order_and_a_stop_overtakes_them() {
        let q = DeviceQueue::new();
        q.push(set(1, "led.brightness", 1)).ok().unwrap();
        q.push(act(1, "preset.recall")).ok().unwrap();
        assert!(!q.cut.load(Ordering::SeqCst));
        q.push(stop(2)).ok().unwrap();
        assert!(
            q.cut.load(Ordering::SeqCst),
            "a group read in progress is cut"
        );
        q.push(Job {
            stop: true,
            ..act(3, "zoom.stop")
        })
        .ok()
        .unwrap();
        assert_eq!(
            names(&q.take_all()),
            [
                "gimbal.stop",
                "zoom.stop",
                "set led.brightness",
                "preset.recall"
            ]
        );
        assert!(
            !q.cut.load(Ordering::SeqCst),
            "taking the stops clears the cut"
        );
    }

    /// 4.1 `busy`: 32 requests are already queued for the device.
    #[test]
    fn the_33rd_queued_request_is_busy() {
        let q = DeviceQueue::new();
        for i in 0..MAX_QUEUED {
            assert!(q.push(set(1, "led.brightness", i as i64)).is_ok());
        }
        assert!(matches!(
            q.push(set(1, "led.brightness", 0)),
            Err(Refused::Busy(_))
        ));
        assert!(
            matches!(q.push(stop(1)), Err(Refused::Busy(_))),
            "a stop counts too"
        );
    }

    #[test]
    fn a_closed_queue_takes_nothing() {
        let q = DeviceQueue::new();
        q.push(set(1, "led.brightness", 1)).ok().unwrap();
        assert_eq!(q.close().len(), 1);
        assert!(matches!(
            q.push(set(1, "led.brightness", 1)),
            Err(Refused::Closed(_))
        ));
    }

    /// 4.1 rule 9: a request whose connection closed before the worker started it is dropped.
    #[test]
    fn a_closed_connection_drops_its_queued_requests() {
        let q = DeviceQueue::new();
        q.push(set(1, "led.brightness", 1)).ok().unwrap();
        q.push(set(2, "led.brightness", 2)).ok().unwrap();
        assert_eq!(q.close_conn(1, Instant::now()), None);
        assert_eq!(q.take_all().iter().map(|j| j.conn).collect::<Vec<_>>(), [2]);
    }

    /// 5.6: the connection that started a continuing action without `keep` closes: the
    /// model's stop is queued for it, ahead of the other requests.
    #[test]
    fn closing_the_turning_connection_queues_the_stop() {
        let q = DeviceQueue::new();
        let turn = Motion::Continuing {
            stop: "gimbal.stop",
        };
        q.record_act("gimbal.turn", turn, 5, false);
        q.push(set(6, "led.brightness", 1)).ok().unwrap();
        assert_eq!(
            q.close_conn(6, Instant::now()),
            None,
            "another connection: nothing"
        );
        q.push(set(7, "led.brightness", 1)).ok().unwrap();
        assert_eq!(
            q.close_conn(5, Instant::now()).as_deref(),
            Some("gimbal.stop")
        );
        let jobs = q.take_all();
        assert_eq!(
            names(&jobs),
            ["gimbal.stop", "set led.brightness"],
            "the stop goes first; connection 6's set was dropped when it closed"
        );
        assert_eq!(jobs[1].conn, 7);
        assert!(jobs[0].out.is_none() && jobs[0].conn == 0 && jobs[0].stop);
        assert_eq!(q.close_conn(5, Instant::now()), None, "only once");
    }

    /// 5.6 and 15.1 row 89: no stop for a `keep: true` turn; another motion clears the record;
    /// only the recorded stop clears it among the stops.
    #[test]
    fn the_record_follows_the_motion_marks() {
        let turn = Motion::Continuing {
            stop: "gimbal.stop",
        };
        let mut r = Record::default();
        r.after_act("gimbal.turn", turn, 5, true);
        assert_eq!(
            r.on_close(5),
            None,
            "keep: true is never stopped by the daemon"
        );
        r.after_act("gimbal.turn", turn, 5, false);
        r.after_act("preset.save", Motion::None, 6, false);
        r.after_act("zoom.stop", Motion::Stop, 6, false);
        assert_eq!(
            r.clone().on_close(5).as_deref(),
            Some("gimbal.stop"),
            "another stop leaves it"
        );
        r.after_act("preset.recall", Motion::Move, 6, false);
        assert_eq!(r.on_close(5), None, "a recall after a turn clears it");
        r.after_act("gimbal.turn", turn, 5, false);
        r.after_act("gimbal.stop", Motion::Stop, 6, false);
        assert_eq!(r.on_close(5), None, "its own stop clears it");
        r.after_act("gimbal.turn", turn, 5, false);
        r.after_act("gimbal.turn", turn, 6, false);
        assert_eq!(r.on_close(5), None, "a later turn replaces the record");
        assert_eq!(r.on_close(6).as_deref(), Some("gimbal.stop"));
    }

    #[test]
    fn requests_not_started_within_the_deadline_expire() {
        let q = DeviceQueue::new();
        let t = Instant::now();
        let mut old = set(1, "led.brightness", 1);
        old.arrived = t - Duration::from_secs(16);
        q.push(old).ok().unwrap();
        q.push(set(2, "led.brightness", 2)).ok().unwrap();
        let late = q.expire(t, Duration::from_secs(15));
        assert_eq!(late.iter().map(|j| j.conn).collect::<Vec<_>>(), [1]);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn put_back_keeps_order_behind_new_stops() {
        let q = DeviceQueue::new();
        q.push(set(1, "a.b", 1)).ok().unwrap();
        q.push(set(1, "c.d", 1)).ok().unwrap();
        let mut jobs = q.take_all();
        let rest = jobs.split_off(1);
        q.push(stop(2)).ok().unwrap();
        q.put_back(rest);
        assert_eq!(names(&q.take_all()), ["gimbal.stop", "set c.d"]);
    }

    #[test]
    fn a_batch_runs_up_to_a_stop_or_a_restart_and_those_run_alone() {
        let restarts =
            |j: &Job| matches!(&j.work, Work::Do { action, .. } if action == "factory_reset");
        let jobs = vec![
            set(1, "a.b", 1),
            act(1, "preset.recall"),
            act(1, "factory_reset"),
            set(1, "c.d", 1),
        ];
        assert_eq!(plan(&jobs, restarts, single).take, 2);
        assert_eq!(plan(&jobs[2..], restarts, single).take, 1);
        assert_eq!(plan(&[stop(1), set(1, "a.b", 1)], restarts, single).take, 1);
        assert_eq!(plan(&[], restarts, single).take, 0);
    }

    /// 4.1 rule 9: a single-leaf `set` is answered without being sent when a later queued `set`
    /// of the same connection has the same single leaf.
    #[test]
    fn single_leaf_sets_of_one_connection_coalesce() {
        let none = |_: &Job| false;
        let jobs = vec![
            set(1, "led.brightness", 1),
            set(2, "led.brightness", 2),
            set(1, "led.brightness", 3),
            job(
                1,
                Work::Set(
                    Patch::new()
                        .leaf("led.brightness", 1)
                        .leaf("zoom.factor", 2),
                ),
            ),
            set(1, "zoom.factor", 2),
        ];
        assert_eq!(
            plan(&jobs, none, single).coalesced,
            [true, false, false, false, false],
            "only an earlier single-leaf set of the same connection and leaf"
        );
        assert_eq!(
            plan(
                &[set(1, "led.brightness", 1), set(1, "led.brightness", 9)],
                none,
                |_| None
            )
            .coalesced,
            [false, false],
            "a later set whose value the setting cannot take does not replace an earlier one"
        );
        let _ = json!(null);
    }

    /// A write in between (here the mode `exposure.time` requires) can make the later request
    /// a `conflict`; the earlier one must then be sent, not answered `ok` unsent.
    #[test]
    fn a_write_in_between_keeps_both_sets() {
        let none = |_: &Job| false;
        let jobs = [
            set(1, "exposure.time", 1000),
            job(1, Work::Set(Patch::new().leaf("exposure.mode", "auto"))),
            set(1, "exposure.time", 2000),
        ];
        assert_eq!(plan(&jobs, none, single).coalesced, [false, false, false]);
        let jobs = [
            set(1, "exposure.time", 1000),
            job(2, Work::Get(None)),
            act(2, "preset.recall"),
            set(1, "exposure.time", 2000),
        ];
        assert_eq!(
            plan(&jobs, none, single).coalesced,
            [false, false, false, false],
            "an action in between keeps both"
        );
        let jobs = [
            set(1, "exposure.time", 1000),
            job(2, Work::Get(None)),
            set(1, "exposure.time", 2000),
        ];
        assert_eq!(
            plan(&jobs, none, single).coalesced,
            [true, false, false],
            "a get in between does not"
        );
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 queue::`
Expected: PASS: 11 tests.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy -p obsbot4linuxd --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/queue.rs
git commit -m "obsbot4linuxd: device queue with stops first, batch plan, coalescing, motion record"
```

### Task b-06 (key `b-06-table`): Device table

**Files:**

- Replace: `crates/obsbot4linuxd/src/table.rs` (the stub of b-01)
- Test: unit tests inside the file

**Interfaces:**

- Consumes: `crate::identity::{PortReason, choose_id}`, `crate::names`, `crate::outbound::{ConnId, Outbound, ok_line, error, error_line}`, `crate::queue::{DeviceQueue, Job, Work}` (all complete in b-01); `obsbot_proto::{Description, Device, Event, Hello, HelloOk, OkBody, RequestId, RevState, State, ErrorCode, values::same, validate::validate_get}`; `obsbot_device::contract::ModelEntry`.
- Produces: `table::{PlugId = u64, Cause::{Request, Poll}, Routed::{Unknown, Unsupported(RevState, Description), Unavailable(RevState, Description), Worker(Arc<DeviceQueue>)}, Routed::answer(&Job), volatile_topics(&Description, &State, &State) -> Option<BTreeSet<String>>, Table::{new(&str), startup(&[PlugId]), hello(&Outbound, &RequestId, &Hello, Instant, Duration) -> bool, set_live(ConnId, &[String]), remove_conn(ConnId), topics() -> BTreeSet<String>, route(&str) -> Routed, plug_unsupported(PlugId, Device), plug_hidden(PlugId, &'static ModelEntry, Device, Arc<DeviceQueue>), describe(PlugId, Description), announce(PlugId, Option<&str>, PortReason, Option<&str>, State) -> Option<String>, identity(PlugId, Option<&str>, Option<&str>) -> Option<String>, publish(PlugId, &State, Cause) -> u64, current(PlugId) -> Option<(u64, State)>, id_of(PlugId) -> Option<String>, unplug(PlugId), queues() -> Vec<Arc<DeviceQueue>>, devices() -> Vec<Device>}}`.

The table logs the lines of 5.11 itself (appeared, hand-over, went away), at the moment its events go out. The order of events it sends is the order of 4.1 Events and 5.5; its tests check them against the shapes of the golden `hotplug.jsonl` of W0.

- [ ] **Step 1: Write the failing tests**

Add the `#[cfg(test)] mod tests` block at the end of the file of Step 3 to the stub.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 table::`
Expected: FAIL: the tests panic with `not yet implemented: task b-06-table`.

- [ ] **Step 3: Write the code**

Replace `crates/obsbot4linuxd/src/table.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The device table (5.1): every entry's id, description, rev and last state, and the
//! connections that receive events, under one lock. Every change of an entry goes through
//! here, so the events of 4.1 leave in one order for every connection, and a `hello` snapshot
//! and the start of its event delivery are atomic (4.1 `hello` timing).
//!
//! An enumeration of a supported device is a plug (`PlugId`); its worker addresses the table
//! by it. A plug's entry stays hidden until the worker announces it after its first read
//! attempt (5.2), or until a `hello` reaches its deadline (4.1).

use crate::identity::{PortReason, choose_id};
use crate::names;
use crate::outbound::{ConnId, Outbound, ok_line};
use crate::outbound::{error, error_line};
use crate::queue::{DeviceQueue, Job, Work};
use obsbot_device::contract::ModelEntry;
use obsbot_proto::ErrorCode;
use obsbot_proto::validate::validate_get;
use obsbot_proto::values::same;
use obsbot_proto::{
    Description, Device, Event, Hello, HelloOk, OkBody, RequestId, RevState, State,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// One enumeration of a device, numbered by the registry from 1.
pub type PlugId = u64;

/// Who changed a state (4.1 rule 8): a request (its batch's re-read), or a periodic read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Request,
    Poll,
}

/// Where a reader sends a device request (4.1 rule 2).
pub enum Routed {
    /// No listed device has this id.
    Unknown,
    /// An unsupported device: `get` from its fixed state, `set` and `do` refused.
    Unsupported(RevState, Description),
    /// An unplugged `sn:` device: `get` from its published state, `set` and `do` refused.
    Unavailable(RevState, Description),
    /// A supported, available device: its queue.
    Worker(Arc<DeviceQueue>),
}

impl Routed {
    /// Answers a request the reader (or a worker that ended) answers at once (4.1 rule 2):
    /// `unknown_device`; for an unsupported or unplugged device a `get` from its state (after
    /// `validate_get`), a `set` or `do` with `unsupported_device` or `unavailable`.
    pub fn answer(&self, job: &Job) {
        let id = &job.device;
        let (rs, desc, refusal) = match self {
            Routed::Unknown => {
                return job.fail(ErrorCode::UnknownDevice, &format!("no device {id}"));
            }
            Routed::Worker(_) => {
                return job.fail(ErrorCode::Internal, "a queued request was answered at once");
            }
            Routed::Unsupported(rs, desc) => (
                rs,
                desc,
                (
                    ErrorCode::UnsupportedDevice,
                    format!("{id} is not supported by obsbot4linux"),
                ),
            ),
            Routed::Unavailable(rs, desc) => (
                rs,
                desc,
                (ErrorCode::Unavailable, format!("{id} is unplugged")),
            ),
        };
        match &job.work {
            Work::Get(paths) => {
                if let Some(paths) = paths
                    && let Err(r) = validate_get(desc, paths)
                {
                    return job.answer(error_line(Some(&job.id), error(r.code, &r.message)));
                }
                let state = match paths {
                    Some(p) => rs.state.select(p),
                    None => rs.state.clone(),
                };
                job.answer(ok_line(
                    &job.id,
                    OkBody::State(RevState { rev: rs.rev, state }),
                ));
            }
            Work::Set(_) | Work::Do { .. } => job.fail(refusal.0, &refusal.1),
        }
    }
}

enum Route {
    Unsupported,
    Worker(Arc<DeviceQueue>),
    Unplugged,
}

struct Entry {
    device: Device,
    rev: u64,
    state: State,
    announced: bool,
    plug: Option<PlugId>,
    model: Option<&'static ModelEntry>,
    route: Route,
}

struct Sub {
    out: Outbound,
    subscribe: bool,
    live: BTreeSet<String>,
}

struct Inner {
    entries: Vec<Entry>,
    subs: BTreeMap<ConnId, Sub>,
    startup: BTreeSet<PlugId>,
}

pub struct Table {
    inner: Mutex<Inner>,
    startup_done: Condvar,
    daemon: String,
}

/// The live topics of a state change that touches only volatile leaves (4.1 rule 8);
/// `None` when it touches anything else (a leaf without a live topic, `available`, `pending`
/// or `unread`).
pub fn volatile_topics(desc: &Description, old: &State, new: &State) -> Option<BTreeSet<String>> {
    if old.available != new.available || old.unread != new.unread {
        return None;
    }
    let pending_same = old.pending.len() == new.pending.len()
        && old
            .pending
            .iter()
            .all(|(k, v)| new.pending.get(k).is_some_and(|w| same(v, w)));
    if !pending_same {
        return None;
    }
    let mut topics = BTreeSet::new();
    for (path, setting) in &desc.settings {
        let a = old.at(path).unwrap_or(&Value::Null);
        let b = new.at(path).unwrap_or(&Value::Null);
        if !same(a, b) {
            topics.insert(setting.live.clone()?);
        }
    }
    Some(topics)
}

fn same_state(a: &State, b: &State) -> bool {
    a.available == b.available
        && a.unread == b.unread
        && same(
            &serde_json::to_value(&a.pending).unwrap_or(Value::Null),
            &serde_json::to_value(&b.pending).unwrap_or(Value::Null),
        )
        && same(
            &Value::Object(a.values.clone()),
            &Value::Object(b.values.clone()),
        )
}

fn line(event: &Event) -> String {
    serde_json::to_string(event).expect("an event always serialises")
}

impl Inner {
    fn listed(&self) -> Vec<Device> {
        self.entries
            .iter()
            .filter(|e| e.announced)
            .map(|e| e.device.clone())
            .collect()
    }

    fn by_plug(&self, plug: PlugId) -> Option<usize> {
        self.entries.iter().position(|e| e.plug == Some(plug))
    }

    fn by_id(&self, id: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.announced && e.device.id == id)
    }

    /// Sends one line to the subscribed connections; `only` limits it to connections whose
    /// live list holds one of these topics.
    fn send(&self, text: &str, only: Option<&BTreeSet<String>>) {
        for sub in self.subs.values().filter(|s| s.subscribe) {
            if let Some(topics) = only
                && sub.live.is_disjoint(topics)
            {
                continue;
            }
            sub.out.send(text.to_string());
        }
    }

    fn send_devices(&self) {
        let devices = self.listed();
        self.send(&line(&Event::Devices { devices }), None);
    }

    fn send_state(&self, i: usize, only: Option<&BTreeSet<String>>) {
        let e = &self.entries[i];
        let event = Event::State {
            device: e.device.id.clone(),
            rev: e.rev,
            state: e.state.clone(),
        };
        self.send(&line(&event), only);
    }

    /// Lists a hidden entry: `devices`, then its first `state` (4.1 Events).
    fn list(&mut self, i: usize) {
        self.entries[i].announced = true;
        self.entries[i].rev = 1;
        self.send_devices();
        self.send_state(i, None);
    }

    fn held(&self, id: &str, except: usize) -> bool {
        self.entries
            .iter()
            .enumerate()
            .any(|(k, e)| k != except && e.announced && e.device.available && e.device.id == id)
    }

    /// The hand-over of 5.5 from the listed `port:` entry `i` to `sn_id`: three events in a
    /// row, the rev above both entries, `pending` and `unread` carried over.
    fn hand_over(&mut self, i: usize, sn_id: &str, serial: &str) {
        let old_id = self.entries[i].device.id.clone();
        let taken = self
            .entries
            .iter()
            .position(|e| e.announced && e.device.id == sn_id);
        let rev = self.entries[i]
            .rev
            .max(taken.map_or(0, |t| self.entries[t].rev))
            + 1;
        let display = self.entries[i].model.map_or("", |m| m.display_name);
        let mut device = self.entries[i].device.clone();
        device.id = sn_id.to_string();
        device.name = names::name_of(display, sn_id, &device.usb.path);
        device.serial = Some(serial.to_string());
        device.available = true;
        device.replaced_by = None;
        let mut ghost = self.entries[i].device.clone();
        ghost.available = false;
        ghost.replaced_by = Some(sn_id.to_string());
        // Event 1: the sn: entry (available) and the old port: entry with replaced_by.
        let mut first: Vec<Device> = Vec::new();
        for (k, e) in self.entries.iter().enumerate().filter(|(_, e)| e.announced) {
            if Some(k) == taken {
                first.push(device.clone());
            } else if k == i {
                if taken.is_none() {
                    first.push(device.clone());
                }
                first.push(ghost.clone());
            } else {
                first.push(e.device.clone());
            }
        }
        self.send(&line(&Event::Devices { devices: first }), None);
        let entry = self.entries.remove(i);
        let target = match taken {
            Some(t) => {
                let t = if t > i { t - 1 } else { t };
                self.entries[t].plug = entry.plug;
                self.entries[t].route = entry.route;
                self.entries[t].model = entry.model;
                t
            }
            None => {
                self.entries.insert(
                    i,
                    Entry {
                        device: device.clone(),
                        rev: 0,
                        state: State::new(true),
                        announced: true,
                        plug: entry.plug,
                        model: entry.model,
                        route: entry.route,
                    },
                );
                i
            }
        };
        let mut state = entry.state;
        state.available = true;
        self.entries[target].device = device;
        self.entries[target].state = state;
        self.entries[target].rev = rev;
        self.send_devices();
        self.send_state(target, None);
        eprintln!("{}", names::log_handover(&old_id, sn_id));
    }
}

impl Table {
    /// An empty table; `daemon` is the version the `hello` reply names.
    pub fn new(daemon: &str) -> Table {
        Table {
            inner: Mutex::new(Inner {
                entries: Vec::new(),
                subs: BTreeMap::new(),
                startup: BTreeSet::new(),
            }),
            startup_done: Condvar::new(),
            daemon: daemon.to_string(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The plugs of the first enumeration whose first read attempt every `hello` waits for.
    pub fn startup(&self, plugs: &[PlugId]) {
        self.lock().startup.extend(plugs.iter().copied());
    }

    /// Answers a `hello` that arrived at `arrived` (4.1 `hello` timing): waits until the first
    /// read attempt of every device of the first enumeration has ended, at most until
    /// `arrived + deadline`; a device still in its first read is then listed with its leaves
    /// `null` and in `unread`. The reply and the connection's registration for events happen
    /// under one lock. Returns `false` when the reply could not be queued.
    pub fn hello(
        &self,
        out: &Outbound,
        id: &RequestId,
        hello: &Hello,
        arrived: Instant,
        deadline: std::time::Duration,
    ) -> bool {
        let until = arrived + deadline;
        let mut inner = self.lock();
        while !inner.startup.is_empty() {
            let now = Instant::now();
            if now >= until {
                break;
            }
            inner = self
                .startup_done
                .wait_timeout(inner, until - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        let late: Vec<PlugId> = std::mem::take(&mut inner.startup).into_iter().collect();
        for plug in late {
            if let Some(i) = inner.by_plug(plug)
                && !inner.entries[i].announced
            {
                inner.list(i);
                eprintln!(
                    "{}",
                    names::log_appeared(&inner.entries[i].device, Some(PortReason::NoSerial))
                );
            }
        }
        let devices = inner.listed();
        let states = inner
            .entries
            .iter()
            .filter(|e| e.announced)
            .map(|e| {
                (
                    e.device.id.clone(),
                    RevState {
                        rev: e.rev,
                        state: e.state.clone(),
                    },
                )
            })
            .collect();
        let reply = OkBody::Hello(HelloOk {
            protocol: obsbot_proto::PROTOCOL,
            daemon: self.daemon.clone(),
            devices,
            states,
        });
        out.set_client(&hello.client);
        let sent = out.send(ok_line(id, reply));
        inner.subs.insert(
            out.conn(),
            Sub {
                out: out.clone(),
                subscribe: hello.subscribe,
                live: hello.live.iter().cloned().collect(),
            },
        );
        sent
    }

    /// The `live` verb: replaces the connection's live list.
    pub fn set_live(&self, conn: ConnId, topics: &[String]) {
        if let Some(sub) = self.lock().subs.get_mut(&conn) {
            sub.live = topics.iter().cloned().collect();
        }
    }

    /// The connection closed.
    pub fn remove_conn(&self, conn: ConnId) {
        self.lock().subs.remove(&conn);
    }

    /// The live topics at least one subscribed connection holds (5.7 live tiers).
    pub fn topics(&self) -> BTreeSet<String> {
        self.lock()
            .subs
            .values()
            .filter(|s| s.subscribe)
            .flat_map(|s| s.live.iter().cloned())
            .collect()
    }

    /// Where a request for `id` goes (4.1 rule 2).
    pub fn route(&self, id: &str) -> Routed {
        let inner = self.lock();
        let Some(i) = inner.by_id(id) else {
            return Routed::Unknown;
        };
        let e = &inner.entries[i];
        let rs = RevState {
            rev: e.rev,
            state: e.state.clone(),
        };
        match &e.route {
            Route::Unsupported => Routed::Unsupported(rs, e.device.description.clone()),
            Route::Unplugged => Routed::Unavailable(rs, e.device.description.clone()),
            Route::Worker(q) => Routed::Worker(q.clone()),
        }
    }

    /// An unsupported device appeared (5.4): listed at once with its fixed state, rev 1.
    pub fn plug_unsupported(&self, plug: PlugId, device: Device) {
        let mut inner = self.lock();
        eprintln!("{}", names::log_appeared(&device, None));
        inner.entries.push(Entry {
            device,
            rev: 1,
            state: State::new(true),
            announced: false,
            plug: Some(plug),
            model: None,
            route: Route::Unsupported,
        });
        let i = inner.entries.len() - 1;
        inner.list(i);
    }

    /// A supported device appeared: a hidden entry under its `port:` id until its worker
    /// announces it.
    pub fn plug_hidden(
        &self,
        plug: PlugId,
        model: &'static ModelEntry,
        device: Device,
        queue: Arc<DeviceQueue>,
    ) {
        let state = names::initial_state(&device.description);
        self.lock().entries.push(Entry {
            device,
            rev: 0,
            state,
            announced: false,
            plug: Some(plug),
            model: Some(model),
            route: Route::Worker(queue),
        });
    }

    /// The worker opened the camera: the hidden entry takes the model's description, and its
    /// state every described leaf, `null` and in `unread` (what a `hello` at its deadline
    /// shows, 4.1).
    pub fn describe(&self, plug: PlugId, description: Description) {
        let mut inner = self.lock();
        if let Some(i) = inner.by_plug(plug) {
            inner.entries[i].state = names::initial_state(&description);
            inner.entries[i].device.description = description;
        }
    }

    /// The worker's first read attempt ended (5.2): the entry is listed under `sn:<serial>`
    /// (taking an unplugged entry with that id, 5.3) or under its `port:` id. An entry a
    /// `hello` already listed at its deadline is handed over instead. Returns the id.
    pub fn announce(
        &self,
        plug: PlugId,
        serial: Option<&str>,
        reason: PortReason,
        firmware: Option<&str>,
        state: State,
    ) -> Option<String> {
        let mut inner = self.lock();
        let i = inner.by_plug(plug)?;
        let path = inner.entries[i].device.usb.path.clone();
        let (id, why) = choose_id(serial, reason, &path, |id| inner.held(id, i));
        let display = inner.entries[i].model.map_or("", |m| m.display_name);
        if inner.entries[i].announced {
            inner.entries[i].state = State {
                available: true,
                ..state
            };
            if firmware.is_some() && inner.entries[i].device.firmware.is_none() {
                inner.entries[i].device.firmware = firmware.map(String::from);
            }
            if id.starts_with("sn:") && inner.entries[i].device.id.starts_with("port:") {
                inner.hand_over(i, &id, serial.unwrap_or_default());
            } else {
                inner.entries[i].rev += 1;
                inner.send_devices();
                inner.send_state(i, None);
            }
        } else {
            {
                let e = &mut inner.entries[i];
                e.device.id = id.clone();
                e.device.name = names::name_of(display, &id, &path);
                e.device.serial = if why.is_none() {
                    serial.map(String::from)
                } else {
                    None
                };
                e.device.firmware = firmware.map(String::from);
                e.state = State {
                    available: true,
                    ..state
                };
            }
            let taken = inner
                .entries
                .iter()
                .position(|e| e.announced && !e.device.available && e.device.id == id);
            eprintln!("{}", names::log_appeared(&inner.entries[i].device, why));
            match taken {
                Some(t) => {
                    let new = inner.entries.remove(i);
                    let t = if t > i { t - 1 } else { t };
                    let e = &mut inner.entries[t];
                    e.rev += 1;
                    e.device = new.device;
                    e.state = new.state;
                    e.plug = new.plug;
                    e.model = new.model;
                    e.route = new.route;
                    inner.send_devices();
                    inner.send_state(t, None);
                }
                None => inner.list(i),
            }
        }
        inner.startup.remove(&plug);
        drop(inner);
        self.startup_done.notify_all();
        Some(id)
    }

    /// A later identity attempt answered (5.5): a valid serial that no available device holds
    /// hands a `port:` entry over to `sn:`; a firmware answered for the first time changes the
    /// entry (a `devices` event). Returns the new id after a hand-over.
    pub fn identity(
        &self,
        plug: PlugId,
        serial: Option<&str>,
        firmware: Option<&str>,
    ) -> Option<String> {
        let mut inner = self.lock();
        let i = inner.by_plug(plug)?;
        if !inner.entries[i].announced {
            return None;
        }
        let mut changed = false;
        if let Some(fw) = firmware
            && inner.entries[i].device.firmware.is_none()
        {
            inner.entries[i].device.firmware = Some(fw.to_string());
            changed = true;
        }
        if let Some(s) = serial
            && inner.entries[i].device.id.starts_with("port:")
        {
            let path = inner.entries[i].device.usb.path.clone();
            let (id, why) = choose_id(Some(s), PortReason::NoSerial, &path, |id| inner.held(id, i));
            if why.is_none() {
                inner.hand_over(i, &id, s);
                return Some(id);
            }
        }
        if changed {
            inner.send_devices();
        }
        None
    }

    /// Publishes a worker's view of its device: values, `pending` and `unread`. A change gets
    /// a new rev and a `state` event; a change caused by no request that touches only volatile
    /// leaves goes only to the connections that subscribe one of their live topics (4.1 rule
    /// 8). An unchanged state keeps its rev and sends nothing. Returns the rev that reflects
    /// the state. A plug that no longer serves its entry (unplugged) changes nothing.
    pub fn publish(&self, plug: PlugId, state: &State, cause: Cause) -> u64 {
        let mut inner = self.lock();
        let Some(i) = inner.by_plug(plug) else {
            return 0;
        };
        let new = State {
            available: inner.entries[i].state.available,
            ..state.clone()
        };
        if !inner.entries[i].announced {
            inner.entries[i].state = new;
            return inner.entries[i].rev;
        }
        if same_state(&inner.entries[i].state, &new) {
            return inner.entries[i].rev;
        }
        let only = match cause {
            Cause::Request => None,
            Cause::Poll => volatile_topics(
                &inner.entries[i].device.description,
                &inner.entries[i].state,
                &new,
            ),
        };
        inner.entries[i].state = new;
        inner.entries[i].rev += 1;
        inner.send_state(i, only.as_ref());
        inner.entries[i].rev
    }

    /// The rev and the published state of a plug's entry.
    pub fn current(&self, plug: PlugId) -> Option<(u64, State)> {
        let inner = self.lock();
        let i = inner.by_plug(plug)?;
        Some((inner.entries[i].rev, inner.entries[i].state.clone()))
    }

    /// The id a plug's entry is listed under now.
    pub fn id_of(&self, plug: PlugId) -> Option<String> {
        let inner = self.lock();
        inner
            .by_plug(plug)
            .map(|i| inner.entries[i].device.id.clone())
    }

    /// The enumeration went away (5.3): an unsupported or `port:` entry is removed (one
    /// `devices` event); an `sn:` entry stays listed with `available: false`, no video node,
    /// `pending` emptied and its `readback: false` leaves `null` (4.2), then one `state`
    /// event. A hidden entry goes without an event.
    pub fn unplug(&self, plug: PlugId) {
        let mut inner = self.lock();
        let Some(i) = inner.by_plug(plug) else {
            return;
        };
        inner.startup.remove(&plug);
        if !inner.entries[i].announced {
            inner.entries.remove(i);
            drop(inner);
            self.startup_done.notify_all();
            return;
        }
        let id = inner.entries[i].device.id.clone();
        eprintln!("{}", names::log_gone(&id));
        if id.starts_with("sn:") {
            let e = &mut inner.entries[i];
            e.plug = None;
            e.route = Route::Unplugged;
            e.device.available = false;
            e.device.video_node = None;
            e.state.available = false;
            e.state.pending.clear();
            let blind: Vec<String> = e
                .device
                .description
                .settings
                .iter()
                .filter(|(_, s)| !s.readback)
                .map(|(p, _)| p.clone())
                .collect();
            for path in blind {
                e.state.set(&path, Value::Null);
            }
            e.rev += 1;
            inner.send_devices();
            inner.send_state(i, None);
        } else {
            inner.entries.remove(i);
            inner.send_devices();
        }
    }

    /// The queues of every available supported device: for the deadline sweeper, and for a
    /// closing connection (its queued requests are dropped, 4.1 rule 9; its continuing action
    /// stopped, 5.6).
    pub fn queues(&self) -> Vec<Arc<DeviceQueue>> {
        self.lock()
            .entries
            .iter()
            .filter_map(|e| match &e.route {
                Route::Worker(q) => Some(q.clone()),
                _ => None,
            })
            .collect()
    }

    /// The listed devices, for tests and logs.
    pub fn devices(&self) -> Vec<Device> {
        self.lock().listed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::contract::{Bus, Found};
    use obsbot_device::models::fake::{self, FakeBus, KEYS};
    use obsbot_proto::{Patch, ServerMessage};
    use serde_json::json;
    use std::sync::mpsc::Receiver;
    use std::time::Duration;

    fn found() -> Vec<Found> {
        FakeBus::new(&KEYS).unwrap().scan()
    }

    fn conn(n: ConnId) -> (Outbound, Receiver<String>) {
        Outbound::new(n, None, 64)
    }

    fn drain(rx: &Receiver<String>) -> Vec<ServerMessage> {
        rx.try_iter()
            .map(|l| serde_json::from_str(&l).unwrap())
            .collect()
    }

    fn kinds(msgs: &[ServerMessage]) -> Vec<String> {
        msgs.iter()
            .map(|m| match m {
                ServerMessage::Event(Event::Devices { devices }) => format!(
                    "devices[{}]",
                    devices
                        .iter()
                        .map(|d| {
                            let mut s = d.id.clone();
                            if !d.available {
                                s.push('!');
                            }
                            if let Some(r) = &d.replaced_by {
                                s.push_str(&format!(">{r}"));
                            }
                            s
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                ServerMessage::Event(Event::State { device, rev, state }) => {
                    format!(
                        "state {device} {rev}{}",
                        if state.available { "" } else { " off" }
                    )
                }
                ServerMessage::Reply(_) => "reply".into(),
                _ => "other".into(),
            })
            .collect()
    }

    fn subscribed(t: &Table, n: ConnId, live: &[&str]) -> Receiver<String> {
        let (out, rx) = conn(n);
        assert!(t.hello(
            &out,
            &RequestId::Int(1),
            &Hello::new("test 1.0").subscribe(true).live(live),
            Instant::now(),
            Duration::from_secs(15)
        ));
        drain(&rx);
        std::mem::forget(out);
        rx
    }

    fn supported(t: &Table, plug: PlugId, f: &Found) -> Arc<DeviceQueue> {
        let q = DeviceQueue::new();
        let model = f.model.unwrap();
        let desc = fake::description(model.key).unwrap();
        let device = names::supported_device(
            f,
            model,
            desc,
            &obsbot_proto::port_id(&f.usb.path),
            None,
            None,
        );
        t.plug_hidden(plug, model, device, q.clone());
        q
    }

    fn some_state(v: i64) -> State {
        let mut s = State::new(true);
        s.set("led.brightness", json!(v));
        s
    }

    /// 4.1 Events: an added entry is a `devices` event, then its `state` at rev 1; an
    /// unsupported entry is removed on unplug with one `devices` event and no state.
    #[test]
    fn an_unsupported_device_comes_and_goes() {
        let t = Table::new("1.0.0");
        let rx = subscribed(&t, 1, &[]);
        let f = found();
        t.plug_unsupported(3, names::unsupported_device(&f[2]));
        assert_eq!(
            kinds(&drain(&rx)),
            ["devices[port:9-3]", "state port:9-3 1"]
        );
        assert!(matches!(t.route("port:9-3"), Routed::Unsupported(rs, _) if rs.rev == 1));
        t.unplug(3);
        assert_eq!(kinds(&drain(&rx)), ["devices[]"]);
        assert!(matches!(t.route("port:9-3"), Routed::Unknown));
    }

    /// 5.2: a supported device is listed only after its first read attempt.
    #[test]
    fn a_supported_device_is_listed_when_announced() {
        let t = Table::new("1.0.0");
        let rx = subscribed(&t, 1, &[]);
        let f = found();
        supported(&t, 1, &f[0]);
        assert!(drain(&rx).is_empty(), "hidden until its first read ended");
        assert!(matches!(t.route("port:9-1"), Routed::Unknown));
        let id = t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            Some("1.0.0.0"),
            some_state(1),
        );
        assert_eq!(id.as_deref(), Some("sn:FAKEA0000001"));
        assert_eq!(
            kinds(&drain(&rx)),
            ["devices[sn:FAKEA0000001]", "state sn:FAKEA0000001 1"]
        );
        let d = &t.devices()[0];
        assert_eq!(
            (d.name.as_str(), d.firmware.as_deref()),
            ("Fake A 0001", Some("1.0.0.0"))
        );
        assert!(matches!(t.route("sn:FAKEA0000001"), Routed::Worker(_)));
    }

    /// 4.1 Events and 4.2: an unplugged sn: entry stays listed, available false, no video
    /// node, pending emptied, readback:false leaves null; the re-plugged device takes the entry
    /// back once its identity is read: devices, then state, its rev going on.
    #[test]
    fn an_sn_device_is_kept_while_unplugged_and_taken_back_on_replug() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        let mut s = some_state(1);
        s.set("led.blink", json!(true));
        s.pending.insert("led.brightness".into(), json!(2));
        t.announce(1, Some("FAKEA0000001"), PortReason::NoSerial, None, s);
        let rx = subscribed(&t, 1, &[]);
        t.unplug(1);
        let msgs = drain(&rx);
        assert_eq!(
            kinds(&msgs),
            ["devices[sn:FAKEA0000001!]", "state sn:FAKEA0000001 2 off"]
        );
        let ServerMessage::Event(Event::State { state, .. }) = &msgs[1] else {
            panic!()
        };
        assert!(state.pending.is_empty());
        assert_eq!(
            state.at("led.blink"),
            Some(&Value::Null),
            "readback:false: null again"
        );
        assert_eq!(
            state.at("led.brightness"),
            Some(&json!(1)),
            "the last known values"
        );
        assert_eq!(t.devices()[0].video_node, None);
        assert!(matches!(t.route("sn:FAKEA0000001"), Routed::Unavailable(rs, _) if rs.rev == 2));
        t.publish(1, &some_state(9), Cause::Poll);
        assert!(
            drain(&rx).is_empty(),
            "the old plug publishes nothing any more"
        );

        supported(&t, 2, &f[0]);
        t.announce(
            2,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        assert_eq!(
            kinds(&drain(&rx)),
            ["devices[sn:FAKEA0000001]", "state sn:FAKEA0000001 3"]
        );
        assert_eq!(t.devices().len(), 1, "one entry");
        assert!(matches!(t.route("sn:FAKEA0000001"), Routed::Worker(_)));
    }

    /// 5.5 and the golden `hotplug.jsonl`: the hand-over is three events in a row; the old
    /// port: entry first shows replaced_by, then is gone; the rev is above both entries'.
    #[test]
    fn the_hand_over_is_three_events_and_takes_the_unplugged_entry() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        for v in 2..=8 {
            t.publish(1, &some_state(v), Cause::Request);
        }
        t.unplug(1);
        supported(&t, 2, &f[0]);
        let rx = subscribed(&t, 1, &[]);
        t.announce(2, None, PortReason::NoSerial, None, some_state(2));
        assert_eq!(
            kinds(&drain(&rx)),
            ["devices[sn:FAKEA0000001!,port:9-1]", "state port:9-1 1"]
        );
        let mut s = some_state(2);
        s.pending.insert("led.brightness".into(), json!(3));
        t.publish(2, &s, Cause::Request);
        drain(&rx);
        assert_eq!(
            t.identity(2, Some("FAKEA0000001"), Some("1.0.0.0"))
                .as_deref(),
            Some("sn:FAKEA0000001")
        );
        let msgs = drain(&rx);
        assert_eq!(
            kinds(&msgs),
            [
                "devices[sn:FAKEA0000001,port:9-1!>sn:FAKEA0000001]",
                "devices[sn:FAKEA0000001]",
                "state sn:FAKEA0000001 10"
            ],
            "rev = max(port rev 2, sn rev 9) + 1"
        );
        let ServerMessage::Event(Event::State { state, .. }) = &msgs[2] else {
            panic!()
        };
        assert_eq!(
            state.pending.get("led.brightness"),
            Some(&json!(3)),
            "pending carries over"
        );
        assert!(
            matches!(t.route("port:9-1"), Routed::Unknown),
            "the old id is gone"
        );
        assert!(matches!(t.route("sn:FAKEA0000001"), Routed::Worker(_)));
        assert_eq!(t.devices()[0].firmware.as_deref(), Some("1.0.0.0"));
        assert_eq!(
            t.publish(2, &some_state(4), Cause::Request),
            11,
            "the plug serves the sn: entry"
        );
    }

    #[test]
    fn a_hand_over_without_an_old_entry_renames_in_place() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(1, None, PortReason::NoSerial, None, some_state(1));
        let rx = subscribed(&t, 1, &[]);
        t.identity(1, Some("FAKEA0000001"), None);
        assert_eq!(
            kinds(&drain(&rx)),
            [
                "devices[sn:FAKEA0000001,port:9-1!>sn:FAKEA0000001]",
                "devices[sn:FAKEA0000001]",
                "state sn:FAKEA0000001 2"
            ]
        );
    }

    /// 5.5 duplicates: a serial an available device holds keeps the newcomer on port:.
    #[test]
    fn a_duplicate_serial_stays_on_its_port_id() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        supported(&t, 2, &f[1]);
        let id = t.announce(
            2,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        assert_eq!(id.as_deref(), Some("port:9-2"));
        assert_eq!(t.devices()[1].serial, None);
        assert_eq!(
            t.identity(2, Some("FAKEA0000001"), None),
            None,
            "still held: no hand-over"
        );
    }

    /// 4.1 rule 2: the reader's answers for unknown, unsupported and unplugged devices.
    #[test]
    fn requests_that_need_no_worker_are_answered_at_once() {
        use crate::queue::{Job, Work};
        let t = Table::new("1.0.0");
        let f = found();
        t.plug_unsupported(3, names::unsupported_device(&f[2]));
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        t.unplug(1);
        let (out, rx) = conn(1);
        let ask = |device: &str, work: Work| {
            let job = Job {
                id: RequestId::Int(5),
                out: Some(out.clone()),
                conn: 1,
                device: device.into(),
                work,
                arrived: Instant::now(),
                stop: false,
            };
            t.route(device).answer(&job);
            let line: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
            line
        };
        let set = || Work::Set(Patch::new().leaf("led.brightness", 1));
        let code = |v: Value| v["error"]["code"].clone();
        assert_eq!(code(ask("sn:NOPE00000000", set())), json!("unknown_device"));
        assert_eq!(code(ask("port:9-3", set())), json!("unsupported_device"));
        assert_eq!(
            ask("port:9-3", Work::Get(None))["ok"],
            json!({"rev": 1, "state": {"available": true, "pending": {}, "unread": []}})
        );
        assert_eq!(
            code(ask("port:9-3", Work::Get(Some(vec!["led".into()])))),
            json!("unsupported")
        );
        assert_eq!(code(ask("sn:FAKEA0000001", set())), json!("unavailable"));
        let got = ask("sn:FAKEA0000001", Work::Get(Some(vec!["led".into()])));
        assert_eq!(got["ok"]["rev"], 2);
        assert_eq!(got["ok"]["state"]["available"], false);
        assert_eq!(
            got["ok"]["state"]["led"],
            json!({"blink": null, "brightness": 1}),
            "readback:false is null again after unplug"
        );
        assert_eq!(
            code(ask(
                "sn:FAKEA0000001",
                Work::Do {
                    action: "sleep".into(),
                    params: Default::default()
                }
            )),
            json!("unavailable")
        );
    }

    /// A described hidden entry shows every leaf null and unread at a `hello` deadline.
    #[test]
    fn describe_gives_the_hidden_entry_its_leaves() {
        let t = Table::new("1.0.0");
        let f = found();
        let model = f[0].model.unwrap();
        let device =
            names::supported_device(&f[0], model, Description::default(), "port:9-1", None, None);
        t.plug_hidden(1, model, device, DeviceQueue::new());
        t.describe(1, fake::description("fake-a").unwrap());
        t.startup(&[1]);
        let (out, rx) = conn(1);
        t.hello(
            &out,
            &RequestId::Int(1),
            &Hello::new("x 1"),
            Instant::now(),
            Duration::ZERO,
        );
        let msgs = drain(&rx);
        let ServerMessage::Reply(r) = &msgs[0] else {
            panic!()
        };
        let obsbot_proto::ReplyBody::Ok(OkBody::Hello(h)) = &r.body else {
            panic!()
        };
        let s = &h.states["port:9-1"].state;
        assert_eq!(s.at("led.brightness"), Some(&Value::Null));
        assert!(s.unread.contains(&"led.brightness".to_string()));
        assert_eq!(
            h.devices[0].description.settings.len(),
            fake::description("fake-a").unwrap().settings.len()
        );
    }

    /// 4.1 rules 1 and 3: an unchanged state keeps its rev and sends nothing.
    #[test]
    fn only_a_change_gets_a_rev_and_an_event() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        let rx = subscribed(&t, 1, &[]);
        assert_eq!(t.publish(1, &some_state(1), Cause::Poll), 1);
        let mut float = State::new(true);
        float.set("led.brightness", json!(1.0));
        assert_eq!(
            t.publish(1, &float, Cause::Poll),
            1,
            "1 and 1.0 are the same value"
        );
        assert!(drain(&rx).is_empty());
        assert_eq!(t.publish(1, &some_state(2), Cause::Poll), 2);
        assert_eq!(kinds(&drain(&rx)), ["state sn:FAKEA0000001 2"]);
    }

    /// 4.1 rule 8: a change of volatile leaves only, caused by no request, goes only to the
    /// connections subscribing one of their topics; a request's change goes to everyone.
    #[test]
    fn volatile_changes_reach_only_their_live_subscribers() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        let mut s = State::new(true);
        s.set("gimbal.pan", json!(0));
        s.set("led.brightness", json!(1));
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            s.clone(),
        );
        let pose = subscribed(&t, 1, &["pose"]);
        let none = subscribed(&t, 2, &[]);
        let (quiet, quiet_rx) = conn(3);
        t.hello(
            &quiet,
            &RequestId::Int(1),
            &Hello::new("cli 1.0"),
            Instant::now(),
            Duration::from_secs(1),
        );
        drain(&quiet_rx);
        assert_eq!(
            t.topics(),
            ["pose".to_string()].into(),
            "subscribe:false connections do not count"
        );
        s.set("gimbal.pan", json!(20));
        t.publish(1, &s, Cause::Poll);
        assert_eq!(kinds(&drain(&pose)), ["state sn:FAKEA0000001 2"]);
        assert!(drain(&none).is_empty());
        s.set("led.brightness", json!(2));
        t.publish(1, &s, Cause::Poll);
        assert_eq!(
            drain(&none).len(),
            1,
            "a non-volatile change reaches everyone, with the new pose"
        );
        s.set("gimbal.pan", json!(30));
        t.publish(1, &s, Cause::Request);
        assert_eq!(drain(&none).len(), 1, "caused by a request: everyone");
        assert!(drain(&quiet_rx).is_empty(), "no events without subscribe");
        t.set_live(2, &["pose".to_string()]);
        s.set("gimbal.pan", json!(40));
        t.publish(1, &s, Cause::Poll);
        assert_eq!(drain(&none).len(), 1, "after live [pose]");
    }

    #[test]
    fn volatile_topics_treat_pending_unread_and_available_as_changes() {
        let desc = fake::description("fake-a").unwrap();
        let a = State::new(true);
        let mut b = a.clone();
        b.set("gimbal.pan", json!(1));
        assert_eq!(
            volatile_topics(&desc, &a, &b),
            Some(["pose".to_string()].into())
        );
        b.unread.push("led.brightness".into());
        assert_eq!(volatile_topics(&desc, &a, &b), None);
        let mut c = a.clone();
        c.pending.insert("led.brightness".into(), json!(1));
        assert_eq!(volatile_topics(&desc, &a, &c), None);
        let _ = Patch::new();
    }

    /// 4.1 `hello` timing: the reply waits for the first read attempts of the first
    /// enumeration, at most until its deadline; a device still reading is then listed with its
    /// leaves null and unread; its first read later publishes a new state.
    #[test]
    fn hello_waits_for_the_first_enumeration_until_its_deadline() {
        let t = Arc::new(Table::new("1.0.0"));
        let f = found();
        supported(&t, 1, &f[0]);
        supported(&t, 2, &f[1]);
        t.startup(&[1, 2]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        let (out, rx) = conn(1);
        let t0 = Instant::now();
        let hello = Hello::new("test 1.0").subscribe(true);
        assert!(t.hello(
            &out,
            &RequestId::Int(1),
            &hello,
            t0,
            Duration::from_millis(200)
        ));
        assert!(t0.elapsed() >= Duration::from_millis(200));
        let msgs = drain(&rx);
        let ServerMessage::Reply(r) = &msgs[0] else {
            panic!()
        };
        let obsbot_proto::ReplyBody::Ok(OkBody::Hello(h)) = &r.body else {
            panic!()
        };
        assert_eq!(
            h.devices.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
            ["sn:FAKEA0000001", "port:9-2"]
        );
        let late = &h.states["port:9-2"];
        assert_eq!(late.rev, 1);
        assert_eq!(late.state.at("led.brightness"), Some(&Value::Null));
        assert!(late.state.unread.contains(&"led.brightness".to_string()));
        t.announce(
            2,
            Some("FAKEB0000001"),
            PortReason::NoSerial,
            None,
            some_state(3),
        );
        assert_eq!(
            kinds(&drain(&rx)),
            [
                "devices[sn:FAKEA0000001,sn:FAKEB0000001,port:9-2!>sn:FAKEB0000001]",
                "devices[sn:FAKEA0000001,sn:FAKEB0000001]",
                "state sn:FAKEB0000001 2"
            ]
        );
        let (out2, rx2) = conn(2);
        let t1 = Instant::now();
        t.hello(
            &out2,
            &RequestId::Int(1),
            &hello,
            t1,
            Duration::from_secs(15),
        );
        assert!(
            t1.elapsed() < Duration::from_millis(100),
            "later hellos do not wait"
        );
        assert_eq!(drain(&rx2).len(), 1);
    }

    #[test]
    fn hello_returns_once_the_first_reads_end() {
        let t = Arc::new(Table::new("1.0.0"));
        let f = found();
        supported(&t, 1, &f[0]);
        t.startup(&[1]);
        let t2 = t.clone();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            t2.announce(
                1,
                Some("FAKEA0000001"),
                PortReason::NoSerial,
                None,
                some_state(1),
            );
        });
        let (out, rx) = conn(1);
        let t0 = Instant::now();
        t.hello(
            &out,
            &RequestId::Int(1),
            &Hello::new("x 1"),
            t0,
            Duration::from_secs(15),
        );
        assert!(t0.elapsed() < Duration::from_secs(2));
        h.join().unwrap();
        let msgs = drain(&rx);
        let ServerMessage::Reply(r) = &msgs[0] else {
            panic!()
        };
        let obsbot_proto::ReplyBody::Ok(OkBody::Hello(h)) = &r.body else {
            panic!()
        };
        assert_eq!(
            h.states["sn:FAKEA0000001"].state.at("led.brightness"),
            Some(&json!(1))
        );
        assert_eq!(h.daemon, "1.0.0");
        assert_eq!(h.protocol, 2);
    }

    /// 5.1: a slow subscriber is closed when its queue is full; the others keep receiving.
    #[test]
    fn a_full_subscriber_is_closed_and_the_rest_go_on() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(0),
        );
        let (slow, _slow_rx) = Outbound::new(9, None, 3);
        t.hello(
            &slow,
            &RequestId::Int(1),
            &Hello::new("slow 1").subscribe(true),
            Instant::now(),
            Duration::from_secs(1),
        );
        let fast = subscribed(&t, 2, &[]);
        for v in 1..=10 {
            t.publish(1, &some_state(v), Cause::Request);
        }
        assert!(slow.is_closed());
        assert_eq!(drain(&fast).len(), 10);
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 table::`
Expected: PASS: 14 tests.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy -p obsbot4linuxd --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/table.rs
git commit -m "obsbot4linuxd: device table with events, hello snapshot, hand-over and live filtering"
```

### Task b-07 (key `b-07-worker`): Plain worker and write path

**Files:**

- Modify: `crates/obsbot4linuxd/src/worker.rs` (replace the `run` stub; keep `Plugged` and `spawn` as b-01 wrote them)
- Replace: `crates/obsbot4linuxd/src/batch.rs`
- Test: unit tests inside `worker.rs` (the module below)

**Interfaces:**

- Consumes: `crate::pending::{Pending, sent, echo}`, `crate::schedule::{Schedule, PERIODIC}` (b-03); `crate::identity::Series` (b-04); `crate::queue::{DeviceQueue, Job, Work, plan, Plan}` (b-05); `crate::table::{Table, Cause, PlugId}` (b-06); `crate::names::{initial_state, log_open_failed, log_raw, log_refused_frame}`, `crate::outbound::{ok_line, error, error_line}` (b-01); `obsbot_proto::validate::{validate_set, validate_do, validate_get, leaves, check_value, Fit}`; `trait Camera`, `trait Bus`, `Ctx`, `Target`, `Motion`, `CameraError`.
- Produces: `worker::run(Plugged)` (the thread body; `spawn` from b-01 calls it) and, private to the crate:

```rust
/// One worker's view of its device. `state` holds the values and `unread`; `pending` is kept
/// in `pending` and copied into the published state by `view`.
pub(crate) struct Worker {
    pub plug: PlugId,
    pub cam: Box<dyn Camera>,
    pub desc: Description,
    pub plan: Vec<Group>,
    /// `Camera::motion` of every action of the description, asked once at open.
    pub motion: BTreeMap<String, Motion>,
    pub model: &'static ModelEntry,
    pub table: Arc<Table>,
    pub queue: Arc<DeviceQueue>,
    pub timing: Timing,
    pub state: State,
    pub pending: Pending,
    pub schedule: Schedule,
    pub series: Series,
    /// After a frame with a `restart` warning: nothing is sent until then (5.6).
    pub quiet_until: Option<Instant>,
    /// `(leaf, raw value)` pairs already logged (4.2).
    pub raw_logged: BTreeSet<(String, String)>,
}
```

with `impl Worker` methods in `worker.rs`: `fn view(&self) -> State`, `fn publish(&mut self, cause: Cause) -> u64`, `fn failed(&self, error: &CameraError)`, `fn read_group(&mut self, name: &str, ctx: &Ctx) -> bool`, `fn first_read(&mut self)`, `fn tick(&mut self, tier: Tier)`, `fn attempt(&mut self, need: Need)`, `fn end(&mut self)`, `fn serve(&mut self, jobs: Vec<Job>)`; and in `batch.rs`: `pub(crate) fn batch(&mut self, jobs: Vec<Job>, coalesced: Vec<bool>)`. The struct is written whole here; the fields `motion` and `quiet_until` are used from b-13 and b-12 on (the skeleton's crate-wide `allow` stays until b-14).

Everything model-specific comes from `self.cam`; the worker never names a path, group or action of a model. The only names it knows are protocol names: the leaf `asleep` (4.2), the parameter `keep` (4.4, 5.6, decision 14).

**This task is the plain worker (B-1).** Each algorithm below is the whole worker of track B; the steps marked **[b-11]**, **[b-12]**, **[b-13]** or **[b-14]** belong to that later task, which writes them together with the named tests that pin them (decision 24). Build nothing of a marked step here; where a marked step replaces a plain one, the plain one is given next to it:

- **[b-11]** coalescing: `serve` passes `single_leaf = |_| None`; `batch` has no `Coalesced` outcome.
- **[b-12]** the restart quiet time and the wake read: `serve` passes `restarts = |_| false`; there is no `quiet_until` step, no `restart` flag, no `wake()`; after a wake the groups are read at their next tick.
- **[b-13]** the continuing-motion record and the daemon's own stops at shutdown: `batch` does not call `queue.record_act`; at shutdown every job is dropped unsent.
- **[b-14]** identity attempts 2 and 3 and the wake series: only the attempt at plug-in is made.

The contexts it gives the camera (W0 decision 4, spec 4.1 rule 9, 5.6):

| Operation                                | `Ctx`                                                                                     |
| ---------------------------------------- | ----------------------------------------------------------------------------------------- |
| identity attempt                         | `Ctx::new(None, Some(queue.halt.clone()))`                                                |
| poll, first read, wake read              | `Ctx::new(None, Some(queue.cut.clone()))`                                                 |
| `apply`, `act` of a request              | `Ctx::new(Some(job.arrived + timing.request_deadline), Some(queue.ended.clone()))`        |
| touched re-read of a batch               | `Ctx::new(Some(earliest deadline of the batch's sent requests), Some(queue.cut.clone()))` |
| the daemon's own stop at shutdown [b-13] | `Ctx::new(Some(job.arrived + timing.request_deadline), None)`                             |

(`DeviceQueue::end` sets `ended`, `halt` and `cut`; `shutdown` sets `halt` and `cut`; a queued stop sets `cut`; `take_all` clears `cut` unless `halt` is set.)

**`failed(error)`** (decision 22, 8.3): every `Err` of `cam.identity`, `cam.read`, `cam.apply` and `cam.act` passes through it first. For `CameraError::NotInSendTable(message)` it writes `names::log_refused_frame(&id, &message)` to stderr (id from `table.id_of(plug)`, else the port id of `found.usb.path`); every other error is not logged here.

**`run(p)`**, in this order:

1. `match p.bus.open(&p.found)`: on `Err(e)` go to "Open failure" below.
2. Build the `Worker`: `desc = cam.description().clone()`, `plan = cam.poll_plan().to_vec()`, `motion` = `cam.motion(name)` for every action name of `desc`; `queue.set_stops(names whose motion is Motion::Stop)`; `table.describe(plug, desc.clone())`; `state = names::initial_state(&desc)`; `pending = Pending::new()`; `series = Series::new(model.identity, model.firmware_query)`; `schedule = Schedule::new(Instant::now(), &timing)` (replaced in step 5).
3. If `series.at_plug_in()` gives a `Need`, `attempt(need)` (5.5: the first attempt at plug-in, asleep or awake).
4. `first_read()`: let `status` be the plan group whose leaves contain `"asleep"`. If there is one: `read_group(status)`; `asleep = state.asleep()` (a failed read counts as awake, decision 11). `series.after_first_status(asleep)`. Then for every other group in plan order: skip it when `asleep && !group.asleep`; else `read_group(name)` with the poll context, and for a `Tier::Touched` group `schedule.touched_read(name, ok)`. Stop early when `queue.halt` is set.
5. `schedule = Schedule::new(Instant::now(), &timing)`.
6. `table.announce(plug, series.serial.as_deref(), series.reason(), series.firmware.as_deref(), self.view())`.
7. The loop, until it returns:
   1. If `queue.is_ended()`: `end()`; return.
   2. If `queue.is_shutdown()`: take every job and drop it unsent; `queue.close()`; return. **[b-13]** Before dropping them: for each job with `out == None` and `stop` (the daemon's own stop, 5.9), and only while no `quiet_until` lies in the future, `cam.act(stop, {}, &state, own-stop ctx)` (an error goes through `failed` and is otherwise ignored).
   3. **[b-12]** If `quiet_until` is set and in the future: `queue.wait(quiet_until)`; continue. When it has passed, set it to `None`.
   4. If `queue.len() > 0`: `serve(queue.take_all())`; continue.
   5. If `schedule.due(now)` gives a tier: `tick(tier)`; continue.
   6. `queue.wait(schedule.next_due())`.

**`serve(jobs)`**: `let p = queue::plan(&jobs, restarts, single_leaf)`. Here `restarts = |_| false` and `single_leaf = |_| None`. **[b-12]** `restarts(job)` is true for a `Set` with any leaf whose setting's `warning` holds `Warning::Restart` (via `validate::leaves`; an invalid patch is false) and for a `Do` whose action's `warning` holds it. **[b-11]** `single_leaf(patch)` is `Some(path)` when `validate::leaves(&desc, patch)` gives exactly one leaf whose setting is not read-only and `check_value(setting, value, Fit::Write)` is `Ok`. Split off `jobs[p.take..]` and `queue.put_back` them; `batch(jobs[..p.take], p.coalesced)`.

**`tick(tier)`**:

1. `was_asleep = state.asleep()`; `topics = table.topics()`.
2. **[b-14]** If `tier == model.identity_tier` and `series.at_tick(!was_asleep)` gives a `Need`: `attempt(need)`.
3. For each name of `schedule.groups(&plan, tier, &topics, was_asleep)`: if `queue.len() > 0`, `serve(queue.take_all())` first (requests are served between groups, 5.7); if `queue.halt` is set, stop; `read_group(name, poll ctx)`; for a `Tier::Touched` group (a retry) `schedule.touched_read(name, ok)`.
4. `schedule.ticked(tier, Instant::now(), &timing)`.
5. `publish(Cause::Poll)`.
6. **[b-12]** If `was_asleep && !state.asleep()`: `wake()`.

**[b-12] `wake()`** (5.7 "Wake and retries"): **[b-14]** if `series.at_wake()` gives a `Need`, `attempt(need)` (5.5 wake series); `groups = Schedule::wake_groups(&plan, &state.unread)`; read each with the poll context (touched groups: `schedule.touched_read`); `schedule.woke(&plan, &groups, &table.topics(), Instant::now(), &timing)`; `publish(Cause::Poll)`.

**`attempt(need)`**: `let got = cam.identity(need, identity ctx)`; an `Err` goes through `failed`; `series.answered(got.ok().as_ref())`; if the device is listed (after step 6 of `run`): `table.identity(plug, series.serial.as_deref(), series.firmware.as_deref())` (the table hands over or records the firmware; it does nothing when nothing is new).

**`read_group(name, ctx) -> bool`**:

1. Find the group. If `state.asleep()` and `!group.asleep`: do not read it (decision 10); if its tier is `Touched`, put its leaves in `state.unread` (those not there yet, in group order) and `schedule.touched_read(name, false)`; return `false`.
2. `match cam.read(name, ctx)`:
   - `Ok(values)`: for each leaf of `group.leaves`: `v = values.get(leaf).cloned().unwrap_or(Null)`; if `v` is a string for which `obsbot_proto::values::is_raw` holds and `(leaf, v)` is not in `raw_logged`, log `names::log_raw(&id, leaf, v)` once (id from `table.id_of(plug)`, else the port id); `state.set(leaf, v)`; remove `leaf` from `state.unread`. Then `pending.group_read(&desc.settings, &values, Instant::now())`. Return `true`.
   - `Err(e)`: `failed(&e)`; put every leaf of the group into `state.unread` (keeping its last value; no duplicates; keep `unread` in the order leaves were added). Return `false`.

**`view()`**: `State { available: true, pending: pending.map(), unread: state.unread.clone(), values: state.values.clone() }`. **`publish(cause)`**: `table.publish(plug, &self.view(), cause)`.

**`end()`** (5.6 "Unavailable"): `for job in queue.close()`: skip a job whose `out` is `None`; else `table.route(&job.device).answer(&job)` (an unplugged `sn:` entry answers `get` from its last state and `set`/`do` with `unavailable`; a removed `port:` entry answers `unknown_device`). Publish nothing.

**Open failure** (decision 17): log `names::log_open_failed(port id, &e.to_string())`; `table.announce(plug, None, PortReason::NoSerial, None, State::new(true))`; then loop: on `queue.is_ended()` → `end()`, return; on `queue.is_shutdown()` → `queue.close()`, return; for each job of `queue.take_all()`: a `Get` from `table.current(plug)` (with `validate_get` against the empty description), `Set`/`Do` answered `camera` with message `"the camera could not be opened: <e>"`; else `queue.wait(now + 1 s)`.

**`batch(jobs, coalesced)`** in `batch.rs` (4.1 rules 3 to 6 and 9, 5.6). What reaches the camera is the value the request names, through `pending::sent` (decision 9); what the state, `pending` and the later requests of the batch see is its `echo`.

1. `working = self.view()`; `touched: BTreeSet<String>` (group names); `written: Vec<(leaf, echo, accepted_at: Instant, readable_now: bool)>`; `outcomes: Vec<Outcome>` where `enum Outcome { Skip, Get(Option<Vec<String>>), Done(Option<Params>), Refused(Refusal), Failed { applied: Vec<String>, failed: String, error: CameraError } }` (**[b-11]** adds `Coalesced`); `deadlines: Vec<Instant>` (**[b-12]** adds `restart = false`).
2. For each job `i` in order:
   1. If `job.out` is `Some(out)` and `out.is_closed()`: `Skip` (4.1 rule 9: its connection closed before the worker started it; the connection's close marks its outbound closed first, so this also holds for a job already taken into this batch, b-08 step 5). If `Instant::now() >= job.arrived + timing.request_deadline`: answer `camera` "the request was not started within 15 s" and `Skip`.
   2. `Get(paths)`: with `Some(paths)`, `validate_get(&desc, &paths)` → `Refused` on error, else `Get(paths)`. (No camera read: 4.1 rule 9.)
   3. `Set(patch)`: `validate_set(&desc, &working, &patch)` → `Refused` on error. **[b-11]** If `coalesced[i]`: `Coalesced`. Else: `order = cam.order(&leaf paths)`; for each leaf in that order: `e = echo(setting, value)`; `cam.apply(leaf, &sent(setting, value), &request ctx)`. On `Ok`: push the leaf to `applied`; `working.set(leaf, e.clone())`; `touched.extend(cam.touched(Target::Leaf(leaf)))`; if the setting has `readback`, push `(leaf, e, now, !state.asleep() || group_of(leaf).asleep)` to `written`, else `state.set(leaf, e)` (a `readback:false` leaf takes the written value once accepted, 5.6); **[b-12]** if its `warning` holds `Restart`, `restart = true`. On `Err(err)`: `failed(&err)`; `Failed { applied, failed: leaf, error: err }` and stop the loop over the batch. Else `Done(None)`. Record `job.arrived + request_deadline` in `deadlines`.
   4. `Do { action, params }`: `validate_do(&desc, &working, &action, &params)` → `Refused`; then `cam.check(&action, &params, &working)` → `Refused`. `p = params` with every value that has a setting in the action's `params` replaced by its `sent` value. `cam.act(&action, &p, &working, &request ctx)`: on `Ok(result)`: `touched.extend(cam.touched(Target::Action(&action)))`; **[b-13]** `queue.record_act(&action, motion[&action], job.conn, p.get("keep") == Some(&Value::Bool(true)))`; **[b-12]** if the action's `warning` holds `Restart`, `restart = true`; `Done(result)`. On `Err(err)`: `failed(&err)`; `Failed { applied: [], failed: action, error: err }` and stop the loop. Record the deadline.
3. If the loop stopped at job `i`: `queue.put_back(jobs[i + 1..])` (4.1 rule 9; when the device went, `end()` answers them next).
4. If `queue.is_ended()` (the device went during the batch, 5.6 "Unavailable"): answer every job up to the failure in order without re-reading and without publishing: `Get` from `table.route(&job.device)` as in `end()`, `Done` (**[b-11]** and `Coalesced`) `ok` with the rev of the last state this worker published (they were accepted before the device went), `Refused` as below, `Failed` as `camera` with `applied` and `failed` and **no** `rev`. Return.
5. **[b-12]** If `restart` (the batch is this one request, decision 8): for each `written` leaf `pending.written(setting, leaf, e, None, Some(accepted_at))`; `rev = publish(Cause::Request)`; answer a `Set` `ok {rev}`, a `Do` `ok {rev: current rev, result}` (5.6: "An action replies with the current rev"); `quiet_until = Some(Instant::now() + timing.request_deadline)`; return.
6. Re-read: `ctx = Ctx::new(deadlines.iter().min().copied(), Some(queue.cut.clone()))`; `ok_groups = {}`; for each plan group in plan order whose name is in `touched`: if `read_group(name, &ctx)` is `true`, add it to `ok_groups`; for a `Touched` group also `schedule.touched_read(name, ok)`.
7. For each `(leaf, e, at, readable)` of `written`, in order: `read = ok_groups contains group_of(leaf)` then `Some(state.at(leaf))` else `None`; `pending.written(setting, leaf, e, read, readable.then_some(at))`.
8. `rev = publish(Cause::Request)`.
9. Answer in job order: `Get(paths)` → `ok_line(id, OkBody::State(RevState { rev, state }))` from `table.current(plug)` with `state.select(paths)` for `Some(paths)`; `Done(result)` → `ok_line(id, OkBody::Rev(RevOk { rev, result }))`; **[b-11]** `Coalesced` → the same with `result: None`; `Refused(r)` → `error_line(Some(id), error(r.code, &r.message))`; `Failed` → `ErrorBody { code: Camera (Internal for CameraError::NotInSendTable, already logged by `failed`), message: error.to_string(), rev: Some(rev), applied: Some(applied), failed: Some(failed), .. }`; `Skip` → nothing. A job with `out: None` gets no reply.
10. **[b-12]** If the batch's re-read turned `asleep` from true to false, `wake()`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/obsbot4linuxd/src/worker.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::names;
    use crate::outbound::Outbound;
    use crate::queue::{Job, Work};
    use obsbot_device::models::fake::{FakeBus, Op};
    use obsbot_proto::{Description, Hello, Patch, RequestId};
    use serde_json::{Value, json};
    use std::sync::mpsc::Receiver;
    use std::time::{Duration, Instant};

    fn fast() -> Timing {
        Timing {
            scan: Duration::from_millis(100),
            request_deadline: Duration::from_secs(15),
            second: Duration::from_millis(100),
            five_seconds: Duration::from_millis(500),
            minute: Duration::from_secs(3),
        }
    }

    struct Rig {
        bus: Arc<FakeBus>,
        table: Arc<Table>,
        queue: Arc<DeviceQueue>,
        events: Receiver<String>,
        out: Outbound,
        replies: Receiver<String>,
        handle: JoinHandle<()>,
    }

    /// One worker over one fake device, as the registry would start it, with one subscribed
    /// connection (`events`) and one requester (`out`/`replies`).
    fn rig(model: &str, setup: impl FnOnce(&FakeBus)) -> Rig {
        let bus = Arc::new(FakeBus::new(&[model]).unwrap());
        setup(&bus);
        let found = bus.scan().remove(0);
        let entry = found.model.unwrap();
        let table = Arc::new(Table::new("test"));
        let queue = DeviceQueue::new();
        let id = obsbot_proto::port_id(&found.usb.path);
        let device =
            names::supported_device(&found, entry, Description::default(), &id, None, None);
        table.plug_hidden(1, entry, device, queue.clone());
        let (sub, events) = Outbound::new(1, None, 10_000);
        table.hello(
            &sub,
            &RequestId::Int(0),
            &Hello::new("t 1").subscribe(true),
            Instant::now(),
            Duration::ZERO,
        );
        events.recv().unwrap();
        let (out, replies) = Outbound::new(2, None, 10_000);
        let p = Plugged {
            plug: 1,
            found,
            model: entry,
            bus: bus.clone(),
            table: table.clone(),
            queue: queue.clone(),
            timing: fast(),
        };
        let handle = std::thread::spawn(move || run(p));
        Rig {
            bus,
            table,
            queue,
            events,
            out,
            replies,
            handle,
        }
    }

    fn next(rx: &Receiver<String>) -> Value {
        serde_json::from_str(&rx.recv_timeout(Duration::from_secs(5)).expect("a line")).unwrap()
    }

    fn until(rx: &Receiver<String>, pred: impl Fn(&Value) -> bool) -> Value {
        loop {
            let v = next(rx);
            if pred(&v) {
                return v;
            }
        }
    }

    fn job(r: &Rig, id: i64, device: &str, work: Work) -> Job {
        Job {
            id: RequestId::Int(id),
            out: Some(r.out.clone()),
            conn: 2,
            device: device.into(),
            work,
            arrived: Instant::now(),
            stop: false,
        }
    }

    fn stop(r: Rig) {
        r.queue.shutdown();
        r.handle.join().unwrap();
    }

    /// 5.2: identity attempt, then the group holding `asleep`, then every other group; then
    /// the entry is listed (devices, then state at rev 1).
    #[test]
    fn the_first_read_attempt_is_announced() {
        let r = rig("fake-a", |_| {});
        let devices = until(&r.events, |v| v["event"] == "devices");
        assert_eq!(devices["devices"][0]["id"], "sn:FAKEA0000001");
        let st = until(&r.events, |v| v["event"] == "state");
        assert_eq!(st["rev"], 1);
        assert_eq!(st["state"]["led"]["brightness"], 0);
        assert_eq!(st["state"]["unread"], json!([]));
        let ops = r.bus.ops("sn:FAKEA0000001");
        assert_eq!(
            ops[..3],
            [
                Op::Open,
                Op::Identity(obsbot_device::contract::Need {
                    serial: true,
                    firmware: true
                }),
                Op::Read("status".into())
            ]
        );
        assert!(
            ops.contains(&Op::Read("presets".into())),
            "touched groups are read at plug-in"
        );
        stop(r);
    }

    /// 5.2 and 5.5: asleep at plug-in, only the groups readable asleep are read; one identity
    /// attempt; the rest is null and unread.
    #[test]
    fn asleep_at_plug_in_only_the_groups_readable_asleep_are_read() {
        let r = rig("fake-b", |bus| bus.sleep("sn:FAKEB0000001").unwrap());
        let st = until(&r.events, |v| v["event"] == "state");
        assert_eq!(st["state"]["asleep"], true);
        assert!(
            st["state"]["unread"]
                .as_array()
                .unwrap()
                .contains(&json!("led.brightness"))
        );
        assert_eq!(st["state"]["led"]["brightness"], Value::Null);
        std::thread::sleep(Duration::from_millis(1200));
        let ops = r.bus.ops("sn:FAKEB0000001");
        let reads: Vec<&Op> = ops.iter().filter(|o| matches!(o, Op::Read(_))).collect();
        assert!(
            reads.iter().all(|o| **o == Op::Read("status".into())),
            "{reads:?}"
        );
        assert_eq!(
            ops.iter().filter(|o| matches!(o, Op::Identity(_))).count(),
            1
        );
        stop(r);
    }

    /// 4.1 rules 3 and 4: the state that reflects the write is published before the reply,
    /// with the leaf pending while the readback lags.
    #[test]
    fn a_set_publishes_before_it_replies() {
        let r = rig("fake-b", |_| {});
        until(&r.events, |v| v["event"] == "state");
        r.queue
            .push(job(
                &r,
                7,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("led.brightness", 3)),
            ))
            .ok()
            .unwrap();
        let reply = next(&r.replies);
        let ev = until(&r.events, |v| v["event"] == "state");
        assert_eq!(reply["ok"]["rev"], ev["rev"]);
        assert_eq!(ev["state"]["pending"], json!({"led.brightness": 3}));
        let later = until(&r.events, |v| {
            v["event"] == "state" && v["state"]["pending"] == json!({})
        });
        assert_eq!(later["state"]["led"]["brightness"], 3);
        stop(r);
    }

    /// 5.6 "Unavailable": the worker ends; a queued request is answered as for an unavailable
    /// device.
    #[test]
    fn the_end_answers_the_queue() {
        let r = rig("fake-a", |_| {});
        until(&r.events, |v| v["event"] == "state");
        r.bus.set_delay("fake-a", Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(300));
        r.queue
            .push(job(&r, 8, "sn:FAKEA0000001", Work::Get(None)))
            .ok()
            .unwrap();
        r.queue
            .push(job(
                &r,
                9,
                "sn:FAKEA0000001",
                Work::Set(Patch::new().leaf("led.brightness", 1)),
            ))
            .ok()
            .unwrap();
        r.bus.unplug("sn:FAKEA0000001").unwrap();
        r.table.unplug(1);
        r.queue.end();
        let t0 = Instant::now();
        r.handle.join().unwrap();
        assert!(
            t0.elapsed() < Duration::from_secs(1),
            "the 2 s operation was cut"
        );
        let a = next(&r.replies);
        let b = next(&r.replies);
        assert_eq!(a["ok"]["state"]["available"], false);
        assert_eq!(b["error"]["code"], "unavailable");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 worker::`
Expected: FAIL: each test times out on `a line` or panics `not yet implemented: task b-07-worker` in the worker thread.

- [ ] **Step 3: Write the worker**

Replace the stub `run` in `crates/obsbot4linuxd/src/worker.rs` and write `crates/obsbot4linuxd/src/batch.rs` with the struct, the methods and the algorithms above, leaving out every step marked **[b-11]** to **[b-14]** (use the plain step given beside it). `batch.rs` holds only `impl Worker { pub(crate) fn batch(...) }`, its `Outcome` enum and helpers it alone uses.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 worker::`
Expected: PASS: 4 tests.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy -p obsbot4linuxd --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/worker.rs crates/obsbot4linuxd/src/batch.rs
git commit -m "obsbot4linuxd: device worker with first read, poll tiers, identity series and write path"
```

### Task b-08 (key `b-08-conn`): Acceptor and connections

**Files:**

- Replace: `crates/obsbot4linuxd/src/conn.rs`
- Test: unit tests inside the file (the module below)

**Interfaces:**

- Consumes: `crate::lines::{read_line, Line}` (b-02); `crate::queue::{DeviceQueue, Job, Work, Refused}` (b-05); `crate::table::{Table, Routed}` (b-06); `crate::outbound::{ConnId, Outbound (with `mark_closed`), ok_line, error, error_line}`, `crate::{MAX_CONNECTIONS, OUTBOUND_LINES, ACCEPT_BACKOFF, Timing}` (b-01); `obsbot_proto::{Request, RequestError, Verb, PROTOCOL, MAX_REQUEST_LINE, OkBody, Empty, ErrorCode}`.
- Produces (the module starts with exactly these items; replace each `todo!()`):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The acceptor and the connections (4.1, 5.1). Written in task b-08-conn.

use crate::Timing;
use crate::outbound::ConnId;
use crate::table::Table;
use std::collections::BTreeMap;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

/// Every accepted connection, for the limit of 64 and for shutdown.
pub struct Conns {
    streams: Mutex<BTreeMap<ConnId, UnixStream>>,
    next: AtomicU64,
}

impl Conns {
    pub fn new() -> Conns {
        Conns {
            streams: Mutex::new(BTreeMap::new()),
            next: AtomicU64::new(1),
        }
    }

    /// Registers an accepted stream; `None` (and the stream is closed) when `MAX_CONNECTIONS`
    /// are open already.
    pub fn admit(&self, stream: &UnixStream) -> Option<ConnId> {
        let _ = (stream, &self.streams, &self.next);
        todo!("task b-08-conn")
    }

    pub fn remove(&self, conn: ConnId) {
        let _ = conn;
        todo!("task b-08-conn")
    }

    pub fn len(&self) -> usize {
        todo!("task b-08-conn")
    }

    /// Shuts every connection down in both directions (5.9 shutdown).
    pub fn close_all(&self) {
        todo!("task b-08-conn")
    }
}

/// What every connection thread shares.
pub struct Shared {
    pub table: Arc<Table>,
    pub timing: Timing,
    pub conns: Conns,
}

/// The acceptor (5.1): accepts until `stop` is set; a failed accept waits `ACCEPT_BACKOFF`.
pub fn accept_loop(listener: UnixListener, shared: Arc<Shared>, stop: Arc<AtomicBool>) {
    let _ = (listener, shared, stop);
    todo!("task b-08-conn")
}

/// One connection: its reader runs on the calling thread, its writer on a thread of its own.
pub fn serve(stream: UnixStream, conn: ConnId, shared: Arc<Shared>) {
    let _ = (stream, conn, shared);
    todo!("task b-08-conn")
}
```

**`Conns`**: `streams` holds a clone of every admitted stream. `admit`: under the lock, when `len() >= MAX_CONNECTIONS` shut the stream down (`Shutdown::Both`) and return `None`; else take the next id (from 1), store `stream.try_clone()` and return it. `remove(id)`: drop its clone. `len()`: the number stored. `close_all()`: `shutdown(Both)` on every stored stream (the readers then end). Locks poison-tolerant.

**`accept_loop`**: `for stream in listener.incoming()`: first, if `stop` is set, return (the daemon wakes a blocked `accept` by connecting once). `Ok(s)`: `conns.admit(&s)`; `None` drops it (closed at once, 4.1); `Some(id)`: spawn a thread named `conn <id>` running `serve(s, id, shared.clone())`; if the spawn fails, `conns.remove(id)`. `Err(e)`: log `obsbot4linuxd: accept: <e>` and sleep `ACCEPT_BACKOFF`.

**`serve(stream, conn, shared)`**:

1. `(out, rx) = Outbound::new(conn, stream.try_clone().ok(), OUTBOUND_LINES)`. A guard value is created now whose `Drop` runs the close steps of 5 below, so a panic in this thread closes only this connection (5.9).
2. Spawn the writer (thread `conn <id> writer`) with a clone of the stream: `for line in rx { write line, then b"\n" }`; on a write error shut the stream down in both directions and stop. It holds no `Outbound`, so it ends once every sender is gone, after writing what was queued (a `protocol` error reaches the client before the close).
3. Read lines with `read_line(&mut BufReader::new(stream), MAX_REQUEST_LINE)`: `Eof`, `TooLong` (4.1: closes the connection) or an I/O error end the loop. `Text` that is empty after trimming is skipped. `NotUtf8` is handled as a line for which `Request::from_line` failed with `id: None` and message `"the line is not UTF-8"`.
4. Per line, with `parsed = Request::from_line(&text)`:
   - Before `hello`: a `Hello` with `protocol == PROTOCOL` → `shared.table.hello(&out, &id, &hello, arrival, shared.timing.request_deadline)` (it waits for start-up; lines behind it wait in the socket, so they are handled after the reply). A `Hello` with another protocol → `protocol` error `"protocol <n> is not supported"`, `supported: [2]`, end. Any other request → `protocol` error with the request's id, `"the first message must be hello"`, `supported: [2]`, end. A `RequestError` → the same with `e.id` (`null` when none), end.
   - After `hello`: `Err(e)` → `invalid` with `e.id` and `e.message`; `Hello` → `invalid` "hello was already sent"; `Live { topics }` → `table.set_live(conn, &topics)` and `ok {}`; `Get`/`Set`/`Do` → route: build `Job { id, out: Some(out.clone()), conn, device, work, arrived: now, stop: false }`; `match table.route(&device)`: `Routed::Worker(q)`: for a `Do`, `job.stop = q.is_stop(&action)`; `q.push(job)`: `Err(Refused::Busy(j))` → `j.fail(Busy, "32 requests are queued for <device>")`; `Err(Refused::Closed(j))` → `table.route(&device).answer(&j)` (the device just went). Any other `Routed` → `routed.answer(&job)`.
5. Close (the guard's `Drop`), in this order: `out.mark_closed()` first (4.1 rule 9: a worker skips every request of this connection it has not started, also one it has already taken into a batch, because the jobs hold clones of `out`; `mark_closed` leaves the socket to the writer, so a queued `protocol` error still reaches the client); then `shared.table.remove_conn(conn)`; for every `q` of `shared.table.queues()`: `q.close_conn(conn, Instant::now())` (drops its queued requests and queues the stop of its continuing action, 5.6); `shared.conns.remove(conn)`. The guard holds the reader's `out` and drops it last.

- [ ] **Step 1: Write the failing tests**

Write `crates/obsbot4linuxd/src/conn.rs` with the items above (bodies `todo!()`), then append its tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::PortReason;
    use crate::names;
    use crate::queue::{DeviceQueue, Work};
    use obsbot_device::contract::{Bus, Motion};
    use obsbot_device::models::fake::{FakeBus, KEYS};
    use obsbot_proto::State;
    use serde_json::{Value, json};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::time::{Duration, Instant};

    /// A table with fake-a listed (queue returned) and the unknown device.
    fn shared() -> (Arc<Shared>, Arc<DeviceQueue>) {
        let found = FakeBus::new(&KEYS).unwrap().scan();
        let table = Arc::new(Table::new("test"));
        let q = DeviceQueue::new();
        q.set_stops(vec!["gimbal.stop".into(), "zoom.stop".into()]);
        let model = found[0].model.unwrap();
        let device = names::supported_device(
            &found[0],
            model,
            obsbot_device::models::fake::description("fake-a").unwrap(),
            "port:9-1",
            None,
            None,
        );
        table.plug_hidden(1, model, device, q.clone());
        table.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            State::new(true),
        );
        table.plug_unsupported(3, names::unsupported_device(&found[2]));
        let timing = crate::Timing::SPEC;
        (
            Arc::new(Shared {
                table,
                timing,
                conns: Conns::new(),
            }),
            q,
        )
    }

    fn client(shared: &Arc<Shared>) -> (UnixStream, BufReader<UnixStream>) {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let id = shared.conns.admit(&theirs).unwrap();
        let s = shared.clone();
        std::thread::spawn(move || serve(theirs, id, s));
        ours.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let reader = BufReader::new(ours.try_clone().unwrap());
        (ours, reader)
    }

    fn send(s: &mut UnixStream, v: Value) {
        writeln!(s, "{v}").unwrap();
    }

    fn recv(r: &mut BufReader<UnixStream>) -> Value {
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    fn eof(r: &mut BufReader<UnixStream>) -> bool {
        let mut rest = Vec::new();
        r.read_to_end(&mut rest).is_ok()
    }

    fn hello(s: &mut UnixStream, r: &mut BufReader<UnixStream>) -> Value {
        send(
            s,
            json!({"id": 1, "hello": {"protocol": 2, "client": "t 1", "subscribe": false}}),
        );
        recv(r)
    }

    #[test]
    fn hello_then_requests_are_routed() {
        let (sh, q) = shared();
        let (mut s, mut r) = client(&sh);
        assert_eq!(
            hello(&mut s, &mut r)["ok"]["devices"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        send(
            &mut s,
            json!({"id": 2, "device": "sn:FAKEA0000001", "set": {"led.brightness": 1}}),
        );
        send(
            &mut s,
            json!({"id": 3, "device": "port:9-3", "do": "sleep"}),
        );
        assert_eq!(
            recv(&mut r)["error"]["code"],
            "unsupported_device",
            "answered at once"
        );
        assert!(crate::schedule::PERIODIC.len() == 3);
        let jobs = q.take_all();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].device, "sn:FAKEA0000001");
        assert!(matches!(&jobs[0].work, Work::Set(_)));
        send(
            &mut s,
            json!({"id": 4, "device": "sn:FAKEA0000001", "do": "gimbal.stop"}),
        );
        send(
            &mut s,
            json!({"id": 5, "hello": {"protocol": 2, "client": "again"}}),
        );
        assert_eq!(recv(&mut r)["error"]["code"], "invalid");
        assert!(q.take_all()[0].stop, "the queue knows the model's stops");
        send(&mut s, json!({"id": 6, "live": ["pose"]}));
        assert_eq!(recv(&mut r), json!({"id": 6, "ok": {}}));
        assert_eq!(sh.table.topics(), Default::default(), "subscribe was false");
    }

    #[test]
    fn the_first_line_must_be_hello_and_the_connection_closes() {
        let (sh, _) = shared();
        let (mut s, mut r) = client(&sh);
        send(&mut s, json!({"id": 9, "device": "port:9-3", "get": null}));
        let v = recv(&mut r);
        assert_eq!(v["id"], 9);
        assert_eq!(v["error"]["code"], "protocol");
        assert_eq!(v["error"]["supported"], json!([2]));
        assert!(eof(&mut r));
    }

    #[test]
    fn busy_is_answered_by_the_reader() {
        let (sh, _q) = shared();
        let (mut s, mut r) = client(&sh);
        hello(&mut s, &mut r);
        for i in 0..crate::MAX_QUEUED {
            send(
                &mut s,
                json!({"id": 100 + i, "device": "sn:FAKEA0000001", "get": null}),
            );
        }
        send(
            &mut s,
            json!({"id": 99, "device": "sn:FAKEA0000001", "get": null}),
        );
        let v = recv(&mut r);
        assert_eq!(
            (v["id"].clone(), v["error"]["code"].clone()),
            (json!(99), json!("busy"))
        );
    }

    /// 4.1 rule 9 and 5.6: when a connection closes, its queued requests are dropped and the
    /// stop of its continuing action is queued.
    #[test]
    fn a_closing_connection_drops_its_requests_and_stops_its_turn() {
        let (sh, q) = shared();
        let (mut s, mut r) = client(&sh);
        hello(&mut s, &mut r);
        send(
            &mut s,
            json!({"id": 2, "device": "sn:FAKEA0000001", "set": {"led.brightness": 1}}),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while q.len() == 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        q.record_act(
            "gimbal.turn",
            Motion::Continuing {
                stop: "gimbal.stop",
            },
            1,
            false,
        );
        drop(r);
        s.shutdown(std::net::Shutdown::Both).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while sh.conns.len() > 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        let jobs = q.take_all();
        assert_eq!(jobs.len(), 1);
        assert!(
            jobs[0].out.is_none() && jobs[0].stop,
            "only the daemon's own stop"
        );
    }

    /// 4.1 rule 9: a request the worker has already taken into a batch is no longer in the
    /// queue; the close marks its connection closed, so the batch skips it (b-07 batch step
    /// 2.1).
    #[test]
    fn a_closing_connection_marks_requests_already_taken_closed() {
        let (sh, q) = shared();
        let (mut s, mut r) = client(&sh);
        hello(&mut s, &mut r);
        send(
            &mut s,
            json!({"id": 2, "device": "sn:FAKEA0000001", "set": {"led.brightness": 1}}),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while q.len() == 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        let taken = q.take_all();
        let out = taken[0].out.clone().expect("a client's request");
        assert!(!out.is_closed());
        drop(r);
        s.shutdown(std::net::Shutdown::Both).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while sh.conns.len() > 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(out.is_closed(), "the batch skips it");
    }

    #[test]
    fn a_line_over_64_kib_closes_the_connection() {
        let (sh, _) = shared();
        let (mut s, mut r) = client(&sh);
        hello(&mut s, &mut r);
        s.write_all(&vec![b'x'; obsbot_proto::MAX_REQUEST_LINE + 1])
            .unwrap();
        assert!(eof(&mut r));
    }

    #[test]
    fn at_most_64_connections_are_admitted() {
        let conns = Conns::new();
        let mut keep = Vec::new();
        for _ in 0..crate::MAX_CONNECTIONS {
            let (a, b) = UnixStream::pair().unwrap();
            assert!(conns.admit(&b).is_some());
            keep.push((a, b));
        }
        let (mut a, b) = UnixStream::pair().unwrap();
        assert_eq!(conns.admit(&b), None);
        drop(b);
        assert_eq!(a.read(&mut [0u8; 1]).unwrap(), 0, "closed at once");
        conns.remove(1);
        assert_eq!(conns.len(), crate::MAX_CONNECTIONS - 1);
        let (_c, d) = UnixStream::pair().unwrap();
        assert!(conns.admit(&d).is_some());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 conn::`
Expected: FAIL: the tests panic with `not yet implemented`.

- [ ] **Step 3: Write the code**

Fill the bodies as described above.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 conn::`
Expected: PASS: 7 tests.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy -p obsbot4linuxd --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/conn.rs
git commit -m "obsbot4linuxd: acceptor, connection reader and writer, request routing"
```

### Task b-09 (key `b-09-registry`): Registry thread

**Files:**

- Replace: `crates/obsbot4linuxd/src/registry.rs`
- Test: unit tests inside the file (the module below)

**Interfaces:**

- Consumes: `crate::hotplug::{Seen, diff}`, `crate::names::{supported_device, unsupported_device}`, `crate::worker::Plugged`, `crate::daemon::Fatal` (b-01); `crate::queue::DeviceQueue` (b-05: `new`, `end`, `shutdown`, `is_ended`, `is_shutdown`); `crate::table::{Table, PlugId}` (b-06: `plug_unsupported`, `plug_hidden`, `startup`, `unplug`, `devices`); `trait Bus`.
- Produces (the module starts with exactly these items; replace each `todo!()`):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The registry thread (5.3): one scan a second through `trait Bus`, plug and unplug.
//! Written in task b-09-registry.

use crate::Timing;
use crate::daemon::Fatal;
use crate::hotplug::Seen;
use crate::queue::DeviceQueue;
use crate::table::{PlugId, Table};
use crate::worker::Plugged;
use obsbot_device::contract::Bus;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;

/// Starts the worker of a plugged supported device.
pub type Spawner = Box<dyn Fn(Plugged) -> std::io::Result<JoinHandle<()>> + Send>;

struct Known {
    seen: Seen,
    plug: PlugId,
    queue: Option<Arc<DeviceQueue>>,
}

pub struct Registry {
    bus: Arc<dyn Bus>,
    table: Arc<Table>,
    timing: Timing,
    spawn: Spawner,
    known: Vec<Known>,
    next: PlugId,
    workers: Vec<(PlugId, JoinHandle<()>)>,
}

impl Registry {
    pub fn new(bus: Arc<dyn Bus>, table: Arc<Table>, timing: Timing, spawn: Spawner) -> Registry {
        Registry {
            bus,
            table,
            timing,
            spawn,
            known: Vec::new(),
            next: 1,
            workers: Vec::new(),
        }
    }

    /// One enumeration pass; returns the plugs of the supported devices it added. With
    /// `startup`, each of them joins the set every `hello` waits for (5.2), before its worker
    /// starts.
    pub fn scan(&mut self, startup: bool) -> Vec<PlugId> {
        let _ = startup;
        let _ = (
            &self.bus,
            &self.table,
            &self.timing,
            &self.spawn,
            &self.known,
            &self.next,
            &self.workers,
        );
        todo!("task b-09-registry")
    }

    /// Scans every `timing.scan` until `stop` is set, then shuts the workers down and joins
    /// them, at most `timing.request_deadline` long.
    pub fn run(self, stop: Arc<AtomicBool>, fatal: Arc<Fatal>) {
        let _ = (stop, fatal);
        todo!("task b-09-registry")
    }
}
```

**`scan(startup)`** (5.3):

1. `found = bus.scan()`; `d = hotplug::diff(&known seens, &found)`.
2. Removals first (4.1 Events: a removal is its own `devices` event before any event that adds an entry under the same id): for each index of `d.gone`, in descending order, remove that `Known` from `known`, call `table.unplug(plug)`, then `queue.end()` if it has one.
3. Additions, in scan order: for each index of `d.new`: `plug = next; next += 1`; with `f = found[i].clone()`:
   - `f.model == None`: `table.plug_unsupported(plug, names::unsupported_device(&f))`; push `Known { seen, plug, queue: None }`.
   - `Some(model)`: `q = DeviceQueue::new()`; `table.plug_hidden(plug, model, names::supported_device(&f, model, Description::default(), &port_id(&f.usb.path), None, None), q.clone())`; if `startup`, `table.startup(&[plug])` before the worker starts; `(spawn)(Plugged { plug, found: f, model, bus: bus.clone(), table: table.clone(), queue: q.clone(), timing })`: `Ok(h)` → push `(plug, h)` to `workers`; `Err(e)` → log `obsbot4linuxd: cannot start the worker of <path>: <e>` and `table.unplug(plug)`. Push the `Known`; collect `plug` into the result.
4. Return the supported plugs added.

**`run(stop, fatal)`**: loop: sleep `timing.scan` in steps of at most 50 ms, returning to the shutdown below as soon as `stop` is set; `scan(false)`; drop the handles of finished workers (`is_finished`; a worker's panic was already recorded in `fatal` by `critical`). Shutdown: `queue.shutdown()` for every `Known` with a queue; then wait for every worker handle to finish, together at most `timing.request_deadline + 1 s` (poll `is_finished` every 10 ms), and join the finished ones. `fatal` is not touched otherwise; the registry thread itself runs under `critical`.

- [ ] **Step 1: Write the failing tests**

Write `crates/obsbot4linuxd/src/registry.rs` with the items above (bodies `todo!()`), then append its tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::models::fake::{FakeBus, KEYS};
    use std::sync::Mutex;
    use std::time::Duration;

    type Started = Arc<Mutex<Vec<(PlugId, Arc<DeviceQueue>)>>>;

    fn registry(bus: Arc<FakeBus>, table: Arc<Table>) -> (Registry, Started) {
        let started = Arc::new(Mutex::new(Vec::new()));
        let seen = started.clone();
        let spawn: Spawner = Box::new(move |p: Plugged| {
            seen.lock().unwrap().push((p.plug, p.queue.clone()));
            Ok(std::thread::spawn(|| ()))
        });
        let bus: Arc<dyn Bus> = bus;
        let timing = crate::Timing {
            scan: Duration::from_millis(100),
            ..crate::Timing::SPEC
        };
        (Registry::new(bus, table, timing, spawn), started)
    }

    #[test]
    fn the_first_scan_lists_the_unsupported_device_and_starts_two_workers() {
        let bus = Arc::new(FakeBus::new(&KEYS).unwrap());
        let table = Arc::new(Table::new("t"));
        let (mut reg, started) = registry(bus.clone(), table.clone());
        assert_eq!(reg.scan(true), [1, 2]);
        assert_eq!(
            started
                .lock()
                .unwrap()
                .iter()
                .map(|s| s.0)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        let listed: Vec<String> = table.devices().iter().map(|d| d.id.clone()).collect();
        assert_eq!(
            listed,
            ["port:9-3"],
            "supported devices wait for their workers"
        );
        assert_eq!(
            reg.scan(false),
            Vec::<PlugId>::new(),
            "an unchanged scan does nothing"
        );
        assert!(bus.ops("port:9-3").is_empty());
    }

    #[test]
    fn an_unplug_ends_the_worker_and_a_new_device_number_starts_a_new_one() {
        let bus = Arc::new(FakeBus::new(&KEYS).unwrap());
        let table = Arc::new(Table::new("t"));
        let (mut reg, started) = registry(bus.clone(), table.clone());
        reg.scan(true);
        let first = started.lock().unwrap()[0].1.clone();
        bus.unplug("sn:FAKEA0000001").unwrap();
        bus.plug("fake-a").unwrap();
        assert_eq!(reg.scan(false), [4], "a new plug for the new enumeration");
        assert!(first.is_ended(), "the old worker is told to end");
        bus.unplug("port:9-3").unwrap();
        reg.scan(false);
        assert!(
            table.devices().is_empty(),
            "the unsupported entry is removed"
        );
    }

    #[test]
    fn run_ends_when_stopped_and_shuts_the_workers_down() {
        let bus = Arc::new(FakeBus::new(&KEYS).unwrap());
        let table = Arc::new(Table::new("t"));
        let (mut reg, started) = registry(bus, table);
        reg.scan(true);
        let stop = Arc::new(AtomicBool::new(true));
        reg.run(stop, crate::daemon::Fatal::new());
        assert!(started.lock().unwrap().iter().all(|(_, q)| q.is_shutdown()));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 registry::`
Expected: FAIL: the tests panic with `not yet implemented: task b-09-registry`.

- [ ] **Step 3: Write the code**

Fill the bodies as described above.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 registry::`
Expected: PASS: 3 tests.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy -p obsbot4linuxd --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/registry.rs
git commit -m "obsbot4linuxd: registry thread with scan diff, plug, unplug and worker start"
```

### Task b-10 (key `b-10-daemon`): Daemon, test-control socket, binary (milestone B-1)

**Files:**

- Modify: `crates/obsbot4linuxd/src/daemon.rs` (replace the `run` stub). `lib.rs` keeps the skeleton's crate-wide `allow` until b-14 (decision 24).
- Replace: `crates/obsbot4linuxd/src/control.rs`, `crates/obsbot4linuxd/src/main.rs`
- Test: `crates/obsbot4linuxd/tests/core.rs`; unit test inside `control.rs`

**Interfaces:**

- Consumes: everything above: `socket::open`, `Bound` (b-02); `args::{parse, Args, USAGE}` (b-02); `lines::read_line` (b-02); `conn::{Conns, Shared, accept_loop}` (b-08); `registry::{Registry, Spawner}` (b-09); `worker::spawn` (b-01, b-07); `table::Table` (b-06); `queue::answer_late` (b-05); `daemon::{Fatal, critical}` (b-01); `FakeBus::handle_line`; `nix::sys::signal::{SigSet, Signal}`.
- Produces: milestone B-1 (the daemon core with the plain worker of b-07; what follows in b-11 to b-14 is listed under "Milestone B-1"); `daemon::run(Config, Timing) -> io::Result<()>` (so `obsbot4linuxd::run` and `run_with` work); `control::{control_path(&Path) -> PathBuf, serve(&Path, Arc<FakeBus>, Arc<AtomicBool>) -> io::Result<JoinHandle<()>>}`; the binary `obsbot4linuxd [--fake <models>] [--help] [--version]`.

**`control::serve(socket, bus, stop)`** (5.12): `path = control_path(socket)`; `listener = socket::bind_at(&path)?` (mode 0600, a stale file replaced); spawn a thread `test-control` that accepts until `stop` is set (checked after each accept; the daemon wakes it by connecting once); per connection a thread that reads lines with `read_line(…, MAX_REQUEST_LINE)` and writes `bus.handle_line(&line)` plus `\n` for each, until the end of the stream. Return the accept thread's handle.

**`daemon::run(config, timing)`** (5.2, 5.9):

1. `bound = socket::open(&config.socket)?` (lock first, then inherit or bind; the error names the lock file).
2. `bus: Arc<dyn Bus>` = `config.fake.clone()`, or for `None` return `Err(io::Error::new(ErrorKind::Unsupported, "the real device path comes with B-2 (task b-15)"))`. (b-15 replaces this one line.)
3. Fake mode: `control = Some(control::serve(&config.socket, fake.clone(), control_stop.clone())?)`.
4. `table = Arc::new(Table::new(env!("CARGO_PKG_VERSION")))`; `fatal = Fatal::new()`; `spawner: Spawner = Box::new(move |p| worker::spawn(p, fatal.clone()))`; `registry = Registry::new(bus, table.clone(), timing, spawner)`; `registry.scan(true)` (the first enumeration pass; its supported devices join the start-up set, their workers start).
5. `shared = Arc::new(Shared { table: table.clone(), timing, conns: Conns::new() })`; start with `critical`: `acceptor` (`accept_loop(bound.listener.try_clone()?, shared.clone(), accept_stop.clone())`), `registry` (`registry.run(registry_stop.clone(), fatal.clone())`), `sweeper` (every `SWEEP_EVERY` until `sweep_stop`: for each `q` of `table.queues()`, `queue::answer_late(q.expire(Instant::now(), timing.request_deadline))`).
6. Wait: every 50 ms, until `config.stop` is set or `fatal.get()` is `Some`.
7. Shut down in this order (5.9): set `accept_stop`, connect once to `config.socket` to wake `accept`, join the acceptor (at most 1 s); `shared.conns.close_all()` and wait until `shared.conns.len() == 0` (at most 2 s): each reader's guard queues the stop of a continuing action without `keep` (from b-13 on; on B-1 the record stays empty); set `registry_stop` and join the registry thread (it shuts the workers down: they send the queued own stops and end); set `sweep_stop`, join; in fake mode set `control_stop`, connect once to the control path, join, remove the control socket file; remove `bound.owned` if it is `Some`; drop `bound` (the lock).
8. Return `Err(io::Error::other(format!("the {name} thread panicked")))` when `fatal` holds a name, else `Ok(())`.

**`main`** (decisions 19, 20): parse `std::env::args().skip(1)` with `args::parse`: `Err(e)` → print `obsbot4linuxd: <e>` and `USAGE` to stderr, exit 2; `Help` → `USAGE` to stdout, exit 0; `Version` → `obsbot4linuxd <CARGO_PKG_VERSION>`, exit 0. `socket = obsbot_proto::socket_path()`, else print `obsbot4linuxd: neither OBSBOT4LINUX_SOCKET nor XDG_RUNTIME_DIR is set` and exit 1. `config = Config::new(socket)`; with `fake`, `config.fake = Some(Arc::new(FakeBus::new(&keys)?))` (an error exits 2). Block `SIGTERM` and `SIGINT` (`SigSet::thread_block`) before any thread starts; spawn one thread that `wait`s for either and sets `config.stop`. `obsbot4linuxd::run(config)`: `Ok` → exit 0; `Err(e)` → print `obsbot4linuxd: <e>`, exit 1.

- [ ] **Step 1: Write the failing tests**

Create `crates/obsbot4linuxd/tests/core.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! B-1: the smallest daemon tracks C and D run their live tests against (spec 10.3): the
//! fake devices over the socket, the five verbs, the validator's errors, the write path with
//! its touched re-read, `pending` and `rev`, events before replies, periodic reads, the
//! test-control socket and the `--fake` binary.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::{Value, json};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

#[test]
fn hello_lists_the_three_fakes_with_their_states() {
    let d = Daemon::start();
    let (_c, ok) = d.hello(false, &[]);
    assert_eq!(ok["protocol"], 2);
    assert_eq!(ids(&ok["devices"]), [A, B, UNKNOWN]);
    let support: Vec<&Value> = ok["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| &d["support"])
        .collect();
    assert_eq!(
        support,
        [&json!("tested"), &json!("untested"), &json!("unsupported")]
    );
    assert_eq!(ok["devices"][0]["name"], "Fake A 0001");
    assert_eq!(ok["devices"][0]["firmware"], "1.0.0.0");
    let a = &ok["states"][A];
    assert_eq!(a["rev"], 1);
    assert_eq!(
        at(&a["state"], "led.brightness"),
        &json!(0),
        "read at plug-in"
    );
    assert_eq!(a["state"]["unread"], json!([]));
    assert_eq!(
        ok["states"][UNKNOWN],
        json!({"rev": 1, "state": {"available": true, "pending": {}, "unread": []}})
    );
}

/// 4.1 rule 3: the state event reaches every subscribed connection, the requester included,
/// before the reply; the reply's rev is that state's rev.
#[test]
fn a_set_publishes_its_state_before_the_reply() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    let (mut other, _) = d.hello(true, &[]);
    let r = c.set(A, json!({"led.brightness": 2}));
    let rev = r["ok"]["rev"].clone();
    let ev = c
        .events
        .iter()
        .find(|e| state_of(A)(e))
        .expect("the event came first")
        .clone();
    assert_eq!(ev["rev"], rev);
    assert_eq!(at(&ev["state"], "led.brightness"), &json!(2));
    let seen = other.event(T, |e| state_of(A)(e) && e["rev"] == rev);
    assert!(seen.is_some(), "the other subscriber gets it too");
    let got = c.get(A, Some(&["led"]));
    assert_eq!(got["ok"]["rev"], rev);
    assert_eq!(at(&got["ok"]["state"], "led.brightness"), &json!(2));
    assert!(
        got["ok"]["state"].get("zoom").is_none(),
        "only the paths asked for"
    );
}

/// 4.1 rules 3 and 4 with fake-b's lagging readback: the reply's state lists the leaf in
/// `pending`; a later read shows it and it leaves `pending`.
#[test]
fn a_lagging_readback_is_pending_until_a_read_shows_it() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    let r = c.set(B, json!({"led": {"brightness": 3}}));
    let rev = r["ok"]["rev"].as_u64().unwrap();
    let ev = c
        .events
        .iter()
        .find(|e| state_of(B)(e) && e["rev"] == json!(rev))
        .expect("the state of the reply's rev")
        .clone();
    assert_eq!(ev["state"]["pending"], json!({"led.brightness": 3}));
    assert_eq!(
        at(&ev["state"], "led.brightness"),
        &json!(0),
        "the camera's value"
    );
    let cleared = c
        .event(T, |e| state_of(B)(e) && e["state"]["pending"] == json!({}))
        .expect("pending clears");
    assert_eq!(at(&cleared["state"], "led.brightness"), &json!(3));
    assert!(cleared["rev"].as_u64().unwrap() > rev);
}

/// 4.1 rule 6: a motion action replies when the camera accepted it; fake-a's `gimbal.move`
/// touches no group, so the pose arrives with a later read.
#[test]
fn an_action_replies_on_acceptance_and_its_result_is_returned() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &["pose"]);
    let r = c.act(A, "gimbal.move", json!({"pan": 20}));
    assert!(r["ok"]["rev"].is_u64(), "{r}");
    assert!(
        c.event(T, |e| state_of(A)(e)
            && at(&e["state"], "gimbal.pan") == &json!(20))
            .is_some()
    );
    let v = c.act(A, "view.save", json!({"name": "Desk"}));
    assert_eq!(v["ok"]["result"], json!({"id": 0}));
    let p = c.act(A, "preset.save", json!({"slot": 0}));
    assert!(p["ok"].is_object());
    let got = c.get(A, Some(&["presets"]));
    assert_eq!(
        got["ok"]["state"]["presets"][0]["name"], "",
        "no name is invented (spec 2)"
    );
}

/// The validator's codes reach the client (4.1 Errors), and nothing reaches the camera.
#[test]
fn refused_requests_send_nothing() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let before = d.ops(A).len();
    let code = |r: Value| r["error"]["code"].as_str().unwrap_or("ok").to_string();
    assert_eq!(code(c.set(A, json!({"led.brightness": "high"}))), "invalid");
    assert_eq!(code(c.set(A, json!({"led.brightnes": 1}))), "unsupported");
    assert_eq!(code(c.set(A, json!({"led.brightness": 4}))), "out_of_range");
    assert_eq!(
        code(c.set(A, json!({"exposure.time": 20000}))),
        "conflict",
        "requires exposure.mode=manual"
    );
    assert_eq!(
        code(c.act(A, "gimbal.move", json!({}))),
        "invalid",
        "the model's check"
    );
    assert_eq!(
        code(c.act(A, "preset.recall", json!({"slot": 2}))),
        "conflict",
        "empty slot"
    );
    assert_eq!(
        code(c.act(A, "factory_reset", json!({}))),
        "invalid",
        "no confirm"
    );
    assert_eq!(code(c.get(A, Some(&["zone.area.pan.min"]))), "unsupported");
    assert_eq!(
        code(c.set("sn:NOPE00000000", json!({"led.brightness": 1}))),
        "unknown_device"
    );
    assert_eq!(writes_after(&d, A, before), Vec::<Op>::new());
}

/// 5.4 and walk-through 2 (8.4): the unsupported device is listed, answered at once, and
/// receives nothing at all.
#[test]
fn the_unknown_device_is_answered_at_once_and_receives_nothing() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let got = c.get(UNKNOWN, None);
    assert_eq!(
        got["ok"],
        json!({"rev": 1, "state": {"available": true, "pending": {}, "unread": []}})
    );
    assert_eq!(
        c.set(UNKNOWN, json!({"led.brightness": 1}))["error"]["code"],
        "unsupported_device"
    );
    assert_eq!(
        c.act(UNKNOWN, "sleep", json!({}))["error"]["code"],
        "unsupported_device"
    );
    assert_eq!(d.ops(UNKNOWN), Vec::<Op>::new());
}

/// 4.1: the first message must be `hello`; a second `hello` is `invalid`; `live` replies `{}`.
#[test]
fn the_hello_rules_of_4_1() {
    let d = Daemon::start();
    let mut first = d.connect();
    first.send(&json!({"id": 1, "device": A, "get": null}));
    let r = first.line(T).unwrap();
    assert_eq!(r["error"]["code"], "protocol");
    assert_eq!(r["error"]["supported"], json!([2]));
    assert_eq!(r["id"], 1);
    assert!(first.closed_within(T), "the connection is closed");

    let mut old = d.connect();
    old.send(&json!({"id": 1, "hello": {"protocol": 3, "client": "future 9"}}));
    assert_eq!(old.line(T).unwrap()["error"]["code"], "protocol");
    assert!(old.closed_within(T));

    let mut junk = d.connect();
    junk.send_raw(b"not json\n");
    let r = junk.line(T).unwrap();
    assert_eq!(
        (r["id"].clone(), r["error"]["code"].clone()),
        (json!(null), json!("protocol"))
    );

    let (mut c, _) = d.hello(false, &[]);
    c.send(&json!({"id": 7, "hello": {"protocol": 2, "client": "again"}}));
    assert_eq!(c.reply(7)["error"]["code"], "invalid");
    c.send(&json!({"id": 8, "live": ["pose"]}));
    assert_eq!(c.reply(8), json!({"id": 8, "ok": {}}));
    c.send_raw(b"{\"id\":9}\n");
    assert_eq!(c.reply(9)["error"]["code"], "invalid", "no verb");
}

/// 5.12: the test-control socket plugs and unplugs; 4.1 Events: an unplugged sn: entry stays
/// listed unavailable, a port: entry is removed, a re-plug is devices then state.
#[test]
fn the_test_control_socket_drives_hotplug() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    assert_eq!(
        d.control(&format!("{{\"unplug\":\"{UNKNOWN}\"}}")),
        json!({"ok": true})
    );
    let ev = c.event(T, devices).expect("devices");
    assert_eq!(ids(&ev["devices"]), [A, B]);
    assert_eq!(d.control("{\"plug\":\"unknown\"}"), json!({"ok": UNKNOWN}));
    let ev = c.event(T, devices).unwrap();
    assert_eq!(ids(&ev["devices"]), [A, B, UNKNOWN]);
    let st = c.event(T, state_of(UNKNOWN)).unwrap();
    assert_eq!(st["rev"], 1, "a re-added port: entry starts at rev 1");

    d.control(&format!("{{\"unplug\":\"{B}\"}}"));
    let ev = c.event(T, devices).unwrap();
    assert_eq!(ids(&ev["devices"]), [A, &format!("{B}!"), UNKNOWN]);
    let st = c.event(T, state_of(B)).unwrap();
    assert_eq!(st["state"]["available"], false);
    assert_eq!(
        c.set(B, json!({"led.brightness": 1}))["error"]["code"],
        "unavailable"
    );
    assert_eq!(
        c.get(B, None)["ok"]["state"]["available"],
        false,
        "get from the last state"
    );
    d.control("{\"plug\":\"fake-b\"}");
    let ev = c.event(T, devices).unwrap();
    assert_eq!(ids(&ev["devices"]), [A, B, UNKNOWN]);
    let st = c.event(T, state_of(B)).unwrap();
    assert_eq!(st["state"]["available"], true);
    assert!(c.set(B, json!({"led.brightness": 1}))["ok"].is_object());
}

/// 5.12 and 10.3: `obsbot4linuxd --fake fake-a,fake-b,unknown` serves the fakes on the
/// socket that `OBSBOT4LINUX_SOCKET` names, with the test-control socket beside it, and
/// exits 0 on SIGTERM (5.9).
#[test]
fn the_binary_serves_fake_mode_and_ends_on_sigterm() {
    let dir = temp_dir();
    let socket = dir.join("s.sock");
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_obsbot4linuxd"))
        .args(["--fake", "fake-a,fake-b,unknown"])
        .env("OBSBOT4LINUX_SOCKET", &socket)
        .env_remove("LISTEN_PID")
        .env_remove("LISTEN_FDS")
        .spawn()
        .unwrap();
    assert!(eventually(T, || std::os::unix::net::UnixStream::connect(
        &socket
    )
    .is_ok()));
    let mut c = Client::connect(&socket);
    let ok = c.hello(false, &[]);
    assert_eq!(ids(&ok["devices"]), [A, B, UNKNOWN]);
    let mut test = socket.as_os_str().to_owned();
    test.push(".test");
    assert!(std::path::Path::new(&test).exists());
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(0));
    assert!(!socket.exists(), "a socket it bound is removed at exit");
    let _ = std::fs::remove_dir_all(dir);
}
```

Append to `crates/obsbot4linuxd/src/control.rs` (after writing its two signatures with `todo!()` bodies):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::models::fake::KEYS;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::Ordering;

    #[test]
    fn control_lines_reach_the_fake_bus() {
        let dir = std::env::temp_dir().join(format!("o4ld-ctl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("s.sock");
        let bus = Arc::new(FakeBus::new(&KEYS).unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let h = serve(&socket, bus.clone(), stop.clone()).unwrap();
        let path = control_path(&socket);
        assert!(path.ends_with("s.sock.test"));
        let mut s = UnixStream::connect(&path).unwrap();
        writeln!(s, "{{\"plug\":\"fake-b\"}}").unwrap();
        writeln!(s, "nonsense").unwrap();
        let mut r = BufReader::new(s.try_clone().unwrap());
        let mut a = String::new();
        r.read_line(&mut a).unwrap();
        assert_eq!(a.trim_end(), r#"{"ok":"sn:FAKEB0000002"}"#);
        let mut b = String::new();
        r.read_line(&mut b).unwrap();
        assert!(b.starts_with(r#"{"error":"not JSON"#));
        stop.store(true, Ordering::SeqCst);
        let _ = UnixStream::connect(&path);
        h.join().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test core`
Expected: FAIL: every test panics in `Daemon::start` with `the daemon ended at start-up` (the stub `run` panics).

- [ ] **Step 3: Write the code**

Write `daemon::run`, `control.rs` and `main.rs` as above. Leave `lib.rs` alone: the crate-wide `allow` is removed by b-14, once the chain has wired the last worker feature.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4`
Expected: PASS: every unit test, `tests/harness.rs` and the 9 tests of `tests/core.rs`.

Run: `cargo fmt --all --check && nice -n 19 cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings`
Expected: no output.

Run: `OBSBOT4LINUX_SOCKET=$TMPDIR/o4l.sock nice -n 19 cargo run -q -p obsbot4linuxd -- --fake fake-a,fake-b,unknown` in one terminal and, in another, `printf '%s\n' '{"id":1,"hello":{"protocol":2,"client":"sh 1"}}' | socat - UNIX-CONNECT:$TMPDIR/o4l.sock`; Ctrl-C the daemon.
Expected: one line with `"ok":{"protocol":2,…}` listing `sn:FAKEA0000001`, `sn:FAKEB0000001`, `port:9-3`; the daemon logs one `appeared` line per device and exits 0 on Ctrl-C.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd
git commit -m "obsbot4linuxd: daemon start-up and shutdown, test-control socket, binary (B-1)"
```

When this PR has landed, B-1 is on `rebuild`: report it to the coordinator (tracks C and D start their live tests).

### Task b-11 (key `b-11-tests-requests`): Coalescing; tests: write path, queue rules, limits

**Files:**

- Create: `crates/obsbot4linuxd/tests/requests.rs`, `crates/obsbot4linuxd/tests/limits.rs`
- Modify: `crates/obsbot4linuxd/src/worker.rs` (`serve`), `crates/obsbot4linuxd/src/batch.rs` (the `Coalesced` outcome)
- Modify only where one of this task's tests shows a defect of B-1: any file under `crates/obsbot4linuxd/src/` (the chain is stacked: no other task runs beside this one), with the spec section the test cites, named in the PR text

**Interfaces:**

- Consumes: the harness of b-01 (`tests/common/mod.rs`) and the daemon of b-10 with what the chain added before this task; `obsbot_device::models::fake::Op`; `queue::plan` with its `coalesced` (b-05); `obsbot_proto::validate::{leaves, check_value, Fit}`; `obsbot4linuxd::MAX_CONNECTIONS`.
- Produces: the steps marked **[b-11]** in b-07's algorithms: `serve` passes the real `single_leaf`; `batch` gains the `Coalesced` outcome (steps 1, 2.3, 4 and 9); the named integration tests of spec 10.3 B listed in "Track B acceptance" below.

Stacked on `b-10-daemon`. The feature this task adds: single-leaf coalescing (4.1 rule 9, decision 7). One or two of the tests below pin it; every other test of the two files pins behaviour B-1 already has and runs here for the first time: if one of those fails, it is a defect of B-1, fixed in the module it names in this PR.

- [ ] **Step 1: Write the tests**

Create `crates/obsbot4linuxd/tests/requests.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The write path and the queue rules (spec 10.3 B): rev order and event-before-reply,
//! pipelined requests, shared re-reads, single-leaf coalescing, whole-patch validation, the
//! apply order, `applied`/`failed`, `requires`, live-topic filtering, and a request sending
//! exactly what it names (section 2).

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::{Value, json};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

fn reads_of(d: &Daemon, id: &str, group: &str, skip: usize) -> usize {
    d.ops(id)[skip..]
        .iter()
        .filter(|op| **op == Op::Read(group.into()))
        .count()
}

/// 4.1 rule 2: requests may be pipelined; one device's requests are answered in order.
#[test]
fn pipelined_requests_are_answered_in_request_order() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    for (i, v) in [1, 2, 3, 0].iter().enumerate() {
        c.send(&json!({"id": i, "device": A, "set": {"led.brightness": v}}));
    }
    c.send(&json!({"id": 4, "device": A, "get": ["led"]}));
    let mut order = Vec::new();
    for _ in 0..5 {
        let r = c.line(T).expect("a reply");
        order.push(r["id"].as_i64().unwrap());
        if r["id"] == 4 {
            assert_eq!(
                at(&r["ok"]["state"], "led.brightness"),
                &json!(0),
                "the get sees the batch"
            );
        }
    }
    assert_eq!(order, [0, 1, 2, 3, 4]);
}

/// 4.1 rule 9 and 10.3: N pipelined sets on one connection cause at most two re-reads
/// (fake-b's `beacon` group is read only every 60 s, so every read of it here is a re-read).
#[test]
fn pipelined_sets_share_at_most_two_rereads() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let skip = d.ops(B).len();
    let colors = [
        "green", "blue", "red", "green", "blue", "red", "green", "blue",
    ];
    for (i, color) in colors.iter().enumerate() {
        c.send(&json!({"id": i, "device": B, "set": {"beacon.color": color}}));
    }
    for i in 0..colors.len() {
        assert!(c.reply(i as i64)["ok"].is_object());
    }
    let rereads = reads_of(&d, B, "beacon", skip);
    assert!((1..=2).contains(&rereads), "{rereads} re-reads of beacon");
}

/// 4.1 rule 9: a single-leaf `set` is answered `ok` without being sent when a later queued
/// `set` of the same connection has the same leaf, dotted or nested.
#[test]
fn a_single_leaf_set_is_coalesced_with_a_later_one() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-b", Duration::from_millis(300));
    c.send(&json!({"id": 1, "device": B, "set": {"tracking.mode": "person"}}));
    let started = Op::Apply("tracking.mode".into(), json!("person"));
    assert!(
        eventually(T, || d.ops(B).contains(&started)),
        "request 1 runs"
    );
    let skip = d.ops(B).len();
    c.send(&json!({"id": 2, "device": B, "set": {"beacon.color": "green"}}));
    c.send(&json!({"id": 3, "device": B, "set": {"beacon": {"color": "blue"}}}));
    c.send(&json!({"id": 4, "device": B, "set": {"beacon.color": "nope"}}));
    for id in 1..=4 {
        let r = c.reply(id);
        if id == 4 {
            assert_eq!(r["error"]["code"], "out_of_range");
        } else {
            assert!(r["ok"]["rev"].is_u64(), "{r}");
        }
    }
    d.bus.set_delay("fake-b", Duration::ZERO);
    let applied: Vec<Op> = writes_after(&d, B, skip);
    assert_eq!(
        applied,
        [Op::Apply("beacon.color".into(), json!("blue"))],
        "green is never sent; the refused value does not replace blue"
    );
}

/// 4.1 rule 5: the whole patch is validated first; one bad leaf sends nothing.
#[test]
fn one_bad_leaf_refuses_the_whole_patch() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let skip = d.ops(A).len();
    let r = c.set(A, json!({"led.brightness": 1, "zoom.factor": 9}));
    assert_eq!(r["error"]["code"], "out_of_range");
    assert_eq!(writes_after(&d, A, skip), Vec::<Op>::new());
}

/// 4.2 and 8.8: `requires` is checked against the state with the patch applied, and the
/// model's apply order puts the required path first.
#[test]
fn requires_and_the_apply_order() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    assert_eq!(
        c.set(A, json!({"exposure.time": 20000}))["error"]["code"],
        "conflict"
    );
    let skip = d.ops(A).len();
    let r = c.set(A, json!({"exposure": {"time": 20000, "mode": "manual"}}));
    assert!(r["ok"].is_object(), "{r}");
    assert_eq!(
        writes_after(&d, A, skip),
        [
            Op::Apply("exposure.mode".into(), json!("manual")),
            Op::Apply("exposure.time".into(), json!(20000))
        ]
    );
}

/// The current state inside a batch includes the earlier requests' written leaves, so a
/// pipelined `exposure.mode=manual` lets the next request write `exposure.time`.
#[test]
fn a_pipelined_requirement_counts_within_one_batch() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(200));
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(
        eventually(T, || d.ops(A).contains(&started)),
        "request 1 runs"
    );
    c.send(&json!({"id": 2, "device": A, "set": {"exposure.mode": "manual"}}));
    c.send(&json!({"id": 3, "device": A, "set": {"exposure.time": 20000}}));
    for id in 1..=3 {
        assert!(c.reply(id)["ok"].is_object(), "request {id}");
    }
}

/// 4.1 rule 5: a camera failure midway stops the patch, re-reads, and answers `camera` with
/// `applied`, `failed` and the rev of the re-read state. `fail_next` hits the second apply:
/// it is set once that apply has started and waits out the fake's delay (the fake checks
/// `fail_next` after its delay).
#[test]
fn a_failure_midway_reports_applied_and_failed() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(400));
    c.send(
        &json!({"id": 1, "device": A, "set": {"exposure.mode": "manual", "exposure.time": 20000}}),
    );
    let second = Op::Apply("exposure.time".into(), json!(20000));
    assert!(
        eventually(T, || d.ops(A).contains(&second)),
        "the second apply started"
    );
    d.bus.fail_next(A).unwrap();
    let r = c.reply(1);
    d.bus.set_delay("fake-a", Duration::ZERO);
    assert_eq!(r["error"]["code"], "camera", "{r}");
    assert_eq!(r["error"]["applied"], json!(["exposure.mode"]));
    assert_eq!(r["error"]["failed"], "exposure.time");
    assert!(r["error"]["rev"].is_u64(), "re-read before answering");
}

/// Section 2 and 10.3: a `set` causes at most one `apply` per leaf of its patch and no `act`,
/// a `do` exactly one `act` and no `apply`; apart from reads nothing else reaches the fake.
#[test]
fn a_request_sends_exactly_what_it_names() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    c.set(A, json!({"tracking.mode": "normal"}));
    let cases: Vec<(Value, Vec<Op>)> = vec![
        (
            json!({"device": A, "set": {"led.brightness": 2, "gimbal.invert": true}}),
            vec![
                Op::Apply("gimbal.invert".into(), json!(true)),
                Op::Apply("led.brightness".into(), json!(2)),
            ],
        ),
        (
            json!({"device": A, "do": "gimbal.move", "with": {"pan": 20}}),
            vec![Op::Act(
                "gimbal.move".into(),
                json!({"pan": 20}).as_object().unwrap().clone(),
            )],
        ),
        (
            json!({"device": A, "do": "gimbal.turn", "with": {"pan_speed": 10, "keep": true}}),
            vec![Op::Act(
                "gimbal.turn".into(),
                json!({"pan_speed": 10, "keep": true})
                    .as_object()
                    .unwrap()
                    .clone(),
            )],
        ),
        (
            json!({"device": A, "do": "preset.save", "with": {"slot": 0}}),
            vec![Op::Act(
                "preset.save".into(),
                json!({"slot": 0}).as_object().unwrap().clone(),
            )],
        ),
    ];
    for (mut req, want) in cases {
        let skip = d.ops(A).len();
        let id = c.id();
        req["id"] = json!(id);
        c.send(&req);
        assert!(c.reply(id)["ok"].is_object());
        let mut got = writes_after(&d, A, skip);
        if let Op::Apply(..) = want[0] {
            got.sort_by_key(|op| format!("{op:?}"));
        }
        assert_eq!(got, want, "{req}");
    }
    let state = c.get(A, Some(&["tracking.mode"]));
    assert_eq!(
        at(&state["ok"]["state"], "tracking.mode"),
        &json!("normal"),
        "left alone"
    );
}

/// 4.1 rule 8: a change of volatile leaves that no request caused goes only to connections
/// subscribing its topic. fake-a's `gimbal.move` touches no group: the pose change is found
/// by a periodic read.
#[test]
fn a_volatile_change_reaches_only_its_live_subscribers() {
    let d = Daemon::start();
    let (mut pose, _) = d.hello(true, &["pose"]);
    let (mut plain, _) = d.hello(true, &[]);
    let (mut c, _) = d.hello(false, &[]);
    c.act(A, "gimbal.move", json!({"pan": 30}));
    assert!(
        pose.event(T, |e| state_of(A)(e)
            && at(&e["state"], "gimbal.pan") == &json!(30))
            .is_some()
    );
    assert!(
        plain
            .event(Duration::from_millis(800), state_of(A))
            .is_none(),
        "no event for a connection without the topic"
    );
    c.set(A, json!({"led.brightness": 3}));
    let ev = plain
        .event(T, state_of(A))
        .expect("a non-volatile change reaches everyone");
    assert_eq!(
        at(&ev["state"], "gimbal.pan"),
        &json!(30),
        "with the new pose in it"
    );
}

/// Section 2 and review focus 1: a request sends the value it names. A client's float noise
/// on a stepped leaf (a QML slider, JavaScript arithmetic) is put on its step; a parameter
/// without a step is not rounded to its precision (fake-a's `pan`: precision 0.1).
#[test]
fn a_value_is_sent_as_the_request_names_it() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let skip = d.ops(A).len();
    assert!(c.set(A, json!({"led.brightness": 2.9999999999999996}))["ok"].is_object());
    assert!(c.act(A, "gimbal.move", json!({"pan": 10.04}))["ok"].is_object());
    assert_eq!(
        writes_after(&d, A, skip),
        [
            Op::Apply("led.brightness".into(), json!(3)),
            Op::Act(
                "gimbal.move".into(),
                json!({"pan": 10.04}).as_object().unwrap().clone()
            ),
        ]
    );
    let got = c.get(A, Some(&["led"]));
    assert_eq!(
        got["ok"]["state"]["pending"],
        json!({}),
        "the readback 3 shows the write at once"
    );
}
```

Create `crates/obsbot4linuxd/tests/limits.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The daemon's limits (spec 4.1, 5.1, 10.3 B): the 64 KiB request line, 64 connections, and
//! a subscriber that does not read.

mod common;

use common::*;
use serde_json::json;
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

#[test]
fn a_request_line_of_64_kib_is_read_and_one_byte_more_closes_the_connection() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let mut line = br#"{"id":1,"live":[]}"#.to_vec();
    line.resize(obsbot_proto::MAX_REQUEST_LINE, b' ');
    line.push(b'\n');
    c.send_raw(&line);
    assert_eq!(c.reply(1), json!({"id": 1, "ok": {}}));
    let mut long = vec![b' '; obsbot_proto::MAX_REQUEST_LINE + 1];
    long.push(b'\n');
    c.send_raw(&long);
    assert!(c.closed_within(T));
}

#[test]
fn the_65th_connection_is_closed_at_once() {
    let d = Daemon::start();
    let mut open: Vec<Client> = (0..obsbot4linuxd::MAX_CONNECTIONS)
        .map(|_| d.connect())
        .collect();
    let (mut first, _) = (open.remove(0), ());
    first.hello(false, &[]);
    let mut extra = d.connect();
    assert!(extra.closed_within(Duration::from_secs(2)), "the 65th");
    first.close();
    assert!(eventually(T, || {
        let mut c = d.connect();
        c.send(&json!({"id": 1, "hello": {"protocol": 2, "client": "late"}}));
        c.line(Duration::from_millis(500))
            .is_some_and(|r| r["ok"].is_object())
    }));
    drop(open);
}

/// 5.1: when a connection's 64-line outbound queue is full, the connection is closed; the
/// daemon and the other connections go on.
#[test]
fn a_subscriber_that_does_not_read_is_disconnected() {
    let d = Daemon::start();
    let (mut slow, _) = d.hello(true, &[]);
    let (mut c, _) = d.hello(false, &[]);
    for i in 0..600 {
        let r = c.set(A, json!({"led.brightness": i % 4}));
        assert!(r["ok"].is_object());
    }
    assert!(
        slow.closed_within(Duration::from_secs(10)),
        "closed for its full queue"
    );
    assert!(c.get(A, None)["ok"].is_object(), "the rest goes on");
}
```

- [ ] **Step 2: Run them to see the feature fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test requests --test limits`
Expected: `requests::a_single_leaf_set_is_coalesced_with_a_later_one` fails (`green` is sent). Every other test passes; a failure of another test is a B-1 defect: fix the module it names, say so in the PR text, rerun.

- [ ] **Step 3: Add the feature**

Write the steps marked **[b-11]** in b-07's algorithms: `serve` passes the real `single_leaf`; `batch` gains the `Coalesced` outcome (steps 1, 2.3, 4 and 9).

- [ ] **Step 4: Run them to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test requests --test limits`
Expected: PASS: `requests` 10, `limits` 3.

- [ ] **Step 5: Check the whole crate**

Run: `cargo fmt --all --check && nice -n 19 cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings && nice -n 19 cargo test -p obsbot4linuxd -j 4`
Expected: no fmt or clippy output; every test passes.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot4linuxd
git commit -m "obsbot4linuxd: single-leaf coalescing; integration tests of the write path, the queue rules and the limits"
```

### Task b-12 (key `b-12-tests-asleep-startup`): Restart quiet time and wake read; tests: asleep, wake, restart, start-up

**Files:**

- Create: `crates/obsbot4linuxd/tests/asleep.rs`, `crates/obsbot4linuxd/tests/startup.rs`
- Modify: `crates/obsbot4linuxd/src/worker.rs` (loop, `serve`, `tick`, `wake`), `crates/obsbot4linuxd/src/batch.rs` (restart)
- Modify only where one of this task's tests shows a defect of B-1: any file under `crates/obsbot4linuxd/src/` (the chain is stacked: no other task runs beside this one), with the spec section the test cites, named in the PR text

**Interfaces:**

- Consumes: the harness of b-01 (`tests/common/mod.rs`) and the daemon of b-10 with what the chain added before this task; `obsbot_device::models::fake::Op`; `schedule::{Schedule::wake_groups, Schedule::woke}` (b-03); `obsbot_proto::Warning`.
- Produces: the steps marked **[b-12]** in b-07's algorithms: `serve` passes the real `restarts`; the loop's `quiet_until` step (7.3); `batch` gains the `restart` flag (steps 1, 2.3, 2.4) and step 5 (answer on acceptance, then quiet); `wake()` (without its **[b-14]** identity line), called from `tick` step 6 and `batch` step 10; the named integration tests of spec 10.3 B listed in "Track B acceptance" below.

Stacked on `b-11-tests-requests`. The feature this task adds: the restart quiet time (5.6 "Restart", decision 8) and the wake read (5.7 "Wake and retries"). One or two of the tests below pin it; every other test of the two files pins behaviour B-1 already has and runs here for the first time: if one of those fails, it is a defect of B-1, fixed in the module it names in this PR.

- [ ] **Step 1: Write the tests**

Create `crates/obsbot4linuxd/tests/asleep.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Asleep, wake and retries (spec 5.7, 10.3 B): `asleep: ignored` refused, `asleep: unknown`
//! written and pending until the first read after wake, the wake read of `unread` groups, the
//! retry of a `touched` read that failed or could not run, and the settle bound.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::json;
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

fn asleep(v: bool) -> impl Fn(&serde_json::Value) -> bool {
    move |e| state_of(B)(e) && e["state"]["asleep"] == json!(v)
}

#[test]
fn asleep_ignored_is_refused_and_unknown_is_pending_until_wake() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.control(&format!("{{\"sleep\":\"{B}\"}}"));
    c.event(T, asleep(true))
        .expect("the status read shows asleep");
    let skip = d.ops(B).len();
    assert_eq!(
        c.set(B, json!({"zoom.factor": 1.5}))["error"]["code"],
        "asleep"
    );
    let r = c.set(B, json!({"led.brightness": 4}));
    assert!(r["ok"].is_object(), "{r}");
    let ev = c
        .events
        .iter()
        .rev()
        .find(|e| state_of(B)(e))
        .unwrap()
        .clone();
    assert_eq!(ev["state"]["pending"], json!({"led.brightness": 4}));
    assert!(
        !d.ops(B)[skip..].contains(&Op::Read("led".into())),
        "no read of a group the camera cannot answer asleep"
    );
    std::thread::sleep(Duration::from_millis(1500));
    let still = c.get(B, Some(&["led"]));
    assert_eq!(
        still["ok"]["state"]["pending"],
        json!({"led.brightness": 4}),
        "asleep, the bound of 1000 ms has not started"
    );
    d.control(&format!("{{\"wake\":\"{B}\"}}"));
    let ev = c
        .event(T, |e| state_of(B)(e) && e["state"]["pending"] == json!({}))
        .expect("the first read after wake shows it");
    assert_eq!(at(&ev["state"], "led.brightness"), &json!(4));
    assert_eq!(ev["state"]["asleep"], false);
}

/// fake-b stores `exposure.time` rounded down to 100: the readback never shows 1234, and the
/// settle bound (1000 ms) ends `pending` at a read of its group; the camera's value stands.
#[test]
fn the_settle_bound_ends_pending_and_the_camera_value_stands() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    let r = c.set(B, json!({"exposure": {"mode": "manual", "time": 1234}}));
    assert!(r["ok"].is_object(), "{r}");
    let ev = c
        .events
        .iter()
        .rev()
        .find(|e| state_of(B)(e))
        .unwrap()
        .clone();
    assert_eq!(ev["state"]["pending"], json!({"exposure.time": 1234}));
    let t0 = std::time::Instant::now();
    let done = c
        .event(T, |e| state_of(B)(e) && e["state"]["pending"] == json!({}))
        .expect("the bound ends it");
    assert!(
        t0.elapsed() >= Duration::from_millis(800),
        "not before the bound"
    );
    assert_eq!(at(&done["state"], "exposure.time"), &json!(1200));
}

/// 5.7: a `touched` group whose re-read could not run while asleep is read again at the
/// wake, with the other groups that have a leaf in `unread`.
#[test]
fn the_wake_read_reads_the_touched_group_the_camera_could_not_answer() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    assert!(c.act(B, "preset.save", json!({"slot": 0, "name": "Desk"}))["ok"].is_object());
    d.control(&format!("{{\"sleep\":\"{B}\"}}"));
    c.event(T, asleep(true)).unwrap();
    let r = c.act(B, "preset.delete", json!({"slot": 0}));
    assert!(r["ok"].is_object(), "asleep: unknown is sent: {r}");
    let st = c.get(B, None);
    let unread = st["ok"]["state"]["unread"].clone();
    assert!(
        unread.as_array().unwrap().contains(&json!("presets")),
        "{unread}"
    );
    assert_eq!(
        c.act(B, "preset.delete", json!({"slot": 0}))["error"]["code"],
        "conflict",
        "a slot action while its list is unread"
    );
    d.control(&format!("{{\"wake\":\"{B}\"}}"));
    let ev = c
        .event(T, |e| state_of(B)(e) && e["state"]["unread"] == json!([]))
        .expect("the wake read");
    assert_eq!(ev["state"]["presets"], json!([]), "the delete shows");
}

/// 5.7: a `touched` read that failed is read again with the next tick of the 60 s tier
/// (3 s under the test timing). `fail_next` is set once the re-read has started; the fake
/// checks it after its delay.
#[test]
fn a_failed_touched_read_is_read_again_with_the_minute_tier() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.bus.set_delay("fake-b", Duration::from_millis(300));
    c.send(&json!({"id": 1, "device": B, "do": "preset.save", "with": {"slot": 1}}));
    let act = d.ops(B).len();
    assert!(eventually(T, || d.ops(B)[act..].contains(&Op::Read("presets".into()))));
    d.bus.fail_next(B).unwrap();
    assert!(c.reply(1)["ok"].is_object(), "the action was accepted");
    d.bus.set_delay("fake-b", Duration::ZERO);
    let st = c.get(B, Some(&["presets"]));
    assert!(
        st["ok"]["state"]["unread"]
            .as_array()
            .unwrap()
            .contains(&json!("presets"))
    );
    let ev = c
        .event(Duration::from_secs(8), |e| {
            state_of(B)(e)
                && !e["state"]["unread"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("presets"))
        })
        .expect("read again");
    assert_eq!(ev["state"]["presets"][0]["slot"], 1);
}

/// 5.6 "Restart": a write with `warning: restart` is answered once the camera accepted it,
/// without a re-read; its leaf is pending; nothing else is sent until the device goes (the
/// request pipelined behind it is answered as for an unavailable device), which empties
/// `pending`; the device comes back as ordinary hotplug.
#[test]
fn a_restart_write_is_answered_on_acceptance_and_then_nothing_is_sent() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    let skip = d.ops(A).len();
    let mut lines = Vec::new();
    for req in [
        json!({"id": 1, "device": A, "set": {"audio.enabled": true}}),
        json!({"id": 2, "device": A, "set": {"led.brightness": 1}}),
    ] {
        lines.extend_from_slice(req.to_string().as_bytes());
        lines.push(b'\n');
    }
    c.send_raw(&lines);
    let r = c.reply(1);
    assert!(r["ok"]["rev"].is_u64(), "{r}");
    let ev = c
        .events
        .iter()
        .rev()
        .find(|e| state_of(A)(e))
        .unwrap()
        .clone();
    assert_eq!(ev["state"]["pending"], json!({"audio.enabled": true}));
    assert_eq!(
        c.reply(2)["error"]["code"],
        "unavailable",
        "not sent after the restart write; answered when the device went"
    );
    let gone = c
        .event(T, |e| state_of(A)(e) && e["state"]["available"] == false)
        .unwrap();
    assert_eq!(gone["state"]["pending"], json!({}));
    c.event(T, |e| state_of(A)(e) && e["state"]["available"] == true)
        .expect("back");
    let after: Vec<Op> = d.ops(A)[skip..].to_vec();
    let write = Op::Apply("audio.enabled".into(), json!(true));
    let sent = after.iter().position(|op| *op == write).expect("sent");
    let open = after
        .iter()
        .position(|op| *op == Op::Open)
        .expect("re-opened");
    assert!(sent < open, "{after:?}");
    assert!(
        after[sent + 1..open].is_empty(),
        "nothing after the restart write until the device went: {after:?}"
    );
}
```

Create `crates/obsbot4linuxd/tests/startup.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Start-up (spec 4.1 `hello` timing, 5.2): `hello` waits for the first enumeration and the
//! first read attempts, at most 15 s; a device still reading is listed with its leaves `null`
//! and in `unread`; requests that follow `hello` are handled after its reply.

mod common;

use common::*;
use obsbot_device::models::fake::KEYS;
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn hello_waits_for_the_first_read_attempts() {
    let d = Daemon::start_with(&KEYS, fast(), |bus| {
        bus.set_delay("fake-b", Duration::from_millis(150))
    });
    let t0 = Instant::now();
    let (_c, ok) = d.hello(false, &[]);
    assert!(
        t0.elapsed() >= Duration::from_millis(600),
        "fake-b's first read takes 1.2 s"
    );
    let b = &ok["states"][B]["state"];
    assert_eq!(at(b, "led.brightness"), &json!(0), "read, not null");
    assert_eq!(b["unread"], json!([]));
}

/// 4.1 `hello` timing: answered at the latest 15 s after it arrived; the slow device is listed
/// with its leaves null and in `unread`; a request pipelined behind the `hello` is answered
/// after the reply.
#[test]
fn hello_is_answered_by_its_deadline_while_a_first_read_is_slow() {
    let d = Daemon::start_with(&KEYS, fast(), |bus| {
        bus.set_delay("fake-a", Duration::from_secs(20))
    });
    let mut c = d.connect();
    let t0 = Instant::now();
    c.send(&json!({"id": 1, "hello": {"protocol": 2, "client": "test 1.0", "subscribe": false}}));
    c.send(&json!({"id": 2, "device": B, "get": ["led"]}));
    let first = c.line(Duration::from_secs(20)).expect("the hello reply");
    let took = t0.elapsed();
    assert_eq!(first["id"], 1, "the reply comes first");
    assert!(
        took >= Duration::from_secs(14) && took < Duration::from_millis(15_500),
        "{took:?}"
    );
    let ok = &first["ok"];
    assert_eq!(
        ids(&ok["devices"]),
        ["port:9-1", B, UNKNOWN],
        "no identity answered yet"
    );
    let a = &ok["states"]["port:9-1"]["state"];
    assert_eq!(at(a, "led.brightness"), &json!(null));
    assert!(
        a["unread"]
            .as_array()
            .unwrap()
            .contains(&json!("led.brightness"))
    );
    let second = c.line(Duration::from_secs(1)).expect("the get");
    assert_eq!(second["id"], 2);
    assert!(second["ok"].is_object());
}
```

- [ ] **Step 2: Run them to see the feature fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test asleep --test startup`
Expected: `asleep::a_restart_write_is_answered_on_acceptance_and_then_nothing_is_sent` fails: without the quiet time the request pipelined behind the restart write is sent into the restarting fake and answered `camera`, not `unavailable` (should the registry end the worker first, it is answered `unavailable` already; the review then checks the marked steps). The wake-read tests may already pass on B-1, because a `touched` group the camera could not answer is also read again with the minute tier (3 s here); the review checks `wake()` against the marked steps. Every other test passes; a failure of another test is a B-1 defect: fix the module it names, say so in the PR text, rerun.

- [ ] **Step 3: Add the feature**

Write the steps marked **[b-12]** in b-07's algorithms: `serve` passes the real `restarts`; the loop's `quiet_until` step (7.3); `batch` gains the `restart` flag (steps 1, 2.3, 2.4) and step 5 (answer on acceptance, then quiet); `wake()` (without its **[b-14]** identity line), called from `tick` step 6 and `batch` step 10.

- [ ] **Step 4: Run them to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test asleep --test startup`
Expected: PASS: `asleep` 5, `startup` 2 (about 16 s: one test waits out the 15 s `hello` deadline).

- [ ] **Step 5: Check the whole crate**

Run: `cargo fmt --all --check && nice -n 19 cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings && nice -n 19 cargo test -p obsbot4linuxd -j 4`
Expected: no fmt or clippy output; every test passes.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot4linuxd
git commit -m "obsbot4linuxd: restart quiet time and wake read; integration tests of asleep, wake, retries, restart and start-up"
```

### Task b-13 (key `b-13-tests-deadline-motion`): Motion record and own stops; tests: deadline, busy, unplug, stops and turns

**Files:**

- Create: `crates/obsbot4linuxd/tests/deadline.rs`, `crates/obsbot4linuxd/tests/motion.rs`
- Modify: `crates/obsbot4linuxd/src/batch.rs` (`record_act`), `crates/obsbot4linuxd/src/worker.rs` (shutdown)
- Modify only where one of this task's tests shows a defect of B-1: any file under `crates/obsbot4linuxd/src/` (the chain is stacked: no other task runs beside this one), with the spec section the test cites, named in the PR text

**Interfaces:**

- Consumes: the harness of b-01 (`tests/common/mod.rs`) and the daemon of b-10 with what the chain added before this task; `obsbot_device::models::fake::Op`; `queue::{DeviceQueue::record_act, Record}` (b-05); `Motion::Continuing` (W0).
- Produces: the steps marked **[b-13]** in b-07's algorithms: `batch` step 2.4 calls `queue.record_act` with `keep`; the loop's shutdown step (7.2) sends the queued own stops with the own-stop context; the named integration tests of spec 10.3 B listed in "Track B acceptance" below.

Stacked on `b-12-tests-asleep-startup`. The feature this task adds: the continuing-motion record and the daemon's own stops at shutdown (5.6 "Continuing motion", 5.9, decision 14). One or two of the tests below pin it; every other test of the two files pins behaviour B-1 already has and runs here for the first time: if one of those fails, it is a defect of B-1, fixed in the module it names in this PR.

- [ ] **Step 1: Write the tests**

Create `crates/obsbot4linuxd/tests/deadline.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The queue's limits (spec 4.1 rule 9, 5.6, 10.3 B): the 15 s deadline, `busy` at 32,
//! requests of a closed connection dropped, and the queue of a device that goes.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::json;
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

/// 4.1 rule 9: every device request is answered within 15 s of its arrival, started or not.
#[test]
fn every_request_is_answered_within_the_deadline() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_secs(20));
    let t0 = Instant::now();
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    c.send(&json!({"id": 2, "device": A, "get": ["led"]}));
    for id in [1, 2] {
        let r = c.reply(id);
        assert_eq!(r["error"]["code"], "camera", "{r}");
    }
    let took = t0.elapsed();
    assert!(took >= Duration::from_secs(14), "{took:?}");
    assert!(took < Duration::from_millis(15_500), "{took:?}");
}

/// 4.1 `busy`: 32 requests are already queued for the device; answered at once.
#[test]
fn the_request_after_32_queued_ones_is_busy() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(1500));
    c.send(&json!({"id": 0, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(eventually(T, || d.ops(A).contains(&started)));
    for id in 1..=32 {
        c.send(&json!({"id": id, "device": A, "get": ["led"]}));
    }
    let t0 = Instant::now();
    c.send(&json!({"id": 33, "device": A, "get": ["led"]}));
    let first = c.line(T).expect("a reply");
    assert_eq!(first["id"], 33, "answered before the queue moves: {first}");
    assert_eq!(first["error"]["code"], "busy");
    assert!(t0.elapsed() < Duration::from_millis(500));
    d.bus.set_delay("fake-a", Duration::ZERO);
    for id in 0..=32 {
        assert!(c.reply(id)["ok"].is_object(), "{id}");
    }
}

/// 4.1 rule 9: a request whose connection closed before the worker started it is dropped.
#[test]
fn queued_requests_of_a_closed_connection_are_never_sent() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(500));
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(eventually(T, || d.ops(A).contains(&started)));
    c.send(&json!({"id": 2, "device": A, "set": {"gimbal.invert": true}}));
    c.send(&json!({"id": 3, "device": A, "do": "gimbal.move", "with": {"pan": 10}}));
    std::thread::sleep(Duration::from_millis(50));
    c.close();
    std::thread::sleep(Duration::from_millis(1500));
    d.bus.set_delay("fake-a", Duration::ZERO);
    let ops = d.ops(A);
    assert!(ops.contains(&started), "a started request runs to the end");
    assert!(
        !ops.iter()
            .any(|op| matches!(op, Op::Apply(l, _) if l == "gimbal.invert"))
    );
    assert!(
        !ops.iter()
            .any(|op| matches!(op, Op::Act(a, _) if a == "gimbal.move"))
    );
}

/// 4.1 rule 9 for requests already taken into a batch: the three writes arrive in one write
/// while a slow poll runs, so the worker takes them as one batch; the connection closes during
/// the first apply, and the other two are never sent.
#[test]
fn a_batch_skips_the_requests_of_a_connection_that_closed() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(400));
    let skip = d.ops(A).len();
    assert!(
        eventually(T, || d.ops(A)[skip..]
            .iter()
            .any(|op| matches!(op, Op::Read(_)))),
        "a slow poll is in flight"
    );
    let mut lines = Vec::new();
    for req in [
        json!({"id": 1, "device": A, "set": {"gimbal.invert": true}}),
        json!({"id": 2, "device": A, "do": "gimbal.move", "with": {"pan": 10}}),
        json!({"id": 3, "device": A, "set": {"led.brightness": 2}}),
    ] {
        lines.extend_from_slice(req.to_string().as_bytes());
        lines.push(b'\n');
    }
    c.send_raw(&lines);
    let first = Op::Apply("gimbal.invert".into(), json!(true));
    assert!(
        eventually(T, || d.ops(A)[skip..].contains(&first)),
        "request 1 runs"
    );
    c.close();
    std::thread::sleep(Duration::from_millis(1500));
    d.bus.set_delay("fake-a", Duration::ZERO);
    assert_eq!(writes_after(&d, A, skip), [first]);
}

/// 5.6 "Unavailable": the started request is answered `camera` without `rev`, the queued
/// ones as for an unavailable device; a re-plug starts a new worker with an empty queue.
#[test]
fn the_queue_of_a_device_that_goes_is_answered_and_a_replug_starts_afresh() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_secs(2));
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(eventually(T, || d.ops(A).contains(&started)));
    c.send(&json!({"id": 2, "device": A, "set": {"led.brightness": 2}}));
    c.send(&json!({"id": 3, "device": A, "get": ["led"]}));
    let t0 = Instant::now();
    d.control(&format!("{{\"unplug\":\"{A}\"}}"));
    let first = c.reply(1);
    assert_eq!(first["error"]["code"], "camera");
    assert!(
        first["error"].get("rev").is_none(),
        "nothing was re-read: {first}"
    );
    assert_eq!(first["error"]["failed"], "led.brightness");
    assert_eq!(c.reply(2)["error"]["code"], "unavailable");
    assert_eq!(c.reply(3)["ok"]["state"]["available"], false);
    assert!(
        t0.elapsed() < Duration::from_millis(1500),
        "not after the fake's delay"
    );
    d.bus.set_delay("fake-a", Duration::ZERO);
    d.control("{\"plug\":\"fake-a\"}");
    assert!(eventually(T, || {
        let mut probe = d.connect();
        probe.hello(false, &[]);
        probe.set(A, json!({"led.brightness": 3}))["ok"].is_object()
    }));
}
```

Create `crates/obsbot4linuxd/tests/motion.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Stops and continuing motion (spec 4.1 rule 2, 5.6, 5.9, 15.1 rows 88 and 89): a stop
//! overtakes the queue and cuts a group read; a `gimbal.turn` without `keep: true` is stopped
//! when its connection closes, a shutdown included; no other stop is ever sent: not when the
//! camera falls asleep, not for a `keep: true` turn.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

fn stops(ops: &[Op]) -> usize {
    ops.iter()
        .filter(|op| matches!(op, Op::Act(a, _) if a == "gimbal.stop"))
        .count()
}

fn turn(keep: bool) -> Value {
    if keep {
        json!({"pan_speed": 10, "keep": true})
    } else {
        json!({"pan_speed": 10})
    }
}

#[test]
fn a_stop_overtakes_the_queued_requests() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(300));
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(eventually(T, || d.ops(A).contains(&started)));
    c.send(&json!({"id": 2, "device": A, "set": {"gimbal.invert": true}}));
    c.send(&json!({"id": 3, "device": A, "do": "gimbal.move", "with": {"pan": 5}}));
    c.send(&json!({"id": 4, "device": A, "do": "gimbal.stop"}));
    let order: Vec<i64> = (0..4)
        .map(|_| c.line(T).unwrap()["id"].as_i64().unwrap())
        .collect();
    assert_eq!(order, [1, 4, 2, 3]);
    let ops = d.ops(A);
    let at = |want: &dyn Fn(&Op) -> bool| ops.iter().position(want).unwrap();
    assert!(
        at(&|op| matches!(op, Op::Act(a, _) if a == "gimbal.stop"))
            < at(&|op| matches!(op, Op::Apply(l, _) if l == "gimbal.invert"))
    );
}

/// 5.6 "Stops": a group read in progress ends at the next frame boundary.
#[test]
fn a_stop_cuts_a_group_read_in_progress() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_secs(3));
    let skip = d.ops(A).len();
    assert!(eventually(T, || d.ops(A)[skip..]
        .iter()
        .any(|op| matches!(op, Op::Read(_)))));
    d.bus.set_delay("fake-a", Duration::ZERO);
    let t0 = Instant::now();
    let r = c.act(A, "gimbal.stop", json!({}));
    assert!(r["ok"].is_object(), "{r}");
    assert!(
        t0.elapsed() < Duration::from_secs(1),
        "not after the 3 s read"
    );
}

#[test]
fn a_turn_without_keep_is_stopped_when_its_connection_closes() {
    let d = Daemon::start();
    let (mut x, _) = d.hello(false, &[]);
    assert!(x.act(A, "gimbal.turn", turn(false))["ok"].is_object());
    let skip = d.ops(A).len();
    x.close();
    assert!(
        eventually(T, || stops(&d.ops(A)[skip..]) == 1),
        "one stop for it"
    );
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(stops(&d.ops(A)[skip..]), 1);
    assert_eq!(
        d.ops(A)[skip..].iter().find(|op| matches!(op, Op::Act(..))),
        Some(&Op::Act("gimbal.stop".into(), Default::default())),
        "the stop is sent with no parameters"
    );
}

/// 15.1 row 89: a `keep: true` turn goes on; the daemon sends no stop for it.
#[test]
fn a_keep_turn_is_never_stopped_by_the_daemon() {
    let d = Daemon::start();
    let (mut y, _) = d.hello(false, &[]);
    assert!(y.act(A, "gimbal.turn", turn(true))["ok"].is_object());
    let skip = d.ops(A).len();
    y.close();
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(stops(&d.ops(A)[skip..]), 0);
}

/// 5.6: any other motion action of the device clears the record, so closing the turning
/// connection does not stop the other client's motion.
#[test]
fn another_motion_clears_the_turn_record() {
    let d = Daemon::start();
    let (mut x, _) = d.hello(false, &[]);
    let (mut y, _) = d.hello(false, &[]);
    assert!(x.act(A, "gimbal.turn", turn(false))["ok"].is_object());
    assert!(y.act(A, "zoom.move", json!({"factor": 2, "speed": "medium"}))["ok"].is_object());
    let skip = d.ops(A).len();
    x.close();
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(stops(&d.ops(A)[skip..]), 0);
}

/// 15.1 row 89 and CAM-7: the daemon sends no stop when the camera falls asleep during a turn.
#[test]
fn no_stop_when_the_camera_falls_asleep_during_a_turn() {
    let d = Daemon::start();
    let (mut x, _) = d.hello(true, &[]);
    assert!(x.act(A, "gimbal.turn", turn(false))["ok"].is_object());
    let skip = d.ops(A).len();
    d.control(&format!("{{\"sleep\":\"{A}\"}}"));
    x.event(T, |e| state_of(A)(e) && e["state"]["asleep"] == true)
        .unwrap();
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(stops(&d.ops(A)[skip..]), 0);
}

/// 5.9: a shutdown closes the connections, so a turn without `keep` gets its stop; a
/// `keep: true` turn goes on.
#[test]
fn shutdown_stops_a_turn_without_keep_only() {
    for (keep, want) in [(false, 1), (true, 0)] {
        let d = Daemon::start();
        let (mut x, _) = d.hello(false, &[]);
        assert!(x.act(A, "gimbal.turn", turn(keep))["ok"].is_object());
        let skip = d.ops(A).len();
        let bus = d.bus.clone();
        d.stop().expect("a clean stop");
        assert_eq!(stops(&bus.ops(A)[skip..]), want, "keep: {keep}");
    }
}
```

- [ ] **Step 2: Run them to see the feature fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test deadline --test motion`
Expected: `motion::a_turn_without_keep_is_stopped_when_its_connection_closes` and `motion::shutdown_stops_a_turn_without_keep_only` fail (no stop is sent). Every other test passes; a failure of another test is a B-1 defect: fix the module it names, say so in the PR text, rerun.

- [ ] **Step 3: Add the feature**

Write the steps marked **[b-13]** in b-07's algorithms: `batch` step 2.4 calls `queue.record_act` with `keep`; the loop's shutdown step (7.2) sends the queued own stops with the own-stop context.

- [ ] **Step 4: Run them to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test deadline --test motion`
Expected: PASS: `deadline` 5 (about 15 s: one test waits out the request deadline), `motion` 7.

- [ ] **Step 5: Check the whole crate**

Run: `cargo fmt --all --check && nice -n 19 cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings && nice -n 19 cargo test -p obsbot4linuxd -j 4`
Expected: no fmt or clippy output; every test passes.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot4linuxd
git commit -m "obsbot4linuxd: continuing-motion record and own stops; integration tests of the deadline, busy, unplug, stops and turns"
```

### Task b-14 (key `b-14-tests-hotplug-process`): Identity attempts 2 and 3; tests: hotplug, hand-over, identity, process

**Files:**

- Create: `crates/obsbot4linuxd/tests/hotplug.rs`, `crates/obsbot4linuxd/tests/process.rs`
- Modify: `crates/obsbot4linuxd/src/worker.rs` (`tick`, `wake`), `crates/obsbot4linuxd/src/lib.rs` (the `allow` line)
- Modify only where one of this task's tests shows a defect of B-1: any file under `crates/obsbot4linuxd/src/` (the chain is stacked: no other task runs beside this one), with the spec section the test cites, named in the PR text

**Interfaces:**

- Consumes: the harness of b-01 (`tests/common/mod.rs`) and the daemon of b-10 with what the chain added before this task; `obsbot_device::models::fake::Op`; `identity::Series::{at_tick, at_wake}` (b-04); `obsbot4linuxd::{Config, run_with}`; `process.rs` also runs the binary (`env!("CARGO_BIN_EXE_obsbot4linuxd")`) and uses `nix` and `sh`.
- Produces: the steps marked **[b-14]** in b-07's algorithms: `tick` step 2 (`series.at_tick`) and the first line of `wake()` (`series.at_wake`). Then remove the skeleton's `#![allow(dead_code, unused_variables, unused_imports)]` line and the comment above it from `lib.rs`, and fix every warning that appears (each is a stub leftover or a real omission; the chain has wired every feature); the named integration tests of spec 10.3 B listed in "Track B acceptance" below.

Stacked on `b-13-tests-deadline-motion`. The feature this task adds: identity attempts 2 and 3 and the wake series (5.5, decision 5). One or two of the tests below pin it; every other test of the two files pins behaviour B-1 already has and runs here for the first time: if one of those fails, it is a defect of B-1, fixed in the module it names in this PR.

- [ ] **Step 1: Write the tests**

Create `crates/obsbot4linuxd/tests/hotplug.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Hotplug and identity (spec 4.1 Events, 5.3 to 5.5, 10.3 B): a device-number change as an
//! unplug plus a plug, the `port:` → `sn:` hand-over with `replaced_by` and its rev, an
//! identity series that ends at the first complete answer, and requests for an unavailable
//! device answered at once.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

/// The events of one device id and the device lists, in arrival order, until `pred` holds.
fn collect_until(c: &mut Client, pred: impl Fn(&Value) -> bool) -> Vec<Value> {
    let mut out = Vec::new();
    let until = Instant::now() + T;
    while Instant::now() < until {
        let Some(ev) = c.line(Duration::from_millis(200)) else {
            continue;
        };
        let stop = pred(&ev);
        out.push(ev);
        if stop {
            return out;
        }
    }
    panic!("the expected event did not come: {out:?}");
}

fn summary(events: &[Value], id: &str) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e["event"].as_str() {
            Some("devices") => Some(format!("devices {}", ids(&e["devices"]).join(","))),
            Some("state") if e["device"] == id => {
                Some(format!("state {}", e["state"]["available"]))
            }
            _ => None,
        })
        .collect()
}

/// 5.3: a device-number change under the same path is handled as an unplug followed by a plug,
/// even when no scan saw the path missing; the sn: entry is kept and taken back.
#[test]
fn a_new_device_number_is_an_unplug_and_a_plug() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.control(&format!("{{\"unplug\":\"{A}\"}}"));
    d.control("{\"plug\":\"fake-a\"}");
    let events = collect_until(&mut c, |e| {
        state_of(A)(e) && e["state"]["available"] == true
    });
    assert_eq!(
        summary(&events, A),
        [
            format!("devices {A}!,{B},{UNKNOWN}"),
            "state false".to_string(),
            format!("devices {A},{B},{UNKNOWN}"),
            "state true".to_string(),
        ]
    );
}

/// 5.5 and the golden hotplug.jsonl: the first identity attempt after the re-plug fails, the
/// device enters as `port:`; the next attempt answers the serial: two devices events and a
/// state event in a row, the old id answered `unknown_device` from then on.
#[test]
fn the_hand_over_from_port_to_sn() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.control(&format!("{{\"unplug\":\"{A}\"}}"));
    let off = c
        .event(T, |e| state_of(A)(e) && e["state"]["available"] == false)
        .unwrap();
    let old_rev = off["rev"].as_u64().unwrap();
    d.bus.fail_next(A).unwrap();
    d.control("{\"plug\":\"fake-a\"}");
    let port = c
        .event(T, state_of("port:9-1"))
        .expect("enters as port:9-1");
    assert_eq!(port["rev"], 1);
    let events = collect_until(&mut c, state_of(A));
    let tail: Vec<&Value> = events.iter().rev().take(3).collect();
    let (handed, without, state) = (tail[2], tail[1], tail[0]);
    assert!(devices(handed) && devices(without), "{events:?}");
    let old = handed["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|dev| dev["id"] == "port:9-1")
        .expect("the old entry in event 1");
    assert_eq!(old["available"], false);
    assert_eq!(old["replaced_by"], A);
    assert_eq!(ids(&without["devices"]), [A, B, UNKNOWN]);
    assert!(
        state["rev"].as_u64().unwrap() > old_rev.max(1),
        "the rev never goes down"
    );
    assert_eq!(state["state"]["available"], true);
    assert_eq!(c.get("port:9-1", None)["error"]["code"], "unknown_device");
}

/// 10.3: an identity series ends at the first complete answer: one attempt, no more.
#[test]
fn an_identity_series_ends_at_the_first_complete_answer() {
    let d = Daemon::start();
    let (_c, _) = d.hello(false, &[]);
    std::thread::sleep(Duration::from_millis(1600));
    let attempts = d
        .ops(A)
        .iter()
        .filter(|op| matches!(op, Op::Identity(_)))
        .count();
    assert_eq!(attempts, 1, "three 500 ms identity ticks passed");
}

/// 4.1 rule 2: requests for an unavailable device are answered by the reader at once, even
/// while other devices' workers are busy.
#[test]
fn requests_for_an_unavailable_device_are_answered_at_once() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.control(&format!("{{\"unplug\":\"{B}\"}}"));
    c.event(T, |e| state_of(B)(e) && e["state"]["available"] == false)
        .unwrap();
    d.bus.set_delay("fake-a", Duration::from_secs(3));
    let t0 = Instant::now();
    assert_eq!(
        c.set(B, json!({"led.brightness": 1}))["error"]["code"],
        "unavailable"
    );
    assert_eq!(c.act(B, "sleep", json!({}))["error"]["code"], "unavailable");
    assert_eq!(c.get(B, Some(&["led"]))["ok"]["state"]["available"], false);
    assert!(t0.elapsed() < Duration::from_millis(300));
    d.bus.set_delay("fake-a", Duration::ZERO);
}
```

Create `crates/obsbot4linuxd/tests/process.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The process (spec 5.9, 5.10, 11): the single-instance lock, socket activation through
//! `LISTEN_FDS`, the exit codes of the binary and a clean stop.

mod common;

use common::*;
use obsbot4linuxd::{Config, run_with};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixListener;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

/// 5.9: a second daemon on the same socket path exits with a message naming the lock file.
#[test]
fn a_second_daemon_on_the_same_socket_is_refused_by_the_lock() {
    let d = Daemon::start();
    let mut second = Config::new(d.socket.clone());
    second.fake = Some(std::sync::Arc::new(
        obsbot_device::models::fake::FakeBus::new(&["fake-a"]).unwrap(),
    ));
    let err = run_with(second, fast()).expect_err("refused");
    assert!(err.to_string().contains("s.sock.lock"), "{err}");
    let (_c, ok) = d.hello(false, &[]);
    assert_eq!(ids(&ok["devices"]).len(), 3, "the first one goes on");
}

#[test]
fn a_clean_stop_returns_ok_and_removes_the_sockets() {
    let d = Daemon::start();
    let socket = d.socket.clone();
    d.stop().expect("Ok");
    assert!(!socket.exists());
    let mut test = socket.as_os_str().to_owned();
    test.push(".test");
    assert!(!std::path::Path::new(&test).exists());
}

/// 11 and 5.10: under systemd the daemon serves the socket it inherits (fd 3, `LISTEN_PID`
/// and `LISTEN_FDS`), takes the lock beside its path, and leaves the socket file to systemd.
#[test]
fn socket_activation_serves_the_inherited_socket() {
    let dir = temp_dir();
    let socket = dir.join("s.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let fd = listener.as_raw_fd();
    let mut cmd = Command::new("sh");
    cmd.args([
        "-c",
        "export LISTEN_PID=$$ LISTEN_FDS=1; exec \"$0\" --fake fake-a",
        env!("CARGO_BIN_EXE_obsbot4linuxd"),
    ])
    .env("OBSBOT4LINUX_SOCKET", &socket)
    .stdin(Stdio::null());
    // SAFETY: dup2 is async-signal-safe; it gives the child the listener as fd 3, as systemd
    // does, and clears close-on-exec on the copy.
    unsafe {
        cmd.pre_exec(move || {
            if nix::libc::dup2(fd, 3) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().unwrap();
    let mut c = Client::connect(&socket);
    let ok = c.hello(false, &[]);
    assert_eq!(ids(&ok["devices"]), [A]);
    assert!(
        dir.join("s.sock.lock").exists(),
        "the lock is taken for an inherited socket too"
    );
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(0));
    assert!(socket.exists(), "an inherited socket file is systemd's");
    drop(listener);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_binary_refuses_bad_arguments_and_a_missing_socket_path() {
    let bin = env!("CARGO_BIN_EXE_obsbot4linuxd");
    let out = Command::new(bin)
        .args(["--fake", "tiny2lite"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown fake model"));
    let out = Command::new(bin)
        .args(["--fake", "fake-a"])
        .env_remove("OBSBOT4LINUX_SOCKET")
        .env_remove("XDG_RUNTIME_DIR")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("XDG_RUNTIME_DIR"));
    let out = Command::new(bin).arg("--version").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("obsbot4linuxd "));
}

/// SIGINT ends the daemon like SIGTERM (5.9).
#[test]
fn sigint_ends_the_daemon_with_0() {
    let dir = temp_dir();
    let socket = dir.join("s.sock");
    let mut child = Command::new(env!("CARGO_BIN_EXE_obsbot4linuxd"))
        .args(["--fake", "unknown"])
        .env("OBSBOT4LINUX_SOCKET", &socket)
        .env_remove("LISTEN_PID")
        .env_remove("LISTEN_FDS")
        .spawn()
        .unwrap();
    assert!(eventually(T, || std::os::unix::net::UnixStream::connect(
        &socket
    )
    .is_ok()));
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGINT,
    )
    .unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(0));
    let _ = std::fs::remove_dir_all(dir);
}

/// Review focus 5: a stop of the daemon returns while a camera operation hangs: polls and
/// identity attempts are cut at their next frame boundary.
#[test]
fn a_stop_returns_while_a_camera_operation_hangs() {
    let d = Daemon::start();
    let (_c, _) = d.hello(false, &[]);
    let skip = d.ops(A).len();
    d.bus.set_delay("fake-a", Duration::from_secs(30));
    assert!(
        eventually(T, || d.ops(A).len() > skip),
        "an operation is in flight"
    );
    let t0 = std::time::Instant::now();
    d.stop().expect("Ok");
    assert!(t0.elapsed() < Duration::from_secs(3), "{:?}", t0.elapsed());
}
```

- [ ] **Step 2: Run them to see the feature fail**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test hotplug --test process`
Expected: `hotplug::the_hand_over_from_port_to_sn` fails (the re-plugged device stays `port:9-1`). Every other test passes; a failure of another test is a B-1 defect: fix the module it names, say so in the PR text, rerun.

- [ ] **Step 3: Add the feature**

Write the steps marked **[b-14]** in b-07's algorithms: `tick` step 2 (`series.at_tick`) and the first line of `wake()` (`series.at_wake`). Then remove the skeleton's `#![allow(dead_code, unused_variables, unused_imports)]` line and the comment above it from `lib.rs`, and fix every warning that appears (each is a stub leftover or a real omission; the chain has wired every feature).

- [ ] **Step 4: Run them to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4 --test hotplug --test process`
Expected: PASS: `hotplug` 4, `process` 6.

- [ ] **Step 5: Check the whole crate**

Run: `cargo fmt --all --check && nice -n 19 cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings && nice -n 19 cargo test -p obsbot4linuxd -j 4`
Expected: no fmt or clippy output; every test passes.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot4linuxd
git commit -m "obsbot4linuxd: identity attempts 2 and 3; integration tests of hotplug, hand-over, identity and the process"
```

### Task b-15 (key `b-15-real-bus`): Real device path (B-2)

**Files:**

- Modify: `crates/obsbot4linuxd/src/daemon.rs` (step 2 of `run`, one line, and one unit test)

**Interfaces:**

- Consumes (track A, task `a-13-registry-and-bus`, names as track A's plan gives them; if the landed code differs, the landed names win in this one line and the test): `obsbot_device::bus::SysfsBus` (`impl Bus`), `SysfsBus::new()` (root `/sys`, the real registry), `SysfsBus::with_root(root: impl Into<PathBuf>)` (another root, for the test; always the real registry), `obsbot_device::registry::REGISTRY`.
- Produces: `obsbot4linuxd::run(Config::new(path))` serves the cameras the sysfs scan finds; `obsbot4linuxd` without `--fake` runs on them.

Starts once b-14 (the end of the chain, which may touch `daemon.rs`) and track A's `a-13-registry-and-bus` have landed on `rebuild`. The daemon gains no other code: the worker, the queue, the table and the registry are the ones fake mode runs (spec 10.3 B-2 "workers over `trait Camera` with `tiny2lite`"). The test must not touch a real device: it runs on an empty sysfs root.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module of `crates/obsbot4linuxd/src/daemon.rs`:

```rust
    /// B-2: without `--fake` the daemon scans sysfs through track A's bus; on a root without
    /// any video device it serves an empty device list and stops cleanly. No real device is
    /// touched: the root is an empty directory.
    #[test]
    fn the_real_path_serves_what_the_sysfs_scan_finds() {
        let dir = std::env::temp_dir().join(format!("o4ld-real-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sys")).unwrap();
        let socket = dir.join("s.sock");
        let mut config = Config::new(socket.clone());
        let stop = config.stop.clone();
        config.fake = None;
        let bus: Arc<dyn obsbot_device::contract::Bus> =
            Arc::new(obsbot_device::bus::SysfsBus::with_root(dir.join("sys")));
        let h = std::thread::spawn(move || serve(config, Timing::SPEC, bus));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::os::unix::net::UnixStream::connect(&socket).is_err() {
            assert!(std::time::Instant::now() < until);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let mut s = std::os::unix::net::UnixStream::connect(&socket).unwrap();
        use std::io::{BufRead, Write};
        writeln!(s, r#"{{"id":1,"hello":{{"protocol":2,"client":"t 1"}}}}"#).unwrap();
        let mut line = String::new();
        std::io::BufReader::new(s).read_line(&mut line).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["ok"]["devices"], serde_json::json!([]));
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        h.join().unwrap().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 4 daemon::`
Expected: FAIL to compile: `cannot find function serve in this scope` (and nothing else: track A's `obsbot_device::bus` and `obsbot_device::registry::REGISTRY` resolve once `a-13-registry-and-bus` has landed).

- [ ] **Step 3: Write the code**

In `daemon.rs`, move steps 1 and 3 to 8 of `run` (b-10) into `fn serve(config: Config, timing: Timing, bus: Arc<dyn Bus>) -> io::Result<()>`, unchanged; `run` keeps only the choice of the bus and calls `serve`: `config.fake` → the fake bus as before; `None` → `Arc::new(obsbot_device::bus::SysfsBus::new())`. Nothing else changes.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 4`, then the workspace gate (`cargo fmt --all --check`, clippy with `-D warnings`, `cargo test --workspace --locked`).
Expected: PASS, every test of the crate, the new one included.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/daemon.rs
git commit -m "obsbot4linuxd: real device path through the sysfs bus (B-2)"
```

Acceptance with camera: none in this track (W4 covers the real path, spec 10.3 B).

---

## Track B acceptance (spec 10.3 "Acceptance without camera"), mapped

All with `fake-a`, `fake-b` and `unknown` running together (the harness starts `KEYS`), except where a test needs one model only.

| Criterion (spec 10.3 B)                                                                                                                      | Test (task)                                                                                                                                                                                                                                                                                                                                                                                |
| -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| rev order and event-before-reply                                                                                                             | `core::a_set_publishes_its_state_before_the_reply` (b-10); `table::tests::only_a_change_gets_a_rev_and_an_event` (b-06); `worker::tests::a_set_publishes_before_it_replies` (b-07)                                                                                                                                                                                                         |
| pipelined requests                                                                                                                           | `requests::pipelined_requests_are_answered_in_request_order` (b-11)                                                                                                                                                                                                                                                                                                                        |
| N pipelined `set`s on one connection cause at most two re-reads                                                                              | `requests::pipelined_sets_share_at_most_two_rereads` (b-11)                                                                                                                                                                                                                                                                                                                                |
| single-leaf coalescing                                                                                                                       | `requests::a_single_leaf_set_is_coalesced_with_a_later_one` (b-11); `queue::tests::single_leaf_sets_of_one_connection_coalesce`, `queue::tests::a_write_in_between_keeps_both_sets` (b-05)                                                                                                                                                                                                 |
| stops overtake the queue                                                                                                                     | `motion::a_stop_overtakes_the_queued_requests`, `motion::a_stop_cuts_a_group_read_in_progress` (b-13); `queue::tests::requests_queue_in_order_and_a_stop_overtakes_them` (b-05)                                                                                                                                                                                                            |
| whole-patch validation, apply order, `applied`/`failed`                                                                                      | `requests::one_bad_leaf_refuses_the_whole_patch`, `requests::requires_and_the_apply_order`, `requests::a_failure_midway_reports_applied_and_failed` (b-11)                                                                                                                                                                                                                                 |
| `requires` gives `conflict`                                                                                                                  | `core::refused_requests_send_nothing` (b-10); `requests::requires_and_the_apply_order` (b-11)                                                                                                                                                                                                                                                                                              |
| `pending` appears and clears (lagging readback of `fake-b`, and the settle bound)                                                            | `core::a_lagging_readback_is_pending_until_a_read_shows_it` (b-10); `asleep::the_settle_bound_ends_pending_and_the_camera_value_stands` (b-12); `pending::tests::*` (b-03)                                                                                                                                                                                                                 |
| `asleep: ignored` refused, `asleep: unknown` written and pending until wake                                                                  | `asleep::asleep_ignored_is_refused_and_unknown_is_pending_until_wake` (b-12)                                                                                                                                                                                                                                                                                                               |
| the 15 s deadline                                                                                                                            | `deadline::every_request_is_answered_within_the_deadline` (b-13)                                                                                                                                                                                                                                                                                                                           |
| `busy` at 32                                                                                                                                 | `deadline::the_request_after_32_queued_ones_is_busy` (b-13); `queue::tests::the_33rd_queued_request_is_busy` (b-05); `conn::tests::busy_is_answered_by_the_reader` (b-08)                                                                                                                                                                                                                  |
| requests of a closed connection dropped                                                                                                      | `deadline::queued_requests_of_a_closed_connection_are_never_sent`, `deadline::a_batch_skips_the_requests_of_a_connection_that_closed` (b-13); `conn::tests::a_closing_connection_drops_its_requests_and_stops_its_turn`, `conn::tests::a_closing_connection_marks_requests_already_taken_closed` (b-08); `outbound::tests::mark_closed_keeps_the_queued_lines_and_refuses_new_ones` (b-01) |
| `gimbal.turn` stopped on connection close unless `keep`                                                                                      | `motion::a_turn_without_keep_is_stopped_when_its_connection_closes`, `motion::a_keep_turn_is_never_stopped_by_the_daemon`, `motion::another_motion_clears_the_turn_record` (b-13)                                                                                                                                                                                                          |
| hotplug add, unplug, re-plug                                                                                                                 | `core::the_test_control_socket_drives_hotplug` (b-10)                                                                                                                                                                                                                                                                                                                                      |
| `port:` → `sn:` hand-over with `replaced_by`                                                                                                 | `hotplug::the_hand_over_from_port_to_sn` (b-14); `table::tests::the_hand_over_is_three_events_and_takes_the_unplugged_entry` (b-06)                                                                                                                                                                                                                                                        |
| the unsupported entry receives no frame                                                                                                      | `core::the_unknown_device_is_answered_at_once_and_receives_nothing` (b-10); `registry::tests::the_first_scan_lists_the_unsupported_device_and_starts_two_workers` (b-09)                                                                                                                                                                                                                   |
| `hello` waits for the first enumeration                                                                                                      | `startup::hello_waits_for_the_first_read_attempts` (b-12); `table::tests::hello_returns_once_the_first_reads_end` (b-06)                                                                                                                                                                                                                                                                   |
| live-topic filtering                                                                                                                         | `requests::a_volatile_change_reaches_only_its_live_subscribers` (b-11); `table::tests::volatile_changes_reach_only_their_live_subscribers` (b-06)                                                                                                                                                                                                                                          |
| a slow subscriber is disconnected                                                                                                            | `limits::a_subscriber_that_does_not_read_is_disconnected` (b-11); `outbound::tests::a_full_queue_closes_the_connection_instead_of_blocking` (b-01)                                                                                                                                                                                                                                         |
| the 64 KiB request limit                                                                                                                     | `limits::a_request_line_of_64_kib_is_read_and_one_byte_more_closes_the_connection` (b-11); `lines::tests::the_cap_allows_exactly_its_length_and_refuses_one_byte_more` (b-02)                                                                                                                                                                                                              |
| the 64-connection limit                                                                                                                      | `limits::the_65th_connection_is_closed_at_once` (b-11); `conn::tests::at_most_64_connections_are_admitted` (b-08)                                                                                                                                                                                                                                                                          |
| the single-instance lock                                                                                                                     | `process::a_second_daemon_on_the_same_socket_is_refused_by_the_lock` (b-14); `socket::tests::a_second_daemon_does_not_take_over_the_socket` (b-02)                                                                                                                                                                                                                                         |
| the panic policy                                                                                                                             | `daemon::tests::a_panic_in_a_critical_thread_is_fatal` (b-01); the connection guard of b-08 (reviewed; no fake can panic)                                                                                                                                                                                                                                                                  |
| `hello` answered by its deadline while a first read is slow                                                                                  | `startup::hello_is_answered_by_its_deadline_while_a_first_read_is_slow` (b-12); `table::tests::hello_waits_for_the_first_enumeration_until_its_deadline` (b-06)                                                                                                                                                                                                                            |
| the hand-over event sequence and its rev                                                                                                     | `hotplug::the_hand_over_from_port_to_sn` (b-14); `table::tests::the_hand_over_is_three_events_and_takes_the_unplugged_entry` (b-06)                                                                                                                                                                                                                                                        |
| requests for unsupported and unavailable devices answered at once                                                                            | `core::the_unknown_device_is_answered_at_once_and_receives_nothing` (b-10); `hotplug::requests_for_an_unavailable_device_are_answered_at_once` (b-14); `table::tests::requests_that_need_no_worker_are_answered_at_once` (b-06)                                                                                                                                                            |
| a `restart` write answered on acceptance, nothing sent after it until the device goes                                                        | `asleep::a_restart_write_is_answered_on_acceptance_and_then_nothing_is_sent` (b-12)                                                                                                                                                                                                                                                                                                        |
| a device-number change handled as unplug plus plug                                                                                           | `hotplug::a_new_device_number_is_an_unplug_and_a_plug` (b-14); `hotplug::tests::a_new_device_number_under_a_known_path_is_gone_and_new` (b-01); `registry::tests::an_unplug_ends_the_worker_and_a_new_device_number_starts_a_new_one` (b-09)                                                                                                                                               |
| the queue of an unplugged device answered and a new worker on re-plug                                                                        | `deadline::the_queue_of_a_device_that_goes_is_answered_and_a_replug_starts_afresh` (b-13); `worker::tests::the_end_answers_the_queue` (b-07)                                                                                                                                                                                                                                               |
| a re-added `port:` entry starting at rev 1                                                                                                   | `core::the_test_control_socket_drives_hotplug` (b-10); `table::tests::an_unsupported_device_comes_and_goes` (b-06)                                                                                                                                                                                                                                                                         |
| the wake read of `unread` groups and the retry of a failed `touched` read                                                                    | `asleep::the_wake_read_reads_the_touched_group_the_camera_could_not_answer`, `asleep::a_failed_touched_read_is_read_again_with_the_minute_tier` (b-12); `schedule::tests::*` (b-03)                                                                                                                                                                                                        |
| an identity series ending at the first complete answer                                                                                       | `hotplug::an_identity_series_ends_at_the_first_complete_answer` (b-14); `identity::tests::a_complete_first_answer_ends_the_series` (b-04)                                                                                                                                                                                                                                                  |
| a `set` causes at most one `apply` per leaf and no `act`, a `do` exactly one `act` and no `apply`, nothing else reaches the fake (section 2) | `requests::a_request_sends_exactly_what_it_names`, `requests::a_value_is_sent_as_the_request_names_it` (b-11); `pending::tests::sent_keeps_the_value_and_only_puts_float_noise_on_its_step` (b-03)                                                                                                                                                                                         |
| no stop reaches the fake when it falls asleep during a turn, nor for a `keep:true` turn at shutdown                                          | `motion::no_stop_when_the_camera_falls_asleep_during_a_turn`, `motion::shutdown_stops_a_turn_without_keep_only` (b-13)                                                                                                                                                                                                                                                                     |
| acceptance with camera: none in this track                                                                                                   | no task touches a camera; B-2's test runs on an empty sysfs root                                                                                                                                                                                                                                                                                                                           |

## Camera items this track leaves open

None of them is in a task's acceptance; each is answered by track A (CS-1) or W4 (CS-2), and none changes daemon code unless its answer changes a description (a contract-change PR) or a model.

- **CAM-1** (identity answers asleep and after restarts): the daemon's identity series is tested on the fakes only; how many attempts a real camera needs is open.
- **CAM-2, CAM-3, CAM-22** (`asleep` values; awake at plug-in; asleep pose): the daemon follows each description's `asleep`; the real values are open.
- **CAM-5** (`settle_ms` per leaf): the daemon uses each description's `settle_ms`; the bound of the Tiny 2 Lite is open.
- **CAM-6** (`readback:false` across sleep and re-plug): the daemon keeps the value across sleep and clears it at unplug (4.2); open on the camera.
- **CAM-7** (does a turn stop when the camera falls asleep): the daemon sends nothing (tested); what the camera does is open.
- **CAM-10** (cold read and 5 s tier duration): the `hello` deadline is tested with a slow fake; the real latency is open.
- **CAM-20, CAM-21** (not-ready status at boot; a silent query channel): the daemon publishes what the model reads; open.
- **CAM-25** (sysfs attributes): B-2 depends on track A's enumeration; open until CS-1.
- **CAM-26** (two cameras at once): the fakes cover several devices; a duplicate serial is covered by unit tests only (decision 16).
- **CAM-27** (`NoNewPrivileges=yes` user unit on Arch: starts, sees the camera, handles hotplug) and **CAM-28** (upgrade rehearsal): W4.
- **CAM-29** (the kernel's UVC control timeout): bounds a real operation the daemon's contexts cannot cut inside a control transfer; recorded if it happens.
- **CAM-34** (manual moves while tracking): the daemon sends only the move (tested on fake-a); the camera's reaction is open.

## Self-review

1. **Spec coverage.** Section 2: no stop of the daemon's own except the connection-bound one (b-05 record, b-13 wiring and tests), every request sends what it names, float noise only put on its step (b-03 `sent`, b-07, b-11). 4.1: the five verbs, every daemon error code (`protocol`, `invalid`, `unknown_device`, `unsupported`, `unsupported_device`, `out_of_range`, `conflict`, `unavailable`, `asleep`, `busy`, `camera`; `internal` for `NotInSendTable`, logged), rules 1 to 9 (rule 9 also for requests already in a batch: b-01 `mark_closed`, b-08 close order), events, `hello` timing, the 64 KiB and 64-connection limits (b-02, b-05, b-06, b-07, b-08, b-10, b-11 to b-14). 4.2: `unread`, absent vs null, `raw:<n>` logging, the `readback:false` lifetime, precision of echoed values, the one shortening formatter (b-01, b-03, b-06, b-07). 5.1 threads (b-08 to b-10); 5.2 start-up (b-06, b-07, b-09, b-10); 5.3 enumeration and hotplug (b-01 diff, b-09); 5.4 unsupported devices (b-01, b-06, b-09); 5.5 identity, duplicates, hand-over (b-04, b-06, b-07 attempt 1, b-14 attempts 2 and 3); 5.6 write path, restart, unavailable, stops, continuing motion (b-05, b-07, b-11 coalescing, b-12 restart, b-13 record); 5.7 polling, asleep, wake and retries (b-03, b-07, b-12 wake read); 5.8 timeouts (`REQUEST_DEADLINE`, `ACCEPT_BACKOFF`; `ANSWER_POLL`, `FRAME_GAP` and `NODE_LOCK_WAIT` are track A's transport); 5.9 lock, shutdown, failures (b-01, b-02, b-10, b-13 own stops); 5.10 socket activation and files (b-02, b-10); 5.11 logs (b-01, b-06, b-07, outbound); 5.12 fake mode and test control (b-02 args, b-10). 8.2 and 8.3: the daemon uses `trait Camera` and `trait Bus` only and has no send path of its own; a refused frame is logged (b-01 `log_refused_frame`, b-07 `failed`). 10.3 B: the table above. 11: in-process tests on temporary sockets, in parallel, without `XDG_RUNTIME_DIR`.
2. **Placeholder scan.** Pure modules, the skeleton and every test are given whole. Four I/O tasks (b-07 worker, b-08 conn, b-09 registry, b-10 daemon) give their types and signatures in full and their bodies as numbered algorithms with the tests that pin them; the steps the chain b-11 to b-14 adds are marked in b-07's algorithms with the task that adds them, and each chain task names them again in its Step 3. No step says "handle errors appropriately". b-15 uses the names of track A's plan and confines them to one line and one test.
3. **Type consistency.** Every module, signature and test of this plan was compiled together in one workspace on the landed W0 tree; the names in each task's Interfaces block are those of that build. Names checked across the tasks and the task list: `Outbound::mark_closed`, `names::log_refused_frame`, `pending::sent`, `Worker::failed`, the four chain keys, `a-13-registry-and-bus`, `obsbot_device::bus::SysfsBus`, `obsbot_device::registry::REGISTRY`.
4. **Review Focus.** Six input classes, each with a test in its owning task.
5. **Public text.** No private path, host or name; synthetic serials only; product ids only the fakes' `fa0x`.

## Proof (what was built and run for this plan)

Built on 2026-11-13 with rustc 1.99.1 in a scratch copy of `rebuild` at `3187b84` (W0 as landed there), the plan's code extracted from this file by script (every block in task order: "Create" and "Replace" write a file, "Append" adds to it, the conn and registry items plus their tests, the control stub signatures b-10 names). An earlier round built the same code on a copy with the W0 plan's code extracted at stage G; the code this round changed is listed below and was rebuilt.

- **Compiled together** (`cargo fmt --all --check`, `cargo clippy -p obsbot4linuxd --all-targets --offline -- -D warnings`, both clean): every file of this plan in its final state: the skeleton, the eight complete modules, the stub signatures of `worker`, `conn`, `registry`, `control` and `daemon::run`, the unit-test modules of all of them, `tests/common/mod.rs` and the ten integration test files.
- **Compiled in task order:** the skeleton state of b-01 alone (18 unit tests and `tests/harness.rs` pass), and each wave-2 task alone on top of it (b-02: 29, b-03: 34, b-04: 26, b-05: 29, b-06: 32 unit tests, all passing, fmt and clippy clean).
- **Run green:** the 78 unit tests of the complete modules (`lib` 2, `outbound` 7, `names` 5, `hotplug` 3, `daemon` 1, `lines` 4, `socket` 4, `args` 3, `pending` 10, `schedule` 6, `identity` 8, `queue` 11, `table` 14) and `tests/harness.rs` 1.
- **Changed in the fix round and rebuilt:** `Outbound::mark_closed` and its test (b-01); `names::log_refused_frame` in the log-line test (b-01); the skeleton's `allow` comment (b-01); `pending::sent` and its test (b-03); the coalescing condition of `queue::plan` and `queue::tests::a_write_in_between_keeps_both_sets` (b-05); `conn::tests::a_closing_connection_marks_requests_already_taken_closed` (b-08, compiled); `requests::a_value_is_sent_as_the_request_names_it` (b-11), the restart test (b-12), `deadline::a_batch_skips_the_requests_of_a_connection_that_closed` and the stop-cut test (b-13) (compiled).
- **Compiled, not run** (their bodies are b-07 to b-10 and the chain's features, which this plan gives as algorithms): the unit tests of `worker` (4), `conn` (7), `registry` (3) and `control` (1), and the integration tests `live` (9), `requests` (10), `limits` (3), `asleep` (5), `startup` (2), `deadline` (5), `motion` (7), `hotplug` (4), `process` (6). Against the stubs they fail as each task's Step 2 says.
- **Not compiled:** the test of b-15, which names track A's `obsbot_device::bus::SysfsBus` and `obsbot_device::registry::REGISTRY` (track A's `a-13-registry-and-bus`, not landed).
- **Unproven:** that the algorithms of b-07 to b-10 and the chain's marked steps make the compiled-only tests pass as written; their timing margins (the fake delays and the 100 ms test tiers) were chosen from the fakes' code, not measured on a running daemon. Which of the chain's tests fail before its feature (each chain task's Step 2) is reasoned from the fakes' code, not run.
