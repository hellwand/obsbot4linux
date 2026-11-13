# obsbot4linux Groundwork (Stage G) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the 0.13 single crate into the cargo workspace of the 1.0 design, with the carried
modules at their final paths in `crates/obsbot-device` and the 0.13 daemon and CLI unchanged on
top of them, without changing one byte sent to the camera or one line of CLI output.

**Architecture:** Pure moves on the branch `rebuild`, in eight pull requests. Task 1 adds guards
to the 0.13 layout (a byte guard over every command builder, a CLI help guard, a package
contents check, a test-name list and a move check). Task 2 makes the workspace with the files
moved flat. Tasks 3 to 5 move the carried modules into `family`, `transport` and
`models/tiny2lite` in parallel, keeping the old module paths alive through re-exports. Task 6
splits `camera.rs`, Task 7 switches every path to the final one and removes the re-exports,
Task 8 runs one mutation pass. Every task leaves a green build and passes every guard.

**Tech Stack:** Rust (edition 2024, `rust-version` 1.88, CI toolchain 1.99.1), cargo workspace
(resolver 3), clap 4, nix 0.31, serde; GitHub Actions (`ci.yml`, `release.yml`); cargo-deb
3.8.0, cargo-generate-rpm 0.21.1, makepkg in an Arch container; cargo-mutants 27.1.0; bash,
bsdtar, jq, python3 (only for the two one-off split scripts).

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` (revision 2), sections
3 (architecture), 3.2 (carry-over), 3.3 (licence), 10.1 (branches and rules), 10.2 (stage G),
11 (testing), 12.1 (packages).

## Global Constraints

- "No behaviour change." (spec 10.2). Not one byte sent to the camera, no ioctl, no timing and
  no CLI output may differ. The only edits a move may make are listed per task: `use` lines,
  `mod` declarations, module paths in code and comments, `pub(crate)` on items another file now
  uses, file headers and module doc comments of new files.
- Crates: `crates/obsbot-device` (package `obsbot-device`, library `obsbot_device`) and
  `crates/obsbot4linux` (package `obsbot4linux`, binary `obsbot4linux`). `obsbot-proto` and
  `obsbot4linuxd` are created in stage W0, not here.
- "All crates take `version`, `rust-version` and `license = "GPL-3.0-or-later"` from
  `[workspace.package]`." (spec 3)
- "Files that carry an EUPL-1.2 Article 5 note keep it, and the note follows their code: every
  1.0 file that receives code from such a file carries the note too" (spec 3.3).
- Packages as today: "Arch, deb, rpm and tarball, each for x86_64 and aarch64: eight
  artifacts." "There is no libudev dependency." "Packages do not enable the socket unit."
  (spec 12.1). The package name stays `obsbot4linux` in every format.
- The 0.13 daemon and CLI keep working on top of the workspace (spec 10.2).
- `main` is not touched by this plan. Every task is one pull request into `rebuild` with the
  `required` check green and a review by someone other than its author (spec 10.1). When `main`
  moves, `rebuild` is rebased onto it between pull requests (spec 10.1).
- No camera I/O. Nothing in this plan runs the binary except with `--help` or `--version`. The
  camera smoke test (CS-0) comes after the tasks and gates the release, not a task.
- Commits are signed; messages are written through a quoted heredoc and end with the
  `Co-Authored-By:` line of the model that wrote them. No session trailer, no session link.
- Public text (code comments, commit messages, pull requests): English, plain, factual; no
  private paths or names, no serial numbers, no vendor-app or firmware internals (spec 8.5).
- Builds run with low priority and their own target directory:
  `export CARGO_TARGET_DIR="$SCRATCH/target-<task key>"`, `nice -n 19`, `-j 3`. `SCRATCH` is a
  directory outside the repository (for example `SCRATCH=$(mktemp -d)`). Every wait has a
  timeout.

## Review Focus

1. **Timing constants and the one sequence counter across the `camera.rs` split.** A reply gap
   of 50 ms, 20 reply attempts, a pair gap of 100 ms, a lock wait of 5 s and exactly one
   process-wide `static SEQ` must survive the split; no unit test sees a changed gap or a second
   counter. Pinned by the move check (G6) and by Task 6, Step 7 (grep counts).
2. **ioctl definitions and `#[repr(C)]` layouts moved between `transport/uvc.rs` and
   `enumeration.rs`.** A copy instead of a move, or a changed number, compiles and passes the
   tests. Pinned by G6 and Task 4, Step 6 (each of the five ioctls and four layouts exists
   exactly once, with its 0.13 numbers).
3. **The release profile.** `[profile.release] strip = true` must move to the workspace root;
   in a member manifest cargo ignores it and the packages ship an unstripped binary with the
   same file list. Pinned by Task 2, Step 12 (`file` says `stripped`).
4. **Dependency versions and features.** Two crates unify features differently than one; the
   lock file must resolve the same crates at the same versions. Pinned by Task 2, Step 11
   (`cargo metadata` before and after).
5. **Package names and metadata.** The deb and rpm name, description, licence and asset paths
   come from the binary crate's manifest after the move. Pinned by Task 2, Step 11 (package
   metadata before and after) and by the `contents` job of the release dry run (G5).

---

## Before Task 1

These are not tasks of this plan; the coordinator checks them before the first pull request.

- **P1. Ruleset for `rebuild`** (spec 10.1): signed commits, the `required` check, no deletion.
  It does not exist yet (checked 2026-11-13). Without non-fast-forward protection, because
  `rebuild` is rebased when `main` moves:

  ```bash
  gh api -X POST repos/hellwand/obsbot4linux/rulesets --input - <<'EOF'
  {"name": "rebuild", "target": "branch", "enforcement": "active",
   "conditions": {"ref_name": {"include": ["refs/heads/rebuild"], "exclude": []}},
   "bypass_actors": [{"actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always"}],
   "rules": [{"type": "deletion"}, {"type": "required_signatures"},
     {"type": "required_status_checks", "parameters": {"do_not_enforce_on_create": false,
      "strict_required_status_checks_policy": true,
      "required_status_checks": [{"context": "required", "integration_id": 15368}]}}]}
  EOF
  ```

  Expected: a JSON answer with `"name": "rebuild"`; afterwards
  `gh api repos/hellwand/obsbot4linux/rulesets --jq '.[].name'` lists `main` and `rebuild`.

- **P2. CI on `rebuild`.** `ci.yml` and `release.yml` already run for pull requests into
  `rebuild` (checked on `origin/main` 66287b5). Nothing to do.
