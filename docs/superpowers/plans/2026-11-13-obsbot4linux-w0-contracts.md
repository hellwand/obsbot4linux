# obsbot4linux 1.0, stage W0 (contracts) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the contracts that the four parallel tracks of the 1.0 rebuild (A device, B daemon, C client and CLI, D widget) build against without talking to each other: the protocol-2 types, the description format and its validator, the schema and the generated description files, `trait Camera` with `trait Transport` and `trait Bus`, the fake models, the Tiny 2 Lite description as data, golden transcripts, the daemon and CLI entry points as stubs, and the normative part of `docs/protocol-2.md`.

**Architecture:** `obsbot-proto` holds the typed envelopes with a generic payload (state, patch and action parameters are path → JSON value), the description types with their builders, the validator and the schema generator. `obsbot-device` gets a `contract` module (the traits and their supporting types, plus a conformance check every model must pass), the fake models behind a fake bus, and the Tiny 2 Lite description as data. `obsbot4linuxd` and the CLI become compiling stubs whose entry points are fixed. Generated files (`schema/protocol-2.json`, `descriptions/**`) have stale tests and are rewritten by `scripts/regenerate.sh`.

**Tech Stack:** Rust 2024 edition, MSRV 1.88 (CI toolchain 1.99.1), serde 1, serde_json 1 (default features: maps sorted by key), schemars 1.2 (JSON Schema draft 2020-12), jq in CI.

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` (revision 2, approved 2026-11-13), sections 3, 4, 5.1 to 5.3, 5.12, 6, 8, 9.3, 10.3 (W0) and 11; field ledger `docs/field-ledger.md`.

## Global Constraints

- Cargo workspace, one repository, four crates: `crates/obsbot-proto`, `crates/obsbot-device`, `crates/obsbot4linuxd`, `crates/obsbot4linux` (spec 3).
- `obsbot-proto`: "Depends on serde, serde_json, schemars." Nothing else (spec 3).
- "Dependency direction: proto ← device ← daemon; proto ← CLI. The `[dependencies]` of the CLI crate never contain `obsbot-device` or `obsbot4linuxd`; a CI step checks this with `cargo metadata`. Its `[dev-dependencies]` may contain `obsbot4linuxd`" (spec 3).
- "All crates take `version`, `rust-version` and `license = "GPL-3.0-or-later"` from `[workspace.package]`" (spec 3). Every new source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`; a file that receives code from a file with an EUPL-1.2 Article 5 note carries the note too (spec 3.3).
- Contract-change rule: "A change to the `obsbot-proto` types, the description format, `trait Camera`, `trait Transport` or the schema goes first as its own small PR into `rebuild`. Tracks rebase onto it and never change those files inside a track PR" (spec 10.1). After W0 these files are: `crates/obsbot-proto/src/*` except `client*`, `crates/obsbot-device/src/contract/*`, `schema/protocol-2.json`.
- Protocol: "JSON lines over the Unix socket `$XDG_RUNTIME_DIR/obsbot4linux.sock` (mode 0600; the environment variable `OBSBOT4LINUX_SOCKET` overrides the path)"; "A request line longer than 64 KiB closes the connection"; "at most 64 connections"; `protocol: 2` (spec 4.1).
- Error codes, exactly: `protocol`, `invalid`, `unknown_device`, `unsupported`, `unsupported_device`, `out_of_range`, `conflict`, `unavailable`, `asleep`, `busy`, `camera`, `internal` (spec 4.1).
- Setting types: `bool`, `enum`, `number`, `range` (`{min,max}`), `pose` (`{pan,tilt,zoom}`), `point` (`{x,y}`), `box` (`{left,top,right,bottom}`), `string`, `list`. Units: `deg`, `deg/s`, `factor`, `us`, `K`, `percent`, `s`, `fraction`, `fps`, or absent for a raw camera value (spec 4.2).
- "A serial is valid only if it matches `[A-Za-z0-9]{8,32}`" (spec 5.5).
- Paths: "lower-case `snake_case` segments joined by `.`, the camera function first (`zoom`, `tracking`, `exposure`), then the leaf; no model name in a path" (spec 4.2).
- Precision: "angles 0.1, zoom 0.01, fractions 0.01"; "Values are rounded to it where they are created" (spec 4.2).
- Tiers: "1 s, 5 s, 60 s, or `touched`"; "A writable leaf with `readback:true` is in a group with a periodic tier (1 s, 5 s or 60 s) … the trait conformance test checks it" (spec 5.7).
- `REQUEST_DEADLINE` 15 s; `CLIENT_REPLY_TIMEOUT` 30 s; device queue at most 32 requests; outbound queue 64 lines (spec 5.1, 5.6, 5.8).
- "Public enums are `#[non_exhaustive]`, so additive protocol changes do not break Rust callers" (spec 6.1).
- Fakes: "`fake-a` uses the Tiny 2 Lite's description, for client work; `support: tested`"; "`fake-b` is deliberately different: other enum values, whole groups missing (no `gimbal`, no `views`), a setting and an action under a group no client knows, another preset count (5), `readback:false` leaves, leaves with `asleep: ignored` and `asleep: unknown`, a `requires`, a readback that lags its write by a set time, `support: untested`"; "`unknown` is a device with an unregistered product id of a known vendor id"; "synthetic serials (`sn:FAKEA0000001`)"; test control on `<socket path>.test` with `plug`, `unplug`, `sleep`, `wake`, `fail_next`, "not part of protocol 2 and not in any schema or description" (spec 5.12).
- "No raw send path exists anywhere: not in the daemon, the CLI, the protocol or the fakes" (spec 8.3).
- Tiny 2 Lite: "`settle_ms`: 3000 for every writable leaf with a readback"; "`readback:false`: `hand.move_gimbal`, `led.blink`"; "`presets`: at most 3 (`limits.presets`)"; "the description written in stage W0 contains the field, value or bound" of every row of 9.3 (spec 8.8, 4.3, 9.3).
- "Golden files cover protocol 2 and the descriptions. An intentional change updates them in the same PR" (spec 11).
- Public text (spec 8.5): English; paths, command ids, ranges, units, the model's own USB ids, evidence levels and opaque keys only; no private paths, no vendor-app or firmware names, no serial numbers (the synthetic `EXAMPLE0000001`, `FAKEA…`, `FAKEB…` are allowed).
- No camera I/O anywhere in W0; "Acceptance with camera: none" (spec 10.3 W0).
- Repository facts this plan relies on: `edition = "2024"`, `rust-version = "1.88"`, CI toolchain `1.99.1` with `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings`; commits on `rebuild` are signed (ruleset, spec 10.1).

## Review Focus

Inputs the spec implies that a person will hit, with the test that pins each (already in the owning task):

1. A client's float arithmetic sends `2.0000000001` for a stepped value (a QML slider, JavaScript `0.1 + 0.2`): it must be accepted, not refused as off-step. Test row "float noise on a stepped value" in Task 2.
2. The CLI's `min,max` value syntax sent as a JSON array (`{"zone.area.pan":[-20,20]}`): `invalid`, never a panic or a silent coercion. Test row "a range as an array" in Task 2.
3. Malformed paths (`led..brightness`, `led.`, an empty key) in a patch: `unsupported`, never a panic. Test rows in Task 2.
4. A newer client sends fields this daemon does not know (`"trace":"abc"` in a request): ignored, the request is served. Test in Task 1 (`requests_parse_into_their_verbs`).
5. A `get` for a component of a leaf (`zone.area.pan.min`): `unsupported`, because only leaves and groups are paths. Test in Task 2 (`get_accepts_leaves_groups_and_daemon_fields_only`).

---

## What W0 needs from stage G

W0 needs stage G merged into `rebuild` (spec 10.3: "Depends on: stage G merged into `rebuild`"), and from it only:

- the workspace root `Cargo.toml` with `[workspace]`, `[workspace.package]` (`version`, `rust-version`, `license`) and `[workspace.dependencies]` holding `serde` (with `derive`) and `serde_json`;
- `crates/obsbot-device/` with `Cargo.toml`, `src/lib.rs`, the module `src/models/` as a directory module (`src/models/mod.rs`) and the Tiny 2 Lite as a directory module (`src/models/tiny2lite/mod.rs`);
- `crates/obsbot4linux/` with the 0.13 command line (`src/main.rs`, `src/cli.rs`, `src/client.rs`, `src/daemon.rs`, `src/proto.rs`);
- `crates/obsbot-device/src/legacy/` with `mod.rs`, `state.rs`, `full_status.rs` and `current_pose.rs` (G's groundwork plan, Tasks 6 and 7; decision 15).

W0 uses none of the moved camera code (`transport`, `family`, the Tiny 2 Lite flows, `registry`): it only adds files next to it. If G named a path differently (for example `models.rs` instead of `models/mod.rs`), the task that touches it adds its one line to G's file instead; the line is the same. W0 never edits G's carried modules, except Task 6, which deletes the 0.13 command line and `legacy/full_status.rs`.

## Decisions this plan takes

The spec leaves these details of the contract open; this plan fixes them. Each can be overruled by the maintainers with a contract-change PR.

1. **State shape.** A state is the nested tree of spec 4.3 (`{"zoom":{"factor":1.5}}`) plus the daemon's `available`, `pending` (flat map leaf path → value) and `unread` (leaf paths). `State::at("zoom.factor")` walks it. Patches may be nested or dotted (spec 4.1).
2. **Canonical JSON.** Maps are sorted by key (serde_json without `preserve_order`); struct fields keep declaration order; an integral number is written without a fraction (`3`, not `3.0`); `readonly`, `nullable`, `readback` and `no_effect` are always written (a JavaScript client reads a missing `readback` as false); other optional Setting fields are left out when empty; every writable enum carries `writable`. Golden files are in this form.
3. **Forward compatibility.** `Support`, `SettingType`, `Unit`, `Asleep`, `Warning` and `ErrorCode` have a catch-all `Other(String)`; `Event` has `Unknown`. A newer daemon's additive values parse (spec 4.1 rule 7).
4. **`trait Camera` signature details** (spec 8.2 shows the shape): `identity`, `read`, `apply` and `act` take a `&Ctx` that the model checks before every frame (the 15 s deadline of 4.1 rule 9; a stop cutting a group read, 5.6); `Ctx::new` takes both at once, because a request's touched re-read has the request's deadline and is also a group read a stop cuts. `identity` takes a `Need` (5.5: send only the missing queries). `act` takes `&Params` (an object) and the published `&State` that `check` saw (a used or empty preset slot, the mirror and invert flags), and returns `Option<Params>`. Spec 8.2's `stops` and `continuing` become one method `motion(action) -> Motion` with `Motion::{None, Move, Continuing { stop }, Stop}`, because the daemon must also know which actions are motion actions: 5.6 "any other motion action of that device clears the record" (4.1 rule 6 lists them). A `Stop` clears the record only when it is the stop the record names. One method is added: `check(action, params, state) -> Result<(), Refusal>`, the model's checks that the description cannot express, run before any frame (an empty slot, an unknown view id, a list in `unread`: `conflict`; `gimbal.move` without an axis: `invalid`). The daemon calls `check` only after `validate_do` passed, so the model's codes rank after every code of the validator (decision 9). A group whose read depends on a condition (the Tiny 2 Lite's target, 8.8) is decided by the model from its own last reads (`read` docs).
5. **`touched` includes the written leaf's own group.** The conformance check requires it, so the state the reply publishes can show the write (spec 9.4 counts lags "at the touched re-read").
6. **Enumeration seam.** `trait Bus { scan, open }`, `Found`, `ModelEntry` (data only: key, display name, USB pairs, support, identity source `IdentitySource::{UsbSerial, Query, Absent}`, whether a firmware query exists, identity tier, extension-unit GUID, selectors; the two identity fields let the daemon end an identity series and send no attempt for a model with neither query, 5.5), `Registry` with `lookup` and `shared_pairs`, `Lookup`. The fake bus and track A's sysfs bus implement `Bus`, so the daemon has one enumeration path and `unknown` "enters through the same registry lookup as a real device" (spec 5.12). The constructor of a model stays with its bus.
7. **`trait Transport`** (spec 3.1): `xu_len`, `xu_get`, `xu_set`, `ctrl_get`, `ctrl_set`, `ctrl_range`, `usb` (fresh sysfs facts), `usb_serial`, `on_air`. Track A adds anything else by a contract-change PR.
8. **Daemon entry point.** `obsbot4linuxd::Config { socket, fake: Option<Arc<FakeBus>>, stop }` and `run(Config) -> std::io::Result<()>`. In-process tests build the `FakeBus` themselves and may use its Rust-only `set_delay`. The test-control socket answers each line with `{"ok":<id or true>}` or `{"error":"<message>"}` (`FakeBus::handle_line`).
9. **Validator.** One order for the whole request: `invalid`, `unsupported`, `out_of_range`, `conflict`, `asleep` (the order of spec 5.6), then path order. `Camera::check` runs only when the validator passed, so its refusals come after all of these (on the Tiny 2 Lite a `preset.save` while asleep, with `presets` in `unread`, is `asleep`, not `conflict`). An empty patch, a group written as a value, a leaf named twice, an unknown or missing parameter and `null` as a written value are `invalid`; a value off the step, an enum value outside `writable`, a too long or non-ASCII string and a range with `min` above `max` are `out_of_range` (`min == max` is allowed).
10. **`on_air.apps`** is a `list` of `{name}` items, because the description format's lists describe object items (spec 4.2 `item{<field> → Setting}`). This changes the state shape of spec 4.3 (`apps[]` of process names) to `[{"name": …}]`, which clients see; it goes into the spec's "Changes" table with W0.
11. **CLI-only error codes** (`usage`, `ambiguous_device`, `no_device`, `daemon`) are not in `ErrorCode`; track C defines them in the CLI, as spec 6.2 says they "exist only in the CLI".
12. **Tiny 2 Lite numbers the spec does not state.** Backed by the field ledger: gain 1–64, exposure compensation 0–18 default 9, focus position 0–100. Not backed by a ledger row, a capture or a measurement (none of the spec, the ledger, the carried code or the protocol notes holds them; they are working values of this plan): brightness, contrast, saturation, hue, sharpness 0–100 default 50; white balance 2000–10000 K step 100 default 5000, automatic by default; gain default 1; exposure time 100–250000 µs (1–2500 in 100 µs), default 33000 µs; focus position default 0. `tiny2lite::description::UNCONFIRMED` lists each of these by key (`image.brightness@min`, `image.white_balance.temperature#step`, …), and a unit test checks every key resolves. They replace 0.13's run-time `ctrl_range` check in the manual-exposure flow, so a wrong bound refuses valid values or lets the camera reject a write: track A confirms each in the ledger (from `Transport::ctrl_range` on the camera, or a capture) or corrects it, and empties `UNCONFIRMED`, before the evidence-level test of stage W3 (8.5). From 0.13.2: the hand area takes the gimbal range (pan −150..150, tilt −70..90), `hand.return_after` ≥ 0 s with no maximum, `gimbal.move` pan −150..150, tilt −70..90, speed 1–180 °/s.
13. **Fakes.** Vendor id `0x3564` (in the vendor table, so `unknown` is unsupported, not ignored), products `fa0a`, `fa0b`, `fa00`; USB paths `9-<n>`; a restart (a `warning: restart` write or action) makes the device vanish for 1 s and come back with a new device number; `fake-b`'s `led.brightness` readback lags 250 ms and its `exposure.time` is stored rounded down to 100, so both ways out of `pending` happen; `fake-b`'s settle bound is 1000 ms. `fake-a`'s poll plan has one group per first path segment, not the Tiny 2 Lite's real grouping.
14. **The client** (spec 6.1) is written by track C, as spec 10.3 says; W0 reserves `crates/obsbot-proto/src/client.rs` so track C never edits `lib.rs`. `obsbot-proto` holds only the constants a client needs (`PROTOCOL`, the socket names, `MAX_REQUEST_LINE`, `REQUEST_DEADLINE`, `CLIENT_REPLY_TIMEOUT`); the daemon's limits (64 connections, 32 queued requests, 64 outbound lines) are constants of `obsbot4linuxd`, written by track B.
15. **The 0.13 modules stage G keeps in `obsbot-device`.** G moves the 0.13 `state.rs` to `crates/obsbot-device/src/legacy/state.rs` and splits `legacy/full_status.rs` and `legacy/current_pose.rs` off the 0.13 camera module. Task 6 deletes `legacy/full_status.rs`, whose only users are the 0.13 daemon and CLI. `legacy/state.rs` and `legacy/current_pose.rs` stay: the carried Tiny 2 Lite flows still use them, and track A removes them with the flows it rewrites (spec 3.2). So at the end of W0 the 0.13 daemon, client, protocol, CLI and full status are gone, and the 0.13 state module is not.

## File structure

| File                                                                                                         | Responsibility                                                         | Task        |
| ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- | ----------- |
| `Cargo.toml` (root, modify)                                                                                  | members, `obsbot-proto`/`obsbot4linuxd` path deps, `schemars`          | 1, 11       |
| `crates/obsbot-proto/Cargo.toml`                                                                             | the proto crate                                                        | 1           |
| `crates/obsbot-proto/src/lib.rs`                                                                             | modules, re-exports, protocol constants, `socket_path`                 | 1           |
| `crates/obsbot-proto/src/ids.rs`                                                                             | `RequestId`, serial format, `sn_id`, `port_id`, `short_id`             | 1           |
| `crates/obsbot-proto/src/values.rs`                                                                          | canonical numbers, rounding, `raw:<n>`, value equality                 | 1           |
| `crates/obsbot-proto/src/description.rs`                                                                     | `Device`, `Usb`, `Description`, `Setting`, `Action`, enums, builders   | 1           |
| `crates/obsbot-proto/src/state.rs`                                                                           | `State`, `Patch`, `Params`, `Values`, path helpers                     | 1           |
| `crates/obsbot-proto/src/message.rs`                                                                         | `Request`, `Verb`, `Hello`, `Reply`, `Event`, `ErrorBody`, `ErrorCode` | 1           |
| `crates/obsbot-proto/src/client.rs`                                                                          | reserved for track C                                                   | 1           |
| `crates/obsbot-proto/src/validate.rs`                                                                        | validator and description self-check                                   | 1 (stub), 2 |
| `crates/obsbot-proto/src/schema.rs`                                                                          | schema generation                                                      | 1 (stub), 3 |
| `crates/obsbot-proto/tests/types.rs`, `tests/golden/types.jsonl`                                             | per-type golden round trips                                            | 1           |
| `crates/obsbot-proto/tests/validate.rs`                                                                      | validator table tests                                                  | 2           |
| `crates/obsbot-proto/tests/schema.rs`, `schema/protocol-2.json`                                              | schema and its stale test                                              | 3           |
| `crates/obsbot-proto/tests/transcripts.rs`, `tests/golden/{session,errors,hotplug}.jsonl`                    | golden transcripts                                                     | 4           |
| `crates/obsbot-device/Cargo.toml`, `src/lib.rs` (modify)                                                     | `obsbot-proto` dependency, `contract` and `descriptions` modules       | 5, 10       |
| `crates/obsbot-device/src/contract/{mod,camera,transport,bus}.rs`                                            | the device contract                                                    | 5           |
| `crates/obsbot-device/tests/contract.rs`                                                                     | registry, context, tier and send-table rules                           | 5           |
| `crates/obsbot4linux/**` (modify), `scripts/check-cli-deps.sh`, `.github/workflows/ci.yml` (modify)          | the 0.13 command line removed; CLI stub; dependency check              | 6           |
| `crates/obsbot-device/src/legacy/full_status.rs` (delete), `legacy/mod.rs` (modify)                          | the 0.13 full status removed (decision 15)                             | 6           |
| `crates/obsbot-device/src/contract/conformance.rs`, `tests/conformance.rs`                                   | trait conformance check                                                | 5 (stub), 7 |
| `crates/obsbot-device/src/models/tiny2lite/description.rs`, `tiny2lite/mod.rs` (modify)                      | the Tiny 2 Lite description and its 9.3 marks                          | 8           |
| `crates/obsbot-device/src/models/fake/{mod,camera,a,b,bus}.rs`, `models/mod.rs` (modify), `tests/fakes.rs`   | the fakes and the fake bus                                             | 9           |
| `crates/obsbot-device/src/descriptions.rs`, `tests/generated.rs`, `descriptions/**`, `scripts/regenerate.sh` | generated description files and stale check                            | 10          |
| `crates/obsbot4linuxd/**`                                                                                    | daemon crate stub with its fixed entry point                           | 11          |
| `docs/protocol-2.md`                                                                                         | normative client reference and path dictionary                         | 12          |

## Tasks, waves and dependencies

Several builders work side by side: every task in one wave can run in parallel. A task branches from `rebuild` after its dependencies have landed; where one dependency is still in review, the task is stacked on that branch (column "stacked on").

| Task | Title                                | Wave | Depends on | Stacked on | Effort |
| ---- | ------------------------------------ | ---- | ---------- | ---------- | ------ |
| 1    | Protocol types                       | 1    | –          | –          | L      |
| 2    | Validator                            | 2    | 1          | 1          | M      |
| 3    | Schema and its stale test            | 2    | 1          | 1          | S      |
| 4    | Golden transcripts                   | 2    | 1          | 1          | S      |
| 5    | Device contract types                | 2    | 1          | 1          | M      |
| 6    | Remove the 0.13 command line         | 2    | 1          | 1          | S      |
| 7    | Trait conformance check              | 3    | 2, 5       | –          | M      |
| 8    | Tiny 2 Lite description as data      | 3    | 2, 5       | –          | M      |
| 9    | Fake models and fake bus             | 4    | 7, 8       | –          | L      |
| 12   | `docs/protocol-2.md`, normative part | 4    | 2, 8       | 8          | S      |
| 10   | Generated description files          | 5    | 3, 9       | 9          | S      |
| 11   | Daemon crate stub                    | 5    | 6, 9       | 9          | S      |

Tasks 7 and 8 each depend on two wave-2 tasks (2 and 5): such a task starts from `rebuild` once both have landed, or is stacked on the one of the two that lands last, never on a branch that lacks the other. Task 8 needs Task 5 for the `obsbot-proto` dependency of `crates/obsbot-device/Cargo.toml`. Task 12 needs Task 8, because its path dictionary has a row for every leaf of the Tiny 2 Lite description.

`Cargo.lock` changes in Tasks 1, 5, 6 and 11. When a rebase conflicts in it, take `rebuild`'s version and run `cargo build --workspace`; commit the result.

The code of this plan was built and run in a copy of the workspace on 2026-11-13 (rustc 1.99.1), again after the review's fix round: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and every test below passed. The code in the steps is that code.

---

### Task 1: Protocol types

**Files:**

- Modify: `Cargo.toml` (workspace root)
- Create: `crates/obsbot-proto/Cargo.toml`
- Create: `crates/obsbot-proto/src/lib.rs`, `src/ids.rs`, `src/values.rs`, `src/description.rs`, `src/state.rs`, `src/message.rs`
- Create (stubs filled by later tasks): `crates/obsbot-proto/src/client.rs`, `src/validate.rs`, `src/schema.rs`
- Test: `crates/obsbot-proto/tests/types.rs`, `crates/obsbot-proto/tests/golden/types.jsonl`; unit tests inside `ids.rs`, `values.rs`, `state.rs`

**Interfaces:**

- Consumes: the workspace of stage G.
- Produces (crate `obsbot_proto`; the code below is complete, these are the names other tasks use):
  - constants `PROTOCOL: u32 = 2`, `SOCKET_ENV`, `SOCKET_NAME`, `MAX_REQUEST_LINE = 65536`, `REQUEST_DEADLINE = 15 s`, `CLIENT_REPLY_TIMEOUT = 30 s`; `fn socket_path() -> Option<PathBuf>`
  - `ids`: `enum RequestId { Int(i64), Str(String) }`, `fn is_valid_serial(&str) -> bool`, `fn sn_id(&str) -> String`, `fn port_id(&str) -> String`, `fn short_id(&str) -> String`
  - `values`: `fn number(f64) -> Value`, `fn rounded(f64, precision: f64) -> Value`, `fn raw(i64) -> Value`, `fn is_raw(&str) -> bool`, `fn same(&Value, &Value) -> bool`
  - `description`: `struct Device { id, name, model, support: Support, serial: Option<String>, firmware: Option<String>, video_node: Option<String>, available: bool, replaced_by: Option<String>, usb: Usb, description: Description (flattened) }`, `struct Usb { vendor, product, product_name: Option<String>, bcd_device, path, serial_present: bool, interfaces: Vec<String>, extension_units: Vec<ExtensionUnit> }`, `struct ExtensionUnit { unit: u8, guid: String, controls: u32 }`, `struct Description { settings: BTreeMap<String, Setting>, actions: BTreeMap<String, Action>, limits: BTreeMap<String, u32> }` with `setting`, `action`, `limit`, `is_group`; `struct Setting { kind: SettingType, label, readonly, nullable, readback, asleep: Option<Asleep>, no_effect, values, writable, min, max, step: Option<Bound>, unit: Option<UnitSpec>, precision: Option<Bound>, max_len: Option<u32>, ascii, item: Option<BTreeMap<String, Setting>>, optional, requires: Vec<Requirement>, default: Option<Value>, live: Option<String>, settle_ms: Option<u32>, warning: Vec<Warning> }` with builders `new`, `boolean`, `enumeration`, `number`, `string`, `list`, `composite`, `range`, `min`, `max`, `step`, `precision`, `precisions`, `unit`, `units`, `writable_only`, `readonly`, `nullable`, `no_readback`, `asleep`, `no_effect`, `optional`, `requires`, `default_value`, `live`, `settle_ms`, `warning`; `struct Action { label, params, result, asleep: Asleep, requires, warning }` with `new`, `param`, `result`, `requires`, `warning`; `enum Support`, `enum SettingType` (with `components()`), `enum Unit`, `enum Bound { One(Number), Each(BTreeMap<String, Number>) }` (with `get(Option<&str>)`), `enum UnitSpec`, `enum Asleep`, `enum Warning`, `struct Requirement { path, equals: Value }`
  - `state`: `type Params = Map<String, Value>`, `type Values = BTreeMap<String, Value>`, `const DAEMON_PATHS`, `struct State { available, pending: BTreeMap<String, Value>, unread: Vec<String>, values: Map<String, Value> (flattened) }` with `new`, `at`, `set`, `with`, `select`, `asleep`; `struct Patch(pub Map<String, Value>)` with `new`, `leaf`, `is_empty`
  - `validate` (stub): `struct Refusal { code: ErrorCode, path: Option<String>, message: String }` with `new(ErrorCode, &str, &str)`
  - `message`: `struct Hello { protocol, client, subscribe, live }` with `new`, `subscribe`, `live`; `struct Request { id: RequestId, verb: Verb }` with `from_line(&str) -> Result<Request, RequestError>`; `enum Verb { Hello(Hello), Get { device, paths: Option<Vec<String>> }, Set { device, patch: Patch }, Do { device, action, params: Params }, Live { topics } }` with `device()`; `struct RequestError { id: Option<RequestId>, message }`; `struct Reply { id: Option<RequestId>, body: ReplyBody }`; `enum ReplyBody { Ok(OkBody), Error(ErrorBody) }`; `enum OkBody { Hello(HelloOk), State(RevState), Rev(RevOk), Empty(Empty) }`; `struct HelloOk { protocol, daemon, devices: Vec<Device>, states: BTreeMap<String, RevState> }`; `struct RevState { rev: u64, state: State }`; `struct RevOk { rev: u64, result: Option<Params> }`; `struct ErrorBody { code, message, rev, applied, failed, supported }` with `new`; `enum ErrorCode` with `ALL: [ErrorCode; 12]` and `as_str()`; `enum Event { State { device, rev, state }, Devices { devices }, Unknown }`; `enum ServerMessage { Event(Event), Reply(Reply) }`

- [ ] **Step 1: Add the crate to the workspace**

In the root `Cargo.toml`, make sure `[workspace] members` includes `"crates/obsbot-proto"` (if G uses `members = ["crates/*"]`, nothing to do), and add to `[workspace.dependencies]`:

```toml
obsbot-proto = { path = "crates/obsbot-proto" }
schemars = "1.2"
```

`[workspace.dependencies]` must already hold `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"` (from stage G); add them in this form if not.

Create `crates/obsbot-proto/Cargo.toml`:

```toml
[package]
name = "obsbot-proto"
description = "Protocol 2 of obsbot4linux: messages, device descriptions, validator, schema and client"
version.workspace = true
edition = "2024"
rust-version.workspace = true
license.workspace = true
publish = false

[dependencies]
schemars.workspace = true
serde.workspace = true
serde_json.workspace = true
```

