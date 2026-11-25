# obsbot4linux 1.0, stage W3 (packaging and release candidate) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the 1.0 tool as packages and publish the release candidate `v1.0.0-rc.1` (spec 10.3 W3): the eight artifacts of 12.1 with both binaries, the user units (`ExecStart` = `obsbot4linuxd`, `NoNewPrivileges=yes`), the completions and the schema; a release path that publishes a `vX.Y.Z-rc.N` tag from `rebuild` as a pre-release; the evidence-level test of 8.5; the Tiny 2 Lite at `support: tested` (8.8); the CHANGELOG with the migration notes (12.2) and the README with the support table (8.6) and without the 0.13 statements that a gimbal move switches the tracking to static.

**Architecture:** No new crate and no contract change. Packaging stays where it is (`packaging/`, the CLI crate's `[package.metadata.deb]` and `generate-rpm` tables, `.github/workflows/release.yml`, the release scripts in `scripts/`). Two new checks: `scripts/check-units.sh` (systemd accepts the units, and the daemon started as the socket unit starts it, with `no_new_privs`, answers on the passed socket) in `ci.yml`, and the evidence-level test (`obsbot_device::evidence`, the Tiny 2 Lite's table `models/tiny2lite/levels.rs`, `tests/evidence.rs`) in the workspace tests. The release candidate is the version `1.0.0-rc.1` on `rebuild`, tagged `v1.0.0-rc.1` there; the release workflow's guard accepts a tag `v1.0.0-rc.N` on `rebuild` and every other tag only on `main`.