- **P3. Camera-free fixes in carried code land on `main` first** (spec 10.1, 10.2 "Depends
  on"). Tasks 1 to 7 can start before they land; each later rebase of `rebuild` onto `main` then
  carries them (git follows the renames; a `main` change to `camera.rs` after Task 6 has to be
  ported by hand into the split file, noted in the tracking issue). Task 8's named mutants need
  three of them (Task 8, Step 5): a test seam for the answer poll (rejected, empty, no answer,
  read error), a test for "a rejected tracking query gives `tracking: null`, not a failed read",
  and the tilt negation of `gimbal` as a tested pure function. They are not filed yet.

## Decisions

Engineering decisions for this stage, with the short reason.

1. **Package names.** The workspace has two packages: `obsbot-device` (library `obsbot_device`,
   `publish = false`, no packaging metadata) and `obsbot4linux` (the binary, with
   `[package.metadata.deb]` and `[package.metadata.generate-rpm]`). cargo-deb and
   cargo-generate-rpm name the package after the cargo package, so it stays `obsbot4linux`; the
   Arch name comes from `pkgname=obsbot4linux` in the PKGBUILD and the tarball name from
   `release.yml`. The release jobs select the package with `cargo deb -p obsbot4linux` and
   `cargo generate-rpm -p crates/obsbot4linux` (cargo-generate-rpm 0.21.1 takes the crate's
   directory; checked). Asset paths in the binary crate's metadata are relative to its
   directory (`../../README.md`); `target/release/obsbot4linux` keeps meaning the workspace
   target directory (checked with both tools: same file list as 0.13.2, same deb control file).
2. **Version.** `[workspace.package] version = "0.13.2"`; every crate has
   `version.workspace = true`. The version stays 0.13.2 through the whole stage: nothing is
   released from `rebuild`. `release.yml` and `scripts/changelog-section.sh` read the first
   `^version = ` line of the root `Cargo.toml`, which is the `[workspace.package]` line, so they
   keep working unchanged; the `msrv` jobs read `^rust-version = ` the same way. The bump to
   0.14.0 (workspace version, `Cargo.lock`, PKGBUILD `pkgver`, CHANGELOG heading) is the
   release step after the camera smoke test, not a task here.
3. **The 0.13 binary keeps its name and behaviour.** It is `[[bin]] name = "obsbot4linux"` in
   the package `obsbot4linux`; its modules `cli`, `client`, `daemon`, `proto` and `main` move
   unchanged except their `use` lines. The clap name is the literal `name = "obsbot4linux"`, the
   version comes from `CARGO_PKG_VERSION` of the same package (now inherited). The 0.13 library
   target `obsbot4linux` disappears: its modules are `obsbot_device` now. Nothing outside the
   repository links that library (checked).
4. **0.13 code that 1.0 does not carry goes to `obsbot_device::legacy`.** `state.rs` (spec 3.2:
   dropped), the 0.13 status shape (`FullStatus`, `CAPABILITIES`, `LIMITS`, `StateView`,
   `ZoneInfo`, `Zoom`, `zone_info`, `full_status`, `full_status_with`) and `current_pose`
   (spec 3.2: not carried). The carried flows still call `State` (remembered gimbal pose and
   exposure mode), so `legacy` has to live in the device crate during this stage. Stage W0 or
   track A deletes `legacy` together with the 0.13 daemon and CLI; nothing in `legacy` is a
   final path.
5. **File names.** `models.rs` becomes `registry.rs` and the detection half of `device.rs`
   becomes `enumeration.rs`, the names of spec 3.1. The device half of `device.rs` becomes
   `transport/uvc.rs` ("one implementation talks to USB UVC devices", spec 3.1). The exchange
   part of `camera.rs` (short and framed writes, answer polling, the sequence counter) becomes
   `family/exchange.rs`: it depends only on `transport` and `family::frame`, which is the
   direction of 1.0. `frame.rs` and `streaming.rs` keep their file names under `family/`.
6. **Old module paths during the parallel wave.** Tasks 3 to 5 keep every old path valid with a
   re-export in `lib.rs` (`pub use family::{frame, streaming};`, an inline `pub mod device`,
   `pub use models::tiny2lite::{commands, presets, status};`), so none of them edits a file of
   another task. Task 2 lays `lib.rs` out in blank-line separated groups, one per task, so the
   three pull requests rebase onto each other without a conflict (checked). Task 7 removes every
   re-export.
7. **Tests move with their code.** Every `#[cfg(test)] mod` moves as a whole with the module
   it tests, including the tests milestone 0.13.3 added (`commands.rs`: every short-command
   builder and all AI modes; `status.rs`: every field of the status block; `camera.rs`: the
   tilt/pitch conversions, hand and zone readbacks). The `camera.rs` test modules go with the
   functions they test: `framing_tests`, `live_pose_tests`, `angle_conversion_tests` and
   `picture_and_sound_tests` to `models/tiny2lite/camera.rs`, `full_status_tests` to
   `legacy/full_status.rs`. The test in `device.rs` that tests `first_by_id` goes to
   `enumeration.rs`. The last two path segments of every test name (module and function) do
   not change, which is what the test-name guard compares.
8. **Mutation pass** (spec 10.2 and 11): its own last task, Task 8. Tool
   cargo-mutants 27.1.0, in place, over `obsbot-device` without `legacy/state.rs` and
   `legacy/current_pose.rs` (not carried), with the whole workspace's tests. Survivors are
   killed by new tests in that pull request (test modules only) or documented in its
   description; a survivor in a function without I/O must be killed or shown equivalent. Not a
   CI gate.
9. **CHANGELOG.** One line, in Task 2. The other tasks change nothing a user or packager sees.
10. **What the guards cannot see.** Panic messages carry source paths (`src/frame.rs:…` becomes
    `crates/obsbot-device/src/family/frame.rs:…`); they are not designed output and are
    accepted. Everything else a user sees is covered by G2, G3 and G5.

## Target layout

After Task 7. `crates/obsbot-device/src/`:

| File                                                         | From (0.13.2)      | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------------------------------------------------------ | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lib.rs`                                                     | `src/lib.rs`       | `pub mod enumeration, error, family, legacy, models, registry, transport`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `error.rs`                                                   | `src/error.rs`     | unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `registry.rs`                                                | `src/models.rs`    | `Model`, `MODELS`, `from_by_id`, `from_card`, tests                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `enumeration.rs`                                             | `src/device.rs`    | `V4L2_CAP_META_CAPTURE`, `V4l2Capability`, `vidioc_querycap`, `Detected`, `card_name`, `detect`, `first_by_id`, test `by_id_detection_does_not_depend_on_directory_order`                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `transport/mod.rs`                                           | new                | `pub mod uvc;`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `transport/uvc.rs`                                           | `src/device.rs`    | `LOCK_WAIT`, UVC query constants, `XU_SIZE`, `V4L2_CTRL_FLAG_DISABLED`, `UvcXuControlQuery`, `V4l2Control`, `V4l2QueryCtrl`, four ioctls, `CtrlRange`, `Device`, `lock_exclusive`, `check_len`, `impl Device`, two tests                                                                                                                                                                                                                                                                                                                                                                           |
| `family/mod.rs`                                              | new                | `pub mod exchange, frame, streaming;`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `family/frame.rs`                                            | `src/frame.rs`     | unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `family/streaming.rs`                                        | `src/streaming.rs` | unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `family/exchange.rs`                                         | `src/camera.rs`    | `REPLY_GAP`, `REPLY_ATTEMPTS`, `PAIR_GAP`, `sequence()`, `struct Camera`, `open`, `short`, `framed`, `pair`, `query`, `query_to`, `answer`                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `models/mod.rs`                                              | new                | `pub mod tiny2lite;`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `models/tiny2lite/mod.rs`                                    | new                | `pub mod camera, commands, presets, status;`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `models/tiny2lite/commands.rs`                               | `src/commands.rs`  | unchanged but `use` lines                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `models/tiny2lite/presets.rs`                                | `src/presets.rs`   | unchanged but `use` lines                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `models/tiny2lite/status.rs`                                 | `src/status.rs`    | unchanged but `use` lines                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `models/tiny2lite/camera.rs`                                 | `src/camera.rs`    | `IMAGE_CONTROLS`, `FOCUS_ABSOLUTE`, `EXPOSURE_AUTO`, `FOCUS_AUTO`, `ZOOM_ABSOLUTE`, `ImageControl`, `LivePose`, `Zone`, `Hand`, `HandArea`, `hand_enabled`, `tilt_to_pitch_range`, `pitch_to_tilt_range`, `Framing`, `Offset`, `Identity`, `decode_volume`, `manual_exposure`, `auto_zoom_speed_reading`; `impl Camera`: `status` and every read and write flow from `tracking` to `framing` and from `sleep` to `presets`, `save_preset`, `rename_preset`, `delete_preset`, `recall_preset`; test modules `framing_tests`, `live_pose_tests`, `angle_conversion_tests`, `picture_and_sound_tests` |
| `legacy/mod.rs`                                              | new                | `pub mod current_pose, full_status, state;`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `legacy/state.rs`                                            | `src/state.rs`     | unchanged but `use` lines                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `legacy/full_status.rs`                                      | `src/camera.rs`    | `CAPABILITIES`, `Zoom`, `FullStatus`, `Limits`, `LIMITS`, `StateView`, `ZoneInfo`; `impl Camera`: `zone_info`, `full_status`, `full_status_with`; test module `full_status_tests`                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `legacy/current_pose.rs`                                     | `src/camera.rs`    | `PAN_ABSOLUTE`, `TILT_ABSOLUTE`; `impl Camera`: `current_pose`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `../tests/byte_guard.rs`, `../tests/fixtures/byte-guard.txt` | Task 1             | byte guard                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

`crates/obsbot4linux/`: `src/{main,cli,client,daemon,proto}.rs` from `src/`, unchanged but
`use` lines and two module paths in code; `tests/cli_surface.rs` and
`tests/fixtures/cli-surface.txt` from Task 1. Root: `Cargo.toml` (virtual manifest),
`Cargo.lock`, everything else where it is.

Visibility widened to `pub(crate)` (the only non-path edits of the stage): the fields `dev` and
`seq` of `Camera`, the methods `short`, `framed`, `pair`, `query`, `query_to`, `answer`, the
constants `PAIR_GAP` and `ZOOM_ABSOLUTE`, the method `current_pose`.

## Guards

Defined in Task 1, named in every task. "Base" is the commit the pull request starts from,
"head" its last commit.

- **G1 Tests.** `nice -n 19 cargo test --workspace --locked -j 3` passes, and
  `scripts/groundwork/test-names.sh` gives the same list for base and head:
  `LC_ALL=C comm -3 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"` prints nothing
  (Tasks 1 and 8 add names and say which).
- **G2 Bytes.** `cargo test --workspace --locked --test byte_guard` passes and the fixture
  `byte-guard.txt` is not edited (a rename is fine): `git diff -M --stat <base> HEAD` shows it
  with `0` changed lines or not at all.
- **G3 CLI surface.** `cargo test --workspace --locked --test cli_surface` passes and
  `cli-surface.txt` is not edited.
- **G4 Lint and MSRV.** `cargo fmt --check`;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`; the CI job `msrv`
  (Rust 1.88) green. Locally the MSRV build needs rustup and is optional; CI is the check.
- **G5 Packaging.** The release dry run of `release.yml` on the pull request is green,
  including the job `contents` (the file list of all eight packages equals
  `packaging/contents.txt`). It runs when the pull request touches
  `.github/workflows/release.yml`, `packaging/**`, `scripts/**`, a `Cargo.toml`, `Cargo.lock`, `README.md`,
  `CHANGELOG.md`, `LICENSE` or `renovate.json`: Tasks 1 and 2. The tag run of 0.14.0 runs it
  again on the final tree.
- **G6 Move check.** `scripts/groundwork/move-check.sh <base> HEAD` prints only the lines the
  task lists under "Expected move check".
- **G7 Review.** `git diff -M --stat <base> HEAD` shows the moves as renames; the reviewer reads
  every changed line outside a pure rename.

The gate a builder runs before every push:

```bash
export CARGO_TARGET_DIR="$SCRATCH/target-<task key>"
nice -n 19 cargo fmt --check
nice -n 19 cargo clippy --workspace --all-targets --locked -j 3 -- -D warnings
nice -n 19 cargo test --workspace --locked -j 3
shellcheck scripts/*.sh scripts/groundwork/*.sh
actionlint .github/workflows/*.yml
```

## Order and parallel work

| Task                        | Key                            | Wave | Starts from                       | Depends on |
| --------------------------- | ------------------------------ | ---- | --------------------------------- | ---------- |
| 1 Guards                    | `groundwork-guards`            | 1    | `origin/rebuild`                  | –          |
| 2 Workspace                 | `groundwork-workspace`         | 2    | branch of Task 1                  | 1          |
| 3 family                    | `groundwork-family`            | 3    | branch of Task 2                  | 2          |
| 4 transport and enumeration | `groundwork-transport`         | 3    | branch of Task 2                  | 2          |
| 5 tiny2lite modules         | `groundwork-tiny2lite-modules` | 3    | branch of Task 2                  | 2          |
| 6 Split `camera.rs`         | `groundwork-camera-split`      | 4    | `origin/rebuild` after 1–5 landed | 3, 4, 5    |
| 7 Final paths               | `groundwork-final-paths`       | 5    | branch of Task 6                  | 6          |
| 8 Mutation pass             | `groundwork-mutants`           | 6    | branch of Task 7                  | 7          |

Tasks 3, 4 and 5 run side by side: they own disjoint files and one blank-line separated group
of `lib.rs` each. Everything else is stacked, because each task moves files the next one
needs. The camera split cannot run beside the wave-3 tasks: it needs the directories they
create. Landing is a fast-forward of `rebuild` in key order (1, 2, 3, 4, 5, then 6, 7, 8);
Tasks 3 to 5 rebase onto each other without conflict in any order. Branch names:
`groundwork/<key without the "groundwork-" prefix>`.

---

### Task 1: Guards in the 0.13 layout

**Files:**

- Create: `tests/byte_guard.rs`, `tests/fixtures/byte-guard.txt` (generated),
  `tests/cli_surface.rs`, `tests/fixtures/cli-surface.txt` (generated),
  `scripts/groundwork/move-check.sh`, `scripts/groundwork/test-names.sh`,
  `scripts/package-contents.sh`, `packaging/contents.txt` (generated from the dry run)
- Modify: `.github/workflows/release.yml` (new job `contents`; `release` needs it)

**Interfaces:**

- Consumes: the 0.13 library paths `obsbot4linux::commands`, `obsbot4linux::frame`,
  `obsbot4linux::presets`; the binary `obsbot4linux`.
- Produces: integration tests `byte_guard::builders_emit_the_recorded_bytes`,
  `cli_surface::help_texts_are_unchanged`, `cli_surface::version_is_the_package_version`;
  environment switches `BYTE_GUARD_BLESS` and `CLI_SURFACE_BLESS` (only this task sets them);
  `scripts/groundwork/test-names.sh REV`, `scripts/groundwork/move-check.sh OLD_REV [NEW_REV]`,
  `scripts/package-contents.sh DIR X.Y.Z`; the `release.yml` job `contents` comparing with
  `packaging/contents.txt`.

- [ ] **Step 1: Branch**

```bash
git fetch origin
git switch -c groundwork/guards origin/rebuild
```

- [ ] **Step 2: Write the byte guard** (`tests/byte_guard.rs`; `cargo fmt` formats it in Step 4)

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Byte guard of the groundwork stage (rebuild plan, stage G): every frame and short command the
//! builders emit for a fixed list of calls, one line per call, compared with
//! `tests/fixtures/byte-guard.txt`. That file was written from the 0.13 code before the
//! workspace move, so a move that changes one byte that would go to the camera fails here.
//!
//! Only the task that adds this test writes the file:
//! `BYTE_GUARD_BLESS=1 cargo test --test byte_guard`. Every later run compares.

use clap::ValueEnum;
use obsbot4linux::commands::{self as cmd, *};
use obsbot4linux::frame::{self, FRAME_SIZE};
use obsbot4linux::presets::{self, Pose, StartupAi};
use std::fmt::Display;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/byte-guard.txt");
const SEQ: u16 = 0x1234;
const SEQS: [u16; 4] = [SEQ, SEQ + 1, SEQ + 2, SEQ + 3];

/// How one builder result is written into the fixture.
trait Show {
    fn show(&self) -> String;
}

impl<const N: usize> Show for [u8; N] {
    fn show(&self) -> String {
        hex::encode(self)
    }
}

impl<const N: usize, const M: usize> Show for [[u8; N]; M] {
    fn show(&self) -> String {
        self.iter().map(hex::encode).collect::<Vec<_>>().join(" ")
    }
}

impl Show for u8 {
    fn show(&self) -> String {
        self.to_string()
    }
}

impl Show for u16 {
    fn show(&self) -> String {
        format!("{self:#06x}")
    }
}

impl Show for bool {
    fn show(&self) -> String {
        self.to_string()
    }
}

impl<T: Show> Show for Option<T> {
    fn show(&self) -> String {
        self.as_ref().map_or("none".into(), Show::show)
    }
}

impl<T: Show, E: Display> Show for Result<T, E> {
    fn show(&self) -> String {
        match self {
            Ok(v) => v.show(),
            Err(e) => format!("error: {e}"),
        }
    }
}

fn rec(out: &mut Vec<String>, label: String, value: impl Show) {
    out.push(format!("{label} = {}", value.show()));
}

/// Poses at and beyond every bound the pose builders check, and an asymmetric one.
const POSES: [(f32, f32, f32); 10] = [
    (10.5, -5.25, 2.0),
    (0.0, 0.0, 1.0),
    (-150.0, -70.0, 1.0),
    (150.0, 90.0, 4.0),
    (150.1, 0.0, 1.0),
    (0.0, 90.5, 1.0),
    (0.0, -70.5, 1.0),
    (0.0, 0.0, 4.01),
    (0.0, 0.0, 0.99),
    (f32::NAN, 0.0, 1.0),
];

fn poses() -> impl Iterator<Item = Pose> {
    POSES.into_iter().map(|(pan, tilt, zoom)| Pose { pan, tilt, zoom })
}

/// Area ranges (min, max) in the camera's angles, at and beyond the bounds, and wrong ways round.
const RANGES: [(f32, f32); 11] = [
    (-45.0, 45.0),
    (-150.0, 150.0),
    (-150.5, 0.0),
    (0.0, 150.5),
    (-90.0, 70.0),
    (-90.5, 0.0),
    (0.0, 70.5),
    (-30.0, 10.0),
    (10.0, 10.0),
    (10.0, -10.0),
    (f32::NAN, 1.0),
];

type Short3 = fn(bool) -> [u8; 3];
type Framed = fn(bool, u16) -> [u8; FRAME_SIZE];
type SeqOnly = fn(u16) -> [u8; FRAME_SIZE];

fn record() -> Vec<String> {
    let mut out = Vec::new();
    let o = &mut out;

    // frame codec
    for seq in [0x0000, 0x0001, SEQ, 0xffff] {
        let q = frame::query_frame(0x04, 0x0104, seq);
        rec(o, format!("frame::query_frame(0x04, 0x0104, {seq:#06x})"), q);
        let f = frame::frame(0x02, 0xa0c2, &[1, 0, 0, 0], seq);
        rec(o, format!("frame::frame(0x02, 0xa0c2, [1 0 0 0], {seq:#06x})"), f);
        let w = frame::query_frame_with(0x04, 0x1504, &[7, 0, 0, 0], seq);
        rec(o, format!("frame::query_frame_with(0x04, 0x1504, [7 0 0 0], {seq:#06x})"), w);
    }
    let full = frame::frame(0x04, 0x3e44, &[0xa5; frame::MAX_DATA], SEQ);
    rec(o, "frame::frame(0x04, 0x3e44, [0xa5; MAX_DATA])".into(), full);
    rec(o, "frame::crc16_usb(\"123456789\")".into(), frame::crc16_usb(b"123456789"));

    // short commands (selector 6)
    let short3: [(&str, Short3); 6] = [
        ("hdr", cmd::hdr),
        ("led_blink", cmd::led_blink),
        ("mirror", cmd::mirror),
        ("mic_in_sleep", cmd::mic_in_sleep),
        ("auto_gain", cmd::auto_gain),
        ("microphone", cmd::microphone),
    ];
    for (name, f) in short3 {
        for on in [false, true] {
            rec(o, format!("cmd::{name}({on})"), f(on));
        }
    }
    for on in [false, true] {
        rec(o, format!("cmd::sleep_background_mirror({on})"), cmd::sleep_background_mirror(on));
    }
    for m in AiMode::value_variants() {
        rec(o, format!("cmd::ai_mode({m:?})"), cmd::ai_mode(*m));
        rec(o, format!("cmd::default_work_mode({m:?})"), cmd::default_work_mode(*m));
    }
    for s in [i16::MIN, -600, -1, 0, 1, 600, i16::MAX] {
        rec(o, format!("cmd::auto_sleep({s})"), cmd::auto_sleep(s));
    }
    for s in [0u16, 1, 600, 32767, 32768, u16::MAX] {
        rec(o, format!("cmd::auto_sleep_off({s})"), cmd::auto_sleep_off(s));
    }
    for b in SleepBackground::value_variants() {
        rec(o, format!("cmd::sleep_background({b:?})"), cmd::sleep_background(*b));
    }
    for b in 0..=4u8 {
        rec(o, format!("cmd::led({b})"), cmd::led(b));
    }
    for p in [0u8, 1, 50, 100, 101, 255] {
        rec(o, format!("cmd::wake_zoom({p})"), cmd::wake_zoom(p));
    }
    for f in [0.99f32, 1.0, 1.5, 2.5, 4.0, 4.01, f32::NAN] {
        rec(o, format!("cmd::wake_zoom_percent({f})"), cmd::wake_zoom_percent(f));
    }
    for v in FieldOfView::value_variants() {
        rec(o, format!("cmd::field_of_view({v:?})"), cmd::field_of_view(*v));
    }
    for v in NoiseReduction::value_variants() {
        rec(o, format!("cmd::noise_reduction({v:?})"), cmd::noise_reduction(*v));
    }
    for v in PickupDistance::value_variants() {
        rec(o, format!("cmd::pickup_distance({v:?})"), cmd::pickup_distance(*v));
    }
    for v in ExposureMode::value_variants() {
        rec(o, format!("cmd::exposure_mode_select({v:?})"), cmd::exposure_mode_select(*v));
    }
    rec(o, "cmd::startup_go()".into(), cmd::startup_go());

    // framed commands (selector 2)
    let framed: [(&str, Framed); 9] = [
        ("invert", cmd::invert),
        ("zone_tracking", cmd::zone_tracking),
        ("hand_pan", cmd::hand_pan),
        ("hand_tilt", cmd::hand_tilt),
        ("hand_gimbal", cmd::hand_gimbal),
        ("autofocus_mode", cmd::autofocus_mode),
        ("zone_follow", cmd::zone_follow),
        ("sleep", cmd::sleep),
        ("auto_zoom", cmd::auto_zoom),
    ];
    for (name, f) in framed {
        for on in [false, true] {
            rec(o, format!("cmd::{name}({on})"), f(on, SEQ));
        }
    }
    let seq_only: [(&str, SeqOnly); 6] = [
        ("zone_return_go", cmd::zone_return_go),
        ("hand_return_go", cmd::hand_return_go),
        ("hand_return_reset", cmd::hand_return_reset),
        ("factory_reset", cmd::factory_reset),
        ("gimbal_stop", cmd::gimbal_stop),
        ("zoom_stop", cmd::zoom_stop),
    ];
    for (name, f) in seq_only {
        rec(o, format!("cmd::{name}()"), f(SEQ));
    }
    for v in TrackingSpeed::value_variants() {
        rec(o, format!("cmd::tracking_speed({v:?})"), cmd::tracking_speed(*v, SEQ));
    }
    for v in SpeedMode::value_variants() {
        rec(o, format!("cmd::speed_mode({v:?})"), cmd::speed_mode(*v, SEQ));
    }
    for v in FramingSize::value_variants() {
        rec(o, format!("cmd::framing_size({v:?})"), cmd::framing_size(*v, SEQ));
    }
    for v in AutoZoomSpeed::value_variants() {
        rec(o, format!("cmd::auto_zoom_speed({v:?})"), cmd::auto_zoom_speed(*v, SEQ));
    }
    for v in PartMode::value_variants() {
        rec(o, format!("cmd::part_mode({v:?})"), cmd::part_mode(*v, SEQ));
    }
    for v in HandSide::value_variants() {
        rec(o, format!("cmd::hand_side({v:?})"), cmd::hand_side(*v, SEQ));
    }
    for v in AutofocusMode::value_variants() {
        rec(o, format!("cmd::autofocus_mode_select({v:?})"), cmd::autofocus_mode_select(*v, SEQ));
    }
    for g in Gesture::value_variants() {
        for on in [false, true] {
            rec(o, format!("cmd::gesture({g:?}, {on})"), cmd::gesture(*g, on, SEQ));
        }
    }
    for r in [0.99f32, 1.0, 2.5, 4.0, 4.01, f32::NAN] {
        rec(o, format!("cmd::gesture_zoom_ratio({r})"), cmd::gesture_zoom_ratio(r, SEQ));
    }
    for s in [-1.0f32, -0.0, 0.0, 2.5, 600.0, f32::INFINITY, f32::NAN] {
        rec(o, format!("cmd::hand_return_after({s})"), cmd::hand_return_after(s, SEQ));
    }
    for v in [0u8, 1, 50, 100, 101, 255] {
        rec(o, format!("cmd::audio_volume({v})"), cmd::audio_volume(v, SEQ));
    }
    for x in [-0.01f32, 0.0, 0.25, 1.0, 1.01] {
        for y in [-0.01f32, 0.0, 0.75, 1.0, 1.01] {
            rec(o, format!("cmd::focus_point({x}, {y})"), cmd::focus_point(x, y, SEQ));
            rec(o, format!("cmd::view_center({x}, {y})"), cmd::view_center(x, y, SEQ));
        }
    }
    for r in RANGES {
        for (yaw, pitch) in [(r, (-30.0, 10.0)), ((-45.0, 45.0), r)] {
            rec(o, format!("cmd::zone_area({yaw:?}, {pitch:?})"), cmd::zone_area(yaw, pitch, SEQS));
            rec(o, format!("cmd::hand_area({yaw:?}, {pitch:?})"), cmd::hand_area(yaw, pitch, SEQS));
        }
    }
    rec(o, "cmd::hand_area_reset()".into(), cmd::hand_area_reset(SEQS));
    let axes = [None, Some(-150.0f32), Some(-70.0), Some(0.0), Some(12.5), Some(90.0), Some(150.0), Some(150.5), Some(-90.5)];
    for yaw in axes {
        for pitch in axes {
            rec(o, format!("cmd::gimbal_pose({yaw:?}, {pitch:?})"), cmd::gimbal_pose(yaw, pitch, SEQ));
        }
    }
    for (yaw, pitch) in [(None, None), (Some(12.5f32), None), (None, Some(-5.0f32)), (Some(12.5), Some(-5.0)), (Some(150.5), Some(0.0))] {
        for speed in [0.99f32, 1.0, 45.0, 180.0, 180.5, f32::NAN] {
            let f = cmd::gimbal_pose_speed(yaw, pitch, speed, SEQ);
            rec(o, format!("cmd::gimbal_pose_speed({yaw:?}, {pitch:?}, {speed})"), f);
        }
    }
    for pan in [-178.5f32, -178.0, -0.0, 0.0, 12.5, 178.0, 178.5, f32::NAN] {
        for tilt in [-178.5f32, -178.0, -0.0, 0.0, 5.0, 178.0, 178.5] {
            let f = cmd::gimbal_rate(pan, tilt, false, false, SEQ);
            rec(o, format!("cmd::gimbal_rate({pan}, {tilt}, false, false)"), f);
        }
    }
    for (mirror, invert) in [(true, false), (false, true), (true, true)] {
        let f = cmd::gimbal_rate(12.5, 5.0, mirror, invert, SEQ);
        rec(o, format!("cmd::gimbal_rate(12.5, 5, {mirror}, {invert})"), f);
    }
    for speed in [0i32, 1, 10, 30, 50, i32::MAX] {
        for ratio in [0.99f32, 1.0, 2.5, 4.0, 4.01] {
            rec(o, format!("cmd::zoom_to({speed}, {ratio})"), cmd::zoom_to(speed, ratio, SEQ));
        }
    }
    for vertical in [false, true] {
        for f in [-0.17f32, -0.16, -0.0, 0.0, 0.1, 0.16, 0.17, f32::NAN] {
            let b = cmd::framing_offset(vertical, f, SEQ);
            rec(o, format!("cmd::framing_offset({vertical}, {f})"), b);
        }
    }
    for pan in [-150.5f32, -150.0, 0.0, 150.0, 150.5] {
        for tilt in [-70.5f32, -70.0, 0.0, 90.0, 90.5] {
            rec(o, format!("cmd::gimbal_angles_ok({pan}, {tilt})"), cmd::gimbal_angles_ok(pan, tilt));
        }
    }

    // presets, zone views, start pose and return positions
    let names = ["", "Desk", "Bereich1", "äöü", "12345678901234567890123456789012", "123456789012345678901234567890123"];
    for slot in [0u8, 1, 2, 3, 255] {
        for p in poses() {
            rec(o, format!("presets::create({slot}, {p:?})"), presets::create(slot, p, SEQ));
            rec(o, format!("presets::update({slot}, {p:?})"), presets::update(slot, p, SEQ));
        }
        for name in names {
            rec(o, format!("presets::rename({slot}, {name:?})"), presets::rename(slot, name, SEQ));
        }
        rec(o, format!("presets::delete({slot})"), presets::delete(slot, SEQ, SEQ + 1));
        rec(o, format!("presets::recall({slot})"), presets::recall(slot, SEQ));
    }
    for id in [0u32, 1, 7, 255, u32::MAX] {
        for p in poses() {
            rec(o, format!("presets::view_add({id}, {p:?})"), presets::view_add(id, p, SEQ));
        }
        for name in names {
            rec(o, format!("presets::view_rename({id}, {name:?})"), presets::view_rename(id, name, SEQ));
        }
        rec(o, format!("presets::view_remove({id})"), presets::view_remove(id, SEQ));
        rec(o, format!("presets::view_go({id})"), presets::view_go(id, SEQ));
        for command in [0x1504u16, 0x1444] {
            let q = presets::view_query(command, id, SEQ);
            rec(o, format!("presets::view_query({command:#06x}, {id})"), q);
        }
    }
    for source in [0u32, 1, 2, 3, 4] {
        for p in poses() {
            let f = presets::startup_pose(source, p, SEQ);
            rec(o, format!("presets::startup_pose({source}, {p:?})"), f);
        }
    }
    for p in poses() {
        rec(o, format!("presets::zone_return_set({p:?})"), presets::zone_return_set(p, SEQ));
        rec(o, format!("presets::hand_return_set({p:?})"), presets::hand_return_set(p, SEQ));
    }
    for ai in StartupAi::value_variants() {
        let f = presets::startup_picture_settings(ai.byte(), SEQ);
        rec(o, format!("presets::startup_picture_settings({ai:?})"), f);
    }
    let block: Vec<u8> = (0u8..48).collect();
    for len in [0usize, 39, 40, 48] {
        for ai in [0x00u8, 0xfe, 0xff] {
            let f = presets::startup_with_ai(&block[..len], ai, SEQ);
            rec(o, format!("presets::startup_with_ai(0..{len}, {ai:#04x})"), f);
        }
    }
    out
}

#[test]
fn builders_emit_the_recorded_bytes() {
    let now = record().join("\n") + "\n";
    if std::env::var_os("BYTE_GUARD_BLESS").is_some() {
        std::fs::write(FIXTURE, &now).unwrap();
        return;
    }
    let recorded = std::fs::read_to_string(FIXTURE).expect("tests/fixtures/byte-guard.txt");
    let (now, recorded): (Vec<&str>, Vec<&str>) = (now.lines().collect(), recorded.lines().collect());
    assert_eq!(now.len(), recorded.len(), "the number of recorded calls changed");
    for (n, r) in now.iter().zip(&recorded) {
        assert_eq!(n, r, "a builder emits other bytes than before the move");
    }
}
```

- [ ] **Step 3: Run it to see it fail** (there is no recording yet)

Run: `nice -n 19 cargo test --locked -j 3 --test byte_guard`
Expected: FAIL, `panicked at tests/byte_guard.rs:…: tests/fixtures/byte-guard.txt: Os { code: 2, kind: NotFound, … }`

- [ ] **Step 4: Record the bytes of the base commit, format, and check that it passes**

```bash
mkdir -p tests/fixtures
BYTE_GUARD_BLESS=1 nice -n 19 cargo test --locked -j 3 --test byte_guard
nice -n 19 cargo fmt
nice -n 19 cargo test --locked -j 3 --test byte_guard
wc -l tests/fixtures/byte-guard.txt
head -n 1 tests/fixtures/byte-guard.txt
```

Expected: both runs `test result: ok. 1 passed`; `831 tests/fixtures/byte-guard.txt` on top
of 66287b5 (another number only if `main` changed a builder's call list, which it cannot);
first line `frame::query_frame(0x04, 0x0104, 0x0000) = aa0100000c0085940a0404010000…`.

- [ ] **Step 5: Prove the guard fails on one changed byte, then undo**

```bash
sed -i 's/\[0x01, 0x01, on as u8\]/[0x01, 0x01, !on as u8]/' src/commands.rs
nice -n 19 cargo test --locked -j 3 --test byte_guard 2>&1 | grep -E 'left:|right:|test result'
git checkout -- src/commands.rs
```

Expected: `left: "cmd::hdr(false) = 010101"`, `right: "cmd::hdr(false) = 010100"`,
`test result: FAILED`. After the checkout `git status --short src` prints nothing.

- [ ] **Step 6: Write the CLI surface guard** (`tests/cli_surface.rs`)

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! CLI surface guard of the groundwork stage (rebuild plan, stage G): the `--help` text of the
//! CLI and of every subcommand, compared with `tests/fixtures/cli-surface.txt`, which was
//! written from the 0.13 code before the workspace move, and `--version`. Only `--help` and
//! `--version` run: nothing opens a device or a socket.
//!
//! Only the task that adds this test writes the file:
//! `CLI_SURFACE_BLESS=1 cargo test --test cli_surface`. Every later run compares.

use std::process::Command;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cli-surface.txt"
);

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_obsbot4linux"))
        .args(args)
        .output()
        .expect("obsbot4linux runs");
    assert!(out.status.success(), "obsbot4linux {args:?} failed");
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// The names in the "Commands:" section of a help text: the lines indented by exactly two
/// spaces (a longer indent continues a description).
fn subcommands(help: &str) -> Vec<String> {
    help.lines()
        .skip_while(|l| *l != "Commands:")
        .skip(1)
        .take_while(|l| l.starts_with(' '))
        .filter(|l| !l.starts_with("   "))
        .filter_map(|l| l.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(String::from)
        .collect()
}

fn walk(path: &mut Vec<String>, out: &mut String) {
    let mut args: Vec<&str> = path.iter().map(String::as_str).collect();
    args.push("--help");
    let help = run(&args);
    out.push_str(&format!("=== obsbot4linux {}\n{help}", args.join(" ")));
    for sub in subcommands(&help) {
        path.push(sub);
        walk(path, out);
        path.pop();
    }
}

#[test]
fn help_texts_are_unchanged() {
    let mut out = String::new();
    walk(&mut Vec::new(), &mut out);
    // a hidden subcommand is not listed under "Commands:"
    walk(&mut vec!["gimbal".into(), "center".into()], &mut out);
    if std::env::var_os("CLI_SURFACE_BLESS").is_some() {
        std::fs::write(FIXTURE, &out).unwrap();
        return;
    }
    let recorded = std::fs::read_to_string(FIXTURE).expect("tests/fixtures/cli-surface.txt");
    assert_eq!(out, recorded, "a help text of the CLI changed");
}

#[test]
fn version_is_the_package_version() {
    // the workspace move must keep the version the binary reports
    assert_eq!(
        run(&["--version"]),
        format!("obsbot4linux {}\n", env!("CARGO_PKG_VERSION"))
    );
}
```

- [ ] **Step 7: Run it to see it fail, record, run again**

```bash
nice -n 19 cargo test --locked -j 3 --test cli_surface 2>&1 | grep 'test result'
CLI_SURFACE_BLESS=1 nice -n 19 cargo test --locked -j 3 --test cli_surface
nice -n 19 cargo test --locked -j 3 --test cli_surface 2>&1 | grep 'test result'
grep -c '^=== ' tests/fixtures/cli-surface.txt
```

Expected: first `test result: FAILED. 1 passed; 1 failed` (no recording), then
`test result: ok. 2 passed`; `88` help texts (root, 86 listed subcommands at every level,
the hidden `gimbal center`).

- [ ] **Step 8: Prove it fails on a changed help text, then undo**

```bash
sed -i 's/about = "Control the OBSBOT Tiny 2 Lite"/about = "Control the OBSBOT Tiny 2 Lite!"/' src/cli.rs
nice -n 19 cargo test --locked -j 3 --test cli_surface 2>&1 | grep -E 'help text|test result'
git checkout -- src/cli.rs
```

Expected: `a help text of the CLI changed` and `test result: FAILED. 1 passed; 1 failed`.

- [ ] **Step 9: Write the three scripts and make them executable**

`scripts/groundwork/test-names.sh`:

```bash
#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Test-name guard of the groundwork stage. Lists the tests of a commit by the last two parts of
# their path (module and name, which a move keeps), sorted. Run it for the base and the head of
# a pull request and compare the two lists: a pure move changes no line.
# Usage: scripts/groundwork/test-names.sh REV > names.txt
set -euo pipefail
rev=${1:?usage: test-names.sh REV}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git archive "$rev" | tar -x -C "$work"
cd "$work"
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$work/target} nice -n 19 \
  cargo test --workspace --locked -q -j 3 -- --list --format terse 2>/dev/null |
  awk -F': ' '$2 == "test" { n = split($1, p, "::"); print (n > 1 ? p[n - 1] "::" : "") p[n] }' |
  LC_ALL=C sort
```

`scripts/groundwork/move-check.sh`:

```bash
#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Move check of the groundwork stage. Prints the lines of Rust code that differ between two
# commits, compared as a multiset: the file a line is in and the order of lines do not count,
# and `use` statements, `mod name;` declarations, `pub(crate)` / `pub(super)`, indentation and
# blank lines are set aside. A pure move prints only the structural lines its task lists.
# Usage: scripts/move-check.sh OLD_REV [NEW_REV]    (NEW_REV defaults to HEAD)
set -euo pipefail
old=${1:?usage: move-check.sh OLD_REV [NEW_REV]}
new=${2:-HEAD}

lines() {
  git ls-tree -r --name-only "$1" | grep '\.rs$' | while read -r f; do
    git show "$1:$f"
  done | awk '
    in_use { if (/;[[:space:]]*$/) in_use = 0; next }
    /^[[:space:]]*(pub(\([a-z]+\))? )?use / { if (!/;[[:space:]]*$/) in_use = 1; next }
    /^[[:space:]]*(pub(\([a-z]+\))? )?mod [a-z0-9_]+;[[:space:]]*$/ { next }
    /^[[:space:]]*$/ { next }
    { sub(/^[[:space:]]+/, ""); sub(/^pub\((crate|super)\) /, ""); print }
  ' | LC_ALL=C sort
}

diff <(lines "$old") <(lines "$new") | grep '^[<>]' || true
```

`scripts/package-contents.sh`:

```bash
#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Lists the files of every package a release build made: one line per entry, "<package>
# <mode> <path>", sorted, with the version replaced by VERSION and the architecture by ARCH,
# so that two builds can be compared. The release workflow compares the list with
# packaging/contents.txt. Needs bsdtar (libarchive).
# Usage: scripts/package-contents.sh DIR X.Y.Z    (DIR holds the obsbot4linux* packages)
set -eu
dir=${1:?usage: package-contents.sh DIR X.Y.Z}
ver=${2:?usage: package-contents.sh DIR X.Y.Z}

find "$dir" -type f -name 'obsbot4linux*' | while read -r pkg; do
  name=$(basename "$pkg")
  case $name in
  *.deb) bsdtar -xOf "$pkg" 'data.tar*' | bsdtar -tvf - ;;
  *) bsdtar -tvf "$pkg" ;;
  esac | awk -v p="$name" '{ print p, $1, $NF }'
done | sed -e "s/$ver/VERSION/g" -e 's/x86_64\|aarch64\|amd64\|arm64/ARCH/g' | LC_ALL=C sort
```

```bash
chmod +x scripts/groundwork/test-names.sh scripts/groundwork/move-check.sh scripts/package-contents.sh
shellcheck scripts/*.sh scripts/groundwork/*.sh
```

Expected: shellcheck prints nothing.

- [ ] **Step 10: Add the job `contents` to `.github/workflows/release.yml`**

Insert the job `contents` before the job `release`, and add `contents` to the `needs` of
`release`. The end of the file's job list then reads (the job `release` keeps all its other
lines):

```yaml
  # Lists the files of every package and compares the list with packaging/contents.txt, so that
  # a change to what a package holds shows up in review. A release needs it too.
  contents:
    needs: [version, binary, deb, rpm, arch]
    runs-on: ubuntu-22.04
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - uses: actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2
        with:
          path: dist
          pattern: dist-*
          merge-multiple: true
      - run: sudo apt-get install -y --no-install-recommends libarchive-tools
      - name: Compare the package contents
        env:
          VER: ${{ needs.version.outputs.ver }}
        run: |
          scripts/package-contents.sh dist "$VER" > contents.txt
          cat contents.txt
          diff -u packaging/contents.txt contents.txt
      # the list itself, also when it differs, to update packaging/contents.txt from
      - if: always()
        uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
        with:
          name: package-contents
          path: contents.txt
          if-no-files-found: error

  # Tags only: a pull request builds the packages above and publishes nothing.
  release:
    if: startsWith(github.ref, 'refs/tags/')
    needs: [binary, deb, rpm, arch, contents]
```

Run: `actionlint .github/workflows/*.yml`
Expected: no output.

- [ ] **Step 11: Gate**

```bash
export CARGO_TARGET_DIR="$SCRATCH/target-groundwork-guards"
nice -n 19 cargo fmt --check
nice -n 19 cargo clippy --all-targets --locked -j 3 -- -D warnings
nice -n 19 cargo test --locked -j 3 2>&1 | grep 'test result'
git add tests scripts .github/workflows/release.yml
git status --short   # every file of this task staged, nothing else
```

Expected test results on top of 66287b5: `118 passed` (library), `25 passed` (binary),
`1 passed` (byte_guard), `2 passed` (cli_surface), `0 passed` (doc tests).

Commit (Step 12) before the name lists, because `test-names.sh` reads committed trees.

- [ ] **Step 12: Commit**

```bash
git commit -F - <<'EOF'
Add guards for the groundwork move

A byte guard records every frame and short command the builders emit for a fixed list of
calls, a CLI guard records the help text of every subcommand, and the release dry run lists
the files of every package. The workspace move must leave all three unchanged.

Co-Authored-By: <model of this session> <noreply@anthropic.com>
EOF
scripts/groundwork/test-names.sh origin/rebuild > "$SCRATCH/names-base.txt"
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-head.txt"
LC_ALL=C comm -3 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"
```

Expected from `comm`: exactly three lines, each indented by one tab (only in head):
`builders_emit_the_recorded_bytes`, `help_texts_are_unchanged`,
`version_is_the_package_version`.

- [ ] **Step 13: Push, open the pull request into `rebuild`, record the package list**

```bash
git push -u origin groundwork/guards
gh pr create -R hellwand/obsbot4linux --base rebuild --head groundwork/guards \
  --title "Groundwork: guards for the workspace move" --body-file "$SCRATCH/pr-body.md"
```

The body says: part of the groundwork stage, Task 1 of
`docs/superpowers/plans/2026-11-13-obsbot4linux-groundwork.md`; no behaviour change; what each
guard records. The release dry run starts (the pull request touches `release.yml` and
`scripts/`). Its job `contents` fails the first time: `packaging/contents.txt` does not exist
yet. The job uploads the list it made as the artifact `package-contents`; take it from there:

```bash
run=$(gh run list -R hellwand/obsbot4linux --workflow release.yml --branch groundwork/guards \
  --limit 1 --json databaseId --jq '.[0].databaseId')
timeout 3600 gh run watch "$run" -R hellwand/obsbot4linux --interval 30 > /dev/null || true
gh run download "$run" -R hellwand/obsbot4linux -n package-contents -D "$SCRATCH/contents"
cp "$SCRATCH/contents/contents.txt" packaging/contents.txt
cut -d' ' -f1 packaging/contents.txt | sort -u | wc -l
grep -c 'obsbot4linux_VERSION_ARCH.deb' packaging/contents.txt
grep 'VERSION-1.ARCH.rpm' packaging/contents.txt | sort -u
```

Expected: `4` (each normalised package name stands for its two architectures); 28 deb lines (14 per architecture); the rpm
lines (each twice in the file, once per architecture):

```text
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.service
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/lib/systemd/user/obsbot4linux.socket
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/doc/obsbot4linux/CHANGELOG.md
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/doc/obsbot4linux/README.md
obsbot4linux-VERSION-1.ARCH.rpm -rw-r--r-- ./usr/share/licenses/obsbot4linux/LICENSE
obsbot4linux-VERSION-1.ARCH.rpm -rwxr-xr-x ./usr/bin/obsbot4linux
```

Commit `packaging/contents.txt` ("Record the package contents of 0.13.2", same trailer rule),
push, and wait for the checks: `timeout 3600 gh pr checks <PR> -R hellwand/obsbot4linux
--watch --interval 30`. Expected: `required` and every release job green, `contents` included.

**Expected move check:** none (nothing moves). **Guards:** G1 (three names added), G2 and G3
(created here), G4, G5 (`contents` green), G7.

---

### Task 2: The workspace, files moved flat

**Files:**

- Create: `crates/obsbot-device/Cargo.toml`, `crates/obsbot4linux/Cargo.toml`
- Move (git mv): `src/{camera,commands,device,error,frame,lib,presets,state,status,streaming}.rs`
  → `crates/obsbot-device/src/`; `src/models.rs` → `crates/obsbot-device/src/registry.rs`;
  `src/{cli,client,daemon,main,proto}.rs` → `crates/obsbot4linux/src/`;
  `tests/byte_guard.rs`, `tests/fixtures/byte-guard.txt` → `crates/obsbot-device/tests/`;
  `tests/cli_surface.rs`, `tests/fixtures/cli-surface.txt` → `crates/obsbot4linux/tests/`
- Modify: `Cargo.toml` (virtual manifest), `Cargo.lock`, `crates/obsbot-device/src/lib.rs`,
  `crates/obsbot-device/src/device.rs`, `crates/obsbot4linux/src/{cli,client,daemon}.rs`,
  `crates/obsbot-device/tests/byte_guard.rs`, `.github/workflows/ci.yml`,
  `.github/workflows/release.yml`, `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`
- Unchanged and checked: `packaging/arch/PKGBUILD`, `scripts/build-arch.sh` (they build and
  test at the workspace root; the binary stays at `target/release/obsbot4linux`)

**Interfaces:**

- Consumes: Task 1's guards and scripts.
- Produces: package `obsbot-device` with the flat modules `obsbot_device::{camera, commands,
  device, error, frame, presets, registry, state, status, streaming}`; package `obsbot4linux`
  (binary only); `lib.rs` in four blank-line separated groups that Tasks 3, 4, 5 and 6 each
  edit alone.

- [ ] **Step 1: Branch from Task 1 and record the before state**

```bash
git switch -c groundwork/workspace origin/groundwork/guards
export CARGO_TARGET_DIR="$SCRATCH/target-groundwork-workspace"
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-base.txt"
cargo metadata --format-version 1 --locked | jq -r '.packages[] | select(.source != null) | "\(.name) \(.version)"' | sort > "$SCRATCH/deps-base.txt"
cargo metadata --no-deps --format-version 1 --locked | jq -c '.packages[] | select(.name == "obsbot4linux") | {name, version, license, description, homepage, repository, authors, edition, rust_version}' > "$SCRATCH/meta-base.txt"
git rev-parse HEAD > "$SCRATCH/base-rev.txt"
```

- [ ] **Step 2: Move the files**

```bash
mkdir -p crates/obsbot-device/src crates/obsbot-device/tests/fixtures \
  crates/obsbot4linux/src crates/obsbot4linux/tests/fixtures
for f in camera commands device error frame lib presets state status streaming; do
  git mv "src/$f.rs" "crates/obsbot-device/src/$f.rs"
done
git mv src/models.rs crates/obsbot-device/src/registry.rs
for f in cli client daemon main proto; do
  git mv "src/$f.rs" "crates/obsbot4linux/src/$f.rs"
done
git mv tests/byte_guard.rs crates/obsbot-device/tests/byte_guard.rs
git mv tests/fixtures/byte-guard.txt crates/obsbot-device/tests/fixtures/byte-guard.txt
git mv tests/cli_surface.rs crates/obsbot4linux/tests/cli_surface.rs
git mv tests/fixtures/cli-surface.txt crates/obsbot4linux/tests/fixtures/cli-surface.txt
rmdir src tests/fixtures tests
```

- [ ] **Step 3: Write the root `Cargo.toml`** (replaces the whole file)

```toml
[workspace]
resolver = "3"
members = ["crates/obsbot-device", "crates/obsbot4linux"]

[workspace.package]
version = "0.13.2"
edition = "2024"
rust-version = "1.88"
authors = ["Thomas Kroll"]
license = "GPL-3.0-or-later"
repository = "https://github.com/hellwand/obsbot4linux"
homepage = "https://github.com/hellwand/obsbot4linux"
publish = false

[workspace.dependencies]
obsbot-device = { path = "crates/obsbot-device" }
clap = { version = "4", features = ["derive"] }
nix = { version = "0.31", features = ["fs", "ioctl"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
hex = "0.4"

[profile.release]
strip = true
```

- [ ] **Step 4: Write `crates/obsbot-device/Cargo.toml`**

```toml
[package]
name = "obsbot-device"
description = "Camera access for obsbot4linux: transport, frame codec and the camera models"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
publish.workspace = true

[dependencies]
clap.workspace = true
nix.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[dev-dependencies]
hex.workspace = true
```

- [ ] **Step 5: Write `crates/obsbot4linux/Cargo.toml`** (the package metadata of the root
      manifest, with paths relative to the crate)

```toml
[package]
name = "obsbot4linux"
description = "Control OBSBOT webcams on Linux from the console and the DMS widget"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
publish.workspace = true
readme = "../../README.md"

[[bin]]
name = "obsbot4linux"
path = "src/main.rs"

[dependencies]
obsbot-device.workspace = true
clap.workspace = true
nix.workspace = true
serde.workspace = true
serde_json.workspace = true

[package.metadata.deb]
maintainer = "Thomas Kroll <t.kroll@outlook.com>"
section = "video"
extended-description = "Most settings OBSBOT Center offers, from the console and from the DMS bar widget.\nWhat is missing: https://github.com/hellwand/obsbot4linux/blob/main/docs/center-parity.md\nThe obsbot4linux daemon (systemd user socket) owns the camera state."
depends = "$auto"
license-file = ["../../LICENSE", "0"]
assets = [
    ["target/release/obsbot4linux", "usr/bin/", "755"],
    ["../../packaging/systemd/obsbot4linux.service", "usr/lib/systemd/user/", "644"],
    ["../../packaging/systemd/obsbot4linux.socket", "usr/lib/systemd/user/", "644"],
    ["../../README.md", "usr/share/doc/obsbot4linux/", "644"],
    ["../../CHANGELOG.md", "usr/share/doc/obsbot4linux/", "644"],
]

[package.metadata.generate-rpm]
assets = [
    { source = "target/release/obsbot4linux", dest = "/usr/bin/obsbot4linux", mode = "755" },
    { source = "../../packaging/systemd/obsbot4linux.service", dest = "/usr/lib/systemd/user/obsbot4linux.service", mode = "644" },
    { source = "../../packaging/systemd/obsbot4linux.socket", dest = "/usr/lib/systemd/user/obsbot4linux.socket", mode = "644" },
    { source = "../../README.md", dest = "/usr/share/doc/obsbot4linux/README.md", mode = "644", doc = true },
    { source = "../../CHANGELOG.md", dest = "/usr/share/doc/obsbot4linux/CHANGELOG.md", mode = "644", doc = true },
    { source = "../../LICENSE", dest = "/usr/share/licenses/obsbot4linux/LICENSE", mode = "644" },
]
```

- [ ] **Step 6: Write `crates/obsbot-device/src/lib.rs`** (one group per later task)

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Control library for the OBSBOT Tiny 2 Lite.

pub mod camera;
pub mod error;
pub mod state;

pub mod frame;
pub mod streaming;

pub mod device;
pub mod registry;

pub mod commands;
pub mod presets;
pub mod status;
```

- [ ] **Step 7: Fix the paths that name the old crate or `models`**

```bash
sed -i -e 's/^use crate::models::{self, Model};$/use crate::registry::{self, Model};/' \
  -e 's/models::from_card(/registry::from_card(/' -e 's/models::from_by_id(/registry::from_by_id(/' \
  crates/obsbot-device/src/device.rs
sed -i 's/obsbot4linux::/obsbot_device::/g' crates/obsbot4linux/src/cli.rs \
  crates/obsbot4linux/src/client.rs crates/obsbot4linux/src/daemon.rs
sed -i 's/obsbot_device::models::MODELS/obsbot_device::registry::MODELS/' crates/obsbot4linux/src/cli.rs
sed -i 's/^use obsbot4linux::/use obsbot_device::/' crates/obsbot-device/tests/byte_guard.rs
grep -rn 'obsbot4linux::\|crate::models' crates
```

Expected from the grep: nothing.

- [ ] **Step 8: Update the lock file and check that only our packages changed**

```bash
nice -n 19 cargo check --workspace -j 3
git diff Cargo.lock
```

Expected: the diff adds a `[[package]] name = "obsbot-device"` entry (dependencies `clap`,
`hex`, `nix`, `serde`, `serde_json`, `thiserror`) and changes the `obsbot4linux` entry's
dependencies to `clap`, `nix`, `obsbot-device`, `serde`, `serde_json`. No other entry changes.

- [ ] **Step 9: CI, release workflow and docs**

`.github/workflows/ci.yml`: in the jobs `clippy`, `test` and `msrv` add `--workspace`:

```yaml
- run: cargo clippy --workspace --all-targets --locked -- -D warnings
```

```yaml
- run: cargo test --workspace --locked
```

```yaml
cargo "+$msrv" test --workspace --locked
```

`.github/workflows/release.yml`: the same three lines in the job `test`; in `on.pull_request.
paths` after `- Cargo.lock` add `- crates/*/Cargo.toml`; in the job `deb` the package command
becomes

```yaml
cargo deb -p obsbot4linux --no-build --deb-revision "" -o "dist/obsbot4linux_${ver}_$DEB.deb"
```

and in the job `rpm`

```yaml
cargo generate-rpm -p crates/obsbot4linux -o "dist/obsbot4linux-$ver-1.$ARCH.rpm"
```

`CONTRIBUTING.md`, item 3: `cargo clippy --all-targets --locked -- -D warnings` becomes
`cargo clippy --workspace --all-targets --locked -- -D warnings`, and both `cargo test` become
`cargo test --workspace`. `README.md`: `` in `src/models.rs` `` becomes
`` in `crates/obsbot-device/src/registry.rs` ``. `CHANGELOG.md`, last line of
`## [Unreleased]` / `### Changed`:

```markdown
- The source is a cargo workspace now: `crates/obsbot-device` holds the camera access,
  `crates/obsbot4linux` the CLI and the daemon. The program, its output and the packages do not
  change.
```

Run: `actionlint .github/workflows/*.yml`
Expected: no output.

- [ ] **Step 10: Gate**

```bash
nice -n 19 cargo fmt --check
nice -n 19 cargo clippy --workspace --all-targets --locked -j 3 -- -D warnings
nice -n 19 cargo test --workspace --locked -j 3 2>&1 | grep 'test result'
```

Expected: no fmt or clippy output; `118 passed`, `1 passed` (byte_guard), `25 passed`,
`2 passed` (cli_surface), `0 passed` (doc tests).

- [ ] **Step 11: Dependencies and package metadata did not change**

```bash
cargo metadata --format-version 1 --locked | jq -r '.packages[] | select(.source != null) | "\(.name) \(.version)"' | sort | diff "$SCRATCH/deps-base.txt" -
cargo metadata --no-deps --format-version 1 --locked | jq -c '.packages[] | select(.name == "obsbot4linux") | {name, version, license, description, homepage, repository, authors, edition, rust_version}' | diff "$SCRATCH/meta-base.txt" -
```

Expected: both diffs print nothing.

- [ ] **Step 12: The release binary is still stripped**

```bash
nice -n 19 cargo build --release --locked -j 3
file "$CARGO_TARGET_DIR/release/obsbot4linux" | grep -o 'stripped$'
```

Expected: `stripped`. The packages themselves are built by the release dry run of the pull
request (G5).

- [ ] **Step 13: Commit, guards, pull request**

```bash
git add -A
git status --short
git commit -F - <<'EOF'
Make the source a cargo workspace

The library modules move to crates/obsbot-device, models.rs as registry.rs, and the 0.13 CLI
and daemon to crates/obsbot4linux, which keeps the package name, the binary and the package
metadata. Version, rust-version and licence come from [workspace.package]. No behaviour change.

Co-Authored-By: <model of this session> <noreply@anthropic.com>
EOF
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-head.txt"
LC_ALL=C comm -3 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"
scripts/groundwork/move-check.sh "$(cat "$SCRATCH/base-rev.txt")" HEAD
git diff -M --stat "$(cat "$SCRATCH/base-rev.txt")" HEAD
```

Expected: `git status` lists only the files of this task; `comm` prints nothing; the stat
shows every moved file as a rename (`{src => crates/obsbot-device/src}/frame.rs | 0`,
`models.rs` renamed to `registry.rs` with `0`, `device.rs` with `6`, the two fixtures with `0`).

**Expected move check:**

```text
< && let Some(model) = models::from_card(&card)
> && let Some(model) = registry::from_card(&card)
< let Some(found) = obsbot4linux::device::detect() else {
> let Some(found) = obsbot_device::device::detect() else {
< let log = obsbot4linux::state::path().with_file_name("daemon.log");
> let log = obsbot_device::state::path().with_file_name("daemon.log");
< let model = models::from_by_id(&device.file_name()?.to_string_lossy())?;
> let model = registry::from_by_id(&device.file_name()?.to_string_lossy())?;
```

(`lib.rs` changes only `mod` lines, which the check sets aside; its doc line stays.) Push,
pull request into `rebuild` titled "Groundwork: make the source a cargo workspace", stacked on
Task 1's pull request; wait for the checks as in Task 1, Step 13.
**Guards:** G1, G2, G3, G4, G5 (the dry run runs: `Cargo.toml` changed; `contents` green with
the unchanged `packaging/contents.txt`), G6, G7.

---

### Task 3: `family` (frame codec and on-air probe)

**Files:**

- Move: `crates/obsbot-device/src/frame.rs` → `crates/obsbot-device/src/family/frame.rs`,
  `crates/obsbot-device/src/streaming.rs` → `crates/obsbot-device/src/family/streaming.rs`
- Create: `crates/obsbot-device/src/family/mod.rs`
- Modify: `crates/obsbot-device/src/lib.rs` (the group `pub mod frame; pub mod streaming;`
  only)

**Interfaces:**

- Consumes: Task 2's flat `obsbot_device::{frame, streaming}`.
- Produces: `obsbot_device::family::frame` and `obsbot_device::family::streaming`; the old paths
  `obsbot_device::frame` and `obsbot_device::streaming` stay valid through
  `pub use family::{frame, streaming};` until Task 7.

- [ ] **Step 1: Branch from Task 2, record the base**

```bash
git switch -c groundwork/family origin/groundwork/workspace
export CARGO_TARGET_DIR="$SCRATCH/target-groundwork-family"
git rev-parse HEAD > "$SCRATCH/base-rev.txt"
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-base.txt"
```

- [ ] **Step 2: Move**

```bash
mkdir -p crates/obsbot-device/src/family
git mv crates/obsbot-device/src/frame.rs crates/obsbot-device/src/family/frame.rs
git mv crates/obsbot-device/src/streaming.rs crates/obsbot-device/src/family/streaming.rs
```

- [ ] **Step 3: Write `crates/obsbot-device/src/family/mod.rs`**

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Building blocks that are not tied to one camera model: the frame codec and the on-air probe.

pub mod frame;
pub mod streaming;
```

- [ ] **Step 4: Replace the group in `lib.rs`**

The two lines `pub mod frame;` and `pub mod streaming;` become:

```rust
pub mod family;
pub use family::{frame, streaming};
```

- [ ] **Step 5: Gate.** Run the gate block of the Guards section. Expected: no output from
      fmt, clippy, shellcheck and actionlint; five `test result: ok` lines with 118, 1, 25, 2
      and 0 passed tests (on top of 66287b5).

- [ ] **Step 6: Commit, guards, pull request**

```bash
git add -A
git commit -F - <<'EOF'
Move the frame codec and the on-air probe to family

frame.rs and streaming.rs move unchanged; lib.rs keeps their old paths as re-exports until
the final path change of the groundwork stage.

Co-Authored-By: <model of this session> <noreply@anthropic.com>
EOF
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-head.txt"
LC_ALL=C comm -3 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"
scripts/groundwork/move-check.sh "$(cat "$SCRATCH/base-rev.txt")" HEAD
```

Expected: `comm` prints nothing. **Expected move check:**

```text
> // SPDX-License-Identifier: GPL-3.0-or-later
> //! Building blocks that are not tied to one camera model: the frame codec and the on-air probe.
```

Pull request into `rebuild` titled "Groundwork: move the frame codec and the on-air probe to
family", stacked on Task 2's. **Guards:** G1, G2, G3, G4, G6, G7 (G5 does not run: no
packaging input changes).

---

### Task 4: `transport` and `enumeration` (split of `device.rs`)

**Files:**

- Move: `crates/obsbot-device/src/device.rs` → `crates/obsbot-device/src/transport/uvc.rs`
- Create: `crates/obsbot-device/src/transport/mod.rs`, `crates/obsbot-device/src/enumeration.rs`
- Modify: `crates/obsbot-device/src/lib.rs` (the group `pub mod device; pub mod registry;`
  only)

**Interfaces:**

- Consumes: Task 2's `obsbot_device::device` and `obsbot_device::registry`.
- Produces: `obsbot_device::transport::uvc::{Device, CtrlRange, XU_SIZE}`,
  `obsbot_device::enumeration::{Detected, detect}`; the old path
  `obsbot_device::device::{CtrlRange, Detected, Device, detect}` stays valid through an inline
  re-export module until Task 7.

- [ ] **Step 1: Branch from Task 2, record the base**

```bash
git switch -c groundwork/transport origin/groundwork/workspace
export CARGO_TARGET_DIR="$SCRATCH/target-groundwork-transport"
git rev-parse HEAD > "$SCRATCH/base-rev.txt"
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-base.txt"
```

- [ ] **Step 2: Move and split.** Save the script below as `$SCRATCH/split-device.py` (not in
      the repository) and run it from the repository root.

```bash
mkdir -p crates/obsbot-device/src/transport
git mv crates/obsbot-device/src/device.rs crates/obsbot-device/src/transport/uvc.rs
python3 -I "$SCRATCH/split-device.py"
```

```python
"""Splits crates/obsbot-device/src/transport/uvc.rs (the 0.13 device.rs, moved there with git
mv) into uvc.rs (the device and its ioctls) and enumeration.rs (finding the camera). Every
block is copied unchanged. Run from the repository root: python3 -I split-device.py"""

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
```

Expected: no output (every anchor matched). If an assertion fails, `main` changed `device.rs`:
find the anchors with `grep -n`, correct the numbers and say so in the pull request.

- [ ] **Step 3: Write `crates/obsbot-device/src/transport/mod.rs`**

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Access to the camera's USB device: extension-unit queries and V4L2 controls.

pub mod uvc;
```

- [ ] **Step 4: Replace the group in `lib.rs`**

The two lines `pub mod device;` and `pub mod registry;` become:

```rust
pub mod enumeration;
pub mod registry;
pub mod transport;
/// The 0.13 path of the device module, until Task 7 of the groundwork plan removes it.
pub mod device {
    pub use crate::enumeration::{Detected, detect};
    pub use crate::transport::uvc::{CtrlRange, Device};
}
```

- [ ] **Step 5: Gate.** Run the gate block of the Guards section. Expected: no output from
      fmt, clippy, shellcheck and actionlint; five `test result: ok` lines with 118, 1, 25, 2
      and 0 passed tests (on top of 66287b5).

- [ ] **Step 6: Each ioctl and layout of `device.rs` exists exactly once**

```bash
D=crates/obsbot-device/src
grep -rn 'ioctl_read!\|ioctl_readwrite!\|^struct V4l2\|^struct UvcXu' crates | wc -l
grep -n 'ioctl_read!\|ioctl_readwrite!\|^struct V4l2\|^struct UvcXu' $D/enumeration.rs $D/transport/uvc.rs | sed 's/:[0-9]*:/: /'
```

Expected: `11` in the whole tree (the six ioctls and five layouts of 0.13: these nine and the
on-air probe's `vidioc_reqbufs` with `V4l2RequestBuffers`), and from the second grep exactly:

```text
crates/obsbot-device/src/enumeration.rs: struct V4l2Capability {
crates/obsbot-device/src/enumeration.rs: ioctl_read!(vidioc_querycap, b'V', 0, V4l2Capability);
crates/obsbot-device/src/transport/uvc.rs: struct UvcXuControlQuery {
crates/obsbot-device/src/transport/uvc.rs: struct V4l2Control {
crates/obsbot-device/src/transport/uvc.rs: struct V4l2QueryCtrl {
crates/obsbot-device/src/transport/uvc.rs: ioctl_readwrite!(uvcioc_ctrl_query, b'u', 0x21, UvcXuControlQuery);
crates/obsbot-device/src/transport/uvc.rs: ioctl_readwrite!(vidioc_g_ctrl, b'V', 27, V4l2Control);
crates/obsbot-device/src/transport/uvc.rs: ioctl_readwrite!(vidioc_s_ctrl, b'V', 28, V4l2Control);
crates/obsbot-device/src/transport/uvc.rs: ioctl_readwrite!(vidioc_queryctrl, b'V', 36, V4l2QueryCtrl);
```

- [ ] **Step 7: Commit, guards, pull request**

```bash
git add -A
git commit -F - <<'EOF'
Split device.rs into transport/uvc.rs and enumeration.rs

The device, its ioctls and the extension-unit access move to transport/uvc.rs; finding the
camera (by-id links, card name) moves to enumeration.rs with its test. Every block is copied
unchanged. lib.rs keeps the old device path as a re-export until the final path change.

Co-Authored-By: <model of this session> <noreply@anthropic.com>
EOF
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-head.txt"
LC_ALL=C comm -3 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"
scripts/groundwork/move-check.sh "$(cat "$SCRATCH/base-rev.txt")" HEAD
```

Expected: `comm` prints nothing. **Expected move check:**

```text
> #[cfg(test)]
> // Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.
> // SPDX-License-Identifier: GPL-3.0-or-later
> // SPDX-License-Identifier: GPL-3.0-or-later
> //! Access to the camera's USB device: extension-unit queries and V4L2 controls.
> //! Finding a supported camera without talking to it: `/dev/v4l/by-id` links first, the V4L2
> //! card name as fallback.
> /// The 0.13 path of the device module, until Task 7 of the groundwork plan removes it.
> mod tests {
> pub mod device {
> }
> }
```

Pull request into `rebuild` titled "Groundwork: split device.rs into transport and
enumeration", stacked on Task 2's. **Guards:** G1, G2, G3, G4, G6, G7.

---

### Task 5: `models/tiny2lite` (commands, presets, status)

**Files:**

- Move: `crates/obsbot-device/src/{commands,presets,status}.rs` →
  `crates/obsbot-device/src/models/tiny2lite/`
- Create: `crates/obsbot-device/src/models/mod.rs`,
  `crates/obsbot-device/src/models/tiny2lite/mod.rs`
- Modify: `crates/obsbot-device/src/lib.rs` (the group `pub mod commands; pub mod presets;
  pub mod status;` only)

**Interfaces:**

- Consumes: Task 2's flat `obsbot_device::{commands, presets, status}`.
- Produces: `obsbot_device::models::tiny2lite::{commands, presets, status}`; the old paths stay
  valid through `pub use models::tiny2lite::{commands, presets, status};` until Task 7.

- [ ] **Step 1: Branch from Task 2, record the base**

```bash
git switch -c groundwork/tiny2lite-modules origin/groundwork/workspace
export CARGO_TARGET_DIR="$SCRATCH/target-groundwork-tiny2lite-modules"
git rev-parse HEAD > "$SCRATCH/base-rev.txt"
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-base.txt"
```

- [ ] **Step 2: Move**

```bash
mkdir -p crates/obsbot-device/src/models/tiny2lite
for f in commands presets status; do
  git mv "crates/obsbot-device/src/$f.rs" "crates/obsbot-device/src/models/tiny2lite/$f.rs"
done
```

- [ ] **Step 3: Write the two module files**

`crates/obsbot-device/src/models/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The camera models, one module each.

pub mod tiny2lite;
```

`crates/obsbot-device/src/models/tiny2lite/mod.rs`:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! The OBSBOT Tiny 2 Lite (USB 3564:fef9).

pub mod commands;
pub mod presets;
pub mod status;
```

- [ ] **Step 4: Replace the group in `lib.rs`**

The three lines `pub mod commands;`, `pub mod presets;`, `pub mod status;` become:

```rust
pub mod models;
pub use models::tiny2lite::{commands, presets, status};
```

- [ ] **Step 5: Gate.** Run the gate block of the Guards section. Expected: no output from
      fmt, clippy, shellcheck and actionlint; five `test result: ok` lines with 118, 1, 25, 2
      and 0 passed tests (on top of 66287b5).

- [ ] **Step 6: Commit, guards, pull request**

```bash
git add -A
git commit -F - <<'EOF'
Move the Tiny 2 Lite command builders and decoders to models/tiny2lite

commands.rs, presets.rs and status.rs move unchanged, with their tests; lib.rs keeps their old
paths as re-exports until the final path change of the groundwork stage.

Co-Authored-By: <model of this session> <noreply@anthropic.com>
EOF
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-head.txt"
LC_ALL=C comm -3 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"
scripts/groundwork/move-check.sh "$(cat "$SCRATCH/base-rev.txt")" HEAD
```

Expected: `comm` prints nothing. **Expected move check:**

```text
> // SPDX-License-Identifier: GPL-3.0-or-later
> // SPDX-License-Identifier: GPL-3.0-or-later
> //! The OBSBOT Tiny 2 Lite (USB 3564:fef9).
> //! The camera models, one module each.
```

Pull request into `rebuild` titled "Groundwork: move the Tiny 2 Lite builders and decoders to
models/tiny2lite", stacked on Task 2's. **Guards:** G1, G2, G3, G4, G6, G7.

---

### Task 6: Split `camera.rs`

Starts after Tasks 1 to 5 have landed on `rebuild`.

**Files:**

- Move: `crates/obsbot-device/src/camera.rs` → `crates/obsbot-device/src/models/tiny2lite/camera.rs`
  (then rewritten by the split), `crates/obsbot-device/src/state.rs` →
  `crates/obsbot-device/src/legacy/state.rs`
- Create: `crates/obsbot-device/src/family/exchange.rs`,
  `crates/obsbot-device/src/legacy/mod.rs`, `crates/obsbot-device/src/legacy/full_status.rs`,
  `crates/obsbot-device/src/legacy/current_pose.rs`
- Modify: `crates/obsbot-device/src/lib.rs` (the group `pub mod camera; pub mod error; pub mod
  state;`), `crates/obsbot-device/src/family/mod.rs`,
  `crates/obsbot-device/src/models/tiny2lite/mod.rs`, `crates/obsbot4linux/src/cli.rs` and
  `crates/obsbot4linux/src/daemon.rs` (the `use` lines that name `camera`),
  `docs/center-parity.md` (two paths)

**Interfaces:**

- Consumes: `crate::family::frame::{Answer, Sequence, parse_answer, query_frame}`,
  `crate::transport::uvc::{Device, CtrlRange}`, `crate::models::tiny2lite::{commands, presets,
  status}`, `crate::family::streaming` (Tasks 3 to 5).
- Produces: `obsbot_device::family::exchange::{Camera, PAIR_GAP}` (`Camera::open() ->
  Result<Camera>`; `pub(crate)` fields `dev: Device`, `seq: &'static Sequence`; `pub(crate)`
  methods `short(&self, &[u8]) -> Result<()>`, `framed(&self, &[u8]) -> Result<()>`,
  `pair(&self, [[u8; 60]; 2]) -> Result<()>`, `query(&self, u16) -> Result<Vec<u8>>`,
  `query_to(&self, u8, u16) -> Result<Vec<u8>>`, `answer(&self, u16, u16) -> Result<Vec<u8>>`);
  `obsbot_device::models::tiny2lite::camera::{Identity, LivePose, Zone, Hand, HandArea,
  Framing, Offset, ImageControl, IMAGE_CONTROLS, ZOOM_ABSOLUTE}` and every flow method of
  `Camera`; `obsbot_device::legacy::full_status::{CAPABILITIES, FullStatus, Limits, LIMITS,
  StateView, ZoneInfo, Zoom}` with `Camera::{zone_info, full_status, full_status_with}`;
  `obsbot_device::legacy::current_pose` with `Camera::current_pose` (`pub(crate)`);
  `obsbot_device::legacy::state` (old path `obsbot_device::state` kept as a re-export until
  Task 7).

- [ ] **Step 1: Branch from `rebuild`, record the base**

```bash
git fetch origin
git switch -c groundwork/camera-split origin/rebuild
export CARGO_TARGET_DIR="$SCRATCH/target-groundwork-camera-split"
git rev-parse HEAD > "$SCRATCH/base-rev.txt"
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-base.txt"
```

- [ ] **Step 2: Move, then split.** Save the script below as `$SCRATCH/split-camera.py` (not
      in the repository) and run it from the repository root.

```bash
mkdir -p crates/obsbot-device/src/legacy
git mv crates/obsbot-device/src/state.rs crates/obsbot-device/src/legacy/state.rs
git mv crates/obsbot-device/src/camera.rs crates/obsbot-device/src/models/tiny2lite/camera.rs
python3 -I "$SCRATCH/split-camera.py"
```

```python
"""Splits crates/obsbot-device/src/models/tiny2lite/camera.rs (the 0.13 camera.rs, moved there
with git mv) into four files. Every block is copied unchanged; the only edits are the visibility
widenings listed in VIS. Run from the repository root: python3 -I split-camera.py"""

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
```

Expected: no output. If an assertion fails, `main` changed `camera.rs`: find the anchors with
`grep -n`, correct the numbers and say so in the pull request.

- [ ] **Step 3: Write `crates/obsbot-device/src/legacy/mod.rs`**

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! What 0.13 keeps around the camera and what only its daemon and CLI use. The rebuild does not
//! carry it over (spec section 3.2); the 0.13 daemon and CLI use it until they are replaced.

pub mod current_pose;
pub mod full_status;
pub mod state;
```

- [ ] **Step 4: Module declarations**

In `crates/obsbot-device/src/lib.rs` the three lines `pub mod camera;`, `pub mod error;`,
`pub mod state;` become:

```rust
pub mod error;
pub mod legacy;
pub use legacy::state;
```

In `crates/obsbot-device/src/family/mod.rs` add `pub mod exchange;` before `pub mod frame;`.
In `crates/obsbot-device/src/models/tiny2lite/mod.rs` add `pub mod camera;` before
`pub mod commands;`.

- [ ] **Step 5: The binary's imports of `camera`, and the docs**

In `crates/obsbot4linux/src/cli.rs` the line
`use obsbot_device::camera::{Camera, Identity};` becomes

```rust
use obsbot_device::family::exchange::Camera;
use obsbot_device::models::tiny2lite::camera::Identity;
```

In `crates/obsbot4linux/src/daemon.rs` the line
`use obsbot_device::camera::{CAPABILITIES, Camera, Identity, ZoneInfo};` becomes

```rust
use obsbot_device::family::exchange::Camera;
use obsbot_device::legacy::full_status::{CAPABILITIES, ZoneInfo};
use obsbot_device::models::tiny2lite::camera::Identity;
```

```bash
sed -i 's#`src/camera.rs`#`crates/obsbot-device/src/legacy/full_status.rs`#' docs/center-parity.md
grep -rn 'obsbot_device::camera\|crate::camera' crates
```

Expected from the grep: nothing.

- [ ] **Step 6: Gate**

```bash
nice -n 19 cargo fmt
nice -n 19 cargo clippy --workspace --all-targets --locked -j 3 -- -D warnings
nice -n 19 cargo test --workspace --locked -j 3 2>&1 | grep 'test result'
```

Expected: no clippy output (no unused import, no dead code); the same five `test result`
lines as before.

- [ ] **Step 7: Timing constants and the sequence counter exist once, unchanged**

```bash
grep -rn 'static SEQ\|const REPLY_GAP\|const REPLY_ATTEMPTS\|const PAIR_GAP\|const LOCK_WAIT' crates
```

Expected, exactly these five lines:

```text
crates/obsbot-device/src/family/exchange.rs:…:const REPLY_GAP: Duration = Duration::from_millis(50);
crates/obsbot-device/src/family/exchange.rs:…:const REPLY_ATTEMPTS: usize = 20;
crates/obsbot-device/src/family/exchange.rs:…:pub(crate) const PAIR_GAP: Duration = Duration::from_millis(100);
crates/obsbot-device/src/family/exchange.rs:…:    static SEQ: OnceLock<Sequence> = OnceLock::new();
crates/obsbot-device/src/transport/uvc.rs:…:const LOCK_WAIT: Duration = Duration::from_secs(5);
```

- [ ] **Step 8: Commit, guards, pull request**

```bash
git add -A
git commit -F - <<'EOF'
Split camera.rs into the exchange, the Tiny 2 Lite flows and the 0.13 status

The exchange with the extension unit (short and framed writes, answer polling, the sequence
counter) moves to family/exchange.rs; the Tiny 2 Lite reads and writes stay as
models/tiny2lite/camera.rs; the 0.13 status shape, its capability list and the remembered
pose move to legacy, with state.rs. Every block is copied unchanged; items another file now
uses become pub(crate).

Co-Authored-By: <model of this session> <noreply@anthropic.com>
EOF
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-head.txt"
LC_ALL=C comm -3 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"
scripts/groundwork/move-check.sh "$(cat "$SCRATCH/base-rev.txt")" HEAD
git diff -M --stat "$(cat "$SCRATCH/base-rev.txt")" HEAD
```

Expected: `comm` prints nothing; the stat shows `{ => legacy}/state.rs | 0` and
`{ => models/tiny2lite}/camera.rs` as a rename. **Expected move check:**

```text
> // Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.
> // Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.
> // Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.
> // SPDX-License-Identifier: GPL-3.0-or-later
> // SPDX-License-Identifier: GPL-3.0-or-later
> // SPDX-License-Identifier: GPL-3.0-or-later
> // SPDX-License-Identifier: GPL-3.0-or-later
> //! The exchange with the camera's extension unit: short commands, framed commands and the
> //! The pose 0.13 saves when none is given: the remembered gimbal pose, else the one V4L2
> //! The status of 0.13 (`status --json` and the daemon's state): the full status, its
> //! What 0.13 keeps around the camera and what only its daemon and CLI use. The rebuild does not
> //! capability list and limits, and the readbacks the daemon keeps between full reads.
> //! carry it over (spec section 3.2); the 0.13 daemon and CLI use it until they are replaced.
> //! last set, with the live zoom factor.
> //! polling for their answers, with one sequence number counter for the whole process.
> impl Camera {
> impl Camera {
> impl Camera {
> }
> }
> }
```

Pull request into `rebuild` titled "Groundwork: split camera.rs". **Guards:** G1, G2, G3, G4,
G6, G7.

---

### Task 7: Final module paths

**Files:**

- Modify: `crates/obsbot-device/src/lib.rs`, `crates/obsbot-device/src/models/tiny2lite/
  {commands,presets,status,camera}.rs`, `crates/obsbot-device/src/legacy/state.rs`,
  `crates/obsbot-device/src/registry.rs` (doc comment),
  `crates/obsbot-device/tests/byte_guard.rs`, `crates/obsbot4linux/src/{cli,client,daemon}.rs`

**Interfaces:**

- Consumes: the paths of Tasks 3 to 6.
- Produces: the final module tree of "Target layout"; the old paths `obsbot_device::{frame,
  streaming, device, commands, presets, status, state}` no longer exist.

- [ ] **Step 1: Branch from Task 6, record the base**

```bash
git switch -c groundwork/final-paths origin/groundwork/camera-split
export CARGO_TARGET_DIR="$SCRATCH/target-groundwork-final-paths"
git rev-parse HEAD > "$SCRATCH/base-rev.txt"
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-base.txt"
```

- [ ] **Step 2: Write the final `crates/obsbot-device/src/lib.rs`**

```rust
// SPDX-License-Identifier: GPL-3.0-or-later

//! Control library for the OBSBOT Tiny 2 Lite.

pub mod enumeration;
pub mod error;
pub mod family;
pub mod legacy;
pub mod models;
pub mod registry;
pub mod transport;
```

- [ ] **Step 3: Rewrite the old paths**

```bash
D=crates/obsbot-device/src
B=crates/obsbot4linux/src
sed -i -e 's/\[`crate::frame`\]/[`crate::family::frame`]/' -e 's/^use crate::frame::/use crate::family::frame::/' \
  $D/models/tiny2lite/commands.rs
sed -i -e 's/^use crate::commands::/use crate::models::tiny2lite::commands::/' \
  -e 's/^use crate::frame::/use crate::family::frame::/' $D/models/tiny2lite/presets.rs
sed -i 's/^use crate::commands::/use crate::models::tiny2lite::commands::/' \
  $D/models/tiny2lite/status.rs $D/legacy/state.rs
sed -i 's/^    use crate::commands::AutoZoomSpeed;/    use crate::models::tiny2lite::commands::AutoZoomSpeed;/' \
  $D/models/tiny2lite/camera.rs
sed -i 's/`device::detect` feeds it/`enumeration::detect` feeds it/' $D/registry.rs
sed -i -e 's/^use obsbot_device::commands::/use obsbot_device::models::tiny2lite::commands::/' \
  -e 's/^use obsbot_device::frame::/use obsbot_device::family::frame::/' \
  -e 's/^use obsbot_device::presets::/use obsbot_device::models::tiny2lite::presets::/' \
  crates/obsbot-device/tests/byte_guard.rs
sed -i -e 's/^use obsbot_device::commands::/use obsbot_device::models::tiny2lite::commands::/' \
  -e 's/^use obsbot_device::device::Detected;/use obsbot_device::enumeration::Detected;/' \
  -e 's/^use obsbot_device::presets::/use obsbot_device::models::tiny2lite::presets::/' \
  -e 's/obsbot_device::device::detect()/obsbot_device::enumeration::detect()/' $B/cli.rs
sed -i 's/obsbot_device::state::path()/obsbot_device::legacy::state::path()/' $B/client.rs
sed -i -e 's/^use obsbot_device::state;/use obsbot_device::legacy::state;/' \
  -e 's/^use obsbot_device::streaming;/use obsbot_device::family::streaming;/' $B/daemon.rs
nice -n 19 cargo fmt
grep -rnE '(crate|obsbot_device)::(camera|commands|device|frame|presets|state|status|streaming)\b' crates
```

Expected from the grep: nothing.

- [ ] **Step 4: Gate.** Run the gate block of the Guards section. Expected: no output from
      fmt, clippy, shellcheck and actionlint; five `test result: ok` lines with 118, 1, 25, 2
      and 0 passed tests (on top of 66287b5).

- [ ] **Step 5: Release build and help text** (G3 by hand once more on the release profile)

```bash
nice -n 19 cargo build --release --locked -j 3
"$CARGO_TARGET_DIR/release/obsbot4linux" --version
```

Expected: `obsbot4linux 0.13.2`.

- [ ] **Step 6: Commit, guards, pull request**

```bash
git add -A
git commit -F - <<'EOF'
Use the final module paths and drop the re-exports

Every use line and the two module paths in code name the modules where they live now;
lib.rs declares only the final modules. No behaviour change.

Co-Authored-By: <model of this session> <noreply@anthropic.com>
EOF
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-head.txt"
LC_ALL=C comm -3 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"
scripts/groundwork/move-check.sh "$(cat "$SCRATCH/base-rev.txt")" HEAD
```

Expected: `comm` prints nothing. **Expected move check:**

```text
< //! selector 2 (see [`crate::frame`]). Only ids from the protocol reference are used.
> //! selector 2 (see [`crate::family::frame`]). Only ids from the protocol reference are used.
< //! work so it can be tested without a camera; `device::detect` feeds it from `/dev`.
> //! work so it can be tested without a camera; `enumeration::detect` feeds it from `/dev`.
< /// The 0.13 path of the device module, until Task 7 of the groundwork plan removes it.
< let Some(found) = obsbot_device::device::detect() else {
> let Some(found) = obsbot_device::enumeration::detect() else {
< let log = obsbot_device::state::path().with_file_name("daemon.log");
> let log = obsbot_device::legacy::state::path().with_file_name("daemon.log");
< pub mod device {
< }
```

Pull request into `rebuild` titled "Groundwork: final module paths", stacked on Task 6's.
**Guards:** G1, G2, G3, G4, G6, G7.

---

### Task 8: One mutation pass over the carried modules

**Files:**

- Modify (test modules only, `#[cfg(test)]` code): any file under
  `crates/obsbot-device/src/{family,transport,models,registry.rs,enumeration.rs}` and
  `crates/obsbot-device/src/legacy/full_status.rs` whose survivor a new test kills
- No file outside `#[cfg(test)]` code changes.

**Interfaces:**

- Consumes: the final tree of Task 7.
- Produces: new test functions inside existing test modules (names describe the behaviour
  they pin); the run record in the pull request description.

- [ ] **Step 1: Branch from Task 7, install the tool**

```bash
git switch -c groundwork/mutants origin/groundwork/final-paths
export CARGO_TARGET_DIR="$SCRATCH/target-groundwork-mutants"
git rev-parse HEAD > "$SCRATCH/base-rev.txt"
scripts/groundwork/test-names.sh HEAD > "$SCRATCH/names-base.txt"
nice -n 19 cargo install --locked cargo-mutants@27.1.0
cargo mutants --version
```

Expected: `cargo-mutants 27.1.0`.

- [ ] **Step 2: List the scope**

```bash
SCOPE=(--package obsbot-device --test-workspace=true
  --exclude crates/obsbot-device/src/legacy/state.rs
  --exclude crates/obsbot-device/src/legacy/current_pose.rs)
cargo mutants "${SCOPE[@]}" --list | wc -l
cargo mutants "${SCOPE[@]}" --list | cut -d: -f1 | sort | uniq -c
```

Expected on top of 66287b5: `847`; per file: `enumeration.rs` 16, `family/exchange.rs` 14,
`family/frame.rs` 53, `family/streaming.rs` 23, `legacy/full_status.rs` 5,
`models/tiny2lite/camera.rs` 237, `models/tiny2lite/commands.rs` 208,
`models/tiny2lite/presets.rs` 169, `models/tiny2lite/status.rs` 81, `registry.rs` 5,
`transport/uvc.rs` 36 (other numbers if `main` added code; the run records what it found).

- [ ] **Step 3: Run the pass** (in place, one job, low priority: in-place mutation needs a
      clean tree and touches only this worktree; about 20 minutes)

```bash
git status --short   # must print nothing
timeout 7200 nice -n 19 cargo mutants "${SCOPE[@]}" --in-place --output "$SCRATCH/mutants" 2>&1 | tail -n 1
git status --short   # must print nothing again
```

Expected: the baseline test run passes, then a last line
`847 mutants tested in …: N missed, M caught, U unviable` (and timeouts if any). The lists
are in `$SCRATCH/mutants/mutants.out/{missed,caught,timeout,unviable}.txt`.

- [ ] **Step 4: Give every missed mutant a disposition**

Each line of `missed.txt` gets exactly one:

- **K, killed here:** a new test in the module's existing test module. Expected bytes come from
  a recording (cite "capture N") or from the layout the code documents, never from running the
  code. Rerun that mutant to show it is caught:
  `cargo mutants "${SCOPE[@]}" --in-place -F '<regex matching only that line of missed.txt>'`
  → `1 mutant tested …: 1 caught` (escape `( ) [ ] { } * + . ?` in the regex).
- **D, needs the device:** a method of `Camera` or `Device` whose effect is an extension-unit
  or V4L2 call, or an ioctl structure field. Track A's replay transport covers these.
- **L, 0.13 only:** a write of the remembered state (`State { … }.save()`) inside a flow; not
  carried (spec 3.2).
- **E, equivalent:** no input can tell the mutant from the code; one sentence why.
- **M, waits for main:** the named mutants of Step 5 while their test seam is not on `main`.

A survivor in a function that does no I/O (a builder, decoder, conversion or parser) may only
be K or E.

- [ ] **Step 5: The named mutants of spec 10.2**

```bash
F=crates/obsbot-device/src
grep 'delete match arm 0x89 in parse_answer' "$SCRATCH/mutants/mutants.out/caught.txt"
grep -E 'delete - in (tilt_to_pitch_range|pitch_to_tilt_range|HandArea::decode|LivePose::decode|Camera::gimbal)' \
  "$SCRATCH/mutants/mutants.out/caught.txt" "$SCRATCH/mutants/mutants.out/missed.txt"
# a rejected answer handled as stale (hand-made: cargo-mutants cannot delete that arm)
sed -i 's/Some(Answer::Rejected(code)) => return Err(Error::Rejected(command, code)),/Some(Answer::Rejected(_)) => {}/' $F/family/exchange.rs
nice -n 19 cargo test --workspace --locked -j 3 2>&1 | grep -E '^error|test result'
git checkout -- $F/family/exchange.rs
# a rejected tracking query failing the whole read (hand-made)
sed -i 's/Err(Error::NoAnswer(_) | Error::Rejected(..)) => None,/Err(Error::NoAnswer(_)) => None,/' $F/legacy/full_status.rs
nice -n 19 cargo test --workspace --locked -j 3 2>&1 | grep -E '^error|test result'
git checkout -- $F/legacy/full_status.rs
git status --short
```

Expected on top of 66287b5: the `0x89` arm deletion is in `caught.txt`; every `delete -` in
the four conversion functions is caught, and `delete - in Camera::gimbal` (the tilt negation
of the gimbal move) is in `missed.txt`; both hand-made mutants compile and give five
`test result: ok` lines (survived); `git status` prints nothing. The three survivors get disposition M and
are listed in the pull request as open items of the stage acceptance (see "After the tasks").
Once `main` has the seams of "Before Task 1", P3, and `rebuild` is rebased, this step is
repeated and must show all of them caught (a hand-made mutant then gives a `test result: FAILED` line).

- [ ] **Step 6: Gate and guards**

Run the gate block of the Guards section. Expected: no output from fmt, clippy, shellcheck and
actionlint; all tests pass, with more of them where K tests were added;
`LC_ALL=C comm -23 "$SCRATCH/names-base.txt" "$SCRATCH/names-head.txt"` prints nothing (no
test lost) and `comm -13` prints exactly the new K tests; G2 and G3 unchanged;
`scripts/groundwork/move-check.sh "$(cat "$SCRATCH/base-rev.txt")" HEAD` prints only lines of
the new tests.

- [ ] **Step 7: Commit, pull request with the record**

```bash
git add -A
git commit -F - <<'EOF'
Add the tests that kill mutation survivors in carried code

One cargo-mutants 27.1.0 pass over obsbot-device (without the 0.13-only state and pose
modules). The pull request records the run and the disposition of every survivor.

Co-Authored-By: <model of this session> <noreply@anthropic.com>
EOF
```

The pull request ("Groundwork: mutation pass over the carried modules", stacked on Task 7's)
records: the command line, the tool version, the commit, the last line of the run, a table of
dispositions with counts (K, D, L, E, M), the full `missed.txt` in a collapsed section with the
disposition at the end of each line, and the three named items with their state. If no
survivor is K, there is no code change and no pull request: the same record goes into the
tracking issue of the stage, and the task ends there.
**Guards:** G1 (names only added), G2, G3, G4, G6, G7.

---

## After the tasks: CS-0 and the 0.14.0 release

Not tasks of this plan; listed so that nobody releases early.

1. The named mutants of Task 8, Step 5 are all caught (needs P3 on `main` and a rebase).
2. The camera smoke test CS-0 (spec 10.2, CAM-30) on the real Tiny 2 Lite with a build of the
   tip of `rebuild`: `status`, one setting set and read back, one gimbal move, `preset list`.
3. A release pull request into `rebuild`: workspace version 0.14.0, `Cargo.lock`, PKGBUILD
   `pkgver`, the CHANGELOG heading.
4. `main` fast-forwards to `rebuild`; tag `v0.14.0`; the release workflow runs the `contents`
   job on the tag.

## Self-review

- **Spec coverage (10.2 content):** workspace with `[workspace.package]` and
  `[workspace.dependencies]` – Task 2; `crates/obsbot-device` with `transport`, `family`,
  `models/tiny2lite`, `registry` as plain modules – Tasks 2 to 5 and 7; `camera.rs` split into
  transport-side exchange and Tiny 2 Lite flows – Task 6; the 0.13 daemon and CLI unchanged in
  `crates/obsbot4linux` – Tasks 2, 6, 7 and G3; byte tests moved with their code – every task,
  G1; CI and packaging for the workspace, package metadata in the binary crate – Task 2, G5;
  one mutation pass, survivors fixed or documented – Task 8. Acceptance without camera: every
  0.13.2 test present – G1; the diff shows renames and module paths only – G6, G7; `--help`
  equals 0.13.2's – G3; eight artifacts with the 0.13 binary – G5; the named mutants – Task 8,
  Step 5 (open until P3). Acceptance with camera – "After the tasks". Spec 3.3 licence notes –
  the split scripts write the EUPL note into every file that receives code from a noted file
  (`enumeration.rs`, `exchange.rs`, `full_status.rs`, `current_pose.rs`, `camera.rs`).
- **Placeholder scan:** the only filled-in values left to the builder are the model name in
  the `Co-Authored-By` line, the pull request number in `gh pr checks`, and the regex of a K
  rerun, each defined where it appears.
- **Type and name consistency:** the module paths of each task's "Produces" are the ones the
  next task's "Consumes" and `sed` lines use (checked by building the whole sequence once in a
  scratch copy of `rebuild`: every task's gate, G1, G2, G3 and G6 gave the outputs written
  here).
- **Review Focus:** each of the five items has its check in the owning task (Task 6 Step 7,
  Task 4 Step 6, Task 2 Steps 11 and 12, G5).