Create `crates/obsbot-proto/src/lib.rs` with only the license line for now:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
```

- [ ] **Step 2: Write the failing tests**

Create `crates/obsbot-proto/tests/golden/types.jsonl`:

```text
# One line per case: the type name, a space, the canonical JSON of one value of that type.
# Every line must parse into its type and serialise back byte for byte (tests/types.rs).
RequestId 7
RequestId "a-7"
Support "tested"
Support "untested"
Support "unsupported"
Support "beta"
Unit "deg/s"
Unit "K"
Unit "lux"
UnitSpec "fraction"
UnitSpec {"pan":"deg","tilt":"deg","zoom":"factor"}
Bound 0.01
Bound {"max":150,"min":-150}
Asleep "ignored"
Asleep "sometimes"
Warning "destructive"
Warning "explode"
SettingType "box"
SettingType "matrix"
ErrorCode "out_of_range"
ErrorCode "rate_limited"
Requirement {"path":"exposure.mode","equals":"manual"}
ExtensionUnit {"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}
Usb {"vendor":"3564","product":"fa02","product_name":null,"bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e","01","01"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]}
Setting {"type":"bool","label":"Invert pan direction","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"settle_ms":3000}
Setting {"type":"enum","label":"Field of view","readonly":false,"nullable":false,"readback":true,"asleep":"ignored","no_effect":false,"values":["wide","medium","narrow","custom"],"writable":["wide","medium","narrow"],"settle_ms":3000}
Setting {"type":"number","label":"Exposure time","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":100,"max":250000,"unit":"us","precision":1,"requires":[{"path":"exposure.mode","equals":"manual"}],"default":33000,"live":"auto","settle_ms":3000}
Setting {"type":"range","label":"Zone pan","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":{"max":-150,"min":-150},"max":{"max":150,"min":150},"unit":"deg","precision":0.1,"settle_ms":3000}
Setting {"type":"pose","label":"Hand return pose","readonly":true,"nullable":true,"readback":true,"no_effect":false,"min":{"pan":-150,"tilt":-70,"zoom":1},"max":{"pan":150,"tilt":90,"zoom":4},"unit":{"pan":"deg","tilt":"deg","zoom":"factor"},"precision":{"pan":0.1,"tilt":0.1,"zoom":0.01}}
Setting {"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true,"optional":true}
Setting {"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}
Setting {"type":"bool","label":"LED blink","readonly":false,"nullable":false,"readback":false,"asleep":"unknown","no_effect":false}
Setting {"type":"bool","label":"Microphone","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"settle_ms":3000,"warning":["restart"]}
Setting {"type":"number","label":"Wake zoom","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":true,"min":1,"max":4,"unit":"factor","precision":0.01,"settle_ms":3000}
Setting {"type":"matrix","label":"Colour matrix","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"settle_ms":3000}
Action {"label":"Stop the gimbal","params":{},"asleep":"unknown"}
Action {"label":"Save a view","params":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true}},"result":{"id":{"type":"number","label":"View id","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"step":1}},"asleep":"ignored"}
Action {"label":"Factory reset","params":{"confirm":{"type":"bool","label":"Confirm","readonly":false,"nullable":false,"readback":true,"no_effect":false}},"asleep":"unknown","warning":["destructive","restart"]}
Description {"settings":{"led.blink":{"type":"bool","label":"LED blink","readonly":false,"nullable":false,"readback":false,"asleep":"unknown","no_effect":false}},"actions":{"gimbal.stop":{"label":"Stop the gimbal","params":{},"asleep":"unknown"}},"limits":{"presets":3}}
Device {"id":"port:3-2.1","name":"USB 3564:fa02 3-2.1","model":"unknown","support":"unsupported","serial":null,"firmware":null,"video_node":"/dev/video4","available":true,"usb":{"vendor":"3564","product":"fa02","product_name":null,"bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[]},"settings":{},"actions":{},"limits":{}}
Device {"id":"port:3-2.2","name":"Example Camera 3-2.2","model":"example","support":"tested","serial":null,"firmware":null,"video_node":null,"available":false,"replaced_by":"sn:EXAMPLE0000001","usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.2","serial_present":false,"interfaces":["0e"],"extension_units":[]},"settings":{},"actions":{},"limits":{}}
State {"available":true,"pending":{"led.brightness":2},"unread":["presets"],"asleep":false,"led":{"brightness":1},"presets":null,"zone":{"area":{"pan":{"max":20,"min":-20}}}}
Patch {"exposure":{"mode":"manual","time":20000},"led.brightness":2}
Hello {"protocol":2,"client":"widget 1.0","subscribe":true,"live":["pose"]}
Request {"id":1,"hello":{"protocol":2,"client":"obsbot4linux 1.0.0","subscribe":false,"live":[]}}
Request {"id":2,"device":"sn:EXAMPLE0000001","get":null}
Request {"id":3,"device":"sn:EXAMPLE0000001","get":["zoom.factor","presets"]}
Request {"id":4,"device":"sn:EXAMPLE0000001","set":{"zone":{"area":{"pan":{"max":20,"min":-20}}}}}
Request {"id":"v","device":"sn:EXAMPLE0000001","do":"view.save","with":{"name":"Desk","pose":{"pan":0,"tilt":5,"zoom":1.2}}}
Request {"id":6,"device":"sn:EXAMPLE0000001","do":"gimbal.stop"}
Request {"id":7,"live":["pose","target"]}
Reply {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}
Reply {"id":2,"ok":{"rev":5,"state":{"available":true,"pending":{},"unread":[],"zoom":{"factor":1.5}}}}
Reply {"id":4,"ok":{"rev":6}}
Reply {"id":"v","ok":{"rev":7,"result":{"id":3}}}
Reply {"id":7,"ok":{}}
Reply {"id":8,"error":{"code":"camera","message":"the camera did not answer","rev":9,"applied":["exposure.mode"],"failed":"exposure.time"}}
Reply {"id":null,"error":{"code":"protocol","message":"the first message must be hello","supported":[2]}}
ErrorBody {"code":"out_of_range","message":"led.brightness: 4 is above the maximum 3"}
Event {"event":"state","device":"sn:EXAMPLE0000001","rev":6,"state":{"available":true,"pending":{"led.brightness":2},"unread":[],"led":{"brightness":1}}}
Event {"event":"devices","devices":[]}
ServerMessage {"event":"devices","devices":[]}
ServerMessage {"id":4,"ok":{"rev":6}}
```

Create `crates/obsbot-proto/tests/types.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Every protocol type round-trips byte for byte against `tests/golden/types.jsonl`, and the
//! additive-change rules of 4.1 rule 7 hold: unknown enum values, types and codes parse.

use obsbot_proto::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

const GOLDEN: &str = include_str!("golden/types.jsonl");

fn round<T: Serialize + DeserializeOwned>(json: &str) -> T {
    let value: T =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("does not parse: {e}\n{json}"));
    let back = serde_json::to_string(&value).unwrap();
    assert_eq!(back, json, "not byte-equal after a round trip");
    value
}

#[test]
fn every_golden_line_round_trips_byte_equal() {
    let mut seen = std::collections::BTreeSet::new();
    for line in GOLDEN
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (name, json) = line.split_once(' ').expect("type name, space, JSON");
        seen.insert(name);
        match name {
            "RequestId" => drop(round::<RequestId>(json)),
            "Support" => drop(round::<Support>(json)),
            "Unit" => drop(round::<Unit>(json)),
            "UnitSpec" => drop(round::<UnitSpec>(json)),
            "Bound" => drop(round::<Bound>(json)),
            "Asleep" => drop(round::<Asleep>(json)),
            "Warning" => drop(round::<Warning>(json)),
            "SettingType" => drop(round::<SettingType>(json)),
            "ErrorCode" => drop(round::<ErrorCode>(json)),
            "Requirement" => drop(round::<Requirement>(json)),
            "ExtensionUnit" => drop(round::<ExtensionUnit>(json)),
            "Usb" => drop(round::<Usb>(json)),
            "Setting" => drop(round::<Setting>(json)),
            "Action" => drop(round::<Action>(json)),
            "Description" => drop(round::<Description>(json)),
            "Device" => drop(round::<Device>(json)),
            "State" => drop(round::<State>(json)),
            "Patch" => drop(round::<Patch>(json)),
            "Hello" => drop(round::<Hello>(json)),
            "Request" => drop(round::<Request>(json)),
            "Reply" => drop(round::<Reply>(json)),
            "ErrorBody" => drop(round::<ErrorBody>(json)),
            "Event" => drop(round::<Event>(json)),
            "ServerMessage" => drop(round::<ServerMessage>(json)),
            other => panic!("unknown type name {other}"),
        }
    }
    assert_eq!(seen.len(), 24, "every type has at least one golden line");
}

#[test]
fn additive_values_from_a_newer_daemon_still_parse() {
    assert_eq!(round::<Support>(r#""beta""#), Support::Other("beta".into()));
    assert_eq!(
        round::<SettingType>(r#""matrix""#),
        SettingType::Other("matrix".into())
    );
    assert_eq!(round::<Unit>(r#""lux""#), Unit::Other("lux".into()));
    assert_eq!(
        round::<ErrorCode>(r#""rate_limited""#),
        ErrorCode::Other("rate_limited".into())
    );
    let unknown: ServerMessage = serde_json::from_str(r#"{"event":"battery","level":3}"#).unwrap();
    assert_eq!(
        unknown,
        ServerMessage::Event(Event::Unknown),
        "unknown event types are ignorable"
    );
    let reply: Reply =
        serde_json::from_str(r#"{"id":4,"ok":{"rev":6,"extra":true},"trace":"x"}"#).unwrap();
    assert_eq!(
        reply.body,
        ReplyBody::Ok(OkBody::Rev(RevOk {
            rev: 6,
            result: None
        })),
        "unknown fields are ignored"
    );
}

#[test]
fn requests_parse_into_their_verbs() {
    // Review Focus 4: a newer client's extra fields are ignored, not refused.
    let extra = Request::from_line(r#"{"id":9,"device":"d","get":null,"trace":"abc"}"#).unwrap();
    assert_eq!(
        extra.verb,
        Verb::Get {
            device: "d".into(),
            paths: None
        }
    );
    let get = Request::from_line(r#"{"id":2,"device":"sn:EXAMPLE0000001","get":null}"#).unwrap();
    assert_eq!(
        get.verb,
        Verb::Get {
            device: "sn:EXAMPLE0000001".into(),
            paths: None
        },
        "get:null is the full state"
    );
    let stop = Request::from_line(r#"{"id":6,"device":"d","do":"gimbal.stop"}"#).unwrap();
    assert_eq!(
        stop.verb,
        Verb::Do {
            device: "d".into(),
            action: "gimbal.stop".into(),
            params: Params::new()
        },
        "with defaults to {{}}"
    );
    let hello: Request =
        serde_json::from_str(r#"{"id":1,"hello":{"protocol":2,"client":"x"}}"#).unwrap();
    assert_eq!(
        hello.verb,
        Verb::Hello(Hello {
            protocol: 2,
            client: "x".into(),
            subscribe: true,
            live: vec![]
        }),
        "subscribe defaults to true on the wire"
    );
    assert_eq!(
        serde_json::to_value(Hello::new("cli 1.0")).unwrap(),
        json!({"protocol":2,"client":"cli 1.0","subscribe":false,"live":[]})
    );
}

#[test]
fn malformed_requests_are_refused_with_their_id_when_it_is_usable() {
    let cases = [
        (r#"{"id":1}"#, Some(RequestId::Int(1)), "no verb"),
        (
            r#"{"id":1,"device":"d","get":null,"set":{}}"#,
            Some(RequestId::Int(1)),
            "more than one verb",
        ),
        (r#"{"device":"d","get":null}"#, None, "no id"),
        (
            r#"{"id":"x","get":null}"#,
            Some(RequestId::Str("x".into())),
            "names no device",
        ),
        (
            r#"{"id":2,"device":"d","set":[1]}"#,
            Some(RequestId::Int(2)),
            "malformed",
        ),
        (r#"{"id":1.5,"live":[]}"#, None, "malformed"),
        ("not json", None, "not JSON"),
        (r#"[1,2]"#, None, "malformed"),
    ];
    for (line, id, part) in cases {
        let err = Request::from_line(line).unwrap_err();
        assert_eq!(err.id, id, "{line}");
        assert!(err.message.contains(part), "{line}: {}", err.message);
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p obsbot-proto --test types`
Expected: FAIL to compile, `error[E0432]: unresolved import` / `cannot find type` for `RequestId`, `Support` and the other types.

- [ ] **Step 4: Write the types**

Replace `crates/obsbot-proto/src/lib.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Protocol 2 of obsbot4linux (design spec, section 4): the message envelopes, the error
//! codes, the device descriptions, the validator, path helpers and the JSON schema. The
//! synchronous client is the module `client`.
//!
//! State, patches and action parameters are generic: leaf path → JSON value, checked at run
//! time against the device description. There are no per-setting types or methods.

pub mod client;
pub mod description;
pub mod ids;
pub mod message;
pub mod schema;
pub mod state;
pub mod validate;
pub mod values;

pub use description::{
    Action, Asleep, Bound, Description, Device, ExtensionUnit, Requirement, Setting, SettingType,
    Support, Unit, UnitSpec, Usb, Warning,
};
pub use ids::{RequestId, is_valid_serial, port_id, short_id, sn_id};
pub use message::{
    Empty, ErrorBody, ErrorCode, Event, Hello, HelloOk, OkBody, Reply, ReplyBody, Request,
    RequestError, RevOk, RevState, ServerMessage, Verb,
};
pub use state::{Params, Patch, State, Values};

use std::path::PathBuf;
use std::time::Duration;

/// The protocol number this crate speaks (`hello.protocol`).
pub const PROTOCOL: u32 = 2;
/// Environment variable that overrides the socket path.
pub const SOCKET_ENV: &str = "OBSBOT4LINUX_SOCKET";
/// File name of the socket in `$XDG_RUNTIME_DIR`.
pub const SOCKET_NAME: &str = "obsbot4linux.sock";
/// A request line longer than this closes the connection (4.1).
pub const MAX_REQUEST_LINE: usize = 64 * 1024;
/// Every device request is answered within this time of its arrival (4.1 rule 9, 5.8).
pub const REQUEST_DEADLINE: Duration = Duration::from_secs(15);
/// The client library reports a lost daemon after this time without a reply (5.8).
pub const CLIENT_REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// The socket path: `$OBSBOT4LINUX_SOCKET` if set and not empty, else
/// `$XDG_RUNTIME_DIR/obsbot4linux.sock`; `None` when neither variable is set.
pub fn socket_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(SOCKET_ENV).filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|d| !d.is_empty())
        .map(|dir| PathBuf::from(dir).join(SOCKET_NAME))
}
```

Create `crates/obsbot-proto/src/ids.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Request ids, device ids and the one formatter that shortens serials (4.2, 5.5).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The `id` of a request: an integer or a string, echoed verbatim in its reply.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RequestId {
    Int(i64),
    Str(String),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::Int(n) => write!(f, "{n}"),
            RequestId::Str(s) => write!(f, "{s:?}"),
        }
    }
}

/// Whether a serial is valid for an `sn:` id: `[A-Za-z0-9]{8,32}` (5.5). The pattern is part
/// of the id format of protocol 2.
pub fn is_valid_serial(serial: &str) -> bool {
    (8..=32).contains(&serial.len()) && serial.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `sn:<serial>`; the caller has checked the serial with [`is_valid_serial`].
pub fn sn_id(serial: &str) -> String {
    format!("sn:{serial}")
}

/// `port:<usb path>`, for a device without a valid serial.
pub fn port_id(usb_path: &str) -> String {
    format!("port:{usb_path}")
}

/// The id or serial as human output, logs and the journal show it (4.2): an `sn:` id or a bare
/// serial becomes `…` plus its last four characters; a `port:` id is shown as it is.
pub fn short_id(id_or_serial: &str) -> String {
    if id_or_serial.starts_with("port:") {
        return id_or_serial.to_string();
    }
    let serial = id_or_serial.strip_prefix("sn:").unwrap_or(id_or_serial);
    let chars: Vec<char> = serial.chars().collect();
    let tail: String = chars[chars.len().saturating_sub(4)..].iter().collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serials_follow_the_id_format_of_protocol_2() {
        assert!(is_valid_serial("EXAMPLE0000001"));
        assert!(is_valid_serial("ABCD1234"));
        assert!(!is_valid_serial("ABC1234"), "7 characters is too short");
        assert!(
            !is_valid_serial(&"A".repeat(33)),
            "33 characters is too long"
        );
        assert!(
            !is_valid_serial("EXAMPLE-000001"),
            "only letters and digits"
        );
        assert!(!is_valid_serial(""));
    }

    #[test]
    fn human_output_never_shows_more_than_four_serial_characters() {
        assert_eq!(short_id("sn:EXAMPLE0000001"), "…0001");
        assert_eq!(short_id("EXAMPLE0000001"), "…0001");
        assert_eq!(short_id("port:3-2.1"), "port:3-2.1");
        assert_eq!(sn_id("EXAMPLE0000001"), "sn:EXAMPLE0000001");
        assert_eq!(port_id("3-2.1"), "port:3-2.1");
    }
}
```

Create `crates/obsbot-proto/src/values.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Value helpers shared by the daemon, the models and the clients: canonical numbers,
//! rounding to a precision (4.2 "Precision"), `raw:<n>` for undecodable values and value
//! equality that treats `1` and `1.0` as the same number.

use serde_json::{Number, Value};

/// A JSON number in canonical form: an integral value within ±2^53 is written as an integer
/// (`3`, not `3.0`), anything else as a float. Non-finite input gives `null`.
pub fn number(x: f64) -> Value {
    if !x.is_finite() {
        return Value::Null;
    }
    if x.fract() == 0.0 && x.abs() < 9_007_199_254_740_992.0 {
        return Value::Number(Number::from(x as i64));
    }
    Number::from_f64(x).map_or(Value::Null, Value::Number)
}

/// `x` rounded to `precision` (0.1, 0.01, 1, 100 …) in canonical form, so that state and
/// `--json` never show binary float noise.
pub fn rounded(x: f64, precision: f64) -> Value {
    if precision <= 0.0 {
        return number(x);
    }
    let steps = (x / precision).round();
    let decimals = (-precision.log10()).ceil().max(0.0) as i32;
    let scale = 10f64.powi(decimals);
    number((steps * precision * scale).round() / scale)
}

/// The published form of a camera value the model cannot decode: `raw:<n>` (4.2).
pub fn raw(n: i64) -> Value {
    Value::String(format!("raw:{n}"))
}

/// Whether a string is a `raw:<n>` value.
pub fn is_raw(text: &str) -> bool {
    text.strip_prefix("raw:").is_some_and(|n| {
        !n.is_empty()
            && n.trim_start_matches('-')
                .bytes()
                .all(|b| b.is_ascii_digit())
    })
}

/// Value equality in which numbers compare by value (`1` equals `1.0`), recursively.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn integral_numbers_are_written_without_a_fraction() {
        assert_eq!(number(3.0).to_string(), "3");
        assert_eq!(number(-150.0).to_string(), "-150");
        assert_eq!(number(1.5).to_string(), "1.5");
        assert_eq!(number(f64::NAN), Value::Null);
    }

    #[test]
    fn rounding_removes_float_noise() {
        assert_eq!(rounded(0.1 + 0.2, 0.1).to_string(), "0.3");
        assert_eq!(rounded(12.345_678, 0.01).to_string(), "12.35");
        assert_eq!(rounded(-30.04, 0.1).to_string(), "-30");
        assert_eq!(rounded(1234.0, 100.0).to_string(), "1200");
    }

    #[test]
    fn raw_values_are_recognised() {
        assert_eq!(raw(3), json!("raw:3"));
        assert!(is_raw("raw:3"));
        assert!(is_raw("raw:-1"));
        assert!(!is_raw("raw:"));
        assert!(!is_raw("raw:x"));
        assert!(!is_raw("auto"));
    }

    #[test]
    fn numbers_compare_by_value() {
        assert!(same(&json!(1), &json!(1.0)));
        assert!(same(
            &json!({"min": 1, "max": 2}),
            &json!({"max": 2.0, "min": 1.0})
        ));
        assert!(!same(&json!(1), &json!("1")));
        assert!(!same(&json!([1]), &json!([1, 2])));
    }
}
```

Create `crates/obsbot-proto/src/description.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Device entries and descriptions (4.2): `Device`, `Usb`, `Description`, `Setting`, `Action`
//! and their enums, plus builder methods that models use to write their descriptions.
//!
//! Wire rules: maps are sorted by key; `readonly`, `nullable`, `readback` and `no_effect` are
//! always written; every other optional field is left out when it is empty. Each public enum
//! has a catch-all variant, so a newer daemon's additive values (4.1 rule 7) still parse.

use crate::values::number;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

fn yes() -> bool {
    true
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// One entry of the device list (4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Device {
    /// `sn:<serial>` or `port:<usb path>` (5.5).
    pub id: String,
    pub name: String,
    /// The model key (`tiny2lite`, `fake-a`, `fake-b`), or `unknown` for an unsupported device.
    pub model: String,
    pub support: Support,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    /// `/dev/videoN` of the video node with sysfs index 0.
    pub video_node: Option<String>,
    pub available: bool,
    /// Set only on the old `port:` entry during an identity hand-over (5.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
    pub usb: Usb,
    #[serde(flatten)]
    pub description: Description,
}

/// The USB facts of a device, read from sysfs at enumeration (4.2). Never holds the serial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Usb {
    /// Four lower-case hex digits (`"3564"`).
    pub vendor: String,
    pub product: String,
    pub product_name: Option<String>,
    pub bcd_device: String,
    /// The USB device name in sysfs (`"3-2.1"`).
    pub path: String,
    pub serial_present: bool,
    /// Interface classes in interface order, two lower-case hex digits (`"0e"`, `"01"`).
    pub interfaces: Vec<String>,
    pub extension_units: Vec<ExtensionUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionUnit {
    pub unit: u8,
    /// Lower-case `8-4-4-4-12` form.
    pub guid: String,
    pub controls: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Support {
    Tested,
    Untested,
    Unsupported,
    /// A level this crate does not know (a newer daemon).
    #[serde(untagged)]
    Other(String),
}

/// What a model offers (4.2): the part of a `Device` that `descriptions/<model>.json` holds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Description {
    /// Leaf path → setting.
    pub settings: BTreeMap<String, Setting>,
    /// Action name → action.
    pub actions: BTreeMap<String, Action>,
    /// List path → maximum item count, where the camera has one.
    pub limits: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SettingType {
    Bool,
    Enum,
    Number,
    /// `{min, max}`
    Range,
    /// `{pan, tilt, zoom}`
    Pose,
    /// `{x, y}`
    Point,
    /// `{left, top, right, bottom}`
    Box,
    String,
    /// A read-only list of objects; `item` describes their fields.
    List,
    #[serde(untagged)]
    Other(String),
}

impl SettingType {
    /// The component names of an object-shaped type, `None` for the other types.
    pub fn components(&self) -> Option<&'static [&'static str]> {
        match self {
            SettingType::Range => Some(&["min", "max"]),
            SettingType::Pose => Some(&["pan", "tilt", "zoom"]),
            SettingType::Point => Some(&["x", "y"]),
            SettingType::Box => Some(&["left", "top", "right", "bottom"]),
            _ => None,
        }
    }
}

/// The closed unit list of 4.2; absent for a raw camera value. New units are additive.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub enum Unit {
    #[serde(rename = "deg")]
    Deg,
    #[serde(rename = "deg/s")]
    DegPerSecond,
    #[serde(rename = "factor")]
    Factor,
    #[serde(rename = "us")]
    Microseconds,
    #[serde(rename = "K")]
    Kelvin,
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "s")]
    Seconds,
    #[serde(rename = "fraction")]
    Fraction,
    #[serde(rename = "fps")]
    Fps,
    #[serde(untagged)]
    Other(String),
}

/// A bound, step or precision: one number for `number`; for `range`, `pose`, `point` and
/// `box` an object of the same shape as the value, one number per component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Bound {
    One(Number),
    Each(BTreeMap<String, Number>),
}

impl Bound {
    /// The bound of one component (`None` for the whole value of a `number`).
    pub fn get(&self, component: Option<&str>) -> Option<f64> {
        match (self, component) {
            (Bound::One(n), _) => n.as_f64(),
            (Bound::Each(map), Some(c)) => map.get(c).and_then(Number::as_f64),
            (Bound::Each(_), None) => None,
        }
    }
}

/// A unit: one for the whole value, or one per component (a `pose` has degrees and a factor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UnitSpec {
    One(Unit),
    Each(BTreeMap<String, Unit>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Asleep {
    Accepted,
    Ignored,
    Unknown,
    #[serde(untagged)]
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Warning {
    Restart,
    Destructive,
    #[serde(untagged)]
    Other(String),
}

/// A condition on another path, checked against the current state with the patch applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Requirement {
    pub path: String,
    pub equals: Value,
}

/// One leaf of the state, or one action parameter or result field (4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Setting {
    #[serde(rename = "type")]
    pub kind: SettingType,
    pub label: String,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub nullable: bool,
    /// `false`: the camera accepts the setting but cannot report it.
    #[serde(default = "yes")]
    pub readback: bool,
    /// Present on every writable leaf; absent on read-only leaves and parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asleep: Option<Asleep>,
    #[serde(default)]
    pub no_effect: bool,
    /// enum: every value a state may show.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    /// enum: the values a `set` accepts. Present on every writable enum, absent on read-only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writable: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<Bound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Bound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<Bound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<UnitSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<Bound>,
    /// string: the maximum length in characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_len: Option<u32>,
    /// string: printable ASCII only (0x20 to 0x7e).
    #[serde(default, skip_serializing_if = "is_false")]
    pub ascii: bool,
    /// list: the fields of one item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<BTreeMap<String, Setting>>,
    /// Action parameters only: the parameter may be left out.
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
    /// The camera's own default as it reports it (UVC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// The live topic of a volatile leaf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<String>,
    /// The longest time the readback may lag a write of this leaf (4.1 rule 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settle_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning: Vec<Warning>,
}

/// One action of a device (4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Action {
    pub label: String,
    /// Parameter name → setting; empty for an action without parameters.
    #[serde(default)]
    pub params: BTreeMap<String, Setting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<BTreeMap<String, Setting>>,
    pub asleep: Asleep,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning: Vec<Warning>,
}

fn num(x: f64) -> Number {
    match number(x) {
        Value::Number(n) => n,
        _ => panic!("description bound {x} is not a finite number"),
    }
}

fn each(pairs: &[(&str, f64)]) -> Bound {
    Bound::Each(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), num(*v)))
            .collect(),
    )
}

/// Builders for model descriptions. They never touch the wire format.
impl Setting {
    /// A writable setting of `kind` with no constraints.
    pub fn new(kind: SettingType, label: &str) -> Setting {
        Setting {
            kind,
            label: label.to_string(),
            readonly: false,
            nullable: false,
            readback: true,
            asleep: None,
            no_effect: false,
            values: Vec::new(),
            writable: Vec::new(),
            min: None,
            max: None,
            step: None,
            unit: None,
            precision: None,
            max_len: None,
            ascii: false,
            item: None,
            optional: false,
            requires: Vec::new(),
            default: None,
            live: None,
            settle_ms: None,
            warning: Vec::new(),
        }
    }

    pub fn boolean(label: &str) -> Setting {
        Setting::new(SettingType::Bool, label)
    }

    /// An enum whose values are all writable; narrow with [`Setting::writable_only`].
    pub fn enumeration(label: &str, values: &[&str]) -> Setting {
        let mut s = Setting::new(SettingType::Enum, label);
        s.values = values.iter().map(|v| v.to_string()).collect();
        s.writable = s.values.clone();
        s
    }

    /// A number between `min` and `max`.
    pub fn number(label: &str, min: f64, max: f64) -> Setting {
        Setting::new(SettingType::Number, label).min(min).max(max)
    }

    /// A string of at most `max_len` characters.
    pub fn string(label: &str, max_len: u32, ascii: bool) -> Setting {
        let mut s = Setting::new(SettingType::String, label);
        s.max_len = Some(max_len);
        s.ascii = ascii;
        s
    }

    /// A read-only list whose items have the given fields.
    pub fn list(label: &str, item: &[(&str, Setting)]) -> Setting {
        let mut s = Setting::new(SettingType::List, label).readonly();
        s.item = Some(
            item.iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        );
        s
    }

    /// An object-shaped setting (`range`, `pose`, `point`, `box`) with per-component bounds.
    pub fn composite(
        kind: SettingType,
        label: &str,
        min: &[(&str, f64)],
        max: &[(&str, f64)],
    ) -> Setting {
        let mut s = Setting::new(kind, label);
        s.min = Some(each(min));
        s.max = Some(each(max));
        s
    }

    /// A `range` whose two ends both lie between `lo` and `hi`.
    pub fn range(label: &str, lo: f64, hi: f64) -> Setting {
        Setting::composite(
            SettingType::Range,
            label,
            &[("min", lo), ("max", lo)],
            &[("min", hi), ("max", hi)],
        )
    }

    pub fn min(mut self, x: f64) -> Setting {
        self.min = Some(Bound::One(num(x)));
        self
    }

    pub fn max(mut self, x: f64) -> Setting {
        self.max = Some(Bound::One(num(x)));
        self
    }

    pub fn step(mut self, x: f64) -> Setting {
        self.step = Some(Bound::One(num(x)));
        self
    }

    pub fn precision(mut self, x: f64) -> Setting {
        self.precision = Some(Bound::One(num(x)));
        self
    }

    pub fn precisions(mut self, pairs: &[(&str, f64)]) -> Setting {
        self.precision = Some(each(pairs));
        self
    }

    pub fn unit(mut self, unit: Unit) -> Setting {
        self.unit = Some(UnitSpec::One(unit));
        self
    }

    pub fn units(mut self, pairs: &[(&str, Unit)]) -> Setting {
        self.unit = Some(UnitSpec::Each(
            pairs
                .iter()
                .map(|(k, u)| (k.to_string(), u.clone()))
                .collect(),
        ));
        self
    }

    /// Narrows the writable values of an enum.
    pub fn writable_only(mut self, values: &[&str]) -> Setting {
        self.writable = values.iter().map(|v| v.to_string()).collect();
        self
    }

    /// Read-only: no writable values, no asleep behaviour, no settle bound.
    pub fn readonly(mut self) -> Setting {
        self.readonly = true;
        self.writable.clear();
        self.asleep = None;
        self.settle_ms = None;
        self
    }

    pub fn nullable(mut self) -> Setting {
        self.nullable = true;
        self
    }

    /// `readback: false`: the camera accepts the value but cannot report it; no settle bound.
    pub fn no_readback(mut self) -> Setting {
        self.readback = false;
        self.settle_ms = None;
        self
    }

    pub fn asleep(mut self, asleep: Asleep) -> Setting {
        self.asleep = Some(asleep);
        self
    }

    pub fn no_effect(mut self) -> Setting {
        self.no_effect = true;
        self
    }

    pub fn optional(mut self) -> Setting {
        self.optional = true;
        self
    }

    pub fn requires(mut self, path: &str, equals: Value) -> Setting {
        self.requires.push(Requirement {
            path: path.to_string(),
            equals,
        });
        self
    }

    pub fn default_value(mut self, value: Value) -> Setting {
        self.default = Some(value);
        self
    }

    pub fn live(mut self, topic: &str) -> Setting {
        self.live = Some(topic.to_string());
        self
    }

    pub fn settle_ms(mut self, ms: u32) -> Setting {
        self.settle_ms = Some(ms);
        self
    }

    pub fn warning(mut self, warning: Warning) -> Setting {
        self.warning.push(warning);
        self
    }
}

impl Action {
    pub fn new(label: &str, asleep: Asleep) -> Action {
        Action {
            label: label.to_string(),
            params: BTreeMap::new(),
            result: None,
            asleep,
            requires: Vec::new(),
            warning: Vec::new(),
        }
    }

    pub fn param(mut self, name: &str, setting: Setting) -> Action {
        self.params.insert(name.to_string(), setting);
        self
    }

    pub fn result(mut self, name: &str, setting: Setting) -> Action {
        self.result
            .get_or_insert_with(BTreeMap::new)
            .insert(name.to_string(), setting);
        self
    }

    pub fn requires(mut self, path: &str, equals: Value) -> Action {
        self.requires.push(Requirement {
            path: path.to_string(),
            equals,
        });
        self
    }

    pub fn warning(mut self, warning: Warning) -> Action {
        self.warning.push(warning);
        self
    }
}

impl Description {
    pub fn setting(mut self, path: &str, setting: Setting) -> Description {
        self.settings.insert(path.to_string(), setting);
        self
    }

    pub fn action(mut self, name: &str, action: Action) -> Description {
        self.actions.insert(name.to_string(), action);
        self
    }

    pub fn limit(mut self, list: &str, max_items: u32) -> Description {
        self.limits.insert(list.to_string(), max_items);
        self
    }

    /// Whether `path` is a group: a proper prefix (followed by `.`) of a setting path.
    pub fn is_group(&self, path: &str) -> bool {
        let prefix = format!("{path}.");
        self.settings.keys().any(|k| k.starts_with(&prefix))
    }
}
```

Create `crates/obsbot-proto/src/state.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The generic payload: state, patches, action parameters and path helpers (4.1, 4.2, 6.1).
//!
//! A state is a nested JSON object in the shape of the model's state tree (section 4.3 for
//! the Tiny 2 Lite), plus the daemon's fields `available`, `pending` (leaf path → written
//! value) and `unread` (leaf paths). A leaf path joins the object keys with `.`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Action parameters and action results: name → value.
pub type Params = Map<String, Value>;

/// Leaf path → value, as a model reads one group.
pub type Values = BTreeMap<String, Value>;

/// The paths the daemon adds to every state; they are never settings.
pub const DAEMON_PATHS: [&str; 3] = ["available", "pending", "unread"];

/// The published state of one device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct State {
    pub available: bool,
    /// Leaf path → the value written; the camera accepted it but does not show it yet.
    #[serde(default)]
    pub pending: BTreeMap<String, Value>,
    /// Leaf paths never read since the device last became available, or whose last read failed.
    #[serde(default)]
    pub unread: Vec<String>,
    /// The state tree: group → … → leaf value.
    #[serde(flatten)]
    pub values: Map<String, Value>,
}

impl State {
    /// An empty state: no values, nothing pending, nothing unread.
    pub fn new(available: bool) -> State {
        State {
            available,
            pending: BTreeMap::new(),
            unread: Vec::new(),
            values: Map::new(),
        }
    }

    /// The value at a leaf or group path of the state tree. `None` when the path is absent.
    /// The daemon's fields are struct fields, not paths of the tree.
    pub fn at(&self, path: &str) -> Option<&Value> {
        let mut parts = path.split('.');
        let mut node = self.values.get(parts.next()?)?;
        for part in parts {
            node = node.as_object()?.get(part)?;
        }
        Some(node)
    }

    /// Sets the value at a leaf path, creating the groups on the way.
    pub fn set(&mut self, path: &str, value: Value) {
        let parts: Vec<&str> = path.split('.').collect();
        let (last, groups) = parts.split_last().expect("split always yields one part");
        let mut map = &mut self.values;
        for group in groups {
            let entry = map
                .entry(group.to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }
            map = entry.as_object_mut().expect("made an object above");
        }
        map.insert(last.to_string(), value);
    }

    /// A copy with the given leaves set: the state a `requires` check sees (4.2).
    pub fn with(&self, leaves: &[(String, Value)]) -> State {
        let mut next = self.clone();
        for (path, value) in leaves {
            next.set(path, value.clone());
        }
        next
    }

    /// The reply of a `get` for these paths (4.1 rule 9): the paths asked for, plus
    /// `available`, `pending` and `unread` in full. Absent paths are left out.
    pub fn select(&self, paths: &[String]) -> State {
        let mut out = State {
            available: self.available,
            pending: self.pending.clone(),
            unread: self.unread.clone(),
            values: Map::new(),
        };
        for path in paths {
            if DAEMON_PATHS.contains(&path.as_str()) {
                continue;
            }
            if let Some(value) = self.at(path) {
                out.set(path, value.clone());
            }
        }
        out
    }

    /// Whether the state shows the camera asleep (`asleep: true`).
    pub fn asleep(&self) -> bool {
        self.at("asleep") == Some(&Value::Bool(true))
    }
}

/// The payload of `set`: leaf paths, nested (`{"zoom":{"factor":2}}`) or dotted
/// (`{"zoom.factor":2}`). [`crate::validate::leaves`] turns it into leaves.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Patch(pub Map<String, Value>);

impl Patch {
    pub fn new() -> Patch {
        Patch(Map::new())
    }

    /// Adds one leaf under its dotted path.
    pub fn leaf(mut self, path: &str, value: impl Into<Value>) -> Patch {
        self.0.insert(path.to_string(), value.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> State {
        let mut s = State::new(true);
        s.set("zoom.factor", json!(1.5));
        s.set("zone.area.pan", json!({"min": -20, "max": 20}));
        s.set("asleep", json!(false));
        s.pending.insert("zoom.factor".into(), json!(2));
        s.unread.push("presets".into());
        s
    }

    #[test]
    fn paths_walk_the_nested_tree() {
        let s = sample();
        assert_eq!(s.at("zoom.factor"), Some(&json!(1.5)));
        assert_eq!(
            s.at("zone.area"),
            Some(&json!({"pan": {"min": -20, "max": 20}}))
        );
        assert_eq!(
            s.at("zone.area.pan.min"),
            Some(&json!(-20)),
            "a component is reachable too"
        );
        assert_eq!(
            s.at("gimbal.pan"),
            None,
            "absent paths are None, never null"
        );
        assert!(!s.asleep());
    }

    #[test]
    fn a_get_reply_keeps_the_daemon_fields_in_full() {
        let got = sample().select(&["zoom".to_string(), "nothing.here".to_string()]);
        assert_eq!(got.at("zoom.factor"), Some(&json!(1.5)));
        assert_eq!(got.at("zone.area.pan"), None, "only the paths asked for");
        assert_eq!(got.pending.get("zoom.factor"), Some(&json!(2)));
        assert_eq!(got.unread, vec!["presets".to_string()]);
    }

    #[test]
    fn with_applies_leaves_without_touching_the_original() {
        let s = sample();
        let next = s.with(&[("exposure.mode".into(), json!("manual"))]);
        assert_eq!(next.at("exposure.mode"), Some(&json!("manual")));
        assert_eq!(s.at("exposure.mode"), None);
    }

    #[test]
    fn a_patch_built_with_leaf_uses_dotted_keys() {
        let p = Patch::new()
            .leaf("led.brightness", 2)
            .leaf("zoom.factor", 1.5);
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            r#"{"led.brightness":2,"zoom.factor":1.5}"#
        );
    }
}
```

Create `crates/obsbot-proto/src/message.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The envelopes of protocol 2 (4.1): requests, replies, events and errors. One JSON object
//! per line; [`Request::from_line`] is the daemon's parser, `serde_json::to_string` gives the
//! canonical line of every type.

use crate::description::Device;
use crate::ids::RequestId;
use crate::state::{Params, Patch, State};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

fn yes() -> bool {
    true
}

/// The `hello` verb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Hello {
    pub protocol: u32,
    /// `"<name> <version>"` of the client.
    pub client: String,
    /// Events for this connection; `true` when left out on the wire.
    #[serde(default = "yes")]
    pub subscribe: bool,
    /// Live topics (4.1 rule 8); empty when left out.
    #[serde(default)]
    pub live: Vec<String>,
}

impl Hello {
    /// A `hello` for protocol 2 with `subscribe: false` and no live topics.
    pub fn new(client: &str) -> Hello {
        Hello {
            protocol: crate::PROTOCOL,
            client: client.to_string(),
            subscribe: false,
            live: Vec::new(),
        }
    }

    pub fn subscribe(mut self, on: bool) -> Hello {
        self.subscribe = on;
        self
    }

    pub fn live(mut self, topics: &[&str]) -> Hello {
        self.live = topics.iter().map(|t| t.to_string()).collect();
        self
    }
}

/// One request line. Exactly one verb.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub id: RequestId,
    pub verb: Verb,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Verb {
    Hello(Hello),
    /// `paths: None` asks for the full state (`"get": null`).
    Get {
        device: String,
        paths: Option<Vec<String>>,
    },
    Set {
        device: String,
        patch: Patch,
    },
    /// `"do"` with its parameters (`"with"`, `{}` when left out).
    Do {
        device: String,
        action: String,
        params: Params,
    },
    Live {
        topics: Vec<String>,
    },
}

impl Verb {
    /// The device id a device request names; `None` for `hello` and `live`.
    pub fn device(&self) -> Option<&str> {
        match self {
            Verb::Get { device, .. } | Verb::Set { device, .. } | Verb::Do { device, .. } => {
                Some(device)
            }
            Verb::Hello(_) | Verb::Live { .. } => None,
        }
    }
}

/// Why a request line was refused before it reached a device: always error code `invalid`
/// (the daemon answers `protocol` instead when this is the first line of a connection).
#[derive(Debug, Clone, PartialEq)]
pub struct RequestError {
    /// The id, when the line had a usable one; the reply carries `id: null` otherwise.
    pub id: Option<RequestId>,
    pub message: String,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RequestError {}

/// The wire form of a request: every verb optional, so that "no verb" and "two verbs" can be
/// told apart from malformed JSON.
#[derive(Serialize, Deserialize, JsonSchema)]
struct WireRequest {
    #[serde(default)]
    id: Option<RequestId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hello: Option<Hello>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "nullable")]
    #[schemars(with = "Option<Vec<String>>")]
    get: Option<Option<Vec<String>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    set: Option<Patch>,
    #[serde(default, rename = "do", skip_serializing_if = "Option::is_none")]
    act: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    with: Option<Params>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    live: Option<Vec<String>>,
}

/// `Some(None)` for an explicit `null`, `None` for a missing field.
mod nullable {
    use super::*;

    pub fn serialize<S: Serializer>(
        v: &Option<Option<Vec<String>>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match v {
            Some(inner) => inner.serialize(s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Option<Vec<String>>>, D::Error> {
        Option::<Vec<String>>::deserialize(d).map(Some)
    }
}

impl Request {
    /// Parses one request line. Errors carry the id when the line had a usable one.
    pub fn from_line(line: &str) -> Result<Request, RequestError> {
        let value: Value = serde_json::from_str(line).map_err(|e| RequestError {
            id: None,
            message: format!("not JSON: {e}"),
        })?;
        let id = value
            .get("id")
            .and_then(|v| serde_json::from_value::<RequestId>(v.clone()).ok());
        let wire: WireRequest = serde_json::from_value(value).map_err(|e| RequestError {
            id: id.clone(),
            message: format!("malformed request: {e}"),
        })?;
        Request::try_from(wire)
    }
}

impl TryFrom<WireRequest> for Request {
    type Error = RequestError;

    fn try_from(w: WireRequest) -> Result<Request, RequestError> {
        let fail = |id: Option<RequestId>, message: &str| RequestError {
            id,
            message: message.to_string(),
        };
        let verbs = [
            w.hello.is_some(),
            w.get.is_some(),
            w.set.is_some(),
            w.act.is_some(),
            w.live.is_some(),
        ];
        let id = w.id.clone();
        match verbs.iter().filter(|v| **v).count() {
            0 => return Err(fail(id, "the request has no verb")),
            1 => {}
            _ => return Err(fail(id, "the request has more than one verb")),
        }
        let Some(id) = w.id else {
            return Err(fail(None, "the request has no id"));
        };
        let device = |d: Option<String>| {
            d.ok_or_else(|| fail(Some(id.clone()), "the request names no device"))
        };
        let verb = if let Some(hello) = w.hello {
            Verb::Hello(hello)
        } else if let Some(paths) = w.get {
            Verb::Get {
                device: device(w.device)?,
                paths,
            }
        } else if let Some(patch) = w.set {
            Verb::Set {
                device: device(w.device)?,
                patch,
            }
        } else if let Some(action) = w.act {
            Verb::Do {
                device: device(w.device)?,
                action,
                params: w.with.unwrap_or_default(),
            }
        } else {
            Verb::Live {
                topics: w.live.unwrap_or_default(),
            }
        };
        Ok(Request { id, verb })
    }
}

impl From<&Request> for WireRequest {
    fn from(r: &Request) -> WireRequest {
        let mut w = WireRequest {
            id: Some(r.id.clone()),
            device: r.verb.device().map(str::to_string),
            hello: None,
            get: None,
            set: None,
            act: None,
            with: None,
            live: None,
        };
        match &r.verb {
            Verb::Hello(h) => w.hello = Some(h.clone()),
            Verb::Get { paths, .. } => w.get = Some(paths.clone()),
            Verb::Set { patch, .. } => w.set = Some(patch.clone()),
            Verb::Do { action, params, .. } => {
                w.act = Some(action.clone());
                w.with = (!params.is_empty()).then(|| params.clone());
            }
            Verb::Live { topics } => w.live = Some(topics.clone()),
        }
        w
    }
}

impl Serialize for Request {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        WireRequest::from(self).serialize(s)
    }
}

impl<'de> Deserialize<'de> for Request {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Request, D::Error> {
        let wire = WireRequest::deserialize(d)?;
        Request::try_from(wire).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for Request {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Request".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = WireRequest::json_schema(generator);
        schema.insert(
            "description".into(),
            "One request line: `id` and exactly one verb (`hello`, `get`, `set`, `do`, `live`). \
             `get`, `set` and `do` name a `device`; `with` holds the parameters of `do`."
                .into(),
        );
        schema
    }
}

/// A reply: `{"id":…,"ok":{…}}` or `{"id":…,"error":{…}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Reply {
    /// The request's id; `null` only for a `protocol` or `invalid` error without a usable id.
    pub id: Option<RequestId>,
    #[serde(flatten)]
    pub body: ReplyBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplyBody {
    Ok(OkBody),
    Error(ErrorBody),
}

/// The `ok` payload. Its shape follows the verb it answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[non_exhaustive]
pub enum OkBody {
    /// The reply to `hello`.
    Hello(HelloOk),
    /// The reply to `get`.
    State(RevState),
    /// The reply to `set` and `do`.
    Rev(RevOk),
    /// The reply to `live`.
    Empty(Empty),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HelloOk {
    pub protocol: u32,
    /// The daemon's version.
    pub daemon: String,
    pub devices: Vec<Device>,
    /// Device id → its rev and state.
    pub states: BTreeMap<String, RevState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RevState {
    pub rev: u64,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RevOk {
    pub rev: u64,
    /// An action's result, when its description declares one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Params>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Empty {}

/// The `error` payload (4.1, Errors).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    /// English, for logs.
    pub message: String,
    /// Present whenever the daemon re-read the device before answering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<u64>,
    /// `camera`: the leaf paths written before the failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied: Option<Vec<String>>,
    /// `camera`: the leaf path or action that failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// `protocol`: the protocol numbers the daemon speaks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported: Option<Vec<u32>>,
}

impl ErrorBody {
    pub fn new(code: ErrorCode, message: &str) -> ErrorBody {
        ErrorBody {
            code,
            message: message.to_string(),
            rev: None,
            applied: None,
            failed: None,
            supported: None,
        }
    }
}

/// The error codes the daemon sends (4.1). A code this crate does not know parses as
/// `Other`; clients treat it as `internal` (4.1 rule 7).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorCode {
    Protocol,
    Invalid,
    UnknownDevice,
    Unsupported,
    UnsupportedDevice,
    OutOfRange,
    Conflict,
    Unavailable,
    Asleep,
    Busy,
    Camera,
    Internal,
    #[serde(untagged)]
    Other(String),
}

impl ErrorCode {
    /// Every code the daemon may send, in the order of the table in 4.1.
    pub const ALL: [ErrorCode; 12] = [
        ErrorCode::Protocol,
        ErrorCode::Invalid,
        ErrorCode::UnknownDevice,
        ErrorCode::Unsupported,
        ErrorCode::UnsupportedDevice,
        ErrorCode::OutOfRange,
        ErrorCode::Conflict,
        ErrorCode::Unavailable,
        ErrorCode::Asleep,
        ErrorCode::Busy,
        ErrorCode::Camera,
        ErrorCode::Internal,
    ];

    /// The wire name (`"out_of_range"`).
    pub fn as_str(&self) -> &str {
        match self {
            ErrorCode::Protocol => "protocol",
            ErrorCode::Invalid => "invalid",
            ErrorCode::UnknownDevice => "unknown_device",
            ErrorCode::Unsupported => "unsupported",
            ErrorCode::UnsupportedDevice => "unsupported_device",
            ErrorCode::OutOfRange => "out_of_range",
            ErrorCode::Conflict => "conflict",
            ErrorCode::Unavailable => "unavailable",
            ErrorCode::Asleep => "asleep",
            ErrorCode::Busy => "busy",
            ErrorCode::Camera => "camera",
            ErrorCode::Internal => "internal",
            ErrorCode::Other(code) => code,
        }
    }
}

/// An event (only on connections with `subscribe: true`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// The full state of one device.
    State {
        device: String,
        rev: u64,
        state: State,
    },
    /// The full device list.
    Devices { devices: Vec<Device> },
    /// An event type this crate does not know; clients ignore it (4.1 rule 7).
    #[serde(other, skip_serializing)]
    #[schemars(skip)]
    Unknown,
}

/// One line from the daemon: an event (it has `event`) or a reply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ServerMessage {
    Event(Event),
    Reply(Reply),
}
```

Create the three stubs that later tasks fill. `crates/obsbot-proto/src/client.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The synchronous client (design spec 6.1). Written in track C.
```

`crates/obsbot-proto/src/validate.rs` holds `Refusal` from the start, because `trait Camera` (Task 5) names it; Task 2 keeps it unchanged:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The validator. Written in W0 task 2.

use crate::message::ErrorCode;

/// Why a request is refused. `path` names the leaf, parameter or action at fault.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub code: ErrorCode,
    pub path: Option<String>,
    pub message: String,
}

impl Refusal {
    pub fn new(code: ErrorCode, path: &str, message: &str) -> Refusal {
        Refusal {
            code,
            path: Some(path.to_string()),
            message: message.to_string(),
        }
    }
}
```

`crates/obsbot-proto/src/schema.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generation of `schema/protocol-2.json`. Written in W0 task 3.
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo build -p obsbot-proto` (updates `Cargo.lock` with schemars), then `cargo test -p obsbot-proto`
Expected: PASS: `test result: ok. 10 passed` (unit tests of `ids`, `values`, `state`) and `test result: ok. 4 passed` (`tests/types.rs`).

Run: `cargo fmt --all --check && cargo clippy -p obsbot-proto --all-targets --locked -- -D warnings`
Expected: no output, exit 0.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/obsbot-proto
git commit -m "obsbot-proto: protocol 2 types, description format and canonical JSON"
```

### Task 2: Validator

**Files:**

- Modify: `crates/obsbot-proto/src/validate.rs` (replace the stub)
- Test: `crates/obsbot-proto/tests/validate.rs`

**Interfaces:**

- Consumes (Task 1): `Description`, `Setting`, `SettingType`, `Bound`, `Asleep`, `Warning`, `State` (`at`, `with`, `asleep`), `Patch`, `Params`, `ErrorCode`, `values::{same, is_raw}`, `state::DAEMON_PATHS`.
- Produces (module `obsbot_proto::validate`):
  - `struct Refusal { pub code: ErrorCode, pub path: Option<String>, pub message: String }` with `fn new(ErrorCode, &str, &str) -> Refusal` (already in Task 1's stub; keep it as it is)
  - `enum Fit { Write, State }`
  - `fn leaves(&Description, &Patch) -> Result<Vec<(String, Value)>, Refusal>`
  - `fn validate_set(&Description, &State, &Patch) -> Result<Vec<(String, Value)>, Refusal>` (leaves in path order)
  - `fn validate_do(&Description, &State, action: &str, &Params) -> Result<(), Refusal>`
  - `fn validate_get(&Description, &[String]) -> Result<(), Refusal>`
  - `fn check_value(&Setting, &Value, Fit) -> Result<(), (ErrorCode, String)>`
  - `fn check_description(&Description) -> Vec<String>` (empty when consistent)

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot-proto/tests/validate.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Table tests of the validator. Each row names the rule of the spec it pins.

use obsbot_proto::validate::{
    Fit, check_description, check_value, validate_do, validate_get, validate_set,
};
use obsbot_proto::{
    Action, Asleep, Description, ErrorCode, Patch, Setting, SettingType, State, Unit, Warning,
};
use serde_json::{Value, json};

/// A small device: every setting type, a `requires`, an `asleep: ignored` leaf, a read-only
/// leaf, a destructive action and an action with an optional parameter.
fn desc() -> Description {
    let w = |s: Setting| s.asleep(Asleep::Unknown).settle_ms(3000);
    Description::default()
        .setting("asleep", Setting::boolean("Asleep").readonly())
        .setting(
            "led.brightness",
            w(Setting::number("LED brightness", 0.0, 3.0).step(1.0)),
        )
        .setting(
            "zoom.factor",
            Setting::number("Zoom", 1.0, 4.0)
                .precision(0.01)
                .asleep(Asleep::Ignored)
                .settle_ms(3000),
        )
        .setting(
            "zoom.field_of_view",
            w(
                Setting::enumeration("Field of view", &["wide", "medium", "custom"])
                    .writable_only(&["wide", "medium"]),
            ),
        )
        .setting(
            "exposure.mode",
            w(Setting::enumeration("Exposure mode", &["auto", "manual"])),
        )
        .setting(
            "exposure.time",
            w(Setting::number("Exposure time", 100.0, 250_000.0).unit(Unit::Microseconds))
                .requires("exposure.mode", json!("manual")),
        )
        .setting(
            "zone.area.pan",
            w(Setting::range("Zone pan", -150.0, 150.0).unit(Unit::Deg)),
        )
        .setting(
            "zone.return",
            Setting::composite(
                SettingType::Pose,
                "Zone return",
                &[("pan", -150.0), ("tilt", -70.0), ("zoom", 1.0)],
                &[("pan", 150.0), ("tilt", 90.0), ("zoom", 4.0)],
            )
            .readonly(),
        )
        .setting("gimbal.invert", w(Setting::boolean("Invert")))
        .setting(
            "presets",
            Setting::list(
                "Presets",
                &[
                    ("slot", Setting::number("Slot", 0.0, 2.0)),
                    ("name", Setting::string("Name", 32, true)),
                ],
            ),
        )
        .action(
            "preset.save",
            Action::new("Save a preset", Asleep::Ignored)
                .param("slot", Setting::number("Slot", 0.0, 2.0).step(1.0))
                .param("name", Setting::string("Name", 32, true).optional()),
        )
        .action(
            "factory_reset",
            Action::new("Factory reset", Asleep::Unknown)
                .param("confirm", Setting::boolean("Confirm"))
                .warning(Warning::Destructive)
                .warning(Warning::Restart),
        )
        .limit("presets", 3)
}

fn awake() -> State {
    let mut s = State::new(true);
    s.set("asleep", json!(false));
    s.set("exposure.mode", json!("auto"));
    s
}

fn patch(v: Value) -> Patch {
    Patch(v.as_object().expect("a patch is an object").clone())
}

fn code_of_set(state: &State, p: Value) -> Option<ErrorCode> {
    validate_set(&desc(), state, &patch(p))
        .err()
        .map(|r| r.code)
}

#[test]
fn the_test_description_is_consistent() {
    assert_eq!(check_description(&desc()), Vec::<String>::new());
}

#[test]
fn set_table() {
    use ErrorCode::*;
    let rows: Vec<(&str, Value, Option<ErrorCode>)> = vec![
        ("valid, dotted", json!({"led.brightness": 2}), None),
        ("valid, nested", json!({"led": {"brightness": 2}}), None),
        (
            "valid, float that is a whole number",
            json!({"led.brightness": 2.0}),
            None,
        ),
        (
            "valid range",
            json!({"zone.area.pan": {"min": -20, "max": 20}}),
            None,
        ),
        (
            "valid range, nested and dotted mixed",
            json!({"zone": {"area.pan": {"min": -20, "max": 20}}}),
            None,
        ),
        (
            "wrong type: string for a number",
            json!({"led.brightness": "2"}),
            Some(Invalid),
        ),
        (
            "wrong type: number for a bool",
            json!({"gimbal.invert": 1}),
            Some(Invalid),
        ),
        (
            "wrong type: null",
            json!({"gimbal.invert": null}),
            Some(Invalid),
        ),
        (
            "wrong type: range without max",
            json!({"zone.area.pan": {"min": -20}}),
            Some(Invalid),
        ),
        (
            "wrong type: range with an extra field",
            json!({"zone.area.pan": {"min": -20, "max": 20, "mid": 0}}),
            Some(Invalid),
        ),
        (
            "read-only leaf (4.2)",
            json!({"zone.return": {"pan": 0, "tilt": 0, "zoom": 1}}),
            Some(Invalid),
        ),
        ("read-only list", json!({"presets": []}), Some(Invalid)),
        (
            "a group is not a leaf (4.2 Leaves)",
            json!({"zoom": 2}),
            Some(Invalid),
        ),
        (
            "a leaf named twice",
            json!({"zoom": {"factor": 2}, "zoom.factor": 3}),
            Some(Invalid),
        ),
        ("empty patch", json!({}), Some(Invalid)),
        (
            "unknown path, a typo",
            json!({"led.brigthness": 2}),
            Some(Unsupported),
        ),
        (
            "a path of another model, not on this device",
            json!({"tracking.mode": "static"}),
            Some(Unsupported),
        ),
        ("above max", json!({"led.brightness": 4}), Some(OutOfRange)),
        ("below min", json!({"zoom.factor": 0.5}), Some(OutOfRange)),
        (
            "not on the step",
            json!({"led.brightness": 1.5}),
            Some(OutOfRange),
        ),
        (
            "enum value shown but not writable",
            json!({"zoom.field_of_view": "custom"}),
            Some(OutOfRange),
        ),
        (
            "enum value unknown",
            json!({"zoom.field_of_view": "narrow"}),
            Some(OutOfRange),
        ),
        (
            "range component out of bounds",
            json!({"zone.area.pan": {"min": -160, "max": 20}}),
            Some(OutOfRange),
        ),
        (
            "range min above max",
            json!({"zone.area.pan": {"min": 30, "max": 20}}),
            Some(OutOfRange),
        ),
        (
            "requires not met: time alone while auto (4.2)",
            json!({"exposure.time": 20000}),
            Some(Conflict),
        ),
        (
            "requires met by the same patch (4.2)",
            json!({"exposure": {"mode": "manual", "time": 20000}}),
            None,
        ),
        (
            "invalid wins over unsupported",
            json!({"led.brightness": "x", "nope": 1}),
            Some(Invalid),
        ),
        (
            "unsupported wins over out_of_range",
            json!({"led.brightness": 9, "nope": 1}),
            Some(Unsupported),
        ),
        (
            "out_of_range wins over conflict",
            json!({"led.brightness": 9, "exposure.time": 20000}),
            Some(OutOfRange),
        ),
        // Review Focus 1: float noise from a client's arithmetic is not a step violation.
        (
            "float noise on a stepped value",
            json!({"led.brightness": 2.000_000_000_1}),
            None,
        ),
        // Review Focus 2: the CLI's `min,max` syntax sent as a JSON array.
        (
            "a range as an array",
            json!({"zone.area.pan": [-20, 20]}),
            Some(Invalid),
        ),
        // Review Focus 3: malformed paths are refused, never a panic.
        (
            "an empty path segment",
            json!({"led..brightness": 2}),
            Some(Unsupported),
        ),
        ("a trailing dot", json!({"led.": 2}), Some(Unsupported)),
        ("an empty key", json!({"": 2}), Some(Unsupported)),
    ];
    for (name, p, want) in rows {
        assert_eq!(code_of_set(&awake(), p), want, "row: {name}");
    }
}

#[test]
fn asleep_ignored_is_refused_only_while_asleep() {
    let mut asleep = awake();
    asleep.set("asleep", json!(true));
    assert_eq!(
        code_of_set(&asleep, json!({"zoom.factor": 2})),
        Some(ErrorCode::Asleep),
        "ignored while asleep"
    );
    assert_eq!(
        code_of_set(&awake(), json!({"zoom.factor": 2})),
        None,
        "awake: written"
    );
    assert_eq!(
        code_of_set(&asleep, json!({"led.brightness": 2})),
        None,
        "asleep: unknown is written (5.7)"
    );
    assert_eq!(
        code_of_set(&asleep, json!({"zoom.factor": 2, "exposure.time": 300})),
        Some(ErrorCode::Conflict),
        "conflict wins over asleep"
    );
}

#[test]
fn a_refusal_names_the_path_at_fault() {
    let r = validate_set(
        &desc(),
        &awake(),
        &patch(json!({"led.brightness": 2, "zoom.factor": 9})),
    )
    .unwrap_err();
    assert_eq!(r.code, ErrorCode::OutOfRange);
    assert_eq!(r.path.as_deref(), Some("zoom.factor"));
    assert!(r.message.contains("zoom.factor"), "{}", r.message);
}

#[test]
fn a_valid_patch_comes_back_as_leaves_in_path_order() {
    let leaves = validate_set(
        &desc(),
        &awake(),
        &patch(json!({"zoom": {"factor": 2}, "led.brightness": 1})),
    )
    .unwrap();
    assert_eq!(
        leaves,
        vec![
            ("led.brightness".to_string(), json!(1)),
            ("zoom.factor".to_string(), json!(2))
        ]
    );
}

#[test]
fn do_table() {
    use ErrorCode::*;
    let rows: Vec<(&str, &str, Value, Option<ErrorCode>)> = vec![
        ("valid", "preset.save", json!({"slot": 1}), None),
        (
            "valid with the optional name",
            "preset.save",
            json!({"slot": 1, "name": "Desk"}),
            None,
        ),
        (
            "unknown action",
            "preset.load",
            json!({"slot": 1}),
            Some(Unsupported),
        ),
        ("missing parameter", "preset.save", json!({}), Some(Invalid)),
        (
            "unknown parameter",
            "preset.save",
            json!({"slot": 1, "speed": 3}),
            Some(Invalid),
        ),
        (
            "wrong type",
            "preset.save",
            json!({"slot": "one"}),
            Some(Invalid),
        ),
        (
            "out of range",
            "preset.save",
            json!({"slot": 3}),
            Some(OutOfRange),
        ),
        (
            "name too long",
            "preset.save",
            json!({"slot": 0, "name": "x".repeat(33)}),
            Some(OutOfRange),
        ),
        (
            "name not ASCII",
            "preset.save",
            json!({"slot": 0, "name": "Küche"}),
            Some(OutOfRange),
        ),
        (
            "destructive without confirm (4.2 warning)",
            "factory_reset",
            json!({}),
            Some(Invalid),
        ),
        (
            "destructive with confirm false",
            "factory_reset",
            json!({"confirm": false}),
            Some(Invalid),
        ),
        (
            "destructive with confirm true",
            "factory_reset",
            json!({"confirm": true}),
            None,
        ),
    ];
    for (name, action, params, want) in rows {
        let params = params.as_object().unwrap().clone();
        let got = validate_do(&desc(), &awake(), action, &params)
            .err()
            .map(|r| r.code);
        assert_eq!(got, want, "row: {name}");
    }
    let mut asleep = awake();
    asleep.set("asleep", json!(true));
    let params = json!({"slot": 1}).as_object().unwrap().clone();
    assert_eq!(
        validate_do(&desc(), &asleep, "preset.save", &params)
            .err()
            .map(|r| r.code),
        Some(ErrorCode::Asleep)
    );
}

#[test]
fn get_accepts_leaves_groups_and_daemon_fields_only() {
    let ok = |paths: &[&str]| {
        validate_get(
            &desc(),
            &paths.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
        )
    };
    assert!(
        ok(&[
            "zoom.factor",
            "zone",
            "zone.area",
            "pending",
            "available",
            "unread"
        ])
        .is_ok()
    );
    assert_eq!(ok(&["tracking"]).unwrap_err().code, ErrorCode::Unsupported);
    assert_eq!(
        ok(&["zoom.factorx"]).unwrap_err().code,
        ErrorCode::Unsupported
    );
    // Review Focus 5: a component of a leaf is not a path of its own.
    assert_eq!(
        ok(&["zone.area.pan.min"]).unwrap_err().code,
        ErrorCode::Unsupported
    );
}

#[test]
fn state_values_may_be_null_where_nullable_and_raw_where_undecodable() {
    let mode = Setting::enumeration("Mode", &["a", "b"]);
    assert!(
        check_value(&mode, &json!("raw:7"), Fit::State).is_ok(),
        "4.2 Undecodable values"
    );
    assert!(
        check_value(&mode, &json!("raw:7"), Fit::Write).is_err(),
        "raw values are never written"
    );
    assert!(
        check_value(&mode, &Value::Null, Fit::State).is_err(),
        "null only where nullable"
    );
    assert!(check_value(&mode.clone().nullable(), &Value::Null, Fit::State).is_ok());
}

#[test]
fn check_description_finds_broken_descriptions() {
    let broken = Description::default()
        .setting("Zoom.Factor", Setting::number("Zoom", 4.0, 1.0))
        .setting(
            "zone.area",
            Setting::boolean("Zone")
                .asleep(Asleep::Unknown)
                .settle_ms(3000),
        )
        .setting("zone.area.pan", Setting::range("Pan", -150.0, 150.0))
        .setting(
            "mode",
            Setting::enumeration("Mode", &["a"])
                .writable_only(&["b"])
                .asleep(Asleep::Unknown)
                .settle_ms(3000),
        )
        .action(
            "factory_reset",
            Action::new("Reset", Asleep::Unknown).warning(Warning::Destructive),
        )
        .limit("zone.area", 3);
    let problems = check_description(&broken).join("\n");
    for expected in [
        "Zoom.Factor: not lower-case",
        "Zoom.Factor: min is above max",
        "Zoom.Factor: a writable leaf needs asleep",
        "zone.area: a leaf that is also a group",
        "zone.area.pan: a writable leaf needs asleep",
        "mode: writable holds a value that values does not",
        "factory_reset: a destructive action needs the bool parameter confirm",
        "limits.zone.area: not a list setting",
    ] {
        assert!(
            problems.contains(expected),
            "missing {expected:?} in:\n{problems}"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot-proto --test validate`
Expected: FAIL to compile, `unresolved imports obsbot_proto::validate::Fit, …`.

- [ ] **Step 3: Write the validator**

Replace `crates/obsbot-proto/src/validate.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The validator (4.1 rule 5, 4.2, 5.6): checks a `set` patch, a `do` and a `get` against a
//! device description and its current state, before anything is sent to the camera. The
//! daemon is authoritative; clients may run the same checks.
//!
//! The whole request is checked. When several things are wrong, the refusal with the first
//! code in this order wins, and within one code the first path in path order:
//! `invalid`, `unsupported`, `out_of_range`, `conflict`, `asleep` (the order of 5.6).

use crate::description::{Asleep, Bound, Description, Setting, SettingType, Warning};
use crate::message::ErrorCode;
use crate::state::{DAEMON_PATHS, Params, Patch, State};
use crate::values::{is_raw, same};
use serde_json::Value;
use std::collections::BTreeSet;

/// Why a request is refused. `path` names the leaf, parameter or action at fault.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub code: ErrorCode,
    pub path: Option<String>,
    pub message: String,
}

impl Refusal {
    pub fn new(code: ErrorCode, path: &str, message: &str) -> Refusal {
        Refusal {
            code,
            path: Some(path.to_string()),
            message: message.to_string(),
        }
    }
}

/// What a value is checked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// A value a client writes: `writable[]`, bounds, step; never `null` or `raw:<n>`.
    Write,
    /// A value a state may show: `values[]`, bounds; `null` where `nullable`; `raw:<n>`.
    State,
}

fn rank(code: &ErrorCode) -> u8 {
    match code {
        ErrorCode::Invalid => 0,
        ErrorCode::Unsupported => 1,
        ErrorCode::OutOfRange => 2,
        ErrorCode::Conflict => 3,
        ErrorCode::Asleep => 4,
        _ => 5,
    }
}

/// Keeps the refusal that wins under the order above.
#[derive(Default)]
struct First(Option<Refusal>);

impl First {
    fn add(&mut self, code: ErrorCode, path: &str, message: String) {
        let better = match &self.0 {
            None => true,
            Some(old) => rank(&code) < rank(&old.code),
        };
        if better {
            let path = (!path.is_empty()).then(|| path.to_string());
            self.0 = Some(Refusal {
                code,
                path,
                message,
            });
        }
    }

    fn done(self) -> Result<(), Refusal> {
        self.0.map_or(Ok(()), Err)
    }
}

/// The leaves of a patch, nested or dotted, as (leaf path, value) in path order. Refuses an
/// empty patch, a group written as a value, a leaf named twice (`invalid`) and a path that is
/// not in the description (`unsupported`).
pub fn leaves(desc: &Description, patch: &Patch) -> Result<Vec<(String, Value)>, Refusal> {
    let (out, first) = split(desc, patch);
    first.done()?;
    Ok(out)
}

/// The leaves of a patch and the first structural problem, if any.
fn split(desc: &Description, patch: &Patch) -> (Vec<(String, Value)>, First) {
    let mut first = First::default();
    let mut out: Vec<(String, Value)> = Vec::new();
    if patch.is_empty() {
        first.add(ErrorCode::Invalid, "", "the patch is empty".into());
    }
    collect(desc, "", &patch.0, &mut out, &mut first);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    for pair in out.windows(2) {
        if pair[0].0 == pair[1].0 {
            first.add(
                ErrorCode::Invalid,
                &pair[0].0,
                format!("the patch names {} twice", pair[0].0),
            );
        }
    }
    out.dedup_by(|a, b| a.0 == b.0);
    (out, first)
}

fn collect(
    desc: &Description,
    prefix: &str,
    map: &serde_json::Map<String, Value>,
    out: &mut Vec<(String, Value)>,
    first: &mut First,
) {
    for (key, value) in map {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if desc.settings.contains_key(&path) {
            out.push((path, value.clone()));
        } else if desc.is_group(&path) {
            match value.as_object() {
                Some(inner) => collect(desc, &path, inner, out, first),
                None => first.add(
                    ErrorCode::Invalid,
                    &path,
                    format!("{path} is a group, not a leaf"),
                ),
            }
        } else {
            first.add(
                ErrorCode::Unsupported,
                &path,
                format!("{path} is not a setting of this device"),
            );
        }
    }
}

/// Checks a `set` against the description and the current state; returns its leaves in path
/// order. The model's apply order is `Camera::order`, not this order.
pub fn validate_set(
    desc: &Description,
    state: &State,
    patch: &Patch,
) -> Result<Vec<(String, Value)>, Refusal> {
    let (leaves, mut first) = split(desc, patch);
    for (path, value) in &leaves {
        let setting = &desc.settings[path];
        if setting.readonly {
            first.add(ErrorCode::Invalid, path, format!("{path} is read-only"));
        } else if let Err((code, message)) = check_value(setting, value, Fit::Write) {
            first.add(code, path, format!("{path}: {message}"));
        }
    }
    let next = state.with(&leaves);
    for (path, _) in &leaves {
        let setting = &desc.settings[path];
        for req in &setting.requires {
            if !next.at(&req.path).is_some_and(|v| same(v, &req.equals)) {
                first.add(
                    ErrorCode::Conflict,
                    path,
                    format!("{path} requires {} = {}", req.path, req.equals),
                );
            }
        }
        if state.asleep() && setting.asleep == Some(Asleep::Ignored) {
            first.add(
                ErrorCode::Asleep,
                path,
                format!("the camera is asleep and ignores {path}"),
            );
        }
    }
    first.done()?;
    Ok(leaves)
}

/// Checks a `do` against the description and the current state: the action exists, every
/// parameter is known and fits, no required parameter is missing, a destructive action has
/// `confirm: true`, its `requires` hold and the camera is not asleep for an `ignored` action.
pub fn validate_do(
    desc: &Description,
    state: &State,
    action: &str,
    params: &Params,
) -> Result<(), Refusal> {
    let Some(spec) = desc.actions.get(action) else {
        return Err(Refusal::new(
            ErrorCode::Unsupported,
            action,
            &format!("{action} is not an action of this device"),
        ));
    };
    let mut first = First::default();
    for (name, value) in params {
        match spec.params.get(name) {
            None => first.add(
                ErrorCode::Invalid,
                name,
                format!("{action} has no parameter {name}"),
            ),
            Some(setting) => {
                if let Err((code, message)) = check_value(setting, value, Fit::Write) {
                    first.add(code, name, format!("{action} {name}: {message}"));
                }
            }
        }
    }
    for (name, setting) in &spec.params {
        if !setting.optional && !params.contains_key(name) {
            first.add(
                ErrorCode::Invalid,
                name,
                format!("{action} needs the parameter {name}"),
            );
        }
    }
    if spec.warning.contains(&Warning::Destructive)
        && params.get("confirm") != Some(&Value::Bool(true))
    {
        first.add(
            ErrorCode::Invalid,
            "confirm",
            format!("{action} deletes data on the camera: send confirm: true"),
        );
    }
    for req in &spec.requires {
        if !state.at(&req.path).is_some_and(|v| same(v, &req.equals)) {
            first.add(
                ErrorCode::Conflict,
                action,
                format!("{action} requires {} = {}", req.path, req.equals),
            );
        }
    }
    if state.asleep() && spec.asleep == Asleep::Ignored {
        first.add(
            ErrorCode::Asleep,
            action,
            format!("the camera is asleep and ignores {action}"),
        );
    }
    first.done()
}

/// Checks the paths of a `get`: each is a daemon field, a leaf or a group of the description.
pub fn validate_get(desc: &Description, paths: &[String]) -> Result<(), Refusal> {
    let mut first = First::default();
    for path in paths {
        let known = DAEMON_PATHS.contains(&path.as_str())
            || desc.settings.contains_key(path)
            || desc.is_group(path);
        if !known {
            first.add(
                ErrorCode::Unsupported,
                path,
                format!("{path} is not a path of this device"),
            );
        }
    }
    first.done()
}

fn as_number(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite())
}

const EPS: f64 = 1e-9;

fn check_number(
    s: &Setting,
    x: f64,
    component: Option<&str>,
    fit: Fit,
) -> Result<(), (ErrorCode, String)> {
    let bound = |b: &Option<Bound>| b.as_ref().and_then(|b| b.get(component));
    let what = component.map_or(String::new(), |c| format!("{c} "));
    if let Some(min) = bound(&s.min)
        && x < min - EPS
    {
        return Err((
            ErrorCode::OutOfRange,
            format!("{what}{x} is below the minimum {min}"),
        ));
    }
    if let Some(max) = bound(&s.max)
        && x > max + EPS
    {
        return Err((
            ErrorCode::OutOfRange,
            format!("{what}{x} is above the maximum {max}"),
        ));
    }
    if fit == Fit::Write
        && let Some(step) = bound(&s.step).filter(|st| *st > 0.0)
    {
        let k = (x - bound(&s.min).unwrap_or(0.0)) / step;
        if (k - k.round()).abs() > 1e-6 {
            return Err((
                ErrorCode::OutOfRange,
                format!("{what}{x} is not a multiple of the step {step}"),
            ));
        }
    }
    Ok(())
}

/// Checks one value against one setting. The error carries the code and an English message
/// without the path.
pub fn check_value(s: &Setting, v: &Value, fit: Fit) -> Result<(), (ErrorCode, String)> {
    if v.is_null() {
        return if fit == Fit::State && s.nullable {
            Ok(())
        } else {
            Err((
                ErrorCode::Invalid,
                "null is not a value of this setting".into(),
            ))
        };
    }
    if fit == Fit::State && v.as_str().is_some_and(is_raw) {
        return Ok(());
    }
    let invalid = |m: &str| Err((ErrorCode::Invalid, m.to_string()));
    match &s.kind {
        SettingType::Bool => {
            if v.is_boolean() {
                Ok(())
            } else {
                invalid("expected true or false")
            }
        }
        SettingType::Enum => {
            let Some(text) = v.as_str() else {
                return invalid("expected one of the names of this setting");
            };
            let allowed = if fit == Fit::Write {
                &s.writable
            } else {
                &s.values
            };
            if allowed.iter().any(|a| a == text) {
                Ok(())
            } else {
                Err((
                    ErrorCode::OutOfRange,
                    format!("{text} is not one of {}", allowed.join("|")),
                ))
            }
        }
        SettingType::Number => match as_number(v) {
            Some(x) => check_number(s, x, None, fit),
            None => invalid("expected a number"),
        },
        SettingType::String => {
            let Some(text) = v.as_str() else {
                return invalid("expected a string");
            };
            if let Some(max) = s.max_len
                && text.chars().count() > max as usize
            {
                return Err((
                    ErrorCode::OutOfRange,
                    format!("longer than {max} characters"),
                ));
            }
            if s.ascii && !text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
                return Err((
                    ErrorCode::OutOfRange,
                    "only printable ASCII characters are allowed".into(),
                ));
            }
            Ok(())
        }
        SettingType::List => {
            if fit == Fit::Write {
                return invalid("a list is read-only");
            }
            let Some(items) = v.as_array() else {
                return invalid("expected a list");
            };
            let fields = s.item.clone().unwrap_or_default();
            for item in items {
                let Some(obj) = item.as_object() else {
                    return invalid("expected a list of objects");
                };
                for (name, field) in &fields {
                    let value = obj.get(name).unwrap_or(&Value::Null);
                    check_value(field, value, Fit::State)
                        .map_err(|(c, m)| (c, format!("item {name}: {m}")))?;
                }
            }
            Ok(())
        }
        SettingType::Other(kind) => {
            if fit == Fit::State {
                Ok(())
            } else {
                invalid(&format!("a setting of type {kind} cannot be written"))
            }
        }
        kind => {
            let names = kind
                .components()
                .expect("the remaining types are object-shaped");
            let Some(obj) = v.as_object() else {
                return invalid(&format!("expected an object with {}", names.join(", ")));
            };
            let keys: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
            if keys != names.iter().copied().collect() {
                return invalid(&format!("expected exactly the fields {}", names.join(", ")));
            }
            for name in names {
                let Some(x) = as_number(&obj[*name]) else {
                    return invalid(&format!("{name} must be a number"));
                };
                check_number(s, x, Some(name), fit)?;
            }
            if *kind == SettingType::Range && as_number(&obj["min"]) > as_number(&obj["max"]) {
                return Err((ErrorCode::OutOfRange, "min is above max".into()));
            }
            Ok(())
        }
    }
}

fn path_ok(path: &str) -> bool {
    !path.is_empty()
        && path.split('.').all(|seg| {
            let mut bytes = seg.bytes();
            bytes.next().is_some_and(|b| b.is_ascii_lowercase())
                && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}

fn check_setting(
    where_: &str,
    s: &Setting,
    desc: &Description,
    leaf: bool,
    problems: &mut Vec<String>,
) {
    let mut bad = |m: &str| problems.push(format!("{where_}: {m}"));
    if s.label.is_empty() {
        bad("has no label");
    }
    match &s.kind {
        SettingType::Enum => {
            if s.values.is_empty() {
                bad("an enum without values");
            }
            if s.writable.iter().any(|w| !s.values.contains(w)) {
                bad("writable holds a value that values does not");
            }
            if leaf && s.readonly && !s.writable.is_empty() {
                bad("a read-only enum with writable values");
            }
            if (!leaf || !s.readonly) && s.writable.is_empty() {
                bad("a writable enum without writable values");
            }
        }
        SettingType::Number => {
            for (name, b) in [
                ("min", &s.min),
                ("max", &s.max),
                ("step", &s.step),
                ("precision", &s.precision),
            ] {
                if matches!(b, Some(Bound::Each(_))) {
                    bad(&format!("{name} of a number must be one number"));
                }
            }
        }
        SettingType::List => {
            if !s.readonly || s.item.is_none() {
                bad("a list must be read-only and describe its item");
            }
        }
        kind => {
            if let Some(names) = kind.components() {
                for (name, b) in [("min", &s.min), ("max", &s.max)] {
                    if let Some(Bound::Each(map)) = b {
                        let keys: BTreeSet<&str> = map.keys().map(String::as_str).collect();
                        if keys != names.iter().copied().collect() {
                            bad(&format!(
                                "{name} must have exactly the fields {}",
                                names.join(", ")
                            ));
                        }
                    } else if b.is_some() {
                        bad(&format!(
                            "{name} of an object-shaped setting must be an object"
                        ));
                    }
                }
            }
        }
    }
    let components: Vec<Option<&str>> = match s.kind.components() {
        Some(names) => names.iter().map(|n| Some(*n)).collect(),
        None => vec![None],
    };
    for c in components {
        if let (Some(lo), Some(hi)) = (
            s.min.as_ref().and_then(|b| b.get(c)),
            s.max.as_ref().and_then(|b| b.get(c)),
        ) && lo > hi
        {
            bad("min is above max");
        }
    }
    if let Some(default) = &s.default
        && check_value(s, default, Fit::State).is_err()
    {
        bad("the default does not fit the setting");
    }
    for req in &s.requires {
        match desc.settings.get(&req.path) {
            None => bad(&format!("requires the unknown path {}", req.path)),
            Some(target) => {
                if check_value(target, &req.equals, Fit::State).is_err() {
                    bad(&format!("requires a value that {} cannot have", req.path));
                }
            }
        }
    }
    if let Some(topic) = &s.live
        && !path_ok(topic)
    {
        bad("the live topic is not a lower-case name");
    }
    if leaf {
        if s.optional {
            bad("optional is for action parameters only");
        }
        if s.readonly && (s.asleep.is_some() || s.settle_ms.is_some()) {
            bad("a read-only leaf has neither asleep nor settle_ms");
        }
        if !s.readonly && s.asleep.is_none() {
            bad("a writable leaf needs asleep");
        }
        if !s.readonly && s.readback && s.settle_ms.is_none() {
            bad("a writable leaf with a readback needs settle_ms");
        }
        if !s.readback && s.settle_ms.is_some() {
            bad("a leaf without readback has no settle_ms");
        }
    }
}

/// Checks that a description is consistent in itself; returns the problems, empty when there
/// are none. Every model and fake must pass (trait conformance, 10.3 W0).
pub fn check_description(desc: &Description) -> Vec<String> {
    let mut problems = Vec::new();
    for (path, setting) in &desc.settings {
        if !path_ok(path) {
            problems.push(format!(
                "{path}: not lower-case snake_case segments joined by '.'"
            ));
        }
        if DAEMON_PATHS.contains(&path.as_str()) {
            problems.push(format!("{path}: reserved for the daemon"));
        }
        if desc.is_group(path) {
            problems.push(format!("{path}: a leaf that is also a group"));
        }
        check_setting(path, setting, desc, true, &mut problems);
        for (field, item) in setting.item.iter().flatten() {
            check_setting(
                &format!("{path}[].{field}"),
                item,
                desc,
                false,
                &mut problems,
            );
        }
    }
    for (name, action) in &desc.actions {
        if !path_ok(name) {
            problems.push(format!(
                "{name}: not lower-case snake_case segments joined by '.'"
            ));
        }
        if action.label.is_empty() {
            problems.push(format!("{name}: has no label"));
        }
        for (param, setting) in action.params.iter().chain(action.result.iter().flatten()) {
            if !path_ok(param) || param.contains('.') {
                problems.push(format!(
                    "{name}.{param}: a parameter name is one lower-case segment"
                ));
            }
            check_setting(
                &format!("{name}.{param}"),
                setting,
                desc,
                false,
                &mut problems,
            );
        }
        if action.warning.contains(&Warning::Destructive)
            && action
                .params
                .get("confirm")
                .is_none_or(|c| c.kind != SettingType::Bool)
        {
            problems.push(format!(
                "{name}: a destructive action needs the bool parameter confirm"
            ));
        }
        for req in &action.requires {
            if !desc.settings.contains_key(&req.path) {
                problems.push(format!("{name}: requires the unknown path {}", req.path));
            }
        }
    }
    for list in desc.limits.keys() {
        if desc
            .settings
            .get(list)
            .is_none_or(|s| s.kind != SettingType::List)
        {
            problems.push(format!("limits.{list}: not a list setting"));
        }
    }
    problems
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-proto --test validate`
Expected: PASS: `test result: ok. 9 passed`.

Run: `cargo fmt --all --check && cargo clippy -p obsbot-proto --all-targets --locked -- -D warnings`
Expected: no output, exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-proto/src/validate.rs crates/obsbot-proto/tests/validate.rs
git commit -m "obsbot-proto: validator for set, do and get, and the description self-check"
```

### Task 3: Schema and its stale test

**Files:**

- Modify: `crates/obsbot-proto/src/schema.rs` (replace the stub)
- Create: `schema/protocol-2.json` (generated)
- Test: `crates/obsbot-proto/tests/schema.rs`

**Interfaces:**

- Consumes (Task 1): `Request`, `ServerMessage`, `Description`, `ErrorCode::ALL`, `ErrorCode::as_str`.
- Produces: `obsbot_proto::schema::protocol_schema() -> serde_json::Value`, `obsbot_proto::schema::schema_text() -> String`; the file `schema/protocol-2.json`; the environment variable `OBSBOT_REGENERATE` (set: the stale test writes the file instead of comparing) that `scripts/regenerate.sh` (Task 10) uses.

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot-proto/tests/schema.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Stale test of `schema/protocol-2.json`. `scripts/regenerate.sh` rewrites the file
//! (`OBSBOT_REGENERATE=1`); without it, any difference fails.

use obsbot_proto::schema::{protocol_schema, schema_text};
use std::path::PathBuf;

fn schema_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schema/protocol-2.json")
}

#[test]
fn schema_file_is_current() {
    let path = schema_file();
    let want = schema_text();
    if std::env::var_os("OBSBOT_REGENERATE").is_some() {
        std::fs::write(&path, &want).expect("write schema/protocol-2.json");
        return;
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        have == want,
        "schema/protocol-2.json is stale: run scripts/regenerate.sh and commit the result"
    );
}

#[test]
fn schema_names_every_envelope_and_the_description_format() {
    let schema = protocol_schema();
    let defs = schema["$defs"].as_object().expect("$defs");
    for name in [
        "Request",
        "Reply",
        "Event",
        "ErrorBody",
        "ErrorCode",
        "Device",
        "Description",
        "Setting",
        "Action",
        "State",
    ] {
        assert!(defs.contains_key(name), "the schema lacks {name}");
    }
    let codes = serde_json::to_string(&defs["ErrorCode"]).unwrap();
    for code in obsbot_proto::ErrorCode::ALL {
        assert!(
            codes.contains(&format!("\"{}\"", code.as_str())),
            "ErrorCode lacks {}",
            code.as_str()
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot-proto --test schema`
Expected: FAIL to compile, `unresolved imports obsbot_proto::schema::protocol_schema, …`.

- [ ] **Step 3: Write the schema generator**

Replace `crates/obsbot-proto/src/schema.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The JSON schema of protocol 2, `schema/protocol-2.json` (design spec, sections 3 and 4).
//! It covers every line on the socket (requests, replies, events) and, through `Device`, the
//! description format. It is generated from the types of this crate and checked for staleness
//! by `tests/schema.rs`.

use crate::description::Description;
use crate::message::{Request, ServerMessage};
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};

/// The schema as a JSON value.
pub fn protocol_schema() -> Value {
    let mut generator = SchemaSettings::draft2020_12().into_generator();
    let request = generator.subschema_for::<Request>();
    let server = generator.subschema_for::<ServerMessage>();
    generator.subschema_for::<Description>();
    let defs = generator.take_definitions(true);
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "obsbot4linux protocol 2",
        "description": "One JSON object per line on the socket: a Request from a client, or a ServerMessage (a Reply or an Event) from the daemon. Description is the content of descriptions/<model>.json.",
        "oneOf": [request, server],
        "$defs": defs,
    })
}

/// The file content: pretty JSON with a final newline.
pub fn schema_text() -> String {
    let mut text =
        serde_json::to_string_pretty(&protocol_schema()).expect("a JSON value always serialises");
    text.push('\n');
    text
}
```

- [ ] **Step 4: See the stale test fail, then generate the file**

Run: `cargo test -p obsbot-proto --test schema`
Expected: FAIL in `schema_file_is_current` with `schema/protocol-2.json is stale: run scripts/regenerate.sh and commit the result` (the file does not exist yet; this is the check working).

Run: `mkdir -p schema && OBSBOT_REGENERATE=1 cargo test -p obsbot-proto --test schema`
Expected: PASS, and `schema/protocol-2.json` exists (about 1000 lines; `$defs` holds `Request`, `Reply`, `Event`, `ErrorBody`, `ErrorCode`, `Device`, `Description`, `Setting`, `Action`, `State` among others).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot-proto --test schema`
Expected: PASS: `test result: ok. 2 passed`.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot-proto/src/schema.rs crates/obsbot-proto/tests/schema.rs schema/protocol-2.json
git commit -m "obsbot-proto: generate schema/protocol-2.json with a stale test"
```

### Task 4: Golden transcripts

**Files:**

- Create: `crates/obsbot-proto/tests/golden/session.jsonl`, `errors.jsonl`, `hotplug.jsonl`
- Test: `crates/obsbot-proto/tests/transcripts.rs`

**Interfaces:**

- Consumes (Task 1): `Request::from_line`, `ServerMessage`, `Reply`, `ReplyBody`, `OkBody`, `Event`, `ErrorCode::ALL`.
- Produces: the transcripts tracks C and D build against (spec 10.3 Track D "Depends on: W0 (golden transcripts)"). Format: `> ` a line from a client, `< ` a line from the daemon, `#` a comment. The example device is `sn:EXAMPLE0000001` of the illustrative model `example`, USB product `fa01`; the unsupported device is `port:3-2.4`, product `fa02`. A golden file names no product id but the Tiny 2 Lite's own `fef9` and synthetic ids `fa0x` (spec 8.5 public text; a real model's id from the vendor app would publish vendor-app knowledge).

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot-proto/tests/transcripts.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Golden protocol transcripts (`tests/golden/*.jsonl` except `types.jsonl`): `> ` lines go
//! from a client to the daemon, `< ` lines from the daemon to a client, `#` lines are
//! comments. Every line round-trips byte for byte, and the ordering rules of 4.1 that a
//! transcript can show hold in each file. Tracks C and D build against these files.

use obsbot_proto::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn transcripts() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl") && !p.ends_with("types.jsonl"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

/// The daemon's lines of one transcript, parsed, after checking every line's round trip.
fn parse(name: &str, text: &str) -> Vec<ServerMessage> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = format!("{name}:{}", n + 1);
        if let Some(json) = line.strip_prefix("> ") {
            let req = Request::from_line(json).unwrap_or_else(|e| panic!("{at}: {e}"));
            assert_eq!(
                serde_json::to_string(&req).unwrap(),
                json,
                "{at}: not byte-equal"
            );
        } else if let Some(json) = line.strip_prefix("< ") {
            let msg: ServerMessage =
                serde_json::from_str(json).unwrap_or_else(|e| panic!("{at}: {e}"));
            assert_eq!(
                serde_json::to_string(&msg).unwrap(),
                json,
                "{at}: not byte-equal"
            );
            out.push(msg);
        } else {
            panic!("{at}: a line starts with '> ', '< ' or '#'");
        }
    }
    out
}

#[test]
fn the_three_transcripts_exist() {
    let names: Vec<String> = transcripts().into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["errors.jsonl", "hotplug.jsonl", "session.jsonl"]);
}

#[test]
fn every_transcript_line_round_trips_byte_equal() {
    for (name, text) in transcripts() {
        parse(&name, &text);
    }
}

#[test]
fn the_error_transcript_holds_every_error_code() {
    let (_, text) = transcripts()
        .into_iter()
        .find(|(n, _)| n == "errors.jsonl")
        .unwrap();
    let codes: Vec<ErrorCode> = parse("errors.jsonl", &text)
        .into_iter()
        .map(|m| match m {
            ServerMessage::Reply(Reply {
                body: ReplyBody::Error(e),
                ..
            }) => e.code,
            other => panic!("errors.jsonl holds only error replies, not {other:?}"),
        })
        .collect();
    assert_eq!(codes, ErrorCode::ALL.to_vec());
}

/// 4.1 rule 1 and Events: a rev only goes up per entry; a state event names a listed device;
/// a client forgets an id's rev when a devices event no longer lists it.
#[test]
fn revs_and_device_lists_follow_the_ordering_rules() {
    for (name, text) in transcripts() {
        let mut listed: Option<BTreeSet<String>> = None;
        let mut revs: BTreeMap<String, u64> = BTreeMap::new();
        for msg in parse(&name, &text) {
            match msg {
                ServerMessage::Reply(Reply {
                    body: ReplyBody::Ok(OkBody::Hello(h)),
                    ..
                }) => {
                    listed = Some(h.devices.iter().map(|d| d.id.clone()).collect());
                    revs = h.states.iter().map(|(id, s)| (id.clone(), s.rev)).collect();
                }
                ServerMessage::Event(Event::Devices { devices }) => {
                    let ids: BTreeSet<String> = devices.iter().map(|d| d.id.clone()).collect();
                    revs.retain(|id, _| ids.contains(id));
                    listed = Some(ids);
                }
                ServerMessage::Event(Event::State { device, rev, .. }) => {
                    if let Some(ids) = &listed {
                        assert!(
                            ids.contains(&device),
                            "{name}: state of {device}, which no list holds"
                        );
                    }
                    if let Some(old) = revs.get(&device) {
                        assert!(
                            rev > *old,
                            "{name}: rev of {device} went from {old} to {rev}"
                        );
                    }
                    revs.insert(device, rev);
                }
                _ => {}
            }
        }
    }
}

/// 5.5: the hand-over is devices (old entry with replaced_by), devices (without it), state.
#[test]
fn the_hand_over_is_three_events_in_order() {
    let (_, text) = transcripts()
        .into_iter()
        .find(|(n, _)| n == "hotplug.jsonl")
        .unwrap();
    let msgs = parse("hotplug.jsonl", &text);
    let at = msgs
        .iter()
        .position(|m| matches!(m, ServerMessage::Event(Event::Devices { devices }) if devices.iter().any(|d| d.replaced_by.is_some())))
        .expect("a devices event with replaced_by");
    let ServerMessage::Event(Event::Devices { devices: first }) = &msgs[at] else {
        unreachable!()
    };
    let old = first.iter().find(|d| d.replaced_by.is_some()).unwrap();
    assert!(!old.available);
    let new_id = old.replaced_by.clone().unwrap();
    assert!(first.iter().any(|d| d.id == new_id && d.available));
    let ServerMessage::Event(Event::Devices { devices: second }) = &msgs[at + 1] else {
        panic!("devices event expected")
    };
    assert!(second.iter().all(|d| d.id != old.id));
    assert!(
        matches!(&msgs[at + 2], ServerMessage::Event(Event::State { device, .. }) if *device == new_id)
    );
}

/// Public text (spec 8.5): a golden file may name the Tiny 2 Lite's own USB id (`fef9`) and
/// otherwise only synthetic product ids of the fakes' range `fa0x`, never another real model's
/// id. Covers `types.jsonl` too, as `"product":"<id>"` and in a name as `3564:<id>`.
#[test]
fn golden_files_name_no_other_real_product_id() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let allowed = |id: &str| id == "fef9" || (id.len() == 4 && id.starts_with("fa0"));
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        for marker in ["\"product\":\"", "3564:"] {
            for rest in text.split(marker).skip(1) {
                let id: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
                assert!(
                    allowed(&id),
                    "{}: product id {id} is neither fef9 nor fa0x",
                    path.display()
                );
                seen += 1;
            }
        }
    }
    assert!(seen > 0, "the golden files name product ids");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot-proto --test transcripts`
Expected: FAIL: `the_three_transcripts_exist` (left `[]`) and the tests that look up `errors.jsonl` and `hotplug.jsonl` panic on `unwrap()` of `None`. `golden_files_name_no_other_real_product_id` passes already on Task 1's `types.jsonl`; it guards the three files written next.

- [ ] **Step 3: Write the transcripts**

Create `crates/obsbot-proto/tests/golden/session.jsonl`:

```text
# A widget connects, writes one leaf, reads it back, recalls a preset and changes its live list.
# Example device sn:EXAMPLE0000001 (model "example") and one unsupported device port:3-2.4.
> {"id":1,"hello":{"protocol":2,"client":"widget 1.0","subscribe":true,"live":["pose"]}}
< {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[{"id":"sn:EXAMPLE0000001","name":"Example Camera 0001","model":"example","support":"tested","serial":"EXAMPLE0000001","firmware":"1.2.3.4","video_node":"/dev/video2","available":true,"usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}},{"id":"port:3-2.4","name":"USB 3564:fa02 3-2.4","model":"unknown","support":"unsupported","serial":null,"firmware":null,"video_node":"/dev/video6","available":true,"usb":{"vendor":"3564","product":"fa02","product_name":null,"bcd_device":"0100","path":"3-2.4","serial_present":false,"interfaces":["0e","0e"],"extension_units":[]},"settings":{},"actions":{},"limits":{}}],"states":{"port:3-2.4":{"rev":1,"state":{"available":true,"pending":{},"unread":[]}},"sn:EXAMPLE0000001":{"rev":4,"state":{"available":true,"pending":{},"unread":[],"asleep":false,"gimbal":{"pan":12.5},"led":{"brightness":1},"presets":[{"name":"Desk","slot":0}]}}}}}
# The write is accepted; the touched re-read does not show it yet, so it is pending (4.1 rules 3 and 4).
> {"id":2,"device":"sn:EXAMPLE0000001","set":{"led.brightness":2}}
< {"event":"state","device":"sn:EXAMPLE0000001","rev":5,"state":{"available":true,"pending":{"led.brightness":2},"unread":[],"asleep":false,"gimbal":{"pan":12.5},"led":{"brightness":1},"presets":[{"name":"Desk","slot":0}]}}
< {"id":2,"ok":{"rev":5}}
# The next read of the group shows the value: it leaves pending.
< {"event":"state","device":"sn:EXAMPLE0000001","rev":6,"state":{"available":true,"pending":{},"unread":[],"asleep":false,"gimbal":{"pan":12.5},"led":{"brightness":2},"presets":[{"name":"Desk","slot":0}]}}
> {"id":3,"device":"sn:EXAMPLE0000001","get":["led"]}
< {"id":3,"ok":{"rev":6,"state":{"available":true,"pending":{},"unread":[],"led":{"brightness":2}}}}
# A motion action replies on acceptance; the pose then moves through later events (4.1 rule 6).
> {"id":4,"device":"sn:EXAMPLE0000001","do":"preset.recall","with":{"slot":0}}
< {"id":4,"ok":{"rev":6}}
< {"event":"state","device":"sn:EXAMPLE0000001","rev":7,"state":{"available":true,"pending":{},"unread":[],"asleep":false,"gimbal":{"pan":-20},"led":{"brightness":2},"presets":[{"name":"Desk","slot":0}]}}
> {"id":5,"live":["pose","target"]}
< {"id":5,"ok":{}}
# Requests for the unsupported device are answered at once (4.1 rule 2, 5.4).
> {"id":6,"device":"port:3-2.4","get":null}
< {"id":6,"ok":{"rev":1,"state":{"available":true,"pending":{},"unread":[]}}}
> {"id":7,"device":"port:3-2.4","set":{"led.brightness":2}}
< {"id":7,"error":{"code":"unsupported_device","message":"port:3-2.4 is not supported by obsbot4linux"}}
```

Create `crates/obsbot-proto/tests/golden/errors.jsonl`:

```text
# One reply per error code the daemon sends, in the order of the table in 4.1.
< {"id":null,"error":{"code":"protocol","message":"the first message must be hello","supported":[2]}}
< {"id":2,"error":{"code":"invalid","message":"led.brightness: expected a number"}}
< {"id":3,"error":{"code":"unknown_device","message":"no device sn:EXAMPLE0000009"}}
< {"id":4,"error":{"code":"unsupported","message":"tracking.mode is not a setting of this device"}}
< {"id":5,"error":{"code":"unsupported_device","message":"port:3-2.4 is not supported by obsbot4linux"}}
< {"id":6,"error":{"code":"out_of_range","message":"led.brightness: 4 is above the maximum 3"}}
< {"id":7,"error":{"code":"conflict","message":"preset slot 2 is empty"}}
< {"id":8,"error":{"code":"unavailable","message":"sn:EXAMPLE0000001 is unplugged"}}
< {"id":9,"error":{"code":"asleep","message":"the camera is asleep and ignores zoom.field_of_view"}}
< {"id":10,"error":{"code":"busy","message":"32 requests are queued for sn:EXAMPLE0000001"}}
< {"id":11,"error":{"code":"camera","message":"the camera did not answer","rev":9,"applied":["exposure.mode"],"failed":"exposure.time"}}
< {"id":12,"error":{"code":"internal","message":"a frame outside the send table was refused"}}
```

Create `crates/obsbot-proto/tests/golden/hotplug.jsonl`:

```text
# Hotplug and the identity hand-over, as a subscribed client sees them (4.1 Events, 5.3 to 5.5).
# 1. An unsupported device is plugged in: devices, then its state.
< {"event":"devices","devices":[{"id":"sn:EXAMPLE0000001","name":"Example Camera 0001","model":"example","support":"tested","serial":"EXAMPLE0000001","firmware":"1.2.3.4","video_node":"/dev/video2","available":true,"usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}},{"id":"port:3-2.4","name":"USB 3564:fa02 3-2.4","model":"unknown","support":"unsupported","serial":null,"firmware":null,"video_node":"/dev/video6","available":true,"usb":{"vendor":"3564","product":"fa02","product_name":null,"bcd_device":"0100","path":"3-2.4","serial_present":false,"interfaces":["0e","0e"],"extension_units":[]},"settings":{},"actions":{},"limits":{}}]}
< {"event":"state","device":"port:3-2.4","rev":1,"state":{"available":true,"pending":{},"unread":[]}}
# 2. The sn: device is unplugged: it stays listed, then one state with available:false and its last values.
< {"event":"devices","devices":[{"id":"sn:EXAMPLE0000001","name":"Example Camera 0001","model":"example","support":"tested","serial":"EXAMPLE0000001","firmware":"1.2.3.4","video_node":null,"available":false,"usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}},{"id":"port:3-2.4","name":"USB 3564:fa02 3-2.4","model":"unknown","support":"unsupported","serial":null,"firmware":null,"video_node":"/dev/video6","available":true,"usb":{"vendor":"3564","product":"fa02","product_name":null,"bcd_device":"0100","path":"3-2.4","serial_present":false,"interfaces":["0e","0e"],"extension_units":[]},"settings":{},"actions":{},"limits":{}}]}
< {"event":"state","device":"sn:EXAMPLE0000001","rev":8,"state":{"available":false,"pending":{},"unread":[],"asleep":false,"gimbal":{"pan":-20},"led":{"brightness":2},"presets":[{"name":"Desk","slot":0}]}}
# 3. The unsupported port: entry is unplugged: it is removed, and no state event follows.
< {"event":"devices","devices":[{"id":"sn:EXAMPLE0000001","name":"Example Camera 0001","model":"example","support":"tested","serial":"EXAMPLE0000001","firmware":"1.2.3.4","video_node":null,"available":false,"usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}}]}
# 4. The camera comes back; its serial is not answered at once, so it enters as port:3-2.1.
< {"event":"devices","devices":[{"id":"sn:EXAMPLE0000001","name":"Example Camera 0001","model":"example","support":"tested","serial":"EXAMPLE0000001","firmware":"1.2.3.4","video_node":null,"available":false,"usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}},{"id":"port:3-2.1","name":"Example Camera 3-2.1","model":"example","support":"tested","serial":null,"firmware":null,"video_node":"/dev/video2","available":true,"usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}}]}
< {"event":"state","device":"port:3-2.1","rev":1,"state":{"available":true,"pending":{},"unread":[],"asleep":false,"gimbal":{"pan":-20},"led":{"brightness":2},"presets":[{"name":"Desk","slot":0}]}}
# 5. A later identity attempt answers the serial: the hand-over, three events in a row (5.5).
< {"event":"devices","devices":[{"id":"sn:EXAMPLE0000001","name":"Example Camera 0001","model":"example","support":"tested","serial":"EXAMPLE0000001","firmware":"1.2.3.4","video_node":"/dev/video2","available":true,"usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}},{"id":"port:3-2.1","name":"Example Camera 3-2.1","model":"example","support":"tested","serial":null,"firmware":null,"video_node":"/dev/video2","available":false,"replaced_by":"sn:EXAMPLE0000001","usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}}]}
< {"event":"devices","devices":[{"id":"sn:EXAMPLE0000001","name":"Example Camera 0001","model":"example","support":"tested","serial":"EXAMPLE0000001","firmware":"1.2.3.4","video_node":"/dev/video2","available":true,"usb":{"vendor":"3564","product":"fa01","product_name":"Example Camera","bcd_device":"0100","path":"3-2.1","serial_present":false,"interfaces":["0e","0e"],"extension_units":[{"unit":2,"guid":"00000000-0000-0000-0000-000000000001","controls":12}]},"settings":{"asleep":{"type":"bool","label":"Asleep","readonly":true,"nullable":false,"readback":true,"no_effect":false},"gimbal.pan":{"type":"number","label":"Pan","readonly":true,"nullable":false,"readback":true,"no_effect":false,"unit":"deg","precision":0.1,"live":"pose"},"led.brightness":{"type":"number","label":"LED brightness","readonly":false,"nullable":false,"readback":true,"asleep":"unknown","no_effect":false,"min":0,"max":3,"step":1,"settle_ms":3000},"presets":{"type":"list","label":"Presets","readonly":true,"nullable":false,"readback":true,"no_effect":false,"item":{"name":{"type":"string","label":"Name","readonly":false,"nullable":false,"readback":true,"no_effect":false,"max_len":32,"ascii":true},"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}}}},"actions":{"preset.recall":{"label":"Recall a preset","params":{"slot":{"type":"number","label":"Slot","readonly":false,"nullable":false,"readback":true,"no_effect":false,"min":0,"max":2,"step":1}},"asleep":"unknown"}},"limits":{"presets":3}}]}
< {"event":"state","device":"sn:EXAMPLE0000001","rev":9,"state":{"available":true,"pending":{},"unread":[],"asleep":false,"gimbal":{"pan":-20},"led":{"brightness":2},"presets":[{"name":"Desk","slot":0}]}}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-proto --test transcripts`
Expected: PASS: `test result: ok. 6 passed`. A line that is not in canonical form fails with `<file>:<line>: not byte-equal` and shows both forms; fix the file, not the types.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-proto/tests/transcripts.rs crates/obsbot-proto/tests/golden/session.jsonl crates/obsbot-proto/tests/golden/errors.jsonl crates/obsbot-proto/tests/golden/hotplug.jsonl
git commit -m "obsbot-proto: golden transcripts for hello, events, errors, hotplug and hand-over"
```

### Task 5: Device contract types

**Files:**

- Modify: `crates/obsbot-device/Cargo.toml`, `crates/obsbot-device/src/lib.rs`
- Create: `crates/obsbot-device/src/contract/mod.rs`, `camera.rs`, `transport.rs`, `bus.rs`
- Create (stub filled by Task 7): `crates/obsbot-device/src/contract/conformance.rs`
- Test: `crates/obsbot-device/tests/contract.rs`

**Interfaces:**

- Consumes (Task 1): `Description`, `Params`, `State`, `Values`, `Support`, `Usb`, and `obsbot_proto::validate::Refusal` (defined in Task 1's stub of `validate.rs`).

- Produces (module `obsbot_device::contract`, re-exported names):
  - `enum CameraError { NoAnswer(String), Rejected(String), Gone, Locked, Cut, NotInSendTable(String), Io(String) }`, `type Result<T> = std::result::Result<T, CameraError>` (in `contract::camera`)
  - `enum Tier { Second, FiveSeconds, Minute, Touched }` with `period() -> Option<Duration>`
  - `struct LiveTier { topic: String, tier: Tier }`, `struct Group { name: String, tier: Tier, asleep: bool, live: Option<LiveTier>, leaves: Vec<String> }`
  - `struct Need { serial: bool, firmware: bool }`, `struct Identity { serial: Option<String>, firmware: Option<String> }`
  - `enum Target<'a> { Leaf(&'a str), Action(&'a str) }`
  - `struct Ctx` with `new(Option<Instant>, Option<Arc<AtomicBool>>)`, `none()`, `until(Instant)`, `interruptible(Arc<AtomicBool>)`, `check() -> Result<()>`
  - `enum Motion { None, Move, Continuing { stop: &'static str }, Stop }`
  - `enum Frame { Framed { selector: u8, receiver: u8, command: u16 }, Short { selector: u8, command: u8 }, Uvc { control: u32 } }`, `struct SendEntry { frame: Frame, evidence: &'static str, identity: bool }`, `struct SendTable { entries: Vec<SendEntry> }` with `empty()`, `allows(&Frame)`
  - `trait Camera: Send` with exactly these methods:

```rust
fn description(&self) -> &Description;
fn identity(&mut self, need: Need, ctx: &Ctx) -> Result<Identity>;
fn poll_plan(&self) -> &[Group];
fn read(&mut self, group: &str, ctx: &Ctx) -> Result<Values>;
fn order(&self, leaves: &[&str]) -> Vec<String>;
fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<()>;
fn check(&self, action: &str, params: &Params, state: &State) -> std::result::Result<(), Refusal>;
fn act(&mut self, action: &str, params: &Params, state: &State, ctx: &Ctx) -> Result<Option<Params>>;
fn touched(&self, target: Target<'_>) -> Vec<String>;
fn motion(&self, action: &str) -> Motion;
fn send_table(&self) -> &SendTable;
```

- `struct ControlRange { min, max, step, default: i32 }`, `struct OnAir { active: bool, apps: Vec<String> }`, `trait Transport: Send { xu_len, xu_get, xu_set, ctrl_get, ctrl_set, ctrl_range, usb, usb_serial, on_air }` (signatures in the code below)
- `struct UsbId { vendor: u16, product: u16 }` with `from_usb(&Usb) -> Option<UsbId>`; `enum IdentitySource { UsbSerial, Query, Absent }`; `struct ModelEntry { key, display_name, usb: &'static [UsbId], support: Support, identity: IdentitySource, firmware_query: bool, identity_tier: Tier, xu_guid: Option<&'static str>, selectors: &'static [u8] }`; `struct Registry { vendors: &'static [u16], models: &'static [ModelEntry] }` with `lookup(UsbId) -> Lookup` and `shared_pairs() -> Vec<UsbId>`; `enum Lookup { NotListed, Unsupported, Model(&'static ModelEntry) }`; `struct Found { usb: Usb, devnum: u32, video_node: Option<String>, model: Option<&'static ModelEntry> }`; `trait Bus: Send + Sync { fn scan(&self) -> Vec<Found>; fn open(&self, found: &Found) -> Result<Box<dyn Camera>>; }`

- [ ] **Step 1: Add the dependency and the module**

In `crates/obsbot-device/Cargo.toml`, `[dependencies]`, add (keep G's entries):

```toml
obsbot-proto.workspace = true
serde_json.workspace = true
```

(`serde_json` may be there already from G.) In `crates/obsbot-device/src/lib.rs`, add next to G's module lines:

```rust
pub mod contract;
```

- [ ] **Step 2: Write the failing test**

Create `crates/obsbot-device/tests/contract.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The small rules inside the contract types: registry lookup (8.1), the operation context
//! (4.1 rule 9, 5.6), tiers (5.7) and the send table (8.3).

use obsbot_device::contract::{
    Ctx, Frame, IdentitySource, Lookup, ModelEntry, Registry, SendEntry, SendTable, Tier, UsbId,
};
use obsbot_proto::{Support, Usb};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const A: ModelEntry = ModelEntry {
    key: "model-a",
    display_name: "Model A",
    usb: &[
        UsbId {
            vendor: 0x3564,
            product: 0x0001,
        },
        UsbId {
            vendor: 0x3564,
            product: 0x0002,
        },
    ],
    support: Support::Tested,
    identity: IdentitySource::UsbSerial,
    firmware_query: true,
    identity_tier: Tier::FiveSeconds,
    xu_guid: Some("00000000-0000-0000-0000-000000000001"),
    selectors: &[2, 6],
};
const B: ModelEntry = ModelEntry {
    key: "model-b",
    display_name: "Model B",
    usb: &[UsbId {
        vendor: 0x3564,
        product: 0x0002,
    }],
    support: Support::Untested,
    identity: IdentitySource::Absent,
    firmware_query: false,
    identity_tier: Tier::FiveSeconds,
    xu_guid: None,
    selectors: &[],
};
static ONE: Registry = Registry {
    vendors: &[0x3564],
    models: &[A],
};
static CLASH: Registry = Registry {
    vendors: &[0x3564],
    models: &[A, B],
};

#[test]
fn lookup_uses_the_vendor_table_and_the_usb_pairs_only() {
    assert!(
        matches!(
            ONE.lookup(UsbId {
                vendor: 0x046d,
                product: 0x0001
            }),
            Lookup::NotListed
        ),
        "unknown vendor: not listed (5.3)"
    );
    assert!(
        matches!(
            ONE.lookup(UsbId {
                vendor: 0x3564,
                product: 0x0009
            }),
            Lookup::Unsupported
        ),
        "known vendor, unknown product (5.4)"
    );
    assert!(
        matches!(ONE.lookup(UsbId { vendor: 0x3564, product: 0x0002 }), Lookup::Model(m) if m.key == "model-a"),
        "a model may have several pairs"
    );
}

#[test]
fn a_pair_of_two_models_is_found() {
    assert!(ONE.shared_pairs().is_empty());
    assert_eq!(
        CLASH.shared_pairs(),
        [UsbId {
            vendor: 0x3564,
            product: 0x0002
        }]
    );
}

#[test]
fn usb_ids_come_from_the_hex_strings_of_sysfs() {
    let usb = Usb {
        vendor: "3564".into(),
        product: "fef9".into(),
        product_name: None,
        bcd_device: "0100".into(),
        path: "3-2.1".into(),
        serial_present: false,
        interfaces: vec![],
        extension_units: vec![],
    };
    assert_eq!(
        UsbId::from_usb(&usb),
        Some(UsbId {
            vendor: 0x3564,
            product: 0xfef9
        })
    );
    assert_eq!(
        UsbId::from_usb(&Usb {
            vendor: "zz".into(),
            ..usb
        }),
        None
    );
}

#[test]
fn a_context_cuts_at_its_deadline_or_when_interrupted() {
    assert!(Ctx::none().check().is_ok());
    assert!(
        Ctx::until(Instant::now() + Duration::from_secs(60))
            .check()
            .is_ok()
    );
    assert!(
        Ctx::until(Instant::now() - Duration::from_millis(1))
            .check()
            .is_err(),
        "deadline passed"
    );
    let flag = Arc::new(AtomicBool::new(false));
    let ctx = Ctx::interruptible(flag.clone());
    assert!(ctx.check().is_ok());
    flag.store(true, Ordering::SeqCst);
    assert!(ctx.check().is_err(), "a stop arrived");

    // A request's touched re-read: under the request's deadline and cut by a stop (5.6).
    let flag = Arc::new(AtomicBool::new(false));
    let both = Ctx::new(
        Some(Instant::now() + Duration::from_secs(60)),
        Some(flag.clone()),
    );
    assert!(both.check().is_ok());
    flag.store(true, Ordering::SeqCst);
    assert!(
        both.check().is_err(),
        "the stop cuts it before the deadline"
    );
    let late = Ctx::new(
        Some(Instant::now() - Duration::from_millis(1)),
        Some(Arc::new(AtomicBool::new(false))),
    );
    assert!(late.check().is_err(), "the deadline cuts it without a stop");
}

#[test]
fn tiers_and_send_tables() {
    assert_eq!(Tier::Second.period(), Some(Duration::from_secs(1)));
    assert_eq!(Tier::FiveSeconds.period(), Some(Duration::from_secs(5)));
    assert_eq!(Tier::Minute.period(), Some(Duration::from_secs(60)));
    assert_eq!(Tier::Touched.period(), None, "never periodic");
    let table = SendTable {
        entries: vec![SendEntry {
            frame: Frame::Short {
                selector: 6,
                command: 0x1a,
            },
            evidence: "capture 07",
            identity: false,
        }],
    };
    assert!(table.allows(&Frame::Short {
        selector: 6,
        command: 0x1a
    }));
    assert!(!table.allows(&Frame::Short {
        selector: 6,
        command: 0x1b
    }));
    assert!(!SendTable::empty().allows(&Frame::Uvc {
        control: 0x0098_0900
    }));
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p obsbot-device --test contract`
Expected: FAIL to compile, `file not found for module contract` (and unresolved imports).

- [ ] **Step 4: Write the contract**

Create `crates/obsbot-device/src/contract/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The contract of the device crate (design spec 8.2, 10.3 W0): `trait Camera`,
//! `trait Transport`, `trait Bus` and the registry types, plus the trait conformance check.
//! These files change only by a contract-change PR (10.1).

pub mod bus;
pub mod camera;
pub mod conformance;
pub mod transport;

pub use bus::{Bus, Found, IdentitySource, Lookup, ModelEntry, Registry, UsbId};
pub use camera::{
    Camera, CameraError, Ctx, Frame, Group, Identity, LiveTier, Motion, Need, SendEntry, SendTable,
    Target, Tier,
};
pub use transport::{ControlRange, OnAir, Transport};
```

Create `crates/obsbot-device/src/contract/camera.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `trait Camera` (design spec 8.2) and its supporting types: groups and tiers, touched
//! targets, identity, the per-operation context and the send table. The daemon knows only
//! these: no command id, no offset, no selector of any model (section 5).

use obsbot_proto::validate::Refusal;
use obsbot_proto::{Description, Params, State, Values};
use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Why a camera operation failed. The daemon maps every variant to the error code `camera`,
/// except `NotInSendTable`, which is `internal` (8.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CameraError {
    /// A query was not answered within `ANSWER_POLL` (5.8).
    NoAnswer(String),
    /// The camera answered with a refusal.
    Rejected(String),
    /// The device node is gone (unplug): reported at once, without waiting for a timeout.
    Gone,
    /// The per-operation lock on the video node was not free within `NODE_LOCK_WAIT`.
    Locked,
    /// The context's deadline passed or a stop interrupted a group read (`Ctx::check`).
    Cut,
    /// The transport wrapper refused a frame outside the model's send table (8.3).
    NotInSendTable(String),
    /// Any other operating-system error.
    Io(String),
}

impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CameraError::NoAnswer(what) => write!(f, "the camera did not answer {what}"),
            CameraError::Rejected(what) => write!(f, "the camera refused {what}"),
            CameraError::Gone => f.write_str("the camera is gone"),
            CameraError::Locked => f.write_str("the video node stayed locked by another process"),
            CameraError::Cut => f.write_str("cut short by the request deadline or a stop"),
            CameraError::NotInSendTable(what) => {
                write!(f, "refused a frame outside the send table: {what}")
            }
            CameraError::Io(what) => write!(f, "I/O error: {what}"),
        }
    }
}

impl std::error::Error for CameraError {}

pub type Result<T> = std::result::Result<T, CameraError>;

/// How often a group is read (5.7). Intervals count from the end of the previous read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    Second,
    FiveSeconds,
    Minute,
    /// Read at plug-in and after a request that touches it, never periodically.
    Touched,
}

impl Tier {
    /// The period of a periodic tier; `None` for `Touched`.
    pub fn period(self) -> Option<Duration> {
        match self {
            Tier::Second => Some(Duration::from_secs(1)),
            Tier::FiveSeconds => Some(Duration::from_secs(5)),
            Tier::Minute => Some(Duration::from_secs(60)),
            Tier::Touched => None,
        }
    }
}

/// A faster tier a group uses while at least one connection subscribes the topic (5.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveTier {
    pub topic: String,
    pub tier: Tier,
}

/// One group of the poll plan: the unit of reading (5.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub tier: Tier,
    /// Whether the group can be read while the camera is asleep.
    pub asleep: bool,
    pub live: Option<LiveTier>,
    /// The leaf paths one read of this group returns. Every described leaf with
    /// `readback: true` is in exactly one group; a `readback: false` leaf is in none.
    pub leaves: Vec<String>,
}

/// What an identity attempt asks for (5.5): only what is still missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Need {
    pub serial: bool,
    pub firmware: bool,
}

/// What an identity attempt got. `serial` is the raw answer: the daemon checks it with
/// `obsbot_proto::is_valid_serial`. Never logged (4.2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    pub serial: Option<String>,
    pub firmware: Option<String>,
}

/// What a re-read follows: a written leaf or a finished action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'a> {
    Leaf(&'a str),
    Action(&'a str),
}

/// The bounds of one camera operation. A model calls [`Ctx::check`] before every frame it
/// sends: a request stops at its deadline (4.1 rule 9), a group read stops when a stop action
/// arrives (5.6 "Stops"). Both end as `CameraError::Cut`.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    deadline: Option<Instant>,
    interrupt: Option<Arc<AtomicBool>>,
}

impl Ctx {
    /// Any combination: a request's touched re-read has the request's deadline and is also a
    /// group read that a stop cuts.
    pub fn new(deadline: Option<Instant>, interrupt: Option<Arc<AtomicBool>>) -> Ctx {
        Ctx {
            deadline,
            interrupt,
        }
    }

    /// No deadline, no interrupt (tests, identity attempts).
    pub fn none() -> Ctx {
        Ctx::default()
    }

    /// A request: stops once `deadline` has passed.
    pub fn until(deadline: Instant) -> Ctx {
        Ctx {
            deadline: Some(deadline),
            interrupt: None,
        }
    }

    /// A poll: stops once `flag` is set.
    pub fn interruptible(flag: Arc<AtomicBool>) -> Ctx {
        Ctx {
            deadline: None,
            interrupt: Some(flag),
        }
    }

    /// `Err(Cut)` when the deadline has passed or the interrupt flag is set.
    pub fn check(&self) -> Result<()> {
        let late = self.deadline.is_some_and(|d| Instant::now() >= d);
        let stopped = self
            .interrupt
            .as_ref()
            .is_some_and(|f| f.load(Ordering::SeqCst));
        if late || stopped {
            Err(CameraError::Cut)
        } else {
            Ok(())
        }
    }
}

/// What an action is to the worker's queue and its continuing-motion record (4.1 rules 2
/// and 6, 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// Not a motion action.
    None,
    /// A motion action that ends by itself (`preset.recall`, `gimbal.move`). It clears the
    /// continuing-motion record of its device.
    Move,
    /// A motion action that goes on until `stop` (`gimbal.turn` → `gimbal.stop`). It replaces
    /// the record of its device with the connection that sent it.
    Continuing { stop: &'static str },
    /// A stop: taken before every queued request (4.1 rule 2). The stop a record names ends
    /// that motion, so it clears the record; another stop leaves it.
    Stop,
}

/// One frame a model may send (8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// A framed command or query on an extension-unit selector.
    Framed {
        selector: u8,
        receiver: u8,
        command: u16,
    },
    /// A short command on an extension-unit selector.
    Short { selector: u8, command: u8 },
    /// A write of a standard UVC control (V4L2 control id).
    Uvc { control: u32 },
}

/// One entry of a send table, with its evidence (`"capture 11"`, `"measured 2026-10-31"`,
/// or `"0.13.2 only, U-9"` until camera session CS-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendEntry {
    pub frame: Frame,
    pub evidence: &'static str,
    /// An identity query: the only frames allowed on the identity receiver (8.3).
    pub identity: bool,
}

/// Every frame a model may send. The transport wrapper refuses anything else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendTable {
    pub entries: Vec<SendEntry>,
}

impl SendTable {
    /// The table of a model that sends nothing (the fakes).
    pub fn empty() -> SendTable {
        SendTable::default()
    }

    pub fn allows(&self, frame: &Frame) -> bool {
        self.entries.iter().any(|e| e.frame == *frame)
    }
}

/// One camera model behind the daemon (8.2). The daemon calls it from one worker thread per
/// device; nothing else touches the device. Signatures change only by a contract-change PR.
pub trait Camera: Send {
    /// Settings (with `settle_ms`), actions and limits.
    fn description(&self) -> &Description;

    /// One identity attempt (5.5): sends only the queries `need` asks for. A model whose
    /// identity source is the USB serial string reads it from sysfs and sends no request.
    /// The daemon builds `need` from the registry entry (`ModelEntry::identity` and
    /// `firmware_query`) and calls this only when `need` asks for something.
    fn identity(&mut self, need: Need, ctx: &Ctx) -> Result<Identity>;

    /// The groups, their tiers, whether they are readable asleep, their live topics.
    fn poll_plan(&self) -> &[Group];

    /// Reads one group: every leaf of the group → its value (`null` where nullable and not
    /// set, `raw:<n>` where undecodable). An error means the group's leaves go to `unread`.
    /// A group whose read depends on a condition (on the Tiny 2 Lite the target, read only
    /// while the tracking mode is not `static`, 8.8) is decided by the model from its own
    /// last reads: while the condition does not hold, it returns its last values, or `null`
    /// for a nullable leaf, without sending a frame.
    fn read(&mut self, group: &str, ctx: &Ctx) -> Result<Values>;

    /// The safe apply order of the leaves of one patch: a permutation of `leaves` in which
    /// every `requires` path comes before the leaf that requires it.
    fn order(&self, leaves: &[&str]) -> Vec<String>;

    /// Writes one leaf; the value has passed the validator.
    fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<()>;

    /// The model's own checks of an action against the published state, before any frame is
    /// sent: an empty preset slot, an unknown view id, a list in `unread` (`conflict`), an
    /// action whose parameters miss a combination the description cannot express (`invalid`).
    /// The daemon calls it only after `obsbot_proto::validate::validate_do` passed, so its
    /// codes come after every code of the validator.
    fn check(
        &self,
        action: &str,
        params: &Params,
        state: &State,
    ) -> std::result::Result<(), Refusal>;

    /// Runs one action; its parameters have passed the validator and [`Camera::check`].
    /// `state` is the published state that `check` saw (a used or empty preset slot, the
    /// mirror and invert flags). Returns the result the description declares
    /// (`view.save` → `{"id": …}`), else `None`.
    fn act(
        &mut self,
        action: &str,
        params: &Params,
        state: &State,
        ctx: &Ctx,
    ) -> Result<Option<Params>>;

    /// The groups to re-read after a leaf was written or an action ran. For a leaf with a
    /// readback this includes the leaf's own group.
    fn touched(&self, target: Target<'_>) -> Vec<String>;

    /// What `action` is to the queue and to the continuing-motion record (4.1 rules 2 and 6,
    /// 5.6). `Motion::None` for every name that is not a motion action or a stop. The daemon
    /// may ask once per action of the description when it opens the device.
    fn motion(&self, action: &str) -> Motion;

    /// Every frame this model may send (8.3).
    fn send_table(&self) -> &SendTable;
}
```

Create `crates/obsbot-device/src/contract/transport.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `trait Transport` (design spec 3.1): extension-unit get, set and length by selector, UVC
//! control ioctls, and the facts read from sysfs. One implementation talks to USB UVC devices
//! (track A); a replay implementation answers from recorded exchanges (track A). A model owns
//! one transport; the daemon never sees it.

use crate::contract::camera::Result;
use obsbot_proto::Usb;

/// Range and default of a UVC control, as the camera reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlRange {
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub default: i32,
}

/// One sample of the on-air probe (4.3 `on_air`): taken together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OnAir {
    pub active: bool,
    /// Names of processes that hold the video node open, best effort.
    pub apps: Vec<String>,
}

pub trait Transport: Send {
    /// GET_LEN of an extension-unit selector.
    fn xu_len(&mut self, selector: u8) -> Result<u16>;
    /// GET_CUR of an extension-unit selector into `buf` (its length is the selector's length).
    fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()>;
    /// SET_CUR of an extension-unit selector. The wrapper checks the frame first (8.3).
    fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()>;
    /// VIDIOC_G_CTRL.
    fn ctrl_get(&mut self, id: u32) -> Result<i32>;
    /// VIDIOC_S_CTRL. The wrapper checks the control against the send table first.
    fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()>;
    /// VIDIOC_QUERYCTRL; `None` when the camera does not have the control.
    fn ctrl_range(&mut self, id: u32) -> Result<Option<ControlRange>>;
    /// The USB facts, read from sysfs now (the audio interface of 4.3 `audio.enabled` comes
    /// and goes with the camera's restarts).
    fn usb(&mut self) -> Result<Usb>;
    /// The USB serial string from sysfs, for a model whose identity source it is. Never logged.
    fn usb_serial(&mut self) -> Result<Option<String>>;
    /// One sample of the on-air probe on the video node with index 0.
    fn on_air(&mut self) -> Result<OnAir>;
}
```

Create `crates/obsbot-device/src/contract/bus.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! How the daemon finds devices: the registry types (8.1) and `trait Bus`, the enumeration
//! seam. The real bus (track A) scans sysfs once a second and opens the USB transport; the
//! fake bus (`models::fake::FakeBus`) holds fake devices in memory (5.12). The daemon's
//! registry thread calls `scan` and starts a worker with `open` for every supported device.

use crate::contract::camera::{Camera, Result, Tier};
use obsbot_proto::{Support, Usb};

/// A USB (vendor id, product id) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UsbId {
    pub vendor: u16,
    pub product: u16,
}

impl UsbId {
    /// From the four-hex-digit strings of `Usb` (`"3564"`, `"fef9"`).
    pub fn from_usb(usb: &Usb) -> Option<UsbId> {
        let vendor = u16::from_str_radix(&usb.vendor, 16).ok()?;
        let product = u16::from_str_radix(&usb.product, 16).ok()?;
        Some(UsbId { vendor, product })
    }
}

/// Where a model's serial comes from (5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// The USB serial string, read from sysfs once at plug-in; no request is sent for it.
    UsbSerial,
    /// The model's identity query, sent while no valid serial has been answered.
    Query,
    /// None: the device keeps its `port:` id.
    Absent,
}

/// The registry entry of one model (8.2): data only. The bus that opens a model's devices
/// knows its constructor.
#[derive(Debug)]
pub struct ModelEntry {
    /// `tiny2lite`, `fake-a`, …
    pub key: &'static str,
    /// The display name of 4.2 `name` (`OBSBOT Tiny 2 Lite`).
    pub display_name: &'static str,
    /// Every USB pair this model answers to.
    pub usb: &'static [UsbId],
    pub support: Support,
    /// Where the serial comes from (5.5).
    pub identity: IdentitySource,
    /// Whether the model has a firmware query. Without one, `firmware` is never asked for
    /// and stays "not reported"; with neither query nor `Query` source no attempt is sent.
    pub firmware_query: bool,
    /// The tier whose next two reads carry identity attempts 2 and 3 (5.5).
    pub identity_tier: Tier,
    /// The GUID of the model's extension unit, found in the USB descriptor (8.3); `None` for a
    /// model without one.
    pub xu_guid: Option<&'static str>,
    /// The extension-unit selectors the model uses; every other selector is refused (8.3).
    pub selectors: &'static [u8],
}

/// The vendor table and the registered models (8.1).
#[derive(Debug)]
pub struct Registry {
    pub vendors: &'static [u16],
    pub models: &'static [ModelEntry],
}

/// What the registry says about one USB device.
#[derive(Debug, Clone, Copy)]
pub enum Lookup {
    /// The vendor id is not in the vendor table: the device is not listed at all.
    NotListed,
    /// A known vendor id with an unregistered product id: an unsupported entry (5.4).
    Unsupported,
    Model(&'static ModelEntry),
}

impl Registry {
    /// Matches by (vendor id, product id) only; the product string is never used (5.3).
    pub fn lookup(&self, id: UsbId) -> Lookup {
        if !self.vendors.contains(&id.vendor) {
            return Lookup::NotListed;
        }
        match self.models.iter().find(|m| m.usb.contains(&id)) {
            Some(model) => Lookup::Model(model),
            None => Lookup::Unsupported,
        }
    }

    /// Pairs that belong to more than one model; the registry test requires none (8.1).
    pub fn shared_pairs(&self) -> Vec<UsbId> {
        let mut all: Vec<UsbId> = self
            .models
            .iter()
            .flat_map(|m| m.usb.iter().copied())
            .collect();
        all.sort();
        let mut shared: Vec<UsbId> = all
            .windows(2)
            .filter(|w| w[0] == w[1])
            .map(|w| w[0])
            .collect();
        shared.dedup();
        shared
    }
}

/// One USB device with a video node and a vendor id from the vendor table, as one scan saw it.
#[derive(Debug, Clone)]
pub struct Found {
    pub usb: Usb,
    /// The device number on its bus: with `usb.path` it identifies one enumeration (5.3).
    pub devnum: u32,
    /// `/dev/videoN` of the node with index 0.
    pub video_node: Option<String>,
    /// The registered model, or `None` for an unsupported device.
    pub model: Option<&'static ModelEntry>,
}

/// The enumeration seam of the daemon.
pub trait Bus: Send + Sync {
    /// One enumeration pass. Sends nothing to any device and opens no video node.
    fn scan(&self) -> Vec<Found>;

    /// Opens the camera of a supported device (`found.model` is `Some`). Sends nothing.
    fn open(&self, found: &Found) -> Result<Box<dyn Camera>>;
}
```

Create the stub `crates/obsbot-device/src/contract/conformance.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The trait conformance check. Written in W0 task 7.
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test contract`
Expected: PASS: `test result: ok. 5 passed`.

Run: `cargo test -p obsbot-device` (G's carried tests still pass), then `cargo fmt --all --check && cargo clippy -p obsbot-device --all-targets --locked -- -D warnings`
Expected: every test result ok; no clippy output.

- [ ] **Step 6: Commit**

```bash
git add Cargo.lock crates/obsbot-device/Cargo.toml crates/obsbot-device/src/lib.rs crates/obsbot-device/src/contract crates/obsbot-device/tests/contract.rs
git commit -m "obsbot-device: trait Camera, trait Transport, trait Bus and the registry types"
```

### Task 6: Remove the 0.13 command line

**Files:**

- Delete: `crates/obsbot4linux/src/cli.rs`, `src/client.rs`, `src/daemon.rs`, `src/proto.rs`, `crates/obsbot-device/src/legacy/full_status.rs`, and tests of the 0.13 CLI under `crates/obsbot4linux/tests/` (G's `--help` comparison among them)
- Modify: `crates/obsbot4linux/Cargo.toml`, `crates/obsbot4linux/src/main.rs`, `crates/obsbot-device/src/legacy/mod.rs` (one line), `.github/workflows/ci.yml`
- Keep: `crates/obsbot-device/src/legacy/state.rs` and `legacy/current_pose.rs` (decision 15)
- Create: `scripts/check-cli-deps.sh`

**Interfaces:**

- Consumes (Task 1): the crate `obsbot-proto`.
- Produces: a CLI crate whose only `[dependencies]` entry is `obsbot-proto`, and the CI check `scripts/check-cli-deps.sh` (spec 3). Track C writes the CLI in this crate; Task 11 adds the `[dev-dependencies]` entry `obsbot4linuxd`.

- [ ] **Step 1: Delete the 0.13 command line**

```bash
git rm crates/obsbot4linux/src/cli.rs crates/obsbot4linux/src/client.rs crates/obsbot4linux/src/daemon.rs crates/obsbot4linux/src/proto.rs
git rm crates/obsbot-device/src/legacy/full_status.rs
```

In `crates/obsbot-device/src/legacy/mod.rs`, remove the line `pub mod full_status;` (and a `pub use` of its items, if G wrote one). The 0.13 full status (`FullStatus`, `full_status_with`, `CAPABILITIES`, `ZoneInfo`) had no user but the 0.13 daemon and CLI; `git grep -n "full_status\|FullStatus\|ZoneInfo\|CAPABILITIES" crates/` must find nothing afterwards. `legacy/state.rs` and `legacy/current_pose.rs` stay (decision 15). Remove the 0.13 CLI's tests (`git rm` the files under `crates/obsbot4linux/tests/` that run the 0.13 binary).

- [ ] **Step 2: Write the stub**

Replace `crates/obsbot4linux/src/main.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linux`: the command line (design spec 6.2), a pure client of protocol 2. Track C
//! writes it; the 0.13 command line stays on `main` for 0.14.x.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("obsbot4linux: the 1.0 command line is not built yet (stage W0 of the 1.0 rebuild)");
    ExitCode::FAILURE
}
```

In `crates/obsbot4linux/Cargo.toml`, replace the whole `[dependencies]` table with the one below and remove a `[lib]` section if G left one; keep `[package]`, the binary target and every `[package.metadata.*]` table as they are:

```toml
[dependencies]
obsbot-proto.workspace = true
```

Remove the `[dev-dependencies]` entries that only the deleted tests used.

- [ ] **Step 3: Write the dependency check and see it fail**

Create `scripts/check-cli-deps.sh` and make it executable (`chmod +x scripts/check-cli-deps.sh`):

```sh
#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Design spec 3: the [dependencies] of the CLI crate never contain obsbot-device or
# obsbot4linuxd (its [dev-dependencies] may). Fails with a message otherwise.
set -eu
cd "$(dirname "$0")/.."
deps=$(cargo metadata --format-version 1 --no-deps --locked |
  jq -r '.packages[] | select(.name == "obsbot4linux") | .dependencies[] | select(.kind == null) | .name')
for forbidden in obsbot-device obsbot4linuxd; do
  if printf '%s\n' "$deps" | grep -qx "$forbidden"; then
    echo "the obsbot4linux CLI depends on $forbidden; only obsbot-proto is allowed (spec section 3)" >&2
    exit 1
  fi
done
echo "CLI dependencies: $(printf '%s ' $deps)"
```

To see that the check can fail, add `obsbot-device.workspace = true` under `[dependencies]` of `crates/obsbot4linux/Cargo.toml` for a moment:

Run: `scripts/check-cli-deps.sh; echo "exit=$?"`
Expected: `the obsbot4linux CLI depends on obsbot-device; only obsbot-proto is allowed (spec section 3)` and `exit=1`.

Remove the line again.

- [ ] **Step 4: Run the check and the build**

Run: `cargo build --workspace && scripts/check-cli-deps.sh`
Expected: the build succeeds; the script prints `CLI dependencies: obsbot-proto` and exits 0.

Run: `cargo run -q -p obsbot4linux; echo "exit=$?"`
Expected: `obsbot4linux: the 1.0 command line is not built yet (stage W0 of the 1.0 rebuild)` and `exit=1`.

- [ ] **Step 5: Run the check in CI**

In `.github/workflows/ci.yml`, job `test`, add a step after the step that runs `cargo test`:

```yaml
- run: scripts/check-cli-deps.sh
```

(`jq` is installed on the `ubuntu-22.04` runners.) The acceptance is the `required` check passing on the PR, not a local YAML parse.

- [ ] **Step 6: Commit**

```bash
git add -A crates/obsbot4linux crates/obsbot-device/src/legacy scripts/check-cli-deps.sh .github/workflows/ci.yml Cargo.lock
git commit -m "Remove the 0.13 command line from rebuild; CLI stub and dependency check"
```

### Task 7: Trait conformance check

**Files:**

- Modify: `crates/obsbot-device/src/contract/conformance.rs` (replace the stub)
- Test: `crates/obsbot-device/tests/conformance.rs`

**Interfaces:**

- Consumes (Task 5): `Camera`, `Ctx`, `Motion`, `Target`, `Tier`, `Group`; (Task 2): `check_description`, `check_value`, `Fit`; (Task 1): `values::{number, same}`, `Bound`, `Setting`, `SettingType`, `Warning`.
- Produces: `obsbot_device::contract::conformance::check(&mut dyn Camera) -> Vec<String>` (empty when the camera conforms; the camera must be awake) and `sample_write(&Setting, Option<&Value>) -> Option<Value>`. Task 9 runs it on `fake-a` and `fake-b`; track A runs it on `tiny2lite` over the replay transport (spec 8.7 step 6).

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot-device/tests/conformance.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance check itself: a small in-memory camera passes, and each kind of flaw it
//! must catch makes it fail. (The fakes are checked in tests/fakes.rs.)

use obsbot_device::contract::conformance::check;
use obsbot_device::contract::{
    Camera, CameraError, Ctx, Group, Identity, Motion, Need, SendTable, Target, Tier,
};
use obsbot_proto::validate::Refusal;
use obsbot_proto::{Action, Asleep, Description, Params, Setting, State, Values};
use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq)]
enum Flaw {
    None,
    WritableInTouchedGroup,
    TouchedLacksOwnGroup,
    ReadsOutOfRange,
    OrderIgnoresRequires,
    LeafInNoGroup,
    ContinuingWithoutStop,
}

struct Mini {
    desc: Description,
    plan: Vec<Group>,
    values: Values,
    flaw: Flaw,
}

fn group(name: &str, tier: Tier, asleep: bool, leaves: &[&str]) -> Group {
    Group {
        name: name.into(),
        tier,
        asleep,
        live: None,
        leaves: leaves.iter().map(|l| l.to_string()).collect(),
    }
}

impl Mini {
    fn new(flaw: Flaw) -> Mini {
        let w = |s: Setting| s.asleep(Asleep::Unknown).settle_ms(3000);
        let desc = Description::default()
            .setting("asleep", Setting::boolean("Asleep").readonly())
            .setting(
                "led.brightness",
                w(Setting::number("LED brightness", 0.0, 3.0).step(1.0)),
            )
            .setting(
                "led.blink",
                Setting::boolean("LED blink")
                    .asleep(Asleep::Unknown)
                    .no_readback(),
            )
            .setting(
                "exposure.mode",
                w(Setting::enumeration("Exposure mode", &["auto", "manual"])),
            )
            .setting(
                "exposure.time",
                w(Setting::number("Exposure time", 100.0, 1000.0))
                    .requires("exposure.mode", json!("manual")),
            )
            .action("gimbal.turn", Action::new("Turn", Asleep::Unknown))
            .action("gimbal.stop", Action::new("Stop", Asleep::Unknown));
        let led_tier = if flaw == Flaw::WritableInTouchedGroup {
            Tier::Touched
        } else {
            Tier::Second
        };
        let mut plan = vec![
            group("status", Tier::Second, true, &["asleep"]),
            group("led", led_tier, false, &["led.brightness"]),
            group(
                "exposure",
                Tier::FiveSeconds,
                false,
                &["exposure.mode", "exposure.time"],
            ),
        ];
        if flaw == Flaw::LeafInNoGroup {
            plan[2].leaves.pop();
        }
        let values = [
            ("asleep", json!(false)),
            ("led.brightness", json!(1)),
            ("exposure.mode", json!("auto")),
            ("exposure.time", json!(300)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        Mini {
            desc,
            plan,
            values,
            flaw,
        }
    }
}

impl Camera for Mini {
    fn description(&self) -> &Description {
        &self.desc
    }
    fn identity(&mut self, _need: Need, _ctx: &Ctx) -> Result<Identity, CameraError> {
        Ok(Identity::default())
    }
    fn poll_plan(&self) -> &[Group] {
        &self.plan
    }
    fn read(&mut self, name: &str, _ctx: &Ctx) -> Result<Values, CameraError> {
        let g = self
            .plan
            .iter()
            .find(|g| g.name == name)
            .ok_or(CameraError::NoAnswer(name.into()))?;
        let mut out: Values = g
            .leaves
            .iter()
            .map(|l| (l.clone(), self.values[l].clone()))
            .collect();
        if self.flaw == Flaw::ReadsOutOfRange && name == "led" {
            out.insert("led.brightness".into(), json!(9));
        }
        Ok(out)
    }
    fn order(&self, leaves: &[&str]) -> Vec<String> {
        let mut out: Vec<String> = leaves.iter().map(|l| l.to_string()).collect();
        out.sort_by_key(|l| if l == "exposure.mode" { 0 } else { 1 });
        if self.flaw == Flaw::OrderIgnoresRequires {
            out.reverse();
        }
        out
    }
    fn apply(&mut self, leaf: &str, value: &Value, _ctx: &Ctx) -> Result<(), CameraError> {
        self.values.insert(leaf.into(), value.clone());
        Ok(())
    }
    fn check(&self, _action: &str, _params: &Params, _state: &State) -> Result<(), Refusal> {
        Ok(())
    }
    fn act(
        &mut self,
        _action: &str,
        _params: &Params,
        _state: &State,
        _ctx: &Ctx,
    ) -> Result<Option<Params>, CameraError> {
        Ok(None)
    }
    fn touched(&self, target: Target<'_>) -> Vec<String> {
        match target {
            Target::Leaf(_) if self.flaw == Flaw::TouchedLacksOwnGroup => vec![],
            Target::Leaf(leaf) => self
                .plan
                .iter()
                .filter(|g| g.leaves.iter().any(|l| l == leaf))
                .map(|g| g.name.clone())
                .collect(),
            Target::Action(_) => vec![],
        }
    }
    fn motion(&self, action: &str) -> Motion {
        match action {
            "gimbal.turn" => Motion::Continuing {
                stop: "gimbal.stop",
            },
            "gimbal.stop" if self.flaw == Flaw::ContinuingWithoutStop => Motion::Move,
            "gimbal.stop" => Motion::Stop,
            _ => Motion::None,
        }
    }
    fn send_table(&self) -> &SendTable {
        static EMPTY: SendTable = SendTable {
            entries: Vec::new(),
        };
        &EMPTY
    }
}

#[test]
fn a_conforming_camera_passes() {
    assert_eq!(check(&mut Mini::new(Flaw::None)), Vec::<String>::new());
}

#[test]
fn each_flaw_is_caught() {
    let rows = [
        (
            Flaw::WritableInTouchedGroup,
            "led.brightness: writable with a readback, so its group needs a periodic tier",
        ),
        (
            Flaw::TouchedLacksOwnGroup,
            "led.brightness: touched [] lacks its own group led",
        ),
        (Flaw::ReadsOutOfRange, "led.brightness: read 9 does not fit"),
        (Flaw::OrderIgnoresRequires, "exposure.time: order"),
        (
            Flaw::LeafInNoGroup,
            "exposure.time: has a readback but is in no group",
        ),
        (
            Flaw::ContinuingWithoutStop,
            "gimbal.turn: continuing, but its stop gimbal.stop is not a stop",
        ),
    ];
    for (flaw, expected) in rows {
        let problems = check(&mut Mini::new(flaw)).join("\n");
        assert!(
            problems.contains(expected),
            "expected {expected:?} in:\n{problems}"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot-device --test conformance`
Expected: FAIL to compile, `unresolved import obsbot_device::contract::conformance::check`.

- [ ] **Step 3: Write the check**

Replace `crates/obsbot-device/src/contract/conformance.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The trait conformance check (design spec 10.3 W0, 8.7 step 6). Every model and every fake
//! passes it: the fakes in W0, `tiny2lite` on the replay transport in track A.
//!
//! It checks: the description is consistent; every leaf with a readback is in exactly one
//! group and every readable leaf reads back a value that fits its setting; every writable
//! leaf with a readback is in a group with a periodic tier (5.7); every writable leaf takes a
//! valid written value, and after the touched re-read its value fits (it shows the written
//! value, or the daemon would list it in `pending`); `touched` includes the leaf's own group;
//! `order` puts `requires` paths first; the stop of every continuing action is an action that
//! the model marks as a stop.
//! The camera must be awake. Writes that restart the camera (`warning: restart`) are skipped.

use crate::contract::camera::{Camera, Ctx, Motion, Target, Tier};
use obsbot_proto::validate::{Fit, check_description, check_value};
use obsbot_proto::values::{number, same};
use obsbot_proto::{Bound, Setting, SettingType, Warning};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Runs the check; returns the problems, empty when the camera conforms.
pub fn check(camera: &mut dyn Camera) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    let desc = camera.description().clone();
    problems.extend(
        check_description(&desc)
            .into_iter()
            .map(|p| format!("description: {p}")),
    );

    let plan = camera.poll_plan().to_vec();
    let mut group_of: BTreeMap<String, String> = BTreeMap::new();
    let mut names = BTreeSet::new();
    for group in &plan {
        if !names.insert(group.name.clone()) {
            problems.push(format!("group {}: named twice", group.name));
        }
        for leaf in &group.leaves {
            if let Some(other) = group_of.insert(leaf.clone(), group.name.clone()) {
                problems.push(format!("{leaf}: in groups {other} and {}", group.name));
            }
            match desc.settings.get(leaf) {
                None => problems.push(format!(
                    "group {}: {leaf} is not a described leaf",
                    group.name
                )),
                Some(s) if !s.readback => problems.push(format!(
                    "{leaf}: readback false but in group {}",
                    group.name
                )),
                Some(_) => {}
            }
        }
        if let Some(live) = &group.live
            && !group
                .leaves
                .iter()
                .any(|l| desc.settings[l].live.as_deref() == Some(live.topic.as_str()))
        {
            problems.push(format!(
                "group {}: live tier for {} but no leaf of that topic",
                group.name, live.topic
            ));
        }
    }
    for (path, s) in &desc.settings {
        if s.readback && !group_of.contains_key(path) {
            problems.push(format!("{path}: has a readback but is in no group"));
        }
        if !s.readonly && s.readback {
            let tier = group_of
                .get(path)
                .and_then(|g| plan.iter().find(|p| &p.name == g))
                .map(|g| g.tier);
            if tier == Some(Tier::Touched) {
                problems.push(format!(
                    "{path}: writable with a readback, so its group needs a periodic tier (5.7)"
                ));
            }
        }
    }

    let mut current: BTreeMap<String, Value> = BTreeMap::new();
    for group in &plan {
        match camera.read(&group.name, &Ctx::none()) {
            Err(e) => problems.push(format!("group {}: read failed: {e}", group.name)),
            Ok(values) => {
                let got: BTreeSet<&String> = values.keys().collect();
                let want: BTreeSet<&String> = group.leaves.iter().collect();
                if got != want {
                    problems.push(format!(
                        "group {}: read returned {got:?}, the plan says {want:?}",
                        group.name
                    ));
                }
                for (leaf, value) in values {
                    if let Some(s) = desc.settings.get(&leaf)
                        && let Err((_, m)) = check_value(s, &value, Fit::State)
                    {
                        problems.push(format!("{leaf}: read {value} does not fit: {m}"));
                    }
                    current.insert(leaf, value);
                }
            }
        }
    }

    let writable: Vec<(&String, &Setting)> =
        desc.settings.iter().filter(|(_, s)| !s.readonly).collect();
    for (path, s) in &writable {
        for req in &s.requires {
            let order = camera.order(&[path.as_str(), req.path.as_str()]);
            let at = |p: &str| order.iter().position(|o| o == p);
            if at(&req.path).zip(at(path)).is_none_or(|(r, l)| r > l) {
                problems.push(format!(
                    "{path}: order {order:?} does not put {} first",
                    req.path
                ));
            }
        }
    }
    let all: Vec<&str> = writable.iter().map(|(p, _)| p.as_str()).collect();
    let mut ordered = camera.order(&all);
    ordered.sort();
    let mut expected: Vec<String> = all.iter().map(|p| p.to_string()).collect();
    expected.sort();
    if ordered != expected {
        problems.push("order: not a permutation of the leaves it was given".into());
    }

    for (path, s) in &writable {
        if s.warning.contains(&Warning::Restart) {
            continue;
        }
        let Some(value) = sample_write(s, current.get(*path)) else {
            problems.push(format!("{path}: no writable value to test with"));
            continue;
        };
        for req in &s.requires {
            if let Err(e) = camera.apply(&req.path, &req.equals, &Ctx::none()) {
                problems.push(format!(
                    "{path}: writing its requirement {} failed: {e}",
                    req.path
                ));
            }
        }
        if let Err(e) = camera.apply(path, &value, &Ctx::none()) {
            problems.push(format!("{path}: apply {value} failed: {e}"));
            continue;
        }
        let touched = camera.touched(Target::Leaf(path));
        if let Some(own) = group_of.get(*path)
            && !touched.contains(own)
        {
            problems.push(format!(
                "{path}: touched {touched:?} lacks its own group {own}"
            ));
        }
        for group in &touched {
            if !names.contains(group) {
                problems.push(format!("{path}: touches the unknown group {group}"));
                continue;
            }
            match camera.read(group, &Ctx::none()) {
                Err(e) => problems.push(format!("{path}: re-read of {group} failed: {e}")),
                Ok(values) => {
                    if let Some(read) = values.get(*path)
                        && let Err((_, m)) = check_value(s, read, Fit::State)
                    {
                        problems.push(format!(
                            "{path}: after writing {value} it reads {read}, which does not fit: {m}"
                        ));
                    }
                    current.extend(values);
                }
            }
        }
    }

    for name in desc.actions.keys() {
        if let Motion::Continuing { stop } = camera.motion(name) {
            if !desc.actions.contains_key(stop) {
                problems.push(format!(
                    "{name}: continuing, but its stop {stop} is not an action"
                ));
            } else if camera.motion(stop) != Motion::Stop {
                problems.push(format!(
                    "{name}: continuing, but its stop {stop} is not a stop"
                ));
            }
        }
        for group in camera.touched(Target::Action(name)) {
            if !names.contains(&group) {
                problems.push(format!("{name}: touches the unknown group {group}"));
            }
        }
    }
    problems
}

fn bound(b: &Option<Bound>, component: Option<&str>) -> Option<f64> {
    b.as_ref().and_then(|b| b.get(component))
}

/// A writable value for `s` that differs from `current` where the setting allows it.
pub fn sample_write(s: &Setting, current: Option<&Value>) -> Option<Value> {
    let differs = |v: &Value| current.is_none_or(|c| !same(c, v));
    let pick = |candidates: Vec<Value>| {
        let fitting: Vec<Value> = candidates
            .into_iter()
            .filter(|v| check_value(s, v, Fit::Write).is_ok())
            .collect();
        fitting
            .iter()
            .find(|v| differs(v))
            .or(fitting.first())
            .cloned()
    };
    match &s.kind {
        SettingType::Bool => pick(vec![json!(true), json!(false)]),
        SettingType::Enum => pick(s.writable.iter().map(|v| json!(v)).collect()),
        SettingType::Number => {
            let lo = bound(&s.min, None);
            let hi = bound(&s.max, None);
            let mut c: Vec<Value> = [hi, lo].into_iter().flatten().map(number).collect();
            c.push(number(lo.unwrap_or(0.0) + 1.0));
            pick(c)
        }
        SettingType::String => {
            let len = s.max_len.unwrap_or(16).min(11) as usize;
            pick(vec![json!(&"Conformance"[..len]), json!("")])
        }
        kind => {
            let names = kind.components()?;
            let corner = |high: bool| {
                let mut m = Map::new();
                for name in names {
                    let lo = bound(&s.min, Some(name)).unwrap_or(0.0);
                    let hi = bound(&s.max, Some(name)).unwrap_or(lo);
                    let x = if *kind == SettingType::Range {
                        if *name == "min" { lo } else { hi }
                    } else if high {
                        hi
                    } else {
                        lo
                    };
                    m.insert(name.to_string(), number(x));
                }
                Value::Object(m)
            };
            pick(vec![corner(true), corner(false)])
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test conformance`
Expected: PASS: `test result: ok. 2 passed` (the conforming camera passes; each of the six flaws is reported).

Run: `cargo fmt --all --check && cargo clippy -p obsbot-device --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-device/src/contract/conformance.rs crates/obsbot-device/tests/conformance.rs
git commit -m "obsbot-device: trait conformance check for every model and fake"
```

### Task 8: Tiny 2 Lite description as data

**Files:**

- Create: `crates/obsbot-device/src/models/tiny2lite/description.rs`
- Modify: `crates/obsbot-device/src/models/tiny2lite/mod.rs` (one line)
- Test: unit tests inside `description.rs`

**Interfaces:**

- Consumes (Task 1): the description builders of `Setting`, `Action`, `Description`; `Asleep`, `SettingType`, `Unit`, `Warning`. (Task 2): `obsbot_proto::validate::check_description` in the tests. (Task 5): the `obsbot-proto` and `serde_json` dependencies of `crates/obsbot-device/Cargo.toml`; without them this file does not compile.
- Produces (module `obsbot_device::models::tiny2lite::description`): `fn description() -> Description` (84 leaves of spec 4.3, 27 actions of 4.4, `limits.presets = 3`), `fn pose(label: &str) -> Setting`, `const MODES: [&str; 8]`, `const SETTLE_MS: u32 = 3000`, `const UNBACKED: &[(&str, &str)]` (every mark of spec 9.3), `const UNCONFIRMED: &[&str]` (the bounds, steps and defaults of decision 12 that no ledger row backs yet). Task 9's `fake-a` uses `description()`; Task 10 generates `descriptions/tiny2lite.json` from it; after W0 track A owns this file (spec 10.3 Track A owner files) and changes its content only by contract-change PRs.

- [ ] **Step 1: Add the module**

In `crates/obsbot-device/src/models/tiny2lite/mod.rs`, add next to G's module lines:

```rust
pub mod description;
```

- [ ] **Step 2: Write the failing test**

The tests are the `#[cfg(test)] mod tests` at the end of the file of Step 4. Write that module first into `crates/obsbot-device/src/models/tiny2lite/description.rs`, with this above it:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

use obsbot_proto::{Action, Asleep, Description, Setting, SettingType, Unit, Warning};
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p obsbot-device --lib tiny2lite::description`
Expected: FAIL to compile, `cannot find function description in this scope` and `cannot find value UNBACKED` (and `UNCONFIRMED`).

- [ ] **Step 4: Write the description**

Replace `crates/obsbot-device/src/models/tiny2lite/description.rs` with the whole file:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The description of the Tiny 2 Lite as data (design spec 4.3, 4.4, 8.8; field ledger
//! `docs/field-ledger.md`). `descriptions/tiny2lite.json` is generated from it.
//!
//! The fields, values and bounds of spec section 9.3 are 0.13.2 functions without backing
//! today. They are in the description, and [`UNBACKED`] marks each with its row; camera
//! session CS-1 keeps or removes them by a contract-change PR before the release candidate.

use obsbot_proto::{Action, Asleep, Description, Setting, SettingType, Unit, Warning};
use serde_json::json;

/// The tracking modes of `tracking.mode` and `power_on.mode`.
pub const MODES: [&str; 8] = [
    "static",
    "normal",
    "upper_body",
    "close_up",
    "headless",
    "lower_body",
    "hand",
    "group",
];

/// The one model-wide settle bound (8.8): 3000 ms for every writable leaf with a readback.
pub const SETTLE_MS: u32 = 3000;

/// Every mark of spec 9.3: (key, row). Keys: `<path>` a field; `<path>=<value>` an enum
/// value; `<path>@min` or `@max` a bound; `<path>#<aspect>` a property that is not a value or
/// a bound; `do:<action>`, `do:<action>.<param>=<value>`, `do:<action>.<param>@max` and
/// `do:<action>#<aspect>` the same for actions.
pub const UNBACKED: &[(&str, &str)] = &[
    ("framing.offset.x@min", "U-4"),
    ("framing.offset.x@max", "U-4"),
    ("framing.offset.y@min", "U-4"),
    ("framing.offset.y@max", "U-4"),
    ("framing.offset.x#direction", "U-4"),
    ("framing.offset.y#direction", "U-4"),
    ("hand.axes.pan", "U-5"),
    ("hand.axes.tilt", "U-5"),
    ("hand.return_after", "U-6"),
    ("do:hand.area_reset", "U-7"),
    ("hand.area.pan#order", "U-8"),
    ("hand.area.tilt#order", "U-8"),
    ("zone.area.pan#order", "U-8"),
    ("zone.area.tilt#order", "U-8"),
    ("do:gimbal.move.speed@max", "U-9"),
    ("do:gimbal.move#speed_near_tilt_limit", "U-9"),
    ("do:gimbal.move#speed_one_axis", "U-9"),
    ("do:zoom.move.speed=slow", "U-12"),
    ("do:zoom.move.speed=fast", "U-12"),
    ("do:focus.point", "U-16"),
    ("power_on.start.mode=static", "U-18"),
    ("power_on.start.mode=upper_body", "U-18"),
    ("power_on.start.mode=close_up", "U-18"),
    ("power_on.start.mode=headless", "U-18"),
    ("power_on.start.mode=lower_body", "U-18"),
    ("power_on.start.mode=hand", "U-18"),
    ("power_on.start.mode=group", "U-18"),
    ("power_on.settings.tracking_speed#values", "U-19"),
    ("power_on.settings.autofocus#values", "U-19"),
    ("power_on.settings.auto_exposure#values", "U-19"),
    ("power_on.settings.exposure_compensation#values", "U-19"),
    ("power_on.settings.white_balance_mode#values", "U-19"),
    ("power_on.settings.white_balance_temperature#values", "U-19"),
    ("power_on.settings.contrast#values", "U-19"),
    ("power_on.settings.saturation#values", "U-19"),
    ("power_on.settings.sharpness#values", "U-19"),
    ("power_on.settings.hue#values", "U-19"),
    ("exposure.compensation#ev_scale", "U-20"),
    ("audio.enabled#readback", "U-23"),
    ("exposure.anti_flicker=auto", "U-25"),
    ("power_on.start.source=preset_0", "U-26"),
    ("power_on.start.source=preset_1", "U-26"),
    ("power_on.start.source=preset_2", "U-26"),
];

/// Bounds, steps and defaults that this description holds but neither the spec nor the field
/// ledger backs yet (the W0 plan's decision 12): `<path>@min`, `<path>@max`, `<path>#step`,
/// `<path>#default`. They are not evidence. Track A confirms each in the ledger, from a capture
/// or a measurement on the camera, or changes it, before the evidence-level test of stage W3.
pub const UNCONFIRMED: &[&str] = &[
    "exposure.time@min",
    "exposure.time@max",
    "exposure.time#default",
    "exposure.gain#default",
    "focus.position#default",
    "image.brightness@min",
    "image.brightness@max",
    "image.brightness#default",
    "image.contrast@min",
    "image.contrast@max",
    "image.contrast#default",
    "image.saturation@min",
    "image.saturation@max",
    "image.saturation#default",
    "image.hue@min",
    "image.hue@max",
    "image.hue#default",
    "image.sharpness@min",
    "image.sharpness@max",
    "image.sharpness#default",
    "image.white_balance.auto#default",
    "image.white_balance.temperature@min",
    "image.white_balance.temperature@max",
    "image.white_balance.temperature#step",
    "image.white_balance.temperature#default",
];

/// A writable leaf with a readback: asleep `unknown` until CAM-2, the model-wide settle bound.
fn w(s: Setting) -> Setting {
    s.asleep(Asleep::Unknown).settle_ms(SETTLE_MS)
}

/// A writable leaf the camera cannot report (`readback: false`, 8.8).
fn w_no_readback(s: Setting) -> Setting {
    s.asleep(Asleep::Unknown).no_readback()
}

/// A pose `{pan, tilt, zoom}` in degrees and zoom factor, within the gimbal range.
pub fn pose(label: &str) -> Setting {
    Setting::composite(
        SettingType::Pose,
        label,
        &[("pan", -150.0), ("tilt", -70.0), ("zoom", 1.0)],
        &[("pan", 150.0), ("tilt", 90.0), ("zoom", 4.0)],
    )
    .units(&[
        ("pan", Unit::Deg),
        ("tilt", Unit::Deg),
        ("zoom", Unit::Factor),
    ])
    .precisions(&[("pan", 0.1), ("tilt", 0.1), ("zoom", 0.01)])
}

fn angle_range(label: &str, lo: f64, hi: f64) -> Setting {
    w(Setting::range(label, lo, hi).unit(Unit::Deg).precision(0.1))
}

fn uvc(label: &str, min: f64, max: f64, default: f64) -> Setting {
    w(Setting::number(label, min, max)
        .step(1.0)
        .precision(1.0)
        .default_value(json!(default)))
}

fn slot() -> Setting {
    Setting::number("Preset slot", 0.0, 2.0).step(1.0)
}

fn view_id() -> Setting {
    Setting::new(SettingType::Number, "View id")
        .min(0.0)
        .step(1.0)
}

fn name() -> Setting {
    Setting::string("Name", 32, true)
}

fn raw_block(label: &str) -> Setting {
    Setting::new(SettingType::Number, label).readonly()
}

/// The Tiny 2 Lite's settings (4.3), actions (4.4) and limits.
pub fn description() -> Description {
    let mut start_modes: Vec<&str> = MODES.to_vec();
    start_modes.push("unset");
    Description::default()
        .setting("asleep", Setting::boolean("Asleep").readonly())
        .setting("on_air.active", Setting::boolean("On air").readonly())
        .setting(
            "on_air.apps",
            Setting::list(
                "Applications using the camera",
                &[("name", Setting::string("Process name", 255, false))],
            ),
        )
        .setting(
            "gimbal.pan",
            Setting::new(SettingType::Number, "Pan")
                .unit(Unit::Deg)
                .precision(0.1)
                .readonly()
                .live("pose"),
        )
        .setting(
            "gimbal.tilt",
            Setting::new(SettingType::Number, "Tilt")
                .unit(Unit::Deg)
                .precision(0.1)
                .readonly()
                .live("pose"),
        )
        .setting(
            "gimbal.invert",
            w(Setting::boolean("Reverse the pan direction")),
        )
        .setting(
            "zoom.factor",
            w(Setting::number("Zoom", 1.0, 4.0)
                .unit(Unit::Factor)
                .precision(0.01)),
        )
        .setting(
            "zoom.field_of_view",
            Setting::enumeration("Field of view", &["wide", "medium", "narrow", "custom"])
                .writable_only(&["wide", "medium", "narrow"])
                .asleep(Asleep::Ignored)
                .settle_ms(SETTLE_MS),
        )
        .setting(
            "tracking.mode",
            w(Setting::enumeration("Tracking mode", &MODES)),
        )
        .setting(
            "tracking.profile",
            w(Setting::enumeration(
                "Tracking profile",
                &["standard", "headroom", "sport"],
            )),
        )
        .setting(
            "tracking.speed",
            w(Setting::enumeration(
                "Tracking speed",
                &["lazy", "slow", "normal", "fast", "very_fast", "auto"],
            )),
        )
        .setting(
            "tracking.target.detected",
            Setting::boolean("Target detected").readonly(),
        )
        .setting(
            "tracking.target.box",
            Setting::composite(
                SettingType::Box,
                "Target box",
                &[("left", 0.0), ("top", 0.0), ("right", 0.0), ("bottom", 0.0)],
                &[("left", 1.0), ("top", 1.0), ("right", 1.0), ("bottom", 1.0)],
            )
            .unit(Unit::Fraction)
            .precision(0.01)
            .readonly()
            .nullable()
            .live("target"),
        )
        .setting(
            "framing.auto_zoom",
            w(Setting::boolean("Automatic framing zoom")),
        )
        .setting(
            "framing.size",
            w(Setting::enumeration(
                "Framing size",
                &[
                    "long_shot",
                    "medium_long_shot",
                    "medium_shot",
                    "medium_close_up",
                    "close_up",
                    "big_close_up",
                ],
            )),
        )
        .setting(
            "framing.zoom_speed",
            w(Setting::enumeration(
                "Framing zoom speed",
                &["slow", "medium", "fast"],
            )),
        )
        .setting(
            "framing.offset.x",
            w(Setting::number("Framing offset x", -0.16, 0.16)
                .unit(Unit::Fraction)
                .precision(0.01)),
        )
        .setting(
            "framing.offset.y",
            w(Setting::number("Framing offset y", -0.16, 0.16)
                .unit(Unit::Fraction)
                .precision(0.01)),
        )
        .setting(
            "framing.part",
            w(Setting::enumeration(
                "Framed part",
                &["normal", "head_removal", "lower_body"],
            )),
        )
        .setting(
            "gestures.target_lock",
            w(Setting::boolean("Gesture: target lock")),
        )
        .setting("gestures.zoom", w(Setting::boolean("Gesture: zoom")))
        .setting(
            "gestures.zoom_ratio",
            w(Setting::number("Gesture zoom ratio", 1.0, 4.0)
                .unit(Unit::Factor)
                .step(0.1)
                .precision(0.1)),
        )
        .setting(
            "gestures.dynamic_zoom",
            w(Setting::boolean("Gesture: dynamic zoom")),
        )
        .setting(
            "gestures.direction_change",
            w(Setting::boolean("Gesture: reverse the dynamic zoom")),
        )
        .setting("zone.enabled", w(Setting::boolean("Zone tracking")))
        .setting(
            "zone.follow",
            w(Setting::boolean("Zone: follow the target")),
        )
        .setting(
            "zone.area.pan",
            angle_range("Zone pan range", -150.0, 150.0),
        )
        .setting(
            "zone.area.tilt",
            angle_range("Zone tilt range", -70.0, 90.0),
        )
        .setting("zone.return", pose("Zone return pose").readonly())
        .setting(
            "hand.side",
            w(Setting::enumeration("Hand", &["left", "right"])),
        )
        .setting(
            "hand.area.pan",
            angle_range("Hand tracking pan range", -150.0, 150.0),
        )
        .setting(
            "hand.area.tilt",
            angle_range("Hand tracking tilt range", -70.0, 90.0),
        )
        .setting(
            "hand.axes.pan",
            w(Setting::boolean("Hand tracking may pan")),
        )
        .setting(
            "hand.axes.tilt",
            w(Setting::boolean("Hand tracking may tilt")),
        )
        .setting(
            "hand.return_after",
            w(Setting::new(SettingType::Number, "Hand return after")
                .min(0.0)
                .unit(Unit::Seconds)
                .precision(0.1)),
        )
        .setting(
            "hand.move_gimbal",
            w_no_readback(Setting::boolean("Hand tracking moves the gimbal")),
        )
        .setting(
            "hand.return",
            pose("Hand return pose").readonly().nullable(),
        )
        .setting(
            "presets",
            Setting::list(
                "Presets",
                &[("slot", slot()), ("name", name()), ("pose", pose("Pose"))],
            ),
        )
        .setting(
            "views",
            Setting::list(
                "Views",
                &[("id", view_id()), ("name", name()), ("pose", pose("Pose"))],
            ),
        )
        .setting(
            "exposure.mode",
            w(Setting::enumeration("Exposure", &["auto", "manual"])),
        )
        .setting(
            "exposure.region",
            w(Setting::enumeration(
                "Exposure metering",
                &["face", "global"],
            )),
        )
        .setting(
            "exposure.time",
            w(Setting::number("Exposure time", 100.0, 250_000.0)
                .unit(Unit::Microseconds)
                .precision(1.0))
            .requires("exposure.mode", json!("manual"))
            .live("auto")
            .default_value(json!(33_000)),
        )
        .setting("exposure.gain", uvc("Gain", 1.0, 64.0, 1.0).live("auto"))
        .setting(
            "exposure.compensation",
            uvc("Exposure compensation", 0.0, 18.0, 9.0),
        )
        .setting("exposure.hdr", w(Setting::boolean("HDR")))
        .setting(
            "exposure.anti_flicker",
            w(
                Setting::enumeration("Anti-flicker", &["off", "50hz", "60hz", "auto"])
                    .writable_only(&["off", "50hz", "60hz"]),
            ),
        )
        .setting("image.brightness", uvc("Brightness", 0.0, 100.0, 50.0))
        .setting("image.contrast", uvc("Contrast", 0.0, 100.0, 50.0))
        .setting("image.saturation", uvc("Saturation", 0.0, 100.0, 50.0))
        .setting("image.hue", uvc("Hue", 0.0, 100.0, 50.0))
        .setting("image.sharpness", uvc("Sharpness", 0.0, 100.0, 50.0))
        .setting(
            "image.white_balance.auto",
            w(Setting::boolean("Automatic white balance")).default_value(json!(true)),
        )
        .setting(
            "image.white_balance.temperature",
            w(
                Setting::number("White balance temperature", 2000.0, 10_000.0)
                    .step(100.0)
                    .unit(Unit::Kelvin)
                    .precision(1.0),
            )
            .requires("image.white_balance.auto", json!(false))
            .live("auto")
            .default_value(json!(5000)),
        )
        .setting("image.mirror", w(Setting::boolean("Mirror")))
        .setting(
            "image.frame_rate",
            Setting::number("Frame rate", 30.0, 60.0)
                .unit(Unit::Fps)
                .readonly(),
        )
        .setting(
            "focus.mode",
            w(Setting::enumeration(
                "Focus",
                &["continuous", "single", "manual"],
            )),
        )
        .setting(
            "focus.region",
            w(Setting::enumeration("Focus region", &["face", "global"])),
        )
        .setting(
            "focus.position",
            uvc("Focus position", 0.0, 100.0, 0.0)
                .requires("focus.mode", json!("manual"))
                .no_effect(),
        )
        .setting(
            "audio.enabled",
            w(Setting::boolean("Microphone")).warning(Warning::Restart),
        )
        .setting(
            "audio.volume",
            w(Setting::number("Microphone volume", 0.0, 100.0)
                .step(1.0)
                .unit(Unit::Percent)
                .precision(1.0)),
        )
        .setting(
            "audio.noise_reduction",
            w(Setting::enumeration(
                "Noise reduction",
                &["off", "weak", "medium", "strong"],
            )),
        )
        .setting(
            "audio.auto_gain",
            w(Setting::boolean("Automatic microphone gain")),
        )
        .setting(
            "audio.in_sleep",
            w(Setting::boolean("Microphone while asleep")),
        )
        .setting(
            "led.brightness",
            w(Setting::number("LED brightness", 0.0, 3.0)
                .step(1.0)
                .precision(1.0)),
        )
        .setting("led.blink", w_no_readback(Setting::boolean("LED blink")))
        .setting("sleep.auto.enabled", w(Setting::boolean("Auto-sleep")))
        .setting(
            "sleep.auto.seconds",
            w(Setting::number("Auto-sleep after", 1.0, 32_767.0)
                .step(1.0)
                .unit(Unit::Seconds)
                .precision(1.0)),
        )
        .setting(
            "sleep.background",
            w(
                Setting::enumeration("Sleep background", &["black", "image", "own_image"])
                    .writable_only(&["black", "image"]),
            ),
        )
        .setting(
            "sleep.background_mirror",
            w(Setting::boolean("Mirror the sleep background")),
        )
        .setting(
            "sleep.wake_zoom",
            w(Setting::number("Zoom at wake", 1.0, 4.0)
                .unit(Unit::Factor)
                .precision(0.01))
            .no_effect(),
        )
        .setting(
            "power_on.mode",
            w(Setting::enumeration("Tracking mode at power-on", &MODES)),
        )
        .setting(
            "power_on.start.source",
            Setting::enumeration(
                "Initial state source",
                &["own", "preset_0", "preset_1", "preset_2"],
            )
            .readonly(),
        )
        .setting("power_on.start.pose", pose("Initial pose").readonly())
        .setting(
            "power_on.start.mode",
            w(
                Setting::enumeration("Initial tracking mode", &start_modes).writable_only(&[
                    "static",
                    "normal",
                    "upper_body",
                    "headless",
                    "group",
                    "unset",
                ]),
            ),
        )
        .setting(
            "power_on.settings.tracking_speed",
            raw_block("Initial tracking speed (raw)").nullable(),
        )
        .setting(
            "power_on.settings.autofocus",
            raw_block("Initial autofocus (raw)").nullable(),
        )
        .setting(
            "power_on.settings.auto_exposure",
            raw_block("Initial automatic exposure (raw)").nullable(),
        )
        .setting(
            "power_on.settings.exposure_compensation",
            raw_block("Initial exposure compensation (raw)"),
        )
        .setting(
            "power_on.settings.white_balance_mode",
            raw_block("Initial white balance mode (raw)").nullable(),
        )
        .setting(
            "power_on.settings.white_balance_temperature",
            raw_block("Initial white balance temperature (raw)").nullable(),
        )
        .setting(
            "power_on.settings.contrast",
            raw_block("Initial contrast (raw)").nullable(),
        )
        .setting(
            "power_on.settings.saturation",
            raw_block("Initial saturation (raw)").nullable(),
        )
        .setting(
            "power_on.settings.sharpness",
            raw_block("Initial sharpness (raw)").nullable(),
        )
        .setting(
            "power_on.settings.hue",
            raw_block("Initial hue (raw)").nullable(),
        )
        .action(
            "gimbal.move",
            Action::new("Move the gimbal", Asleep::Unknown)
                .param(
                    "pan",
                    Setting::number("Pan", -150.0, 150.0)
                        .unit(Unit::Deg)
                        .precision(0.1)
                        .optional(),
                )
                .param(
                    "tilt",
                    Setting::number("Tilt", -70.0, 90.0)
                        .unit(Unit::Deg)
                        .precision(0.1)
                        .optional(),
                )
                .param(
                    "speed",
                    Setting::number("Speed", 1.0, 180.0)
                        .unit(Unit::DegPerSecond)
                        .precision(0.1)
                        .optional(),
                ),
        )
        .action(
            "gimbal.stop",
            Action::new("Stop the gimbal", Asleep::Unknown),
        )
        .action(
            "gimbal.turn",
            Action::new("Turn the gimbal", Asleep::Unknown)
                .param(
                    "pan_speed",
                    Setting::number("Pan speed", -178.0, 178.0)
                        .unit(Unit::DegPerSecond)
                        .precision(0.1)
                        .optional(),
                )
                .param(
                    "tilt_speed",
                    Setting::number("Tilt speed", -178.0, 178.0)
                        .unit(Unit::DegPerSecond)
                        .precision(0.1)
                        .optional(),
                )
                .param(
                    "keep",
                    Setting::boolean("Keep turning after the connection closes").optional(),
                ),
        )
        .action(
            "zoom.move",
            Action::new("Zoom at a speed", Asleep::Unknown)
                .param(
                    "factor",
                    Setting::number("Zoom", 1.0, 4.0)
                        .unit(Unit::Factor)
                        .precision(0.01),
                )
                .param(
                    "speed",
                    Setting::enumeration("Speed", &["slow", "medium", "fast"]),
                ),
        )
        .action("zoom.stop", Action::new("Stop zooming", Asleep::Unknown))
        .action(
            "preset.save",
            Action::new("Save a preset", Asleep::Ignored)
                .param("slot", slot())
                .param("name", name().optional())
                .param("pose", pose("Pose").optional()),
        )
        .action(
            "preset.recall",
            Action::new("Recall a preset", Asleep::Unknown).param("slot", slot()),
        )
        .action(
            "preset.rename",
            Action::new("Rename a preset", Asleep::Unknown)
                .param("slot", slot())
                .param("name", name()),
        )
        .action(
            "preset.delete",
            Action::new("Delete a preset", Asleep::Unknown).param("slot", slot()),
        )
        .action(
            "view.save",
            Action::new("Save a view", Asleep::Ignored)
                .param("name", name())
                .param("pose", pose("Pose").optional())
                .result("id", view_id()),
        )
        .action(
            "view.go",
            Action::new("Go to a view", Asleep::Unknown).param("id", view_id()),
        )
        .action(
            "view.rename",
            Action::new("Rename a view", Asleep::Unknown)
                .param("id", view_id())
                .param("name", name()),
        )
        .action(
            "view.delete",
            Action::new("Delete a view", Asleep::Unknown).param("id", view_id()),
        )
        .action(
            "zone.return_here",
            Action::new("Set the zone return pose here", Asleep::Ignored),
        )
        .action(
            "zone.go_return",
            Action::new("Go to the zone return pose", Asleep::Unknown),
        )
        .action(
            "hand.return_here",
            Action::new("Set the hand return pose here", Asleep::Ignored),
        )
        .action(
            "hand.go_return",
            Action::new("Go to the hand return pose", Asleep::Unknown),
        )
        .action(
            "hand.return_clear",
            Action::new("Clear the hand return pose", Asleep::Unknown),
        )
        .action(
            "hand.area_reset",
            Action::new("Reset the hand tracking area", Asleep::Unknown),
        )
        .action(
            "focus.point",
            Action::new("Focus on a point", Asleep::Unknown)
                .param(
                    "x",
                    Setting::number("x", 0.0, 1.0)
                        .unit(Unit::Fraction)
                        .precision(0.01),
                )
                .param(
                    "y",
                    Setting::number("y", 0.0, 1.0)
                        .unit(Unit::Fraction)
                        .precision(0.01),
                ),
        )
        .action(
            "power_on.set_pose",
            Action::new("Set the initial pose", Asleep::Unknown).param("pose", pose("Pose")),
        )
        .action(
            "power_on.use_current",
            Action::new("Use the current pose as the initial pose", Asleep::Ignored),
        )
        .action(
            "power_on.use_preset",
            Action::new("Use a preset as the initial pose", Asleep::Unknown).param("slot", slot()),
        )
        .action(
            "power_on.go",
            Action::new("Go to the initial pose", Asleep::Ignored),
        )
        .action("sleep", Action::new("Sleep", Asleep::Unknown))
        .action("wake", Action::new("Wake", Asleep::Accepted))
        .action(
            "factory_reset",
            Action::new("Factory reset", Asleep::Unknown)
                .param("confirm", Setting::boolean("Confirm"))
                .warning(Warning::Destructive)
                .warning(Warning::Restart),
        )
        .limit("presets", 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::validate::check_description;
    use std::collections::BTreeSet;

    #[test]
    fn the_description_is_consistent() {
        assert_eq!(check_description(&description()), Vec::<String>::new());
    }

    /// Every leaf of the state model of 4.3 (the rows of the field ledger without the daemon's
    /// `available`, `pending` and `unread`) and every action of 4.4, and nothing else.
    #[test]
    fn it_has_exactly_the_paths_and_actions_of_the_spec() {
        const LEAVES: [&str; 84] = [
            "asleep",
            "audio.auto_gain",
            "audio.enabled",
            "audio.in_sleep",
            "audio.noise_reduction",
            "audio.volume",
            "exposure.anti_flicker",
            "exposure.compensation",
            "exposure.gain",
            "exposure.hdr",
            "exposure.mode",
            "exposure.region",
            "exposure.time",
            "focus.mode",
            "focus.position",
            "focus.region",
            "framing.auto_zoom",
            "framing.offset.x",
            "framing.offset.y",
            "framing.part",
            "framing.size",
            "framing.zoom_speed",
            "gestures.direction_change",
            "gestures.dynamic_zoom",
            "gestures.target_lock",
            "gestures.zoom",
            "gestures.zoom_ratio",
            "gimbal.invert",
            "gimbal.pan",
            "gimbal.tilt",
            "hand.area.pan",
            "hand.area.tilt",
            "hand.axes.pan",
            "hand.axes.tilt",
            "hand.move_gimbal",
            "hand.return",
            "hand.return_after",
            "hand.side",
            "image.brightness",
            "image.contrast",
            "image.frame_rate",
            "image.hue",
            "image.mirror",
            "image.saturation",
            "image.sharpness",
            "image.white_balance.auto",
            "image.white_balance.temperature",
            "led.blink",
            "led.brightness",
            "on_air.active",
            "on_air.apps",
            "power_on.mode",
            "power_on.settings.auto_exposure",
            "power_on.settings.autofocus",
            "power_on.settings.contrast",
            "power_on.settings.exposure_compensation",
            "power_on.settings.hue",
            "power_on.settings.saturation",
            "power_on.settings.sharpness",
            "power_on.settings.tracking_speed",
            "power_on.settings.white_balance_mode",
            "power_on.settings.white_balance_temperature",
            "power_on.start.mode",
            "power_on.start.pose",
            "power_on.start.source",
            "presets",
            "sleep.auto.enabled",
            "sleep.auto.seconds",
            "sleep.background",
            "sleep.background_mirror",
            "sleep.wake_zoom",
            "tracking.mode",
            "tracking.profile",
            "tracking.speed",
            "tracking.target.box",
            "tracking.target.detected",
            "views",
            "zone.area.pan",
            "zone.area.tilt",
            "zone.enabled",
            "zone.follow",
            "zone.return",
            "zoom.factor",
            "zoom.field_of_view",
        ];
        const ACTIONS: [&str; 27] = [
            "factory_reset",
            "focus.point",
            "gimbal.move",
            "gimbal.stop",
            "gimbal.turn",
            "hand.area_reset",
            "hand.go_return",
            "hand.return_clear",
            "hand.return_here",
            "power_on.go",
            "power_on.set_pose",
            "power_on.use_current",
            "power_on.use_preset",
            "preset.delete",
            "preset.recall",
            "preset.rename",
            "preset.save",
            "sleep",
            "view.delete",
            "view.go",
            "view.rename",
            "view.save",
            "wake",
            "zone.go_return",
            "zone.return_here",
            "zoom.move",
            "zoom.stop",
        ];
        let d = description();
        assert_eq!(
            d.settings.keys().map(String::as_str).collect::<Vec<_>>(),
            LEAVES
        );
        assert_eq!(
            d.actions.keys().map(String::as_str).collect::<Vec<_>>(),
            ACTIONS
        );
        assert_eq!(d.limits.get("presets"), Some(&3));
    }

    #[test]
    fn readback_false_and_asleep_values_follow_section_8_8() {
        let d = description();
        let no_readback: Vec<&String> = d
            .settings
            .iter()
            .filter(|(_, s)| !s.readback)
            .map(|(k, _)| k)
            .collect();
        assert_eq!(no_readback, ["hand.move_gimbal", "led.blink"]);
        assert_eq!(
            d.settings["zoom.field_of_view"].asleep,
            Some(Asleep::Ignored)
        );
        assert_eq!(d.actions["power_on.go"].asleep, Asleep::Ignored);
        assert_eq!(d.actions["wake"].asleep, Asleep::Accepted);
        for pose_taking in [
            "preset.save",
            "view.save",
            "power_on.use_current",
            "zone.return_here",
            "hand.return_here",
        ] {
            assert_eq!(
                d.actions[pose_taking].asleep,
                Asleep::Ignored,
                "{pose_taking} (CAM-22)"
            );
        }
        for (path, s) in &d.settings {
            if !s.readonly && s.readback {
                assert_eq!(s.settle_ms, Some(SETTLE_MS), "{path}");
            }
        }
    }

    /// Every mark names something the description has, and every row of 9.3 is marked.
    #[test]
    fn every_unbacked_mark_resolves_and_every_row_of_9_3_is_marked() {
        let d = description();
        for (key, row) in UNBACKED {
            let (base, aspect) = key
                .split_once('#')
                .map_or((*key, None), |(b, a)| (b, Some(a)));
            let (base, value) = base
                .split_once('=')
                .map_or((base, None), |(b, v)| (b, Some(v)));
            let (base, bound) = base
                .split_once('@')
                .map_or((base, None), |(b, x)| (b, Some(x)));
            let setting = match base.strip_prefix("do:") {
                Some(rest) => {
                    let (action, param) = match d.actions.get(rest) {
                        Some(_) => (rest, None),
                        None => rest
                            .rsplit_once('.')
                            .map(|(a, p)| (a, Some(p)))
                            .expect("an action or action.param"),
                    };
                    let a = d
                        .actions
                        .get(action)
                        .unwrap_or_else(|| panic!("{key}: no action {action}"));
                    param.map(|p| {
                        a.params
                            .get(p)
                            .unwrap_or_else(|| panic!("{key}: no parameter {p}"))
                    })
                }
                None => Some(
                    d.settings
                        .get(base)
                        .unwrap_or_else(|| panic!("{key}: no setting {base}")),
                ),
            };
            if let Some(v) = value {
                assert!(
                    setting.is_some_and(|s| s.values.iter().any(|x| x == v)),
                    "{key}: no value {v}"
                );
            }
            if let Some(b) = bound {
                let s = setting.expect("a bound belongs to a setting or parameter");
                let present = if b == "min" {
                    s.min.is_some()
                } else {
                    s.max.is_some()
                };
                assert!(present, "{key}: no bound {b}");
            }
            assert!(aspect.is_none_or(|a| !a.is_empty()), "{key}");
            assert!(row.starts_with("U-"), "{key}: {row}");
        }
        let rows: BTreeSet<&str> = UNBACKED.iter().map(|(_, r)| *r).collect();
        let want: BTreeSet<&str> = [
            "U-4", "U-5", "U-6", "U-7", "U-8", "U-9", "U-12", "U-16", "U-18", "U-19", "U-20",
            "U-23", "U-25", "U-26",
        ]
        .into();
        assert_eq!(rows, want);
    }

    /// Every unconfirmed number names a bound, step or default the description has, so that
    /// track A's ledger work and the W3 evidence-level test find each one.
    #[test]
    fn every_unconfirmed_number_is_in_the_description() {
        let d = description();
        for key in UNCONFIRMED {
            let (path, what) = key
                .split_once(['@', '#'])
                .unwrap_or_else(|| panic!("{key}: path@bound or path#aspect"));
            let s = d
                .settings
                .get(path)
                .unwrap_or_else(|| panic!("{key}: no setting {path}"));
            let present = match what {
                "min" => s.min.is_some(),
                "max" => s.max.is_some(),
                "step" => s.step.is_some(),
                "default" => s.default.is_some(),
                _ => false,
            };
            assert!(present, "{key}: the setting has no {what}");
        }
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --lib tiny2lite::description`
Expected: PASS: `test result: ok. 5 passed`.

Run: `cargo fmt --all --check && cargo clippy -p obsbot-device --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot-device/src/models/tiny2lite/description.rs crates/obsbot-device/src/models/tiny2lite/mod.rs
git commit -m "tiny2lite: the description as data, with the unbacked fields of 9.3 marked"
```

### Task 12: `docs/protocol-2.md`, normative part

**Files:**

- Create: `docs/protocol-2.md`

**Interfaces:**

- Consumes: the decisions of this plan, the types of Task 1, the validation order of Task 2, the paths of Task 8. The task is stacked on Task 8 (or starts once it has landed), because Step 2 compares the dictionary with Task 8's description.
- Produces: the client reference of spec 4 ("Its normative part is written in stage W0 from this section; stage W2 adds the worked examples") and the path dictionary of spec 4.2 that every model PR extends.

- [ ] **Step 1: Write the document**

Create `docs/protocol-2.md`:

```markdown
# obsbot4linux protocol 2

The protocol between the obsbot4linux daemon (`obsbot4linuxd`) and its clients: the CLI, the
widget and the desktop app. This part is normative. The machine-readable form is
`schema/protocol-2.json`; the Rust types are the crate `obsbot-proto`; the golden transcripts in
`crates/obsbot-proto/tests/golden/` show every message. Worked examples follow in a later
revision of this file.

## 1. Transport

- A Unix socket at `$OBSBOT4LINUX_SOCKET`, else `$XDG_RUNTIME_DIR/obsbot4linux.sock`, mode 0600.
- One JSON object per line, UTF-8. A request line longer than 64 KiB closes the connection.
  Lines from the daemon have no length limit; clients must not cap them.
- At most 64 connections; a further connection is closed at once.
- The first message on a connection is `hello`. Requests sent after `hello` and before its
  reply are kept and handled after the reply.

## 2. Requests

Every request has `id` (an integer or a string, echoed verbatim) and exactly one verb.

| Verb    | Request                                                                      | `ok` payload                                 |
| ------- | ---------------------------------------------------------------------------- | -------------------------------------------- |
| `hello` | `{"id","hello":{"protocol":2,"client":"<name version>","subscribe","live"}}` | `{"protocol":2,"daemon","devices","states"}` |
| `get`   | `{"id","device","get":["<path>"…]}` or `"get":null` for the full state       | `{"rev","state"}`                            |
| `set`   | `{"id","device","set":<patch>}`                                              | `{"rev"}`                                    |
| `do`    | `{"id","device","do":"<action>","with":{…}}`; `with` may be left out         | `{"rev","result"?}`                          |
| `live`  | `{"id","live":["<topic>"…]}`: replaces the connection's live topics          | `{}`                                         |

- `subscribe` is `true` and `live` is `[]` when left out.
- A patch is an object of leaf paths, nested (`{"zoom":{"factor":2}}`) or dotted
  (`{"zoom.factor":2}`), or both. Only leaves are written.
- `get` paths are leaves, groups (`zone`, `zone.area`) or the daemon fields `available`,
  `pending`, `unread`. The reply holds the paths asked for plus `available`, `pending` and
  `unread` in full.
- Unknown fields of a request are ignored.

## 3. Replies and events

- Reply: `{"id":<id>,"ok":{…}}` or `{"id":<id>,"error":{…}}`. `id` is `null` only when the
  request had no usable id.
- `{"event":"state","device","rev","state"}`: the full state of one device.
- `{"event":"devices","devices":[<Device>…]}`: the full device list, sent whenever an entry is
  added, removed or changed.
- Events go only to connections with `subscribe: true`.

## 4. Errors

`{"code","message","rev"?,"applied"?,"failed"?,"supported"?}`. `message` is English, for logs.
`rev` is present whenever the daemon re-read the device before answering.

| Code                 | When                                                                                                                                                                    |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `protocol`           | the first message is not `hello`, or its protocol is not supported; `supported: [2]`; the connection closes                                                             |
| `invalid`            | malformed request, no verb or two verbs, wrong value type, a read-only path, a group written as a value, a second `hello`, a destructive action without `confirm: true` |
| `unknown_device`     | no device with this id                                                                                                                                                  |
| `unsupported`        | well-formed, but the path or action is not in this device's description (a typo included)                                                                               |
| `unsupported_device` | a `set` or `do` for a device with `support: unsupported`                                                                                                                |
| `out_of_range`       | the value lies outside the description: bounds, step, `writable`, string length or characters                                                                           |
| `conflict`           | the device's current state does not allow it: a `requires` not met, an empty preset slot, an unknown view id, a list not read yet                                       |
| `unavailable`        | a `set` or `do` for an unplugged device                                                                                                                                 |
| `asleep`             | the camera is asleep and the setting or action is `asleep: ignored`                                                                                                     |
| `busy`               | 32 requests are already queued for the device                                                                                                                           |
| `camera`             | the camera refused or did not answer, the node stayed locked, or the 15 s deadline passed; `applied`, `failed`                                                          |
| `internal`           | a daemon bug                                                                                                                                                            |

**Validation order.** A `set` or `do` is checked as a whole before anything is sent. When
several things are wrong, the first code in the order `invalid`, `unsupported`,
`out_of_range`, `conflict`, `asleep` wins, and within one code the first path in path order.
The model's own checks of a `do` against the state (an empty preset slot, an unknown view id,
a list not read yet: `conflict`; `gimbal.move` without an axis: `invalid`) run only after
these checks passed, so a client that runs the validator of `obsbot-proto` sees every other
refusal first.

## 5. Ordering, `rev` and `pending`

1. `rev` is per device entry: 1 for the first state of a new entry, it only goes up while the
   entry exists, and it restarts with the daemon. A `port:` entry added again starts at 1.
   Clients forget the rev of an id when a `devices` event no longer lists it. Every change of a
   published state (values, `pending`, `unread`, `available`) gets a new rev.
2. Requests for one available, supported device are answered in request order. Answered at
   once instead: `busy`, `unknown_device`, every request for an unavailable or unsupported
   device. A stop (an action the model marks as a stop; on the Tiny 2 Lite `gimbal.stop` and
   `zoom.stop`) is taken before every queued request. Requests may be pipelined.
3. For a `set` or `do` that changes state, the state event reaches every subscribed connection
   **before** the reply. The reply's `rev` is the rev of the first state that reflects the
   request. If nothing changed, the reply carries the current rev.
4. `pending` maps a leaf path to the value written: the camera accepted the write, its readback
   does not show it yet. The path leaves `pending` when a read shows the written value (after
   rounding to the precision), or when `settle_ms` has passed at a read of its group; then the
   camera's value stands. Clients show their own value from sending a request until its reply,
   and the state, `pending` included, after that. They run no timers of their own.
5. A failed `set` or `do` after a frame went out answers `camera` with `applied` and `failed`.
   There is no rollback.
6. Motion actions reply when the camera accepted the command; the pose moves through later
   events.
7. Additive changes keep protocol 2: fields, actions, models, enum values, live topics, setting
   types, units. Clients ignore unknown fields and event types, show unknown enum values as
   raw text, show settings of an unknown type read-only as text, and treat an unknown error
   code as `internal`.
8. A state change that touches only volatile leaves and that no request caused goes only to
   connections whose live topics include one of those leaves' `live` topics.
9. A request is answered within 15 s of its arrival.

After any reconnect a client drops its pending requests and own values, sends `hello` and
rebuilds from the reply. Revs of two connections are never compared.

## 6. Devices

`Device`: `id`, `name`, `model`, `support`, `serial`, `firmware`, `video_node`, `available`,
`replaced_by` (only during a hand-over), `usb`, `settings`, `actions`, `limits`.

- `id`: `sn:<serial>` when the serial matches `[A-Za-z0-9]{8,32}`, else `port:<usb path>`.
- `support`: `tested`, `untested` or `unsupported`. An unsupported device has `model:
"unknown"`, empty `settings`, `actions` and `limits`, and the fixed state
  `{"available":true,"pending":{},"unread":[]}` at rev 1.
- `usb`: `vendor`, `product`, `bcd_device` (four lower-case hex digits), `product_name`
  (or `null`), `path` (`3-2.1`), `serial_present`, `interfaces` (two hex digits each, in
  interface order), `extension_units` (`unit`, `guid`, `controls`). Never the serial itself.
- `limits`: list path → maximum item count, where the camera has one.
- Human output and logs show an `sn:` id or a serial as `…` and its last four characters;
  `--json` and the socket carry the full id.

## 7. Descriptions

`descriptions/<model>.json` holds a model's `settings`, `actions` and `limits`, exactly as a
`Device` carries them. Maps are sorted by key.

**Setting** (one leaf, one action parameter or one result field):

| Field                                           | Meaning                                                                                                                                     |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `type`                                          | `bool`, `enum`, `number`, `range` `{min,max}`, `pose` `{pan,tilt,zoom}`, `point` `{x,y}`, `box` `{left,top,right,bottom}`, `string`, `list` |
| `label`                                         | English, for CLI output                                                                                                                     |
| `readonly`, `nullable`, `readback`, `no_effect` | always present; `readback: false` means the camera cannot report the value                                                                  |
| `asleep`                                        | `accepted`, `ignored` or `unknown`; on every writable leaf                                                                                  |
| `values`, `writable`                            | enum: every value a state may show, and the ones a `set` accepts (on every writable enum)                                                   |
| `min`, `max`, `step`, `precision`               | a number; for `range`, `pose`, `point`, `box` an object with one number per component                                                       |
| `unit`                                          | `deg`, `deg/s`, `factor`, `us`, `K`, `percent`, `s`, `fraction`, `fps`; per component for a `pose`; absent for a raw camera value           |
| `max_len`, `ascii`                              | string: length in characters; printable ASCII only                                                                                          |
| `item`                                          | list: the fields of one item; lists are read-only                                                                                           |
| `optional`                                      | action parameters only                                                                                                                      |
| `requires`                                      | `[{path, equals}…]`, checked against the current state with the patch applied                                                               |
| `default`                                       | the camera's own default as it reports it                                                                                                   |
| `live`                                          | the live topic of a volatile leaf                                                                                                           |
| `settle_ms`                                     | the longest time the readback may lag a write; on every writable leaf with a readback                                                       |
| `warning`                                       | `restart`, `destructive`; a destructive action needs `confirm: true`                                                                        |

**Action**: `label`, `params` (name → Setting), `result` (name → Setting, optional), `asleep`,
`requires`, `warning`.

## 8. Values

- Angles in degrees; pan positive = the camera turns to its left; tilt positive = up; 0/0 =
  straight ahead. Picture coordinates 0..1 from the top-left corner. Zoom as a factor (1.0 =
  widest). Gimbal speeds in °/s. Durations in seconds. Exposure time in microseconds. Colour
  temperature in K. Percentages 0–100.
- Numbers are rounded to their precision where they are created; an integral value is written
  without a fraction.
- A field the model does not have is absent. `null` means: the camera reports "not set" (only
  where `nullable`), the leaf was never read (it is in `unread`), or a `readback: false` leaf
  not written since the device last became available.
- `unread` lists the leaf paths never read since the device last became available, and those
  whose last read failed (they keep their last value).
- A camera value the model cannot decode is the string `raw:<n>`.
- List items carry the camera's own numbers: preset slots start at 0; view ids are the
  camera's ids.

## 9. Path dictionary

One meaning per path on every model. Naming: lower-case `snake_case` segments joined by `.`,
the camera function first, no model name. A model PR that adds a path or an enum value adds its
line here.

| Path                                                                                     | Type         | Unit                       | Meaning                                                                                                                                                                                                            |
| ---------------------------------------------------------------------------------------- | ------------ | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `asleep`                                                                                 | bool         | –                          | the camera sleeps (read-only)                                                                                                                                                                                      |
| `on_air.active`                                                                          | bool         | –                          | some process reads the video node (read-only)                                                                                                                                                                      |
| `on_air.apps`                                                                            | list         | –                          | items `{name}`: those processes, best effort                                                                                                                                                                       |
| `gimbal.pan`, `gimbal.tilt`                                                              | number       | deg                        | the live pose (read-only, topic `pose`)                                                                                                                                                                            |
| `gimbal.invert`                                                                          | bool         | –                          | reverses the gimbal's pan direction                                                                                                                                                                                |
| `zoom.factor`                                                                            | number       | factor                     | zoom                                                                                                                                                                                                               |
| `zoom.field_of_view`                                                                     | enum         | –                          | field of view                                                                                                                                                                                                      |
| `tracking.mode`                                                                          | enum         | –                          | what the tracking follows                                                                                                                                                                                          |
| `tracking.profile`                                                                       | enum         | –                          | tracking profile                                                                                                                                                                                                   |
| `tracking.speed`                                                                         | enum         | –                          | tracking speed                                                                                                                                                                                                     |
| `tracking.target.detected`                                                               | bool         | –                          | a target is detected (read-only)                                                                                                                                                                                   |
| `tracking.target.box`                                                                    | box          | fraction                   | the target in the picture (read-only, topic `target`)                                                                                                                                                              |
| `framing.auto_zoom`                                                                      | bool         | –                          | the tracking zooms to frame the target                                                                                                                                                                             |
| `framing.size`                                                                           | enum         | –                          | shot size of the framing                                                                                                                                                                                           |
| `framing.zoom_speed`                                                                     | enum         | –                          | zoom speed of the framing                                                                                                                                                                                          |
| `framing.offset.x`, `framing.offset.y`                                                   | number       | fraction                   | offset of the framed target                                                                                                                                                                                        |
| `framing.part`                                                                           | enum         | –                          | framed body part                                                                                                                                                                                                   |
| `gestures.target_lock`, `gestures.zoom`, `gestures.dynamic_zoom`                         | bool         | –                          | hand gestures on or off                                                                                                                                                                                            |
| `gestures.zoom_ratio`                                                                    | number       | factor                     | zoom of the zoom gesture                                                                                                                                                                                           |
| `gestures.direction_change`                                                              | bool         | –                          | reverses the dynamic zoom gesture                                                                                                                                                                                  |
| `zone.enabled`, `zone.follow`                                                            | bool         | –                          | zone tracking; following the target in the zone                                                                                                                                                                    |
| `zone.area.pan`, `zone.area.tilt`                                                        | range        | deg                        | the tracking zone                                                                                                                                                                                                  |
| `zone.return`                                                                            | pose         | pan, tilt deg; zoom factor | where zone tracking returns (read-only)                                                                                                                                                                            |
| `hand.side`                                                                              | enum         | –                          | the hand the hand tracking follows                                                                                                                                                                                 |
| `hand.area.pan`, `hand.area.tilt`                                                        | range        | deg                        | the hand tracking area                                                                                                                                                                                             |
| `hand.axes.pan`, `hand.axes.tilt`                                                        | bool         | –                          | hand tracking may pan; may tilt                                                                                                                                                                                    |
| `hand.return_after`                                                                      | number       | s                          | time outside the area before the return                                                                                                                                                                            |
| `hand.move_gimbal`                                                                       | bool         | –                          | hand tracking moves the gimbal                                                                                                                                                                                     |
| `hand.return`                                                                            | pose         | pan, tilt deg; zoom factor | where hand tracking returns (read-only)                                                                                                                                                                            |
| `presets`                                                                                | list         | –                          | items `{slot, name, pose}`                                                                                                                                                                                         |
| `views`                                                                                  | list         | –                          | items `{id, name, pose}`                                                                                                                                                                                           |
| `exposure.mode`                                                                          | enum         | –                          | automatic or manual exposure                                                                                                                                                                                       |
| `exposure.region`                                                                        | enum         | –                          | metering of the automatic exposure                                                                                                                                                                                 |
| `exposure.time`                                                                          | number       | us                         | exposure time (topic `auto`)                                                                                                                                                                                       |
| `exposure.gain`                                                                          | number       | raw                        | gain (topic `auto`)                                                                                                                                                                                                |
| `exposure.compensation`                                                                  | number       | raw                        | exposure compensation, the camera's raw value                                                                                                                                                                      |
| `exposure.hdr`                                                                           | bool         | –                          | HDR                                                                                                                                                                                                                |
| `exposure.anti_flicker`                                                                  | enum         | –                          | mains frequency the exposure avoids                                                                                                                                                                                |
| `image.brightness`, `image.contrast`, `image.saturation`, `image.hue`, `image.sharpness` | number       | raw                        | UVC picture controls                                                                                                                                                                                               |
| `image.white_balance.auto`                                                               | bool         | –                          | automatic white balance                                                                                                                                                                                            |
| `image.white_balance.temperature`                                                        | number       | K                          | white balance (topic `auto`)                                                                                                                                                                                       |
| `image.mirror`                                                                           | bool         | –                          | mirrored picture                                                                                                                                                                                                   |
| `image.frame_rate`                                                                       | number       | fps                        | frame rate (read-only)                                                                                                                                                                                             |
| `focus.mode`                                                                             | enum         | –                          | focus mode                                                                                                                                                                                                         |
| `focus.region`                                                                           | enum         | –                          | focus region                                                                                                                                                                                                       |
| `focus.position`                                                                         | number       | raw                        | manual focus position                                                                                                                                                                                              |
| `audio.enabled`                                                                          | bool         | –                          | microphone                                                                                                                                                                                                         |
| `audio.volume`                                                                           | number       | percent                    | microphone volume                                                                                                                                                                                                  |
| `audio.noise_reduction`                                                                  | enum         | –                          | noise reduction                                                                                                                                                                                                    |
| `audio.auto_gain`, `audio.in_sleep`                                                      | bool         | –                          | automatic gain; microphone while asleep                                                                                                                                                                            |
| `led.brightness`                                                                         | number       | raw                        | status LED brightness                                                                                                                                                                                              |
| `led.blink`                                                                              | bool         | –                          | status LED blinks                                                                                                                                                                                                  |
| `sleep.auto.enabled`, `sleep.auto.seconds`                                               | bool, number | –, s                       | auto-sleep and its delay                                                                                                                                                                                           |
| `sleep.background`, `sleep.background_mirror`                                            | enum, bool   | –                          | picture while asleep; mirrored                                                                                                                                                                                     |
| `sleep.wake_zoom`                                                                        | number       | factor                     | zoom at wake                                                                                                                                                                                                       |
| `power_on.mode`                                                                          | enum         | –                          | tracking mode at power-on                                                                                                                                                                                          |
| `power_on.start.source`                                                                  | enum         | –                          | where the initial state came from (read-only)                                                                                                                                                                      |
| `power_on.start.pose`                                                                    | pose         | pan, tilt deg; zoom factor | initial pose (read-only)                                                                                                                                                                                           |
| `power_on.start.mode`                                                                    | enum         | –                          | tracking mode of the initial state                                                                                                                                                                                 |
| `power_on.settings.<name>`                                                               | number       | raw                        | the initial state's settings: `tracking_speed`, `autofocus`, `auto_exposure`, `exposure_compensation`, `white_balance_mode`, `white_balance_temperature`, `contrast`, `saturation`, `sharpness`, `hue` (read-only) |

Paths of the test models only (fake mode): `beacon.color` (enum) and `beacon.pulse` (bool) of
`fake-b`, a group no client knows.

Actions: `gimbal.move`, `gimbal.stop`, `gimbal.turn`, `zoom.move`, `zoom.stop`, `preset.save`,
`preset.recall`, `preset.rename`, `preset.delete`, `view.save`, `view.go`, `view.rename`,
`view.delete`, `zone.return_here`, `zone.go_return`, `hand.return_here`, `hand.go_return`,
`hand.return_clear`, `hand.area_reset`, `focus.point`, `power_on.set_pose`,
`power_on.use_current`, `power_on.use_preset`, `power_on.go`, `sleep`, `wake`,
`factory_reset`; `beacon.flash` of `fake-b`. Their parameters are in each model's description.

## 10. Live topics

| Topic    | Leaves (Tiny 2 Lite)                                                |
| -------- | ------------------------------------------------------------------- |
| `pose`   | `gimbal.pan`, `gimbal.tilt`                                         |
| `target` | `tracking.target.box`                                               |
| `auto`   | `exposure.time`, `exposure.gain`, `image.white_balance.temperature` |
```

- [ ] **Step 2: Check it against the code**

Run: `cargo test -p obsbot-device --lib tiny2lite::description` (Task 8 is below this branch), and compare the dictionary's paths with `it_has_exactly_the_paths_and_actions_of_the_spec`: every leaf there has a row (grouped rows such as `image.brightness, …` count for each leaf).
Expected: no leaf missing. Review check: the document names no private path, vendor-app or firmware name, or serial number (spec 8.5).

- [ ] **Step 3: Commit**

```bash
git add docs/protocol-2.md
git commit -m "docs: protocol 2, normative part and path dictionary"
```

### Task 9: Fake models and fake bus

**Files:**

- Create: `crates/obsbot-device/src/models/fake/mod.rs`, `camera.rs`, `a.rs`, `b.rs`, `bus.rs`
- Modify: `crates/obsbot-device/src/models/mod.rs` (one line)
- Test: `crates/obsbot-device/tests/fakes.rs`

**Interfaces:**

- Consumes (Task 5): `Camera`, `CameraError`, `Ctx`, `Group`, `Identity`, `IdentitySource`, `LiveTier`, `Motion`, `Need`, `SendTable`, `Target`, `Tier`, `Bus`, `Found`, `Lookup`, `ModelEntry`, `Registry`, `UsbId`; (Task 7) `conformance::check` in the tests; (Task 8) `tiny2lite::description::description`; (Task 1, 2) the proto types, `values::{number, same}`, `validate::Refusal`, `validate::validate_set` in the tests.
- Produces (module `obsbot_device::models::fake`):
  - `const VENDOR: u16 = 0x3564`, `const UNKNOWN_PRODUCT: u16 = 0xfa00`, `const KEYS: [&str; 3] = ["fake-a", "fake-b", "unknown"]`, `const RESTART: Duration` (1 s), `static REGISTRY: Registry`, `fn description(key: &str) -> Option<Description>`
  - `mod a`: `const PRODUCT: u16 = 0xfa0a`, `const ENTRY: ModelEntry` (`support: Tested`, `identity: Query`, `firmware_query: true`), `fn description() -> Description`; its `motion` marks are those of the Tiny 2 Lite (4.1 rules 2 and 6): `gimbal.turn` continuing with the stop `gimbal.stop`, `gimbal.stop` and `zoom.stop` stops, `gimbal.move`, `zoom.move`, `preset.recall`, `view.go`, `zone.go_return`, `hand.go_return`, `power_on.go` moves
  - `mod b`: `const PRODUCT: u16 = 0xfa0b`, `const ENTRY: ModelEntry` (`support: Untested`, `identity: Query`, `firmware_query: true`; no stop and no continuing action, `preset.recall` a move), `const LAG: Duration` (250 ms), `const SETTLE_MS: u32 = 1000`, `const TIME_STEP: f64 = 100.0`, `fn description() -> Description`
  - `struct FakeBus` with `new(&[&str]) -> Result<FakeBus, String>`, `plug(&str) -> Result<String, String>` (returns the device id), `unplug(&str)`, `sleep(&str)`, `wake(&str)`, `fail_next(&str)` (each `-> Result<(), String>`, ids `sn:` or `port:`), `set_delay(&str, Duration)`, `ops(&str) -> Vec<Op>`, `handle_line(&str) -> String` (the test-control socket of spec 5.12), and `impl Bus for FakeBus`
  - `enum Op { Open, Identity(Need), Read(String), Apply(String, Value), Act(String, Params) }`
  - Behaviour the daemon's tests rely on: fake serials `FAKEA0000001`, `FAKEB0000001`, … in plug order; `unknown` has no serial and its id is `port:9-<n>`; a re-plug brings back the first unplugged device of the model with its serial and USB path and a new device number; `fail_next` fails the next operation once (identity included) and survives an unplug; a read of a group not readable asleep fails while asleep; `fake-b`'s `led.brightness` readback lags `LAG`; its `exposure.time` is stored rounded down to `TIME_STEP`; a write or action with `warning: restart` hides the device from `scan` for `RESTART` and it returns with a new device number; `factory_reset` restores the initial values (presets empty).

- [ ] **Step 1: Add the module**

In `crates/obsbot-device/src/models/mod.rs`, add next to G's module lines:

```rust
pub mod fake;
```

- [ ] **Step 2: Write the failing test**

Create `crates/obsbot-device/tests/fakes.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Behaviour of the fake models through `trait Camera` and of the fake bus (design spec
//! 5.12). The daemon's tests (track B) rely on every behaviour pinned here.

use obsbot_device::contract::conformance::check;
use obsbot_device::contract::{
    Bus, Camera, CameraError, Ctx, Found, IdentitySource, Motion, Need, Target,
};
use obsbot_device::models::fake::{self, FakeBus, Op};
use obsbot_proto::validate::validate_set;
use obsbot_proto::{ErrorCode, Params, Patch, State};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn bus() -> FakeBus {
    FakeBus::new(&fake::KEYS).expect("the three fakes")
}

fn found(bus: &FakeBus, product: &str) -> Found {
    bus.scan()
        .into_iter()
        .find(|f| f.usb.product == product)
        .unwrap_or_else(|| panic!("no device with product {product}"))
}

fn open(bus: &FakeBus, product: &str) -> Box<dyn Camera> {
    bus.open(&found(bus, product)).expect("open")
}

fn params(v: Value) -> Params {
    v.as_object().expect("an object").clone()
}

/// The state a daemon would publish after reading every group once.
fn state_of(cam: &mut dyn Camera) -> State {
    let mut state = State::new(true);
    for group in cam.poll_plan().to_vec() {
        match cam.read(&group.name, &Ctx::none()) {
            Ok(values) => values.into_iter().for_each(|(k, v)| state.set(&k, v)),
            Err(_) => state.unread.extend(group.leaves.clone()),
        }
    }
    state
}

fn read_leaf(cam: &mut dyn Camera, group: &str, leaf: &str) -> Value {
    cam.read(group, &Ctx::none()).expect("read")[leaf].clone()
}

#[test]
fn fake_a_and_fake_b_pass_the_conformance_check() {
    let bus = bus();
    assert_eq!(
        check(&mut *open(&bus, "fa0a")),
        Vec::<String>::new(),
        "fake-a"
    );
    assert_eq!(
        check(&mut *open(&bus, "fa0b")),
        Vec::<String>::new(),
        "fake-b"
    );
}

#[test]
fn the_registry_has_no_shared_pair_and_finds_the_unknown_device_unsupported() {
    assert!(fake::REGISTRY.shared_pairs().is_empty());
    let scan = bus().scan();
    let models: Vec<Option<&str>> = scan.iter().map(|f| f.model.map(|m| m.key)).collect();
    assert_eq!(
        models,
        [Some("fake-a"), Some("fake-b"), None],
        "unknown enters through the registry lookup (5.12)"
    );
    for f in &scan {
        assert_eq!(f.usb.vendor, "3564");
        assert!(
            f.video_node
                .as_deref()
                .is_some_and(|n| n.starts_with("/dev/video"))
        );
    }
    assert_eq!(
        scan[0].model.unwrap().support,
        obsbot_proto::Support::Tested
    );
    assert_eq!(
        scan[1].model.unwrap().support,
        obsbot_proto::Support::Untested
    );
}

#[test]
fn fake_b_differs_from_the_tiny_2_lite_where_the_spec_says() {
    let a = fake::description("fake-a").unwrap();
    let b = fake::description("fake-b").unwrap();
    assert!(
        !b.settings.keys().any(|k| k.starts_with("gimbal.")),
        "no gimbal"
    );
    assert!(!b.settings.contains_key("views"), "no views");
    assert!(
        b.settings.contains_key("beacon.color") && b.actions.contains_key("beacon.flash"),
        "a group no client knows"
    );
    assert_eq!(b.limits["presets"], 5);
    assert_ne!(
        a.settings["tracking.mode"].values, b.settings["tracking.mode"].values,
        "other enum values"
    );
    let asleep: Vec<_> = b
        .settings
        .values()
        .filter_map(|s| s.asleep.clone())
        .collect();
    for kind in [
        obsbot_proto::Asleep::Ignored,
        obsbot_proto::Asleep::Unknown,
        obsbot_proto::Asleep::Accepted,
    ] {
        assert!(asleep.contains(&kind), "{kind:?}");
    }
    assert_eq!(
        b.settings.values().filter(|s| !s.readback).count(),
        2,
        "two readback:false leaves"
    );
    assert!(!b.settings["exposure.time"].requires.is_empty());
}

#[test]
fn a_written_value_reads_back() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    cam.apply("led.brightness", &json!(2), &Ctx::none())
        .unwrap();
    assert_eq!(cam.touched(Target::Leaf("led.brightness")), ["led"]);
    assert_eq!(read_leaf(&mut *cam, "led", "led.brightness"), json!(2));
}

#[test]
fn fake_b_lags_one_readback_and_rounds_another_so_pending_and_settle_both_happen() {
    let bus = bus();
    let mut cam = open(&bus, "fa0b");
    cam.apply("led.brightness", &json!(3), &Ctx::none())
        .unwrap();
    assert_eq!(
        read_leaf(&mut *cam, "led", "led.brightness"),
        json!(0),
        "the touched re-read does not show it yet: pending"
    );
    std::thread::sleep(fake::b::LAG + Duration::from_millis(50));
    assert_eq!(
        read_leaf(&mut *cam, "led", "led.brightness"),
        json!(3),
        "a later read shows it: it leaves pending"
    );
    assert!(
        fake::b::LAG.as_millis() < u128::from(fake::b::SETTLE_MS),
        "the lag is inside the settle bound"
    );

    cam.apply("exposure.mode", &json!("manual"), &Ctx::none())
        .unwrap();
    cam.apply("exposure.time", &json!(1234), &Ctx::none())
        .unwrap();
    assert_eq!(
        read_leaf(&mut *cam, "exposure", "exposure.time"),
        json!(1200),
        "never 1234: only the settle bound ends pending"
    );
}

#[test]
fn asleep_the_fake_reads_only_groups_readable_asleep_and_keeps_writes_until_wake() {
    let bus = bus();
    let mut cam = open(&bus, "fa0b");
    bus.sleep("sn:FAKEB0000001").unwrap();
    assert_eq!(read_leaf(&mut *cam, "status", "asleep"), json!(true));
    assert!(
        matches!(cam.read("led", &Ctx::none()), Err(CameraError::NoAnswer(_))),
        "led is not readable asleep"
    );

    let state = state_of(&mut *cam);
    assert!(state.asleep());
    let desc = cam.description().clone();
    let refused = validate_set(&desc, &state, &Patch::new().leaf("zoom.factor", 1.5)).unwrap_err();
    assert_eq!(
        refused.code,
        ErrorCode::Asleep,
        "asleep: ignored is refused"
    );
    assert!(
        validate_set(&desc, &state, &Patch::new().leaf("led.brightness", 4)).is_ok(),
        "asleep: unknown is written"
    );

    cam.apply("led.brightness", &json!(4), &Ctx::none())
        .unwrap();
    bus.wake("sn:FAKEB0000001").unwrap();
    std::thread::sleep(fake::b::LAG + Duration::from_millis(50));
    assert_eq!(
        read_leaf(&mut *cam, "led", "led.brightness"),
        json!(4),
        "the first read after wake shows it"
    );
}

#[test]
fn sleep_and_wake_actions_change_asleep() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    cam.act("sleep", &Params::new(), &State::new(true), &Ctx::none())
        .unwrap();
    assert_eq!(read_leaf(&mut *cam, "status", "asleep"), json!(true));
    cam.act("wake", &Params::new(), &State::new(true), &Ctx::none())
        .unwrap();
    assert_eq!(read_leaf(&mut *cam, "status", "asleep"), json!(false));
}

#[test]
fn fail_next_fails_exactly_one_operation_so_its_group_goes_unread_once() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    bus.fail_next("sn:FAKEA0000001").unwrap();
    assert!(matches!(
        cam.read("led", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(cam.read("led", &Ctx::none()).is_ok());
    let state = {
        bus.fail_next("sn:FAKEA0000001").unwrap();
        state_of(&mut *cam)
    };
    assert!(
        !state.unread.is_empty(),
        "the failed group's leaves are unread"
    );
}

#[test]
fn identity_answers_only_what_is_needed() {
    // The daemon asks only what the entry declares (5.5): the fakes have both queries, so
    // their serial and firmware reach the device list.
    for entry in [&fake::a::ENTRY, &fake::b::ENTRY] {
        assert_eq!(
            (entry.identity, entry.firmware_query),
            (IdentitySource::Query, true),
            "{}",
            entry.key
        );
    }
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    let both = cam
        .identity(
            Need {
                serial: true,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(both.serial.as_deref(), Some("FAKEA0000001"));
    assert!(obsbot_proto::is_valid_serial(
        both.serial.as_deref().unwrap()
    ));
    assert!(both.firmware.is_some());
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
    bus.fail_next("sn:FAKEA0000001").unwrap();
    assert!(
        cam.identity(
            Need {
                serial: true,
                firmware: true
            },
            &Ctx::none()
        )
        .is_err(),
        "an identity read fails too"
    );
}

#[test]
fn unplug_and_replug_give_the_same_device_under_a_new_device_number() {
    let bus = bus();
    let before = found(&bus, "fa0a");
    let mut old = bus.open(&before).unwrap();
    bus.unplug("sn:FAKEA0000001").unwrap();
    assert_eq!(bus.scan().len(), 2);
    assert_eq!(
        old.read("led", &Ctx::none()),
        Err(CameraError::Gone),
        "a read of a gone device fails at once"
    );
    assert_eq!(
        bus.plug("fake-a").unwrap(),
        "sn:FAKEA0000001",
        "the same serial comes back"
    );
    let after = found(&bus, "fa0a");
    assert_eq!(after.usb.path, before.usb.path, "same port, same path");
    assert_ne!(after.devnum, before.devnum, "a new enumeration (5.3)");
    assert_eq!(
        old.read("led", &Ctx::none()),
        Err(CameraError::Gone),
        "the old handle stays gone"
    );
    assert!(bus.open(&after).unwrap().read("led", &Ctx::none()).is_ok());
    assert!(
        bus.open(&before).is_err(),
        "the old enumeration cannot be opened"
    );
}

#[test]
fn the_unknown_device_receives_nothing() {
    let bus = bus();
    let unknown = found(&bus, "fa00");
    assert!(unknown.model.is_none());
    assert!(bus.open(&unknown).is_err());
    let id = format!("port:{}", unknown.usb.path);
    for _ in 0..3 {
        bus.scan();
    }
    assert_eq!(bus.ops(&id), Vec::<Op>::new());
}

#[test]
fn restart_warnings_make_the_device_reenumerate_and_factory_reset_clears_presets() {
    let bus = bus();
    let before = found(&bus, "fa0a");
    let mut cam = bus.open(&before).unwrap();
    cam.act(
        "preset.save",
        &params(json!({"slot": 0})),
        &State::new(true),
        &Ctx::none(),
    )
    .unwrap();
    assert_eq!(
        read_leaf(&mut *cam, "presets", "presets")
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let t0 = Instant::now();
    cam.act(
        "factory_reset",
        &params(json!({"confirm": true})),
        &State::new(true),
        &Ctx::none(),
    )
    .unwrap();
    assert!(
        bus.scan().iter().all(|f| f.usb.path != before.usb.path),
        "gone while it restarts"
    );
    assert_eq!(cam.read("led", &Ctx::none()), Err(CameraError::Gone));
    std::thread::sleep(fake::RESTART.saturating_sub(t0.elapsed()) + Duration::from_millis(50));
    let after = found(&bus, "fa0a");
    assert_ne!(after.devnum, before.devnum, "back as a new enumeration");
    let mut cam = bus.open(&after).unwrap();
    assert_eq!(
        read_leaf(&mut *cam, "presets", "presets"),
        json!([]),
        "the presets are deleted"
    );

    cam.apply("audio.enabled", &json!(false), &Ctx::none())
        .unwrap();
    assert!(
        bus.scan().iter().all(|f| f.usb.path != before.usb.path),
        "the microphone write restarts the camera too"
    );
}

#[test]
fn check_refuses_what_the_state_does_not_allow() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    let state = state_of(&mut *cam);
    let code = |cam: &dyn Camera, action: &str, p: Value, state: &State| {
        cam.check(action, &params(p), state).err().map(|r| r.code)
    };
    assert_eq!(
        code(&*cam, "gimbal.move", json!({}), &state),
        Some(ErrorCode::Invalid),
        "4.4: at least one axis"
    );
    assert_eq!(code(&*cam, "gimbal.move", json!({"pan": 10}), &state), None);
    assert_eq!(
        code(&*cam, "preset.recall", json!({"slot": 1}), &state),
        Some(ErrorCode::Conflict),
        "empty slot"
    );
    assert_eq!(
        code(&*cam, "view.go", json!({"id": 7}), &state),
        Some(ErrorCode::Conflict),
        "unknown view id"
    );
    let mut unread = state.clone();
    unread.unread.push("presets".into());
    assert_eq!(
        code(&*cam, "preset.save", json!({"slot": 0}), &unread),
        Some(ErrorCode::Conflict),
        "list in unread"
    );

    cam.act(
        "preset.save",
        &params(json!({"slot": 1, "name": "Desk"})),
        &state,
        &Ctx::none(),
    )
    .unwrap();
    let state = state_of(&mut *cam);
    assert_eq!(
        code(&*cam, "preset.recall", json!({"slot": 1}), &state),
        None
    );
    assert_eq!(state.at("presets").unwrap()[0]["name"], json!("Desk"));
}

#[test]
fn actions_change_the_state_and_view_save_returns_its_id() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    let first = cam
        .act(
            "view.save",
            &params(json!({"name": "Desk"})),
            &State::new(true),
            &Ctx::none(),
        )
        .unwrap();
    let second = cam
        .act(
            "view.save",
            &params(json!({"name": "Door"})),
            &State::new(true),
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(first, Some(params(json!({"id": 0}))));
    assert_eq!(second, Some(params(json!({"id": 1}))));
    cam.act(
        "gimbal.move",
        &params(json!({"pan": 20})),
        &State::new(true),
        &Ctx::none(),
    )
    .unwrap();
    let state = state_of(&mut *cam);
    assert_eq!(state.at("gimbal.pan"), Some(&json!(20)));
    assert_eq!(
        state.at("tracking.mode"),
        Some(&json!("static")),
        "4.4: a move switches tracking to static"
    );
    assert_eq!(
        cam.touched(Target::Action("gimbal.move")),
        ["tracking", "framing"]
    );
}

/// 4.1 rules 2 and 6, 5.6: the daemon learns from `motion` which actions jump the queue and
/// which clear a continuing turn's record. Without the `Move` marks, a connection that turned
/// the gimbal and closed later would stop another client's preset recall.
#[test]
fn motion_marks_follow_rules_2_and_6_of_4_1() {
    let bus = bus();
    let a = open(&bus, "fa0a");
    let motion: Vec<(String, Motion)> = a
        .description()
        .actions
        .keys()
        .map(|name| (name.clone(), a.motion(name)))
        .filter(|(_, m)| *m != Motion::None)
        .collect();
    let want = [
        ("gimbal.move", Motion::Move),
        ("gimbal.stop", Motion::Stop),
        (
            "gimbal.turn",
            Motion::Continuing {
                stop: "gimbal.stop",
            },
        ),
        ("hand.go_return", Motion::Move),
        ("power_on.go", Motion::Move),
        ("preset.recall", Motion::Move),
        ("view.go", Motion::Move),
        ("zone.go_return", Motion::Move),
        ("zoom.move", Motion::Move),
        ("zoom.stop", Motion::Stop),
    ]
    .map(|(n, m)| (n.to_string(), m));
    assert_eq!(motion, want);
    assert_eq!(
        a.motion("preset.recall"),
        Motion::Move,
        "a recall after a turn clears the turn's record, so closing the turning connection does not stop the recall"
    );
    assert_eq!(a.motion("preset.save"), Motion::None);
    let b = open(&bus, "fa0b");
    assert_eq!(b.motion("preset.recall"), Motion::Move);
    assert!(
        b.description()
            .actions
            .keys()
            .all(|n| !matches!(b.motion(n), Motion::Stop | Motion::Continuing { .. })),
        "fake-b has no stops and no continuing action"
    );
    assert!(
        a.send_table().entries.is_empty() && b.send_table().entries.is_empty(),
        "the fakes send nothing"
    );
}

#[test]
fn a_delay_is_cut_by_the_deadline_and_by_a_stop() {
    let bus = bus();
    bus.set_delay("fake-a", Duration::from_secs(5));
    let mut cam = open(&bus, "fa0a");
    let t0 = Instant::now();
    let cut = cam.apply(
        "led.brightness",
        &json!(1),
        &Ctx::until(Instant::now() + Duration::from_millis(100)),
    );
    assert_eq!(cut, Err(CameraError::Cut));
    assert!(
        t0.elapsed() < Duration::from_secs(1),
        "cut at the next frame boundary, not after the delay"
    );
    let flag = Arc::new(AtomicBool::new(true));
    assert_eq!(
        cam.read("led", &Ctx::interruptible(flag)),
        Err(CameraError::Cut)
    );
}

#[test]
fn every_operation_is_recorded_in_order() {
    let bus = bus();
    let mut cam = open(&bus, "fa0b");
    cam.identity(
        Need {
            serial: true,
            firmware: false,
        },
        &Ctx::none(),
    )
    .unwrap();
    cam.read("led", &Ctx::none()).unwrap();
    cam.apply("led.brightness", &json!(1), &Ctx::none())
        .unwrap();
    cam.act(
        "beacon.flash",
        &params(json!({"times": 2})),
        &State::new(true),
        &Ctx::none(),
    )
    .unwrap();
    assert_eq!(
        bus.ops("sn:FAKEB0000001"),
        [
            Op::Open,
            Op::Identity(Need {
                serial: true,
                firmware: false
            }),
            Op::Read("led".into()),
            Op::Apply("led.brightness".into(), json!(1)),
            Op::Act("beacon.flash".into(), params(json!({"times": 2}))),
        ]
    );
}

#[test]
fn the_test_control_lines_of_5_12() {
    let bus = bus();
    assert_eq!(
        bus.handle_line(r#"{"plug":"fake-b"}"#),
        r#"{"ok":"sn:FAKEB0000002"}"#,
        "a second fake-b"
    );
    assert_eq!(
        bus.handle_line(r#"{"sleep":"sn:FAKEB0000002"}"#),
        r#"{"ok":true}"#
    );
    assert_eq!(
        bus.handle_line(r#"{"wake":"sn:FAKEB0000002"}"#),
        r#"{"ok":true}"#
    );
    assert_eq!(
        bus.handle_line(r#"{"fail_next":"sn:FAKEB0000002"}"#),
        r#"{"ok":true}"#
    );
    assert_eq!(
        bus.handle_line(r#"{"unplug":"port:9-3"}"#),
        r#"{"ok":true}"#,
        "the unknown device by its port: id"
    );
    assert_eq!(bus.scan().len(), 3);
    assert!(
        bus.handle_line(r#"{"plug":"tiny2lite"}"#)
            .starts_with(r#"{"error":"unknown fake model tiny2lite"#)
    );
    assert!(
        bus.handle_line(r#"{"unplug":"sn:NOPE00000000"}"#)
            .starts_with(r#"{"error":"#)
    );
    assert!(
        bus.handle_line(r#"{"plug":"fake-a","sleep":"x"}"#)
            .starts_with(r#"{"error":"#)
    );
    assert!(
        bus.handle_line("nonsense")
            .starts_with(r#"{"error":"not JSON"#)
    );
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p obsbot-device --test fakes`
Expected: FAIL to compile, `file not found for module fake` and unresolved imports.

- [ ] **Step 4: Write the fakes**

Create `crates/obsbot-device/src/models/fake/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The fake models of fake mode (design spec 5.12): second implementations of `trait Camera`,
//! in memory, and the fake bus that plugs, unplugs, sleeps and wakes them.
//!
//! - `fake-a` has the Tiny 2 Lite's description, for client work; `support: tested`.
//! - `fake-b` is deliberately different: other enum values, no `gimbal` and no `views`, the
//!   group `beacon` that no client knows, 5 presets, two `readback: false` leaves, leaves with
//!   `asleep: ignored`, `accepted` and `unknown`, a `requires`, a readback that lags its write
//!   by [`b::LAG`], a readback that rounds down, `support: untested`.
//! - `unknown` has a known vendor id and an unregistered product id: an unsupported device.
//!
//! The fakes send nothing: their send tables are empty and they have no transport.

pub mod a;
pub mod b;
mod bus;
mod camera;

pub use bus::{FakeBus, Op};

use crate::contract::Registry;
use obsbot_proto::Description;
use std::time::Duration;

/// The vendor id of every fake device: a vendor id of the real vendor table, so that
/// `unknown` is unsupported, not ignored (5.3).
pub const VENDOR: u16 = 0x3564;
/// The product id of `unknown`; no fake model registers it.
pub const UNKNOWN_PRODUCT: u16 = 0xfa00;
/// The model keys `obsbot4linuxd --fake` accepts.
pub const KEYS: [&str; 3] = ["fake-a", "fake-b", "unknown"];
/// How long a fake device is gone after a write or action with `warning: restart`.
pub const RESTART: Duration = Duration::from_secs(1);

/// The registry of fake mode: the vendor table and the two fake models.
pub static REGISTRY: Registry = Registry {
    vendors: &[VENDOR],
    models: &[a::ENTRY, b::ENTRY],
};

/// The description of a fake model (`fake-a`, `fake-b`); `None` for any other key.
pub fn description(key: &str) -> Option<Description> {
    match key {
        "fake-a" => Some(a::description()),
        "fake-b" => Some(b::description()),
        _ => None,
    }
}
```

Create `crates/obsbot-device/src/models/fake/camera.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The in-memory camera behind both fake models. A [`Shape`] holds what differs between
//! them; [`Dev`] is one fake device, shared with the fake bus that plugs and controls it.

use crate::contract::{
    Camera, CameraError, Ctx, Group, Identity, Motion, Need, SendTable, Target, Tier,
};
use crate::models::fake::RESTART;
use crate::models::fake::bus::Op;
use obsbot_proto::validate::Refusal;
use obsbot_proto::values::{number, same};
use obsbot_proto::{Description, Params, Setting, SettingType, State, Usb, Values, Warning};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// What one fake model is.
pub(crate) struct Shape {
    pub key: &'static str,
    pub description: Description,
    pub plan: Vec<Group>,
    pub initial: Values,
    pub serial_prefix: &'static str,
    pub firmware: &'static str,
    /// The motion actions and stops; every other action is `Motion::None`.
    pub motion: &'static [(&'static str, Motion)],
    /// Groups touched in addition to a written leaf's own group, and by each action.
    pub touched: fn(Target<'_>) -> Vec<&'static str>,
    /// Leaves whose readback shows a write only after this time.
    pub lag: &'static [(&'static str, Duration)],
    /// Leaves the camera rounds down to a step, so the readback never shows some writes.
    pub floor: &'static [(&'static str, f64)],
    /// The model's actions other than `sleep`, `wake` and the generic restart and reset.
    pub act: fn(&mut Values, &str, &Params) -> Option<Params>,
    pub check: fn(&str, &Params, &State) -> Result<(), Refusal>,
}

/// One fake device.
pub(crate) struct Dev {
    pub shape: Option<&'static Shape>,
    pub serial: Option<String>,
    pub usb: Usb,
    pub video_node: String,
    pub devnum: u32,
    pub plugged: bool,
    pub restart_until: Option<Instant>,
    pub asleep: bool,
    pub fail_next: bool,
    pub delay: Duration,
    /// What the camera stores, leaf path → value.
    pub values: Values,
    /// Writes not visible yet: leaf path → (value, visible from).
    pub lagging: BTreeMap<String, (Value, Instant)>,
    pub ops: Vec<Op>,
}

impl Dev {
    pub fn id_matches(&self, id: &str) -> bool {
        match (id.strip_prefix("sn:"), id.strip_prefix("port:")) {
            (Some(serial), _) => self.serial.as_deref() == Some(serial),
            (_, Some(path)) => self.usb.path == path,
            _ => false,
        }
    }
}

pub(crate) fn lock(dev: &Mutex<Dev>) -> MutexGuard<'_, Dev> {
    dev.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A first value for every leaf with a readback: `null` where nullable, else the default,
/// the first enum value, `false`, 0 clamped into the bounds, the full range, an empty list.
pub(crate) fn initial_values(desc: &Description) -> Values {
    desc.settings
        .iter()
        .filter(|(_, s)| s.readback)
        .map(|(path, s)| (path.clone(), sample(s)))
        .collect()
}

fn clamp0(s: &Setting, component: Option<&str>) -> f64 {
    let lo = s
        .min
        .as_ref()
        .and_then(|b| b.get(component))
        .unwrap_or(f64::MIN);
    let hi = s
        .max
        .as_ref()
        .and_then(|b| b.get(component))
        .unwrap_or(f64::MAX);
    0f64.clamp(lo, hi)
}

fn sample(s: &Setting) -> Value {
    if s.nullable {
        return Value::Null;
    }
    if let Some(d) = &s.default {
        return d.clone();
    }
    match &s.kind {
        SettingType::Bool => json!(false),
        SettingType::Enum => json!(s.values[0]),
        SettingType::Number => number(clamp0(s, None)),
        SettingType::String => json!(""),
        SettingType::List => json!([]),
        SettingType::Range => {
            let lo = s
                .min
                .as_ref()
                .and_then(|b| b.get(Some("min")))
                .unwrap_or(0.0);
            let hi = s
                .max
                .as_ref()
                .and_then(|b| b.get(Some("max")))
                .unwrap_or(lo);
            json!({"min": number(lo), "max": number(hi)})
        }
        kind => {
            let mut m = Map::new();
            for c in kind.components().unwrap_or(&[]) {
                m.insert(c.to_string(), number(clamp0(s, Some(c))));
            }
            Value::Object(m)
        }
    }
}

/// (group name, tier, readable asleep, live topic and its faster tier).
pub(crate) type Rule<'a> = (&'a str, Tier, bool, Option<(&'a str, Tier)>);

/// Builds a poll plan: one group per first path segment, `status` for `asleep`. `rules`
/// gives (segment, tier, readable asleep, live topic and its tier); other segments get
/// `default`. Leaves without a readback are in no group.
pub(crate) fn plan_by_segment(desc: &Description, rules: &[Rule<'_>], default: Tier) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for path in desc
        .settings
        .iter()
        .filter(|(_, s)| s.readback)
        .map(|(p, _)| p)
    {
        let segment = path.split('.').next().unwrap_or(path);
        let name = if segment == "asleep" {
            "status"
        } else {
            segment
        };
        if let Some(g) = groups.iter_mut().find(|g| g.name == name) {
            g.leaves.push(path.clone());
            continue;
        }
        let rule = rules.iter().find(|r| r.0 == name);
        groups.push(Group {
            name: name.to_string(),
            tier: rule.map_or(default, |r| r.1),
            asleep: rule.is_some_and(|r| r.2),
            live: rule
                .and_then(|r| r.3)
                .map(|(topic, tier)| crate::contract::LiveTier {
                    topic: topic.into(),
                    tier,
                }),
            leaves: vec![path.clone()],
        });
    }
    groups
}

/// The live pose `{pan, tilt, zoom}` from the stored values.
pub(crate) fn live_pose(values: &Values) -> Value {
    json!({
        "pan": values.get("gimbal.pan").cloned().unwrap_or(json!(0)),
        "tilt": values.get("gimbal.tilt").cloned().unwrap_or(json!(0)),
        "zoom": values.get("zoom.factor").cloned().unwrap_or(json!(1)),
    })
}

/// `conflict` unless `list` was read and has an item whose `key` equals `value`.
pub(crate) fn needs_item(
    state: &State,
    list: &str,
    key: &str,
    value: Option<&Value>,
) -> Result<(), Refusal> {
    if state.unread.iter().any(|p| p == list) {
        return Err(Refusal::new(
            obsbot_proto::ErrorCode::Conflict,
            list,
            &format!("{list} was not read yet"),
        ));
    }
    let Some(value) = value else { return Ok(()) };
    let found = state
        .at(list)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|i| i.get(key).is_some_and(|v| same(v, value)))
        });
    if found {
        Ok(())
    } else {
        Err(Refusal::new(
            obsbot_proto::ErrorCode::Conflict,
            key,
            &format!("no item of {list} has {key} {value}"),
        ))
    }
}

pub(crate) struct FakeCamera {
    pub shape: &'static Shape,
    pub dev: Arc<Mutex<Dev>>,
    pub devnum: u32,
}

impl FakeCamera {
    fn gone(&self, d: &Dev) -> bool {
        !d.plugged || d.restart_until.is_some() || d.devnum != self.devnum
    }

    /// Records the operation, waits the model's delay (cut by `ctx` like a frame boundary),
    /// then fails once if `fail_next` is set.
    fn begin(&self, op: Op, ctx: &Ctx) -> Result<MutexGuard<'_, Dev>, CameraError> {
        let delay = {
            let mut d = lock(&self.dev);
            if self.gone(&d) {
                return Err(CameraError::Gone);
            }
            d.ops.push(op);
            d.delay
        };
        let end = Instant::now() + delay;
        loop {
            ctx.check()?;
            let now = Instant::now();
            if now >= end {
                break;
            }
            std::thread::sleep((end - now).min(Duration::from_millis(10)));
        }
        let mut d = lock(&self.dev);
        if self.gone(&d) {
            return Err(CameraError::Gone);
        }
        if d.fail_next {
            d.fail_next = false;
            return Err(CameraError::NoAnswer(
                "the next operation (fail_next)".into(),
            ));
        }
        Ok(d)
    }

    fn restart(&self, d: &mut Dev) {
        d.restart_until = Some(Instant::now() + RESTART);
        d.lagging.clear();
    }
}

impl Camera for FakeCamera {
    fn description(&self) -> &Description {
        &self.shape.description
    }

    fn identity(&mut self, need: Need, ctx: &Ctx) -> Result<Identity, CameraError> {
        let d = self.begin(Op::Identity(need), ctx)?;
        Ok(Identity {
            serial: if need.serial { d.serial.clone() } else { None },
            firmware: need.firmware.then(|| self.shape.firmware.to_string()),
        })
    }

    fn poll_plan(&self) -> &[Group] {
        &self.shape.plan
    }

    fn read(&mut self, group: &str, ctx: &Ctx) -> Result<Values, CameraError> {
        let mut d = self.begin(Op::Read(group.to_string()), ctx)?;
        let Some(g) = self.shape.plan.iter().find(|g| g.name == group) else {
            return Err(CameraError::NoAnswer(format!("the unknown group {group}")));
        };
        if d.asleep && !g.asleep {
            return Err(CameraError::NoAnswer(format!("group {group} while asleep")));
        }
        let now = Instant::now();
        let due: Vec<String> = d
            .lagging
            .iter()
            .filter(|(_, (_, at))| *at <= now)
            .map(|(k, _)| k.clone())
            .collect();
        for leaf in due {
            if let Some((value, _)) = d.lagging.remove(&leaf) {
                d.values.insert(leaf, value);
            }
        }
        let asleep = d.asleep;
        Ok(g.leaves
            .iter()
            .map(|leaf| {
                let value = if leaf == "asleep" {
                    json!(asleep)
                } else {
                    d.values.get(leaf).cloned().unwrap_or(Value::Null)
                };
                (leaf.clone(), value)
            })
            .collect())
    }

    fn order(&self, leaves: &[&str]) -> Vec<String> {
        let required_later = |leaf: &str| {
            let reqs = &self
                .shape
                .description
                .settings
                .get(leaf)
                .map(|s| s.requires.clone())
                .unwrap_or_default();
            reqs.iter().any(|r| leaves.contains(&r.path.as_str()))
        };
        let mut out: Vec<String> = leaves.iter().map(|l| l.to_string()).collect();
        out.sort_by_key(|l| required_later(l));
        out
    }

    fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<(), CameraError> {
        let mut d = self.begin(Op::Apply(leaf.to_string(), value.clone()), ctx)?;
        let mut stored = value.clone();
        if let Some((_, step)) = self.shape.floor.iter().find(|(l, _)| *l == leaf)
            && let Some(x) = value.as_f64()
        {
            stored = number((x / step).floor() * step);
        }
        match self.shape.lag.iter().find(|(l, _)| *l == leaf) {
            Some((_, lag)) => {
                d.lagging
                    .insert(leaf.to_string(), (stored, Instant::now() + *lag));
            }
            None => {
                d.values.insert(leaf.to_string(), stored);
            }
        }
        let restarts = self
            .shape
            .description
            .settings
            .get(leaf)
            .is_some_and(|s| s.warning.contains(&Warning::Restart));
        if restarts {
            self.restart(&mut d);
        }
        Ok(())
    }

    fn check(&self, action: &str, params: &Params, state: &State) -> Result<(), Refusal> {
        (self.shape.check)(action, params, state)
    }

    fn act(
        &mut self,
        action: &str,
        params: &Params,
        _state: &State,
        ctx: &Ctx,
    ) -> Result<Option<Params>, CameraError> {
        let mut d = self.begin(Op::Act(action.to_string(), params.clone()), ctx)?;
        let warning = self
            .shape
            .description
            .actions
            .get(action)
            .map(|a| a.warning.clone())
            .unwrap_or_default();
        let result = match action {
            "sleep" => {
                d.asleep = true;
                None
            }
            "wake" => {
                d.asleep = false;
                None
            }
            _ => (self.shape.act)(&mut d.values, action, params),
        };
        if warning.contains(&Warning::Destructive) {
            d.values = self.shape.initial.clone();
        }
        if warning.contains(&Warning::Restart) {
            self.restart(&mut d);
        }
        Ok(result)
    }

    fn touched(&self, target: Target<'_>) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Target::Leaf(leaf) = target
            && let Some(g) = self
                .shape
                .plan
                .iter()
                .find(|g| g.leaves.iter().any(|l| l == leaf))
        {
            out.push(g.name.clone());
        }
        for g in (self.shape.touched)(target) {
            if !out.iter().any(|o| o == g) {
                out.push(g.to_string());
            }
        }
        out
    }

    fn motion(&self, action: &str) -> Motion {
        self.shape
            .motion
            .iter()
            .find(|(a, _)| *a == action)
            .map_or(Motion::None, |(_, m)| *m)
    }

    fn send_table(&self) -> &SendTable {
        static EMPTY: SendTable = SendTable {
            entries: Vec::new(),
        };
        &EMPTY
    }
}
```

Create `crates/obsbot-device/src/models/fake/a.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `fake-a`: the Tiny 2 Lite's description in memory, for client work (5.12).

use crate::contract::{IdentitySource, ModelEntry, Motion, Target, Tier, UsbId};
use crate::models::fake::camera::{Shape, initial_values, live_pose, needs_item, plan_by_segment};
use crate::models::fake::{UNKNOWN_PRODUCT, VENDOR};
use crate::models::tiny2lite;
use obsbot_proto::validate::Refusal;
use obsbot_proto::{Description, ErrorCode, Params, State, Support, Values};
use serde_json::{Value, json};
use std::sync::OnceLock;

pub const PRODUCT: u16 = 0xfa0a;
const _: () = assert!(PRODUCT != UNKNOWN_PRODUCT);

pub const ENTRY: ModelEntry = ModelEntry {
    key: "fake-a",
    display_name: "Fake A",
    usb: &[UsbId {
        vendor: VENDOR,
        product: PRODUCT,
    }],
    support: Support::Tested,
    identity: IdentitySource::Query,
    firmware_query: true,
    identity_tier: Tier::FiveSeconds,
    xu_guid: None,
    selectors: &[],
};

pub fn description() -> Description {
    tiny2lite::description::description()
}

fn touched(target: Target<'_>) -> Vec<&'static str> {
    match target {
        Target::Leaf("tracking.mode") => vec!["framing"],
        Target::Leaf(l) if l.starts_with("gestures.") => vec!["zoom"],
        Target::Leaf(_) => vec![],
        Target::Action(a) => match a {
            "gimbal.move" | "gimbal.turn" => vec!["tracking", "framing"],
            "preset.recall" | "view.go" => vec!["zoom", "gimbal"],
            "preset.save" | "preset.rename" | "preset.delete" => vec!["presets"],
            "power_on.use_preset" => vec!["presets", "power_on"],
            "power_on.set_pose" | "power_on.use_current" => vec!["power_on"],
            "view.save" | "view.rename" | "view.delete" => vec!["views"],
            "zone.return_here" => vec!["zone"],
            "hand.return_here" | "hand.return_clear" | "hand.area_reset" => vec!["hand"],
            "focus.point" => vec!["focus"],
            "sleep" | "wake" => vec!["status"],
            _ => vec![],
        },
    }
}

/// The Tiny 2 Lite's motion actions (4.1 rule 6) and stops (4.1 rule 2).
const MOTION: &[(&str, Motion)] = &[
    ("gimbal.move", Motion::Move),
    (
        "gimbal.turn",
        Motion::Continuing {
            stop: "gimbal.stop",
        },
    ),
    ("zoom.move", Motion::Move),
    ("preset.recall", Motion::Move),
    ("view.go", Motion::Move),
    ("zone.go_return", Motion::Move),
    ("hand.go_return", Motion::Move),
    ("power_on.go", Motion::Move),
    ("gimbal.stop", Motion::Stop),
    ("zoom.stop", Motion::Stop),
];

fn set_pose(values: &mut Values, pose: &Value) {
    values.insert("gimbal.pan".into(), pose["pan"].clone());
    values.insert("gimbal.tilt".into(), pose["tilt"].clone());
    values.insert("zoom.factor".into(), pose["zoom"].clone());
}

fn list(values: &mut Values, path: &str) -> Vec<Value> {
    values
        .get(path)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn act(values: &mut Values, action: &str, p: &Params) -> Option<Params> {
    let pose = |values: &Values| p.get("pose").cloned().unwrap_or_else(|| live_pose(values));
    match action {
        "gimbal.move" => {
            for (param, leaf) in [("pan", "gimbal.pan"), ("tilt", "gimbal.tilt")] {
                if let Some(v) = p.get(param) {
                    values.insert(leaf.into(), v.clone());
                }
            }
            values.insert("tracking.mode".into(), json!("static"));
        }
        "gimbal.turn" => {
            values.insert("tracking.mode".into(), json!("static"));
        }
        "zoom.move" => {
            values.insert("zoom.factor".into(), p["factor"].clone());
        }
        "preset.save" => {
            let slot = p["slot"].as_u64().unwrap_or(0);
            let name = p
                .get("name")
                .cloned()
                .unwrap_or_else(|| json!(format!("Preset {}", slot + 1)));
            let item = json!({"slot": slot, "name": name, "pose": pose(values)});
            let mut items: Vec<Value> = list(values, "presets")
                .into_iter()
                .filter(|i| i["slot"] != json!(slot))
                .collect();
            items.push(item);
            items.sort_by_key(|i| i["slot"].as_u64());
            values.insert("presets".into(), Value::Array(items));
        }
        "preset.recall" | "view.go" => {
            let (path, key, wanted) = if action == "view.go" {
                ("views", "id", &p["id"])
            } else {
                ("presets", "slot", &p["slot"])
            };
            if let Some(item) = list(values, path).into_iter().find(|i| &i[key] == wanted) {
                set_pose(values, &item["pose"]);
            }
        }
        "preset.rename" | "view.rename" | "preset.delete" | "view.delete" => {
            let (path, key) = if action.starts_with("view.") {
                ("views", "id")
            } else {
                ("presets", "slot")
            };
            let wanted = &p[key];
            let mut items = list(values, path);
            if action.ends_with(".delete") {
                items.retain(|i| &i[key] != wanted);
            } else if let Some(item) = items.iter_mut().find(|i| &i[key] == wanted) {
                item["name"] = p["name"].clone();
            }
            values.insert(path.into(), Value::Array(items));
        }
        "view.save" => {
            let mut items = list(values, "views");
            let id = (0u64..)
                .find(|n| !items.iter().any(|i| i["id"] == json!(n)))
                .unwrap_or(0);
            items.push(json!({"id": id, "name": p["name"], "pose": pose(values)}));
            values.insert("views".into(), Value::Array(items));
            return Some(json!({"id": id}).as_object().cloned().unwrap_or_default());
        }
        "zone.return_here" => {
            values.insert("zone.return".into(), live_pose(values));
        }
        "hand.return_here" => {
            values.insert("hand.return".into(), live_pose(values));
        }
        "hand.return_clear" => {
            values.insert("hand.return".into(), Value::Null);
        }
        "hand.area_reset" => {
            values.insert("hand.area.pan".into(), json!({"min": -45, "max": 45}));
            values.insert("hand.area.tilt".into(), json!({"min": -30, "max": 30}));
        }
        "focus.point" => {
            values.insert("focus.mode".into(), json!("single"));
        }
        "power_on.set_pose" | "power_on.use_current" => {
            values.insert("power_on.start.pose".into(), pose(values));
            values.insert("power_on.start.source".into(), json!("own"));
        }
        "power_on.use_preset" => {
            let slot = p["slot"].clone();
            if let Some(item) = list(values, "presets")
                .into_iter()
                .find(|i| i["slot"] == slot)
            {
                values.insert("power_on.start.pose".into(), item["pose"].clone());
                values.insert(
                    "power_on.start.source".into(),
                    json!(format!("preset_{slot}")),
                );
            }
        }
        _ => {}
    }
    None
}

fn check(action: &str, p: &Params, state: &State) -> Result<(), Refusal> {
    match action {
        "gimbal.move" if !p.contains_key("pan") && !p.contains_key("tilt") => Err(Refusal::new(
            ErrorCode::Invalid,
            "gimbal.move",
            "gimbal.move needs pan, tilt or both",
        )),
        "preset.save" => needs_item(state, "presets", "slot", None),
        "preset.recall" | "preset.rename" | "preset.delete" | "power_on.use_preset" => {
            needs_item(state, "presets", "slot", p.get("slot"))
        }
        "view.save" => needs_item(state, "views", "id", None),
        "view.go" | "view.rename" | "view.delete" => needs_item(state, "views", "id", p.get("id")),
        _ => Ok(()),
    }
}

pub(crate) fn shape() -> &'static Shape {
    static SHAPE: OnceLock<Shape> = OnceLock::new();
    SHAPE.get_or_init(|| {
        let description = description();
        let plan = plan_by_segment(
            &description,
            &[
                ("status", Tier::Second, true, None),
                ("on_air", Tier::Second, false, None),
                ("tracking", Tier::Second, false, None),
                (
                    "gimbal",
                    Tier::FiveSeconds,
                    false,
                    Some(("pose", Tier::Second)),
                ),
                ("presets", Tier::Touched, false, None),
                ("views", Tier::Minute, false, None),
                ("zone", Tier::Minute, false, None),
                ("hand", Tier::Minute, false, None),
                ("power_on", Tier::Minute, false, None),
            ],
            Tier::FiveSeconds,
        );
        let initial = initial_values(&description);
        Shape {
            key: "fake-a",
            description,
            plan,
            initial,
            serial_prefix: "FAKEA",
            firmware: "1.0.0.0",
            motion: MOTION,
            touched,
            lag: &[],
            floor: &[],
            act,
            check,
        }
    })
}
```

Create `crates/obsbot-device/src/models/fake/b.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `fake-b`: a model deliberately unlike the Tiny 2 Lite (5.12), so that the daemon, the CLI
//! and the widget are built against two shapes from the start.

use crate::contract::{IdentitySource, ModelEntry, Motion, Target, Tier, UsbId};
use crate::models::fake::camera::{Shape, initial_values, needs_item, plan_by_segment};
use crate::models::fake::{UNKNOWN_PRODUCT, VENDOR};
use obsbot_proto::validate::Refusal;
use obsbot_proto::{Action, Asleep, Description, Params, Setting, State, Support, Unit, Values};
use serde_json::{Value, json};
use std::sync::OnceLock;
use std::time::Duration;

pub const PRODUCT: u16 = 0xfa0b;
const _: () = assert!(PRODUCT != UNKNOWN_PRODUCT);

/// How long the readback of `led.brightness` lags a write.
pub const LAG: Duration = Duration::from_millis(250);
/// The settle bound of every writable leaf with a readback.
pub const SETTLE_MS: u32 = 1000;
/// The camera stores `exposure.time` rounded down to this step.
pub const TIME_STEP: f64 = 100.0;

pub const ENTRY: ModelEntry = ModelEntry {
    key: "fake-b",
    display_name: "Fake B",
    usb: &[UsbId {
        vendor: VENDOR,
        product: PRODUCT,
    }],
    support: Support::Untested,
    identity: IdentitySource::Query,
    firmware_query: true,
    identity_tier: Tier::FiveSeconds,
    xu_guid: None,
    selectors: &[],
};

fn w(s: Setting, asleep: Asleep) -> Setting {
    s.asleep(asleep).settle_ms(SETTLE_MS)
}

pub fn description() -> Description {
    Description::default()
        .setting("asleep", Setting::boolean("Asleep").readonly())
        .setting("on_air.active", Setting::boolean("On air").readonly())
        .setting(
            "on_air.apps",
            Setting::list(
                "Applications using the camera",
                &[("name", Setting::string("Process name", 255, false))],
            ),
        )
        .setting(
            "tracking.mode",
            w(
                Setting::enumeration("Tracking", &["off", "person", "desk"]),
                Asleep::Unknown,
            ),
        )
        .setting(
            "zoom.factor",
            w(
                Setting::number("Zoom", 1.0, 2.0)
                    .unit(Unit::Factor)
                    .precision(0.01),
                Asleep::Ignored,
            ),
        )
        .setting(
            "led.brightness",
            w(
                Setting::number("LED brightness", 0.0, 5.0)
                    .step(1.0)
                    .precision(1.0),
                Asleep::Unknown,
            ),
        )
        .setting(
            "led.blink",
            Setting::boolean("LED blink")
                .asleep(Asleep::Accepted)
                .no_readback(),
        )
        .setting(
            "exposure.mode",
            w(
                Setting::enumeration("Exposure", &["auto", "manual"]),
                Asleep::Unknown,
            ),
        )
        .setting(
            "exposure.time",
            w(
                Setting::number("Exposure time", 100.0, 10_000.0)
                    .unit(Unit::Microseconds)
                    .precision(1.0),
                Asleep::Unknown,
            )
            .requires("exposure.mode", json!("manual"))
            .live("auto"),
        )
        .setting(
            "beacon.color",
            w(
                Setting::enumeration("Beacon colour", &["red", "green", "blue"]),
                Asleep::Accepted,
            ),
        )
        .setting(
            "beacon.pulse",
            Setting::boolean("Beacon pulse")
                .asleep(Asleep::Unknown)
                .no_readback(),
        )
        .setting(
            "presets",
            Setting::list(
                "Presets",
                &[
                    ("slot", Setting::number("Preset slot", 0.0, 4.0).step(1.0)),
                    ("name", Setting::string("Name", 16, true)),
                ],
            ),
        )
        .action(
            "preset.save",
            Action::new("Save a preset", Asleep::Ignored)
                .param("slot", Setting::number("Preset slot", 0.0, 4.0).step(1.0))
                .param("name", Setting::string("Name", 16, true).optional()),
        )
        .action(
            "preset.recall",
            Action::new("Recall a preset", Asleep::Unknown)
                .param("slot", Setting::number("Preset slot", 0.0, 4.0).step(1.0)),
        )
        .action(
            "preset.delete",
            Action::new("Delete a preset", Asleep::Unknown)
                .param("slot", Setting::number("Preset slot", 0.0, 4.0).step(1.0)),
        )
        .action(
            "beacon.flash",
            Action::new("Flash the beacon", Asleep::Accepted)
                .param("times", Setting::number("Times", 1.0, 3.0).step(1.0)),
        )
        .action("sleep", Action::new("Sleep", Asleep::Unknown))
        .action("wake", Action::new("Wake", Asleep::Accepted))
        .limit("presets", 5)
}

fn touched(target: Target<'_>) -> Vec<&'static str> {
    match target {
        Target::Leaf(_) => vec![],
        Target::Action("preset.save" | "preset.delete") => vec!["presets"],
        Target::Action("sleep" | "wake") => vec!["status"],
        Target::Action(_) => vec![],
    }
}

fn act(values: &mut Values, action: &str, p: &Params) -> Option<Params> {
    let mut items: Vec<Value> = values
        .get("presets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    match action {
        "preset.save" => {
            let slot = p["slot"].clone();
            let name = p
                .get("name")
                .cloned()
                .unwrap_or_else(|| json!(format!("Preset {}", slot.as_u64().unwrap_or(0) + 1)));
            items.retain(|i| i["slot"] != slot);
            items.push(json!({"slot": slot, "name": name}));
            items.sort_by_key(|i| i["slot"].as_u64());
        }
        "preset.delete" => items.retain(|i| i["slot"] != p["slot"]),
        _ => return None,
    }
    values.insert("presets".into(), Value::Array(items));
    None
}

fn check(action: &str, p: &Params, state: &State) -> Result<(), Refusal> {
    match action {
        "preset.save" => needs_item(state, "presets", "slot", None),
        "preset.recall" | "preset.delete" => needs_item(state, "presets", "slot", p.get("slot")),
        _ => Ok(()),
    }
}

pub(crate) fn shape() -> &'static Shape {
    static SHAPE: OnceLock<Shape> = OnceLock::new();
    SHAPE.get_or_init(|| {
        let description = description();
        let plan = plan_by_segment(
            &description,
            &[
                ("status", Tier::Second, true, None),
                ("on_air", Tier::Second, false, None),
                ("led", Tier::Second, false, None),
                ("exposure", Tier::Second, false, None),
                ("beacon", Tier::Minute, false, None),
                ("presets", Tier::Touched, false, None),
            ],
            Tier::FiveSeconds,
        );
        let initial = initial_values(&description);
        Shape {
            key: "fake-b",
            description,
            plan,
            initial,
            serial_prefix: "FAKEB",
            firmware: "0.9.1",
            motion: &[("preset.recall", Motion::Move)],
            touched,
            lag: &[("led.brightness", LAG)],
            floor: &[("exposure.time", TIME_STEP)],
            act,
            check,
        }
    })
}
```

Create `crates/obsbot-device/src/models/fake/bus.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The fake bus of fake mode (5.12): the devices `obsbot4linuxd --fake` sees instead of the
//! sysfs scan, and the test control that plugs, unplugs, sleeps, wakes and fails them.

use crate::contract::{Bus, Camera, CameraError, Found, Lookup, Need, UsbId};
use crate::models::fake::camera::{Dev, FakeCamera, Shape, lock};
use crate::models::fake::{KEYS, REGISTRY, UNKNOWN_PRODUCT, VENDOR, a, b};
use obsbot_proto::{Params, Usb};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// One camera operation on a fake device, in the order they happened. Tests read them with
/// [`FakeBus::ops`]; the unsupported device must never have any.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Open,
    Identity(Need),
    Read(String),
    Apply(String, Value),
    Act(String, Params),
}

struct Inner {
    devices: Vec<Arc<Mutex<Dev>>>,
    next_devnum: u32,
    next_port: u32,
    serials: BTreeMap<&'static str, u32>,
    delays: BTreeMap<String, Duration>,
}

/// The fake devices of one daemon. All methods are safe to call from any thread.
pub struct FakeBus {
    inner: Mutex<Inner>,
}

fn shape_of(model: &str) -> Option<&'static Shape> {
    match model {
        "fake-a" => Some(a::shape()),
        "fake-b" => Some(b::shape()),
        _ => None,
    }
}

impl FakeBus {
    /// A bus with one device per key, plugged in this order (`fake-a`, `fake-b`, `unknown`;
    /// a key may repeat). Any other key is an error that names it.
    pub fn new(models: &[&str]) -> Result<FakeBus, String> {
        let bus = FakeBus {
            inner: Mutex::new(Inner {
                devices: Vec::new(),
                next_devnum: 1,
                next_port: 1,
                serials: BTreeMap::new(),
                delays: BTreeMap::new(),
            }),
        };
        for model in models {
            bus.plug(model)?;
        }
        Ok(bus)
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn find(&self, id: &str) -> Result<Arc<Mutex<Dev>>, String> {
        self.inner()
            .devices
            .iter()
            .find(|d| lock(d).id_matches(id))
            .cloned()
            .ok_or_else(|| format!("no fake device {id}"))
    }

    /// Plugs a device of `model`: the first unplugged device of that model comes back (same
    /// serial, same USB path, a new device number), else a new one is made. Returns its id
    /// as the daemon will show it once the identity is read.
    pub fn plug(&self, model: &str) -> Result<String, String> {
        if !KEYS.contains(&model) {
            return Err(format!(
                "unknown fake model {model}; known: {}",
                KEYS.join(", ")
            ));
        }
        let mut inner = self.inner();
        let devnum = inner.next_devnum;
        inner.next_devnum += 1;
        let shape = shape_of(model);
        let back = inner.devices.iter().find(|d| {
            let d = lock(d);
            !d.plugged && d.shape.map(|s| s.key) == shape.map(|s| s.key)
        });
        if let Some(dev) = back.cloned() {
            let mut d = lock(&dev);
            d.plugged = true;
            d.devnum = devnum;
            d.asleep = false;
            return Ok(id_of(&d));
        }
        let port = inner.next_port;
        inner.next_port += 1;
        let serial = shape.map(|s| {
            let n = inner.serials.entry(s.key).or_insert(0);
            *n += 1;
            format!("{}{:07}", s.serial_prefix, n)
        });
        let (product, name, interfaces) = match model {
            "fake-a" => (a::PRODUCT, "Fake A", vec!["0e", "0e", "01", "01"]),
            "fake-b" => (b::PRODUCT, "Fake B", vec!["0e", "0e"]),
            _ => (UNKNOWN_PRODUCT, "Unknown Camera", vec!["0e", "0e"]),
        };
        let usb = Usb {
            vendor: format!("{VENDOR:04x}"),
            product: format!("{product:04x}"),
            product_name: Some(name.to_string()),
            bcd_device: "0100".into(),
            path: format!("9-{port}"),
            serial_present: false,
            interfaces: interfaces.into_iter().map(String::from).collect(),
            extension_units: Vec::new(),
        };
        let dev = Dev {
            shape,
            serial,
            usb,
            video_node: format!("/dev/video{}", 100 + 2 * port),
            devnum,
            plugged: true,
            restart_until: None,
            asleep: false,
            fail_next: false,
            delay: inner.delays.get(model).copied().unwrap_or_default(),
            values: shape.map(|s| s.initial.clone()).unwrap_or_default(),
            lagging: BTreeMap::new(),
            ops: Vec::new(),
        };
        let id = id_of(&dev);
        inner.devices.push(Arc::new(Mutex::new(dev)));
        Ok(id)
    }

    /// Unplugs the device with this id (`sn:` or `port:`). The camera keeps its values.
    pub fn unplug(&self, id: &str) -> Result<(), String> {
        let dev = self.find(id)?;
        let mut d = lock(&dev);
        d.plugged = false;
        d.restart_until = None;
        d.lagging.clear();
        Ok(())
    }

    /// Puts the device to sleep, as the camera's own auto-sleep would.
    pub fn sleep(&self, id: &str) -> Result<(), String> {
        lock(&*self.find(id)?).asleep = true;
        Ok(())
    }

    pub fn wake(&self, id: &str) -> Result<(), String> {
        lock(&*self.find(id)?).asleep = false;
        Ok(())
    }

    /// The next camera operation on this device fails with `NoAnswer`, an identity attempt
    /// included. The flag survives an unplug, so a re-plug can start with a failed identity.
    pub fn fail_next(&self, id: &str) -> Result<(), String> {
        lock(&*self.find(id)?).fail_next = true;
        Ok(())
    }

    /// Every camera operation on devices of `model`, plugged now or later, takes this long
    /// (cut by the operation's context). Not on the test-control socket: for in-process tests.
    pub fn set_delay(&self, model: &str, delay: Duration) {
        let mut inner = self.inner();
        inner.delays.insert(model.to_string(), delay);
        for dev in &inner.devices {
            let mut d = lock(dev);
            if d.shape.is_some_and(|s| s.key == model) {
                d.delay = delay;
            }
        }
    }

    /// The operations sent to the device with this id, oldest first.
    pub fn ops(&self, id: &str) -> Vec<Op> {
        self.find(id)
            .map(|d| lock(&d).ops.clone())
            .unwrap_or_default()
    }

    /// One line of the test-control socket (`{"plug":"fake-a"}`, `{"unplug":"<id>"}`,
    /// `{"sleep":"<id>"}`, `{"wake":"<id>"}`, `{"fail_next":"<id>"}`) → the reply line:
    /// `{"ok":"<id>"}` for `plug`, `{"ok":true}` for the others, `{"error":"<message>"}`.
    pub fn handle_line(&self, line: &str) -> String {
        let reply = match serde_json::from_str::<Value>(line) {
            Err(e) => Err(format!("not JSON: {e}")),
            Ok(Value::Object(map)) if map.len() == 1 => {
                let (verb, arg) = map.into_iter().next().expect("one entry");
                match (verb.as_str(), arg.as_str()) {
                    ("plug", Some(m)) => self.plug(m).map(|id| json!(id)),
                    ("unplug", Some(id)) => self.unplug(id).map(|_| json!(true)),
                    ("sleep", Some(id)) => self.sleep(id).map(|_| json!(true)),
                    ("wake", Some(id)) => self.wake(id).map(|_| json!(true)),
                    ("fail_next", Some(id)) => self.fail_next(id).map(|_| json!(true)),
                    _ => Err(format!("unknown control {verb}")),
                }
            }
            Ok(_) => Err("a control line is one object with one key".into()),
        };
        match reply {
            Ok(v) => json!({ "ok": v }).to_string(),
            Err(e) => json!({ "error": e }).to_string(),
        }
    }
}

fn id_of(d: &Dev) -> String {
    match &d.serial {
        Some(serial) => obsbot_proto::sn_id(serial),
        None => obsbot_proto::port_id(&d.usb.path),
    }
}

impl Bus for FakeBus {
    fn scan(&self) -> Vec<Found> {
        let mut inner = self.inner();
        let now = Instant::now();
        let mut found = Vec::new();
        let devices = inner.devices.clone();
        for dev in devices {
            let mut d = lock(&dev);
            if !d.plugged {
                continue;
            }
            if let Some(until) = d.restart_until {
                if now < until {
                    continue;
                }
                d.restart_until = None;
                d.devnum = inner.next_devnum;
                inner.next_devnum += 1;
            }
            let model = match UsbId::from_usb(&d.usb).map(|id| REGISTRY.lookup(id)) {
                Some(Lookup::Model(entry)) => Some(entry),
                Some(Lookup::Unsupported) => None,
                Some(Lookup::NotListed) | None => continue,
            };
            found.push(Found {
                usb: d.usb.clone(),
                devnum: d.devnum,
                video_node: Some(d.video_node.clone()),
                model,
            });
        }
        found
    }

    fn open(&self, found: &Found) -> Result<Box<dyn Camera>, CameraError> {
        let inner = self.inner();
        for dev in &inner.devices {
            let mut d = lock(dev);
            if d.usb.path == found.usb.path
                && d.plugged
                && d.restart_until.is_none()
                && d.devnum == found.devnum
            {
                let Some(shape) = d.shape else {
                    return Err(CameraError::Io("not a supported fake model".into()));
                };
                d.ops.push(Op::Open);
                let devnum = d.devnum;
                drop(d);
                return Ok(Box::new(FakeCamera {
                    shape,
                    dev: dev.clone(),
                    devnum,
                }));
            }
        }
        Err(CameraError::Gone)
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test fakes`
Expected: PASS: `test result: ok. 18 passed` (about 1 s: the restart and lag tests sleep).

Run: `cargo fmt --all --check && cargo clippy -p obsbot-device --all-targets --locked -- -D warnings`
Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot-device/src/models/mod.rs crates/obsbot-device/src/models/fake crates/obsbot-device/tests/fakes.rs
git commit -m "obsbot-device: fake-a, fake-b, unknown and the fake bus with test control"
```

### Task 10: Generated description files

**Files:**

- Create: `crates/obsbot-device/src/descriptions.rs`, `scripts/regenerate.sh`
- Modify: `crates/obsbot-device/src/lib.rs` (one line)
- Create (generated): `descriptions/tiny2lite.json`, `descriptions/fake/fake-a.json`, `descriptions/fake/fake-b.json`
- Test: `crates/obsbot-device/tests/generated.rs`

**Interfaces:**

- Consumes (Task 8): `tiny2lite::description::description`; (Task 9): `fake::a::description`, `fake::b::description`; (Task 2): `check_description`; (Task 3): the `OBSBOT_REGENERATE` convention and `crates/obsbot-proto/tests/schema.rs`.
- Produces: `obsbot_device::descriptions::{all() -> Vec<(&'static str, Description)>, render(&Description) -> String, compare(&str, &Description, &str) -> Result<(), String>}`; the three description files; `scripts/regenerate.sh` (track C adds the completions to it, spec 6.2). A new model adds its line to `all()`.

- [ ] **Step 1: Add the module**

In `crates/obsbot-device/src/lib.rs`, add:

```rust
pub mod descriptions;
```

- [ ] **Step 2: Write the failing test**

Create `crates/obsbot-device/tests/generated.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Stale test of the generated description files. `scripts/regenerate.sh` sets
//! `OBSBOT_REGENERATE=1`, which writes them instead.

use obsbot_device::descriptions::{all, compare, render};
use obsbot_proto::Description;
use obsbot_proto::validate::check_description;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn description_files_are_current() {
    let regenerate = std::env::var_os("OBSBOT_REGENERATE").is_some();
    let mut stale = Vec::new();
    for (path, description) in all() {
        let file = root().join(path);
        if regenerate {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, render(&description)).unwrap();
            continue;
        }
        let have = std::fs::read_to_string(&file).unwrap_or_default();
        if let Err(e) = compare(path, &description, &have) {
            stale.push(e);
        }
    }
    assert!(stale.is_empty(), "{}", stale.join("\n"));
}

#[test]
fn a_changed_description_is_reported_stale() {
    let (path, description) = all().remove(0);
    let file = render(&description);
    let mut changed = description.clone();
    changed.settings.get_mut("led.brightness").unwrap().label = "LED level".into();
    assert!(compare(path, &description, &file).is_ok());
    assert!(
        compare(path, &changed, &file)
            .unwrap_err()
            .contains("scripts/regenerate.sh")
    );
}

#[test]
fn every_description_file_round_trips_and_is_consistent() {
    for (path, description) in all() {
        assert_eq!(
            check_description(&description),
            Vec::<String>::new(),
            "{path}"
        );
        let text = render(&description);
        let parsed: Description = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed, description, "{path}");
        assert_eq!(render(&parsed), text, "{path}: not byte-equal");
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p obsbot-device --test generated`
Expected: FAIL to compile, `file not found for module descriptions`.

- [ ] **Step 4: Write the generator and the script**

Create `crates/obsbot-device/src/descriptions.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The generated description files (design spec 3): `descriptions/<model>.json` for every
//! real model and `descriptions/fake/<model>.json` for the fakes. `tests/generated.rs` fails
//! when a file differs from the code; `scripts/regenerate.sh` rewrites them. A new model adds
//! its line to [`all`].

use crate::models::{fake, tiny2lite};
use obsbot_proto::Description;

/// (file path relative to the repository root, description), in a fixed order.
pub fn all() -> Vec<(&'static str, Description)> {
    vec![
        (
            "descriptions/tiny2lite.json",
            tiny2lite::description::description(),
        ),
        ("descriptions/fake/fake-a.json", fake::a::description()),
        ("descriptions/fake/fake-b.json", fake::b::description()),
    ]
}

/// The file content of one description: pretty JSON with a final newline.
pub fn render(description: &Description) -> String {
    let mut text =
        serde_json::to_string_pretty(description).expect("a description always serialises");
    text.push('\n');
    text
}

/// `Ok` when `have` (the file content) equals the rendered description, else the reason.
pub fn compare(path: &str, description: &Description, have: &str) -> Result<(), String> {
    if have == render(description) {
        Ok(())
    } else {
        Err(format!(
            "{path} is stale: run scripts/regenerate.sh and commit the result"
        ))
    }
}
```

Create `scripts/regenerate.sh` and make it executable (`chmod +x scripts/regenerate.sh`):

```sh
#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Regenerates the generated files from the code (design spec 3): schema/protocol-2.json and
# descriptions/. Run it after changing obsbot-proto types or a model's description, and
# commit the result. The stale tests fail until then.
set -eu
cd "$(dirname "$0")/.."
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot-proto --test schema
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot-device --test generated
git status --short schema descriptions
```

- [ ] **Step 5: See the stale check fail, then generate**

Run: `cargo test -p obsbot-device --test generated`
Expected: FAIL in `description_files_are_current`, naming all three files with `is stale: run scripts/regenerate.sh and commit the result` (they do not exist yet).

Run: `scripts/regenerate.sh`
Expected: both test runs pass; `git status` lists `descriptions/tiny2lite.json` (1875 lines), `descriptions/fake/fake-a.json` (the same content: fake-a uses the Tiny 2 Lite's description) and `descriptions/fake/fake-b.json` (289 lines) as new; `schema/protocol-2.json` unchanged.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p obsbot-device --test generated`
Expected: PASS: `test result: ok. 3 passed`. The test `a_changed_description_is_reported_stale` shows that the check fails when a description changes without regenerating.

- [ ] **Step 7: Commit**

```bash
git add crates/obsbot-device/src/lib.rs crates/obsbot-device/src/descriptions.rs crates/obsbot-device/tests/generated.rs scripts/regenerate.sh descriptions
git commit -m "Generated description files with a stale test; scripts/regenerate.sh"
```

### Task 11: Daemon crate stub

**Files:**

- Modify: `Cargo.toml` (root), `crates/obsbot4linux/Cargo.toml`
- Create: `crates/obsbot4linuxd/Cargo.toml`, `crates/obsbot4linuxd/src/lib.rs`, `crates/obsbot4linuxd/src/main.rs`

**Interfaces:**

- Consumes (Task 9): `obsbot_device::models::fake::FakeBus`; (Task 6) the CLI crate without 0.13 code.
- Produces: `obsbot4linuxd::Config { pub socket: PathBuf, pub fake: Option<Arc<FakeBus>>, pub stop: Arc<AtomicBool> }`, `Config::new(PathBuf) -> Config`, `obsbot4linuxd::run(Config) -> std::io::Result<()>`; the binary `obsbot4linuxd`. Track B (B-1) writes the daemon behind this entry point without changing it; track C's tests start it in-process (spec 11) through the CLI crate's `[dev-dependencies]`.

- [ ] **Step 1: Add the crate to the workspace**

In the root `Cargo.toml`, make sure `[workspace] members` includes `"crates/obsbot4linuxd"`, and add to `[workspace.dependencies]` (keep G's `obsbot-device` entry if it has one):

```toml
obsbot-device = { path = "crates/obsbot-device" }
obsbot4linuxd = { path = "crates/obsbot4linuxd" }
```

Create `crates/obsbot4linuxd/Cargo.toml`:

```toml
[package]
name = "obsbot4linuxd"
description = "The obsbot4linux daemon: owns the cameras and serves protocol 2"
version.workspace = true
edition = "2024"
rust-version.workspace = true
license.workspace = true
publish = false

[lib]
path = "src/lib.rs"

[[bin]]
name = "obsbot4linuxd"
path = "src/main.rs"

[dependencies]
obsbot-device.workspace = true
obsbot-proto.workspace = true
```

In `crates/obsbot4linux/Cargo.toml`, add:

```toml
[dev-dependencies]
obsbot4linuxd.workspace = true
```

- [ ] **Step 2: Write the stub**

Create `crates/obsbot4linuxd/src/lib.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The obsbot4linux daemon (design spec 5) as a library, so that tests run it in-process on a
//! temporary socket (section 11). Stage W0 fixes only this entry point; track B writes the
//! daemon behind it.

use obsbot_device::models::fake::FakeBus;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

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

/// Runs the daemon until `config.stop` is set or a signal ends it. Returns once the socket,
/// the lock and every worker are released.
pub fn run(config: Config) -> std::io::Result<()> {
    let _ = config;
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "obsbot4linuxd is not built yet (stage W0)",
    ))
}
```

Create `crates/obsbot4linuxd/src/main.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linuxd`: the daemon binary (design spec 5). Track B writes it.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("obsbot4linuxd: not built yet (stage W0 of the 1.0 rebuild)");
    ExitCode::FAILURE
}
```

- [ ] **Step 3: Check the whole workspace**

Run: `cargo build --workspace && scripts/check-cli-deps.sh`
Expected: the build succeeds; `CLI dependencies: obsbot-proto` (the dev-dependency on the daemon is allowed, spec 3).

Run: `cargo run -q -p obsbot4linuxd; echo "exit=$?"`
Expected: `obsbot4linuxd: not built yet (stage W0 of the 1.0 rebuild)` and `exit=1`.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked`
Expected: no fmt or clippy output; every `test result: ok`, none failed, none ignored.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock crates/obsbot4linuxd crates/obsbot4linux/Cargo.toml
git commit -m "obsbot4linuxd: crate stub with its fixed entry point run(Config)"
```

---

## W0 acceptance (spec 10.3), mapped

| Criterion                                                                                                                                                     | Where                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| the workspace builds and its tests pass                                                                                                                       | Task 11 Step 3                                                |
| the schema stale test passes                                                                                                                                  | Task 3                                                        |
| every golden file round-trips byte-equal through the proto types                                                                                              | Tasks 1 (types), 4 (transcripts), 10 (descriptions)           |
| `fake-a` and `fake-b` pass a trait conformance test                                                                                                           | Task 9 `fake_a_and_fake_b_pass_the_conformance_check`, Task 7 |
| every writable leaf with a readback in a group with a periodic tier                                                                                           | Task 7 (`WritableInTouchedGroup` row)                         |
| the `cargo metadata` check of section 3 passes                                                                                                                | Task 6                                                        |
| the four crates as compiling stubs; the 0.13 daemon, client, protocol and CLI removed (the 0.13 `state` module stays in `legacy/` until track A, decision 15) | Tasks 1, 5, 6, 11                                             |
| the Tiny 2 Lite description as data (every field of 4.3 and 4.4, U-rows included)                                                                             | Task 8                                                        |
| `descriptions/tiny2lite.json` and `descriptions/fake/*.json`                                                                                                  | Task 10                                                       |
| golden transcripts (`hello`, events, every error code, an unsupported entry, a hand-over)                                                                     | Task 4                                                        |
| the normative part of `docs/protocol-2.md`                                                                                                                    | Task 12                                                       |
| acceptance with camera: none                                                                                                                                  | no task touches a camera                                      |

## Self-review

1. **Spec coverage.** Section 3: crates, dependency direction and its CI check (Tasks 1, 5, 6, 11), `schema/` and `descriptions/` and `scripts/regenerate.sh` (3, 10). Section 4.1: every verb, reply, event and error code as a type (1), the ordering rules as transcript checks where a transcript can show them (4); the rest is daemon behaviour (track B). Section 4.2: every Device, Setting and Action field (1), the rules leaves, one meaning per path, naming, absent vs null, `unread`, `raw:<n>`, precision, identity shortening (1, 2, 12). Sections 4.3 and 4.4: every leaf and action (8), checked against the field ledger's rows. Sections 5.1 to 5.3: the enumeration seam `Bus`/`Found`/`Registry` (5) and the fakes' plug, unplug and re-enumeration (9); threads and start-up are track B. Section 5.12: the fakes and the test control (9). Section 6: the client is track C (decision 14); the CLI crate's dependency rule (6). Section 8.1 to 8.3: registry, `trait Camera`, send table types (5). Section 9.3: every row marked (8). Section 10.3 W0: the acceptance table above. Section 11: golden files and stale tests (1, 3, 4, 10). Not in W0 by the spec: daemon behaviour, the transports, the CLI commands, the widget.
2. **Placeholder scan.** Every code step holds the whole file or the exact lines; the stubs of Tasks 1, 5 and 11 are complete files whose content later tasks replace.
3. **Type consistency.** The names in every Interfaces block were compiled together in one workspace: `Refusal` lives in `obsbot_proto::validate` from Task 1 on; `Ctx`, `Motion`, `Need`, `Target`, `Group`, `Tier`, `LiveTier`, `SendTable`, `IdentitySource`, `ModelEntry` in `obsbot_device::contract`; `act` takes `&State` everywhere it is called or implemented (Tasks 5, 7, 9); `FakeBus` in `obsbot_device::models::fake`; `Config` in `obsbot4linuxd`.
4. **Review Focus.** Five uncovered input classes, each with its test in the owning task (Tasks 1 and 2).

## Execution handoff

Plan complete. Please review the plan. Does it capture what you want?