**Tech Stack:** Rust 2024 edition, MSRV 1.88 (CI toolchain 1.99.1); cargo-deb 3.8.0 and cargo-generate-rpm 0.21.1 (the versions `release.yml` pins); makepkg (Arch); systemd-analyze, systemd-socket-activate and setpriv (systemd and util-linux) for the unit check; bsdtar for the package listings.

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` (revision 2, approved 2026-11-13, with section 15.1, rows 88 to 100), sections 2, 5.10, 8.5, 8.6, 8.8, 9.1, 9.2, 9.4, 10.1, 10.3 (W2, W3), 10.4, 10.6, 11, 12.1, 12.2, 14; the W1 plans `docs/superpowers/plans/2026-11-13-obsbot4linux-w1-{a-device,b-daemon,c-client-cli}.md` (their code is on `rebuild`).

## Review corrections (2026-11-24)

A review of the first draft found 5 major and 12 minor points. Each is listed with what changed; the code of the changed blocks was built and tested again in a scratch worktree of `origin/rebuild` at `05e66e9` (see "Proof"). Three questions stay with the owner (P5, P6, P7 under "Product questions"). Nothing was pushed, and no issue was opened.

| #          | Finding                                                                                                                                                                                                                                                                                                                | What changed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 (major)  | w3-9 could cut the candidate without the 1.0 unit: its chain never reached w3-2, so `v1.0.0-rc.1` could ship `ExecStart=/usr/bin/obsbot4linux daemon`.                                                                                                                                                                 | `w3-2-units` is in `w3-9.depends_on` (JSON and table). w3-9 Step 3 greps the service for `ExecStart=/usr/bin/obsbot4linuxd` and `NoNewPrivileges=yes` and runs `scripts/check-units.sh target/debug`; Step 6 checks the service inside every published package (measured on the four local packages).                                                                                                                                                                                                                                                                                                    |
| 2a (major) | No W2 gate, although spec 10.3 puts W3 after W2, and the plan said w3-1 to w3-7 "can be built now".                                                                                                                                                                                                                    | The JSON carries `w2` on every task (w3-1 `w2+w1:c-13-parity-rows`, w3-3 `w2+w1:a-14-remove-carried`); the plan states that building them before W2 departs from 10.3 and is the owner's decision (P6, recommendation: allow). The W2 plan (`w2-plan-draft.md`) was read and the shared files are listed.                                                                                                                                                                                                                                                                                                |
| 2b (major) | w3-8 and w3-9 needed CS-1, but their gate was only `publish`.                                                                                                                                                                                                                                                          | Gate `w2+cs-1+publish` on both: the owner's go alone cannot start them.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 2c (major) | w3-7's required Opus forge-lens review and security check were prose only.                                                                                                                                                                                                                                             | New JSON field `land_gate` on w3-7 (and in the task table); the prose stays.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 3 (major)  | w3-1 Step 2 runs `--test parity`, created by W1's `c-13-parity-rows`; the proof claimed a base (`4499637`) that has neither that file nor `a-14`.                                                                                                                                                                      | Gate `w1:c-13-parity-rows` on w3-1 (JSON, plan); both W1 tasks are on `rebuild` since `05e66e9`. "Proof" now names the real base: everything was re-built and re-run on `05e66e9`; the first draft's `4499637` is stated as superseded (603 tests then, 547 now: a-14 removed the carried modules).                                                                                                                                                                                                                                                                                                      |
| 4 (major)  | The evidence test let every value and bound inherit its field's level, so `power_on.start.source=preset_0..2` and `exposure.anti_flicker=auto` counted as `captured` while the ledger says `none`; only `UNBACKED` held the truth. `keys()` missed `writable`, list items, results, `precision` and `each` components. | Option 1 of the review. Every mark of 9.3 on a value, bound or aspect has a row of level `none` (38 new rows, generated from `UNBACKED`); `evidence::check` holds rows and marks against each other in both directions; the new test `the_marks_of_9_3_follow_the_ledger` holds `UNBACKED` against the ledger's §9.3 column per field and against every evidence cell that says `none`; `keys()` covers `writable`, `precision`, `each` components, list item fields and action results. Decision 7 rewritten; the strict per-value reading is P7 for the owner. Five mutations fail as stated (Step 6). |
| 5 (major)  | The docs-only rule of `ci.yml` skipped the evidence test for a pull request that edits only `docs/field-ledger.md`.                                                                                                                                                                                                    | w3-3 Step 4: `docs/field-ledger*.md` and `docs/protocol-2*.md` (read by W2's `doc_examples`) count as code in the `changes` job; checked on sample file lists, `actionlint` clean.                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 6 (major)  | The release guard kept a `rebuild` path after 1.0: any `v*-rc.*` on `origin/rebuild`.                                                                                                                                                                                                                                  | The exception names `v1.0.0-rc.*` only (decision 11, Review Focus 6, guard table with `v1.1.0-rc.1`, `v1.0.0-rc.2`, `v1.0.0-beta.1`); W4's release pull request removes it and `rebuild` from `ci.yml` (w3-9 Step 7, notes 6).                                                                                                                                                                                                                                                                                                                                                                           |
| 7 (minor)  | `check-units.sh` unproven on systemd 249, no fallback; it set `OBSBOT4LINUX_SOCKET` itself; `NoNewPrivs` came from `setpriv`.                                                                                                                                                                                          | The script probes `systemd-analyze --user verify` on a trivial unit and skips that step with a notice (success line repeats it; `CHECK_UNITS_ANALYZE=required` fails); a key list catches a typo without systemd; the daemon runs with `XDG_RUNTIME_DIR` set and no `OBSBOT4LINUX_SOCKET`, like the unit; wording says that `setpriv` sets the same flag and the unit line is checked as text. A findings path for the first pull request (decision 3, note 8). Found while testing: `verify` loads the other units of its directory, so the probe has a directory of its own.                           |
| 8 (minor)  | Public text: the README made a statement about where the knowledge comes from; spec 8.5 and the run brief conflict; the "Untested models" line was not 8.6's.                                                                                                                                                          | The README, the CHANGELOG and the code comments (`evidence.rs`) say nothing about the origin; the conflict is P5 for the owner with a recommendation. The untested line is spec 8.6's: "built from an owner's recordings, not verified by the maintainers" (README, CHANGELOG, w3-8's diff).                                                                                                                                                                                                                                                                                                             |
| 9 (minor)  | w3-5 (wave 1) grepped for a link that w3-4 (wave 2) adds.                                                                                                                                                                                                                                                              | The grep moved to w3-4 Step 7.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 10 (minor) | w3-1's depth numbers disagreed (87, 90, 110); long `$TMPDIR` broke `check()` like a deep target did.                                                                                                                                                                                                                   | One command, one number: a target directory of exactly 100 bytes (Review Focus 1, Step 3, JSON acceptance); before 12 of 13 fail, after 13 pass. The three helpers fall back to `/tmp` for a path over 100 bytes (measured with a 94-byte `$TMPDIR`).                                                                                                                                                                                                                                                                                                                                                    |
| 11 (minor) | w3-4 Step 5 unset `TMPDIR` (tests wrote to `/tmp`); Step 4 used plain `nice`.                                                                                                                                                                                                                                          | Step 5 keeps the task's short `TMPDIR` and unsets only `XDG_RUNTIME_DIR`; Step 4 uses `nice -n 19`; the tarball is listed with `git ls-files -co --exclude-standard` (new files are not tracked before the first commit: found while re-running it).                                                                                                                                                                                                                                                                                                                                                     |
| 12 (minor) | `check()` gates the Arch release jobs on timing-sensitive daemon tests, run once.                                                                                                                                                                                                                                      | Step 5 repeats `cargo test -p obsbot4linuxd` ten times: 10 of 10 on `05e66e9` (artificial load is forbidden on the desktop, so repetition). A failure is a finding for track B.                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 13 (minor) | The migration notes sat in the rc section, so `changelog-section.sh v1.0.0` would lack them; 10.6 step 5 (rollback packages) was unplanned.                                                                                                                                                                            | The rollback sentence is in the 1.0 notes (w3-6, with a grep); decision 14 and w3-9 Step 7 hand W4 the rename `[1.0.0-rc.1]` → `[1.0.0]`, so the notes are one section.                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 14 (minor) | The README's `tested` text claimed more than has happened at candidate time.                                                                                                                                                                                                                                           | It says the maintainers test every setting and action and that a candidate is published before that check is finished; w3-9 adds a one-line "release candidate" paragraph to the notes.                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 15 (minor) | No checks for "no libudev", "packages do not enable the socket unit" and the README's glibc floor.                                                                                                                                                                                                                     | A step in the `contents` job (w3-4 Step 3, 6b): `readelf` for `NEEDED udev` and the highest `GLIBC_x.y`, install scripts per format. Measured: the floor is `GLIBC_2.34` for the tarball, deb and rpm and `GLIBC_2.39` for the Arch package built here, so the README's claim now names deb, rpm and tarball and the check skips the Arch package (decision 18). The step ran locally with failure cases; the rpm branch is proven only by the dry run.                                                                                                                                                  |
| 16 (minor) | Weak spots of w3-3's tests: hard-coded control ids; "covers that CS-1 step in part"; "captures 01 and 16"; stale comment in `tests/live.rs`; `levels.rs` as a snapshot.                                                                                                                                                | The ids come from `decode::UVC` and every UVC leaf with bounds must have a fixture; the doc says the replay does not replace the live step; the module doc names 01, 14, 16 and 18; `live.rs`'s comment is updated; Step 3 says to regenerate `levels.rs` (generator now takes `description.rs` too) when the ledger or `UNBACKED` changes, and the generator reproduces the committed file byte for byte.                                                                                                                                                                                               |
| 17 (minor) | Process: `plan_doc` does not exist on `rebuild`; the W2 plan could not be read; w3-4 Step 7 let the builder copy CI's listing into `contents.txt`.                                                                                                                                                                     | Note 7: the plan document lands first as a docs-only pull request. The W2 plan was read: its findings (no `ci.yml` change by W2, `docs/migration-1.0.md`, `jq` for `check()`, ledger edits, `UNBACKED` changes) are in "What W3 needs" and in w3-3, w3-4 and w3-6. Step 7: `contents.txt` is the local listing, never overwritten by the artifact; the reviewer reads a line-by-line diff.                                                                                                                                                                                                               |

Corrections found while testing, beyond the review: the ledger table is re-aligned by w3-3's script (a hand edit would conflict with W2's ledger edits); the daemon tests leave `o4ld-*` directories in `$TMPDIR` (P4: contained in the task's own); the first draft's `--test parity` and `a-14` assumptions were tested on a base that did not hold them.

## Global Constraints

- W3 content (spec 10.3): "two-binary packages (12.1) with units (`ExecStart` = `obsbot4linuxd`, `NoNewPrivileges=yes`), completions and the schema; the release workflow publishes a tag `v1.0.0-rc.N` as a pre-release; the Tiny 2 Lite set to `support: tested` (8.8); CHANGELOG with the migration notes; README support table, and the README without the 0.13 statements that the gimbal moves switch the tracking to static first (section 2, 15.1 row 88)."
- W3 owner files (spec 10.3): "`.github/`, `packaging/` (except completions), `CHANGELOG.md`, `README.md`, and the support level in the `tiny2lite` registry entry." This plan also touches the release scripts in `scripts/`, three test helpers of track C, the model's evidence table and its ledger cells (each named in its task and in "Notes for the coordinator").
- W3 acceptance without camera (spec 10.3): "a workflow run produces the eight artifacts, and a listing of each shows both binaries, the units, the completions and the schema; `makepkg` with its `check()` passes without `XDG_RUNTIME_DIR` (the tests use a fake daemon on a temporary socket); the evidence-level test of 8.5 passes for `tiny2lite`; the release candidate is published." Acceptance with camera: none (W4 runs on this candidate).
- Packages (spec 12.1): "One package `obsbot4linux` with both binaries, the systemd user units, the bash and zsh completions (installed to the standard completion directories) and `/usr/share/obsbot4linux/protocol-2.json`." "Formats as today: Arch, deb, rpm and tarball, each for x86_64 and aarch64: eight artifacts." "Packages do not enable the socket unit."
- Evidence (spec 8.5): "`captured` and `measured` are the only levels that let a field into a released description. A level belongs to each field, and also to each value of an enum's `values[]` and `writable[]` and to each bound of a number or range … Fields at the level `daemon` are not camera values … The model module records the levels as constants. A test in stage W3 fails if any field, value or bound of any registered model, `tested` or `untested`, has another level."
- Support (spec 8.6, 8.8): "Package descriptions name tested models only." "The README support table has one row per registered model: display name, USB ids, support level, and the firmware versions the maintainers tested". Release notes: "`<display name> (USB <vendor>:<product>): tested on firmware <versions>`". "**Support:** `untested` on `rebuild` until stage W3. The release candidate carries `support: tested`, the claim that stage W4 then verifies on every parity row."
- Migration notes (spec 12.2): the command map (9.1) and the exceptions (9.2); the upgrade steps; `$XDG_STATE_HOME/obsbot4linux/` no longer used; the gimbal moves no longer switch the tracking; `preset save N` sends no name.
- Release day (spec 10.6): the candidate `v1.0.0-rc.N` is published and W4 passes on it before `main` fast-forwards to `rebuild` and `v1.0.0` is tagged. So the candidate is tagged on `rebuild`.
- Section 2 (the owner's line, restated 2026-11-13): a request sends exactly what it names; daemon, model and command line carry as little logic of their own as possible and expose only what the camera supports; clients chain no second request.
- Contract-change rule (spec 10.1): no task of this plan changes `obsbot-proto` types or validator, the description format, `trait Camera`, `trait Transport`, `trait Bus`, the schema, the golden transcripts or `descriptions/*.json`. `tiny2lite`'s `UNCONFIRMED` list and the ledger's evidence cells change; the description itself does not (its generated file stays byte-equal, which `tests/generated.rs` proves).
- Public text (spec 8.5): "recorded from the vendor app" and "measured on the camera" are the only statements about how knowledge was obtained that the spec allows; no names of vendor-app methods or firmware symbols, no serial numbers, no private paths or hosts. The run's brief forbids any statement that knowledge comes from the vendor's app. The two rules conflict; P5 was decided as recommended in the cross-check of 2026-11-24 (the brief's rule wins; the owner reviews it): this plan uses neither phrase. The README, the CHANGELOG and the code comments name no vendor application and say nothing about where the camera knowledge comes from; captures are cited by key ("capture 11"), and the line for an untested model is the spec's own (8.6: "built from an owner's recordings, not verified by the maintainers").
- No camera: nothing in this plan opens a video device. `scripts/check-units.sh` runs the daemon with `--fake`; the `#[ignore]` live tests stay ignored.
- Machine (the run's brief): every cargo command `nice -n 19` with `-j 2` (a scratch worktree of one builder may use `-j 4`), `CARGO_TARGET_DIR` and `TMPDIR` in the task's own build directory (never `/tmp`), removed at the end. No artificial CPU load: the desktop forbids it, so a timing risk is covered by repetition. `Cargo.lock` changes only in w3-9.
- Repository rules: every pull request goes into `rebuild`; a commit reaches it only as the green head of a pull request (check `required`), signed, fast-forward. `release.yml` runs on pull requests that touch `packaging/**`, `scripts/**`, `Cargo.toml`, `README.md`, `CHANGELOG.md` and the workflow itself: that run is the dry run of the eight packages and their listing.

## Review Focus

Inputs and situations a reviewer should try, each pinned by a test or a check in its task:

1. **A deep build directory** (an AUR helper's `~/.cache/yay/obsbot4linux/src/obsbot4linux-1.0.0rc.1/`): the tests' Unix sockets must not live under the target directory (107-byte limit). Measured on 2026-11-24: `makepkg` in a directory 87 bytes deep failed 12 tests of `obsbot-proto/tests/client.rs` with "path must be shorter than SUN_LEN". The reproduction pinned for this plan is one command with one number: `cargo test -p obsbot-proto --test client` with `CARGO_TARGET_DIR` a path of exactly 100 bytes (w3-1 Step 3): before w3-1, 12 of 13 fail with `SUN_LEN`; after it, 13 pass (both measured on 2026-11-24). Task w3-1 moves the sockets to `$TMPDIR` (and to `/tmp` when `$TMPDIR` is itself too long: measured with a 94-byte `$TMPDIR`); the proof is a `makepkg` run in a deep directory (w3-4).
2. **A typo in a unit file** (`NoNewPrivilegs=yes`): systemd only warns ("Unknown key … ignoring") and `systemd-analyze verify` exits 0 (measured). `check-units.sh` fails on a key that is not in its own list (this holds on a runner where `systemd-analyze --user verify` does not work), on any line `systemd-analyze` prints about the units, and on a missing `NoNewPrivileges=yes` (w3-2, each measured).
3. **A forged or misplaced tag:** `v1.0.0` pushed on a `rebuild` commit, `v1.1.0-rc.1` on `rebuild`, `v1.0.0-rc.1` on a commit whose `Cargo.toml` says `0.13.2`, or `v1.0.0-beta.1`. The guard refuses the first two (branch), `changelog-section.sh` the other two (version match, rc form) (w3-7, each run locally).
4. **The version spellings of a release candidate:** Cargo `1.0.0-rc.1`; deb `1.0.0~rc.1` (cargo-deb converts it, measured); rpm `1.0.0~rc.1` (cargo-generate-rpm refuses `1.0.0-rc.1`, "contains invalid character", measured; `-s 'version = "1.0.0~rc.1"'` works, measured); Arch `1.0.0rc.1` (`vercmp 1.0.0rc.1 1.0.0` = −1, while `1.0.0_rc.1` and `1.0.0.rc.1` sort after 1.0.0, measured). Each must sort before `1.0.0`, so that the release replaces the candidate.
5. **`support: tested` while a mark of 9.3 stands:** the evidence test fails (w3-3 `a_tested_model_may_keep_no_mark_of_9_3`, and a trial flip on 2026-11-24 failed exactly `every_registered_model_meets_the_evidence_rule`). The version `1.0.0-rc.1` makes the same rule strict for every registered model, tested or untested (spec 8.5), so the candidate cannot be built before CS-1 has settled every U-row.
6. **A branch `rebuild` that comes back after 1.0:** spec 10.1 removes the ruleset after the final merge. The guard's exception is limited to `v1.0.0-rc.*`, and W4's release pull request removes it and `rebuild` from `ci.yml`'s triggers (decision 11, hand-over in w3-9 Step 7); without that, anyone with write access could open a pull request into a recreated `rebuild`, get `required` green and tag a candidate of code that never reached `main`.

---

## What W3 needs from W2, W1 and the camera

- **W2** (spec 10.3: "Depends on: W2"; its plan, `w2-plan-draft.md` beside this one, was read for this revision): W3 uses none of its code. Spec 10.3 orders W3 after W2, and W2 itself waits for CS-1 (w2-7 and w2-8 carry the gate `camera`), so the critical path is G → W0 → A → CS-1 → W2 → W3 → W4. This plan's w3-1 to w3-7 need nothing of W2, so they could be built before W2; that would depart from 10.3 and is the owner's decision (P6; recommendation: allow it for w3-1 to w3-7, because the shared files below are few and a rebase over W2 is mechanical). The JSON carries the gate `w2` on every task, which the owner's go can waive. What W2 hands to W3: (a) `docs/migration-1.0.md` (w2-4, no gate), the draft of the migration notes that w3-6 moves into the CHANGELOG (w2-8 updates it with CS-1's results); (b) `crates/obsbot4linux/tests/check_model.rs` (w2-2) runs `scripts/check-model.sh` in the workspace tests and needs `bash` and `jq`: W2's w2-9 adds `checkdepends=('jq')` to `PKGBUILD` and `jq` to the Arch container before w2-2 lands (w3-4 Step 2b only checks that they are there); (c) `docs/protocol-2.md` is read by W2's `doc_examples` test (the docs-only rule of `ci.yml` counts it as code, w3-3 Step 4). Shared files, checked against W2's plan: `.github/workflows/ci.yml` (W2 does not touch it: the earlier statement that W2 adds a check-script step is withdrawn); `docs/field-ledger.md` (w2-5 appends a section at the end; w2-7 and w2-8 edit evidence cells of the State table, where w3-3 appends to ten cells: w3-3 Step 1 is a script that appends only where the text is missing and re-aligns the column, so it can be run again after any rebase); `scripts/` (W2 adds `check-model.sh`, w3-2 adds `check-units.sh`, w3-7 edits three release scripts: no common file); `CHANGELOG.md` (W2 writes none); `packaging/completions/*` (w2-8 changes them after CS-1; w3-4 only packages them); `description.rs` (w2-8 changes `UNBACKED` after CS-1: `levels.rs` is then regenerated, w3-3 Step 3).
- **W1** (all landed before W3): `obsbot4linuxd` with `--fake` (B-1) and the real bus (b-15), the CLI (track C, with `c-13-parity-rows`, which creates `crates/obsbot4linux/tests/parity.rs`: w3-1 runs it, hence its gate `w1:c-13-parity-rows`), track A complete (`a-14-remove-carried` changes `obsbot-device/src/lib.rs`, `models/tiny2lite/mod.rs` and `registry.rs`, which w3-3 edits by one line each or reads: hence its gate `w1:a-14-remove-carried`). Both are on `rebuild` since `05e66e9` (checked on 2026-11-24; the first draft of this plan was built on `4499637`, before them). Names used: `obsbot_device::registry::REGISTRY` (`Registry { vendors, models: &[ModelEntry] }`), `obsbot_device::contract::{ModelEntry, ControlRange, Transport}`, `obsbot_device::descriptions::all()`, `obsbot_device::transport::replay::Replay::{new, range}`, `obsbot_device::models::tiny2lite::{decode::FOCUS_ABSOLUTE, description::{UNBACKED, UNCONFIRMED}, plan::ENTRY}`, `obsbot_proto::{Description, Setting, Support, Bound}`. If a landed name differs, the landed name wins in that one place and the builder lists it.
- **The camera (CS-1, spec 10.4):** w3-8 and w3-9 wait for it. CS-1's contract-change PRs remove or back every U-row of 9.3 (`UNBACKED` empty), set `asleep` and `settle_ms`, and decide CAM-14; only then can the Tiny 2 Lite be `tested` and the version `1.0.0-rc.1` pass the evidence test. W4 (CS-2) runs on the candidate.

## What the recordings settle (and what they do not)

The captures (USB captures on the owner's Tiny 2 Lite, keys 01 to 18) hold, besides the extension-unit frames, the camera's answers to the UVC range requests (GET_MIN, GET_MAX, GET_RES, GET_DEF) for its processing-unit and camera-terminal controls: capture 01 (firmware 6.2.7.4) for exposure time, gain, backlight compensation, brightness, contrast, hue, saturation, sharpness and white-balance temperature; captures 14, 16 and 18 (after the update to 6.2.8.1) for the same plus absolute focus and the white-balance auto default. Read on 2026-11-24 with a pcap parser (setup packets of class requests, matched to their completions); the two firmware versions answer identically. Every one of them equals the description:

| Leaf                                                             | Recorded (min, max, res, def)      | Description                         |
| ---------------------------------------------------------------- | ---------------------------------- | ----------------------------------- |
| `exposure.time`                                                  | 1, 2500, 1, 330 (100 µs units)     | 100..250000 µs, default 33000       |
| `exposure.gain`                                                  | 1, 64, 1, 1                        | 1..64, step 1, default 1            |
| `exposure.compensation`                                          | 0, 18, 1, 9                        | 0..18, step 1, default 9            |
| `image.brightness`, `contrast`, `hue`, `saturation`, `sharpness` | 0, 100, 1, 50                      | 0..100, step 1, default 50          |
| `image.white_balance.temperature`                                | 2000, 10000, 100, 5000             | 2000..10000, step 100, default 5000 |
| `focus.position`                                                 | 0, 100, 1, 0 (captures 14, 16, 18) | 0..100, step 1, default 0           |
| `image.white_balance.auto`                                       | default 1 (captures 14, 16, 18)    | default true                        |

So the 25 entries of `UNCONFIRMED` (the W0 plan's decision 12: bounds, steps and defaults without backing) are `captured`, and w3-3 records them as such. As replay fixtures (single exchanges with their capture key, spec 8.5), they run the comparison of the live test `live_the_uvc_ranges_the_description_holds` (track A, `tests/live.rs`) against the replay transport: **it does not replace that CS-1 step**: the replay returns only what the test feeds it, so it proves that the description equals the recorded answers (two firmware versions), not what the unit of the session answers; CS-1 still runs the live test on the unit. No CAM item is settled by it.

One more fact from the same answers, for **U-25 and CAM-23** (not built here, a contract matter of CS-1): the camera answers the power-line-frequency range with max 3 and default 3 (capture 01), that is, it offers UVC's `auto` value and starts with it, while the kernel driver offers 0 to 2 (spec 15 row 50). Whether the status block ever reports `auto` stays U-25.

Nothing else of W3 can be replaced by recordings: the units, the packages and the release path are camera-free, and `support: tested` is the maintainers' claim that W4 verifies.

## Decisions this plan takes

Each can be overruled; the reason follows the dash.

1. **One package keeps its name and layout.** `/usr/bin/obsbot4linux` and `/usr/bin/obsbot4linuxd`; units in `/usr/lib/systemd/user/`; bash completion `/usr/share/bash-completion/completions/obsbot4linux` (all formats); zsh completion `/usr/share/zsh/site-functions/_obsbot4linux` (Arch, rpm) and `/usr/share/zsh/vendor-completions/_obsbot4linux` (deb: Debian's zsh reads `vendor-completions`, not `site-functions`); schema `/usr/share/obsbot4linux/protocol-2.json`; README, CHANGELOG, licence as today. Tarball: both binaries, `systemd/`, `completions/`, `protocol-2.json`, README, CHANGELOG, LICENSE — spec 12.1 "the standard completion directories".
2. **Only the service line changes in the units:** `ExecStart=/usr/bin/obsbot4linuxd` and `NoNewPrivileges=yes`; the socket unit stays byte-equal — surgical; the socket path is the same as 0.13's.
3. **The unit check runs in `ci.yml`'s test job, on the debug binaries the tests built,** without a user manager. (1) The lines of the units as text, and every key of both units against the script's own list: a typo is only a warning to systemd. (2) `systemd-analyze --user verify` with a temporary `XDG_RUNTIME_DIR` (without one it fails "Failed to initialize manager", measured), any output about the units counts as failure; it is probed first on a trivial unit in a directory of its own, and where it does not work at all (the runner's systemd 249 is unproven here, measured only with 262) the step is skipped with a notice that the success line repeats, unless `CHECK_UNITS_ANALYZE=required`. (3) `systemd-socket-activate` + `setpriv --no-new-privs` + `obsbot4linuxd --fake fake-a,unknown`, started as the unit starts it (`XDG_RUNTIME_DIR` set, `OBSBOT4LINUX_SOCKET` unset), a `devices --json` through the passed socket, `NoNewPrivs: 1` in `/proc`, exit 0 on SIGTERM. `setpriv` sets the same process flag that the unit's `NoNewPrivileges=yes` sets; the unit line itself is checked as text in (1). This is the closest a runner gets to "the hardened unit starts" (CAM-27 is W4's on the real unit).
4. **Package descriptions name no model** (`Control OBSBOT webcams on Linux: a camera daemon and its command line`) — spec 8.6 allows tested models only, and the README's support table carries the levels; naming none needs no change when a model's level changes. The deb's extended description loses the 0.13 text that names the vendor app and `docs/center-parity.md`.
5. **`check()` unsets `XDG_RUNTIME_DIR` and `OBSBOT4LINUX_SOCKET`** and runs `cargo test --frozen --workspace` — the acceptance says "without `XDG_RUNTIME_DIR`"; unsetting it proves it on every machine, not only in the container.
6. **Test sockets in `$TMPDIR` with the test process id, not under the target directory** (w3-1) — measured failure in a deep build directory (Review Focus 1). The id keeps parallel runs apart; `ScriptServer` removes its socket on drop, the fake daemon harness now removes the `.lock` the daemon keeps (5.9), so a run of these tests leaves nothing behind (measured: none of their files in `$TMPDIR` after 20 runs). A `$TMPDIR` whose socket path would exceed 100 bytes falls back to `/tmp` for these few files (measured with a 94-byte `$TMPDIR`: 13, 3 and 11 tests pass, nothing left in either directory); a builder's own `$TMPDIR` is short, so the fallback never touches the shared `/tmp` there.
7. **The evidence table is per field, with explicit rows for what differs.** A value or bound without a row takes its field's level (the ledger's own rule: "where values or bounds of one row have different evidence, the evidence column names each"), with one exception: `none` is never inherited. Every mark of 9.3 that names a value, a bound or an aspect (`=auto`, `@min`, `#order`, …) has a row of level `none` of its own (38 rows, after the 136 of the fields and the recorded bounds: 174); `evidence::check` holds those rows and `UNBACKED` against each other in both directions (a mark without a row, a row without a mark, a mark whose row has another level), and `the_marks_of_9_3_follow_the_ledger` holds `UNBACKED` against the ledger's §9.3 column per field and against every evidence cell that says `none`. So the table records the truth for every value and bound without backing, and for the rest the ledger names the exceptions. The strict reading, a row for every value and bound, is not taken: it would add a few hundred rows that repeat their field's row; if the owner wants it, it is a change of 8.5 as a 15.1 row (P7). `keys()` also covers `writable`, `precision`, the components of `each` bounds, the fields of list items and the results of actions, so a row may name any of them. Keys follow `UNBACKED`'s scheme (`<path>`, `<path>=<value>`, `<path>@min|@max`, `<path>#step|#default|#precision`, `<path>[].<field>`, `do:<action>…`).
8. **The rule is strict from version 1 on, and for a `tested` model at any version** — spec 8.5 says the test fails for any registered model with another level; on `rebuild` before CS-1 the U-rows still stand (they are listed in `UNBACKED`, and the test pins that list exactly), so the strict form is switched on by the version `1.0.0-rc.1` (w3-9) and by `support: tested` (w3-8). A U-row cannot reach a release.
9. **`evidence::of(key)` maps registry keys to tables in `obsbot-device`, not a field of `ModelEntry`** — a field would be a contract change (`contract/bus.rs`); a registered model without a table fails the test, so a new model cannot forget it.
10. **The levels and the marks follow the ledger word for word** (`the_field_rows_follow_the_ledger` and `the_marks_of_9_3_follow_the_ledger` read `docs/field-ledger.md`) — the ledger stays the one place a person edits; the constants cannot drift from it. `levels.rs` is a snapshot of the ledger and of `UNBACKED` taken on 2026-11-24: if either has changed when w3-3 is built, or after CS-1's contract-change PRs, the builder regenerates the file with the script of Step 3 instead of pasting it, and the tests say what differs.
11. **The release candidate is tagged on `rebuild`.** The guard accepts a tag `v1.0.0-rc.*` whose commit is on `origin/main` or `origin/rebuild`, every other tag only on `main`; `release-green.sh` (green as a pull request) and `changelog-section.sh` (the tag equals `Cargo.toml`'s version) run for both — spec 10.6 publishes the candidate before `main` moves. The exception names `v1.0.0-rc.*` only, not any `v*-rc.*`: spec 10.1 removes the `rebuild` ruleset after the final merge and 10.7 says every later release, a candidate included, comes from `main` (an rc of 1.1 is tagged on `main`, `changelog-section.sh` accepts it). W4's release pull request removes the exception and `rebuild` from `ci.yml`'s triggers (w3-9 Step 7). Who can create such a tag: anyone with write access, as for `vX.Y.Z` today (the tag ruleset blocks only update and deletion); the commit must still be a green pull-request head with that exact version, so a tag cannot publish another commit's code.
12. **A candidate is a GitHub pre-release and does not deploy the project page** (`--prerelease`; the `pages` job skipped for `-rc.`): the page shows the latest release, which a pre-release never is.
13. **Version spellings** as Review Focus 4: `changelog-section.sh` accepts `X.Y.Z` and `X.Y.Z-rc.N` (N ≥ 1) only, and wants `pkgver` without the `-`; `build-arch.sh` and `package-contents.sh` derive the Arch form; the rpm job passes `-s 'version = "<ver with ~>"'` always (a plain version passes unchanged, measured).
14. **The CHANGELOG's `[Unreleased]` becomes the 1.0 notes** (w3-6): the entries that describe 0.13 text (README and package wording, `audio --help`, `docs/center-parity.md` for 0.13, the workspace note of stage G) are dropped, because 1.0 removed what they describe; the repository move and the `PKGBUILD` fix stay. w3-9 renames the section to `[1.0.0-rc.1]`; W4 renames that heading to `[1.0.0]` and adds what changed since, so that `changelog-section.sh v1.0.0` prints the migration notes too (a new section above the candidate's would print only the difference). The notes end with the rollback line of spec 10.6 step 5 (the 0.13.2 packages and widget 0.29.2). If `main` has released 0.14.0 by then, the rebase keeps its section and w3-9 removes what the 1.0 notes repeat.
15. **The command map is generated from spec 9.1** (both command columns verbatim; the note column without spec-internal keys such as U-n, CAM-n and section numbers), and the exceptions PX-1 to PX-7 are copied from 9.2. w3-9 adds the PX-U-n exceptions CS-1 produced and re-checks rows that CS-1 changed (CAM-14's `focus auto off`). The source of the prose is spec 12.2; W2's draft `docs/migration-1.0.md` is compared with it (w3-6 Step 3).
16. **The README is rewritten for 1.0** (the 0.13 README documents the removed commands and states how knowledge was obtained, which is not allowed until P5 is decided): what the tool is, the support table, install and upgrade, the generic command line, links to the protocol and the ledger, the licence section unchanged — spec 10.3 asks for the support table and for removing the static-switch statements; the rest of the 0.13 text is wrong for 1.0. Its wording holds at candidate time, when only CS-1 has run and W4 verifies the parity rows afterwards (8.8).
17. **No `-j` or `nice` in `PKGBUILD` or the workflows** — they run on dedicated runners or a user's own build; the machine rule applies to builders on the owner's desktop only.
18. **The contents job also checks the claims of spec 12.1** (w3-4): no `libudev` in `NEEDED` of either binary, no install script in any package (so none enables the socket unit), and the glibc floor of the README (2.34) for the deb, rpm and tarball binaries. The Arch package is built on current Arch and has no floor: measured 2026-11-24, the same release build needs `GLIBC_2.34` in the tarball, deb and rpm, and `GLIBC_2.39` in the package `makepkg` builds on this desktop, so the README names the floor for those three only.

## Product questions (recommendations, not built)

All of them were decided as recommended in the cross-check of 2026-11-24, for the owner to review later; the build order follows those answers.

- **P1.** The project page (`site/`) describes 0.13. Recommendation: rewrite it in W4's release pull request, before `v1.0.0`; the candidate does not deploy it (decision 12), so it can wait.
- **P2.** `docs/center-parity.md` describes 0.13 and names the vendor app throughout. Recommendation: delete it before the candidate; W3's packages and README no longer link it. **Decided as recommended (cross-check 2026-11-24): w3-5 deletes it together with the README that links it** (W2 is complete by then and has no task for it; deleting it in the same pull request as the README leaves no dangling link on `rebuild`; the other links, in the CHANGELOG's `[Unreleased]` and the deb description, go in the same round with w3-6 and w3-4).
- **P3.** The firmware column of the README for the tested Tiny 2 Lite. Recommendation: the version the CS-1 unit reports (`obsbot4linux --json devices`, field `firmware`), filled by w3-8; no value is written before.
- **P4.** The daemon tests of track B leave one `o4ld-<pid>-<n>` directory per run in `$TMPDIR` (measured again on 2026-11-24 after a workspace test run). Recommendation: a small B follow-up removes them in its harness; not part of W3 (B's files). With the task's own `$TMPDIR` they stay inside the build directory.
- **P5.** Public text. Spec 8.5 allows "recorded from the vendor app" and "measured on the camera" as the only statements about how knowledge was obtained; the run's brief forbids any statement that knowledge comes from the vendor's app. They conflict. Recommendation: the brief's rule wins for the public text (README, CHANGELOG, code comments, package text), the spec's second phrase ("measured on the camera") stays free to use, and the spec gets a 15.1 row that drops the first phrase. Until then this plan says nothing about the origin of knowledge (captures are cited by key only). **Decided as recommended (cross-check 2026-11-24); the owner reviews it; the 15.1 row is the coordinator's to draft.**
- **P6.** W3 before W2. Spec 10.3 says W3 depends on W2, and W2 waits for CS-1. w3-1 to w3-7 need no code of W2. Recommendation: allow w3-1 to w3-7 to start once their own gates hold, because the packaging, the release path and the CHANGELOG then get their review time before the camera session; w3-8 and w3-9 still wait for CS-1. If the owner prefers the spec's order, the gate `w2` stands as the JSON carries it. **Decided as recommended (cross-check 2026-11-24), in the form the build order needs: W3 starts after every W2 task without the gate `camera` has landed; w2-7 and w2-8 come after CS-1; the owner reviews it.**
- **P7.** Level per value and bound. The evidence test records `none` per value and bound but lets a value or bound inherit its field's `captured` or `measured` (decision 7). If the owner reads 8.5 as one explicit level for every value and bound, that is a table a few hundred rows longer and a 15.1 row; recommendation: keep the inheritance, which the ledger's own rule states.

## File structure

| File                                                                                        | Responsibility                                                                                      | Task             |
| ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ---------------- |
| `docs/superpowers/plans/2026-11-24-obsbot4linux-w3-packaging.md` (new)                      | this plan, on `rebuild`                                                                             | w3-0             |
| `crates/obsbot-proto/tests/client.rs`, `tests/client_subscribe.rs` (one helper each)        | test sockets in `$TMPDIR`                                                                           | w3-1             |
| `crates/obsbot4linux/tests/common/mod.rs` (one helper), `tests/fake_daemon/mod.rs` (`halt`) | test sockets in `$TMPDIR`; the harness removes the daemon's lock file                               | w3-1             |
| `packaging/systemd/obsbot4linux.service`                                                    | `ExecStart=/usr/bin/obsbot4linuxd`, `NoNewPrivileges=yes`                                           | w3-2             |
| `scripts/check-units.sh` (new)                                                              | the camera-free unit check                                                                          | w3-2             |
| `.github/workflows/ci.yml` (one step)                                                       | runs the unit check                                                                                 | w3-2             |
| `.github/workflows/ci.yml` (one `case` line, one comment)                                   | the docs-only rule counts the ledger and the protocol document as code, because tests read them     | w3-3             |
| `crates/obsbot-device/src/evidence.rs` (new), `src/lib.rs` (one line)                       | levels, rows, `of`, `keys`, `field_of`, `check` (none rows against marks)                           | w3-3             |
| `crates/obsbot-device/src/models/tiny2lite/levels.rs` (new), `mod.rs` (one line)            | the Tiny 2 Lite's table                                                                             | w3-3             |
| `crates/obsbot-device/src/models/tiny2lite/description.rs` (`UNCONFIRMED`)                  | emptied: the recordings back all 25                                                                 | w3-3             |
| `crates/obsbot-device/tests/evidence.rs` (new)                                              | the evidence-level test, the marks against the ledger and the recorded UVC ranges                   | w3-3             |
| `crates/obsbot-device/tests/live.rs` (one comment)                                          | no longer says the bounds are not backed                                                            | w3-3             |
| `docs/field-ledger.md` (ten evidence cells, by script)                                      | the bounds' evidence                                                                                | w3-3             |
| `crates/obsbot4linux/Cargo.toml` (`description`, deb and rpm tables)                        | deb and rpm contents                                                                                | w3-4             |
| `packaging/arch/PKGBUILD` (its `jq` lines are W2's w2-9)                                    | Arch contents, `check()`, `checkdepends` `jq`; later the `pkgver` comment and value                 | w3-4, w3-7, w3-9 |
| `.github/workflows/release.yml`                                                             | two binaries in every package, the claims of 12.1 in the `contents` job; later the candidate's path | w3-4, w3-7       |
| `packaging/contents.txt`                                                                    | the expected package listing                                                                        | w3-4             |
| `README.md`                                                                                 | the 1.0 README; later the tested row                                                                | w3-5, w3-8       |
| `docs/center-parity.md` (delete)                                                            | the 0.13 document that the README linked (P2)                                                       | w3-5             |
| `CHANGELOG.md`                                                                              | the 1.0 notes; later the support line, the candidate's heading                                      | w3-6, w3-8, w3-9 |
| `scripts/changelog-section.sh`, `scripts/build-arch.sh`, `scripts/package-contents.sh`      | the candidate's version spellings                                                                   | w3-7             |
| `crates/obsbot-device/src/models/tiny2lite/plan.rs` (`ENTRY.support` and its test)          | `tested`                                                                                            | w3-8             |
| `crates/obsbot4linux/tests/replay.rs` (two marked lines, W2's file)                         | `tested` in the hook-up test                                                                        | w3-8             |
| `Cargo.toml` (`version`), `Cargo.lock`                                                      | `1.0.0-rc.1`                                                                                        | w3-9             |

## Tasks, waves and dependencies

A task branches from `rebuild` once its gates and its dependencies have landed. Tasks of one wave touch disjoint files, with one exception: w3-2 and w3-3 both edit `.github/workflows/ci.yml`, in different places (a step at the end of the test job, the docs-only `case` of the `changes` job); the build order of the cross-check (2026-11-24) puts them into different rounds. Only w3-9 changes `Cargo.lock`. Gates, as the JSON's `gate` field spells them (joined by `+`): `w2` — the W2 tasks without the gate `camera` have landed (spec 10.3: W3 depends on W2; P6, defined below); `w1:<key>` — that W1 task is on `rebuild` (both named ones are, since `05e66e9`); `cs-1` — camera session CS-1 has run and its contract-change PRs have landed, so that `UNBACKED` is empty (spec 10.4); `publish` — the owner's go for a step that publishes or claims `tested`. A gated task is not issued before its gates hold: the owner's go alone does not start w3-8 or w3-9. w3-7 also carries `land_gate`: one Opus review with the forge lens and the security check before it lands.

**Gate `w2`, as decided in the cross-check of 2026-11-24 (P6 taken as recommended):** every W2 task that is not gated `camera` has landed (w2-0 to w2-6 and w2-9). w2-7 and w2-8 follow CS-1 and are not waited for by w3-0 to w3-7; for w3-8 and w3-9 the gate `cs-1` means that CS-1 has run and its contract-change pull requests, w2-7 and w2-8, have landed.

| Task | Title                                           | Wave | Depends on       | Gate                        | Effort |
| ---- | ----------------------------------------------- | ---- | ---------------- | --------------------------- | ------ |
| w3-0 | This plan on `rebuild` (docs-only pull request) | 0    | –                | `w2`                        | S      |
| w3-1 | Test sockets outside the target directory       | 1    | w3-0             | `w2+w1:c-13-parity-rows`    | S      |
| w3-2 | The 1.0 units and their check                   | 1    | w3-0             | `w2`                        | S      |
| w3-3 | The evidence-level test (8.5)                   | 1    | w3-0             | `w2+w1:a-14-remove-carried` | M      |
| w3-5 | The 1.0 README                                  | 1    | w3-0             | `w2`                        | S      |
| w3-6 | The 1.0 CHANGELOG and migration notes           | 1    | w3-0             | `w2`                        | M      |
| w3-4 | Two-binary packages                             | 2    | w3-1             | `w2`                        | M      |
| w3-8 | The Tiny 2 Lite at `support: tested`            | 2    | w3-3, w3-5, w3-6 | `w2+cs-1+publish`           | S      |
| w3-7 | The release path of a candidate                 | 3    | w3-4             | `w2` (and `land_gate`)      | M      |
| w3-9 | Release candidate `v1.0.0-rc.1`                 | 4    | w3-2, w3-7, w3-8 | `w2+cs-1+publish`           | M      |

w3-1 to w3-7 need no camera. w3-8 and w3-9 wait for CS-1 and its contract-change PRs (every U-row of 9.3 backed or removed), and then for the owner's go. w3-9 depends on w3-2 as well: without the 1.0 service unit the candidate would ship `ExecStart=/usr/bin/obsbot4linux daemon`, a command 1.0 removed, and the socket unit would start nothing. The code below was built and run in a scratch worktree of `rebuild` at `05e66e9` (see "Proof"; the first draft was built on `4499637`, which did not yet hold `c-13-parity-rows` and `a-14-remove-carried`).

---

### Task w3-0 (key `w3-0-plan-doc`): This plan on `rebuild`

A docs-only pull request, before any builder of W3 starts: the tasks name this document as their `plan_doc`, and a document that is not in the repository cannot be a builder's contract (the plans of W0, W1 and W2 landed the same way). Added in the cross-check of 2026-11-24 (coordinator note 7 asked for it; the task list had no entry).

**Files:**

- Create: `docs/superpowers/plans/2026-11-24-obsbot4linux-w3-packaging.md` (this document, as the coordinator holds it)

- [ ] **Step 1: Check the text.** `grep -nEi '/hom[e]/|thom[a]s' docs/superpowers/plans/2026-11-24-obsbot4linux-w3-packaging.md` prints exactly one line: the unchanged `maintainer = …` context line of `crates/obsbot4linux/Cargo.toml` in w3-4 Step 1, which is already public in that file; anything else is a private path or name and is removed. `grep -n 'sn:[A-Z0-9]\{8,\}' …` finds only synthetic serials. No sentence says that knowledge comes from the vendor's software (P5).
- [ ] **Step 2: Format.** `prettier --check` on the file if installed.
- [ ] **Step 3: Commit.** `git add docs/superpowers/plans/2026-11-24-obsbot4linux-w3-packaging.md && git commit -m "docs: plan for stage W3 (packaging and release candidate)"`. The pull request changes docs only: the `required` gate reports without running fmt, clippy and test. Afterwards the worktree that has `rebuild` checked out is fast-forwarded, so that every task's `plan_doc` resolves.

---

### Task w3-1 (key `w3-1-test-sockets`): Test sockets outside the target directory

**Files:**

- Modify: `crates/obsbot-proto/tests/client.rs` (the helper `socket`), `crates/obsbot-proto/tests/client_subscribe.rs` (the helper `socket`)
- Modify: `crates/obsbot4linux/tests/common/mod.rs` (the helper `socket`), `crates/obsbot4linux/tests/fake_daemon/mod.rs` (`FakeDaemon::halt`)

**Interfaces:** unchanged (`fn socket(name: &str) -> PathBuf` in each file; `FakeDaemon::halt(&mut self)`).

**Gate:** `w2` and `w1:c-13-parity-rows`: Step 2 runs `--test parity`, which `crates/obsbot4linux/tests/parity.rs` of that W1 task provides (it is on `rebuild` since `05e66e9`).

**Why:** `makepkg`'s `check()` (spec 10.3 W3 acceptance) runs the tests in the source directory, which a package build puts deep down. Measured on 2026-11-24 with a build directory 87 bytes long: 12 tests of `tests/client.rs` failed with `bind the script socket: … "path must be shorter than SUN_LEN"`; the command line's helper asserts at most 100 bytes and would fail the same way. Track B's tests already use `std::env::temp_dir()`. A `$TMPDIR` that is itself too long (a user's makepkg environment) would break these tests the same way, so a path over 100 bytes falls back to `/tmp` (the three helpers; the 100 is the limit the command line's helper already asserts).

- [ ] **Step 1: Change the four places**

```diff
diff --git a/crates/obsbot-proto/tests/client.rs b/crates/obsbot-proto/tests/client.rs
index dd69da8..cb56d1e 100644
--- a/crates/obsbot-proto/tests/client.rs
+++ b/crates/obsbot-proto/tests/client.rs
@@ -11,8 +11,17 @@ use serde_json::json;
 use std::path::PathBuf;
 use std::time::Duration;

+/// `$TMPDIR/o4l-c1-<pid>-<name>.sock`. Not under the target directory: a Unix socket path
+/// holds at most 107 bytes, and a package build runs these tests in a deep directory. A
+/// `$TMPDIR` that is itself too long (a user's makepkg environment) falls back to `/tmp`.
 fn socket(name: &str) -> PathBuf {
-    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c1-{name}.sock"))
+    let file = format!("o4l-c1-{}-{name}.sock", std::process::id());
+    let path = std::env::temp_dir().join(&file);
+    if path.as_os_str().len() <= 100 {
+        path
+    } else {
+        PathBuf::from("/tmp").join(file)
+    }
 }

 const HELLO: &str =
diff --git a/crates/obsbot-proto/tests/client_subscribe.rs b/crates/obsbot-proto/tests/client_subscribe.rs
index 35c31b1..e850853 100644
--- a/crates/obsbot-proto/tests/client_subscribe.rs
+++ b/crates/obsbot-proto/tests/client_subscribe.rs
@@ -10,8 +10,17 @@ use serde_json::json;
 use std::path::PathBuf;
 use std::time::Duration;

+/// `$TMPDIR/o4l-c10-<pid>-<name>.sock`. Not under the target directory: a Unix socket path
+/// holds at most 107 bytes, and a package build runs these tests in a deep directory. A
+/// `$TMPDIR` that is itself too long (a user's makepkg environment) falls back to `/tmp`.
 fn socket(name: &str) -> PathBuf {
-    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c10-{name}.sock"))
+    let file = format!("o4l-c10-{}-{name}.sock", std::process::id());
+    let path = std::env::temp_dir().join(&file);
+    if path.as_os_str().len() <= 100 {
+        path
+    } else {
+        PathBuf::from("/tmp").join(file)
+    }
 }

 fn golden(name: &str) -> String {
diff --git a/crates/obsbot4linux/tests/common/mod.rs b/crates/obsbot4linux/tests/common/mod.rs
index 3c65736..2df1404 100644
--- a/crates/obsbot4linux/tests/common/mod.rs
+++ b/crates/obsbot4linux/tests/common/mod.rs
@@ -15,10 +15,17 @@ use std::collections::BTreeMap;
 use std::ffi::OsString;
 use std::path::{Path, PathBuf};

-/// A socket path for one test, unique per name. The name is kept short: a Unix socket path
-/// holds at most 107 bytes, and a daemon adds `.lock` and `.test` (5 bytes) to its path.
+/// A socket path for one test, unique per name and test process: `$TMPDIR/o4l-c-<pid>-<name>`.
+/// It is not under the target directory, which a package build puts deep down; the name is
+/// kept short: a Unix socket path holds at most 107 bytes, and a daemon adds `.lock` and
+/// `.test` (5 bytes) to its path. A `$TMPDIR` that is itself too long (a user's makepkg
+/// environment) falls back to `/tmp`.
 pub fn socket(name: &str) -> PathBuf {
-    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c-{name}"));
+    let file = format!("o4l-c-{}-{name}", std::process::id());
+    let mut path = std::env::temp_dir().join(&file);
+    if path.as_os_str().len() > 100 {
+        path = PathBuf::from("/tmp").join(file);
+    }
     assert!(
         path.as_os_str().len() <= 100,
         "socket path too long for a Unix socket: {}",
diff --git a/crates/obsbot4linux/tests/fake_daemon/mod.rs b/crates/obsbot4linux/tests/fake_daemon/mod.rs
index 7e09ffb..86839bc 100644
--- a/crates/obsbot4linux/tests/fake_daemon/mod.rs
+++ b/crates/obsbot4linux/tests/fake_daemon/mod.rs
@@ -84,6 +84,10 @@ impl FakeDaemon {
         self.stop.store(true, Ordering::SeqCst);
         if let Some(thread) = self.thread.take() {
             let ended = thread.join().expect("the daemon thread does not panic");
+            // the daemon keeps its `<socket>.lock` (5.9); the test's daemon is gone, so is it
+            let mut lock = self.socket.clone().into_os_string();
+            lock.push(".lock");
+            let _ = std::fs::remove_file(lock);
             assert!(ended.is_ok(), "run returned {ended:?}");
         }
     }
```

`std::path::PathBuf` stays imported in both proto test files (other code uses it; `cargo clippy` shows an unused import if not).

- [ ] **Step 2: Run the changed tests twenty times**

Run (from the task's worktree, `CARGO_TARGET_DIR` and `TMPDIR` under the task's build directory):

```sh
for i in $(seq 20); do
  nice -n 19 cargo test -q -j 2 --locked -p obsbot-proto --test client --test client_subscribe \
    -p obsbot4linux --test e2e --test read --test write --test watch --test parity \
    --test check_model --test replay || break
done; ls "$TMPDIR"
```

Expected: 20 green runs; no `o4l-*` file left in `$TMPDIR` (measured on `05e66e9`: 20 of 20, nothing left). `check_model` and `replay` are W2's (w2-2, w2-6): they take their sockets from the same `common::socket`, and their harnesses remove the daemon's `<socket>.lock` when they drop (W2 cross-check correction 1), so they leave nothing either; `check_model` needs `bash` and `jq` (not measured on `05e66e9`, which predates W2: added in the cross-check of 2026-11-24).

- [ ] **Step 3: The deep-directory case**

Run the proto client tests with a target directory of exactly 100 bytes, and then with a `$TMPDIR` that is too long:

```sh
deep=$(python3 -I -c 'import os; b=os.environ["CARGO_TARGET_DIR"] + "/"; print(b + "a" * (100 - len(b)))')
CARGO_TARGET_DIR=$deep nice -n 19 cargo test -q -j 2 --locked -p obsbot-proto --test client
rm -rf "$deep"
long=$TMPDIR/$(python3 -I -c 'print("d" * 50)'); mkdir -p "$long"       # 94 bytes here: over the limit
TMPDIR=$long nice -n 19 cargo test -q -j 2 --locked -p obsbot-proto --test client --test client_subscribe -p obsbot4linux --test e2e
ls "$long" /tmp | grep -c o4l-; rm -rf "$long"
```

Expected: `13 passed` for the first command. Before the change the same command fails 12 of the 13 tests with "path must be shorter than SUN_LEN" (measured on 2026-11-24: 1 passed, 12 failed). The second run passes (13, 3 and 11 tests) with the sockets in `/tmp` and leaves nothing in either directory: the `grep -c` prints `0`. The target directory must be shorter than 100 bytes for the first command; a builder's is.

- [ ] **Step 4: Workspace gate** — `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`: green, the ignored tests are the live tests and the zsh syntax test.

- [ ] **Step 5: Commit** — `tests: sockets in TMPDIR, not under the target directory` (signed).

### Task w3-2 (key `w3-2-units`): The 1.0 units and their check

**Files:**

- Modify: `packaging/systemd/obsbot4linux.service`
- Create: `scripts/check-units.sh` (mode 755)
- Modify: `.github/workflows/ci.yml` (one step in job `test`)

**Interfaces:** `scripts/check-units.sh BINDIR` — exit 0 and one line on success (it names a skipped `systemd-analyze` step when there is one); exit 1 with the reason. `CHECK_UNITS_ANALYZE=required` turns that skip into a failure. The script uses tabs, as the repository's formatter hook leaves shell files.

**Gate:** `w2` (spec 10.3).

- [ ] **Step 1: The service**

```diff
diff --git a/packaging/systemd/obsbot4linux.service b/packaging/systemd/obsbot4linux.service
index 6702576..2b3bdf0 100644
--- a/packaging/systemd/obsbot4linux.service
+++ b/packaging/systemd/obsbot4linux.service
@@ -4,5 +4,6 @@ Requires=obsbot4linux.socket
 After=obsbot4linux.socket

 [Service]
-ExecStart=/usr/bin/obsbot4linux daemon
+ExecStart=/usr/bin/obsbot4linuxd
+NoNewPrivileges=yes
 Restart=on-failure
```

`packaging/systemd/obsbot4linux.socket` stays as it is.

- [ ] **Step 2: The check script**

Create `scripts/check-units.sh`:

```sh
#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Checks the systemd user units of packaging/systemd without a camera and without a user
# manager (design spec 5.10, 10.3 W3, 12.1):
#  1. the service starts /usr/bin/obsbot4linuxd and has the line NoNewPrivileges=yes, the
#     socket listens on %t/obsbot4linux.sock, and every key of both units is one this script
#     knows (to systemd an unknown key is only a warning, so a typo would ship unnoticed);
#  2. systemd-analyze accepts both units, with the service pointed at the built daemon, and says
#     nothing about them. Where `systemd-analyze --user verify` does not work at all (it is
#     probed on a trivial unit first; the CI runner's systemd is older than a desktop's), this
#     step is skipped with a notice and the success line says so; CHECK_UNITS_ANALYZE=required
#     turns the skip into a failure;
#  3. the daemon, started as the socket unit starts it (the listening socket passed in
#     LISTEN_FDS, XDG_RUNTIME_DIR set and OBSBOT4LINUX_SOCKET not, so that its paths come from
#     the unit's own environment) and under no_new_privs, answers on that socket and lists its
#     devices, and ends with exit code 0 on SIGTERM. It runs on fake devices: no camera is
#     opened. The unit's NoNewPrivileges=yes is checked as a line (1); its effect is shown by
#     setpriv --no-new-privs, which sets the same process flag.
# Usage: scripts/check-units.sh BINDIR    (BINDIR holds the built obsbot4linuxd and obsbot4linux)
# Needs systemd-analyze, systemd-socket-activate, setpriv and timeout. The work directory is
# made under $TMPDIR, or under /tmp when $TMPDIR would make the socket path too long.
set -eu
bindir=$(cd "${1:?usage: check-units.sh BINDIR}" && pwd)
units=$(cd "$(dirname "$0")/../packaging/systemd" && pwd)
fail() {
	echo "check-units: $*" >&2
	exit 1
}

# 1. the units as text
grep -qx 'ExecStart=/usr/bin/obsbot4linuxd' "$units/obsbot4linux.service" ||
	fail "obsbot4linux.service does not start /usr/bin/obsbot4linuxd"
grep -qx 'NoNewPrivileges=yes' "$units/obsbot4linux.service" ||
	fail "obsbot4linux.service lacks NoNewPrivileges=yes"
grep -qx 'ListenStream=%t/obsbot4linux.sock' "$units/obsbot4linux.socket" ||
	fail "obsbot4linux.socket does not listen on %t/obsbot4linux.sock"
for unit in obsbot4linux.service obsbot4linux.socket; do
	keys=$(sed -n 's/^\([A-Za-z0-9]*\)=.*/\1/p' "$units/$unit")
	for key in $keys; do
		case $key in
		Description | Requires | After | ExecStart | Restart | NoNewPrivileges) ;;
		ListenStream | SocketMode | WantedBy) ;;
		*) fail "$unit uses the key $key, which this script does not know (a typo? else add it to the list in scripts/check-units.sh)" ;;
		esac
	done
