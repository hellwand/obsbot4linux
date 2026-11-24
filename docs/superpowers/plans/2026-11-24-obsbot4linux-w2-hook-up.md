# obsbot4linux 1.0, stage W2 (hook-up) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hook the parts of W1 together and make them checkable without a camera (design spec 10.3 W2): the daemon running the Tiny 2 Lite model end to end in a dev build (on a camera recorded on the owner's device, through the replay transport), `scripts/check-model.sh` (spec 9.4) with its CI run against `fake-a` and `fake-b`, `docs/protocol-2.md` completed with worked transcripts and the client library, the migration notes drafted (12.2), and the evidence the recordings already hold towards camera session 1, as fixtures and ledger notes. Two further tasks wait for the camera: the CS-1 results (U-rows, `settle_ms`, `asleep`) and the status-block change of CAM-20.

**Architecture:** No new crate and no contract change. One small glue change in the daemon (a doc-hidden `obsbot4linuxd::run_on(config, timing, bus)` that serves a bus the caller builds) lets a test put track A's `Tiny2Lite` on the replay transport behind the daemon of track B and drive it with the command line of track C. Everything else is new test files, one script, one example and docs. `scripts/check-model.sh` is bash with `jq`, driving the command line only (it never talks to the socket itself); its CI run is a `cargo test` of the CLI crate that starts an in-process fake daemon with short poll tiers and runs the script as a child process, so it runs in CI's `test` and `msrv` jobs without a workflow change.

