# obsbot4linux 1.0, stage W1 track C (client and CLI) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the synchronous client module of `obsbot-proto` (spec 6.1) and the new command line `obsbot4linux` (spec 6.2) on the contracts of stage W0: a generic CLI that maps 1:1 onto protocol 2, sends exactly one request per command and chains none (spec 2), with the value syntax per setting type, human output and `--json`, device selection, the CLI-only error codes and the exit codes, `watch`, `devices --report`, and bash and zsh completions generated from the CLI definition and the embedded description files, with their stale test.

**Architecture:** `obsbot-proto/src/client.rs` plus `src/client/` (all inside the owner glob `client*`, so `lib.rs` stays W0's): a `Connection` (JSON lines, reply timeout, `hello` classification), a blocking `Client`, a subscribed `Sender`/`Receiver` pair with a reader thread, typed `ClientError`, and a doc-hidden scripted daemon that serves golden-transcript scripts to tests. The CLI crate becomes a library (`obsbot4linux::run`) with a thin `main`: `args` (clap definition), `errors` (codes, exit codes, output of errors), `value` (syntax per setting type), `select` and `session` (device choice, connecting, client errors), `render` and `describe` (human output from the description), `read`, `write` and `watch` (the commands), `completions` (generator) plus `build.rs` (embeds `descriptions/*.json`). Tests run the CLI in-process against the scripted daemon (waves 1 to 3) and against an in-process `obsbot4linuxd --fake fake-a,fake-b,unknown` (waves 4 and 5, after track B's B-1).