done

work=$(mktemp -d "${TMPDIR:-/tmp}/check-units.XXXXXX")
if [ "${#work}" -gt 80 ]; then
	rm -rf "$work"
	work=$(mktemp -d "/tmp/check-units.XXXXXX")
fi
daemon=
cleanup() {
	if [ -n "$daemon" ]; then kill "$daemon" 2>/dev/null || true; fi
	rm -rf "$work"
}
trap cleanup EXIT
trap 'exit 1' INT TERM
mkdir -m 700 "$work/run" "$work/units" "$work/probe"

# 2. systemd-analyze, with ExecStart on the built binary (it checks that it is executable)
# The probe has a directory of its own: verify also loads the other units of the directory.
analyze() { # DIR FILE...: the output of `systemd-analyze --user verify FILE...` run in DIR
	dir=$1
	shift
	(cd "$dir" && env -u DBUS_SESSION_BUS_ADDRESS XDG_RUNTIME_DIR="$work/run" \
		systemd-analyze --user verify "$@" 2>&1)
}
sed "s|^ExecStart=/usr/bin/obsbot4linuxd\$|ExecStart=$bindir/obsbot4linuxd|" \
	"$units/obsbot4linux.service" >"$work/units/obsbot4linux.service"
cp "$units/obsbot4linux.socket" "$work/units/obsbot4linux.socket"
printf '[Service]\nExecStart=/bin/true\n' >"$work/probe/probe.service"
skipped=
if probe=$(analyze "$work/probe" ./probe.service) && [ -z "$probe" ]; then
	out=$(analyze "$work/units" ./obsbot4linux.service ./obsbot4linux.socket) ||
		fail "systemd-analyze refused the units:
$out"
	if printf '%s\n' "$out" | grep obsbot4linux; then
		fail "systemd-analyze warns about the units (above)"
	fi
else
	skipped="systemd-analyze --user verify does not work here ($(printf '%s' "$probe" | head -n1))"
	[ "${CHECK_UNITS_ANALYZE:-}" != required ] || fail "$skipped"
	echo "check-units: notice: $skipped; its step is skipped" >&2
fi

# 3. socket activation under no_new_privs, on fake devices, with the unit's environment: the
# runtime directory, no socket override
sock=$work/run/obsbot4linux.sock
[ "${#sock}" -le 100 ] || fail "the socket path is too long for a Unix socket: $sock"
systemd-socket-activate -l "$sock" env -u OBSBOT4LINUX_SOCKET XDG_RUNTIME_DIR="$work/run" \
	setpriv --no-new-privs "$bindir/obsbot4linuxd" --fake fake-a,unknown \
	>"$work/daemon.log" 2>&1 &
daemon=$!
tries=0
until [ -S "$sock" ]; do
	tries=$((tries + 1))
	[ "$tries" -le 100 ] || fail "no listening socket after 10 s"
	sleep 0.1
done
list=$(env -u OBSBOT4LINUX_SOCKET XDG_RUNTIME_DIR="$work/run" timeout 40 \
	"$bindir/obsbot4linux" --json devices) ||
	fail "obsbot4linux devices failed on the activated socket; daemon log:
$(cat "$work/daemon.log")"
for model in fake-a unknown; do
	printf '%s\n' "$list" | grep -q "\"model\":\"$model\"" ||
		fail "the device list lacks $model: $list"
done
grep -qx 'NoNewPrivs:[[:space:]]*1' "/proc/$daemon/status" ||
	fail "the daemon does not run with no_new_privs"
kill -TERM "$daemon"
status=0
wait "$daemon" || status=$?
daemon=
[ "$status" -eq 0 ] || fail "the daemon ended with $status on SIGTERM; log:
$(cat "$work/daemon.log")"
echo "check-units: units accepted; the socket-activated daemon answered and ended cleanly${skipped:+ ($skipped)}"
```

- [ ] **Step 3: Run it, and see it fail for the right reasons**

```sh
nice -n 19 cargo build -j 2 --locked -p obsbot4linux -p obsbot4linuxd
scripts/check-units.sh "$CARGO_TARGET_DIR/debug"
```

Expected: `check-units: units accepted; the socket-activated daemon answered and ended cleanly`.

Then, each on a scratch copy of the service file and restored afterwards (all measured on 2026-11-24 with systemd 262):

| Change                                                                                                         | Result                                                                                                                                                       |
| -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| add a line `NoNewPrivilegs=yes`                                                                                | exit 1, `check-units: obsbot4linux.service uses the key NoNewPrivilegs, which this script does not know (a typo? …)`                                         |
| add `SocketMode=0600` under `[Service]` (a known key in the wrong section: the script's list passes it)        | exit 1, systemd-analyze's line `… Unknown key 'SocketMode' in section [Service], ignoring.` and `check-units: systemd-analyze warns about the units (above)` |
| remove `NoNewPrivileges=yes`                                                                                   | exit 1, `obsbot4linux.service lacks NoNewPrivileges=yes`                                                                                                     |
| a `systemd-analyze` that fails first in `PATH` (a shim that prints `Failed to initialize manager` and exits 1) | exit 0, a `notice:` line, and the success line ends with `(systemd-analyze --user verify does not work here (…))`                                            |
| the same shim with `CHECK_UNITS_ANALYZE=required`                                                              | exit 1, `check-units: systemd-analyze --user verify does not work here (…)`                                                                                  |
| `TMPDIR` of 104 bytes                                                                                          | exit 0 (the work directory falls back to `/tmp`)                                                                                                             |

Run the passing case 20 times in a loop: 20 of 20, nothing left in `$TMPDIR`. `shellcheck scripts/check-units.sh` is clean. The binaries of a plain `cargo test --workspace` are enough: after the workspace gate of this revision, `scripts/check-units.sh target/debug` passed.

- [ ] **Step 4: CI**

```diff
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -81,6 +81,8 @@
       - run: rustup toolchain install "$RUSTUP_TOOLCHAIN" --profile minimal --no-self-update
       - run: cargo test --workspace --locked
       - run: scripts/check-cli-deps.sh
+      # the user units, with the binaries the tests built (no camera, no user manager)
+      - run: scripts/check-units.sh target/debug

   # Builds and tests with the oldest Rust that Cargo.toml's rust-version claims.
   msrv:
```

`cargo test --workspace` builds `target/debug/obsbot4linux` and `obsbot4linuxd` (measured: the script passes on the binaries of a workspace test run). `systemd-socket-activate`, `systemd-analyze` and `setpriv` are part of systemd and util-linux on `ubuntu-22.04`. The runner's systemd 249 is unproven here (measured with 262 only). The script does not depend on it: if `--user verify` does not work on 249, step 2 is skipped with a notice, the key list and step 3 still run, and the builder reports the notice in the pull request as a finding (not a plan mismatch); the owner may then accept the skip or make the step required. Acceptance: the pull request's `test` job prints the script's success line, with or without that notice.

- [ ] **Step 5: Gate and commit** — `actionlint .github/workflows/ci.yml`, `shellcheck scripts/check-units.sh`, the workspace gate; commit `packaging: the service starts obsbot4linuxd with NoNewPrivileges; scripts/check-units.sh in CI`. Acceptance: the pull request's `test` job prints the script's success line.

### Task w3-3 (key `w3-3-evidence-levels`): The evidence-level test (8.5)

**Files:**

- Create: `crates/obsbot-device/src/evidence.rs`; Modify: `crates/obsbot-device/src/lib.rs` (one line)
- Create: `crates/obsbot-device/src/models/tiny2lite/levels.rs`; Modify: `crates/obsbot-device/src/models/tiny2lite/mod.rs` (one line)
- Modify: `crates/obsbot-device/src/models/tiny2lite/description.rs` (`UNCONFIRMED` only)
- Create: `crates/obsbot-device/tests/evidence.rs`
- Modify: `crates/obsbot-device/tests/live.rs` (one comment: the bounds are no longer "not backed yet")
- Modify: `docs/field-ledger.md` (the evidence cell of ten rows, by script)
- Modify: `.github/workflows/ci.yml` (the docs-only rule: the ledger and `docs/protocol-2.md` count as code, because tests read them)

**Interfaces (module `obsbot_device::evidence`):**

- `pub enum Level { Captured, Measured, Daemon, None }`
- `pub struct Row { pub key: &'static str, pub level: Level, pub evidence: &'static str }`
- `pub struct Evidence { pub levels: &'static [Row], pub unbacked: &'static [(&'static str, &'static str)], pub unconfirmed: &'static [&'static str] }`
- `pub fn of(key: &str) -> Option<Evidence>`; `pub fn keys(d: &Description) -> BTreeSet<String>` (fields, values and `writable`, bounds with the components of `each` bounds, steps, defaults, `precision`, list item fields, action parameters and results); `pub fn field_of(key: &str, d: &Description) -> Option<String>`; `pub fn check(entry: &ModelEntry, d: &Description, e: &Evidence, strict: bool) -> Vec<String>` (rows against the description, levels against evidence texts, every row of level `none` against `unbacked` and every mark of `unbacked` against a row of level `none`, a mark on a whole field excepted)
- `obsbot_device::models::tiny2lite::levels::{EVIDENCE, LEVELS}`

**Gate:** `w2+w1:a-14-remove-carried` (it edits `lib.rs`, `models/tiny2lite/mod.rs` and `registry.rs`; this task adds one line to each of the first two and reads `registry::REGISTRY`). The one-line additions go next to the module lines that remain after a-14 (the diffs below are made on `05e66e9`, where `pub mod legacy;` is gone).

- [ ] **Step 1: The ledger cells**

In `docs/field-ledger.md`, section "State", append to the **Evidence** cell of these rows:

| Row                                                                                                                                                           | Append to its Evidence cell                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `exposure.time`, `exposure.gain`, `image.brightness`, `image.contrast`, `image.saturation`, `image.hue`, `image.sharpness`, `image.white_balance.temperature` | `; bounds, step and default: capture 01 (UVC range answers)` |
| `focus.position`, `image.white_balance.auto`                                                                                                                  | `; default: capture 14, 16, 18 (UVC range answers)`          |

For example, `exposure.time` reads `measured 2026-10-31 (#97); bounds, step and default: capture 01 (UVC range answers)` afterwards. The cells of `exposure.compensation` (capture 08b, 12) already back its bounds and stay. Do it with this script (run from the repository root, Python 3, not committed), which appends only where the cell does not end with `(UVC range answers)` yet and re-aligns the Evidence column of the table as the file is aligned: it can be run again after a rebase over W2's ledger edits (w2-7, w2-8) without a conflict to resolve by hand.

```python
import re, sys
path = sys.argv[1]
lines = open(path).read().split('\n')
bounds = {p: '; bounds, step and default: capture 01 (UVC range answers)' for p in
  ['exposure.time','exposure.gain','image.brightness','image.contrast','image.saturation','image.hue','image.sharpness','image.white_balance.temperature']}
bounds.update({p: '; default: capture 14, 16, 18 (UVC range answers)' for p in ['focus.position','image.white_balance.auto']})
def split(line):
    parts = re.split(r'(?<!\\)\|', line)
    return parts[1:-1]
i = 0
while i < len(lines):
    if lines[i].startswith('| Path'):
        j = i
        while j < len(lines) and lines[j].startswith('|'): j += 1
        hdr = [c.strip() for c in split(lines[i])]
        e = hdr.index('Evidence')
        rows = [split(l) for l in lines[i:j]]
        for r in rows[2:]:
            m = re.match(r'\s*`([^`]+)`', r[0])
            if m and m.group(1) in bounds and not r[e].strip().endswith('(UVC range answers)'):
                r[e] = ' ' + r[e].strip() + bounds[m.group(1)] + ' '
        w = max(len(r[e].strip()) for r in [rows[0]] + rows[2:])
        for k, r in enumerate(rows):
            if k == 1: r[e] = ' ' + '-' * w + ' '
            else: r[e] = ' ' + r[e].strip().ljust(w) + ' '
            lines[i + k] = '|' + '|'.join(r) + '|'
        i = j
    else:
        i += 1
open(path, 'w').write('\n'.join(lines))
```

```sh
python3 -I /path/to/ledger.py docs/field-ledger.md
git diff --stat docs/field-ledger.md      # measured on 05e66e9: 89 lines changed (the column is re-aligned), 10 of them gain the text
```

- [ ] **Step 2: The module**

Create `crates/obsbot-device/src/evidence.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Evidence levels of the registered models (design spec 8.5). Each model records, as
//! constants in its module, the level of every field of its description (every setting path
//! and every action), taken from its field ledger, and a row of its own for each value or
//! bound whose evidence differs from its field's; a value or bound without a row takes its
//! field's level, as the ledger's rule says. The level `none` is never inherited: a value,
//! bound or aspect without backing has a row of level `none` of its own and a mark of spec 9.3
//! in the model's `unbacked` list, and [`check`] holds the two lists against each other. Only
//! `captured` and `measured` let a field, value or bound into a released description; `daemon`
//! marks what is not a camera value. [`check`] lists every way a model falls short;
//! `tests/evidence.rs` runs it for every model of the registry.

use crate::contract::ModelEntry;
use crate::models::tiny2lite;
use obsbot_proto::{Bound, Description, Setting, Support};
use std::collections::BTreeSet;

/// The level of one field, value or bound (spec 8.5; the words of the spec's preamble).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// In a capture of the camera's USB traffic, cited by its key ("capture N").
    Captured,
    /// Written from Linux and read back on the camera, or its effect observed.
    Measured,
    /// Not a camera value: a Linux fact the family reads for every model (`on_air.*`).
    Daemon,
    /// No backing today: allowed only for a mark of spec 9.3 in the model's `unbacked` list,
    /// and every such mark that names a value, bound or aspect has a row of this level.
    None,
}

/// One row: a key, its level and its evidence as the ledger gives it. Keys: `<path>`,
/// `<path>=<value>`, `<path>@min`, `<path>@max`, `<path>#step`, `<path>#default`,
/// `<path>#precision`, a component of an `each` bound as `<path>@min.<component>`, a field of
/// a list item as `<path>[].<field>`, and the same under `do:<action>`, `do:<action>.<param>`
/// and `do:<action>#result.<field>` for actions. A mark of spec 9.3 may also name an aspect
/// the description does not enumerate (`<path>#direction`, `#order`, `#values`): such a key is
/// valid for a row when `unbacked` lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub key: &'static str,
    pub level: Level,
    pub evidence: &'static str,
}

/// What a model records about its evidence.
#[derive(Debug, Clone, Copy)]
pub struct Evidence {
    /// One row per field and action, plus the values and bounds with evidence of their own.
    pub levels: &'static [Row],
    /// The marks of spec 9.3: (key, row).
    pub unbacked: &'static [(&'static str, &'static str)],
    /// Bounds, steps and defaults the description holds without backing yet.
    pub unconfirmed: &'static [&'static str],
}

/// The evidence of a registered model, by its registry key. A model added to the registry
/// adds its line here; `tests/evidence.rs` fails for a registered model without one.
pub fn of(key: &str) -> Option<Evidence> {
    match key {
        "tiny2lite" => Some(tiny2lite::levels::EVIDENCE),
        _ => None,
    }
}

/// Every key a row may name: the fields, and the values, bounds, steps and defaults the
/// description holds, down to the fields of list items, action parameters and action results.
pub fn keys(d: &Description) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (path, s) in &d.settings {
        setting(path, s, &mut keys);
    }
    for (name, a) in &d.actions {
        let field = format!("do:{name}");
        for (param, s) in &a.params {
            setting(&format!("{field}.{param}"), s, &mut keys);
        }
        for (name, s) in a.result.iter().flatten() {
            setting(&format!("{field}#result.{name}"), s, &mut keys);
        }
        keys.insert(field);
    }
    keys
}

fn setting(key: &str, s: &Setting, keys: &mut BTreeSet<String>) {
    keys.insert(key.to_string());
    for v in s.values.iter().chain(&s.writable) {
        keys.insert(format!("{key}={v}"));
    }
    for (bound, suffix) in [
        (&s.min, "@min"),
        (&s.max, "@max"),
        (&s.step, "#step"),
        (&s.precision, "#precision"),
    ] {
        if let Some(bound) = bound {
            keys.insert(format!("{key}{suffix}"));
            if let Bound::Each(components) = bound {
                for c in components.keys() {
                    keys.insert(format!("{key}{suffix}.{c}"));
                }
            }
        }
    }
    if s.default.is_some() {
        keys.insert(format!("{key}#default"));
    }
    for (name, item) in s.item.iter().flatten() {
        setting(&format!("{key}[].{name}"), item, keys);
    }
}

/// The field a key belongs to (`exposure.time@min` → `exposure.time`,
/// `do:gimbal.move.speed@max` → `do:gimbal.move`), if the description has it.
pub fn field_of(key: &str, d: &Description) -> Option<String> {
    let base = key.split(['=', '@', '#', '[']).next().unwrap_or(key);
    match base.strip_prefix("do:") {
        Some(rest) => d
            .actions
            .keys()
            .find(|a| rest == a.as_str() || rest.starts_with(&format!("{a}.")))
            .map(|a| format!("do:{a}")),
        None => d.settings.contains_key(base).then(|| base.to_string()),
    }
}

/// Whether `key` names a whole field or action of `d`, not a value, bound or aspect of one.
fn is_field(key: &str, d: &Description) -> bool {
    d.settings.contains_key(key)
        || key
            .strip_prefix("do:")
            .is_some_and(|a| d.actions.contains_key(a))
}

/// Every way `e` falls short for the model `entry` with the description `d`; empty when it
/// passes. Every row names something the description has (or a mark of 9.3), has a level that
/// fits its evidence, and a row of level `none` and a mark of 9.3 go together (a mark on a
/// whole field may keep the field's own level: the ledger's evidence for the field, with the
/// mark as its caveat). `strict` (every release from 1.0 on) or a `tested` model allows no mark
/// of 9.3 and no unconfirmed number; otherwise each must be listed and nothing else may lack
/// backing.
pub fn check(entry: &ModelEntry, d: &Description, e: &Evidence, strict: bool) -> Vec<String> {
    let mut problems = Vec::new();
    let keys = keys(d);
    let mut seen = BTreeSet::new();
    for row in e.levels {
        if !seen.insert(row.key) {
            problems.push(format!("{}: two rows", row.key));
        }
        if !keys.contains(row.key) && !e.unbacked.iter().any(|(k, _)| *k == row.key) {
            problems.push(format!(
                "{}: not a field, value or bound of the description",
                row.key
            ));
        }
        let fits = match row.level {
            Level::Captured => row.evidence.starts_with("capture "),
            Level::Measured => row.evidence.starts_with("measured "),
            Level::Daemon => row.evidence.starts_with("daemon") && row.key.starts_with("on_air."),
            Level::None => row.evidence == "none",
        };
        if !fits {
            problems.push(format!(
                "{}: level {:?} does not fit its evidence \"{}\"",
                row.key, row.level, row.evidence
            ));
        }
        if row.level == Level::None && !e.unbacked.iter().any(|(k, _)| *k == row.key) {
            problems.push(format!("{}: no backing and no mark of 9.3", row.key));
        }
    }
    for path in d.settings.keys() {
        if !seen.contains(path.as_str()) {
            problems.push(format!("{path}: no level"));
        }
    }
    for name in d.actions.keys() {
        if !seen.contains(format!("do:{name}").as_str()) {
            problems.push(format!("do:{name}: no level"));
        }
    }
    for (key, mark) in e.unbacked {
        if field_of(key, d).is_none() {
            problems.push(format!("{key} ({mark}): not in the description"));
        }
        // no inheritance of `none`: a mark names a value, bound or aspect without backing
        // only if that value, bound or aspect has its own row of level none
        match e.levels.iter().find(|r| r.key == *key) {
            Some(r) if r.level == Level::None => {}
            Some(_) if is_field(key, d) => {}
            Some(r) => problems.push(format!(
                "{key} ({mark}): a mark of 9.3, but its row has level {:?}, not none",
                r.level
            )),
            None => problems.push(format!(
                "{key} ({mark}): a mark of 9.3 without a row of level none"
            )),
        }
    }
    for key in e.unconfirmed {
        if !keys.contains(*key) {
            problems.push(format!("{key} (unconfirmed): not in the description"));
        }
    }
    if strict || entry.support == Support::Tested {
        for (key, mark) in e.unbacked {
            problems.push(format!("{key}: {mark} of spec 9.3 has no backing"));
        }
        for key in e.unconfirmed {
            problems.push(format!("{key}: unconfirmed"));
        }
    }
    problems
}
```

In `crates/obsbot-device/src/lib.rs`, after `pub mod error;`:

```diff
diff --git a/crates/obsbot-device/src/lib.rs b/crates/obsbot-device/src/lib.rs
index fc8f471..e878e33 100644
--- a/crates/obsbot-device/src/lib.rs
+++ b/crates/obsbot-device/src/lib.rs
@@ -8,6 +8,7 @@ pub mod contract;
 pub mod descriptions;
 pub mod enumeration;
 pub mod error;
+pub mod evidence;
 pub mod family;
 pub mod models;
 pub mod registry;