**Tech Stack:** Rust 2024 edition, MSRV 1.88 (CI toolchain 1.99.1); bash ≥ 4 and jq ≥ 1.6 for the script (both on CI's ubuntu-22.04 runner); `tshark` only for the planner's extraction of the recorded blocks (not needed by any builder: the fixture bytes are inlined below).

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` (revision 2, approved 2026-11-13, with section 15.1 rows 88 to 100), sections 2, 4 (preamble), 6.1, 8.5, 8.8, 9.1 to 9.4, 10.1, 10.3 (W2, W3), 10.4, 11, 12.1, 12.2, 14, 15.1; the W1 plans `docs/superpowers/plans/2026-11-13-obsbot4linux-w1-{a-device,b-daemon,c-client-cli}.md` (their code is what this plan builds on).

## Global Constraints

- "**W2: hook-up** — Content: daemon and `tiny2lite` end to end in a dev build; `scripts/check-model.sh` (9.4); `docs/protocol-2.md` completed with worked transcripts; the migration notes drafted (12.2). Owner files: `scripts/`, `docs/`; glue in crates only by contract-change PRs. Depends on: A, B, C. Acceptance without camera: the check script passes against `fake-a` and `fake-b` in CI; the CLI end-to-end tests pass; descriptions and completions are regenerated and their stale tests pass; no U-row that CS-1 failed is left in `descriptions/tiny2lite.json`. Acceptance with camera: none." (spec 10.3)
- "A request sends exactly what it names: the frames of that setting or action, nothing before them and nothing after them." Clients "never chain a second request to the one the user made." (spec 2) The check script is a test procedure run by a person, not a client: each of its rows is one request the person can see, and it names the setup writes of a `requires` row in its output (decision 6).
- Contract files are not changed by any task without the camera (spec 10.1, this run's rule): `crates/obsbot-proto/src/*` except `client*`, `crates/obsbot-device/src/contract/*`, `schema/protocol-2.json`, the golden transcripts, `descriptions/*.json`. Only the camera-gated task w2-8 changes `descriptions/tiny2lite.json`, as the contract-change PR spec 10.3 track A names ("Results go into the ledger and arrive as contract-change PRs").
- "Fixtures are single exchanges (request frame and answer), inline in a test or in a small text table, each with its capture key. Recording files never enter the repository. Identity answers and descriptors that hold a serial string are never fixtures; tests use synthetic values." (spec 8.5) The protocol repository is read-only reference; nothing is copied from it but the status-block bytes inlined in Task w2-5, and its `firmware/` folder is not quoted.
- Public text (spec 8.5, this run's rule): code comments, tests, docs, issues and pull requests describe this project's behaviour and what was **recorded on the owner's device** (`capture N`) or measured; never that knowledge comes from the vendor's software or firmware; no private names, hosts or paths; no product id of a camera without recordings; serials only as the synthetic `EXAMPLE…`, `FAKEA…`, `FAKEB…`.
- "`scripts/check-model.sh --device <id> [--asleep] [--destructive]` checks one device against its own description … CI runs it against `fake-a` and `fake-b` with `--assume-pass`, which answers every question to the person with pass." (spec 9.4)
- "`docs/protocol-2.md` is the client reference: per-message schemas, a worked sequence with `pending`, lifecycle diagrams and a multi-device transcript. Its normative part is written in stage W0 from this section; stage W2 adds the worked examples." (spec 4) "`Client::subscribe` returns the hello reply, a sender and a receiver" (15.1 row 96; track C note 5 asks W2 to show it).
- "The CHANGELOG of 1.0 carries the migration notes" (spec 12.2): `CHANGELOG.md` is W3's file (spec 10.3 W3 owner files), so W2 drafts them in `docs/migration-1.0.md` and W3 moves them.
- No camera: nothing opens a video node. The replay bus of Task w2-6 reports no video node; the check-script test runs only against the fakes; live tests stay ignored.
- Machine: every cargo command `nice -n 19` with `-j 2`; `CARGO_TARGET_DIR` and `TMPDIR` under `$CACHE/obsbot-build/<task key>` (`$CACHE`: the build machine's cache directory, named in the coordinator's brief; never `/tmp`), removed as the last step. Test sockets under `CARGO_TARGET_TMPDIR` (the CLI crate's `common::socket` asserts ≤ 100 bytes).
- **Owner files and reviewed deviations** (spec 10.3: W2 owns `scripts/` and `docs/`; "glue in crates only by contract-change PRs"). W2 reaches outside them in these places, each a deviation the reviewer of that pull request confirms, none a contract file: w2-1 (daemon glue) and w2-9 (`packaging/arch/PKGBUILD` and `scripts/build-arch.sh`, W3's files, one line each) are glue PRs of their own; w2-2, w2-3, w2-5 and w2-6 add **test and example files under `crates/`** (`check_model.rs`, `watch.rs`, `doc_examples.rs`, `capture_evidence.rs`, `replay.rs`) and w2-6 adds a dev-dependency to `crates/obsbot4linux/Cargo.toml` and one line to `Cargo.lock`; w2-2 adds one line to `.gitignore` and deletes `scripts/check-readback.sh`; w2-7 (gate `camera`) changes track A's model code (`read.rs`, `write.rs`), as spec 14 CAM-20 names. Nothing else is touched.
- Repository facts: `edition = "2024"`, `rust-version = "1.88"`, CI `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `scripts/check-cli-deps.sh`; the `required` gate includes `changelog` (no version change) and `msrv` (Rust 1.88: no let-chains in this plan's code, no newer std API); every new source file starts with `// SPDX-License-Identifier: GPL-3.0-or-later` (a shell script with `# SPDX-…` on its second line); commits on `rebuild` are signed; every PR goes into `rebuild`.

## Review Focus

Things a reviewer of this stage should look at, each pinned by a test or a step in its task:

1. **The script restores what it changed, also when interrupted, and says what it cannot restore.** `a_terminated_run_restores_what_it_wrote` (w2-2) sends SIGTERM mid-run and compares every writable leaf with its value before; `fake_a_passes_every_row_and_is_left_as_it_was` does the same after a full run with `--destructive`. Presets, views and the power-on and return poses are lists and stored poses, not leaves: the script prints them before the first action row, says in every question that a change of them is not restored, works only on a preset slot the snapshot shows free and a view it saved itself, and deletes both again (`the_owners_presets_and_views_are_left_alone`).
2. **A row can fail.** `a_value_that_never_shows_fails_its_row` (w2-2) puts a command line in front of the script that answers one `set` without sending it: that row fails and the script exits 1. Without it the pass assertions could not fail.
3. **Restart and destructive rows stay out unless asked.** `without_destructive_no_restart_row_reaches_the_camera` (w2-2) checks the fake's operation log, not only the script's output.
4. **What reaches the recorded camera is exactly the recorded frame.** w2-6 compares the replay transport's log with the bytes of capture 05 (`set image.mirror=on` → short `14 01 01`) and the move of 15.1 row 88 (one `0x6444`, no tracking frame); the only write of a whole first read is the preset-list rewind of 8.8.
5. **Docs that cannot drift.** w2-3's test parses every transcript line of `docs/protocol-2.md` through the protocol types and checks event-before-reply; the client example in the doc must equal `crates/obsbot-proto/examples/watch.rs`, which clippy compiles. **A gap, stated:** `ci.yml`'s `changes` job counts a change of only `docs/*` or `*.md` as "no code", which skips fmt, clippy and test, so a later pull request that edits only `docs/protocol-2.md` lands green without running `doc_examples`, and the next code pull request goes red for a change it did not make. W3 (owner of `.github/`) makes `docs/protocol-2.md` count as code in `changes` (its task w3-3, Step 4; coordinator note 3); until that has landed, a reviewer of a pull request that touches that file runs `cargo test -p obsbot-proto --test doc_examples` and says so.
6. **Foreign text** (retrospective lens 7a): the script prints device names, labels and values it got from the daemon to the terminal and into a tab-separated file. The results file gets no serial (the short id only, also in a failure message); labels come from the description of the model (our text); values are JSON from `jq -c`, so a tab or a newline in a value is escaped as `\t`/`\n`. JSON leaves DEL and the C1 controls (U+0080 to U+009F) as they are, so every camera-sourced text (preset and view names, the firmware string, state values in the printed state change) also goes through a jq filter that replaces them, and every field of a results line has its control characters replaced before it is written. No value reaches a shell command line except as one quoted argument. The client example of w2-3 prints a device id with `{:?}`.
7. **Every action and every asleep row of an action asks first.** `a_person_who_declines_every_action_sends_none` (w2-2) answers no to every question and compares the fake's operation log: nothing but the sleeps and wakes of the settings' asleep rows reaches the device. A `gimbal.turn` runs asleep only with its stop; the trap stops a turn that is still going.
8. **`seen` and the lag are measured from the reply by rev.** `seen=reply` only when the first `status` after the reply has the same `rev` as the reply and shows the value; the lag runs from the reply. `fake_b_passes_every_row_and_tells_the_reply_from_a_later_read` pins `led.brightness=5` (250 ms lag) to `later` however slow the command line is to start.
9. **An asleep row writes a value the snapshot does not show**, after the `requires` of its leaf were set while awake: `fake_a_passes_every_row…` compares every asleep row with the snapshot and fails on a `conflict`.

---

## What W2 needs from W1 (and what is landing now)

W2 starts after the last W1 round has landed. On `origin/rebuild` (32ea9ab when this plan was last checked) rounds 1 to 7 of W1 are in (up to and with `a-14-remove-carried`, `b-12-tests-asleep-startup`, `c-13-parity-rows` and `b-13-tests-deadline-motion`); `b-14-tests-hotplug-process` and `b-15-real-bus` and issue #250 (terminal text escaping in the CLI) are still to land. This plan is written against their plans (all code below was last run on 32ea9ab plus a b-15 as its plan shapes it, see Proof):

- **b-15 (`b-15-real-bus`)**, the glue point. Its plan (W1-B, task b-15 step 3) moves the lock, the socket, the threads and the shutdown of `run` into `fn serve(config: Config, timing: Timing, bus: Arc<dyn Bus>)` (`Config` by value) and lets `run` choose the bus (`config.fake` → the fake bus; `None` → `obsbot_device::bus::SysfsBus::new()`) and call `serve`. Its test `the_real_path_serves_what_the_sysfs_scan_finds` calls `serve(config, Timing::SPEC, bus)` with `config.fake = None`, so its `serve` starts the test-control socket only for a fake bus. Task w2-1 is written against exactly that shape: `run_on` is `serve` made reachable. The landed b-10 has another one (`serve(&Config, Timing, &Bound)` with the test-control socket inside); w2-1's step 0 re-checks the landed b-15 before the task is issued.
- **b-11 to b-14** add worker features the check script meets: the restart quiet time and the wake read (b-12) for the restart and asleep rows, the turn's stop at connection close (b-13; the CLI always sends `keep:true`, so the script stops its own turns with `gimbal.stop`), identity attempts 2 and 3 (b-14). The script code was run against the daemon of `rebuild` 32ea9ab (b-11 to b-13 in); the builder of w2-2 runs it against the landed daemon, b-14 and b-15 included.
- **c-13** (parity rows) and **#250** (escaping) touch files W2 does not touch; #250 changes only characters that no W2 test prints.
- **a-14** removes the carried 0.13 modules; W2 uses only the 1.0 modules (`models::tiny2lite::{decode, plan, model}`, `transport::replay`, `bus`).

Names this plan uses from W1 (all landed, checked in the tree): `obsbot4linuxd::{Config, Timing, run, run_with}`; `obsbot_device::contract::{Bus, Camera, Found, Frame}`, `obsbot_device::contract::camera::Result`; `obsbot_device::models::tiny2lite::{plan::ENTRY, decode::status}`; `obsbot_device::transport::replay::{Log, Replay}`; `obsbot_device::models::fake::{FakeBus, Op, description}`; the test support module `crates/obsbot-device/tests/support/mod.rs` (`recorded`, `model`, `usb`, `commands`, `payloads`, `short`, `framed`, `block`); the CLI crate's `tests/common::{socket, cli}`; `obsbot_proto::client::{Client, Incoming}`, `obsbot_proto::{Event, Hello, Request, ServerMessage, short_id}`.

## Decisions this plan takes

Engineering choices; each can be overruled in review. The reason follows the dash.

1. **The glue is one doc-hidden function, `obsbot4linuxd::run_on(config, timing, bus)`, in its own PR first** (w2-1) — spec 10.3 W2 allows "glue in crates only by contract-change PRs"; `Config::fake` is typed `Arc<FakeBus>` (W0), so a test cannot hand the daemon another bus without it. It is not a contract file and changes neither `Config` nor `run`. `config.fake` then decides only whether the test-control socket runs. With b-15's planned `serve(config, timing, bus)` the function is a call of it (review correction: the first draft moved the lock into `run_on` and gave `serve` another signature, which would have broken b-15's own test).
2. **"End to end in a dev build" without a camera is the replay hook-up** (w2-6): the command line → the daemon → `Tiny2Lite` behind the send-table check → the replay transport answering with the recorded exchanges of track A's test support. The real camera end to end is CS-1 and CS-2 (w2-8, W4). A sysfs fixture tree would make the real bus open `/dev/videoN` of the machine, so it is not used.
3. **The replay test lives in the CLI crate** and includes `crates/obsbot-device/tests/support/mod.rs` by `#[path]` — the CLI crate already has the daemon and the device crate as dev-dependencies (spec 3 allows them), so the whole chain runs where the user's command does; one copy of the recorded camera. It adds the dev-dependency `hex` (the support module decodes hex), the only `Cargo.lock` change of W2.
4. **The check script drives the command line only** (`obsbot4linux … --json`), never the socket — one contract (spec 2), and the script then checks the CLI's value syntax for every row as a side effect. `$OBSBOT4LINUX` names another command line (tests), `$OBSBOT4LINUX_SOCKET` the daemon, as for any client.
5. **bash + jq.** jq is the one extra dependency; CI's `ubuntu-22.04` runner has jq 1.6 and bash 5.1 (the code is run with jq 1.6, see Proof). The release workflow's Arch job runs `check()` = `cargo test --frozen` in an Arch container that installs only `cargo`, and `release.yml` runs on every pull request that touches `scripts/**`: without jq there the dry run goes red from w2-2 on. So `jq` goes into the container and into the PKGBUILD's `checkdepends` first, in its own glue pull request (w2-9), and w2-2 depends on it. The deb and rpm jobs package the binary of the `binary` job and run no tests.
6. **Setting rows** follow 9.4: every writable enum value, both bool values (the one that differs from the snapshot first, so the snapshot value is the last written; `true` then `false` when the snapshot is `null`), the minimum and the maximum of a number (only the minimum when there is no maximum), the full span `min.min,max.max` of a range. 9.4 says "minimum and maximum of each number and range"; a range is one row here because the Tiny 2 Lite's description gives each range one writable span (its bounds `min.min`, `min.max`, `max.min`, `max.max` leave no narrower value), and it is stated so that a review can ask for a second row. A leaf with `requires` gets its required values set first, each as its own `set` that the output names (`setup for <leaf>: set <path>=<value>`), and restored after its rows. **Each row restores its leaf at once** (9.4) and waits for the restore to show; a restore that fails is a `restore` row with `warn:` and the trap names what is left (decision 9). Rows that restart the camera are the exception: each of their writes restarts it, so they are not restored row by row, the trap restores them once.
7. **"Shows the written value"** = the leaf equals it (numbers within half the precision, per component for objects) and the path is not in `pending`. **`seen` is decided by the rev, the lag is timed from the reply** (9.4: "the time from the reply to that state"). `seen=reply` (lag 0) only when the first `status` after the reply carries the same `rev` as the `set` reply and shows the value: that is the state the reply published. Otherwise `seen=later`, with the time from the reply (the clock starts when the command line returns it, so the request's own round trip is not in it) to the first state that shows the value (a poll every 0.2 s). A tier read between the reply and the `status` bumps the rev, so a lagging leaf cannot read as `reply`; an unrelated change in that gap makes a `reply` look `later` and overstates the lag, which is the safe direction for `settle_ms`. The rounding up to a whole second is the ledger's (w2-8), not the script's.
8. **Action rows**: the parameters are every required parameter, or the first non-bool parameter when none is required (a move needs an axis); the value is the parameter's `default`, else the point three quarters up its bounds on its step or precision (inside the range, not at a limit, not 0 for a symmetric range), the first writable enum value, `true` for a bool, or the text `check`. **A preset slot or a view id is never the owner's**: `preset.save` takes the first slot the snapshot shows free (the bounds of its `slot` parameter minus the slots of the snapshot's `presets`; none, when the snapshot shows no list), the rows after it (`preset.rename`, `.recall`, `.delete`, `power_on.use_preset`) use that slot, `view.save name=check` makes the script's own view (found again by the name `check` and an id the snapshot did not hold), `view.go`, `.rename` and `.delete` use it. A row that needs an item and finds none is `not run: no item of its own`. Actions run in this order: those ending in `.save` first; then group by group (the path before the last dot), a `.stop` after the other actions of its group, a `.delete` last; then `sleep` and `wake`. An action refused with `conflict` or `asleep` is recorded `not run: <code>` and fails nothing (a list slot that is empty on this camera); any other refusal fails the row. **The person confirms each action before it runs** (the question says that what the action stores is not restored) **and its effect after it** — the script cannot know the effect; the ledger's effect column says it (9.4). A `.turn` action is always followed by its `.stop`, also asleep, and the trap stops it. Rows that need a person in view are marked in the output and the results file (column `how`).
9. **Restore**: each setting row restores its leaf at once (9.4); the trap on EXIT, INT and TERM stops a turn that is still going, sets every leaf the script wrote back to its snapshot value, last written first, skipping leaves that already show it and leaves whose snapshot value is `null`, deletes the script's own preset and view, and moves the gimbal back to the snapshot's `gimbal.pan`/`gimbal.tilt` when an action ran and the device has `gimbal.move`. A leaf that does not show its snapshot value afterwards is named (`not restored: <path>`) and makes the exit code 1. The script knows the leaf names `gimbal.pan` and `gimbal.tilt` and the action names `gimbal.move`, `preset.*` and `view.*` from the path dictionary of `docs/protocol-2.md` (one meaning per path on every model), not from a model. **What is not restored**: stored presets and views of the owner are never touched (decision 8); what `power_on.set_pose`, `power_on.use_preset` and `power_on.use_current` store, and `hand.return_clear`, `zone.return_here` and `hand.return_here` change, stay changed; the state before the first action row is printed so that the person can put it back, and every question says so.
10. **Results file**: tab-separated `kind path value result lag_ms seen how`, three `#` header lines (UTC time, the short id, model, firmware as a JSON string, support); `how` is the kind of the row (9.4): `readback`, `no_effect (readback only)`, `readback:false (person in view)` or `action (person in view)`; default `check-model-<model>-<date>.tsv` in the current directory (`.gitignore` names it), `--results <file>` to choose. No full id or serial in it, also not in a failure message (it is pasted into pull requests); control characters of any field are replaced when the line is written.
11. **Waits as the spec says, overridable for tests**: `CHECK_MODEL_WAIT` (70 s), `CHECK_MODEL_SLEEP_WAIT` (30 s), `CHECK_MODEL_RETURN_WAIT` (60 s), `CHECK_MODEL_POLL` (0.2 s), `CHECK_MODEL_FINISH_WAIT` (10 s, the waits of the trap). The CI test runs the daemon with tiers of 100/200/400 ms and waits of 5/5/10 s, so fake-b's 60 s group shows within the test's time. The person's answers are read from `/dev/tty`, or from the file `CHECK_MODEL_TTY` names (one answer per line; the tests use it); the questions go to stderr.
12. **Exit codes of the script**: 0 when no row that ran failed and every written leaf was restored; 1 when a row failed or a leaf is left changed; 2 usage (or no jq, or no answers to read); a failing `describe` or `status` at the start passes the command line's code (3, 4); 3 when the device is not available, or asleep and the person does not want it woken (an asleep device is woken first when the person agrees, as the 0.13 script did).
13. **The transcripts of `docs/protocol-2.md` are recorded, not written**: both from `obsbot4linuxd --fake fake-b,unknown` (the lagging-write transcript from fake-b alone with the unsupported device, so that no event of another fake can enter a subscribed recording), with the recording script given in w2-3. Lines of a `jsonl` block are complete and are checked by a test; the multi-device block elides descriptions and states with `…` and is a `text` block. The daemon's version in the elided `hello` reply is written `1.0.0`.
14. **The client example is a compiled example** (`crates/obsbot-proto/examples/watch.rs`) shown byte-equal in the doc — a doc block nobody compiles drifts; clippy `--all-targets` compiles examples. It sends no write.
15. **The migration notes are a draft in `docs/migration-1.0.md`**, user-facing: the command map without CAM and U references (those are the spec's), the exceptions of 9.2 in plain words, the upgrade steps of 12.2. W3 moves them into the CHANGELOG; W4 (CAM-28) rehearses them.
16. **Capture evidence becomes fixtures plus ledger notes, not description changes**: recorded blocks through the 1.0 decoder in a new test file of the device crate; the findings in a new ledger section. Levels in the description change only with CS-1 (w2-8). Two of the tests pin today's decoding of a block whose meaning CS-1 settles; the PR of that decision changes them.
17. **No coordinator "regenerate" task**: the W2 acceptance "descriptions and completions are regenerated and their stale tests pass" is a check after the last landing (note 5 for the coordinator), and part of w2-8 when CS-1 changes the description.
18. **The CLI tests that pin values of `descriptions/tiny2lite.json`** (track C's note 3: C-6, C-7, C-8 and the C-12 snapshot) are not decoupled in W2: w2-8 updates them in the contract-change PR that changes the description, as track C's plan already says.
19. **Asleep rows** (`--asleep`, spec 9.4): for a leaf, the first row value that differs from the snapshot (a number that has both bounds also tries the point three quarters up; without any differing value the row is `not run`, so that `accepted` always means the camera took a new value); the `requires` of the leaf are set while awake, before the sleep; for an action, the question comes first, then the item it needs is saved (awake), then `do sleep`, the action (with its stop, for a turn), `do wake`. A `requires` refusal can no longer read as `ignored (reply conflict)`.
20. **Two glue pull requests and the plan itself land first** (w2-0, w2-9, w2-1): the plan document lands on `rebuild` as a docs-only pull request before any builder starts (the tasks name it as `plan_doc`; its text holds no private path or name); `jq` for the package checks (w2-9) before w2-2, because `release.yml` runs on w2-2's own pull request; `run_on` (w2-1) before w2-6.
21. **How long a check run takes** is estimated, not measured, and the estimate is for planning CS-1 (spec 10.4): the Tiny 2 Lite's description gives 145 setting rows (59 leaves) and 24 action rows besides `sleep` and `wake`, and with `--asleep` 58 leaf rows and 18 action rows. A setting row is a `set`, a `status` and a restore: a few seconds when the touched re-read shows the value (`settle_ms` is 3000), so about 10 to 15 minutes; an asleep row costs the sleep (the first status after the sleep command showed asleep after 5.2 s in capture 13), the write and the wake (3.3 s), about 10 to 20 s, so about 15 to 25 minutes; the action rows are the person's pace. The waits give a bound that nobody should plan on: 70 s per row and per restore, and 30 + 30 + 70 s per asleep row, hours.

## Product questions (recommendations; nothing built on them)

All of them were decided as recommended in the cross-check of 2026-11-24, for the owner to review later; the build order follows those answers.

- **P1. Take CAM-20's model change before CS-1?** All five enumerations recorded on the owner's device show all-zero status blocks after plug-in and after each restart (Task w2-5), and today the model decodes such a block into values (`sleep.auto.seconds` 0, `image.frame_rate` 0) that the daemon would publish for the first seconds and that an auto-sleep write would build on (the auto-sleep word is taken from the last status read, 8.8). Recommendation: keep w2-7 gated on CS-1 as spec 14 says, but run CAM-20 first in CS-1; if the owner prefers, the change of w2-7 is safe on captured evidence alone (it turns a zero block into "no read", which only delays the first values by one status read). Built today: nothing; the gate is `camera`.
- **P2. Should the check script's action rows take parameters from a file** (`--params <file>`, action → parameters) so that W4 can run each 9.1 row with the values the owner wants (a gentle turn speed, a pose)? The preset slot and the view are already the script's own (decision 8). Recommendation: not in W2; the person can decline each action and run it by hand; add it in W4 if the session shows the need. Built today: no.

---

## Camera: what the recordings stand in for, and what stays for CS-1

The protocol repository holds USB recordings made on the owner's Tiny 2 Lite. Where they show a camera behaviour, W2 turns them into fixtures (w2-5, w2-6) and ledger notes. **None of this replaces CS-1**: every item below still runs in the camera session, from Linux, as spec 14 lists it.

| Item         | What the recordings show (task)                                                                                                                                                                                                      | Covers in part                                                       | Still for CS-1                                                       |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| CAM-1        | the serial query `0x18c8` answered with 14 characters within 75 ms each time it was sent: at plug-in (captures 01, 16), after both microphone restarts (14), after the factory reset (16), 2.7–6.6 s after enumeration (w2-5 ledger) | "after plug-in, after the microphone restart, after a factory reset" | the query from Linux; asleep; right after plug-in; the time it takes |
| CAM-3        | awake at plug-in; the first status read after the sleep command shows asleep (5.2 s), the first after the wake awake (3.3 s), upper bounds (capture 13, w2-5 test)                                                                   | "is the camera awake at plug-in"; an upper bound for wake            | the time from wake to awake from Linux; `zoom.factor` asleep         |
| CAM-8 / U-23 | `status 0x22` reads `0x10` with the microphone on, `0x00` off, changing with the command before the restart (capture 14, w2-5 test)                                                                                                  | "does a status-block value follow the microphone"                    | a cold plug with the microphone off; the audio interface then        |
| CAM-20       | a status read 0.8–4.5 s after every recorded enumeration returns all-zero blocks; the next read (about 4 s later) is full (captures 01, 14, 16, w2-5 test)                                                                           | "does the first field read 0 until it is ready"                      | the same from Linux; then w2-7                                       |
| CAM-25       | the USB descriptor of capture 01 (ids, bcdDevice, interface classes, extension unit 2 and its GUID) is the `usb` of the replay hook-up (w2-6)                                                                                        | the descriptor facts, not sysfs                                      | the sysfs attributes while plugged                                   |
| CAM-24, 9.1  | every set and act of the replay hook-up reaches the recorded camera as its recorded frame (w2-6); the check script's rows against `fake-a` (w2-2)                                                                                    | rehearsal only                                                       | W4 (CS-2)                                                            |

Not covered by any recording: CAM-2, CAM-4 to CAM-7, CAM-10, CAM-13 to CAM-19, CAM-22, CAM-23 (the U-rows), CAM-33 to CAM-35. The recordings show the sequence a host program sent, never the camera's answer to a write that program did not make.

## File structure

| File                                                                                    | Responsibility                                                       | Task             |
| --------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------------- |
| `docs/superpowers/plans/2026-11-24-obsbot4linux-w2-hook-up.md` (new)                    | this plan, on `rebuild`                                              | w2-0             |
| `crates/obsbot4linuxd/src/lib.rs` (modify)                                              | `run_on`, doc-hidden, and its test                                   | w2-1             |
| `crates/obsbot4linuxd/src/daemon.rs` (modify, only if b-15 left it so)                  | `serve` visible in the crate; test-control socket only in fake mode  | w2-1             |
| `scripts/check-model.sh` (new)                                                          | the check of spec 9.4                                                | w2-2             |
| `crates/obsbot4linux/tests/check_model.rs` (new)                                        | the script against `fake-a` and `fake-b` in CI                       | w2-2             |
| `scripts/check-readback.sh` (delete), `.gitignore` (one line)                           | 0.13's script, replaced (9.4); the default results file              | w2-2             |
| `packaging/arch/PKGBUILD`, `scripts/build-arch.sh` (one line each)                      | `jq` for the tests the Arch package build runs                       | w2-9             |
| `docs/protocol-2.md` (modify)                                                           | sections 11 (worked transcripts, lifecycles) and 12 (client library) | w2-3             |
| `crates/obsbot-proto/examples/watch.rs` (new)                                           | the client example the doc shows                                     | w2-3             |
| `crates/obsbot-proto/tests/doc_examples.rs` (new)                                       | the doc's transcripts and example stay true                          | w2-3             |
| `docs/migration-1.0.md` (new)                                                           | the migration notes, drafted                                         | w2-4             |
| `crates/obsbot-device/tests/capture_evidence.rs` (new)                                  | recorded status blocks through the decoder                           | w2-5, w2-7       |
| `docs/field-ledger.md` (modify)                                                         | section "Recorded evidence towards camera session 1"                 | w2-5, w2-7, w2-8 |
| `crates/obsbot4linux/tests/replay.rs` (new)                                             | command line → daemon → `tiny2lite` → recorded camera                | w2-6             |
| `crates/obsbot4linux/Cargo.toml`, `Cargo.lock`                                          | dev-dependency `hex`                                                 | w2-6             |
| `crates/obsbot-device/src/models/tiny2lite/read.rs`                                     | a zero status block is no read (CAM-20)                              | w2-7 (camera)    |
| `crates/obsbot-device/src/models/tiny2lite/write.rs`                                    | `sleep_word` reads the block through the same check                  | w2-7 (camera)    |
| `descriptions/tiny2lite.json`, model description, ledger, completions, pinned CLI tests | CS-1 results                                                         | w2-8 (camera)    |

## Tasks, waves, dependencies and gates

A task branches from `rebuild` once its dependencies and its gate are met. Tasks of one wave touch disjoint files; `Cargo.lock` changes only in w2-6. Gate `w1:<key>`: the W1 task with that key has landed on `rebuild` (`b-15-real-bus` is the last W1 task, so this gate means "W1 complete"); `camera`: needs camera session CS-1 with the owner present; `null`: none. w2-0 is a docs-only pull request that puts this document on `rebuild` (every task names it as its `plan_doc`); it comes first and needs no builder beyond a copy and the check of its first step.

| Task | Key                          | Title                                                                | Wave | Depends on       | Gate                     | Effort |
| ---- | ---------------------------- | -------------------------------------------------------------------- | ---- | ---------------- | ------------------------ | ------ |
| w2-0 | `w2-0-plan-doc`              | This plan on `rebuild` (docs-only pull request)                      | 0    | –                | null                     | S      |
| w2-9 | `w2-9-jq-for-package-checks` | `jq` in the Arch container and `checkdepends` (glue PR)              | 1    | w2-0             | null                     | S      |
| w2-1 | `w2-1-daemon-bus-entry`      | Daemon: `run_on`, a bus the caller builds (glue PR)                  | 1    | w2-0             | `w1:b-15-real-bus`       | S      |
| w2-2 | `w2-2-check-model-script`    | `scripts/check-model.sh` and its run against the fakes               | 1    | w2-0, w2-9       | `w1:b-15-real-bus`       | L      |
| w2-3 | `w2-3-protocol-doc`          | `docs/protocol-2.md`: worked transcripts, lifecycles, client library | 1    | w2-0             | `w1:b-15-real-bus`       | M      |
| w2-4 | `w2-4-migration-notes`       | Migration notes drafted                                              | 1    | w2-0             | null                     | S      |
| w2-5 | `w2-5-capture-evidence`      | Recorded status blocks as fixtures; ledger notes towards CS-1        | 1    | w2-0             | `w1:a-14-remove-carried` | M      |
| w2-6 | `w2-6-replay-hookup`         | Command line, daemon and `tiny2lite` end to end on a recorded camera | 2    | w2-0, w2-1       | null (via w2-1)          | M      |
| w2-7 | `w2-7-boot-block`            | A status block that reads 0 is no read (CAM-20)                      | 3    | w2-5             | `camera`                 | S      |
| w2-8 | `w2-8-cs1-results`           | CS-1 results into ledger, description and completions                | 4    | w2-4, w2-5, w2-7 | `camera`                 | L      |

w2-6 is wave 2 because it needs `run_on`; it is not stacked on w2-1 (one small PR, landed first). w2-8 comes after w2-7 because both edit `docs/field-ledger.md` and `capture_evidence.rs` may need w2-7's form. w2-9 and w2-2 are not stacked either: w2-9 lands first, and the pull request of w2-2 is then the first whose `release.yml` dry run needs `jq` in the Arch container. Every task's code (except w2-8, whose content is the camera's answer) was built and run in a scratch copy of `rebuild` (see "Proof").

---

### Task w2-0 (key `w2-0-plan-doc`): This plan on `rebuild`

A docs-only pull request, before any builder starts: the tasks name this document as their `plan_doc`, and a document that is not in the repository cannot be a builder's contract. The W1 plans were landed the same way.

**Files:**

- Create: `docs/superpowers/plans/2026-11-24-obsbot4linux-w2-hook-up.md` (this document, as the coordinator holds it)

- [ ] **Step 1: Check the text.** `grep -nEi '/hom[e]/|thom[a]s' docs/superpowers/plans/2026-11-24-obsbot4linux-w2-hook-up.md` prints nothing (public-text rule: no private path or name; the owner is "the owner"), and `grep -n 'sn:[A-Z0-9]\{8,\}' …` finds only the synthetic `EXAMPLE…`, `FAKEA…` and `FAKEB…` serials.
- [ ] **Step 2: Format.** `prettier --check docs/superpowers/plans/2026-11-24-obsbot4linux-w2-hook-up.md` if installed (the file is a plan like the W1 ones; `prettier --write` aligns the tables).
- [ ] **Step 3: Commit.** `git add docs/superpowers/plans/2026-11-24-obsbot4linux-w2-hook-up.md && git commit -m "docs: plan for stage W2 (hook-up)"`. The pull request changes docs only: the `required` gate reports without running fmt, clippy and test.

---

### Task w2-9 (key `w2-9-jq-for-package-checks`): `jq` in the Arch container and in `checkdepends`

A glue pull request into W3's files (`packaging/`, `scripts/build-arch.sh`), one line each, before w2-2. Why: `release.yml` runs on every pull request that touches `scripts/**`, `Cargo.toml` or `Cargo.lock`, so it runs on the pull requests of w2-2 and w2-6; its `arch` job runs `scripts/build-arch.sh`, which runs makepkg, whose `check()` is `cargo test --frozen`, in an Arch container that installs only `cargo`. From w2-2 on that test run includes `check_model.rs`, which needs `jq` (the script says so and exits 2): the dry run of every release would go red, and W3's release candidate could not be built before packaging changed.

**Files:**

- Modify: `packaging/arch/PKGBUILD` (after `makedepends`: a comment and `checkdepends=('jq')`)
- Modify: `scripts/build-arch.sh` (the `pacman` line installs `jq` with `cargo`; the comment above it says why)

- [ ] **Step 1: Change them.**

```diff
--- a/packaging/arch/PKGBUILD
+++ b/packaging/arch/PKGBUILD
@@ makedepends=('cargo')
+# The check() below runs the workspace tests, among them scripts/check-model.sh, which needs jq.
+checkdepends=('jq')
 options=('!debug')
--- a/scripts/build-arch.sh
+++ b/scripts/build-arch.sh
@@
-# Arch does not support partial upgrades, so installing cargo needs -Syu. That pulls the
+# Arch does not support partial upgrades, so installing cargo (and jq, which the tests of
+# check() need: scripts/check-model.sh) needs -Syu. That pulls the
@@
-  pacman -Syu --noconfirm --needed --disable-sandbox cargo
+  pacman -Syu --noconfirm --needed --disable-sandbox cargo jq
```

- [ ] **Step 2: Check.** `shellcheck scripts/build-arch.sh` (no output); `cd packaging/arch && makepkg --printsrcinfo | grep checkdepends` prints `checkdepends = jq`; `git status --short` shows only these two files.
- [ ] **Step 3: Commit and CI.** `git add packaging/arch/PKGBUILD scripts/build-arch.sh && git commit -m "packaging: jq for the tests that makepkg's check() runs (check-model.sh)"`. The pull request runs `release.yml` as a dry run (it touches `packaging/**` and `scripts/**`): the Arch jobs must stay green. W3's tasks that edit the same two files (w3-4, w3-7) rebase on this one.

---

### Task w2-1 (key `w2-1-daemon-bus-entry`): Daemon: `run_on`, a bus the caller builds

The glue PR of spec 10.3 W2 ("glue in crates only by contract-change PRs"): its own small PR, landed before w2-6. It is written against b-15 **as its plan shapes it** (W1-B plan, task b-15, step 3): `fn serve(config: Config, timing: Timing, bus: Arc<dyn Bus>)` holds the lock, the socket, the threads and the shutdown, `run` chooses the bus and calls it, and b-15's test `the_real_path_serves_what_the_sysfs_scan_finds` calls `serve(config, Timing::SPEC, bus)` with `config.fake = None`. Under that shape `run_on` is `serve` made reachable from outside the crate; the first draft of this task moved the lock into `run_on` and gave `serve` another signature, which would have broken b-15's test while the acceptance claimed it unchanged.

**Files:**

- Modify: `crates/obsbot4linuxd/src/lib.rs` (one doc-hidden function after `run_with`; one unit test)
- Modify (only if b-15 left it so): `crates/obsbot4linuxd/src/daemon.rs` (`serve` visible inside the crate: `pub(crate) fn serve`)

**Interfaces:**

- Consumes (b-15 as planned): `daemon::serve(config: Config, timing: Timing, bus: Arc<dyn Bus>) -> io::Result<()>`, which starts the test-control socket (`control::serve`) only when `config.fake` is `Some` (b-15's own test needs that: it passes `fake = None`), and removes only a socket file it bound.
- Produces: `#[doc(hidden)] pub fn obsbot4linuxd::run_on(config: Config, timing: Timing, bus: Arc<dyn obsbot_device::contract::Bus>) -> std::io::Result<()>` — the daemon of `run_with` on the given bus; the test-control socket only when `config.fake` is `Some`.

- [ ] **Step 0: Check the landed b-15 against those three lines** (the coordinator or the builder, before the first edit): `grep -n 'fn serve\|fn run\|control::serve' crates/obsbot4linuxd/src/daemon.rs`. The plan below applies as it stands when `serve` takes `(Config, Timing, Arc<dyn Bus>)` and starts `control::serve` only for `Some(fake)`. If the landed shape differs, make the smallest change that gives these three properties and list it in the PR: `run` behaves as before; `run_on` runs the lock, the socket, the threads and the shutdown of `run` on the given bus; the test-control socket exists only when `config.fake` is `Some`. If b-15's `serve` still starts the test-control socket unconditionally (the b-10 shape), that is the one change to make in `daemon.rs`: `let control = match &config.fake { Some(fake) => Some(control::serve(&config.socket, fake.clone(), control_stop.clone())?), None => None };` and the two `stop_control(…)` calls (the error path of `start_threads` and the end of the shutdown) run only `if let Some(control) = control`.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module of `crates/obsbot4linuxd/src/lib.rs`, after `run_uses_the_intervals_of_the_spec`:

```rust
    /// W2: `run_on` serves the bus it is given, and without `config.fake` it serves no
    /// test-control socket. The bus here is a fake bus with one `fake-b`; `config.fake` stays
    /// `None`, as for a model on the replay transport.
    #[test]
    fn run_on_serves_the_given_bus_without_a_test_control_socket() {
        use std::io::{BufRead, Write};
        use std::os::unix::net::UnixStream;
        use std::sync::atomic::Ordering;
        use std::time::Instant;
        let dir = std::env::temp_dir().join(format!("o4ld-on-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("s.sock");
        let config = Config::new(socket.clone());
        let stop = config.stop.clone();
        let bus: Arc<dyn obsbot_device::contract::Bus> =
            Arc::new(FakeBus::new(&["fake-b"]).unwrap());
        let h = std::thread::spawn(move || run_on(config, Timing::SPEC, bus));
        let until = Instant::now() + Duration::from_secs(5);
        while UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < until, "the daemon does not accept");
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut s = UnixStream::connect(&socket).unwrap();
        writeln!(s, r#"{{"id":1,"hello":{{"protocol":2,"client":"t 1"}}}}"#).unwrap();
        let mut line = String::new();
        std::io::BufReader::new(s).read_line(&mut line).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["ok"]["devices"][0]["model"], "fake-b", "{line}");
        assert!(
            !control::control_path(&socket).exists(),
            "no test-control socket without config.fake"
        );
        stop.store(true, Ordering::SeqCst);
        h.join().unwrap().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `nice -n 19 cargo test -p obsbot4linuxd --lib -j 2 tests::run_on`
Expected: FAIL to compile: `cannot find function run_on in this scope`.

- [ ] **Step 3: Write the code**

Add to `crates/obsbot4linuxd/src/lib.rs`, after `run_with`:

```rust
/// [`run_with`] on a bus the caller builds, instead of the one `config` names: a model on the
/// replay transport, so that a test runs the daemon and a real model end to end without a
/// camera (stage W2). `config.fake` only decides whether the test-control socket is served.
/// For in-process tests only.
#[doc(hidden)]
pub fn run_on(
    config: Config,
    timing: Timing,
    bus: Arc<dyn obsbot_device::contract::Bus>,
) -> std::io::Result<()> {
    daemon::serve(config, timing, bus)
}
```

and in `crates/obsbot4linuxd/src/daemon.rs` make `serve` reachable from `lib.rs` when b-15 left it private:

```diff
-fn serve(config: Config, timing: Timing, bus: Arc<dyn Bus>) -> io::Result<()> {
+pub(crate) fn serve(config: Config, timing: Timing, bus: Arc<dyn Bus>) -> io::Result<()> {
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linuxd -j 2`
Expected: PASS, every test of the crate, `run_on_serves_the_given_bus_without_a_test_control_socket` included; b-15's `the_real_path_serves_what_the_sysfs_scan_finds` is unchanged and passes (`run_on` adds a caller of `serve` and leaves its signature alone).
Then the workspace gate: `nice -n 19 cargo fmt --all --check`, `nice -n 19 cargo clippy -j 2 --workspace --all-targets --locked -- -D warnings`, `nice -n 19 cargo test -j 2 --workspace --locked`.

- [ ] **Step 5: Commit**

```bash
git add crates/obsbot4linuxd/src/lib.rs crates/obsbot4linuxd/src/daemon.rs
git commit -m "obsbot4linuxd: run_on, the daemon on a bus the caller builds (W2 glue)"
```

---

### Task w2-2 (key `w2-2-check-model-script`): `scripts/check-model.sh` and its run against the fakes

**Files:**

- Create: `scripts/check-model.sh` (mode 755)
- Create: `crates/obsbot4linux/tests/check_model.rs`
- Delete: `scripts/check-readback.sh` (spec 9.4: the new script replaces it; it drives `--direct`, which 1.0 removed; only historic plan text names it)
- Modify: `.gitignore` (one line, `check-model-*.tsv`: the default results file lands in the current directory, which may be the repository)

**Depends on:** w2-9 (`jq` in the Arch container): `release.yml` runs on this task's own pull request.

**Interfaces:**

- Consumes: the command line (`describe`, `status`, `get`, `set`, `do`, `devices`, each with `--json --device`), `$OBSBOT4LINUX_SOCKET`; in the test `obsbot4linuxd::{Config, Timing, run_with}`, `FakeBus::{new, ops}`, `Op`, `fake::description`, `common::{socket, cli}`, `nix::sys::signal`.
- Produces: `scripts/check-model.sh --device <id> [--asleep] [--destructive] [--assume-pass] [--results <file>]` (exit 0, 1, 2, or the command line's 3/4; 3 also for a device that is not available or stays asleep); environment `OBSBOT4LINUX`, `CHECK_MODEL_WAIT`, `CHECK_MODEL_SLEEP_WAIT`, `CHECK_MODEL_RETURN_WAIT`, `CHECK_MODEL_POLL`, `CHECK_MODEL_FINISH_WAIT`, `CHECK_MODEL_TTY` (the person's answers from a file); the results file format of decision 10.

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot4linux/tests/check_model.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! `scripts/check-model.sh` (design spec 9.4) against `fake-a` and `fake-b` with
//! `--assume-pass`, as stage W2's acceptance asks ("the check script passes against fake-a and
//! fake-b in CI"). The daemon runs in-process in fake mode with short poll tiers, so that a
//! lagging readback shows within the test's time; the script runs as it would on a camera,
//! with the command line of this crate. The person's answers of a run without `--assume-pass`
//! come from a file (`CHECK_MODEL_TTY`). Needs bash and jq.

mod common;

use obsbot_device::models::fake::{FakeBus, Op};
use obsbot4linuxd::{Config, Timing};
use serde_json::Value;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const A: &str = "sn:FAKEA0000001";
const B: &str = "sn:FAKEB0000001";

/// Poll tiers short enough that a value of fake-b's 60 s group shows within a second; the
/// deadline is the spec's.
const FAST: Timing = Timing {
    scan: Duration::from_millis(50),
    request_deadline: obsbot_proto::REQUEST_DEADLINE,
    second: Duration::from_millis(100),
    five_seconds: Duration::from_millis(200),
    minute: Duration::from_millis(400),
};

struct Daemon {
    socket: PathBuf,
    bus: Arc<FakeBus>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<std::io::Result<()>>>,
}

impl Daemon {
    fn start(name: &str) -> Daemon {
        let socket = common::socket(&format!("m-{name}"));
        let _ = std::fs::remove_file(&socket);
        let bus = Arc::new(FakeBus::new(&["fake-a", "fake-b", "unknown"]).expect("the fakes"));
        let mut config = Config::new(socket.clone());
        config.fake = Some(Arc::clone(&bus));
        let stop = Arc::clone(&config.stop);
        let thread = thread::spawn(move || obsbot4linuxd::run_with(config, FAST));
        let until = Instant::now() + Duration::from_secs(20);
        while UnixStream::connect(&socket).is_err() {
            assert!(!thread.is_finished(), "the fake daemon ended");
            assert!(Instant::now() < until, "the fake daemon does not accept");
            thread::sleep(Duration::from_millis(20));
        }
        Daemon {
            socket,
            bus,
            stop,
            thread: Some(thread),
        }
    }

    /// The full state of one device, as `status --json` prints it.
    fn state(&self, id: &str) -> Value {
        let (code, out, err) =
            common::cli(Some(&self.socket), &["status", "--json", "--device", id]);
        assert_eq!(code, 0, "{err}");
        serde_json::from_str::<Value>(&out).unwrap()["state"].clone()
    }

    /// The script with the given arguments; the results file is returned with the output.
    fn command(&self, results: &Path, cli: &Path, args: &[&str]) -> Command {
        let mut c = Command::new("bash");
        c.arg(script())
            .args(args)
            .arg("--results")
            .arg(results)
            .env("OBSBOT4LINUX", cli)
            .env("OBSBOT4LINUX_SOCKET", &self.socket)
            .env("CHECK_MODEL_WAIT", "5")
            .env("CHECK_MODEL_SLEEP_WAIT", "5")
            .env("CHECK_MODEL_RETURN_WAIT", "10")
            .env("CHECK_MODEL_POLL", "0.05")
            .stdin(Stdio::null());
        c
    }

    fn check(&self, name: &str, args: &[&str]) -> (Output, Vec<Row>) {
        let results = results_file(name);
        let out = self
            .command(
                &results,
                Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
                args,
            )
            .output()
            .expect("bash runs");
        (out, rows(&results))
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        // the daemon keeps its `<socket>.lock` (5.9); the test's daemon is gone, so is it
        let mut lock = self.socket.clone().into_os_string();
        lock.push(".lock");
        let _ = std::fs::remove_file(lock);
    }
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/check-model.sh")
}

fn results_file(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("check-model-{name}.tsv"));
    let _ = std::fs::remove_file(&path);
    path
}

/// One line of the results file: kind, path, value, result.
#[derive(Debug, Clone)]
struct Row {
    kind: String,
    path: String,
    value: String,
    result: String,
    seen: String,
    how: String,
}

/// The rows written so far. The script may be in the middle of a line (the SIGTERM test reads
/// while it runs): only lines that end in a newline and hold every column are rows.
fn rows(results: &Path) -> Vec<Row> {
    let text = std::fs::read_to_string(results).unwrap_or_default();
    let complete = &text[..text.rfind('\n').map_or(0, |i| i + 1)];
    complete
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() == 7).then(|| Row {
                kind: f[0].into(),
                path: f[1].into(),
                value: f[2].into(),
                result: f[3].into(),
                seen: f[5].into(),
                how: f[6].into(),
            })
        })
        .collect()
}

fn row<'a>(rows: &'a [Row], kind: &str, path: &str, value: &str) -> &'a Row {
    rows.iter()
        .find(|r| r.kind == kind && r.path == path && r.value == value)
        .unwrap_or_else(|| panic!("no row {kind} {path} {value} in {rows:#?}"))
}

fn report(out: &Output) -> String {
    format!(
        "exit {:?}\n--- stdout\n{}\n--- stderr\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// The writable leaves of a device that read back, with their values in `state`.
fn writable_values(state: &Value, model: &str) -> Vec<(String, Value)> {
    let d = obsbot_device::models::fake::description(model).unwrap();
    d.settings
        .iter()
        .filter(|(_, s)| !s.readonly && s.readback)
        .map(|(p, _)| {
            let v = p.split('.').fold(state, |v, k| &v[k]).clone();
            (p.clone(), v)
        })
        .collect()
}

/// Whether two JSON values are the same, numbers compared as numbers (the script prints `5`
/// where a state may hold `5.0`).
fn same_json(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() < 1e-9,
        _ => a == b,
    }
}

/// The preset slots and view ids of a device, with their names, as the state shows them.
fn lists(state: &Value) -> (Value, Value) {
    (state["presets"].clone(), state["views"].clone())
}

/// The `Act` operations of a device since `from`, as (action, parameters).
fn acts(d: &Daemon, from: usize) -> Vec<(String, serde_json::Map<String, Value>)> {
    d.bus
        .ops(A)
        .into_iter()
        .skip(from)
        .filter_map(|op| match op {
            Op::Act(name, params) => Some((name, params)),
            _ => None,
        })
        .collect()
}

/// fake-a, the Tiny 2 Lite's shape: every row passes, the `requires` rows included, the
/// restart rows run last and pass when the device is back under its id, and afterwards every
/// leaf the script wrote has its value from before the run.
#[test]
fn fake_a_passes_every_row_and_is_left_as_it_was() {
    let d = Daemon::start("a");
    let before = writable_values(&d.state(A), "fake-a");
    let (out, rows) = d.check(
        "a",
        &[
            "--device",
            "fake-a",
            "--assume-pass",
            "--asleep",
            "--destructive",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    let failed: Vec<&Row> = rows
        .iter()
        .filter(|r| r.result.starts_with("fail"))
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
    // every writable enum value, both bools, number bounds, range span (9.4)
    for value in ["\"auto\"", "\"manual\""] {
        assert_eq!(row(&rows, "setting", "exposure.mode", value).result, "pass");
    }
    assert_eq!(row(&rows, "setting", "exposure.time", "100").result, "pass");
    assert_eq!(
        row(&rows, "setting", "exposure.time", "250000").result,
        "pass"
    );
    assert_eq!(
        row(
            &rows,
            "setting",
            "zone.area.pan",
            "{\"min\":-150,\"max\":150}"
        )
        .result,
        "pass"
    );
    assert_eq!(
        row(&rows, "setting", "led.blink", "true").result,
        "pass (confirmed)"
    );
    // the restart rows (spec 9.4: only with --destructive, last)
    let restart = row(&rows, "setting", "audio.enabled", "false");
    assert_eq!(
        (restart.result.as_str(), restart.seen.as_str()),
        ("pass", "returned")
    );
    let reset = rows.iter().find(|r| r.path == "factory_reset").unwrap();
    assert_eq!(
        (reset.result.as_str(), reset.seen.as_str()),
        ("pass", "returned")
    );
    let last_two: Vec<&str> = rows.iter().rev().take(3).map(|r| r.path.as_str()).collect();
    assert_eq!(
        last_two,
        ["factory_reset", "audio.enabled", "audio.enabled"]
    );
    // asleep rows only for `asleep: unknown`; zoom.field_of_view is `ignored` (measured)
    assert!(
        rows.iter()
            .any(|r| r.kind == "asleep" && r.path == "audio.volume")
    );
    assert!(
        !rows
            .iter()
            .any(|r| r.kind == "asleep" && r.path == "zoom.field_of_view")
    );
    // an asleep row writes a value that differs from the snapshot (otherwise `accepted` proves
    // nothing), and a leaf with `requires` gets its setup while awake: no row is answered
    // with a refusal of the setup (`conflict`)
    for r in rows.iter().filter(|r| r.kind == "asleep") {
        let was = before.iter().find(|(p, _)| *p == r.path).map(|(_, v)| v);
        let written = serde_json::from_str::<Value>(&r.value).ok();
        if let (Some(was), Some(written)) = (was, written) {
            assert!(
                !same_json(was, &written),
                "asleep {} writes the value it already shows ({was})",
                r.path
            );
        }
        assert!(!r.result.contains("conflict"), "{r:?}");
    }
    assert!(
        rows.iter()
            .any(|r| r.kind == "asleep" && r.path == "exposure.time" && r.result == "accepted"),
        "exposure.time asleep, after its setup: {rows:#?}"
    );
    // every row says which kind it is (spec 9.4)
    assert!(rows.iter().all(|r| !r.how.is_empty()), "{rows:#?}");
    assert_eq!(
        row(&rows, "setting", "led.blink", "true").how,
        "readback:false (person in view)"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("not restored by this script"),
        "an action's question says what the script does not restore"
    );
    // a turn is always followed by a stop, awake and asleep
    let acts = acts(&d, 0);
    let turns = acts.iter().filter(|(a, _)| a == "gimbal.turn").count();
    let stops = acts.iter().filter(|(a, _)| a == "gimbal.stop").count();
    assert!(turns >= 2 && stops > turns, "{turns} turns, {stops} stops");
    // left as it was
    let after = writable_values(&d.state(A), "fake-a");
    let changed: Vec<_> = before
        .iter()
        .zip(&after)
        .filter(|(b, a)| b.1 != a.1)
        .map(|(b, a)| (b.0.clone(), b.1.clone(), a.1.clone()))
        .collect();
    assert!(changed.is_empty(), "not restored: {changed:#?}");
}

/// fake-b, of another shape: its own leaves and values, a lagging readback seen at a later
/// read than the reply's, and its 60 s group.
#[test]
fn fake_b_passes_every_row_and_tells_the_reply_from_a_later_read() {
    let d = Daemon::start("b");
    let before = writable_values(&d.state(B), "fake-b");
    let (out, rows) = d.check("b", &["--device", "fake-b", "--assume-pass", "--asleep"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert!(
        !rows.iter().any(|r| r.result.starts_with("fail")),
        "{rows:#?}"
    );
    for value in ["\"red\"", "\"green\"", "\"blue\""] {
        let r = row(&rows, "setting", "beacon.color", value);
        assert_eq!(r.result, "pass");
    }
    // `seen` follows the rev: the state of the reply's own rev did not show the lagging
    // brightness (250 ms) however slow the command line was to start
    assert_eq!(
        row(&rows, "setting", "led.brightness", "5").seen,
        "later",
        "fake-b's lagging readback shows after the reply: {rows:#?}"
    );
    assert!(
        rows.iter()
            .any(|r| r.kind == "setting" && r.seen == "reply")
    );
    assert!(
        !rows.iter().any(|r| r.path.starts_with("gimbal")),
        "fake-b has no gimbal"
    );
    let after = writable_values(&d.state(B), "fake-b");
    assert_eq!(before, after, "fake-b is left as it was");
}

/// Without `--destructive` the rows that restart the camera are listed and nothing of them
/// reaches the fake.
#[test]
fn without_destructive_no_restart_row_reaches_the_camera() {
    let d = Daemon::start("nd");
    let (out, rows) = d.check("nd", &["--device", "fake-a", "--assume-pass"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert_eq!(
        row(&rows, "setting", "audio.enabled", "-").result,
        "not run (--destructive)"
    );
    assert_eq!(
        row(&rows, "action", "factory_reset", "-").result,
        "not run (--destructive)"
    );
    let ops = d.bus.ops(A);
    assert!(
        !ops.iter().any(|op| matches!(op,
            Op::Apply(p, _) if p == "audio.enabled")
            || matches!(op, Op::Act(a, _) if a == "factory_reset")),
        "{ops:?}"
    );
}

/// A row fails when the state never shows the written value: here a command line that
/// answers `set led.brightness=…` without sending it. The script exits 1.
#[test]
fn a_value_that_never_shows_fails_its_row() {
    let d = Daemon::start("f");
    let wrapper = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("check-model-drop-led.sh");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\ncase \"$*\" in *led.brightness=*) echo '{{\"rev\":1}}'; exit 0 ;; esac\nexec '{}' \"$@\"\n",
            env!("CARGO_BIN_EXE_obsbot4linux")
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    let results = results_file("f");
    let out = d
        .command(&results, &wrapper, &["--device", "fake-b", "--assume-pass"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", report(&out));
    let rows = rows(&results);
    assert_eq!(row(&rows, "setting", "led.brightness", "0").result, "pass");
    assert!(
        row(&rows, "setting", "led.brightness", "5")
            .result
            .starts_with("fail: not shown"),
        "{rows:#?}"
    );
}

/// A run ended by SIGTERM still sets every leaf it wrote back to its snapshot value (9.4: the
/// trap on EXIT, INT and TERM).
#[test]
fn a_terminated_run_restores_what_it_wrote() {
    let d = Daemon::start("t");
    let before = writable_values(&d.state(A), "fake-a");
    let results = results_file("t");
    let mut child: Child = d
        .command(
            &results,
            Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
            &["--device", "fake-a", "--assume-pass"],
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // a probe that returns "not yet": wait until a few rows have run
    let until = Instant::now() + Duration::from_secs(60);
    while rows(&results).len() < 12 {
        assert!(Instant::now() < until, "the script wrote no rows");
        assert!(
            child.try_wait().unwrap().is_none(),
            "the script ended early"
        );
        thread::sleep(Duration::from_millis(20));
    }
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(143));
    let after = writable_values(&d.state(A), "fake-a");
    assert_eq!(before, after, "restored after SIGTERM");
}

/// The person's answers come from a file, one per line (`CHECK_MODEL_TTY`).
fn answers(name: &str, text: &str) -> PathBuf {
    let path =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("check-model-{name}.answers"));
    std::fs::write(&path, text).unwrap();
    path
}

/// Every action and every asleep row asks first (9.4: "run one by one after a confirmation").
/// A person who answers no sends nothing but the sleeps and wakes of the asleep rows of
/// settings: no preset, view, power-on or gimbal action reaches the camera, and no action row
/// is recorded as run.
#[test]
fn a_person_who_declines_every_action_sends_none() {
    let d = Daemon::start("no");
    let results = results_file("no");
    let out = d
        .command(
            &results,
            Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
            &["--device", "fake-a", "--asleep"],
        )
        .env("CHECK_MODEL_TTY", answers("no", &"n\n".repeat(400)))
        .output()
        .unwrap();
    // the rows of `readback:false` leaves are not confirmed: the run fails, as it should
    assert_eq!(out.status.code(), Some(1), "{}", report(&out));
    let rows = rows(&results);
    let run_actions: Vec<&Row> = rows
        .iter()
        .filter(|r| {
            (r.kind == "action" || (r.kind == "asleep" && r.how.starts_with("action")))
                && !r.result.starts_with("not run")
        })
        .collect();
    assert!(run_actions.is_empty(), "{run_actions:#?}");
    assert!(rows.iter().any(|r| r.kind == "asleep"
        && r.how.starts_with("action")
        && r.result == "not run (declined)"));
    let names: Vec<String> = acts(&d, 0).into_iter().map(|(a, _)| a).collect();
    assert!(
        names.iter().all(|a| a == "sleep" || a == "wake"),
        "an action reached the camera after a no: {names:?}"
    );
}

/// The presets and views of the owner are never touched: the script saves its own preset in a
/// slot the snapshot shows free and its own view under the name `check`, works on those only,
/// and deletes them again, so the lists are as before.
#[test]
fn the_owners_presets_and_views_are_left_alone() {
    let d = Daemon::start("own");
    let cli = |args: &[&str]| {
        let (code, _, err) = common::cli(Some(&d.socket), args);
        assert_eq!(code, 0, "{err}");
    };
    cli(&[
        "do",
        "preset.save",
        "slot=0",
        "name=mine",
        "--device",
        "fake-a",
    ]);
    cli(&["do", "view.save", "name=mine", "--device", "fake-a"]);
    let before = lists(&d.state(A));
    assert_eq!(before.0[0]["name"], "mine");
    assert_eq!(before.1[0]["name"], "mine");
    let from = d.bus.ops(A).len();
    let (out, rows) = d.check("own", &["--device", "fake-a", "--assume-pass", "--asleep"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert_eq!(
        lists(&d.state(A)),
        before,
        "the owner's lists are as before"
    );
    let on_items: Vec<_> = acts(&d, from)
        .into_iter()
        .filter(|(a, _)| {
            a.starts_with("preset.") || a.starts_with("view.") || a == "power_on.use_preset"
        })
        .collect();
    assert!(on_items.len() >= 8, "{on_items:#?}");
    for (action, params) in &on_items {
        let own = if action.starts_with("view.") {
            params.get("id")
        } else {
            params.get("slot")
        };
        if let Some(own) = own {
            assert_ne!(
                own,
                &Value::from(0),
                "{action} {params:?} works on the owner's item"
            );
        }
    }
    assert_eq!(row(&rows, "action", "preset.save", "slot=1").result, "pass");
}

/// A camera that is asleep at the start is woken first when the person agrees (as the 0.13
/// script did), and the script ends with a no instead of failing every row when they do not.
#[test]
fn a_sleeping_camera_is_woken_first_when_the_person_agrees() {
    let d = Daemon::start("w");
    let (code, _, err) = common::cli(Some(&d.socket), &["do", "sleep", "--device", "fake-a"]);
    assert_eq!(code, 0, "{err}");
    let until = Instant::now() + Duration::from_secs(10);
    while d.state(A)["asleep"] != Value::Bool(true) {
        assert!(Instant::now() < until, "the fake does not fall asleep");
        thread::sleep(Duration::from_millis(50));
    }
    let results = results_file("w-no");
    let out = d
        .command(
            &results,
            Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
            &["--device", "fake-a"],
        )
        .env("CHECK_MODEL_TTY", answers("w-no", "n\n"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", report(&out));
    assert_eq!(d.state(A)["asleep"], Value::Bool(true), "nothing was sent");
    let (out, rows) = d.check("w", &["--device", "fake-a", "--assume-pass"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert!(
        !rows.iter().any(|r| r.result.starts_with("fail")),
        "{rows:#?}"
    );
}

#[test]
fn without_device_it_is_a_usage_error() {
    let out = Command::new("bash")
        .arg(script())
        .env("OBSBOT4LINUX", env!("CARGO_BIN_EXE_obsbot4linux"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage: check-model.sh --device"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `nice -n 19 cargo test -p obsbot4linux -j 2 --test check_model`
Expected: FAIL: 9 tests fail, each because bash cannot open `scripts/check-model.sh` (`No such file or directory`, exit 127, or no results file).

- [ ] **Step 3: Write the script**

Create `scripts/check-model.sh` and `chmod 755` it:

```bash
#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# check-model.sh: checks one device against its own description (design spec 9.4).
#
#   scripts/check-model.sh --device <id> [--asleep] [--destructive] [--assume-pass]
#                          [--results <file>]
#
# The rows come from `obsbot4linux describe --json`: every writable enum value, both values of
# each bool, the minimum and the maximum of each number (one row each) and the full span of
# each range (one row: `min.min,max.max`). A setting row passes when the state shows the
# written value within 70 s; the value is then restored, after every row, and the restore must
# show too. The row says how it was seen: `reply` (the state the reply published already shows
# it, the same rev) or `later` (a later state), with the lag from the reply. The last column
# says which kind a row is: `readback`, `no_effect (readback only)`, `readback:false (person
# in view)` or `action (person in view)`; the rows that need a person in view ask the person
# running the script, `--assume-pass` answers every question with pass (CI runs the script so
# against the fake devices). Rows whose `warning` holds `restart` or `destructive` run only
# with `--destructive`, last; they are not restored row by row (the camera restarts for each).
# A leaf with `requires` gets its required values set first, each as its own `set` the output
# names, and restored after its rows.
#
# Actions and asleep rows ask first. What an action stores on the camera is not restored:
# stored presets, views, power-on and return poses. The state before the first action row is
# printed, and the rows that work on a list item use a preset slot or a view the snapshot
# shows free or absent, saved by the script itself and deleted by it again; an action that
# needs such an item and finds none is not run. `--asleep` adds the asleep rows: sleep, write
# a value that differs from the snapshot (after the `requires` set while awake), wake, look
# for it. A `.turn` action runs asleep only with its `.stop`. The state is snapshot first; on
# exit, an interrupt or a termination every leaf the script wrote is set back to its snapshot
# value, the script's own presets and views are deleted, a turn is stopped and the pose is
# moved back. Output: one line per row and a results file (tab-separated).
#
# Needs bash, jq and the command line `obsbot4linux` (or the one named by $OBSBOT4LINUX); the
# daemon is reached as the command line reaches it ($OBSBOT4LINUX_SOCKET or the default).
# Exit: 0 every row that ran passed; 1 a row failed or a leaf could not be restored; 2 usage;
# 3 or 4 as the command line (3 also: the device is not available or asleep and not woken).

set -u

CLI=${OBSBOT4LINUX:-obsbot4linux}
# The slowest periodic tier (60 s) plus 10 s (9.4); the asleep waits are 30 s. Tests shorten them.
WAIT=${CHECK_MODEL_WAIT:-70}
SLEEP_WAIT=${CHECK_MODEL_SLEEP_WAIT:-30}
RETURN_WAIT=${CHECK_MODEL_RETURN_WAIT:-60}
POLL=${CHECK_MODEL_POLL:-0.2}
# Where the person's answers come from (tests: a file with one answer per line).
ANSWERS=${CHECK_MODEL_TTY:-/dev/tty}

DEVICE="" ASLEEP=0 DESTRUCTIVE=0 ASSUME=0 RESULTS=""

usage() {
	echo "usage: check-model.sh --device <id> [--asleep] [--destructive] [--assume-pass] [--results <file>]" >&2
	exit 2
}

while [ $# -gt 0 ]; do
	case $1 in
	--device)
		[ $# -ge 2 ] || usage
		DEVICE=$2
		shift 2
		;;
	--asleep)
		ASLEEP=1
		shift
		;;
	--destructive)
		DESTRUCTIVE=1
		shift
		;;
	--assume-pass)
		ASSUME=1
		shift
		;;
	--results)
		[ $# -ge 2 ] || usage
		RESULTS=$2
		shift 2
		;;
	-h | --help) usage ;;
	*)
		echo "check-model.sh: unknown argument: $1" >&2
		usage
		;;
	esac
done
[ -n "$DEVICE" ] || usage
command -v jq >/dev/null || {
	echo "check-model.sh: jq is needed" >&2
	exit 2
}
if [ "$ASSUME" != 1 ]; then
	exec 3<"$ANSWERS" || {
		echo "check-model.sh: cannot read the answers from $ANSWERS" >&2
		exit 2
	}
fi

# Text that came from the camera (a preset or view name, a firmware string) reaches the output
# only through this filter: JSON escapes the C0 controls, the filter replaces DEL and C1.
JQ_CLEAN='def clean: if type == "string" then gsub("[\u007f-\u009f]"; "?") else . end; def cleanall: walk(clean);'
# Numbers are rounded to nine decimals, so that a value's text does not depend on how a jq
# version prints a float; `mid` is the point three quarters up a range (on its step or
# precision, inside the range, not at a limit).
# shellcheck disable=SC2016 # jq variables, not shell ones
JQ_NUM='def num: if type == "number" then ((. * 1e9 | round) / 1e9) else . end;
    def mid($lo; $hi; $step; $prec):
        ($lo + ($hi - $lo) * 3 / 4) as $m
        | if $step != null then ((($m - $lo) / $step | floor) * $step + $lo)
          elif $prec != null then (($m / $prec | round) * $prec)
          else $m end;'

NOTE="not restored by this script: stored presets, views, power-on and return poses"

# One request through the command line; its JSON on stdout, its exit code returned.
cli() {
	local sub=$1
	shift
	"$CLI" "$sub" --json --device "$DEVICE" "$@"
}

now_ms() { date +%s%3N; }

# The code of a failed reply (one of the protocol's own codes), else `error`.
errcode() {
	local code
	code=$(jq -r '.error.code // empty' <<<"$1" 2>/dev/null | head -n 1 | tr -cd 'a-z_')
	echo "${code:-error}"
}

DESC=$(cli describe) || exit $?
ID=$(jq -r .id <<<"$DESC")
MODEL=$(jq -r .model <<<"$DESC")
SHORT=$("$CLI" devices --device "$DEVICE" | head -n 1 | cut -f 1 -d ' ')
FULL=$(cli status) || exit $?
SNAP=$(jq -c .state <<<"$FULL")

# The state the rows start from: the camera must be there and awake (the 0.13 script woke it).
if [ "$(jq -r .available <<<"$SNAP")" != true ]; then
	echo "check-model.sh: $SHORT is not available" >&2
	exit 3
fi
if [ "$(jq -r .asleep <<<"$SNAP")" = true ]; then
	jq -e '.actions.wake' <<<"$DESC" >/dev/null || {
		echo "check-model.sh: $SHORT is asleep and has no wake action" >&2
		exit 3
	}
fi

[ -n "$RESULTS" ] || RESULTS="check-model-$MODEL-$(date +%Y-%m-%d).tsv"
{
	echo "# check-model.sh $(date -u +%Y-%m-%dT%H:%M:%SZ)"
	echo "# device $SHORT model $MODEL firmware $(jq -c "$JQ_CLEAN"'.firmware // "not reported" | clean' <<<"$DESC") support $(jq -r .support <<<"$DESC")"
	printf '# kind\tpath\tvalue\tresult\tlag_ms\tseen\thow\n'
} >"$RESULTS"

FAILED=0
CHANGED=()
MOVED=0
TURN_STOPS=()
OWN_PRESET="" PRESET_SAVED=0 OWN_VIEW=""
FINISHING=0

# The value of a dotted path in a state.
leaf() { jq -c --arg p "$2" 'getpath($p | split("."))' <<<"$1"; }

# Whether a state shows `value` at `path`: the leaf equals it (numbers within half the
# precision, per component for a pose or a range) and the path is not pending.
shows() {
	jq -e --arg p "$2" --argjson v "$3" --argjson prec "$4" '
        def near($a; $b): if ($a | type) == "number" and ($b | type) == "number"
            then (($a - $b) | fabs) <= ($prec / 2 + 1e-9) else $a == $b end;
        def same($a; $b): if ($a | type) == "object" and ($b | type) == "object"
            then [$b | keys[] as $k | near($a[$k]; $b[$k])] | all else near($a; $b) end;
        (.pending | has($p) | not) and same(getpath($p | split("."))?; $v)
    ' <<<"$1" >/dev/null 2>&1
}

# The command-line text of a JSON value (spec 6.2 value syntax). Numbers are rounded to nine
# decimals, so that the text does not depend on how a jq version prints a float.
text() {
	jq -r "$JQ_NUM"'if type == "object" then
               (if has("pan") then "\(.pan | num),\(.tilt | num),\(.zoom | num)"
                else "\(.min | num),\(.max | num)" end)
           elif type == "boolean" then (if . then "on" else "off" end)
           else num | tostring end' <<<"$1"
}

full_state() {
	local out
	out=$(cli status) || return 1
	jq -c .state <<<"$out"
}

# One line of output and one of the results file: kind path value result [lag_ms seen how].
# Control characters of any field are replaced, so that a field cannot make another line.
record() {
	local i f out=()
	for i in 1 2 3 4 5 6 7; do
		f=${!i:-}
		out+=("${f//[[:cntrl:]]/ }")
	done
	printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "${out[@]}" >>"$RESULTS"
	printf '%-8s %-34s %-18s %s%s%s\n' "${out[0]}" "${out[1]}" "${out[2]}" "${out[3]}" \
		"${out[4]:+  lag ${out[4]} ms (${out[5]})}" "${out[6]:+  [${out[6]}]}"
	case $4 in fail*) FAILED=1 ;; esac
}

# Asks the person; with --assume-pass the answer is pass (the question is still printed).
ask() {
	if [ "$ASSUME" = 1 ]; then
		echo "  ask: $1 -> pass (--assume-pass)"
		return 0
	fi
	local answer
	printf '%s [y/n] ' "$1" >&2
	read -r answer <&3
	[ "$answer" = y ]
}

# Waits up to $1 seconds for the state to show $3 at $2 (precision $4).
wait_for() {
	local until=$(($(date +%s) + $1)) st
	while :; do
		st=$(full_state) || return 1
		if shows "$st" "$2" "$3" "$4"; then return 0; fi
		[ "$(date +%s)" -lt "$until" ] || return 1
		sleep "$POLL"
	done
}

# Waits up to $1 seconds for the leaf `asleep` to be $2.
wait_asleep() {
	local until=$(($(date +%s) + $1))
	while :; do
		[ "$(leaf "$(full_state)" asleep)" = "$2" ] && return 0
		[ "$(date +%s)" -lt "$until" ] || return 1
		sleep "$POLL"
	done
}

# The camera must be awake for the rows; a sleeping one is woken when the person agrees.
if [ "$(jq -r .asleep <<<"$SNAP")" = true ]; then
	ask "$SHORT is asleep: wake it for the check?" || exit 3
	cli "do" wake >/dev/null || exit 3
	wait_asleep "$SLEEP_WAIT" false || {
		echo "check-model.sh: $SHORT is still asleep after ${SLEEP_WAIT} s" >&2
		exit 3
	}
	SNAP=$(full_state) || exit 3
fi

# The setting rows of one leaf: the values to write, in order.
values_of() {
	jq -c --arg p "$1" --argjson snap "$(leaf "$SNAP" "$1")" '.settings[$p] as $s |
        if $s.type == "bool" then (if $snap == true then [false, true] else [true, false] end)
        elif $s.type == "enum" then $s.writable
        elif $s.type == "number" then [$s.min, $s.max] | map(select(. != null))
        elif $s.type == "range" then [{min: $s.min.min, max: $s.max.max}]
        else [] end | .[]' <<<"$DESC"
}

precision_of() {
	jq -r --arg p "$1" '.settings[$p].precision // 0 |
        if type == "object" then ([.[]] | max) else . end' <<<"$DESC"
}

# Which kind a setting row is (spec 9.4).
how_of() {
	jq -r --arg p "$1" '.settings[$p] |
        if .readback == false then "readback:false (person in view)"
        elif .no_effect then "no_effect (readback only)"
        else "readback" end' <<<"$DESC"
}

readback_of() { jq -r --arg p "$1" '.settings[$p].readback' <<<"$DESC"; }

# Puts one leaf back to its snapshot value after a row; a leaf that reads back must show it.
# A restore that fails is a warning row here; the trap tries again and names what is left.
restore_row() {
	local path=$1 snap reply
	snap=$(leaf "$SNAP" "$path")
	[ "$snap" = null ] && return 0
	if ! reply=$(cli set "$path=$(text "$snap")" 2>&1); then
		restore_warning "$path" "$snap" "restore refused ($(errcode "$reply"))"
		return 1
	fi
	[ "$(readback_of "$path")" = true ] || return 0
	wait_for "$WAIT" "$path" "$snap" "$(precision_of "$path")" || {
		restore_warning "$path" "$snap" "restore not shown within ${WAIT} s"
		return 1
	}
}

restore_warning() {
	if [ "$FINISHING" = 1 ]; then
		echo "  warn: $1 $2: $3"
	else
		record restore "$1" "$2" "warn: $3"
	fi
}

# The setup a leaf needs: its `requires` (spec 4.2), as `path=value` arguments.
setup_of() {
	jq -r --arg p "$1" '.settings[$p].requires // [] | .[] |
        "\(.path)=\(.equals | if type == "boolean" then (if . then "on" else "off" end) else tostring end)"' <<<"$DESC"
}

# Writes the setup of a leaf, each required value as its own `set` the output names; the row
# of kind $2 fails when one is refused.
setup_leaf() {
	local path=$1 kind=$2 req reqs
	mapfile -t reqs < <(setup_of "$path")
	for req in "${reqs[@]}"; do
		[ -n "$req" ] || continue
		echo "  setup for $path: set $req"
		CHANGED+=("${req%%=*}")
		cli set "$req" >/dev/null 2>&1 || {
			record "$kind" "$path" - "fail: setup $req refused"
			return 1
		}
	done
}

restore_setup() {
	local req reqs
	mapfile -t reqs < <(setup_of "$1")
	for req in "${reqs[@]}"; do
		[ -n "$req" ] && restore_row "${req%%=*}"
	done
	return 0
}

# One setting row: write, wait for the value, restore. `reply` when the state of the same rev
# as the reply's already shows the value; `later` otherwise, with the time from the reply.
setting_row() {
	local path=$1 value=$2 prec how reply replied rrev out srev st seen=later lag=0
	prec=$(precision_of "$path")
	how=$(how_of "$path")
	CHANGED+=("$path")
	if ! reply=$(cli set "$path=$(text "$value")" 2>&1); then
		record setting "$path" "$value" "fail: $(errcode "$reply")" "" "" "$how"
		return
	fi
	replied=$(now_ms)
	if [ "$(readback_of "$path")" = false ]; then
		if ask "$(jq -r --arg p "$path" '.settings[$p].label' <<<"$DESC") = $(text "$value"): is it so on the camera? (person in view)"; then
			record setting "$path" "$value" "pass (confirmed)" "" "" "$how"
		else
			record setting "$path" "$value" "fail (not confirmed)" "" "" "$how"
		fi
		restore_row "$path"
		return
	fi
	rrev=$(jq -r '.rev // empty' <<<"$reply" 2>/dev/null)
	out=$(cli status 2>/dev/null) || out=""
	st=$(jq -c .state <<<"$out" 2>/dev/null) || st=""
	srev=$(jq -r '.rev // empty' <<<"$out" 2>/dev/null)
	if [ -n "$st" ] && shows "$st" "$path" "$value" "$prec"; then
		if [ -n "$rrev" ] && [ "$srev" = "$rrev" ]; then seen=reply; fi
		lag=$(($(now_ms) - replied))
	elif wait_for "$WAIT" "$path" "$value" "$prec"; then
		lag=$(($(now_ms) - replied))
	else
		record setting "$path" "$value" "fail: not shown within ${WAIT} s" "" "" "$how"
		restore_row "$path"
		return
	fi
	[ "$seen" = reply ] && lag=0
	record setting "$path" "$value" pass "$lag" "$seen" "$how"
	restore_row "$path"
}

check_leaf() {
	local path=$1 value
	setup_leaf "$path" setting || {
		restore_setup "$path"
		return
	}
	while IFS= read -r value; do
		[ -n "$value" ] && setting_row "$path" "$value"
	done < <(values_of "$path")
	restore_setup "$path"
}

# The item an action row works on is one the script saved itself: a preset slot that the
# snapshot shows free (`preset.save` takes it, the rows after it use it, `preset.delete` ends
# it) and the view the script saved under the name `check`. An item of the owner is never
# renamed, recalled, used or deleted. A snapshot that shows no list at all leaves no slot.
OWN_PRESET=$(jq -r --argjson snap "$SNAP" '
    (.actions["preset.save"].params.slot // null) as $s
    | if $s == null or $s.min == null or $s.max == null or ($snap.presets | type) != "array" then empty
      else ([range($s.min; $s.max + 1; ($s.step // 1))] - [$snap.presets[].slot]) | first // empty end' <<<"$DESC")

# Looks for the script's own items in a state.
resync_own() {
	local st=$1
	PRESET_SAVED=0
	if [ -n "$OWN_PRESET" ] && jq -e --argjson s "$OWN_PRESET" '[.presets[]?.slot] | index($s) != null' <<<"$st" >/dev/null 2>&1; then
		PRESET_SAVED=1
	fi
	if [ -n "$OWN_VIEW" ] && ! jq -e --argjson i "$OWN_VIEW" '[.views[]?.id] | index($i) != null' <<<"$st" >/dev/null 2>&1; then
		OWN_VIEW=""
	fi
	if [ -z "$OWN_VIEW" ]; then
		OWN_VIEW=$(jq -rn --argjson a "$SNAP" --argjson b "$st" '
            if ($a.views | type) != "array" then empty
            else ([$a.views[].id]) as $old
                | [$b.views[]? | select(.name == "check") | select(.id as $i | $old | index($i) | not) | .id] | first // empty end' 2>/dev/null) || OWN_VIEW=""
	fi
}

# The parameters of an action's row: every required parameter, or the first parameter that
# is not a bool when none is required (a move needs one axis). The value is the parameter's
# default, else the point three quarters up its bounds (on its step or precision: inside the
# range, not at a limit, not zero for a symmetric one), its first writable value, `true` for a
# bool (`confirm=true` reaches only the rows of --destructive) or the text `check`. A preset
# slot or a view id is the script's own item, or `__none__` when there is none yet.
params_of() {
	local saved=${2:-$PRESET_SAVED} view=${3:-$OWN_VIEW} live_preset=""
	[ "$saved" = 1 ] && live_preset=$OWN_PRESET
	jq -r --arg a "$1" --arg own_preset "$OWN_PRESET" --arg live_preset "$live_preset" --arg own_view "$view" "$JQ_NUM"'
        def own($key):
            if $key == "slot" and ($a | startswith("preset.") or . == "power_on.use_preset") then
                (if $a == "preset.save" then $own_preset else $live_preset end)
            elif $key == "id" and ($a | startswith("view.")) then $own_view
            else null end;
        def value($key; $s):
            if own($key) != null then (own($key) | if . == "" then "__none__" else . end)
            elif $s.default != null then ($s.default | num | tostring)
            elif $s.type == "bool" then "true"
            elif $s.type == "enum" then $s.writable[0]
            elif $s.type == "string" then "check"
            elif $s.type == "pose" then
                [mid($s.min.pan; $s.max.pan; null; $s.precision.pan),
                 mid($s.min.tilt; $s.max.tilt; null; $s.precision.tilt),
                 mid($s.min.zoom; $s.max.zoom; null; $s.precision.zoom)] | map(num | tostring) | join(",")
            elif $s.min != null and $s.max != null then (mid($s.min; $s.max; $s.step; $s.precision) | num | tostring)
            elif $s.min != null then ($s.min | num | tostring)
            else "0" end;
        (.actions[$a].params // {} | to_entries) as $all
        | ([$all[] | select(.value.optional | not)]
           | if length == 0 then [$all[] | select(.value.type != "bool")][:1] else . end)[]
        | .key + "=" + value(.key; .value)' <<<"$DESC"
}

# The leaves whose values differ between two states (camera text is cleaned).
diff_states() {
	jq -rn --argjson a "$1" --argjson b "$2" "$JQ_CLEAN"'
        [$a, $b | del(.pending, .unread) | paths(scalars)] | unique[] as $p
        | select(($a | getpath($p)) != ($b | getpath($p)))
        | "    \($p | map(tostring) | join(".")): \($a | getpath($p) | clean | tojson) -> \($b | getpath($p) | clean | tojson)"'
}

# The stop of a continuing action: `<group>.stop` for a `.turn` action that has one.
stop_of() {
	case $1 in
	*.turn)
		local stop="${1%.*}.stop"
		jq -e --arg a "$stop" '.actions[$a]' <<<"$DESC" >/dev/null && echo "$stop"
		;;
	esac
	return 0
}

action_row() {
	local action=$1 before after reply params code stop
	local how="action (person in view)"
	mapfile -t params < <(params_of "$action")
	if [[ " ${params[*]} " == *=__none__* ]]; then
		record action "$action" - "not run: no item of its own (none is free or saved)" "" "" "$how"
		return
	fi
	stop=$(stop_of "$action")
	if ! ask "run do $action ${params[*]}? ($NOTE; person in view)"; then
		record action "$action" "${params[*]}" "not run (declined)" "" "" "$how"
		return
	fi
	MOVED=1
	[ -n "$stop" ] && TURN_STOPS+=("$stop")
	before=$(full_state)
	if ! reply=$(cli "do" "$action" "${params[@]}" 2>&1); then
		code=$(errcode "$reply")
		case $code in
		conflict | asleep) record action "$action" "${params[*]}" "not run: $code" "" "" "$how" ;;
		*) record action "$action" "${params[*]}" "fail: $code" "" "" "$how" ;;
		esac
		return
	fi
	sleep "$POLL"
	after=$(full_state)
	resync_own "$after"
	echo "  state change after do $action:"
	diff_states "$before" "$after"
	if ask "does the camera show the effect of $action (ledger: effect column)?"; then
		record action "$action" "${params[*]}" pass "" "" "$how"
	else
		record action "$action" "${params[*]}" "fail (not confirmed)" "" "" "$how"
	fi
}

# A row whose write or action restarts the camera: it passes when the device returns under
# the same id within 60 s with the expected state.
restart_row() {
	local kind=$1 name=$2 value=$3 reply until how
	if [ "$kind" = setting ]; then
		how=$(how_of "$name")
		CHANGED+=("$name")
		reply=$(cli set "$name=$(text "$value")" 2>&1) || {
			record "$kind" "$name" "$value" "fail: $(errcode "$reply")" "" "" "$how"
			return
		}
	else
		local params
		how="action (person in view)"
		mapfile -t params < <(params_of "$name")
		value="${params[*]}"
		ask "run do $name $value? the camera restarts (person in view)" || {
			record "$kind" "$name" "$value" "not run (declined)" "" "" "$how"
			return
		}
		reply=$(cli "do" "$name" "${params[@]}" 2>&1) || {
			record "$kind" "$name" "$value" "fail: $(errcode "$reply")" "" "" "$how"
			return
		}
	fi
	until=$(($(date +%s) + RETURN_WAIT))
	while :; do
		if [ "$("$CLI" --json devices 2>/dev/null | jq -r --arg id "$ID" '.devices[] | select(.id == $id) | .available')" = true ]; then
			if [ "$kind" = action ] || wait_for "$WAIT" "$name" "$value" "$(precision_of "$name")"; then
				record "$kind" "$name" "$value" pass "" returned "$how"
				return
			fi
		fi
		[ "$(date +%s)" -lt "$until" ] || break
		sleep "$POLL"
	done
	record "$kind" "$name" "$value" "fail: not back under $SHORT within ${RETURN_WAIT} s" "" "" "$how"
}

# The first value of a leaf's rows that differs from the snapshot; a write of the value the
# camera already shows would read `accepted` whether or not the camera took it.
differing_value() {
	local path=$1 prec value
	prec=$(precision_of "$path")
	while IFS= read -r value; do
		[ -n "$value" ] || continue
		if ! shows "$SNAP" "$path" "$value" "$prec"; then
			printf '%s\n' "$value"
			return 0
		fi
	done < <(values_of "$path"; mid_of "$path")
	return 1
}

# For a number that has both bounds: the point three quarters up (the third value to try when
# the snapshot already shows the minimum or the maximum).
mid_of() {
	jq -c --arg p "$1" "$JQ_NUM"'.settings[$p] | select(.type == "number" and .min != null and .max != null)
        | mid(.min; .max; .step; .precision) | num' <<<"$DESC"
}

# Saves the item an asleep row of an action needs, awake, when the script has none yet.
ensure_own() {
	case $1 in
	preset.rename | preset.delete | preset.recall | power_on.use_preset)
		[ "$PRESET_SAVED" = 1 ] && return 0
		[ -n "$OWN_PRESET" ] || return 1
		jq -e '.actions["preset.save"]' <<<"$DESC" >/dev/null || return 1
		echo "  setup for $1: do preset.save slot=$OWN_PRESET"
		cli "do" preset.save "slot=$OWN_PRESET" >/dev/null 2>&1 || return 1
		resync_own "$(full_state)"
		[ "$PRESET_SAVED" = 1 ]
		;;
	view.rename | view.delete | view.go)
		[ -n "$OWN_VIEW" ] && return 0
		jq -e '.actions["view.save"]' <<<"$DESC" >/dev/null || return 1
		echo "  setup for $1: do view.save name=check"
		cli "do" view.save name=check >/dev/null 2>&1 || return 1
		resync_own "$(full_state)"
		[ -n "$OWN_VIEW" ]
		;;
	esac
	return 0
}

# An asleep row: sleep, write, wake, look for the value (spec 9.4 `--asleep`).
asleep_row() {
	local kind=$1 name=$2 value="" reply code stop="" how params
	if [ "$kind" = setting ]; then
		how=$(how_of "$name")
		value=$(differing_value "$name") || {
			record asleep "$name" - "not run: no value that differs from the snapshot" "" "" "$how"
			return
		}
		setup_leaf "$name" asleep || {
			restore_setup "$name"
			return
		}
	else
		how="action (person in view)"
		# what the row would work on, before anything is saved: the question comes first
		value=$(params_of "$name" 1 "${OWN_VIEW:-<its own view>}" | tr '\n' ' ')
		case $name in
		*.turn)
			stop=$(stop_of "$name")
			[ -n "$stop" ] || {
				record asleep "$name" "$value" "not run: no stop action to end it" "" "" "$how"
				return
			}
			;;
		esac
		if [[ " $value " == *=__none__* ]]; then
			record asleep "$name" - "not run: no item of its own (none is free or saved)" "" "" "$how"
			return
		fi
		ask "asleep row: sleep, do $name $value${stop:+, then $stop}, wake. $NOTE; person in view. Run it?" || {
			record asleep "$name" "$value" "not run (declined)" "" "" "$how"
			return
		}
		ensure_own "$name" || {
			record asleep "$name" "$value" "not run: no item of its own (none is free or saved)" "" "" "$how"
			return
		}
		mapfile -t params < <(params_of "$name")
		value="${params[*]}"
		MOVED=1
		[ -n "$stop" ] && TURN_STOPS+=("$stop")
	fi
	if ! { cli "do" sleep >/dev/null && wait_asleep "$SLEEP_WAIT" true; }; then
		record asleep "$name" "$value" "fail: no asleep:true within ${SLEEP_WAIT} s" "" "" "$how"
		return
	fi
	if [ "$kind" = setting ]; then
		CHANGED+=("$name")
		reply=$(cli set "$name=$(text "$value")" 2>&1)
	else
		reply=$(cli "do" "$name" "${params[@]}" 2>&1)
		[ -n "$stop" ] && cli "do" "$stop" >/dev/null 2>&1
	fi
	code=$(jq -r '.error.code // "ok"' <<<"$reply" 2>/dev/null)
	if ! { cli "do" wake >/dev/null && wait_asleep "$SLEEP_WAIT" false; }; then
		record asleep "$name" "$value" "fail: no asleep:false within ${SLEEP_WAIT} s" "" "" "$how"
		return
	fi
	# A turn goes on after the wake (the command line sends keep:true): stop it again.
	[ -n "$stop" ] && cli "do" "$stop" >/dev/null 2>&1
	if [ "$code" != ok ]; then
		record asleep "$name" "$value" "ignored (reply $(errcode "$reply"))" "" "" "$how"
	elif [ "$kind" = setting ] && [ "$(readback_of "$name")" = true ]; then
		if wait_for "$WAIT" "$name" "$value" "$(precision_of "$name")"; then
			record asleep "$name" "$value" accepted "" "" "$how"
		else
			record asleep "$name" "$value" "ignored (not shown after wake)" "" "" "$how"
		fi
	elif ask "after the wake: does the camera show $name $value? (person in view)"; then
		record asleep "$name" "$value" accepted "" "" "$how"
	else
		record asleep "$name" "$value" "ignored (not confirmed)" "" "" "$how"
	fi
	if [ "$kind" = setting ]; then
		restore_row "$name"
		restore_setup "$name"
	else
		resync_own "$(full_state)"
	fi
}

# The script's own presets and views are deleted again (a list item is not a leaf).
# shellcheck disable=SC2329 # run by the trap
cleanup_own() {
	local st=$1
	if [ -n "$OWN_PRESET" ] && jq -e --argjson s "$OWN_PRESET" '[.presets[]?.slot] | index($s) != null' <<<"$st" >/dev/null 2>&1; then
		cli "do" preset.delete "slot=$OWN_PRESET" >/dev/null 2>&1
	fi
	if [ -n "$OWN_VIEW" ]; then
		cli "do" view.delete "id=$OWN_VIEW" >/dev/null 2>&1
	fi
}

# On exit, an interrupt or a termination: every leaf written goes back to its snapshot value,
# the last one written first; the script's own presets and views are deleted, a turn is
# stopped, then the pose of the snapshot (spec 9.4). A leaf that does not show its snapshot
# value afterwards is named and makes the exit code 1.
# shellcheck disable=SC2329 # run by the trap
finish() {
	local rc=$? i path st stop bad=0
	declare -A done_
	trap - EXIT INT TERM
	FINISHING=1
	WAIT=${CHECK_MODEL_FINISH_WAIT:-10}
	# after a restart row the device may still be settling: wait for it
	local until=$(($(date +%s) + WAIT))
	while :; do
		st=$(full_state 2>/dev/null) || st=""
		[ "$(leaf "${st:-null}" available 2>/dev/null)" = true ] && break
		[ "$(date +%s)" -lt "$until" ] || break
		sleep "$POLL"
	done
	[ -n "$st" ] || st=$SNAP
	for stop in "${TURN_STOPS[@]}"; do
		cli "do" "$stop" >/dev/null 2>&1
	done
	for ((i = ${#CHANGED[@]} - 1; i >= 0; i--)); do
		path=${CHANGED[i]}
		shows "$st" "$path" "$(leaf "$SNAP" "$path")" "$(precision_of "$path")" 2>/dev/null && continue
		restore_row "$path" 2>/dev/null && st=$(full_state 2>/dev/null || echo "$st")
	done
	resync_own "$st"
	cleanup_own "$st"
	if [ "$MOVED" = 1 ] && jq -e '.actions["gimbal.move"]' <<<"$DESC" >/dev/null; then
		local pan tilt
		pan=$(leaf "$SNAP" gimbal.pan)
		tilt=$(leaf "$SNAP" gimbal.tilt)
		if [ "$pan" != null ] && [ "$tilt" != null ]; then
			cli "do" gimbal.move "pan=$pan" "tilt=$tilt" >/dev/null 2>&1
		fi
	fi
	st=$(full_state 2>/dev/null) || st=""
	for path in "${CHANGED[@]}"; do
		[ -n "${done_[$path]:-}" ] && continue
		done_[$path]=1
		[ "$(leaf "$SNAP" "$path")" = null ] && continue
		[ "$(readback_of "$path")" = true ] || continue
		if ! shows "$st" "$path" "$(leaf "$SNAP" "$path")" "$(precision_of "$path")"; then
			echo "not restored: $path (snapshot $(leaf "$SNAP" "$path"))"
			bad=1
		fi
	done
	echo "results: $RESULTS"
	[ "$bad" = 0 ] || [ "$rc" != 0 ] || rc=1
	exit "$rc"
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# What may restart the camera or destroy something.
risky() { jq -e --arg k "$1" --arg n "$2" '.[$k][$n].warning // [] | any(. == "restart" or . == "destructive")' <<<"$DESC" >/dev/null; }

mapfile -t LEAVES < <(jq -r '.settings | to_entries[] | select(.value.readonly | not) | .key' <<<"$DESC")
# Actions: those that save first, so that a row that needs a list item finds the one the
# script saved; then group by group (the path before the last dot), a stop after the moves of
# its group and a delete last; `sleep` and `wake` run on their own after them.
mapfile -t ACTIONS < <(jq -r '.actions | keys | map(select(. != "sleep" and . != "wake"))
    | sort_by((if endswith(".save") then 0 elif endswith(".stop") then 2
               elif endswith(".delete") then 3 else 1 end) as $c
              | [(if $c == 0 then 0 else 1 end), (split(".") | .[:-1] | join(".")), $c])[]' <<<"$DESC")

for path in "${LEAVES[@]}"; do
	if risky settings "$path"; then continue; fi
	check_leaf "$path"
done

# What the action rows may change and the script does not restore, as it was before.
echo "stored on the camera before the action rows ($NOTE):"
jq -c "$JQ_CLEAN"'{presets, views, power_on: .power_on.start, hand_return: .hand.return, zone_return: .zone.return} | cleanall' <<<"$SNAP"
[ -n "$OWN_PRESET" ] && echo "  the script's own preset slot: $OWN_PRESET (free in the snapshot)"

for action in "${ACTIONS[@]}"; do
	if risky actions "$action"; then continue; fi
	action_row "$action"
done
# `sleep`, then `wake`; the rows after them need the camera awake.
for action in sleep wake; do
	if jq -e --arg a "$action" '.actions[$a]' <<<"$DESC" >/dev/null; then
		action_row "$action"
	fi
done
wait_asleep "$SLEEP_WAIT" false || record action wake - "fail: still asleep after ${SLEEP_WAIT} s"

if [ "$ASLEEP" = 1 ]; then
	for path in "${LEAVES[@]}"; do
		risky settings "$path" && continue
		[ "$(jq -r --arg p "$path" '.settings[$p].asleep' <<<"$DESC")" = unknown ] && asleep_row setting "$path"
	done
	for action in "${ACTIONS[@]}"; do
		risky actions "$action" && continue
		[ "$(jq -r --arg a "$action" '.actions[$a].asleep' <<<"$DESC")" = unknown ] && asleep_row action "$action"
	done
fi

for path in "${LEAVES[@]}"; do
	risky settings "$path" || continue
	[ "$DESTRUCTIVE" = 1 ] || {
		record setting "$path" - "not run (--destructive)" "" "" "$(how_of "$path")"
		continue
	}
done
for action in "${ACTIONS[@]}"; do
	risky actions "$action" || continue
	[ "$DESTRUCTIVE" = 1 ] || {
		record action "$action" - "not run (--destructive)" "" "" "action (person in view)"
		continue
	}
done
if [ "$DESTRUCTIVE" = 1 ]; then
	echo "presets and views before the restart and destructive rows:"
	full_state 2>/dev/null | jq -c "$JQ_CLEAN"'{presets, views} | cleanall' 2>/dev/null || true
	for path in "${LEAVES[@]}"; do
		risky settings "$path" || continue
		while IFS= read -r value; do
			[ -n "$value" ] && restart_row setting "$path" "$value"
		done < <(values_of "$path")
	done
	for action in "${ACTIONS[@]}"; do
		risky actions "$action" && restart_row action "$action" -
	done
fi

exit "$FAILED"
```

Run: `shellcheck -s bash scripts/check-model.sh` (if shellcheck is installed)
Expected: no output.

Then `git rm scripts/check-readback.sh` and add the line `check-model-*.tsv` to `.gitignore`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot4linux -j 2 --test check_model`
Expected: `test result: ok. 9 passed`, in about 30 s. Then run it 20 times in a row (retrospective lesson 9): `for i in $(seq 20); do nice -n 19 cargo test -q -p obsbot4linux -j 2 --test check_model 2>&1 | grep 'test result'; done` — 20 lines `ok. 9 passed`. Look at one results file (`$CARGO_TARGET_DIR/tmp/check-model-a.tsv`): about 250 rows for `fake-a`, no `fail`, the actions `preset.save` first (in the free slot `slot=0`) and `factory_reset` last, a `how` in every row.
Then run the tests once more with the jq of CI's runner first in `PATH` (the first CI run is otherwise the only proof): `PATH=<directory holding a jq 1.6 binary named jq>:$PATH nice -n 19 cargo test -p obsbot4linux -j 2 --test check_model` — 9 passed (about 80 s, jq 1.6 is slower). Then the workspace gate (fmt, clippy `-D warnings`, `cargo test --workspace --locked`, `scripts/check-cli-deps.sh`).

If a row fails against the landed daemon (b-14 and b-15 were not in the scratch build), do not change the script's rules to make it pass: find which daemon behaviour the row meets, and report `plan_mismatch` with the row and the daemon's reply unless the fix is a spelling of the same rule.

- [ ] **Step 5: Commit**

```bash
git add scripts/check-model.sh crates/obsbot4linux/tests/check_model.rs .gitignore
git rm -q scripts/check-readback.sh
git commit -m "scripts: check-model.sh checks one device against its description; CI runs it on the fakes"
```

---

### Task w2-3 (key `w2-3-protocol-doc`): `docs/protocol-2.md`: worked transcripts, lifecycles, client library

**Files:**

- Modify: `docs/protocol-2.md` (one sentence of the preamble; new sections 11 and 12 at the end)
- Create: `crates/obsbot-proto/examples/watch.rs`
- Create: `crates/obsbot-proto/tests/doc_examples.rs`

**Interfaces:**

- Consumes: `obsbot_proto::{Request, ServerMessage, Event, Hello, short_id}`, `obsbot_proto::client::{Client, Incoming}`; the daemon binary in fake mode (for re-recording only).
- Produces: nothing other code uses.

- [ ] **Step 1: Write the failing test**

Create `crates/obsbot-proto/tests/doc_examples.rs`:

````rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The worked transcripts and the client example of `docs/protocol-2.md` (sections 11 and 12)
//! stay true: every line of a `jsonl` block round-trips through the protocol types byte for
//! byte, as the golden transcripts do; a state event comes before the reply that carries its
//! rev (4.1 rule 3); and the `rust` block is the example `examples/watch.rs`, which the build
//! compiles.

use obsbot_proto::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn doc() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/protocol-2.md");
    std::fs::read_to_string(path).unwrap()
}

/// The fenced blocks of one language, in order.
fn blocks(text: &str, lang: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        match current.as_mut() {
            None if line == format!("```{lang}") => current = Some(String::new()),
            Some(_) if line == "```" => out.push(current.take().unwrap()),
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
            None => {}
        }
    }
    out
}

#[test]
fn every_transcript_line_round_trips_and_events_come_before_their_replies() {
    let text = doc();
    let blocks = blocks(&text, "jsonl");
    assert!(!blocks.is_empty(), "section 11 has a jsonl block");
    let mut lines = 0;
    for block in blocks {
        // the device of each request, and the revs of the state events seen so far
        let mut device_of: BTreeMap<String, String> = BTreeMap::new();
        let mut seen: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for line in block.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            lines += 1;
            if let Some(json) = line.strip_prefix("> ") {
                let req = Request::from_line(json).unwrap_or_else(|e| panic!("{line}: {e}"));
                assert_eq!(serde_json::to_string(&req).unwrap(), json, "not byte-equal");
                let v: serde_json::Value = serde_json::from_str(json).unwrap();
                if let (Some(id), Some(device)) = (v.get("id"), v["device"].as_str()) {
                    device_of.insert(id.to_string(), device.to_string());
                }
            } else if let Some(json) = line.strip_prefix("< ") {
                let msg: ServerMessage =
                    serde_json::from_str(json).unwrap_or_else(|e| panic!("{line}: {e}"));
                assert_eq!(serde_json::to_string(&msg).unwrap(), json, "not byte-equal");
                let v: serde_json::Value = serde_json::from_str(json).unwrap();
                if v["event"] == "state" {
                    let device = v["device"].as_str().unwrap().to_string();
                    seen.entry(device)
                        .or_default()
                        .push(v["rev"].as_u64().unwrap());
                } else if let (Some(rev), Some(device), None) = (
                    v["ok"]["rev"].as_u64(),
                    device_of.get(&v["id"].to_string()),
                    v["ok"].get("state"),
                ) {
                    assert!(
                        seen.get(device).is_some_and(|revs| revs.contains(&rev)),
                        "{line}: no state event with rev {rev} before the reply"
                    );
                }
            } else {
                panic!("{line}: a transcript line starts with '> ', '< ' or '#'");
            }
        }
    }
    assert!(lines >= 10, "the transcripts hold {lines} lines");
}

#[test]
fn the_client_example_is_the_one_the_build_compiles() {
    let text = doc();
    let rust = blocks(&text, "rust");
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/watch.rs");
    let example = std::fs::read_to_string(example).unwrap();
    assert_eq!(
        rust,
        [example],
        "section 12 shows examples/watch.rs as it is"
    );
}
````

- [ ] **Step 2: Run the test to verify it fails**

Run: `nice -n 19 cargo test -p obsbot-proto -j 2 --test doc_examples`
Expected: FAIL: `section 11 has a jsonl block`, and `No such file or directory` for `examples/watch.rs`.

- [ ] **Step 3: Write the example and the doc**

Create `crates/obsbot-proto/examples/watch.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The client library in a few lines (design spec 6.1, `docs/protocol-2.md` section 12): list
//! the devices once with the blocking client, then print every state event of a subscribed
//! connection. It sends no write. Run it with `cargo run -p obsbot-proto --example watch`.

use obsbot_proto::client::{Client, ClientError, Incoming};
use obsbot_proto::{Event, Hello, short_id};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A blocking client: one request at a time, no events (subscribe: false).
    let mut client = Client::connect(Hello::new("example 1.0"))?;
    for device in client.devices().to_vec() {
        // `{:?}` quotes the short id and escapes any control character it holds: an id comes
        // from the camera, and what a terminal prints is not for the camera to choose.
        println!(
            "{:?}  {}  {:?}",
            short_id(&device.id),
            device.model,
            device.support
        );
        if device.available && !device.description.settings.is_empty() {
            match client.get(&device.id, Some(&["zoom.factor"])) {
                Ok((rev, state)) => println!("  rev {rev}: zoom {:?}", state.at("zoom.factor")),
                // A model without the leaf refuses with a code; that device ends nothing.
                Err(ClientError::Refused(body)) => {
                    println!("  zoom.factor: {}", body.code.as_str())
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    // A subscribed client: the hello reply, a sender and a receiver (15.1 row 96).
    let (hello, _sender, receiver) = Client::subscribe(Hello::new("example 1.0"))?;
    println!("daemon {} speaks protocol {}", hello.daemon, hello.protocol);
    for incoming in receiver {
        if let Incoming::Event(Event::State { device, rev, state }) = incoming? {
            println!(
                "{:?} rev {rev}: pending {:?}",
                short_id(&device),
                state.pending
            );
        }
    }
    Ok(())
}
```

In `docs/protocol-2.md`, replace the last two lines of the preamble

```text
`crates/obsbot-proto/tests/golden/` show every message. Worked examples follow in a later
revision of this file.
```

with

```text
`crates/obsbot-proto/tests/golden/` show every message. Sections 11 and 12 are not normative:
worked transcripts, lifecycles and the client library in use.
```

and append after section 10 (the code block of section 12 is the example file, byte for byte; the outer fence below has four backticks only so that this plan can show the inner ones):

````markdown
## 11. Worked transcripts

Recorded from `obsbot4linuxd --fake …` (the test models of fake mode, design spec 5.12): `> `
lines go from the client to the daemon, `< ` lines from the daemon to the client, `#` lines are
comments. Every line of a `jsonl` block below is a complete message; a test reads this file and
checks that each of them round-trips through `obsbot-proto` byte for byte and that a state
event comes before the reply that carries its rev.

### 11.1 A write whose readback lags, and a batch

```jsonl
# A client subscribes. fake-b's LED brightness reads back 250 ms after a write.
> {"id":1,"hello":{"protocol":2,"client":"example 1.0","subscribe":true,"live":[]}}
# The hello reply (devices and states) is left out here; section 11.2 shows one.
> {"id":2,"device":"sn:FAKEB0000001","set":{"led.brightness":3}}
# The touched re-read still shows 0: the camera accepted 3, its readback lags. The
# event comes before the reply, with the written value in `pending`.
< {"event":"state","device":"sn:FAKEB0000001","rev":2,"state":{"available":true,"pending":{"led.brightness":3},"unread":[],"asleep":false,"beacon":{"color":"red","pulse":null},"exposure":{"mode":"auto","time":100},"led":{"blink":null,"brightness":0},"on_air":{"active":false,"apps":[]},"presets":[],"tracking":{"mode":"off"},"zoom":{"factor":1}}}
< {"id":2,"ok":{"rev":2}}
# The next read of the LED group (a 1 s tier) shows 3: `pending` is empty again.
< {"event":"state","device":"sn:FAKEB0000001","rev":3,"state":{"available":true,"pending":{},"unread":[],"asleep":false,"beacon":{"color":"red","pulse":null},"exposure":{"mode":"auto","time":100},"led":{"blink":null,"brightness":3},"on_air":{"active":false,"apps":[]},"presets":[],"tracking":{"mode":"off"},"zoom":{"factor":1}}}
# Two requests pipelined: the second requires `exposure.mode` = manual, which the
# first writes. Here both ran in one batch: one re-read, one rev. Had the first been
# taken before the second arrived, each would get its own re-read and rev; the second
# is valid either way, as a written value counts as the current one.
> {"id":3,"device":"sn:FAKEB0000001","set":{"exposure.mode":"manual"}}
> {"id":4,"device":"sn:FAKEB0000001","set":{"exposure.time":500}}
< {"event":"state","device":"sn:FAKEB0000001","rev":4,"state":{"available":true,"pending":{},"unread":[],"asleep":false,"beacon":{"color":"red","pulse":null},"exposure":{"mode":"manual","time":500},"led":{"blink":null,"brightness":3},"on_air":{"active":false,"apps":[]},"presets":[],"tracking":{"mode":"off"},"zoom":{"factor":1}}}
< {"id":3,"ok":{"rev":4}}
< {"id":4,"ok":{"rev":4}}
# A value outside the description is refused before anything is sent; no rev.
> {"id":5,"device":"sn:FAKEB0000001","set":{"exposure.time":20000}}
< {"id":5,"error":{"code":"out_of_range","message":"exposure.time: 20000 is above the maximum 10000"}}
```

A client that shows the value from the reply on sees `led.brightness` = 3 at once: rev 2 holds
it in `pending`, rev 3 in the value. It runs no timer of its own (section 5, rule 4). Had the
readback not shown 3 within the leaf's `settle_ms` (1000 ms on fake-b, 3000 ms on the Tiny 2
Lite), the next read of its group after that bound would have dropped the path from `pending`
and the camera's value would stand.

### 11.2 Several devices, an unsupported one, unplug and re-plug

fake-b and an unsupported device; descriptions, USB facts and most of each state are shown as
`…`. The `# test control` lines are what the test-control socket of fake mode was sent, the
stand-in for a cable being pulled.

```text
> {"id":1,"hello":{"protocol":2,"client":"example 1.0","subscribe":true,"live":[]}}
< {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[{"id":"sn:FAKEB0000001","name":"Fake B 0001","model":"fake-b","support":"untested","serial":"FAKEB0000001","firmware":"0.9.1","video_node":"/dev/video102","available":true,"usb":{…},"settings":{…},"actions":{…},"limits":{"presets":5}},{"id":"port:9-2","name":"Unknown Camera 9-2","model":"unknown","support":"unsupported","serial":null,"firmware":null,"video_node":"/dev/video104","available":true,"usb":{…},"settings":{},"actions":{},"limits":{}}],"states":{"port:9-2":{"rev":1,"state":{…}},"sn:FAKEB0000001":{"rev":1,"state":{…}}}}}
> {"id":2,"device":"port:9-2","do":"wake"}
< {"id":2,"error":{"code":"unsupported_device","message":"port:9-2 is not supported by obsbot4linux"}}
# test control: {"unplug":"sn:FAKEB0000001"} -> {"ok":true}
< {"event":"devices","devices":[{"id":"sn:FAKEB0000001","name":"Fake B 0001","model":"fake-b","support":"untested","serial":"FAKEB0000001","firmware":"0.9.1","video_node":null,"available":false,"usb":{…},"settings":{…},"actions":{…},"limits":{"presets":5}},{"id":"port:9-2","name":"Unknown Camera 9-2","model":"unknown","support":"unsupported","serial":null,"firmware":null,"video_node":"/dev/video104","available":true,"usb":{…},"settings":{},"actions":{},"limits":{}}]}
< {"event":"state","device":"sn:FAKEB0000001","rev":2,"state":{"available":false,"pending":{},"unread":[],"led":{"blink":null,"brightness":0},…}}
> {"id":3,"device":"sn:FAKEB0000001","set":{"led.brightness":1}}
< {"id":3,"error":{"code":"unavailable","message":"sn:FAKEB0000001 is unplugged"}}
# test control: {"plug":"fake-b"} -> {"ok":"sn:FAKEB0000001"}
< {"event":"devices","devices":[{"id":"sn:FAKEB0000001","name":"Fake B 0001","model":"fake-b","support":"untested","serial":"FAKEB0000001","firmware":"0.9.1","video_node":"/dev/video102","available":true,"usb":{…},"settings":{…},"actions":{…},"limits":{"presets":5}},{"id":"port:9-2","name":"Unknown Camera 9-2","model":"unknown","support":"unsupported","serial":null,"firmware":null,"video_node":"/dev/video104","available":true,"usb":{…},"settings":{},"actions":{},"limits":{}}]}
< {"event":"state","device":"sn:FAKEB0000001","rev":3,"state":{"available":true,"pending":{},"unread":[],"led":{"blink":null,"brightness":0},…}}
```

- The unsupported device is listed and refused (`unsupported_device`); nothing is sent to it.
- An `sn:` entry stays listed while unplugged, `available: false`, and a request for it is
  answered at once with `unavailable`. The re-plug brings it back under the same id; its rev
  goes on (3). A `port:` entry is removed at unplug and starts at rev 1 when it is added again
  (section 5, rule 1).
- The `devices` event comes before the `state` event of the same change.

### 11.3 Lifecycles

A connection:

```text
connect ── hello ──► reply (devices, states) ──► requests, pipelined; events if subscribe:true
   │                     │
   │                     └─ before the first enumeration has read each device: the reply
   │                        waits, at most 15 s; a device not read by then is `unread`
   └─ first line not hello, or another protocol ──► error `protocol`, connection closed
close ──► requests still queued for this connection are dropped; a `gimbal.turn` it started
          without `keep: true` is stopped
```

A device entry:

```text
plugged ──► scan finds it ──► first read (identity attempt, every group) ──► listed, rev 1
   ▲                                                                          │
   │      serial answered later: `port:` entry handed over to `sn:`            │
   │      (`replaced_by`, two `devices` events, then a `state` event)          │
   │                                                                          ▼
re-plug ◄── unplugged: `sn:` stays (available:false), `port:` is removed ◄── polled by tier
```

A `set`:

```text
queued ──► validated whole (invalid, unsupported, out_of_range, conflict, asleep: nothing sent)
   └─► leaves written in apply order ──► touched groups re-read once per batch
         └─► state event (written values the readback does not show yet are `pending`)
               └─► reply {rev}  ──►  later reads clear `pending` (or `settle_ms` passes)
```

## 12. The client library

`obsbot-proto` holds the synchronous client (design spec 6.1). `Client::connect` opens a
connection with `subscribe: false` and blocks per request; `Client::subscribe` returns the
`hello` reply, a `Sender` and a `Receiver`, whose reader thread delivers replies and events in
arrival order. There is no retry: when a connection ends, the caller connects again and
rebuilds from the new `hello` reply. This is `crates/obsbot-proto/examples/watch.rs`; the
test of this file checks that the two are the same:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The client library in a few lines (design spec 6.1, `docs/protocol-2.md` section 12): list
//! the devices once with the blocking client, then print every state event of a subscribed
//! connection. It sends no write. Run it with `cargo run -p obsbot-proto --example watch`.

use obsbot_proto::client::{Client, ClientError, Incoming};
use obsbot_proto::{Event, Hello, short_id};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A blocking client: one request at a time, no events (subscribe: false).
    let mut client = Client::connect(Hello::new("example 1.0"))?;
    for device in client.devices().to_vec() {
        // `{:?}` quotes the short id and escapes any control character it holds: an id comes
        // from the camera, and what a terminal prints is not for the camera to choose.
        println!(
            "{:?}  {}  {:?}",
            short_id(&device.id),
            device.model,
            device.support
        );
        if device.available && !device.description.settings.is_empty() {
            match client.get(&device.id, Some(&["zoom.factor"])) {
                Ok((rev, state)) => println!("  rev {rev}: zoom {:?}", state.at("zoom.factor")),
                // A model without the leaf refuses with a code; that device ends nothing.
                Err(ClientError::Refused(body)) => {
                    println!("  zoom.factor: {}", body.code.as_str())
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    // A subscribed client: the hello reply, a sender and a receiver (15.1 row 96).
    let (hello, _sender, receiver) = Client::subscribe(Hello::new("example 1.0"))?;
    println!("daemon {} speaks protocol {}", hello.daemon, hello.protocol);
    for incoming in receiver {
        if let Incoming::Event(Event::State { device, rev, state }) = incoming? {
            println!(
                "{:?} rev {rev}: pending {:?}",
                short_id(&device),
                state.pending
            );
        }
    }
    Ok(())
}
```
````

- [ ] **Step 4: Re-record the transcripts against the landed daemon**

The lines of section 11 were recorded from the daemon of the scratch build. Record them again from the landed daemon, in the task's build directory (`$TMPDIR` as in Global Constraints), with this throw-away client (not committed; save it as `$TMPDIR/record.py`):

```python
#!/usr/bin/env python3
# Records the transcripts of docs/protocol-2.md section 11 from a running fake daemon.
# usage: record.py <socket> pending   (obsbot4linuxd --fake fake-b,unknown)
#        record.py <socket> multi     (obsbot4linuxd --fake fake-b,unknown)
# Prints "> " request lines and "< " daemon lines as sent and received; `multi` also prints
# "# test control" lines for what it sends to <socket>.test.
import json
import select
import socket
import sys
import time

path, mode = sys.argv[1], sys.argv[2]
conn = socket.socket(socket.AF_UNIX)
conn.connect(path)
buf = b""


def send(obj):
    line = json.dumps(obj, separators=(",", ":"))
    print("> " + line, flush=True)
    conn.sendall((line + "\n").encode())


def lines(timeout):
    global buf
    end = time.time() + timeout
    while True:
        while b"\n" in buf:
            line, buf = buf.split(b"\n", 1)
            yield line.decode()
        left = end - time.time()
        if left <= 0:
            return
        if not select.select([conn], [], [], left)[0]:
            return
        data = conn.recv(65536)
        if not data:
            return
        buf += data


def recv(until_id=None, wait=5.0):
    for line in lines(wait):
        print("< " + line, flush=True)
        if until_id is not None and json.loads(line).get("id") == until_id:
            return


def control(obj):
    c = socket.socket(socket.AF_UNIX)
    c.connect(path + ".test")
    c.sendall((json.dumps(obj) + "\n").encode())
    reply = c.recv(4096).decode().strip()
    c.close()
    print(
        "# test control: " + json.dumps(obj, separators=(",", ":")) + " -> " + reply,
        flush=True,
    )


B = "sn:FAKEB0000001"
send(
    {
        "id": 1,
        "hello": {
            "protocol": 2,
            "client": "example 1.0",
            "subscribe": True,
            "live": [],
        },
    }
)
recv(1)
if mode == "pending":
    send({"id": 2, "device": B, "set": {"led.brightness": 3}})
    recv(2)
    recv(None, wait=3)
    send({"id": 3, "device": B, "set": {"exposure.mode": "manual"}})
    send({"id": 4, "device": B, "set": {"exposure.time": 500}})
    recv(4)
    send({"id": 5, "device": B, "set": {"exposure.time": 20000}})
    recv(5)
else:
    send({"id": 2, "device": "port:9-2", "do": "wake"})
    recv(2)
    control({"unplug": B})
    recv(None, wait=1.5)
    send({"id": 3, "device": B, "set": {"led.brightness": 1}})
    recv(3)
    control({"plug": "fake-b"})
    recv(None, wait=2)
```

```bash
nice -n 19 cargo build -j 2 --bins
D="$CARGO_TARGET_DIR"/debug/obsbot4linuxd
S=$TMPDIR/p.sock; rm -f "$S"
OBSBOT4LINUX_SOCKET=$S timeout 60 "$D" --fake fake-b,unknown > "$TMPDIR"/p.log 2>&1 &
P=$!; sleep 2
python3 -I "$TMPDIR"/record.py "$S" pending | grep -v '"id":1,"ok"' > "$TMPDIR"/pending.txt
kill $P
S=$TMPDIR/m.sock; rm -f "$S"
OBSBOT4LINUX_SOCKET=$S timeout 60 "$D" --fake fake-b,unknown > "$TMPDIR"/m.log 2>&1 &
P=$!; sleep 2
python3 -I "$TMPDIR"/record.py "$S" multi > "$TMPDIR"/multi.txt
kill $P
```

Both recordings run on `fake-b` and the unsupported device alone, so that no event of another fake device can enter the subscribed recording and the number of lines does not depend on one. Expected: the 12 lines of `pending.txt` equal the 12 `> `/`< ` lines of the `jsonl` block of 11.1, in order, except that the two pipelined `set`s may come as two batches (13 lines: a state event with rev 4, the reply `{"id":3,"ok":{"rev":4}}`, a state event with rev 5, `{"id":4,"ok":{"rev":5}}`; the recording of the scratch build came out so): that is timing, and the doc says so; keep the doc's one-batch lines. `multi.txt` shows the order of 11.2: the `unsupported_device` refusal, after the unplug a `devices` event then a `state` event with `available:false` and rev 2, the `unavailable` refusal, after the plug a `devices` event then a `state` event with rev 3. If the landed daemon adds a field or changes a message text, take the recorded line into the doc (descriptions and states elided as in 11.2); if an event comes after its reply, or `pending` does not appear in 11.1, stop and report `plan_mismatch` (a daemon defect against 4.1).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot-proto -j 2 --test doc_examples` and `nice -n 19 cargo build -j 2 -p obsbot-proto --example watch`
Expected: `ok. 2 passed`; the example builds. Then check that the test can fail: swap the lines `< {"event":"state",…,"rev":2,…}` and `< {"id":2,"ok":{"rev":2}}` in the doc, run again: `no state event with rev 2 before the reply`; swap them back. Then the workspace gate. Note for this and every later pull request that edits `docs/protocol-2.md`: `ci.yml` skips fmt, clippy and test for a change of docs only (review focus 5), so the reviewer runs `cargo test -p obsbot-proto --test doc_examples` by hand when the pull request touches nothing else.

- [ ] **Step 6: Commit**

```bash
git add docs/protocol-2.md crates/obsbot-proto/examples/watch.rs crates/obsbot-proto/tests/doc_examples.rs
git commit -m "docs: protocol 2 worked transcripts, lifecycles and the client library, checked by a test"
```

---

### Task w2-4 (key `w2-4-migration-notes`): Migration notes drafted

**Files:**

- Create: `docs/migration-1.0.md`

**Interfaces:** none (W3 moves the text into `CHANGELOG.md`; W4 rehearses it, CAM-28).

- [ ] **Step 1: Write the notes**

Create `docs/migration-1.0.md`:

```markdown
# Moving from obsbot4linux 0.13 to 1.0

Draft of the migration notes that the 1.0 release notes carry (design spec 12.2). obsbot4linux
1.0 replaces the 0.13 daemon, its protocol, the command line and the widget's channel. Nothing
set on the camera is lost: the camera stores its settings, and 1.0 reads every value from it.

## Upgrade steps

1. Install obsbot4linux 1.0 and update the widget to 1.0 together with it.
2. `systemctl --user daemon-reload`
3. End the 0.13 daemon. Either `systemctl --user stop obsbot4linux.service`, or end the
   `obsbot4linux daemon` process that `obsbot4linux watch` started (the widget starts it), or
   log out.
4. Remove drop-ins whose `ExecStart` runs `obsbot4linux daemon`
   (`systemctl --user cat obsbot4linux.service` lists them).
5. `systemctl --user enable --now obsbot4linux.socket`
6. Reload the shell that hosts the widget.

Setups that never enabled the socket (the default for widget users) have no daemon otherwise:
1.0 clients no longer start one. A command that cannot reach the daemon exits with 4 and prints
the `systemctl` line of step 5.

`$XDG_STATE_HOME/obsbot4linux/` (it held `state.json` and `daemon.log`) is no longer used and
can be deleted. Nothing is imported from it.

## What behaves differently

- **Gimbal moves leave the tracking alone.** The 0.13 commands `gimbal [--pan P] [--tilt T]`
  and `gimbal rate` switched the tracking to static before they moved. `do gimbal.move` and
  `do gimbal.turn` send only the move and leave the tracking as it is. A script that relied on
  the switch runs `obsbot4linux set tracking.mode=static` first, as its own step.
- **Presets saved without a name get no name.** The 0.13 command `preset save N` without a
  name saved the preset as `Preset <N + 1>`. `do preset.save slot=N` sends no name: what the
  camera shows for such a preset is what `get presets` shows. A script that wants the old
  names passes `name=…` itself.
- **One generic command line.** Every setting is a path (`set zoom.factor=2`), every action a
  name (`do preset.recall slot=0`); `describe` lists what a camera offers. Paths use `_`;
  enum values may be typed with `-` or `_`.
- **`--json`** prints the protocol objects, and errors as `{"error":{"code","message"}}`.
- **Exit codes:** 0 ok; 1 internal error; 2 invalid input; 3 the device cannot do it now (or
  there is no usable camera); 4 the daemon cannot be reached.
- **Exposure time** is in microseconds (0.13 used steps of 100 µs): multiply old values by 100.

## Command map

The left column is the 0.13 command, the right one the 1.0 command (`obsbot4linux` left out).
Commands not listed are gone; the next section says why.

| 0.13                                                                    | 1.0                                                                  | Note                                                                 |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `--json`                                                                | `--json`                                                             | protocol objects                                                     |
| `--help`, `--version`                                                   | `--help`, `--version`                                                |                                                                      |
| `daemon`                                                                | `obsbot4linuxd`, started by `obsbot4linux.socket`                    | a separate program                                                   |
| `watch`                                                                 | `watch`                                                              | prints events only                                                   |
| `live-pose on\|off`                                                     | `watch --live pose`                                                  |                                                                      |
| `detect`, `detect --json`                                               | `devices`, `devices --json`                                          | needs the daemon                                                     |
| `status`, `status --json`                                               | `status`, `status --json`                                            | new layout                                                           |
| `sleep`                                                                 | `do sleep`                                                           |                                                                      |
| `wake`                                                                  | `do wake`                                                            | replies once the camera accepted it; `status` shows when it is awake |
| `wake zoom F`                                                           | `set sleep.wake_zoom=F`                                              |                                                                      |
| `wake mode M`                                                           | `set power_on.mode=M`                                                |                                                                      |
| `tracking MODE`                                                         | `set tracking.mode=MODE`                                             |                                                                      |
| `speed standard\|sport\|headroom`                                       | `set tracking.profile=…`                                             |                                                                      |
| `speed-mode MODE`                                                       | `set tracking.speed=MODE`                                            |                                                                      |
| `framing auto-zoom on\|off`                                             | `set framing.auto_zoom=…`                                            |                                                                      |
| `framing size S`                                                        | `set framing.size=S`                                                 |                                                                      |
| `framing zoom-speed S`                                                  | `set framing.zoom_speed=S`                                           |                                                                      |
| `framing offset [--x X] [--y Y]`                                        | `set [framing.offset.x=X] [framing.offset.y=Y]`                      |                                                                      |
| `framing part P`                                                        | `set framing.part=P`                                                 |                                                                      |
| `hdr on\|off`                                                           | `set exposure.hdr=…`                                                 |                                                                      |
| `exposure global\|face`                                                 | `set exposure.mode=auto exposure.region=…`                           |                                                                      |
| `exposure manual`                                                       | `set exposure.mode=manual`                                           |                                                                      |
| `exposure manual --time T`                                              | `set exposure.mode=manual exposure.time=<T×100>`                     | microseconds                                                         |
| `exposure manual --gain G`                                              | `set exposure.mode=manual exposure.gain=G`                           |                                                                      |
| `zoom F`                                                                | `set zoom.factor=F`                                                  |                                                                      |
| `zoom F --speed S`                                                      | `do zoom.move factor=F speed=slow\|medium\|fast`                     | 10, 30, 50; other speeds are gone                                    |
| `zoom stop`                                                             | `do zoom.stop`                                                       |                                                                      |
| `gimbal [--pan P] [--tilt T]`                                           | `do gimbal.move [pan=P] [tilt=T]`                                    | a missing axis stays; the tracking stays as it is                    |
| `gimbal … --speed S`                                                    | `do gimbal.move … speed=S`                                           | the tracking stays as it is                                          |
| `gimbal stop`                                                           | `do gimbal.stop`                                                     |                                                                      |
| `gimbal rate [--pan P] [--tilt T]`                                      | `do gimbal.turn [pan_speed=P] [tilt_speed=T]`                        | turns until `do gimbal.stop`; the tracking stays as it is            |
| `fov wide\|medium\|narrow`                                              | `set zoom.field_of_view=…`                                           |                                                                      |
| `led N`                                                                 | `set led.brightness=N`                                               |                                                                      |
| `led blink on\|off`                                                     | `set led.blink=…`                                                    | the camera does not report it                                        |
| `mirror on\|off`                                                        | `set image.mirror=…`                                                 |                                                                      |
| `invert on\|off`                                                        | `set gimbal.invert=…`                                                |                                                                      |
| `auto-sleep off`                                                        | `set sleep.auto.enabled=off`                                         |                                                                      |
| `auto-sleep N` (N ≥ 1)                                                  | `set sleep.auto.enabled=on sleep.auto.seconds=N`                     |                                                                      |
| `sleep-background black\|image`                                         | `set sleep.background=…`                                             |                                                                      |
| `sleep-background-mirror on\|off`                                       | `set sleep.background_mirror=…`                                      |                                                                      |
| `audio noise L`                                                         | `set audio.noise_reduction=L`                                        |                                                                      |
| `audio gain on\|off`                                                    | `set audio.auto_gain=…`                                              |                                                                      |
| `audio distance near\|standard\|far`                                    | `set audio.volume=10\|50\|100`                                       | the pickup distance is the volume in bands                           |
| `audio volume N`                                                        | `set audio.volume=N`                                                 |                                                                      |
| `audio sleep-mic on\|off`                                               | `set audio.in_sleep=…`                                               |                                                                      |
| `audio mic on\|off`                                                     | `set audio.enabled=…`                                                | the camera restarts                                                  |
| `gesture set target-lock\|zoom\|dynamic-zoom\|direction-change on\|off` | `set gestures.<name>=…`                                              | names with `_`                                                       |
| `gesture zoom-ratio R`                                                  | `set gestures.zoom_ratio=R`                                          |                                                                      |
| `hand right\|left`                                                      | `set hand.side=…`                                                    |                                                                      |
| `hand return-here`                                                      | `do hand.return_here`                                                | the current pose                                                     |
| `hand return-go`                                                        | `do hand.go_return`                                                  |                                                                      |
| `hand return-reset`                                                     | `do hand.return_clear`                                               |                                                                      |
| `hand area --pan MIN MAX --tilt MIN MAX`                                | `set hand.area.pan=MIN,MAX hand.area.tilt=MIN,MAX`                   |                                                                      |
| `hand area-reset`                                                       | `do hand.area_reset`                                                 |                                                                      |
| `hand pan\|tilt on\|off`                                                | `set hand.axes.pan=…` / `set hand.axes.tilt=…`                       |                                                                      |
| `hand return-after S`                                                   | `set hand.return_after=S`                                            |                                                                      |
| `hand gimbal on\|off`                                                   | `set hand.move_gimbal=…`                                             | the camera does not report it                                        |
| `zone on\|off`                                                          | `set zone.enabled=…`                                                 |                                                                      |
| `zone follow on\|off`                                                   | `set zone.follow=…`                                                  |                                                                      |
| `zone area --pan MIN MAX --tilt MIN MAX`                                | `set zone.area.pan=MIN,MAX zone.area.tilt=MIN,MAX`                   |                                                                      |
| `zone return-here`                                                      | `do zone.return_here`                                                | the current pose                                                     |
| `zone return-go`                                                        | `do zone.go_return`                                                  |                                                                      |
| `zone view list`                                                        | `get views`                                                          |                                                                      |
| `zone view save NAME`                                                   | `do view.save name=NAME`                                             | the current pose                                                     |
| `zone view save NAME --pan P --tilt T --zoom Z`                         | `do view.save name=NAME pose=P,T,Z`                                  |                                                                      |
| `zone view rename ID NAME`                                              | `do view.rename id=ID name=NAME`                                     |                                                                      |
| `zone view delete ID`                                                   | `do view.delete id=ID`                                               |                                                                      |
| `zone view go ID`                                                       | `do view.go id=ID`                                                   |                                                                      |
| `focus mode face\|global`                                               | `set focus.region=…`                                                 |                                                                      |
| `focus auto on`                                                         | `set focus.mode=continuous`                                          |                                                                      |
| `focus auto off`                                                        | `set focus.mode=manual`                                              | final mapping checked on the camera before the release               |
| `focus af continuous\|single\|manual`                                   | `set focus.mode=…`                                                   |                                                                      |
| `focus point X Y`                                                       | `do focus.point x=X y=Y`                                             |                                                                      |
| `focus set N`                                                           | `set focus.mode=manual focus.position=N`                             |                                                                      |
| `startup set --pan P --tilt T --zoom Z`                                 | `do power_on.set_pose pose=P,T,Z`                                    |                                                                      |
| `startup set --preset N`                                                | `do power_on.use_preset slot=N`                                      |                                                                      |
| `startup set --current`                                                 | `do power_on.use_current`                                            | the current pose                                                     |
| `startup set --ai M\|none`                                              | `set power_on.start.mode=M\|unset`                                   |                                                                      |
| `startup set <pose option> --ai M`                                      | the `do` of the pose option, then `set power_on.start.mode=M`        | two commands                                                         |
| `startup show`                                                          | `get power_on`                                                       |                                                                      |
| `startup go`                                                            | `do power_on.go`                                                     |                                                                      |
| `image brightness\|contrast\|saturation\|hue\|sharpness V`              | `set image.<name>=V`                                                 |                                                                      |
| `image white_balance_auto 0\|1`                                         | `set image.white_balance.auto=off\|on`                               |                                                                      |
| `image white_balance_temperature V`                                     | `set image.white_balance.auto=off image.white_balance.temperature=V` | the temperature needs the automatic white balance off                |
| `image gain V`                                                          | `set exposure.gain=V`                                                |                                                                      |
| `image power_line_frequency 0\|1\|2`                                    | `set exposure.anti_flicker=off\|50hz\|60hz`                          |                                                                      |
| `image backlight_compensation V`                                        | `set exposure.compensation=V`                                        |                                                                      |
| `image auto_exposure 0\|1\|3`                                           | `set exposure.mode=auto\|manual\|auto`                               |                                                                      |
| `image exposure_time V`                                                 | `set exposure.mode=manual exposure.time=<V×100>`                     | microseconds                                                         |
| `image focus V`                                                         | `set focus.mode=manual focus.position=V`                             |                                                                      |
| `image autofocus 0\|1`                                                  | `set focus.mode=manual\|continuous`                                  | as `focus auto`                                                      |
| `preset list`                                                           | `get presets`                                                        |                                                                      |
| `preset save N`                                                         | `do preset.save slot=N`                                              | the current pose; no name                                            |
| `preset save N --name S`                                                | `do preset.save slot=N name=S`                                       |                                                                      |
| `preset save N --pan P --tilt T --zoom Z`                               | `do preset.save slot=N pose=P,T,Z`                                   |                                                                      |
| `preset rename N S`                                                     | `do preset.rename slot=N name=S`                                     |                                                                      |
| `preset delete N`                                                       | `do preset.delete slot=N`                                            |                                                                      |
| `preset recall N`                                                       | `do preset.recall slot=N`                                            |                                                                      |
| `factory-reset --yes`                                                   | `do factory_reset confirm=true`                                      | the camera restarts                                                  |
| `factory-reset` without `--yes`                                         | `do factory_reset` is refused                                        |                                                                      |

## What is gone, and why

- `gimbal center X Y` (hidden): it moved the gimbal only toward the left and downward.
- `image gamma`: the camera has no gamma control; the 0.13 command always failed.
- `--direct`: the command line never talks to the camera; socket activation starts the daemon.
- Requests sent to `watch` on stdin: the widget speaks the protocol on the socket.
- `detect` without a daemon: `devices` needs the daemon. It exits 0 when it lists a camera and
  3 when the list is empty; a script that needs a usable camera reads `support` from
  `devices --json`, or runs `status`, which exits 3 without one.
- `zoom F --speed S` with any number S: only `slow`, `medium` and `fast`.
- `auto-sleep 0`: the camera cannot store "off" together with 0 s.

Rows that the 1.0 release could not confirm on the camera are named in the release notes,
with the 0.13 command they affect.
```

- [ ] **Step 2: Check them against the spec**

Compare, row by row, the command map with spec 9.1 and the "What is gone" list with 9.2 (PX-1 to PX-7): every 0.13 command of 9.1 has a row or a line, and every 1.0 command is the one 9.1 names. Compare "Upgrade steps" and "What behaves differently" with every bullet of 12.2. Run `prettier --check docs/migration-1.0.md` if prettier is installed.
Expected: nothing missing; the only differences are the spec's camera and U references, which the notes leave out (decision 15).

- [ ] **Step 3: Commit**

```bash
git add docs/migration-1.0.md
git commit -m "docs: migration notes from 0.13 to 1.0, drafted for the release notes"
```

---

### Task w2-5 (key `w2-5-capture-evidence`): Recorded status blocks as fixtures; ledger notes towards CS-1

**Files:**

- Create: `crates/obsbot-device/tests/capture_evidence.rs`
- Modify: `docs/field-ledger.md` (a new last section)

**Interfaces:**

- Consumes: `obsbot_device::models::tiny2lite::decode::status`, the test support's `block`.
- Produces: the fixtures w2-7 changes once CS-1 has answered CAM-20.

The bytes below were taken by the planner from the recordings with `tshark` (GET_CUR answers of extension unit 2, selector 6, 60 bytes; times relative to the start of each recording). The builder copies them; nothing else is taken from the recordings.

- [ ] **Step 1: Write the tests**

Create `crates/obsbot-device/tests/capture_evidence.rs`:

```rust
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

/// CAM-20 in part: after every enumeration recorded on the owner's device (capture 01 at
/// plug-in, capture 14 after both microphone restarts, capture 16 at plug-in and after the
/// factory reset) a status read 0.8 to 4.5 s after the enumeration returned all-zero blocks
/// (after the second restart a full block was read 1.7 s before them); the next read, about
/// 4 s later, was a full block, and the serial query sent in between was answered with 14
/// characters. The decoder turns such a block into values that
/// look real (`sleep.auto.seconds` 0, `image.frame_rate` 0). CS-1 decides whether a block whose
/// first byte is 0 counts as no read (spec 14, CAM-20); until then this test pins today's
/// decoding, and the pull request of that decision changes it.
#[test]
fn a_boot_block_decodes_as_values_until_cam_20_is_settled() {
    let values = decode::status(&[0u8; 60]);
    assert_eq!(values["asleep"], json!(false));
    assert_eq!(values["sleep.auto.seconds"], json!(0));
    assert_eq!(values["image.frame_rate"], json!(0));
    assert_eq!(values["zoom.field_of_view"], json!("wide"));
}
```

- [ ] **Step 2: Run them**

Run: `nice -n 19 cargo test -p obsbot-device -j 2 --test capture_evidence`
Expected: `ok. 3 passed`. They pass at once: they pin what the 1.0 decoder does with recorded blocks. Check that each can fail: change `ASLEEP_13`'s byte 0x02 (`01` → `00`, the third byte) and the first test fails; change `MIC_ON_14`'s `10` at byte 0x22 to `00` and the second fails at `differ`; both changes undone.

- [ ] **Step 3: Write the ledger notes**

Append to `docs/field-ledger.md`:

```markdown
## Recorded evidence towards camera session 1 (spec §14)

What the status blocks and queries recorded on the owner's device already show about four
camera items. It narrows each item; none is answered by it, and camera session 1 runs every
one. The blocks are fixtures of `crates/obsbot-device/tests/capture_evidence.rs`.

| Item        | Recorded                                                                                                                                                                                                                                                                                      | Captures   | Still for CS-1                                                                                       |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------- |
| CAM-1       | the serial query `0x18c8` is answered with 14 characters within 75 ms each time it is sent: at plug-in, after both microphone restarts and after the factory reset, 2.7 to 6.6 s after the enumeration                                                                                        | 01, 14, 16 | the query sent from Linux; asleep; right after plug-in                                               |
| CAM-3       | awake at plug-in (the first full block reads `status 0x02` = 0); the first block read after the sleep command shows asleep (5.2 s after it), the first after the wake command awake (3.3 s); nothing was read in between, so both times are upper bounds                                      | 01, 13     | the time from wake to awake, from Linux; `zoom.factor` while asleep                                  |
| CAM-8, U-23 | `status 0x22` reads `0x10` while the microphone is on and `0x00` while it is off; it changes with the command `short 1c 01 <v>`, before the restart. No leaf reads it today                                                                                                                   | 14         | a cold plug with the microphone off; whether the audio interface is absent then                      |
| CAM-20      | a status read 0.8 to 4.5 s after each enumeration returns all-zero blocks (after the second microphone restart a full block came 1.7 s before them); the next read, about 4 s later, is a full block. The serial query is answered in between. The decoder reads a zero block as values today | 01, 14, 16 | the same from Linux; then whether a block whose first byte is 0 counts as no read (the model change) |
```

Run: `prettier --check docs/field-ledger.md` (if installed; `prettier --write` aligns the table).

- [ ] **Step 4: Gate and commit**

Run the workspace gate, then:

```bash
git add crates/obsbot-device/tests/capture_evidence.rs docs/field-ledger.md
git commit -m "tiny2lite: recorded status blocks around sleep, the microphone restart and plug-in as fixtures"
```

---

### Task w2-6 (key `w2-6-replay-hookup`): Command line, daemon and `tiny2lite` end to end on a recorded camera

**Files:**

- Create: `crates/obsbot4linux/tests/replay.rs`
- Modify: `crates/obsbot4linux/Cargo.toml` (`[dev-dependencies]`: `hex.workspace = true` above `obsbot-device`)
- Modify: `Cargo.lock` (generated: `hex` in the dependencies of `obsbot4linux`)

**Interfaces:**

- Consumes (w2-1): `obsbot4linuxd::run_on`; (track A) `Bus`, `Camera`, `Found`, `Frame`, `camera::Result`, `tiny2lite::plan::ENTRY`, `Replay`, `Log`, the test support module; (track C) `common::{socket, cli}` and the commands.
- Produces: nothing other code uses.

- [ ] **Step 1: Add the dev-dependency**

In `crates/obsbot4linux/Cargo.toml`:

```toml
[dev-dependencies]
hex.workspace = true
obsbot-device.workspace = true
obsbot4linuxd.workspace = true
```

Run: `nice -n 19 cargo build -j 2 --workspace` (without `--locked`, once: it writes `hex` into `Cargo.lock`), then `scripts/check-cli-deps.sh`.
Expected: `Cargo.lock` changes by one line; the check prints `CLI dependencies: clap nix obsbot-proto serde serde_json` (dev-dependencies are not listed).

- [ ] **Step 2: Write the tests**

Create `crates/obsbot4linux/tests/replay.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Stage W2 hook-up (design spec 10.3 W2): the command line, the daemon and the Tiny 2 Lite
//! model end to end, on a camera recorded on the owner's device. The model runs on the replay
//! transport of track A with the recorded answers of `obsbot-device`'s test support, behind a
//! bus that lists it once; nothing opens a video node or any other device file.
//!
//! What a request sends is compared with the frames recorded on the owner's device: a request
//! sends exactly what it names and nothing before or after it (spec 2).

mod common;
#[path = "../../obsbot-device/tests/support/mod.rs"]
mod support;

use obsbot_device::contract::camera::Result as CameraResult;
use obsbot_device::contract::{Bus, Camera, CameraError, Found, Frame};
use obsbot_device::models::tiny2lite;
use obsbot_device::transport::replay::{Log, Replay};
use obsbot4linuxd::{Config, Timing};
use serde_json::{Value, json};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// The id of the recorded camera: `sn:` and the synthetic serial of the test support.
const ID: &str = "sn:EXAMPLE0000001";

/// A bus with one Tiny 2 Lite on the replay transport. It lists the camera on every scan and
/// opens it once; the camera is the model behind the send-table check, as on the real bus. A
/// second open (a retry or a reopen after an error) is refused with an error and counted: the
/// recorded answers are consumed by the first, and `Hookup` fails the test with the count
/// instead of the daemon dying on a panic in a thread.
struct ReplayBus {
    replay: Mutex<Option<Replay>>,
    opens: Arc<AtomicUsize>,
}

impl Bus for ReplayBus {
    fn scan(&self) -> Vec<Found> {
        vec![Found {
            usb: support::usb(),
            devnum: 4,
            video_node: None,
            model: Some(&tiny2lite::plan::ENTRY),
        }]
    }

    fn open(&self, _found: &Found) -> CameraResult<Box<dyn Camera>> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        let replay = self
            .replay
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .ok_or_else(|| {
                CameraError::Io("the recorded camera answers one open, not two".into())
            })?;
        let (camera, _) = support::model(replay);
        Ok(Box::new(camera))
    }
}

/// The daemon on the replay bus, and the log of what reached the recorded camera.
struct Hookup {
    socket: PathBuf,
    log: Log,
    opens: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<std::io::Result<()>>>,
}

impl Hookup {
    /// Starts the daemon with the spec's intervals and waits (at most 20 s) until it accepts.
    fn start(name: &str) -> Hookup {
        let socket = common::socket(&format!("r-{name}"));
        let _ = std::fs::remove_file(&socket);
        let replay = support::recorded();
        let log = replay.log();
        let opens = Arc::new(AtomicUsize::new(0));
        let bus: Arc<dyn Bus> = Arc::new(ReplayBus {
            replay: Mutex::new(Some(replay)),
            opens: Arc::clone(&opens),
        });
        let config = Config::new(socket.clone());
        let stop = Arc::clone(&config.stop);
        let thread = thread::spawn(move || obsbot4linuxd::run_on(config, Timing::SPEC, bus));
        let until = Instant::now() + Duration::from_secs(20);
        while UnixStream::connect(&socket).is_err() {
            if thread.is_finished() {
                panic!("the daemon ended: {:?}", thread.join());
            }
            assert!(Instant::now() < until, "the daemon does not accept");
            thread::sleep(Duration::from_millis(20));
        }
        Hookup {
            socket,
            log,
            opens,
            stop,
            thread: Some(thread),
        }
    }

    fn cli(&self, args: &[&str]) -> (u8, String, String) {
        common::cli(Some(&self.socket), args)
    }

    /// What reached the camera since the last `clear`: commands, buffer writes and UVC
    /// writes in order; queries (reads) left out.
    fn writes(&self) -> Vec<Frame> {
        support::commands(&self.log)
    }

    fn payloads(&self) -> Vec<Vec<u8>> {
        support::payloads(&self.log)
    }
}

impl Drop for Hookup {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let ended = thread.join();
            if !thread::panicking() {
                assert!(matches!(ended, Ok(Ok(()))), "run returned {ended:?}");
                let opens = self.opens.load(Ordering::SeqCst);
                assert_eq!(
                    opens, 1,
                    "the daemon opened the recorded camera {opens} times: a reopen needs a \
                     recording that answers it"
                );
            }
        }
        // the daemon keeps its `<socket>.lock` (5.9); the test's daemon is gone, so is it
        let mut lock = self.socket.clone().into_os_string();
        lock.push(".lock");
        let _ = std::fs::remove_file(lock);
    }
}

fn parse(out: &str) -> Value {
    serde_json::from_str(out.trim_end()).unwrap_or_else(|e| panic!("{e}: {out}"))
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// The recorded camera is listed with the model's identity and its USB facts, and every
/// value the command line shows was decoded by the model from a recorded answer.
#[test]
fn the_recorded_camera_is_listed_and_read_through_the_daemon() {
    let h = Hookup::start("list");
    let (code, out, err) = h.cli(&["devices", "--json"]);
    assert_eq!(code, 0, "{err}");
    let devices = parse(&out)["devices"].clone();
    assert_eq!(devices.as_array().map(Vec::len), Some(1), "{devices}");
    let d = &devices[0];
    assert_eq!(d["id"], ID, "the serial query answered at plug-in (5.5)");
    assert_eq!(d["model"], "tiny2lite");
    // Stage W3 sets the model to `tested` (spec 8.8); its pull request changes this line.
    assert_eq!(d["support"], "untested", "spec 8.8: untested until W3");
    assert_eq!(d["firmware"], "6.2.7.4", "capture 01");
    assert_eq!(d["usb"]["vendor"], "3564");
    assert_eq!(d["usb"]["product"], "fef9");
    assert_eq!(d["usb"]["serial_present"], false);

    let (code, out, err) = h.cli(&[
        "get",
        "--json",
        "zoom.factor",
        "audio.volume",
        "image.mirror",
        "tracking.mode",
        "asleep",
    ]);
    assert_eq!(code, 0, "{err}");
    let state = &parse(&out)["state"];
    assert_eq!(state["zoom"]["factor"], json!(1.6), "capture 04");
    assert_eq!(state["audio"]["volume"], json!(55), "measured 2026-10-31");
    assert_eq!(state["image"]["mirror"], json!(false), "the status block");
    assert_eq!(
        state["tracking"]["mode"],
        json!("static"),
        "the status block"
    );
    assert_eq!(state["asleep"], json!(false), "the status block");
}

/// Reads send no command. The one write of the first read is the rewind of the preset list
/// (spec 8.8: the one exception of section 2, until CAM-19); a `status` after it writes
/// nothing at all.
#[test]
fn reading_writes_only_the_preset_list_rewind() {
    let h = Hookup::start("reads");
    let (code, _, err) = h.cli(&["status"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(h.writes(), [Frame::Block { selector: 0x0c }]);
    h.log.clear();
    let (code, out, err) = h.cli(&["status"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("tiny2lite"), "{out}");
    // Stage W3 sets the model to `tested` (spec 8.8); its pull request changes this line.
    assert!(out.contains("untested"), "{out}");
    assert!(out.contains("…0001"), "the shortened id: {out}");
    assert_eq!(h.writes(), [], "a status reads and writes nothing");
}

/// `set image.mirror=on` reaches the camera as the short command recorded in capture 05,
/// `14 01 01`, and as nothing else.
#[test]
fn a_set_reaches_the_camera_as_its_recorded_frame_only() {
    let h = Hookup::start("set");
    let (code, _, err) = h.cli(&["get", "image.mirror"]);
    assert_eq!(code, 0, "{err}");
    h.log.clear();
    let (code, out, err) = h.cli(&["set", "image.mirror=on"]);
    assert_eq!((code, out.as_str()), (0, ""), "{err}");
    assert_eq!(h.writes(), [support::short(0x14)]);
    assert_eq!(h.payloads(), [vec![0x14, 0x01, 0x01]]);
}

/// `do gimbal.move` sends the one move frame and no tracking switch before it (spec 2, 15.1
/// row 88): the pan and tilt in the camera's angles, the roll kept.
#[test]
fn a_gimbal_move_reaches_the_camera_as_one_move_and_no_tracking_frame() {
    let h = Hookup::start("move");
    let (code, _, err) = h.cli(&["get", "tracking.mode"]);
    assert_eq!(code, 0, "{err}");
    h.log.clear();
    let (code, _, err) = h.cli(&["do", "gimbal.move", "pan=100", "tilt=-30"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(h.writes(), [support::framed(0x6444)]);
    assert_eq!(h.payloads(), [floats(&[-1000.0, 30.0, 100.0])]);
    let (_, out, _) = h.cli(&["get", "--json", "tracking.mode"]);
    assert_eq!(parse(&out)["state"]["tracking"]["mode"], json!("static"));
}

/// A value the description refuses is answered by the daemon's validator and reaches the
/// camera as nothing.
#[test]
fn a_refused_value_reaches_the_camera_as_nothing() {
    let h = Hookup::start("refused");
    let (code, _, err) = h.cli(&["get", "led.brightness"]);
    assert_eq!(code, 0, "{err}");
    h.log.clear();
    let (code, _, err) = h.cli(&["set", "led.brightness=9"]);
    assert_eq!(code, 2, "out_of_range: {err}");
    assert_eq!(h.writes(), []);
}
```

- [ ] **Step 3: Run them**

Run: `nice -n 19 cargo test -p obsbot4linux -j 2 --test replay`
Expected: `ok. 5 passed` in well under a second. Then 20 times in a row (`ok. 5 passed` each time). Check that the hook-up test can fail: in `a_gimbal_move_reaches_the_camera_as_one_move_and_no_tracking_frame` expect `floats(&[-1000.0, -30.0, 100.0])` (the tilt sign of the wire flipped): it fails with the recorded bytes; undo. And make `ReplayBus::open` count twice (`fetch_add(2, …)`): every test fails with `the daemon opened the recorded camera 2 times`; undo. A daemon that reopens the camera (an identity retry or a reopen after an error) is therefore a clear failure of the test that meets it, not a panic in a daemon thread.

**Notes for W3.** Two tests assert `untested` (spec 8.8: untested until W3): `the_recorded_camera_is_listed_and_read_through_the_daemon` (`support == "untested"` in `devices --json`) and `reading_writes_only_the_preset_list_rewind` (`untested` in the human `status`). The W3 pull request that sets the Tiny 2 Lite to `tested` (w3-8) changes those two lines (each is marked with a comment); it is a file outside W3's owner files, named here so that it is not found by a red CI. The `Drop` of `Hookup` (and of `check_model.rs`'s `Daemon`) removes the `<socket>.lock` the daemon keeps, so that no file is left behind once W3's w3-1 moves the test sockets from the target directory to `$TMPDIR` (cross-check 2026-11-24).
Then the workspace gate.

- [ ] **Step 4: Commit**

```bash
git add crates/obsbot4linux/tests/replay.rs crates/obsbot4linux/Cargo.toml Cargo.lock
git commit -m "obsbot4linux: end to end through the daemon and tiny2lite on a camera recorded on the owner's device"
```

---

### Task w2-7 (key `w2-7-boot-block`, gate `camera`): A status block that reads 0 is no read (CAM-20)

Issued only after CS-1 has answered CAM-20 with "yes, the first field reads 0 until the camera is ready". If CS-1 answers "no", this task is closed without a change and its tests stay as w2-5 left them; if CS-1 shows another not-ready pattern (capture 14 recorded one full block with byte 1 = 0 before the zero blocks after the second restart), the task is re-planned from the session's notes.

**Files:**

- Modify: `crates/obsbot-device/src/models/tiny2lite/read.rs` (`status_block`, the `"status"` arm)
- Modify: `crates/obsbot-device/src/models/tiny2lite/write.rs` (`sleep_word`)
- Modify: `crates/obsbot-device/tests/capture_evidence.rs` (the CAM-20 test)
- Modify: `docs/field-ledger.md` (the CAM-20 row of the evidence section: the CS-1 result and date)

**Interfaces:** `Camera::read("status")` returns `Err(CameraError::NoAnswer(_))` for a block whose byte 0 is 0; the daemon then lists the group's leaves in `unread` (b-07) and the model's memory keeps no auto-sleep word from it.

- [ ] **Step 1: Write the failing test**

In `crates/obsbot-device/tests/capture_evidence.rs`, replace the test `a_boot_block_decodes_as_values_until_cam_20_is_settled` and its doc comment with:

```rust
/// CAM-20: after every enumeration recorded on the owner's device (captures 01, 14 and 16) a
/// status read returned all-zero blocks, and camera session 1 measured the same from Linux.
/// Such a block is no read: the status group's leaves stay `unread`.
#[test]
fn a_boot_block_is_no_read() {
    let (mut cam, _) = model(recorded().buffer(6, [0u8; 60]));
    let read = cam.read("status", &Ctx::none());
    assert!(
        matches!(read, Err(CameraError::NoAnswer(_))),
        "a block that reads 0 is no read: {read:?}"
    );
    let (mut cam, _) = model(recorded());
    assert!(
        cam.read("status", &Ctx::none()).is_ok(),
        "a full block is read"
    );
}

/// An auto-sleep write takes the other half of its word from the status block when it has
/// none yet (8.8); on a booting camera it sends nothing instead of a word built from zeros.
#[test]
fn an_auto_sleep_write_is_not_built_on_a_boot_block() {
    let (mut cam, log) = model(recorded().buffer(6, [0u8; 60]));
    let r = cam.apply("sleep.auto.enabled", &json!(true), &Ctx::none());
    assert!(matches!(r, Err(CameraError::NoAnswer(_))), "{r:?}");
    assert_eq!(commands(&log), []);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `nice -n 19 cargo test -p obsbot-device -j 2 --test capture_evidence`
Expected: FAIL in `a_boot_block_is_no_read` (`a block that reads 0 is no read: Ok(…)`) and in `an_auto_sleep_write_is_not_built_on_a_boot_block` (the write succeeds).

- [ ] **Step 3: Write the code**

In `crates/obsbot-device/src/models/tiny2lite/read.rs`, the `"status"` arm becomes:

```rust
/// The status block, or no read while the camera boots: the block then reads 0 (CAM-20:
/// captures 01, 14 and 16; measured in camera session 1).
pub(crate) fn status_block(link: &mut Link, ctx: &Ctx) -> Result<[u8; 60]> {
    let block = link.read(SHORT, ctx)?;
    if block[0] == 0 {
        return Err(CameraError::NoAnswer(
            "the status block reads 0: the camera is not ready (CAM-20)".into(),
        ));
    }
    Ok(block)
}

// in `read`:
        "status" => {
            let block = status_block(link, ctx)?;
            let values = decode::status(&block);
            memory.mode = values.get("tracking.mode").cloned();
            memory.sleep_word = Some(i16::from_le_bytes([block[0x0a], block[0x0b]]));
            Ok(values)
        }
```

and in `crates/obsbot-device/src/models/tiny2lite/write.rs`, `sleep_word` takes the block through the same function:

```diff
     if let Some(w) = memory.sleep_word {
         return Ok(w);
     }
-    let block = link.read(SHORT, ctx)?;
+    let block = crate::models::tiny2lite::read::status_block(link, ctx)?;
     let w = i16::from_le_bytes([block[0x0a], block[0x0b]]);
     memory.sleep_word = Some(w);
     Ok(w)
```

In `capture_evidence.rs`, `use support::block;` becomes `use obsbot_device::contract::{Camera, CameraError, Ctx};` and `use support::{block, commands, model, recorded};`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `nice -n 19 cargo test -p obsbot-device -j 2` and the workspace gate.
Expected: PASS; `tiny2lite_read` and `conformance` unchanged (the recorded `STATUS` of the support module starts with `27`).

- [ ] **Step 5: Ledger and commit**

In the evidence section of `docs/field-ledger.md`, the CAM-20 row's last column becomes `measured <date of CS-1>: …` with what the session recorded, and the row "Still for CS-1" text is replaced by "settled (w2-7)".

```bash
git add crates/obsbot-device/src/models/tiny2lite/read.rs crates/obsbot-device/src/models/tiny2lite/write.rs crates/obsbot-device/tests/capture_evidence.rs docs/field-ledger.md
git commit -m "tiny2lite: a status block that reads 0 while the camera boots is no read (CAM-20)"
```

---

### Task w2-8 (key `w2-8-cs1-results`, gate `camera`): CS-1 results into ledger, description and completions

Issued after camera session CS-1 (spec 10.4) with its results file and notes. Its content is the camera's answer, so this task is a procedure; every step names the file, the check and the command. It is the W2 acceptance line "no U-row that CS-1 failed is left in `descriptions/tiny2lite.json`" and track A's "Results go into the ledger and arrive as contract-change PRs (U-rows removed, `settle_ms`, `asleep` values, `focus auto off` mapping)".

**Files (as the results need):**

- Modify: `crates/obsbot-device/src/models/tiny2lite/description.rs` (`UNBACKED` and the fields), `crates/obsbot-device/src/models/tiny2lite/levels.rs` (the level constants of spec 8.5, regenerated with the script of W3's task w3-3 Step 3: w3-3 lands before CS-1, and its evidence test holds `levels.rs` against the ledger and `UNBACKED`), `descriptions/tiny2lite.json` (regenerated)
- Modify: `packaging/completions/obsbot4linux.bash`, `packaging/completions/_obsbot4linux` (regenerated)
- Modify: `docs/field-ledger.md`, `docs/migration-1.0.md` (rows whose mapping or availability changed; PX-U-n exceptions under "What is gone")
- Modify: the CLI tests that pin values of `descriptions/tiny2lite.json` (track C note 3: unit tests of `render.rs`, `describe.rs`, `completions.rs`, the snapshot `crates/obsbot4linux/tests/snapshots/status-fake-a.txt` if fake-a's state shows a changed value; `tests/parity.rs` rows whose value left the description)

**Steps:**

- [ ] **Step 1: The session's inputs.** CS-1 ran track A's live tests (`cargo test -p obsbot-device --test live -- --ignored` with the environment variable of A-15) and `scripts/check-model.sh --device tiny2lite --asleep --results cs1.tsv` on a dev build of the landed tip. Collect: the results file, the live tests' output, the notes for CAM items without a script row (CAM-4, 6, 7, 13, 14, 16 to 19, 22, 33 to 35).
- [ ] **Step 2: Ledger first.** For every passed row: evidence `measured <date>` in `docs/field-ledger.md`; `settle_ms` per 9.4 ("Where every row of a leaf showed its value at the touched re-read or at a read of a 1 s group, `settle_ms` is the largest lag rounded up to the next whole second; else the model-wide bound and the note `below <tier>`"); `asleep` from the `asleep` rows (`accepted`/`ignored`). For every failed U-row: the exception PX-U-n with the 0.13 rows of 9.3's "Fail" column.
- [ ] **Step 3: Contract-change PR.** In the model's description source: remove each failed field, value or bound (9.3: "what fails leaves the description by a contract-change PR before the release candidate"); set `settle_ms` and `asleep` per leaf; the `focus auto off` mapping (CAM-14) goes into `docs/migration-1.0.md` and the parity test row. Run `scripts/regenerate.sh`; `git status --short` shows `descriptions/tiny2lite.json` and `packaging/completions/*` (and nothing in `schema/`: no type changes). Then regenerate `crates/obsbot-device/src/models/tiny2lite/levels.rs` from the changed ledger and `UNBACKED` with the generator of the W3 plan (task w3-3, Step 3), never by hand; `cargo test -p obsbot-device --test evidence` must pass (still non-strict: the version is 0.13.2 and the model `untested` until w3-8 and w3-9).
- [ ] **Step 4: Pinned tests.** Run `nice -n 19 cargo test -j 2 --workspace --locked`; each failure in the CLI crate that names a removed value or a changed label is updated to the new description in this PR; any other failure is a defect and stops the task (`plan_mismatch`).
- [ ] **Step 5: Check.** `jq -r '.settings | to_entries[] | "\(.key) \(.value.values // "")"' descriptions/tiny2lite.json` shows no value of a failed U-row; the evidence-level test of 8.5 (W3) will fail on any field without `captured`/`measured`, so every kept U-row has its CS-1 date in the ledger. Workspace gate.
- [ ] **Step 6: Commit** (one PR, contract change, reviewed by a maintainer): `tiny2lite: camera session 1 results: <n> U-rows kept, <m> removed, settle_ms and asleep per leaf`.

---

## Acceptance of W2 (spec 10.3), mapped

| Acceptance without camera                                               | Where                                                                                                                                                                                             |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the check script passes against `fake-a` and `fake-b` in CI             | w2-2 `crates/obsbot4linux/tests/check_model.rs` (9 tests, part of `cargo test --workspace`, so CI's `test` and `msrv` jobs run them; the Arch job of `release.yml` runs them with `jq` from w2-9) |
| the CLI end-to-end tests pass                                           | track C's `e2e.rs` and `parity.rs` on the landed tip (coordinator check, note 5); w2-6 `replay.rs` adds the end to end on the recorded Tiny 2 Lite                                                |
| descriptions and completions are regenerated and their stale tests pass | coordinator check after the last W2 landing (note 5): `scripts/regenerate.sh` leaves `git status --short` empty; w2-8 regenerates them with the CS-1 description                                  |
| no U-row that CS-1 failed is left in `descriptions/tiny2lite.json`      | w2-8 (gate `camera`)                                                                                                                                                                              |
| content: daemon and `tiny2lite` end to end in a dev build               | w2-1 + w2-6 (recorded camera); the real camera in CS-1 (w2-8 step 1: the check script on the dev build)                                                                                           |
| content: `scripts/check-model.sh` (9.4)                                 | w2-2                                                                                                                                                                                              |
| content: `docs/protocol-2.md` completed with worked transcripts         | w2-3 (worked sequence with `pending`, a batch, a multi-device transcript with an unsupported device, lifecycle diagrams, the client library)                                                      |
| content: the migration notes drafted (12.2)                             | w2-4                                                                                                                                                                                              |
| acceptance with camera                                                  | none in W2 (10.3); w2-7 and w2-8 are gated because CS-1 feeds them                                                                                                                                |

## Camera items this stage leaves open

- **CS-1 as a whole** (track A's live tests, CAM-1 to CAM-8, CAM-10, CAM-13 to CAM-20, CAM-22, CAM-23, CAM-25, CAM-33 to CAM-35): w2-8 takes the results. The recorded evidence of w2-5 narrows CAM-1, CAM-3, CAM-8/U-23 and CAM-20 and is listed for the session.
- **CAM-20**: w2-7 (gate `camera`).
- **CAM-24, CAM-27, CAM-28** (W4): the check script with `--destructive` on the release candidate; the migration notes of w2-4 are rehearsed there.

## Self-review

1. **Spec coverage.** 10.3 W2 content and acceptance: mapped above, every line with its task. 9.4: rows from `describe --json` (every writable enum value, both bools, number bounds, range span: one row, decision 6), 70 s wait, restore after every row, lag from the reply and `reply`/`later` by rev, `readback:false` asks, `no_effect` rows by readback only, the kind of each row said in the output and in the results file (`how`), the rows that need a person in view marked, the setup writes of a `requires` row named, actions as a checklist, each asked first, with the state change printed, `--asleep` rows for `asleep: unknown` (sleep, 30 s, write a value that differs from the snapshot, wake, 30 s, 70 s; actions asked first), `restart`/`destructive` rows only with `--destructive`, last, after presets and views are printed, passing when back under the same id within 60 s, snapshot and trap on EXIT/INT/TERM, null snapshot values skipped, one line per row and a results file, `--assume-pass` in CI, 0.13's `check-readback.sh` deleted: all in w2-2's script, each with a test or a visible row in the CI run. 4 (preamble) and 6.1/15.1 row 96: w2-3. 12.2: w2-4, every bullet. 8.5: fixtures with capture keys, no serial (w2-5, w2-6). 2: w2-6 pins the frames; the script is one request per row. 11: in-process daemon on a temporary socket (w2-2, w2-6).
2. **Placeholder scan.** Every file of tasks w2-1 to w2-7 is given whole, or as the exact functions to replace (w2-1's `daemon.rs`). w2-8 is a procedure: its content is the camera's answer, and the plan says exactly which files and tests change and how each is checked.
3. **Type consistency.** The names used from W1 are listed above and were compiled in the scratch build; `run_on`'s signature is the same in w2-1 and w2-6.
4. **Retrospective lessons.** New integration tests ran twenty times in a loop (Proof); every wait in a test is a probe that returns "not yet" until a bounded deadline (`Daemon::start`, `Hookup::start`, the SIGTERM test's wait for rows, which reads only complete lines); the code was built once, here, and the builders put it in place. After the review of 2026-11-24 every changed piece of code was built and run again on the tip of `rebuild` (Proof), and each new test was shown to fail when the rule it pins is taken out (a question removed, the owner's slot used, an asleep value that equals the snapshot).
5. **Owner's lines.** No logic of its own in daemon, model or command line: W2 adds none to them (w2-1 is a seam, w2-7 is a camera-gated decoding rule the spec names). Only what the camera supports: the script builds its rows from the device's description, nothing else.

## Proof

- **Base of the scratch build** (`$CACHE/obsbot-build/fix-w2`, removed afterwards): `origin/rebuild` at 32ea9ab (W1 rounds 1 to 7: `a-14`, `b-12`, `c-13`, `b-13` in), plus a b-15 written exactly in the shape of its plan (`serve(config: Config, timing, bus)` with the lock inside, `run` choosing the bus, the test-control socket only for `Some(fake)`, b-15's own test `the_real_path_serves_what_the_sysfs_scan_finds` verbatim, which passes), then the code of w2-1 to w2-6 and w2-9 exactly as given here (w2-7 was applied on top of w2-5, run, and taken out again). Toolchain: rustc 1.100.0 (the machine's); `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets --locked -- -D warnings` clean, `shellcheck -s bash scripts/check-model.sh` and `scripts/build-arch.sh` clean.
- **Workspace tests:** 573 passed, 0 failed, 14 ignored (the 13 live camera tests of A-15 and the zsh syntax test) with `--no-fail-fast`. One of the first run's tests, `obsbot4linuxd` `deadline::every_request_is_answered_within_the_deadline` (b-13, not a W2 file), failed once while the machine was loaded (`the request was not started within 15 s`) and passed in three single runs; it is named here, not hidden: a flake of b-13's test under load, for the owner of b-13. New: `run_on_serves_the_given_bus_without_a_test_control_socket` (1), `check_model` (9, about 30 s), `replay` (5), `doc_examples` (2), `capture_evidence` (3).
- **Twenty runs in a row** (retrospective lesson 9) of `check_model` and `replay`, with `doc_examples` after each: 20 of 20 iterations green, every time `check_model` 9 passed, `replay` 5 passed and `doc_examples` 2 passed (320 of 320 test results).
- **With jq 1.6 and the CI toolchain's bash version:** `check_model` passes (9 of 9, about 80 s) with the static jq 1.6 binary first in `PATH`; the filter that replaces DEL and the C1 controls gives the same bytes in jq 1.6 and 1.8.2. bash was 5.3 (CI's runner has 5.1; the script uses nothing newer than bash 4.4: `mapfile`, associative arrays, `${!i}`, `[[ … ]]`).
- **The tests can fail:** `a_value_that_never_shows_fails_its_row` is the script's negative case; with the question before an asleep action taken out `a_person_who_declines_every_action_sends_none` fails; with the owner's slot used as the script's own slot `the_owners_presets_and_views_are_left_alone` fails; with an asleep row that writes the first value even when the snapshot shows it `fake_a_passes_every_row_and_is_left_as_it_was` fails; with a second open counted `replay` fails; swapping an event and its reply in the doc fails `doc_examples`.
- **Transcripts:** section 11 was recorded from the scratch daemon binary on `fake-b,unknown`; the recording equals the doc's lines except that the pipelined pair came as two batches (13 lines), as the doc says it may. The example `watch.rs` ran against the scratch fake daemon and printed the devices, `zoom.factor` of the fake and the protocol (the error path for a model without `zoom.factor` compiles and is not run: no fake lacks it).
- **w2-7** (gated) applied on top of w2-5 on the tip: `capture_evidence` 4 passed with it (the boot test replaced by two), every other `obsbot-device` test unchanged (161 unit tests and the integration tests), and then taken out again.
- **Evidence extraction:** status blocks and serial-query timings were read from captures 01, 13, 14 and 16 with `tshark` (`usbvideo.control_value` of GET_CUR on unit 2, selector 6; SET_CUR frames on selector 2). Serial bytes were never printed or kept: only the answer length (14) and the times.
- **Unproven:** everything against the landed b-14 and b-15 (the scratch build had a b-15 in the shape of its plan, not the landed one); the script on a real camera (CS-1, W4); the MSRV build with Rust 1.88 (no 1.88 toolchain here; the code avoids let-chains and newer std API; CI's `msrv` job checks it); the script on bash 5.1 (run with 5.3); the Arch container run of w2-9 (the pull request's own `release.yml` dry run is its proof); w2-8 (procedure only).

## Notes for the coordinator

1. **Gates.** w2-0 first: this document, scrubbed, lands as a docs-only pull request before any builder starts (every task's `plan_doc` is its path on `rebuild`; `git -C <worktree of rebuild> pull --ff-only` afterwards). w2-9 next, before w2-2. w2-1, w2-2 and w2-3 start only after `b-15-real-bus` has landed (W1 complete); w2-5 after `a-14-remove-carried` (landed); w2-4 any time after w2-0. w2-6 starts after w2-1 has landed. w2-7 and w2-8 are not issued before CS-1 has happened with the owner present (gate `camera`). Issue #250 needs no gate: no W2 file depends on it.
2. **Two glue PRs first.** w2-1 changes daemon code outside W2's owner files (spec 10.3 "glue in crates only by contract-change PRs"); land it as its own small PR, reviewed as a contract-style change (it adds a doc-hidden entry and changes no behaviour of `run`). w2-9 changes two lines of W3's files (`packaging/arch/PKGBUILD`, `scripts/build-arch.sh`); land it before w2-2 and tell W3 (note 3). The other deviations from the owner files are listed under Global Constraints.
3. **For W3 (packaging and `.github/`).** The workspace tests need `bash` and `jq` (w2-2's `check_model.rs`): w2-9 puts `jq` into the PKGBUILD's `checkdepends` and into the Arch container of `scripts/build-arch.sh`; W3's tasks that edit those two files rebase on it, and any deb or rpm build that runs `cargo test` needs `jq` installed (the present ones run no tests; the runner has jq 1.6). **`ci.yml`'s `changes` job** counts a change of only `docs/*` or `*.md` as no code and then skips fmt, clippy and test: W3's w3-3 (Step 4) makes `docs/protocol-2.md` and `docs/field-ledger.md` count as code there; until it has landed, `doc_examples` is not run for a pull request that edits only that file. **`support`**: the pull request that sets the Tiny 2 Lite to `tested` (w3-8) changes two lines of `crates/obsbot4linux/tests/replay.rs` (w2-6 marks both). `docs/migration-1.0.md` (w2-4) is the source of the CHANGELOG's migration notes (w3-6 Step 3) and stays in `docs/`. `scripts/check-model.sh` is not packaged (decided in the cross-check of 2026-11-24): spec 12.1 lists what the package holds, and spec 8.7 step 7 has owners run the script on a dev build, that is, from a checkout.
4. **For CS-1 (the owner present).** Bring the dev build of the landed tip; run track A's live tests, then `scripts/check-model.sh --device tiny2lite --asleep` (no `--destructive`: that is W4's, CAM-24), with the items of the table "Camera: what the recordings stand in for" first, CAM-20 among them. The results file goes into w2-8. What to expect (decision 21 has the estimate): about 145 setting rows and, asleep, about 76 more, so plan for an hour or more with the questions; the script asks before every action and every asleep action row, and says each time that what they store (presets, views, power-on and return poses) is not restored: it saves its own preset in a free slot and its own view called `check` and deletes them again, and it prints the state before the first action row so that the owner can put back a power-on pose or a return pose by hand if it matters. Have the camera awake (a sleeping one is woken after a question) and in view for the rows marked "person in view".
5. **After the last W2 landing** (deterministic checks, no agent): `scripts/regenerate.sh` and then `git status --short` empty; `scripts/check-cli-deps.sh`; `cargo test --workspace --locked` with the count of tests and the names of the ignored ones (the camera live tests and the zsh syntax test only); `cargo test -p obsbot4linux --test e2e --test parity --test replay --test check_model` green; `release.yml`'s dry run on the pull request of w2-2 (the Arch jobs) green.
6. **Budget.** Planning was done with the code built (retrospective "build once"): the builders put the code in place and run the steps; the per-task review can be light for w2-0, w2-3, w2-4, w2-5 (docs and fixtures) and should be full for w2-1 (glue), w2-9 (packaging glue) and w2-2 (the script restores the camera's state and asks before it acts; review focus 1 to 3 and 6 to 9).
7. **A flaky test outside W2.** `obsbot4linuxd`'s `deadline::every_request_is_answered_within_the_deadline` failed once under load in the scratch workspace run (Proof); it is b-13's, not W2's. A red run of it on a W2 pull request is a rerun, and the flake is for the owner of b-13.

## Review corrections (2026-11-24)

A review of the first draft (blocker and major findings, and the minors that were cheap) changed the plan and its code as listed. Every changed piece of code was built and run again on the tip of `rebuild` (Proof). No push, no issue was made for any of them.

| #   | Finding                                                                                                                                                                                                  | What changed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | major: an asleep row of an action ran with no question (on the Tiny 2 Lite, in CS-1, among others `view.delete id=0`, `preset.delete`, `power_on.set_pose`, `gimbal.turn pan_speed=89` with `keep:true`) | `asleep_row` asks first, like `action_row`, with the question naming what is not restored; a `.turn` action runs asleep only with its `.stop` (sent before the wake and again after it) and the trap stops a turn that is still going; test `a_person_who_declines_every_action_sends_none` (answers from a file, `CHECK_MODEL_TTY`) compares the fake's operation log                                                                                                                                               |
| 2   | major: presets, views and the power-on state were overwritten, renamed or deleted and never restored, and the plan claimed a full restore                                                                | the state before the first action row is printed (presets, views, power-on start, return poses); every question says the change is not restored; the script works only on a preset slot the snapshot shows free and on a view it saved itself (`check`), found again by name and id, and deletes both at the end and in the trap; a row that needs an item and has none is `not run`; what stays unrestorable (power-on and return poses) is named in decision 9; test `the_owners_presets_and_views_are_left_alone` |
| 3   | major: asleep rows gave wrong ledger input: the value could equal the snapshot, and `requires` leaves got no setup (reply `conflict` read as an asleep answer)                                           | the first row value that differs from the snapshot is written (a number with both bounds also tries the point three quarters up; none: `not run`); the `requires` of the leaf are set while awake, named in the output, and restored; `fake_a_passes_every_row…` compares every asleep row with the snapshot and fails on any `conflict`                                                                                                                                                                             |
| 4   | major: `seen` and the lag fed `settle_ms` wrongly: a tier read forced `reply`, the lag started before the `set`, and the fake-b test depended on timing                                                  | `seen=reply` only when the first `status` has the same `rev` as the `set` reply and shows the value; the lag is timed from the reply; the test pins `led.brightness=5` to `later` whatever the spawn time (decision 7, review focus 8)                                                                                                                                                                                                                                                                               |
| 5   | major: w2-1's code contradicted b-15's plan (`serve(config, timing, bus)`, `Config` by value), so b-15's test would not compile                                                                          | w2-1 is rewritten against b-15's planned shape: `run_on` calls `serve`, which becomes `pub(crate)`; step 0 re-checks the landed b-15; built and run with a b-15 in exactly that shape, b-15's test unchanged and passing                                                                                                                                                                                                                                                                                             |
| 6   | major: the release workflow's Arch job (makepkg `check()` without `jq`) goes red from w2-2 on                                                                                                            | new task w2-9 (glue PR): `jq` in the Arch container's `pacman` line and `checkdepends=('jq')`; w2-2 depends on it; note 3 tells W3                                                                                                                                                                                                                                                                                                                                                                                   |
| 7   | major: the plan, once on `rebuild`, held private paths and a name, and was not on `rebuild`                                                                                                              | paths became `$CACHE/…`, the name "the owner"; new task w2-0 lands the plan, scrubbed, before any builder starts; every task depends on it                                                                                                                                                                                                                                                                                                                                                                           |
| 8   | major: a full serial could reach the results file (`fail: not back under sn:…`); the firmware string went in raw                                                                                         | the failure message uses the short id; the header writes the firmware as a JSON string (`jq -c`); every results field has its control characters replaced; camera text goes through a filter for DEL and C1                                                                                                                                                                                                                                                                                                          |
| 9   | major: a pull request that edits only `docs/protocol-2.md` skips `doc_examples` in CI                                                                                                                    | the gap is stated (review focus 5, w2-3 step 5); W3 is asked to count the file as code in `changes` (note 3); until then the reviewer runs the test                                                                                                                                                                                                                                                                                                                                                                  |
| 10  | minor: w2-7 step 5 left `write.rs` out of the commit                                                                                                                                                     | `write.rs` is in the `git add` line                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 11  | minor: spec 9.4 items the script did not cover                                                                                                                                                           | (1) each row says its kind, column `how`; (2) rows that need a person in view are marked; (3) the setup writes of a `requires` row are printed; (4) each row restores its leaf and waits for it; (5) the range is one row, stated in decision 6; (6) `scripts/check-readback.sh` is deleted in w2-2                                                                                                                                                                                                                  |
| 12  | minor: robustness on the real camera                                                                                                                                                                     | an unavailable device ends with 3; a sleeping one is woken after a question; `requires` setups are read with `mapfile`, not word splitting; a failed row records the error code, never the raw reply; `rows()` of the test reads only complete lines; `.gitignore` names the default results file; the duration of a run is estimated (decision 21, note 4)                                                                                                                                                          |
| 13  | minor: foreign text: DEL and C1 are not escaped by JSON; `watch.rs` printed the id with `{}` and ended on the first device without `zoom.factor`                                                         | the filter and the field cleaning of finding 8; `watch.rs` prints the short id with `{:?}` and prints the refusal code for a device that has no `zoom.factor`, then goes on (the doc's copy is byte-equal, tested)                                                                                                                                                                                                                                                                                                   |
| 14  | minor: `replay.rs` asserts `untested`, which W3 changes; `ReplayBus::open` panics on a second open                                                                                                       | the assertion is marked and named for W3 (w2-6 notes, note 3); `ReplayBus` counts its opens, refuses a second one with an error and `Hookup` fails the test with the count                                                                                                                                                                                                                                                                                                                                           |
| 15  | minor: owner files: only w2-1 was framed as glue                                                                                                                                                         | the deviations are listed once under Global Constraints (test and example files under `crates/`, the dev-dependency, `.gitignore`, w2-9's two lines, w2-7's model code) for each reviewer to confirm                                                                                                                                                                                                                                                                                                                 |
| 16  | minor: unproven on CI's jq 1.6; the pending recording ran on three fakes                                                                                                                                 | the tests were run with a jq 1.6 binary first in `PATH` (9 of 9), numbers in a value's text are rounded to nine decimals so that they do not depend on a jq version; both transcripts are recorded on `fake-b` and the unsupported device alone; bash 5.1 and the MSRV build stay for CI                                                                                                                                                                                                                             |

## Cross-check corrections (2026-11-24)

The cross-check of the W2 and W3 plans against each other, against the rest of W1 and against the widget's track D changed this plan as listed. None of the changes was built; each is a few lines.

| #   | Finding                                                                                                                                                                                                  | What changed                                                                                                   |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| 1   | W3's w3-1 moves the command line's test sockets from the target directory to `$TMPDIR`; the daemons of `check_model.rs` and `replay.rs` keep their `<socket>.lock` (5.9), which would then be left there | the `Drop` of `Daemon` (w2-2) and of `Hookup` (w2-6) removes `<socket>.lock`, as W3 does in `FakeDaemon::halt` |
| 2   | `replay.rs` asserts `untested` in two tests, the notes named one line                                                                                                                                    | both lines are marked; the notes and coordinator note 3 say two lines; W3's w3-8 lists the file                |
| 3   | W3's w3-3 lands before CS-1, and its evidence test holds `levels.rs` against the ledger and `UNBACKED`: w2-8, which changes both, would turn it red                                                      | w2-8 regenerates `levels.rs` with w3-3's generator and lists the file                                          |
| 4   | the `changes` rule of `ci.yml` and `check-model.sh` in the packages were left open for W3                                                                                                                | W3's w3-3 Step 4 is named as the change of `changes`; the script is not packaged (note 3)                      |
