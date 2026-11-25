# obsbot4linux 1.0, stage W4 (live parity and release) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify the release candidate on the real Tiny 2 Lite and release obsbot4linux 1.0.0 and widget 1.0 on the same day (spec 10.3 W4, 10.6): camera session CS-2 on the candidate (CAM-9, CAM-24, CAM-27, CAM-28), every row of 9.1 verified or an exception of 9.2, a tracking table of those results in the release pull request, a new candidate for every change to what the packages ship, then `main` fast-forwarded to `rebuild`, `v1.0.0` tagged, widget 1.0 released, and the `rebuild` exception removed from the release path.

**Architecture:** No new crate, no new script in the repository and no contract change planned. W4 is a camera session plus release mechanics. The only code changes are the fixes and exceptions CS-2 produces (each its own issue and pull request; a contract change goes first, spec 10.1) and the version and CHANGELOG lines of the release. The parity results are joined from `scripts/check-model.sh`'s results file and a map of the 105 rows of 9.1 (kept with the maintainers' session notes); the joined table goes into the release pull request (spec 10.3 W4 owner files: "release PR (tracking table of 9.1 results)").

**Tech Stack:** the published candidate `v1.0.0-rc.1` (Arch package on the owner's desktop), `scripts/check-model.sh` and the `#[ignore]` live tests of `crates/obsbot-device/tests/live.rs` from a worktree at the candidate's tag; `gh` for releases and pull requests; the widget repository `hellwand/obsbot4linux-dux` (branch `develop`, release on `main`); a USB capture of the vendor app for CAM-18 (usbmon, a second machine with the camera passed through).

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` (revision 2 with section 15.1), sections 2, 8.5, 8.6, 8.8, 9.1 to 9.4, 10.1, 10.3 (W4), 10.4, 10.6, 10.7, 12.2, 12.3, 12.5, 14 and 14.1; the W3 plan `docs/superpowers/plans/2026-11-24-obsbot4linux-w3-packaging.md` (task w3-9 Step 7, the hand-over to W4, and coordinator note 6); the track D plan `docs/superpowers/plans/2026-11-13-obsbot4linux-w1-d-widget.md` (M2 and R1).

## Global Constraints

- W4 content (spec 10.3): "install the release candidate on the Arch desktop, starting from the 0.13 state (socket disabled, the 0.13 daemon a child of the widget's `watch`) and following the migration notes; run `check-model.sh` on the Tiny 2 Lite, with `--destructive` last; map its results to every row of 9.1; widget 1.0 against the candidate; the hardened unit starts, sees the camera, and handles hotplug."
- W4 acceptance (spec 10.3): "CAM-9, CAM-24, CAM-27, CAM-28 (and CAM-26 if a second unit is at hand); every row of 9.1 verified or an exception of 9.2. A failing row is fixed or becomes an exception, and a new candidate is built. Then the release follows 10.6." Acceptance without camera: none.
- Release day (spec 10.6), in this order: widget 0.29.2 is out (done 2026-11-13); the candidate is published and W4 has passed on it; `main` fast-forwards to `rebuild`, tag `v1.0.0`, the release workflow publishes, the maintainers' private mirror follows the tag; widget 1.0 the same day (`develop` fast-forwarded into the widget's `main`, `plugin.json` requires obsbot4linux ≥ 1.0, the DankMaterialShell plugin registry entry updated); the rollback packages stay downloadable and the release notes name them.
- 1.0 is done (spec 10.6) when every row of 9.1 is verified on the real Tiny 2 Lite or is an exception of 9.2; CI is green with `fake-a`, `fake-b` and `unknown` together; the packages are proven by the candidate; widget 1.0 is tested against the candidate; the migration notes are written; both walk-throughs (8.4, 8.7) pass, and walk-through 2 runs in CI with the `unknown` fake.
- Branches (spec 10.1): before tagging, `HEAD..origin/main` is empty; the `rebuild` ruleset and the `ci.yml` trigger on `rebuild` are removed after the final merge. After 1.0 (spec 10.7) every release, a candidate included, comes from `main`.
- Section 2 (the owner's line): a request sends exactly what it names; no command sequence is added as a workaround for camera behaviour. A row that fails because the camera behaves differently from the description is fixed in the description (a contract-change pull request, spec 10.1) or becomes an exception (9.2), never by a chain of requests in the daemon or a client.
- Evidence (spec 8.5): every CS-2 answer is recorded as `measured 2026-MM-DD` in `docs/field-ledger.md` and in spec 14.1; nothing is guessed. A result that is not measured stays open and is named as open.
- Public text (spec 8.5 and P5 of the W3 plan, decided 2026-11-24): the CHANGELOG, README, project page, issues and pull requests name no vendor application, no private path, host, file name or serial number, and say nothing about where camera knowledge comes from; captures are cited by key.
- Machine (the run's brief): builds `nice -n 19`, `-j 6` at most, `CARGO_TARGET_DIR` under the builder's own cache directory, never `/tmp`; every wait has a timeout; no `tail -f`, no `pgrep -f`.
- Repository rules: every pull request goes into `rebuild` (until the final merge), one issue each (`Fixes #N` in the description), signed commits, landed fast-forward as the green head of its pull request (check `required`). No agent pushes to `rebuild` or `main` or creates a tag; the coordinator does, after the owner's go where this plan says so.

## Review Focus

1. **Code that reaches the release untested.** `v1.0.0` must be the last candidate plus the release pull request, and that pull request may change only the version strings, `Cargo.lock`'s four workspace version lines, `CHANGELOG.md`, `site/` and `docs/`. Any other change after a candidate needs a new candidate and CS-2b on it (decision 2, task w4-11 Step 3: a `git diff --stat` check against the last candidate's tag).
2. **The release pull request removing its own CI trigger.** For a `pull_request` event GitHub reads the workflow file of the pull request's merge commit; a pull request into `rebuild` that removes `rebuild` from `ci.yml`'s `branches:` would get no `required` check and could not land (GitHub's documented behaviour, not tried here). So the removal of the `rebuild` exception is a pull request into `main` after the tag (decision 3, task w4-14), not part of the release pull request that W3's hand-over named.
3. **The upgrade rehearsal from the real 0.x state.** The owner's desktop runs the candidate since 2026-11-25 (socket enabled by hand). CAM-28 needs the 0.x state first: the rehearsal goes back to 0.14.0 and widget 0.29.2 (which also rehearses the rollback line of the notes) and then follows the CHANGELOG's "Upgrading from 0.13" word for word. Any step that the notes do not state is a failure of the notes, not of the person (task w4-6 block A).
4. **A camera that falls asleep during a check.** In CS-1 two results (CAM-36, the first U-8 run) were confounded by the camera falling asleep about 10 s after a wake without a video stream (hypothesis, not measured). CS-2 measures first whether the camera stays awake with automatic sleep off, and runs the awake blocks only in a state it has measured (task w4-6, block B Step 1).
5. **Rows that need a person.** `check-model.sh` asks the person running it for `readback:false` rows, actions and asleep rows on `/dev/tty`. Those answers are the owner's, typed in his own terminal; an agent never answers them, and `--assume-pass` is CI's only.

---

## What W4 starts from (2026-11-25)

- **Tool.** `rebuild` = `80b64e1` "Release 1.0.0-rc.1", tagged `v1.0.0-rc.1`; the pre-release is published (9 assets: the eight packages and `SHA256SUMS`). `main` = `aa532b6` "Release 0.14.0" is an ancestor of `rebuild`, so `HEAD..origin/main` is empty today. The candidate's CHANGELOG has `## [Unreleased]` (empty) above `## [1.0.0-rc.1] - 2026-11-25` with the "release candidate" paragraph, the supported-models line (Tiny 2 Lite, firmware 6.2.8.1), the migration notes and the rollback line (0.14.0 packages and widget 0.29.2). The release guard still accepts `v1.0.0-rc.*` on `rebuild` (`release.yml` job `verify`), and `ci.yml` still runs on `rebuild`. The project page (`site/`) still describes 0.13.2 (P1 of the W3 plan).
- **Widget.** `develop` = `68d7185`, `plugin.json` version `1.0.0-rc.1`, CHANGELOG section `[1.0.0-rc.1] - 2026-11-25`; `main` = `1eb5a10` "Release 0.29.2" (tag `v0.29.2`). Widget 1.0 has never been on `main`, so `dms plugins update` still installs 0.29.2 (track D decision 1).
- **Owner's desktop.** The candidate is installed (Arch package `1.0.0rc.1-1`, it replaced 0.14.0), `obsbot4linux.socket` enabled by hand, `obsbot4linuxd` running; the widget runs from `develop`. First live check: the widget shows the camera and the zoom works. `$XDG_STATE_HOME/obsbot4linux/` still holds the 0.x `state.json` and `daemon.log` (the notes say it can be deleted).
- **CS-1 (2026-11-25)** answered most of track A's items (spec 14.1, 9.3 "Results of CS-1"), on `rebuild` `9fcf96f` with the 1.0 daemon and the 0.14.0 command line. Open from it, and in W4's scope: CAM-5 (the per-leaf lags of `check-model.sh --asleep`), CAM-18 (which preset the second, clearing frame of `preset.delete` addresses: the capture of 2026-11-25 holds no delete frame), CAM-33 (with a moving person), CAM-36 (zoom and field of view while awake: a rerun of `live_cam_5`, which now wakes the camera before each leaf), U-19 (more values; gates nothing, see decision 7), the go-return actions of CAM-22, CAM-6 over a cold re-plug and the factory reset, CAM-1 asleep.
- **Open issues for W4:** #328 (a pose-taking action fails with the start-pose message; a fix is in progress on its own branch from `80b64e1`), #329 (a zoom or field-of-view write equal to the held value does nothing after the camera's own auto zoom; not yet checked on 1.0), #277 (0.13.2: back-to-back requests wait ~1.3 s; re-measure on 1.0), #332 (rename one test after CAM-20; no assertion change). Milestone 1.0.0 still lists #47, #61, #73 and #89: their pull requests went into `rebuild`, where `Fixes #N` does not close an issue.

## Decisions this plan takes

Each can be overruled; the reason follows the dash.

1. **CS-2 runs on `v1.0.0-rc.1` now, not on a later candidate.** Most of 9.1 does not depend on the fixes in flight (#328 touches the pose-taking actions only); a later candidate is re-checked only where its diff reaches (decision 2) — the camera time goes to the rows once, and the fixes CS-2 finds go into the same next candidate.
2. **Every change to shipped content after a candidate means a new candidate, and CS-2b re-checks only what the diff touches.** Shipped content: every file except `CHANGELOG.md`, `site/`, `docs/` and the version strings. CS-2b runs, on the new candidate, the 9.1 rows and CAM items whose code, description or ledger rows the diff `v1.0.0-rc.N..v1.0.0-rc.N+1` changes, plus the smoke of CAM-30 (status, one set and read back, one gimbal move, `get presets`) and the hardened-unit start (CAM-27) — spec 10.3 asks for a new candidate after a fix; a full CS-2 again would cost hours for rows the diff cannot reach. A change to the daemon's worker, bus or protocol code counts as touching every row: then CS-2b is the full `check-model.sh` run (block B) again.
3. **The release pull request (into `rebuild`) does not remove the `rebuild` exception; a pull request into `main` does, right after `v1.0.0`.** — Review Focus 2. The order closes the gap before the `rebuild` branch and ruleset are deleted (spec 10.1), and the change itself is checked on `main` with the guard table of w3-7 (`v1.0.0-rc.1` on a commit that is only on a `rebuild` branch is refused).
4. **One CHANGELOG section through all candidates.** A new candidate renames `## [1.0.0-rc.N]` to `## [1.0.0-rc.N+1] - <date>` and adds its changes inside that section; the release renames it to `## [1.0.0] - <date>` and drops the "release candidate" paragraph — `scripts/changelog-section.sh` prints exactly one section for a tag, and the release notes of `v1.0.0` must carry the migration notes and the rollback line (w3-9 Step 7 item 1). The candidates' own notes stay on their GitHub pre-releases.
5. **#329 does not block 1.0 unless it reproduces from Linux alone.** It was seen after the vendor app had stored an initial state with automatic zoom; 1.0 sends the same frames as 0.14.0 (section 2: no sequence of our own to get past it). CS-2 block F reproduces it on the candidate and measures whether `do zoom.move` (the path the automatic zoom uses) reaches the target. If it reproduces only after the vendor app's initial state, the CHANGELOG gets one line under the release ("Known issues") and the issue stays open for after 1.0; if it reproduces from a state reached by obsbot4linux alone, the rows `zoom F` and `fov` fail and follow decision 2.
6. **#328: the fix in progress lands before the next candidate; CS-2 records the behaviour on rc.1 and CS-2b verifies the fix.** The root cause named in the issue (a pose read while the camera is asleep, tilt −85) is a hypothesis; CS-2 block C reads the pose and `asleep` at the moment of the action, awake and asleep, so the cause is measured, not assumed.
7. **U-19 and CAM-18 gate nothing for 1.0.** U-19 failed in CS-1 and is the exception PX-U-19 in the candidate; more values only prepare an additive 1.1 (spec 10.7: a new value is a minor version). CAM-18 is evidence for the frames of `preset.delete`: the 9.1 row `preset delete N` is verified by its effect in block B (the other presets unchanged); only if 1.0's delete changes another preset does the row fail (decision 2). Both run in block F, after the gating blocks, and may be skipped if the session runs long; a skip is named in the results.
8. **The per-leaf `settle_ms` stays model-wide unless a new candidate is cut anyway.** `check-model.sh --asleep` gives each leaf's lag (CAM-5); the ledger records them (`measured`). A lag above the model-wide 3000 ms fails that row (a pending value would end before the camera shows it) and is fixed by a contract-change pull request; lags below it only lower bounds, which is an improvement, not a correctness fix — so it rides along in a candidate that decision 2 requires for another reason, and otherwise waits for 1.0.1.
9. **The parity map and the join stay with the maintainers' session notes, not in the repository.** They are used once per release that runs like 1.0 (spec 10.7); the result, the tracking table, is public in the release pull request. If a later major release needs the map again, it can be moved into `scripts/` then.
10. **The upgrade rehearsal uses the widget's `develop` as the stand-in for its release-day `main`.** The widget's 1.0 is on `develop` until R1 (track D decision 1); for the user, step 5 of the notes is `dms plugins update`, which pulls `main`. The rehearsal checks out `develop` in the plugin's clone instead and states this in its result; R1 changes nothing in the widget's code, so the difference is the branch name only.
11. **Milestone 1.0.0 issues are closed by hand on release day**, each with the pull request that fixed it on `rebuild` (w4-4 finds them) — their `Fixes #N` lines were in pull requests into `rebuild`, which do not close issues on GitHub; an issue whose fix the audit does not find goes to the owner, not into 1.0 silently.

## Product questions (recommendations, not built)

- **Q1.** CAM-26 (two cameras at once) needs a second unit. Recommendation: skip for 1.0 (spec 10.3: "if a second unit is at hand"); the fakes cover it in CI.
- **Q2.** A CS-2 row that fails on the camera's own behaviour and has no description fix (as U-5 and U-16 in CS-1): exception PX-n of 9.2 and a new candidate (spec 10.3). Recommendation: accept that path without a separate question per row, since spec 9.2 already lists the rule; the owner sees each exception in the release pull request's tracking table.
- **Q3.** The DankMaterialShell plugin registry entry (spec 10.6 step 4) is an external repository. Recommendation: the coordinator opens the pull request there after the widget's `v1.0.0`, with the owner's go for that release day; no text names where camera knowledge comes from.

## File structure

| File                                                                                                   | Responsibility                                                                  | Task        |
| ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- | ----------- |
| `docs/superpowers/plans/2026-11-25-obsbot4linux-w4-release.md` (new)                                   | this plan, on `rebuild`                                                         | w4-0        |
| `crates/obsbot-device/tests/capture_evidence.rs` (one test name)                                       | #332                                                                            | w4-2        |
| `site/index.html`, `site/style.css` (and `site/assets/` if a picture changes)                          | the project page for 1.0 (P1 of the W3 plan)                                    | w4-3        |
| `docs/field-ledger.md`, spec 14.1 and 9.3/9.2, `description.rs`, `levels.rs`, completions, `docs/*.md` | CS-2's answers, fixes and exceptions (one issue and pull request each)          | w4-8        |
| `Cargo.toml`, `Cargo.lock`, `packaging/arch/PKGBUILD`, `CHANGELOG.md`                                  | a further candidate `1.0.0-rc.N`                                                | w4-9        |
| `Cargo.toml`, `Cargo.lock`, `packaging/arch/PKGBUILD`, `CHANGELOG.md`                                  | `1.0.0`                                                                         | w4-11       |
| `.github/workflows/release.yml` (job `verify`), `.github/workflows/ci.yml` (`branches:`)               | the `rebuild` exception removed (pull request into `main`)                      | w4-14       |
| widget: `plugin.json`, `CHANGELOG.md`, `.github/workflows/ci.yml` (`e2e` checkout ref)                 | widget 1.0.0 (R1)                                                               | w4-5, w4-13 |
| maintainers' session notes (private): the parity map, the CS-2 runbook, results                        | the 9.1 map and its join; what the owner and agents do in CS-2; the raw results | w4-1, w4-6  |

## Tasks, waves and gates

Gates: `none` — an agent may start once the dependencies have landed; `camera` — needs the Tiny 2 Lite and the owner present (CS-2 or CS-2b); `owner go` — the owner says go for a step that publishes, tags, changes the installed packages of the desktop or deletes something. A gated task is not started before its gate holds; the owner's go alone does not start a `camera` task.

| Task  | Title                                                                      | Wave | Depends on               | Gate              | Effort |
| ----- | -------------------------------------------------------------------------- | ---- | ------------------------ | ----------------- | ------ |
| w4-0  | This plan on `rebuild` (docs-only pull request)                            | 0    | –                        | none              | S      |
| w4-1  | The parity map: every row of 9.1 to its CS-2 check                         | 1    | w4-0                     | none              | S      |
| w4-2  | #332: the CAM-20 test name                                                 | 1    | w4-0                     | none              | S      |
| w4-3  | The project page for 1.0 (P1)                                              | 1    | w4-0                     | none              | M      |
| w4-4  | Release audit: milestone 1.0.0, the "done" list of 10.6                    | 1    | w4-0                     | none              | S      |
| w4-5  | Widget R1 prepared on a branch (not opened)                                | 1    | –                        | none              | S      |
| w4-6  | CS-2 on `v1.0.0-rc.1` (blocks A to G)                                      | 1    | w4-1 (for block B)       | camera + owner go | L      |
| w4-7  | #328 (fix in progress) and the other camera-free fixes CS-2 does not touch | 1    | –                        | none              | S      |
| w4-8  | CS-2 results: ledger, spec 14.1, fixes and exceptions (one issue each)     | 2    | w4-6                     | none              | M      |
| w4-9  | Candidate `v1.0.0-rc.2` (only if w4-7 or w4-8 changed shipped content)     | 3    | w4-7, w4-8               | owner go          | S      |
| w4-10 | CS-2b on `v1.0.0-rc.2`: the rows its diff touches, smoke, CAM-27           | 4    | w4-9                     | camera            | S      |
| w4-11 | Release pull request: `1.0.0`, the tracking table                          | 5    | w4-8 (w4-10), w4-3, w4-4 | none              | S      |
| w4-12 | Release: `main` fast-forward, tag `v1.0.0`, check what was published       | 6    | w4-11                    | owner go          | S      |
| w4-13 | Widget 1.0 release (R1), the same day                                      | 7    | w4-12, w4-5              | owner go          | S      |
| w4-14 | After the release: remove the `rebuild` exception, ruleset and branch      | 8    | w4-12, w4-13             | owner go          | S      |

**Critical path:** w4-6 (CS-2, camera and owner) → w4-8 (its fixes and exceptions) together with w4-7 (#328) → w4-9 (`rc.2`, owner go) → w4-10 (CS-2b, camera) → w4-11 → w4-12 (owner go) → w4-13 → w4-14. Without a change to shipped content after rc.1, w4-9 and w4-10 drop and w4-8 leads straight to w4-11. With #328's fix in flight, `rc.2` is expected.

**In parallel, now, without the camera:** w4-0, w4-1, w4-2, w4-3, w4-4, w4-5, w4-7. None of them touches a file another of them touches (w4-2: one test file; w4-3: `site/`; w4-5: the widget repository; w4-1 and w4-4: no repository file; w4-7: #328's own files). w4-6 block B waits for w4-1 only because its results are joined with the map; the session itself can start without it.

---

### Task w4-0 (key `w4-0-plan-doc`): This plan on `rebuild`

A docs-only pull request, before any W4 builder starts: the tasks name this document as their plan (the plans of W0 to W3 landed the same way).

**Files:** Create `docs/superpowers/plans/2026-11-25-obsbot4linux-w4-release.md`.

- [ ] **Step 1: Check the text.** `grep -nEi '/hom[e]/|thom[a]s' <file>` prints nothing; `grep -nE 'sn:[A-Z0-9]{8,}|[A-Z0-9]{14}' <file>` finds no serial; no vendor application is named and no sentence says where camera knowledge comes from; no private note file is named.
- [ ] **Step 2: Format.** `prettier --check <file>`.
- [ ] **Step 3: Commit and pull request.** `docs: plan for stage W4 (live parity and release)`, signed; pull request into `rebuild` with `Fixes #<issue>`; `required` reports without fmt, clippy and test (docs only).

### Task w4-1 (key `w4-1-parity-map`): The parity map

**Gate:** none. **Output:** with the maintainers' session notes (decision 9), not in the repository.

- [ ] **Step 1: The map.** One line per row of spec 9.1 (105 rows on 2026-11-25), tab-separated: the 0.13.2 command as 9.1 spells it, the 1.0 command, and how CS-2 verifies it — one of `check:<kind>:<path>[=<value>]` (a row of `check-model.sh`'s results file, kinds as the script writes them), `cli:<command>` (a command line run on the candidate against the real camera whose output an agent checks, for rows such as `--help`, `--version`, `devices`, `watch --live pose`, `describe`, the exit codes), `px:<PX-n>` (an exception of 9.2), or `block:<letter>` (a CS-2 block step, for rows that need the person or a re-plug). A row may name several checks (`;`-joined); it passes when all pass.
- [ ] **Step 2: The join.** A short shell or `jq` script beside the map: reads the map and one or more `check-model.sh` results files plus the block results, and prints the tracking table (Markdown: 0.13.2, 1.0, result `verified`/`exception PX-n`/`failed`/`not run`, evidence key). Deterministic; no judgement in it (rule: code answers what code can).
- [ ] **Step 3: Prove it without the camera.** Run `scripts/check-model.sh --device <fake-a id> --assume-pass --results <file>` against `obsbot4linuxd --fake fake-a` from the candidate's tag (as CI does), feed that file to the join with every `cli:` and `block:` result set to `not run`: every one of the 105 rows appears exactly once, every `check:` reference finds its results line (a missing line prints `unmapped`, and the step fails while any is printed), and the rows whose 1.0 column is `–` (5 on 2026-11-25) show `exception`; a row with an exception for part of it (as `zoom F --speed S`, PX-6) shows its verified part and names the exception.

### Task w4-2 (key `w4-2-cam20-test-name`): #332

**Gate:** none. **Files:** `crates/obsbot-device/tests/capture_evidence.rs`.

- [ ] **Step 1:** Rename `a_boot_block_decodes_as_values_until_cam_20_is_settled` to `a_zero_status_block_decodes_as_values_cam_20_answered_no`; no assertion changes; `git grep` the old name: no hit left.
- [ ] **Step 2:** `cargo test -p obsbot-device --test capture_evidence` green; pull request with `Fixes #332`. This changes a test only, so it is not shipped content (decision 2).

### Task w4-3 (key `w4-3-project-page`): The project page for 1.0

**Gate:** none. **Files:** `site/` (P1 of the W3 plan). The `pages` job deploys on `v1.0.0` only (a candidate skips it, decision 12 of the W3 plan), so the page can land on `rebuild` any time before w4-11.

- [ ] **Step 1:** Rewrite the page from the 1.0 README: what the tool is, the support table (one row per registered model, as the README), install and upgrade (link to the CHANGELOG's "Upgrading from 0.13"), the generic command line (`status`, `get`, `set`, `do`, `describe`, `devices`, `watch`), the widget, the licence. No 0.13 command, no statement that a gimbal move switches the tracking, no version number that goes stale (the page links the latest release).
- [ ] **Step 2:** Public-text check as w4-0 Step 1; the commands on the page run against `obsbot4linuxd --fake fake-a` with exit 0; open the page locally in a browser at phone and desktop width.
- [ ] **Step 3:** One issue, one pull request into `rebuild`.

### Task w4-4 (key `w4-4-release-audit`): Release audit

**Gate:** none. **Output:** a list for the release pull request (w4-11); no repository file.

- [ ] **Step 1: Milestone 1.0.0.** For #47, #61, #73 and #89 (and any issue added to the milestone by then): find the pull request on `rebuild` that fixed it (`gh pr list --base rebuild --state merged --search "<n>"`, then read the diff), and state per issue: fixed by #PR, or not fixed. A not-fixed issue goes to the owner (move to Backlog or fix before the release); it is not closed.
- [ ] **Step 2: The "done" list of 10.6**, each with its evidence: the CI job that runs `fake-a`, `fake-b` and `unknown` together (name and a green run on `rebuild`); walk-through 2 in CI with `unknown` (the test name); walk-through 1 (8.7) on this spec (where it was walked; if nowhere, say so); the packages proven by the candidate (w3-9 Step 6 record); the migration notes (CHANGELOG section). The two camera items (9.1 rows, widget against the candidate) come from w4-6.
- [ ] **Step 3: Open issues of the tool and the widget** with the label `bug`: list each with "blocks 1.0" or "after 1.0" and a reason; the owner sees the list in the release pull request.

### Task w4-5 (key `w4-5-widget-r1-branch`): Widget R1 prepared

**Gate:** none to prepare; the pull request opens only after the tool's `v1.0.0` (w4-13), because the `e2e` job checks out the tool at `v1.0.0`. **Repository:** `hellwand/obsbot4linux-dux`, branch `release/1.0.0` from `develop`, pushed, no pull request.

- [ ] **Step 1:** `plugin.json` `version` `1.0.0`; CHANGELOG: `## [1.0.0-rc.1] - 2026-11-25` becomes `## [1.0.0] - <date>` (date filled on release day), the candidate sentence replaced by the track D R1 first line ("Needs obsbot4linux 1.0 (protocol 2). Update the widget together with the tool, enable the daemon's socket once (`systemctl --user enable --now obsbot4linux.socket`) and restart the shell."), an empty `## [Unreleased]` above it; the `e2e` job's checkout of the tool `ref: v1.0.0` instead of `rebuild`, with its comment.
- [ ] **Step 2:** `scripts/release-notes.sh HEAD v1.0.0` finds the section; the JS and QML checks pass locally. Rebase onto `develop` again if `develop` moves (M2 fixes).

### Task w4-6 (key `w4-6-cs2`): Camera session CS-2 on `v1.0.0-rc.1`

**Gate:** camera (the owner present), and the owner's go for block A (it changes the desktop's installed packages and goes back to 0.x for a while) and for block G (destructive). **Runbook:** the maintainers' CS-2 runbook (private session notes) holds the commands, the owner's physical steps, durations and the restore steps; this task states what each block must answer. Every block writes its raw output to the session's results directory; nothing is pushed and no issue is opened during the session (w4-8 does that).

- **Block P, preflight (agents, no camera):** a worktree at the tag `v1.0.0-rc.1` with `scripts/check-model.sh` and the live test executable built (`cargo test -p obsbot-device --test live --no-run`); the candidate's and 0.14.0's Arch packages and the `SHA256SUMS` downloaded and checked; the installed state recorded (package versions, unit states, the widget's commit, `$XDG_STATE_HOME/obsbot4linux/`, `ps` of every obsbot process). **Pass:** checksums `OK`, the record written.
- **Block A, rollback and upgrade rehearsal (CAM-28, CAM-27; owner go):** (1) Go back as the notes' rollback line says: the 0.14.0 package, widget 0.29.2 (the plugin's clone at `v0.29.2`), shell reloaded; disable and stop the socket, so that the state is spec 10.3's 0.13 state (socket disabled, the 0.x daemon a child of the widget's `watch`), and check that state with `ps` and `systemctl --user is-enabled`. (2) Follow "Upgrading from 0.13" of the candidate's CHANGELOG step by step, with the candidate's package and the widget's `develop` (decision 10), doing nothing the notes do not say. (3) Then the hardened unit: `obsbot4linuxd` runs as a child of the user manager with `NoNewPrivs: 1` in its `/proc/<pid>/status`, `devices` lists the camera as `tested`; unplug and re-plug the camera: it leaves the list and comes back under the same `sn:` id, and the widget follows. **Pass (CAM-28):** every step of the notes worked as written and no extra step was needed; the old daemon is gone; the widget shows the camera after step 5. **Pass (CAM-27):** as in (3). A missing or wrong step is a CHANGELOG fix (w4-8), not a failed session.
- **Block B, `check-model.sh` on the candidate (CAM-24, CAM-5, #277):** Step 1: measure whether the camera stays awake for 60 s with automatic sleep off and no video stream (`get asleep` every 5 s); if not, keep a video stream open for the awake rows and check that `do sleep` still reaches `asleep` with it open. Step 2: #277 — four back-to-back `set zoom.factor` and one spaced, timed; pass when no back-to-back request waits for a full read (CS-1 measured a cold read of every group at 1.4 s, CAM-10); the numbers go into #277. Step 3: the owner runs `scripts/check-model.sh --device <id> --asleep --results <file>` in his own terminal (it asks him on `/dev/tty`; Review Focus 5), without `--destructive`. **Pass:** every row has a result (a failed row is a result, not a failed session); the script exits after restoring every leaf; the results file holds the lag of every readback row (CAM-5) and the asleep result of every leaf and action whose `asleep` is `unknown` (CAM-2, CAM-22's go-return actions).
- **Block C, a person in view (CAM-33, CAM-34, #328):** CAM-33 with the person moving across the picture: `tracking.profile` `standard`, `sport`, `headroom`, the values of `tracking.speed`, and `framing.*`, one at a time, `status` before and after each (a gesture can switch the AI mode); the owner says what he saw. CAM-34 once more with the 1.0 command line (`do gimbal.move`, `do gimbal.turn` while tracking). #328: at zoom 1×, awake with a stream open, `do power_on.use_current`, with `get asleep` and the live pose read right before it; then the same asleep. **Pass:** each item has the owner's observation recorded; for #328 the pose and `asleep` at the moment of the action are recorded.
- **Block D, widget 1.0 against the candidate (track D M2):** the track D manual list M-1 to M-24 on the real camera wherever it applies (the fake-only items, such as the test-control socket, are marked not applicable), plus CAM-34 (an arrow while tracking sends only the move), CAM-35 (a preset saved without a name shows "(unnamed)" in the widget and `""` in `get presets`), CAM-3 and CAM-22 (what the widget offers while asleep), and the upgrade of block A. **Pass:** every applicable item passes, or its failure is recorded for a widget issue (w4-8); screenshots of every popout tab and the desktop widget.
- **Block E, device-direct (CAM-36; daemon stopped):** stop `obsbot4linux.socket` and `.service` and check that no `obsbot4linuxd` runs; `OBSBOT_LIVE=1 OBSBOT_LIVE_WRITE=1 <live exe> --ignored --nocapture --test-threads=1 live_cam_5_every_writable_leaf_reads_back_and_its_lag`; start the socket again. **Pass:** `zoom.factor` and `zoom.field_of_view` each read back awake, or their failure shows `asleep` false at the FAIL (then the coupling of the two is the finding); the test restored what it wrote.
- **Block F, second writer (CAM-18, #329, U-19; may be skipped, decision 7):** the camera passed to the vendor app on a second machine, a USB capture on every bus of the host, not on one hub. CAM-18: save three presets, delete the middle one in the vendor app, capture the frames; back on Linux, `get presets`: which preset changed. #329: the vendor app stores an initial state with automatic zoom; back on Linux with the candidate, `set zoom.factor=1` and `set zoom.field_of_view=wide`, then `do zoom.move factor=1 speed=medium` (decision 5). U-19: in the vendor app's initial-state dialog, a second value for each of the ten `power_on.settings` leaves, read the block from Linux after each. Restore the owner's initial state at the end. **Pass:** the capture holds the delete frames (or the block says that it does not and why); #329 reproduced or not, with the zoom path's result; the U-19 values recorded.
- **Block G, destructive, last (CAM-9, `--destructive` rows; owner go):** save `status --json`, `get presets` and the views first. CAM-9: `do sleep`, wait for `asleep:true`, `do factory_reset confirm=true`; does the device return, and under the same `sn:` id, within 60 s; then CAM-6 once (does `led.blink` survive the reset) and CAM-1 after the reset if the live test can run. Then `scripts/check-model.sh --device <id> --destructive --results <file>` in the owner's terminal (the microphone rows and the factory reset row; the camera restarts for each). Put the settings back from the saved JSON and re-save the presets the owner wants. **Pass:** CAM-9 answered; every destructive row has a result; settings and presets restored.

**CS-2 is complete** when blocks A to E and G have run and each item has a recorded answer; a skipped item (block F, or a step of another block) is named in the results summary as skipped, with the reason, never guessed.

### Task w4-7 (key `w4-7-camera-free-fixes`): #328 and other camera-free fixes

**Gate:** none. #328's fix is in progress on its own branch from `80b64e1` (the error names the pose it read; asleep handling as its pull request states). It lands on `rebuild` through its own pull request and review, independently of CS-2; because it changes shipped content, it makes `rc.2` necessary (decision 2). Any other camera-free fix that reaches `rebuild` before w4-9 rides in the same candidate; none is started for W4 beyond the issues named here.

### Task w4-8 (key `w4-8-cs2-results`): CS-2 results

**Gate:** none (after w4-6). One issue and one pull request per finding; a contract change goes first as its own pull request (spec 10.1).

- [ ] **Step 1: The record.** `docs/field-ledger.md` (each answer `measured <date>`, CAM-5 lags per leaf, asleep results), spec 14.1 (CAM-9, 18, 24, 27, 28, 33, 36 and the items CS-1 left open that CS-2 answered), spec 9.3/9.2 if a row changed. Docs only: not shipped content.
- [ ] **Step 2: Each failed row**, per section 2 and Q2: a description fix (contract-change pull request; `levels.rs` regenerated with w3-3's generator, completions with `scripts/regenerate.sh`) or an exception PX-n in 9.2 and in the CHANGELOG's exceptions; a migration-notes fix from block A; a widget issue per failed M2 item (in the widget repository, on `develop`). #329 per decision 5; #277 closed with the numbers if block B passed.
- [ ] **Step 3: The tracking table.** Run w4-1's join over block B's and G's results files and the block results; no row may stay `unmapped` or `not run` unless its block was skipped by name. The table goes into the release pull request (w4-11).

### Task w4-9 (key `w4-9-rc2`): Candidate `v1.0.0-rc.2`

**Gate:** owner go (a tag publishes). **Only if** `git diff --stat v1.0.0-rc.1 origin/rebuild` names a file outside `CHANGELOG.md`, `site/`, `docs/` (decision 2); otherwise skip this task and w4-10.

- [ ] **Step 1:** Version `1.0.0-rc.2` in `Cargo.toml` (`[workspace.package]`), `Cargo.lock` (the four workspace crates), `pkgver=1.0.0rc.2` in `packaging/arch/PKGBUILD`; CHANGELOG per decision 4 (the section renamed, the changes since rc.1 added inside it, the "release candidate" paragraph kept).
- [ ] **Step 2:** The checks of w3-9 Step 3 (`changelog-section.sh v1.0.0-rc.2`, the service's `ExecStart` and `NoNewPrivileges` lines, the workspace tests, `check-units.sh`); pull request into `rebuild`, `required` and the `release.yml` dry run green; landed fast-forward.
- [ ] **Step 3:** Owner go; signed tag `v1.0.0-rc.2` on the landed commit; wait for the `Release` run (up to 90 minutes, in the foreground, polling); the checks of w3-9 Step 6 (pre-release, 9 assets, checksums, listing equal to `packaging/contents.txt`, the service of every package starts `obsbot4linuxd`).

### Task w4-10 (key `w4-10-cs2b`): CS-2b on `v1.0.0-rc.2`

**Gate:** camera. Install the new candidate's Arch package on the desktop (`systemctl --user daemon-reload`, the socket restarts the daemon); the smoke (status, one set and read back, one gimbal move, `get presets`), CAM-27 (block A step 3 again, including the re-plug), and the 9.1 rows and CAM items that the diff `v1.0.0-rc.1..v1.0.0-rc.2` reaches (decision 2; #328 at least: `do power_on.use_current` awake at zoom 1× and asleep). **Pass:** each re-checked row passes, its result replaces the rc.1 result in the tracking table, marked with the candidate it was verified on.

### Task w4-11 (key `w4-11-release-pr`): The release pull request

**Gate:** none to open; it lands only after the owner has read it (the go of w4-12 covers both).

- [ ] **Step 1:** Version `1.0.0` (`Cargo.toml`, `Cargo.lock`, `pkgver=1.0.0`); CHANGELOG: the candidate's section becomes `## [1.0.0] - <date>`, the "release candidate" paragraph goes, what changed since the last candidate is in the section; the rollback line names the 0.14.0 packages and widget 0.29.2; `## [Unreleased]` empty above it. The `rebuild` exception stays (decision 3).
- [ ] **Step 2:** `scripts/changelog-section.sh v1.0.0` prints the whole section (migration notes and rollback line included); `git diff --stat <last candidate tag> HEAD` names only `Cargo.toml`, `Cargo.lock`, `packaging/arch/PKGBUILD`, `CHANGELOG.md`, `site/`, `docs/` (Review Focus 1: anything else stops the task and goes back to w4-9).
- [ ] **Step 3:** The description: the tracking table of w4-8 Step 3 (every 9.1 row: verified on which candidate, or exception PX-n), the CAM items of W4 with their answers (CAM-9, 24, 27, 28; CAM-26 skipped per Q1), the audit list of w4-4, the open issues that do not block 1.0. `required` and the `release.yml` dry run green.

### Task w4-12 (key `w4-12-release`): Release `v1.0.0`

**Gate:** owner go.

- [ ] **Step 1:** Land w4-11 on `rebuild` (fast-forward, signed head). `git merge-base --is-ancestor origin/main origin/rebuild` (spec 10.1: `HEAD..origin/main` empty; if `main` moved, rebase `rebuild` onto it first and run the checks again).
- [ ] **Step 2:** Fast-forward `main` to the landed commit (the repository's guarded push), then `git tag -s v1.0.0 -m 'obsbot4linux 1.0.0' <sha>` and push the tag; wait for the `Release` run in the foreground (up to 90 minutes).
- [ ] **Step 3:** Check what was published: not a pre-release, 9 assets, checksums `OK`, listing equal to `packaging/contents.txt`, every package's service starts `obsbot4linuxd` (w3-9 Step 6's loop), the release notes carry the migration notes and the rollback line, the project page deployed (the `pages` job ran) and shows 1.0. The maintainers' private mirror has the tag (its next sync, or a sync triggered by hand).
- [ ] **Step 4:** Close the milestone's issues per decision 11 (comment: the pull request on `rebuild` and "released in 1.0.0"), then the milestone.

### Task w4-13 (key `w4-13-widget-release`): Widget 1.0 (R1), the same day

**Gate:** owner go (the same release day). **Repository:** the widget's.

- [ ] **Step 1:** Rebase `release/1.0.0` (w4-5) onto `develop`, fill the date, open the "Release 1.0.0" pull request into `develop`; green (its `e2e` job now finds the tool's tag `v1.0.0`); land fast-forward.
- [ ] **Step 2:** A pull request from `develop` into `main` (its head is the release commit); green; fast-forward `main`; the release workflow's dry run; signed tag `v1.0.0`; check the published release.
- [ ] **Step 3:** `dms plugins update` on the desktop moves the plugin from `develop` to `main` at `v1.0.0` (the plugin's clone switched to `main` first), shell reloaded, the widget shows the camera. Then the plugin registry entry (Q3).

### Task w4-14 (key `w4-14-after-release`): Remove the `rebuild` path

**Gate:** owner go (deletes a branch and a ruleset).

- [ ] **Step 1:** Pull request into `main`: `release.yml` job `verify` loses the `tag:v1.0.0-rc.*` case and its comment; `ci.yml` loses `rebuild` from `push` and `pull_request` `branches:`. Check with w3-7's guard table: a `v1.0.0-rc.1`-style tag on a commit only on `rebuild` is refused; `v1.1.0-rc.1` on `main` is accepted. Landed fast-forward on `main`.
- [ ] **Step 2:** Delete the `rebuild` ruleset, then the `rebuild` branch (spec 10.1); the widget's `develop` stays (its own decision). Remove the W-stage worktrees and their build directories.

---

## Acceptance of W4 (spec 10.3), mapped

| Acceptance with camera                                     | Where                                                                      |
| ---------------------------------------------------------- | -------------------------------------------------------------------------- |
| CAM-28, upgrade rehearsal from the 0.13 state              | w4-6 block A (1), (2); notes fixes in w4-8                                 |
| CAM-27, the hardened unit starts, sees the camera, hotplug | w4-6 block A (3); again in w4-10                                           |
| CAM-24, every row of 9.1 on the real unit                  | w4-6 blocks B, C, G and the `cli:`/`block:` rows; the table of w4-8 Step 3 |
| CAM-9, factory reset while asleep                          | w4-6 block G                                                               |
| CAM-26, two cameras                                        | skipped (Q1); the fakes cover it                                           |
| every row verified or an exception of 9.2                  | w4-8 Step 2 and 3; w4-11 Step 3                                            |
| a failing row is fixed and a new candidate built           | w4-8, w4-9, w4-10 (decision 2)                                             |
| widget 1.0 against the candidate                           | w4-6 block D                                                               |
| the release follows 10.6                                   | w4-12, w4-13, w4-14                                                        |

## Camera items in W4

- **Gating 1.0:** CAM-9, CAM-24, CAM-27, CAM-28 (spec 10.3), and every row of 9.1.
- **Answered in W4 if they show, gating nothing on their own:** CAM-5 (lags; decision 8), CAM-2 and CAM-22 leftovers (asleep rows of block B), CAM-33 with a moving person (a leaf without visible effect gets `no_effect: true` by a contract change only if a candidate is cut anyway; otherwise after 1.0), CAM-36 (block E), CAM-6 and CAM-1 after the reset (block G), CAM-18 and U-19 (decision 7), CAM-21 and CAM-29 only if they happen.
- **Not in W4:** CAM-11, CAM-12 (need a recording first, after 1.0), CAM-26 (Q1), CAM-31, CAM-32 and the T items (other models or after 1.0).

## Self-review

1. **Spec coverage.** 10.3 W4 content and acceptance: mapped above. 10.6 release day: steps 1 (done), 2 (w4-6 to w4-10), 3 (w4-12), 4 (w4-13), 5 (the rollback line, checked in block A and w4-11); "done": w4-4 Step 2 and w4-11. 10.1: w4-12 Step 1, w4-14. 10.7: w4-14 (every later candidate from `main`). 12.2: block A follows it. 8.6: the supported-models line stays (firmware from the CS-2 unit; a different firmware is added, not replaced). The W3 hand-over (w3-9 Step 7): item 1 in w4-11 (with decision 4 for further candidates), item 2 in w4-14 (decision 3 changes where, and why), item 3 in w4-3.
2. **Placeholders.** Values not known today are named with their source: the dates of the tags, the device id (`devices --json`), CS-2's results, the number of further candidates.
3. **Owner's lines.** No task adds logic to daemon, model or command line; fixes follow section 2; no agent pushes to `rebuild` or `main` or tags; every publishing, desktop-changing or destructive step carries the owner's go; the answers to `check-model.sh`'s questions are the owner's.
4. **Public text.** This plan names no vendor application, private path, file or serial; the session details stay in the private runbook.

## Notes for the coordinator

1. **Start now, without the camera:** w4-0 (this pull request), then w4-1, w4-2, w4-3, w4-4, w4-5 in parallel (disjoint files, see "In parallel"); #328's pull request (w4-7) continues as it is.
2. **CS-2 needs the owner for about four to five hours** (estimate, not measured), best in two sittings: A, B, C (about three hours) and D, E, F, G (about two). Block B's `check-model.sh` run is the longest single step.
3. **Review by risk:** w4-3 a text review (public-text rule, spec 8.6); w4-8's contract changes an Opus review each; w4-9 and w4-11 the release checks of w3-9; w4-14 the forge lens and the security check (it changes who can publish), as w3-7 had.
4. **Spec text:** CS-2's answers go into 14.1 by w4-8; decision 3 departs from the W3 plan's hand-over (not from the spec); decisions 2 and 4 refine 10.3 and 10.6 without changing them. If the owner wants decision 2 in the spec, it is a 15.1 row.