```

- [ ] **Step 3: The Tiny 2 Lite's table**

`levels.rs` is generated from the ledger and from `UNBACKED` (the script below), then formatted with `cargo fmt`; afterwards it is edited by hand like any constant, and `the_field_rows_follow_the_ledger`, `the_marks_of_9_3_follow_the_ledger` and `check` keep it equal to the ledger and to `UNBACKED`. **The file below is a snapshot of 2026-11-24 (ledger of Step 1 applied, `UNBACKED` of `05e66e9`): if the ledger or `UNBACKED` differs when this task is built, and again after CS-1's contract-change PRs, the builder regenerates the file with this script instead of pasting it, and lists the difference in the pull request.** Generator (run from the repository root, Python 3, not committed); the field rows are the ledger's evidence columns, then one row of level `none` for every mark of `UNBACKED` that names a value, bound or aspect, then the recorded UVC bounds:

```python
import re, sys
ledger, description = sys.argv[1], sys.argv[2]
def split_row(line):
    return [c.replace('\\|', '|').strip() for c in re.split(r'(?<!\\)\|', line.strip())[1:-1]]
rows = []
hdr = None
for line in open(ledger):
    if line.startswith('| Path') or line.startswith('| Action'):
        hdr = split_row(line); kind = hdr[0]; continue
    if line.startswith('## ') : hdr = hdr if hdr else None
    if hdr and line.startswith('| `'):
        c = dict(zip(hdr, split_row(line)))
        name = re.match(r'`([^`]+)`', c[kind]).group(1)
        key = ('do:' if kind == 'Action' else '') + name
        if key in ('available', 'pending', 'unread'):
            continue  # daemon state, not in a description
        ev = c['Evidence']
        if ev.startswith('daemon'): lv = 'Daemon'
        elif ev.startswith('capture '): lv = 'Captured'
        elif ev.startswith('measured '): lv = 'Measured'
        elif ev == 'none': lv = 'None'
        else: raise SystemExit(f'{key}: unknown evidence {ev!r}')
        rows.append((key, lv, ev))
bounds = [
 ('exposure.time@min','capture 01'),('exposure.time@max','capture 01'),('exposure.time#default','capture 01'),
 ('exposure.gain#default','capture 01'),
 ('focus.position#default','capture 14, 16, 18'),
]
for p in ['image.brightness','image.contrast','image.saturation','image.hue','image.sharpness']:
    bounds += [(p+'@min','capture 01'),(p+'@max','capture 01'),(p+'#default','capture 01')]
bounds += [('image.white_balance.auto#default','capture 14, 16, 18')]
bounds += [('image.white_balance.temperature'+s,'capture 01') for s in ('@min','@max','#step','#default')]
src = open(description).read()
block = src[src.index('pub const UNBACKED'):]
block = block[:block.index('];')]
marks = re.findall(r'\("([^"]+)", "(U-\d+)"\)', block)
# a mark on a whole field keeps the field's row (its level is the ledger's); a mark on a value,
# bound or aspect gets a row of level none of its own
none_rows = [(k, 'None', 'none') for k, _ in marks if re.search(r'[=@#\[]', k)]
def lit(s): return '"' + s.replace('\\','\\\\').replace('"','\\"') + '"'
out = []
out.append('''// SPDX-License-Identifier: GPL-3.0-or-later

//! The evidence levels of the Tiny 2 Lite (design spec 8.5), one row per field and action of
//! its description, with the evidence column of `docs/field-ledger.md` as it stands, and one
//! row per value, bound, step or default whose evidence differs from its field's. A value or
//! bound without a row of its own takes its field's level, except `none`: that is never
//! inherited, every value, bound or aspect without backing has a row of its own and a mark of
//! spec 9.3 in [`UNBACKED`], and the test checks the two lists against each other and the marks
//! against the ledger. `tests/evidence.rs` checks the rows against the description and the
//! ledger.

use super::description::{UNBACKED, UNCONFIRMED};
use crate::evidence::{Evidence, Level, Row};

const fn row(key: &'static str, level: Level, evidence: &'static str) -> Row {
    Row {
        key,
        level,
        evidence,
    }
}

/// The evidence of the Tiny 2 Lite for [`crate::evidence::of`].
pub const EVIDENCE: Evidence = Evidence {
    levels: LEVELS,
    unbacked: UNBACKED,
    unconfirmed: UNCONFIRMED,
};

/// Every field and action, in ledger order; then one row of level `none` per mark of spec 9.3
/// that names a value, bound or aspect (the order of `UNBACKED`); then the bounds, steps and
/// defaults of the UVC controls: the camera's answers to the UVC range requests (GET_MIN,
/// GET_MAX, GET_RES, GET_DEF) recorded in captures 01, 14, 16 and 18.
pub const LEVELS: &[Row] = &[''')
def emit(rs):
    for key, lv, ev in rs:
        out.append(f'    row({lit(key)}, Level::{lv}, {lit(ev)}),')
emit(rows)
out.append('    // marks of spec 9.3 on a value, a bound or an aspect: no backing')
emit(none_rows)
out.append('    // the camera\'s UVC range answers')
emit([(k, 'Captured', e) for k, e in bounds])
out.append('];')
print('\n'.join(out))
```

```sh
python3 -I /path/to/levels.py docs/field-ledger.md crates/obsbot-device/src/models/tiny2lite/description.rs \
  > crates/obsbot-device/src/models/tiny2lite/levels.rs
cargo fmt --all
```

The result, as committed (`crates/obsbot-device/src/models/tiny2lite/levels.rs`):

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The evidence levels of the Tiny 2 Lite (design spec 8.5), one row per field and action of
//! its description, with the evidence column of `docs/field-ledger.md` as it stands, and one
//! row per value, bound, step or default whose evidence differs from its field's. A value or
//! bound without a row of its own takes its field's level, except `none`: that is never
//! inherited, every value, bound or aspect without backing has a row of its own and a mark of
//! spec 9.3 in [`UNBACKED`], and the test checks the two lists against each other and the marks
//! against the ledger. `tests/evidence.rs` checks the rows against the description and the
//! ledger.

use super::description::{UNBACKED, UNCONFIRMED};
use crate::evidence::{Evidence, Level, Row};

const fn row(key: &'static str, level: Level, evidence: &'static str) -> Row {
    Row {
        key,
        level,
        evidence,
    }
}

/// The evidence of the Tiny 2 Lite for [`crate::evidence::of`].
pub const EVIDENCE: Evidence = Evidence {
    levels: LEVELS,
    unbacked: UNBACKED,
    unconfirmed: UNCONFIRMED,
};

/// Every field and action, in ledger order; then one row of level `none` per mark of spec 9.3
/// that names a value, bound or aspect (the order of `UNBACKED`); then the bounds, steps and
/// defaults of the UVC controls: the camera's answers to the UVC range requests (GET_MIN,
/// GET_MAX, GET_RES, GET_DEF) recorded in captures 01, 14, 16 and 18.
pub const LEVELS: &[Row] = &[
    row("asleep", Level::Captured, "capture 13; measured 2026-10-24"),
    row(
        "on_air.active",
        Level::Daemon,
        "daemon (the probe measured on this model 2026-10-25)",
    ),
    row(
        "on_air.apps",
        Level::Daemon,
        "daemon (the probe measured on this model 2026-10-25)",
    ),
    row("gimbal.pan", Level::Measured, "measured 2026-10-31 (#67)"),
    row("gimbal.tilt", Level::Measured, "measured 2026-10-31 (#67)"),
    row("gimbal.invert", Level::Captured, "capture 05, 17"),
    row(
        "zoom.factor",
        Level::Captured,
        "capture 04, 11; measured 2026-10-24",
    ),
    row(
        "zoom.field_of_view",
        Level::Captured,
        "capture 09; measured 2026-10-24",
    ),
    row("tracking.mode", Level::Captured, "capture 11"),
    row(
        "tracking.profile",
        Level::Captured,
        "capture 11 (0, 2); measured 2026-10-24 (0, 2), 2026-10-31 (#98: 1)",
    ),
    row(
        "tracking.speed",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "tracking.target.detected",
        Level::Measured,
        "measured 2026-10-25",
    ),
    row(
        "tracking.target.box",
        Level::Measured,
        "measured 2026-10-25; layout #71",
    ),
    row(
        "framing.auto_zoom",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "framing.size",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "framing.zoom_speed",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "framing.offset.x",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; bounds and direction not checked)",
    ),
    row(
        "framing.offset.y",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; bounds and direction not checked)",
    ),
    row(
        "framing.part",
        Level::Measured,
        "measured 2026-10-31 (#98: read back; effect not observed)",
    ),
    row(
        "gestures.target_lock",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "gestures.zoom",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "gestures.zoom_ratio",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "gestures.dynamic_zoom",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "gestures.direction_change",
        Level::Captured,
        "capture 04; measured 2026-10-24",
    ),
    row(
        "zone.enabled",
        Level::Captured,
        "capture 11; measured 2026-10-31 (#81)",
    ),
    row(
        "zone.follow",
        Level::Captured,
        "capture 11, 18; measured 2026-10-25",
    ),
    row(
        "zone.area.pan",
        Level::Captured,
        "capture 11; measured 2026-10-25, 2026-10-27 (#58)",
    ),
    row(
        "zone.area.tilt",
        Level::Captured,
        "capture 11; measured 2026-10-25, 2026-10-27 (#58)",
    ),
    row(
        "zone.return",
        Level::Captured,
        "capture 11, 18; measured 2026-10-25",
    ),
    row("hand.side", Level::Captured, "capture 11, 17"),
    row("hand.area.pan", Level::Captured, "capture 11"),
    row("hand.area.tilt", Level::Captured, "capture 11"),
    row("hand.axes.pan", Level::None, "none"),
    row("hand.axes.tilt", Level::None, "none"),
    row("hand.return_after", Level::None, "none"),
    row("hand.move_gimbal", Level::Captured, "capture 11"),
    row(
        "hand.return",
        Level::Captured,
        "capture 11; measured 2026-10-25",
    ),
    row("presets", Level::Measured, "measured 2026-10-31 (#104)"),
    row(
        "views",
        Level::Captured,
        "capture 11, 18; measured 2026-10-31 (#100: empty list)",
    ),
    row(
        "exposure.mode",
        Level::Measured,
        "measured 2026-10-31 (#97)",
    ),
    row(
        "exposure.region",
        Level::Captured,
        "capture 12; measured 2026-10-24",
    ),
    row(
        "exposure.time",
        Level::Measured,
        "measured 2026-10-31 (#97); bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "exposure.gain",
        Level::Captured,
        "capture 12; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row("exposure.compensation", Level::Captured, "capture 08b, 12"),
    row(
        "exposure.hdr",
        Level::Captured,
        "capture 12; measured 2026-10-24",
    ),
    row(
        "exposure.anti_flicker",
        Level::Captured,
        "capture 12; measured 2026-10-24 (0–2); value `auto`: none",
    ),
    row(
        "image.brightness",
        Level::Measured,
        "measured 2026-10-24; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.contrast",
        Level::Captured,
        "capture 08b, 12; measured 2026-10-24; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.saturation",
        Level::Captured,
        "capture 08b, 12; measured 2026-10-24; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.hue",
        Level::Captured,
        "capture 08b, 12; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.sharpness",
        Level::Captured,
        "capture 08b, 12; measured 2026-10-24; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.white_balance.auto",
        Level::Captured,
        "capture 08b, 12; default: capture 14, 16, 18 (UVC range answers)",
    ),
    row(
        "image.white_balance.temperature",
        Level::Captured,
        "capture 08b, 12; bounds, step and default: capture 01 (UVC range answers)",
    ),
    row(
        "image.mirror",
        Level::Captured,
        "capture 05; measured 2026-10-24",
    ),
    row("image.frame_rate", Level::Measured, "measured 2026-10-24"),
    row(
        "focus.mode",
        Level::Captured,
        "capture 12 (focus auto); measured 2026-10-31 (#97)",
    ),
    row(
        "focus.region",
        Level::Captured,
        "capture 12; measured 2026-10-24",
    ),
    row(
        "focus.position",
        Level::Captured,
        "capture 12; measured 2026-10-31 (#97: read back; #99: no visible effect); default: capture 14, 16, 18 (UVC range answers)",
    ),
    row(
        "audio.enabled",
        Level::Captured,
        "capture 14; measured 2026-10-24 (restart; the interface goes and returns)",
    ),
    row(
        "audio.volume",
        Level::Measured,
        "measured 2026-10-31 (#97: 30, 55, 85 read back)",
    ),
    row(
        "audio.noise_reduction",
        Level::Captured,
        "capture 06; measured 2026-10-24",
    ),
    row(
        "audio.auto_gain",
        Level::Captured,
        "capture 06; measured 2026-10-24",
    ),
    row(
        "audio.in_sleep",
        Level::Captured,
        "capture 06; measured 2026-10-24",
    ),
    row(
        "led.brightness",
        Level::Captured,
        "capture 07; measured 2026-10-24",
    ),
    row(
        "led.blink",
        Level::Captured,
        "capture 11, 18; measured 2026-10-25 (the LED blinks)",
    ),
    row(
        "sleep.auto.enabled",
        Level::Captured,
        "capture 02; measured 2026-10-24",
    ),
    row(
        "sleep.auto.seconds",
        Level::Captured,
        "capture 02; measured 2026-10-24",
    ),
    row(
        "sleep.background",
        Level::Captured,
        "capture 03, 18 (`own_image`); measured 2026-10-24",
    ),
    row(
        "sleep.background_mirror",
        Level::Captured,
        "capture 03; measured 2026-10-24",
    ),
    row(
        "sleep.wake_zoom",
        Level::Measured,
        "measured 2026-10-31 (#97: read back; #102: no effect)",
    ),
    row(
        "power_on.mode",
        Level::Measured,
        "measured 2026-10-31 (#97: read back; #102: applied at power-on)",
    ),
    row(
        "power_on.start.source",
        Level::Captured,
        "capture 08; measured 2026-10-25 (`own`); `preset_<slot>`: none",
    ),
    row(
        "power_on.start.pose",
        Level::Captured,
        "capture 08; measured 2026-10-25",
    ),
    row(
        "power_on.start.mode",
        Level::Captured,
        "capture 08 (normal, unset)",
    ),
    row(
        "power_on.settings.tracking_speed",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.autofocus",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.auto_exposure",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.exposure_compensation",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.white_balance_mode",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.white_balance_temperature",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.contrast",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.saturation",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.sharpness",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "power_on.settings.hue",
        Level::Captured,
        "capture 08b (one value)",
    ),
    row(
        "do:gimbal.move",
        Level::Captured,
        "capture 18 (`0x6444`); measured 2026-10-25; measured 2026-10-31 (#98: speed 1, 5, 100 °/s; tilt 80, pan 145)",
    ),
    row(
        "do:gimbal.stop",
        Level::Measured,
        "measured 2026-10-31 (#98)",
    ),
    row(
        "do:gimbal.turn",
        Level::Captured,
        "capture 10; measured 2026-10-31 (#98, the sign flip included)",
    ),
    row(
        "do:zoom.move",
        Level::Captured,
        "capture 11 (30); measured 2026-10-31 (#98)",
    ),
    row("do:zoom.stop", Level::Measured, "measured 2026-10-31 (#98)"),
    row("do:preset.save", Level::Captured, "capture 10"),
    row("do:preset.recall", Level::Captured, "capture 10"),
    row("do:preset.rename", Level::Captured, "capture 10"),
    row("do:preset.delete", Level::Captured, "capture 10"),
    row("do:view.save", Level::Captured, "capture 11"),
    row("do:view.go", Level::Measured, "measured 2026-10-31 (#100)"),
    row("do:view.rename", Level::Captured, "capture 11"),
    row("do:view.delete", Level::Captured, "capture 11"),
    row(
        "do:zone.return_here",
        Level::Captured,
        "capture 11; measured 2026-10-25",
    ),
    row("do:zone.go_return", Level::Measured, "measured 2026-10-25"),
    row(
        "do:hand.return_here",
        Level::Captured,
        "capture 11; measured 2026-10-25",
    ),
    row("do:hand.go_return", Level::Measured, "measured 2026-10-25"),
    row(
        "do:hand.return_clear",
        Level::Measured,
        "measured 2026-10-25",
    ),
    row("do:hand.area_reset", Level::None, "none"),
    row(
        "do:focus.point",
        Level::Measured,
        "measured 2026-10-31 (#97: accepted, point read back; #99: the picture blurs instead of focusing)",
    ),
    row(
        "do:power_on.set_pose",
        Level::Captured,
        "capture 08; measured 2026-10-25 (readback)",
    ),
    row("do:power_on.use_current", Level::Captured, "capture 08"),
    row("do:power_on.use_preset", Level::Captured, "capture 10"),
    row(
        "do:power_on.go",
        Level::Measured,
        "measured 2026-10-31 (#79)",
    ),
    row(
        "do:sleep",
        Level::Captured,
        "capture 13; measured 2026-10-24",
    ),
    row(
        "do:wake",
        Level::Captured,
        "capture 13; measured 2026-10-24",
    ),
    row(
        "do:factory_reset",
        Level::Captured,
        "capture 16; measured 2026-10-24",
    ),
    // marks of spec 9.3 on a value, a bound or an aspect: no backing
    row("framing.offset.x@min", Level::None, "none"),
    row("framing.offset.x@max", Level::None, "none"),
    row("framing.offset.y@min", Level::None, "none"),
    row("framing.offset.y@max", Level::None, "none"),
    row("framing.offset.x#direction", Level::None, "none"),
    row("framing.offset.y#direction", Level::None, "none"),
    row("hand.area.pan#order", Level::None, "none"),
    row("hand.area.tilt#order", Level::None, "none"),
    row("zone.area.pan#order", Level::None, "none"),
    row("zone.area.tilt#order", Level::None, "none"),
    row("do:gimbal.move.speed@max", Level::None, "none"),
    row("do:gimbal.move#speed_near_tilt_limit", Level::None, "none"),
    row("do:gimbal.move#speed_one_axis", Level::None, "none"),
    row("do:zoom.move.speed=slow", Level::None, "none"),
    row("do:zoom.move.speed=fast", Level::None, "none"),
    row("power_on.start.mode=static", Level::None, "none"),
    row("power_on.start.mode=upper_body", Level::None, "none"),
    row("power_on.start.mode=close_up", Level::None, "none"),
    row("power_on.start.mode=headless", Level::None, "none"),
    row("power_on.start.mode=lower_body", Level::None, "none"),
    row("power_on.start.mode=hand", Level::None, "none"),
    row("power_on.start.mode=group", Level::None, "none"),
    row(
        "power_on.settings.tracking_speed#values",
        Level::None,
        "none",
    ),
    row("power_on.settings.autofocus#values", Level::None, "none"),
    row(
        "power_on.settings.auto_exposure#values",
        Level::None,
        "none",
    ),
    row(
        "power_on.settings.exposure_compensation#values",
        Level::None,
        "none",
    ),
    row(
        "power_on.settings.white_balance_mode#values",
        Level::None,
        "none",
    ),
    row(
        "power_on.settings.white_balance_temperature#values",
        Level::None,
        "none",
    ),
    row("power_on.settings.contrast#values", Level::None, "none"),
    row("power_on.settings.saturation#values", Level::None, "none"),
    row("power_on.settings.sharpness#values", Level::None, "none"),
    row("power_on.settings.hue#values", Level::None, "none"),
    row("exposure.compensation#ev_scale", Level::None, "none"),
    row("audio.enabled#readback", Level::None, "none"),
    row("exposure.anti_flicker=auto", Level::None, "none"),
    row("power_on.start.source=preset_0", Level::None, "none"),
    row("power_on.start.source=preset_1", Level::None, "none"),
    row("power_on.start.source=preset_2", Level::None, "none"),
    // the camera's UVC range answers
    row("exposure.time@min", Level::Captured, "capture 01"),
    row("exposure.time@max", Level::Captured, "capture 01"),
    row("exposure.time#default", Level::Captured, "capture 01"),
    row("exposure.gain#default", Level::Captured, "capture 01"),
    row(
        "focus.position#default",
        Level::Captured,
        "capture 14, 16, 18",
    ),
    row("image.brightness@min", Level::Captured, "capture 01"),
    row("image.brightness@max", Level::Captured, "capture 01"),
    row("image.brightness#default", Level::Captured, "capture 01"),
    row("image.contrast@min", Level::Captured, "capture 01"),
    row("image.contrast@max", Level::Captured, "capture 01"),
    row("image.contrast#default", Level::Captured, "capture 01"),
    row("image.saturation@min", Level::Captured, "capture 01"),
    row("image.saturation@max", Level::Captured, "capture 01"),
    row("image.saturation#default", Level::Captured, "capture 01"),
    row("image.hue@min", Level::Captured, "capture 01"),
    row("image.hue@max", Level::Captured, "capture 01"),
    row("image.hue#default", Level::Captured, "capture 01"),
    row("image.sharpness@min", Level::Captured, "capture 01"),
    row("image.sharpness@max", Level::Captured, "capture 01"),
    row("image.sharpness#default", Level::Captured, "capture 01"),
    row(
        "image.white_balance.auto#default",
        Level::Captured,
        "capture 14, 16, 18",
    ),
    row(
        "image.white_balance.temperature@min",
        Level::Captured,
        "capture 01",
    ),
    row(
        "image.white_balance.temperature@max",
        Level::Captured,
        "capture 01",
    ),
    row(
        "image.white_balance.temperature#step",
        Level::Captured,
        "capture 01",
    ),
    row(
        "image.white_balance.temperature#default",
        Level::Captured,
        "capture 01",
    ),
];
```

In `crates/obsbot-device/src/models/tiny2lite/mod.rs`, and in `description.rs`:

```diff
diff --git a/crates/obsbot-device/src/models/tiny2lite/mod.rs b/crates/obsbot-device/src/models/tiny2lite/mod.rs
index d6ccdec..c5694ec 100644
--- a/crates/obsbot-device/src/models/tiny2lite/mod.rs
+++ b/crates/obsbot-device/src/models/tiny2lite/mod.rs
@@ -6,6 +6,7 @@ pub mod act;
 pub mod commands;
 pub mod decode;
 pub mod description;
+pub mod levels;
 pub mod model;
 pub mod plan;
 pub mod presets;
```