**Tech Stack:** Rust 2024 edition, MSRV 1.88 (CI toolchain 1.99.1), `obsbot-proto` unchanged in its dependencies (serde, serde_json, schemars); the CLI adds the workspace's `clap` 4 (derive), `nix` 0.31 with the feature `signal`, `serde`, `serde_json`; bash for the completion behaviour test, zsh for its syntax check (an ignored test).

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` (revision 2, approved 2026-11-13, with section 15.1), sections 2, 3, 4.1, 4.2, 4.4, 5.8, 5.10, 6, 8.4, 8.6, 9.1, 9.2, 10.1, 10.3 (track C), 11, 12.2, 14; the W0 plan `docs/superpowers/plans/2026-11-13-obsbot4linux-w0-contracts.md` (its code is the contract this plan builds on).

## Global Constraints

- "The CLI is generic. It maps 1:1 onto protocol 2, so a new model or setting needs no CLI change. There are no per-feature subcommands and no shortcuts." (spec 6.2)
- "A request sends exactly what it names … Clients follow the same rule: the CLI and the widget never chain a second request to the one the user made." (spec 2) "Every command sends the one request it maps to and chains no other (section 2): `do gimbal.move` and `do gimbal.turn` leave the tracking as it is, and a user who wants it off first runs `set tracking.mode=static`." (spec 6.2)
- "The `[dependencies]` of the CLI crate never contain `obsbot-device` or `obsbot4linuxd`; a CI step checks this with `cargo metadata`. Its `[dev-dependencies]` may contain `obsbot4linuxd`" (spec 3); the check is W0's `scripts/check-cli-deps.sh`.
- Track C owner files: "`crates/obsbot-proto/src/client*`, `crates/obsbot4linux`, `packaging/completions/`" (spec 10.3). Contract files are never edited in a track PR (spec 10.1): `crates/obsbot-proto/src/*` except `client*`, `crates/obsbot-device/src/contract/*`, `schema/protocol-2.json` (W0 Global Constraints).
- "`Client::connect` blocks per request. `Client::subscribe` returns a sender and a receiver; a reader thread delivers replies (with their request ids) and events in arrival order." "Errors are typed (`ErrorCode`, with an unknown-code variant). There is no built-in retry." "Public enums are `#[non_exhaustive]`." (spec 6.1)
- "`CLIENT_REPLY_TIMEOUT` | 30 s, in the client library | the client reports a lost daemon (CLI exit 4)" (spec 5.8). "Lines from the daemon have no length limit, and clients must not cap them." (spec 4.1)
- "Exit codes: 0 ok; 1 `internal`; 2 invalid input (`invalid`, `out_of_range`, `unsupported`, `unknown_device`, `usage`, `ambiguous_device`); 3 device not usable (`conflict`, `unavailable`, `asleep`, `busy`, `camera`, `no_device`, `unsupported_device`); 4 daemon unreachable (`daemon`, `protocol`)." Clients "treat an unknown error code as `internal`" (spec 6.2, 4.1 rule 7).
- "With `--json`, an error prints `{"error":{"code","message"}}` on stdout; help and version stay plain text. An error the daemon sent keeps its code." `usage`, `ambiguous_device`, `no_device` and `daemon` "exist only in the CLI and are never sent by the daemon" (spec 6.2).
- "`--device` takes the full id, a serial suffix of at least four characters, or a model key (`unknown` included); it must match exactly one listed device. Without `--device` these commands use the only usable device (support `tested` or `untested`, available)." (spec 6.2)
- Unsupported device: "refused by the CLI before anything is sent (exit 3, code `unsupported_device`) with the message: "<name> (USB <vendor>:<product>) is not supported by obsbot4linux <version>. `obsbot4linux devices --report` prints what an issue report needs: https://github.com/hellwand/obsbot4linux/issues"" (spec 6.2)
- No usable device: exit 3, `no_device`, "no usable OBSBOT camera", "followed by the list of unsupported and unavailable devices, if any, and the `devices --report` hint" (spec 6.2).
- "Value syntax, one per setting type: bool `on|off|true|false`; enum by name (`upper_body`, also `upper-body`); number `1.5`; range `min,max`; pose `pan,tilt,zoom`; point `x,y`; string as given. Only leaves are set. Action parameters use the same syntax. A `key=value` argument never starts with `-`; free text after `=` may." (spec 6.2)
- `--json` shapes: "`status`: `{"device":<Device>,"rev","state"}`; `get`, `set`, `do`: the `ok` payload of the reply; `devices`: `{"devices":[<Device>…]}`; `describe`: the device entry (`<Device>`); with a path or an action, that `Setting` or `Action` object." "`describe <path|action>` for a path or action not in the device's description exits 2 with code `unsupported`." (spec 6.2)
- "`devices --report` prints JSON in both modes … per device, `model`, `support`, `firmware`, `video_node` and the `usb` object … no id and no serial value: only `serial_present`." `devices` "exits 0 when it lists a device and 3 (`no_device`) when the list is empty" (spec 6.2, PX-5).
- "`watch` sends `hello` with `subscribe:true` and the given live topics, prints the reply as its first line and then every event, one JSON object per line, as received. It reads nothing from stdin. It exits 0 on SIGINT or SIGTERM and 4 when the connection ends." "`watch` takes no `--device`." (spec 6.2)
- "One-shot commands send `hello` with `subscribe:false` and read with `get`." "`gimbal.turn`: the CLI always sends `keep:true` with it." (spec 6.2)
- Completions: "for bash and zsh cover subcommands, flags, `get`/`describe` paths, `set` paths with `=`, enum values after `=`, action names and their parameters. They are generated from the CLI definition and the embedded description files (`descriptions/*.json` of real models only, never the fakes), committed under `packaging/completions/`, and a CLI-crate test fails when they are stale. The CLI embeds every `descriptions/*.json` file present at build time (a build-script glob) … Tab never contacts the daemon, and `--device` values are not completed." (spec 6.2)
- "`preset save N` without `--name` sends no name, and the CLI never makes one up. For a preset whose `name` is empty, `preset list` and `status` may print a placeholder of their own (for example `(unnamed)`) as display only; … `--json` prints the name as the camera reported it." (spec 6.2)
- "A client that cannot connect (no socket file, connection refused) exits 4 and prints `systemctl --user enable --now obsbot4linux.socket`." Older daemon: "an obsbot4linux daemon older than 1.0 is running: restart it or log out", exit 4. Protocol: "the obsbot4linux daemon speaks protocol <n>, this client speaks 2: install the same version of both and restart the daemon". "No CLI command, `watch` included, starts a daemon." "Clients never stop or restart a daemon." (spec 5.10, 6.2)
- "The full id and serial travel on the socket and in the CLI's `--json` and `watch` output. Human CLI output … show `…` plus the last four characters" through one formatter (`obsbot_proto::short_id`) (spec 4.2). "The CLI prints the level next to every device that is not `tested`." (spec 8.6)
- "Tests run `obsbot4linuxd::run(Config)` in-process on a temporary socket, in parallel, without systemd and without `XDG_RUNTIME_DIR`." (spec 11) Test sockets live under `CARGO_TARGET_TMPDIR` (the target directory), never under `/tmp`.
- Public text (spec 8.5): English; no private paths or names, no names of other software or firmware, no serial numbers (the synthetic `EXAMPLE…`, `FAKEA…`, `FAKEB…` are allowed).
- Repository facts: `edition = "2024"`, `rust-version = "1.88"`, CI toolchain 1.99.1 with `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test`; commits on `rebuild` are signed; every new source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later`.
- No camera anywhere in track C: "Acceptance with camera: none in this track." (spec 10.3)

## Review Focus

Inputs a person will hit that the spec's own acceptance list does not name, each pinned by a test in its owning task:

1. A path typed with dashes from 0.13 habit (`set framing.zoom-speed=fast`): only enum values take `-` for `_`; the path is sent as typed and the daemon's `unsupported` (exit 2) is shown. Test `a_dashed_path_is_sent_as_typed` (Task C-10).
2. The shortened id copied from human output into `--device …0001`: it matches no device (exit 2, `unknown_device`). Test `a_pasted_short_id_matches_no_device` (Task C-5); accepting it is product question Q1.
3. `obsbot4linux watch | head -1`: the reader goes away; `watch` ends quietly with 0 instead of an error loop. Test `watch_into_a_closed_pipe_exits_0` (Task C-11).
4. A quoted composite with spaces (`zone.area.pan="-20, 20"`): accepted as `{"min":-20,"max":20}`. Row in `range_is_min_comma_max` (Task C-4).
5. A support level from a newer daemon (`"support":"experimental"`, spec 4.1 rule 7): shown as it is, never usable without `--device`. Test `an_unknown_support_level_is_shown_and_not_usable` (Task C-5).

---

## What track C needs from W0 and from the other tracks

From **W0** (W0 has landed on `rebuild` at `3187b84`: builders read the tree, not the W0 plan; if the tree differs from the names below, the tree wins and the builder adapts the names in its task, nothing else):

- `obsbot_proto`: `PROTOCOL`, `CLIENT_REPLY_TIMEOUT`, `socket_path() -> Option<PathBuf>`, `short_id(&str) -> String`, `RequestId::{Int, Str}`, `Hello { protocol, client, subscribe, live }` with `Hello::new(&str)`, `.subscribe(bool)`, `.live(&[&str])`; `Request { id, verb }`; `Verb::{Hello(Hello), Get { device, paths: Option<Vec<String>> }, Set { device, patch: Patch }, Do { device, action, params: Params }, Live { topics }}`; `Reply { id: Option<RequestId>, body: ReplyBody }`; `ReplyBody::{Ok(OkBody), Error(ErrorBody)}`; `OkBody::{Hello(HelloOk), State(RevState), Rev(RevOk), Empty(Empty)}`; `HelloOk { protocol, daemon, devices: Vec<Device>, states: BTreeMap<String, RevState> }`; `RevState { rev: u64, state: State }`; `RevOk { rev: u64, result: Option<Params> }`; `ErrorBody { code, message, rev, applied, failed, supported }` with `ErrorBody::new(ErrorCode, &str)`; `ErrorCode` (12 variants, `Other(String)`, `ALL`, `as_str()`); `Event::{State { device, rev, state }, Devices { devices }, Unknown}`; `ServerMessage::{Event, Reply}`; `Device { id, name, model, support, serial, firmware, video_node, available, replaced_by, usb, description }`; `Usb`; `Description { settings, actions, limits }` with its builders; `Setting` (fields of 4.2, builders `boolean`, `enumeration`, `number`, `string`, `list`, `range`, `new`, `.unit`, `.optional`, `.asleep`); `SettingType` with `components()`; `Unit`; `UnitSpec::{One, Each}`; `Bound::get`; `Action` with `Action::new`, `.param`; `Asleep`; `Support::{Tested, Untested, Unsupported, Other}`; `State { available, pending, unread, values }` with `State::new`, `at`, `set`; `Patch(pub Map)` with `Patch::new`, `leaf`; `Params`; `state::DAEMON_PATHS`; `values::{number, same}`.
- Golden files `crates/obsbot-proto/tests/golden/{session,errors,hotplug}.jsonl` (read by tests, not edited); `descriptions/tiny2lite.json`, `descriptions/fake/fake-b.json` (read by tests and by `build.rs`).
- `scripts/check-cli-deps.sh`, `scripts/regenerate.sh` (Task C-8 adds one line).
- The CLI crate as W0 leaves it: `[dependencies] obsbot-proto`, `[dev-dependencies] obsbot4linuxd`, the stub `src/main.rs`.
- `obsbot4linuxd::Config { pub socket: PathBuf, pub fake: Option<Arc<FakeBus>>, pub stop: Arc<AtomicBool> }`, `Config::new(PathBuf) -> Config`, `obsbot4linuxd::run(Config) -> std::io::Result<()>` (Tasks C-12, C-13).
- `obsbot_device::models::fake::{FakeBus, Op}`: `FakeBus::new(&[&str]) -> Result<FakeBus, String>`, `ops(&self, &str) -> Vec<Op>`, `sleep(&self, &str) -> Result<(), String>`, `wake(&self, &str) -> Result<(), String>`; `Op::{Open, Identity(Need), Read(String), Apply(String, Value), Act(String, Params)}`; `obsbot_device::models::fake::description(&str) -> Option<Description>` (Tasks C-12, C-13, as dev-dependency).

From **track B**: milestone **B-1** for Tasks C-12 and C-13 only: `obsbot4linuxd::run(Config)` behind W0's unchanged entry point, serving protocol 2 (spec 4.1, 5.4, 5.12) for the fake bus in `Config::fake`; it binds `Config::socket` (accepting connections means it is ready), and returns `Ok(())` once `Config::stop` is set. Every other task runs against the scripted daemon of Task C-1 and needs nothing from track B.

From **track A**: nothing. From **track D**: only the outcome of its first check (the widget opening the Unix socket); if that check fails, the conditional Task C-R (`relay`) is built.

## Decisions this plan takes

Each can be overruled by the maintainers; the reason follows the dash.

1. **Client layout.** `src/client.rs` plus `src/client/{error,connection,blocking,subscribe,script}.rs` — all inside the owner glob `client*`, so W0's `lib.rs` (`pub mod client;`) stays as it is.
2. **`Client::subscribe` returns `(HelloOk, Sender, Receiver)`**, not spec 6.1's `(tx, rx)` — "the caller keeps its own device list from `hello`" needs the reply, and an older daemon or a protocol mismatch is reported by the call itself, not later on the receiver.
3. **`Connection` is public** — `watch` must print lines "as received" (a newer daemon's fields and number spellings unchanged), which a typed re-serialisation would lose; a `relay` would need it too.
4. **`hello` classification** — a reply that does not parse as a protocol-2 reply, or whose `ok` payload is not a `hello` reply, is `OldDaemon`; the error `protocol` or a `hello` reply of another protocol is `Protocol`; any other error is `Refused`. Spec 5.10: "A reply to `hello` that is not a protocol-2 object … means an older daemon".
5. **Request ids** are integers counting from 1 per connection, `hello` being 1 — as in the golden transcripts, so the session transcript replays unchanged.
6. **`connect` forces `subscribe:false`, `subscribe` forces `true`** — the blocking client has no reader for events; the subscribed one exists for them.
7. **The subscriber's reply timeout** is tracked per request in the `Receiver`; an overdue reply ends the receiver with `Timeout` — spec 5.8 puts `CLIENT_REPLY_TIMEOUT` in the client library, for both forms.
8. **A scripted daemon in `obsbot_proto::client::script`** (doc-hidden, std only) — one helper for the proto and the CLI tests before B-1; it compares requests as JSON values and fails on any line after the script, which is how every command test proves "one request per command".
9. **The CLI crate gets a library target**: `obsbot4linux::run(args, socket, exit_on_signal, out, err) -> u8`, with a thin `main` — in-process tests without environment variables (process-global; `set_var` is `unsafe` in edition 2024) and without spawning, as `obsbot4linuxd` is "also a library with `run(Config)` for tests" (spec 3).
10. **clap 4 (derive) for the definition; an own generator for the completion scripts** — clap is already a workspace dependency and gives help, version and usage errors; `clap_complete` cannot complete description paths, values after `=` or the `=` word break, so the scripts are generated by `completions.rs` from the clap `Command` and the embedded descriptions.
11. **The CLI parses syntax only and runs no validator** — bounds, steps, writable values, read-only leaves and `requires` are the daemon's (spec 6.1: "the daemon stays authoritative"), so codes and messages come from one place. A path, parameter or action the description lacks is sent with its text as a string, so the daemon answers it with its own code (`unsupported`, or `invalid` for a group or an unknown parameter). Only text the CLI cannot type (a non-number for a number, a wrong component count, a list, a type it does not know) is `usage`.
12. **`-` for `_` only in enum values, and only when the dashed text is not itself a value** — spec 6.2 names it for enums; paths are never rewritten (Review Focus 1).
13. **`keep: true` for every action whose description has a parameter `keep`**; any other `keep` is `usage` — spec 6.2's "`gimbal.turn`: the CLI always sends `keep:true`" without naming a model in the CLI.
14. **`--json` output is one compact line in protocol field order**, serialised from the typed objects like the golden lines; `status` is composed as `{"device":…,"rev":…,"state":…}`. A newer daemon's fields this client does not know are dropped there (spec 4.1 rule 7 lets clients ignore them); `watch` prints raw lines.
15. **JSON errors**: the CLI's own errors print `{"error":{"code","message"}}`, plus `devices` (full ids) for `ambiguous_device` and `no_device`; an error the daemon sent prints its whole body (`rev`, `applied`, `failed`, `supported`) — spec 6.2 wants the matching devices printed; an additive field keeps the shape.
16. **Human output** (`render.rs`, `describe.rs`): `status` and `get` print paths (what `set` takes) grouped by their first segment, `describe` prints the labels; units as suffixes (`°`, `°/s`, `×`, ` µs`, ` K`, `%`, ` s`, ` fps`, none for `fraction`); a writable bool `on`/`off`, a read-only one `yes`/`no`; `null` as `–`; a pending path shows its pending value and `(pending)`; `(unread)`; an empty `name` in a list item `(unnamed)`; one device line `<short id>  <name>  <model>[  <support>][  unavailable]`.
17. **`set` and `do` print nothing in human form on success**, `do` prints the result fields when the action has a result; `--json` prints the `ok` payload.
18. **`devices`**: `--device` lists only the matched device; an empty list is `no_device` with "no OBSBOT camera is listed" (PX-5); `--report` is pretty-printed (it is pasted into issues).
19. **`watch`** blocks SIGINT and SIGTERM in its only thread and lets a `sigwait` thread exit 0 while holding the stdout lock, so no line is cut; a closed stdout ends it with 0; the end of the connection is `daemon` (exit 4); `--device` is `usage`. Only the binary turns the signal handling on (`exit_on_signal`), never an in-process test.
20. **Unknown values from a newer daemon**: a support level is shown as text and is not usable; a setting type the CLI does not know is shown as JSON text and `set` refuses it with `usage` (spec 4.1 rule 7).
21. **Completions** go to `packaging/completions/obsbot4linux.bash` and `packaging/completions/_obsbot4linux`; words are the union over every embedded description (a path means the same on every model, spec 4.2); `keep` is not offered (decision 13); `--device` values never; the bash script works with and without `=` in `COMP_WORDBREAKS`. `scripts/regenerate.sh` gets one line for them (spec 6.2: "A new model regenerates the description file and the completions in its PR with `scripts/regenerate.sh`").
22. **Test sockets** under `CARGO_TARGET_TMPDIR`; names unique per test.
23. **End-to-end snapshots** (`crates/obsbot4linux/tests/snapshots/`) are written on the first run against B-1 with `OBSBOT_REGENERATE=1` and reviewed like code; every e2e test also asserts the spec facts it relies on.
24. **All CLI dependencies are added in Task C-2** (`clap`, `nix` with `signal`, `serde`, `serde_json`), so no later task edits `Cargo.toml` except C-12, which adds the dev-dependency `obsbot-device` (the fake bus and its operation log for the e2e tests; dev-dependencies are allowed by spec 3 and the check).
25. **`describe <group>` is `unsupported`** — only leaves and actions are `Setting` or `Action` objects (spec 6.2).

## Product questions for the coordinator (not built)

- **Q1.** Should `--device` accept the shortened id that human output prints (`…0001`)? Recommendation: yes, strip a leading `…` and match it as a serial suffix — the user pastes what the CLI shows; it adds no device fact. Built today: no (exit 2, pinned by a test, Review Focus 2).
- **Q2.** Spec 4.2 says "Generic UIs ask before they send either" (`restart`, `destructive`). Should the CLI prompt? Recommendation: no prompt — scripts and `scripts/check-model.sh` call the CLI; destructive actions already need `confirm=true` (the daemon refuses without it); at most a one-line note on stderr after a `restart` write. Built today: no prompt and no note.

## Decisions taken after the plan review (2026-11-13)

The coordinator decided the product questions of the review and the seam points between the tracks. Where a line below differs from the text of this plan, the line wins; the code of the plan is otherwise unchanged. The product questions above are answered by these lines.

1. **c-5.** Question 7, the shortened id: `--device` accepts the shortened id that human output prints (`…0001`). Task c-5 changes `select::matching` to strip one leading `…` from the wanted text before matching (so `…0001` matches as the serial suffix `0001`, with the usual rules: a unique match selects, several give `ambiguous_device`), and replaces the test `a_pasted_short_id_matches_no_device` (exit 2, `unknown_device`) with one that pins the match; Review Focus 2 and product question Q1 are answered by this line.
2. **c-10.** Question 8, no prompt in the command line: `restart` and `destructive` actions are sent as typed; scripts use the CLI and destructive actions need `confirm=true`, which the daemon enforces. No confirmation, no note on stderr. Task c-10 builds it as the plan says (`a_destructive_action_without_confirm_is_the_daemons_invalid` stays). Interactive UIs such as the widget ask; spec 4.2 now says so.
3. **c-12.** Gate: B-1 is the task b-10 landed on `rebuild`; c-12 and then c-13 are issued only after it has landed (the coordinator holds them; the task list cannot say it).
4. **c-3.** `Client::subscribe` returns `(HelloOk, Sender, Receiver)` (decision 2); spec 6.1 now says so. Nothing changes in the plan.

## File structure

| File                                                                                | Responsibility                                                                 | Task                       |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | -------------------------- |
| `crates/obsbot-proto/src/client.rs` (W0 stub, replaced)                             | client module root and re-exports                                              | C-1, C-3                   |
| `crates/obsbot-proto/src/client/error.rs`                                           | `ClientError`                                                                  | C-1                        |
| `crates/obsbot-proto/src/client/connection.rs`                                      | `Connection`, `HelloReply`, line reading with deadline, `hello` classification | C-1, C-3 (two methods)     |
| `crates/obsbot-proto/src/client/blocking.rs`                                        | `Client` (blocking)                                                            | C-1                        |
| `crates/obsbot-proto/src/client/script.rs`                                          | scripted daemon for tests (doc-hidden)                                         | C-1                        |
| `crates/obsbot-proto/src/client/subscribe.rs`                                       | `Sender`, `Receiver`, `Incoming`, `Client::subscribe`                          | C-3                        |
| `crates/obsbot-proto/tests/client.rs`, `tests/client_subscribe.rs`                  | client tests                                                                   | C-1, C-3                   |
| `crates/obsbot4linux/Cargo.toml`                                                    | lib + bin, dependencies                                                        | C-2, C-12 (dev-dependency) |
| `crates/obsbot4linux/src/{lib,main,args,errors}.rs`                                 | entry point, definition, errors and exit codes                                 | C-2                        |
| `crates/obsbot4linux/src/value.rs`                                                  | value syntax                                                                   | C-2 (stub), C-4            |
| `crates/obsbot4linux/src/{select,session}.rs`                                       | device selection, connecting, client errors                                    | C-2 (stubs), C-5           |
| `crates/obsbot4linux/src/render.rs`                                                 | human output of states and device lists                                        | C-2 (stub), C-6            |
| `crates/obsbot4linux/src/describe.rs`                                               | human output of descriptions                                                   | C-2 (stub), C-7            |
| `crates/obsbot4linux/build.rs`, `src/completions.rs`                                | embedded descriptions, completion generator                                    | C-2 (stub), C-8            |
| `packaging/completions/obsbot4linux.bash`, `packaging/completions/_obsbot4linux`    | generated completions                                                          | C-8                        |
| `scripts/regenerate.sh` (one line)                                                  | regenerates the completions too                                                | C-8                        |
| `crates/obsbot4linux/src/read.rs`                                                   | `status`, `get`, `describe`, `devices`                                         | C-2 (stub), C-9            |
| `crates/obsbot4linux/src/write.rs`                                                  | `set`, `do`, `keep`                                                            | C-2 (stub), C-10           |
| `crates/obsbot4linux/src/watch.rs`                                                  | `watch`                                                                        | C-2 (stub), C-11           |
| `crates/obsbot4linux/tests/common/mod.rs`                                           | in-process runner, fixtures of three devices                                   | C-2                        |
| `crates/obsbot4linux/tests/cli_errors.rs`                                           | usage errors, help, version                                                    | C-2                        |
| `crates/obsbot4linux/tests/completions.rs`                                          | stale test, bash behaviour, zsh syntax                                         | C-8                        |
| `crates/obsbot4linux/tests/{read,write,watch}.rs`                                   | commands against the scripted daemon                                           | C-9, C-10, C-11            |
| `crates/obsbot4linux/tests/fake_daemon/mod.rs`, `tests/e2e.rs`, `tests/snapshots/*` | end to end against the fake daemon                                             | C-12                       |
| `crates/obsbot4linux/tests/parity.rs`                                               | every row of 9.1 against `fake-a`                                              | C-13                       |

## Tasks, waves and dependencies

A task branches from `rebuild` after its dependencies have landed; where its only dependency is still in review, it is stacked on that branch. Tasks of one wave touch disjoint files. `Cargo.lock` changes in C-2 and C-12: on a rebase conflict take `rebuild`'s version and run `cargo build --workspace`.

| Task | Title                                          | Wave | Depends on      | Stacked on | Needs B-1 | Effort |
| ---- | ---------------------------------------------- | ---- | --------------- | ---------- | --------- | ------ |
| C-1  | Blocking client and scripted daemon            | 1    | –               | –          | no        | L      |
| C-2  | CLI skeleton: definition, errors, exit codes   | 1    | –               | –          | no        | M      |
| C-3  | Subscribed client                              | 2    | C-1             | C-1        | no        | M      |
| C-4  | Value syntax                                   | 2    | C-2             | C-2        | no        | S      |
| C-5  | Device selection and session                   | 2    | C-1, C-2        | –          | no        | S      |
| C-6  | Human output of states and devices             | 2    | C-2             | C-2        | no        | M      |
| C-7  | Human output of descriptions                   | 2    | C-2             | C-2        | no        | S      |
| C-8  | Completions and their stale test               | 2    | C-2             | C-2        | no        | M      |
| C-9  | `status`, `get`, `describe`, `devices`         | 3    | C-5, C-6, C-7   | –          | no        | M      |
| C-10 | `set` and `do`                                 | 3    | C-4, C-5, C-6   | –          | no        | S      |
| C-11 | `watch`                                        | 3    | C-5             | C-5        | no        | S      |
| C-12 | End to end against the fake daemon             | 4    | C-9, C-10, C-11 | –          | yes       | M      |
| C-13 | Every parity row against `fake-a`              | 5    | C-12            | C-12       | yes       | M      |
| C-R  | `relay` (only if track D's socket check fails) | –    | C-1, C-2        | –          | no        | S      |

C-5 depends on two wave-1 tasks: it starts once both have landed. C-13 is stacked on C-12, so it is in the wave after it. C-12 and C-13 start only after track B's B-1 has landed on `rebuild` (they fail without a daemon behind `run`); until then they are not issued: the coordinator gates them on B-1 (note 4 of "Notes for the coordinator"). The code of every task was built and run in a copy of the W0 workspace on 2026-11-13 (rustc 1.99.1), task by task in this order (see "Proof" at the end).

---

### Task C-1: Blocking client and scripted daemon

**Files:**

- Modify: `crates/obsbot-proto/src/client.rs` (W0's stub: replace the whole file)
- Create: `crates/obsbot-proto/src/client/error.rs`, `src/client/connection.rs`, `src/client/blocking.rs`, `src/client/script.rs`
- Test: `crates/obsbot-proto/tests/client.rs`; unit tests in `src/client/connection.rs`

**Interfaces:**

- Consumes (W0): `PROTOCOL`, `CLIENT_REPLY_TIMEOUT`, `socket_path()`, `RequestId`, `Hello`, `Request`, `Verb`, `Reply`, `ReplyBody`, `OkBody`, `HelloOk`, `ErrorBody`, `ErrorCode`, `ServerMessage`, `Device`, `State`, `Patch`, `Params`, `values::same`.
- Produces (module `obsbot_proto::client`):
  - `#[non_exhaustive] pub enum ClientError { NoSocketPath, Connect { path: PathBuf, source: io::Error }, Disconnected(Option<io::Error>), Timeout, OldDaemon, Protocol { supported: Vec<u32>, message: String }, Refused(Box<ErrorBody>), Malformed(String), NotAnObject }` with `Display` and `std::error::Error`.
  - `pub struct HelloReply { pub ok: HelloOk, pub line: String }`.
  - `pub struct Connection` with `pub fn open(path: &Path) -> Result<Connection, ClientError>`, `pub fn open_default() -> Result<Connection, ClientError>`, `pub fn set_reply_timeout(&mut self, timeout: Duration)`, `pub fn reply_timeout(&self) -> Duration`, `pub fn send(&mut self, verb: Verb) -> Result<RequestId, ClientError>`, `pub fn read_line(&mut self, wait: Option<Duration>) -> Result<String, ClientError>`, `pub fn hello(&mut self, hello: Hello) -> Result<HelloReply, ClientError>`.
  - `pub struct Client` with `pub fn connect(hello: Hello) -> Result<Client, ClientError>`, `pub fn connect_to(path: &Path, hello: Hello) -> Result<Client, ClientError>`, `pub fn hello(&self) -> &HelloOk`, `pub fn devices(&self) -> &[Device]`, `pub fn set_reply_timeout(&mut self, timeout: Duration)`, `pub fn get(&mut self, device: &str, paths: Option<&[&str]>) -> Result<(u64, State), ClientError>`, `pub fn set(&mut self, device: &str, patch: Patch) -> Result<u64, ClientError>`, `pub fn act(&mut self, device: &str, action: &str, params: Value) -> Result<(u64, Option<Params>), ClientError>`.
  - `#[doc(hidden)] pub mod script`: `pub struct ScriptServer` with `pub fn start(path: &Path, transcript: &str) -> ScriptServer`, `pub fn path(&self) -> &Path`, `pub fn finish(self) -> Result<Vec<String>, String>`.
  - Crate-internal, used by C-3: `connection::WriteHalf { fn next_id(&self) -> RequestId, fn send(&mut self, verb: Verb) -> Result<RequestId, ClientError> }`, `connection::ReadHalf { fn line(&mut self, deadline: Option<Instant>) -> Result<String, ClientError> }`, `blocking::params_of(params: Value) -> Result<Params, ClientError>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/obsbot-proto/tests/client.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The blocking client (spec 6.1) against a scripted daemon: the exact lines it sends, the
//! replies it returns, and how it reports an older daemon, a protocol mismatch, a refusal,
//! a lost connection and a missing reply (4.1, 5.8, 5.10).

use obsbot_proto::client::script::ScriptServer;
use obsbot_proto::client::{Client, ClientError, Connection};
use obsbot_proto::{ErrorCode, Hello, PROTOCOL, Patch};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

fn socket(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c1-{name}.sock"))
}

const HELLO: &str =
    r#"> {"id":1,"hello":{"protocol":2,"client":"test 1.0","subscribe":false,"live":[]}}"#;
const HELLO_OK: &str =
    r#"< {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}"#;

fn script(lines: &[&str]) -> String {
    lines.join("\n")
}

#[test]
fn the_blocking_client_sends_one_line_per_call_and_returns_the_reply() {
    let server = ScriptServer::start(
        &socket("calls"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","get":["led"]}"#,
            r#"< {"id":2,"ok":{"rev":6,"state":{"available":true,"pending":{},"unread":[],"led":{"brightness":2}}}}"#,
            r#"> {"id":3,"device":"sn:EXAMPLE0000001","get":null}"#,
            r#"< {"id":3,"ok":{"rev":6,"state":{"available":true,"pending":{},"unread":[]}}}"#,
            r#"> {"id":4,"device":"sn:EXAMPLE0000001","set":{"led.brightness":2}}"#,
            r#"< {"id":4,"ok":{"rev":7}}"#,
            r#"> {"id":5,"device":"sn:EXAMPLE0000001","do":"view.save","with":{"name":"Desk"}}"#,
            r#"< {"id":5,"ok":{"rev":8,"result":{"id":3}}}"#,
            r#"> {"id":6,"device":"sn:EXAMPLE0000001","do":"gimbal.stop"}"#,
            r#"< {"id":6,"ok":{"rev":8}}"#,
        ]),
    );
    // `connect_to` sends subscribe:false even when the hello asks for events.
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0").subscribe(true)).unwrap();
    assert_eq!(c.hello().protocol, PROTOCOL);
    assert!(c.devices().is_empty());
    let (rev, state) = c.get("sn:EXAMPLE0000001", Some(&["led"])).unwrap();
    assert_eq!((rev, state.at("led.brightness")), (6, Some(&json!(2))));
    let (rev, _) = c.get("sn:EXAMPLE0000001", None).unwrap();
    assert_eq!(rev, 6);
    let rev = c
        .set("sn:EXAMPLE0000001", Patch::new().leaf("led.brightness", 2))
        .unwrap();
    assert_eq!(rev, 7);
    let (rev, result) = c
        .act("sn:EXAMPLE0000001", "view.save", json!({"name": "Desk"}))
        .unwrap();
    assert_eq!(rev, 8);
    assert_eq!(result.unwrap()["id"], json!(3));
    let (rev, result) = c
        .act("sn:EXAMPLE0000001", "gimbal.stop", serde_json::Value::Null)
        .unwrap();
    assert_eq!((rev, result), (8, None));
    drop(c);
    assert_eq!(server.finish().unwrap().len(), 6);
}

#[test]
fn events_on_the_connection_are_skipped_while_a_reply_is_awaited() {
    let server = ScriptServer::start(
        &socket("events"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","set":{"led.brightness":2}}"#,
            r#"< {"event":"state","device":"sn:EXAMPLE0000001","rev":5,"state":{"available":true,"pending":{"led.brightness":2},"unread":[]}}"#,
            r#"< {"event":"frobnicate","from":"a newer daemon"}"#,
            r#"< {"id":2,"ok":{"rev":5}}"#,
        ]),
    );
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    let rev = c
        .set("sn:EXAMPLE0000001", Patch::new().leaf("led.brightness", 2))
        .unwrap();
    assert_eq!(rev, 5);
    drop(c);
    server.finish().unwrap();
}

#[test]
fn a_refusal_carries_the_daemons_error_body() {
    let server = ScriptServer::start(
        &socket("refused"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","set":{"exposure.mode":"manual","exposure.time":20000}}"#,
            r#"< {"id":2,"error":{"code":"camera","message":"the camera did not answer","rev":9,"applied":["exposure.mode"],"failed":"exposure.time"}}"#,
            r#"> {"id":3,"device":"sn:EXAMPLE0000001","get":null}"#,
            r#"< {"id":3,"error":{"code":"teapot","message":"a code from a newer daemon"}}"#,
        ]),
    );
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    let patch = Patch::new()
        .leaf("exposure.mode", "manual")
        .leaf("exposure.time", 20000);
    match c.set("sn:EXAMPLE0000001", patch) {
        Err(ClientError::Refused(body)) => {
            assert_eq!(body.code, ErrorCode::Camera);
            assert_eq!(body.rev, Some(9));
            assert_eq!(body.applied, Some(vec!["exposure.mode".to_string()]));
            assert_eq!(body.failed.as_deref(), Some("exposure.time"));
        }
        other => panic!("{other:?}"),
    }
    match c.get("sn:EXAMPLE0000001", None) {
        Err(ClientError::Refused(body)) => {
            assert_eq!(body.code, ErrorCode::Other("teapot".into()));
        }
        other => panic!("{other:?}"),
    }
    drop(c);
    server.finish().unwrap();
}

/// Spec 5.10: a 0.13 daemon answers the hello line with `ok:false` and a string `error`.
#[test]
fn a_0_13_daemon_is_reported_as_older_than_1_0() {
    let server = ScriptServer::start(
        &socket("old"),
        &script(&[
            HELLO,
            r#"< {"id":null,"ok":false,"error":"bad request: missing field `args` at line 1 column 83"}"#,
        ]),
    );
    let got = Client::connect_to(server.path(), Hello::new("test 1.0"));
    assert!(
        matches!(got, Err(ClientError::OldDaemon)),
        "{:?}",
        got.err()
    );
    server.finish().unwrap();
}

#[test]
fn a_daemon_of_another_protocol_is_reported_with_its_numbers() {
    let server = ScriptServer::start(
        &socket("protocol"),
        &script(&[
            HELLO,
            r#"< {"id":null,"error":{"code":"protocol","message":"protocol 2 is not supported","supported":[3]}}"#,
            "!close",
        ]),
    );
    match Client::connect_to(server.path(), Hello::new("test 1.0")) {
        Err(ClientError::Protocol { supported, .. }) => assert_eq!(supported, vec![3]),
        other => panic!("{:?}", other.err()),
    }
    server.finish().unwrap();
}

#[test]
fn no_socket_is_a_connect_error_naming_the_path() {
    let path = socket("missing");
    let _ = std::fs::remove_file(&path);
    match Client::connect_to(&path, Hello::new("test 1.0")) {
        Err(ClientError::Connect { path: p, .. }) => assert_eq!(p, path),
        other => panic!("{:?}", other.err()),
    }
}

#[test]
fn a_closed_connection_ends_the_call() {
    let server = ScriptServer::start(&socket("closed"), &script(&[HELLO, HELLO_OK, "!close"]));
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    server.finish().unwrap();
    let got = c.get("sn:EXAMPLE0000001", None);
    assert!(matches!(got, Err(ClientError::Disconnected(_))), "{got:?}");
}

/// Spec 5.8: no reply within the reply timeout is reported, not waited for forever.
#[test]
fn a_missing_reply_times_out() {
    let server = ScriptServer::start(
        &socket("timeout"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","get":null}"#,
        ]),
    );
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    c.set_reply_timeout(Duration::from_millis(200));
    let got = c.get("sn:EXAMPLE0000001", None);
    assert!(matches!(got, Err(ClientError::Timeout)), "{got:?}");
    drop(c);
    server.finish().unwrap();
}

#[test]
fn a_hello_without_reply_times_out() {
    let server = ScriptServer::start(&socket("hello-timeout"), &script(&[HELLO]));
    let mut conn = Connection::open(server.path()).unwrap();
    conn.set_reply_timeout(Duration::from_millis(200));
    let got = conn.hello(Hello::new("test 1.0"));
    assert!(matches!(got, Err(ClientError::Timeout)), "{:?}", got.err());
    drop(conn);
    server.finish().unwrap();
}

#[test]
fn action_parameters_must_be_an_object_and_nothing_is_sent_otherwise() {
    let server = ScriptServer::start(&socket("not-object"), &script(&[HELLO, HELLO_OK]));
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    let got = c.act("sn:EXAMPLE0000001", "preset.recall", json!([1]));
    assert!(matches!(got, Err(ClientError::NotAnObject)), "{got:?}");
    drop(c);
    assert_eq!(server.finish().unwrap().len(), 1, "only hello was sent");
}

/// Spec 4.1: lines from the daemon have no length limit, and clients must not cap them.
#[test]
fn a_long_line_from_the_daemon_is_read_whole() {
    let long = "x".repeat(200 * 1024);
    let reply = format!(
        r#"< {{"id":1,"ok":{{"protocol":2,"daemon":"{long}","devices":[],"states":{{}}}}}}"#
    );
    let server = ScriptServer::start(&socket("long"), &script(&[HELLO, &reply]));
    let c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    assert_eq!(c.hello().daemon.len(), long.len());
    drop(c);
    server.finish().unwrap();
}

#[test]
fn a_reply_to_another_request_is_malformed() {
    let server = ScriptServer::start(
        &socket("other-id"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","get":null}"#,
            r#"< {"id":7,"ok":{"rev":1,"state":{"available":true,"pending":{},"unread":[]}}}"#,
        ]),
    );
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    let got = c.get("sn:EXAMPLE0000001", None);
    assert!(matches!(got, Err(ClientError::Malformed(_))), "{got:?}");
    drop(c);
    server.finish().unwrap();
}

/// The scripted daemon itself: a client that sends one request more than the script holds
/// fails the script (the CLI's one-request tests rely on it).
#[test]
fn the_script_catches_an_extra_request() {
    let server = ScriptServer::start(&socket("extra"), &script(&[HELLO, HELLO_OK]));
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    c.set_reply_timeout(Duration::from_millis(100));
    let _ = c.get("sn:EXAMPLE0000001", None);
    drop(c);
    let err = server.finish().unwrap_err();
    assert!(err.contains("one line more"), "{err}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot-proto --test client`
Expected: the test does not compile: `obsbot_proto::client` has no `script`, `Client`, `ClientError` or `Connection` (W0's `client.rs` is empty).

- [ ] **Step 3: Write the client**

Replace `crates/obsbot-proto/src/client.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The synchronous client (design spec 6.1). A [`Client`] blocks per request on its own
//! connection (`subscribe: false`). State, patches and action parameters are path → JSON
//! value; there are no per-setting methods. There is no built-in retry: when the connection
//! ends, the caller decides whether to reconnect (4.1, "Reconnect").

mod blocking;
mod connection;
mod error;
#[doc(hidden)]
pub mod script;

pub use blocking::Client;
pub use connection::{Connection, HelloReply};
pub use error::ClientError;
```

Create `crates/obsbot-proto/src/client/error.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Why a client call failed (spec 6.1: errors are typed; the daemon's code is an
//! [`ErrorCode`](crate::ErrorCode), which has a variant for codes this crate does not know).

use crate::message::ErrorBody;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// Why a client call failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum ClientError {
    /// Neither `OBSBOT4LINUX_SOCKET` nor `XDG_RUNTIME_DIR` names a socket.
    NoSocketPath,
    /// The socket could not be connected: no socket file, connection refused, no permission.
    Connect { path: PathBuf, source: io::Error },
    /// The daemon closed the connection, or reading or writing it failed.
    Disconnected(Option<io::Error>),
    /// No reply within the reply timeout (`CLIENT_REPLY_TIMEOUT`, spec 5.8).
    Timeout,
    /// The reply to `hello` is not a protocol-2 object: a daemon older than 1.0 (spec 5.10).
    OldDaemon,
    /// The daemon speaks other protocol numbers: it answered `hello` with the error `protocol`,
    /// or with a `hello` reply of another protocol.
    Protocol {
        supported: Vec<u32>,
        message: String,
    },
    /// The daemon refused the request; the body as the daemon sent it (boxed: it is large).
    Refused(Box<ErrorBody>),
    /// A line from the daemon is not a protocol-2 message, or a reply does not fit its request.
    Malformed(String),
    /// Action parameters that are not a JSON object (nothing was sent).
    NotAnObject,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::NoSocketPath => {
                f.write_str("neither OBSBOT4LINUX_SOCKET nor XDG_RUNTIME_DIR is set")
            }
            ClientError::Connect { path, source } => {
                write!(f, "cannot connect to {}: {source}", path.display())
            }
            ClientError::Disconnected(None) => f.write_str("the daemon closed the connection"),
            ClientError::Disconnected(Some(e)) => write!(f, "the connection failed: {e}"),
            ClientError::Timeout => f.write_str("no reply from the daemon in time"),
            ClientError::OldDaemon => f.write_str("the daemon does not speak protocol 2"),
            ClientError::Protocol { supported, .. } => {
                write!(f, "the daemon speaks protocol {supported:?}, not 2")
            }
            ClientError::Refused(body) => write!(f, "{}: {}", body.code.as_str(), body.message),
            ClientError::Malformed(m) => write!(f, "unreadable message from the daemon: {m}"),
            ClientError::NotAnObject => f.write_str("action parameters must be a JSON object"),
        }
    }
}

impl std::error::Error for ClientError {}
```

Create `crates/obsbot-proto/src/client/connection.rs` (its unit tests classify the replies of a protocol-2 daemon, a 0.13 daemon and a daemon of another protocol):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! One connection to the daemon: JSON lines over the Unix socket (spec 4.1). Lines from the
//! daemon have no length limit and are not capped here. Every wait for a reply is bounded by
//! the reply timeout (`CLIENT_REPLY_TIMEOUT`, 30 s, spec 5.8).

use crate::client::ClientError;
use crate::ids::RequestId;
use crate::message::{ErrorCode, Hello, HelloOk, OkBody, Reply, ReplyBody, Request, Verb};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

/// The reply to `hello`: parsed, and the line exactly as the daemon sent it (`watch` prints
/// it unchanged, spec 6.2).
#[derive(Debug, Clone)]
pub struct HelloReply {
    pub ok: HelloOk,
    pub line: String,
}

/// The sending side: request ids count up from 1 per connection.
pub(crate) struct WriteHalf {
    stream: UnixStream,
    next_id: i64,
}

impl WriteHalf {
    /// The id the next request gets.
    pub(crate) fn next_id(&self) -> RequestId {
        RequestId::Int(self.next_id)
    }

    /// Sends one request with the next id and returns that id.
    pub(crate) fn send(&mut self, verb: Verb) -> Result<RequestId, ClientError> {
        let id = self.next_id();
        self.next_id += 1;
        let request = Request {
            id: id.clone(),
            verb,
        };
        let mut line = serde_json::to_string(&request).expect("a request always serialises");
        line.push('\n');
        self.stream
            .write_all(line.as_bytes())
            .map_err(|e| ClientError::Disconnected(Some(e)))?;
        Ok(id)
    }
}

/// The receiving side. Keeps a partial line across timeouts.
pub(crate) struct ReadHalf {
    reader: BufReader<UnixStream>,
    buf: Vec<u8>,
}

impl ReadHalf {
    /// The next line without its newline. `deadline: None` waits as long as it takes.
    pub(crate) fn line(&mut self, deadline: Option<Instant>) -> Result<String, ClientError> {
        loop {
            let timeout = match deadline {
                None => None,
                Some(at) => {
                    let left = at.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Err(ClientError::Timeout);
                    }
                    Some(left)
                }
            };
            self.reader
                .get_ref()
                .set_read_timeout(timeout)
                .map_err(|e| ClientError::Disconnected(Some(e)))?;
            match self.reader.read_until(b'\n', &mut self.buf) {
                Ok(_) if self.buf.last() == Some(&b'\n') => {
                    let mut bytes = std::mem::take(&mut self.buf);
                    bytes.pop();
                    return String::from_utf8(bytes)
                        .map_err(|_| ClientError::Malformed("a line is not UTF-8".into()));
                }
                // End of stream, also inside a line.
                Ok(_) => return Err(ClientError::Disconnected(None)),
                Err(e)
                    if matches!(
                        e.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(ClientError::Disconnected(Some(e))),
            }
        }
    }
}

/// One connection to the daemon. [`Client`](crate::client::Client) builds on it; `watch`
/// uses it directly to print lines as received.
pub struct Connection {
    w: WriteHalf,
    r: ReadHalf,
    reply_timeout: Duration,
}

impl Connection {
    /// Connects to the socket at `path`.
    pub fn open(path: &Path) -> Result<Connection, ClientError> {
        let stream = UnixStream::connect(path).map_err(|source| ClientError::Connect {
            path: path.to_path_buf(),
            source,
        })?;
        let read = stream
            .try_clone()
            .map_err(|e| ClientError::Disconnected(Some(e)))?;
        Ok(Connection {
            w: WriteHalf { stream, next_id: 1 },
            r: ReadHalf {
                reader: BufReader::new(read),
                buf: Vec::new(),
            },
            reply_timeout: crate::CLIENT_REPLY_TIMEOUT,
        })
    }

    /// Connects to [`socket_path`](crate::socket_path).
    pub fn open_default() -> Result<Connection, ClientError> {
        let path = crate::socket_path().ok_or(ClientError::NoSocketPath)?;
        Connection::open(&path)
    }

    /// How long a reply may take; `CLIENT_REPLY_TIMEOUT` unless set (tests shorten it).
    pub fn set_reply_timeout(&mut self, timeout: Duration) {
        self.reply_timeout = timeout;
    }

    pub fn reply_timeout(&self) -> Duration {
        self.reply_timeout
    }

    /// Sends one request with the next id (1 for the first request of a connection) and
    /// returns that id.
    pub fn send(&mut self, verb: Verb) -> Result<RequestId, ClientError> {
        self.w.send(verb)
    }

    /// The next line from the daemon, without its newline. `wait: None` waits as long as it
    /// takes; otherwise [`ClientError::Timeout`] once `wait` has passed. The end of the
    /// connection is [`ClientError::Disconnected`].
    pub fn read_line(&mut self, wait: Option<Duration>) -> Result<String, ClientError> {
        self.r.line(wait.map(|w| Instant::now() + w))
    }

    /// Sends `hello` and reads its reply within the reply timeout. A reply that is not a
    /// protocol-2 `hello` reply is [`ClientError::OldDaemon`] (spec 5.10); the error
    /// `protocol`, or a `hello` reply of another protocol, is [`ClientError::Protocol`].
    pub fn hello(&mut self, hello: Hello) -> Result<HelloReply, ClientError> {
        let id = self.send(Verb::Hello(hello))?;
        let line = self.read_line(Some(self.reply_timeout))?;
        let ok = hello_reply(&line, &id)?;
        Ok(HelloReply { ok, line })
    }
}

/// Classifies the first line a daemon sent in answer to `hello` with the id `id`.
pub(crate) fn hello_reply(line: &str, id: &RequestId) -> Result<HelloOk, ClientError> {
    let Ok(reply) = serde_json::from_str::<Reply>(line) else {
        return Err(ClientError::OldDaemon);
    };
    match reply.body {
        ReplyBody::Ok(OkBody::Hello(h)) if h.protocol != crate::PROTOCOL => {
            Err(ClientError::Protocol {
                supported: vec![h.protocol],
                message: format!("the daemon answered hello with protocol {}", h.protocol),
            })
        }
        ReplyBody::Ok(OkBody::Hello(h)) if reply.id.as_ref() == Some(id) => Ok(h),
        ReplyBody::Ok(OkBody::Hello(_)) => Err(ClientError::Malformed(format!(
            "the reply to hello carries another id: {:?}",
            reply.id
        ))),
        ReplyBody::Error(e) if e.code == ErrorCode::Protocol => Err(ClientError::Protocol {
            supported: e.supported.clone().unwrap_or_default(),
            message: e.message,
        }),
        ReplyBody::Error(e) => Err(ClientError::Refused(Box::new(e))),
        ReplyBody::Ok(_) => Err(ClientError::OldDaemon),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one() -> RequestId {
        RequestId::Int(1)
    }

    #[test]
    fn a_protocol_2_hello_reply_is_accepted() {
        let line = r#"{"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}"#;
        let ok = hello_reply(line, &one()).unwrap();
        assert_eq!(ok.daemon, "1.0.0");
        assert!(ok.devices.is_empty());
    }

    /// Spec 5.10: 0.13 answers a hello line with `ok:false` and a string `error`.
    #[test]
    fn the_reply_of_a_0_13_daemon_is_an_old_daemon() {
        for line in [
            r#"{"id":null,"ok":false,"error":"bad request: missing field `args` at line 1 column 83"}"#,
            r#"{"id":1,"ok":true,"result":{},"text":"ok"}"#,
            r#"{"event":"state","state":{"zoom":1.0}}"#,
            "not json",
            r#"{"id":1,"ok":{"rev":3}}"#,
        ] {
            assert!(
                matches!(hello_reply(line, &one()), Err(ClientError::OldDaemon)),
                "{line}"
            );
        }
    }

    #[test]
    fn the_error_protocol_names_the_daemons_protocols() {
        let line = r#"{"id":null,"error":{"code":"protocol","message":"protocol 2 is not supported","supported":[3]}}"#;
        match hello_reply(line, &one()) {
            Err(ClientError::Protocol { supported, message }) => {
                assert_eq!(supported, vec![3]);
                assert_eq!(message, "protocol 2 is not supported");
            }
            other => panic!("{other:?}"),
        }
        let newer = r#"{"id":1,"ok":{"protocol":3,"daemon":"2.0.0","devices":[],"states":{}}}"#;
        assert!(matches!(
            hello_reply(newer, &one()),
            Err(ClientError::Protocol { supported, .. }) if supported == vec![3]
        ));
    }

    #[test]
    fn another_error_to_hello_is_a_refusal() {
        let line = r#"{"id":1,"error":{"code":"invalid","message":"a second hello"}}"#;
        assert!(matches!(
            hello_reply(line, &one()),
            Err(ClientError::Refused(e)) if e.code == ErrorCode::Invalid
        ));
    }
}
```

Create `crates/obsbot-proto/src/client/blocking.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The blocking client: one request at a time on a connection with `subscribe: false`.

use crate::client::{ClientError, Connection};
use crate::description::Device;
use crate::message::{Hello, HelloOk, OkBody, ReplyBody, ServerMessage, Verb};
use crate::state::{Params, Patch, State};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

/// A connection that blocks per request (spec 6.1).
pub struct Client {
    conn: Connection,
    hello: HelloOk,
}

/// Action parameters as an object: `null` is no parameter, anything but an object is refused.
pub(crate) fn params_of(params: Value) -> Result<Params, ClientError> {
    match params {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(Params::new()),
        _ => Err(ClientError::NotAnObject),
    }
}

impl Client {
    /// Connects to [`socket_path`](crate::socket_path) and sends `hello` with
    /// `subscribe: false`, whatever `hello` says.
    pub fn connect(hello: Hello) -> Result<Client, ClientError> {
        let path = crate::socket_path().ok_or(ClientError::NoSocketPath)?;
        Client::connect_to(&path, hello)
    }

    /// As [`Client::connect`], on the socket at `path`.
    pub fn connect_to(path: &Path, hello: Hello) -> Result<Client, ClientError> {
        let mut conn = Connection::open(path)?;
        let reply = conn.hello(hello.subscribe(false))?;
        Ok(Client {
            conn,
            hello: reply.ok,
        })
    }

    /// The `hello` reply: daemon version, devices and their states at connect time.
    pub fn hello(&self) -> &HelloOk {
        &self.hello
    }

    /// The device list of the `hello` reply.
    pub fn devices(&self) -> &[Device] {
        &self.hello.devices
    }

    /// How long a reply may take; `CLIENT_REPLY_TIMEOUT` unless set.
    pub fn set_reply_timeout(&mut self, timeout: Duration) {
        self.conn.set_reply_timeout(timeout);
    }

    /// `get`: the full state (`paths: None`) or the paths asked for.
    pub fn get(
        &mut self,
        device: &str,
        paths: Option<&[&str]>,
    ) -> Result<(u64, State), ClientError> {
        let verb = Verb::Get {
            device: device.to_string(),
            paths: paths.map(|p| p.iter().map(|s| s.to_string()).collect()),
        };
        match self.call(verb)? {
            OkBody::State(rs) => Ok((rs.rev, rs.state)),
            _ => Err(ClientError::Malformed(
                "the reply to get holds no state".into(),
            )),
        }
    }

    /// `set`: one patch; returns the reply's rev.
    pub fn set(&mut self, device: &str, patch: Patch) -> Result<u64, ClientError> {
        let verb = Verb::Set {
            device: device.to_string(),
            patch,
        };
        match self.call(verb)? {
            OkBody::Rev(r) => Ok(r.rev),
            _ => Err(ClientError::Malformed(
                "the reply to set holds no rev".into(),
            )),
        }
    }

    /// `do`: one action with its parameters (a JSON object, or `null` for none); returns
    /// the reply's rev and the action's result, if it has one.
    pub fn act(
        &mut self,
        device: &str,
        action: &str,
        params: Value,
    ) -> Result<(u64, Option<Params>), ClientError> {
        let verb = Verb::Do {
            device: device.to_string(),
            action: action.to_string(),
            params: params_of(params)?,
        };
        match self.call(verb)? {
            OkBody::Rev(r) => Ok((r.rev, r.result)),
            _ => Err(ClientError::Malformed(
                "the reply to do holds no rev".into(),
            )),
        }
    }

    /// Sends one request and waits for its reply. Events are skipped; a reply without an id
    /// (an error the daemon could not tie to a request) is taken as this request's.
    fn call(&mut self, verb: Verb) -> Result<OkBody, ClientError> {
        let id = self.conn.send(verb)?;
        let deadline = Instant::now() + self.conn.reply_timeout();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = self.conn.read_line(Some(left))?;
            let message: ServerMessage = serde_json::from_str(&line)
                .map_err(|e| ClientError::Malformed(format!("{e}: {line}")))?;
            let ServerMessage::Reply(reply) = message else {
                continue;
            };
            if reply.id.is_some() && reply.id.as_ref() != Some(&id) {
                return Err(ClientError::Malformed(format!(
                    "a reply to another request: {line}"
                )));
            }
            return match reply.body {
                ReplyBody::Ok(body) => Ok(body),
                ReplyBody::Error(e) => Err(ClientError::Refused(Box::new(e))),
            };
        }
    }
}
```

Create `crates/obsbot-proto/src/client/script.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Test support, not part of the client API: a scripted daemon. It serves one transcript in
//! the form of `tests/golden/*.jsonl` to the first connection on a socket, so that client and
//! CLI tests run without a daemon (design spec 10.3, track C: before the fake daemon exists).
//!
//! Transcript lines:
//!
//! - `> {…}`: the client must send this line next; compared as JSON values;
//! - `< …`: written to the client unchanged (any text, so a 0.13 reply can be scripted);
//! - `!sleep <ms>`: waits;
//! - `!close`: closes the connection here;
//! - `#` lines and empty lines are skipped.
//!
//! After the last line the server reads until the client closes the connection: any further
//! line from the client is a mismatch. So a test sees every request a command sends, and
//! a command that sends one request too many fails.

use crate::values::same;
use serde_json::Value;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// How long the server waits for the client at each step.
const STEP: Duration = Duration::from_secs(10);

/// A scripted daemon on one socket, serving one connection.
pub struct ScriptServer {
    path: PathBuf,
    handle: Option<JoinHandle<Result<Vec<String>, String>>>,
}

impl ScriptServer {
    /// Binds `path` (a stale file there is removed) and serves `transcript` to the first
    /// connection, in a thread.
    pub fn start(path: &Path, transcript: &str) -> ScriptServer {
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).expect("bind the script socket");
        let script = transcript.to_string();
        let handle = thread::spawn(move || serve(listener, &script));
        ScriptServer {
            path: path.to_path_buf(),
            handle: Some(handle),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Waits for the script to end: the lines the client sent, or the first mismatch.
    pub fn finish(mut self) -> Result<Vec<String>, String> {
        let handle = self.handle.take().expect("finish runs once");
        handle
            .join()
            .unwrap_or_else(|_| Err("the script thread panicked".into()))
    }
}

impl Drop for ScriptServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn accept(listener: &UnixListener) -> Result<UnixStream, String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let until = Instant::now() + STEP;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                return Ok(stream);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < until => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(format!("no client connected: {e}")),
        }
    }
}

/// The next line from the client; `None` at the end of the connection.
fn next_line(reader: &mut BufReader<UnixStream>) -> Result<Option<String>, String> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(line.trim_end_matches('\n').to_string())),
        Err(e) => Err(format!("reading the client: {e}")),
    }
}

fn serve(listener: UnixListener, script: &str) -> Result<Vec<String>, String> {
    let stream = accept(&listener)?;
    stream
        .set_read_timeout(Some(STEP))
        .map_err(|e| e.to_string())?;
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut sent = Vec::new();
    for (n, line) in script.lines().enumerate() {
        let at = n + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(want) = line.strip_prefix("> ") {
            let want: Value = serde_json::from_str(want)
                .map_err(|e| format!("script line {at} is not JSON: {e}"))?;
            let got = next_line(&mut reader)?
                .ok_or_else(|| format!("line {at}: the client closed instead of sending {want}"))?;
            let value: Value = serde_json::from_str(&got)
                .map_err(|e| format!("line {at}: the client sent no JSON ({e}): {got}"))?;
            if !same(&value, &want) {
                return Err(format!("line {at}: expected {want}, got {got}"));
            }
            sent.push(got);
        } else if let Some(text) = line.strip_prefix("< ") {
            writer
                .write_all(format!("{text}\n").as_bytes())
                .map_err(|e| format!("line {at}: writing to the client: {e}"))?;
        } else if let Some(ms) = line.strip_prefix("!sleep ") {
            let ms: u64 = ms.parse().map_err(|_| format!("line {at}: bad !sleep"))?;
            thread::sleep(Duration::from_millis(ms));
        } else if line == "!close" {
            return Ok(sent);
        } else {
            return Err(format!("script line {at} is not '> ', '< ', '!' or '#'"));
        }
    }
    match next_line(&mut reader)? {
        None => Ok(sent),
        Some(extra) => Err(format!("the client sent one line more: {extra}")),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-proto`
Expected: `tests/client.rs` 13 passed; the lib's unit tests include the 4 of `client::connection`; every W0 test still passes.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: no output from fmt; clippy clean.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-proto/src/client.rs crates/obsbot-proto/src/client crates/obsbot-proto/tests/client.rs
git commit -m "obsbot-proto: the blocking client, its errors and a scripted daemon for tests"
```

### Task C-2: CLI skeleton: definition, errors, exit codes

**Files:**

- Modify: `crates/obsbot4linux/Cargo.toml` (the target and dependency tables only; `[package]` and every `[package.metadata.*]` table stay as they are)
- Replace: `crates/obsbot4linux/src/main.rs` (W0's stub)
- Create: `crates/obsbot4linux/src/lib.rs`, `src/args.rs`, `src/errors.rs`; the stubs `src/value.rs`, `src/select.rs`, `src/session.rs`, `src/render.rs`, `src/describe.rs`, `src/completions.rs`, `src/read.rs`, `src/write.rs`, `src/watch.rs` (later tasks replace each stub whole)
- Test: `crates/obsbot4linux/tests/common/mod.rs`, `crates/obsbot4linux/tests/cli_errors.rs`; unit tests in `src/args.rs` and `src/errors.rs`

**Interfaces:**

- Consumes (W0): `obsbot_proto::{Device, ErrorBody, ErrorCode, Support, short_id, socket_path}` and the fixture types of `tests/common` (`Description`, `HelloOk`, `Reply`, …).
- Produces (crate `obsbot4linux`):
  - `pub const VERSION: &str` (`CARGO_PKG_VERSION`); `pub struct Ctx<'a> { pub socket: Option<PathBuf>, pub device: Option<String>, pub json: bool, pub exit_on_signal: bool, pub out: &'a mut dyn Write, pub err: &'a mut dyn Write }`; `pub fn run(args: &[OsString], socket: Option<PathBuf>, exit_on_signal: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8`.
  - `args`: `pub struct Cli { pub json: bool, pub device: Option<String>, pub command: Command }`; `pub enum Command { Status, Get { paths: Vec<String> }, Set { assignments: Vec<String> }, Do { action: String, params: Vec<String> }, Describe { target: Option<String> }, Devices { report: bool }, Watch { live: Vec<String> } }`; `pub fn command() -> clap::Command`; `pub fn parse(args: &[OsString]) -> Result<Cli, clap::Error>`; `pub fn wants_json(args: &[OsString]) -> bool`.
  - `errors`: `pub enum CliCode { Proto(ErrorCode), Usage, AmbiguousDevice, NoDevice, Daemon }` with `as_str(&self) -> &str`, `exit(&self) -> u8`; `pub struct CliError { pub code: CliCode, pub message: String, pub devices: Vec<Device>, pub hint: Option<String>, pub body: Option<Box<ErrorBody>> }` with `new(CliCode, impl Into<String>)`, `usage(impl Into<String>)`, `refused(ErrorBody)`, `with_devices(Vec<Device>)`, `with_hint(impl Into<String>)`, `json_line(&self) -> String`, `human(&self) -> String`; `pub fn device_line(d: &Device) -> String`; `pub fn support_name(s: &Support) -> String`; `pub fn report(e: &CliError, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8`.
  - Signatures the stubs fix for C-9, C-10, C-11: `read::status(ctx: &mut Ctx) -> Result<(), CliError>`, `read::get(ctx: &mut Ctx, paths: &[String]) -> Result<(), CliError>`, `read::describe(ctx: &mut Ctx, target: Option<&str>) -> Result<(), CliError>`, `read::devices(ctx: &mut Ctx, report: bool) -> Result<(), CliError>`, `write::set(ctx: &mut Ctx, assignments: &[String]) -> Result<(), CliError>`, `write::act(ctx: &mut Ctx, action: &str, args: &[String]) -> Result<(), CliError>`, `watch::watch(ctx: &mut Ctx, live: &[String]) -> Result<(), CliError>`, and `write::KEEP: &str = "keep"` (C-8 uses it).
  - Test support `tests/common/mod.rs`: `socket(name: &str) -> PathBuf`, `cli(socket: Option<&Path>, args: &[&str]) -> (u8, String, String)`, `hello_line() -> String`, `hello_reply_line(devices: &[Device]) -> String`, `script(devices: &[Device], lines: &[&str]) -> String`, `fake_a()`, `fake_b()`, `unknown()`, `three() -> Vec<Device>`, `state_of(d: &Device) -> State`.

- [ ] **Step 1: Change the manifest**

In `crates/obsbot4linux/Cargo.toml`, replace the `[[bin]]`, `[dependencies]` and `[dev-dependencies]` tables (W0 left `[[bin]]`, `obsbot-proto` and `obsbot4linuxd`) with:

```toml
[lib]
path = "src/lib.rs"

[[bin]]
name = "obsbot4linux"
path = "src/main.rs"

[dependencies]
clap.workspace = true
nix = { workspace = true, features = ["signal"] }
obsbot-proto.workspace = true
serde.workspace = true
serde_json.workspace = true

[dev-dependencies]
obsbot4linuxd.workspace = true
```

- [ ] **Step 2: Write the failing tests**

Create `crates/obsbot4linux/tests/common/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Shared by the command-line tests: running the command line in-process, socket paths, and
//! the `hello` exchange of a daemon with three devices shaped like the fakes of spec 5.12
//! (`fake-a` with the Tiny 2 Lite's description, `fake-b`, and an unsupported `unknown`).

#![allow(dead_code)]

use obsbot_proto::{
    Description, Device, HelloOk, OkBody, Reply, ReplyBody, RequestId, RevState, State, Support,
    Usb,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// A socket path for one test, unique per name. The name is kept short: a Unix socket path
/// holds at most 107 bytes, and a daemon adds `.lock` and `.test` (5 bytes) to its path.
pub fn socket(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c-{name}"));
    assert!(
        path.as_os_str().len() <= 100,
        "socket path too long for a Unix socket: {}",
        path.display()
    );
    path
}

/// Runs the command line in-process against `socket`: (exit code, stdout, stderr).
pub fn cli(socket: Option<&Path>, args: &[&str]) -> (u8, String, String) {
    let args: Vec<OsString> = std::iter::once("obsbot4linux")
        .chain(args.iter().copied())
        .map(OsString::from)
        .collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = obsbot4linux::run(
        &args,
        socket.map(Path::to_path_buf),
        false,
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// The `hello` line the command line sends (spec 6.2: `subscribe: false`).
pub fn hello_line() -> String {
    format!(
        r#"> {{"id":1,"hello":{{"protocol":2,"client":"obsbot4linux {}","subscribe":false,"live":[]}}}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

fn description(file: &str) -> Description {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../descriptions")
        .join(file);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn device(id: &str, name: &str, model: &str, support: Support, product: &str, port: u32) -> Device {
    let serial = id.strip_prefix("sn:").map(str::to_string);
    let description = match model {
        "fake-a" => description("tiny2lite.json"),
        "fake-b" => description("fake/fake-b.json"),
        _ => Description::default(),
    };
    Device {
        id: id.into(),
        name: name.into(),
        model: model.into(),
        support,
        firmware: serial.as_ref().map(|_| "1.0.0.0".to_string()),
        serial,
        video_node: Some(format!("/dev/video{}", 100 + 2 * port)),
        available: true,
        replaced_by: None,
        usb: Usb {
            vendor: "3564".into(),
            product: product.into(),
            product_name: Some(name.trim_end_matches(char::is_numeric).trim().into()),
            bcd_device: "0100".into(),
            path: format!("9-{port}"),
            serial_present: false,
            interfaces: vec!["0e".into(), "0e".into()],
            extension_units: vec![],
        },
        description,
    }
}

pub fn fake_a() -> Device {
    device(
        "sn:FAKEA0000001",
        "Fake A 0001",
        "fake-a",
        Support::Tested,
        "fa0a",
        1,
    )
}

pub fn fake_b() -> Device {
    device(
        "sn:FAKEB0000001",
        "Fake B 0001",
        "fake-b",
        Support::Untested,
        "fa0b",
        2,
    )
}

pub fn unknown() -> Device {
    let mut d = device(
        "port:9-3",
        "Unknown Camera 9-3",
        "unknown",
        Support::Unsupported,
        "fa00",
        3,
    );
    d.usb.product_name = Some("Unknown Camera".into());
    d
}

/// A small state for each device.
pub fn state_of(d: &Device) -> State {
    let mut s = State::new(d.available);
    match d.model.as_str() {
        "fake-a" => {
            s.set("zoom.factor", json!(1.5));
            s.set("tracking.mode", json!("normal"));
            s.set(
                "presets",
                json!([{"slot": 0, "name": "", "pose": {"pan": 0, "tilt": 0, "zoom": 1}}]),
            );
        }
        "fake-b" => {
            s.set("led.brightness", json!(2));
            s.set("beacon.color", json!("red"));
        }
        _ => {}
    }
    s
}

/// The `hello` reply line (`< …`) of a daemon listing `devices`.
pub fn hello_reply_line(devices: &[Device]) -> String {
    let states: BTreeMap<String, RevState> = devices
        .iter()
        .map(|d| {
            (
                d.id.clone(),
                RevState {
                    rev: 1,
                    state: state_of(d),
                },
            )
        })
        .collect();
    let reply = Reply {
        id: Some(RequestId::Int(1)),
        body: ReplyBody::Ok(OkBody::Hello(HelloOk {
            protocol: 2,
            daemon: "1.0.0".into(),
            devices: devices.to_vec(),
            states,
        })),
    };
    format!("< {}", serde_json::to_string(&reply).unwrap())
}

/// A script: the hello exchange for `devices`, then `lines`.
pub fn script(devices: &[Device], lines: &[&str]) -> String {
    let mut all = vec![hello_line(), hello_reply_line(devices)];
    all.extend(lines.iter().map(|l| l.to_string()));
    all.join("\n")
}

/// The three devices of the fake daemon.
pub fn three() -> Vec<Device> {
    vec![fake_a(), fake_b(), unknown()]
}
```

Create `crates/obsbot4linux/tests/cli_errors.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Usage errors, help and version (design spec 6.2, "Errors"): nothing here needs a daemon.

mod common;

use common::cli;

/// Spec 10.3 C: the `--json` error shape for a usage error.
#[test]
fn a_usage_error_with_json_prints_the_error_object_on_stdout() {
    let (code, out, err) = cli(None, &["--json", "frobnicate"]);
    assert_eq!(code, 2);
    assert_eq!(err, "");
    let v: serde_json::Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["error"]["code"], "usage");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("frobnicate"),
        "{out}"
    );
    assert_eq!(v.as_object().unwrap().len(), 1);
    assert_eq!(v["error"].as_object().unwrap().len(), 2, "code and message");
}

#[test]
fn json_after_the_subcommand_counts_too() {
    let (code, out, _) = cli(None, &["get", "--json"]);
    assert_eq!(code, 2);
    assert!(
        out.starts_with(r#"{"error":{"code":"usage","message":"#),
        "{out}"
    );
}

#[test]
fn a_usage_error_without_json_goes_to_stderr() {
    let (code, out, err) = cli(None, &["set"]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.starts_with("error: "), "{err}");
    assert!(err.contains("Usage: obsbot4linux set"), "{err}");
}

/// Spec 6.2: help and version stay plain text, also with `--json`.
#[test]
fn help_and_version_are_plain_text_and_exit_0() {
    let (code, out, _) = cli(None, &["--json", "--help"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("Usage: obsbot4linux [OPTIONS] <COMMAND>"),
        "{out}"
    );
    for sub in ["status", "get", "set", "do", "describe", "devices", "watch"] {
        assert!(out.contains(&format!("  {sub}")), "{sub} in {out}");
    }
    let (code, out, _) = cli(None, &["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("obsbot4linux {}\n", env!("CARGO_PKG_VERSION")));
}

/// Parity exception PX-3: `--direct` is gone.
#[test]
fn the_0_13_direct_flag_is_a_usage_error() {
    let (code, _, err) = cli(None, &["--direct", "status"]);
    assert_eq!(code, 2);
    assert!(err.contains("--direct"), "{err}");
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p obsbot4linux --test cli_errors`
Expected: does not compile: there is no library `obsbot4linux` yet (`unresolved import` / `use of undeclared crate obsbot4linux`).

- [ ] **Step 4: Write the skeleton**

Create `crates/obsbot4linux/src/lib.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linux`: the command line (design spec 6.2), a pure client of protocol 2. Every
//! command sends the one request it maps to and chains no other (spec 2): `status` and `get`
//! a `get`, `set` a `set`, `do` a `do`; `describe`, `devices` and `watch` only `hello`.

pub mod args;
pub mod completions;
pub mod describe;
pub mod errors;
pub mod read;
pub mod render;
pub mod select;
pub mod session;
pub mod value;
pub mod watch;
pub mod write;

use args::Command;
use errors::{CliError, report};
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

/// The version this command line reports and names in its messages.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What every command gets.
pub struct Ctx<'a> {
    /// The daemon's socket (`None`: neither `OBSBOT4LINUX_SOCKET` nor `XDG_RUNTIME_DIR` set).
    pub socket: Option<PathBuf>,
    /// `--device` as given.
    pub device: Option<String>,
    /// `--json`.
    pub json: bool,
    /// `watch` exits 0 on SIGINT and SIGTERM. Only the binary sets it: in-process tests
    /// must not change the test process's signal handling.
    pub exit_on_signal: bool,
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

/// Runs one command line (`args` starts with the program name) and returns its exit code.
pub fn run(
    args: &[OsString],
    socket: Option<PathBuf>,
    exit_on_signal: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let cli = match args::parse(args) {
        Ok(cli) => cli,
        Err(e) => return clap_error(&e, args::wants_json(args), out, err),
    };
    let mut ctx = Ctx {
        socket,
        device: cli.device,
        json: cli.json,
        exit_on_signal,
        out,
        err,
    };
    let result = match &cli.command {
        Command::Status => read::status(&mut ctx),
        Command::Get { paths } => read::get(&mut ctx, paths),
        Command::Describe { target } => read::describe(&mut ctx, target.as_deref()),
        Command::Devices { report } => read::devices(&mut ctx, *report),
        Command::Set { assignments } => write::set(&mut ctx, assignments),
        Command::Do { action, params } => write::act(&mut ctx, action, params),
        Command::Watch { live } => watch::watch(&mut ctx, live),
    };
    match result {
        Ok(()) => 0,
        Err(e) => report(&e, ctx.json, ctx.out, ctx.err),
    }
}

/// Help and version print as plain text and exit 0 (spec 6.2: they stay plain text with
/// `--json`); every other parse error is `usage` (exit 2).
fn clap_error(e: &clap::Error, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    use clap::error::ErrorKind;
    let text = e.render().to_string();
    if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
        let _ = write!(out, "{text}");
        return 0;
    }
    if json {
        let first = text.lines().next().unwrap_or_default();
        let message = first.strip_prefix("error: ").unwrap_or(first);
        return report(&CliError::usage(message), true, out, err);
    }
    let _ = write!(err, "{text}");
    errors::CliCode::Usage.exit()
}
```

Replace `crates/obsbot4linux/src/main.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linux`: the command line (design spec 6.2). Everything is in the library, so that
//! tests run it in-process.

use std::ffi::OsString;
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let code = obsbot4linux::run(
        &args,
        obsbot_proto::socket_path(),
        true,
        &mut io::stdout(),
        &mut io::stderr(),
    );
    ExitCode::from(code)
}
```

Create `crates/obsbot4linux/src/args.rs` (with its unit tests):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command-line definition (design spec 6.2). It maps 1:1 onto protocol 2: no
//! per-feature subcommands and no shortcuts. The completions are generated from it
//! ([`command`]) and from the embedded description files.

use clap::{ColorChoice, CommandFactory, Parser, Subcommand};
use std::ffi::OsString;

/// obsbot4linux: control OBSBOT cameras through the obsbot4linux daemon.
#[derive(Parser, Debug, PartialEq)]
#[command(
    name = "obsbot4linux",
    version,
    disable_help_subcommand = true,
    color = ColorChoice::Never,
    after_help = "Values: bool on|off|true|false; enum by name; number 1.5; range min,max; \
pose pan,tilt,zoom; point x,y; string as given."
)]
pub struct Cli {
    /// Print the protocol objects as JSON
    #[arg(long, global = true)]
    pub json: bool,
    /// The device: its full id, a serial suffix of at least four characters, or a model key
    #[arg(long, global = true, value_name = "ID")]
    pub device: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Command {
    /// The state of one device, grouped
    Status,
    /// Read leaves or groups
    Get {
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<String>,
    },
    /// Write one patch
    Set {
        #[arg(required = true, value_name = "PATH=VALUE")]
        assignments: Vec<String>,
    },
    /// Run an action
    Do {
        #[arg(value_name = "ACTION")]
        action: String,
        #[arg(value_name = "PARAM=VALUE")]
        params: Vec<String>,
    },
    /// The device description, or one setting or action of it
    Describe {
        #[arg(value_name = "PATH|ACTION")]
        target: Option<String>,
    },
    /// The device list
    Devices {
        /// Print what an issue report needs (USB facts, no serial)
        #[arg(long)]
        report: bool,
    },
    /// Print the hello reply and every event as JSON lines
    Watch {
        /// Live topics to receive (pose, target, auto, …)
        #[arg(long, value_name = "TOPIC", num_args = 1..)]
        live: Vec<String>,
    },
}

/// The clap command, for the completion generator.
pub fn command() -> clap::Command {
    Cli::command()
}

/// Parses the arguments (the program name first).
pub fn parse(args: &[OsString]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(args)
}

/// Whether `--json` is among the arguments before a `--`. Used when parsing fails, so that a
/// usage error still prints JSON (spec 6.2, "Errors").
pub fn wants_json(args: &[OsString]) -> bool {
    args.iter()
        .skip(1)
        .take_while(|a| *a != "--")
        .any(|a| a == "--json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        std::iter::once("obsbot4linux")
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect()
    }

    #[test]
    fn the_definition_is_consistent() {
        command().debug_assert();
    }

    #[test]
    fn flags_go_before_or_after_the_subcommand() {
        let a = parse(&os(&["--json", "--device", "fake-a", "status"])).unwrap();
        let b = parse(&os(&["status", "--device", "fake-a", "--json"])).unwrap();
        assert_eq!(a, b);
        assert!(a.json);
        assert_eq!(a.device.as_deref(), Some("fake-a"));
        assert_eq!(a.command, Command::Status);
    }

    /// Spec 6.2: a key=value argument never starts with `-`; free text after `=` may.
    #[test]
    fn values_after_the_equals_sign_may_start_with_a_minus() {
        let cli = parse(&os(&["do", "gimbal.move", "pan=-20", "tilt=-5.5"])).unwrap();
        assert_eq!(
            cli.command,
            Command::Do {
                action: "gimbal.move".into(),
                params: vec!["pan=-20".into(), "tilt=-5.5".into()],
            }
        );
        let cli = parse(&os(&["set", "zone.area.pan=-20,20", "led.brightness=2"])).unwrap();
        assert_eq!(
            cli.command,
            Command::Set {
                assignments: vec!["zone.area.pan=-20,20".into(), "led.brightness=2".into()],
            }
        );
    }

    #[test]
    fn get_and_set_need_at_least_one_argument() {
        assert!(parse(&os(&["get"])).is_err());
        assert!(parse(&os(&["set"])).is_err());
        assert!(parse(&os(&["do"])).is_err());
    }

    #[test]
    fn watch_takes_live_topics() {
        let cli = parse(&os(&["watch", "--live", "pose", "target"])).unwrap();
        assert_eq!(
            cli.command,
            Command::Watch {
                live: vec!["pose".into(), "target".into()],
            }
        );
    }

    #[test]
    fn json_is_found_before_a_double_dash_only() {
        assert!(wants_json(&os(&["set", "--json", "x"])));
        assert!(!wants_json(&os(&["set", "--", "--json"])));
        assert!(!wants_json(&os(&["set", "x"])));
    }
}
```

Create `crates/obsbot4linux/src/errors.rs` (with its unit tests; `every_code_has_its_exit_code` is the exit-code table of spec 6.2, the CLI-only codes included):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Errors and exit codes of the command line (design spec 6.2, "Errors" and "Exit codes").
//! An error the daemon sent keeps its code; `usage`, `ambiguous_device`, `no_device` and
//! `daemon` exist only here and are never sent by the daemon.

use obsbot_proto::{Device, ErrorBody, ErrorCode, Support, short_id};
use serde_json::{Value, json};
use std::io::Write;

/// The code of an error the command line reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCode {
    /// A protocol error code: sent by the daemon, or found by the command line itself
    /// (`unknown_device`, `unsupported_device`, `unsupported`, `protocol`).
    Proto(ErrorCode),
    /// Bad arguments, a value that does not parse.
    Usage,
    /// `--device` matches several devices; several usable devices without it.
    AmbiguousDevice,
    /// No usable device.
    NoDevice,
    /// No socket, connection refused, no reply within `CLIENT_REPLY_TIMEOUT`.
    Daemon,
}

impl CliCode {
    /// The code as `--json` prints it.
    pub fn as_str(&self) -> &str {
        match self {
            CliCode::Proto(code) => code.as_str(),
            CliCode::Usage => "usage",
            CliCode::AmbiguousDevice => "ambiguous_device",
            CliCode::NoDevice => "no_device",
            CliCode::Daemon => "daemon",
        }
    }

    /// The exit code: 0 ok; 1 `internal`; 2 invalid input; 3 device not usable; 4 daemon
    /// unreachable. A code this client does not know counts as `internal` (spec 4.1 rule 7).
    pub fn exit(&self) -> u8 {
        match self {
            CliCode::Usage | CliCode::AmbiguousDevice => 2,
            CliCode::NoDevice => 3,
            CliCode::Daemon => 4,
            CliCode::Proto(code) => match code {
                ErrorCode::Invalid
                | ErrorCode::OutOfRange
                | ErrorCode::Unsupported
                | ErrorCode::UnknownDevice => 2,
                ErrorCode::Conflict
                | ErrorCode::Unavailable
                | ErrorCode::Asleep
                | ErrorCode::Busy
                | ErrorCode::Camera
                | ErrorCode::UnsupportedDevice => 3,
                ErrorCode::Protocol => 4,
                _ => 1,
            },
        }
    }
}

/// An error the command line reports and exits with.
#[derive(Debug, Clone, PartialEq)]
pub struct CliError {
    pub code: CliCode,
    /// English, one line; `--json` prints it as `message`.
    pub message: String,
    /// The devices the error is about (`ambiguous_device`, `no_device`): human output lists
    /// them with shortened ids, `--json` lists their full ids under `devices`.
    pub devices: Vec<Device>,
    /// A line human output prints last.
    pub hint: Option<String>,
    /// The daemon's error as it sent it; `--json` prints it whole.
    pub body: Option<Box<ErrorBody>>,
}

impl CliError {
    pub fn new(code: CliCode, message: impl Into<String>) -> CliError {
        CliError {
            code,
            message: message.into(),
            devices: Vec::new(),
            hint: None,
            body: None,
        }
    }

    pub fn usage(message: impl Into<String>) -> CliError {
        CliError::new(CliCode::Usage, message)
    }

    /// The error the daemon sent, with its code and every field.
    pub fn refused(body: ErrorBody) -> CliError {
        CliError {
            code: CliCode::Proto(body.code.clone()),
            message: body.message.clone(),
            devices: Vec::new(),
            hint: None,
            body: Some(Box::new(body)),
        }
    }

    pub fn with_devices(mut self, devices: Vec<Device>) -> CliError {
        self.devices = devices;
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> CliError {
        self.hint = Some(hint.into());
        self
    }

    /// The line `--json` prints: `{"error":{"code","message"}}`; the daemon's error body as
    /// it sent it (every field, in protocol order) when there is one; `devices` (full ids)
    /// when the error names devices.
    pub fn json_line(&self) -> String {
        let inner = match &self.body {
            Some(body) => serde_json::to_string(body).expect("an error body serialises"),
            None => {
                let mut error = json!({"code": self.code.as_str(), "message": self.message});
                if !self.devices.is_empty() {
                    let ids = self.devices.iter().map(|d| json!(d.id)).collect();
                    error["devices"] = Value::Array(ids);
                }
                error.to_string()
            }
        };
        format!("{{\"error\":{inner}}}")
    }

    /// The lines human output prints on stderr.
    pub fn human(&self) -> String {
        let mut out = format!("obsbot4linux: {}\n", self.message);
        for d in &self.devices {
            out.push_str(&format!("  {}\n", device_line(d)));
        }
        if let Some(body) = &self.body {
            if let Some(applied) = &body.applied {
                out.push_str(&format!("  applied: {}\n", applied.join(", ")));
            }
            if let Some(failed) = &body.failed {
                out.push_str(&format!("  failed: {failed}\n"));
            }
        }
        if let Some(hint) = &self.hint {
            out.push_str(&format!("{hint}\n"));
        }
        out
    }
}

/// One device as human output lists it: shortened id, name, model, and the support level
/// when it is not `tested` (spec 8.6), and `unavailable` while it is unplugged.
pub fn device_line(d: &Device) -> String {
    let mut line = format!("{}  {}  {}", short_id(&d.id), d.name, d.model);
    match &d.support {
        Support::Tested => {}
        Support::Untested => line.push_str("  untested"),
        Support::Unsupported => line.push_str("  unsupported"),
        other => line.push_str(&format!("  {}", support_name(other))),
    }
    if !d.available {
        line.push_str("  unavailable");
    }
    line
}

/// The wire name of a support level, a level this client does not know included.
pub fn support_name(s: &Support) -> String {
    match serde_json::to_value(s) {
        Ok(Value::String(name)) => name,
        _ => "unknown".into(),
    }
}

/// Prints an error: `--json` on stdout, human output on stderr. Returns the exit code.
pub fn report(e: &CliError, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    if json {
        let _ = writeln!(out, "{}", e.json_line());
    } else {
        let _ = write!(err, "{}", e.human());
    }
    e.code.exit()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec 6.2, "Exit codes": one row per code, the CLI-only codes included.
    #[test]
    fn every_code_has_its_exit_code() {
        let table: &[(CliCode, &str, u8)] = &[
            (CliCode::Proto(ErrorCode::Internal), "internal", 1),
            (CliCode::Proto(ErrorCode::Invalid), "invalid", 2),
            (CliCode::Proto(ErrorCode::OutOfRange), "out_of_range", 2),
            (CliCode::Proto(ErrorCode::Unsupported), "unsupported", 2),
            (
                CliCode::Proto(ErrorCode::UnknownDevice),
                "unknown_device",
                2,
            ),
            (CliCode::Usage, "usage", 2),
            (CliCode::AmbiguousDevice, "ambiguous_device", 2),
            (CliCode::Proto(ErrorCode::Conflict), "conflict", 3),
            (CliCode::Proto(ErrorCode::Unavailable), "unavailable", 3),
            (CliCode::Proto(ErrorCode::Asleep), "asleep", 3),
            (CliCode::Proto(ErrorCode::Busy), "busy", 3),
            (CliCode::Proto(ErrorCode::Camera), "camera", 3),
            (CliCode::NoDevice, "no_device", 3),
            (
                CliCode::Proto(ErrorCode::UnsupportedDevice),
                "unsupported_device",
                3,
            ),
            (CliCode::Daemon, "daemon", 4),
            (CliCode::Proto(ErrorCode::Protocol), "protocol", 4),
            // 4.1 rule 7: an unknown code is treated as internal.
            (
                CliCode::Proto(ErrorCode::Other("teapot".into())),
                "teapot",
                1,
            ),
        ];
        for (code, name, exit) in table {
            assert_eq!(code.as_str(), *name);
            assert_eq!(code.exit(), *exit, "{name}");
        }
        // Every code the daemon sends is in the table.
        for code in ErrorCode::ALL {
            assert!(
                table
                    .iter()
                    .any(|(c, ..)| *c == CliCode::Proto(code.clone())),
                "{}",
                code.as_str()
            );
        }
    }

    #[test]
    fn the_json_error_has_code_and_message() {
        let e = CliError::usage("zoom.factor: 'x' is not a number");
        assert_eq!(
            e.json_line(),
            r#"{"error":{"code":"usage","message":"zoom.factor: 'x' is not a number"}}"#
        );
    }

    #[test]
    fn a_daemon_error_keeps_every_field_it_sent() {
        let mut body = ErrorBody::new(ErrorCode::Camera, "the camera did not answer");
        body.rev = Some(9);
        body.applied = Some(vec!["exposure.mode".into()]);
        body.failed = Some("exposure.time".into());
        let e = CliError::refused(body);
        assert_eq!(e.code.exit(), 3);
        assert_eq!(
            e.json_line(),
            r#"{"error":{"code":"camera","message":"the camera did not answer","rev":9,"applied":["exposure.mode"],"failed":"exposure.time"}}"#
        );
        assert_eq!(
            e.human(),
            "obsbot4linux: the camera did not answer\n  applied: exposure.mode\n  failed: exposure.time\n"
        );
    }
}
```

Create the stubs. `crates/obsbot4linux/src/read.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The reading commands (design spec 6.2). Written in task C-9;
//! this stub fixes the signatures `lib.rs` calls.

use crate::Ctx;
use crate::errors::{CliCode, CliError};
use obsbot_proto::ErrorCode;

pub fn status(ctx: &mut Ctx) -> Result<(), CliError> {
    let _ = ctx;
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "status is not built yet (track C)",
    ))
}

pub fn get(ctx: &mut Ctx, paths: &[String]) -> Result<(), CliError> {
    let _ = (ctx, paths);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "get is not built yet (track C)",
    ))
}

pub fn describe(ctx: &mut Ctx, target: Option<&str>) -> Result<(), CliError> {
    let _ = (ctx, target);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "describe is not built yet (track C)",
    ))
}

pub fn devices(ctx: &mut Ctx, report: bool) -> Result<(), CliError> {
    let _ = (ctx, report);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "devices is not built yet (track C)",
    ))
}
```

`crates/obsbot4linux/src/write.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The writing commands (design spec 6.2). Written in task C-10;
//! this stub fixes the signatures `lib.rs` calls.

use crate::Ctx;
use crate::errors::{CliCode, CliError};
use obsbot_proto::ErrorCode;

/// The parameter that keeps a continuing action going after its connection closes
/// (spec 4.4, `gimbal.turn`).
pub const KEEP: &str = "keep";

pub fn set(ctx: &mut Ctx, assignments: &[String]) -> Result<(), CliError> {
    let _ = (ctx, assignments);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "set is not built yet (track C)",
    ))
}

pub fn act(ctx: &mut Ctx, action: &str, args: &[String]) -> Result<(), CliError> {
    let _ = (ctx, action, args);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "do is not built yet (track C)",
    ))
}
```

`crates/obsbot4linux/src/watch.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `watch` (design spec 6.2). Written in task C-11;
//! this stub fixes the signatures `lib.rs` calls.

use crate::Ctx;
use crate::errors::{CliCode, CliError};
use obsbot_proto::ErrorCode;

pub fn watch(ctx: &mut Ctx, live: &[String]) -> Result<(), CliError> {
    let _ = (ctx, live);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "watch is not built yet (track C)",
    ))
}
```

The six documentation-only stubs. `crates/obsbot4linux/src/value.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The value syntax of the command line (design spec 6.2, "Value syntax"). Written in
//! task C-4.
```

`crates/obsbot4linux/src/select.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Device selection (design spec 6.2, "Device selection"). Written in
//! task C-5.
```

`crates/obsbot4linux/src/session.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Connecting to the daemon, and what the command line says when that fails (design
//! spec 5.10 and 6.2). Written in
//! task C-5.
```

`crates/obsbot4linux/src/render.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Human output of states and device lists (design spec 6.2, "Output"). Written in
//! task C-6.
```

`crates/obsbot4linux/src/describe.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Human output of `describe` (design spec 6.2). Written in
//! task C-7.
```

`crates/obsbot4linux/src/completions.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Shell completions for bash and zsh (design spec 6.2, "Completions"). Written in
//! task C-8.
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot4linux`
Expected: `tests/cli_errors.rs` 5 passed; the lib's unit tests (`args`, `errors`) pass.

Run: `scripts/check-cli-deps.sh`
Expected: `CLI dependencies: clap nix obsbot-proto serde serde_json`, exit 0.

Run: `cargo run -q -p obsbot4linux -- --version; echo "exit=$?"`
Expected: `obsbot4linux <workspace version>` and `exit=0`.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add Cargo.lock crates/obsbot4linux
git commit -m "obsbot4linux: CLI skeleton with its definition, error codes and exit codes"
```

### Task C-3: Subscribed client

**Files:**

- Modify: `crates/obsbot-proto/src/client.rs` (two lines), `crates/obsbot-proto/src/client/connection.rs` (two methods)
- Create: `crates/obsbot-proto/src/client/subscribe.rs`
- Test: `crates/obsbot-proto/tests/client_subscribe.rs` (replays W0's golden `session.jsonl`)

**Interfaces:**

- Consumes (C-1): `Connection`, `WriteHalf::{next_id, send}`, `ReadHalf::line`, `blocking::params_of`, `ClientError`, `ScriptServer`; (W0) `Event`, `ReplyBody`, `ServerMessage`, `RequestId`, `Patch`, the golden transcript `crates/obsbot-proto/tests/golden/session.jsonl`.
- Produces: `#[non_exhaustive] pub enum Incoming { Reply { id: Option<RequestId>, body: ReplyBody }, Event(Event) }`; `pub struct Sender` with `get(&self, device: &str, paths: Option<&[&str]>) -> Result<RequestId, ClientError>`, `set(&self, device: &str, patch: Patch) -> Result<RequestId, ClientError>`, `act(&self, device: &str, action: &str, params: Value) -> Result<RequestId, ClientError>`, `live(&self, topics: &[&str]) -> Result<RequestId, ClientError>`, `set_reply_timeout(&mut self, timeout: Duration)`; `pub struct Receiver` implementing `Iterator<Item = Result<Incoming, ClientError>>` (ends after an error; closes the connection on drop); `Client::subscribe(hello: Hello) -> Result<(HelloOk, Sender, Receiver), ClientError>`, `Client::subscribe_to(path: &Path, hello: Hello) -> Result<(HelloOk, Sender, Receiver), ClientError>`; crate-internal `ReadHalf::stream(&self) -> Result<UnixStream, ClientError>`, `Connection::into_halves(self) -> (WriteHalf, ReadHalf)`.

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot-proto/tests/client_subscribe.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The subscribed client (spec 6.1) replays the golden session transcript of W0 and reports
//! a lost connection and an overdue reply (5.8).

use obsbot_proto::client::script::ScriptServer;
use obsbot_proto::client::{Client, ClientError, Incoming};
use obsbot_proto::{ErrorCode, Event, Hello, OkBody, Patch, ReplyBody, RequestId};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

fn socket(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c10-{name}.sock"))
}

fn golden(name: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn next(rx: &mut obsbot_proto::client::Receiver) -> Incoming {
    rx.next().expect("a message").expect("not an error")
}

fn reply_id(msg: &Incoming) -> Option<RequestId> {
    match msg {
        Incoming::Reply { id, .. } => id.clone(),
        _ => None,
    }
}

fn state_rev(msg: &Incoming) -> Option<u64> {
    match msg {
        Incoming::Event(Event::State { rev, .. }) => Some(*rev),
        _ => None,
    }
}

#[test]
fn the_golden_session_replays_through_sender_and_receiver() {
    let server = ScriptServer::start(&socket("session"), &golden("session.jsonl"));
    let (hello, sender, mut rx) =
        Client::subscribe_to(server.path(), Hello::new("widget 1.0").live(&["pose"])).unwrap();
    assert_eq!(hello.devices.len(), 2);
    assert_eq!(hello.states["sn:EXAMPLE0000001"].rev, 4);

    let id = sender
        .set("sn:EXAMPLE0000001", Patch::new().leaf("led.brightness", 2))
        .unwrap();
    assert_eq!(id, RequestId::Int(2));
    // 4.1 rule 3: the state event comes before the reply.
    assert_eq!(state_rev(&next(&mut rx)), Some(5));
    let reply = next(&mut rx);
    assert_eq!(reply_id(&reply), Some(RequestId::Int(2)));
    assert_eq!(state_rev(&next(&mut rx)), Some(6));

    assert_eq!(
        sender.get("sn:EXAMPLE0000001", Some(&["led"])).unwrap(),
        RequestId::Int(3)
    );
    match next(&mut rx) {
        Incoming::Reply {
            body: ReplyBody::Ok(OkBody::State(rs)),
            ..
        } => assert_eq!(rs.state.at("led.brightness"), Some(&json!(2))),
        other => panic!("{other:?}"),
    }

    sender
        .act("sn:EXAMPLE0000001", "preset.recall", json!({"slot": 0}))
        .unwrap();
    assert_eq!(reply_id(&next(&mut rx)), Some(RequestId::Int(4)));
    assert_eq!(state_rev(&next(&mut rx)), Some(7));

    sender.live(&["pose", "target"]).unwrap();
    match next(&mut rx) {
        Incoming::Reply {
            id: Some(RequestId::Int(5)),
            body: ReplyBody::Ok(OkBody::Empty(_)),
        } => {}
        other => panic!("{other:?}"),
    }

    sender.get("port:3-2.4", None).unwrap();
    assert_eq!(reply_id(&next(&mut rx)), Some(RequestId::Int(6)));
    sender
        .set("port:3-2.4", Patch::new().leaf("led.brightness", 2))
        .unwrap();
    match next(&mut rx) {
        Incoming::Reply {
            body: ReplyBody::Error(e),
            ..
        } => assert_eq!(e.code, ErrorCode::UnsupportedDevice),
        other => panic!("{other:?}"),
    }
    drop(sender);
    drop(rx);
    assert_eq!(server.finish().unwrap().len(), 7);
}

#[test]
fn unknown_event_types_are_skipped() {
    let server = ScriptServer::start(
        &socket("unknown-event"),
        &[
            r#"> {"id":1,"hello":{"protocol":2,"client":"test 1.0","subscribe":true,"live":[]}}"#,
            r#"< {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}"#,
            r#"< {"event":"frobnicate","from":"a newer daemon"}"#,
            r#"< {"event":"devices","devices":[]}"#,
            "!close",
        ]
        .join("\n"),
    );
    let (_, _sender, mut rx) = Client::subscribe_to(server.path(), Hello::new("test 1.0")).unwrap();
    assert_eq!(
        next(&mut rx),
        Incoming::Event(Event::Devices { devices: vec![] })
    );
    assert!(matches!(
        rx.next(),
        Some(Err(ClientError::Disconnected(None)))
    ));
    assert!(rx.next().is_none(), "the receiver ends after an error");
    server.finish().unwrap();
}

/// Spec 5.8: a reply that does not come within the reply timeout ends the receiver.
#[test]
fn an_overdue_reply_ends_the_receiver_with_a_timeout() {
    let server = ScriptServer::start(
        &socket("overdue"),
        &[
            r#"> {"id":1,"hello":{"protocol":2,"client":"test 1.0","subscribe":true,"live":[]}}"#,
            r#"< {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}"#,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","get":null}"#,
        ]
        .join("\n"),
    );
    let (_, mut sender, mut rx) =
        Client::subscribe_to(server.path(), Hello::new("test 1.0")).unwrap();
    sender.set_reply_timeout(Duration::from_millis(200));
    sender.get("sn:EXAMPLE0000001", None).unwrap();
    assert!(matches!(rx.next(), Some(Err(ClientError::Timeout))));
    assert!(rx.next().is_none());
    drop(sender);
    server.finish().unwrap();
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot-proto --test client_subscribe`
Expected: does not compile: `obsbot_proto::client` has no `Incoming`, and `Client` has no `subscribe_to`.

- [ ] **Step 3: Write the subscriber**

In `crates/obsbot-proto/src/client.rs`, add `mod subscribe;` after `pub mod script;` and `pub use subscribe::{Incoming, Receiver, Sender};` after `pub use error::ClientError;`, and replace the module's doc comment with:

```rust
//! The synchronous client (design spec 6.1). A [`Client`] blocks per request on its own
//! connection (`subscribe: false`). [`Client::subscribe`] returns the `hello` reply, a
//! [`Sender`] and a [`Receiver`]; a reader thread delivers replies (with their request ids)
//! and events in arrival order. State, patches and action parameters are path → JSON value;
//! there are no per-setting methods. There is no built-in retry: when the connection ends,
//! the caller decides whether to reconnect (4.1, "Reconnect").
```

In `crates/obsbot-proto/src/client/connection.rs`, add to `impl ReadHalf` after `fn line`:

```rust
    /// A second handle on the socket, to shut it down from another thread.
    pub(crate) fn stream(&self) -> Result<UnixStream, ClientError> {
        self.reader
            .get_ref()
            .try_clone()
            .map_err(|e| ClientError::Disconnected(Some(e)))
    }
```

and to `impl Connection` after `fn hello`:

```rust
    pub(crate) fn into_halves(self) -> (WriteHalf, ReadHalf) {
        (self.w, self.r)
    }
```

Create `crates/obsbot-proto/src/client/subscribe.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The subscribed client: a [`Sender`] for requests and a [`Receiver`] whose reader thread
//! delivers replies and events in arrival order (spec 6.1). A request without a reply after
//! the reply timeout ends the receiver with [`ClientError::Timeout`] (spec 5.8).

use crate::client::blocking::params_of;
use crate::client::connection::{ReadHalf, WriteHalf};
use crate::client::{Client, ClientError, Connection};
use crate::ids::RequestId;
use crate::message::{Event, Hello, HelloOk, ReplyBody, ServerMessage, Verb};
use crate::state::Patch;
use serde_json::Value;
use std::collections::HashMap;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// One message for a subscribed client.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Incoming {
    /// A reply. `id` is `None` only for an error the daemon could not tie to a request.
    Reply {
        id: Option<RequestId>,
        body: ReplyBody,
    },
    /// An event. Event types this crate does not know are skipped (spec 4.1 rule 7).
    Event(Event),
}

/// Requests sent and not answered yet: id → the time its reply is due.
type Pending = Arc<Mutex<HashMap<RequestId, Instant>>>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Sends requests on a subscribed connection; each call returns the request id its reply
/// will carry.
pub struct Sender {
    w: Arc<Mutex<WriteHalf>>,
    pending: Pending,
    timeout: Duration,
}

impl Sender {
    pub fn get(&self, device: &str, paths: Option<&[&str]>) -> Result<RequestId, ClientError> {
        self.send(Verb::Get {
            device: device.to_string(),
            paths: paths.map(|p| p.iter().map(|s| s.to_string()).collect()),
        })
    }

    pub fn set(&self, device: &str, patch: Patch) -> Result<RequestId, ClientError> {
        self.send(Verb::Set {
            device: device.to_string(),
            patch,
        })
    }

    pub fn act(&self, device: &str, action: &str, params: Value) -> Result<RequestId, ClientError> {
        self.send(Verb::Do {
            device: device.to_string(),
            action: action.to_string(),
            params: params_of(params)?,
        })
    }

    /// Replaces the connection's live topics (spec 4.1 rule 8).
    pub fn live(&self, topics: &[&str]) -> Result<RequestId, ClientError> {
        self.send(Verb::Live {
            topics: topics.iter().map(|t| t.to_string()).collect(),
        })
    }

    /// How long a reply may take; `CLIENT_REPLY_TIMEOUT` unless set.
    pub fn set_reply_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    fn send(&self, verb: Verb) -> Result<RequestId, ClientError> {
        // The pending entry exists before the line leaves, so a fast reply always finds it.
        // `pending` is not held across the write: the reader thread takes it for every reply
        // and must not wait behind a slow write.
        let mut w = lock(&self.w);
        let id = w.next_id();
        lock(&self.pending).insert(id.clone(), Instant::now() + self.timeout);
        w.send(verb).inspect_err(|_| {
            lock(&self.pending).remove(&id);
        })
    }
}

/// Replies and events in arrival order. It ends after an error: the connection ended
/// ([`ClientError::Disconnected`]), a line was unreadable, or a reply was overdue.
pub struct Receiver {
    rx: mpsc::Receiver<Result<Incoming, ClientError>>,
    pending: Pending,
    stream: UnixStream,
    done: bool,
}

impl Receiver {
    fn overdue(&self) -> bool {
        let now = Instant::now();
        lock(&self.pending).values().any(|due| *due <= now)
    }
}

impl Iterator for Receiver {
    type Item = Result<Incoming, ClientError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            let due = lock(&self.pending).values().min().copied();
            let got = match due {
                None => self.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(at) => self
                    .rx
                    .recv_timeout(at.saturating_duration_since(Instant::now())),
            };
            match got {
                Ok(Ok(item)) => return Some(Ok(item)),
                Ok(Err(e)) => {
                    self.done = true;
                    return Some(Err(e));
                }
                Err(RecvTimeoutError::Timeout) if self.overdue() => {
                    self.done = true;
                    let _ = self.stream.shutdown(Shutdown::Both);
                    return Some(Err(ClientError::Timeout));
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.done = true;
                    return None;
                }
            }
        }
    }
}

impl Drop for Receiver {
    /// Closes the connection, which ends the reader thread.
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

fn read_loop(mut r: ReadHalf, tx: mpsc::Sender<Result<Incoming, ClientError>>, pending: Pending) {
    loop {
        let item = match r.line(None) {
            Err(e) => {
                let _ = tx.send(Err(e));
                return;
            }
            Ok(line) => match serde_json::from_str::<ServerMessage>(&line) {
                Err(e) => {
                    let _ = tx.send(Err(ClientError::Malformed(format!("{e}: {line}"))));
                    return;
                }
                Ok(ServerMessage::Event(Event::Unknown)) => continue,
                Ok(ServerMessage::Event(event)) => Incoming::Event(event),
                Ok(ServerMessage::Reply(reply)) => {
                    if let Some(id) = &reply.id {
                        lock(&pending).remove(id);
                    }
                    Incoming::Reply {
                        id: reply.id,
                        body: reply.body,
                    }
                }
            },
        };
        if tx.send(Ok(item)).is_err() {
            return;
        }
    }
}

impl Client {
    /// Connects to [`socket_path`](crate::socket_path) and sends `hello` with
    /// `subscribe: true`, whatever `hello` says. Returns the `hello` reply (the caller keeps
    /// its own device list from it and from the `devices` events), the sender and the
    /// receiver.
    pub fn subscribe(hello: Hello) -> Result<(HelloOk, Sender, Receiver), ClientError> {
        let path = crate::socket_path().ok_or(ClientError::NoSocketPath)?;
        Client::subscribe_to(&path, hello)
    }

    /// As [`Client::subscribe`], on the socket at `path`.
    pub fn subscribe_to(
        path: &Path,
        hello: Hello,
    ) -> Result<(HelloOk, Sender, Receiver), ClientError> {
        let mut conn = Connection::open(path)?;
        let reply = conn.hello(hello.subscribe(true))?;
        let timeout = conn.reply_timeout();
        let (w, r) = conn.into_halves();
        let stream = r.stream()?;
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel();
        let reader_pending = Arc::clone(&pending);
        thread::Builder::new()
            .name("obsbot-client-reader".into())
            .spawn(move || read_loop(r, tx, reader_pending))
            .map_err(|e| ClientError::Disconnected(Some(e)))?;
        let sender = Sender {
            w: Arc::new(Mutex::new(w)),
            pending: Arc::clone(&pending),
            timeout,
        };
        let receiver = Receiver {
            rx,
            pending,
            stream,
            done: false,
        };
        Ok((reply.ok, sender, receiver))
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot-proto`
Expected: `tests/client_subscribe.rs` 3 passed, `tests/client.rs` still 13 passed.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot-proto/src/client.rs crates/obsbot-proto/src/client crates/obsbot-proto/tests/client_subscribe.rs
git commit -m "obsbot-proto: the subscribed client with sender, receiver and reply timeout"
```

### Task C-4: Value syntax

**Files:**

- Replace: `crates/obsbot4linux/src/value.rs` (C-2's stub)
- Test: unit tests in the same file

**Interfaces:**

- Consumes (C-2): `crate::VERSION`, `errors::CliError`; (W0) `values::number`, `Action`, `Description`, `Params`, `Patch`, `Setting`, `SettingType::components`.
- Produces: `pub fn split_assignment(arg: &str) -> Result<(&str, &str), CliError>`; `pub fn parse_value(path: &str, setting: &Setting, text: &str) -> Result<Value, CliError>`; `pub fn parse_patch(desc: &Description, args: &[String]) -> Result<Patch, CliError>`; `pub fn parse_params(action: Option<&Action>, args: &[String]) -> Result<Params, CliError>`.

- [ ] **Step 1: Write the failing test**

Append to the stub `crates/obsbot4linux/src/value.rs` (keep its license line; its doc comment is replaced in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::{Asleep, Unit};
    use serde_json::json;

    fn ok(setting: &Setting, text: &str) -> Value {
        parse_value("p", setting, text).unwrap()
    }

    fn usage(setting: &Setting, text: &str) -> String {
        let e = parse_value("p", setting, text).unwrap_err();
        assert_eq!(e.code.as_str(), "usage");
        assert_eq!(e.code.exit(), 2);
        e.message
    }

    #[test]
    fn bool_takes_on_off_true_false() {
        let s = Setting::boolean("Mirror");
        assert_eq!(ok(&s, "on"), json!(true));
        assert_eq!(ok(&s, "true"), json!(true));
        assert_eq!(ok(&s, "off"), json!(false));
        assert_eq!(ok(&s, "false"), json!(false));
        assert_eq!(
            usage(&s, "yes"),
            "p: expected on, off, true or false, got 'yes'"
        );
        usage(&s, "On");
        usage(&s, "1");
    }

    #[test]
    fn enum_takes_the_name_and_a_minus_for_an_underscore() {
        let s = Setting::enumeration("Mode", &["static", "upper_body", "50hz"]);
        assert_eq!(ok(&s, "upper_body"), json!("upper_body"));
        assert_eq!(ok(&s, "upper-body"), json!("upper_body"));
        assert_eq!(ok(&s, "50hz"), json!("50hz"));
        // Not a value of the description: sent as typed, the daemon answers it.
        assert_eq!(ok(&s, "sideways"), json!("sideways"));
        assert_eq!(ok(&s, "side-ways"), json!("side-ways"));
    }

    #[test]
    fn a_value_with_a_minus_of_its_own_stays_as_it_is() {
        let s = Setting::enumeration("Odd", &["a-b", "a_b"]);
        assert_eq!(ok(&s, "a-b"), json!("a-b"));
        assert_eq!(ok(&s, "a_b"), json!("a_b"));
    }

    #[test]
    fn number_is_decimal_and_integral_numbers_stay_integers() {
        let s = Setting::number("Zoom", 1.0, 4.0).unit(Unit::Factor);
        assert_eq!(ok(&s, "1.5").to_string(), "1.5");
        assert_eq!(ok(&s, "2").to_string(), "2");
        assert_eq!(ok(&s, "2.0").to_string(), "2");
        assert_eq!(ok(&s, "-20").to_string(), "-20");
        // Out of bounds is the daemon's to refuse (out_of_range), not a usage error.
        assert_eq!(ok(&s, "9").to_string(), "9");
        assert_eq!(usage(&s, "1,5"), "p: '1,5' is not a number");
        usage(&s, "");
        usage(&s, "inf");
        usage(&s, "NaN");
        usage(&s, "0x10");
    }

    #[test]
    fn range_is_min_comma_max() {
        let s = Setting::range("Pan area", -150.0, 150.0).unit(Unit::Deg);
        assert_eq!(ok(&s, "-20,20"), json!({"min": -20, "max": 20}));
        assert_eq!(ok(&s, "-20.5,20"), json!({"min": -20.5, "max": 20}));
        // Review focus: a quoted value with spaces after the commas.
        assert_eq!(ok(&s, "-20, 20"), json!({"min": -20, "max": 20}));
        assert_eq!(usage(&s, "20"), "p: expected min,max, got '20'");
        usage(&s, "1,2,3");
        usage(&s, "a,b");
        usage(&s, "-20,");
    }

    #[test]
    fn pose_point_and_box_take_their_components_in_order() {
        let pose = Setting::new(SettingType::Pose, "Pose");
        assert_eq!(
            ok(&pose, "0,5,1.2"),
            json!({"pan": 0, "tilt": 5, "zoom": 1.2})
        );
        assert_eq!(usage(&pose, "0,5"), "p: expected pan,tilt,zoom, got '0,5'");
        let point = Setting::new(SettingType::Point, "Point");
        assert_eq!(ok(&point, "0.5,0.25"), json!({"x": 0.5, "y": 0.25}));
        let bx = Setting::new(SettingType::Box, "Box");
        assert_eq!(
            ok(&bx, "0,0,1,1"),
            json!({"left": 0, "top": 0, "right": 1, "bottom": 1})
        );
    }

    /// Spec 6.2: free text after `=` may start with `-`; a string is taken as given.
    #[test]
    fn string_is_taken_as_given() {
        let s = Setting::string("Name", 32, true);
        assert_eq!(ok(&s, "Desk"), json!("Desk"));
        assert_eq!(ok(&s, "-x, y=z"), json!("-x, y=z"));
        assert_eq!(ok(&s, ""), json!(""));
    }

    #[test]
    fn a_list_and_an_unknown_type_cannot_be_written() {
        let list = Setting::list("Presets", &[("slot", Setting::number("Slot", 0.0, 2.0))]);
        assert_eq!(usage(&list, "x"), "p is a list and cannot be written");
        let future = Setting::new(SettingType::Other("colour".into()), "Colour");
        assert!(usage(&future, "red").contains("cannot write"));
    }

    fn desc() -> Description {
        Description::default()
            .setting(
                "zone.area.pan",
                Setting::range("Pan", -150.0, 150.0).asleep(Asleep::Unknown),
            )
            .setting("led.brightness", Setting::number("LED", 0.0, 3.0))
            .setting("image.mirror", Setting::boolean("Mirror"))
    }

    #[test]
    fn a_patch_holds_one_dotted_key_per_argument() {
        let args: Vec<String> = [
            "zone.area.pan=-20,20",
            "led.brightness=2",
            "image.mirror=on",
        ]
        .map(String::from)
        .to_vec();
        let patch = parse_patch(&desc(), &args).unwrap();
        assert_eq!(
            serde_json::to_string(&patch).unwrap(),
            r#"{"image.mirror":true,"led.brightness":2,"zone.area.pan":{"max":20,"min":-20}}"#
        );
    }

    /// The daemon answers paths it does not have (`unsupported`) and groups (`invalid`).
    #[test]
    fn a_path_the_description_lacks_is_sent_as_text() {
        let args: Vec<String> = ["image.gamma=1", "zone=1"].map(String::from).to_vec();
        let patch = parse_patch(&desc(), &args).unwrap();
        assert_eq!(patch.0["image.gamma"], json!("1"));
        assert_eq!(patch.0["zone"], json!("1"));
    }

    #[test]
    fn bad_assignments_are_usage_errors() {
        let one = |a: &str| parse_patch(&desc(), &[a.to_string()]).unwrap_err().message;
        assert_eq!(
            one("led.brightness"),
            "expected NAME=VALUE, got 'led.brightness'"
        );
        assert_eq!(one("=2"), "'=2' names no path before the '='");
        assert_eq!(
            one("led.brightness=x"),
            "led.brightness: 'x' is not a number"
        );
        let twice = ["led.brightness=1", "led.brightness=2"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            parse_patch(&desc(), &twice).unwrap_err().message,
            "led.brightness is named twice"
        );
    }

    #[test]
    fn parameters_use_the_same_syntax() {
        let action = Action::new("Save a view", Asleep::Ignored)
            .param("name", Setting::string("Name", 32, true))
            .param("pose", Setting::new(SettingType::Pose, "Pose").optional());
        let args: Vec<String> = ["name=Desk", "pose=0,5,1.2"].map(String::from).to_vec();
        let params = parse_params(Some(&action), &args).unwrap();
        assert_eq!(
            Value::Object(params),
            json!({"name": "Desk", "pose": {"pan": 0, "tilt": 5, "zoom": 1.2}})
        );
        // An unknown parameter or action goes to the daemon as text.
        let params = parse_params(Some(&action), &["colour=red".to_string()]).unwrap();
        assert_eq!(params["colour"], json!("red"));
        let params = parse_params(None, &["slot=1".to_string()]).unwrap();
        assert_eq!(params["slot"], json!("1"));
        assert!(parse_params(None, &[]).unwrap().is_empty());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot4linux --lib value`
Expected: does not compile: `parse_value`, `parse_patch`, `parse_params` are not found.

- [ ] **Step 3: Write the parser**

Replace the part of `crates/obsbot4linux/src/value.rs` before `#[cfg(test)]` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The value syntax of the command line (design spec 6.2, "Value syntax"): one syntax per
//! setting type, the same for `set` leaves and `do` parameters. Only the text is parsed
//! here; bounds, steps, writable values, read-only leaves and `requires` are the daemon's
//! to check, and its refusal keeps its code (the daemon stays authoritative, spec 6.1).

use crate::VERSION;
use crate::errors::CliError;
use obsbot_proto::values::number;
use obsbot_proto::{Action, Description, Params, Patch, Setting, SettingType};
use serde_json::{Map, Value};

/// Splits `PATH=VALUE` at its first `=`. The value may be empty and may start with `-`.
pub fn split_assignment(arg: &str) -> Result<(&str, &str), CliError> {
    match arg.split_once('=') {
        Some(("", _)) => Err(CliError::usage(format!(
            "'{arg}' names no path before the '='"
        ))),
        Some(pair) => Ok(pair),
        None => Err(CliError::usage(format!("expected NAME=VALUE, got '{arg}'"))),
    }
}

fn number_of(path: &str, text: &str) -> Result<Value, CliError> {
    match text.trim().parse::<f64>() {
        Ok(x) if x.is_finite() => Ok(number(x)),
        _ => Err(CliError::usage(format!("{path}: '{text}' is not a number"))),
    }
}

/// Parses the text of one value for `setting` (`path` names it in messages):
///
/// - bool: `on`, `off`, `true`, `false`;
/// - enum: the value's name; `-` may stand for `_` (`upper-body`);
/// - number: a decimal number (`1.5`, `-20`);
/// - range, pose, point, box: the components in their order, comma-separated
///   (`min,max`; `pan,tilt,zoom`; `x,y`; `left,top,right,bottom`);
/// - string: the text as given, empty included.
///
/// A list cannot be written, and a type this command line does not know cannot be parsed.
pub fn parse_value(path: &str, setting: &Setting, text: &str) -> Result<Value, CliError> {
    match &setting.kind {
        SettingType::Bool => match text {
            "on" | "true" => Ok(Value::Bool(true)),
            "off" | "false" => Ok(Value::Bool(false)),
            _ => Err(CliError::usage(format!(
                "{path}: expected on, off, true or false, got '{text}'"
            ))),
        },
        SettingType::Enum => {
            let underscored = text.replace('-', "_");
            let known = |v: &str| setting.values.iter().any(|x| x == v);
            if !known(text) && known(&underscored) {
                Ok(Value::String(underscored))
            } else {
                Ok(Value::String(text.to_string()))
            }
        }
        SettingType::Number => number_of(path, text),
        SettingType::String => Ok(Value::String(text.to_string())),
        SettingType::List => Err(CliError::usage(format!(
            "{path} is a list and cannot be written"
        ))),
        kind => match kind.components() {
            Some(names) => {
                let parts: Vec<&str> = text.split(',').collect();
                if parts.len() != names.len() {
                    return Err(CliError::usage(format!(
                        "{path}: expected {}, got '{text}'",
                        names.join(",")
                    )));
                }
                let mut object = Map::new();
                for (name, part) in names.iter().zip(parts) {
                    object.insert(name.to_string(), number_of(path, part)?);
                }
                Ok(Value::Object(object))
            }
            None => Err(CliError::usage(format!(
                "{path} has a setting type that obsbot4linux {VERSION} cannot write"
            ))),
        },
    }
}

/// `set PATH=VALUE …` → one patch with dotted keys. A path the description does not have
/// is sent with its text as a string, so that the daemon answers it (`unsupported`, or
/// `invalid` for a group). Naming a path twice is a usage error.
pub fn parse_patch(desc: &Description, args: &[String]) -> Result<Patch, CliError> {
    let mut patch = Patch::new();
    for arg in args {
        let (path, text) = split_assignment(arg)?;
        if patch.0.contains_key(path) {
            return Err(CliError::usage(format!("{path} is named twice")));
        }
        let value = match desc.settings.get(path) {
            Some(setting) => parse_value(path, setting, text)?,
            None => Value::String(text.to_string()),
        };
        patch.0.insert(path.to_string(), value);
    }
    Ok(patch)
}

/// `do ACTION PARAM=VALUE …` → the action's parameters. A parameter the action does not have
/// (or an action the description does not have) is sent with its text as a string, so that
/// the daemon answers it. Naming a parameter twice is a usage error.
pub fn parse_params(action: Option<&Action>, args: &[String]) -> Result<Params, CliError> {
    let mut params = Params::new();
    for arg in args {
        let (name, text) = split_assignment(arg)?;
        if params.contains_key(name) {
            return Err(CliError::usage(format!("{name} is named twice")));
        }
        let value = match action.and_then(|a| a.params.get(name)) {
            Some(setting) => parse_value(name, setting, text)?,
            None => Value::String(text.to_string()),
        };
        params.insert(name.to_string(), value);
    }
    Ok(params)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p obsbot4linux --lib value && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: 12 tests of `value::tests` pass; clean.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linux/src/value.rs
git commit -m "obsbot4linux: the value syntax per setting type"
```

### Task C-5: Device selection and session

**Files:**

- Replace: `crates/obsbot4linux/src/select.rs`, `crates/obsbot4linux/src/session.rs` (C-2's stubs)
- Test: unit tests in both files

**Interfaces:**

- Consumes (C-1): `obsbot_proto::client::{Client, ClientError}` with `Client::connect_to`, `Client::devices`; (C-2) `VERSION`, `CliCode`, `CliError` (`new`, `usage`, `refused`, `with_devices`, `with_hint`, `human`, `json_line`), `errors::device_line`; (W0) `Device`, `ErrorBody`, `ErrorCode`, `Hello`, `Support`, `Usb`, `Description`.
- Produces: `select`: `pub fn report_hint() -> String`, `pub fn usable(d: &Device) -> bool`, `pub fn matching<'a>(devices: &'a [Device], wanted: &str) -> Vec<&'a Device>`, `pub fn select<'a>(devices: &'a [Device], wanted: Option<&str>) -> Result<&'a Device, CliError>`, `pub fn refuse_unsupported(d: &Device) -> Result<(), CliError>`. `session`: `pub const ENABLE: &str = "systemctl --user enable --now obsbot4linux.socket"`, `pub const ISSUES: &str = "https://github.com/hellwand/obsbot4linux/issues"`, `pub fn client_name() -> String`, `pub fn connect(socket: Option<&Path>) -> Result<Client, CliError>`, `pub fn connect_device(socket: Option<&Path>, wanted: Option<&str>) -> Result<(Client, Device), CliError>`, `pub fn client_error(e: ClientError) -> CliError`.

- [ ] **Step 1: Write the failing tests**

Append to the stub `crates/obsbot4linux/src/select.rs`:

```rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use obsbot_proto::{Description, Usb};

    pub(crate) fn device(id: &str, model: &str, support: Support, available: bool) -> Device {
        let serial = id.strip_prefix("sn:").map(str::to_string);
        Device {
            id: id.into(),
            name: format!("{model} device"),
            model: model.into(),
            support,
            serial,
            firmware: None,
            video_node: Some("/dev/video0".into()),
            available,
            replaced_by: None,
            usb: Usb {
                vendor: "3564".into(),
                product: "fa00".into(),
                product_name: None,
                bcd_device: "0100".into(),
                path: "9-3".into(),
                serial_present: false,
                interfaces: vec!["0e".into()],
                extension_units: vec![],
            },
            description: Description::default(),
        }
    }

    fn three() -> Vec<Device> {
        vec![
            device("sn:FAKEA0000001", "fake-a", Support::Tested, true),
            device("sn:FAKEB0000001", "fake-b", Support::Untested, true),
            device("port:9-3", "unknown", Support::Unsupported, true),
        ]
    }

    fn ids(e: &CliError) -> Vec<&str> {
        e.devices.iter().map(|d| d.id.as_str()).collect()
    }

    #[test]
    fn device_takes_a_full_id_a_serial_suffix_or_a_model_key() {
        let list = three();
        assert_eq!(
            select(&list, Some("sn:FAKEB0000001")).unwrap().model,
            "fake-b"
        );
        assert_eq!(select(&list, Some("fake-a")).unwrap().id, "sn:FAKEA0000001");
        assert_eq!(select(&list, Some("B0000001")).unwrap().model, "fake-b");
        assert_eq!(select(&list, Some("unknown")).unwrap().id, "port:9-3");
        assert_eq!(select(&list, Some("port:9-3")).unwrap().model, "unknown");
    }

    #[test]
    fn a_suffix_needs_four_characters_and_must_be_unique() {
        let list = three();
        let e = select(&list, Some("001")).unwrap_err();
        assert_eq!(e.code.as_str(), "unknown_device");
        assert_eq!(e.code.exit(), 2);
        let e = select(&list, Some("0001")).unwrap_err();
        assert_eq!(e.code, CliCode::AmbiguousDevice);
        assert_eq!(e.code.exit(), 2);
        assert_eq!(ids(&e), ["sn:FAKEA0000001", "sn:FAKEB0000001"]);
    }

    #[test]
    fn without_device_the_only_usable_device_is_taken() {
        let mut list = three();
        let e = select(&list, None).unwrap_err();
        assert_eq!(e.code, CliCode::AmbiguousDevice);
        assert_eq!(ids(&e), ["sn:FAKEA0000001", "sn:FAKEB0000001"]);
        list[1].available = false;
        assert_eq!(select(&list, None).unwrap().model, "fake-a");
    }

    /// Spec 8.4 step 5: with a supported camera connected too, the unsupported one is ignored.
    #[test]
    fn unsupported_devices_never_count_as_usable() {
        let list = three();
        assert!(!usable(&list[2]));
        let only_unknown = vec![list[2].clone()];
        let e = select(&only_unknown, None).unwrap_err();
        assert_eq!(e.code, CliCode::NoDevice);
        assert_eq!(e.code.exit(), 3);
        assert_eq!(e.message, "no usable OBSBOT camera");
        assert_eq!(ids(&e), ["port:9-3"]);
        assert!(e.hint.as_deref().unwrap().contains("devices --report"));
    }

    #[test]
    fn an_empty_list_is_no_device() {
        let e = select(&[], None).unwrap_err();
        assert_eq!(e.code, CliCode::NoDevice);
        assert!(e.devices.is_empty());
    }

    #[test]
    fn an_unsupported_device_is_refused_with_the_report_hint() {
        let list = three();
        assert!(refuse_unsupported(&list[0]).is_ok());
        let e = refuse_unsupported(&list[2]).unwrap_err();
        assert_eq!(e.code.as_str(), "unsupported_device");
        assert_eq!(e.code.exit(), 3);
        assert_eq!(
            e.message,
            format!(
                "unknown device (USB 3564:fa00) is not supported by obsbot4linux {VERSION}. \
                 `obsbot4linux devices --report` prints what an issue report needs: \
                 https://github.com/hellwand/obsbot4linux/issues"
            )
        );
    }

    /// Human output shortens ids (spec 4.2); `--json` carries them in full.
    #[test]
    fn an_ambiguous_choice_lists_short_ids_for_people_and_full_ids_in_json() {
        let e = select(&three(), Some("0001")).unwrap_err();
        let human = e.human();
        assert!(human.contains("…0001  fake-a device  fake-a\n"), "{human}");
        assert!(!human.contains("FAKEA0000001"), "{human}");
        assert_eq!(
            e.json_line(),
            r#"{"error":{"code":"ambiguous_device","devices":["sn:FAKEA0000001","sn:FAKEB0000001"],"message":"--device 0001 matches 2 devices"}}"#
        );
    }

    /// Review focus: human output shows `…0001`; pasted into `--device` it matches nothing
    /// (exit 2). Accepting it is a product question of the plan, not built.
    #[test]
    fn a_pasted_short_id_matches_no_device() {
        let e = select(&three(), Some("…0001")).unwrap_err();
        assert_eq!(e.code.as_str(), "unknown_device");
        assert_eq!(e.code.exit(), 2);
    }

    /// Review focus: a support level from a newer daemon (4.1 rule 7) is shown as it is and
    /// never counts as usable.
    #[test]
    fn an_unknown_support_level_is_shown_and_not_usable() {
        let d = device(
            "sn:NEWER0000001",
            "newer",
            Support::Other("experimental".into()),
            true,
        );
        assert!(!usable(&d));
        assert_eq!(
            crate::errors::device_line(&d),
            "…0001  newer device  newer  experimental"
        );
        let e = select(std::slice::from_ref(&d), None).unwrap_err();
        assert_eq!(e.code, CliCode::NoDevice);
    }
}
```

Append to the stub `crates/obsbot4linux/src/session.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::ErrorBody;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn an_unreachable_daemon_is_exit_4_with_the_enable_line() {
        let e = client_error(ClientError::Connect {
            path: PathBuf::from("/run/user/1000/obsbot4linux.sock"),
            source: io::Error::from(io::ErrorKind::NotFound),
        });
        assert_eq!(e.code, CliCode::Daemon);
        assert_eq!(e.code.exit(), 4);
        assert!(e.message.contains("/run/user/1000/obsbot4linux.sock"));
        assert!(e.message.ends_with(ENABLE), "{}", e.message);
        let e = client_error(ClientError::NoSocketPath);
        assert_eq!(e.code.exit(), 4);
        assert!(e.message.ends_with(ENABLE));
        assert_eq!(client_error(ClientError::Timeout).code.exit(), 4);
        assert_eq!(client_error(ClientError::Disconnected(None)).code.exit(), 4);
    }

    #[test]
    fn an_older_daemon_is_exit_4_with_the_restart_message() {
        let e = client_error(ClientError::OldDaemon);
        assert_eq!(e.code.as_str(), "protocol");
        assert_eq!(e.code.exit(), 4);
        assert_eq!(
            e.message,
            "an obsbot4linux daemon older than 1.0 is running: restart it or log out"
        );
        let e = client_error(ClientError::Protocol {
            supported: vec![3],
            message: "protocol 2 is not supported".into(),
        });
        assert_eq!(
            e.message,
            "the obsbot4linux daemon speaks protocol 3, this client speaks 2: \
             install the same version of both and restart the daemon"
        );
        assert_eq!(e.code.exit(), 4);
    }

    #[test]
    fn a_refusal_keeps_the_daemons_code() {
        let e = client_error(ClientError::Refused(Box::new(ErrorBody::new(
            ErrorCode::OutOfRange,
            "led.brightness: 4 is above the maximum 3",
        ))));
        assert_eq!(e.code, CliCode::Proto(ErrorCode::OutOfRange));
        assert_eq!(e.code.exit(), 2);
        assert_eq!(e.message, "led.brightness: 4 is above the maximum 3");
        assert!(e.body.is_some());
    }

    #[test]
    fn an_unreadable_reply_is_internal() {
        let e = client_error(ClientError::Malformed("not JSON".into()));
        assert_eq!(e.code.exit(), 1);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot4linux --lib select session`
Expected: does not compile: `select`, `usable`, `refuse_unsupported`, `client_error`, `ENABLE` not found.

- [ ] **Step 3: Write selection and session**

Replace the part of `crates/obsbot4linux/src/select.rs` before `#[cfg(test)]` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Device selection (design spec 6.2, "Device selection", "Unsupported devices", "No usable
//! device"). Only the device list of the `hello` reply is used; nothing is sent.

use crate::VERSION;
use crate::errors::{CliCode, CliError};
use crate::session::ISSUES;
use obsbot_proto::{Device, ErrorCode, Support};

/// The hint that ends the `no_device` and `unsupported_device` messages.
pub fn report_hint() -> String {
    format!("`obsbot4linux devices --report` prints what an issue report needs: {ISSUES}")
}

/// Whether a command may use the device without `--device`: available, and `tested` or
/// `untested`. Unsupported devices never count as usable.
pub fn usable(d: &Device) -> bool {
    d.available && matches!(d.support, Support::Tested | Support::Untested)
}

/// The listed devices `--device` matches: by full id, by a serial suffix of at least four
/// characters, or by model key (`unknown` included).
pub fn matching<'a>(devices: &'a [Device], wanted: &str) -> Vec<&'a Device> {
    devices
        .iter()
        .filter(|d| {
            d.id == wanted
                || d.model == wanted
                || (wanted.chars().count() >= 4
                    && d.serial.as_deref().is_some_and(|s| s.ends_with(wanted)))
        })
        .collect()
}

/// The one device a command acts on. With `--device` it must match exactly one listed
/// device, whatever its support or availability; without, there must be exactly one usable
/// device.
pub fn select<'a>(devices: &'a [Device], wanted: Option<&str>) -> Result<&'a Device, CliError> {
    match wanted {
        Some(w) => {
            let found = matching(devices, w);
            match found.as_slice() {
                [one] => Ok(one),
                [] => Err(CliError::new(
                    CliCode::Proto(ErrorCode::UnknownDevice),
                    format!("--device {w} matches no listed device"),
                )
                .with_hint("`obsbot4linux devices` lists them")),
                many => Err(CliError::new(
                    CliCode::AmbiguousDevice,
                    format!("--device {w} matches {} devices", many.len()),
                )
                .with_devices(many.iter().map(|d| (*d).clone()).collect())),
            }
        }
        None => {
            let found: Vec<&Device> = devices.iter().filter(|d| usable(d)).collect();
            match found.as_slice() {
                [one] => Ok(one),
                [] => Err(CliError::new(CliCode::NoDevice, "no usable OBSBOT camera")
                    .with_devices(devices.to_vec())
                    .with_hint(report_hint())),
                many => Err(CliError::new(
                    CliCode::AmbiguousDevice,
                    format!("{} usable devices: choose one with --device", many.len()),
                )
                .with_devices(many.iter().map(|d| (*d).clone()).collect())),
            }
        }
    }
}

/// `status`, `get`, `set` and `do` refuse an unsupported device before anything is sent
/// (spec 6.2, exit 3, code `unsupported_device`).
pub fn refuse_unsupported(d: &Device) -> Result<(), CliError> {
    if d.support != Support::Unsupported {
        return Ok(());
    }
    Err(CliError::new(
        CliCode::Proto(ErrorCode::UnsupportedDevice),
        format!(
            "{} (USB {}:{}) is not supported by obsbot4linux {VERSION}. {}",
            d.name,
            d.usb.vendor,
            d.usb.product,
            report_hint()
        ),
    ))
}
```

Replace the part of `crates/obsbot4linux/src/session.rs` before `#[cfg(test)]` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Connecting to the daemon, and what the command line says when that fails (design spec
//! 5.10 and 6.2). The command line never starts, stops or restarts a daemon.

use crate::VERSION;
use crate::errors::{CliCode, CliError};
use crate::select::{refuse_unsupported, select};
use obsbot_proto::client::{Client, ClientError};
use obsbot_proto::{Device, ErrorCode, Hello};
use std::path::Path;

/// The line a user runs to start the daemon (spec 5.10).
pub const ENABLE: &str = "systemctl --user enable --now obsbot4linux.socket";

/// Where issue reports go (spec 6.2, 8.4).
pub const ISSUES: &str = "https://github.com/hellwand/obsbot4linux/issues";

/// The `client` of this command line's `hello`.
pub fn client_name() -> String {
    format!("obsbot4linux {VERSION}")
}

/// Connects and sends `hello` with `subscribe: false` (spec 6.2, "One-shot commands").
pub fn connect(socket: Option<&Path>) -> Result<Client, CliError> {
    let path = socket.ok_or_else(|| client_error(ClientError::NoSocketPath))?;
    Client::connect_to(path, Hello::new(&client_name())).map_err(client_error)
}

/// Connects, selects the device (`--device` or the only usable one, spec 6.2) and refuses an
/// unsupported one before anything is sent after `hello`. For `status`, `get`, `set`, `do`.
pub fn connect_device(
    socket: Option<&Path>,
    wanted: Option<&str>,
) -> Result<(Client, Device), CliError> {
    let client = connect(socket)?;
    let device = select(client.devices(), wanted)?.clone();
    refuse_unsupported(&device)?;
    Ok((client, device))
}

/// What the command line reports for a client error (spec 6.2 table, 5.10).
pub fn client_error(e: ClientError) -> CliError {
    match e {
        ClientError::NoSocketPath => CliError::new(
            CliCode::Daemon,
            format!(
                "no daemon socket: neither OBSBOT4LINUX_SOCKET nor XDG_RUNTIME_DIR is set; \
                 start the daemon with: {ENABLE}"
            ),
        ),
        ClientError::Connect { path, source } => CliError::new(
            CliCode::Daemon,
            format!(
                "cannot connect to the obsbot4linux daemon at {} ({source}); \
                 start it with: {ENABLE}",
                path.display()
            ),
        ),
        ClientError::Disconnected(_) => CliError::new(
            CliCode::Daemon,
            "the obsbot4linux daemon closed the connection",
        ),
        ClientError::Timeout => CliError::new(
            CliCode::Daemon,
            "the obsbot4linux daemon did not reply within 30 s",
        ),
        ClientError::OldDaemon => CliError::new(
            CliCode::Proto(ErrorCode::Protocol),
            "an obsbot4linux daemon older than 1.0 is running: restart it or log out",
        ),
        ClientError::Protocol { supported, .. } => {
            let numbers: Vec<String> = supported.iter().map(u32::to_string).collect();
            CliError::new(
                CliCode::Proto(ErrorCode::Protocol),
                format!(
                    "the obsbot4linux daemon speaks protocol {}, this client speaks 2: \
                     install the same version of both and restart the daemon",
                    numbers.join(", ")
                ),
            )
        }
        ClientError::Refused(body) => CliError::refused(*body),
        ClientError::NotAnObject => CliError::usage("action parameters must be an object"),
        other => CliError::new(
            CliCode::Proto(ErrorCode::Internal),
            format!("the obsbot4linux daemon sent something this client cannot read: {other}"),
        ),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot4linux --lib && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: the 9 tests of `select::tests` and the 4 of `session::tests` pass; clean.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linux/src/select.rs crates/obsbot4linux/src/session.rs
git commit -m "obsbot4linux: device selection, connecting and the messages for an unreachable daemon"
```

### Task C-6: Human output of states and devices

**Files:**

- Replace: `crates/obsbot4linux/src/render.rs` (C-2's stub)
- Test: unit tests in the same file (snapshots from W0's golden states and description files)

**Interfaces:**

- Consumes (C-2): `errors::device_line`; (W0) `Description`, `Device`, `Setting`, `SettingType`, `State`, `Unit`, `UnitSpec`, `HelloOk`, `ServerMessage`, the golden files `session.jsonl`, `hotplug.jsonl`, `descriptions/tiny2lite.json`, `descriptions/fake/fake-b.json`.
- Produces: `pub fn unit_suffix(unit: &Unit) -> String`; `pub fn value_text(setting: &Setting, v: &Value) -> String`; `pub fn item_text(setting: &Setting, item: &Value) -> String`; `pub fn state_text(desc: &Description, state: &State) -> String`; `pub fn status_text(device: &Device, state: &State) -> String`; `pub fn devices_text(devices: &[Device]) -> String`.

- [ ] **Step 1: Write the failing test**

Append to the stub `crates/obsbot4linux/src/render.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::{HelloOk, OkBody, Reply, ReplyBody, ServerMessage};
    use serde_json::json;

    const SESSION: &str = include_str!("../../obsbot-proto/tests/golden/session.jsonl");
    const HOTPLUG: &str = include_str!("../../obsbot-proto/tests/golden/hotplug.jsonl");
    const FAKE_B: &str = include_str!("../../../descriptions/fake/fake-b.json");
    const TINY2LITE: &str = include_str!("../../../descriptions/tiny2lite.json");

    fn messages(text: &str) -> Vec<ServerMessage> {
        text.lines()
            .filter_map(|l| l.strip_prefix("< "))
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn hello() -> HelloOk {
        match messages(SESSION).into_iter().next().unwrap() {
            ServerMessage::Reply(Reply {
                body: ReplyBody::Ok(OkBody::Hello(h)),
                ..
            }) => h,
            other => panic!("{other:?}"),
        }
    }

    fn event_state(text: &str, rev: u64) -> State {
        messages(text)
            .into_iter()
            .find_map(|m| match m {
                ServerMessage::Event(obsbot_proto::Event::State { rev: r, state, .. })
                    if r == rev =>
                {
                    Some(state)
                }
                _ => None,
            })
            .unwrap()
    }

    /// Spec 10.3 C: rendering snapshots from the golden states.
    #[test]
    fn the_golden_hello_renders_grouped_with_units() {
        let h = hello();
        let device = &h.devices[0];
        let state = &h.states[&device.id].state;
        assert_eq!(
            status_text(device, state),
            "…0001  Example Camera 0001  example\n\
             available: yes\n\
             asleep: no\n\
             gimbal\n  pan: 12.5°\n\
             led\n  brightness: 1\n\
             presets:\n  - name Desk, slot 0\n"
        );
    }

    #[test]
    fn a_pending_value_is_shown_and_marked() {
        let h = hello();
        let state = event_state(SESSION, 5);
        assert_eq!(
            state_text(&h.devices[0].description, &state),
            "available: yes\nasleep: no\ngimbal\n  pan: 12.5°\nled\n  brightness: 2 (pending)\n\
             presets:\n  - name Desk, slot 0\n"
        );
    }

    #[test]
    fn an_unplugged_device_and_an_unsupported_entry_render() {
        let h = hello();
        let state = event_state(HOTPLUG, 8);
        assert!(state_text(&h.devices[0].description, &state).starts_with("available: no\n"));
        let unknown = &h.devices[1];
        assert_eq!(
            status_text(unknown, &h.states[&unknown.id].state),
            "port:3-2.4  USB 3564:fa02 3-2.4  unknown  unsupported\navailable: yes\n"
        );
        assert_eq!(
            devices_text(&h.devices),
            "…0001  Example Camera 0001  example\nport:3-2.4  USB 3564:fa02 3-2.4  unknown  unsupported\n"
        );
    }

    #[test]
    fn null_unread_raw_and_unnamed_presets_render_as_display_only() {
        let desc: Description = serde_json::from_str(TINY2LITE).unwrap();
        let mut state = State::new(true);
        state.set("hand.return", Value::Null);
        state.set("exposure.anti_flicker", json!("raw:7"));
        state.set("zoom.factor", json!(1.5));
        state.set(
            "presets",
            json!([{"slot": 0, "name": "", "pose": {"pan": 10, "tilt": -5, "zoom": 1}}]),
        );
        state.set("zone.area.pan", json!({"min": -20, "max": 20}));
        state.unread.push("zoom.factor".into());
        assert_eq!(
            state_text(&desc, &state),
            "available: yes\n\
             unread: zoom.factor\n\
             exposure\n  anti_flicker: raw:7\n\
             hand\n  return: –\n\
             presets:\n  - name (unnamed), pose pan 10° tilt -5° zoom 1×, slot 0\n\
             zone\n  area.pan: min -20° max 20°\n\
             zoom\n  factor: 1.5× (unread)\n"
        );
    }

    /// fake-b: a group no client knows renders like any other; units from the description.
    #[test]
    fn a_description_of_another_shape_renders_without_model_knowledge() {
        let desc: Description = serde_json::from_str(FAKE_B).unwrap();
        let mut state = State::new(true);
        state.set("beacon.color", json!("green"));
        state.set("exposure.time", json!(2000));
        state.set("presets", json!([]));
        state.set("on_air.active", json!(true));
        state.set("on_air.apps", json!([{"name": "zoom"}]));
        assert_eq!(
            state_text(&desc, &state),
            "available: yes\n\
             beacon\n  color: green\n\
             exposure\n  time: 2000 µs\n\
             on_air\n  active: yes\n  apps:\n    - name zoom\n\
             presets: (none)\n"
        );
    }

    #[test]
    fn every_unit_has_a_suffix() {
        let units = [
            (Unit::Deg, "°"),
            (Unit::DegPerSecond, "°/s"),
            (Unit::Factor, "×"),
            (Unit::Microseconds, " µs"),
            (Unit::Kelvin, " K"),
            (Unit::Percent, "%"),
            (Unit::Seconds, " s"),
            (Unit::Fraction, ""),
            (Unit::Fps, " fps"),
            (Unit::Other("lux".into()), " lux"),
        ];
        for (unit, suffix) in units {
            assert_eq!(unit_suffix(&unit), suffix);
        }
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot4linux --lib render`
Expected: does not compile: `status_text`, `state_text`, `devices_text`, `unit_suffix` not found.

- [ ] **Step 3: Write the renderer**

Replace the part of `crates/obsbot4linux/src/render.rs` before `#[cfg(test)]` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Human output of states and device lists (design spec 6.2, "Output"): grouped by the first
//! path segment, with units, serials shortened (4.2), pending values marked `(pending)`.
//! Every label, unit and type comes from the device's description; nothing here knows a
//! model. `--json` never goes through this module.

use crate::errors::device_line;
use obsbot_proto::{Description, Device, Setting, SettingType, State, Unit, UnitSpec};
use serde_json::Value;

/// The suffix a unit adds to a number.
pub fn unit_suffix(unit: &Unit) -> String {
    match unit {
        Unit::Deg => "°".into(),
        Unit::DegPerSecond => "°/s".into(),
        Unit::Factor => "×".into(),
        Unit::Microseconds => " µs".into(),
        Unit::Kelvin => " K".into(),
        Unit::Percent => "%".into(),
        Unit::Seconds => " s".into(),
        Unit::Fraction => String::new(),
        Unit::Fps => " fps".into(),
        other => match serde_json::to_value(other) {
            Ok(Value::String(name)) => format!(" {name}"),
            _ => String::new(),
        },
    }
}

fn unit_of<'a>(setting: &'a Setting, component: Option<&str>) -> Option<&'a Unit> {
    match &setting.unit {
        Some(UnitSpec::One(unit)) => Some(unit),
        Some(UnitSpec::Each(map)) => component.and_then(|c| map.get(c)),
        None => None,
    }
}

fn scalar(setting: &Setting, component: Option<&str>, v: &Value) -> String {
    match v {
        Value::Null => "–".into(),
        Value::Bool(b) => match (setting.readonly, b) {
            (true, true) => "yes".into(),
            (true, false) => "no".into(),
            (false, true) => "on".into(),
            (false, false) => "off".into(),
        },
        Value::Number(n) => {
            let suffix = unit_of(setting, component)
                .map(unit_suffix)
                .unwrap_or_default();
            format!("{n}{suffix}")
        }
        Value::String(s) if s.is_empty() => "\"\"".into(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// One value of a setting on one line. `null` is `–`; a bool is `on`/`off` when writable and
/// `yes`/`no` when read-only; an object-shaped value lists its components in order
/// (`pan 12.5° tilt -3° zoom 1.5×`); a value of an unexpected shape, an undecodable `raw:<n>`
/// included, is shown as it is.
pub fn value_text(setting: &Setting, v: &Value) -> String {
    match (setting.kind.components(), v) {
        (Some(names), Value::Object(map)) => names
            .iter()
            .filter_map(|c| {
                map.get(*c)
                    .map(|x| format!("{c} {}", scalar(setting, Some(c), x)))
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => scalar(setting, None, v),
    }
}

/// One item of a list: its fields in name order, `field value`, joined by `, `. A `name`
/// that is empty shows as `(unnamed)`: display only, never written (spec 6.2, "Preset
/// names").
pub fn item_text(setting: &Setting, item: &Value) -> String {
    let Value::Object(fields) = item else {
        return item.to_string();
    };
    fields
        .iter()
        .map(|(name, v)| {
            let described = setting.item.as_ref().and_then(|i| i.get(name));
            let text = match (name.as_str(), v, described) {
                ("name", Value::String(s), _) if s.is_empty() => "(unnamed)".to_string(),
                (_, _, Some(field)) => value_text(field, v),
                (_, _, None) => v.to_string(),
            };
            format!("{name} {text}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The state, grouped: `available`, the `unread` paths if any, then every leaf of the
/// description that the state holds, in path order, under a line with its first segment.
/// A pending path shows its pending value, marked `(pending)`; an unread path is marked
/// `(unread)`. Paths the state does not hold are left out.
pub fn state_text(desc: &Description, state: &State) -> String {
    let mut out = format!(
        "available: {}\n",
        if state.available { "yes" } else { "no" }
    );
    if !state.unread.is_empty() {
        out.push_str(&format!("unread: {}\n", state.unread.join(", ")));
    }
    let mut group: Option<&str> = None;
    for (path, setting) in &desc.settings {
        let pending = state.pending.get(path);
        let Some(value) = pending.or_else(|| state.at(path)) else {
            continue;
        };
        let (indent, name) = match path.split_once('.') {
            Some((first, rest)) => {
                if group != Some(first) {
                    out.push_str(first);
                    out.push('\n');
                    group = Some(first);
                }
                ("  ", rest)
            }
            None => {
                group = None;
                ("", path.as_str())
            }
        };
        let mut marks = String::new();
        if pending.is_some() {
            marks.push_str(" (pending)");
        }
        if state.unread.iter().any(|p| p == path) {
            marks.push_str(" (unread)");
        }
        match (&setting.kind, value) {
            (SettingType::List, Value::Array(items)) => {
                let empty = if items.is_empty() { " (none)" } else { "" };
                out.push_str(&format!("{indent}{name}:{empty}{marks}\n"));
                for item in items {
                    out.push_str(&format!("{indent}  - {}\n", item_text(setting, item)));
                }
            }
            _ => out.push_str(&format!(
                "{indent}{name}: {}{marks}\n",
                value_text(setting, value)
            )),
        }
    }
    out
}

/// `status` in human form: the device line, then the state.
pub fn status_text(device: &Device, state: &State) -> String {
    format!(
        "{}\n{}",
        device_line(device),
        state_text(&device.description, state)
    )
}

/// `devices` in human form: one line per device.
pub fn devices_text(devices: &[Device]) -> String {
    devices
        .iter()
        .map(|d| format!("{}\n", device_line(d)))
        .collect()
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p obsbot4linux --lib render && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: the 6 tests of `render::tests` pass; clean.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linux/src/render.rs
git commit -m "obsbot4linux: grouped human output of states and device lists"
```

### Task C-7: Human output of descriptions

**Files:**

- Replace: `crates/obsbot4linux/src/describe.rs` (C-2's stub)
- Test: unit tests in the same file

**Interfaces:**

- Consumes (C-2): `errors::device_line`; (W0) `Action`, `Bound::get`, `Device`, `Setting`, `SettingType`, `UnitSpec`, `values::number`, the golden `session.jsonl`, `descriptions/tiny2lite.json`, `descriptions/fake/fake-b.json`.
- Produces: `pub fn setting_core(s: &Setting) -> String`; `pub fn setting_text(s: &Setting) -> String`; `pub fn action_text(a: &Action) -> String`; `pub fn device_text(d: &Device) -> String`.

- [ ] **Step 1: Write the failing test**

Append to the stub `crates/obsbot4linux/src/describe.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::{Description, HelloOk, OkBody, Reply, ReplyBody, ServerMessage};

    const SESSION: &str = include_str!("../../obsbot-proto/tests/golden/session.jsonl");
    const TINY2LITE: &str = include_str!("../../../descriptions/tiny2lite.json");
    const FAKE_B: &str = include_str!("../../../descriptions/fake/fake-b.json");

    fn hello() -> HelloOk {
        let line = SESSION.lines().find_map(|l| l.strip_prefix("< ")).unwrap();
        match serde_json::from_str::<ServerMessage>(line).unwrap() {
            ServerMessage::Reply(Reply {
                body: ReplyBody::Ok(OkBody::Hello(h)),
                ..
            }) => h,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_golden_device_entry_renders() {
        let h = hello();
        assert_eq!(
            device_text(&h.devices[0]),
            "…0001  Example Camera 0001  example\n\
             firmware: 1.2.3.4\n\
             video node: /dev/video2\n\
             usb: 3564:fa01 Example Camera, path 3-2.1\n\
             settings:\n\
             \x20 asleep: bool, read-only (Asleep)\n\
             \x20 gimbal.pan: number, unit deg, read-only, live pose (Pan)\n\
             \x20 led.brightness: number 0..3, step 1, asleep unknown, settle 3000 ms (LED brightness)\n\
             \x20 presets: list of name, slot, read-only (Presets)\n\
             actions:\n\
             \x20 preset.recall: slot (number 0..2, step 1), asleep unknown (Recall a preset)\n\
             limits:\n\
             \x20 presets: 3\n"
        );
        assert_eq!(
            device_text(&h.devices[1]),
            "port:3-2.4  USB 3564:fa02 3-2.4  unknown  unsupported\n\
             firmware: –\n\
             video node: /dev/video6\n\
             usb: 3564:fa02 –, path 3-2.4\n\
             settings: (none)\n\
             actions: (none)\n"
        );
    }

    #[test]
    fn the_tiny_2_lite_lines_show_every_constraint() {
        let d: Description = serde_json::from_str(TINY2LITE).unwrap();
        let s = |p: &str| setting_text(&d.settings[p]);
        let a = |n: &str| action_text(&d.actions[n]);
        assert_eq!(
            s("exposure.time"),
            "number 100..250000, unit us, asleep unknown, requires exposure.mode=manual, \
             default 33000, live auto, settle 3000 ms (Exposure time)"
        );
        assert_eq!(
            s("zoom.field_of_view"),
            "enum wide|medium|narrow (also shown: custom), asleep ignored, settle 3000 ms (Field of view)"
        );
        assert_eq!(
            s("zone.area.pan"),
            "range min -150..150, max -150..150, unit deg, asleep unknown, settle 3000 ms (Zone pan range)"
        );
        assert_eq!(
            s("led.blink"),
            "bool, no readback, asleep unknown (LED blink)"
        );
        assert_eq!(
            a("factory_reset"),
            "confirm (bool), asleep unknown, warning destructive, warning restart (Factory reset)"
        );
        assert_eq!(
            a("view.save"),
            "name (string of at most 32 characters, printable ASCII), \
             pose (pose pan -150..150, tilt -70..90, zoom 1..4, unit pan deg tilt deg zoom factor, optional), \
             asleep ignored, result id (number ≥ 0, step 1) (Save a view)"
        );
    }

    #[test]
    fn fake_b_lines_come_from_its_own_description() {
        let d: Description = serde_json::from_str(FAKE_B).unwrap();
        assert_eq!(
            setting_text(&d.settings["beacon.color"]),
            "enum red|green|blue, asleep accepted, settle 1000 ms (Beacon colour)"
        );
        assert_eq!(
            action_text(&d.actions["beacon.flash"]),
            "times (number 1..3, step 1), asleep accepted (Flash the beacon)"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot4linux --lib describe`
Expected: does not compile: `device_text`, `setting_text`, `action_text` not found.

- [ ] **Step 3: Write the description output**

Replace the part of `crates/obsbot4linux/src/describe.rs` before `#[cfg(test)]` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Human output of `describe` (design spec 6.2): a device entry, a setting or an action, one
//! line per setting and per action, every fact taken from the description. `--json` prints
//! the protocol objects instead and never goes through this module.

use crate::errors::device_line;
use obsbot_proto::{Action, Bound, Device, Setting, SettingType, UnitSpec};
use serde::Serialize;
use serde_json::Value;

/// The wire name of a protocol enum value (`range`, `deg/s`, `ignored`, `restart`).
fn wire<T: Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(Value::String(s)) => s,
        Ok(other) => other.to_string(),
        Err(_) => "?".into(),
    }
}

fn bound(b: &Option<Bound>, component: Option<&str>) -> Option<String> {
    b.as_ref()
        .and_then(|b| b.get(component))
        .map(|x| obsbot_proto::values::number(x).to_string())
}

/// `lo..hi`, `≥ lo`, `≤ hi`, or nothing.
fn bounds(s: &Setting, component: Option<&str>) -> Option<String> {
    match (bound(&s.min, component), bound(&s.max, component)) {
        (Some(lo), Some(hi)) => Some(format!("{lo}..{hi}")),
        (Some(lo), None) => Some(format!("≥ {lo}")),
        (None, Some(hi)) => Some(format!("≤ {hi}")),
        (None, None) => None,
    }
}

/// A setting without its label: type, values or bounds, step, unit, then its flags.
pub fn setting_core(s: &Setting) -> String {
    let mut out = wire(&s.kind);
    let mut parts: Vec<String> = Vec::new();
    match &s.kind {
        SettingType::Enum => {
            let shown = if s.writable.is_empty() {
                &s.values
            } else {
                &s.writable
            };
            out.push(' ');
            out.push_str(&shown.join("|"));
            let extra: Vec<&String> = s.values.iter().filter(|v| !shown.contains(v)).collect();
            if !extra.is_empty() {
                let extra: Vec<&str> = extra.iter().map(|v| v.as_str()).collect();
                out.push_str(&format!(" (also shown: {})", extra.join("|")));
            }
        }
        SettingType::String => {
            if let Some(n) = s.max_len {
                out.push_str(&format!(" of at most {n} characters"));
            }
            if s.ascii {
                parts.push("printable ASCII".into());
            }
        }
        SettingType::List => {
            if let Some(item) = &s.item {
                let names: Vec<&str> = item.keys().map(String::as_str).collect();
                out.push_str(&format!(" of {}", names.join(", ")));
            }
        }
        kind => match kind.components() {
            Some(names) => {
                let each: Vec<String> = names
                    .iter()
                    .filter_map(|c| bounds(s, Some(c)).map(|b| format!("{c} {b}")))
                    .collect();
                if !each.is_empty() {
                    out.push(' ');
                    out.push_str(&each.join(", "));
                }
            }
            None => {
                if let Some(b) = bounds(s, None) {
                    out.push(' ');
                    out.push_str(&b);
                }
                if let Some(step) = bound(&s.step, None) {
                    parts.push(format!("step {step}"));
                }
            }
        },
    }
    match &s.unit {
        Some(UnitSpec::One(unit)) => parts.push(format!("unit {}", wire(unit))),
        Some(UnitSpec::Each(units)) => {
            let each: Vec<String> = units
                .iter()
                .map(|(c, u)| format!("{c} {}", wire(u)))
                .collect();
            parts.push(format!("unit {}", each.join(" ")));
        }
        None => {}
    }
    if s.readonly {
        parts.push("read-only".into());
    }
    if s.nullable {
        parts.push("nullable".into());
    }
    if !s.readback {
        parts.push("no readback".into());
    }
    if s.no_effect {
        parts.push("no effect".into());
    }
    if s.optional {
        parts.push("optional".into());
    }
    if let Some(asleep) = &s.asleep {
        parts.push(format!("asleep {}", wire(asleep)));
    }
    for r in &s.requires {
        parts.push(format!("requires {}={}", r.path, plain(&r.equals)));
    }
    if let Some(d) = &s.default {
        parts.push(format!("default {}", plain(d)));
    }
    if let Some(topic) = &s.live {
        parts.push(format!("live {topic}"));
    }
    if let Some(ms) = s.settle_ms {
        parts.push(format!("settle {ms} ms"));
    }
    for w in &s.warning {
        parts.push(format!("warning {}", wire(w)));
    }
    for p in parts {
        out.push_str(", ");
        out.push_str(&p);
    }
    out
}

/// A JSON value as text without the quotes of a string.
fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// A setting on one line, its label last.
pub fn setting_text(s: &Setting) -> String {
    format!("{} ({})", setting_core(s), s.label)
}

/// An action on one line: its parameters, asleep behaviour, conditions, warnings and result,
/// its label last.
pub fn action_text(a: &Action) -> String {
    let mut parts: Vec<String> = Vec::new();
    if a.params.is_empty() {
        parts.push("no parameters".into());
    }
    for (name, p) in &a.params {
        parts.push(format!("{name} ({})", setting_core(p)));
    }
    parts.push(format!("asleep {}", wire(&a.asleep)));
    for r in &a.requires {
        parts.push(format!("requires {}={}", r.path, plain(&r.equals)));
    }
    for w in &a.warning {
        parts.push(format!("warning {}", wire(w)));
    }
    if let Some(result) = &a.result {
        for (name, r) in result {
            parts.push(format!("result {name} ({})", setting_core(r)));
        }
    }
    format!("{} ({})", parts.join(", "), a.label)
}

/// The device entry: its line, firmware, video node, USB facts, then every setting, action
/// and limit.
pub fn device_text(d: &Device) -> String {
    let dash = |v: &Option<String>| v.clone().unwrap_or_else(|| "–".into());
    let mut out = format!("{}\n", device_line(d));
    out.push_str(&format!("firmware: {}\n", dash(&d.firmware)));
    out.push_str(&format!("video node: {}\n", dash(&d.video_node)));
    out.push_str(&format!(
        "usb: {}:{} {}, path {}\n",
        d.usb.vendor,
        d.usb.product,
        dash(&d.usb.product_name),
        d.usb.path
    ));
    let desc = &d.description;
    if desc.settings.is_empty() {
        out.push_str("settings: (none)\n");
    } else {
        out.push_str("settings:\n");
        for (path, s) in &desc.settings {
            out.push_str(&format!("  {path}: {}\n", setting_text(s)));
        }
    }
    if desc.actions.is_empty() {
        out.push_str("actions: (none)\n");
    } else {
        out.push_str("actions:\n");
        for (name, a) in &desc.actions {
            out.push_str(&format!("  {name}: {}\n", action_text(a)));
        }
    }
    if !desc.limits.is_empty() {
        out.push_str("limits:\n");
        for (path, n) in &desc.limits {
            out.push_str(&format!("  {path}: {n}\n"));
        }
    }
    out
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p obsbot4linux --lib describe && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: the 3 tests of `describe::tests` pass; clean. If a label of `descriptions/tiny2lite.json` changed on `rebuild` after W0 (track A owns that file), take the label from the file: the test pins the format, not the label.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linux/src/describe.rs
git commit -m "obsbot4linux: human output of a device description"
```

### Task C-8: Completions and their stale test

**Files:**

- Create: `crates/obsbot4linux/build.rs`, `packaging/completions/obsbot4linux.bash`, `packaging/completions/_obsbot4linux` (generated)
- Replace: `crates/obsbot4linux/src/completions.rs` (C-2's stub)
- Modify: `scripts/regenerate.sh` (one line and its comment; see note 1 at the end)
- Test: `crates/obsbot4linux/tests/completions.rs`; unit tests in `src/completions.rs`

**Interfaces:**

- Consumes (C-2): `args::command()`, `write::KEEP`; (W0) `Description`, `Setting`, `SettingType`, `state::DAEMON_PATHS`, `descriptions/*.json`, `descriptions/fake/fake-b.json` (test only).
- Produces: `build.rs` writes `$OUT_DIR/descriptions.rs` with `pub static EMBEDDED: &[(&str, &str)]` (model key, JSON of every `descriptions/*.json`, not `descriptions/fake/`); `completions`: `pub fn descriptions() -> Vec<(&'static str, Description)>`, `pub struct Words { pub subcommands: Vec<String>, pub global_flags: Vec<String>, pub flags: BTreeMap<String, Vec<String>>, pub get_paths: Vec<String>, pub describe_names: Vec<String>, pub set_paths: Vec<String>, pub actions: Vec<String>, pub params: BTreeMap<String, Vec<String>>, pub values: BTreeMap<String, Vec<String>>, pub topics: Vec<String> }`, `pub fn words_of(descriptions: &[(&str, Description)]) -> Words`, `pub fn words() -> Words`, `pub fn bash() -> String`, `pub fn zsh() -> String`.

- [ ] **Step 1: Write the failing tests**

Create `crates/obsbot4linux/tests/completions.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The completions (design spec 6.2): the committed files under `packaging/completions/` are
//! current (`scripts/regenerate.sh` rewrites them, `OBSBOT_REGENERATE=1`), and the bash script
//! completes what the spec lists.

use obsbot4linux::completions::{bash, zsh};
use std::path::PathBuf;
use std::process::Command;

fn file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packaging/completions")
        .join(name)
}

#[test]
fn the_completion_files_are_current() {
    for (name, want) in [("obsbot4linux.bash", bash()), ("_obsbot4linux", zsh())] {
        let path = file(name);
        if std::env::var_os("OBSBOT_REGENERATE").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &want).unwrap();
            continue;
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            have == want,
            "packaging/completions/{name} is stale: run scripts/regenerate.sh and commit the result"
        );
    }
}

/// What bash offers for `line` (the cursor at its end), with `=` in COMP_WORDBREAKS (bash's
/// default) or without it.
fn complete(line: &str, equals_breaks: bool) -> Vec<String> {
    let breaks = if equals_breaks {
        " \t\n\"'><=;|&(:"
    } else {
        " \t\n\"'><;|&(:"
    };
    let out = Command::new("bash")
        .arg("-c")
        .arg(
            r#"source "$SCRIPT"; COMP_WORDBREAKS=$BREAKS; COMP_LINE=$LINE; COMP_POINT=${#COMP_LINE};
               _obsbot4linux; printf '%s\n' "${COMPREPLY[@]}""#,
        )
        .env("SCRIPT", file("obsbot4linux.bash"))
        .env("LINE", line)
        .env("BREAKS", breaks)
        .output()
        .expect("bash runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

#[test]
fn bash_completes_subcommands_and_flags() {
    assert_eq!(complete("obsbot4linux s", true), ["status", "set"]);
    assert_eq!(
        complete("obsbot4linux --device fake-a st", true),
        ["status"]
    );
    assert_eq!(complete("obsbot4linux devices --r", true), ["--report"]);
    assert_eq!(complete("obsbot4linux --j", true), ["--json"]);
    assert!(
        complete("obsbot4linux --device ", true).is_empty(),
        "no --device values"
    );
    assert_eq!(complete("obsbot4linux watch --live p", true), ["pose"]);
}

#[test]
fn bash_completes_paths_values_actions_and_parameters() {
    assert_eq!(
        complete("obsbot4linux get zone.area.p", true),
        ["zone.area.pan"]
    );
    assert_eq!(
        complete("obsbot4linux describe preset.sa", true),
        ["preset.save"]
    );
    assert_eq!(
        complete("obsbot4linux set tracking.mo", true),
        ["tracking.mode="]
    );
    assert_eq!(
        complete("obsbot4linux set tracking.mode=up", true),
        ["upper_body"]
    );
    assert_eq!(
        complete("obsbot4linux set tracking.mode=up", false),
        ["tracking.mode=upper_body"]
    );
    assert_eq!(
        complete("obsbot4linux set image.mirror=o", true),
        ["off", "on"]
    );
    assert_eq!(complete("obsbot4linux do zoom.mo", true), ["zoom.move"]);
    assert_eq!(
        complete("obsbot4linux do zoom.move ", true),
        ["factor=", "speed="]
    );
    assert_eq!(
        complete("obsbot4linux do zoom.move speed=", true),
        ["fast", "medium", "slow"]
    );
    assert_eq!(
        complete("obsbot4linux do gimbal.turn ", true),
        ["pan_speed=", "tilt_speed="],
        "keep is sent by the command line itself"
    );
}

/// The zsh script must at least parse. zsh is not on every CI runner: run with
/// `cargo test -p obsbot4linux --test completions -- --ignored` where zsh is installed.
#[test]
#[ignore = "needs zsh"]
fn the_zsh_script_parses() {
    let status = Command::new("zsh")
        .arg("-n")
        .arg(file("_obsbot4linux"))
        .status()
        .expect("zsh runs");
    assert!(status.success());
}
```

Append to the stub `crates/obsbot4linux/src/completions.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_models_are_embedded() {
        let keys: Vec<&str> = descriptions().iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"tiny2lite"), "{keys:?}");
        assert!(!keys.iter().any(|k| k.starts_with("fake")), "{keys:?}");
    }

    #[test]
    fn the_words_come_from_the_definition_and_the_descriptions() {
        let w = words();
        assert_eq!(
            w.subcommands,
            ["status", "get", "set", "do", "describe", "devices", "watch"]
        );
        assert_eq!(
            w.global_flags,
            ["--json", "--device", "--help", "--version"]
        );
        assert_eq!(w.flags["devices"], ["--report"]);
        assert_eq!(w.flags["watch"], ["--live"]);
        assert!(w.set_paths.contains(&"zoom.factor=".to_string()));
        assert!(
            !w.set_paths.contains(&"gimbal.pan=".to_string()),
            "read-only leaves are not set"
        );
        assert!(w.get_paths.contains(&"zone.area".to_string()), "groups");
        assert!(
            w.get_paths.contains(&"pending".to_string()),
            "daemon fields"
        );
        assert!(w.describe_names.contains(&"preset.save".to_string()));
        assert_eq!(w.values["zoom.move speed"], ["fast", "medium", "slow"]);
        assert_eq!(w.values["image.mirror"], ["false", "off", "on", "true"]);
        assert_eq!(
            w.values["zoom.field_of_view"],
            ["medium", "narrow", "wide"],
            "only writable values"
        );
        assert_eq!(
            w.params["gimbal.turn"],
            ["pan_speed=", "tilt_speed="],
            "no keep"
        );
        assert!(!w.params.contains_key("gimbal.stop"));
        assert_eq!(w.topics, ["auto", "pose", "target"]);
    }

    #[test]
    fn a_second_model_adds_its_words_without_code() {
        let fake_b: Description =
            serde_json::from_str(include_str!("../../../descriptions/fake/fake-b.json")).unwrap();
        let mut all = descriptions();
        all.push(("fake-b", fake_b));
        let w = words_of(&all);
        assert!(w.set_paths.contains(&"beacon.color=".to_string()));
        assert_eq!(w.params["beacon.flash"], ["times="]);
        // A path both models have: the values of both.
        assert!(w.values["tracking.mode"].contains(&"person".to_string()));
        assert!(w.values["tracking.mode"].contains(&"upper_body".to_string()));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p obsbot4linux --test completions`
Expected: does not compile: `obsbot4linux::completions` has no `bash` or `zsh`.

- [ ] **Step 3: Write the build script and the generator**

Create `crates/obsbot4linux/build.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Embeds every `descriptions/*.json` present at build time (the real models; never
//! `descriptions/fake/`) for the completion generator (design spec 6.2): a new model's file
//! needs no CLI code.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let dir = manifest.join("../../descriptions");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut out = String::from(
        "/// The description files of the real models at build time: (model key, JSON).\n\
         pub static EMBEDDED: &[(&str, &str)] = &[\n",
    );
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.display());
        let key = file
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a UTF-8 file name");
        let path = fs::canonicalize(file).expect("the description file exists");
        out.push_str(&format!(
            "    ({key:?}, include_str!({:?})),\n",
            path.display().to_string()
        ));
    }
    out.push_str("];\n");
    let target =
        PathBuf::from(env::var_os("OUT_DIR").expect("set by cargo")).join("descriptions.rs");
    fs::write(target, out).expect("write descriptions.rs");
}
```

Replace the part of `crates/obsbot4linux/src/completions.rs` before `#[cfg(test)]` with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Shell completions for bash and zsh (design spec 6.2, "Completions"), generated from the
//! command-line definition ([`crate::args::command`]) and the description files embedded at
//! build time (`descriptions/*.json`, real models only). They cover subcommands, flags,
//! `get`/`describe` paths, `set` paths with `=`, enum and bool values after `=`, action
//! names and their parameters, and live topics. They are static: Tab never contacts the
//! daemon, and `--device` values are not completed. `scripts/regenerate.sh` writes them to
//! `packaging/completions/`; a test fails when the committed files are stale.

use crate::write::KEEP;
use obsbot_proto::state::DAEMON_PATHS;
use obsbot_proto::{Description, Setting, SettingType};
use std::collections::{BTreeMap, BTreeSet};

include!(concat!(env!("OUT_DIR"), "/descriptions.rs"));

/// The descriptions embedded at build time: (model key, description).
pub fn descriptions() -> Vec<(&'static str, Description)> {
    EMBEDDED
        .iter()
        .map(|(key, json)| {
            let desc = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("descriptions/{key}.json does not parse: {e}"));
            (*key, desc)
        })
        .collect()
}

/// Every word the completions offer.
#[derive(Debug, Default, PartialEq)]
pub struct Words {
    pub subcommands: Vec<String>,
    /// Long flags of the command itself (`--json`, `--device`, `--help`, `--version`).
    pub global_flags: Vec<String>,
    /// Subcommand → its own long flags.
    pub flags: BTreeMap<String, Vec<String>>,
    /// `get`: the daemon fields, every leaf and every group.
    pub get_paths: Vec<String>,
    /// `describe`: every leaf and every action.
    pub describe_names: Vec<String>,
    /// `set`: every writable leaf, with `=`.
    pub set_paths: Vec<String>,
    pub actions: Vec<String>,
    /// Action → its parameters with `=` (not `keep`, which the command line sends itself).
    pub params: BTreeMap<String, Vec<String>>,
    /// `<path>` or `<action> <param>` → the words after `=` (enum writable values; bool
    /// `on off true false`).
    pub values: BTreeMap<String, Vec<String>>,
    pub topics: Vec<String>,
}

fn value_words(s: &Setting) -> Vec<String> {
    match s.kind {
        SettingType::Bool => ["on", "off", "true", "false"].map(String::from).to_vec(),
        SettingType::Enum => s.writable.clone(),
        _ => Vec::new(),
    }
}

fn sorted(set: BTreeSet<String>) -> Vec<String> {
    set.into_iter().collect()
}

/// The words of the command-line definition and of `descriptions`, merged over all models
/// (a path means the same on every model, spec 4.2).
pub fn words_of(descriptions: &[(&str, Description)]) -> Words {
    let mut cmd = crate::args::command();
    cmd.build();
    let long = |a: &clap::Arg| a.get_long().map(|l| format!("--{l}"));
    let global_flags: Vec<String> = cmd.get_arguments().filter_map(long).collect();
    let mut flags = BTreeMap::new();
    let mut subcommands = Vec::new();
    for sub in cmd.get_subcommands() {
        subcommands.push(sub.get_name().to_string());
        let own: Vec<String> = sub
            .get_arguments()
            .filter_map(long)
            .filter(|f| !global_flags.contains(f))
            .collect();
        if !own.is_empty() {
            flags.insert(sub.get_name().to_string(), own);
        }
    }
    let mut get = BTreeSet::new();
    let mut describe = BTreeSet::new();
    let mut set = BTreeSet::new();
    let mut actions = BTreeSet::new();
    let mut params: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut topics = BTreeSet::new();
    get.extend(DAEMON_PATHS.iter().map(|p| p.to_string()));
    for (_, desc) in descriptions {
        for (path, s) in &desc.settings {
            get.insert(path.clone());
            describe.insert(path.clone());
            let mut prefix = path.as_str();
            while let Some((group, _)) = prefix.rsplit_once('.') {
                get.insert(group.to_string());
                prefix = group;
            }
            if let Some(topic) = &s.live {
                topics.insert(topic.clone());
            }
            if !s.readonly {
                set.insert(format!("{path}="));
                let words = value_words(s);
                if !words.is_empty() {
                    values.entry(path.clone()).or_default().extend(words);
                }
            }
        }
        for (name, a) in &desc.actions {
            actions.insert(name.clone());
            describe.insert(name.clone());
            let entry = params.entry(name.clone()).or_default();
            for (param, s) in &a.params {
                if param == KEEP {
                    continue;
                }
                entry.insert(format!("{param}="));
                let words = value_words(s);
                if !words.is_empty() {
                    values
                        .entry(format!("{name} {param}"))
                        .or_default()
                        .extend(words);
                }
            }
        }
    }
    Words {
        subcommands,
        global_flags,
        flags,
        get_paths: sorted(get),
        describe_names: sorted(describe),
        set_paths: sorted(set),
        actions: sorted(actions),
        params: params
            .into_iter()
            .filter(|(_, p)| !p.is_empty())
            .map(|(k, p)| (k, sorted(p)))
            .collect(),
        values: values.into_iter().map(|(k, v)| (k, sorted(v))).collect(),
        topics: sorted(topics),
    }
}

/// The words of the command-line definition and the embedded descriptions.
pub fn words() -> Words {
    words_of(&descriptions())
}

/// Words joined by spaces, broken into lines of at most 96 characters after `indent`.
fn wrap(words: &[String], indent: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for w in words {
        if !line.is_empty() && indent.len() + line.len() + 1 + w.len() > 96 {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(w);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
        .iter()
        .map(|l| format!("{indent}{l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

const HEADER: &str = "Generated by scripts/regenerate.sh from the command-line definition and \
descriptions/*.json; do not edit.";

/// The bash completion script (`packaging/completions/obsbot4linux.bash`).
pub fn bash() -> String {
    let w = words();
    let mut values = String::new();
    for (key, words) in &w.values {
        values.push_str(&format!(
            "        '{key}') REPLY='{}' ;;\n",
            words.join(" ")
        ));
    }
    let mut params = String::new();
    for (key, words) in &w.params {
        params.push_str(&format!(
            "        '{key}') REPLY='{}' ;;\n",
            words.join(" ")
        ));
    }
    let list = |words: &[String]| format!("'\n{}\n'", wrap(words, "    "));
    let flags_of = |sub: &str| {
        let mut all = w.flags.get(sub).cloned().unwrap_or_default();
        all.extend(w.global_flags.iter().cloned());
        all.join(" ")
    };
    format!(
        r#"# bash completion for obsbot4linux.
# {HEADER}

_obsbot4linux_values() {{
    case $1 in
{values}        *) REPLY='' ;;
    esac
}}

_obsbot4linux_params() {{
    case $1 in
{params}        *) REPLY='' ;;
    esac
}}

_obsbot4linux_get_paths={get}

_obsbot4linux_describe_names={describe}

_obsbot4linux_set_paths={set}

_obsbot4linux_actions={actions}

# Completes the value after `=` in $1 from the words in $2.
_obsbot4linux_assign() {{
    local key=${{1%%=*}} val=${{1#*=}}
    local -a found
    mapfile -t found < <(compgen -W "$2" -- "$val")
    if [[ $COMP_WORDBREAKS == *=* ]]; then
        COMPREPLY=("${{found[@]}}")
    else
        COMPREPLY=("${{found[@]/#/$key=}}")
    fi
}}

_obsbot4linux() {{
    local line=${{COMP_LINE:0:COMP_POINT}}
    local -a words
    read -r -a words <<< "$line"
    local cur=''
    if [[ $line != *[[:space:]] ]]; then
        cur=${{words[${{#words[@]}}-1]}}
        unset 'words[${{#words[@]}}-1]'
    fi
    local prev=${{words[${{#words[@]}}-1]}}
    local sub='' action='' i=1 w
    while (( i < ${{#words[@]}} )); do
        w=${{words[i]}}
        case $w in
            --device) (( i += 2 )); continue ;;
            -*) ;;
            *)
                if [[ -z $sub ]]; then
                    sub=$w
                elif [[ $sub == do && -z $action ]]; then
                    action=$w
                fi
                ;;
        esac
        (( i++ ))
    done
    local cands='' nospace='' REPLY=''
    if [[ $prev == --device ]]; then
        COMPREPLY=()
        return
    elif [[ $cur == -* ]]; then
        case $sub in
{sub_flags}            *) cands='{global}' ;;
        esac
    elif [[ -z $sub ]]; then
        cands='{subcommands}'
    else
        case $sub in
            get) cands=$_obsbot4linux_get_paths ;;
            describe) cands=$_obsbot4linux_describe_names ;;
            set)
                if [[ $cur == *=* ]]; then
                    _obsbot4linux_values "${{cur%%=*}}"
                    _obsbot4linux_assign "$cur" "$REPLY"
                    return
                fi
                cands=$_obsbot4linux_set_paths
                nospace=1
                ;;
            do)
                if [[ -z $action ]]; then
                    cands=$_obsbot4linux_actions
                elif [[ $cur == *=* ]]; then
                    _obsbot4linux_values "$action ${{cur%%=*}}"
                    _obsbot4linux_assign "$cur" "$REPLY"
                    return
                else
                    _obsbot4linux_params "$action"
                    cands=$REPLY
                    nospace=1
                fi
                ;;
            watch) cands='{topics}' ;;
        esac
    fi
    mapfile -t COMPREPLY < <(compgen -W "$cands" -- "$cur")
    if [[ -n $nospace ]]; then
        compopt -o nospace 2>/dev/null
    fi
}}

complete -F _obsbot4linux obsbot4linux
"#,
        get = list(&w.get_paths),
        describe = list(&w.describe_names),
        set = list(&w.set_paths),
        actions = list(&w.actions),
        sub_flags = w
            .flags
            .keys()
            .map(|sub| format!("            {sub}) cands='{}' ;;\n", flags_of(sub)))
            .collect::<String>(),
        global = w.global_flags.join(" "),
        subcommands = w.subcommands.join(" "),
        topics = w.topics.join(" "),
    )
}

/// The zsh completion script (`packaging/completions/_obsbot4linux`).
pub fn zsh() -> String {
    let w = words();
    let mut values = String::new();
    for (key, words) in &w.values {
        values.push_str(&format!("    '{key}') reply=({}) ;;\n", words.join(" ")));
    }
    let mut params = String::new();
    for (key, words) in &w.params {
        params.push_str(&format!("    '{key}') reply=({}) ;;\n", words.join(" ")));
    }
    let array = |words: &[String]| format!("(\n{}\n  )", wrap(words, "    "));
    let flags_of = |sub: &str| {
        let mut all = w.flags.get(sub).cloned().unwrap_or_default();
        all.extend(w.global_flags.iter().cloned());
        all.join(" ")
    };
    format!(
        r#"#compdef obsbot4linux
# zsh completion for obsbot4linux.
# {HEADER}

_obsbot4linux_values() {{
  case $1 in
{values}    *) reply=() ;;
  esac
}}

_obsbot4linux_params() {{
  case $1 in
{params}    *) reply=() ;;
  esac
}}

_obsbot4linux() {{
  local cur=${{words[CURRENT]}} prev=${{words[CURRENT-1]}} sub= action= i=2
  local -a reply
  local -a get_paths={get}
  local -a describe_names={describe}
  local -a set_paths={set}
  local -a actions={actions}
  while (( i < CURRENT )); do
    case ${{words[i]}} in
      --device) (( i += 2 )); continue ;;
      -*) ;;
      *)
        if [[ -z $sub ]]; then
          sub=${{words[i]}}
        elif [[ $sub == do && -z $action ]]; then
          action=${{words[i]}}
        fi
        ;;
    esac
    (( i++ ))
  done
  if [[ $prev == --device ]]; then
    return 1
  fi
  if [[ $cur == -* ]]; then
    case $sub in
{sub_flags}      *) compadd -- {global} ;;
    esac
    return
  fi
  if [[ -z $sub ]]; then
    compadd -- {subcommands}
    return
  fi
  case $sub in
    get) compadd -- $get_paths ;;
    describe) compadd -- $describe_names ;;
    set)
      if [[ $cur == *=* ]]; then
        _obsbot4linux_values ${{cur%%=*}}
        compadd -P "${{cur%%=*}}=" -- $reply
      else
        compadd -S '' -- $set_paths
      fi
      ;;
    do)
      if [[ -z $action ]]; then
        compadd -- $actions
      elif [[ $cur == *=* ]]; then
        _obsbot4linux_values "$action ${{cur%%=*}}"
        compadd -P "${{cur%%=*}}=" -- $reply
      else
        _obsbot4linux_params $action
        compadd -S '' -- $reply
      fi
      ;;
    watch) compadd -- {topics} ;;
  esac
}}

_obsbot4linux "$@"
"#,
        get = array(&w.get_paths),
        describe = array(&w.describe_names),
        set = array(&w.set_paths),
        actions = array(&w.actions),
        sub_flags = w
            .flags
            .keys()
            .map(|sub| format!("      {sub}) compadd -- {} ;;\n", flags_of(sub)))
            .collect::<String>(),
        global = w.global_flags.join(" "),
        subcommands = w.subcommands.join(" "),
        topics = w.topics.join(" "),
    )
}
```

In `scripts/regenerate.sh`, replace the comment and the last lines so that the file reads:

```sh
#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Regenerates the generated files from the code (design spec 3, 6.2): schema/protocol-2.json,
# descriptions/ and packaging/completions/. Run it after changing obsbot-proto types, a model's
# description or the command-line definition, and commit the result. The stale tests fail
# until then.
set -eu
cd "$(dirname "$0")/.."
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot-proto --test schema
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot-device --test generated
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot4linux --test completions the_completion_files_are_current
git status --short schema descriptions packaging/completions
```

- [ ] **Step 4: See the stale test fail, then generate**

Run: `cargo test -p obsbot4linux --test completions the_completion_files_are_current`
Expected: FAIL with `packaging/completions/obsbot4linux.bash is stale: run scripts/regenerate.sh and commit the result`.

Run: `scripts/regenerate.sh`
Expected: `git status` lists `packaging/completions/` as new; `schema/` and `descriptions/` unchanged.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot4linux --test completions && cargo test -p obsbot4linux --lib completions`
Expected: 3 passed and 1 ignored (`the_zsh_script_parses`, "needs zsh"); 3 unit tests pass.

Run (where zsh is installed): `cargo test -p obsbot4linux --test completions -- --ignored`
Expected: 1 passed. Say in the PR whether it ran; CI does not run it.

Run: `cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/obsbot4linux/build.rs crates/obsbot4linux/src/completions.rs crates/obsbot4linux/tests/completions.rs packaging/completions scripts/regenerate.sh
git commit -m "obsbot4linux: bash and zsh completions from the CLI definition and the descriptions"
```

### Task C-9: `status`, `get`, `describe`, `devices`

**Files:**

- Replace: `crates/obsbot4linux/src/read.rs` (C-2's stub)
- Test: `crates/obsbot4linux/tests/read.rs`

**Interfaces:**

- Consumes (C-5): `session::{connect, connect_device, client_error}`, `select::{select, report_hint}`; (C-6) `render::{status_text, state_text, devices_text}`; (C-7) `describe::{device_text, setting_text, action_text}`; (C-2) `Ctx`, `CliCode`, `CliError`, `tests/common`; (C-1) `Client::get`, `ScriptServer`; (W0) `RevState`, `Support`, `Usb`, `Device`.
- Produces: the bodies of `read::status`, `read::get`, `read::describe`, `read::devices` with C-2's signatures.

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot4linux/tests/read.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `status`, `get`, `describe` and `devices` against a scripted daemon (design spec 6.2):
//! the one request each sends (the script fails on any further line), device selection,
//! the refusal of an unsupported device before anything is sent, the `--json` shapes, and
//! what an unreachable or older daemon gives.

mod common;

use common::{cli, fake_a, fake_b, hello_line, script, socket, three, unknown};
use obsbot_proto::client::script::ScriptServer;
use serde_json::{Value, json};

fn parse(out: &str) -> Value {
    serde_json::from_str(out.trim_end()).unwrap_or_else(|e| panic!("{e}: {out}"))
}

#[test]
fn status_sends_one_get_for_the_full_state() {
    let server = ScriptServer::start(
        &socket("status"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":null}"#,
                r#"< {"id":2,"ok":{"rev":4,"state":{"available":true,"pending":{"zoom.factor":2},"unread":[],"zoom":{"factor":1.5},"tracking":{"mode":"normal"}}}}"#,
            ],
        ),
    );
    let (code, out, err) = cli(Some(server.path()), &["status", "--device", "fake-a"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "…0001  Fake A 0001  fake-a\n\
         available: yes\n\
         tracking\n  mode: normal\n\
         zoom\n  factor: 2× (pending)\n"
    );
    assert_eq!(server.finish().unwrap().len(), 2);
}

/// Spec 6.2: `status --json` is `{"device":<Device>,"rev","state"}` with the full id.
#[test]
fn status_json_is_device_rev_and_state() {
    let server = ScriptServer::start(
        &socket("status-json"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEB0000001","get":null}"#,
                r#"< {"id":2,"ok":{"rev":3,"state":{"available":true,"pending":{},"unread":[],"led":{"brightness":2}}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "status", "--device", "B0000001"],
    );
    assert_eq!(code, 0);
    let v = parse(&out);
    assert_eq!(v["device"]["id"], "sn:FAKEB0000001");
    assert_eq!(v["device"]["serial"], "FAKEB0000001");
    assert_eq!(v["rev"], 3);
    assert_eq!(v["state"]["led"]["brightness"], 2);
    let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["device", "rev", "state"]);
    assert!(
        out.starts_with(r#"{"device":{"id":"sn:FAKEB0000001","#),
        "{out}"
    );
    server.finish().unwrap();
}

#[test]
fn get_sends_the_paths_as_given_and_prints_the_ok_payload() {
    let server = ScriptServer::start(
        &socket("get"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":["zoom.factor","presets"]}"#,
                r#"< {"id":2,"ok":{"rev":5,"state":{"available":true,"pending":{},"unread":[],"zoom":{"factor":1.5},"presets":[{"slot":0,"name":"","pose":{"pan":0,"tilt":0,"zoom":1}}]}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &[
            "get",
            "zoom.factor",
            "presets",
            "--device",
            "fake-a",
            "--json",
        ],
    );
    assert_eq!(code, 0);
    // The ok payload exactly, the empty name as the camera reported it (spec 6.2).
    assert_eq!(
        out,
        "{\"rev\":5,\"state\":{\"available\":true,\"pending\":{},\"unread\":[],\"presets\":[{\"name\":\"\",\"pose\":{\"pan\":0,\"tilt\":0,\"zoom\":1},\"slot\":0}],\"zoom\":{\"factor\":1.5}}}\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2, "Preset names": a placeholder for an empty name, as display only.
#[test]
fn get_in_human_form_shows_an_unnamed_preset_with_a_placeholder() {
    let server = ScriptServer::start(
        &socket("get-human"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":["presets"]}"#,
                r#"< {"id":2,"ok":{"rev":5,"state":{"available":true,"pending":{},"unread":[],"presets":[{"slot":0,"name":"","pose":{"pan":0,"tilt":0,"zoom":1}}]}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["get", "presets", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "available: yes\npresets:\n  - name (unnamed), pose pan 0° tilt 0° zoom 1×, slot 0\n"
    );
    server.finish().unwrap();
}

/// The daemon answers a path it does not have; its code and exit code stay.
#[test]
fn get_of_an_unknown_path_is_the_daemons_unsupported() {
    let server = ScriptServer::start(
        &socket("get-unknown"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEB0000001","get":["gimbal.pan"]}"#,
                r#"< {"id":2,"error":{"code":"unsupported","message":"gimbal.pan is not a path of this device"}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "get", "gimbal.pan", "--device", "fake-b"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"unsupported\",\"message\":\"gimbal.pan is not a path of this device\"}}\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2 and 8.4 step 5: an unsupported device addressed by status or get is refused before anything is sent (exit 3, `unsupported_device`).
#[test]
fn an_unsupported_device_is_refused_before_anything_is_sent() {
    for (n, args) in [
        vec!["status", "--device", "unknown"],
        vec!["get", "zoom", "--device", "port:9-3"],
    ]
    .into_iter()
    .enumerate()
    {
        let server =
            ScriptServer::start(&socket(&format!("unsupported-{n}")), &script(&three(), &[]));
        let (code, out, err) = cli(Some(server.path()), &args);
        assert_eq!(code, 3, "{args:?}");
        assert_eq!(out, "");
        assert!(
            err.starts_with(&format!(
                "obsbot4linux: Unknown Camera 9-3 (USB 3564:fa00) is not supported by obsbot4linux {}. \
                 `obsbot4linux devices --report` prints what an issue report needs: \
                 https://github.com/hellwand/obsbot4linux/issues",
                env!("CARGO_PKG_VERSION")
            )),
            "{err}"
        );
        assert_eq!(server.finish().unwrap().len(), 1, "only hello: {args:?}");
    }
}

#[test]
fn two_usable_devices_without_device_are_ambiguous() {
    let server = ScriptServer::start(&socket("ambiguous"), &script(&three(), &[]));
    let (code, out, _) = cli(Some(server.path()), &["status", "--json"]);
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"ambiguous_device\",\"devices\":[\"sn:FAKEA0000001\",\"sn:FAKEB0000001\"],\"message\":\"2 usable devices: choose one with --device\"}}\n"
    );
    server.finish().unwrap();
}

/// Spec 8.4 step 5: with a supported camera connected too, it is used without `--device`.
#[test]
fn the_only_usable_device_is_used_beside_an_unsupported_one() {
    let server = ScriptServer::start(
        &socket("only-usable"),
        &script(
            &[fake_a(), unknown()],
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":["zoom.factor"]}"#,
                r#"< {"id":2,"ok":{"rev":2,"state":{"available":true,"pending":{},"unread":[],"zoom":{"factor":1.5}}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(Some(server.path()), &["get", "zoom.factor"]);
    assert_eq!(code, 0);
    assert_eq!(out, "available: yes\nzoom\n  factor: 1.5×\n");
    server.finish().unwrap();
}

#[test]
fn only_an_unsupported_device_is_no_device_with_the_list_and_the_hint() {
    let server = ScriptServer::start(&socket("no-device"), &script(&[unknown()], &[]));
    let (code, out, err) = cli(Some(server.path()), &["status"]);
    assert_eq!(code, 3);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "obsbot4linux: no usable OBSBOT camera\n  \
         port:9-3  Unknown Camera 9-3  unknown  unsupported\n\
         `obsbot4linux devices --report` prints what an issue report needs: \
         https://github.com/hellwand/obsbot4linux/issues\n"
    );
    server.finish().unwrap();
}

#[test]
fn a_device_nobody_has_is_unknown_device() {
    let server = ScriptServer::start(&socket("unknown-device"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "status", "--device", "fake-z"],
    );
    assert_eq!(code, 2);
    assert_eq!(parse(&out)["error"]["code"], "unknown_device");
    server.finish().unwrap();
}

/// Spec 6.2: `describe` prints the device entry; with a path or action, that object.
#[test]
fn describe_json_prints_the_device_entry_setting_or_action() {
    let server = ScriptServer::start(&socket("describe"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["describe", "--device", "fake-b", "--json"],
    );
    assert_eq!(code, 0);
    assert_eq!(parse(&out), serde_json::to_value(fake_b()).unwrap());
    server.finish().unwrap();

    let server = ScriptServer::start(&socket("describe-setting"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["describe", "beacon.color", "--device", "fake-b", "--json"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        serde_json::to_value(&fake_b().description.settings["beacon.color"]).unwrap()
    );
    server.finish().unwrap();

    let server = ScriptServer::start(&socket("describe-action"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["describe", "beacon.flash", "--device", "fake-b", "--json"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        serde_json::to_value(&fake_b().description.actions["beacon.flash"]).unwrap()
    );
    server.finish().unwrap();
}

/// Spec 6.2: a path or action not in the description exits 2 with `unsupported`.
#[test]
fn describe_of_something_the_device_lacks_is_unsupported() {
    let server = ScriptServer::start(&socket("describe-missing"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "describe", "gimbal.pan", "--device", "fake-b"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"unsupported\",\"message\":\"gimbal.pan is not a setting or an action of Fake B 0001\"}}\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2: describe may address an unsupported device; its entry has empty settings.
#[test]
fn describe_prints_an_unsupported_entry() {
    let server = ScriptServer::start(&socket("describe-unknown"), &script(&three(), &[]));
    let (code, out, _) = cli(Some(server.path()), &["describe", "--device", "unknown"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "port:9-3  Unknown Camera 9-3  unknown  unsupported\n\
         firmware: –\n\
         video node: /dev/video106\n\
         usb: 3564:fa00 Unknown Camera, path 9-3\n\
         settings: (none)\n\
         actions: (none)\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2: `devices --json` is `{"devices":[<Device>…]}`, the list of the hello reply.
#[test]
fn devices_json_is_the_hello_list() {
    let server = ScriptServer::start(&socket("devices-json"), &script(&three(), &[]));
    let (code, out, _) = cli(Some(server.path()), &["devices", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        json!({ "devices": serde_json::to_value(three()).unwrap() })
    );
    server.finish().unwrap();
}

/// Spec 8.6: the support level next to every device that is not tested.
#[test]
fn devices_in_human_form_marks_untested_and_unsupported() {
    let server = ScriptServer::start(&socket("devices"), &script(&three(), &[]));
    let (code, out, _) = cli(Some(server.path()), &["devices"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "…0001  Fake A 0001  fake-a\n\
         …0001  Fake B 0001  fake-b  untested\n\
         port:9-3  Unknown Camera 9-3  unknown  unsupported\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2: `devices --report` prints JSON in both modes, with no id and no serial value.
#[test]
fn devices_report_has_usb_facts_and_no_serial() {
    let server = ScriptServer::start(&socket("report"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["devices", "--report", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    let v = parse(&out);
    assert_eq!(
        v,
        json!({"devices": [{
            "model": "fake-a",
            "support": "tested",
            "firmware": "1.0.0.0",
            "video_node": "/dev/video102",
            "usb": serde_json::to_value(&fake_a().usb).unwrap(),
        }]})
    );
    assert!(!out.contains("FAKEA0000001"), "{out}");
    assert!(!out.contains("\"id\""), "{out}");
    server.finish().unwrap();
}

/// Parity exception PX-5: `devices` exits 3 (`no_device`) when the list is empty.
#[test]
fn devices_with_an_empty_list_is_no_device() {
    let server = ScriptServer::start(&socket("devices-empty"), &script(&[], &[]));
    let (code, out, _) = cli(Some(server.path()), &["devices", "--json"]);
    assert_eq!(code, 3);
    assert_eq!(parse(&out)["error"]["code"], "no_device");
    server.finish().unwrap();
}

/// Spec 5.10 and 10.3 C: no socket gives exit 4 and the systemctl line.
#[test]
fn no_socket_is_exit_4_with_the_enable_line() {
    let path = socket("absent");
    let _ = std::fs::remove_file(&path);
    let (code, out, err) = cli(Some(&path), &["status"]);
    assert_eq!(code, 4);
    assert_eq!(out, "");
    assert!(
        err.trim_end()
            .ends_with("start it with: systemctl --user enable --now obsbot4linux.socket"),
        "{err}"
    );
    let (code, _, err) = cli(None, &["devices"]);
    assert_eq!(code, 4);
    assert!(
        err.contains("systemctl --user enable --now obsbot4linux.socket"),
        "{err}"
    );
}

/// Spec 5.10 and 10.3 C: a non-protocol-2 hello reply gives exit 4 and the restart message.
#[test]
fn an_older_daemon_is_exit_4_with_the_restart_message() {
    let server = ScriptServer::start(
        &socket("old"),
        &[
            hello_line(),
            r#"< {"id":null,"ok":false,"error":"bad request: missing field `args` at line 1 column 83"}"#.to_string(),
        ]
        .join("\n"),
    );
    let (code, out, err) = cli(Some(server.path()), &["devices"]);
    assert_eq!(code, 4);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "obsbot4linux: an obsbot4linux daemon older than 1.0 is running: restart it or log out\n"
    );
    server.finish().unwrap();
}

#[test]
fn a_daemon_of_another_protocol_is_exit_4_with_its_number() {
    let server = ScriptServer::start(
        &socket("protocol"),
        &[
            hello_line(),
            r#"< {"id":null,"error":{"code":"protocol","message":"protocol 2 is not supported","supported":[3]}}"#.to_string(),
            "!close".to_string(),
        ]
        .join("\n"),
    );
    let (code, out, _) = cli(Some(server.path()), &["--json", "devices"]);
    assert_eq!(code, 4);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"protocol\",\"message\":\"the obsbot4linux daemon speaks protocol 3, this client speaks 2: install the same version of both and restart the daemon\"}}\n"
    );
    server.finish().unwrap();
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot4linux --test read`
Expected: every test fails: the stubs answer every command with exit 1 and `<command> is not built yet (track C)`.

- [ ] **Step 3: Write the reading commands**

Replace `crates/obsbot4linux/src/read.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The reading commands (design spec 6.2): `status` and `get` send one `get`; `describe` and
//! `devices` send nothing after `hello`, whose reply holds the device list.

use crate::errors::{CliCode, CliError};
use crate::select::{report_hint, select};
use crate::{Ctx, describe as text, render, session};
use obsbot_proto::{Device, ErrorCode, RevState, Support, Usb};
use serde::Serialize;

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("protocol objects serialise")
}

fn print(ctx: &mut Ctx, text: &str) {
    let _ = ctx.out.write_all(text.as_bytes());
}

/// `status`: `get` with `null`. `--json`: `{"device":<Device>,"rev","state"}`.
pub fn status(ctx: &mut Ctx) -> Result<(), CliError> {
    let (mut client, device) =
        session::connect_device(ctx.socket.as_deref(), ctx.device.as_deref())?;
    let (rev, state) = client
        .get(&device.id, None)
        .map_err(session::client_error)?;
    let out = if ctx.json {
        format!(
            "{{\"device\":{},\"rev\":{rev},\"state\":{}}}\n",
            to_json(&device),
            to_json(&state)
        )
    } else {
        render::status_text(&device, &state)
    };
    print(ctx, &out);
    Ok(())
}

/// `get <path>…`: one `get` with the paths as given; the daemon answers paths it does not
/// have. `--json`: the `ok` payload `{"rev","state"}`.
pub fn get(ctx: &mut Ctx, paths: &[String]) -> Result<(), CliError> {
    let (mut client, device) =
        session::connect_device(ctx.socket.as_deref(), ctx.device.as_deref())?;
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    let (rev, state) = client
        .get(&device.id, Some(&paths))
        .map_err(session::client_error)?;
    let out = if ctx.json {
        format!("{}\n", to_json(&RevState { rev, state }))
    } else {
        render::state_text(&device.description, &state)
    };
    print(ctx, &out);
    Ok(())
}

/// `describe [<path|action>]`: the device entry, or one `Setting` or `Action` of it. It may
/// address an unsupported device (its entry has empty settings and actions).
pub fn describe(ctx: &mut Ctx, target: Option<&str>) -> Result<(), CliError> {
    let client = session::connect(ctx.socket.as_deref())?;
    let device = select(client.devices(), ctx.device.as_deref())?.clone();
    let desc = &device.description;
    let out = match target {
        None if ctx.json => format!("{}\n", to_json(&device)),
        None => text::device_text(&device),
        Some(name) => match (desc.settings.get(name), desc.actions.get(name)) {
            (Some(s), _) if ctx.json => format!("{}\n", to_json(s)),
            (Some(s), _) => format!("{name}: {}\n", text::setting_text(s)),
            (None, Some(a)) if ctx.json => format!("{}\n", to_json(a)),
            (None, Some(a)) => format!("{name}: {}\n", text::action_text(a)),
            (None, None) => {
                return Err(CliError::new(
                    CliCode::Proto(ErrorCode::Unsupported),
                    format!("{name} is not a setting or an action of {}", device.name),
                ));
            }
        },
    };
    print(ctx, &out);
    Ok(())
}

/// One device of `devices --report`: what an issue report needs, without id and serial.
#[derive(Serialize)]
struct ReportEntry<'a> {
    model: &'a str,
    support: &'a Support,
    firmware: &'a Option<String>,
    video_node: &'a Option<String>,
    usb: &'a Usb,
}

#[derive(Serialize)]
struct Report<'a> {
    devices: Vec<ReportEntry<'a>>,
}

/// `devices [--report]`: every listed device, or the one `--device` matches. Exit 3
/// (`no_device`) when the list is empty (parity exception PX-5). `--report` prints JSON in
/// both modes, pretty-printed for pasting into an issue, with `serial_present` and no id or
/// serial value.
pub fn devices(ctx: &mut Ctx, report: bool) -> Result<(), CliError> {
    let client = session::connect(ctx.socket.as_deref())?;
    let list: Vec<Device> = match ctx.device.as_deref() {
        Some(wanted) => vec![select(client.devices(), Some(wanted))?.clone()],
        None => client.devices().to_vec(),
    };
    if list.is_empty() {
        return Err(
            CliError::new(CliCode::NoDevice, "no OBSBOT camera is listed").with_hint(report_hint()),
        );
    }
    let out = if report {
        let entries = list
            .iter()
            .map(|d| ReportEntry {
                model: &d.model,
                support: &d.support,
                firmware: &d.firmware,
                video_node: &d.video_node,
                usb: &d.usb,
            })
            .collect();
        let report = Report { devices: entries };
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).expect("a report serialises")
        )
    } else if ctx.json {
        format!("{{\"devices\":{}}}\n", to_json(&list))
    } else {
        render::devices_text(&list)
    };
    print(ctx, &out);
    Ok(())
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p obsbot4linux --test read && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: 20 passed; clean.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linux/src/read.rs crates/obsbot4linux/tests/read.rs
git commit -m "obsbot4linux: status, get, describe and devices"
```

### Task C-10: `set` and `do`

**Files:**

- Replace: `crates/obsbot4linux/src/write.rs` (C-2's stub; keeps `KEEP`)
- Test: `crates/obsbot4linux/tests/write.rs`; unit tests in `src/write.rs`

**Interfaces:**

- Consumes (C-4): `value::{parse_patch, parse_params}`; (C-5) `session::{connect_device, client_error}`; (C-6) `render::value_text`; (C-2) `Ctx`, `CliError`, `tests/common`; (C-1) `Client::{set, act}`, `ScriptServer`; (W0) `Action`, `Params`, `RevOk`, `ErrorCode::ALL`.
- Produces: `pub const KEEP: &str = "keep"` (unchanged), `pub fn keep(action: Option<&Action>, params: &mut Params) -> Result<(), CliError>`, and the bodies of `write::set`, `write::act` with C-2's signatures.

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot4linux/tests/write.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `set` and `do` against a scripted daemon (design spec 6.2): the one request each sends and
//! nothing after it (spec 2), the value syntax on the wire, `keep: true` for a continuing
//! action, and one test per error code the daemon sends, for its exit code.

mod common;

use common::{cli, script, socket, three};
use obsbot_proto::ErrorCode;
use obsbot_proto::client::script::ScriptServer;

#[test]
fn set_sends_one_patch_and_prints_nothing() {
    let server = ScriptServer::start(
        &socket("set"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","set":{"exposure.mode":"manual","exposure.time":20000,"zone.area.pan":{"min":-20,"max":20},"tracking.mode":"upper_body","image.mirror":true}}"#,
                r#"< {"id":2,"ok":{"rev":7}}"#,
            ],
        ),
    );
    let (code, out, err) = cli(
        Some(server.path()),
        &[
            "set",
            "--device",
            "fake-a",
            "exposure.mode=manual",
            "exposure.time=20000",
            "zone.area.pan=-20,20",
            "tracking.mode=upper-body",
            "image.mirror=on",
        ],
    );
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    assert_eq!(server.finish().unwrap().len(), 2, "hello and one set");
}

#[test]
fn set_json_prints_the_rev() {
    let server = ScriptServer::start(
        &socket("set-json"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEB0000001","set":{"led.brightness":5}}"#,
                r#"< {"id":2,"ok":{"rev":9}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "set", "led.brightness=5", "--device", "fake-b"],
    );
    assert_eq!((code, out.as_str()), (0, "{\"rev\":9}\n"));
    server.finish().unwrap();
}

/// Spec 6.2 and 8.4 step 5: an unsupported device addressed by set or do is refused before anything is sent (exit 3, `unsupported_device`).
#[test]
fn an_unsupported_device_is_refused_before_set_or_do() {
    for (n, args) in [
        vec!["set", "led.brightness=1", "--device", "unknown"],
        vec!["do", "sleep", "--device", "unknown"],
    ]
    .into_iter()
    .enumerate()
    {
        let server = ScriptServer::start(
            &socket(&format!("write-unsupported-{n}")),
            &script(&three(), &[]),
        );
        let (code, out, err) = cli(Some(server.path()), &args);
        assert_eq!(code, 3, "{args:?}");
        assert_eq!(out, "");
        assert!(
            err.starts_with(&format!(
                "obsbot4linux: Unknown Camera 9-3 (USB 3564:fa00) is not supported by obsbot4linux {}. \
                 `obsbot4linux devices --report` prints what an issue report needs: \
                 https://github.com/hellwand/obsbot4linux/issues",
                env!("CARGO_PKG_VERSION")
            )),
            "{err}"
        );
        assert_eq!(server.finish().unwrap().len(), 1, "only hello: {args:?}");
    }
}

/// A value that does not parse is a usage error found after hello; nothing else is sent.
#[test]
fn a_value_that_does_not_parse_sends_nothing() {
    let server = ScriptServer::start(&socket("set-usage"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "set", "zoom.factor=wide", "--device", "fake-a"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"usage\",\"message\":\"zoom.factor: 'wide' is not a number\"}}\n"
    );
    assert_eq!(server.finish().unwrap().len(), 1, "only hello");
}

/// Spec 2 and 6.2: `do gimbal.move` sends the move alone, no tracking switch before it.
#[test]
fn do_sends_one_action_with_its_parameters() {
    let server = ScriptServer::start(
        &socket("do"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"gimbal.move","with":{"pan":-20,"tilt":5.5}}"#,
                r#"< {"id":2,"ok":{"rev":3}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &[
            "do",
            "gimbal.move",
            "pan=-20",
            "tilt=5.5",
            "--device",
            "fake-a",
        ],
    );
    assert_eq!((code, out.as_str()), (0, ""));
    assert_eq!(server.finish().unwrap().len(), 2, "hello and one do");
}

/// Spec 4.4 and 6.2: the CLI always sends keep:true with gimbal.turn.
#[test]
fn a_turn_is_sent_with_keep_true() {
    let server = ScriptServer::start(
        &socket("turn"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"gimbal.turn","with":{"pan_speed":20,"keep":true}}"#,
                r#"< {"id":2,"ok":{"rev":3}}"#,
            ],
        ),
    );
    let (code, _, _) = cli(
        Some(server.path()),
        &["do", "gimbal.turn", "pan_speed=20", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    server.finish().unwrap();
}

/// Spec 4.4 and 12.2: `preset.save` without `name` sends no name; the CLI invents none.
#[test]
fn a_preset_saved_without_a_name_is_sent_without_one() {
    let server = ScriptServer::start(
        &socket("preset"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"preset.save","with":{"slot":1}}"#,
                r#"< {"id":2,"ok":{"rev":3}}"#,
            ],
        ),
    );
    let (code, _, _) = cli(
        Some(server.path()),
        &["do", "preset.save", "slot=1", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    assert_eq!(server.finish().unwrap().len(), 2);
}

#[test]
fn an_action_result_is_printed() {
    let server = ScriptServer::start(
        &socket("result"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"view.save","with":{"name":"Desk","pose":{"pan":0,"tilt":5,"zoom":1.2}}}"#,
                r#"< {"id":2,"ok":{"rev":3,"result":{"id":2}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &[
            "do",
            "view.save",
            "name=Desk",
            "pose=0,5,1.2",
            "--device",
            "fake-a",
        ],
    );
    assert_eq!((code, out.as_str()), (0, "id: 2\n"));
    server.finish().unwrap();

    let server = ScriptServer::start(
        &socket("result-json"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"view.save","with":{"name":"Desk"}}"#,
                r#"< {"id":2,"ok":{"rev":3,"result":{"id":2}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &[
            "--json",
            "do",
            "view.save",
            "name=Desk",
            "--device",
            "fake-a",
        ],
    );
    assert_eq!(
        (code, out.as_str()),
        (0, "{\"rev\":3,\"result\":{\"id\":2}}\n")
    );
    server.finish().unwrap();
}

/// Parity row `factory-reset` without `--yes`: the daemon refuses with `invalid` (exit 2);
/// the CLI adds no confirmation of its own.
#[test]
fn a_destructive_action_without_confirm_is_the_daemons_invalid() {
    let server = ScriptServer::start(
        &socket("reset"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"factory_reset"}"#,
                r#"< {"id":2,"error":{"code":"invalid","message":"factory_reset deletes data on the camera: send confirm: true"}}"#,
            ],
        ),
    );
    let (code, _, err) = cli(
        Some(server.path()),
        &["do", "factory_reset", "--device", "fake-a"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        err,
        "obsbot4linux: factory_reset deletes data on the camera: send confirm: true\n"
    );
    server.finish().unwrap();
}

/// Spec 10.3 C: one test per error code, for its exit code; the daemon's code is kept.
#[test]
fn every_daemon_error_code_gives_its_exit_code() {
    let mut codes: Vec<(String, u8)> = ErrorCode::ALL
        .iter()
        .map(|c| {
            let exit = match c {
                ErrorCode::Internal => 1,
                ErrorCode::Invalid
                | ErrorCode::UnknownDevice
                | ErrorCode::Unsupported
                | ErrorCode::OutOfRange => 2,
                ErrorCode::Protocol => 4,
                _ => 3,
            };
            (c.as_str().to_string(), exit)
        })
        .collect();
    codes.push(("teapot".into(), 1));
    for (n, (code, exit)) in codes.iter().enumerate() {
        let reply =
            format!(r#"< {{"id":2,"error":{{"code":"{code}","message":"scripted {code}"}}}}"#);
        let server = ScriptServer::start(
            &socket(&format!("code-{n}")),
            &script(
                &three(),
                &[
                    r#"> {"id":2,"device":"sn:FAKEB0000001","set":{"led.brightness":1}}"#,
                    &reply,
                ],
            ),
        );
        let (got, out, _) = cli(
            Some(server.path()),
            &["--json", "set", "led.brightness=1", "--device", "fake-b"],
        );
        assert_eq!(got, *exit, "{code}");
        assert_eq!(
            out,
            format!("{{\"error\":{{\"code\":\"{code}\",\"message\":\"scripted {code}\"}}}}\n"),
            "{code}"
        );
        server.finish().unwrap();
    }
}

#[test]
fn keep_false_is_refused_before_the_request() {
    let server = ScriptServer::start(&socket("keep-false"), &script(&three(), &[]));
    let (code, _, err) = cli(
        Some(server.path()),
        &[
            "do",
            "gimbal.turn",
            "pan_speed=20",
            "keep=off",
            "--device",
            "fake-a",
        ],
    );
    assert_eq!(code, 2);
    assert!(err.contains("keep=true"), "{err}");
    assert_eq!(server.finish().unwrap().len(), 1);
}

/// Review focus: 0.13 habits type dashes in paths. Only enum values take `-` for `_`; a path
/// is sent as typed and the daemon answers it (`unsupported`, exit 2).
#[test]
fn a_dashed_path_is_sent_as_typed() {
    let server = ScriptServer::start(
        &socket("dashed-path"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","set":{"framing.zoom-speed":"fast"}}"#,
                r#"< {"id":2,"error":{"code":"unsupported","message":"framing.zoom-speed is not a setting of this device"}}"#,
            ],
        ),
    );
    let (code, _, err) = cli(
        Some(server.path()),
        &["set", "framing.zoom-speed=fast", "--device", "fake-a"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        err,
        "obsbot4linux: framing.zoom-speed is not a setting of this device\n"
    );
    server.finish().unwrap();
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot4linux --test write`
Expected: the tests fail with exit 1 and `set is not built yet (track C)` / `do is not built yet (track C)`.

- [ ] **Step 3: Write the writing commands**

Replace `crates/obsbot4linux/src/write.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The writing commands (design spec 6.2): `set` sends one `set` with the whole patch, `do`
//! sends one `do`. Neither chains a second request (spec 2): `do gimbal.move` leaves the
//! tracking as it is, and a user who wants it off runs `set tracking.mode=static` first.

use crate::errors::CliError;
use crate::{Ctx, render, session, value};
use obsbot_proto::{Action, Params, RevOk};
use serde_json::Value;

/// The parameter that keeps a continuing action going after its connection closes
/// (spec 4.4, `gimbal.turn`).
pub const KEEP: &str = "keep";

/// `set <path>=<value> …`: one patch. `--json`: `{"rev"}`; human output prints nothing.
pub fn set(ctx: &mut Ctx, assignments: &[String]) -> Result<(), CliError> {
    let (mut client, device) =
        session::connect_device(ctx.socket.as_deref(), ctx.device.as_deref())?;
    let patch = value::parse_patch(&device.description, assignments)?;
    let rev = client
        .set(&device.id, patch)
        .map_err(session::client_error)?;
    if ctx.json {
        let _ = writeln!(ctx.out, "{{\"rev\":{rev}}}");
    }
    Ok(())
}

/// An action with a `keep` parameter lasts as long as its connection unless `keep` is true
/// (spec 4.4). The command line's connection closes right after the reply, so it always
/// sends `keep: true` (spec 6.2, "`gimbal.turn`"); any other `keep` is a usage error.
pub fn keep(action: Option<&Action>, params: &mut Params) -> Result<(), CliError> {
    if !action.is_some_and(|a| a.params.contains_key(KEEP)) {
        return Ok(());
    }
    match params.get(KEEP) {
        None | Some(Value::Bool(true)) => {
            params.insert(KEEP.into(), Value::Bool(true));
            Ok(())
        }
        Some(_) => Err(CliError::usage(
            "keep: the command line always sends keep=true, because its connection closes \
             right after the reply; stop the action with its stop action",
        )),
    }
}

/// `do <action> [<param>=<value> …]`: one action. `--json`: `{"rev","result"?}`; human
/// output prints the result's fields, if the action has a result, and nothing otherwise.
pub fn act(ctx: &mut Ctx, action: &str, args: &[String]) -> Result<(), CliError> {
    let (mut client, device) =
        session::connect_device(ctx.socket.as_deref(), ctx.device.as_deref())?;
    let spec = device.description.actions.get(action);
    let mut params = value::parse_params(spec, args)?;
    keep(spec, &mut params)?;
    let (rev, result) = client
        .act(&device.id, action, Value::Object(params))
        .map_err(session::client_error)?;
    if ctx.json {
        let reply = RevOk { rev, result };
        let _ = writeln!(
            ctx.out,
            "{}",
            serde_json::to_string(&reply).expect("a reply serialises")
        );
    } else if let Some(result) = result {
        for (name, v) in &result {
            let described = spec
                .and_then(|a| a.result.as_ref())
                .and_then(|r| r.get(name));
            let text = match described {
                Some(setting) => render::value_text(setting, v),
                None => v.to_string(),
            };
            let _ = writeln!(ctx.out, "{name}: {text}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::{Asleep, Setting};
    use serde_json::json;

    fn turn() -> Action {
        Action::new("Turn", Asleep::Unknown)
            .param(
                "pan_speed",
                Setting::number("Pan speed", -178.0, 178.0).optional(),
            )
            .param("keep", Setting::boolean("Keep").optional())
    }

    /// Spec 6.2: the CLI always sends keep:true with gimbal.turn.
    #[test]
    fn a_continuing_action_always_gets_keep_true() {
        let mut p = Params::new();
        p.insert("pan_speed".into(), json!(20));
        keep(Some(&turn()), &mut p).unwrap();
        assert_eq!(Value::Object(p), json!({"pan_speed": 20, "keep": true}));
        let mut p = Params::new();
        p.insert("keep".into(), json!(true));
        keep(Some(&turn()), &mut p).unwrap();
        assert_eq!(p["keep"], json!(true));
    }

    #[test]
    fn keep_false_is_refused_and_other_actions_are_left_alone() {
        let mut p = Params::new();
        p.insert("keep".into(), json!(false));
        assert_eq!(keep(Some(&turn()), &mut p).unwrap_err().code.exit(), 2);
        let move_ = Action::new("Move", Asleep::Unknown);
        let mut p = Params::new();
        keep(Some(&move_), &mut p).unwrap();
        assert!(p.is_empty(), "no keep for an action without it");
        keep(None, &mut p).unwrap();
        assert!(p.is_empty());
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p obsbot4linux --test write && cargo test -p obsbot4linux --lib write && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: 12 passed in `tests/write.rs` (`every_daemon_error_code_gives_its_exit_code` covers all 12 codes the daemon sends and one unknown code); 2 unit tests pass; clean.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linux/src/write.rs crates/obsbot4linux/tests/write.rs
git commit -m "obsbot4linux: set and do, one request each"
```

### Task C-11: `watch`

**Files:**

- Replace: `crates/obsbot4linux/src/watch.rs` (C-2's stub)
- Test: `crates/obsbot4linux/tests/watch.rs` (one test runs the binary in a child process for the signals)

**Interfaces:**

- Consumes (C-1): `Connection::{open, hello, read_line}`, `HelloReply::line`, `ClientError::NoSocketPath`, `ScriptServer`; (C-5) `session::{client_error, client_name}`; (C-2) `Ctx`, `CliCode`, `CliError`, `tests/common::{cli, hello_reply_line, socket, three}`; `nix::sys::signal::{SigSet, Signal}` (`SigSet::empty`, `add`, `thread_block`, `wait`).
- Produces: the body of `watch::watch` with C-2's signature.

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot4linux/tests/watch.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `watch` against a scripted daemon (design spec 6.2): the hello reply as its first line,
//! then every line as received; exit 4 when the connection ends; exit 0 on SIGINT and
//! SIGTERM (the binary, in a child process).

mod common;

use common::{cli, hello_reply_line, socket, three};
use obsbot_proto::client::script::ScriptServer;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn watch_hello(live: &str) -> String {
    format!(
        r#"> {{"id":1,"hello":{{"protocol":2,"client":"obsbot4linux {}","subscribe":true,"live":[{live}]}}}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

const EVENTS: [&str; 3] = [
    r#"{"event":"devices","devices":[]}"#,
    r#"{"event":"frobnicate","from":"a newer daemon"}"#,
    r#"{"event":"state","device":"sn:FAKEA0000001","rev":2,"state":{"available":false,"pending":{},"unread":[],"x":1.50}}"#,
];

#[test]
fn watch_prints_the_hello_reply_and_every_line_as_received_then_exits_4() {
    let reply = hello_reply_line(&three());
    let mut lines = vec![watch_hello(r#""pose","target""#), reply.clone()];
    lines.extend(EVENTS.iter().map(|e| format!("< {e}")));
    lines.push("!close".into());
    let server = ScriptServer::start(&socket("watch"), &lines.join("\n"));
    let (code, out, err) = cli(Some(server.path()), &["watch", "--live", "pose", "target"]);
    assert_eq!(code, 4);
    let mut want = format!("{}\n", reply.strip_prefix("< ").unwrap());
    for e in EVENTS {
        want.push_str(e);
        want.push('\n');
    }
    // Unchanged: an unknown event type and `1.50` stay as the daemon wrote them.
    assert_eq!(out, want);
    assert_eq!(
        err,
        "obsbot4linux: the obsbot4linux daemon closed the connection\n"
    );
    assert_eq!(server.finish().unwrap().len(), 1, "watch sends only hello");
}

#[test]
fn watch_takes_no_device() {
    let (code, out, _) = cli(None, &["--json", "watch", "--device", "fake-a"]);
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"usage\",\"message\":\"watch takes no --device: it prints the events of every device\"}}\n"
    );
}

/// Spec 6.2: `watch` exits 0 on SIGINT or SIGTERM.
#[test]
fn watch_exits_0_on_sigint_and_sigterm() {
    for (n, signal) in ["INT", "TERM"].into_iter().enumerate() {
        let script = [watch_hello(""), hello_reply_line(&three())].join("\n");
        let server = ScriptServer::start(&socket(&format!("watch-signal-{n}")), &script);
        let mut child = Command::new(env!("CARGO_BIN_EXE_obsbot4linux"))
            .arg("watch")
            .env("OBSBOT4LINUX_SOCKET", server.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut first = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut first)
            .unwrap();
        assert!(
            first.starts_with(r#"{"id":1,"ok":{"protocol":2"#),
            "{first}"
        );
        let killed = Command::new("kill")
            .args([format!("-{signal}"), child.id().to_string()])
            .status()
            .unwrap();
        assert!(killed.success());
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(0), "SIG{signal}: {status:?}");
        server.finish().unwrap();
    }
}

/// A writer whose reader went away after `n` lines (`watch | head -1`).
struct Closes(usize);

impl std::io::Write for Closes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.0 == 0 {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        self.0 -= buf.iter().filter(|b| **b == b'\n').count().min(self.0);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Review focus: `watch | head -1` ends quietly with 0 once stdout is gone.
#[test]
fn watch_into_a_closed_pipe_exits_0() {
    // One event only: the server has nothing left to write when the command line closes.
    let lines = [
        watch_hello(""),
        hello_reply_line(&three()),
        format!("< {}", EVENTS[0]),
    ];
    let server = ScriptServer::start(&socket("watch-pipe"), &lines.join("\n"));
    let args: Vec<std::ffi::OsString> = ["obsbot4linux", "watch"].iter().map(Into::into).collect();
    let mut err = Vec::new();
    let code = obsbot4linux::run(
        &args,
        Some(server.path().to_path_buf()),
        false,
        &mut Closes(1),
        &mut err,
    );
    assert_eq!(code, 0);
    assert!(err.is_empty());
    server.finish().unwrap();
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p obsbot4linux --test watch`
Expected: the tests fail with exit 1 and `watch is not built yet (track C)`; the signal test fails reading the first line (the child exits 1 at once).

- [ ] **Step 3: Write `watch`**

Replace `crates/obsbot4linux/src/watch.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `watch` (design spec 6.2): `hello` with `subscribe: true` and the given live topics; the
//! reply as its first line, then every line the daemon sends, unchanged, one JSON object per
//! line. It reads nothing from stdin, takes no `--device`, exits 0 on SIGINT or SIGTERM and
//! 4 when the connection ends.

use crate::Ctx;
use crate::errors::{CliCode, CliError};
use crate::session::{client_error, client_name};
use obsbot_proto::client::{ClientError, Connection};
use obsbot_proto::{ErrorCode, Hello};
use std::io::{self, Write};

/// Writes one line in one call, so that the signal thread never cuts a line.
fn line_out(out: &mut dyn Write, line: &str) -> io::Result<()> {
    out.write_all(format!("{line}\n").as_bytes())?;
    out.flush()
}

/// Blocks SIGINT and SIGTERM in this thread (the only one) and exits 0 when one arrives,
/// after the line being written is complete.
fn exit_on_signal() -> Result<(), CliError> {
    use nix::sys::signal::{SigSet, Signal};
    let mut set = SigSet::empty();
    set.add(Signal::SIGINT);
    set.add(Signal::SIGTERM);
    set.thread_block().map_err(|e| {
        CliError::new(
            CliCode::Proto(ErrorCode::Internal),
            format!("cannot block SIGINT and SIGTERM: {e}"),
        )
    })?;
    std::thread::spawn(move || {
        if set.wait().is_ok() {
            let _whole_lines = io::stdout().lock();
            std::process::exit(0);
        }
    });
    Ok(())
}

pub fn watch(ctx: &mut Ctx, live: &[String]) -> Result<(), CliError> {
    if ctx.device.is_some() {
        return Err(CliError::usage(
            "watch takes no --device: it prints the events of every device",
        ));
    }
    if ctx.exit_on_signal {
        exit_on_signal()?;
    }
    let path = ctx
        .socket
        .as_deref()
        .ok_or_else(|| client_error(ClientError::NoSocketPath))?;
    let mut conn = Connection::open(path).map_err(client_error)?;
    let topics: Vec<&str> = live.iter().map(String::as_str).collect();
    let hello = Hello::new(&client_name()).subscribe(true).live(&topics);
    let reply = conn.hello(hello).map_err(client_error)?;
    if line_out(ctx.out, &reply.line).is_err() {
        return Ok(());
    }
    loop {
        let line = conn.read_line(None).map_err(client_error)?;
        // stdout closed (a pipe whose reader ended): stop like an interrupt.
        if line_out(ctx.out, &line).is_err() {
            return Ok(());
        }
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p obsbot4linux --test watch && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: 4 passed; clean.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linux/src/watch.rs crates/obsbot4linux/tests/watch.rs
git commit -m "obsbot4linux: watch prints the hello reply and every event as received"
```

### Task C-12: End to end against the fake daemon

Starts only after track B's B-1 has landed on `rebuild`.

**Files:**

- Modify: `crates/obsbot4linux/Cargo.toml` (`[dev-dependencies]`: add `obsbot-device.workspace = true` above `obsbot4linuxd.workspace = true`)
- Create: `crates/obsbot4linux/tests/fake_daemon/mod.rs`, `crates/obsbot4linux/tests/e2e.rs`, `crates/obsbot4linux/tests/snapshots/{devices.txt,devices-report.json,status-fake-a.txt,describe-fake-b.txt}` (written by the first run, reviewed)

**Interfaces:**

- Consumes (B-1): `obsbot4linuxd::{Config, run}` serving protocol 2 for a `FakeBus`; (W0) `FakeBus::{new, ops, sleep, wake}`, `Op`, `obsbot_device::models::fake::description`; (C-9, C-10, C-11) the commands; (C-2) `tests/common::{cli, socket}`.
- Produces (test support for C-13): `fake_daemon::FakeDaemon { pub socket: PathBuf, pub bus: Arc<FakeBus> }` with `start(name: &str) -> FakeDaemon`, `cli(&self, args: &[&str]) -> (u8, String, String)`, `op_count(&self, id: &str) -> usize`, `writes_since(&self, id: &str, from: usize) -> Vec<Op>`, `stop(self)`; `fake_daemon::snapshot(name: &str, text: &str)`; `fake_daemon::eventually(what: &str, check: impl FnMut() -> bool)`.

- [ ] **Step 1: Add the dev-dependency**

In `crates/obsbot4linux/Cargo.toml`:

```toml
[dev-dependencies]
obsbot-device.workspace = true
obsbot4linuxd.workspace = true
```

Run: `scripts/check-cli-deps.sh`
Expected: `CLI dependencies: clap nix obsbot-proto serde serde_json` (dev-dependencies are not listed; spec 3 allows them).

- [ ] **Step 2: Write the tests**

Create `crates/obsbot4linux/tests/fake_daemon/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! An in-process `obsbot4linuxd --fake fake-a,fake-b,unknown` on a temporary socket (design
//! spec 5.12 and 11; track B's milestone B-1), the camera operations its fakes received, and
//! snapshot files. A test file uses it with `mod common; mod fake_daemon;`.

#![allow(dead_code)]

use obsbot_device::models::fake::{FakeBus, Op};
use obsbot4linuxd::Config;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// A running fake daemon. Dropping it stops it.
pub struct FakeDaemon {
    pub socket: PathBuf,
    pub bus: Arc<FakeBus>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<std::io::Result<()>>>,
}

impl FakeDaemon {
    /// Starts the daemon with `fake-a`, `fake-b` and `unknown` and waits (at most 20 s) until
    /// it accepts connections.
    pub fn start(name: &str) -> FakeDaemon {
        let socket = crate::common::socket(&format!("e-{name}"));
        let _ = std::fs::remove_file(&socket);
        let bus = Arc::new(FakeBus::new(&["fake-a", "fake-b", "unknown"]).expect("the fakes"));
        let mut config = Config::new(socket.clone());
        config.fake = Some(Arc::clone(&bus));
        let stop = Arc::clone(&config.stop);
        let thread = thread::spawn(move || obsbot4linuxd::run(config));
        let until = Instant::now() + Duration::from_secs(20);
        while UnixStream::connect(&socket).is_err() {
            if thread.is_finished() {
                panic!("the fake daemon ended: {:?}", thread.join());
            }
            assert!(
                Instant::now() < until,
                "the fake daemon does not accept on {}",
                socket.display()
            );
            thread::sleep(Duration::from_millis(20));
        }
        FakeDaemon {
            socket,
            bus,
            stop,
            thread: Some(thread),
        }
    }

    /// Runs the command line in-process against this daemon.
    pub fn cli(&self, args: &[&str]) -> (u8, String, String) {
        crate::common::cli(Some(&self.socket), args)
    }

    /// The number of camera operations the device with this id has received.
    pub fn op_count(&self, id: &str) -> usize {
        self.bus.ops(id).len()
    }

    /// The writes (`Apply`, `Act`) the device with this id received after its first `from`
    /// operations. Reads, identity attempts and opens are left out.
    pub fn writes_since(&self, id: &str, from: usize) -> Vec<Op> {
        self.bus
            .ops(id)
            .into_iter()
            .skip(from)
            .filter(|op| matches!(op, Op::Apply(..) | Op::Act(..)))
            .collect()
    }

    /// Stops the daemon and waits for `run` to return.
    pub fn stop(mut self) {
        self.halt();
    }

    fn halt(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let ended = thread.join().expect("the daemon thread does not panic");
            assert!(ended.is_ok(), "run returned {ended:?}");
        }
    }
}

impl Drop for FakeDaemon {
    fn drop(&mut self) {
        if self.thread.is_some() && !thread::panicking() {
            self.halt();
        }
    }
}

/// Compares `text` with `tests/snapshots/<name>`; `OBSBOT_REGENERATE=1` writes the file.
/// A new or changed snapshot is reviewed in the pull request like code.
pub fn snapshot(name: &str, text: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    if std::env::var_os("OBSBOT_REGENERATE").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        have == text,
        "tests/snapshots/{name} differs (OBSBOT_REGENERATE=1 rewrites it; review the diff):\n{text}"
    );
}

/// Waits (at most 10 s) until `check` holds.
pub fn eventually(what: &str, mut check: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !check() {
        assert!(Instant::now() < until, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(50));
    }
}
```

Create `crates/obsbot4linux/tests/e2e.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! End to end against an in-process `obsbot4linuxd --fake fake-a,fake-b,unknown` (design spec
//! 10.3 track C, 11): the command line, the daemon of track B and the fakes of W0 together.
//! Needs track B's milestone B-1.

mod common;
mod fake_daemon;

use fake_daemon::{FakeDaemon, eventually, snapshot};
use obsbot_device::models::fake::{self, Op};
use serde_json::{Value, json};
use std::io::Write;
use std::sync::{Arc, Mutex};

const A: &str = "sn:FAKEA0000001";
const B: &str = "sn:FAKEB0000001";
const UNKNOWN: &str = "port:9-3";

fn parse(out: &str) -> Value {
    serde_json::from_str(out.trim_end()).unwrap_or_else(|e| panic!("{e}: {out}"))
}

#[test]
fn devices_lists_the_two_fakes_and_the_unsupported_device() {
    let d = FakeDaemon::start("devices");
    let (code, out, _) = d.cli(&["devices", "--json"]);
    assert_eq!(code, 0);
    let v = parse(&out);
    let mut seen: Vec<(String, String, String)> = v["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["id"].as_str().unwrap().to_string(),
                d["model"].as_str().unwrap().to_string(),
                d["support"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        [
            (UNKNOWN.into(), "unknown".into(), "unsupported".into()),
            (A.into(), "fake-a".into(), "tested".into()),
            (B.into(), "fake-b".into(), "untested".into()),
        ]
    );
    let (code, out, _) = d.cli(&["devices"]);
    assert_eq!(code, 0);
    assert!(
        !out.contains("FAKEA0000001"),
        "human output shortens: {out}"
    );
    snapshot("devices.txt", &out);
    let (code, out, _) = d.cli(&["devices", "--report"]);
    assert_eq!(code, 0);
    assert!(!out.contains("FAKE"), "no serial in a report: {out}");
    snapshot("devices-report.json", &out);
}

#[test]
fn two_usable_fakes_need_a_device() {
    let d = FakeDaemon::start("ambiguous");
    let (code, out, _) = d.cli(&["--json", "status"]);
    assert_eq!(code, 2);
    let v = parse(&out);
    assert_eq!(v["error"]["code"], "ambiguous_device");
    assert_eq!(v["error"]["devices"], json!([A, B]));
}

#[test]
fn status_of_fake_a_in_both_forms() {
    let d = FakeDaemon::start("status");
    let (code, out, _) = d.cli(&["status", "--device", "fake-a", "--json"]);
    assert_eq!(code, 0);
    let v = parse(&out);
    assert_eq!(v["device"]["id"], A);
    assert_eq!(v["state"]["available"], true);
    assert!(v["state"]["tracking"]["mode"].is_string());
    let (code, out, _) = d.cli(&["status", "--device", "fake-a"]);
    assert_eq!(code, 0);
    snapshot("status-fake-a.txt", &out);
}

/// Spec 8.4 steps 4 to 6 with the `unknown` fake: listed, refused by the CLI before anything
/// is sent, and it never receives a camera operation.
#[test]
fn the_unsupported_device_is_refused_and_receives_nothing() {
    let d = FakeDaemon::start("unsupported");
    for args in [
        vec!["status", "--device", "unknown"],
        vec!["set", "led.brightness=1", "--device", "unknown"],
        vec!["do", "sleep", "--device", UNKNOWN],
    ] {
        let (code, _, err) = d.cli(&args);
        assert_eq!(code, 3, "{args:?}");
        assert!(err.contains("is not supported by obsbot4linux"), "{err}");
    }
    let (code, out, _) = d.cli(&["describe", "--device", "unknown", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(parse(&out)["settings"], json!({}));
    assert!(d.bus.ops(UNKNOWN).is_empty(), "{:?}", d.bus.ops(UNKNOWN));
}

#[test]
fn a_set_reaches_fake_b_as_one_apply_and_shows_in_the_state() {
    let d = FakeDaemon::start("set");
    let from = d.op_count(B);
    let (code, out, err) = d.cli(&["set", "led.brightness=5", "--device", "fake-b"]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    assert_eq!(
        d.writes_since(B, from),
        [Op::Apply("led.brightness".into(), json!(5))]
    );
    // fake-b's readback lags (W0 decision 13): pending first, or the value once read.
    let (code, out, _) = d.cli(&["--json", "get", "led", "--device", "fake-b"]);
    assert_eq!(code, 0);
    let v = parse(&out);
    assert!(
        v["state"]["pending"]["led.brightness"] == json!(5)
            || v["state"]["led"]["brightness"] == json!(5),
        "{v}"
    );
    eventually("led.brightness to read back", || {
        let (_, out, _) = d.cli(&["--json", "get", "led", "--device", "fake-b"]);
        parse(&out)["state"]["led"]["brightness"] == json!(5)
    });
}

/// The daemon's refusals keep their codes; nothing reaches the fake.
#[test]
fn refusals_reach_no_camera() {
    let d = FakeDaemon::start("refusals");
    let (a, b) = (d.op_count(A), d.op_count(B));
    let cases: [(&[&str], &str, u8); 4] = [
        (
            &["set", "zoom.factor=3", "--device", "fake-b"],
            "out_of_range",
            2,
        ),
        (
            &["set", "exposure.time=500", "--device", "fake-b"],
            "conflict",
            3,
        ),
        (&["set", "gimbal.pan=1", "--device", "fake-a"], "invalid", 2),
        (
            &["set", "image.gamma=1", "--device", "fake-a"],
            "unsupported",
            2,
        ),
    ];
    for (args, code, exit) in cases {
        let mut all = vec!["--json"];
        all.extend_from_slice(args);
        let (got, out, _) = d.cli(&all);
        assert_eq!(got, exit, "{args:?}");
        assert_eq!(parse(&out)["error"]["code"], code, "{args:?}");
    }
    assert!(d.writes_since(A, a).is_empty());
    assert!(d.writes_since(B, b).is_empty());
}

/// fake-b's `zoom.factor` is `asleep: ignored`: refused with `asleep` while it sleeps.
#[test]
fn an_ignored_setting_is_refused_while_asleep() {
    let d = FakeDaemon::start("asleep");
    d.bus.sleep(B).unwrap();
    eventually("fake-b to report asleep", || {
        let (_, out, _) = d.cli(&["--json", "get", "asleep", "--device", "fake-b"]);
        parse(&out)["state"]["asleep"] == json!(true)
    });
    let (code, out, _) = d.cli(&["--json", "set", "zoom.factor=1.5", "--device", "fake-b"]);
    assert_eq!(code, 3);
    assert_eq!(parse(&out)["error"]["code"], "asleep");
    d.bus.wake(B).unwrap();
}

#[test]
fn describe_of_fake_b_is_its_own_description() {
    let d = FakeDaemon::start("describe");
    let desc = fake::description("fake-b").unwrap();
    let (code, out, _) = d.cli(&["describe", "--device", "fake-b", "--json"]);
    assert_eq!(code, 0);
    let v = parse(&out);
    assert_eq!(v["settings"], serde_json::to_value(&desc.settings).unwrap());
    assert_eq!(v["actions"], serde_json::to_value(&desc.actions).unwrap());
    assert_eq!(v["limits"], json!({"presets": 5}));
    let (code, out, _) = d.cli(&["describe", "beacon.flash", "--device", "fake-b", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        serde_json::to_value(&desc.actions["beacon.flash"]).unwrap()
    );
    let (code, out, _) = d.cli(&["describe", "--device", "fake-b"]);
    assert_eq!(code, 0);
    snapshot("describe-fake-b.txt", &out);
}

#[test]
fn an_action_of_a_group_no_client_knows_is_one_act() {
    let d = FakeDaemon::start("beacon");
    let from = d.op_count(B);
    let (code, _, _) = d.cli(&["do", "beacon.flash", "times=2", "--device", "fake-b"]);
    assert_eq!(code, 0);
    let mut params = serde_json::Map::new();
    params.insert("times".into(), json!(2));
    assert_eq!(
        d.writes_since(B, from),
        [Op::Act("beacon.flash".into(), params)]
    );
}

/// Spec 6.2, "Preset names": no name is sent, `--json` shows the empty name the fake stores,
/// human output shows a placeholder.
#[test]
fn a_preset_saved_without_a_name_shows_a_placeholder_only_for_people() {
    let d = FakeDaemon::start("preset");
    let (code, _, _) = d.cli(&["do", "preset.save", "slot=1", "--device", "fake-a"]);
    assert_eq!(code, 0);
    let (_, out, _) = d.cli(&["--json", "get", "presets", "--device", "fake-a"]);
    let presets = parse(&out)["state"]["presets"].clone();
    assert!(
        presets
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["slot"] == 1 && p["name"] == ""),
        "{presets}"
    );
    let (_, out, _) = d.cli(&["get", "presets", "--device", "fake-a"]);
    assert!(out.contains("name (unnamed)"), "{out}");
}

/// A writer the watch thread and the test share.
#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Shared {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

/// `watch` prints the hello reply, then the events a `set` from another connection causes
/// (spec 4.1 rule 3: every subscribed connection), and exits 4 when the daemon stops.
#[test]
fn watch_sees_a_change_made_by_another_command() {
    let d = FakeDaemon::start("watch");
    let socket = d.socket.clone();
    let out = Shared::default();
    let mut writer = out.clone();
    let watch = std::thread::spawn(move || {
        let args: Vec<std::ffi::OsString> = ["obsbot4linux", "watch", "--live", "pose"]
            .iter()
            .map(Into::into)
            .collect();
        obsbot4linux::run(
            &args,
            Some(socket),
            false,
            &mut writer,
            &mut std::io::sink(),
        )
    });
    eventually("the hello reply", || out.text().lines().count() >= 1);
    let first = out.text().lines().next().unwrap().to_string();
    assert!(
        first.starts_with(r#"{"id":1,"ok":{"protocol":2"#),
        "{first}"
    );
    let (code, _, _) = d.cli(&["set", "beacon.color=blue", "--device", "fake-b"]);
    assert_eq!(code, 0);
    eventually("a state event of fake-b with blue", || {
        out.text().lines().skip(1).any(|l| {
            let v: Value = serde_json::from_str(l).unwrap();
            v["event"] == "state" && v["device"] == B && v["state"]["beacon"]["color"] == "blue"
        })
    });
    d.stop();
    assert_eq!(watch.join().unwrap(), 4);
}
```

- [ ] **Step 3: Run them: the snapshots are missing**

Run: `cargo test -p obsbot4linux --test e2e`
Expected: the tests with a `snapshot(…)` call fail with `tests/snapshots/<name> differs`; every other test passes. (Without B-1 every test fails with `the fake daemon ended: … not built yet (stage W0)`: then B-1 has not landed and this task must not start.)

- [ ] **Step 4: Write the snapshots and review them**

Run: `OBSBOT_REGENERATE=1 cargo test -p obsbot4linux --test e2e`

Review each new file under `crates/obsbot4linux/tests/snapshots/` against the spec, and fix the cause, never the snapshot, when one is wrong:

- `devices.txt`: three lines; `…0001  <name>  fake-a`, `…0001  <name>  fake-b  untested`, `port:9-3  <name>  unknown  unsupported`; no full serial (4.2); names as 4.2 defines them (display name plus the last four serial characters; the USB product name plus the USB path for the unsupported device).
- `devices-report.json`: pretty JSON `{"devices":[…]}` with `model`, `support`, `firmware`, `video_node`, `usb` per device; no `id`, no serial, `serial_present` inside `usb` (6.2).
- `status-fake-a.txt`: the device line, `available: yes`, then the groups of the Tiny 2 Lite description in path order with units; `presets` and `views` lists.
- `describe-fake-b.txt`: the `beacon` group and `beacon.flash`; `limits: presets: 5`; `fake-b`'s own enum values (`off|person|desk`).

A difference that comes from the daemon's behaviour (an order, a name, a code) is reported to track B with the spec section; this task does not change a test to match a daemon that contradicts the spec.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p obsbot4linux --test e2e && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check`
Expected: 11 passed; clean. Run it three times; a test that fails once is a race to fix, not to retry.

- [ ] **Step 6: Commit**

```bash
git add Cargo.lock crates/obsbot4linux/Cargo.toml crates/obsbot4linux/tests/fake_daemon crates/obsbot4linux/tests/e2e.rs crates/obsbot4linux/tests/snapshots
git commit -m "obsbot4linux: end-to-end tests against obsbot4linuxd --fake fake-a,fake-b,unknown"
```

### Task C-13: Every parity row against `fake-a`

Starts after C-12 (stacked on it), so after B-1.

**Files:**

- Create: `crates/obsbot4linux/tests/parity.rs`

**Interfaces:**

- Consumes (C-12): `fake_daemon::{FakeDaemon, eventually}` with `start`, `cli`, `op_count`, `writes_since` and the field `bus`; (W0) `FakeBus::ops`, `Op::{Apply, Act, Open}`, `values::same`; spec 9.1 and 9.2.
- Produces: the tests `every_parity_row_reaches_fake_a_as_its_one_request` and `every_0_13_command_of_the_parity_table_has_a_row`.

- [ ] **Step 1: Write the test**

Create `crates/obsbot4linux/tests/parity.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Every row of the parity table (design spec 9.1) turned into its 1.0 command and run
//! against `fake-a` of an in-process `obsbot4linuxd --fake fake-a,fake-b,unknown`: the exit
//! code, and exactly the camera writes the row names. A `set` reaches the fake as one
//! `apply` per leaf and no `act`; a `do` as exactly one `act` and no `apply` (spec 2): the
//! gimbal rows move without touching the tracking. `watch` and `watch --live pose` are
//! covered by `e2e.rs` (`watch_sees_a_change_made_by_another_command`). Needs track B's
//! milestone B-1.

mod common;
mod fake_daemon;

use fake_daemon::{FakeDaemon, eventually};
use obsbot_device::models::fake::Op;
use obsbot_proto::values::same;
use serde_json::{Map, Value, json};

const A: &str = "sn:FAKEA0000001";

/// What a row sends to the camera.
enum Sends {
    /// No camera write (a read, a refusal, help).
    Nothing,
    /// One `apply` per (leaf, JSON value), in any order, and no `act`.
    Apply(&'static [(&'static str, &'static str)]),
    /// Exactly one `act` with these parameters (a JSON object), and no `apply`.
    Act(&'static str, &'static str),
}

use Sends::{Act, Apply, Nothing};

struct Row {
    /// The 0.13.2 command of 9.1.
    was: &'static str,
    /// The 1.0 command; `--device fake-a` is added unless it is `devices`, help or version.
    now: &'static [&'static str],
    exit: u8,
    sends: Sends,
}

const fn row(was: &'static str, now: &'static [&'static str], exit: u8, sends: Sends) -> Row {
    Row {
        was,
        now,
        exit,
        sends,
    }
}

/// In the order they run: presets and views exist before the rows that need them; the
/// tracking is on before the gimbal rows; the rows whose `warning` holds `restart` are last.
const ROWS: &[Row] = &[
    row("--help", &["--help"], 0, Nothing),
    row("--version", &["--version"], 0, Nothing),
    row("--direct (PX-3)", &["--direct", "status"], 2, Nothing),
    row("detect", &["devices"], 0, Nothing),
    row("detect --json", &["devices", "--json"], 0, Nothing),
    row("status", &["status"], 0, Nothing),
    row("status --json", &["status", "--json"], 0, Nothing),
    row("sleep", &["do", "sleep"], 0, Act("sleep", "{}")),
    row("wake", &["do", "wake"], 0, Act("wake", "{}")),
    row(
        "wake zoom F",
        &["set", "sleep.wake_zoom=2"],
        0,
        Apply(&[("sleep.wake_zoom", "2")]),
    ),
    row(
        "wake mode M",
        &["set", "power_on.mode=upper_body"],
        0,
        Apply(&[("power_on.mode", "\"upper_body\"")]),
    ),
    row(
        "tracking MODE",
        &["set", "tracking.mode=normal"],
        0,
        Apply(&[("tracking.mode", "\"normal\"")]),
    ),
    row(
        "speed standard|sport",
        &["set", "tracking.profile=sport"],
        0,
        Apply(&[("tracking.profile", "\"sport\"")]),
    ),
    row(
        "speed headroom",
        &["set", "tracking.profile=headroom"],
        0,
        Apply(&[("tracking.profile", "\"headroom\"")]),
    ),
    row(
        "speed-mode MODE",
        &["set", "tracking.speed=fast"],
        0,
        Apply(&[("tracking.speed", "\"fast\"")]),
    ),
    row(
        "framing auto-zoom on|off",
        &["set", "framing.auto_zoom=on"],
        0,
        Apply(&[("framing.auto_zoom", "true")]),
    ),
    row(
        "framing size S",
        &["set", "framing.size=close_up"],
        0,
        Apply(&[("framing.size", "\"close_up\"")]),
    ),
    row(
        "framing zoom-speed S",
        &["set", "framing.zoom_speed=fast"],
        0,
        Apply(&[("framing.zoom_speed", "\"fast\"")]),
    ),
    row(
        "framing offset --x X --y Y",
        &["set", "framing.offset.x=0.1", "framing.offset.y=-0.1"],
        0,
        Apply(&[("framing.offset.x", "0.1"), ("framing.offset.y", "-0.1")]),
    ),
    row(
        "framing part P",
        &["set", "framing.part=head_removal"],
        0,
        Apply(&[("framing.part", "\"head_removal\"")]),
    ),
    row(
        "hdr on|off",
        &["set", "exposure.hdr=on"],
        0,
        Apply(&[("exposure.hdr", "true")]),
    ),
    row(
        "exposure global|face",
        &["set", "exposure.mode=auto", "exposure.region=face"],
        0,
        Apply(&[
            ("exposure.mode", "\"auto\""),
            ("exposure.region", "\"face\""),
        ]),
    ),
    row(
        "exposure manual",
        &["set", "exposure.mode=manual"],
        0,
        Apply(&[("exposure.mode", "\"manual\"")]),
    ),
    row(
        "exposure manual --time T",
        &["set", "exposure.mode=manual", "exposure.time=20000"],
        0,
        Apply(&[("exposure.mode", "\"manual\""), ("exposure.time", "20000")]),
    ),
    row(
        "exposure manual --gain G",
        &["set", "exposure.mode=manual", "exposure.gain=8"],
        0,
        Apply(&[("exposure.mode", "\"manual\""), ("exposure.gain", "8")]),
    ),
    row(
        "zoom F",
        &["set", "zoom.factor=2"],
        0,
        Apply(&[("zoom.factor", "2")]),
    ),
    row(
        "zoom F --speed S",
        &["do", "zoom.move", "factor=1.5", "speed=medium"],
        0,
        Act("zoom.move", r#"{"factor":1.5,"speed":"medium"}"#),
    ),
    row("zoom stop", &["do", "zoom.stop"], 0, Act("zoom.stop", "{}")),
    row(
        "gimbal --pan P --tilt T",
        &["do", "gimbal.move", "pan=10", "tilt=5"],
        0,
        Act("gimbal.move", r#"{"pan":10,"tilt":5}"#),
    ),
    row(
        "gimbal --pan P (one axis)",
        &["do", "gimbal.move", "pan=-10"],
        0,
        Act("gimbal.move", r#"{"pan":-10}"#),
    ),
    row(
        "gimbal … --speed S",
        &["do", "gimbal.move", "pan=10", "tilt=5", "speed=30"],
        0,
        Act("gimbal.move", r#"{"pan":10,"tilt":5,"speed":30}"#),
    ),
    row(
        "gimbal stop",
        &["do", "gimbal.stop"],
        0,
        Act("gimbal.stop", "{}"),
    ),
    row(
        "gimbal rate --pan P --tilt T",
        &["do", "gimbal.turn", "pan_speed=20", "tilt_speed=0"],
        0,
        Act(
            "gimbal.turn",
            r#"{"pan_speed":20,"tilt_speed":0,"keep":true}"#,
        ),
    ),
    row(
        "gimbal stop (after rate)",
        &["do", "gimbal.stop"],
        0,
        Act("gimbal.stop", "{}"),
    ),
    row(
        "fov wide|medium|narrow",
        &["set", "zoom.field_of_view=wide"],
        0,
        Apply(&[("zoom.field_of_view", "\"wide\"")]),
    ),
    row(
        "led N",
        &["set", "led.brightness=2"],
        0,
        Apply(&[("led.brightness", "2")]),
    ),
    row(
        "led blink on|off",
        &["set", "led.blink=on"],
        0,
        Apply(&[("led.blink", "true")]),
    ),
    row(
        "mirror on|off",
        &["set", "image.mirror=on"],
        0,
        Apply(&[("image.mirror", "true")]),
    ),
    row(
        "invert on|off",
        &["set", "gimbal.invert=on"],
        0,
        Apply(&[("gimbal.invert", "true")]),
    ),
    row(
        "auto-sleep off",
        &["set", "sleep.auto.enabled=off"],
        0,
        Apply(&[("sleep.auto.enabled", "false")]),
    ),
    row(
        "auto-sleep N",
        &["set", "sleep.auto.enabled=on", "sleep.auto.seconds=300"],
        0,
        Apply(&[
            ("sleep.auto.enabled", "true"),
            ("sleep.auto.seconds", "300"),
        ]),
    ),
    row(
        "auto-sleep 0 (PX-7)",
        &["set", "sleep.auto.seconds=0"],
        2,
        Nothing,
    ),
    row(
        "sleep-background black|image",
        &["set", "sleep.background=black"],
        0,
        Apply(&[("sleep.background", "\"black\"")]),
    ),
    row(
        "sleep-background-mirror on|off",
        &["set", "sleep.background_mirror=on"],
        0,
        Apply(&[("sleep.background_mirror", "true")]),
    ),
    row(
        "audio noise L",
        &["set", "audio.noise_reduction=strong"],
        0,
        Apply(&[("audio.noise_reduction", "\"strong\"")]),
    ),
    row(
        "audio gain on|off",
        &["set", "audio.auto_gain=on"],
        0,
        Apply(&[("audio.auto_gain", "true")]),
    ),
    row(
        "audio distance near",
        &["set", "audio.volume=10"],
        0,
        Apply(&[("audio.volume", "10")]),
    ),
    row(
        "audio distance standard",
        &["set", "audio.volume=50"],
        0,
        Apply(&[("audio.volume", "50")]),
    ),
    row(
        "audio distance far",
        &["set", "audio.volume=100"],
        0,
        Apply(&[("audio.volume", "100")]),
    ),
    row(
        "audio volume N",
        &["set", "audio.volume=70"],
        0,
        Apply(&[("audio.volume", "70")]),
    ),
    row(
        "audio sleep-mic on|off",
        &["set", "audio.in_sleep=on"],
        0,
        Apply(&[("audio.in_sleep", "true")]),
    ),
    row(
        "gesture set target-lock",
        &["set", "gestures.target_lock=on"],
        0,
        Apply(&[("gestures.target_lock", "true")]),
    ),
    row(
        "gesture set zoom",
        &["set", "gestures.zoom=on"],
        0,
        Apply(&[("gestures.zoom", "true")]),
    ),
    row(
        "gesture set dynamic-zoom",
        &["set", "gestures.dynamic_zoom=on"],
        0,
        Apply(&[("gestures.dynamic_zoom", "true")]),
    ),
    row(
        "gesture set direction-change",
        &["set", "gestures.direction_change=on"],
        0,
        Apply(&[("gestures.direction_change", "true")]),
    ),
    row(
        "gesture zoom-ratio R",
        &["set", "gestures.zoom_ratio=2.5"],
        0,
        Apply(&[("gestures.zoom_ratio", "2.5")]),
    ),
    row(
        "hand right|left",
        &["set", "hand.side=left"],
        0,
        Apply(&[("hand.side", "\"left\"")]),
    ),
    row(
        "hand return-here",
        &["do", "hand.return_here"],
        0,
        Act("hand.return_here", "{}"),
    ),
    row(
        "hand return-go",
        &["do", "hand.go_return"],
        0,
        Act("hand.go_return", "{}"),
    ),
    row(
        "hand return-reset",
        &["do", "hand.return_clear"],
        0,
        Act("hand.return_clear", "{}"),
    ),
    row(
        "hand area --pan --tilt",
        &["set", "hand.area.pan=-45,45", "hand.area.tilt=-30,30"],
        0,
        Apply(&[
            ("hand.area.pan", r#"{"min":-45,"max":45}"#),
            ("hand.area.tilt", r#"{"min":-30,"max":30}"#),
        ]),
    ),
    row(
        "hand area-reset",
        &["do", "hand.area_reset"],
        0,
        Act("hand.area_reset", "{}"),
    ),
    row(
        "hand pan on|off",
        &["set", "hand.axes.pan=on"],
        0,
        Apply(&[("hand.axes.pan", "true")]),
    ),
    row(
        "hand tilt on|off",
        &["set", "hand.axes.tilt=off"],
        0,
        Apply(&[("hand.axes.tilt", "false")]),
    ),
    row(
        "hand return-after S",
        &["set", "hand.return_after=5"],
        0,
        Apply(&[("hand.return_after", "5")]),
    ),
    row(
        "hand gimbal on|off",
        &["set", "hand.move_gimbal=off"],
        0,
        Apply(&[("hand.move_gimbal", "false")]),
    ),
    row(
        "zone on|off",
        &["set", "zone.enabled=on"],
        0,
        Apply(&[("zone.enabled", "true")]),
    ),
    row(
        "zone follow on|off",
        &["set", "zone.follow=on"],
        0,
        Apply(&[("zone.follow", "true")]),
    ),
    row(
        "zone area --pan --tilt",
        &["set", "zone.area.pan=-20,20", "zone.area.tilt=-10,10"],
        0,
        Apply(&[
            ("zone.area.pan", r#"{"min":-20,"max":20}"#),
            ("zone.area.tilt", r#"{"min":-10,"max":10}"#),
        ]),
    ),
    row(
        "zone return-here",
        &["do", "zone.return_here"],
        0,
        Act("zone.return_here", "{}"),
    ),
    row(
        "zone return-go",
        &["do", "zone.go_return"],
        0,
        Act("zone.go_return", "{}"),
    ),
    row("zone view list", &["get", "views"], 0, Nothing),
    row(
        "zone view save NAME",
        &["do", "view.save", "name=Desk"],
        0,
        Act("view.save", r#"{"name":"Desk"}"#),
    ),
    row(
        "zone view save NAME --pan --tilt --zoom",
        &["do", "view.save", "name=Door", "pose=10,5,1.2"],
        0,
        Act(
            "view.save",
            r#"{"name":"Door","pose":{"pan":10,"tilt":5,"zoom":1.2}}"#,
        ),
    ),
    row(
        "zone view rename ID NAME",
        &["do", "view.rename", "id=0", "name=Desk2"],
        0,
        Act("view.rename", r#"{"id":0,"name":"Desk2"}"#),
    ),
    row(
        "zone view go ID",
        &["do", "view.go", "id=0"],
        0,
        Act("view.go", r#"{"id":0}"#),
    ),
    row(
        "zone view delete ID",
        &["do", "view.delete", "id=1"],
        0,
        Act("view.delete", r#"{"id":1}"#),
    ),
    row(
        "focus mode face|global",
        &["set", "focus.region=face"],
        0,
        Apply(&[("focus.region", "\"face\"")]),
    ),
    row(
        "focus auto on",
        &["set", "focus.mode=continuous"],
        0,
        Apply(&[("focus.mode", "\"continuous\"")]),
    ),
    row(
        "focus auto off (CAM-14)",
        &["set", "focus.mode=manual"],
        0,
        Apply(&[("focus.mode", "\"manual\"")]),
    ),
    row(
        "focus af continuous",
        &["set", "focus.mode=continuous"],
        0,
        Apply(&[("focus.mode", "\"continuous\"")]),
    ),
    row(
        "focus af manual",
        &["set", "focus.mode=manual"],
        0,
        Apply(&[("focus.mode", "\"manual\"")]),
    ),
    row(
        "focus af single",
        &["set", "focus.mode=single"],
        0,
        Apply(&[("focus.mode", "\"single\"")]),
    ),
    row(
        "focus point X Y",
        &["do", "focus.point", "x=0.5", "y=0.5"],
        0,
        Act("focus.point", r#"{"x":0.5,"y":0.5}"#),
    ),
    row(
        "focus set N",
        &["set", "focus.mode=manual", "focus.position=40"],
        0,
        Apply(&[("focus.mode", "\"manual\""), ("focus.position", "40")]),
    ),
    row("preset list", &["get", "presets"], 0, Nothing),
    row(
        "preset save N",
        &["do", "preset.save", "slot=0"],
        0,
        Act("preset.save", r#"{"slot":0}"#),
    ),
    row(
        "preset save N --name S",
        &["do", "preset.save", "slot=1", "name=Door"],
        0,
        Act("preset.save", r#"{"slot":1,"name":"Door"}"#),
    ),
    row(
        "preset save N --pan --tilt --zoom",
        &["do", "preset.save", "slot=2", "pose=10,0,1"],
        0,
        Act(
            "preset.save",
            r#"{"slot":2,"pose":{"pan":10,"tilt":0,"zoom":1}}"#,
        ),
    ),
    row(
        "preset rename N S",
        &["do", "preset.rename", "slot=0", "name=Desk"],
        0,
        Act("preset.rename", r#"{"slot":0,"name":"Desk"}"#),
    ),
    row(
        "preset recall N",
        &["do", "preset.recall", "slot=0"],
        0,
        Act("preset.recall", r#"{"slot":0}"#),
    ),
    row(
        "preset delete N",
        &["do", "preset.delete", "slot=2"],
        0,
        Act("preset.delete", r#"{"slot":2}"#),
    ),
    row(
        "startup set --pan --tilt --zoom",
        &["do", "power_on.set_pose", "pose=0,10,1"],
        0,
        Act(
            "power_on.set_pose",
            r#"{"pose":{"pan":0,"tilt":10,"zoom":1}}"#,
        ),
    ),
    row(
        "startup set --preset N",
        &["do", "power_on.use_preset", "slot=0"],
        0,
        Act("power_on.use_preset", r#"{"slot":0}"#),
    ),
    row(
        "startup set --current",
        &["do", "power_on.use_current"],
        0,
        Act("power_on.use_current", "{}"),
    ),
    // `startup set <pose option> --ai M` is two commands: the `do` of the pose option, then
    // the `set` (spec 9.1); each is its own row and its own request.
    row(
        "startup set --current --ai M (1 of 2)",
        &["do", "power_on.use_current"],
        0,
        Act("power_on.use_current", "{}"),
    ),
    row(
        "startup set --current --ai M (2 of 2)",
        &["set", "power_on.start.mode=upper_body"],
        0,
        Apply(&[("power_on.start.mode", "\"upper_body\"")]),
    ),
    row(
        "startup set --ai M",
        &["set", "power_on.start.mode=upper_body"],
        0,
        Apply(&[("power_on.start.mode", "\"upper_body\"")]),
    ),
    row(
        "startup set --ai none",
        &["set", "power_on.start.mode=unset"],
        0,
        Apply(&[("power_on.start.mode", "\"unset\"")]),
    ),
    row("startup show", &["get", "power_on"], 0, Nothing),
    row(
        "startup go",
        &["do", "power_on.go"],
        0,
        Act("power_on.go", "{}"),
    ),
    row(
        "image brightness V",
        &["set", "image.brightness=60"],
        0,
        Apply(&[("image.brightness", "60")]),
    ),
    row(
        "image contrast V",
        &["set", "image.contrast=60"],
        0,
        Apply(&[("image.contrast", "60")]),
    ),
    row(
        "image saturation V",
        &["set", "image.saturation=60"],
        0,
        Apply(&[("image.saturation", "60")]),
    ),
    row(
        "image hue V",
        &["set", "image.hue=60"],
        0,
        Apply(&[("image.hue", "60")]),
    ),
    row(
        "image sharpness V",
        &["set", "image.sharpness=60"],
        0,
        Apply(&[("image.sharpness", "60")]),
    ),
    row(
        "image white_balance_auto 0",
        &["set", "image.white_balance.auto=off"],
        0,
        Apply(&[("image.white_balance.auto", "false")]),
    ),
    row(
        "image white_balance_temperature V",
        &[
            "set",
            "image.white_balance.auto=off",
            "image.white_balance.temperature=4500",
        ],
        0,
        Apply(&[
            ("image.white_balance.auto", "false"),
            ("image.white_balance.temperature", "4500"),
        ]),
    ),
    row(
        "image white_balance_auto 1",
        &["set", "image.white_balance.auto=on"],
        0,
        Apply(&[("image.white_balance.auto", "true")]),
    ),
    row(
        "image gain V",
        &["set", "exposure.gain=8"],
        0,
        Apply(&[("exposure.gain", "8")]),
    ),
    row(
        "image power_line_frequency 0",
        &["set", "exposure.anti_flicker=off"],
        0,
        Apply(&[("exposure.anti_flicker", "\"off\"")]),
    ),
    row(
        "image power_line_frequency 2",
        &["set", "exposure.anti_flicker=60hz"],
        0,
        Apply(&[("exposure.anti_flicker", "\"60hz\"")]),
    ),
    row(
        "image power_line_frequency 1",
        &["set", "exposure.anti_flicker=50hz"],
        0,
        Apply(&[("exposure.anti_flicker", "\"50hz\"")]),
    ),
    row(
        "image backlight_compensation V",
        &["set", "exposure.compensation=12"],
        0,
        Apply(&[("exposure.compensation", "12")]),
    ),
    row(
        "image auto_exposure 0|3",
        &["set", "exposure.mode=auto"],
        0,
        Apply(&[("exposure.mode", "\"auto\"")]),
    ),
    row(
        "image auto_exposure 1",
        &["set", "exposure.mode=manual"],
        0,
        Apply(&[("exposure.mode", "\"manual\"")]),
    ),
    row(
        "image exposure_time V",
        &["set", "exposure.mode=manual", "exposure.time=10000"],
        0,
        Apply(&[("exposure.mode", "\"manual\""), ("exposure.time", "10000")]),
    ),
    row(
        "image focus V",
        &["set", "focus.mode=manual", "focus.position=30"],
        0,
        Apply(&[("focus.mode", "\"manual\""), ("focus.position", "30")]),
    ),
    row(
        "image autofocus 0 (CAM-14)",
        &["set", "focus.mode=manual"],
        0,
        Apply(&[("focus.mode", "\"manual\"")]),
    ),
    row(
        "image autofocus 1",
        &["set", "focus.mode=continuous"],
        0,
        Apply(&[("focus.mode", "\"continuous\"")]),
    ),
    row(
        "image gamma V (PX-2)",
        &["set", "image.gamma=1"],
        2,
        Nothing,
    ),
    row(
        "factory-reset without --yes",
        &["do", "factory_reset"],
        2,
        Nothing,
    ),
];

/// The rows whose write restarts the camera, run last, each followed by a wait until the
/// device is back.
const RESTARTS: &[Row] = &[
    row(
        "audio mic on|off",
        &["set", "audio.enabled=off"],
        0,
        Apply(&[("audio.enabled", "false")]),
    ),
    row(
        "factory-reset --yes",
        &["do", "factory_reset", "confirm=true"],
        0,
        Act("factory_reset", r#"{"confirm":true}"#),
    ),
];

fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}: {text}"))
}

fn args_of(row: &Row) -> Vec<&'static str> {
    let mut args = row.now.to_vec();
    let global = matches!(args[0], "devices" | "--help" | "--version");
    if !global {
        args.extend(["--device", "fake-a"]);
    }
    args
}

fn check(d: &FakeDaemon, row: &Row) {
    let from = d.op_count(A);
    let args = args_of(row);
    let (code, out, err) = d.cli(&args);
    assert_eq!(code, row.exit, "{}: {args:?}\n{out}{err}", row.was);
    let writes = d.writes_since(A, from);
    match &row.sends {
        Nothing => assert!(writes.is_empty(), "{}: {writes:?}", row.was),
        Act(action, params) => {
            let params: Map<String, Value> = json(params).as_object().unwrap().clone();
            assert_eq!(
                writes,
                [Op::Act(action.to_string(), params)],
                "{}: one act, no apply",
                row.was
            );
        }
        Apply(leaves) => {
            assert!(
                writes.iter().all(|w| matches!(w, Op::Apply(..))),
                "{}: no act: {writes:?}",
                row.was
            );
            assert_eq!(writes.len(), leaves.len(), "{}: {writes:?}", row.was);
            for (leaf, value) in *leaves {
                let want = json(value);
                assert!(
                    writes
                        .iter()
                        .any(|w| matches!(w, Op::Apply(l, v) if l == leaf && same(v, &want))),
                    "{}: no apply of {leaf} = {value}: {writes:?}",
                    row.was
                );
            }
        }
    }
}

fn tracking_mode(d: &FakeDaemon) -> Value {
    let (_, out, _) = d.cli(&["--json", "get", "tracking.mode", "--device", "fake-a"]);
    json(out.trim_end())["state"]["tracking"]["mode"].clone()
}

#[test]
fn every_parity_row_reaches_fake_a_as_its_one_request() {
    let d = FakeDaemon::start("parity");
    for row in ROWS {
        check(&d, row);
        // Spec 2 and 4.4: the gimbal rows leave the tracking as it is.
        if row.now.first() == Some(&"do") && row.now[1].starts_with("gimbal.") {
            assert_eq!(tracking_mode(&d), json!("normal"), "{}", row.was);
        }
    }
    for row in RESTARTS {
        // The restart is ordinary hotplug (spec 5.6): fake-a goes away and comes back with a
        // new device number. Until it goes, the daemon still shows it available, so the next
        // row waits for the re-open first, then for the published availability.
        let from = d.op_count(A);
        check(&d, row);
        eventually("fake-a re-opened after the restart", || {
            d.bus
                .ops(A)
                .iter()
                .skip(from)
                .any(|op| matches!(op, Op::Open))
        });
        eventually("fake-a back after the restart", || {
            let (code, out, _) = d.cli(&["--json", "get", "available", "--device", "fake-a"]);
            code == 0 && json(out.trim_end())["state"]["available"] == json!(true)
        });
    }
}

/// Every 0.13.2 command of 9.1 has a row above, or is covered elsewhere.
#[test]
fn every_0_13_command_of_the_parity_table_has_a_row() {
    const COMMANDS: [&str; 35] = [
        "--json",
        "--direct",
        "--help",
        "--version",
        "daemon",
        "watch",
        "live-pose",
        "detect",
        "status",
        "sleep",
        "wake",
        "tracking",
        "speed",
        "speed-mode",
        "framing",
        "hdr",
        "exposure",
        "zoom",
        "gimbal",
        "fov",
        "led",
        "mirror",
        "invert",
        "auto-sleep",
        "sleep-background",
        "sleep-background-mirror",
        "audio",
        "gesture",
        "hand",
        "zone",
        "focus",
        "startup",
        "image",
        "preset",
        "factory-reset",
    ];
    let elsewhere = [
        "watch: e2e.rs watch_sees_a_change_made_by_another_command",
        "live-pose: watch --live pose, the same e2e test",
        "daemon: obsbot4linuxd, track B",
        "gimbal center: PX-1, no 1.0 command",
    ];
    for command in COMMANDS {
        let has_row = ROWS
            .iter()
            .chain(RESTARTS)
            .any(|r| r.was.split_whitespace().any(|w| w == command));
        let named = elsewhere
            .iter()
            .any(|e| e.starts_with(&format!("{command}:")));
        assert!(has_row || named, "{command} has no row");
    }
}
```

- [ ] **Step 2: Run it**

Run: `cargo test -p obsbot4linux --test parity`
Expected: 2 passed. A row that fails names its 0.13 command, the arguments and the writes the fake received. Before this plan was written, every row was run through W0's validator, `fake-a`'s `check`, `apply` and `act` without the daemon (see "Proof"), so a failing row points at the daemon or at a change of `descriptions/tiny2lite.json`: report the first to track B with the spec section; for the second, change the row's value to one inside the new bounds and say so in the PR. A restart row (`RESTARTS`) is followed by an unplug and a re-plug of `fake-a` with a new device number (spec 5.6, W0's `fake::RESTART`); the next row may only run after the re-open (`Op::Open`) and the published `available: true`, which the loop waits for. A row sent before the re-open is answered `unavailable` (exit 3): that is the test's order, not the daemon's fault.

- [ ] **Step 3: Check the whole workspace**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked`
Expected: clean; every `test result: ok`; ignored: only `the_zsh_script_parses`.

- [ ] **Step 4: Commit**

```bash
git add crates/obsbot4linux/tests/parity.rs
git commit -m "obsbot4linux: every row of the parity table against fake-a, one request each"
```

### Task C-R (conditional): `relay`

Built only if track D's first check shows that the widget cannot open the Unix socket from the desktop shell (spec 6.2, "Relay fallback"; 10.3 D). It is not in the task list; the coordinator adds it with track D's result.

- **Files:** `crates/obsbot4linux/src/args.rs` (a `Relay` subcommand: "Copy stdin lines to the daemon and its lines to stdout"), `crates/obsbot4linux/src/lib.rs` (one dispatch line), new `crates/obsbot4linux/src/relay.rs`, new `crates/obsbot4linux/tests/relay.rs`; `packaging/completions/*` regenerated (a new subcommand).
- **Behaviour:** open `Connection::open(socket)` (no `hello` of its own: the widget sends it); one thread copies each stdin line unchanged to the socket (`write_all` of the line and `\n`), the main thread copies each socket line unchanged to stdout and flushes; it "sends nothing of its own". EOF on stdin shuts the socket's write half and waits for the daemon to close; the end of the connection exits 4 (`daemon`); SIGINT/SIGTERM exit 0 as in `watch`; a line longer than 64 KiB is passed on (the daemon closes the connection, spec 4.1).
- **Tests** (scripted daemon): the lines of `session.jsonl` piped through `relay` arrive byte-equal in both directions; nothing is sent before the first stdin line; exit 4 at `!close`.

---

## Acceptance of track C (spec 10.3), mapped

| Acceptance without camera                                                                                                     | Where                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| value parsing per setting type                                                                                                | C-4 `value::tests` (bool, enum, number, range, pose, point, box, string, list, unknown type)                                                                                                                                                                                                                            |
| rendering snapshots from the golden states                                                                                    | C-6 `the_golden_hello_renders_grouped_with_units`, `a_pending_value_is_shown_and_marked`, `an_unplugged_device_and_an_unsupported_entry_render`; C-7 `the_golden_device_entry_renders`                                                                                                                                  |
| one test per error code, the CLI-only codes included, for its exit code                                                       | C-2 `every_code_has_its_exit_code` (all 16 codes and an unknown one); C-10 `every_daemon_error_code_gives_its_exit_code` (12 daemon codes through a real command); C-5/C-9 the CLI-only codes through commands (`usage`, `ambiguous_device`, `no_device`, `daemon`, `unknown_device`, `unsupported_device`, `protocol`) |
| the `--json` error shape for a usage error                                                                                    | C-2 `a_usage_error_with_json_prints_the_error_object_on_stdout`, `json_after_the_subcommand_counts_too`; C-10 `a_value_that_does_not_parse_sends_nothing`                                                                                                                                                               |
| the `--json` shapes of `devices` and `describe`                                                                               | C-9 `devices_json_is_the_hello_list`, `devices_report_has_usb_facts_and_no_serial`, `describe_json_prints_the_device_entry_setting_or_action`; C-12 `describe_of_fake_b_is_its_own_description`                                                                                                                         |
| an unsupported device refused before anything is sent                                                                         | C-9 `an_unsupported_device_is_refused_before_anything_is_sent` (status, get), C-10 `an_unsupported_device_is_refused_before_set_or_do` (the script sees only `hello`); C-12 `the_unsupported_device_is_refused_and_receives_nothing` (no camera operation)                                                              |
| a non-protocol-2 `hello` reply gives exit 4 and the restart message                                                           | C-1 `a_0_13_daemon_is_reported_as_older_than_1_0`; C-5 `an_older_daemon_is_exit_4_with_the_restart_message`; C-9 `an_older_daemon_is_exit_4_with_the_restart_message`                                                                                                                                                   |
| no socket gives exit 4 and the `systemctl` line                                                                               | C-5 `an_unreachable_daemon_is_exit_4_with_the_enable_line`; C-9 `no_socket_is_exit_4_with_the_enable_line`                                                                                                                                                                                                              |
| end-to-end snapshots against an in-process `obsbot4linuxd --fake fake-a,fake-b,unknown`                                       | C-12 (`devices.txt`, `devices-report.json`, `status-fake-a.txt`, `describe-fake-b.txt` and 11 tests)                                                                                                                                                                                                                    |
| every row of 9.1 turned into its request against `fake-a`, the gimbal rows reaching the fake as one `act` each and no `apply` | C-13 `every_parity_row_reaches_fake_a_as_its_one_request` (and `tracking.mode` unchanged after each gimbal row), `every_0_13_command_of_the_parity_table_has_a_row`                                                                                                                                                     |
| the completions stale test                                                                                                    | C-8 `the_completion_files_are_current`                                                                                                                                                                                                                                                                                  |
| the client module of 6.1                                                                                                      | C-1 `tests/client.rs`, C-3 `tests/client_subscribe.rs` (the golden session replayed)                                                                                                                                                                                                                                    |
| the older-daemon message (10.3 C content)                                                                                     | C-5, C-9 as above                                                                                                                                                                                                                                                                                                       |
| acceptance with camera: none                                                                                                  | no task touches a camera                                                                                                                                                                                                                                                                                                |

## Camera items this track leaves open

No task's acceptance needs the camera. These items of spec 14 change what a user of the CLI sees on the real Tiny 2 Lite, not the CLI's code (it is generic):

- **CAM-35**: what the camera reports as the name of a preset saved without one. The CLI shows `(unnamed)` for an empty name only; any other name the camera invents is shown as reported.
- **CAM-34**: what a manual move does while tracking. The CLI sends only the move either way (C-10, C-13).
- **CAM-14**: `focus auto off` maps to `focus.mode=manual` or `single`; the parity rows use the description's values, and W4 maps the real row.
- **CAM-10**: how long a cold `hello` takes on the camera, against the client's 30 s reply timeout.
- **CAM-7**: whether a `keep:true` turn stops when the camera falls asleep (documentation of `do gimbal.turn`).
- **CAM-24** (stage W4): every row of 9.1 on the real unit; C-13 is its fake rehearsal.
- **U-rows of 9.3** (U-4 to U-26): the values and bounds the CLI completes and sends come from `descriptions/tiny2lite.json`; when CS-1 removes a value, `scripts/regenerate.sh` regenerates the completions and no CLI code changes (guarantee A).

## Self-review

1. **Spec coverage.** Section 2 and 6.2 "one request per command": every command test runs against the scripted daemon, which fails on any line after the script (C-9, C-10, C-11), and C-12/C-13 count the fake's camera operations. 3: dependency rule (C-2 Step 5, C-12 Step 1). 4.1: reply timeout, uncapped lines, ids, unknown codes and events (C-1, C-3). 4.2: shortened ids in human output only (C-2, C-5, C-6, C-9, C-12), units (C-6). 4.4: `keep` (C-10), `preset.save` without name (C-10, C-12). 5.8, 5.10: C-1, C-5, C-9. 6.1: C-1, C-3. 6.2: every bullet (commands C-9 to C-11, selection C-5, unsupported and no device C-5/C-9, value syntax C-4, output C-6/C-7, preset names C-6/C-9/C-12, errors C-2/C-10, report C-9, watch C-11, exit codes C-2, one-shot `subscribe:false` C-2 `hello_line`, `gimbal.turn` C-10, one request C-9 to C-13, completions C-8, relay C-R). 8.4 steps 4 to 6: C-9, C-12. 8.6: C-2 `device_line`, C-9. 9.1 and 9.2 (PX-2, PX-3, PX-5, PX-7): C-13, C-2, C-9. 11: in-process daemon C-12. 12.2: the migration-note behaviours (`gimbal` leaves the tracking, `preset save` without a name) are pinned in C-10/C-13.
2. **Placeholder scan.** Every file of every task is given whole, or as the exact lines to change (C-3's two methods, C-8's `regenerate.sh` shown whole, C-12's manifest table). The snapshot files of C-12 are generated and reviewed, with the review list given.
3. **Type consistency.** All interfaces were compiled together and task by task (see "Proof"): `ClientError::Refused` holds a `Box<ErrorBody>` everywhere; `CliError::body` is `Option<Box<ErrorBody>>`; the stubs of C-2 have the signatures the later tasks fill; `write::KEEP` exists from C-2 on, which C-8 needs before C-10 lands.
4. **Review Focus.** Five inputs, each with its test in its owning task (C-4, C-5, C-10, C-11).
5. **Fix round (2026-11-13).** Re-run after the review: coverage as in the table above, with every sub-row of 9.1 now its own C-13 row (`focus af continuous|single|manual`, `image power_line_frequency 0|1|2`, `image autofocus 0|1`, the two commands of `startup set <pose option> --ai M`) and the restart rows waiting for the re-open; no placeholder; names consistent across tasks and the task list (`common::socket`, `FakeDaemon::bus`, `Op::Open`, C-13 in wave 5); the task list checked with `jq` (13 tasks, 13 fields each, unique keys, every `depends_on` and `stacked_on` key exists and lies in an earlier wave, no file in two tasks of one wave).

## Proof

- **Built from the W0 plan:** the W0 code was extracted from the W0 plan into a copy of `rebuild` (stage G at `37471a0`) with the plan's manual edits, `scripts/regenerate.sh` generated `schema/` and `descriptions/`, and W0's own tests passed.
- **Run green (rustc 1.99.1, `cargo fmt --check`, `cargo clippy -p obsbot-proto -p obsbot4linux --all-targets --locked -D warnings`):** every task's code, task by task on the W0 base in plan order: W0 alone; C-1; C-2; C-1+C-3; C-2+C-4; C-1+C-2+C-5; C-2+C-6; C-2+C-7; C-2+C-8 (with the generated completions); all of wave 2; wave 2 + C-9, + C-10, + C-11 each alone; then everything. Tests: `obsbot-proto` 14 unit (4 new), `tests/client.rs` 13, `tests/client_subscribe.rs` 3; `obsbot4linux` 48 unit, `cli_errors` 5, `completions` 3 + 1 ignored (the zsh syntax test, also run green with zsh 5.9), `read` 20, `write` 12, `watch` 4 (the signal test runs the binary). The scripted-daemon tests of `read`, `write` and `watch` were run five times in a row and those of `obsbot-proto` three times, without a failure.
- **Fix round after review (2026-11-13), on the landed W0 tree (`rebuild` at `3187b84`):** the whole plan's code extracted onto that tree; `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets --locked -- -D warnings` clean (so every task's clippy step is the workspace form), `cargo test --workspace --locked --no-fail-fast` with the counts above, the completion files regenerated and current. The changed `Sender::send` (C-3) and test socket names (C-2, C-12) were run green five times in a row (`read`, `write`, `watch`, `client`, `client_subscribe`). A scratch test (not in the plan) ran the five added parity rows through W0's `validate_set`/`validate_do` and `fake-a`'s `apply`/`check`/`act`, and checked the restart order the C-13 loop now waits for: after an `audio.enabled` write `fake-a` leaves the scan, comes back after `fake::RESTART` with a new device number, and only its re-open adds an `Op::Open` after the row's operations.
- **Parity rows without the daemon:** a scratch test (not part of the plan) ran every row of C-13 through `parse_patch`/`parse_params`/`keep`, W0's `validate_set`/`validate_do`, `fake-a`'s `check`, `order`, `apply`, `act` and the touched re-reads: every row gave its expected exit code and exactly its writes, `tracking.mode` stayed `normal` after each gimbal row; a mutated row (a wrong value) made it fail.
- **Compiled only:** `tests/e2e.rs` (11 tests) and `tests/parity.rs` (the row test) against W0's stub `run`; they fail at start with "obsbot4linuxd is not built yet (stage W0)", as expected until B-1. `every_0_13_command_of_the_parity_table_has_a_row` runs green now.
- **Unproven:** the e2e and parity tests against the real B-1 daemon (its device names, ordering, timing of `pending`, the restart re-plug of a fake with its hand-over); the snapshot contents; the zsh script's behaviour in an interactive zsh (only its syntax was checked); the bash script on the CI runner's bash 5.1 (run here with bash 5.3).

## Notes for the coordinator

1. **One file outside the owner list:** `scripts/regenerate.sh` (W0's, not a contract file) gets one line in C-8, because spec 6.2 has the completions regenerated by that script. No other W1 task edits it.
2. The CLI crate's `[package.metadata.deb]` and `generate-rpm` tables (W3's) are untouched; the completion files are installed by W3 from `packaging/completions/` (`obsbot4linux.bash` → bash-completion's `completions/obsbot4linux`, `_obsbot4linux` → zsh's `site-functions`).
3. **Coupling with track A's description file:** once C-8 has landed, any PR that changes `descriptions/*.json` (track A's `tiny2lite.json`, the U-row removals after CS-1) must run `scripts/regenerate.sh` and commit `packaging/completions/` too, or the completion stale test fails (spec 6.2 wants exactly that for a model PR). The unit tests of C-6, C-7 and C-8 and the C-12 snapshot `status-fake-a.txt` read `descriptions/tiny2lite.json` and pin a few of its labels and bounds (`Exposure time`, `zone.area.pan` −150..150, …); a track-A change to those values updates those expectations in the same PR. Open review point (not changed in the fix round, as it is not a one-line change): test the output format against `descriptions/fake/fake-b.json` or descriptions built with W0's builders instead, so a contract-change PR after CS-1 touches no track-C test.
4. **Action: gate C-12 and C-13 on B-1.** Keep both out of the first issue batch; the build workflow issues them only after track B's B-1 has landed on `rebuild` (a coordinator-side gate, or a second task list released then). The task list holds only this track's keys, so `depends_on` cannot say it. Without B-1 every e2e and parity test fails at start with "not built yet (stage W0)".
5. **Action: record the changed `Client::subscribe` signature** (decision 2: it returns `(HelloOk, Sender, Receiver)`, not spec 6.1's `(tx, rx)`) for the W2 docs task, or as a row of spec 15.1, so that spec 6.1's example and the client part of `docs/protocol-2.md` show `let (hello, tx, rx) = Client::subscribe(…)`.
6. **For track B: test socket paths.** A Unix socket path holds at most 107 bytes and the daemon adds `.lock` and `.test` (5 bytes) to its socket path. Track C's `common::socket` keeps names short and asserts at most 100 bytes, so `makepkg check()` in a deep AUR build directory still binds; track B's tests should do the same.