```diff
diff --git a/crates/obsbot-device/src/models/tiny2lite/description.rs b/crates/obsbot-device/src/models/tiny2lite/description.rs
index 2f2b13b..629cdb0 100644
--- a/crates/obsbot-device/src/models/tiny2lite/description.rs
+++ b/crates/obsbot-device/src/models/tiny2lite/description.rs
@@ -75,37 +75,11 @@ pub const UNBACKED: &[(&str, &str)] = &[
     ("power_on.start.source=preset_2", "U-26"),
 ];

-/// Bounds, steps and defaults that this description holds but neither the spec nor the field
-/// ledger backs yet (the W0 plan's decision 12): `<path>@min`, `<path>@max`, `<path>#step`,
-/// `<path>#default`. They are not evidence. Track A confirms each in the ledger, from a capture
-/// or a measurement on the camera, or changes it, before the evidence-level test of stage W3.
-pub const UNCONFIRMED: &[&str] = &[
-    "exposure.time@min",
-    "exposure.time@max",
-    "exposure.time#default",
-    "exposure.gain#default",
-    "focus.position#default",
-    "image.brightness@min",
-    "image.brightness@max",
-    "image.brightness#default",
-    "image.contrast@min",
-    "image.contrast@max",
-    "image.contrast#default",
-    "image.saturation@min",
-    "image.saturation@max",
-    "image.saturation#default",
-    "image.hue@min",
-    "image.hue@max",
-    "image.hue#default",
-    "image.sharpness@min",
-    "image.sharpness@max",
-    "image.sharpness#default",
-    "image.white_balance.auto#default",
-    "image.white_balance.temperature@min",
-    "image.white_balance.temperature@max",
-    "image.white_balance.temperature#step",
-    "image.white_balance.temperature#default",
-];
+/// Bounds, steps and defaults that this description holds without backing (the W0 plan's
+/// decision 12): `<path>@min`, `<path>@max`, `<path>#step`, `<path>#default`. None today: the
+/// camera's UVC range answers recorded in captures 01, 14, 16 and 18 back every one of them
+/// (`levels.rs`, `tests/evidence.rs`).
+pub const UNCONFIRMED: &[&str] = &[];

 /// A writable leaf with a readback: asleep `unknown` until CAM-2, the model-wide settle bound.
 fn w(s: Setting) -> Setting {
```

In `crates/obsbot-device/tests/live.rs` (the comment that called the bounds not backed):

```diff
diff --git a/crates/obsbot-device/tests/live.rs b/crates/obsbot-device/tests/live.rs
index f96ad15..040a0ba 100644
--- a/crates/obsbot-device/tests/live.rs
+++ b/crates/obsbot-device/tests/live.rs
@@ -303,8 +303,9 @@ fn live_cam_10_cam_20_a_cold_read_of_every_group() {
 #[test]
 #[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
 fn live_the_uvc_ranges_the_description_holds() {
-    // the W0 plan's decision 12: these bounds, steps and defaults are not backed yet; the camera
-    // reports its own (a read). A mismatch is fixed by a contract-change PR of the description.
+    // the bounds, steps and defaults of the description are backed by the recorded range answers
+    // (captures 01, 14, 16, 18; tests/evidence.rs); here the unit reports its own (a read). A
+    // mismatch is fixed by a contract-change PR of the description.
     let Some((found, cam)) = camera(false) else {
         return;
     };
```

The test `every_unconfirmed_number_is_in_the_description` in `description.rs` stays; it has nothing to check now. `descriptions/tiny2lite.json` does not change (`cargo test -p obsbot-device --test generated` stays green).

- [ ] **Step 4: CI must run the test when only the ledger changes**

`ci.yml`'s `changes` job treats `docs/*` and `*.md` as docs-only and skips fmt, clippy and test. A pull request that edited only `docs/field-ledger.md` (a CS-1 ledger pull request, a W2 docs change) would then never run `the_field_rows_follow_the_ledger` and could land green while breaking it. The ledger, and `docs/protocol-2.md`, which W2's `doc_examples` test reads, count as code:

```diff
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -17,7 +17,8 @@
   RUSTUP_TOOLCHAIN: "1.99.1"

 jobs:
-  # Docs-only changes (Markdown, LICENSE, docs/) skip fmt, clippy and test.
+  # Docs-only changes (Markdown, LICENSE, docs/) skip fmt, clippy and test, except the docs a
+  # test reads: docs/field-ledger.md (the evidence test) and docs/protocol-2.md (its transcripts).
   changes:
     runs-on: ubuntu-22.04
     outputs:
@@ -40,6 +41,7 @@
             code=false
             for f in $files; do
               case $f in
+                docs/field-ledger*.md | docs/protocol-2*.md) code=true ;;
                 *.md | LICENSE | docs/*) ;;
                 *) code=true ;;
               esac
```

Measured on the `case` lines with sample file lists: `docs/field-ledger.md` → code; `docs/protocol-2.md CHANGELOG.md` → code; `README.md docs/other.md` → docs-only; `docs/x.md LICENSE` → docs-only. `actionlint` is clean.

- [ ] **Step 5: The test**

Create `crates/obsbot-device/tests/evidence.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The evidence-level test of design spec 8.5: every field, value and bound of every
//! registered model has the level `captured` or `measured` (`daemon` for what is not a camera
//! value). Before 1.0 a mark of spec 9.3 may stand, listed in the model's `unbacked`; from the
//! 1.0 release candidate on (workspace version 1 or later), and for a `tested` model at any
//! version, none may. The levels follow the field ledger, every mark of 9.3 has a row of level
//! `none` (and the other way round) and agrees with the ledger's column §9.3, and the UVC
//! bounds and defaults of the Tiny 2 Lite equal the camera's range answers recorded in
//! captures 01, 14, 16 and 18 (the fixtures below cite 01 and 16).

use obsbot_device::contract::{ControlRange, ModelEntry, Transport};
use obsbot_device::evidence::{self, Evidence, Level, Row};
use obsbot_device::models::tiny2lite::{decode, levels, plan};
use obsbot_device::registry::REGISTRY;
use obsbot_device::transport::replay::Replay;
use obsbot_proto::{Description, Support};
use std::collections::{BTreeMap, BTreeSet};

/// The rule of spec 8.5 holds without exception from the 1.0 release candidate on.
fn strict() -> bool {
    !env!("CARGO_PKG_VERSION").starts_with("0.")
}

fn description_of(key: &str) -> Description {
    let path = format!("descriptions/{key}.json");
    obsbot_device::descriptions::all()
        .into_iter()
        .find(|(p, _)| *p == path)
        .map(|(_, d)| d)
        .unwrap_or_else(|| panic!("{key}: no {path} in descriptions::all"))
}

#[test]
fn every_registered_model_meets_the_evidence_rule() {
    assert!(!REGISTRY.models.is_empty());
    for entry in REGISTRY.models {
        let e = evidence::of(entry.key)
            .unwrap_or_else(|| panic!("{}: no evidence in evidence::of", entry.key));
        let problems = evidence::check(entry, &description_of(entry.key), &e, strict());
        assert!(
            problems.is_empty(),
            "{} (support {:?}, strict {}):\n{}",
            entry.key,
            entry.support,
            strict(),
            problems.join("\n")
        );
    }
}

/// From 1.0 on, every mark of 9.3 still in the description fails, and nothing else does.
#[test]
fn strict_fails_exactly_the_marks_of_9_3() {
    let entry = &plan::ENTRY;
    let e = levels::EVIDENCE;
    let problems = evidence::check(entry, &description_of("tiny2lite"), &e, true);
    let want: Vec<String> = e
        .unbacked
        .iter()
        .map(|(k, m)| format!("{k}: {m} of spec 9.3 has no backing"))
        .collect();
    assert_eq!(problems, want);
}

/// Setting the support level to `tested` (stage W3) fails while a mark of 9.3 stands.
#[test]
fn a_tested_model_may_keep_no_mark_of_9_3() {
    let tested = ModelEntry {
        support: Support::Tested,
        ..plan::ENTRY
    };
    let e = levels::EVIDENCE;
    let problems = evidence::check(&tested, &description_of("tiny2lite"), &e, false);
    assert_eq!(problems.len(), e.unbacked.len(), "{problems:#?}");
    assert!(
        problems
            .iter()
            .all(|p| p.contains("of spec 9.3 has no backing"))
    );
}

#[test]
fn a_missing_wrong_or_stray_row_fails() {
    let d = description_of("tiny2lite");
    // untested and before 1.0, so that only the broken row shows
    let entry = &ModelEntry {
        support: Support::Untested,
        ..plan::ENTRY
    };
    let all = levels::LEVELS;
    let with = |rows: Vec<Row>| Evidence {
        levels: Box::leak(rows.into_boxed_slice()),
        ..levels::EVIDENCE
    };

    let without_led: Vec<Row> = all
        .iter()
        .copied()
        .filter(|r| r.key != "led.brightness")
        .collect();
    let p = evidence::check(entry, &d, &with(without_led), false);
    assert_eq!(p, ["led.brightness: no level"]);

    let mut wrong: Vec<Row> = all.to_vec();
    let i = wrong
        .iter()
        .position(|r| r.key == "led.brightness")
        .unwrap();
    wrong[i].level = Level::Measured; // its evidence starts with "capture 07"
    let p = evidence::check(entry, &d, &with(wrong), false);
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(p[0].starts_with("led.brightness: level Measured does not fit"));

    let mut unbacked: Vec<Row> = all.to_vec();
    unbacked[i] = Row {
        key: "led.brightness",
        level: Level::None,
        evidence: "none",
    };
    let p = evidence::check(entry, &d, &with(unbacked), false);
    assert_eq!(p, ["led.brightness: no backing and no mark of 9.3"]);

    let mut stray: Vec<Row> = all.to_vec();
    stray.push(Row {
        key: "led.colour",
        level: Level::Captured,
        evidence: "capture 07",
    });
    let p = evidence::check(entry, &d, &with(stray), false);
    assert_eq!(
        p,
        ["led.colour: not a field, value or bound of the description"]
    );

    // a mark of 9.3 on a value needs its row of level none, and a row of level none a mark
    let anti = all
        .iter()
        .position(|r| r.key == "exposure.anti_flicker=auto")
        .expect("the value `auto` (U-25) has its row of level none");
    assert_eq!(all[anti].level, Level::None);
    let mut no_row: Vec<Row> = all.to_vec();
    no_row.remove(anti);
    let p = evidence::check(entry, &d, &with(no_row), false);
    assert_eq!(
        p,
        ["exposure.anti_flicker=auto (U-25): a mark of 9.3 without a row of level none"]
    );
    let mut inherited: Vec<Row> = all.to_vec();
    inherited[anti] = Row {
        key: "exposure.anti_flicker=auto",
        level: Level::Captured,
        evidence: "capture 12",
    };
    let p = evidence::check(entry, &d, &with(inherited), false);
    assert_eq!(
        p,
        [
            "exposure.anti_flicker=auto (U-25): a mark of 9.3, but its row has level Captured, not none"
        ]
    );
    let mut no_mark: Vec<Row> = all.to_vec();
    no_mark.push(Row {
        key: "exposure.anti_flicker=50hz",
        level: Level::None,
        evidence: "none",
    });
    let p = evidence::check(entry, &d, &with(no_mark), false);
    assert_eq!(
        p,
        ["exposure.anti_flicker=50hz: no backing and no mark of 9.3"]
    );

    let mut daemon: Vec<Row> = all.to_vec();
    daemon[i] = Row {
        key: "led.brightness",
        level: Level::Daemon,
        evidence: "daemon",
    };
    let p = evidence::check(entry, &d, &with(daemon), false);
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(p[0].starts_with("led.brightness: level Daemon does not fit"));
}

/// Splits one Markdown table row at its unescaped pipes.
fn cells(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cell.push('|');
                chars.next();
            }
            '|' => out.push(std::mem::take(&mut cell).trim().to_string()),
            _ => cell.push(c),
        }
    }
    out.remove(0);
    out
}

/// The row of every field carries the evidence column of `docs/field-ledger.md` word for
/// word, so that a change of the ledger shows here and the other way round.
#[test]
fn the_field_rows_follow_the_ledger() {
    let ledger = include_str!("../../../docs/field-ledger.md");
    let mut header: Option<Vec<String>> = None;
    let mut seen = 0;
    for line in ledger.lines() {
        if line.starts_with("| Path") || line.starts_with("| Action") {
            header = Some(cells(line));
            continue;
        }
        let Some(h) = &header else { continue };
        if !line.starts_with("| `") {
            continue;
        }
        let c = cells(line);
        let name = c[0].split('`').nth(1).unwrap();
        let key = if h[0] == "Action" {
            format!("do:{name}")
        } else {
            name.to_string()
        };
        if ["available", "pending", "unread"].contains(&key.as_str()) {
            continue;
        }
        let evidence = &c[h.iter().position(|x| x == "Evidence").unwrap()];
        let row = levels::LEVELS
            .iter()
            .find(|r| r.key == key)
            .unwrap_or_else(|| panic!("{key}: in the ledger, not in LEVELS"));
        assert_eq!(row.evidence, evidence, "{key}");
        seen += 1;
    }
    let d = description_of("tiny2lite");
    assert_eq!(seen, d.settings.len() + d.actions.len());
}

/// The marks of spec 9.3 are the ledger's: per field, the `U-n` of its §9.3 column equal the
/// marks of `UNBACKED` that belong to the field, and a ledger evidence cell that says `none`
/// for a value or bound comes with such a mark. A mark that the ledger does not carry, or a
/// ledger mark that `UNBACKED` lacks, fails here.
#[test]
fn the_marks_of_9_3_follow_the_ledger() {
    let ledger = include_str!("../../../docs/field-ledger.md");
    let d = description_of("tiny2lite");
    let mut from_ledger: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut says_none: BTreeSet<String> = BTreeSet::new();
    let mut header: Option<Vec<String>> = None;
    for line in ledger.lines() {
        if line.starts_with("| Path") || line.starts_with("| Action") {
            header = Some(cells(line));
            continue;
        }
        let Some(h) = &header else { continue };
        if !line.starts_with("| `") {
            continue;
        }
        let c = cells(line);
        let name = c[0].split('`').nth(1).unwrap();
        let key = if h[0] == "Action" {
            format!("do:{name}")
        } else {
            name.to_string()
        };
        let column = |title: &str| &c[h.iter().position(|x| x == title).unwrap()];
        let marks: BTreeSet<String> = column("§9.3")
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
            .filter(|w| {
                w.strip_prefix("U-")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
            .map(str::to_string)
            .collect();
        if !marks.is_empty() {
            from_ledger.insert(key.clone(), marks);
        }
        if column("Evidence")
            .split(|ch: char| !ch.is_ascii_alphabetic())
            .any(|w| w == "none")
        {
            says_none.insert(key);
        }
    }
    let mut from_model: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (key, mark) in levels::EVIDENCE.unbacked {
        let field = evidence::field_of(key, &d).unwrap_or_else(|| panic!("{key}: no field"));
        from_model
            .entry(field)
            .or_default()
            .insert((*mark).to_string());
    }
    assert_eq!(
        from_model, from_ledger,
        "UNBACKED (left) against the ledger's column §9.3 (right)"
    );
    for field in &says_none {
        assert!(
            from_model.contains_key(field),
            "{field}: the ledger's evidence says none, but UNBACKED has no mark for it"
        );
    }
}

/// One UVC control's range answers as recorded: GET_MIN, GET_MAX, GET_RES, GET_DEF, each the
/// little-endian bytes the camera answered (spec 8.5 fixtures: single exchanges).
struct Recorded {
    leaf: &'static str,
    capture: &'static str,
    answers: [&'static str; 4],
}

const RECORDED: [Recorded; 11] = [
    Recorded {
        leaf: "exposure.time",
        capture: "capture 01",
        answers: ["01000000", "c4090000", "01000000", "4a010000"],
    },
    Recorded {
        leaf: "exposure.gain",
        capture: "capture 01",
        answers: ["0100", "4000", "0100", "0100"],
    },
    Recorded {
        leaf: "exposure.compensation",
        capture: "capture 01",
        answers: ["0000", "1200", "0100", "0900"],
    },
    Recorded {
        leaf: "image.brightness",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.contrast",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.saturation",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.hue",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.sharpness",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.white_balance.temperature",
        capture: "capture 01",
        answers: ["d007", "1027", "6400", "8813"],
    },
    Recorded {
        leaf: "focus.position",
        capture: "capture 16",
        answers: ["0000", "6400", "0100", "0000"],
    },
    // a boolean: the recordings hold only its GET_DEF; min, max and step are V4L2's 0, 1, 1
    Recorded {
        leaf: "image.white_balance.auto",
        capture: "capture 16",
        answers: ["", "", "", "01"],
    },
];

/// The control id the model reads a leaf's range from: its own table, not a copy of it here.
fn control(leaf: &str) -> u32 {
    if leaf == "focus.position" {
        return decode::FOCUS_ABSOLUTE;
    }
    decode::UVC
        .iter()
        .find(|(l, _)| *l == leaf)
        .unwrap_or_else(|| panic!("{leaf}: not a UVC leaf of the model"))
        .1
}

fn le(hex: &str) -> Option<i32> {
    if hex.is_empty() {
        return None;
    }
    let b: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    Some(match b.len() {
        1 => i32::from(b[0]),
        2 => i32::from(i16::from_le_bytes([b[0], b[1]])),
        4 => i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        n => panic!("a {n}-byte answer"),
    })
}

/// The replay transport answers the range requests with the recorded answers, as the camera
/// did, and the description's bounds, steps and defaults equal them: the comparison of the live
/// test `live_the_uvc_ranges_the_description_holds` (tests/live.rs), made on the recorded data
/// of two firmware versions. This does not replace that live test: the replay returns only what
/// is fed to it here, so it proves the description against the recording, not the camera of
/// the session (camera session CS-1 runs the live test on the unit). The control ids are the
/// model's own (`decode::UVC`).
#[test]
fn the_recorded_uvc_ranges_equal_the_description() {
    let d = description_of("tiny2lite");
    let mut replay = Replay::new(2);
    for r in &RECORDED {
        let [min, max, step, default] = r.answers.map(le);
        replay = replay.range(
            control(r.leaf),
            ControlRange {
                min: min.unwrap_or(0),
                max: max.unwrap_or(1),
                step: step.unwrap_or(1),
                default: default.unwrap(),
            },
        );
    }
    // every UVC leaf of the model that has a bound, step or default is among the fixtures
    for (leaf, _) in decode::UVC {
        let s = &d.settings[leaf];
        if [&s.min, &s.max, &s.step].iter().any(|b| b.is_some()) || s.default.is_some() {
            assert!(
                RECORDED.iter().any(|r| r.leaf == leaf),
                "{leaf}: has bounds, but no recorded range answers"
            );
        }
    }
    let mut mismatches = Vec::new();
    for r in &RECORDED {
        let range = replay.ctrl_range(control(r.leaf)).unwrap().unwrap();
        let s = &d.settings[r.leaf];
        // exposure.time is in 100 µs units on the camera, µs in the description
        let scale = if r.leaf == "exposure.time" {
            100.0
        } else {
            1.0
        };
        let ours = |b: &Option<obsbot_proto::Bound>| b.as_ref().and_then(|b| b.get(None));
        let default = s
            .default
            .as_ref()
            .and_then(|v| v.as_f64().or_else(|| v.as_bool().map(f64::from)));
        for (key, ours, camera, recorded) in [
            ("min", ours(&s.min), range.min, !r.answers[0].is_empty()),
            ("max", ours(&s.max), range.max, !r.answers[1].is_empty()),
            ("step", ours(&s.step), range.step, !r.answers[2].is_empty()),
            ("default", default, range.default, true),
        ] {
            let Some(ours) = ours else { continue };
            assert!(recorded, "{}#{key}: not recorded", r.leaf);
            if (ours - f64::from(camera) * scale).abs() > 1e-9 {
                mismatches.push(format!(
                    "{}#{key}: description {ours}, {} {}",
                    r.leaf,
                    r.capture,
                    f64::from(camera) * scale
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    // every bound, step and default these answers back has its row, with the capture
    for r in &RECORDED {
        for row in levels::LEVELS.iter().filter(|row| {
            ["@min", "@max", "#step", "#default"]
                .iter()
                .any(|suffix| row.key == format!("{}{suffix}", r.leaf))
        }) {
            assert_eq!(row.level, Level::Captured, "{}", row.key);
            assert!(
                row.evidence.contains(&r.capture["capture ".len()..]),
                "{}: {} is not in \"{}\"",
                row.key,
                r.capture,
                row.evidence
            );
        }
    }
}
```

- [ ] **Step 6: Run, mutate, loop**

```sh
nice -n 19 cargo test -j 2 --locked -p obsbot-device --test evidence
```

Expected: `7 passed`. Then, each undone afterwards (all measured on 2026-11-24 on `05e66e9`): change one recorded answer (`"3200"` of `image.brightness` to `"3300"`) → `the_recorded_uvc_ranges_equal_the_description` fails with `image.brightness#default: description 50, capture 01 51`; change the evidence text of `tracking.mode` in `levels.rs` (`capture 11` to `capture 12`) → `the_field_rows_follow_the_ledger` fails naming `tracking.mode`; delete the row `exposure.anti_flicker=auto` (level `none`) from `levels.rs` → `every_registered_model_meets_the_evidence_rule` fails with `exposure.anti_flicker=auto (U-25): a mark of 9.3 without a row of level none` (and the tests that pin the list of problems fail with it); change that mark in `UNBACKED` from `U-25` to `U-24` → `the_marks_of_9_3_follow_the_ledger` fails; set `ENTRY.support` to `Support::Tested` in `plan.rs` → `every_registered_model_meets_the_evidence_rule` fails, listing the U-rows (and `plan::tests::the_registry_entry`). Then 20 runs in a loop: 20 of 20.

- [ ] **Step 7: Gate and commit** — the workspace gate (547 passed, 14 ignored on `05e66e9` with this task and the other corrections); commit `obsbot-device: the evidence-level test of spec 8.5; the recorded UVC ranges back the bounds`.

### Task w3-4 (key `w3-4-packages`): Two-binary packages

**Files:**

- Modify: `crates/obsbot4linux/Cargo.toml` (`description`, `[package.metadata.deb]`, `[package.metadata.generate-rpm]`)
- Modify: `packaging/arch/PKGBUILD` (`pkgdesc`, `check()`, `package()`)
- Modify: `.github/workflows/release.yml` (jobs `binary`, `deb`, `rpm`, `contents`)
- Modify: `packaging/contents.txt` (regenerated)

**Depends on:** w3-1 (the `makepkg` check in a deep directory, Step 5). **Gate:** `w2`. No `Cargo.lock` change (a manifest's `description` and metadata tables are not in the lock; `cargo build --locked` passes, measured).

- [ ] **Step 1: deb and rpm**

```diff
--- a/crates/obsbot4linux/Cargo.toml
+++ b/crates/obsbot4linux/Cargo.toml
@@ -1,6 +1,6 @@
 [package]
 name = "obsbot4linux"
-description = "Control OBSBOT webcams on Linux from the console and the DMS widget"
+description = "Control OBSBOT webcams on Linux: a camera daemon and its command line"
 version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
@@ -32,13 +32,17 @@
 [package.metadata.deb]
 maintainer = "Thomas Kroll <t.kroll@outlook.com>"
 section = "video"
-extended-description = "Most settings OBSBOT Center offers, from the console and from the DMS bar widget.\nWhat is missing: https://github.com/hellwand/obsbot4linux/blob/main/docs/center-parity.md\nThe obsbot4linux daemon (systemd user socket) owns the camera state."
+extended-description = "obsbot4linuxd owns the cameras and serves protocol 2 on a systemd user socket;\nobsbot4linux is its command line. The supported models and their support level\nare listed in the README: https://github.com/hellwand/obsbot4linux#supported-cameras"
 depends = "$auto"
 license-file = ["../../LICENSE", "0"]
 assets = [
     ["target/release/obsbot4linux", "usr/bin/", "755"],
+    ["target/release/obsbot4linuxd", "usr/bin/", "755"],
     ["../../packaging/systemd/obsbot4linux.service", "usr/lib/systemd/user/", "644"],
     ["../../packaging/systemd/obsbot4linux.socket", "usr/lib/systemd/user/", "644"],
+    ["../../packaging/completions/obsbot4linux.bash", "usr/share/bash-completion/completions/obsbot4linux", "644"],
+    ["../../packaging/completions/_obsbot4linux", "usr/share/zsh/vendor-completions/", "644"],
+    ["../../schema/protocol-2.json", "usr/share/obsbot4linux/", "644"],
     ["../../README.md", "usr/share/doc/obsbot4linux/", "644"],
     ["../../CHANGELOG.md", "usr/share/doc/obsbot4linux/", "644"],
 ]
@@ -46,8 +50,12 @@
 [package.metadata.generate-rpm]
 assets = [
     { source = "target/release/obsbot4linux", dest = "/usr/bin/obsbot4linux", mode = "755" },
+    { source = "target/release/obsbot4linuxd", dest = "/usr/bin/obsbot4linuxd", mode = "755" },
     { source = "../../packaging/systemd/obsbot4linux.service", dest = "/usr/lib/systemd/user/obsbot4linux.service", mode = "644" },
     { source = "../../packaging/systemd/obsbot4linux.socket", dest = "/usr/lib/systemd/user/obsbot4linux.socket", mode = "644" },
+    { source = "../../packaging/completions/obsbot4linux.bash", dest = "/usr/share/bash-completion/completions/obsbot4linux", mode = "644" },
+    { source = "../../packaging/completions/_obsbot4linux", dest = "/usr/share/zsh/site-functions/_obsbot4linux", mode = "644" },
+    { source = "../../schema/protocol-2.json", dest = "/usr/share/obsbot4linux/protocol-2.json", mode = "644" },
     { source = "../../README.md", dest = "/usr/share/doc/obsbot4linux/README.md", mode = "644", doc = true },
     { source = "../../CHANGELOG.md", dest = "/usr/share/doc/obsbot4linux/CHANGELOG.md", mode = "644", doc = true },
     { source = "../../LICENSE", dest = "/usr/share/licenses/obsbot4linux/LICENSE", mode = "644" },
```

- [ ] **Step 2: Arch**

```diff
--- a/packaging/arch/PKGBUILD
+++ b/packaging/arch/PKGBUILD
@@ -4,7 +4,7 @@
 # Cargo.toml's version; scripts/changelog-section.sh fails the release unless they match.
 pkgver=0.13.2
 pkgrel=1
-pkgdesc='Control OBSBOT webcams (Tiny 2 Lite) from the console, with a daemon for the DMS widget'
+pkgdesc='Control OBSBOT webcams on Linux: a camera daemon and its command line'
 arch=('x86_64' 'aarch64')
 url='https://github.com/hellwand/obsbot4linux'
 license=('GPL-3.0-or-later')
@@ -34,15 +34,20 @@
   cargo build --frozen --release
 }

+# The tests run their daemons in-process on temporary sockets; they need no camera, no systemd
+# and no XDG_RUNTIME_DIR (design spec 11), so it is unset here to prove it.
 check() {
   cd "$_src"
-  cargo test --frozen
+  env -u XDG_RUNTIME_DIR -u OBSBOT4LINUX_SOCKET cargo test --frozen --workspace
 }

 package() {
   cd "$_src"
-  install -Dm755 target/release/obsbot4linux -t "$pkgdir/usr/bin"
+  install -Dm755 target/release/obsbot4linux{,d} -t "$pkgdir/usr/bin"
   install -Dm644 packaging/systemd/obsbot4linux.{socket,service} -t "$pkgdir/usr/lib/systemd/user"
+  install -Dm644 packaging/completions/obsbot4linux.bash "$pkgdir/usr/share/bash-completion/completions/obsbot4linux"
+  install -Dm644 packaging/completions/_obsbot4linux -t "$pkgdir/usr/share/zsh/site-functions"
+  install -Dm644 schema/protocol-2.json -t "$pkgdir/usr/share/$pkgname"
   install -Dm644 README.md CHANGELOG.md -t "$pkgdir/usr/share/doc/$pkgname"
   install -Dm644 LICENSE -t "$pkgdir/usr/share/licenses/$pkgname"
 }
```

- [ ] **Step 2b: `jq` for `check()` is already there (W2's w2-9)** — W2's `crates/obsbot4linux/tests/check_model.rs` (w2-2) runs `scripts/check-model.sh` inside the workspace tests, which needs `bash` and `jq`. W2's task w2-9 (landed before this task) put `checkdepends=('jq')` below `makedepends` in `packaging/arch/PKGBUILD` and `jq` into the `pacman -Syu` line of `scripts/build-arch.sh` (the container runs `makepkg` without `-s`, so a missing `checkdepends` package fails the build). Check that both lines are there (`grep -n "checkdepends=('jq')" packaging/arch/PKGBUILD`; `grep -n 'cargo jq' scripts/build-arch.sh`) and keep them when the diff of Step 2 is applied; do not add them a second time, and do not edit `scripts/build-arch.sh` in this task. If either is missing, stop and report `plan_mismatch`. The deb and rpm jobs package the binaries of the `binary` job and run no tests.

- [ ] **Step 3: The workflow** (the `contents` job also checks the claims of spec 12.1: no `libudev`, no install script, the glibc floor; see Step 6b)

```diff
--- a/.github/workflows/release.yml
+++ b/.github/workflows/release.yml
@@ -151,20 +151,24 @@
           ver=$VER
           mkdir dist
           dir=obsbot4linux-$ver-$ARCH-linux
-          mkdir -p "$dir/systemd"
-          cp target/release/obsbot4linux README.md CHANGELOG.md LICENSE "$dir"
+          mkdir -p "$dir/systemd" "$dir/completions"
+          cp target/release/obsbot4linux target/release/obsbot4linuxd "$dir"
+          cp README.md CHANGELOG.md LICENSE schema/protocol-2.json "$dir"
           cp packaging/systemd/* "$dir/systemd"
+          cp packaging/completions/* "$dir/completions"
           tar -czf "dist/$dir.tar.gz" "$dir"
       - uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
         with:
           name: dist-tarball-${{ matrix.arch }}
           path: dist/*
           if-no-files-found: error
-      # deb and rpm package exactly this binary.
+      # deb and rpm package exactly these binaries.
       - uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
         with:
           name: binary-${{ matrix.arch }}
-          path: target/release/obsbot4linux
+          path: |
+            target/release/obsbot4linux
+            target/release/obsbot4linuxd
           if-no-files-found: error

   deb:
@@ -199,8 +203,8 @@
         run: |
           ver=$VER
           mkdir dist
-          # no rebuild: package the binary from the binary job (artifacts drop the mode)
-          chmod +x target/release/obsbot4linux
+          # no rebuild: package the binaries from the binary job (artifacts drop the mode)
+          chmod +x target/release/obsbot4linux target/release/obsbot4linuxd
           cargo deb -p obsbot4linux --no-build --deb-revision "" -o "dist/obsbot4linux_${ver}_$DEB.deb"
       - uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
         with:
@@ -237,7 +241,7 @@
         run: |
           ver=$VER
           mkdir dist
-          chmod +x target/release/obsbot4linux
+          chmod +x target/release/obsbot4linux target/release/obsbot4linuxd
           cargo generate-rpm -p crates/obsbot4linux -o "dist/obsbot4linux-$ver-1.$ARCH.rpm"
       - uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
         with:
@@ -292,7 +296,7 @@
           path: dist
           pattern: dist-*
           merge-multiple: true
-      - run: sudo apt-get install -y --no-install-recommends libarchive-tools
+      - run: sudo apt-get install -y --no-install-recommends libarchive-tools rpm
       # [profile.release] strip = true must reach every package (cargo ignores a member
       # manifest's profile, and the file list alone would not show it)
       - name: The packaged binaries are stripped
@@ -302,16 +306,54 @@
             *.deb) bsdtar -xOf "$pkg" 'data.tar*' > payload ;;
             *) cp "$pkg" payload ;;
             esac
-            bin=$(bsdtar -tf payload | grep -E '(usr/bin|-linux)/obsbot4linux$')
-            bsdtar -xOf payload "$bin" > obsbot4linux.bin
-            info=$(file -b obsbot4linux.bin)
-            echo "$pkg: $info"
-            case $info in
-            *', stripped') ;;
-            *) echo "::error::$pkg ships an unstripped binary"; exit 1 ;;
+            for name in obsbot4linux obsbot4linuxd; do
+              bin=$(bsdtar -tf payload | grep -E "(usr/bin|-linux)/$name\$")
+              bsdtar -xOf payload "$bin" > binary
+              info=$(file -b binary)
+              echo "$pkg $name: $info"
+              case $info in
+              *', stripped') ;;
+              *) echo "::error::$pkg ships an unstripped $name"; exit 1 ;;
+              esac
+            done
+          done
+          rm -f payload binary
+      # spec 12.1: "There is no libudev dependency" and "Packages do not enable the socket
+      # unit" (a package that carries no install script cannot), and the glibc floor that the
+      # README names holds for both binaries
+      - name: The packages keep the claims of spec 12.1
+        run: |
+          for pkg in dist/obsbot4linux*; do
+            case $pkg in
+            *.deb) bsdtar -xOf "$pkg" 'data.tar*' > payload ;;
+            *) cp "$pkg" payload ;;
+            esac
+            for name in obsbot4linux obsbot4linuxd; do
+              bin=$(bsdtar -tf payload | grep -E "(usr/bin|-linux)/$name\$")
+              bsdtar -xOf payload "$bin" > binary
+              if readelf -d binary | grep -q 'NEEDED.*udev'; then
+                echo "::error::$pkg: $name links libudev"; exit 1
+              fi
+              glibc=$(readelf --dyn-syms -W binary | grep -oE 'GLIBC_[0-9]+\.[0-9]+(\.[0-9]+)?' | sort -Vu | tail -n1)
+              echo "$pkg $name: needs $glibc"
+              # the Arch package is built on current Arch: no floor
+              case $pkg in
+              *.pkg.tar.zst) ;;
+              *)
+                [ "$(printf '%s\nGLIBC_2.34\n' "$glibc" | sort -V | tail -n1)" = GLIBC_2.34 ] || {
+                  echo "::error::$pkg: $name needs $glibc, the README names glibc 2.34"; exit 1
+                } ;;
+              esac
+            done
+            case $pkg in
+            *.deb) scripts=$(bsdtar -xOf "$pkg" 'control.tar*' | bsdtar -tf - | grep -E '/(pre|post)(inst|rm)$' || true) ;;
+            *.rpm) scripts=$(rpm -qp --scripts "$pkg" | grep scriptlet || true) ;;
+            *.pkg.tar.zst) scripts=$(bsdtar -tf "$pkg" | grep -x '\.INSTALL' || true) ;;
+            *) scripts= ;;
             esac
+            [ -z "$scripts" ] || { echo "::error::$pkg has install scripts: $scripts"; exit 1; }
           done
-          rm -f payload obsbot4linux.bin
+          rm -f payload binary
       - name: Compare the package contents
         env:
           VER: ${{ needs.version.outputs.ver }}
```

- [ ] **Step 4: Build the packages locally and list them**

```sh
nice -n 19 cargo build -j 2 --release --locked
ver=0.13.2; d=$TMPDIR/dist; mkdir -p "$d"
# tarball, as the binary job builds it (x86_64)
w=$TMPDIR/tb; dir=obsbot4linux-$ver-x86_64-linux; mkdir -p "$w/$dir/systemd" "$w/$dir/completions"
cp "$CARGO_TARGET_DIR"/release/obsbot4linux{,d} README.md CHANGELOG.md LICENSE schema/protocol-2.json "$w/$dir"
cp packaging/systemd/* "$w/$dir/systemd"; cp packaging/completions/* "$w/$dir/completions"
tar -czf "$d/$dir.tar.gz" -C "$w" "$dir"
nice -n 19 cargo deb -p obsbot4linux --no-build --deb-revision "" -o "$d/obsbot4linux_${ver}_amd64.deb"
nice -n 19 cargo generate-rpm -p crates/obsbot4linux -s "version = \"$ver\"" -o "$d/obsbot4linux-$ver-1.x86_64.rpm"
```

(`cargo deb` warns that `dpkg-shlibdeps` is missing on Arch and leaves `Depends` empty; the CI runner has it.)

- [ ] **Step 5: `makepkg` with `check()`, deep and without `XDG_RUNTIME_DIR`**

```sh
m=$TMPDIR/mk/build-dir-that-is-deep-like-an-aur-helper-cache; mkdir -p "$m"      # 90 bytes in the measured run
git ls-files -co --exclude-standard -z | tar --null -C . --transform "s,^,obsbot4linux-$ver/," -czf "$m/obsbot4linux-$ver.tar.gz" -T -
cp packaging/arch/PKGBUILD "$m/"
(cd "$m" && env -u CARGO_TARGET_DIR -u XDG_RUNTIME_DIR CARGO_BUILD_JOBS=2 \
  OBSBOT4LINUX_TARBALL="$m/obsbot4linux-$ver.tar.gz" PKGDEST="$m/out" PKGEXT=.pkg.tar.zst \
  BUILDDIR="$m/build" SRCDEST="$m/src" nice -n 19 timeout 3000 makepkg --noconfirm)
cp "$m"/out/*.pkg.tar.zst "$d/"
# the daemon tests carry timing windows (the last commits on rebuild fixed three): repeat them
for i in $(seq 10); do nice -n 19 cargo test -q -j 2 --locked -p obsbot4linuxd || break; done
```

`TMPDIR` stays set (it is the task's own short directory under the build directory, never `/tmp`; the sockets of the tests fit in it); only `XDG_RUNTIME_DIR` is unset, which is what the acceptance needs, and `check()` unsets it and `OBSBOT4LINUX_SOCKET` again. Expected: `==> Finished making: obsbot4linux 0.13.2-1`, `check()` green (measured on `05e66e9` in a build directory 90 bytes deep). The tarball must hold the working tree: commit first, or use `git ls-files -co --exclude-standard` as above (new files that are not tracked yet are needed). The ten repetitions of the daemon tests: 10 of 10 green on `05e66e9` (measured; `check()` now gates both Arch release jobs on these timing-sensitive tests, so a failure here is a finding for track B, not something to retry away).

- [ ] **Step 6: The listing**

```sh
scripts/package-contents.sh "$d" "$ver" | awk '{print; print}' | LC_ALL=C sort > packaging/contents.txt
```

(The CI list holds each line twice, once per architecture, after ARCH replaces both.) Expected `packaging/contents.txt`:

```text
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- .BUILDINFO
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- .BUILDINFO
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- .MTREE
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- .MTREE
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- .PKGINFO
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- .PKGINFO
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/lib/systemd/user/obsbot4linux.service
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/lib/systemd/user/obsbot4linux.service
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/lib/systemd/user/obsbot4linux.socket
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/lib/systemd/user/obsbot4linux.socket
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/bash-completion/completions/obsbot4linux
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/bash-completion/completions/obsbot4linux
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/doc/obsbot4linux/CHANGELOG.md
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/doc/obsbot4linux/CHANGELOG.md
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/doc/obsbot4linux/README.md
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/doc/obsbot4linux/README.md
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/licenses/obsbot4linux/LICENSE
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/licenses/obsbot4linux/LICENSE
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/obsbot4linux/protocol-2.json
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/obsbot4linux/protocol-2.json
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/zsh/site-functions/_obsbot4linux
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rw-r--r-- usr/share/zsh/site-functions/_obsbot4linux
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rwxr-xr-x usr/bin/obsbot4linux
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rwxr-xr-x usr/bin/obsbot4linux
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rwxr-xr-x usr/bin/obsbot4linuxd
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst -rwxr-xr-x usr/bin/obsbot4linuxd
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/bin/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/bin/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/lib/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/lib/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/lib/systemd/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/lib/systemd/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/lib/systemd/user/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/lib/systemd/user/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/bash-completion/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/bash-completion/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/bash-completion/completions/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/bash-completion/completions/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/doc/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/doc/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/doc/obsbot4linux/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/doc/obsbot4linux/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/licenses/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/licenses/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/licenses/obsbot4linux/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/licenses/obsbot4linux/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/obsbot4linux/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/obsbot4linux/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/zsh/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/zsh/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/zsh/site-functions/
obsbot4linux-VERSION-1-ARCH.pkg.tar.zst drwxr-xr-x usr/share/zsh/site-functions/
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.service
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.service
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.socket
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.socket
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/bash-completion/completions/obsbot4linux
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/bash-completion/completions/obsbot4linux
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/doc/obsbot4linux/CHANGELOG.md
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/doc/obsbot4linux/CHANGELOG.md
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/doc/obsbot4linux/README.md
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/doc/obsbot4linux/README.md
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/licenses/obsbot4linux/LICENSE
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/licenses/obsbot4linux/LICENSE
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/obsbot4linux/protocol-2.json
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/obsbot4linux/protocol-2.json
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/zsh/site-functions/_obsbot4linux
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/zsh/site-functions/_obsbot4linux
obsbot4linux-VERSION-1.ARCH.rpm -rwxr-xr-x ./usr/bin/obsbot4linux
obsbot4linux-VERSION-1.ARCH.rpm -rwxr-xr-x ./usr/bin/obsbot4linux
obsbot4linux-VERSION-1.ARCH.rpm -rwxr-xr-x ./usr/bin/obsbot4linuxd
obsbot4linux-VERSION-1.ARCH.rpm -rwxr-xr-x ./usr/bin/obsbot4linuxd
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/CHANGELOG.md
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/CHANGELOG.md
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/LICENSE
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/LICENSE
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/README.md
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/README.md
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/completions/_obsbot4linux
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/completions/_obsbot4linux
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/completions/obsbot4linux.bash
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/completions/obsbot4linux.bash
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/protocol-2.json
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/protocol-2.json
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/systemd/obsbot4linux.service
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/systemd/obsbot4linux.service
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/systemd/obsbot4linux.socket
obsbot4linux-VERSION-ARCH-linux.tar.gz -rw-r--r-- obsbot4linux-VERSION-ARCH-linux/systemd/obsbot4linux.socket
obsbot4linux-VERSION-ARCH-linux.tar.gz -rwxr-xr-x obsbot4linux-VERSION-ARCH-linux/obsbot4linux
obsbot4linux-VERSION-ARCH-linux.tar.gz -rwxr-xr-x obsbot4linux-VERSION-ARCH-linux/obsbot4linux
obsbot4linux-VERSION-ARCH-linux.tar.gz -rwxr-xr-x obsbot4linux-VERSION-ARCH-linux/obsbot4linuxd
obsbot4linux-VERSION-ARCH-linux.tar.gz -rwxr-xr-x obsbot4linux-VERSION-ARCH-linux/obsbot4linuxd
obsbot4linux-VERSION-ARCH-linux.tar.gz drwxr-xr-x obsbot4linux-VERSION-ARCH-linux/
obsbot4linux-VERSION-ARCH-linux.tar.gz drwxr-xr-x obsbot4linux-VERSION-ARCH-linux/
obsbot4linux-VERSION-ARCH-linux.tar.gz drwxr-xr-x obsbot4linux-VERSION-ARCH-linux/completions/
obsbot4linux-VERSION-ARCH-linux.tar.gz drwxr-xr-x obsbot4linux-VERSION-ARCH-linux/completions/
obsbot4linux-VERSION-ARCH-linux.tar.gz drwxr-xr-x obsbot4linux-VERSION-ARCH-linux/systemd/
obsbot4linux-VERSION-ARCH-linux.tar.gz drwxr-xr-x obsbot4linux-VERSION-ARCH-linux/systemd/
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.service
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.service
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.socket
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.socket
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/bash-completion/completions/obsbot4linux
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/bash-completion/completions/obsbot4linux
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/doc/obsbot4linux/CHANGELOG.md
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/doc/obsbot4linux/CHANGELOG.md
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/doc/obsbot4linux/README.md
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/doc/obsbot4linux/README.md
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/doc/obsbot4linux/copyright
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/doc/obsbot4linux/copyright
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/obsbot4linux/protocol-2.json
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/obsbot4linux/protocol-2.json
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/zsh/vendor-completions/_obsbot4linux
obsbot4linux_VERSION_ARCH.deb -rw-r--r-- ./usr/share/zsh/vendor-completions/_obsbot4linux
obsbot4linux_VERSION_ARCH.deb -rwxr-xr-x ./usr/bin/obsbot4linux
obsbot4linux_VERSION_ARCH.deb -rwxr-xr-x ./usr/bin/obsbot4linux
obsbot4linux_VERSION_ARCH.deb -rwxr-xr-x ./usr/bin/obsbot4linuxd
obsbot4linux_VERSION_ARCH.deb -rwxr-xr-x ./usr/bin/obsbot4linuxd
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/bin/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/bin/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/lib/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/lib/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/lib/systemd/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/lib/systemd/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/lib/systemd/user/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/lib/systemd/user/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/bash-completion/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/bash-completion/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/bash-completion/completions/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/bash-completion/completions/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/doc/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/doc/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/doc/obsbot4linux/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/doc/obsbot4linux/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/obsbot4linux/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/obsbot4linux/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/zsh/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/zsh/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/zsh/vendor-completions/
obsbot4linux_VERSION_ARCH.deb drwxr-xr-x ./usr/share/zsh/vendor-completions/
```

Every listing shows `obsbot4linux`, `obsbot4linuxd`, both units, both completions and `protocol-2.json` (spec 10.3 W3 acceptance).

- [ ] **Step 6b: The claims of spec 12.1, on the local packages** — spec 12.1: "There is no libudev dependency" and "Packages do not enable the socket unit"; the README names a glibc floor. The `contents` job's new step checks all three on every package: `readelf -d` shows no `NEEDED` entry with `udev` in either binary, the highest `GLIBC_x.y` of `readelf --dyn-syms` is at most 2.34 for the deb, rpm and tarball (not for the Arch package, which is built on current Arch: measured, the same build needs `GLIBC_2.34` in the three and `GLIBC_2.39` in the Arch package on this desktop), and no package has an install script (deb: no `preinst`, `postinst`, `prerm` or `postrm` in `control.tar`; Arch: no `.INSTALL`; rpm: `rpm -qp --scripts` prints no scriptlet). To run it locally, take the step's `run:` text into a file, make a directory `dist` with the four local packages next to it, and run `bash -e file` with a stand-in `rpm` first in `PATH`. Measured on 2026-11-24: passes on the four packages; fails with `has install scripts: ./postinst` for a deb that was repacked with a `postinst`, with `has install scripts: .INSTALL` for an Arch package with one added, and with `needs GLIBC_2.35, the README names glibc 2.34` when `readelf` is replaced by a stand-in that reports it. The rpm branch (`rpm -qp --scripts`, needs the `rpm` package that the step installs) and the aarch64 packages are not run here: the release.yml dry run is their proof.

- [ ] **Step 7: Gate, commit, CI** — `actionlint`, the workspace gate; also `grep -n '#supported-cameras' crates/obsbot4linux/Cargo.toml` finds the link target that the deb's description names (the README keeps the heading "Supported cameras", w3-5); commit `packaging: both binaries, the completions and the schema in every package`. The pull request runs `release.yml` as a dry run: the eight package jobs and `contents` must be green. `packaging/contents.txt` is the local listing of Step 6 and is never overwritten by the run's artifact: if `contents` differs, diff the run's `package-contents` artifact against `packaging/contents.txt` line by line; a difference is either fixed in the workflow, or (a runner-side detail only) written into the pull request line by line with its reason, and the reviewer reads that diff, not just "green". Acceptance: that run green, its eight `dist-*` artifacts present.

### Task w3-5 (key `w3-5-readme`): The 1.0 README

**Files:** Modify: `README.md` (replace the whole file); Delete: `docs/center-parity.md` (P2, decided in the cross-check of 2026-11-24). **Gate:** `w2`.

- [ ] **Step 1: Write the README**

`README.md`, whole:

````markdown
# OBSBot4Linux

Control OBSBOT webcams on Linux. `obsbot4linuxd` owns the cameras and serves protocol 2 on a
systemd user socket; `obsbot4linux` is its command line. The Dux (DankMaterialShell) bar
widget [obsbot4linux-dux](https://github.com/hellwand/obsbot4linux-dux) speaks the same
protocol. What a command sends is tested byte for byte against recorded frames, and the
evidence for every setting is cited in [docs/field-ledger.md](docs/field-ledger.md).

Version 1.0 replaces the 0.13 daemon, protocol and command line. Coming from 0.13, read
"Upgrading from 0.13" in the [CHANGELOG](CHANGELOG.md) first: the commands are new, and the
socket unit must be enabled.

## Supported cameras

| Model              | USB         | Support  | Firmware tested |
| ------------------ | ----------- | -------- | --------------- |
| OBSBOT Tiny 2 Lite | `3564:fef9` | untested | –               |

`tested`: the maintainers test every setting and action of the model on a unit, and a release
candidate is published before that check is finished. `untested`: built from an owner's
recordings, not verified by the maintainers; the command line prints the level next to such a
device. Any other OBSBOT camera (USB vendor `3564`) is listed as `unsupported` and
receives nothing; `obsbot4linux devices --report` prints what an issue report needs.

Only what the camera itself offers and reports is there, and a request sends exactly what it
names: `do gimbal.move` moves the gimbal and leaves the tracking mode as it is. Everything set
is stored by the camera; the daemon keeps no file.

## Install

Download a package from the [releases](https://github.com/hellwand/obsbot4linux/releases)
(x86_64 and aarch64; `SHA256SUMS` lists the checksums), then install it, with `VER` set to the
release's version (for example `VER=1.0.0`):

```sh
sudo apt install ./obsbot4linux_${VER}_amd64.deb             # Debian, Ubuntu (arm64 too)
sudo dnf install ./obsbot4linux-$VER-1.x86_64.rpm            # Fedora, openSUSE: zypper install
sudo pacman -U obsbot4linux-$VER-1-x86_64.pkg.tar.zst        # Arch Linux, Arch Linux ARM (aarch64)
tar -xzf obsbot4linux-$VER-x86_64-linux.tar.gz               # tarball, see below
```

The Arch package of a release candidate is named without the "-" (`1.0.0rc.1`). The
deb, rpm and tarball binaries need glibc 2.34 or newer (Debian 12, Ubuntu 22.04, Fedora 35,
RHEL 9 or openSUSE Leap 15.6 and newer); the Arch package is built on current Arch.

The packages install `/usr/bin/obsbot4linux`, `/usr/bin/obsbot4linuxd`, the user units, bash
and zsh completions and the protocol schema `/usr/share/obsbot4linux/protocol-2.json`. They
enable nothing. Enable the socket once:

```sh
systemctl --user enable --now obsbot4linux.socket
```

The socket starts `obsbot4linux.service` (`obsbot4linuxd`, with `NoNewPrivileges=yes`) on the
first connection. No command starts a daemon itself: without the socket, every command exits 4
and prints the line above. After a package upgrade, restart the daemon:
`systemctl --user restart obsbot4linux.service`.

From the tarball, put both binaries on your `PATH`, copy `systemd/*` to
`~/.config/systemd/user/` and the completions where your shell reads them. The service runs
`/usr/bin/obsbot4linuxd`; for a binary elsewhere, `systemctl --user edit obsbot4linux.service`
and point it there (the empty `ExecStart=` clears the packaged one):

```ini
[Service]
ExecStart=
ExecStart=%h/.local/bin/obsbot4linuxd
```

Build from source (Rust 1.88 or newer):

```sh
cargo build --release --locked      # target/release/obsbot4linux and obsbot4linuxd
cd packaging/arch && makepkg -si    # Arch package of the committed HEAD
```

## Use

The command line is generic: it maps one to one onto protocol 2, so a camera's settings and
actions come from its description, not from commands of their own. Every command sends one
request and chains no other.

```sh
obsbot4linux devices                       # the cameras the daemon sees
obsbot4linux status                        # every setting of the camera, grouped
obsbot4linux describe                      # what the camera offers: settings, values, ranges, actions
obsbot4linux describe gimbal.move          # one setting or action
obsbot4linux get tracking.mode zoom        # some settings or groups
obsbot4linux set tracking.mode=upper_body  # one or more settings, written together
obsbot4linux set exposure.mode=manual exposure.time=20000
obsbot4linux do gimbal.move pan=20 tilt=-5 # an action with its parameters
obsbot4linux do preset.save slot=0 name=Desk
obsbot4linux watch --live pose             # the state as JSON lines, as it changes
```

- **Values:** bool `on|off|true|false`; enum by name (`upper_body`, also `upper-body`); number
  `1.5`; range `min,max`; pose `pan,tilt,zoom`; point `x,y`; string as given.
- **Devices:** with one usable camera, commands use it. Otherwise `--device` takes the full
  id, a serial suffix of at least four characters, the shortened id the output prints
  (`…0001`), or a model key.
- **Output:** human text by default; `--json` prints the protocol objects (errors as
  `{"error":{"code","message"}}`). `devices --report` prints the USB facts of every device
  for an issue report, without serial numbers.
- **Exit codes:** 0 ok; 1 internal error; 2 invalid input; 3 device not usable (asleep, busy,
  unplugged, unsupported, none); 4 daemon unreachable or of another version.
- **Moves while tracking:** `do gimbal.move` and `do gimbal.turn` send only the move. To stop
  the tracking first, run `obsbot4linux set tracking.mode=static` as its own step.
- **Destructive actions** need `confirm=true` (`do factory_reset confirm=true`); the command
  line does not ask.

Protocol 2 is described in [docs/protocol-2.md](docs/protocol-2.md); the evidence for every
setting of the Tiny 2 Lite is in [docs/field-ledger.md](docs/field-ledger.md).

## Licence

GPL-3.0-or-later, see [LICENSE](LICENSE). Code taken over from Tiny4Linux is
EUPL-1.2 and is distributed here under the GPL by the EUPL's compatibility clause
(Article 5); files with such code say so in their header.
````

- [ ] **Step 2: Check it**

```sh
grep -n -i 'static first\|switch.*static\|center\|firmware code\|static analysis' README.md   # expected: nothing
grep -c '^| OBSBOT Tiny 2 Lite | `3564:fef9` | untested |' README.md                       # expected: 1
git rm docs/center-parity.md && git grep -n 'center-parity' -- . ':!docs/superpowers' ':!CHANGELOG.md' ':!crates/obsbot4linux/Cargo.toml'   # expected: nothing (CHANGELOG: w3-6, Cargo.toml: w3-4)
```

(The deb description's link to `#supported-cameras` is checked in w3-4 Step 7.) Every command shown was run against `obsbot4linuxd --fake fake-a,unknown` on 2026-11-24 (again on `05e66e9` for this revision, with `--device fake-a` where two devices are present) (`devices`, `status`, `describe gimbal.move`, `get tracking.mode zoom`, `--device unknown status` → exit 3 and the unsupported message).

- [ ] **Step 3: Commit** — `README: obsbot4linux 1.0` (docs-only: CI skips fmt, clippy and tests; `changelog` and `required` run).

### Task w3-6 (key `w3-6-changelog`): The 1.0 CHANGELOG and migration notes

**Files:** Modify: `CHANGELOG.md` (the `## [Unreleased]` section only; the released sections stay). **Gate:** `w2`.

- [ ] **Step 1: Replace the `[Unreleased]` section**

Everything from `## [Unreleased]` up to (not including) `## [0.13.2] - 2026-10-31` becomes:

```markdown
## [Unreleased]

obsbot4linux 1.0 replaces the 0.13 daemon, protocol, state model and command line. The camera
knowledge of 0.13 (frames, status decoding, recorded bytes and their tests) is carried over; the
rest is new. Read "Upgrading from 0.13" below before installing.

### Untested models

- OBSBOT Tiny 2 Lite (USB 3564:fef9): built from an owner's recordings, not verified by the
  maintainers

### Added

- `obsbot4linuxd`, the camera daemon. It owns every camera, serves protocol 2 on
  `$XDG_RUNTIME_DIR/obsbot4linux.sock` (started by `obsbot4linux.socket`), handles several
  cameras at once and hotplug (a sysfs scan once a second), and gives each camera an id that
  survives re-plugging. An OBSBOT camera that no model covers is listed as `unsupported` and
  receives nothing. The daemon keeps no file.
- Protocol 2: typed envelopes, a path → value state with `pending` and `unread`, one
  description per model (settings, values, ranges, actions), a JSON schema
  (`/usr/share/obsbot4linux/protocol-2.json`) and [docs/protocol-2.md](docs/protocol-2.md).
- The command line `obsbot4linux`, generic over the description: `status`, `get`, `set`, `do`,
  `describe`, `devices` (`--report` for issue reports), `watch`; `--json`, `--device`; exit codes
  0 to 4; bash and zsh completions.
- The packages hold both binaries, the completions and the schema; the user service runs
  `obsbot4linuxd` with `NoNewPrivileges=yes`.

### Changed

- The repository moved to github.com/hellwand/obsbot4linux on 2026-11-12; the old address
  redirects (#109).
- Every command is new; the map from the 0.13 commands is below. The 0.13 commands
  `gimbal [--pan P] [--tilt T]` and `gimbal rate` switched the tracking to static before they
  moved. `do gimbal.move` and `do gimbal.turn` send only the move and leave the tracking as it
  is. A script that relied on the switch runs `obsbot4linux set tracking.mode=static` first, as
  its own step.
- The 0.13 command `preset save N` without a name saved the preset as `Preset <N + 1>`.
  `do preset.save slot=N` sends no name: what the camera shows for such a preset is what the
  state shows, and a script that wants the old names passes `name=…` itself.
- No command starts a daemon any more; without the socket unit every command exits 4 and
  prints `systemctl --user enable --now obsbot4linux.socket`.

### Removed

- `obsbot4linux daemon`, `--direct`, `detect`, request forwarding in `watch`, and the files in
  `$XDG_STATE_HOME/obsbot4linux/` (see the exceptions below).

### Fixed

- The Arch `PKGBUILD` no longer rewrites itself when built from a checkout, so `makepkg -si`
  leaves the repository clean; a local build is labelled with the release version (#120).

### Upgrading from 0.13

1. Install the 1.0 package (it replaces the 0.13 one) and run `systemctl --user daemon-reload`.
2. End the old daemon: `systemctl --user stop obsbot4linux.service`, or end the
   `obsbot4linux daemon` process that `obsbot4linux watch` started, or log out.
3. Remove drop-ins whose `ExecStart` runs `obsbot4linux daemon`
   (`systemctl --user edit obsbot4linux.service` shows them).
4. `systemctl --user enable --now obsbot4linux.socket`. Setups that never enabled the socket
   (the default for widget users) have no daemon otherwise, because 1.0 clients no longer
   start one.
5. Update the widget together with the tool, and reload the shell that hosts it.

`$XDG_STATE_HOME/obsbot4linux/` (it held `state.json` and `daemon.log`) is no longer used and
can be deleted. Nothing is imported, and nothing set on the camera is lost: the camera stores
it.

To go back, the 0.13.2 packages and the widget 0.29.2 stay downloadable from the releases
pages; a rollback beyond them is a fix forward.

Command map (PX-n: the exceptions after the table):

| 0.13.2                                                                  | 1.0                                                                  | Note                                                                                                        |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `--json` (global)                                                       | `--json`                                                             | protocol objects; errors as `{"error":{…}}`                                                                 |
| `--direct` (global)                                                     | –                                                                    | PX-3                                                                                                        |
| `--help`, `--version`                                                   | `--help`, `--version`                                                |                                                                                                             |
| `daemon`                                                                | `obsbot4linuxd`, started by `obsbot4linux.socket`                    |                                                                                                             |
| `watch`                                                                 | `watch`                                                              | prints events; request forwarding is PX-4                                                                   |
| `live-pose on\|off`                                                     | `watch --live pose`; a client sends `live:["pose"]`                  |                                                                                                             |
| `detect`, `detect --json`                                               | `devices`, `devices --json`                                          | PX-5                                                                                                        |
| `status`, `status --json`                                               | `status`, `status --json`                                            | new layout                                                                                                  |
| `sleep`                                                                 | `do sleep`                                                           |                                                                                                             |
| `wake`                                                                  | `do wake`                                                            | replies on acceptance                                                                                       |
| `wake zoom F`                                                           | `set sleep.wake_zoom=F`                                              | no visible effect on the Tiny 2 Lite (#102)                                                                 |
| `wake mode M`                                                           | `set power_on.mode=M`                                                |                                                                                                             |
| `tracking MODE`                                                         | `set tracking.mode=MODE`                                             |                                                                                                             |
| `speed standard\|sport`                                                 | `set tracking.profile=…`                                             |                                                                                                             |
| `speed headroom`                                                        | `set tracking.profile=headroom`                                      |                                                                                                             |
| `speed-mode MODE`                                                       | `set tracking.speed=MODE`                                            |                                                                                                             |
| `framing auto-zoom on\|off`                                             | `set framing.auto_zoom=…`                                            |                                                                                                             |
| `framing size S`                                                        | `set framing.size=S`                                                 |                                                                                                             |
| `framing zoom-speed S`                                                  | `set framing.zoom_speed=S`                                           |                                                                                                             |
| `framing offset [--x X] [--y Y]`                                        | `set [framing.offset.x=X] [framing.offset.y=Y]`                      |                                                                                                             |
| `framing part P`                                                        | `set framing.part=P`                                                 |                                                                                                             |
| `hdr on\|off`                                                           | `set exposure.hdr=…`                                                 |                                                                                                             |
| `exposure global\|face`                                                 | `set exposure.mode=auto exposure.region=…`                           |                                                                                                             |
| `exposure manual`                                                       | `set exposure.mode=manual`                                           |                                                                                                             |
| `exposure manual --time T`                                              | `set exposure.mode=manual exposure.time=<T×100>`                     | time now in µs                                                                                              |
| `exposure manual --gain G`                                              | `set exposure.mode=manual exposure.gain=G`                           |                                                                                                             |
| `zoom F`                                                                | `set zoom.factor=F`                                                  |                                                                                                             |
| `zoom F --speed S`                                                      | `do zoom.move factor=F speed=slow\|medium\|fast`                     | 10/30/50; other S is PX-6                                                                                   |
| `zoom stop`                                                             | `do zoom.stop`                                                       |                                                                                                             |
| `gimbal [--pan P] [--tilt T]`                                           | `do gimbal.move [pan=P] [tilt=T]`                                    | a missing axis keeps its position; the tracking stays as it is                                              |
| `gimbal … --speed S`                                                    | `do gimbal.move … speed=S`                                           | the tracking stays as it is                                                                                 |
| `gimbal stop`                                                           | `do gimbal.stop`                                                     |                                                                                                             |
| `gimbal rate [--pan P] [--tilt T]`                                      | `do gimbal.turn [pan_speed=P] [tilt_speed=T]`                        | the CLI sends `keep:true`; the tracking stays as it is                                                      |
| `gimbal center X Y` (hidden)                                            | –                                                                    | PX-1                                                                                                        |
| `fov wide\|medium\|narrow`                                              | `set zoom.field_of_view=…`                                           |                                                                                                             |
| `led N`                                                                 | `set led.brightness=N`                                               |                                                                                                             |
| `led blink on\|off`                                                     | `set led.blink=…`                                                    | not read back                                                                                               |
| `mirror on\|off`                                                        | `set image.mirror=…`                                                 |                                                                                                             |
| `invert on\|off`                                                        | `set gimbal.invert=…`                                                |                                                                                                             |
| `auto-sleep off`                                                        | `set sleep.auto.enabled=off`                                         |                                                                                                             |
| `auto-sleep N` (N ≥ 1)                                                  | `set sleep.auto.enabled=on sleep.auto.seconds=N`                     |                                                                                                             |
| `auto-sleep 0`                                                          | –                                                                    | PX-7                                                                                                        |
| `sleep-background black\|image`                                         | `set sleep.background=…`                                             |                                                                                                             |
| `sleep-background-mirror on\|off`                                       | `set sleep.background_mirror=…`                                      |                                                                                                             |
| `audio noise L`                                                         | `set audio.noise_reduction=L`                                        |                                                                                                             |
| `audio gain on\|off`                                                    | `set audio.auto_gain=…`                                              |                                                                                                             |
| `audio distance near\|standard\|far`                                    | `set audio.volume=10\|50\|100`                                       | pickup is the volume in bands: near < 50, standard 50–84, far ≥ 85; 10, 50 and 100 are a choice inside them |
| `audio volume N`                                                        | `set audio.volume=N`                                                 |                                                                                                             |
| `audio sleep-mic on\|off`                                               | `set audio.in_sleep=…`                                               |                                                                                                             |
| `audio mic on\|off`                                                     | `set audio.enabled=…`                                                | the camera restarts on both values                                                                          |
| `gesture set target-lock\|zoom\|dynamic-zoom\|direction-change on\|off` | `set gestures.<name>=…`                                              | names with `_`                                                                                              |
| `gesture zoom-ratio R`                                                  | `set gestures.zoom_ratio=R`                                          | precision 0.1                                                                                               |
| `hand right\|left`                                                      | `set hand.side=…`                                                    |                                                                                                             |
| `hand return-here`                                                      | `do hand.return_here`                                                | the current pose                                                                                            |
| `hand return-go`                                                        | `do hand.go_return`                                                  |                                                                                                             |
| `hand return-reset`                                                     | `do hand.return_clear`                                               |                                                                                                             |
| `hand area --pan MIN MAX --tilt MIN MAX`                                | `set hand.area.pan=MIN,MAX hand.area.tilt=MIN,MAX`                   |                                                                                                             |
| `hand area-reset`                                                       | `do hand.area_reset`                                                 |                                                                                                             |
| `hand pan\|tilt on\|off`                                                | `set hand.axes.pan=…` / `set hand.axes.tilt=…`                       |                                                                                                             |
| `hand return-after S`                                                   | `set hand.return_after=S`                                            |                                                                                                             |
| `hand gimbal on\|off`                                                   | `set hand.move_gimbal=…`                                             | not read back                                                                                               |
| `zone on\|off`                                                          | `set zone.enabled=…`                                                 |                                                                                                             |
| `zone follow on\|off`                                                   | `set zone.follow=…`                                                  |                                                                                                             |
| `zone area --pan MIN MAX --tilt MIN MAX`                                | `set zone.area.pan=MIN,MAX zone.area.tilt=MIN,MAX`                   |                                                                                                             |
| `zone return-here`                                                      | `do zone.return_here`                                                | the current pose                                                                                            |
| `zone return-go`                                                        | `do zone.go_return`                                                  |                                                                                                             |
| `zone view list`                                                        | `get views`                                                          |                                                                                                             |
| `zone view save NAME`                                                   | `do view.save name=NAME`                                             | the current pose                                                                                            |
| `zone view save NAME --pan P --tilt T --zoom Z`                         | `do view.save name=NAME pose=P,T,Z`                                  |                                                                                                             |
| `zone view rename ID NAME`                                              | `do view.rename id=ID name=NAME`                                     |                                                                                                             |
| `zone view delete ID`                                                   | `do view.delete id=ID`                                               |                                                                                                             |
| `zone view go ID`                                                       | `do view.go id=ID`                                                   |                                                                                                             |
| `focus mode face\|global`                                               | `set focus.region=…`                                                 |                                                                                                             |
| `focus auto on`                                                         | `set focus.mode=continuous`                                          |                                                                                                             |
| `focus auto off`                                                        | `set focus.mode=manual`                                              |                                                                                                             |
| `focus af continuous\|single\|manual`                                   | `set focus.mode=…`                                                   |                                                                                                             |
| `focus point X Y`                                                       | `do focus.point x=X y=Y`                                             |                                                                                                             |
| `focus set N`                                                           | `set focus.mode=manual focus.position=N`                             | no visible effect on the Tiny 2 Lite (#99)                                                                  |
| `startup set --pan P --tilt T --zoom Z`                                 | `do power_on.set_pose pose=P,T,Z`                                    |                                                                                                             |
| `startup set --preset N`                                                | `do power_on.use_preset slot=N`                                      |                                                                                                             |
| `startup set --current`                                                 | `do power_on.use_current`                                            | the current pose                                                                                            |
| `startup set --ai M\|none`                                              | `set power_on.start.mode=M\|unset`                                   |                                                                                                             |
| `startup set <pose option> --ai M`                                      | the `do` of the pose option, then `set power_on.start.mode=M`        |                                                                                                             |
| `startup show`                                                          | `get power_on`                                                       |                                                                                                             |
| `startup go`                                                            | `do power_on.go`                                                     |                                                                                                             |
| `image brightness\|contrast\|saturation\|hue\|sharpness V`              | `set image.<name>=V`                                                 |                                                                                                             |
| `image white_balance_auto 0\|1`                                         | `set image.white_balance.auto=off\|on`                               |                                                                                                             |
| `image white_balance_temperature V`                                     | `set image.white_balance.auto=off image.white_balance.temperature=V` | needs white balance auto off                                                                                |
| `image gain V`                                                          | `set exposure.gain=V`                                                |                                                                                                             |
| `image power_line_frequency 0\|1\|2`                                    | `set exposure.anti_flicker=off\|50hz\|60hz`                          |                                                                                                             |
| `image backlight_compensation V`                                        | `set exposure.compensation=V`                                        |                                                                                                             |
| `image auto_exposure 0\|1\|3`                                           | `set exposure.mode=auto\|manual\|auto`                               | V4L2 1 = manual; 0.13.2 writes 0 for automatic, and 3 maps to `auto` too                                    |
| `image exposure_time V`                                                 | `set exposure.mode=manual exposure.time=<V×100>`                     | µs                                                                                                          |
| `image focus V`                                                         | `set focus.mode=manual focus.position=V`                             | no visible effect on the Tiny 2 Lite (#99)                                                                  |
| `image autofocus 0\|1`                                                  | `set focus.mode=manual\|continuous`                                  | as `focus auto`                                                                                             |
| `image gamma V`                                                         | –                                                                    | PX-2                                                                                                        |
| `preset list`                                                           | `get presets`                                                        |                                                                                                             |
| `preset save N`                                                         | `do preset.save slot=N`                                              | the current pose; no name                                                                                   |
| `preset save N --name S`                                                | `do preset.save slot=N name=S`                                       |                                                                                                             |
| `preset save N --pan P --tilt T --zoom Z`                               | `do preset.save slot=N pose=P,T,Z`                                   |                                                                                                             |
| `preset rename N S`                                                     | `do preset.rename slot=N name=S`                                     |                                                                                                             |
| `preset delete N`                                                       | `do preset.delete slot=N`                                            |                                                                                                             |
| `preset recall N`                                                       | `do preset.recall slot=N`                                            |                                                                                                             |
| `factory-reset --yes`                                                   | `do factory_reset confirm=true`                                      | erases the camera settings and restarts it                                                                  |
| `factory-reset` without `--yes`                                         | `do factory_reset` is refused with `invalid`                         |                                                                                                             |

Exceptions:

- **PX-1** `gimbal center X Y` (hidden in 0.13): not carried over. It moved the gimbal only
  toward the left and downward in tests.
- **PX-2** `image gamma`: the camera has no gamma control; 0.13's command always failed. No
  function is lost.
- **PX-3** `--direct`: removed. The command line never talks to the camera; socket activation
  starts the daemon.
- **PX-4** request forwarding of `watch` from stdin: removed. The widget speaks protocol 2 on
  the socket.
- **PX-5** `detect` without a daemon and with exit 1: replaced by `devices`, which needs the
  daemon and exits 0 when it lists a device and 3 (`no_device`) when the list is empty. A
  script that needs a usable camera reads `support` from `devices --json`, or runs `status`,
  which exits 3 without one.
- **PX-6** `zoom F --speed S` with any integer S: only `slow|medium|fast` (10, 30, 50), the
  only speeds with a known meaning.
- **PX-7** `auto-sleep 0`: not offered. The camera cannot store "off" together with 0 s, and
  "on with 0 s" was never recorded.
```

The command map is generated from spec 9.1 by this script (run from the spec's checkout; output pasted under "Command map"):

```python
import re, sys
spec = open(sys.argv[1]).read().split('\n')
start = spec.index('### 9.1 Parity table'); end = spec.index('### 9.2 Parity exceptions')
rows = []
for line in spec[start:end]:
    if not line.startswith('| `'):
        continue
    c = [x.strip() for x in re.split(r'(?<!\\)\|', line.strip())[1:-1]]
    old, new, note = c[0], c[1], c[2] if len(c) > 2 else ''
    note = re.sub(r'\b(?:U-\d+|CAM-\d+)\b(?: \([^)]*\))?', '', note)  # camera items
    note = re.sub(r'\s*\([\d., ]*\)', '', note)                      # spec section refs
    note = re.sub(r'(?:effect|`power_on\.start\.source`): *(?=;|$)', '', note)
    note = re.sub(r'`?no_effect`? \(#(\d+)\)', r'no visible effect on the Tiny 2 Lite (#\1)', note)
    note = note.replace('`readback:false`', 'not read back')
    note = note.replace('`warning: [destructive, restart]`', 'erases the camera settings and restarts it')
    note = note.replace('`requires`', 'needs white balance auto off')
    note = re.sub(r'\s*;\s*;\s*', '; ', note)
    note = re.sub(r'^[;\s]+|[;\s,]+$', '', note)
    note = re.sub(r'\s{2,}', ' ', note)
    note = note.replace(', or `single` if  shows that', '')
    note = note.replace('live pose', 'the current pose')
    rows.append((old, new.replace(', or `single` if CAM-14 shows that', ''), note))
w0 = max(len(r[0]) for r in rows + [('0.13.2','','')]); w1 = max(len(r[1]) for r in rows + [('','1.0','')]); w2 = max(len(r[2]) for r in rows + [('','','Note')])
print(f"| {'0.13.2'.ljust(w0)} | {'1.0'.ljust(w1)} | {'Note'.ljust(w2)} |")
print(f"| {'-'*w0} | {'-'*w1} | {'-'*w2} |")
for o, n, t in rows:
    print(f"| {o.ljust(w0)} | {n.ljust(w1)} | {t.ljust(w2)} |")
```

- [ ] **Step 2: Check the migration content of spec 12.2**

```sh
for s in 'systemctl --user daemon-reload' 'systemctl --user stop obsbot4linux.service' \
  'obsbot4linux daemon' 'enable --now obsbot4linux.socket' 'XDG_STATE_HOME/obsbot4linux' \
  'set tracking.mode=static' 'Preset <N + 1>' '**PX-7**' '`factory-reset --yes`' \
  "built from an owner's recordings, not verified by the" 'the 0.13.2 packages and the widget 0.29.2'; do
  grep -qF -- "$s" CHANGELOG.md || echo "missing: $s"; done
scripts/changelog-section.sh v0.13.2 > /dev/null && echo ok
```

Expected: only `ok`. The table has 105 rows, one per row of 9.1.

- [ ] **Step 3: W2's draft** — W2's w2-4 writes the draft of the migration notes to `docs/migration-1.0.md` (no gate; w2-8 updates it after CS-1) and expects W3 to move it into the CHANGELOG. It is on `rebuild` (W2 lands before W3); compare it with this text: every point of 12.2 above stays; wording from the draft that is clearer replaces this text; list what was taken in the pull request. Its command map is the same table without the camera items, so compare it with the one generated here (measured on 2026-11-24: the generator's output equals the table above, 105 rows). `docs/migration-1.0.md` stays; the CHANGELOG carries the notes (spec 12.2), and W4 decides whether the file is deleted.

- [ ] **Step 4: Commit** — `CHANGELOG: the 1.0 notes, the command map and the upgrade steps` (docs-only CI).

### Task w3-7 (key `w3-7-rc-release-path`): The release path of a candidate

**Files:**

- Modify: `scripts/changelog-section.sh`, `scripts/build-arch.sh`, `scripts/package-contents.sh`
- Modify: `packaging/arch/PKGBUILD` (the comment above `pkgver`)
- Modify: `.github/workflows/release.yml` (jobs `verify`, `rpm`, `release`, `pages`)

**Depends on:** w3-4 (same `release.yml` and `PKGBUILD`). **Gate:** `w2`; `land_gate` in the JSON: the review below. Review: this changes who and what can publish; one Opus review with the forge lens (decision 11) and the security check on the pull request before landing.

- [ ] **Step 1: The scripts**

```diff
--- a/scripts/changelog-section.sh
+++ b/scripts/changelog-section.sh
@@ -1,17 +1,21 @@
 #!/bin/sh
-# Print the CHANGELOG.md section for a release tag (vX.Y.Z or X.Y.Z).
-# Fails unless Cargo.toml and packaging/arch/PKGBUILD have that version and CHANGELOG.md has
-# a "## X.Y.Z" (or "## [X.Y.Z]") heading with content below it. Used by the release workflow.
+# Print the CHANGELOG.md section for a release tag (vX.Y.Z or X.Y.Z, or vX.Y.Z-rc.N for a
+# release candidate). Fails unless Cargo.toml has that version, packaging/arch/PKGBUILD has it
+# as Arch writes it (without the "-": 1.0.0rc.1) and CHANGELOG.md has a "## X.Y.Z" (or
+# "## [X.Y.Z]") heading with content below it. Used by the release workflow and ci.yml.
 set -eu
 cd "$(dirname "$0")/.."

 ver=${1:?usage: changelog-section.sh vX.Y.Z}
 ver=${ver#v}
+bad() {
+  echo "not a X.Y.Z or X.Y.Z-rc.N version: $ver" >&2
+  exit 1
+}
 case $ver in
-*[!0-9.]* | *..* | .* | *. | *.*.*.*) echo "not a X.Y.Z version: $ver" >&2; exit 1 ;;
-*.*.*) ;;
-*) echo "not a X.Y.Z version: $ver" >&2; exit 1 ;;
+*[!0-9a-z.-]*) bad ;;
 esac
+printf '%s\n' "$ver" | grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+(-rc\.[1-9][0-9]*)?' || bad

 crate=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
 if [ "$crate" != "$ver" ]; then
@@ -20,8 +24,9 @@
 fi

 pkgver=$(sed -n 's/^pkgver=//p' packaging/arch/PKGBUILD)
-if [ "$pkgver" != "$ver" ]; then
-  echo "packaging/arch/PKGBUILD has pkgver $pkgver, tag is $ver" >&2
+archver=$(printf '%s' "$ver" | tr -d -)
+if [ "$pkgver" != "$archver" ]; then
+  echo "packaging/arch/PKGBUILD has pkgver $pkgver, tag $ver needs $archver" >&2
   exit 1
 fi

```

```diff
--- a/scripts/build-arch.sh
+++ b/scripts/build-arch.sh
@@ -2,12 +2,13 @@
 # SPDX-License-Identifier: GPL-3.0-or-later
 #
 # Build the Arch package of HEAD in an Arch container (used by the release workflow).
-# Usage: build-arch.sh IMAGE X.Y.Z OUTDIR
+# Usage: build-arch.sh IMAGE X.Y.Z OUTDIR    (or X.Y.Z-rc.N)
 #   IMAGE  archlinux / Arch Linux ARM image (its platform decides the package arch)
-#   OUTDIR receives obsbot4linux-X.Y.Z-1-<arch>.pkg.tar.zst
+#   OUTDIR receives obsbot4linux-<pkgver>-1-<arch>.pkg.tar.zst; pkgver is the version without
+#          its "-" (1.0.0-rc.1 is 1.0.0rc.1: makepkg allows no "-", and vercmp sorts it first)
 set -eu
 image=${1:?usage: build-arch.sh IMAGE X.Y.Z OUTDIR}
-ver=${2:?}
+ver=$(printf '%s' "${2:?}" | tr -d -)
 out=$(mkdir -p "${3:?}" && cd "$3" && pwd)
 root=$(cd "$(dirname "$0")/.." && pwd)

```

```diff
--- a/scripts/package-contents.sh
+++ b/scripts/package-contents.sh
@@ -2,13 +2,15 @@
 # SPDX-License-Identifier: GPL-3.0-or-later
 #
 # Lists the files of every package a release build made: one line per entry, "<package>
-# <mode> <path>", sorted, with the version replaced by VERSION and the architecture by ARCH,
+# <mode> <path>", sorted, with the version (also as Arch writes it, without its "-") replaced
+# by VERSION and the architecture by ARCH,
 # so that two builds can be compared. The release workflow compares the list with
 # packaging/contents.txt. Needs bsdtar (libarchive).
 # Usage: scripts/package-contents.sh DIR X.Y.Z    (DIR holds the obsbot4linux* packages)
 set -eu
 dir=${1:?usage: package-contents.sh DIR X.Y.Z}
 ver=${2:?usage: package-contents.sh DIR X.Y.Z}
+archver=$(printf '%s' "$ver" | tr -d -)

 find "$dir" -type f -name 'obsbot4linux*' | while read -r pkg; do
   name=$(basename "$pkg")
@@ -16,4 +18,4 @@
   *.deb) bsdtar -xOf "$pkg" 'data.tar*' | bsdtar -tvf - ;;
   *) bsdtar -tvf "$pkg" ;;
   esac | awk -v p="$name" '{ print p, $1, $NF }'
-done | sed -e "s/$ver/VERSION/g" -e 's/x86_64\|aarch64\|amd64\|arm64/ARCH/g' | LC_ALL=C sort
+done | sed -e "s/$ver/VERSION/g" -e "s/$archver/VERSION/g" -e 's/x86_64\|aarch64\|amd64\|arm64/ARCH/g' | LC_ALL=C sort
```

```diff
--- a/packaging/arch/PKGBUILD
+++ b/packaging/arch/PKGBUILD
@@ -1,7 +1,8 @@
 # Builds the committed HEAD of the checkout this file lives in (uncommitted changes are left
 # out). The release workflow sets OBSBOT4LINUX_TARBALL to a source tarball of the tag instead.
 pkgname=obsbot4linux
-# Cargo.toml's version; scripts/changelog-section.sh fails the release unless they match.
+# Cargo.toml's version, with the "-" of a pre-release dropped (1.0.0-rc.1 is 1.0.0rc.1, which
+# vercmp sorts before 1.0.0); scripts/changelog-section.sh fails the release unless they match.
 pkgver=0.13.2
 pkgrel=1
 pkgdesc='Control OBSBOT webcams on Linux: a camera daemon and its command line'
```

- [ ] **Step 2: The workflow**

```diff
--- a/.github/workflows/release.yml
+++ b/.github/workflows/release.yml
@@ -58,11 +58,12 @@
           test -n "$ver"
           echo "ver=$ver" >> "$GITHUB_OUTPUT"

-  # Fails at once unless the tagged commit is on main, was green as a pull request
-  # (scripts/release-green.sh), Cargo.toml, PKGBUILD and CHANGELOG.md match the tag and ci.yml
-  # tests with the same toolchain. On a pull request it checks the toolchain and the section of
-  # the Cargo.toml version, so a release commit is checked before its tag exists. The job itself
-  # must not be skipped: GitHub then skips every job that depends on it, directly or not.
+  # Fails at once unless the tagged commit is on main (a release candidate v1.0.0-rc.N: on
+  # main or rebuild), was green as a pull request (scripts/release-green.sh), Cargo.toml, PKGBUILD and
+  # CHANGELOG.md match the tag and ci.yml tests with the same toolchain. On a pull request it
+  # checks the toolchain and the section of the Cargo.toml version, so a release commit is checked
+  # before its tag exists. The job itself must not be skipped: GitHub then skips every job that
+  # depends on it, directly or not.
   verify:
     runs-on: ubuntu-22.04
     timeout-minutes: 10
@@ -86,8 +87,20 @@
             echo "release refused: $SHA is not a commit" >&2
             exit 1
           }
-          git merge-base --is-ancestor "$sha" origin/main || {
-            echo "release refused: $sha is not on main" >&2
+          # The release candidates of 1.0 are tagged on rebuild before main fast-forwards to it
+          # (design spec 10.3 W3, 10.6); every other release comes from main (10.7). The
+          # rebuild branch and its ruleset go after the final merge (10.1): W4's release pull
+          # request removes this exception and rebuild from ci.yml's triggers.
+          branches=origin/main
+          case $GITHUB_REF_TYPE:$GITHUB_REF_NAME in
+          tag:v1.0.0-rc.*) branches="origin/main origin/rebuild" ;;
+          esac
+          on=
+          for branch in $branches; do
+            if git merge-base --is-ancestor "$sha" "$branch" 2>/dev/null; then on=$branch; fi
+          done
+          [ -n "$on" ] || {
+            echo "release refused: $sha is not on ${branches//origin\//}" >&2
             exit 1
           }
           scripts/release-green.sh "$sha"
@@ -242,7 +255,10 @@
           ver=$VER
           mkdir dist
           chmod +x target/release/obsbot4linux target/release/obsbot4linuxd
-          cargo generate-rpm -p crates/obsbot4linux -o "dist/obsbot4linux-$ver-1.$ARCH.rpm"
+          # RPM allows no "-" in a version: 1.0.0-rc.1 is 1.0.0~rc.1, which sorts before 1.0.0
+          rpmver=$(printf '%s' "$ver" | tr - '~')
+          cargo generate-rpm -p crates/obsbot4linux -s "version = \"$rpmver\"" \
+            -o "dist/obsbot4linux-$ver-1.$ARCH.rpm"
       - uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
         with:
           name: dist-rpm-${{ matrix.arch }}
@@ -398,13 +414,20 @@
           GH_TOKEN: ${{ github.token }}
         run: |
           scripts/changelog-section.sh "$GITHUB_REF_NAME" > notes.md
-          gh release create "$GITHUB_REF_NAME" --verify-tag --title "$GITHUB_REF_NAME" \
+          # a release candidate is a pre-release: never "latest", the project page keeps the
+          # last release
+          pre=
+          case $GITHUB_REF_NAME in
+          *-rc.*) pre=--prerelease ;;
+          esac
+          gh release create "$GITHUB_REF_NAME" --verify-tag $pre --title "$GITHUB_REF_NAME" \
             --notes-file notes.md dist/*

   # The project page with the new version (pages.yml, also run by hand). Needs GitHub Pages
   # switched on (source: GitHub Actions) and the github-pages environment allowing v* tags.
   pages:
     needs: release
+    if: ${{ !contains(github.ref_name, '-rc.') }}
     permissions:
       contents: read
       pages: write
```

- [ ] **Step 3: Run the version rules**

```sh
for v in v0.13.2 v1.0.0-rc.1 v1.0.0-rc.0 v1.0.0-beta.1 v1.0 v1.0.0rc.1; do
  printf '%s: ' "$v"; scripts/changelog-section.sh "$v" > /dev/null 2> err && echo ok || head -1 err; done
```

Expected (measured):

```text
v0.13.2: ok
v1.0.0-rc.1: Cargo.toml has version 0.13.2, tag is 1.0.0-rc.1
v1.0.0-rc.0: not a X.Y.Z or X.Y.Z-rc.N version: 1.0.0-rc.0
v1.0.0-beta.1: not a X.Y.Z or X.Y.Z-rc.N version: 1.0.0-beta.1
v1.0: not a X.Y.Z or X.Y.Z-rc.N version: 1.0
v1.0.0rc.1: not a X.Y.Z or X.Y.Z-rc.N version: 1.0.0rc.1
```

With a scratch copy at `version = "1.0.0-rc.1"`, `pkgver=1.0.0rc.1` and a `## [1.0.0-rc.1] - …` heading, `v1.0.0-rc.1` prints its section; with `pkgver=1.0.0-rc.1` it fails `packaging/arch/PKGBUILD has pkgver 1.0.0-rc.1, tag 1.0.0-rc.1 needs 1.0.0rc.1`. Restore the copy.

- [ ] **Step 4: Run the guard** — extract the `verify` step's script (replace the last line by `echo "would check green: $sha (on $on)"`) and run it with `GITHUB_REF_TYPE`, `GITHUB_REF_NAME` and `SHA` set, in a clone with `origin/main` and `origin/rebuild`:

| Case                                        | Expected (measured)                                                                              |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| tag `v1.0.0-rc.1`, head of `rebuild`        | would check green (on origin/rebuild)                                                            |
| tag `v1.0.0-rc.2`, head of `rebuild`        | would check green (on origin/rebuild)                                                            |
| tag `v1.1.0-rc.1`, head of `rebuild`        | `release refused: … is not on main` (the exception names `v1.0.0-rc.*` only)                     |
| tag `v1.0.0-beta.1`, head of `rebuild`      | `release refused: … is not on main`                                                              |
| tag `v1.0.0`, head of `rebuild`             | `release refused: … is not on main`                                                              |
| tag `v1.0.0-rc.1`, head of `main`           | would check green (`main` is an ancestor of `rebuild`, so the last branch that matches is named) |
| dispatch (branch `main`), head of `rebuild` | `release refused: … is not on main`                                                              |
| tag `v0.13.3`, head of `main`               | would check green (on origin/main)                                                               |

- [ ] **Step 5: Run `build-arch.sh`'s names** — with a stand-in `docker` first in `PATH` that reads `pkgver` from the copied `PKGBUILD`, checks that the source tarball is `obsbot4linux-<pkgver>.tar.gz` with the prefix `obsbot4linux-<pkgver>/` and creates `out/obsbot4linux-<pkgver>-1-x86_64.pkg.tar.zst`: `scripts/build-arch.sh img 1.0.0-rc.1 out` ends with `out/obsbot4linux-1.0.0rc.1-1-x86_64.pkg.tar.zst` (measured). `package-contents.sh` on packages renamed to the candidate's names (`obsbot4linux-1.0.0rc.1-1-x86_64.pkg.tar.zst`, `obsbot4linux-1.0.0-rc.1-1.x86_64.rpm`, `obsbot4linux_1.0.0-rc.1_amd64.deb`, the tarball with its directory renamed) prints the same list as for 0.13.2 (measured).

- [ ] **Step 6: Gate, commit, CI** — `actionlint .github/workflows/release.yml`, `shellcheck` on the three scripts; commit `release: a vX.Y.Z-rc.N tag publishes a pre-release from rebuild`. The pull request's `release.yml` dry run must be green (it runs `build-arch.sh`, the rpm job with `-s`, and `contents` with the new script). The tag path itself runs first in w3-9.

### Task w3-8 (key `w3-8-support-tested`): The Tiny 2 Lite at `support: tested`

**Gate:** `w2+cs-1+publish` — the JSON carries all three, so the owner's go alone does not start this task; `publish` is the owner's go for the claim. **Before it:** CS-1 has run and its contract-change PRs have landed: `UNBACKED` in `crates/obsbot-device/src/models/tiny2lite/description.rs` is empty (each U-row backed, or removed from the description and named as an exception PX-U-n), and the ledger rows carry CS-1's levels.

**Files:**

- Modify: `crates/obsbot-device/src/models/tiny2lite/plan.rs` (`ENTRY.support`, its doc line, the test `the_registry_entry`)
- Modify: `README.md` (the support row), `CHANGELOG.md` (the support line)
- Modify: `crates/obsbot4linux/tests/replay.rs` (W2's w2-6: the two lines marked "Stage W3 sets the model to `tested`": `assert_eq!(d["support"], "tested", "spec 8.8: tested from W3 on");` and `assert!(!out.contains("untested"), "human output names no level for a tested model (8.6): {out}");`)
- Modify, only if CS-1's pull request w2-8 did not leave it current: `crates/obsbot-device/src/models/tiny2lite/levels.rs` (Step 3; w2-8 regenerates it, so normally nothing changes here)

- [ ] **Step 1: The registry entry**

```diff
-/// The registry entry (8.1, 8.2, 8.8). `support` is `untested` on `rebuild` until stage W3.
+/// The registry entry (8.1, 8.2, 8.8). `support: tested` from stage W3 on: the claim that stage W4
+/// verifies on every parity row.
 pub const ENTRY: ModelEntry = ModelEntry {
@@
-    support: Support::Untested,
+    support: Support::Tested,
@@ fn the_registry_entry()
-        assert_eq!(
-            ENTRY.support,
-            Support::Untested,
-            "tested only from W3 on (8.8)"
-        );
+        assert_eq!(ENTRY.support, Support::Tested, "tested from W3 on (8.8)");
```

- [ ] **Step 2: README and CHANGELOG** — `<FW>` is the firmware the CS-1 unit reported (`obsbot4linux --json devices`, field `firmware`; several versions comma-separated), taken from the CS-1 record, never assumed:

```diff
-| OBSBOT Tiny 2 Lite | `3564:fef9` | untested | –               |
+| OBSBOT Tiny 2 Lite | `3564:fef9` | tested   | <FW>            |
```

```diff
-### Untested models
-
-- OBSBOT Tiny 2 Lite (USB 3564:fef9): built from an owner's recordings, not verified by the
-  maintainers
+### Supported models
+
+- OBSBOT Tiny 2 Lite (USB 3564:fef9): tested on firmware <FW>
```

- [ ] **Step 3: Run** — `cargo test -p obsbot-device --test evidence` and the workspace: green. Before this step, regenerate `levels.rs` if CS-1's contract-change PRs changed `UNBACKED` or the ledger (w3-3 Step 3): the none rows follow `UNBACKED`, and the evidence test fails until they do. With a mark of 9.3 left, `every_registered_model_meets_the_evidence_rule` fails and lists it: then stop, report which U-row is open, do not remove it to make the test pass.

- [ ] **Step 4: Commit** — `tiny2lite: support tested (stage W3)`.

### Task w3-9 (key `w3-9-release-candidate`): Release candidate `v1.0.0-rc.1`

**Gate:** `w2+cs-1+publish` — the owner's go for the tag, after CS-1 (the JSON carries all three). **Before it:** w3-2, w3-7 and w3-8 have landed (w3-2 is a dependency: without it the candidate would ship `ExecStart=/usr/bin/obsbot4linux daemon`, a command 1.0 removed, and the socket unit would start nothing); W2 complete; widget 0.29.2 released (spec 10.6 step 1, it is out since 2026-11-13).

**Files:** Modify: `Cargo.toml` (`version`), `Cargo.lock`, `packaging/arch/PKGBUILD` (`pkgver`), `CHANGELOG.md`.

**Decision (2026-11-25):** 0.14.0 is the last 0.x release; no further 0.x intermediate releases follow. `rebuild` was rebased onto `main` (`Release 0.14.0`), so the CHANGELOG keeps the `[0.14.0]` section and the candidate's notes drop what repeated it (the repository move, the `PKGBUILD` fix). Fixes before 1.0 go to `rebuild`.

- [ ] **Step 1: The version**

```diff
-version = "0.13.2"
+version = "1.0.0-rc.1"
```

in `[workspace.package]` of `Cargo.toml`; `pkgver=1.0.0rc.1` in `packaging/arch/PKGBUILD`; then `nice -n 19 cargo build -j 2 --workspace` (without `--locked`, once): `Cargo.lock` changes the four workspace crates' versions and nothing else (`git diff --stat Cargo.lock`: 4 lines changed each way, measured with a trial bump).

- [ ] **Step 2: The CHANGELOG** — rename `## [Unreleased]` to `## [1.0.0-rc.1] - <date of the tag>` and put an empty `## [Unreleased]` above it. Add the exceptions PX-U-n that CS-1 produced (one bullet each, as in 9.2: the 0.13 commands they cover and why), and check the command map against 9.1 as it stands then (CAM-14 decides `focus auto off`); regenerate the map with w3-6's script if 9.1 changed. Put one sentence under the heading, as its first paragraph: "This is a release candidate: the maintainers verify every row of the parity table on it before 1.0.0." (the README and the tested line of 8.6 are worded to hold now; this sentence says what is still open). If `main` released 0.14.0 meanwhile, drop what the 1.0 notes repeat from it.

- [ ] **Step 3: Checks before the pull request**

```sh
scripts/changelog-section.sh v1.0.0-rc.1 | head -3        # prints the section
grep -qx 'ExecStart=/usr/bin/obsbot4linuxd' packaging/systemd/obsbot4linux.service &&
  grep -qx 'NoNewPrivileges=yes' packaging/systemd/obsbot4linux.service       # w3-2 is in: the 1.0 service
nice -n 19 cargo test -j 2 --workspace --locked             # the evidence test is strict now
scripts/check-units.sh target/debug                         # and the units start the daemon
```

Expected: green; both `grep`s succeed (a candidate with the 0.13 `ExecStart` must not be cut). `every_registered_model_meets_the_evidence_rule` runs strict at version 1 (spec 8.5 for every registered model): a U-row or an unconfirmed number fails it.

- [ ] **Step 4: Land** — pull request into `rebuild`; `required` and the `release.yml` dry run green (polling every 30 s, up to 45 minutes, in the foreground); land fast-forward, signed.

- [ ] **Step 5: Tag (owner's go)** — on the landed commit (`git rev-parse origin/rebuild`):

```sh
git tag -s v1.0.0-rc.1 -m 'obsbot4linux 1.0.0-rc.1' <landed sha>
git push origin v1.0.0-rc.1
```

Wait in the foreground for the `Release` run of the tag (every 30 s; the Arch jobs alone may take 45 minutes, so up to 90 minutes), until it is `completed`.

- [ ] **Step 6: Check what was published**

```sh
gh release view v1.0.0-rc.1 --json isPrerelease,assets --jq '.isPrerelease, (.assets | length), (.assets[].name)'
gh release download v1.0.0-rc.1 -D "$TMPDIR/rc" && (cd "$TMPDIR/rc" && sha256sum -c SHA256SUMS)
scripts/package-contents.sh "$TMPDIR/rc" 1.0.0-rc.1 | diff - packaging/contents.txt
```

The packaged service of every format starts the 1.0 daemon (a stale unit would not show in the listing, which names files only):

```sh
for p in "$TMPDIR"/rc/obsbot4linux*; do
  case $p in *.deb) bsdtar -xOf "$p" 'data.tar*' > "$TMPDIR/payload" ;; *) cp "$p" "$TMPDIR/payload" ;; esac
  f=$(bsdtar -tf "$TMPDIR/payload" | grep 'systemd/.*obsbot4linux.service$')
  bsdtar -xOf "$TMPDIR/payload" "$f" | grep -qx 'ExecStart=/usr/bin/obsbot4linuxd' || echo "BAD $p"
done; rm -f "$TMPDIR/payload"
```

Expected: `true`; 9 assets (the eight packages and `SHA256SUMS`); every checksum `OK`; no difference to `packaging/contents.txt`; no `BAD` line (measured on the four local packages: none). The project page is unchanged (the `pages` job skipped).

- [ ] **Step 7: Report and hand over** — the tag, the release URL, the eight artifact names; hand-over to W4 (CS-2 installs this candidate) and to the widget's d-16. W4's release pull request (before `v1.0.0`) has these items, which this plan does not do because they belong after the candidate:

1. Rename the heading `## [1.0.0-rc.1] - <date>` of the CHANGELOG to `## [1.0.0] - <date>`, delete the "release candidate" paragraph, add what changed since the candidate. Then `scripts/changelog-section.sh v1.0.0` prints the migration notes of 12.2 and the rollback line (the 0.13.2 packages and widget 0.29.2, spec 10.6 step 5): a new section above the candidate's would print only the difference. Version and `pkgver` change to `1.0.0` in the same pull request.
2. Remove the guard's exception for candidates in `release.yml` (`verify`: the `case` with `tag:v1.0.0-rc.*` and its comment) and `rebuild` from the `branches:` of `ci.yml` (and any other trigger of it): spec 10.1 removes the `rebuild` ruleset after the final merge, and a recreated `rebuild` branch must not be able to publish a release candidate (spec 10.7: every later release, a candidate included, comes from `main`). Check it with the guard table of w3-7 Step 4: `v1.0.0-rc.1` on a commit that is only on a `rebuild` branch is refused.
3. The project page (P1). (`docs/center-parity.md`, P2, is deleted by w3-5.)

---

## Acceptance of W3 (spec 10.3), mapped

| Acceptance without camera                                                                                                         | Where                                                                                                                                                 |
| --------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| a workflow run produces the eight artifacts, and a listing of each shows both binaries, the units, the completions and the schema | w3-4 Step 7 (the pull request's `release.yml` dry run and its `contents` job against `packaging/contents.txt`); w3-9 Step 6 (the published candidate) |
| `makepkg` with its `check()` passes without `XDG_RUNTIME_DIR`                                                                     | w3-4 Step 5 (deep directory, `XDG_RUNTIME_DIR` unset, `check()` unsets it again); needs w3-1                                                          |
| the evidence-level test of 8.5 passes for `tiny2lite`                                                                             | w3-3 (passes on `rebuild` with the U-rows listed); strict with w3-8 and w3-9, after CS-1                                                              |
| the release candidate is published                                                                                                | w3-9                                                                                                                                                  |
| units with `ExecStart` = `obsbot4linuxd`, `NoNewPrivileges=yes`                                                                   | w3-2 (and `check-units.sh` in CI); w3-9 depends on it and checks the service again before the candidate is cut and in the published packages          |
| Tiny 2 Lite `support: tested`                                                                                                     | w3-8                                                                                                                                                  |
| CHANGELOG with the migration notes; README support table, no static-switch statements                                             | w3-6, w3-5                                                                                                                                            |

## Camera items and CS-1

- **w3-8, w3-9 wait for CS-1** (spec 10.4: G → W0 → A → CS-1 → W2 → W3 → W4): CAM-23 (every U-row), CAM-14 (`focus auto off`), CAM-2 (`asleep`), CAM-5 (`settle_ms`) reach the description and the ledger through CS-1's contract-change PRs; `UNBACKED` must be empty first.
- **Not replaced by recordings (w3-3):** the UVC ranges step of CS-1 (`live_the_uvc_ranges_the_description_holds`). The recorded answers of captures 01, 14, 16 and 18 (two firmware versions) back the description's bounds as data; CS-1 still runs the live test on the unit.
- **CAM-27** (the hardened unit on the real camera) stays W4's; `check-units.sh` covers the unit's start under `no_new_privs` (set by `setpriv`, the same process flag that the unit's line sets) without a camera.
- **CAM-28** (upgrade rehearsal from the 0.13 desktop state, following the migration notes of w3-6) stays W4's.
- **U-25**: the recorded power-line range (max 3, default 3) is a finding for CS-1, not a change here.

## Self-review

1. **Spec coverage.** 10.3 W3 content: packages (w3-4), units (w3-2), completions and schema (w3-4), the pre-release tag (w3-7, w3-9), `support: tested` (w3-8), CHANGELOG (w3-6), README (w3-5). Acceptance: mapped above. 8.5: the test and its strict switch (w3-3, w3-9). 8.6: README table, release-notes line, package descriptions (w3-4 decision 4, w3-5, w3-8). 12.1: the paths of decision 1. 12.2: w3-6 Step 2's grep list. 10.6: the candidate on `rebuild` (decision 11).
2. **Placeholder scan.** Every file is given whole or as an exact diff. The only values not known today are marked and sourced: `<FW>` (CS-1 record), the date of the tag, CS-1's PX-U-n bullets.
3. **Name consistency.** Keys and files match the task list (`w3-tasks.json`); `evidence::check` has one signature everywhere; `levels::EVIDENCE` uses `description::{UNBACKED, UNCONFIRMED}`; the gate spellings of the table above are the JSON's.
4. **Owner's lines.** No task adds logic to daemon, model or command line; the only code is a test-side table and checker in `obsbot-device`. Nothing is sent to a camera. Public text checked: no vendor-app name in README, CHANGELOG, package text or the new code comments, and no statement about where the camera knowledge comes from (P5); captures cited by key.
5. **Foreign text and forging lenses (retrospective 7).** `check-units.sh` prints daemon log text only to the builder's terminal (CI log). The release guard: decision 11.

## Proof

First built on 2026-11-24 in a scratch worktree of `rebuild` at `4499637`; **re-built for the review corrections on 2026-11-24 in a scratch worktree of `origin/rebuild` at `05e66e9`**, which holds `a-14-remove-carried` and `c-13-parity-rows` (the first draft's base held neither: its `--test parity` and its w3-3 code did not run on the base that its proof named). rustc 1.100.0 (the desktop's Arch package; CI uses 1.99.1, and `rustup` is not installed here, so the 1.88 build is the CI `msrv` job's to prove), `CARGO_TARGET_DIR` and `TMPDIR` in a build directory of its own, `nice -n 19`, `-j 4` (the brief of the correction run; the plan's builders use `-j 2`). The worktree and its build directory were removed afterwards.

- All tasks' changes together (w3-1 to w3-7; w3-8 only as the trial flip of Step 5): `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` clean; `cargo test --workspace --locked --no-fail-fast`: **547 passed, 0 failed, 14 ignored** (13 live tests and the zsh syntax test are the ignored ones); `actionlint` on both workflows and `shellcheck` on `check-units.sh` and the three release scripts clean. The count differs from the first draft's 603 because `a-14-remove-carried` removed the carried 0.13 modules and their tests.
- w3-1: before the change, the 100-byte target directory failed 12 of 13 tests (`SUN_LEN`); after it 13 passed. The changed tests 20 times in a loop: 20 of 20, nothing left in `$TMPDIR`. With a 94-byte `$TMPDIR` the sockets fall back to `/tmp`: 13, 3 and 11 tests pass, nothing left in either directory. `makepkg` in a 90-byte build directory with `XDG_RUNTIME_DIR` unset and `TMPDIR` set to a short directory: `==> Finished making: obsbot4linux 0.13.2-1`, and `check()` ran the workspace tests: 547 passed, 0 failed, 14 ignored.
- w3-2: `check-units.sh` green on the debug binaries of a plain test run, 20 of 20 in a loop; the failure cases of Step 3 fail as stated (typo, wrong section, missing line); with a `systemd-analyze` shim that fails the script skips the step with a notice (and fails with `CHECK_UNITS_ANALYZE=required`); a 104-byte `TMPDIR` works. Measured with systemd 262 only; the runner's 249 is unproven (the fallback exists for it).
- w3-3: `cargo test -p obsbot-device --test evidence`: 7 passed; the mutations of Step 6 fail as stated (the five of them, run one by one and undone); 20 of 20 in a loop; the generator reproduces the committed `levels.rs` byte for byte; `cargo test -p obsbot-device --test generated` green (the description file unchanged). The recorded answers were read from captures 01, 14, 16 and 18 with a pcap parser on 2026-11-24.
- w3-4: deb (cargo-deb 3.8.0), rpm (cargo-generate-rpm 0.21.1), tarball and Arch package built locally for x86_64 on `05e66e9`; their listing equals `packaging/contents.txt` above (148 lines). `readelf` on the shipped binaries: no `libudev`, `GLIBC_2.34` in the tarball, deb and rpm, `GLIBC_2.39` in the Arch package. The claims step of the `contents` job ran locally with the failure cases of Step 6b. `obsbot4linuxd`'s tests, 10 runs back to back: 10 of 10. Not run here: the aarch64 jobs, the Arch container build (`build-arch.sh` needs Docker and a full image update; its names were checked with a stand-in `docker`), `dpkg-shlibdeps`, the rpm scriptlet branch (`rpm` is not installed here). The pull request's `release.yml` run is their proof.
- w3-7: the version rules, the guard cases (the table of Step 4 with the added cases), `build-arch.sh`'s names with the stand-in `docker` (`obsbot4linux-1.0.0rc.1-1-x86_64.pkg.tar.zst` and, for 0.13.2, the old name) and the listing of packages renamed to the candidate's names (equal to the 148 lines), each run as stated. `cargo generate-rpm -s 'version = "1.0.0~rc.1"'` accepted, `1.0.0-rc.1` refused. The tag path of `release.yml` (`verify` on a tag, `--prerelease`, `pages` skipped) is first exercised by w3-9.
- w3-5, w3-6: text. The README's commands ran against `obsbot4linuxd --fake fake-a,unknown` on `05e66e9` (exit 0, and 3 for the unsupported device); the command map generated from the spec equals the table in the plan (105 rows); the grep lists of both tasks ran on the new text.
- Measured tool facts: `vercmp` (1.0.0rc.1 < 1.0.0; 1.0.0_rc.1 and 1.0.0.rc.1 > 1.0.0; rc.9 < rc.10); cargo-deb turns `1.0.0-rc.1` into `Version: 1.0.0~rc.1`; cargo-generate-rpm refuses `1.0.0-rc.1` and writes `Version: 1.0.0~rc.1` with `-s`; `systemd-analyze --user verify` needs `XDG_RUNTIME_DIR`, only warns on an unknown key, and also loads the other units of its directory (so the script's probe has a directory of its own).
- Not run: w3-8 and w3-9 for real (gated).

## Notes for the coordinator

1. **Files outside W3's owner list (spec 10.3):** `crates/obsbot-proto/tests/client.rs`, `tests/client_subscribe.rs`, `crates/obsbot4linux/tests/common/mod.rs`, `tests/fake_daemon/mod.rs` (track C's tests, w3-1: a defect W3's acceptance found); `crates/obsbot-device/src/evidence.rs`, `lib.rs`, `models/tiny2lite/{levels,mod,description}.rs`, `tests/evidence.rs`, `tests/live.rs` (one comment), `docs/field-ledger.md` (track A's area, w3-3: spec 8.5 puts the test in W3 and the constants in the model module); `scripts/check-units.sh`, `changelog-section.sh`, `build-arch.sh`, `package-contents.sh` (W2 owns `scripts/`; these are release-path scripts). None is a contract file. `.github/` is W3's.
2. **W2 overlap (checked against `w2-plan-draft.md`).** w3-1 to w3-7 need nothing of W2's code; spec 10.3 orders W3 after W2, so the JSON gates them with `w2`, which since the cross-check of 2026-11-24 means the W2 tasks without the gate `camera` (P6). Shared files, and what the W2 plan does to them, are listed under "What W3 needs" above: no common file in `ci.yml`, `scripts/` or `CHANGELOG.md`; `docs/field-ledger.md` is edited by w2-5 (a new last section), w2-7 and w2-8 (evidence cells) and by w3-3's script (ten cells, re-runnable); w2-4's `docs/migration-1.0.md` is the source for w3-6 Step 3; w2-2's `check_model.rs` brings `jq` into `check()`, which W2's w2-9 provides (w3-4 Step 2b checks it); w2-8 changes `UNBACKED` (regenerate `levels.rs`).
3. **Gates the JSON carries** (`+`-joined, see the table of waves): w3-1 `w2+w1:c-13-parity-rows`; w3-2, w3-4, w3-5, w3-6, w3-7 `w2`; w3-3 `w2+w1:a-14-remove-carried`; w3-8 and w3-9 `w2+cs-1+publish`; w3-7 also `land_gate` (Opus review, forge lens, security check). w3-9 depends on w3-2, w3-7 and w3-8.
4. **Review by risk (retrospective 3):** Opus review for w3-3 (new checker) and w3-7 (release path, forge lens, security check before landing); w3-1, w3-2, w3-4 a cheaper review plus CI; w3-5, w3-6 a text review against spec 8.5's public-text rule and 12.2.
5. **Build once (retrospective 2):** the code and the diffs of this plan were generated from the scratch worktree of the correction run (`05e66e9`); the builder applies them and runs the steps. The coordinator may land w3-1 to w3-7 as one stack from that code (one commit per task) instead of re-building each.
6. **After w3-9:** widget d-16 (needs `v1.0.0-rc.1`), then W4 (CS-2) on this candidate; W4's release pull request carries the three items of w3-9 Step 7 (the CHANGELOG heading, the removal of the `rebuild` exception, P1 and P2) — they are not in W4's task list yet.
7. **The plan document must be on `rebuild` before a builder starts.** The JSON's `plan_doc` is `docs/superpowers/plans/2026-11-24-obsbot4linux-w3-packaging.md` in the repository (the notes file `w3-plan-draft.md` is its source); it does not exist there yet. Land it as a docs-only pull request first (the plans of W0 and W1 are there the same way).
8. **A skipped `systemd-analyze` step in the first pull request of w3-2** is a finding for the owner, not a failure (decision 3); the same for the first run of the rpm scriptlet check in the `contents` job.

## Cross-check corrections (2026-11-24)

The cross-check of the W2 and W3 plans against each other, against the rest of W1 and against the widget's track D changed this plan as listed. None of the changes was built.

| #   | Finding                                                                                                                                                                 | What changed                                                                                                                                       |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | The task list had no task that lands this plan on `rebuild` (note 7)                                                                                                    | new task w3-0 (`w3-0-plan-doc`), docs-only, first; every other task depends on it                                                                  |
| 2   | Gate `w2` read as "W2 complete", but w2-7 and w2-8 wait for the camera, so W3 could never start before CS-1                                                             | P6 taken as recommended: `w2` means the W2 tasks without the gate `camera`; `cs-1` of w3-8 and w3-9 means CS-1 and its pull requests w2-7 and w2-8 |
| 3   | w3-4 Step 2b would add `jq` to `PKGBUILD` and `scripts/build-arch.sh` a second time: W2's w2-9 adds both before w2-2                                                    | Step 2b checks that they are there; `scripts/build-arch.sh` leaves w3-4's file list                                                                |
| 4   | W2's `check_model.rs` and `replay.rs` use the `common::socket` that w3-1 moves to `$TMPDIR`, and the daemons of their harnesses keep `<socket>.lock`                    | W2's harnesses remove the lock; w3-1 Step 2 runs `--test check_model --test replay` in its loop                                                    |
| 5   | W2's `replay.rs` asserts `untested` twice; w3-8 did not list the file. w2-8 (after CS-1) changes the ledger and `UNBACKED`, which w3-3's test holds against `levels.rs` | w3-8 lists `replay.rs` with both lines; w2-8 regenerates `levels.rs` with w3-3's generator, w3-8 only checks it                                    |
| 6   | P2 handed `docs/center-parity.md` to W2 and, in w3-9 Step 7, to W4                                                                                                      | taken as recommended (before the candidate): w3-5 deletes it with the README that links it                                                         |
| 7   | P5 open; the review table quoted the README's old origin statement                                                                                                      | P5 taken as recommended; the quote is gone from the plan text                                                                                      |
