# obsbot4linux-dux 1.0, stage W1 track D (widget) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Widget 1.0 of `hellwand/obsbot4linux-dux` speaks protocol 2 to the obsbot4linux 1.0 daemon on its Unix socket: `hello`, a device picker, every control gated on a path or action of the selected device's description, enum values, bounds, step, unit and list limits taken from the description, `pending` values shown, live topics only while the popout is open, markers for `untested` and `unsupported` devices, the hint when no daemon listens, and the defaults reset as one patch of `default` values. Each control sends the one request the user made and chains none. Before that, the last 0.x release states its upper bound in `plugin.json` too.

**Architecture:** All logic moves into plain ECMAScript modules under `lib/` (`.mjs`): the protocol, the connection state machine, the device store, the description helpers, the value rules of protocol 2 rule 4, and one `Link` class that ties them together and returns effects instead of doing I/O. The same modules run in the shell's QML engine and in node: their tests run in both (`scripts/check-js.sh`), and an end-to-end test drives `Link` over a node socket against `obsbot4linuxd --fake fake-a,fake-b,unknown` in CI. In QML, one component `CameraLink.qml` carries out the effects with Quickshell's `Socket` and two timers; three bound controls (`SettingToggle`, `SettingChips`, `SettingSlider`) turn a description path into a control; the cards keep their layout and bind each control to its 1.0 path. Nothing in the widget knows a model: its label table only names values it knows, and the description decides what is offered.

**Tech Stack:** QML (Qt 6.12 as shipped by Arch; the shell's Quickshell runtime with `Quickshell.Io` `Socket` and `SplitParser`), ECMAScript modules (`.mjs`) loaded by QML `import "lib/x.mjs" as X` and by node, Node.js 20 or newer (`node:test` for the e2e test only), Qt 6 `qml`, `qmllint` and `qmlcachegen` from `/usr/lib/qt6`, shellcheck, GitHub Actions (Arch container and `ubuntu-24.04`), Rust/cargo only to build the fake daemon in the e2e job.

**Spec:** `docs/superpowers/specs/2026-11-01-obsbot4linux-1.0-design.md` of `hellwand/obsbot4linux` (revision 2, approved 2026-11-13, with section 15.1), sections 1 (guarantee A), 2, 4.1, 4.2, 4.4, 5.4, 5.10, 6.2 (relay), 7, 8.4, 8.6, 10.3 (W0 and track D), 10.6, 11, 12.3, 14. W0 plan `docs/superpowers/plans/2026-11-13-obsbot4linux-w0-contracts.md` (the generated description files and golden transcripts this plan reads).

## Global Constraints

- "A request sends exactly what it names … Clients follow the same rule: the CLI and the widget never chain a second request to the one the user made" (spec 2). "Each control sends the one request the user made and chains none (section 2): the gimbal controls send `gimbal.move` or `gimbal.turn` alone, never a tracking switch before them" (spec 12.3).
- "Saving a preset sends no name unless the user typed one. A control may show a placeholder of its own for a preset whose `name` is empty, as pure display; it never writes that label to the camera" (spec 12.3).
- "Clients show their own value from sending a request until its reply arrives, and the state (its `pending` values included) after that. They run no timers of their own" (spec 4.1 rule 4). "Widget 1.0 settles values by `pending` and `rev`; this replaces its own hold of up to 3 s (obsbot4linux-dux#89)" (spec 12.3).
- "Clients ignore unknown fields and unknown event types, show unknown enum values as raw text, show settings of an unknown type read-only as text, and treat an unknown error code as `internal`" (spec 4.1 rule 7).
- "Reconnect with a backoff from 0.5 s, doubling up to 5 s … After any reconnect a client drops every pending request and every own value, sends `hello` and rebuilds from its reply. Revs from two connections are never compared" (spec 4.1). "Clients forget the rev of an id when a `devices` event no longer lists it" (rule 1).
- Socket: "`$XDG_RUNTIME_DIR/obsbot4linux.sock` … the environment variable `OBSBOT4LINUX_SOCKET` overrides the path"; "The first message on a connection must be `hello`" (spec 4.1). `CLIENT_REPLY_TIMEOUT` 30 s (spec 5.8).
- "A client that cannot connect (no socket file, connection refused) exits 4 and prints `systemctl --user enable --now obsbot4linux.socket`"; older daemon: "an obsbot4linux daemon older than 1.0 is running: restart it or log out"; "Clients never stop or restart a daemon" (spec 5.10).
- Track D content: "every control gated on a path or action of the description, enum values, bounds, step, unit and list limits taken from the description, `pending` values shown, live topics only while the popup is open, markers for `untested` and `unsupported`, the exit-4 hint, and the defaults reset built from `default` values (a `set` patch; which controls it covers is the widget's product choice)" (spec 10.3 D).
- Unsupported device: "The widget lists it in its device picker with the marker `unsupported`, shows no controls for it, and shows the hint to run `obsbot4linux devices --report`"; on unplug "The widget drops the entry; if it was selected, the widget selects the only usable device, or none" (spec 8.4 steps 3 and 7). "Unsupported devices never count as usable" (spec 6.2). "The widget and the desktop app show a marker for `untested` and `unsupported` devices" (spec 8.6).
- Guarantee A: hand-made UIs "gate every control on the device description and take values, bounds, step, unit and list limits from it, never from constants of their own … they keep working for the settings they know and need an update only to show settings that are new to them. 1.0 has no generated UI" (spec 1).
- Widget 1.0: "Its `plugin.json` requires obsbot4linux ≥ 1.0"; "Widget 1.0 is released after the tool, the same day"; the last 0.x release with its upper bound is out first (spec 10.6).
- No camera I/O, no camera needed: every acceptance item of this plan runs against fixtures, the node e2e test against `obsbot4linuxd --fake fake-a,fake-b,unknown`, or the coordinator's manual check list against the same fake daemon (spec 10.3 D "Acceptance with camera: in W4").
- Repository rules (obsbot4linux-dux): one issue per task, the pull request says `Fixes #N`; every pull request adds its line(s) to `CHANGELOG.md` under `## [Unreleased]`; a release only through the tag workflow (`README.md` "Releasing"); the gate is the `required` check of `.github/workflows/ci.yml`.
- Local gate of every task: `bash scripts/check-qml.sh` (Qt 6 `qmlcachegen` and `qmllint` from `/usr/lib/qt6`; `/usr/bin/qmllint` is Qt 5 and is never used), `scripts/check-js.sh` (from Task 2 on), `shellcheck scripts/*.sh`.
- Public text (spec 8.5): code comments, CHANGELOG, README and issues describe the widget's behaviour and what was recorded on the owner's device; no statement of where knowledge comes from beyond what was recorded on the owner's device, no private names, hosts or paths, no product ids of cameras the project has no recordings of. The fake ids (`sn:FAKEA0000001`, `sn:FAKEB0000001`, `port:9-3`, products `fa0a`, `fa0b`, `fa00`) and `EXAMPLE0000001` are synthetic and allowed.
- Owner's layout rule for this widget: cards stack per column without row gaps; a mock-up always shows the complete page. New rows (the camera picker, Metering, the auto-sleep time) keep it (manual item M-23).
- Modules use only what both engines have: the QML engine lacks node-only globals such as `structuredClone` (measured: `structuredClone is not defined` in the `qml` run while node passes); `scripts/check-js.sh` fails on such a use.
- Formatting: the files of this plan were formatted by the plan author's editor hook as shown; keep them byte for byte. The generated files `tests/fixtures/*.description.mjs` and `tests/fixtures/*.transcript.mjs` are written only by `scripts/sync-fixtures.mjs`; a formatter must not touch them (the e2e job's `--check` compares them byte for byte).

## Review Focus

Inputs and failure modes most likely to bite a user, each with the test that pins it (already in the owning task):

1. A slider released between steps or with float noise (`0.1 + 0.2`): the value sent must be the stepped value rounded to the description's precision, inside the bounds. Test "a slider scale maps positions to stepped values" (Task 4); the W0 validator accepted the lines (proof, below).
2. The defaults reset while white balance is manual: `image.white_balance.temperature` requires `image.white_balance.auto=false`, so a patch that included it next to `auto: true` could be refused as a whole with `conflict`. Test "the defaults reset patch" (Task 4) leaves leaves with `requires` out; the W0 validator accepted the resulting patch against fake-a (proof).
3. A daemon older than 1.0 still answering on the socket after the upgrade (spec 5.10): the widget must say so, not show "no camera" or spin. Tests "a 0.13 daemon's answer to hello is an older daemon" (Task 3) and "an older daemon is reported and retried at the slowest pace" (Task 3).
4. Two cameras, or an unsupported one next to a supported one: nothing may be picked for the user when two are usable, and an unplugged selected unsupported device must hand over to the only usable one. Tests "selection: …" (Task 4), "unplugging the selected unsupported device" (Task 5), e2e "unplugging the selected unsupported device" (Task 7).
5. A value the widget has no name for (`raw:9`, a newer enum value, fake-b's `off|person|desk`): chips must offer the description's values and show unknown ones as text, never fall back to the Tiny 2 Lite list. Tests "choices come from the description …" (Task 4), "values by type; raw and unknown types as text" (Task 4), "the description decides the chips; the labels only name them" (Task 4), e2e "fake-b: …" (Task 7).

---

## What track D needs from W0 and the other tracks

From W0 (landed on `rebuild` of `hellwand/obsbot4linux`; the W0 plan's code, built in a scratch copy for this plan, 238 tests green):

- The generated files, read by `scripts/sync-fixtures.mjs` at these paths: `descriptions/tiny2lite.json`, `descriptions/fake/fake-a.json`, `descriptions/fake/fake-b.json` (each `{"settings":{…},"actions":{…},"limits":{…}}`), and the golden transcripts `crates/obsbot-proto/tests/golden/session.jsonl`, `errors.jsonl`, `hotplug.jsonl` (lines `> {…}` client to daemon, `< {…}` daemon to client, `#` comments).
- The protocol shapes the modules read (W0 Task 1, `obsbot-proto`): `Device = {id, name, model, support, serial, firmware, video_node, available, replaced_by?, usb{vendor, product, product_name, bcd_device, path, serial_present, interfaces, extension_units}, settings, actions, limits}`; `Setting = {type, label, readonly, nullable, readback, asleep?, no_effect, values?, writable?, min?, max?, step?, unit?, precision?, max_len?, ascii?, item?, optional?, requires?, default?, live?, settle_ms?, warning?}` (bounds, step, unit and precision are a number/string or an object per component); `Action = {label, params, result?, asleep, requires?, warning?}`; state = nested tree plus `available`, `pending` (flat map), `unread` (leaf paths); replies `{"id","ok":{…}}` / `{"id","error":{"code","message",…}}`; events `{"event":"state","device","rev","state"}`, `{"event":"devices","devices":[…]}`.
- The fakes (W0 Task 9): ids `sn:FAKEA0000001` (fake-a, `support: tested`, the Tiny 2 Lite's description), `sn:FAKEB0000001` (fake-b, `untested`: `tracking.mode` values `off|person|desk`, `zoom.factor` 1–2, `led.brightness` 0–5 with a 250 ms readback lag, presets 0–4 with `limits.presets` 5, no `gimbal`, no `views`, an unknown group `beacon.*`), `port:9-3` (unknown, `unsupported`); USB paths `9-1`, `9-2`, `9-3`.

From track B, milestone **B-1** (daemon core with fake mode), for Task 7 and the manual checks only:

- `obsbot4linuxd --fake fake-a,fake-b,unknown` binds `$OBSBOT4LINUX_SOCKET` when it is set (spec 11: "Without `LISTEN_FDS` the daemon binds `OBSBOT4LINUX_SOCKET` or the default path"), answers protocol 2 as spec 4.1 says, and opens the test-control socket `<socket>.test` (spec 5.12), one JSON line in, one out: `{"plug":"<model>"}` → `{"ok":"<id>"}`, `{"unplug":"<id>"}`, `{"sleep":"<id>"}`, `{"wake":"<id>"}` → `{"ok":true}`, an error → `{"error":"<message>"}` (W0 decision 8, `FakeBus::handle_line`).
- It builds from the workspace root with `cargo build --locked -p obsbot4linuxd`; the binary is `target/debug/obsbot4linuxd`.

From track C: nothing for the build. The manual check M-7 uses `obsbot4linux set` when C has landed (a raw `socat` line works without it). Only if pre-flight P1 fails: `obsbot4linux relay` (spec 6.2 "Relay fallback": stdin lines to the socket, socket lines to stdout, unchanged).

From the tool's release: `v1.0.0-rc.1` before Task 16 lands (its startup check refuses the 0.13.2 dev builds), `v1.0.0-rc.N` for M2 in W4, and `v1.0.0` before R1 (spec 10.6 step 4).

**Contract changes:** none. The widget repository holds no W0 contract file; this plan only reads them.

## Coordinator steps outside the task list

These need the owner's desktop, repository settings or a release tag, which builders do not touch.

**P1, pre-flight on the desktop (before Task 6 starts).** Three facts the QML side rests on and that plain `qml` cannot prove inside the shell: (1) a plugin file can `import "lib/x.mjs" as X`; (2) `Quickshell.env("XDG_RUNTIME_DIR")` and `Quickshell.env("OBSBOT4LINUX_SOCKET")` as the shell sees them; (3) a plugin's `Quickshell.Io` `Socket` connects to a Unix socket, writes a line and reads one whole line of 256 KiB. The probe is a throw-away daemon plugin (written and compiled with Qt 6.12 `qmlcachegen` for this plan; never committed):

```json
{
  "id": "obsbot4linuxProbe",
  "name": "obsbot4linux socket probe",
  "description": "One-off check for the obsbot4linux-dux 1.0 plan: socket, environment and .mjs imports in the shell",
  "version": "0.0.1",
  "author": "obsbot4linux-dux",
  "type": "daemon",
  "component": "./Probe.qml",
  "icon": "science",
  "permissions": []
}
```

```javascript
// The module half of the probe: proves that a plugin file can import an ES module from lib/.
export function hello(n) {
  return JSON.stringify({ probe: n, from: "obsbot4linuxProbe" }) + "\n";
}
```

```qml
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Services
import qs.Modules.Plugins
import "lib/probe.mjs" as ProbeModule

// Answers three questions for the widget 1.0 plan, then stays idle:
// 1. does `import "lib/probe.mjs"` work in a plugin file;
// 2. what Quickshell.env gives for XDG_RUNTIME_DIR and OBSBOT4LINUX_SOCKET;
// 3. can a plugin connect a Quickshell.Io Socket to a Unix socket, write a line and read one.
PluginComponent {
    id: root

    readonly property string runtimeDir: Quickshell.env("XDG_RUNTIME_DIR") ?? ""
    readonly property string probePath: runtimeDir + "/obsbot4linux-probe.sock"

    function report(text) {
        console.info("obsbot4linuxProbe: " + text);
        ToastService.showInfo("obsbot4linux probe", text);
    }

    Component.onCompleted: {
        report("module: " + (typeof ProbeModule.hello === "function" ? "ok" : "missing") + "; XDG_RUNTIME_DIR=" + (runtimeDir !== "" ? runtimeDir : "(unset)") + "; OBSBOT4LINUX_SOCKET=" + (Quickshell.env("OBSBOT4LINUX_SOCKET") ?? "(unset)"));
        socket.path = probePath;
        socket.connected = true;
    }

    Socket {
        id: socket
        parser: SplitParser {
            // the line's length, not the line: the reply is 256 KiB long
            onRead: line => root.report("socket read: " + line.length + " characters, " + line.slice(0, 20) + "…")
        }
        onConnectionStateChanged: {
            if (connected) {
                write(ProbeModule.hello(1));
                flush();
                root.report("socket connected to " + root.probePath + ", line written");
            } else {
                root.report("socket closed");
            }
        }
        onError: error => root.report("socket error " + error + " (0 refused, 2 not found)")
    }
}
```

Steps on the desktop (one shell restart allowed):

1. Copy the three files into `~/.config/dms/plugins/obsbot4linuxProbe/` (with `lib/`).
2. In a terminal, start a line echo server: `python3 -c 'import os,socket; p=os.environ["XDG_RUNTIME_DIR"]+"/obsbot4linux-probe.sock"; os.path.exists(p) and os.remove(p); s=socket.socket(socket.AF_UNIX); s.bind(p); s.listen(1); c,_=s.accept(); print("got", c.recv(4096)); c.sendall(b"{\"id\":1,\"ok\":{\"pad\":\"" + b"x"*262144 + b"\"}}\n"); input("enter to end")'` (its reply is one line of 262168 characters: lines from the daemon have no length limit and a client must not cap them (spec 4.1); a hello reply with the Tiny 2 Lite's description is tens of KB in one line)
3. `dms restart`, then enable the probe: `dms ipc call plugins enable obsbot4linuxProbe` (or Settings → Plugins).
4. Expected toasts and log lines: `module: ok; XDG_RUNTIME_DIR=/run/user/<uid>; OBSBOT4LINUX_SOCKET=(unset)`, then `socket connected … line written`, then `socket read: 262168 characters, {"id":1,"ok":{"pad":…` (a shorter count means the line was cut: (3) fails); the terminal prints the probe's line. Then disable and delete the probe directory.

Results go into the track D tracking issue. If (1) fails: the modules are inlined into one `.js` file per QML import instead (Task 6 stops and the plan is revised). If (3) fails: track C builds `relay` (spec 6.2) and Task 6 replaces the `Socket` in `CameraLink.qml` by a `Process` running `obsbot4linux relay` with `stdinEnabled: true` and a `SplitParser` on stdout; `lib/` does not change, because `CameraLink.qml` is the only file that does I/O (the seam).

**P2, the update channel: answered from source, no desktop step.** Read in `duxlabs-dev/dux-shell` at `ea78a8d2` (the shell installed on the owner's desktop, `dux-shell-git 2.1.0.r113.gea78a8d`): `dms plugins install` runs a go-git `PlainClone` of the plugin's repository (its default branch), and `dms plugins update` runs `worktree.Pull` with default options on that clone (`core/internal/plugins/registry.go`, `realGitClient.Pull`; `manager.go`, `Update` and `UpdateByIDOrName`). Tags play no part. The plugin's `dependencies` are only looked up as commands in the plugin browser (`PluginBrowser.qml`, `checkDependencies`; `PluginRanking.js`, `dependencyCommands`): an entry with a space or a comma, such as `obsbot4linux >= 0.7.3` today or `obsbot4linux >= 0.7.3, < 1.0`, matches no command name and shows as "unknown"; no version range is enforced. So **whatever lands on the widget's `main` reaches every user at their next update**, and the effective upper bound of a 0.x widget is its `StartupCheck.qml` (already in 0.28.0 and later). Consequence: the 1.0 work must not land on `main` before release day (decision 1).

**R0, the last 0.x release (after Task 1 lands on `main`).** "Release 0.29.2" through the repository's procedure (a release pull request that moves `## [Unreleased]` into `## [0.29.2] - <date>` and sets `plugin.json` `version`, the dry run, the signed tag). Spec 10.6 calls it "widget 0.28.0"; that version and 0.29.0/0.29.1 are already released with the refusal of obsbot4linux ≥ 1.0 in their startup check, so what is left is only the string in `plugin.json` (decision 2).

**B0, the integration branch (after R0).** Create `v1` in `hellwand/obsbot4linux-dux` from `main` and give it a ruleset like `main`'s (signed commits, the `required` check, fast-forward only, no deletion). Tasks 2–16 branch from and land on `v1`. CI already fires for pull requests into `v1` (Task 1).

**M1, manual check list (after wave 8, before Task 16).** On the desktop against the fake daemon; section "Manual check list" at the end.

**R1, release day (spec 10.6 step 4, after the tool's `v1.0.0`).** A "Release 1.0.0" pull request into `v1`: `plugin.json` `version` 1.0.0; `## [Unreleased]` becomes `## [1.0.0] - <date>` with a first line "Needs obsbot4linux 1.0 (protocol 2). Update the widget together with the tool, enable the daemon's socket once (`systemctl --user enable --now obsbot4linux.socket`) and restart the shell." and a new empty `## [Unreleased]` above it; the `e2e` job's checkout of the tool uses `ref: v1.0.0` instead of `rebuild` (and its comment says so); `scripts/release-notes.sh HEAD v1.0.0` finds the section. Then a pull request from `v1` into `main` (its head is that release commit, so it is green as a pull request into `main`, as `release-green.sh` requires), fast-forward `main`, dry run, tag `v1.0.0`, update the widget's entry in the plugin registry.

**Landing rule for `CHANGELOG.md`.** Every task adds lines under `## [Unreleased]`. When a stacked or same-wave branch conflicts there, keep both sides' lines (a union), in task order; nothing else in that file conflicts.

## Decisions this plan takes

Engineering decisions, each with its reason. Each can be overruled by the maintainers.

1. **Widget 1.0 lands on an integration branch `v1`, not on `main`.** P2 shows that `dms plugins update` pulls the default branch, so a 1.0 commit on `main` would reach users who still run obsbot4linux 0.13 and switch the widget off. `main` stays releasable for 0.x patches, as the tool's own `main` does (spec 10.1). Cost: Task 1 widens the CI trigger on `main` first; B0 creates the branch and its ruleset; R1 merges by fast-forward. This deviates from the brief, which named the widget's `main` as the target of every pull request.
2. **The last 0.x is 0.29.2, and only `plugin.json` changes.** Spec 10.3 and 10.6 name "widget 0.28.0, the last 0.x release, which refuses obsbot4linux ≥ 1.0 (`plugin.json`: `obsbot4linux >= 0.7.3, < 1.0`)". Released facts: 0.28.0 (#105) already refuses 1.0 in `StartupCheck.qml`; 0.29.0 and 0.29.1 followed; `plugin.json` still says `obsbot4linux >= 0.7.3`. The shell enforces no version range (P2), so the string is documentation. Task 1 adds it; R0 releases it.
3. **Logic in ES modules, tested in two engines.** There is no QML test harness in the repository, and the widget's components import the shell's modules (`qs.Common`, `Quickshell.Io`), so they cannot load outside the shell. Pure logic in `lib/*.mjs` loads in the Qt 6 `qml` tool and in node (measured with Qt 6.12 and node 26: relative imports between modules, optional chaining and template literals work in both; a missing import fails the `qml` run, exit 2). Tests are plain modules with a 25-line harness, run by node and by the `qml` tool offscreen, and both runs must count the same number of passes (`scripts/check-js.sh`). Node's own `node:test` runner is used only by the e2e test, which needs sockets.
4. **One `Link` class returns effects; `CameraLink.qml` only does I/O.** So every rule of protocol 2 that the widget follows is a unit test, the e2e test drives the same class over a node socket, and the `relay` fallback is a one-file change.
5. **Fixtures are copies of the tool's generated files, as modules.** `scripts/sync-fixtures.mjs` writes `tests/fixtures/*.mjs` from a tool checkout; the e2e job checks them with `--check` against the `rebuild` branch, so they cannot drift silently. `tests/fixtures/fakes.mjs` (device facts and first states of the three fakes) is hand-written from what the W0 fakes report; the W0 types parse it (proof), and Task 7's e2e test "the unit tests' fixtures match the fake daemon's hello" compares it with what the running fake daemon sends, so it cannot drift silently either.
6. **The e2e job builds the daemon from the tool's `rebuild` branch and is part of `required` on `v1`.** Spec 10.3 D states the acceptance against `obsbot4linuxd --fake`. A commit hash of `rebuild` is not pinned (it is rebased, spec 10.1); a red `rebuild` turns widget pull requests red until it is fixed, which is accepted for the integration branch. R1 switches the job to the tag `v1.0.0`.
7. **One control, one leaf.** Each bound control writes its own leaf: `exposure.mode` and `exposure.region` become two chip rows (0.29 had one row "Global / Face / Custom" that wrote both), `sleep.auto.enabled` a switch and `sleep.auto.seconds` its own chips, each end of an area range its own slider. A request carries more than one leaf only in the defaults reset (spec 10.3 D: "a `set` patch").
8. **A `requires` that does not hold disables the control and names the need.** The widget never adds the required leaf to the patch (spec 2): the user sets `exposure.mode` to manual first, with its own control. `blocked()` checks the published state, which is what the daemon validates against (spec 4.1 rule 5), not own or pending values.
9. **`asleep: ignored` disables the control while the state says `asleep: true`** (the daemon would answer `asleep`); `accepted` and `unknown` stay usable.
10. **Hidden when the description cannot bound a slider.** A number without `min`, `max` or a step has no slider (the widget has no bounds of its own, guarantee A). Today this hides `hand.return_after` on the Tiny 2 Lite, whose description has no maximum (0.29 offered 0–30 s; see product question 1).
11. **Selection:** the device shown stays while listed (its new id after a hand-over, spec 5.5); otherwise the only usable device, or none (spec 8.4 step 7, applied at start-up too). A lost connection keeps the pick: the store is emptied, so nothing is shown meanwhile, and after the next `hello` the same id is shown again while it is listed (an `sn:` id is stable across daemon restarts; spec 4.1 "Reconnect" drops requests and own values, not the selection). The user picks in the device picker; the pick is not stored across shell restarts (product question 4). "Hide without camera" counts any usable device, selected or not (`anyUsable`), so the pill stays while the user still has to pick.
12. **Fates of 0.29's client-side holds and remembered values** (spec 4.1 rule 4, section 2):
    - `OptimisticState.qml` and its 3 s `patchTimeout`: removed; own values live until the reply (`lib/values.mjs`).
    - `dragHold` (500 ms) and the Zone card's `areaHold` (500 ms): removed; sliders send once on release and show the own value until the reply.
    - `presetTimeout` (8 s "Reading presets…"): removed; the preset list is part of the state, and `unread` says when it was not read.
    - `ledBlink` and `handGimbal` remembered in the plugin settings: removed; these `readback:false` leaves show the state's value, which is `null` ("Not known until it is set here") until written since the device became available (spec 4.2).
    - The optimistic `ai_mode: "static"` on a gimbal move and the text "Moving switches tracking to Static.": removed (spec 2, 15.1 row 88).
    - The 2 s window in which an arrow step builds on the previous own target (`moveTarget`): kept as `STEP_BASE_MS` in `lib/gimbal.mjs`; it picks the base of the request's target and holds no displayed value.
    - The Reset card's 10 s arming timeout: kept; it is a confirmation, not a value.
    - `lastTrackingMode` (the middle click goes back to the last mode other than Static): kept as a widget preference; the middle click sends one `set tracking.mode` and works only when the description's `writable` holds both `static` and the remembered mode.
13. **Live topics:** while the popout is open, `pose` while the Framing card is enabled, `target` while the target box switch is on, `auto` while the Picture card is enabled, each only if the device's description names it; none while the popout is closed, none on the desktop widget (spec 10.3 D: "live topics only while the popup is open").
14. **Pill and summary words follow one meaning per value** (spec 4.2 "One meaning per path"): the pill says "Static" for `tracking.mode=static`, "Tracking" for another value the widget has a name for, and the value's own text for one it does not know (fake-b: "Off").
15. **Problem texts** (no daemon, older daemon, other protocol, lost) are in `lib/labels.mjs`, with the exact `systemctl --user enable --now obsbot4linux.socket` line, shown in the header, the preview frame and the pill tooltip.
16. **No new controls.** Widget 1.0 offers the controls 0.29 offers, on their 1.0 paths; settings 0.29 did not show (focus, anti-flicker, white balance, hue, the microphone switch, framing details) stay out (guarantee A: an update shows settings new to the widget; that is not this track). Two shortcut lists stay as the widget's product choice, both inside the description's bounds: the auto-sleep time chips 30 s, 2 min, 10 min (the label shows the camera's value when it is none of them) and the pickup distance chips 10 / 50 / 100 (spec 4.3, 9.1).

## Product questions for the coordinator

Not built in; the plan builds what the spec says, and each line names the recommendation.

1. `hand.return_after` has no maximum in the Tiny 2 Lite's description, so widget 1.0 hides the "Return after" slider (0.29 capped it at 30 s on its own). Recommendation: keep it hidden; if CS-1 keeps U-6, track A adds a measured maximum to the description and the slider comes back with no widget change.
2. Live topics on the desktop widget: the spec allows them only while the popout is open, so the desktop widget's target box moves only with ordinary events (about once a second while the camera tracks). Recommendation: keep the spec's rule for 1.0; revisit if the owner misses a smooth box on the desktop.
3. The Picture card's exposure row splits into "Exposure: Auto / Manual" and "Metering: Face / Global" (decision 7). Recommendation: accept the split; it is the only way to keep one control per leaf.
4. The camera pick is kept across reconnects but not across shell restarts: with two cameras the user picks again after a restart. Recommendation: do not store it for 1.0 (one camera is the common case and is chosen by itself); if the owner wants it, store the `sn:` id as a widget preference, like `lastTrackingMode`.
5. 0.29 disabled "Tracked hand" and showed "Return here" only in Hand mode, and enabled "Go to return position" and "Clear" only with a stored return position. 1.0 gates them only on the description and the state's `blocked()` (spec 2), with "Used in Hand mode" as display text; the hand area, axes and gimbal switches (`HandControls`) stay shown in Hand mode only, as 0.29's layout. Recommendation: accept (spec 2: the widget exposes what the camera supports; what a press does outside Hand mode is the camera's behaviour, to watch in M2).

## Decisions taken after the plan review (2026-11-13)

The coordinator decided the product questions of the review and the seam points between the tracks. Where a line below differs from the text of this plan, the line wins; the code of the plan is otherwise unchanged. The product questions above are answered by these lines.

1. **d-4.** Question 5, follow-on: `blocked()` reads the state with `pending` laid over it. Task d-4 changes `Describe.blocked` so that a `requires` condition is checked against the pending value of its path when there is one (own values still do not count), and the `asleep` check follows the same state; decision 8's last clause ("not own or pending values") becomes "pending values count, own values do not". Add a test in `tests/describe.test.mjs`: a requirement that is pending gives `null`.
2. **d-13.** Question 8, the widget asks: a `restart` or `destructive` action asks the user before it sends (`Describe.warnings`), as built; the CLI does not. Question 11: the exposure row is split into "Exposure: Auto / Manual" and "Metering: Face / Global" (decision 7), as built. Nothing changes.
3. **d-10.** Question 9: the "Return after" slider stays hidden while the description gives no maximum for `hand.return_after`. Question 13: the tracked-hand buttons are gated only on the description and `blocked()`, without 0.29's own mode checks; watch it in the camera session. Both as built; nothing changes.
4. **d-8.** Question 10: live values (target box, pose) only while the popout is open, as built. Question 16: with two usable cameras and none picked, the bar pill says "Pick camera", not "Offline". Task d-8 changes `pillText` in `ObsbotWidget.qml`: while `link.device` is null and `link.devices` holds a usable device (the case in which `statusText` says "Pick a camera") it returns "Pick camera"; the hidden width texts of `PillContent.qml` gain "Pick camera".
5. **d-9.** Question 12: the picked camera is not remembered across shell restarts, not for 1.0; as built, nothing changes.
6. **d-1.** Questions 14 and 15: the last 0.x widget release is 0.29.2 (0.28.0 to 0.29.1 are released and carry the refusal; 0.29.2 only adds the `< 1.0` bound to `plugin.json`), and widget 1.0 is built on the branch `v1` of the widget repository until release day, because users receive every commit of the default branch at their next plugin update. Task d-1 lands on `main`; the coordinator then makes the release 0.29.2 (R0) and creates `v1` with its ruleset (B0); d-2 and every later task branch from and land on `v1`. The spec (10.3, 10.6) now says so.
7. **d-7.** Gate: B-1 is the task b-10 landed on `rebuild`; d-7 is issued only after it has landed, because its new `e2e` job builds `rebuild` and never skips. d-6 waits for the result of P1 (and for C-R if P1 fails); d-16 is not a W1 task (it waits for the tool's `v1.0.0-rc.1`).

## File structure

All paths are in the widget repository (`hellwand/obsbot4linux-dux`, root = plugin directory).

| File                                                                                                                                           | Responsibility                                                                          | Task        |
| ---------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ----------- |
| `.github/workflows/ci.yml` (modify)                                                                                                            | trigger on `v1`; jobs `js`, `e2e` in `required`                                         | 1, 2, 7, R1 |
| `plugin.json` (modify)                                                                                                                         | dependency bound; 1.0 version and requirement                                           | 1, 16, R1   |
| `CHANGELOG.md` (modify)                                                                                                                        | one or more lines per task                                                              | all         |
| `README.md` (modify)                                                                                                                           | development checks; 1.0 text                                                            | 2, 16       |
| `scripts/check-qml.sh` (modify)                                                                                                                | also compiles and lints `lib/*.mjs`                                                     | 2           |
| `scripts/check-js.sh`                                                                                                                          | module tests in node and in `qml`                                                       | 2           |
| `scripts/sync-fixtures.mjs`                                                                                                                    | copies the tool's generated files into `tests/fixtures/`                                | 2           |
| `tests/harness.mjs`, `tests/assert.mjs`, `tests/run.mjs`                                                                                       | the two-engine test runner                                                              | 2           |
| `tests/fixtures/*.description.mjs`, `*.transcript.mjs` (generated), `tests/fixtures/fakes.mjs`                                                 | fixtures                                                                                | 2           |
| `lib/version.mjs`, `tests/version.test.mjs`                                                                                                    | the startup check's version test                                                        | 2           |
| `lib/protocol.mjs`, `lib/session.mjs`, `tests/protocol.test.mjs`, `tests/session.test.mjs`                                                     | protocol 2 lines; connection state machine                                              | 3           |
| `lib/devices.mjs`, `lib/describe.mjs`, `lib/values.mjs`, `lib/format.mjs`, `lib/gimbal.mjs`, `lib/lists.mjs`, `lib/labels.mjs` and their tests | device store, description helpers, value rules, text, arrows, presets and views, labels | 4           |
| `lib/link.mjs`, `tests/link.test.mjs`                                                                                                          | the link without I/O                                                                    | 5           |
| `CameraLink.qml`, `SettingToggle.qml`, `SettingChips.qml`, `SettingSlider.qml`                                                                 | socket I/O; bound controls                                                              | 6           |
| `tests/e2e/node-link.mjs`, `tests/e2e/fake-daemon.test.mjs`                                                                                    | e2e against the fake daemon                                                             | 7           |
| `ObsbotWidget.qml`, `PillContent.qml`                                                                                                          | popout root on `CameraLink`, the `ctl` interface of the cards, pill, Control Center     | 8           |
| `HeaderBar.qml`, `DevicePicker.qml`                                                                                                            | header, device picker, markers, hints                                                   | 9           |
| `TrackingSection.qml`, `HandControls.qml`                                                                                                      | Tracking card                                                                           | 10          |
| `FramingSection.qml`, `ZoneSection.qml`, `ZoneViews.qml`                                                                                       | Framing and Zone cards                                                                  | 11          |
| `PresetsSection.qml`, `PresetSlot.qml`                                                                                                         | Presets card, desktop preset button                                                     | 12          |
| `PictureSection.qml`, `ImageSection.qml`, `ResetSection.qml`                                                                                   | Picture, Image, Factory reset                                                           | 13          |
| `CameraSection.qml`, `SleepSection.qml`, `AudioSection.qml`, `GestureSection.qml`                                                              | Device tab cards                                                                        | 14          |
| `DesktopPreview.qml`, `DesktopSide.qml`, `DesktopBottom.qml`, `PreviewFrame.qml`; delete `CameraWatch.qml`, `OptimisticState.qml`              | desktop widget, preview frame                                                           | 15          |
| `StartupCheck.qml`, `ObsbotSettings.qml`                                                                                                       | 1.0 startup check; settings page checked                                                | 16          |

## Tasks, waves and dependencies

Tasks of one wave run side by side and touch different files, except `CHANGELOG.md` (landing rule above). A task branches from `v1` (Task 1: from `main`) after its dependencies landed; where one is still in review, it is stacked on that branch.

| Task | Title                                                                      | Wave | Depends on                   | Stacked on            | Base   | Effort |
| ---- | -------------------------------------------------------------------------- | ---- | ---------------------------- | --------------------- | ------ | ------ |
| 1    | CI on `v1` and the 0.x bound in `plugin.json`                              | 1    | –                            | –                     | `main` | S      |
| 2    | Two-engine test harness, fixtures, version module                          | 2    | 1, W0, B0                    | –                     | `v1`   | M      |
| 3    | Protocol lines and the connection state machine                            | 3    | 2                            | 2                     | `v1`   | M      |
| 4    | Devices, description, values, text, arrows, lists, labels                  | 3    | 2                            | 2                     | `v1`   | M      |
| 5    | The link without I/O                                                       | 4    | 3, 4                         | – (after both landed) | `v1`   | S      |
| 6    | `CameraLink.qml` and the bound controls                                    | 5    | 5, P1                        | 5                     | `v1`   | M      |
| 7    | End-to-end test against the fake daemon                                    | 5    | 5, B-1                       | 5                     | `v1`   | M      |
| 8    | Popout root, pill and Control Center on `CameraLink`                       | 6    | 6                            | 6                     | `v1`   | L      |
| 9    | Header and device picker                                                   | 7    | 8                            | 8                     | `v1`   | M      |
| 10   | Tracking card                                                              | 7    | 8                            | 8                     | `v1`   | M      |
| 11   | Framing and Zone cards                                                     | 7    | 8                            | 8                     | `v1`   | L      |
| 12   | Presets card                                                               | 7    | 8                            | 8                     | `v1`   | M      |
| 13   | Picture, Image and Factory reset cards                                     | 7    | 8                            | 8                     | `v1`   | M      |
| 14   | Device tab cards                                                           | 7    | 8                            | 8                     | `v1`   | M      |
| 15   | Desktop widget and preview frame                                           | 8    | 8, 9, 12                     | –                     | `v1`   | L      |
| 16   | Widget 1.0 for the release candidate: startup check, `plugin.json`, README | 9    | 2–15, M1, tool `v1.0.0-rc.1` | –                     | `v1`   | S      |

Wave 7 has six tasks on disjoint files; the coordinator may run them four at a time. Tasks 9–14 depend only on Task 8's `ctl` interface and on Task 6's components (header section "Shared interfaces of the QML tasks"); Task 15 also uses `DevicePicker.qml` (Task 9) and `PresetSlot.qml` (Task 12) and runs in wave 8.

The code of Tasks 2–7 (modules, tests, harness, scripts, `CameraLink.qml`, the bound controls; the e2e files only syntax-checked, as no B-1 daemon exists yet) was written and checked in a scratch copy of the widget (the repository's `main` at `58d807b`) on 2026-11-13 (and again after the review's fix round, against the W0 files as landed on `rebuild` at 3187b84): `scripts/check-js.sh` passed 87 tests in node 26.13.1 and in the Qt 6.12 `qml` tool, `bash scripts/check-qml.sh` passed on all 47 files, `shellcheck scripts/*.sh` passed, `actionlint` passed on the three `ci.yml` versions. The code in the steps is that code.

## Shared interfaces of the QML tasks (Tasks 6–16)

A builder of a QML task sees this section; it fixes every name the cards consume. The modules and components it names are complete in Tasks 3–6.

**Part A, the modules** (Tasks 3–5; a QML file imports one with `import "lib/<name>.mjs" as <Name>`):

- `lib/protocol.mjs`: `PROTOCOL = 2`; `ERROR_CODES: string[12]`; `socketPath(env: (name) => string|undefined) -> string`; `encode(request) -> string` (JSON plus `"\n"`); `hello(id, client, live[]) -> object`; `setLeaves(id, device, patch) -> object`; `act(id, device, action, params?) -> object` (no `with` when `params` is empty); `live(id, topics[]) -> object`; `parse(line) -> {kind: "reply", id, ok} | {kind: "error", id, code, message, body} | {kind: "event", event, body} | {kind: "bad", text}`; `helloOutcome(parsed) -> "ok" | "protocol" | "old"`.
- `lib/session.mjs`: `BACKOFF_FIRST = 500`, `BACKOFF_MAX = 5000`, `HELLO_TIMEOUT = 30000`; `class Session(client)` with `phase` (`"idle" | "connecting" | "hello" | "ready" | "waiting"`), `problem` (`"" | "unreachable" | "old" | "protocol" | "lost"`), `backoff`, `topics`, and methods returning effect arrays: `start()`, `connected()`, `line(text)`, `closed(error?: number)`, `helloTimeout()`, `retry()`, `stop()`, `setLive(topics[])`; `request(build: (id) => object) -> {id: number|null, effects}`. Effects: `{connect: true}`, `{close: true}`, `{write: string}`, `{retryIn: ms}` (`-1` cancels), `{helloTimeoutIn: ms}`, `{ready: helloOk}`, `{reply: parsed}`, `{event: parsed}`, `{lost: true}`, `{problem: string}`.

- `lib/devices.mjs`: `ISSUES`; `usable(device) -> bool`; `anyUsable(devices) -> bool`; `marker(device) -> string` (`""` for tested); `unsupportedHint(device) -> string`; `class DeviceStore` with `devices: Device[]`, `states: {id: {rev, state}}`, `reset()`, `fromHello(ok)`, `onDevices(list) -> removedIds[]`, `onState({device, rev, state}) -> bool`, `device(id)`, `state(id)`, `rev(id)`; `selection(devices, current) -> id | null`.
- `lib/describe.mjs`: `TYPES`; `setting(device, path)`, `action(device, name)`; `has(device, pathOrGroup) -> bool`; `writable(device, path)`; `choices(device, path, labels) -> [{value, label, selectable}]`; `humanize(value)`; `numberSpec(setting, component?) -> {min, max, step, precision, unit} | null`; `decimals(precision)`; `roundTo(value, precision)`; `sliderScale(spec) -> {count, toValue(i), toIndex(v)} | null`; `at(state, path)`; `blocked(device, state, path) -> null | {reason: "unavailable" | "readonly" | "asleep" | "requires", needs?}`; `warnings(device, path) -> string[]`; `limit(device, path) -> number | null`; `topics(device) -> string[]`; `wantedTopics(device, {open, pose, target, auto}) -> string[]`; `sameValue(a, b)`.
- `lib/values.mjs`: `class OwnValues` with `add(id, patch)`, `settle(id) -> bool`, `clear()`, `value(path)`, `size`; `shown(state, own, path)`; `status(state, own, path) -> "own" | "pending" | "unread" | "camera"`; `changed(state, own, edits) -> patch | null`; `defaults(device, state, own, paths) -> patch | null`.
- `lib/format.mjs`: `unitText(unit)`, `number(value, precision, unit)`, `value(setting, v, labels?)`.
- `lib/gimbal.mjs`: `STEP_BASE_MS = 2000`; `moveSpec(device) -> {pan, tilt} | null`; `stepTarget(spec, pose, last, now, dPan, dTilt) -> {pan, tilt} | null` (`null` while the base has no numeric pan and tilt: no step without a known pose); `moveRequest(target) -> {action: "gimbal.move", params: {pan, tilt}}`; `panLeftSign(state) -> 1 | -1`.
- `lib/lists.mjs`: `UNNAMED = "(unnamed)"`; `presetSlots(device) -> number[]`; `presetRows(device, state) -> [{slot, used, name, label, pose}] | null`; `viewRows(state) -> [{id, name, label, pose}] | null`; `saveParams(slot, typed) -> {slot} | {slot, name}`; `nameProblem(device, action, param, text) -> string | null`.
- `lib/labels.mjs`: `labelsFor(path) -> {value: name}`; `labelledPaths() -> string[]`; `problemText(problem) -> {title, body} | null`.

- `lib/link.mjs`: `class Link(client)` with `session`, `store`, `own`, `selected`; getters `ready`, `problem`; input methods returning effects (`start()`, `connected()`, `line(text)`, `closed(error?)`, `helloTimeout()`, `retry()`, `stop()`, `setLive(topics)`, `pick(id)`); readers `device()`, `state()`, `shown(path)`, `status(path)`; requests `set(path, value)`, `setPatch(patch)`, `act(name, params?)`, each returning the effects of exactly one written line, or `[]` when nothing can be sent. Effects are Task 3's I/O effects plus `{changed: true}` and `{failed: {id, code, message}}`; `{ready}`, `{reply}` and `{event}` are consumed inside.

**Part B, `CameraLink.qml` and the bound controls** (Task 6):

- `CameraLink` (an `Item`): inputs `active: bool`, `instanceName: string`, `client: string`, `liveView: {open, pose, target, auto}`; read-only `revision: int`, `link` (the `Link`), `socketPath: string`, `ready: bool`, `problem: string`, `devices: Device[]`, `device: Device | null`, `camState: object | null`, `deviceId: string`, `available: bool`, `usable: bool`, `anyUsable: bool` (some listed device is usable, selected or not), `awake: bool`, `marker: string`, `unsupportedHint: string`, `wantedTopics: string[]`; functions `has(path)`, `setting(path)`, `value(path)`, `status(path)`, `choices(path, labels)`, `spec(path, component?)`, `paramSpec(action, param, component?)`, `blocked(path)`, `warnings(path)`, `defaultsPatch(paths)`, `limit(path)`, `set(path, value)`, `setPatch(patch)`, `act(name, params?)`, `pick(id)`, `retry()`; `signal failed(var id, string code, string message)` (it also shows a toast).
- `SettingToggle` (a `ToggleRow`): `link`, `path`, `text`, `hint`.
- `SettingChips` (a `ChipRow`): `link`, `path`, `text`, `labels`.
- `SettingSlider` (a `LabeledSlider`): `link`, `path`, `text`, `component` (`""`, `"min"` or `"max"`).
- Each bound control: hidden unless `link.has(path)` (the slider also needs bounds and a step); disabled with `opacity 0.38` while `link.blocked(path)` is not null; marks `· pending` while `link.status(path) === "pending"`, `· no visible effect` for a `no_effect` leaf, `· needs <path> = <value>` (the toggle: as its description) while a `requires` does not hold; sends one `link.set` (the toggle and chips on a change, the slider on release).
- Name fields (preset and view rename, view save; Tasks 11 and 12): a rename field is prefilled with `row.name` (the camera's name, `""` for an unnamed one), never with `row.label` (its `(unnamed)` placeholder is display only, spec 12.3); 0.29's `maximumLength: 32`, its `RegularExpressionValidator` and its `trim()` go; the save button is disabled while `Lists.nameProblem(…)` is not null (the bounds come from the description), and the typed text is sent unchanged.

**Part C, the root interface `ctl`** (Task 8 writes it in `ObsbotWidget.qml`; Task 15 gives `DesktopPreview.qml` the members its files use). Every card receives the root as `ctl`.

Kept unchanged from 0.29: `cardEnabled(card)`, `arranging`, `isOpen(id)`, `toggleOpen(id)`, `tabs`, `cardTabs`, `selectTab(id, save)`, `moveCard(…)`, `resetLayout()`, `previewOn`, `previewRunning`, `interactionActive`, `showTargetBox`, `setShowTargetBox(on)`, `fineGimbal`, `gimbalStep`, `degrees(d)`, `wheelZoom`, `middleClickTracking`, `statusTextInBar`, `hideWithoutCamera`, `refresh()` (now `link.retry()`). New or changed (exact):

```qml
CameraLink { id: cameraLink }                 // exposed as:
readonly property alias link: cameraLink      // cards read and send through ctl.link only
function labels(path) { return Labels.labelsFor(path); }
readonly property bool available: link.usable // a supported device is selected and plugged in
readonly property bool live: link.awake       // … and the state says it is awake
readonly property bool streaming: link.value("on_air.active") === true || previewRunning
readonly property var streamingApps           // names from link.value("on_air.apps") items {name}, without "qs", plus "qs" while our preview runs
readonly property string videoNode: link.device !== null && link.device.video_node ? link.device.video_node : ""
readonly property bool previewReady: videoNode !== ""
readonly property var problem: Labels.problemText(link.problem)   // null while connected
readonly property string modeText             // member code below
readonly property string pillText             // "Offline", "Unsupported", "Asleep" or modeText
readonly property string statusText
readonly property string summary
readonly property string baseSummary
function toggleAwake()      // one act: "wake" while asleep, else "sleep"
function toggleTracking()   // one set of tracking.mode (member code below)
function stepZoom(notches)  // one set of zoom.factor (member code below)
```

Removed from the root (each now lives in a card or is gone, decision 12): `trackingModes`, `speeds`, `speedOptions`, `exposureModes`, `manualExposure`, `imageControls`, `imageAtDefaults`, `cam`, `status`, `tracking`, `image`, `zoomFactor`, `pan`, `tilt`, `limits`, `zoomRange`, `model`, `device` (the by-id path), `awake` (use `live`), `toolMissing` (use `problem !== null`), `aiMode`, `speed`, `has()`, `panLeftSign`, `pendingLabel`, `pendingId`, `requestId`, `poseWanted`, `poseSent`, `syncPose()`, `stateRefreshed`, `dragHold`, `OptimisticState`, `send()`, `handleReply()`, `onState()`, `awakeReported`, `setAwake()`, `assumeAwake()`, `setMode()`, `setSpeed()`, `setHdr()`, `factoryReset()`, every preset property and function, `setExposure()`, `setControl()`, `setVolume()`, `ledBlink`, `setLedBlink()`, `handGimbal`, `setHandGimbal()`, `wakeModeIndex()`, `setWakeMode()`, `setZoom()`, `moveTarget`, `moveTargetAt`, `stepBase()`, `moveGimbal()`, `autoSleepTimes`, `autoSleepIndex()`, `CameraWatch`.

The member code, exactly (Task 8 step 3; `Labels` is `lib/labels.mjs`, `Describe` is `lib/describe.mjs`):

```qml
    function labels(path) {
        return Labels.labelsFor(path);
    }
    readonly property bool available: link.usable
    readonly property bool live: link.awake
    readonly property var problem: Labels.problemText(link.problem)
    readonly property bool streaming: link.value("on_air.active") === true || previewRunning
    readonly property var streamingApps: {
        const apps = link.value("on_air.apps");
        const names = Array.isArray(apps) ? apps.map(a => String(a.name)).filter(n => n !== "qs") : [];
        return previewRunning ? names.concat(["qs"]) : names;
    }
    readonly property string videoNode: link.device !== null && link.device.video_node ? link.device.video_node : ""
    readonly property bool previewReady: videoNode !== ""
    // Decision 14: "Static" for static, "Tracking" for a mode the widget names, else the value's text
    readonly property string modeText: {
        const mode = link.value("tracking.mode");
        if (mode === null || mode === undefined)
            return "";
        if (mode === "static")
            return "Static";
        return mode in Labels.labelsFor("tracking.mode") ? "Tracking" : Describe.humanize(mode);
    }
    readonly property string pillText: link.device !== null && link.device.support === "unsupported" ? "Unsupported" : !available ? "Offline" : !live ? "Asleep" : modeText
    readonly property string statusText: {
        if (problem !== null)
            return problem.title;
        if (link.device === null)
            return link.devices.length > 0 ? "Pick a camera" : "No camera";
        if (link.device.support === "unsupported")
            return "Not supported by obsbot4linux";
        if (!available)
            return "Unplugged";
        const target = link.value("tracking.target.detected");
        const mode = Describe.choices(link.device, "tracking.mode", labels("tracking.mode")).find(c => c.value === link.value("tracking.mode"));
        return live ? "Awake · " + (mode ? mode.label : "") + (target === false && link.value("tracking.mode") !== "static" ? " · searching" : "") : "Asleep · lens stowed";
    }
    readonly property string baseSummary: (link.device !== null ? link.device.name + (link.marker !== "" ? " (" + link.marker + ")" : "") : "OBSBOT camera") + " · " + statusText
    readonly property string summary: baseSummary + (streaming ? " · On air" + (streamingApps.length > 0 ? ": " + streamingApps.map(a => appNames[a] ?? a).join(", ") : "") : "")

    // One act: wake while asleep, else sleep. The switch then follows the state (spec 4.4:
    // clients wait for the state, never for their own value). As in 0.29, the preview stops
    // when the user sends sleep; a preview started on a sleeping camera stays on while it wakes.
    function toggleAwake() {
        if (!available)
            return;
        if (live)
            previewOn = false;
        link.act(live ? "sleep" : "wake", {});
    }

    // Middle click: Static <-> the last other mode, one set; only when the description lets
    // both be written (decision 12)
    function toggleTracking() {
        const s = link.setting("tracking.mode");
        const writable = s && Array.isArray(s.writable) ? s.writable : [];
        const mode = link.value("tracking.mode");
        const target = mode === "static" ? lastTrackingMode : "static";
        if (available && writable.includes("static") && writable.includes(target) && link.blocked("tracking.mode") === null)
            link.set("tracking.mode", target);
    }

    // Mouse wheel: 0.1 per notch, rounded to the description's precision of zoom.factor and
    // inside its bounds, one set
    function stepZoom(notches) {
        const spec = link.spec("zoom.factor");
        const now = link.value("zoom.factor");
        if (!spec || spec.min === null || spec.max === null || typeof now !== "number" || link.blocked("zoom.factor") !== null)
            return;
        const next = Math.max(spec.min, Math.min(spec.max, Describe.roundTo(now + notches * 0.1, spec.precision)));
        if (next !== now)
            link.set("zoom.factor", next);
    }

    // The last tracking mode other than Static, saved as before
    Connections {
        target: root.link
        function onRevisionChanged() {
            const mode = root.link.value("tracking.mode");
            if (typeof mode === "string" && mode !== "static" && mode in Labels.labelsFor("tracking.mode") && mode !== root.lastTrackingMode && root.link.status("tracking.mode") === "camera")
                root.pluginService?.savePluginData(root.pluginId, "lastTrackingMode", mode);
        }
    }
```

**Part D, rules for every card task (10–15)**

Each card task rewires its files from 0.29 names to `ctl.link`. Rules for every control (they are also the review checklist):

1. Every control is gated on its own path or action: a bound control (`SettingToggle`, `SettingChips`, `SettingSlider`) for a leaf, `visible: ctl.link.has("<action>")` plus `enabled: ctl.link.blocked("<action>") === null` for an action button. A card is visible when `ctl.cardEnabled("<Card>")` and the device has at least one of its paths (`ctl.link.has("<group>")`).
2. One user action sends exactly one request: `ctl.link.set(path, value)`, `ctl.link.setPatch(patch)` (only the defaults reset and an area edit through `Values.changed`), or `ctl.link.act(name, params)`. Never two calls in one handler; never a `set` before an `act` (spec 2, 12.3).
3. Values come from `ctl.link.value(path)`; bounds, step, unit, enum values and list sizes from the description (`ctl.link.spec`, `paramSpec`, `choices`, `limit`, `lib/lists.mjs`). No literal bound, step or enum list of the widget's own (the two shortcut lists named in decision 16 excepted, both kept inside the description's bounds); labels only through `ctl.labels(path)`.
4. Text of a value: `Format.value(ctl.link.setting(path), ctl.link.value(path), ctl.labels(path))` (`import "lib/format.mjs" as Format`); `null` shows "–".
5. No `Timer` that holds or confirms a value; no `Connections { function onStateRefreshed … }` (the bound controls re-sync themselves); no `pluginData` for camera values.
6. A card keeps its title, order, collapsible behaviour, `cardId`, `arranger`, `controlled`/`expanded` wiring and its `enabled: ctl.available || arranging` / `opacity` lines.

## The tables name, per control: the 1.0 path or action, the component, the text, and what is special. "–" = nothing special.

### Task 1: CI on `v1` and the 0.x bound in `plugin.json`

Base `main`. A small pull request that the 0.x release R0 then ships.

**Files:**

- Modify: `.github/workflows/ci.yml` (the trigger and its comment)
- Modify: `plugin.json` (`dependencies`)
- Modify: `CHANGELOG.md`

**Interfaces:**

- Consumes: nothing.
- Produces: CI runs the `required` check for pull requests into `main` and into `v1` (Tasks 2–16 rely on it); `plugin.json` says `"obsbot4linux >= 0.7.3, < 1.0"`.

- [ ] **Step 1: The check that fails first**

Run: `jq -r '.dependencies[0]' plugin.json; grep -n 'branches:' .github/workflows/ci.yml`
Expected now: `obsbot4linux >= 0.7.3` and `    branches: [main]`.

- [ ] **Step 2: Change the trigger**

In `.github/workflows/ci.yml`, replace the first comment block and the trigger (lines 3–9 of the current file) with:

```yaml
# Pull requests into main and into v1 (the widget 1.0 branch) only: this workflow produces the
# `required` check that the rulesets of both branches require, so it must not run on push or by
# dispatch (that would mark a commit green that was never a pull request head). No paths filter,
# so the check always reports.
on:
  pull_request:
    branches: [main, v1]
```

Leave every job unchanged.

- [ ] **Step 3: State the bound in `plugin.json`**

Replace `"obsbot4linux >= 0.7.3",` by `"obsbot4linux >= 0.7.3, < 1.0",` in `dependencies`. Nothing else in the file changes (the version stays `0.29.1`; R0 sets 0.29.2).

- [ ] **Step 4: CHANGELOG**

Under `## [Unreleased]`, `### Changed`, add:

```markdown
- `plugin.json` states the upper bound the startup check enforces since 0.28.0: obsbot4linux 0.7.3
  or newer, before 1.0. obsbot4linux 1.0 needs widget 1.0 (#N).
- CI also runs for pull requests into `v1`, the branch where widget 1.0 is built until obsbot4linux
  1.0 is released (#N).
```

(`#N`: this task's issue.)

- [ ] **Step 5: Verify**

Run: `jq -r '.dependencies[0]' plugin.json && actionlint .github/workflows/ci.yml && bash scripts/check-qml.sh && shellcheck scripts/*.sh`
Expected: `obsbot4linux >= 0.7.3, < 1.0`, actionlint silent, `check-qml: ok (32 files)`, shellcheck silent. Acceptance of the trigger is GitHub's: the pull request itself shows the `required` check (it targets `main`); the `v1` trigger is proven by Task 2's pull request.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ci.yml plugin.json CHANGELOG.md
git commit -S -m "plugin.json: state the bound before 1.0; CI also for pull requests into v1"
```

---

### Task 2: Two-engine test harness, fixtures, version module

Base `v1` (B0 done). Needs W0 on the tool's `rebuild` (the generated description files and golden transcripts).

**Files:**

- Create: `tests/harness.mjs`, `tests/assert.mjs`, `tests/run.mjs`
- Create: `scripts/check-js.sh` (executable), `scripts/sync-fixtures.mjs`
- Create (generated): `tests/fixtures/tiny2lite.description.mjs`, `fake-a.description.mjs`, `fake-b.description.mjs`, `session.transcript.mjs`, `errors.transcript.mjs`, `hotplug.transcript.mjs`
- Create: `tests/fixtures/fakes.mjs`
- Create: `lib/version.mjs`, `tests/version.test.mjs`
- Modify: `scripts/check-qml.sh`, `.github/workflows/ci.yml`, `README.md`, `CHANGELOG.md`

**Interfaces:**

- Consumes: the tool's `descriptions/tiny2lite.json`, `descriptions/fake/fake-a.json`, `descriptions/fake/fake-b.json`, `crates/obsbot-proto/tests/golden/{session,errors,hotplug}.jsonl` (W0 Tasks 4 and 10).
- Produces:
  - Test module contract: a file `tests/<name>.test.mjs` exports `const tests = { "<name>": () => { … } }` (synchronous; passes when it returns, fails when it throws) and imports `eq(actual, expected, what?)`, `ok(condition, what?)`, `throws(fn, what?)`, `same(a, b)` from `./assert.mjs`. `scripts/check-js.sh` finds every `tests/*.test.mjs`.
  - Fixtures: `tests/fixtures/<name>.description.mjs` default-exports `{settings, actions, limits}`; `tests/fixtures/<name>.transcript.mjs` default-exports `[{from: "client"|"daemon", msg}]`; `tests/fixtures/fakes.mjs` exports `A`, `B`, `UNKNOWN` (Device objects of the three fakes), `STATES` (id → `{rev, state}`), `HELLO` (the `ok` payload of a hello reply).
  - `lib/version.mjs`: `parseVersion(text) -> [major, minor, patch] | null`, `isOlder(version, other) -> bool`, `checkTool(stdout, exitCode, min, max | null, links: {releases, widgetReleases}) -> null | {title, details}`.
  - `scripts/check-js.sh` (exit 0 only when node and `qml` both pass the same count), `scripts/check-qml.sh` (now also `lib/*.mjs`), CI job `js` inside `required`.

- [ ] **Step 1: Write the failing test**

Create `tests/version.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import { checkTool, isOlder, parseVersion } from "../lib/version.mjs";

const links = { releases: "R", widgetReleases: "W" };

export const tests = {
  "a version line parses into three numbers": () => {
    eq(parseVersion("obsbot4linux 0.13.2\n"), [0, 13, 2]);
    eq(parseVersion("no version here"), null);
  },
  // W4 runs widget 1.0 against v1.0.0-rc.N: the candidate must pass the 1.0 check
  "a release candidate counts as its release": () => {
    eq(parseVersion("obsbot4linux 1.0.0-rc.3"), [1, 0, 0]);
    eq(checkTool("obsbot4linux 1.0.0-rc.3", 0, [1, 0, 0], null, links), null);
  },
  "isOlder compares major, minor, patch in that order": () => {
    ok(isOlder([0, 13, 2], [1, 0, 0]), "0.13.2 < 1.0.0");
    ok(!isOlder([1, 0, 0], [1, 0, 0]), "equal is not older");
    ok(isOlder([1, 9, 9], [1, 10, 0]), "minor wins over patch");
  },
  // Widget 1.0 speaks protocol 2 only: a 0.13 tool must be refused at enable time
  "widget 1.0 refuses a 0.x tool and names the minimum": () => {
    const r = checkTool("obsbot4linux 0.13.2", 0, [1, 0, 0], null, links);
    eq(r.title, "obsbot4linux 1.0.0 or newer is required");
    ok(
      r.details.includes("Installed: obsbot4linux 0.13.2"),
      "details name the installed version",
    );
  },
  // The 0.x widget's bound: obsbot4linux 1.0 needs widget 1.0
  "a 0.x widget refuses 1.0 and points to the widget releases": () => {
    const r = checkTool("obsbot4linux 1.0.0", 0, [0, 7, 3], [1, 0, 0], links);
    eq(r.title, "obsbot4linux 1.0 or newer needs a newer widget");
    ok(r.details.includes("W"), "details link the widget releases");
  },
  "a missing tool is reported as not installed": () => {
    eq(
      checkTool("", 127, [1, 0, 0], null, links).title,
      "obsbot4linux is required",
    );
  },
  "an unreadable version is refused, not accepted": () => {
    eq(
      checkTool("obsbot4linux dev", 0, [1, 0, 0], null, links).title,
      "obsbot4linux 1.0.0 or newer is required",
    );
  },
};
```

Create `tests/assert.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The assertions of the test modules. Plain JavaScript, so that the same tests run in node and
// in the QML engine (scripts/check-js.sh).

function show(value) {
  return value === undefined ? "undefined" : JSON.stringify(value);
}

// Deep equality: numbers, strings, booleans and null by ===; arrays by length and items; objects
// by their own keys (an undefined property counts as absent).
export function same(a, b) {
  if (a === b) return true;
  if (
    a === null ||
    b === null ||
    typeof a !== "object" ||
    typeof b !== "object"
  )
    return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  if (Array.isArray(a))
    return a.length === b.length && a.every((x, i) => same(x, b[i]));
  const keys = (o) =>
    Object.keys(o)
      .filter((k) => o[k] !== undefined)
      .sort();
  const ka = keys(a);
  const kb = keys(b);
  return same(ka, kb) && ka.every((k) => same(a[k], b[k]));
}

export function eq(actual, expected, what) {
  if (!same(actual, expected))
    throw new Error(
      `${what ?? "value"}: expected ${show(expected)}, got ${show(actual)}`,
    );
}

export function ok(condition, what) {
  if (!condition) throw new Error(`${what ?? "condition"} is false`);
}

export function throws(fn, what) {
  try {
    fn();
  } catch (e) {
    return;
  }
  throw new Error(`${what ?? "call"} did not throw`);
}
```

Create `tests/harness.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Runs the test modules: in node through tests/run.mjs and in the QML engine through
// scripts/check-js.sh. A test module exports `tests`, an object of synchronous functions; a
// function passes when it returns and fails when it throws.

export function runAll(modules) {
  const lines = [];
  let passed = 0;
  let failed = 0;
  for (const file of Object.keys(modules).sort()) {
    const tests = modules[file].tests ?? {};
    for (const name of Object.keys(tests)) {
      try {
        tests[name]();
        passed += 1;
      } catch (e) {
        failed += 1;
        lines.push(
          `not ok - ${file}: ${name}: ${e && e.message ? e.message : String(e)}`,
        );
      }
    }
  }
  lines.push(`${passed} passed, ${failed} failed`);
  return { passed, failed, lines };
}
```

Create `tests/run.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// node tests/run.mjs [file.test.mjs …]: runs the given test modules, or every tests/*.test.mjs.
// Exits 1 when a test fails or when no test ran.

import { readdirSync } from "node:fs";
import { dirname, join, basename } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { runAll } from "./harness.mjs";

const dir = dirname(fileURLToPath(import.meta.url));
const files =
  process.argv.length > 2
    ? process.argv.slice(2)
    : readdirSync(dir)
        .filter((f) => f.endsWith(".test.mjs"))
        .map((f) => join(dir, f));
const modules = {};
for (const file of files)
  modules[basename(file)] = await import(pathToFileURL(file).href);
const result = runAll(modules);
for (const line of result.lines) console.log(line);
if (result.failed > 0 || result.passed === 0) process.exit(1);
```

- [ ] **Step 2: Run it and see it fail**

Run: `node tests/run.mjs`
Expected: exit 1 with `ERR_MODULE_NOT_FOUND` for `lib/version.mjs`.

- [ ] **Step 3: The version module**

Create `lib/version.mjs` (the logic of today's `StartupCheck.qml`, moved into a module so that it is tested; the startup check itself changes in Task 16):

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The startup check's version test (StartupCheck.qml): which obsbot4linux the widget accepts.

// "obsbot4linux 1.0.0-rc.2" -> [1, 0, 0]; null when the text holds no X.Y.Z. A pre-release
// suffix is ignored, so a release candidate of 1.0.0 counts as 1.0.0: stage W4 runs widget 1.0
// against the candidate.
export function parseVersion(text) {
  const m = /(\d+)\.(\d+)\.(\d+)/.exec(String(text));
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

export function isOlder(version, other) {
  for (let i = 0; i < 3; i++) {
    if (version[i] !== other[i]) return version[i] < other[i];
  }
  return false;
}

// The result of `obsbot4linux --version` (its stdout and exit code) against [min, max): null
// when it fits, else { title, details } for the shell's startup-check message. max null: no
// upper bound.
export function checkTool(stdout, exitCode, min, max, links) {
  const minimum = min.join(".");
  const range = max
    ? `${minimum} or newer (before ${max[0]}.0)`
    : `${minimum} or newer`;
  if (exitCode !== 0) {
    return {
      title: "obsbot4linux is required",
      details: `The 'obsbot4linux' tool is not installed or not on your PATH. This plugin needs obsbot4linux ${range}.\n\nInstall it from ${links.releases}, then enable this plugin again.`,
    };
  }
  const version = parseVersion(stdout);
  const installed = String(stdout).trim();
  if (version && !isOlder(version, min) && (!max || isOlder(version, max)))
    return null;
  if (version && max && !isOlder(version, max)) {
    return {
      title: `obsbot4linux ${max[0]}.0 or newer needs a newer widget`,
      details: `Installed: ${installed}. This widget version works with obsbot4linux ${range}.\n\nUpdate this plugin (obsbot4linux-dux) from ${links.widgetReleases}, then enable it again.`,
    };
  }
  return {
    title: `obsbot4linux ${minimum} or newer is required`,
    details: `Installed: ${installed}. This plugin needs obsbot4linux ${range}.\n\nUpdate it from ${links.releases}, then enable this plugin again.`,
  };
}
```

Run: `node tests/run.mjs`
Expected: `7 passed, 0 failed`.

- [ ] **Step 4: The two-engine runner**

Create `scripts/check-js.sh` and `chmod +x scripts/check-js.sh`:

```bash
#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Run the tests of the widget's JavaScript modules (lib/*.mjs) twice: in node, and in the QML
# engine that the shell runs them in (the Qt 6 `qml` tool, offscreen). Both runs must pass and
# must count the same number of tests. Neither run is ever skipped.
#
# Run it before every pull request:  scripts/check-js.sh
#
# The Qt 6 `qml` tool is looked up like check-qml.sh looks up its tools: $OBSBOT_QT6_DIR, the
# directories qtpaths6 reports, then /usr/lib/qt6 and /usr/lib64/qt6.

set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

die() {
	echo "check-js: $*" >&2
	exit 2
}

candidate_dirs() {
	local root
	if [ -n "${OBSBOT_QT6_DIR:-}" ]; then
		printf '%s\n' "$OBSBOT_QT6_DIR" "$OBSBOT_QT6_DIR/bin"
		return
	fi
	if command -v qtpaths6 >/dev/null 2>&1; then
		qtpaths6 --query QT_INSTALL_BINS 2>/dev/null || true
	fi
	for root in /usr/lib/qt6 /usr/lib64/qt6; do
		printf '%s\n' "$root/bin"
	done
}

find_qml() {
	local dir
	while IFS= read -r dir; do
		if [ -n "$dir" ] && [ -x "$dir/qml" ] && [ -f "$dir/qml" ] &&
			"$dir/qml" --version 2>&1 | grep -q -E ' 6\.[0-9]+'; then
			printf '%s\n' "$dir/qml"
			return 0
		fi
	done < <(candidate_dirs)
	return 1
}

command -v node >/dev/null 2>&1 || die "node not found. Install Node.js 20 or newer."
qml=$(find_qml) || die "qml (Qt 6) not found. Install the Qt 6 declarative tools, or set OBSBOT_QT6_DIR."

shopt -s nullglob
tests=("$repo"/tests/*.test.mjs)
[ "${#tests[@]}" -gt 0 ] || die "no tests/*.test.mjs found"

echo "check-js: node $(node --version)"
node_out=$(node "$repo/tests/run.mjs" "${tests[@]}") || {
	printf '%s\n' "$node_out"
	die "node: tests failed"
}
printf '%s\n' "$node_out"
node_summary=$(printf '%s\n' "$node_out" | tail -n 1)

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# One module that imports every test module and runs them with the same harness as node
{
	i=0
	for f in "${tests[@]}"; do
		printf 'import * as t%d from "file://%s";\n' "$i" "$f"
		i=$((i + 1))
	done
	printf 'import { runAll } from "file://%s/tests/harness.mjs";\n' "$repo"
	printf 'export function run() {\n    return runAll({\n'
	i=0
	for f in "${tests[@]}"; do
		printf '        "%s": t%d,\n' "$(basename "$f")" "$i"
		i=$((i + 1))
	done
	printf '    });\n}\n'
} >"$tmp/all.mjs"

cat >"$tmp/Run.qml" <<'EOF'
import QtQml
import "all.mjs" as All

QtObject {
    Component.onCompleted: {
        const result = All.run();
        for (const line of result.lines)
            console.log("check-js:", line);
        Qt.exit(result.failed === 0 && result.passed > 0 ? 0 : 1);
    }
}
EOF

echo "check-js: qml ($qml)"
status=0
qml_out=$(QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1 timeout 120 "$qml" "$tmp/Run.qml" 2>&1) || status=$?
printf '%s\n' "$qml_out" | grep -F 'check-js:' || true
[ "$status" -eq 0 ] || {
	printf '%s\n' "$qml_out" >&2
	die "qml: tests failed or did not load (exit $status)"
}
printf '%s\n' "$qml_out" | grep -q -F "check-js: $node_summary" ||
	die "qml ran another number of tests than node ($node_summary)"

echo "check-js: ok ($node_summary in node and in qml)"
```

Run: `scripts/check-js.sh`
Expected: the last line `check-js: ok (7 passed, 0 failed in node and in qml)`. Prove that it fails: change `return version[i] < other[i];` in `lib/version.mjs` to `>`, run again: `not ok - version.test.mjs: isOlder …` and exit 2; undo the change. Prove that the QML half gates on its own: add `tests/zz-engine.test.mjs` with `export const tests = { "only node has structuredClone": () => { structuredClone({ a: 1 }); } };`, run: node reports `8 passed, 0 failed`, the `qml` run `structuredClone is not defined`, the script exits 2; delete the file.

- [ ] **Step 5: Fixtures**

Create `scripts/sync-fixtures.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// node scripts/sync-fixtures.mjs [--check] <obsbot4linux checkout>
//
// Copies the tool's generated description files and golden protocol transcripts into
// tests/fixtures/*.mjs (as modules, so that the tests load them in node and in the QML engine
// alike). --check writes nothing and exits 1 when a fixture differs from the checkout.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
const check = args[0] === "--check";
const tool = check ? args[1] : args[0];
if (!tool) {
  console.error(
    "usage: node scripts/sync-fixtures.mjs [--check] <obsbot4linux checkout>",
  );
  process.exit(2);
}
const out = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "tests",
  "fixtures",
);

// fixture name -> file in the tool's repository
const descriptions = {
  "tiny2lite.description": "descriptions/tiny2lite.json",
  "fake-a.description": "descriptions/fake/fake-a.json",
  "fake-b.description": "descriptions/fake/fake-b.json",
};
const transcripts = {
  "session.transcript": "crates/obsbot-proto/tests/golden/session.jsonl",
  "errors.transcript": "crates/obsbot-proto/tests/golden/errors.jsonl",
  "hotplug.transcript": "crates/obsbot-proto/tests/golden/hotplug.jsonl",
};

function header(source) {
  return `// Generated by scripts/sync-fixtures.mjs from obsbot4linux ${source}. Do not edit.\n`;
}

// "> {…}" client to daemon, "< {…}" daemon to client; "#" lines are comments
function transcript(text) {
  return text
    .split("\n")
    .filter((line) => line.startsWith("> ") || line.startsWith("< "))
    .map((line) => ({
      from: line[0] === ">" ? "client" : "daemon",
      msg: JSON.parse(line.slice(2)),
    }));
}

const wanted = {};
for (const [name, file] of Object.entries(descriptions))
  wanted[name] =
    header(file) +
    "export default " +
    JSON.stringify(
      JSON.parse(readFileSync(join(tool, file), "utf8")),
      null,
      1,
    ) +
    ";\n";
for (const [name, file] of Object.entries(transcripts))
  wanted[name] =
    header(file) +
    "export default " +
    JSON.stringify(
      transcript(readFileSync(join(tool, file), "utf8")),
      null,
      1,
    ) +
    ";\n";

let stale = 0;
for (const [name, text] of Object.entries(wanted)) {
  const path = join(out, `${name}.mjs`);
  if (check) {
    let have = "";
    try {
      have = readFileSync(path, "utf8");
    } catch (e) {
      have = "";
    }
    if (have !== text) {
      console.error(
        `stale: tests/fixtures/${name}.mjs (run: node scripts/sync-fixtures.mjs ${tool})`,
      );
      stale += 1;
    }
  } else {
    writeFileSync(path, text);
  }
}
process.exit(stale > 0 ? 1 : 0);
```

Run, with a checkout of the tool's `rebuild` at `$TOOL` (read-only use): `node scripts/sync-fixtures.mjs "$TOOL" && node scripts/sync-fixtures.mjs --check "$TOOL"`
Expected: six files in `tests/fixtures/`, the check exits 0. Never edit them by hand.

Create `tests/fixtures/fakes.mjs` (device facts and first states of the three fakes as W0's fake models report them; descriptions from the synced files):

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The hello reply of a daemon started with `--fake fake-a,fake-b,unknown` before its first
// events: device facts and first states as the fake models report them (stage W0), each
// description from the synced description files. Written by hand for the tests;
// tests/e2e/fake-daemon.test.mjs compares it with the hello of the real fake daemon.

import fakeA from "./fake-a.description.mjs";
import fakeB from "./fake-b.description.mjs";

const empty = { settings: {}, actions: {}, limits: {} };

export const A = Object.assign(
  {
    id: "sn:FAKEA0000001",
    name: "Fake A 0001",
    model: "fake-a",
    support: "tested",
    serial: "FAKEA0000001",
    firmware: "1.0.0.0",
    video_node: "/dev/video102",
    available: true,
    usb: {
      vendor: "3564",
      product: "fa0a",
      product_name: "Fake A",
      bcd_device: "0100",
      path: "9-1",
      serial_present: false,
      interfaces: ["0e", "0e", "01", "01"],
      extension_units: [],
    },
  },
  fakeA,
);
export const B = Object.assign(
  {
    id: "sn:FAKEB0000001",
    name: "Fake B 0001",
    model: "fake-b",
    support: "untested",
    serial: "FAKEB0000001",
    firmware: "0.9.1",
    video_node: "/dev/video104",
    available: true,
    usb: {
      vendor: "3564",
      product: "fa0b",
      product_name: "Fake B",
      bcd_device: "0100",
      path: "9-2",
      serial_present: false,
      interfaces: ["0e", "0e"],
      extension_units: [],
    },
  },
  fakeB,
);
export const UNKNOWN = Object.assign(
  {
    id: "port:9-3",
    name: "Unknown Camera 9-3",
    model: "unknown",
    support: "unsupported",
    serial: null,
    firmware: null,
    video_node: "/dev/video106",
    available: true,
    usb: {
      vendor: "3564",
      product: "fa00",
      product_name: "Unknown Camera",
      bcd_device: "0100",
      path: "9-3",
      serial_present: false,
      interfaces: ["0e", "0e"],
      extension_units: [],
    },
  },
  empty,
);

export const STATES = {
  "port:9-3": { rev: 1, state: { available: true, pending: {}, unread: [] } },
  "sn:FAKEA0000001": {
    rev: 1,
    state: {
      available: true,
      pending: {},
      unread: [],
      asleep: false,
      audio: {
        auto_gain: false,
        enabled: false,
        in_sleep: false,
        noise_reduction: "off",
        volume: 0,
      },
      exposure: {
        anti_flicker: "off",
        compensation: 9.0,
        gain: 1.0,
        hdr: false,
        mode: "auto",
        region: "face",
        time: 33000,
      },
      focus: { mode: "continuous", position: 0.0, region: "face" },
      framing: {
        auto_zoom: false,
        offset: { x: 0, y: 0 },
        part: "normal",
        size: "long_shot",
        zoom_speed: "slow",
      },
      gestures: {
        direction_change: false,
        dynamic_zoom: false,
        target_lock: false,
        zoom: false,
        zoom_ratio: 1,
      },
      gimbal: { invert: false, pan: 0, tilt: 0 },
      hand: {
        area: { pan: { max: 150, min: -150 }, tilt: { max: 90, min: -70 } },
        axes: { pan: false, tilt: false },
        return: null,
        return_after: 0,
        side: "left",
      },
      image: {
        brightness: 50.0,
        contrast: 50.0,
        frame_rate: 30,
        hue: 50.0,
        mirror: false,
        saturation: 50.0,
        sharpness: 50.0,
        white_balance: { auto: true, temperature: 5000 },
      },
      led: { brightness: 0 },
      on_air: { active: false, apps: [] },
      power_on: {
        mode: "static",
        settings: {
          auto_exposure: null,
          autofocus: null,
          contrast: null,
          exposure_compensation: 0,
          hue: null,
          saturation: null,
          sharpness: null,
          tracking_speed: null,
          white_balance_mode: null,
          white_balance_temperature: null,
        },
        start: {
          mode: "static",
          pose: { pan: 0, tilt: 0, zoom: 1 },
          source: "own",
        },
      },
      presets: [],
      sleep: {
        auto: { enabled: false, seconds: 1 },
        background: "black",
        background_mirror: false,
        wake_zoom: 1,
      },
      tracking: {
        mode: "static",
        profile: "standard",
        speed: "lazy",
        target: { box: null, detected: false },
      },
      views: [],
      zone: {
        area: { pan: { max: 150, min: -150 }, tilt: { max: 90, min: -70 } },
        enabled: false,
        follow: false,
        return: { pan: 0, tilt: 0, zoom: 1 },
      },
      zoom: { factor: 1, field_of_view: "wide" },
    },
  },
  "sn:FAKEB0000001": {
    rev: 1,
    state: {
      available: true,
      pending: {},
      unread: [],
      asleep: false,
      beacon: { color: "red" },
      exposure: { mode: "auto", time: 100 },
      led: { brightness: 0 },
      on_air: { active: false, apps: [] },
      presets: [],
      tracking: { mode: "off" },
      zoom: { factor: 1 },
    },
  },
};

export const HELLO = {
  protocol: 2,
  daemon: "1.0.0",
  devices: [A, B, UNKNOWN],
  states: STATES,
};
```

Run: `node -e 'import("./tests/fixtures/fakes.mjs").then(m => console.log(m.HELLO.devices.map(d => d.id)))'`
Expected: `[ 'sn:FAKEA0000001', 'sn:FAKEB0000001', 'port:9-3' ]`. If W0 as landed differs from these facts (a value of a fake's first state, the firmware strings), change `fakes.mjs` to what `rebuild` reports; Task 7 checks it against the running fake daemon.

- [ ] **Step 6: The modules in the QML gate**

In `scripts/check-qml.sh`: replace the first comment line by

```bash
# Check that every QML file and every JavaScript module (lib/*.mjs) of the widget compiles and
# lints. The modules' behaviour is tested by scripts/check-js.sh.
```

after `[ "${#files[@]}" -gt 0 ] || die "no .qml files found in $repo"` add the line

```bash
files+=("$repo"/lib/*.mjs)
```

and in the `qmlcachegen` loop replace the two lines that set `name` and call `qmlcachegen` by

```bash
	name=${f#"$repo"/}
	if ! "$qmlcachegen" --only-bytecode --resource-path "/$name" -o "$tmp/${name//\//_}.cpp" "$f" 2>"$tmp/err"; then
```

Run: `bash scripts/check-qml.sh && shellcheck scripts/*.sh`
Expected: `check-qml: ok (33 files)` (32 QML files and `lib/version.mjs`), shellcheck silent. Prove that a broken module fails: `printf 'export function f( {\n' > lib/zz.mjs; bash scripts/check-qml.sh` prints `FAIL lib/zz.mjs`; delete `lib/zz.mjs`.

- [ ] **Step 7: CI job `js`**

Replace `.github/workflows/ci.yml` with:

```yaml
name: CI

# Pull requests into main and into v1 (the widget 1.0 branch) only: this workflow produces the
# `required` check that the rulesets of both branches require, so it must not run on push or by
# dispatch (that would mark a commit green that was never a pull request head). No paths filter,
# so the check always reports.
on:
  pull_request:
    branches: [main, v1]

permissions:
  contents: read

jobs:
  # Ubuntu's own Qt 6.4 qmllint does not accept the script's options. Arch ships the Qt that the
  # desktop shell runs. The digest pins the image; -Syu installs the current Arch Qt, so a new Qt
  # with a new lint can turn a pull request red: fix that in its own PR, never by loosening the script.
  qml:
    runs-on: ubuntu-24.04
    container: archlinux:latest@sha256:d4594bb2300cba11f41c2f27923eb56f63d9988909c402f9c95517d1e362c899
    timeout-minutes: 15
    steps:
      - run: pacman -Syu --noconfirm --needed qt6-declarative
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - run: bash scripts/check-qml.sh

  # The JavaScript modules (lib/*.mjs): their tests in node and in the QML engine.
  js:
    runs-on: ubuntu-24.04
    container: archlinux:latest@sha256:d4594bb2300cba11f41c2f27923eb56f63d9988909c402f9c95517d1e362c899
    timeout-minutes: 15
    steps:
      - run: pacman -Syu --noconfirm --needed qt6-declarative nodejs
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - run: bash scripts/check-js.sh

  shellcheck:
    runs-on: ubuntu-24.04
    container: archlinux:latest@sha256:d4594bb2300cba11f41c2f27923eb56f63d9988909c402f9c95517d1e362c899
    timeout-minutes: 15
    steps:
      - run: pacman -Syu --noconfirm --needed shellcheck
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - run: shellcheck scripts/*.sh

  # The version in plugin.json must have a non-empty CHANGELOG.md section (what the release needs).
  changelog:
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - run: scripts/release-notes.sh HEAD "v$(jq -r .version plugin.json)" >/dev/null

  # The one check the main ruleset requires. It passes only when every gate succeeded.
  required:
    name: required
    needs: [qml, js, shellcheck, changelog]
    if: always()
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    env:
      QML: ${{ needs.qml.result }}
      JS: ${{ needs.js.result }}
      SHELLCHECK: ${{ needs.shellcheck.result }}
      CHANGELOG: ${{ needs.changelog.result }}
    steps:
      - run: |
          echo "qml=$QML js=$JS shellcheck=$SHELLCHECK changelog=$CHANGELOG"
          test "$QML" = success
          test "$JS" = success
          test "$SHELLCHECK" = success
          test "$CHANGELOG" = success
```

Run: `actionlint .github/workflows/ci.yml`
Expected: silent. Acceptance is GitHub's: this pull request into `v1` shows the `required` check with `js=success` in its log.

- [ ] **Step 8: README and CHANGELOG**

In `README.md`, after the line that starts `Before sending a change, run \`scripts/check-qml.sh\``, add:

```markdown
Also run `scripts/check-js.sh` (needs Node.js 20 or newer and the Qt 6 `qml` tool): it runs the tests of the
JavaScript modules in `lib/` in node and in the QML engine. `tests/fixtures/` is generated from an obsbot4linux
checkout with `node scripts/sync-fixtures.mjs <checkout>`; do not edit those files by hand.
```

`CHANGELOG.md`, `## [Unreleased]`, `### Added`:

```markdown
- Tests for the widget's JavaScript modules (`lib/*.mjs`), run in node and in the QML engine by
  `scripts/check-js.sh` and by CI; the QML check also compiles and lints the modules (#N).
```

- [ ] **Step 9: Commit**

```bash
git add tests lib scripts .github/workflows/ci.yml README.md CHANGELOG.md
git commit -S -m "Tests: run the JavaScript modules' tests in node and in the QML engine"
```

---

### Task 3: Protocol lines and the connection state machine

**Files:**

- Create: `lib/protocol.mjs`, `lib/session.mjs`
- Test: `tests/protocol.test.mjs`, `tests/session.test.mjs`
- Modify: `CHANGELOG.md`

**Interfaces:**

- Consumes: Task 2's harness and `tests/fixtures/session.transcript.mjs`, `errors.transcript.mjs`.
- Produces:
  - `lib/protocol.mjs`: `PROTOCOL = 2`; `ERROR_CODES: string[12]`; `socketPath(env: (name) => string|undefined) -> string`; `encode(request) -> string` (JSON plus `"\n"`); `hello(id, client, live[]) -> object`; `setLeaves(id, device, patch) -> object`; `act(id, device, action, params?) -> object` (no `with` when `params` is empty); `live(id, topics[]) -> object`; `parse(line) -> {kind: "reply", id, ok} | {kind: "error", id, code, message, body} | {kind: "event", event, body} | {kind: "bad", text}`; `helloOutcome(parsed) -> "ok" | "protocol" | "old"`.
  - `lib/session.mjs`: `BACKOFF_FIRST = 500`, `BACKOFF_MAX = 5000`, `HELLO_TIMEOUT = 30000`; `class Session(client)` with `phase` (`"idle" | "connecting" | "hello" | "ready" | "waiting"`), `problem` (`"" | "unreachable" | "old" | "protocol" | "lost"`), `backoff`, `topics`, and methods returning effect arrays: `start()`, `connected()`, `line(text)`, `closed(error?: number)`, `helloTimeout()`, `retry()`, `stop()`, `setLive(topics[])`; `request(build: (id) => object) -> {id: number|null, effects}`. Effects: `{connect: true}`, `{close: true}`, `{write: string}`, `{retryIn: ms}` (`-1` cancels), `{helloTimeoutIn: ms}`, `{ready: helloOk}`, `{reply: parsed}`, `{event: parsed}`, `{lost: true}`, `{problem: string}`.

- [ ] **Step 1: Write the failing tests**

Create `tests/protocol.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import {
  act,
  encode,
  ERROR_CODES,
  hello,
  helloOutcome,
  live,
  parse,
  setLeaves,
  socketPath,
} from "../lib/protocol.mjs";
import session from "./fixtures/session.transcript.mjs";
import errors from "./fixtures/errors.transcript.mjs";

const env = (vars) => (name) => vars[name];

export const tests = {
  // 4.1: OBSBOT4LINUX_SOCKET overrides the path; the default is in XDG_RUNTIME_DIR
  "the socket path follows the environment": () => {
    eq(
      socketPath(env({ XDG_RUNTIME_DIR: "/run/user/1000" })),
      "/run/user/1000/obsbot4linux.sock",
    );
    eq(
      socketPath(
        env({
          XDG_RUNTIME_DIR: "/run/user/1000",
          OBSBOT4LINUX_SOCKET: "/tmp/t.sock",
        }),
      ),
      "/tmp/t.sock",
    );
    eq(socketPath(env({ OBSBOT4LINUX_SOCKET: "" })), "");
    eq(socketPath(env({})), "");
  },
  // The requests the widget writes are the golden transcript's lines, byte for byte
  "requests are written as the golden transcript writes them": () => {
    const sent = session
      .filter((m) => m.from === "client")
      .map((m) => JSON.stringify(m.msg));
    eq(JSON.stringify(hello(1, "widget 1.0", ["pose"])), sent[0]);
    eq(
      JSON.stringify(
        setLeaves(2, "sn:EXAMPLE0000001", { "led.brightness": 2 }),
      ),
      sent[1],
    );
    eq(
      JSON.stringify(act(4, "sn:EXAMPLE0000001", "preset.recall", { slot: 0 })),
      sent[3],
    );
    eq(JSON.stringify(live(5, ["pose", "target"])), sent[4]);
  },
  "an action without parameters has no with": () => {
    eq(act(6, "sn:X", "gimbal.stop", {}), {
      id: 6,
      device: "sn:X",
      do: "gimbal.stop",
    });
    eq(act(6, "sn:X", "gimbal.stop", undefined), {
      id: 6,
      device: "sn:X",
      do: "gimbal.stop",
    });
  },
  "a request is one line": () => {
    const line = encode(
      setLeaves(3, "sn:X", { "zone.area.pan": { min: -20, max: 20 } }),
    );
    ok(
      line.endsWith("\n") && line.indexOf("\n") === line.length - 1,
      "one newline, at the end",
    );
  },
  "every golden daemon line parses as a reply, an error or an event": () => {
    for (const m of session.concat(errors).filter((x) => x.from === "daemon")) {
      const p = parse(JSON.stringify(m.msg));
      ok(
        p.kind === "reply" || p.kind === "error" || p.kind === "event",
        `kind of ${JSON.stringify(m.msg)}`,
      );
    }
  },
  "each error code of the golden file keeps its code": () => {
    const codes = errors
      .filter((x) => x.from === "daemon")
      .map((m) => parse(JSON.stringify(m.msg)).code);
    eq(codes, ERROR_CODES);
  },
  // 4.1 rule 7: an unknown error code is treated as internal; unknown events are passed on
  "an unknown error code reads as internal": () => {
    const p = parse(
      '{"id":3,"error":{"code":"rate_limited","message":"slow down"}}',
    );
    eq([p.kind, p.code, p.message], ["error", "internal", "slow down"]);
  },
  "an unknown event type is an event, not an error": () => {
    eq(parse('{"event":"thermal","device":"sn:X"}').kind, "event");
  },
  "unknown fields are ignored": () => {
    eq(parse('{"id":4,"ok":{"rev":6},"trace":"abc"}'), {
      kind: "reply",
      id: 4,
      ok: { rev: 6 },
    });
  },
  "junk is bad, never a throw": () => {
    for (const line of [
      "",
      "{",
      "[]",
      "null",
      '"text"',
      '{"ok":{}}',
      '{"id":1,"ok":[]}',
    ])
      eq(parse(line).kind, "bad", line);
  },
  // 5.10: a 0.13 daemon answers hello with ok:false and a string error
  "a 0.13 daemon's answer to hello is an older daemon": () => {
    eq(
      helloOutcome(parse('{"id":1,"ok":false,"error":"unknown command"}')),
      "old",
    );
    eq(
      helloOutcome(
        parse(
          '{"id":null,"error":{"code":"protocol","message":"x","supported":[3]}}',
        ),
      ),
      "protocol",
    );
    const golden = session.find((m) => m.from === "daemon").msg;
    eq(helloOutcome(parse(JSON.stringify(golden))), "ok");
  },
};
```

Create `tests/session.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import {
  BACKOFF_FIRST,
  BACKOFF_MAX,
  HELLO_TIMEOUT,
  Session,
} from "../lib/session.mjs";

const HELLO_OK =
  '{"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}';

function ready() {
  const s = new Session("obsbot4linux-dux 1.0.0");
  s.start();
  s.connected();
  s.line(HELLO_OK);
  return s;
}

const kinds = (effects) => effects.map((e) => Object.keys(e)[0]);

export const tests = {
  // 4.1: the first message on a connection is hello, with subscribe:true
  "hello goes first, with the client name and the live list": () => {
    const s = new Session("obsbot4linux-dux 1.0.0");
    eq(s.start(), [{ connect: true }]);
    s.setLive(["pose"]);
    const effects = s.connected();
    eq(JSON.parse(effects[0].write), {
      id: 1,
      hello: {
        protocol: 2,
        client: "obsbot4linux-dux 1.0.0",
        subscribe: true,
        live: ["pose"],
      },
    });
    eq(effects[1], { helloTimeoutIn: HELLO_TIMEOUT });
  },
  "the hello reply makes the session ready": () => {
    const s = new Session("c");
    s.start();
    s.connected();
    const effects = s.line(HELLO_OK);
    eq(kinds(effects), ["ready"]);
    eq(s.phase, "ready");
  },
  // 4.1 Reconnect: 0.5 s, doubling, at most 5 s; reset once a hello succeeded
  "the backoff doubles from 0.5 s to 5 s and resets when ready": () => {
    const s = new Session("c");
    const waits = [];
    for (let i = 0; i < 6; i++) {
      s.start();
      waits.push(s.closed(2).find((e) => "retryIn" in e).retryIn);
    }
    eq(waits, [BACKOFF_FIRST, 1000, 2000, 4000, BACKOFF_MAX, BACKOFF_MAX]);
    s.start();
    s.connected();
    s.line(HELLO_OK);
    eq(s.backoff, BACKOFF_FIRST);
  },
  // 5.10: no socket file or connection refused: the systemctl hint
  "no daemon listening is unreachable": () => {
    const s = new Session("c");
    s.start();
    const effects = s.closed(0);
    ok(
      effects.some((e) => e.problem === "unreachable"),
      "unreachable",
    );
    eq(s.phase, "waiting");
  },
  "a connection that closes after hello is lost, and own values go": () => {
    const s = ready();
    const effects = s.closed(1);
    eq(kinds(effects), ["close", "lost", "problem", "retryIn"]);
    eq(s.problem, "lost");
  },
  "an older daemon is reported and retried at the slowest pace": () => {
    const s = new Session("c");
    s.start();
    s.connected();
    const effects = s.line('{"id":1,"ok":false,"error":"unknown command"}');
    ok(
      effects.some((e) => e.problem === "old"),
      "old",
    );
    ok(
      effects.some((e) => e.retryIn === BACKOFF_MAX),
      "5 s",
    );
  },
  "no hello reply within the client timeout is a lost daemon": () => {
    const s = new Session("c");
    s.start();
    s.connected();
    ok(
      s.helloTimeout().some((e) => e.problem === "lost"),
      "lost",
    );
    eq(ready().helloTimeout(), []);
  },
  // Requests die with their connection (4.1): nothing is queued while not ready
  "a request while not ready sends nothing": () => {
    const s = new Session("c");
    eq(
      s.request((id) => ({ id, live: [] })),
      { id: null, effects: [] },
    );
  },
  "request ids count up after the hello id": () => {
    const s = ready();
    eq(s.request((id) => ({ id, live: [] })).id, 2);
    eq(s.request((id) => ({ id, live: [] })).id, 3);
  },
  "events before the hello reply are ignored; replies and events pass when ready":
    () => {
      const s = new Session("c");
      s.start();
      s.connected();
      eq(s.line('{"event":"state","state":{}}'), []);
      s.line(HELLO_OK);
      eq(kinds(s.line('{"event":"devices","devices":[]}')), ["event"]);
      eq(kinds(s.line('{"id":2,"ok":{"rev":3}}')), ["reply"]);
      eq(s.line("not json"), []);
    },
  "the live list is sent only when it changes, and kept for the next hello":
    () => {
      const s = ready();
      eq(JSON.parse(s.setLive(["target", "pose", "pose"])[0].write).live, [
        "pose",
        "target",
      ]);
      eq(s.setLive(["pose", "target"]), []);
      s.closed(1);
      s.start();
      eq(JSON.parse(s.connected()[0].write).hello.live, ["pose", "target"]);
    },
  "stop closes an open connection and cancels a retry": () => {
    eq(kinds(ready().stop()), ["retryIn", "close", "lost"]);
    const s = new Session("c");
    s.start();
    s.closed(2);
    eq(s.stop(), [{ retryIn: -1 }]);
    eq(s.closed(1), []);
  },
};
```

- [ ] **Step 2: Run them and see them fail**

Run: `node tests/run.mjs tests/protocol.test.mjs tests/session.test.mjs`
Expected: exit 1, `ERR_MODULE_NOT_FOUND` for `lib/protocol.mjs`.

- [ ] **Step 3: Implement**

Create `lib/protocol.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Protocol 2 of obsbot4linux as the widget speaks it (the tool's docs/protocol-2.md): the
// socket path, the request lines and the parsing of what the daemon sends. No state here.

export const PROTOCOL = 2;

// The daemon's error codes (protocol 2, 4.1). Another code is read as "internal" (rule 7).
export const ERROR_CODES = [
  "protocol",
  "invalid",
  "unknown_device",
  "unsupported",
  "unsupported_device",
  "out_of_range",
  "conflict",
  "unavailable",
  "asleep",
  "busy",
  "camera",
  "internal",
];

// $OBSBOT4LINUX_SOCKET, else $XDG_RUNTIME_DIR/obsbot4linux.sock; "" when neither is set.
// env: name -> value, "" for an unset variable (Quickshell.env returns undefined for it).
export function socketPath(env) {
  const own = env("OBSBOT4LINUX_SOCKET") ?? "";
  if (own !== "") return own;
  const runtime = env("XDG_RUNTIME_DIR") ?? "";
  return runtime !== "" ? `${runtime}/obsbot4linux.sock` : "";
}

// One request object as one line for the socket.
export function encode(request) {
  return JSON.stringify(request) + "\n";
}

export function hello(id, client, live) {
  return {
    id,
    hello: { protocol: PROTOCOL, client, subscribe: true, live: live.slice() },
  };
}

export function setLeaves(id, device, patch) {
  return { id, device, set: Object.assign({}, patch) };
}

export function act(id, device, action, params) {
  const request = { id, device, do: action };
  if (params && Object.keys(params).length > 0)
    request.with = Object.assign({}, params);
  return request;
}

export function live(id, topics) {
  return { id, live: topics.slice() };
}

// One line from the daemon:
//   { kind: "reply", id, ok }                   ok: the reply's payload object
//   { kind: "error", id, code, message, body }  code: one of ERROR_CODES ("internal" for others)
//   { kind: "event", event, body }              event: "state", "devices" or a newer name
//   { kind: "bad", text }                       not a protocol-2 object (a 0.13 daemon's reply
//                                               {"ok":false,"error":"…"} included)
export function parse(line) {
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (e) {
    return { kind: "bad", text: String(line) };
  }
  if (msg === null || typeof msg !== "object" || Array.isArray(msg))
    return { kind: "bad", text: String(line) };
  if (typeof msg.event === "string")
    return { kind: "event", event: msg.event, body: msg };
  if (!("id" in msg)) return { kind: "bad", text: String(line) };
  if (msg.ok !== null && typeof msg.ok === "object" && !Array.isArray(msg.ok))
    return { kind: "reply", id: msg.id, ok: msg.ok };
  if (
    msg.error !== null &&
    typeof msg.error === "object" &&
    typeof msg.error.code === "string"
  ) {
    const code = ERROR_CODES.includes(msg.error.code)
      ? msg.error.code
      : "internal";
    return {
      kind: "error",
      id: msg.id,
      code,
      message: String(msg.error.message ?? ""),
      body: msg.error,
    };
  }
  return { kind: "bad", text: String(line) };
}

// The answer to our hello: "ok" (protocol 2), "protocol" (the daemon speaks another protocol:
// its error lists the ones it does), "old" (a daemon older than 1.0: its reply is no protocol-2
// object, 5.10).
export function helloOutcome(parsed) {
  if (
    parsed.kind === "reply" &&
    parsed.ok.protocol === PROTOCOL &&
    Array.isArray(parsed.ok.devices)
  )
    return "ok";
  if (parsed.kind === "error" && parsed.code === "protocol") return "protocol";
  return "old";
}
```

Create `lib/session.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// One connection to the daemon as a state machine, without I/O: the QML side (CameraLink.qml)
// feeds it what the socket did and carries out the effects it returns. Protocol 2, 4.1
// "Reconnect": a backoff from 0.5 s doubling up to 5 s; after any reconnect every request in
// flight is dropped and the client rebuilds from the hello reply.
//
// Effects, in the order to carry them out:
//   { connect: true }          open the socket
//   { close: true }            close the socket
//   { write: "<line>\n" }      write the line and flush
//   { retryIn: ms }            call retry() after ms (one timer; a new one replaces the old)
//   { helloTimeoutIn: ms }     call helloTimeout() after ms, unless ready() came first
//   { ready: <hello ok> }      the hello reply: rebuild devices and states from it
//   { reply: <parsed> }        a reply or error to a request of ours (protocol.parse shape)
//   { event: <parsed> }        an event (protocol.parse shape)
//   { lost: true }             the connection is gone: drop own values and requests in flight
//   { problem: "<kind>" }      what the UI shows while not ready: "unreachable" (no socket file
//                              or refused: the systemctl hint), "old" (a daemon older than 1.0),
//                              "protocol" (another protocol), "lost" (closed or no hello reply)

import {
  encode,
  hello,
  helloOutcome,
  live as liveRequest,
  parse,
} from "./protocol.mjs";

export const BACKOFF_FIRST = 500;
export const BACKOFF_MAX = 5000;
// CLIENT_REPLY_TIMEOUT of protocol 2 (5.8): no hello reply within it is a lost daemon
export const HELLO_TIMEOUT = 30000;

// QLocalSocket::LocalSocketError values that mean "no daemon listens"
const UNREACHABLE = [0, 2]; // ConnectionRefusedError, ServerNotFoundError

export class Session {
  constructor(client) {
    this.client = client;
    this.phase = "idle"; // idle | connecting | hello | ready | waiting
    this.problem = "";
    this.backoff = BACKOFF_FIRST;
    this.nextId = 1;
    this.helloId = 0;
    this.topics = [];
  }

  start() {
    if (this.phase !== "idle" && this.phase !== "waiting") return [];
    this.phase = "connecting";
    return [{ connect: true }];
  }

  // The socket is connected: hello goes first (4.1).
  connected() {
    this.phase = "hello";
    this.helloId = this.nextId++;
    return [
      { write: encode(hello(this.helloId, this.client, this.topics)) },
      { helloTimeoutIn: HELLO_TIMEOUT },
    ];
  }

  // One line from the socket.
  line(text) {
    const parsed = parse(text);
    if (this.phase === "hello") {
      if (parsed.kind === "event") return [];
      const ours =
        parsed.kind === "bad" ||
        parsed.id === this.helloId ||
        parsed.id === null;
      if (!ours) return [];
      const outcome = helloOutcome(parsed);
      if (outcome === "ok") {
        this.phase = "ready";
        this.problem = "";
        this.backoff = BACKOFF_FIRST;
        return [{ ready: parsed.ok }];
      }
      return this.fail(outcome, BACKOFF_MAX);
    }
    if (this.phase !== "ready") return [];
    if (parsed.kind === "event") return [{ event: parsed }];
    if (parsed.kind === "reply" || parsed.kind === "error")
      return [{ reply: parsed }];
    return [];
  }

  // The socket closed, or failed to connect (error: a QLocalSocket::LocalSocketError value,
  // undefined for a plain close).
  closed(error) {
    if (this.phase === "idle" || this.phase === "waiting") return [];
    const unreachable =
      this.phase === "connecting" && UNREACHABLE.includes(error);
    return this.fail(unreachable ? "unreachable" : "lost", this.backoff);
  }

  helloTimeout() {
    return this.phase === "hello" ? this.fail("lost", this.backoff) : [];
  }

  retry() {
    return this.start();
  }

  // Stop for good (the widget instance goes away or is switched off).
  stop() {
    const open = this.phase !== "idle" && this.phase !== "waiting";
    this.phase = "idle";
    this.problem = "";
    this.backoff = BACKOFF_FIRST;
    // retryIn -1: cancel a pending retry
    return open
      ? [{ retryIn: -1 }, { close: true }, { lost: true }]
      : [{ retryIn: -1 }];
  }

  // A request object without its id; returns { id, effects }. Not ready: no id, nothing sent
  // (requests die with their connection, 4.1).
  request(build) {
    if (this.phase !== "ready") return { id: null, effects: [] };
    const id = this.nextId++;
    return { id, effects: [{ write: encode(build(id)) }] };
  }

  // The live topics this connection wants (4.1 rule 8). Sent when ready and changed; kept
  // for the next hello either way.
  setLive(topics) {
    const next = Array.from(new Set(topics)).sort();
    if (next.join("\n") === this.topics.join("\n")) return [];
    this.topics = next;
    if (this.phase !== "ready") return [];
    return [{ write: encode(liveRequest(this.nextId++, next)) }];
  }

  fail(problem, wait) {
    const wasOpen =
      this.phase === "hello" ||
      this.phase === "ready" ||
      this.phase === "connecting";
    this.phase = "waiting";
    this.problem = problem;
    const effects = [];
    if (wasOpen) effects.push({ close: true });
    effects.push({ lost: true }, { problem }, { retryIn: wait });
    this.backoff = Math.min(this.backoff * 2, BACKOFF_MAX);
    return effects;
  }
}
```

- [ ] **Step 4: Run them and see them pass**

Run: `node tests/run.mjs tests/protocol.test.mjs tests/session.test.mjs && scripts/check-js.sh && bash scripts/check-qml.sh`
Expected: `23 passed, 0 failed`; `check-js: ok (30 passed, 0 failed in node and in qml)` (Task 4 landed first: 76); `check-qml: ok`.

- [ ] **Step 5: CHANGELOG and commit**

`CHANGELOG.md`, `## [Unreleased]`, `### Added`: `- Protocol 2 of obsbot4linux 1.0 for the widget: its request lines, the parsing of the daemon's lines and the connection with its reconnect backoff, as tested modules (#N).`

```bash
git add lib/protocol.mjs lib/session.mjs tests/protocol.test.mjs tests/session.test.mjs CHANGELOG.md
git commit -S -m "Protocol 2: request lines, parsing and the connection state machine"
```

---

### Task 4: Devices, description, values, text, arrows, lists, labels

**Files:**

- Create: `lib/devices.mjs`, `lib/describe.mjs`, `lib/values.mjs`, `lib/format.mjs`, `lib/gimbal.mjs`, `lib/lists.mjs`, `lib/labels.mjs`
- Test: `tests/devices.test.mjs`, `tests/describe.test.mjs`, `tests/values.test.mjs`, `tests/lists.test.mjs`, `tests/labels.test.mjs`
- Modify: `CHANGELOG.md`

**Interfaces:**

- Consumes: Task 2's harness and fixtures (`fakes.mjs`, `tiny2lite.description.mjs`, `session.transcript.mjs`, `hotplug.transcript.mjs`).
- Produces (Tasks 5–15 use these names):
  - `lib/devices.mjs`: `ISSUES`; `usable(device) -> bool`; `anyUsable(devices) -> bool`; `marker(device) -> string` (`""` for tested); `unsupportedHint(device) -> string`; `class DeviceStore` with `devices: Device[]`, `states: {id: {rev, state}}`, `reset()`, `fromHello(ok)`, `onDevices(list) -> removedIds[]`, `onState({device, rev, state}) -> bool`, `device(id)`, `state(id)`, `rev(id)`; `selection(devices, current) -> id | null`.
  - `lib/describe.mjs`: `TYPES`; `setting(device, path)`, `action(device, name)`; `has(device, pathOrGroup) -> bool`; `writable(device, path)`; `choices(device, path, labels) -> [{value, label, selectable}]`; `humanize(value)`; `numberSpec(setting, component?) -> {min, max, step, precision, unit} | null`; `decimals(precision)`; `roundTo(value, precision)`; `sliderScale(spec) -> {count, toValue(i), toIndex(v)} | null`; `at(state, path)`; `blocked(device, state, path) -> null | {reason: "unavailable" | "readonly" | "asleep" | "requires", needs?}`; `warnings(device, path) -> string[]`; `limit(device, path) -> number | null`; `topics(device) -> string[]`; `wantedTopics(device, {open, pose, target, auto}) -> string[]`; `sameValue(a, b)`.
  - `lib/values.mjs`: `class OwnValues` with `add(id, patch)`, `settle(id) -> bool`, `clear()`, `value(path)`, `size`; `shown(state, own, path)`; `status(state, own, path) -> "own" | "pending" | "unread" | "camera"`; `changed(state, own, edits) -> patch | null`; `defaults(device, state, own, paths) -> patch | null`.
  - `lib/format.mjs`: `unitText(unit)`, `number(value, precision, unit)`, `value(setting, v, labels?)`.
  - `lib/gimbal.mjs`: `STEP_BASE_MS = 2000`; `moveSpec(device) -> {pan, tilt} | null`; `stepTarget(spec, pose, last, now, dPan, dTilt) -> {pan, tilt} | null` (`null` while the base has no numeric pan and tilt); `moveRequest(target) -> {action: "gimbal.move", params: {pan, tilt}}`; `panLeftSign(state) -> 1 | -1`.
  - `lib/lists.mjs`: `UNNAMED = "(unnamed)"`; `presetSlots(device) -> number[]`; `presetRows(device, state) -> [{slot, used, name, label, pose}] | null`; `viewRows(state) -> [{id, name, label, pose}] | null`; `saveParams(slot, typed) -> {slot} | {slot, name}`; `nameProblem(device, action, param, text) -> string | null`.
  - `lib/labels.mjs`: `labelsFor(path) -> {value: name}`; `labelledPaths() -> string[]`; `problemText(problem) -> {title, body} | null`.

- [ ] **Step 1: Write the failing tests**

Create `tests/devices.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import {
  anyUsable,
  DeviceStore,
  marker,
  selection,
  unsupportedHint,
  usable,
} from "../lib/devices.mjs";
import { A, B, HELLO, UNKNOWN } from "./fixtures/fakes.mjs";
import hotplug from "./fixtures/hotplug.transcript.mjs";

const off = (d) => Object.assign({}, d, { available: false });

export const tests = {
  "markers: none for tested, the level for the others": () => {
    eq([A, B, UNKNOWN].map(marker), ["", "untested", "unsupported"]);
    eq(marker(Object.assign({}, A, { support: "beta" })), "beta");
  },
  // 6.2: unsupported devices never count as usable; unplugged ones neither
  "usable: a supported model that is plugged in": () => {
    eq([A, B, UNKNOWN].map(usable), [true, true, false]);
    eq(usable(off(A)), false);
  },
  // "Hide without camera": the pill stays while a usable camera is listed, picked or not
  // (two usable cameras: nothing is picked, 8.4 step 7, and the picker is in the popout)
  "anyUsable: some listed device is usable, selected or not": () => {
    eq(anyUsable([A, B, UNKNOWN]), true);
    eq(anyUsable([UNKNOWN]), false);
    eq(anyUsable([off(A)]), false);
    eq(anyUsable([]), false);
  },
  // 8.4 step 3: the unsupported device shows the report hint instead of controls
  "the unsupported hint names the device, its USB ids and the report command":
    () => {
      const text = unsupportedHint(UNKNOWN);
      ok(text.includes("Unknown Camera 9-3 (USB 3564:fa00)"), text);
      ok(text.includes("obsbot4linux devices --report"), text);
      ok(
        text.includes("https://github.com/hellwand/obsbot4linux/issues"),
        text,
      );
    },
  // 8.4 step 7 and 6.2: the only usable device, or none; unsupported never auto-selected
  "selection: only one usable device is chosen by itself": () => {
    eq(selection([A, UNKNOWN], null), A.id);
    eq(selection([A, B, UNKNOWN], null), null);
    eq(selection([UNKNOWN], null), null);
    eq(selection([A, B, UNKNOWN], B.id), B.id);
    eq(selection([A, UNKNOWN], UNKNOWN.id), UNKNOWN.id);
  },
  "selection: the selected unsupported device goes, the only usable one takes over":
    () => {
      eq(selection([A], UNKNOWN.id), A.id);
      eq(selection([A, B], UNKNOWN.id), null);
    },
  "selection: an unplugged sn: device stays selected": () => {
    eq(selection([off(A), B], A.id), A.id);
  },
  // 5.5: clients move their selection on a hand-over
  "selection follows replaced_by": () => {
    const port = Object.assign({}, A, {
      id: "port:9-1",
      available: false,
      replaced_by: A.id,
    });
    eq(selection([A, port], "port:9-1"), A.id);
  },
  "the hello reply fills devices and states": () => {
    const store = new DeviceStore();
    store.fromHello(HELLO);
    eq(
      store.devices.map((d) => d.id),
      [A.id, B.id, UNKNOWN.id],
    );
    eq(store.rev(B.id), 1);
    eq(store.state(UNKNOWN.id), { available: true, pending: {}, unread: [] });
  },
  // 4.1 rule 1: a rev never goes back; equal revs are not applied twice
  "a state event applies only with a newer rev": () => {
    const store = new DeviceStore();
    store.fromHello(HELLO);
    ok(
      store.onState({
        device: B.id,
        rev: 2,
        state: {
          available: true,
          pending: {},
          unread: [],
          zoom: { factor: 2 },
        },
      }),
      "rev 2",
    );
    ok(!store.onState({ device: B.id, rev: 2, state: {} }), "rev 2 again");
    ok(!store.onState({ device: B.id, rev: 1, state: {} }), "rev 1");
    eq(store.state(B.id).zoom.factor, 2);
    ok(
      !store.onState({ device: "sn:NOTLISTED001", rev: 9, state: {} }),
      "unlisted device",
    );
  },
  // 4.1 rule 1: a port: entry added again starts at rev 1; clients forget dropped revs
  "a removed entry's rev is forgotten, so its return at rev 1 applies": () => {
    const store = new DeviceStore();
    store.fromHello(HELLO);
    store.onState({
      device: UNKNOWN.id,
      rev: 1,
      state: { available: true, pending: {}, unread: [] },
    });
    eq(store.onDevices([A, B]), [UNKNOWN.id]);
    eq(store.rev(UNKNOWN.id), 0);
    store.onDevices([A, B, UNKNOWN]);
    ok(
      store.onState({
        device: UNKNOWN.id,
        rev: 1,
        state: { available: true, pending: {}, unread: [] },
      }),
      "rev 1 again",
    );
  },
  // The golden hotplug transcript, replayed: the hand-over leaves the sn: entry selected,
  // its rev above both entries
  "the golden hotplug transcript ends on the sn: entry with its newest state":
    () => {
      const store = new DeviceStore();
      let selected = null;
      for (const m of hotplug.filter((x) => x.from === "daemon")) {
        if (m.msg.event === "devices") {
          store.onDevices(m.msg.devices);
          selected = selection(store.devices, selected);
        } else if (m.msg.event === "state") {
          store.onState(m.msg);
        }
      }
      eq(selected, "sn:EXAMPLE0000001");
      eq(
        store.devices.map((d) => d.id),
        ["sn:EXAMPLE0000001"],
      );
      eq(store.rev("sn:EXAMPLE0000001"), 9);
      eq(store.state("sn:EXAMPLE0000001").available, true);
    },
};
```

Create `tests/describe.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import {
  blocked,
  choices,
  decimals,
  has,
  humanize,
  limit,
  numberSpec,
  roundTo,
  setting,
  sliderScale,
  topics,
  wantedTopics,
  warnings,
  writable,
} from "../lib/describe.mjs";
import { A, B, STATES, UNKNOWN } from "./fixtures/fakes.mjs";

const TRACKING_LABELS = {
  static: "Static",
  normal: "Normal Tracking",
  upper_body: "Upper Body",
};
const stateOf = (d, extra) =>
  Object.assign({}, STATES[d.id].state, extra ?? {});

export const tests = {
  // 10.3 D: fake-b's missing groups hide their controls
  "fake-b has no gimbal and no views; fake-a has both": () => {
    ok(has(A, "gimbal") && has(A, "views") && has(A, "gimbal.move"), "fake-a");
    ok(
      !has(B, "gimbal") && !has(B, "views") && !has(B, "gimbal.move"),
      "fake-b",
    );
    ok(
      has(B, "tracking.mode") &&
        has(B, "tracking") &&
        !has(B, "tracking.profile"),
      "fake-b tracking",
    );
    ok(!has(B, "gim") && !has(B, ""), "a group is a whole path segment");
    ok(!has(UNKNOWN, "tracking"), "unsupported: nothing");
  },
  // 10.3 D: fake-b's own enum values, not the Tiny 2 Lite's
  "choices come from the description; unknown values get a text of their own":
    () => {
      eq(
        choices(B, "tracking.mode", TRACKING_LABELS).map((c) => [
          c.value,
          c.label,
        ]),
        [
          ["off", "Off"],
          ["person", "Person"],
          ["desk", "Desk"],
        ],
      );
      eq(
        choices(A, "tracking.mode", TRACKING_LABELS)
          .slice(0, 3)
          .map((c) => c.label),
        ["Static", "Normal Tracking", "Upper Body"],
      );
    },
  "values outside writable are shown but not selectable": () => {
    const fov = choices(A, "zoom.field_of_view", {});
    eq(
      fov.map((c) => [c.value, c.selectable]),
      [
        ["wide", true],
        ["medium", true],
        ["narrow", true],
        ["custom", false],
      ],
    );
  },
  // 10.3 D: fake-b's own bounds
  "number bounds come from the description": () => {
    eq(numberSpec(setting(B, "zoom.factor")), {
      min: 1,
      max: 2,
      step: 0.01,
      precision: 0.01,
      unit: "factor",
    });
    eq(numberSpec(setting(A, "zoom.factor")).max, 4);
    eq(numberSpec(setting(B, "led.brightness")), {
      min: 0,
      max: 5,
      step: 1,
      precision: 1,
      unit: null,
    });
  },
  "range bounds per component": () => {
    const pan = setting(A, "zone.area.pan");
    eq(numberSpec(pan, "min"), {
      min: -150,
      max: 150,
      step: 0.1,
      precision: 0.1,
      unit: "deg",
    });
  },
  // A slider's positions land on the step and never outside the bounds (W0 review focus 1)
  "a slider scale maps positions to stepped values": () => {
    const zoom = sliderScale(numberSpec(setting(B, "zoom.factor")));
    eq(
      [
        zoom.count,
        zoom.toValue(0),
        zoom.toValue(37),
        zoom.toValue(100),
        zoom.toValue(500),
      ],
      [100, 1, 1.37, 2, 2],
    );
    eq(zoom.toIndex(1.5), 50);
    const ratio = sliderScale(numberSpec(setting(A, "gestures.zoom_ratio")));
    eq([ratio.count, ratio.toValue(3)], [30, 1.3]);
    eq(sliderScale(numberSpec(setting(A, "hand.return_after"))), null);
  },
  "rounding and decimals": () => {
    eq(
      [decimals(0.01), decimals(0.1), decimals(1), decimals(undefined)],
      [2, 1, 0, 0],
    );
    eq(roundTo(0.1 + 0.2, 0.1), 0.3);
    eq(humanize("upper_body"), "Upper body");
    eq(humanize("raw:7"), "raw:7");
  },
  "writable: read-only leaves and unknown types are not": () => {
    ok(writable(A, "led.brightness"), "led");
    ok(!writable(A, "gimbal.pan"), "pose is read-only");
    const odd = Object.assign({}, A, {
      settings: {
        "image.matrix": { type: "matrix", label: "M", readonly: false },
      },
    });
    ok(!writable(odd, "image.matrix"), "unknown type");
  },
  // 4.2 requires: the control is blocked; the widget never adds the needed leaf itself
  "a requires that does not hold blocks the control and names the need": () => {
    eq(blocked(B, stateOf(B), "exposure.time"), {
      reason: "requires",
      needs: [{ path: "exposure.mode", equals: "manual" }],
    });
    eq(
      blocked(
        B,
        stateOf(B, { exposure: { mode: "manual", time: 100 } }),
        "exposure.time",
      ),
      null,
    );
  },
  // 5.7: asleep: ignored is refused by the daemon; the control is off while asleep
  "asleep blocks only what the description marks ignored": () => {
    const asleep = stateOf(B, { asleep: true });
    eq(blocked(B, asleep, "zoom.factor"), { reason: "asleep" });
    eq(blocked(B, asleep, "led.brightness"), null);
    eq(blocked(B, asleep, "preset.save"), { reason: "asleep" });
    eq(blocked(B, asleep, "wake"), null);
  },
  "an unavailable or unsupported device blocks everything": () => {
    eq(
      blocked(
        Object.assign({}, B, { available: false }),
        stateOf(B),
        "led.brightness",
      ),
      { reason: "unavailable" },
    );
    eq(blocked(UNKNOWN, STATES[UNKNOWN.id].state, "led.brightness"), {
      reason: "unavailable",
    });
    eq(blocked(B, stateOf(B), "gimbal.move"), { reason: "unavailable" });
    eq(blocked(A, stateOf(A), "gimbal.pan"), { reason: "readonly" });
  },
  "warnings to confirm": () => {
    eq(warnings(A, "factory_reset"), ["destructive", "restart"]);
    eq(warnings(A, "audio.enabled"), ["restart"]);
    eq(warnings(A, "led.brightness"), []);
  },
  // 10.3 D: fake-b's preset count
  "list limits": () => {
    eq(
      [limit(A, "presets"), limit(B, "presets"), limit(A, "views")],
      [3, 5, null],
    );
  },
  // 10.3 D: live topics only while the popout is open
  "live topics: none while closed, only the shown and named ones while open":
    () => {
      eq(topics(A), ["auto", "pose", "target"]);
      eq(topics(B), ["auto"]);
      eq(
        wantedTopics(A, { open: false, pose: true, target: true, auto: true }),
        [],
      );
      eq(
        wantedTopics(A, { open: true, pose: true, target: false, auto: true }),
        ["auto", "pose"],
      );
      eq(
        wantedTopics(B, { open: true, pose: true, target: true, auto: true }),
        ["auto"],
      );
    },
};
```

Create `tests/values.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import { changed, defaults, OwnValues, shown, status } from "../lib/values.mjs";
import { number, unitText, value } from "../lib/format.mjs";
import { setting } from "../lib/describe.mjs";
import { A, B, STATES } from "./fixtures/fakes.mjs";
import session from "./fixtures/session.transcript.mjs";

const IMAGE = [
  "image.brightness",
  "image.contrast",
  "image.saturation",
  "image.hue",
  "image.sharpness",
  "image.white_balance.auto",
  "image.white_balance.temperature",
  "exposure.compensation",
];

export const tests = {
  // 4.1 rule 4: own value from sending until the reply, then the state with its pending values
  "own value, then pending, then the camera's value": () => {
    const own = new OwnValues();
    const before = {
      available: true,
      pending: {},
      unread: [],
      led: { brightness: 1 },
    };
    own.add(7, { "led.brightness": 3 });
    eq(
      [
        shown(before, own, "led.brightness"),
        status(before, own, "led.brightness"),
      ],
      [3, "own"],
    );
    const pendingState = {
      available: true,
      pending: { "led.brightness": 3 },
      unread: [],
      led: { brightness: 1 },
    };
    own.settle(7);
    eq(
      [
        shown(pendingState, own, "led.brightness"),
        status(pendingState, own, "led.brightness"),
      ],
      [3, "pending"],
    );
    const after = {
      available: true,
      pending: {},
      unread: [],
      led: { brightness: 3 },
    };
    eq(
      [
        shown(after, own, "led.brightness"),
        status(after, own, "led.brightness"),
      ],
      [3, "camera"],
    );
  },
  // 10.3 D: a value set by another client (the CLI) shows at once through pending
  "a write of another client shows at once through pending": () => {
    const lines = session.filter(
      (m) => m.from === "daemon" && m.msg.event === "state",
    );
    const first = lines[0].msg.state;
    eq(first.led.brightness, 1);
    eq(shown(first, new OwnValues(), "led.brightness"), 2);
    eq(status(first, new OwnValues(), "led.brightness"), "pending");
  },
  "the newest own value wins; an error takes back only its own": () => {
    const own = new OwnValues();
    own.add(4, { "zoom.factor": 1.5 });
    own.add(5, { "zoom.factor": 1.8 });
    eq(own.value("zoom.factor"), 1.8);
    own.settle(5);
    eq(own.value("zoom.factor"), 1.5);
    own.clear();
    eq(own.value("zoom.factor"), undefined);
  },
  "an unread leaf is null and says so": () => {
    const s = {
      available: true,
      pending: {},
      unread: ["presets"],
      presets: null,
    };
    eq(
      [shown(s, null, "presets"), status(s, null, "presets")],
      [null, "unread"],
    );
    eq(shown(s, null, "not.there"), null);
  },
  // One control, one request, only what the user changed (spec 2)
  "an area edit sends only the leaves that changed": () => {
    const s = STATES[A.id].state;
    eq(
      changed(s, null, {
        "zone.area.pan": s.zone.area.pan,
        "zone.area.tilt": { min: -20, max: 30 },
      }),
      { "zone.area.tilt": { min: -20, max: 30 } },
    );
    eq(changed(s, null, { "zone.area.pan": s.zone.area.pan }), null);
  },
  // 10.3 D defaults reset: one patch of defaults; leaves with requires stay out, so the
  // daemon cannot refuse the whole patch with conflict
  "the defaults reset patch": () => {
    const s = Object.assign({}, STATES[A.id].state, {
      image: Object.assign({}, STATES[A.id].state.image, {
        brightness: 80,
        white_balance: { auto: false, temperature: 4000 },
      }),
    });
    eq(defaults(A, s, null, IMAGE), {
      "image.brightness": 50,
      "image.white_balance.auto": true,
    });
    eq(defaults(A, STATES[A.id].state, null, IMAGE), null);
    eq(defaults(B, STATES[B.id].state, null, IMAGE), null);
  },
  "numbers with units and precision": () => {
    eq(number(-12.54, 0.1, "deg"), "−12.5°");
    eq(number(1.5, 0.01, "factor"), "1.50×");
    eq(number(33000, 1, "us"), "33000 µs");
    eq(unitText("lux"), " lux");
  },
  "values by type; raw and unknown types as text": () => {
    eq(value(setting(A, "image.mirror"), true), "On");
    eq(
      value(setting(A, "tracking.mode"), "upper_body", {
        upper_body: "Upper Body",
      }),
      "Upper Body",
    );
    eq(value(setting(B, "tracking.mode"), "person"), "Person");
    eq(value(setting(A, "tracking.mode"), "raw:9"), "raw:9");
    eq(
      value(setting(A, "zone.area.pan"), { min: -20, max: 20 }),
      "−20.0…20.0°",
    );
    eq(value(setting(A, "hand.return"), null), "–");
    eq(value({ type: "matrix" }, [1, 2]), "[1,2]");
  },
};
```

Create `tests/lists.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import {
  moveRequest,
  moveSpec,
  panLeftSign,
  stepTarget,
} from "../lib/gimbal.mjs";
import {
  nameProblem,
  presetRows,
  presetSlots,
  saveParams,
  UNNAMED,
  viewRows,
} from "../lib/lists.mjs";
import { A, B, STATES } from "./fixtures/fakes.mjs";

export const tests = {
  // 10.3 D: fake-b's preset count (5), not the Tiny 2 Lite's (3)
  "preset slots follow the description": () => {
    eq(presetSlots(A), [0, 1, 2]);
    eq(presetSlots(B), [0, 1, 2, 3, 4]);
    eq(presetSlots({ settings: {}, actions: {}, limits: {} }), []);
  },
  // 12.3: an empty name shows a placeholder of the widget's own, as display only
  "preset rows: used, unnamed, empty; null while unread": () => {
    const s = {
      available: true,
      pending: {},
      unread: [],
      presets: [
        { slot: 0, name: "Desk", pose: { pan: 1, tilt: 2, zoom: 1 } },
        { slot: 2, name: "", pose: null },
      ],
    };
    eq(
      presetRows(A, s).map((r) => [r.slot, r.used, r.label]),
      [
        [0, true, "Desk"],
        [1, false, "Empty"],
        [2, true, UNNAMED],
      ],
    );
    eq(
      presetRows(A, {
        available: true,
        pending: {},
        unread: ["presets"],
        presets: null,
      }),
      null,
    );
    eq(presetRows(B, STATES[B.id].state).length, 5);
  },
  // 12.3: saving a preset sends no name unless the user typed one
  "preset.save carries a name only when one was typed": () => {
    eq(saveParams(1, ""), { slot: 1 });
    eq(saveParams(1, "Desk"), { slot: 1, name: "Desk" });
    eq(saveParams(1, " "), { slot: 1, name: " " });
  },
  "names are checked against the description's bounds": () => {
    eq(nameProblem(B, "preset.save", "name", "x".repeat(16)), null);
    eq(
      nameProblem(B, "preset.save", "name", "x".repeat(17)),
      "at most 16 characters",
    );
    eq(
      nameProblem(A, "preset.rename", "name", "Büro"),
      "letters, digits and punctuation without accents only",
    );
    eq(nameProblem(A, "view.save", "name", ""), "a name is needed");
    eq(nameProblem(A, "preset.save", "name", ""), null);
    eq(nameProblem(B, "view.save", "name", "x"), "not offered by this camera");
  },
  "views: rows, or null while unread": () => {
    eq(
      viewRows({ unread: [], views: [{ id: 3, name: "Board", pose: null }] }),
      [{ id: 3, name: "Board", label: "Board", pose: null }],
    );
    eq(viewRows({ unread: ["views"], views: null }), null);
  },
  "gimbal: no gimbal.move, no arrows": () => {
    eq(moveSpec(B), null);
    eq(moveSpec(A).pan, {
      min: -150,
      max: 150,
      step: 0.1,
      precision: 0.1,
      unit: "deg",
    });
  },
  "a step builds on the live pose, or on the last own target within 2 s; none without either":
    () => {
      const spec = moveSpec(A);
      eq(stepTarget(spec, { pan: 10, tilt: 0 }, null, 0, 5, 0), {
        pan: 15,
        tilt: 0,
      });
      eq(
        stepTarget(
          spec,
          { pan: 10, tilt: 0 },
          { pan: 20, tilt: 5, at: 1000 },
          2500,
          5,
          0,
        ),
        { pan: 25, tilt: 5 },
      );
      eq(
        stepTarget(
          spec,
          { pan: 10, tilt: 0 },
          { pan: 20, tilt: 5, at: 1000 },
          3000,
          5,
          0,
        ),
        { pan: 15, tilt: 0 },
      );
      eq(stepTarget(spec, { pan: 148, tilt: 88 }, null, 0, 5, 5), {
        pan: 150,
        tilt: 90,
      });
      // no known pose and no recent own target: no step (the widget never makes a pose up)
      eq(stepTarget(spec, null, null, 0, -5, 0), null);
      eq(stepTarget(spec, { pan: null, tilt: null }, null, 0, 5, 0), null);
      eq(
        stepTarget(
          spec,
          { pan: null, tilt: null },
          { pan: 20, tilt: 5, at: 1000 },
          2500,
          5,
          0,
        ),
        { pan: 25, tilt: 5 },
      );
    },
  // spec 2, 12.3: a gimbal control sends gimbal.move alone, never a tracking switch
  "a gimbal step is one gimbal.move with pan and tilt": () => {
    eq(moveRequest({ pan: 15, tilt: 0 }), {
      action: "gimbal.move",
      params: { pan: 15, tilt: 0 },
    });
  },
  "the arrows follow the picture": () => {
    eq(panLeftSign({ image: { mirror: true } }), -1);
    eq(panLeftSign(STATES[A.id].state), 1);
    ok(panLeftSign(null) === 1, "no state");
  },
};
```

Create `tests/labels.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import { labelledPaths, labelsFor, problemText } from "../lib/labels.mjs";
import { choices } from "../lib/describe.mjs";
import tiny2lite from "./fixtures/tiny2lite.description.mjs";
import { B } from "./fixtures/fakes.mjs";

const lite = Object.assign({ available: true, support: "tested" }, tiny2lite);

export const tests = {
  // A label names a value the description has; a typo would silently never show
  "every labelled value is a value of the Tiny 2 Lite's description": () => {
    for (const path of labelledPaths()) {
      const values = lite.settings[path]?.values ?? [];
      ok(values.length > 0, `${path} is an enum of the description`);
      for (const value of Object.keys(labelsFor(path)))
        ok(values.includes(value), `${path}: ${value} is in values`);
    }
  },
  "the description decides the chips; the labels only name them": () => {
    eq(
      choices(B, "tracking.mode", labelsFor("tracking.mode")).map(
        (c) => c.label,
      ),
      ["Off", "Person", "Desk"],
    );
    eq(labelsFor("not.labelled"), {});
  },
  // 5.10 and 6.2: the exact command that starts the daemon
  "the unreachable hint carries the systemctl line": () => {
    ok(
      problemText("unreachable").body.includes(
        "systemctl --user enable --now obsbot4linux.socket",
      ),
      "systemctl line",
    );
    eq(
      problemText("old").body,
      "an obsbot4linux daemon older than 1.0 is running: restart it or log out",
    );
    eq(problemText(""), null);
  },
};
```

- [ ] **Step 2: Run them and see them fail**

Run: `node tests/run.mjs tests/devices.test.mjs tests/describe.test.mjs tests/values.test.mjs tests/lists.test.mjs tests/labels.test.mjs`
Expected: exit 1, `ERR_MODULE_NOT_FOUND` for the first missing `lib/` module.

- [ ] **Step 3: Implement**

Create `lib/describe.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// What a device's description (protocol 2, 4.2) allows: every control of the widget asks here
// whether it is shown, which values it offers, its bounds, step and unit, and whether it is
// blocked right now. Nothing here knows a model.

export const TYPES = [
  "bool",
  "enum",
  "number",
  "range",
  "pose",
  "point",
  "box",
  "string",
  "list",
];

export function setting(device, path) {
  return device?.settings?.[path] ?? null;
}

export function action(device, name) {
  return device?.actions?.[name] ?? null;
}

// A leaf, an action, or a group (a prefix of a leaf or action path) of the description.
export function has(device, path) {
  if (setting(device, path) || action(device, path)) return true;
  const prefix = path + ".";
  const names = Object.keys(device?.settings ?? {}).concat(
    Object.keys(device?.actions ?? {}),
  );
  return names.some((n) => n.startsWith(prefix));
}

// A leaf of a type this widget knows and the daemon accepts writes for. A setting of an unknown
// type is shown read-only as text (rule 7).
export function writable(device, path) {
  const s = setting(device, path);
  return s !== null && !s.readonly && TYPES.includes(s.type);
}

// The values an enum shows, each { value, label, selectable }: every value of `values`, in its
// order; selectable when `writable` holds it (all are, when `writable` is missing). labels: the
// widget's own names for values it knows; another value shows as text made from it.
export function choices(device, path, labels) {
  const s = setting(device, path);
  if (!s || s.type !== "enum") return [];
  const writable = Array.isArray(s.writable) ? s.writable : s.values;
  return (s.values ?? []).map((value) => ({
    value,
    label: labels?.[value] ?? humanize(value),
    selectable: !s.readonly && writable.includes(value),
  }));
}

// "upper_body" -> "Upper body"; "raw:7" stays "raw:7"
export function humanize(value) {
  const text = String(value);
  if (/^raw:-?\d+$/.test(text)) return text;
  const words = text.replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function bound(b, key) {
  if (b === undefined || b === null) return null;
  return typeof b === "number" ? b : (b[key] ?? null);
}

// Bounds of a number, or of one component of a range, pose, point or box (component: "min",
// "max", "pan", …): { min, max, step, precision, unit }; missing parts are null. step: the
// description's step, else its precision.
export function numberSpec(s, component) {
  if (!s) return null;
  const unit =
    typeof s.unit === "object" && s.unit !== null
      ? (s.unit[component] ?? null)
      : (s.unit ?? null);
  const precision = bound(s.precision, component);
  const step = bound(s.step, component) ?? precision;
  if (component === undefined || s.type === "number")
    return { min: s.min ?? null, max: s.max ?? null, step, precision, unit };
  return {
    min: bound(s.min, component),
    max: bound(s.max, component),
    step,
    precision,
    unit,
  };
}

// Decimal places of a precision (0.01 -> 2, 1 -> 0).
export function decimals(precision) {
  if (!precision || precision >= 1) return 0;
  return Math.min(6, Math.max(0, Math.round(-Math.log10(precision))));
}

export function roundTo(value, precision) {
  if (!precision) return value;
  const places = decimals(precision);
  const n = Math.round(value / precision) * precision;
  return Number(n.toFixed(places));
}

// A slider over [min, max] in steps: positions 0..count. null without both bounds or a step.
export function sliderScale(spec) {
  if (!spec || spec.min === null || spec.max === null || !spec.step)
    return null;
  const count = Math.max(0, Math.round((spec.max - spec.min) / spec.step));
  const precision = spec.precision ?? spec.step;
  return {
    count,
    toValue: (i) =>
      roundTo(
        Math.min(
          spec.max,
          spec.min + Math.max(0, Math.min(count, i)) * spec.step,
        ),
        precision,
      ),
    toIndex: (v) =>
      Math.max(0, Math.min(count, Math.round((v - spec.min) / spec.step))),
  };
}

// The value at a dotted path of a state (its nested tree, 4.3); undefined when absent.
export function at(state, path) {
  let node = state;
  for (const key of String(path).split(".")) {
    if (node === null || typeof node !== "object" || !(key in node))
      return undefined;
    node = node[key];
  }
  return node;
}

// Why a control for a leaf or action cannot be used now, or null when it can. state: the
// device's published state (what the daemon checks a request against, 4.1 rule 5).
//   { reason: "unavailable" }                   the device is unplugged or unsupported
//   { reason: "readonly" }                      the leaf is read-only or of an unknown type
//   { reason: "asleep" }                        asleep, and the description says it is ignored
//   { reason: "requires", needs: [{path, equals}] } a requires condition does not hold
export function blocked(device, state, path) {
  const s = setting(device, path);
  const a = s ? null : action(device, path);
  if (
    !device ||
    device.available !== true ||
    device.support === "unsupported" ||
    (!s && !a)
  )
    return { reason: "unavailable" };
  if (s && !writable(device, path)) return { reason: "readonly" };
  const spec = s ?? a;
  if (state?.asleep === true && spec.asleep === "ignored")
    return { reason: "asleep" };
  const needs = (spec.requires ?? []).filter(
    (r) => !sameValue(at(state, r.path), r.equals),
  );
  if (needs.length > 0) return { reason: "requires", needs };
  return null;
}

// The warnings a control asks the user to confirm before it sends (4.2 `warning`: "restart",
// "destructive"); [] when there is nothing to ask.
export function warnings(device, path) {
  const spec = setting(device, path) ?? action(device, path);
  return Array.isArray(spec?.warning) ? spec.warning.slice() : [];
}

// The maximum item count of a list (4.2 `limits`), or null when the camera has none.
export function limit(device, path) {
  const n = device?.limits?.[path];
  return typeof n === "number" ? n : null;
}

// The live topics a device's description names (4.2 `live`).
export function topics(device) {
  const all = new Set();
  for (const s of Object.values(device?.settings ?? {})) {
    if (typeof s.live === "string") all.add(s.live);
  }
  return Array.from(all).sort();
}

// Equality of two protocol values: numbers by value, objects and arrays by their parts.
export function sameValue(a, b) {
  if (a === b) return true;
  if (typeof a === "number" && typeof b === "number")
    return Math.abs(a - b) < 1e-9;
  if (
    a === null ||
    b === null ||
    typeof a !== "object" ||
    typeof b !== "object"
  )
    return false;
  const ka = Object.keys(a);
  const kb = Object.keys(b);
  return ka.length === kb.length && ka.every((k) => sameValue(a[k], b[k]));
}

// The live topics the widget asks for (4.1 rule 8): none while the popout is closed; while it
// is open, those of the shown parts that the device's description names. view: { open, pose,
// target, auto }, each a bool: the popout is open, and it shows the live pose, the target box,
// the values the camera adjusts itself.
export function wantedTopics(device, view) {
  if (!view.open) return [];
  const named = topics(device);
  return ["auto", "pose", "target"].filter(
    (t) => view[t] === true && named.includes(t),
  );
}
```

Create `lib/devices.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The device list and the last state of each device, as the hello reply and the devices and
// state events give them (protocol 2, 4.1 "Events" and rule 1), and which device the widget
// shows (8.4 step 7).

export const ISSUES = "https://github.com/hellwand/obsbot4linux/issues";

// A device the widget can control: a supported model (tested or untested) that is plugged in.
export function usable(device) {
  return (
    device.available === true &&
    (device.support === "tested" || device.support === "untested")
  );
}

// Whether any listed device is usable, selected or not ("Hide without camera" keeps the pill
// while the user still has to pick one).
export function anyUsable(devices) {
  return devices.some(usable);
}

// The marker next to a device's name: "" for a tested model, else its support level as the
// daemon names it ("untested", "unsupported", or a newer level as raw text).
export function marker(device) {
  return device.support === "tested" ? "" : String(device.support);
}

// What the widget shows for an unsupported device instead of controls (8.4 step 3).
export function unsupportedHint(device) {
  const usb = device.usb ?? {};
  return (
    `${device.name} (USB ${usb.vendor}:${usb.product}) is not supported by obsbot4linux. ` +
    `\`obsbot4linux devices --report\` prints what an issue report needs: ${ISSUES}`
  );
}

export class DeviceStore {
  constructor() {
    this.reset();
  }

  // Nothing is known (no connection, or before a hello reply).
  reset() {
    this.devices = [];
    this.states = {}; // id -> { rev, state }
  }

  // The hello reply rebuilds everything (4.1 "Reconnect").
  fromHello(ok) {
    this.devices = Array.from(ok.devices ?? []);
    this.states = {};
    for (const d of this.devices) {
      const s = ok.states?.[d.id];
      if (s) this.states[d.id] = { rev: s.rev, state: s.state };
    }
  }

  // A devices event: the full list. Returns the ids no longer listed. The rev and state of
  // such an id are forgotten: an entry added later under the same port: id starts at rev 1.
  onDevices(list) {
    const ids = new Set(list.map((d) => d.id));
    const removed = this.devices.map((d) => d.id).filter((id) => !ids.has(id));
    for (const id of removed) delete this.states[id];
    this.devices = Array.from(list);
    return removed;
  }

  // A state event { device, rev, state }. Applied when its device is listed and its rev is
  // newer than the one known; returns whether it was applied.
  onState(body) {
    if (!this.device(body.device)) return false;
    const known = this.states[body.device];
    if (known && !(body.rev > known.rev)) return false;
    this.states[body.device] = { rev: body.rev, state: body.state };
    return true;
  }

  device(id) {
    return this.devices.find((d) => d.id === id) ?? null;
  }

  state(id) {
    return this.states[id]?.state ?? null;
  }

  rev(id) {
    return this.states[id]?.rev ?? 0;
  }
}

// The device to show after a change of the list: the current one while it is listed (its new
// id after a hand-over, 5.5); else the only usable device, or none (8.4 step 7). An unplugged
// sn: device stays listed and stays selected.
export function selection(devices, current) {
  const now = current ? devices.find((d) => d.id === current) : null;
  if (now) {
    const next = now.replaced_by
      ? devices.find((d) => d.id === now.replaced_by)
      : null;
    return next ? next.id : now.id;
  }
  const candidates = devices.filter(usable);
  return candidates.length === 1 ? candidates[0].id : null;
}
```

Create `lib/values.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The value each control shows (protocol 2, 4.1 rule 4): the client's own value from sending a
// request until its reply arrives, then the state, its pending values included. No timers.

import { at, sameValue, setting, writable } from "./describe.mjs";

// The own values of the set requests in flight: request id -> { path: value }.
export class OwnValues {
  constructor() {
    this.byRequest = new Map();
  }

  // A set request went out with these leaves (a flat patch: path -> value).
  add(id, patch) {
    this.byRequest.set(id, Object.assign({}, patch));
  }

  // Its reply arrived (ok or error): its own values go.
  settle(id) {
    return this.byRequest.delete(id);
  }

  // The connection was lost: every request died with it (4.1 "Reconnect").
  clear() {
    this.byRequest.clear();
  }

  // The newest own value of a path, or undefined.
  value(path) {
    let found;
    let newest = -Infinity;
    for (const [id, patch] of this.byRequest) {
      if (path in patch && id > newest) {
        newest = id;
        found = patch[path];
      }
    }
    return found;
  }

  get size() {
    return this.byRequest.size;
  }
}

// The value a control shows for a leaf.
export function shown(state, own, path) {
  const mine = own?.value(path);
  if (mine !== undefined) return mine;
  if (state?.pending && path in state.pending) return state.pending[path];
  const v = at(state, path);
  return v === undefined ? null : v;
}

// What the value of a leaf stands for: "own" (sent, no reply yet), "pending" (the camera took
// it, its readback does not show it yet), "unread" (never read, or the last read failed), or
// "camera" (as the camera reports it).
export function status(state, own, path) {
  if (own?.value(path) !== undefined) return "own";
  if (state?.pending && path in state.pending) return "pending";
  if (Array.isArray(state?.unread) && state.unread.includes(path))
    return "unread";
  return "camera";
}

// The patch of one control that edits several leaves (an area with two ranges): only the leaves
// whose edited value differs from the value shown. null when nothing changed.
export function changed(state, own, edits) {
  const patch = {};
  for (const [path, value] of Object.entries(edits)) {
    if (!sameValue(shown(state, own, path), value)) patch[path] = value;
  }
  return Object.keys(patch).length > 0 ? patch : null;
}

// The defaults reset (10.3 D): one set patch of the `default` values of the given leaves.
// A leaf is left out when the device lacks it, it is not writable, it has no default, it has a
// `requires` (the daemon refuses a whole patch when one leaf fails, 4.1 rule 5), or it shows
// its default already. null when nothing is left.
export function defaults(device, state, own, paths) {
  const patch = {};
  for (const path of paths) {
    const s = setting(device, path);
    if (
      !s ||
      !writable(device, path) ||
      s.default === undefined ||
      (s.requires ?? []).length > 0
    )
      continue;
    if (!sameValue(shown(state, own, path), s.default)) patch[path] = s.default;
  }
  return Object.keys(patch).length > 0 ? patch : null;
}
```

Create `lib/format.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Values as text, with the units of the description (protocol 2, 4.2 "Units and frames").

import { decimals, humanize } from "./describe.mjs";

const UNITS = {
  deg: "°",
  "deg/s": "°/s",
  factor: "×",
  us: " µs",
  K: " K",
  percent: " %",
  s: " s",
  fps: " fps",
  fraction: "",
};

// A unit the widget does not know is shown by its name (units are additive, 4.1 rule 7).
export function unitText(unit) {
  if (unit === null || unit === undefined) return "";
  return unit in UNITS ? UNITS[unit] : ` ${unit}`;
}

// 12.5 with precision 0.1 and unit deg -> "12.5°"; negative numbers with a real minus sign.
export function number(value, precision, unit) {
  const text = Number(value).toFixed(decimals(precision));
  return text.replace("-", "−") + unitText(unit);
}

function part(s, key) {
  const pick = (b) => (b !== null && typeof b === "object" ? b[key] : b);
  return { precision: pick(s.precision), unit: pick(s.unit) };
}

// One value as the description's type shows it. labels: the widget's own names of enum values.
export function value(s, v, labels) {
  if (v === null || v === undefined) return "–";
  if (typeof v === "string" && /^raw:-?\d+$/.test(v)) return v;
  switch (s?.type) {
    case "bool":
      return v ? "On" : "Off";
    case "enum":
      return labels?.[v] ?? humanize(v);
    case "number":
      return number(v, s.precision, s.unit);
    case "range": {
      const p = part(s, "min");
      return `${number(v.min, p.precision, null)}…${number(v.max, p.precision, p.unit)}`;
    }
    case "pose":
      return ["pan", "tilt", "zoom"]
        .map(
          (k) => `${k} ${number(v[k], part(s, k).precision, part(s, k).unit)}`,
        )
        .join(" · ");
    case "point":
    case "box":
      return Object.keys(v)
        .map(
          (k) => `${k} ${number(v[k], part(s, k).precision, part(s, k).unit)}`,
        )
        .join(" · ");
    case "string":
      return String(v);
    case "list":
      return Array.isArray(v) ? `${v.length}` : "–";
    default:
      // a setting of a type this widget does not know: read-only text (rule 7)
      return typeof v === "object" ? JSON.stringify(v) : String(v);
  }
}
```

Create `lib/gimbal.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The gimbal arrows: the camera has no relative move, so the widget computes the target of a
// step and sends one gimbal.move with it (spec 15 row 14), and nothing else (section 2: no
// tracking switch before it).

import { action, numberSpec, roundTo } from "./describe.mjs";

// How long the target of the last own move stays the base of the next step: the live pose reads
// a point in between while the gimbal is still on its way, and quick presses would fall short.
export const STEP_BASE_MS = 2000;

// The bounds of gimbal.move's pan and tilt, or null when the device has no gimbal.move.
export function moveSpec(device) {
  const a = action(device, "gimbal.move");
  if (!a?.params?.pan || !a?.params?.tilt) return null;
  return { pan: numberSpec(a.params.pan), tilt: numberSpec(a.params.tilt) };
}

function clamp(v, spec) {
  let x = v;
  if (spec.min !== null) x = Math.max(spec.min, x);
  if (spec.max !== null) x = Math.min(spec.max, x);
  return roundTo(x, spec.precision);
}

// The target of one arrow step. pose: { pan, tilt } of the state (the live pose); last: the
// target of the previous own move and when it was sent ({ pan, tilt, at }) or null; now: ms.
// null while the base has no numeric pan and tilt (the pose is not read yet): a step from a
// made-up pose would send the gimbal to a target the user never asked for.
export function stepTarget(spec, pose, last, now, dPan, dTilt) {
  const base = last && now - last.at < STEP_BASE_MS ? last : pose;
  if (typeof base?.pan !== "number" || typeof base?.tilt !== "number")
    return null;
  return {
    pan: clamp(base.pan + dPan, spec.pan),
    tilt: clamp(base.tilt + dTilt, spec.tilt),
  };
}

// The one request of a gimbal control: { action, params }.
export function moveRequest(target) {
  return {
    action: "gimbal.move",
    params: { pan: target.pan, tilt: target.tilt },
  };
}

// The arrows follow the picture: with the image mirrored, the picture's left is the camera's
// right (pan positive = the camera turns to its left, 4.2).
export function panLeftSign(state) {
  return state?.image?.mirror === true ? -1 : 1;
}
```

Create `lib/lists.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Presets and views (protocol 2, 4.2 "Lists", 4.4): rows for the cards, and the parameters of
// the actions on them. A preset saved without a name gets none (spec 12.3): the placeholder
// below is display only and never sent.

import { action, at, limit, numberSpec } from "./describe.mjs";

export const UNNAMED = "(unnamed)";

// The preset slots of a device: from its preset count (limits.presets) and the first slot the
// description allows (the slot parameter of preset.save); [] without presets.
export function presetSlots(device) {
  const save = action(device, "preset.save") ?? action(device, "preset.recall");
  const count = limit(device, "presets");
  const slot = save?.params?.slot ? numberSpec(save.params.slot) : null;
  if (count === null && (slot === null || slot.max === null)) return [];
  const first = slot?.min ?? 0;
  const n = count ?? slot.max - first + 1;
  return Array.from({ length: n }, (_, i) => first + i);
}

// One row per slot: { slot, used, name, label, pose }. null when the list is not read yet
// (it is in `unread`, or the state has no list).
export function presetRows(device, state) {
  const list = at(state, "presets");
  if (
    !Array.isArray(list) ||
    (Array.isArray(state?.unread) && state.unread.includes("presets"))
  )
    return null;
  return presetSlots(device).map((slot) => {
    const p = list.find((item) => item.slot === slot);
    if (!p) return { slot, used: false, name: "", label: "Empty", pose: null };
    const name = typeof p.name === "string" ? p.name : "";
    return {
      slot,
      used: true,
      name,
      label: name !== "" ? name : UNNAMED,
      pose: p.pose ?? null,
    };
  });
}

// The views of the state, or null while not read.
export function viewRows(state) {
  const list = at(state, "views");
  if (
    !Array.isArray(list) ||
    (Array.isArray(state?.unread) && state.unread.includes("views"))
  )
    return null;
  return list.map((v) => ({
    id: v.id,
    name: String(v.name ?? ""),
    label: v.name ? String(v.name) : UNNAMED,
    pose: v.pose ?? null,
  }));
}

// preset.save's parameters: the name only when the user typed one (spec 12.3).
export function saveParams(slot, typed) {
  return typed !== "" ? { slot, name: typed } : { slot };
}

// Why a typed name cannot be sent as parameter `param` of `actionName`, or null: the bounds of
// the description's string (max_len in characters, ascii = printable ASCII only).
export function nameProblem(device, actionName, param, text) {
  const p = action(device, actionName)?.params?.[param];
  if (!p) return "not offered by this camera";
  if (typeof p.max_len === "number" && Array.from(text).length > p.max_len)
    return `at most ${p.max_len} characters`;
  if (p.ascii === true && !/^[\x20-\x7e]*$/.test(text))
    return "letters, digits and punctuation without accents only";
  if (!p.optional && text === "") return "a name is needed";
  return null;
}
```

Create `lib/labels.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The widget's own words: names for enum values it knows (a value it does not know shows as
// text made from the value, describe.humanize) and the texts for a daemon it cannot use.
// Which values a control offers always comes from the device's description, never from here.

const TRACKING = {
  static: "Static",
  normal: "Normal Tracking",
  upper_body: "Upper Body",
  close_up: "Close-up",
  headless: "Headless",
  lower_body: "Lower Body",
  hand: "Hand",
  group: "Group",
};

const LABELS = {
  "tracking.mode": TRACKING,
  "power_on.mode": TRACKING,
  "power_on.start.mode": Object.assign({}, TRACKING, { unset: "Not set" }),
  "tracking.profile": {
    standard: "Standard",
    headroom: "Headroom",
    sport: "Sport",
  },
  "hand.side": { left: "Left", right: "Right" },
  "zoom.field_of_view": {
    wide: "Wide",
    medium: "Medium",
    narrow: "Narrow",
    custom: "Custom",
  },
  "exposure.mode": { auto: "Auto", manual: "Manual" },
  "exposure.region": { face: "Face", global: "Global" },
  "sleep.background": {
    black: "Black",
    image: "Picture",
    own_image: "Own picture",
  },
  "audio.noise_reduction": {
    off: "Off",
    weak: "Weak",
    medium: "Medium",
    strong: "Strong",
  },
};

export function labelsFor(path) {
  return LABELS[path] ?? {};
}

// Every path this table names, for its test against the descriptions.
export function labelledPaths() {
  return Object.keys(LABELS);
}

// What the widget shows while it cannot use the daemon (lib/session.mjs problems; spec 5.10).
export function problemText(problem) {
  switch (problem) {
    case "unreachable":
      return {
        title: "obsbot4linux is not running",
        body: "Start its socket once: systemctl --user enable --now obsbot4linux.socket",
      };
    case "old":
      return {
        title: "obsbot4linux is older than 1.0",
        body: "an obsbot4linux daemon older than 1.0 is running: restart it or log out",
      };
    case "protocol":
      return {
        title: "obsbot4linux speaks another protocol",
        body: "Install the same version of obsbot4linux and this widget, then restart the daemon.",
      };
    case "lost":
      return {
        title: "Connection to obsbot4linux lost",
        body: "Reconnecting…",
      };
    default:
      return null;
  }
}
```

- [ ] **Step 4: Run them and see them pass**

Run: `node tests/run.mjs tests/devices.test.mjs tests/describe.test.mjs tests/values.test.mjs tests/lists.test.mjs tests/labels.test.mjs && scripts/check-js.sh && bash scripts/check-qml.sh`
Expected: `46 passed, 0 failed`; `check-js: ok (53 passed, …)` (76 once Task 3 landed); `check-qml: ok`.

- [ ] **Step 5: CHANGELOG and commit**

`CHANGELOG.md`, `## [Unreleased]`, `### Added`: `- The rules the 1.0 widget follows, as tested modules: which controls a device's description offers and with which values and bounds, what a control shows (its own value until the reply, then the camera's value or its pending one), the device list and the device shown, presets and views, and the gimbal arrows (#N).`

```bash
git add lib tests/devices.test.mjs tests/describe.test.mjs tests/values.test.mjs tests/lists.test.mjs tests/labels.test.mjs CHANGELOG.md
git commit -S -m "Modules: devices, descriptions, shown values, text, arrows, presets and views"
```

---

### Task 5: The link without I/O

Branches from `v1` after Tasks 3 and 4 have both landed (it imports `lib/protocol.mjs` and `lib/session.mjs` of Task 3 and the modules of Task 4; it is not stacked).

**Files:**

- Create: `lib/link.mjs`
- Test: `tests/link.test.mjs`
- Modify: `CHANGELOG.md`

**Interfaces:**

- Consumes: Task 3 (`Session`, `setLeaves`, `act`), Task 4 (`DeviceStore`, `selection`, `OwnValues`, `shown`, `status`; `moveRequest`, `moveSpec`, `stepTarget` in the test), fixtures `fakes.mjs`.
- Produces: `class Link(client)` with `session`, `store`, `own`, `selected`; getters `ready`, `problem`; input methods returning effects (`start()`, `connected()`, `line(text)`, `closed(error?)`, `helloTimeout()`, `retry()`, `stop()`, `setLive(topics)`, `pick(id)`); readers `device()`, `state()`, `shown(path)`, `status(path)`; requests `set(path, value)`, `setPatch(patch)`, `act(name, params?)`, each returning the effects of exactly one written line, or `[]` when nothing can be sent. Effects are Task 3's I/O effects plus `{changed: true}` and `{failed: {id, code, message}}`; `{ready}`, `{reply}` and `{event}` are consumed inside.

- [ ] **Step 1: Write the failing test**

Create `tests/link.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
import { eq, ok } from "./assert.mjs";
import { Link } from "../lib/link.mjs";
import { moveRequest, moveSpec, stepTarget } from "../lib/gimbal.mjs";
import { A, B, HELLO, STATES, UNKNOWN } from "./fixtures/fakes.mjs";

function linked(devices) {
  const link = new Link("obsbot4linux-dux 1.0.0");
  link.start();
  link.connected();
  const hello = Object.assign({}, HELLO, { devices: devices ?? HELLO.devices });
  link.line(JSON.stringify({ id: 1, ok: hello }));
  return link;
}

const writes = (effects) =>
  effects.filter((e) => "write" in e).map((e) => JSON.parse(e.write));

export const tests = {
  "after hello: two usable devices, none chosen; the user picks one": () => {
    const link = linked();
    ok(link.ready, "ready");
    eq(link.device(), null);
    link.pick(B.id);
    eq(link.device().model, "fake-b");
    eq(link.shown("tracking.mode"), "off");
  },
  "one usable device is chosen by itself": () => {
    eq(linked([A, UNKNOWN]).device().id, A.id);
  },
  // spec 2, 12.3: one control, one request
  "a setting control writes exactly one set request with its one leaf": () => {
    const link = linked();
    link.pick(B.id);
    const out = writes(link.set("led.brightness", 3));
    eq(out, [{ id: 2, device: B.id, set: { "led.brightness": 3 } }]);
    eq(
      [link.shown("led.brightness"), link.status("led.brightness")],
      [3, "own"],
    );
  },
  "a gimbal arrow writes exactly one gimbal.move and nothing else": () => {
    const link = linked();
    link.pick(A.id);
    const target = stepTarget(moveSpec(A), link.state().gimbal, null, 0, 5, 0);
    const req = moveRequest(target);
    const out = writes(link.act(req.action, req.params));
    eq(out, [
      { id: 2, device: A.id, do: "gimbal.move", with: { pan: 5, tilt: 0 } },
    ]);
  },
  // 4.1 rules 3 and 4: the state event comes before the reply; pending shows after the reply
  "own value until the reply, then the state's pending value": () => {
    const link = linked();
    link.pick(B.id);
    link.set("led.brightness", 3);
    const pendingState = Object.assign({}, STATES[B.id].state, {
      pending: { "led.brightness": 3 },
    });
    link.line(
      JSON.stringify({
        event: "state",
        device: B.id,
        rev: 2,
        state: pendingState,
      }),
    );
    eq(link.status("led.brightness"), "own");
    link.line('{"id":2,"ok":{"rev":2}}');
    eq(
      [link.shown("led.brightness"), link.status("led.brightness")],
      [3, "pending"],
    );
  },
  "a refused request takes back its own value and reports the error": () => {
    const link = linked();
    link.pick(B.id);
    link.set("exposure.time", 500);
    const effects = link.line(
      '{"id":2,"error":{"code":"conflict","message":"exposure.time requires exposure.mode = manual"}}',
    );
    ok(
      effects.some((e) => e.failed && e.failed.code === "conflict"),
      "failed effect",
    );
    eq(link.shown("exposure.time"), 100);
  },
  // 8.4 step 7: the selected unsupported device goes on unplug; with two usable devices left
  // nothing is chosen for the user
  "unplugging the selected unsupported device": () => {
    const link = linked();
    link.pick(UNKNOWN.id);
    link.line(JSON.stringify({ event: "devices", devices: [A, B] }));
    eq(link.device(), null);
    const one = linked([A, UNKNOWN]);
    one.pick(UNKNOWN.id);
    one.line(JSON.stringify({ event: "devices", devices: [A] }));
    eq(one.device().id, A.id);
  },
  // 4.1 Reconnect: requests and own values die with the connection
  "a lost connection forgets everything and sends nothing": () => {
    const link = linked();
    link.pick(B.id);
    link.set("led.brightness", 3);
    link.closed(1);
    eq([link.device(), link.own.size, link.problem], [null, 0, "lost"]);
    eq(link.set("led.brightness", 2), []);
  },
  // 4.1 Reconnect drops requests and own values, not the user's pick: with two usable
  // cameras the pick comes back after the next hello while its id is listed
  "the pick survives a reconnect while the device is listed": () => {
    const link = linked();
    link.pick(B.id);
    link.closed(1);
    link.retry();
    const hello = writes(link.connected());
    eq(hello.length, 1);
    link.line(JSON.stringify({ id: hello[0].id, ok: HELLO }));
    ok(link.ready, "ready again");
    eq(link.device().id, B.id);
  },
  "an empty patch and no device send nothing": () => {
    const link = linked();
    eq(link.set("led.brightness", 2), []);
    link.pick(B.id);
    eq(link.setPatch({}), []);
  },
  // 4.1 rule 7: an unknown setting of fake-b (beacon.*) does not break anything
  "fake-b's unknown group is carried and ignored": () => {
    const link = linked();
    link.pick(B.id);
    eq(link.shown("beacon.color"), "red");
    const effects = link.line(
      JSON.stringify({ event: "thermal", device: B.id }),
    );
    eq(effects, []);
  },
};
```

- [ ] **Step 2: Run it and see it fail**

Run: `node tests/run.mjs tests/link.test.mjs`
Expected: exit 1, `ERR_MODULE_NOT_FOUND` for `lib/link.mjs`.

- [ ] **Step 3: Implement**

Create `lib/link.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The widget's whole link to the daemon, without I/O: the connection (Session), the devices and
// their states (DeviceStore), the own values in flight (OwnValues) and the device shown. One
// QML component (CameraLink.qml) and the e2e test drive it the same way: they hand it what the
// socket did and carry out the effects it returns (session.mjs lists the I/O effects). Two more
// effects come from here:
//   { changed: true }               devices, states, own values or the selection changed
//   { failed: { id, code, message } }  a request of ours was refused (the UI shows a toast)
// Each control sends exactly one request (spec 2, 12.3): set(), setPatch() and act() each write
// one line and nothing else.

import { act as actRequest, setLeaves } from "./protocol.mjs";
import { Session } from "./session.mjs";
import { DeviceStore, selection } from "./devices.mjs";
import { OwnValues, shown, status } from "./values.mjs";

export class Link {
  constructor(client) {
    this.session = new Session(client);
    this.store = new DeviceStore();
    this.own = new OwnValues();
    this.selected = null;
  }

  get ready() {
    return this.session.phase === "ready";
  }

  get problem() {
    return this.session.problem;
  }

  start() {
    return this.handle(this.session.start());
  }

  connected() {
    return this.handle(this.session.connected());
  }

  line(text) {
    return this.handle(this.session.line(text));
  }

  closed(error) {
    return this.handle(this.session.closed(error));
  }

  helloTimeout() {
    return this.handle(this.session.helloTimeout());
  }

  retry() {
    return this.handle(this.session.retry());
  }

  stop() {
    return this.handle(this.session.stop());
  }

  setLive(topics) {
    return this.session.setLive(topics);
  }

  // The user picks a device in the device picker.
  pick(id) {
    if (!this.store.device(id)) return [];
    this.selected = id;
    return [{ changed: true }];
  }

  device() {
    return this.selected ? this.store.device(this.selected) : null;
  }

  state() {
    return this.selected ? this.store.state(this.selected) : null;
  }

  shown(path) {
    return shown(this.state(), this.own, path);
  }

  status(path) {
    return status(this.state(), this.own, path);
  }

  // One set request with one leaf.
  set(path, value) {
    return this.setPatch({ [path]: value });
  }

  // One set request with the leaves of one control; the own values show until its reply.
  setPatch(patch) {
    const device = this.selected;
    if (!device || !patch || Object.keys(patch).length === 0) return [];
    const sent = this.session.request((id) => setLeaves(id, device, patch));
    if (sent.id === null) return [];
    this.own.add(sent.id, patch);
    return sent.effects.concat([{ changed: true }]);
  }

  // One do request.
  act(name, params) {
    const device = this.selected;
    if (!device) return [];
    return this.session.request((id) =>
      actRequest(id, device, name, params ?? {}),
    ).effects;
  }

  handle(effects) {
    const out = [];
    for (const e of effects) {
      if (e.ready) {
        this.store.fromHello(e.ready);
        this.own.clear();
        this.reselect();
        out.push({ changed: true });
      } else if (e.lost) {
        // the pick stays: device() is null while the store is empty, and the next hello
        // shows the same id again while it is listed (reselect)
        this.store.reset();
        this.own.clear();
        out.push(e, { changed: true });
      } else if (e.event) {
        if (this.onEvent(e.event)) out.push({ changed: true });
      } else if (e.reply) {
        const r = e.reply;
        const had = this.own.settle(r.id);
        if (r.kind === "error")
          out.push({ failed: { id: r.id, code: r.code, message: r.message } });
        if (had || r.kind === "error") out.push({ changed: true });
      } else {
        out.push(e);
      }
    }
    return out;
  }

  onEvent(parsed) {
    const body = parsed.body;
    if (parsed.event === "devices" && Array.isArray(body.devices)) {
      this.store.onDevices(body.devices);
      this.reselect();
      return true;
    }
    if (parsed.event === "state") return this.store.onState(body);
    // an event type this widget does not know (4.1 rule 7)
    return false;
  }

  // After the list changed: the device shown while it is listed (its new id after a
  // hand-over), else the only usable device, or none (devices.selection).
  reselect() {
    this.selected = selection(this.store.devices, this.selected);
  }
}
```

- [ ] **Step 4: Run it and see it pass**

Run: `node tests/run.mjs tests/link.test.mjs && scripts/check-js.sh && bash scripts/check-qml.sh`
Expected: `11 passed, 0 failed`; `check-js: ok (87 passed, 0 failed in node and in qml)`; `check-qml: ok (43 files)`.

- [ ] **Step 5: Prove the one-request rule can fail**

Temporarily make `setPatch` write twice (duplicate `sent.effects` in its return); `node tests/run.mjs tests/link.test.mjs` must report `not ok - link.test.mjs: a setting control writes exactly one set request …`. Undo.

- [ ] **Step 6: CHANGELOG and commit**

`CHANGELOG.md`, `## [Unreleased]`, `### Added`: `- One link object ties the widget's modules together: every control sends exactly one request and nothing before or after it (#N).`

```bash
git add lib/link.mjs tests/link.test.mjs CHANGELOG.md
git commit -S -m "Link: the widget's link to the daemon without I/O, one request per control"
```

---

### Task 6: `CameraLink.qml` and the bound controls

Needs P1 passed (or its fallback chosen). Stacked on Task 5.

**Files:**

- Create: `CameraLink.qml`, `SettingToggle.qml`, `SettingChips.qml`, `SettingSlider.qml`
- Modify: `CHANGELOG.md`

**Interfaces:**

- Consumes: `lib/link.mjs`, `lib/describe.mjs`, `lib/devices.mjs`, `lib/protocol.mjs`, `lib/values.mjs`, `lib/format.mjs`; existing `ToggleRow.qml` (`label`, `description`, `checked`, `signal toggled(bool)`), `ChipRow.qml` (`label`, `options`, `currentIndex`, `stacked`, `signal selected(int)`, `sync()`), `LabeledSlider.qml` (`label`, `minimum`, `maximum`, `value` (int), `formatValue`, `dragging`, `signal released(int)`, `sync()`); `Quickshell.Io` `Socket` (`path`, `connected` (write true to connect), `write(string)`, `flush()`, `parser`, `signal error(int)`, `connectionStateChanged`) and `SplitParser` (`onRead(line)`), read in the shell's runtime source (`src/io/socket.hpp` of `duxlabs-dev/dux-qt-base`): after a connection was up, a close by the daemon does not reconnect by itself; a failed connect emits `error` and leaves the socket closed.
- Produces (Tasks 8–15 use exactly these):
  - `CameraLink` (an `Item`): inputs `active: bool`, `instanceName: string`, `client: string`, `liveView: {open, pose, target, auto}`; read-only `revision: int`, `link` (the `Link`), `socketPath: string`, `ready: bool`, `problem: string`, `devices: Device[]`, `device: Device | null`, `camState: object | null`, `deviceId: string`, `available: bool`, `usable: bool`, `anyUsable: bool` (some listed device is usable, selected or not), `awake: bool`, `marker: string`, `unsupportedHint: string`, `wantedTopics: string[]`; functions `has(path)`, `setting(path)`, `value(path)`, `status(path)`, `choices(path, labels)`, `spec(path, component?)`, `paramSpec(action, param, component?)`, `blocked(path)`, `warnings(path)`, `defaultsPatch(paths)`, `limit(path)`, `set(path, value)`, `setPatch(patch)`, `act(name, params?)`, `pick(id)`, `retry()`; `signal failed(var id, string code, string message)` (it also shows a toast).
  - `SettingToggle` (a `ToggleRow`): `link`, `path`, `text`, `hint`.
  - `SettingChips` (a `ChipRow`): `link`, `path`, `text`, `labels`.
  - `SettingSlider` (a `LabeledSlider`): `link`, `path`, `text`, `component` (`""`, `"min"` or `"max"`).
  - Each bound control: hidden unless `link.has(path)` (the slider also needs bounds and a step); disabled with `opacity 0.38` while `link.blocked(path)` is not null; marks `· pending` while `link.status(path) === "pending"`, `· no visible effect` for a `no_effect` leaf, `· needs <path> = <value>` (the toggle: as its description) while a `requires` does not hold; sends one `link.set` (the toggle and chips on a change, the slider on release).

- [ ] **Step 1: The check that fails first**

Run: `bash scripts/check-qml.sh && ls CameraLink.qml`
Expected: `check-qml: ok (43 files)`, then `ls` fails: the files do not exist.

- [ ] **Step 2: Write the components**

Create `CameraLink.qml`:

```qml
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Services
import "lib/link.mjs" as LinkModule
import "lib/describe.mjs" as Describe
import "lib/devices.mjs" as Devices
import "lib/protocol.mjs" as Protocol
import "lib/values.mjs" as Values

// The link of one widget instance (bar widget, Control Center tile, desktop widget) to the
// obsbot4linux daemon: protocol 2 on its Unix socket. All logic is in lib/link.mjs and the
// modules it uses; this component only carries out the I/O they ask for (the socket, two
// timers) and tells the bindings when something changed (`revision`).
//
// Controls read through the functions below and send through set(), setPatch() and act(): each
// of these sends exactly one request (spec 2, 12.3).
Item {
    id: root

    // false: no connection (the throw-away instance DMS creates to read ccWidgetIcon; the lock
    // screen; the desktop editor)
    property bool active: false
    // For the log, e.g. "bar widget on screen DP-1"
    property string instanceName: ""
    // The client name in hello (protocol 2, 4.1)
    property string client: "obsbot4linux-dux"
    // What the owner shows now; decides the live topics (only while the popout is open, 10.3 D)
    property var liveView: ({
            open: false,
            pose: false,
            target: false,
            auto: false
        })

    // Goes up on every change of devices, states, own values or the selection
    property int revision: 0
    property var link: null

    readonly property string socketPath: Protocol.socketPath(name => Quickshell.env(name) ?? "")
    readonly property bool ready: revision >= 0 && link !== null && link.ready
    // "" while ready or idle, else "unreachable", "old", "protocol" or "lost" (lib/session.mjs)
    readonly property string problem: revision >= 0 && link !== null ? link.problem : ""
    readonly property var devices: revision >= 0 && link !== null ? link.store.devices : []
    readonly property var device: revision >= 0 && link !== null ? link.device() : null
    readonly property var camState: revision >= 0 && link !== null ? link.state() : null
    readonly property string deviceId: device !== null ? device.id : ""
    readonly property bool available: device !== null && device.available === true
    readonly property bool usable: device !== null && Devices.usable(device)
    // Some listed device is usable, selected or not ("Hide without camera")
    readonly property bool anyUsable: Devices.anyUsable(devices)
    readonly property bool awake: usable && camState !== null && camState.asleep !== true
    readonly property string marker: device !== null ? Devices.marker(device) : ""
    readonly property string unsupportedHint: device !== null && device.support === "unsupported" ? Devices.unsupportedHint(device) : ""
    readonly property var wantedTopics: Describe.wantedTopics(device, liveView)

    // A request of ours was refused; a toast shows it too
    signal failed(var id, string code, string message)

    function has(path) {
        return revision >= 0 && Describe.has(device, path);
    }
    function setting(path) {
        return revision >= 0 ? Describe.setting(device, path) : null;
    }
    function value(path) {
        return revision >= 0 && link !== null ? link.shown(path) : null;
    }
    // "own", "pending", "unread" or "camera" (lib/values.mjs)
    function status(path) {
        return revision >= 0 && link !== null ? link.status(path) : "unread";
    }
    function choices(path, labels) {
        return revision >= 0 ? Describe.choices(device, path, labels) : [];
    }
    // Bounds of a leaf, or of one component of it ("min", "pan", …)
    function spec(path, component) {
        return revision >= 0 ? Describe.numberSpec(Describe.setting(device, path), component) : null;
    }
    // Bounds of a parameter of an action
    function paramSpec(action, param, component) {
        const a = revision >= 0 ? Describe.action(device, action) : null;
        return a && a.params && a.params[param] ? Describe.numberSpec(a.params[param], component) : null;
    }
    // null when the control can be used now, else { reason, needs? } (lib/describe.mjs)
    function blocked(path) {
        return revision >= 0 ? Describe.blocked(device, camState, path) : ({
                reason: "unavailable"
            });
    }
    function warnings(path) {
        return revision >= 0 ? Describe.warnings(device, path) : [];
    }
    // The defaults reset of these leaves as one patch, or null (lib/values.mjs defaults)
    function defaultsPatch(paths) {
        return revision >= 0 && link !== null ? Values.defaults(device, camState, link.own, paths) : null;
    }
    function limit(path) {
        return revision >= 0 ? Describe.limit(device, path) : null;
    }

    function set(path, value) {
        run(link.set(path, value));
    }
    function setPatch(patch) {
        run(link.setPatch(patch));
    }
    function act(name, params) {
        run(link.act(name, params));
    }
    function pick(id) {
        run(link.pick(id));
    }
    // Retry button: connect again now
    function retry() {
        if (link !== null && active) {
            retryTimer.stop();
            run(link.retry());
        }
    }

    function log(text) {
        console.info("obsbot4linuxDux (" + instanceName + "): " + text);
    }

    // Carries out the effects of lib/link.mjs, in order
    function run(effects) {
        for (const e of effects) {
            if (e.connect) {
                socket.path = root.socketPath;
                socket.connected = true;
            } else if (e.close) {
                socket.connected = false;
            } else if (e.write !== undefined) {
                socket.write(e.write);
                socket.flush();
            } else if (e.retryIn !== undefined) {
                if (e.retryIn < 0) {
                    retryTimer.stop();
                } else {
                    retryTimer.interval = e.retryIn;
                    retryTimer.restart();
                }
            } else if (e.helloTimeoutIn !== undefined) {
                helloTimer.interval = e.helloTimeoutIn;
                helloTimer.restart();
            } else if (e.changed) {
                root.revision += 1;
            } else if (e.failed) {
                root.log("refused: " + e.failed.code + ": " + e.failed.message);
                ToastService.showError("OBSBOT Control: " + e.failed.message, "");
                root.failed(e.failed.id, e.failed.code, e.failed.message);
            } else if (e.problem !== undefined) {
                root.log("daemon " + e.problem + (root.socketPath !== "" ? " at " + root.socketPath : ""));
            }
        }
    }

    onWantedTopicsChanged: {
        if (link !== null)
            run(link.setLive(wantedTopics));
    }
    onActiveChanged: {
        if (link !== null)
            run(active ? link.start() : link.stop());
    }
    Component.onCompleted: {
        link = new LinkModule.Link(client);
        if (active)
            run(link.start());
    }
    Component.onDestruction: {
        if (link !== null)
            run(link.stop());
    }

    Timer {
        id: retryTimer
        onTriggered: root.run(root.link.retry())
    }

    Timer {
        id: helloTimer
        onTriggered: root.run(root.link.helloTimeout())
    }

    Socket {
        id: socket
        parser: SplitParser {
            onRead: line => root.run(root.link.line(line))
        }
        onConnectionStateChanged: root.run(connected ? root.link.connected() : root.link.closed(undefined))
        // Before a connection exists: no socket file or refused (the systemctl hint). While
        // connected, the close that follows reports it.
        onError: error => {
            if (!connected)
                root.run(root.link.closed(error));
        }
    }
}
```

Create `SettingToggle.qml`:

```qml
import QtQuick

// A bool leaf of the device's description as a switch (10.3 D: every control gated on its
// path). Hidden when the device lacks the leaf; off while blocked (lib/describe.mjs blocked:
// unavailable, read-only, asleep and ignored, a requires not met), with the reason as its
// description; one set request with the one leaf when toggled (spec 2).
ToggleRow {
    id: row

    property var link
    property string path: ""
    property string text: ""
    // Shown under the label while the control can be used
    property string hint: ""

    readonly property var block: link.blocked(path)
    readonly property string shownStatus: link.status(path)

    visible: link.has(path)
    enabled: block === null
    opacity: enabled ? 1 : 0.38
    readonly property var setting: link.setting(path)
    readonly property var current: link.value(path)

    label: text + (shownStatus === "pending" ? " · pending" : "")
    // The reason it is blocked; else whether the value is known and has an effect (4.2: a
    // readback:false leaf is null until it is written; no_effect)
    description: block !== null && block.reason === "requires" ? "Needs " + block.needs.map(n => n.path + " = " + n.equals).join(", ") : current === null ? "Not known until it is set here" : setting !== null && setting.no_effect ? "No visible effect on this camera" : hint
    checked: current === true
    onToggled: on => row.link.set(row.path, on)
}
```

Create `SettingChips.qml`:

```qml
import QtQuick

// An enum leaf of the device's description as single-select chips: the chips are the values
// the description lists (`values`), with the widget's own names where it has them; values that
// are not in `writable` are shown but cannot be picked. A value the description does not list
// (raw:<n>, a newer value) selects no chip and shows as text after the label (4.1 rule 7). One
// set request with the one leaf when a chip is picked (spec 2).
ChipRow {
    id: row

    property var link
    property string path: ""
    property string text: ""
    // value -> the widget's own name; other values show as text made from the value
    property var labels: ({})

    readonly property var block: link.blocked(path)
    readonly property var all: link.choices(path, labels)
    readonly property var current: link.value(path)
    readonly property int currentChoice: all.findIndex(c => c.value === current)

    visible: link.has(path)
    enabled: block === null
    opacity: enabled ? 1 : 0.38
    label: text + (currentChoice < 0 && current !== null ? " (" + String(current) + ")" : "") + (link.status(path) === "pending" ? " · pending" : "") + (link.setting(path)?.no_effect ? " · no visible effect" : "") + (block !== null && block.reason === "requires" ? " · needs " + block.needs.map(n => n.path + " = " + n.equals).join(", ") : "")
    options: all.map(c => c.label)
    currentIndex: currentChoice
    onSelected: index => {
        const c = row.all[index];
        if (c && c.selectable && c.value !== row.current)
            row.link.set(row.path, c.value);
        else
            row.sync();
    }
    onCurrentChoiceChanged: sync()
}
```

Create `SettingSlider.qml`:

```qml
import QtQuick
import "lib/describe.mjs" as Describe
import "lib/format.mjs" as Format

// A number leaf of the device's description, or one component of a range leaf, as a slider:
// bounds, step and unit come from the description (10.3 D), never from constants of the
// widget. The value is sent once, when the drag, key press or wheel step is over: one set
// request with the leaf (for a range component: the whole range with the other component as
// shown). Hidden when the device lacks the leaf or the slider has no bounds and step.
LabeledSlider {
    id: item

    property var link
    property string path: ""
    property string text: ""
    // "" for a number leaf; "min" or "max" for one end of a range leaf
    property string component: ""

    readonly property var setting: link.setting(path)
    readonly property var bounds: setting === null ? null : component === "" ? Describe.numberSpec(setting) : Describe.numberSpec(setting, component)
    readonly property var steps: Describe.sliderScale(bounds)
    readonly property var shownValue: link.value(path)
    readonly property var block: link.blocked(path)

    function current() {
        if (shownValue === null || shownValue === undefined)
            return null;
        return component === "" ? shownValue : shownValue[component];
    }

    visible: link.has(path) && steps !== null
    enabled: block === null
    opacity: enabled ? 1 : 0.38
    label: text + (link.status(path) === "pending" ? " · pending" : "") + (link.setting(path)?.no_effect ? " · no visible effect" : "") + (block !== null && block.reason === "requires" ? " · needs " + block.needs.map(n => n.path + " = " + n.equals).join(", ") : "")
    minimum: 0
    maximum: steps !== null ? steps.count : 0
    value: steps !== null && current() !== null ? steps.toIndex(current()) : 0
    formatValue: i => item.steps === null ? "" : item.current() === null && !item.dragging ? "–" : Format.number(item.steps.toValue(i), item.bounds.precision, item.bounds.unit)
    onReleased: i => {
        if (item.steps === null)
            return;
        const v = item.steps.toValue(i);
        if (item.component === "") {
            item.link.set(item.path, v);
            return;
        }
        const range = Object.assign({}, item.shownValue ?? {});
        range[item.component] = v;
        if (range.min !== undefined && range.max !== undefined && range.min <= range.max)
            item.link.set(item.path, range);
        else
            item.sync();
    }
}
```

Notes for the builder: an `Item` already has a property `state` (QML states), which is why the camera's state is `camState`; `qmllint` reports a `property-override` otherwise, and the same for `scale` in the slider. Every function reads `revision`, so a binding that calls it is re-evaluated after each change.

- [ ] **Step 3: Run the gates**

Run: `bash scripts/check-qml.sh && scripts/check-js.sh && shellcheck scripts/*.sh`
Expected: `check-qml: ok (47 files)`, `check-js: ok (87 passed, …)`. Nothing uses the components yet; the manual check list M1 (items M-1 to M-4) covers them once Task 8 has landed.

- [ ] **Step 4: CHANGELOG and commit**

`CHANGELOG.md`, `## [Unreleased]`, `### Added`: `- The widget connects to the obsbot4linux 1.0 daemon on its Unix socket (protocol 2) instead of running \`obsbot4linux watch\`; controls are bound to a path of the camera's description and show a pending value as "pending" (#N).`

```bash
git add CameraLink.qml SettingToggle.qml SettingChips.qml SettingSlider.qml CHANGELOG.md
git commit -S -m "CameraLink: protocol 2 on the daemon's socket; controls bound to description paths"
```

---

### Task 7: End-to-end test against the fake daemon

Needs B-1 on the tool's `rebuild`. Stacked on Task 5.

**Files:**

- Create: `tests/e2e/node-link.mjs`, `tests/e2e/fake-daemon.test.mjs`
- Modify: `.github/workflows/ci.yml`, `CHANGELOG.md`; `tests/fixtures/fakes.mjs` only if the daemon's hello differs from it (step 3)

**Interfaces:**

- Consumes: `lib/link.mjs`, `lib/describe.mjs`, `lib/devices.mjs`, `lib/lists.mjs`, `lib/gimbal.mjs`, `tests/fixtures/fakes.mjs` (`HELLO`, Task 2); B-1: `obsbot4linuxd --fake fake-a,fake-b,unknown`, `OBSBOT4LINUX_SOCKET`, `<socket>.test`.
- Produces: `class NodeLink(path, client)` (`start()`, `stop()`, `run(effects)`, `until(check, ms, what) -> Promise`, `failures`), `exchange(path, lines, count) -> Promise<object[]>`; CI job `e2e` inside `required`.

- [ ] **Step 1: Write the test and its adapter**

Create `tests/e2e/node-link.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// lib/link.mjs over a node socket: what CameraLink.qml does with Quickshell's Socket, for the
// e2e test against a running fake daemon.

import { createConnection } from "node:net";
import { Link } from "../../lib/link.mjs";

export class NodeLink {
  constructor(path, client) {
    this.path = path;
    this.link = new Link(client);
    this.socket = null;
    this.buffer = "";
    this.timers = [];
    this.failures = [];
    this.waiters = [];
  }

  start() {
    this.run(this.link.start());
  }

  stop() {
    this.run(this.link.stop());
    this.timers.forEach(clearTimeout);
  }

  run(effects) {
    for (const e of effects) {
      if (e.connect) {
        const s = createConnection(this.path);
        this.socket = s;
        s.setEncoding("utf8");
        s.on("connect", () => this.run(this.link.connected()));
        s.on("data", (chunk) => {
          this.buffer += chunk;
          let i;
          while ((i = this.buffer.indexOf("\n")) >= 0) {
            const line = this.buffer.slice(0, i);
            this.buffer = this.buffer.slice(i + 1);
            this.run(this.link.line(line));
          }
        });
        s.on("error", (err) =>
          this.run(
            this.link.closed(
              err.code === "ENOENT" ? 2 : err.code === "ECONNREFUSED" ? 0 : 1,
            ),
          ),
        );
        s.on("close", () => this.run(this.link.closed(undefined)));
      } else if (e.close) {
        this.socket?.destroy();
      } else if (e.write !== undefined) {
        this.socket.write(e.write);
      } else if (e.retryIn !== undefined && e.retryIn >= 0) {
        this.timers.push(
          setTimeout(() => this.run(this.link.retry()), e.retryIn),
        );
      } else if (e.failed) {
        this.failures.push(e.failed);
      }
    }
    this.waiters = this.waiters.filter((w) => !w());
  }

  // Resolves once check() is true after an effect, rejects after ms.
  until(check, ms, what) {
    return new Promise((resolve, reject) => {
      if (check()) return resolve();
      const timer = setTimeout(() => reject(new Error(`timeout: ${what}`)), ms);
      this.waiters.push(() => {
        if (!check()) return false;
        clearTimeout(timer);
        resolve();
        return true;
      });
    });
  }
}

// One line on a socket (the test-control socket, or a second client like the CLI); resolves
// with the lines received until `count` arrived.
export function exchange(path, lines, count) {
  return new Promise((resolve, reject) => {
    const s = createConnection(path);
    let buffer = "";
    const got = [];
    s.setEncoding("utf8");
    s.on("connect", () =>
      s.write(lines.map((l) => JSON.stringify(l) + "\n").join("")),
    );
    s.on("data", (chunk) => {
      buffer += chunk;
      let i;
      while ((i = buffer.indexOf("\n")) >= 0) {
        got.push(JSON.parse(buffer.slice(0, i)));
        buffer = buffer.slice(i + 1);
      }
      if (got.length >= count) {
        s.destroy();
        resolve(got);
      }
    });
    s.on("error", reject);
    setTimeout(() => {
      s.destroy();
      reject(new Error(`no answer on ${path}`));
    }, 10000);
  });
}
```

Create `tests/e2e/fake-daemon.test.mjs`:

```javascript
// SPDX-License-Identifier: GPL-3.0-or-later
//
// The widget's link against the real daemon in fake mode (spec 10.3 D, "Acceptance without
// camera"): OBSBOT4LINUXD names the daemon binary, started here as
// `obsbot4linuxd --fake fake-a,fake-b,unknown` on a temporary socket. Run: node --test tests/e2e/

import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { NodeLink, exchange } from "./node-link.mjs";
import { choices, has, limit } from "../../lib/describe.mjs";
import { marker, unsupportedHint, usable } from "../../lib/devices.mjs";
import { presetSlots } from "../../lib/lists.mjs";
import { moveRequest, moveSpec, stepTarget } from "../../lib/gimbal.mjs";
import { HELLO } from "../fixtures/fakes.mjs";

const daemon = process.env.OBSBOT4LINUXD;
const A = "sn:FAKEA0000001";
const B = "sn:FAKEB0000001";
const UNKNOWN = "port:9-3";
let dir;
let sock;
let child;

async function waitForFile(path, ms) {
  const end = Date.now() + ms;
  while (!existsSync(path)) {
    if (Date.now() > end) throw new Error(`${path} did not appear`);
    await new Promise((r) => setTimeout(r, 50));
  }
}

async function connected() {
  const n = new NodeLink(sock, "obsbot4linux-dux e2e");
  n.start();
  await n.until(() => n.link.ready, 20000, "hello reply");
  return n;
}

before(async () => {
  assert.ok(daemon, "set OBSBOT4LINUXD to the obsbot4linuxd binary");
  dir = mkdtempSync(join(tmpdir(), "obsbot-dux-e2e-"));
  sock = join(dir, "obsbot4linux.sock");
  child = spawn(daemon, ["--fake", "fake-a,fake-b,unknown"], {
    env: Object.assign({}, process.env, { OBSBOT4LINUX_SOCKET: sock }),
    stdio: ["ignore", "inherit", "inherit"],
  });
  await waitForFile(sock, 10000);
  await waitForFile(sock + ".test", 10000);
});

after(() => {
  child?.kill("SIGTERM");
  if (dir) rmSync(dir, { recursive: true, force: true });
});

test("hello lists the three fakes; nothing is chosen with two usable devices", async () => {
  const n = await connected();
  try {
    assert.deepEqual(
      n.link.store.devices.map((d) => d.id).sort(),
      [UNKNOWN, A, B].sort(),
    );
    assert.equal(n.link.device(), null);
    const unknown = n.link.store.device(UNKNOWN);
    assert.equal(marker(unknown), "unsupported");
    assert.equal(usable(unknown), false);
    assert.match(unsupportedHint(unknown), /obsbot4linux devices --report/);
  } finally {
    n.stop();
  }
});

// The unit tests rely on tests/fixtures/fakes.mjs, written by hand: its device entries and first
// states must be what the daemon sends. Runs before the tests below change values. firmware and
// video_node are left out (the daemon may fill them in for a fake); on_air is the daemon's probe.
test("the unit tests' fixtures match the fake daemon's hello", async () => {
  const n = await connected();
  try {
    const fields = [
      "id",
      "name",
      "model",
      "support",
      "serial",
      "available",
      "usb",
      "settings",
      "actions",
      "limits",
    ];
    const facts = (d) => Object.fromEntries(fields.map((k) => [k, d[k]]));
    for (const want of HELLO.devices) {
      const got = n.link.store.device(want.id);
      assert.ok(got, `${want.id} is listed`);
      assert.deepEqual(facts(got), facts(want), want.id);
    }
    await n.until(
      () =>
        HELLO.devices.every(
          (d) => n.link.store.state(d.id)?.unread?.length === 0,
        ),
      5000,
      "the first reads",
    );
    const withoutOnAir = (state) => {
      const copy = Object.assign({}, state);
      delete copy.on_air;
      return copy;
    };
    for (const id of Object.keys(HELLO.states))
      assert.deepEqual(
        withoutOnAir(n.link.store.state(id)),
        withoutOnAir(HELLO.states[id].state),
        id,
      );
  } finally {
    n.stop();
  }
});

test("fake-b: unknown group carried, missing groups absent, own values, bounds and presets", async () => {
  const n = await connected();
  try {
    n.run(n.link.pick(B));
    const d = n.link.device();
    assert.equal(has(d, "gimbal"), false);
    assert.equal(has(d, "views"), false);
    assert.deepEqual(
      choices(d, "tracking.mode", {}).map((c) => c.value),
      ["off", "person", "desk"],
    );
    assert.equal(limit(d, "presets"), 5);
    assert.deepEqual(presetSlots(d), [0, 1, 2, 3, 4]);
    assert.equal(typeof n.link.shown("beacon.color"), "string");
  } finally {
    n.stop();
  }
});

test("a value set by another client shows at once (pending or read back)", async () => {
  const n = await connected();
  try {
    n.run(n.link.pick(B));
    const before = n.link.shown("led.brightness");
    const next = before === 4 ? 2 : 4;
    // a second client, as the CLI is: hello, then one set
    const replies = await exchange(
      sock,
      [
        {
          id: 1,
          hello: { protocol: 2, client: "e2e-cli", subscribe: false, live: [] },
        },
        { id: 2, device: B, set: { "led.brightness": next } },
      ],
      2,
    );
    assert.ok(replies[1].ok, JSON.stringify(replies[1]));
    await n.until(
      () => n.link.shown("led.brightness") === next,
      2000,
      "the other client's value",
    );
  } finally {
    n.stop();
  }
});

test("the widget's own set: one request, own value until the reply", async () => {
  const n = await connected();
  try {
    n.run(n.link.pick(B));
    const effects = n.link.set("tracking.mode", "person");
    assert.equal(effects.filter((e) => "write" in e).length, 1);
    n.run(effects);
    assert.equal(n.link.status("tracking.mode"), "own");
    await n.until(() => n.link.own.size === 0, 5000, "the reply");
    assert.equal(n.link.shown("tracking.mode"), "person");
  } finally {
    n.stop();
  }
});

test("a gimbal step on fake-a is one gimbal.move and leaves the tracking mode alone", async () => {
  const n = await connected();
  try {
    n.run(n.link.pick(A));
    const mode = n.link.shown("tracking.mode");
    const target = stepTarget(
      moveSpec(n.link.device()),
      n.link.state().gimbal,
      null,
      Date.now(),
      5,
      0,
    );
    assert.notEqual(target, null, "fake-a reports its pose");
    const r = moveRequest(target);
    const effects = n.link.act(r.action, r.params);
    assert.equal(effects.filter((e) => "write" in e).length, 1);
    n.run(effects);
    await new Promise((res) => setTimeout(res, 500));
    assert.deepEqual(n.failures, []);
    assert.equal(n.link.shown("tracking.mode"), mode);
  } finally {
    n.stop();
  }
});

test("unplugging the selected unsupported device: it goes, nothing is chosen for the user", async () => {
  const n = await connected();
  try {
    n.run(n.link.pick(UNKNOWN));
    const reply = await exchange(sock + ".test", [{ unplug: UNKNOWN }], 1);
    assert.ok(reply[0].ok, JSON.stringify(reply[0]));
    await n.until(() => !n.link.store.device(UNKNOWN), 5000, "the removal");
    assert.equal(n.link.device(), null);
    await exchange(sock + ".test", [{ plug: "unknown" }], 1);
    await n.until(
      () => n.link.store.devices.some((d) => d.support === "unsupported"),
      5000,
      "the re-plug",
    );
  } finally {
    n.stop();
  }
});
```

- [ ] **Step 2: Run it without a daemon and see it fail**

Run: `node --test tests/e2e/fake-daemon.test.mjs`
Expected: failure in `before`: `set OBSBOT4LINUXD to the obsbot4linuxd binary`. It never skips.

- [ ] **Step 3: Run it against the fake daemon**

Build the daemon from a checkout of the tool's `rebuild` at `$TOOL` (scratch directory, `nice -n 19`, at most 4 jobs; never under `/tmp`): `CARGO_TARGET_DIR=$SCRATCH/target nice -n 19 cargo build -j 4 --locked --manifest-path "$TOOL/Cargo.toml" -p obsbot4linuxd`, then:
Run: `OBSBOT4LINUXD=$SCRATCH/target/debug/obsbot4linuxd node --test tests/e2e/fake-daemon.test.mjs && node scripts/sync-fixtures.mjs --check "$TOOL"`
Expected: pass 7, fail 0 (node prints `ℹ pass 7` with its default reporter, `# pass 7` with `--test-reporter=tap`), exit 0; the fixture check exits 0. A failure of "the unit tests' fixtures match the fake daemon's hello" shows a difference between `tests/fixtures/fakes.mjs` and the daemon's hello: it is fixed in `fakes.mjs` (what the daemon sends wins) in this pull request, and the unit tests are run again (`scripts/check-js.sh`).

- [ ] **Step 4: CI job `e2e`**

Replace `.github/workflows/ci.yml` with:

```yaml
name: CI

# Pull requests into main and into v1 (the widget 1.0 branch) only: this workflow produces the
# `required` check that the rulesets of both branches require, so it must not run on push or by
# dispatch (that would mark a commit green that was never a pull request head). No paths filter,
# so the check always reports.
on:
  pull_request:
    branches: [main, v1]

permissions:
  contents: read

jobs:
  # Ubuntu's own Qt 6.4 qmllint does not accept the script's options. Arch ships the Qt that the
  # desktop shell runs. The digest pins the image; -Syu installs the current Arch Qt, so a new Qt
  # with a new lint can turn a pull request red: fix that in its own PR, never by loosening the script.
  qml:
    runs-on: ubuntu-24.04
    container: archlinux:latest@sha256:d4594bb2300cba11f41c2f27923eb56f63d9988909c402f9c95517d1e362c899
    timeout-minutes: 15
    steps:
      - run: pacman -Syu --noconfirm --needed qt6-declarative
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - run: bash scripts/check-qml.sh

  # The JavaScript modules (lib/*.mjs): their tests in node and in the QML engine.
  js:
    runs-on: ubuntu-24.04
    container: archlinux:latest@sha256:d4594bb2300cba11f41c2f27923eb56f63d9988909c402f9c95517d1e362c899
    timeout-minutes: 15
    steps:
      - run: pacman -Syu --noconfirm --needed qt6-declarative nodejs
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - run: bash scripts/check-js.sh

  # The widget's link against the daemon in fake mode, built from the tool's 1.0 branch
  # (spec 10.3 D: acceptance against obsbot4linuxd --fake fake-a,fake-b,unknown). Also checks that
  # tests/fixtures matches the tool's generated files.
  e2e:
    runs-on: ubuntu-24.04
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          repository: hellwand/obsbot4linux
          ref: rebuild
          path: tool
          persist-credentials: false
      - run: cargo build --locked --manifest-path tool/Cargo.toml -p obsbot4linuxd
      - run: node scripts/sync-fixtures.mjs --check tool
      - run: node --test tests/e2e/fake-daemon.test.mjs
        env:
          OBSBOT4LINUXD: tool/target/debug/obsbot4linuxd

  shellcheck:
    runs-on: ubuntu-24.04
    container: archlinux:latest@sha256:d4594bb2300cba11f41c2f27923eb56f63d9988909c402f9c95517d1e362c899
    timeout-minutes: 15
    steps:
      - run: pacman -Syu --noconfirm --needed shellcheck
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - run: shellcheck scripts/*.sh

  # The version in plugin.json must have a non-empty CHANGELOG.md section (what the release needs).
  changelog:
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - run: scripts/release-notes.sh HEAD "v$(jq -r .version plugin.json)" >/dev/null

  # The one check the main ruleset requires. It passes only when every gate succeeded.
  required:
    name: required
    needs: [qml, js, e2e, shellcheck, changelog]
    if: always()
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    env:
      QML: ${{ needs.qml.result }}
      JS: ${{ needs.js.result }}
      E2E: ${{ needs.e2e.result }}
      SHELLCHECK: ${{ needs.shellcheck.result }}
      CHANGELOG: ${{ needs.changelog.result }}
    steps:
      - run: |
          echo "qml=$QML js=$JS e2e=$E2E shellcheck=$SHELLCHECK changelog=$CHANGELOG"
          test "$QML" = success
          test "$JS" = success
          test "$E2E" = success
          test "$SHELLCHECK" = success
          test "$CHANGELOG" = success
```

Run: `actionlint .github/workflows/ci.yml`
Expected: silent. Acceptance is GitHub's: this pull request shows `e2e=success` in the `required` job's log.

- [ ] **Step 5: CHANGELOG and commit**

`CHANGELOG.md`, `## [Unreleased]`, `### Added`: `- CI tests the widget's link against the obsbot4linux daemon in fake mode (two fake cameras of different shape and one unsupported device), built from obsbot4linux's 1.0 branch (#N).`

```bash
git add tests/e2e .github/workflows/ci.yml CHANGELOG.md   # and tests/fixtures/fakes.mjs if step 3 changed it
git commit -S -m "CI: the widget's link against obsbot4linuxd --fake"
```

---

### Task 8: Popout root, pill and Control Center on `CameraLink`

Stacked on Task 6. After this task the cards still read 0.29 names; Tasks 9–15 move them. `v1` is not shipped, so the popout may show empty cards between these pull requests; the gates stay green because `qmlcachegen` compiles each file on its own.

**Files:**

- Modify: `ObsbotWidget.qml`, `PillContent.qml`, `CHANGELOG.md`

**Interfaces:**

- Consumes: Task 6 (`CameraLink` and its interface), `lib/labels.mjs` (`labelsFor`, `problemText`), `lib/describe.mjs` (`humanize`, `choices`), `lib/format.mjs`.
- Produces: the root interface `ctl` of part C of "Shared interfaces of the QML tasks" (header), with exactly those member names; the code of the new members is there too.

- [ ] **Step 1: The check that fails first**

Run: `grep -c 'CameraWatch\|OptimisticState\|send(\[' ObsbotWidget.qml`
Expected: a count above 0 (the 0.29 data path). After this task it is 0.

- [ ] **Step 2: Replace the data path**

In `ObsbotWidget.qml`:

1. Imports: add `import "lib/labels.mjs" as Labels`, `import "lib/describe.mjs" as Describe`.
2. Delete every member listed under "Removed from the root" in part C of "Shared interfaces of the QML tasks", the `CameraWatch { … }` block, the `OptimisticState { … }` block, the `Timer` blocks `dragHold` and `presetTimeout`, and the comment lines that describe them.
3. Add, where `CameraWatch` was:

```qml
    CameraLink {
        id: cameraLink
        active: root.realInstance
        instanceName: root.instanceName
        client: "obsbot4linux-dux"
        // Live topics only while the popout is open (spec 10.3 D); each only if the device names it
        liveView: ({
                open: root.interactionActive,
                pose: root.cardEnabled("Framing"),
                target: root.showTargetBox,
                auto: root.cardEnabled("Picture")
            })
        // "Hide without camera" counts any usable camera, picked or not (decision 11)
        onAnyUsableChanged: root.applyVisibility()
    }
    // Not `id: link`: an id and a property of the same name shadow each other (qmllint
    // id-shadows-member); inside this file `link` is this alias
    readonly property alias link: cameraLink
```

4. `refresh()` becomes `cameraLink.retry()`. `onAvailableChanged` keeps `previewOn = false` when `available` turns false and calls `applyVisibility()`. Every other read of the root's own `awake` in this file becomes `live` (part C: `awake` is removed). The preview stop rule keeps its shape with `live` (the state's own `asleep`; there is no optimistic value of it any more): replace `awakeReported` by `live` in `onPreviewOnChanged` and `onAwakeReportedChanged` (renamed `onLiveChanged`), and `onAwakeChanged` by the same handler.

- [ ] **Step 3: The new members**

Add the member code of part C of "Shared interfaces of the QML tasks", exactly, below the `CameraLink` block of step 2.

- [ ] **Step 4: Control Center and pill**

`ccWidgetIcon: link.device === null || !available ? "usb_off" : live ? "videocam" : "videocam_off"`; `ccWidgetSecondaryText: root.baseSummary + (root.streaming ? " · On air" : "")`; `ccWidgetIsActive: root.live`; `onCcWidgetToggled: root.toggleAwake()`. `applyVisibility()` becomes `if (hideWithoutCamera) setVisibilityOverride(link.anyUsable); else clearVisibilityOverride();`: with "Hide without camera" on, the pill stays while any usable camera is listed, also while two are listed and none is picked yet (spec 8.4 step 7; the picker is in the popout), and it is called from `onAnyUsableChanged` (step 2), `onAvailableChanged`, `onHideWithoutCameraChanged` and `Component.onCompleted` as before (manual item M-24).

In `PillContent.qml`: the four hidden width texts become `"Static"`, `"Tracking"`, `"Asleep"`, `"Offline"` plus `"Unsupported"` (the slot is as wide as the widest); the visible text binds to `pill.ctl.pillText`. The other `ctl` members it reads (`available`, `live`, `streaming`, `summary`, `stepZoom`, `toggleTracking`, `wheelZoom`, `middleClickTracking`, `statusTextInBar`) keep their names.

- [ ] **Step 5: Run the gates**

Run: `grep -c 'CameraWatch\|OptimisticState\|send(\[' ObsbotWidget.qml PillContent.qml; bash scripts/check-qml.sh && scripts/check-js.sh`
Expected: `0` for both files; `check-qml: ok (47 files)`; `check-js: ok (87 passed, …)`. Manual items: M-1 to M-6 and M-24 (run in M1, after wave 8).

- [ ] **Step 6: CHANGELOG and commit**

`CHANGELOG.md`, `## [Unreleased]`, `### Changed`: `- The bar pill, the Control Center tile and the popout read the camera from obsbot4linux 1.0's daemon. Values the widget sends show at once and stay until the daemon answers; there is no 3 s hold any more (#89). The pill says "Unsupported" for a camera obsbot4linux does not support (#N).`

```bash
git add ObsbotWidget.qml PillContent.qml CHANGELOG.md
git commit -S -m "Popout, pill and Control Center on CameraLink"
```

---

### Task 9: Header and device picker

**Files:**

- Create: `DevicePicker.qml`
- Modify: `HeaderBar.qml`, `CHANGELOG.md`

**Interfaces:**

- Consumes: Task 8's `ctl` (`link`, `available`, `live`, `problem`, `summary`, `toggleAwake()`, `arranging`, `resetLayout()`, `refresh()`), Task 6's `CameraLink` (`devices`, `device`, `pick(id)`, `marker`, `unsupportedHint`), `lib/devices.mjs` (`marker`, `usable`).
- Produces: `DevicePicker` (an `Item` with `property var link`), used by `HeaderBar.qml` here and by `DesktopSide.qml` in Task 15.

- [ ] **Step 1: The check that fails first**

Run: `grep -n 'pendingLabel\|ctl.setAwake\|ctl\.awake\b' HeaderBar.qml`
Expected: matches (0.29 names). After this task: none.

- [ ] **Step 2: `DevicePicker.qml`**

Behaviour, exactly:

1. Visible when `link.devices.length > 1`, or when one device is listed and it is not the one shown (`link.device === null`) or carries a marker (`link.marker !== ""`).
2. One chip per entry of `link.devices`, in the daemon's order (a `ChipRow` with `stacked: true` and `label: "Camera"`; options from the entries). Chip text: `device.name`, then ` · untested` / ` · unsupported` / ` · <level>` from `Devices.marker(device)` when not empty, then ` · unplugged` when `device.available !== true`.
3. `currentIndex` = index of `link.deviceId` in `link.devices`, `-1` when none; a pick calls `link.pick(id)` (no request goes out) and `sync()`.
4. Below the chips, while the shown device is unsupported: `link.unsupportedHint` as selectable `StyledText` (`TextEdit` read-only is fine), wrapping, `Theme.surfaceVariantText`, `Theme.fontSizeSmall` (spec 8.4 step 3).
5. While `link.device === null` and at least one device is listed: the line "Pick a camera to control." in the same style.

- [ ] **Step 3: `HeaderBar.qml`**

1. The status line binds to `bar.ctl.summary`; remove the `pendingLabel` busy indicator and its text (decision 12: each control shows its own value until the reply).
2. Every read of `bar.ctl.awake` (the header icon and the Awake switch) becomes `bar.ctl.live` (part C removes `awake`). The Awake switch: visible `bar.ctl.available && !bar.ctl.arranging`, `checked: bar.ctl.live`, toggled → `bar.ctl.toggleAwake()`; disabled while `bar.ctl.link.blocked(bar.ctl.live ? "sleep" : "wake") !== null`.
3. While `bar.ctl.problem !== null`: a second line with `problem.body` (it holds the `systemctl --user enable --now obsbot4linux.socket` line for "unreachable", spec 5.10) and a "Retry" button → `bar.ctl.refresh()`.
4. Under the header, `DevicePicker { width: parent.width; link: bar.ctl.link }`.

- [ ] **Step 4: Run the gates**

Run: `grep -n 'pendingLabel\|ctl.setAwake\|ctl\.awake\b' HeaderBar.qml; bash scripts/check-qml.sh && scripts/check-js.sh`
Expected: no match; `check-qml: ok (48 files)`; check-js ok. Manual items M-2, M-3, M-8, M-9.

- [ ] **Step 5: CHANGELOG and commit**

`CHANGELOG.md`, `### Added`: `- A camera picker in the popout header when obsbot4linux lists more than one camera, with the marks "untested" and "unsupported"; for an unsupported camera the widget shows no controls and says how to report it (\`obsbot4linux devices --report\`). Without a running daemon the header shows the command that starts it (#N).`

```bash
git add DevicePicker.qml HeaderBar.qml CHANGELOG.md
git commit -S -m "Header: camera picker, support marks, unsupported and no-daemon hints"
```

---

### Task 10: Tracking card

**Files:** Modify `TrackingSection.qml`, `HandControls.qml`, `CHANGELOG.md`.

**Interfaces:** Consumes Task 8's `ctl` (`link`, `labels()`, `available`, `live`, `compact` stays a property of the card, `showTargetBox`, `setShowTargetBox()`), Task 6's bound controls, `lib/format.mjs`. Produces nothing new.

- [ ] **Step 1: The check that fails first**

Run: `grep -c 'ctl\.send\|ctl\.status\|ctl\.cam\|ctl\.has(' TrackingSection.qml HandControls.qml`
Expected: counts above 0. After this task: 0 and 0.

- [ ] **Step 2: Rewire `TrackingSection.qml`**

| 0.29 control                | 1.0                                              | Component                       | Text                                                                          | Special                                                                                                                                                                   |
| --------------------------- | ------------------------------------------------ | ------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| mode chips (`DFilterChips`) | `tracking.mode`                                  | `SettingChips`, `stacked: true` | "" (no label, as today)                                                       | `labels: ctl.labels("tracking.mode")`; a value the description does not list shows as text after the (empty) label                                                        |
| Speed                       | `tracking.profile`                               | `SettingChips`                  | "Speed"                                                                       | `labels: ctl.labels("tracking.profile")`                                                                                                                                  |
| Tracked hand                | `hand.side`                                      | `SettingChips`                  | "Tracked hand"                                                                | visible also needs `!compact`; gated only by `has()` and `blocked()` (no mode gate of the widget's own, product question 5); the hint "Used in Hand mode" is display only |
| Return here                 | do `hand.return_here`                            | `DButton`                       | "Return here"                                                                 | shown while `!compact && ctl.link.has("hand.return_here")`; enabled while `blocked("hand.return_here") === null` (`asleep: ignored` blocks it while asleep); no mode gate |
| Go to return position       | do `hand.go_return`                              | `DButton`                       | "Go to return position"                                                       | gated only by `has()` and `blocked()`; no gate on a stored return position (the text below says "none")                                                                   |
| Clear                       | do `hand.return_clear`                           | `DButton`                       | "Clear"                                                                       | gated only by `has()` and `blocked()`; no optimistic `null` (the state shows it)                                                                                          |
| return position text        | `hand.return`, `hand.area.pan`, `hand.area.tilt` | `StyledText`                    | "Return position: " + `Format.value(…)` or "none"; "Hand area: pan …, tilt …" | the area as `Format.value` of both ranges                                                                                                                                 |
| `HandControls`              | (Task 10 step 3)                                 | –                               | –                                                                             | visible while `!compact && ctl.link.value("tracking.mode") === "hand" && ctl.link.has("hand")`                                                                            |
| Follow new targets          | `zone.follow`                                    | `SettingToggle`                 | "Follow new targets"                                                          | visible also needs `!compact`                                                                                                                                             |
| Show target box in preview  | widget setting                                   | `ToggleRow`                     | as today                                                                      | visible while `ctl.link.has("tracking.target.box") && !compact`                                                                                                           |

Delete `sides`, `hand`, `handReturn`, `handArea`, `handReturnShown`, `deg()`, `modeIndex()` and the `Connections` block.

- [ ] **Step 3: Rewire `HandControls.qml`**

| 0.29 control              | 1.0                  | Component                                          | Text                                          | Special                                                                                                                       |
| ------------------------- | -------------------- | -------------------------------------------------- | --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Hand area: pan from / to  | `hand.area.pan`      | two `SettingSlider`, `component: "min"` / `"max"`  | "Hand area: pan from" / "Hand area: pan to"   | each sends the whole range with the other end as shown; a release that would put `min` above `max` sends nothing and re-syncs |
| Hand area: tilt from / to | `hand.area.tilt`     | two `SettingSlider` (`component: "min"` / `"max"`) | "Hand area: tilt from" / "Hand area: tilt to" | as the pan pair                                                                                                               |
| Default area              | do `hand.area_reset` | `DButton`                                          | "Default area"                                | –                                                                                                                             |
| Hand moves pan            | `hand.axes.pan`      | `SettingToggle`                                    | "Hand moves pan"                              | `hint: "Off: the pan axis stays where it is"`                                                                                 |
| Hand moves tilt           | `hand.axes.tilt`     | `SettingToggle`                                    | "Hand moves tilt"                             | `hint: "Off: the tilt axis stays where it is"`                                                                                |
| Return after              | `hand.return_after`  | `SettingSlider`                                    | "Return after"                                | hidden while the description gives no bounds (decision 10; today on the Tiny 2 Lite)                                          |
| Hand moves the gimbal     | `hand.move_gimbal`   | `SettingToggle`                                    | "Hand moves the gimbal"                       | `hint: "Off stops the gimbal."`; `null` shows "Not known until it is set here" (readback:false)                               |

Delete `fullArea`, the area properties, `setArea()`, `sendArea()`, the hold timer, `returnAfter`, `returnAfterKnown` and the `ctl.handGimbal` use.

- [ ] **Step 4: Run the gates**

Run: `grep -c 'ctl\.send\|ctl\.status\|ctl\.cam\|ctl\.has(' TrackingSection.qml HandControls.qml; bash scripts/check-qml.sh && scripts/check-js.sh`
Expected: `0` and `0`; both gates ok. Manual items M-4, M-5, M-10.

- [ ] **Step 5: CHANGELOG and commit**

`### Changed`: `- Tracking card: every control follows the camera's description, so a camera with other tracking modes shows its own; the hand area sliders send one end at a time; "Hand moves the gimbal" shows "not known" until it is set, as obsbot4linux cannot read it (#N).`

```bash
git add TrackingSection.qml HandControls.qml CHANGELOG.md
git commit -S -m "Tracking card on description paths"
```

---

### Task 11: Framing and Zone cards

**Files:** Modify `FramingSection.qml`, `ZoneSection.qml`, `ZoneViews.qml`, `CHANGELOG.md`.

**Interfaces:** Consumes Task 8's `ctl` (`link`, `labels()`, `fineGimbal`, `gimbalStep`, `degrees()`, `cardEnabled()`), `lib/gimbal.mjs` (`moveSpec`, `stepTarget`, `moveRequest`, `panLeftSign`), `lib/lists.mjs` (`viewRows`, `nameProblem`), `lib/format.mjs`, Task 6's bound controls.

- [ ] **Step 1: The check that fails first**

Run: `grep -c 'ctl\.send\|ctl\.moveGimbal\|ctl\.cam\|Moving switches tracking' FramingSection.qml ZoneSection.qml ZoneViews.qml`
Expected: counts above 0. After: 0, 0, 0.

- [ ] **Step 2: The gimbal arrows in `FramingSection.qml`**

Add `import "lib/gimbal.mjs" as Gimbal` and these members:

```qml
    readonly property var moveSpec: Gimbal.moveSpec(ctl.link.device)
    // The target of the last own move and when it went out ({pan, tilt, at}); the base of the
    // next step for STEP_BASE_MS (decision 12)
    property var lastMove: null
    // The arrows need a pose to step from; while it is not read they are disabled
    readonly property bool poseKnown: typeof ctl.link.value("gimbal.pan") === "number" && typeof ctl.link.value("gimbal.tilt") === "number"

    // One arrow press: one gimbal.move to the computed target, nothing else (spec 2, 12.3)
    function step(dPan, dTilt) {
        if (moveSpec === null || ctl.link.blocked("gimbal.move") !== null)
            return;
        const pose = { pan: ctl.link.value("gimbal.pan"), tilt: ctl.link.value("gimbal.tilt") };
        const sign = Gimbal.panLeftSign(ctl.link.camState);
        const target = Gimbal.stepTarget(moveSpec, pose, lastMove, Date.now(), dPan * sign, dTilt);
        // no pose to step from: nothing is sent (the widget never makes a target up)
        if (target === null)
            return;
        lastMove = { pan: target.pan, tilt: target.tilt, at: Date.now() };
        const request = Gimbal.moveRequest(target);
        ctl.link.act(request.action, request.params);
    }
```

The arrow buttons call `section.step(±ctl.gimbalStep, 0)` / `section.step(0, ±ctl.gimbalStep)` with the signs of 0.29 (left = `+`, right = `−`, up = `+`, down = `−`); the centre button sends one `ctl.link.act("gimbal.move", {pan: 0, tilt: 0})` when `0` lies inside both bounds of `moveSpec`. The pad is visible while `moveSpec !== null` (fake-b has no gimbal: no pad); the four arrows are enabled only while `section.poseKnown` and `ctl.link.blocked("gimbal.move") === null` (the centre button needs no pose). The text "Moving switches tracking to Static." is deleted (spec 2, 15.1 row 88). The pose line: `"Pan " + Format.value(ctl.link.setting("gimbal.pan"), ctl.link.value("gimbal.pan")) + " · Tilt " + Format.value(ctl.link.setting("gimbal.tilt"), ctl.link.value("gimbal.tilt"))` (an unread pose shows "–", never 0), visible while `ctl.link.has("gimbal.pan")`.

- [ ] **Step 3: The rest of `FramingSection.qml`**

| 0.29 control           | 1.0                             | Component                       | Text                             | Special                                                                                          |
| ---------------------- | ------------------------------- | ------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------ |
| Step chips (5°/1°)     | widget setting `ctl.fineGimbal` | `ChipRow`                       | "Step"                           | visible with the pad                                                                             |
| Zoom                   | `zoom.factor`                   | `SettingSlider`                 | "Zoom"                           | bounds and precision from the description (fake-b: 1–2)                                          |
| Start pose text        | `power_on.start.pose`           | `StyledText`                    | "Start pose" + `Format.value(…)` | visible while `ctl.link.has("power_on.start.pose")`                                              |
| Use current            | do `power_on.use_current`       | `DButton`                       | "Use current"                    | `asleep: ignored` blocks it while asleep                                                         |
| Go                     | do `power_on.go`                | `DButton`                       | "Go"                             | `asleep: ignored` blocks it while asleep                                                         |
| Start mode             | `power_on.start.mode`           | `SettingChips`, `stacked: true` | "Start mode"                     | `labels: ctl.labels("power_on.start.mode")`; values outside `writable` show but cannot be picked |
| start mode explanation | –                               | `StyledText`                    | as today                         | visible with the chips                                                                           |

Delete `startModes`, `startModeIndex`, `startPose` and the `Connections` block.

- [ ] **Step 4: `ZoneSection.qml`**

| 0.29 control          | 1.0                   | Component                                          | Text                                    | Special                                                                                                                                                                                            |
| --------------------- | --------------------- | -------------------------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| card                  | group `zone`          | –                                                  | "Zone"                                  | `visible: ctl.cardEnabled("Zone") && ctl.link.has("zone")`; summary: `"Zone " + (ctl.link.value("zone.enabled") ? "on" : "off") + " · pan " + Format.value(pan) + " · tilt " + Format.value(tilt)` |
| Zone tracking         | `zone.enabled`        | `SettingToggle`                                    | "Zone tracking"                         | –                                                                                                                                                                                                  |
| Pan from / to         | `zone.area.pan`       | two `SettingSlider` (`component: "min"` / `"max"`) | "Pan from" / "Pan to"                   | each sends the whole range with the other end as shown (`SettingSlider`, part B)                                                                                                                   |
| Tilt from / to        | `zone.area.tilt`      | two `SettingSlider` (`component: "min"` / `"max"`) | "Tilt from" / "Tilt to"                 | as the pan pair                                                                                                                                                                                    |
| Return here           | do `zone.return_here` | `DButton`                                          | "Return here"                           | `asleep: ignored` blocks it                                                                                                                                                                        |
| Go to return position | do `zone.go_return`   | `DButton`                                          | as today                                | –                                                                                                                                                                                                  |
| return position text  | `zone.return`         | `StyledText`                                       | "Return position: " + `Format.value(…)` | –                                                                                                                                                                                                  |
| `ZoneViews`           | step 5                | –                                                  | –                                       | visible while `ctl.link.has("views")` (fake-b: hidden)                                                                                                                                             |

Delete `fullArea`, `zone`, `cameraArea`, the four area properties, `editing`, `syncArea()`, `setArea()`, `sendArea()`, `areaHold`, `zoneOn`, `fullAreaSet`, `num()`.

- [ ] **Step 5: `ZoneViews.qml`**

1. Rows: `Lists.viewRows(ctl.link.camState)`; `null` → the line "Views not read yet" (no timer); `[]` → "No views saved".
2. Per row: title `row.label` (placeholder `(unnamed)` for an empty name is display only); "Go" → `act("view.go", {id})`; "Rename" (inline field) → `act("view.rename", {id, name})`, the save button disabled while `Lists.nameProblem(device, "view.rename", "name", text)` is not null (the problem shows as the field's hint); "Delete" with the inline confirmation of 0.29 → `act("view.delete", {id})`.
3. "Save current pose as view": field plus button → `act("view.save", {name})`; disabled while `nameProblem(device, "view.save", "name", text)` is not null or `blocked("view.save")` is not null (asleep).
4. A name that starts with `-` is sent as typed (0.29 #103: nothing parses it any more).
5. Name fields (part B): the rename field is prefilled with `row.name` (never `row.label`, so an unnamed view opens an empty field); in both fields `maximumLength: 32`, the `RegularExpressionValidator` and every `trim()` go; the buttons are gated by `nameProblem` as in items 2 and 3, and the typed text is sent unchanged.

- [ ] **Step 6: Run the gates**

Run: `grep -c 'ctl\.send\|ctl\.moveGimbal\|ctl\.cam\|Moving switches tracking' FramingSection.qml ZoneSection.qml ZoneViews.qml; bash scripts/check-qml.sh && scripts/check-js.sh`
Expected: three `0`; both gates ok. Manual items M-4, M-5, M-11, M-12.

- [ ] **Step 7: CHANGELOG and commit**

`### Changed`: `- Framing card: the gimbal arrows send only the move; they no longer switch the tracking to Static first (use the tracking mode for that). The zoom slider and the zone area take their bounds from the camera's description (#N).`

```bash
git add FramingSection.qml ZoneSection.qml ZoneViews.qml CHANGELOG.md
git commit -S -m "Framing and Zone cards: one gimbal.move per arrow, bounds from the description"
```

---

### Task 12: Presets card

**Files:** Modify `PresetsSection.qml`, `PresetSlot.qml`, `CHANGELOG.md`.

**Interfaces:** Consumes Task 8's `ctl` and `lib/lists.mjs` (`presetSlots`, `presetRows`, `saveParams`, `nameProblem`, `UNNAMED`). Produces `PresetSlot { property var link; property var row }` (`row` from `presetRows`), used by `DesktopBottom.qml` in Task 15.

- [ ] **Step 1: The check that fails first**

Run: `grep -c 'presetCommand\|refreshPresets\|presetsFailed\|ctl\.presets' PresetsSection.qml PresetSlot.qml`
Expected: above 0 for `PresetsSection.qml`. After: 0, 0.

- [ ] **Step 2: Rewire `PresetsSection.qml`**

1. Card: `visible: ctl.cardEnabled("Presets") && ctl.link.has("presets")`.
2. `readonly property var rows: Lists.presetRows(ctl.link.device, ctl.link.camState)`; `null` → "Presets not read yet" (no timer, no "Reading presets…"); the `Repeater` runs over `rows` (fake-b: five rows, the Tiny 2 Lite: three).
3. Row title: `"Slot " + (row.slot + 1) + (row.used ? ": " + row.label : "")`; `row.label` is `(unnamed)` for an empty name, display only (spec 12.3).
4. "Recall" (enabled while `row.used`) → `act("preset.recall", {slot})`; "Save current pose" → `act("preset.save", Lists.saveParams(row.slot, ""))`: no name (spec 12.3), disabled while `blocked("preset.save")` is not null (asleep); "Rename" (inline field, enabled while `row.used`) → `act("preset.rename", {slot, name})`, save disabled while `nameProblem(device, "preset.rename", "name", text)` is not null; "Delete" with the inline confirmation of 0.29 (enabled while `row.used`) → `act("preset.delete", {slot})`.
5. Delete every use of `presetCommand`, `refreshPresets`, `presetsFailed`, `presetsRequested`, `listedPresets` and `Component.onCompleted` that asked for the list.
6. The rename field (part B, "Name fields"): prefilled with `row.name` where 0.29 used `row.preset?.name` (never `row.label`: saving an unnamed preset's field unchanged must not write "(unnamed)" to the camera, spec 12.3); `maximumLength: 32`, the `RegularExpressionValidator` and the `trim()` calls go; the save button is gated by `nameProblem` (item 4) and the typed text is sent unchanged.

- [ ] **Step 3: `PresetSlot.qml`**

`property var link`, `property var row`; the button text is `row.label` (or `"Slot " + (row.slot + 1)` while unused, disabled); a click sends one `link.act("preset.recall", {slot: row.slot})`.

- [ ] **Step 4: Run the gates**

Run: `grep -c 'presetCommand\|refreshPresets\|presetsFailed\|ctl\.presets' PresetsSection.qml PresetSlot.qml; bash scripts/check-qml.sh && scripts/check-js.sh`
Expected: `0`, `0`; both gates ok. Manual items M-13, M-14.

- [ ] **Step 5: CHANGELOG and commit**

`### Changed`: `- Presets card: as many slots as the camera has; "Save current pose" sends no name, and a preset without a name shows "(unnamed)" here only. The list comes with the camera's state, so "Reading presets…" and its time-out are gone (#N).`

```bash
git add PresetsSection.qml PresetSlot.qml CHANGELOG.md
git commit -S -m "Presets card: slots from the description, no name unless typed"
```

---

### Task 13: Picture, Image and Factory reset cards

**Files:** Modify `PictureSection.qml`, `ImageSection.qml`, `ResetSection.qml`, `CHANGELOG.md`.

**Interfaces:** Consumes Task 8's `ctl`, Task 6's bound controls and `CameraLink.defaultsPatch(paths)`, `CameraLink.warnings(path)`.

- [ ] **Step 1: The check that fails first**

Run: `grep -c 'ctl\.setControl\|ctl\.setExposure\|ctl\.setHdr\|ctl\.image\|ctl\.factoryReset' PictureSection.qml ImageSection.qml ResetSection.qml`
Expected: above 0. After: 0, 0, 0.

- [ ] **Step 2: `PictureSection.qml`**

| 0.29 control                      | 1.0               | Component       | Text            | Special                                                                                                               |
| --------------------------------- | ----------------- | --------------- | --------------- | --------------------------------------------------------------------------------------------------------------------- |
| HDR                               | `exposure.hdr`    | `SettingToggle` | "HDR"           | –                                                                                                                     |
| Exposure (Global / Face / Custom) | `exposure.mode`   | `SettingChips`  | "Exposure"      | decision 7; `labels: ctl.labels("exposure.mode")`                                                                     |
| (part of the same row)            | `exposure.region` | `SettingChips`  | "Metering"      | decision 7                                                                                                            |
| Exposure time                     | `exposure.time`   | `SettingSlider` | "Exposure time" | blocked with "· needs exposure.mode = manual" while automatic (decision 8)                                            |
| Gain                              | `exposure.gain`   | `SettingSlider` | "Gain"          | the description gives it no `requires`, so it stays usable in Auto exposure; no gate of the widget's own (decision 8) |

- [ ] **Step 3: `ImageSection.qml`**

```qml
    // The sliders of this card; the defaults reset covers exactly these (10.3 D: the widget's
    // product choice, unchanged from 0.29)
    readonly property var controls: [
        { path: "image.brightness", text: "Brightness" },
        { path: "image.contrast", text: "Contrast" },
        { path: "image.saturation", text: "Saturation" },
        { path: "image.sharpness", text: "Sharpness" },
        { path: "exposure.compensation", text: "Backlight compensation" }
    ]
    readonly property var resetPatch: ctl.link.defaultsPatch(controls.map(c => c.path))
```

A `Repeater` over `controls` with one `SettingSlider { link: ctl.link; path: modelData.path; text: modelData.text }` each. "Reset to defaults" → one `ctl.link.setPatch(section.resetPatch)`, enabled while `resetPatch !== null` (nothing to reset: disabled). The card is visible while `ctl.cardEnabled("Image")` and any of the paths is in the description.

- [ ] **Step 4: `ResetSection.qml`**

Visible while `ctl.cardEnabled("Reset") && ctl.link.has("factory_reset")`. The first click arms (unchanged, with its 10 s arming timer, decision 12); "Reset now" sends one `ctl.link.act("factory_reset", {confirm: true})` and sets `ctl.previewOn = false`. The warning text is shown because `ctl.link.warnings("factory_reset")` holds `destructive`; the sentence about the restart is shown when it holds `restart`.

- [ ] **Step 5: Run the gates**

Run: `grep -c 'ctl\.setControl\|ctl\.setExposure\|ctl\.setHdr\|ctl\.image\|ctl\.factoryReset' PictureSection.qml ImageSection.qml ResetSection.qml; bash scripts/check-qml.sh && scripts/check-js.sh`
Expected: three `0`; both gates ok. Manual items M-15, M-16, M-17.

- [ ] **Step 6: CHANGELOG and commit**

`### Changed`: `- Picture card: Exposure (Auto / Manual) and Metering (Face / Global) are two rows; exposure time stays greyed out, with the reason, until the exposure is manual. Image card: "Reset to defaults" sends the camera's own defaults of its sliders in one request (#N).`

```bash
git add PictureSection.qml ImageSection.qml ResetSection.qml CHANGELOG.md
git commit -S -m "Picture, Image and Reset cards on description paths"
```

---

### Task 14: Device tab cards

**Files:** Modify `CameraSection.qml`, `SleepSection.qml`, `AudioSection.qml`, `GestureSection.qml`, `CHANGELOG.md`.

**Interfaces:** Consumes Task 8's `ctl`, Task 6's bound controls, `lib/format.mjs`.

- [ ] **Step 1: The check that fails first**

Run: `grep -c 'ctl\.send\|ctl\.status\|ctl\.has(\|ctl\.cam\|ctl\.ledBlink' CameraSection.qml SleepSection.qml AudioSection.qml GestureSection.qml`
Expected: above 0. After: four `0`.

- [ ] **Step 2: The four cards**

| Card     | 0.29 control              | 1.0                         | Component       | Text                          | Special                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| -------- | ------------------------- | --------------------------- | --------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Camera   | Field of view             | `zoom.field_of_view`        | `SettingChips`  | "Field of view"               | `custom` shows, cannot be picked; asleep: ignored                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Camera   | Status LED                | `led.brightness`            | `SettingSlider` | "Status LED"                  | 0.29 had chips 0–3; bounds now from the description (fake-b 0–5)                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Camera   | Blink status LED          | `led.blink`                 | `SettingToggle` | "Blink status LED"            | readback:false: "Not known until it is set here"                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Camera   | Mirror image              | `image.mirror`              | `SettingToggle` | "Mirror image"                | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Camera   | Invert gimbal             | `gimbal.invert`             | `SettingToggle` | "Invert gimbal"               | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Camera   | info lines                | device entry                | `StyledText`    | name, firmware, `video_node`  | firmware `null` → "–"                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Sleep    | Auto-sleep (Off / times)  | `sleep.auto.enabled`        | `SettingToggle` | "Auto-sleep"                  | decision 7                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Sleep    | (times)                   | `sleep.auto.seconds`        | `ChipRow`       | "After", or "After (<value>)" | shortcut options `[30, 120, 600]` s (the widget's product choice, decision 16) shown as "30 s", "2 min", "10 min", only those inside the description's bounds; selected when the value equals one; when none matches (fake-a starts at 1 s) the label shows the camera's value, `"After (" + Format.value(ctl.link.setting("sleep.auto.seconds"), ctl.link.value("sleep.auto.seconds")) + ")"`, so it is never hidden; one `set` of the seconds only (it does not switch auto-sleep on, spec 4.3) |
| Sleep    | While asleep              | `sleep.background`          | `SettingChips`  | "While asleep"                | `own_image` shows, cannot be picked                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Sleep    | Tracking mode at power-on | `power_on.mode`             | `SettingChips`  | "Tracking mode at power-on"   | `labels: ctl.labels("power_on.mode")`                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Audio    | Noise reduction           | `audio.noise_reduction`     | `SettingChips`  | "Noise reduction"             | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Audio    | Pickup distance           | `audio.volume`              | `ChipRow`       | "Pickup distance"             | Near / Standard / Far set 10 / 50 / 100 (spec 4.3, 9.1: "UIs may offer those three as volume presets"); the selected chip is the band of the shown volume: below 50, 50–84, from 85                                                                                                                                                                                                                                                                                                               |
| Audio    | Microphone volume         | `audio.volume`              | `SettingSlider` | "Microphone volume"           | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Audio    | Automatic gain            | `audio.auto_gain`           | `SettingToggle` | "Automatic gain"              | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Audio    | Microphone while asleep   | `audio.in_sleep`            | `SettingToggle` | "Microphone while asleep"     | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Gestures | Target lock               | `gestures.target_lock`      | `SettingToggle` | "Target lock"                 | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Gestures | Zoom                      | `gestures.zoom`             | `SettingToggle` | "Zoom"                        | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Gestures | Dynamic zoom              | `gestures.dynamic_zoom`     | `SettingToggle` | "Dynamic zoom"                | –                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Gestures | Direction change          | `gestures.direction_change` | `SettingToggle` | "Direction change"            | `hint: "Reverses which hand movement zooms in or out with Dynamic zoom."`                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Gestures | Zoom gesture ratio        | `gestures.zoom_ratio`       | `SettingSlider` | "Zoom gesture ratio"          | step 0.1 from the description                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

Each card is visible while `ctl.cardEnabled("<Card>")` and any of its paths is in the description. Delete the 0.29 option arrays (`fovs`, `levels`, `distances`, `backgrounds`, `gestures` with `arg`/`key`) and every `Connections` block.

- [ ] **Step 3: Run the gates**

Run: `grep -c 'ctl\.send\|ctl\.status\|ctl\.has(\|ctl\.cam\|ctl\.ledBlink' CameraSection.qml SleepSection.qml AudioSection.qml GestureSection.qml; bash scripts/check-qml.sh && scripts/check-js.sh`
Expected: four `0`; both gates ok. Manual items M-18, M-19.

- [ ] **Step 4: CHANGELOG and commit**

`### Changed`: `- Device tab: auto-sleep is a switch plus its time; the status LED brightness is a slider with the camera's own range; "Blink status LED" shows "not known" until it is set (#N).`

```bash
git add CameraSection.qml SleepSection.qml AudioSection.qml GestureSection.qml CHANGELOG.md
git commit -S -m "Device tab cards on description paths"
```

---

### Task 15: Desktop widget and preview frame

**Files:**

- Modify: `DesktopPreview.qml`, `DesktopSide.qml`, `DesktopBottom.qml`, `PreviewFrame.qml`, `CHANGELOG.md`
- Delete: `CameraWatch.qml`, `OptimisticState.qml` (no user left after Task 8 and this task)

**Interfaces:**

Depends on Tasks 8, 9 and 12 (it uses `DevicePicker.qml` and `PresetSlot.qml`); branches from `v1` after they landed.

- Consumes: Task 6 (`CameraLink`), Task 8's `ctl` members used by `PreviewFrame` in the popout (`link`, `available`, `live`, `previewOn`, `previewReady`, `previewRunning`, `videoNode`, `showTargetBox`, `interactionActive`, `problem`, `streaming`, `streamingApps`, `refresh()`), Task 9's `DevicePicker`, Task 12's `PresetSlot`, `lib/lists.mjs`, `lib/labels.mjs`.
- Produces: `DesktopPreview` (the desktop surface) exposes to `DesktopSide`, `DesktopBottom` and `PreviewFrame` the same member names as Task 8's `ctl` for these: `link`, `labels()`, `available`, `live`, `problem`, `previewOn`, `previewReady`, `previewRunning`, `videoNode`, `showTargetBox`, `interactionActive`, `streaming`, `streamingApps`, `modeText`, `toggleAwake()`, `stepZoom()`, `refresh()`.

- [ ] **Step 1: The check that fails first**

Run: `ls CameraWatch.qml OptimisticState.qml; grep -c 'CameraWatch\|OptimisticState\|send(\[' DesktopPreview.qml`
Expected: both files listed; a count above 0. After: `ls` fails for both; the count is 0.

- [ ] **Step 2: `DesktopPreview.qml`**

1. Replace its `CameraWatch`, `OptimisticState`, `trackingModes`, `speeds`, `speedOptions`, `status`, `aiMode`, `zoomFactor`, `zoomRange`, `has()`, `send()`, the preset list request and its timer by `CameraLink { id: cameraLink; active: !root.inert; instanceName: …as today; client: "obsbot4linux-dux"; liveView: ({ open: false, pose: false, target: false, auto: false }) }` (no live topics on the desktop, decision 13) and the members listed under "Produces" (with `readonly property alias link: cameraLink`, and the members `labels()`, `available`, `live`, `problem`, `streaming`, `streamingApps`, `videoNode`, `previewReady`, `modeText`, `toggleAwake()`, `stepZoom()` copied from the member code of part C of "Shared interfaces of the QML tasks"; `refresh()` is `cameraLink.retry()`; `previewRunning`, `appNames` and `showTargetBox` stay as the desktop widget has them).
2. The inert rule (editing, lock screen) stays: `active: !root.inert` switches the connection off there, as the watch was.
3. Its own `awake`, `awakeReported`, `assumeAwake()` and `toolMissing` go (part C); its preview stop rule reads `live` where it read `awake` or `awakeReported`, as Task 8 step 2 item 4 does in the popout root.

- [ ] **Step 3: `DesktopSide.qml` and `DesktopBottom.qml`**

| 0.29 control        | 1.0                           | Component                                                                                                                 | Text             | Special                                                                                     |
| ------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------- |
| status lines        | `ctl.modeText`, `ctl.problem` | `StyledText`                                                                                                              | as today         | `problem.body` while not connected                                                          |
| On air              | `ctl.streaming`               | as today                                                                                                                  | "On air"         | –                                                                                           |
| (new) camera picker | –                             | `DevicePicker { link: ctl.link }`                                                                                         | –                | at the top of the side column                                                               |
| Tracking mode       | `tracking.mode`               | `SettingChips`                                                                                                            | "Tracking mode"  | the two hidden `DFilterChips` that measured the width now measure the description's choices |
| Tracking speed      | `tracking.profile`            | `SettingChips`                                                                                                            | "Tracking speed" | –                                                                                           |
| Zoom − / slider / + | `zoom.factor`                 | `DButton`, `SettingSlider`, `DButton`                                                                                     | "Zoom"           | − and + call `ctl.stepZoom(-1)` / `ctl.stepZoom(1)`                                         |
| Presets row         | `presets`                     | `Repeater` over `Lists.presetRows(link.device, link.camState) ?? []` with `PresetSlot { link: ctl.link; row: modelData }` | "Presets"        | as many as the camera has; "No presets saved" while none is used                            |
| Sleep / Wake        | do `sleep` / `wake`           | `DButton`                                                                                                                 | as today         | `ctl.toggleAwake()`                                                                         |

In both files, besides the table: every `ctl.awake` becomes `ctl.live`; `ctl.toolMissing` becomes `ctl.problem !== null`; `ctl.aiMode` becomes `ctl.modeText`; `ctl.setAwake(…)` becomes `ctl.toggleAwake()`; the comment at the top of `DesktopSide.qml` that lists the `ctl` members it reads names the 1.0 members.

- [ ] **Step 4: `PreviewFrame.qml`**

1. Hints: while `ctl.problem !== null`: icon `usb_off`, title `problem.title`, body `problem.body`, button "Retry" → `ctl.refresh()` (replaces "Control tool not found" and its PATH text); while the shown device is unsupported: title "Camera not supported", body `ctl.link.unsupportedHint`, no button; while no device is shown: "No camera" / "Plug in a camera, or pick one."
2. "Waking camera…" shows while `ctl.previewOn && !ctl.live` (there is no `assumeAwake` any more; the state says when the camera is awake, spec 4.4).
3. The target box reads `ctl.link.value("tracking.target.box")`, an object `{left, top, right, bottom}` in picture fractions or `null` (4.3), instead of a four-number array: `visible: frame.previewShown && ctl.showTargetBox && box !== null`, and the rectangle uses `box.left`, `box.top`, `box.right - box.left`, `box.bottom - box.top`.
4. The video node is `ctl.videoNode` (the device entry's `video_node`; no `readlink` any more).
5. Every other 0.29 read in the file, mapped exactly:

| 0.29 read                             | 1.0                                                              | Why                                                                                                                                                                                                   |
| ------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ctl.awake`, `ctl.awakeReported`      | `ctl.live`                                                       | there is no optimistic value any more, so both are the state's `asleep` (part C)                                                                                                                      |
| `ctl.assumeAwake()` in `hintAction()` | removed with its comment line                                    | decision 12; `startedAsleep` and `frameSeen` may stay, they no longer change the result                                                                                                               |
| `ctl.toolMissing`                     | `ctl.problem !== null` (item 1)                                  | the problem is the daemon, not a missing command                                                                                                                                                      |
| `ctl.cam?.streaming`                  | `ctl.link.value("on_air.active") === true`                       | the daemon's on-air sample; not `ctl.streaming`, which also counts our own preview                                                                                                                    |
| `ctl.cam?.streaming_apps`             | `(ctl.link.value("on_air.apps") ?? []).map(a => String(a.name))` | the daemon's list as it is, `"qs"` included: `otherAppStreams` needs it to tell the shell's own preview on another surface from another app; not `ctl.streamingApps`, which drops the daemon's `"qs"` |
| `ctl.cam?.target?.box`                | `ctl.link.value("tracking.target.box")` (item 3)                 | an object, not an array                                                                                                                                                                               |
| `ctl.cam?.target?.detected === false` | `ctl.link.value("tracking.target.detected") === false`           | the box's border colour                                                                                                                                                                               |

- [ ] **Step 5: Delete the 0.29 data path**

`git rm CameraWatch.qml OptimisticState.qml`. Then `grep -rn 'CameraWatch\|OptimisticState' *.qml` must print nothing.

- [ ] **Step 6: Run the gates**

Run: `grep -rn 'CameraWatch\|OptimisticState' *.qml; grep -n 'send(\[\|ctl\.awake\|ctl\.cam\b\|toolMissing\|assumeAwake' DesktopPreview.qml DesktopSide.qml DesktopBottom.qml PreviewFrame.qml; bash scripts/check-qml.sh && scripts/check-js.sh` (the second grep covers this task's files only: the wave 7 cards may not all have landed when this task runs)
Expected: no match; `check-qml: ok (46 files)` (two files gone, `DevicePicker.qml` from Task 9 present); check-js ok. Manual items M-20, M-21, M-22.

- [ ] **Step 7: CHANGELOG and commit**

`### Changed`: `- Desktop widget: its own link to the obsbot4linux 1.0 daemon, a camera picker, as many preset buttons as the camera has; the preview frame says when the daemon is not running and how to start it, and when a camera is not supported (#N).` `### Removed`: `- The \`obsbot4linux watch\` process and the 3 s hold of values (\`CameraWatch.qml\`, \`OptimisticState.qml\`) (#N).`

```bash
git add DesktopPreview.qml DesktopSide.qml DesktopBottom.qml PreviewFrame.qml CHANGELOG.md
git commit -S -m "Desktop widget and preview frame on CameraLink; drop the watch process"
```

---

### Task 16: Widget 1.0 for the release candidate: startup check, `plugin.json`, README

Base `v1`, after M1 passed and once the tool's release candidate `v1.0.0-rc.1` exists (stage W3). From this commit on, the `v1` head refuses an obsbot4linux older than 1.0 (the dev builds of `rebuild` report 0.13.2 until W3), which is why it comes after M1; W4 (M2) runs this head against the candidate. The version 1.0.0, the CHANGELOG section and the e2e job's tag are release-day steps (R1), not this task.

**Files:** Modify `StartupCheck.qml`, `ObsbotSettings.qml` (only if the check below finds a setting without effect), `plugin.json`, `README.md`, `CHANGELOG.md`.

**Interfaces:** Consumes `lib/version.mjs` (`checkTool(stdout, exitCode, min, max | null, links) -> null | {title, details}`, Task 2). Produces the 1.0 startup check.

- [ ] **Step 1: The tests that pin the 1.0 bound**

Task 2's `tests/version.test.mjs` already holds "widget 1.0 refuses a 0.x tool and names the minimum" and "a release candidate counts as its release". Run: `scripts/check-js.sh`; expected ok.

- [ ] **Step 2: `StartupCheck.qml`**

Replace the whole file by (compiled with Qt 6.12 `qmlcachegen` for this plan):

```qml
import QtQuick
import qs.Common
import "lib/version.mjs" as Version

// Dependency gate: the plugin activates only with obsbot4linux 1.0 or newer, which speaks
// protocol 2 on its socket. A release candidate of 1.0.0 counts as 1.0.0. The camera itself is
// not required.
QtObject {
    readonly property var minVersion: [1, 0, 0]
    readonly property var links: ({
            releases: "https://github.com/hellwand/obsbot4linux/releases",
            widgetReleases: "https://github.com/hellwand/obsbot4linux-dux/releases"
        })

    function check(done) {
        Proc.runCommand("obsbot4linuxDux.depCheck", ["sh", "-c", "command -v obsbot4linux >/dev/null && obsbot4linux --version"], (stdout, exitCode) => done(Version.checkTool(stdout, exitCode, minVersion, null, links)));
    }
}
```

- [ ] **Step 3: `plugin.json`, settings, README**

1. `plugin.json`: `dependencies` become `"obsbot4linux >= 1.0"` and `"qt6-multimedia"`; `description`: `"Bar widget and desktop preview for OBSBOT webcams, built on obsbot4linux 1.0: the controls each camera's description offers, a camera picker and a live preview"`. `version` stays as it is (R1 sets 1.0.0; the CI changelog job needs a section for the version in `plugin.json`).
2. `ObsbotSettings.qml`: check each setting against Tasks 8–15 (the card switches, wheel zoom, middle click, hide without camera, status text, target box); remove one only if nothing reads it any more, and list the result in the pull request.
3. `README.md`: "Install" needs obsbot4linux 1.0 or newer and `systemctl --user enable --now obsbot4linux.socket` once (1.0 clients start no daemon, spec 5.10, 12.2); "Supported cameras" says that the controls follow each camera's description and that untested and unsupported cameras are marked; "Update" says that `dms plugins update` follows the repository's `main` and that the widget and the tool are updated together; the paragraphs about `capabilities` and the `obsbot4linux watch` process go.

- [ ] **Step 4: Gates**

Run: `bash scripts/check-qml.sh && scripts/check-js.sh && shellcheck scripts/*.sh`
Expected: all ok. Manual: install the candidate, `dms restart`, the plugin activates; with obsbot4linux 0.13 installed it refuses with "obsbot4linux 1.0.0 or newer is required".

- [ ] **Step 5: CHANGELOG and commit**

`### Changed`: `- Needs obsbot4linux 1.0 or newer: the startup check refuses older versions and accepts a 1.0 release candidate. Enable the daemon's socket once with \`systemctl --user enable --now obsbot4linux.socket\` (#N).`

```bash
git add StartupCheck.qml ObsbotSettings.qml plugin.json README.md CHANGELOG.md
git commit -S -m "Startup check, plugin.json and README for obsbot4linux 1.0"
```

---

## Manual check list (M1, the coordinator, on the owner's desktop)

Run after wave 8 (Task 15) has landed on `v1`, before Task 16, against the fake daemon. Setup, all read-only for the camera (none is plugged in):

1. Stop anything that holds the default socket: `systemctl --user stop obsbot4linux.socket obsbot4linux.service`; end an `obsbot4linux daemon` process if one runs (the 0.13 widget's watch started it).
2. Build the daemon from `rebuild` in a scratch directory (`nice -n 19`, at most 4 jobs) and run `obsbot4linuxd --fake fake-a,fake-b,unknown` in a terminal (default socket path).
3. Point the plugin at `v1`: `git -C ~/.config/dms/plugins/obsbot4linuxDux fetch origin v1 && git -C ~/.config/dms/plugins/obsbot4linuxDux switch --detach origin/v1`, then `dms restart` (the one restart). Afterwards: `git … switch main` and `dms restart` at the next natural restart.
4. A second client and the test control: `S=$XDG_RUNTIME_DIR/obsbot4linux.sock`; `printf '%s\n' '{"id":1,"hello":{"protocol":2,"client":"m1","subscribe":false,"live":[]}}' '<request>' | socat -t 2 - UNIX-CONNECT:$S`; test control: `printf '%s\n' '{"unplug":"port:9-3"}' | socat -t 2 - UNIX-CONNECT:$S.test`.

| Id   | Check                                                                                                                                                                                                                                                                 | Expected                                                                                                                                                                                                                            | Task       |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| M-1  | Open the popout                                                                                                                                                                                                                                                       | header lists the camera picker with "Fake A 0001", "Fake B 0001 · untested", "Unknown Camera 9-3 · unsupported"; "Pick a camera to control."                                                                                        | 8, 9       |
| M-2  | Pick "Fake B 0001", stop the fake daemon, wait 10 s, start it again                                                                                                                                                                                                   | header: "obsbot4linux is not running" with the `systemctl --user enable --now obsbot4linux.socket` line while it is down; back without a click within 5 s after it is up, with Fake B still shown (the pick survives the reconnect) | 6, 9       |
| M-3  | Pick "Unknown Camera 9-3"                                                                                                                                                                                                                                             | no controls; the unsupported hint with `obsbot4linux devices --report` and the issues link; pill "Unsupported"                                                                                                                      | 8, 9       |
| M-4  | Pick "Fake B 0001"                                                                                                                                                                                                                                                    | Tracking chips Off / Person / Desk; no gimbal pad, no Zone views; zoom slider 1.00–2.00×; Presets card five slots                                                                                                                   | 10, 11, 12 |
| M-5  | Pick "Fake A 0001", open Framing, press an arrow                                                                                                                                                                                                                      | the pose moves; the tracking chip stays where it was; header shows no "Static" switch                                                                                                                                               | 11         |
| M-6  | Close the popout                                                                                                                                                                                                                                                      | `{"live":[]}` reaches the daemon (daemon log or a `socat` subscriber sees no pose events); open again: pose events come                                                                                                             | 8          |
| M-7  | With Fake B shown: `{"id":2,"device":"sn:FAKEB0000001","set":{"led.brightness":4}}` from the second client (or `obsbot4linux set led.brightness=4 --device FAKEB` once track C landed)                                                                                | the Status LED slider shows 4 at once, marked "pending" until the daemon's next read of it shows 4, then plain                                                                                                                      | 6, 14      |
| M-8  | Test control `{"unplug":"port:9-3"}` while the unknown camera is shown                                                                                                                                                                                                | the entry goes; nothing is picked (two usable left), "Pick a camera to control."                                                                                                                                                    | 9          |
| M-9  | Test control `{"plug":"unknown"}`                                                                                                                                                                                                                                     | the entry is back, marked unsupported                                                                                                                                                                                               | 9          |
| M-10 | Fake A, Tracking card: pick "Hand"                                                                                                                                                                                                                                    | tracked-hand chips enabled, hand controls shown; "Hand moves the gimbal" says "Not known until it is set here"                                                                                                                      | 10         |
| M-11 | Fake A, Zone: drag "Pan from" and release                                                                                                                                                                                                                             | one request; the slider shows the dragged value until the reply, then the camera's                                                                                                                                                  | 11         |
| M-12 | Fake A, Zone views: save a view named "-x"                                                                                                                                                                                                                            | it is listed as "-x"                                                                                                                                                                                                                | 11         |
| M-13 | Fake A, Presets: "Save current pose" on slot 1, no name typed; then "Rename" on slot 1                                                                                                                                                                                | slot 1 shows "(unnamed)" in the widget; the second client's `get presets` shows `"name":""`; the rename field opens empty, not with "(unnamed)"                                                                                     | 12         |
| M-14 | Fake B, Presets: recall slot 5                                                                                                                                                                                                                                        | one request; no error toast                                                                                                                                                                                                         | 12         |
| M-15 | Fake A, Picture: exposure Auto                                                                                                                                                                                                                                        | Exposure time greyed out with "needs exposure.mode = manual"; Gain stays usable (its description has no `requires`); choose Manual: Exposure time usable                                                                            | 13         |
| M-16 | Fake A, Image: move Brightness, release, then "Reset to defaults"                                                                                                                                                                                                     | brightness back at the description's default; the button is disabled afterwards                                                                                                                                                     | 13         |
| M-17 | Fake A, Factory reset                                                                                                                                                                                                                                                 | arms first; "Reset now" sends once; the device goes for about a second and comes back                                                                                                                                               | 13         |
| M-18 | Test control `{"sleep":"sn:FAKEB0000001"}`, Fake B shown                                                                                                                                                                                                              | zoom slider greyed out (asleep: ignored); the LED slider stays usable                                                                                                                                                               | 14         |
| M-19 | Fake A, Device tab: auto-sleep switch on, pick "2 min"                                                                                                                                                                                                                | two separate requests, one per control                                                                                                                                                                                              | 14         |
| M-20 | Desktop widget on                                                                                                                                                                                                                                                     | own camera picker; presets row with as many buttons as the shown camera has                                                                                                                                                         | 15         |
| M-21 | Desktop widget, stop the daemon                                                                                                                                                                                                                                       | the preview frame shows the not-running hint with the `systemctl` line                                                                                                                                                              | 15         |
| M-22 | Popout and desktop widget both open, Fake B shown in both, set a value in one                                                                                                                                                                                         | the other shows it at once                                                                                                                                                                                                          | 6, 15      |
| M-23 | Popout (each tab) and desktop widget with Fake A, then Fake B, then the unknown device                                                                                                                                                                                | cards stack per column without row gaps; the camera picker, Metering and the auto-sleep time rows add no empty space; a screenshot of the complete popout per tab and of the desktop widget goes into the tracking issue            | 8–15       |
| M-24 | Settings: "Hide without camera" on; fake-a, fake-b and the unknown device listed, nothing picked (restart the fake daemon first if an earlier item left one picked or unplugged); then test control `{"unplug":"sn:FAKEA0000001"}` and `{"unplug":"sn:FAKEB0000001"}` | the bar pill stays visible while nothing is picked and the popout shows the camera picker; once neither usable camera is plugged in the pill hides. Afterwards switch the setting off and restart the fake daemon                   | 8          |

Results go into the track D tracking issue, one line per item. A failing item is fixed in the owning task's files by a new pull request before Task 16.

**M2 (W4, spec 10.3 W4 "widget 1.0 against the candidate").** The same list against the release candidate on the real Tiny 2 Lite, with these camera items: CAM-34 (what an arrow does while tracking: the widget sends only the move), CAM-35 (the name the camera reports for a preset saved without one: the widget shows "(unnamed)" for an empty name only), CAM-3 and CAM-22 (what is usable while asleep), CAM-28 (the upgrade rehearsal with the widget).

## Track D acceptance (spec 10.3), mapped

| Acceptance without camera (spec 10.3 D)                                             | Where                                                                                                                                                                                              |
| ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| first, the socket check from the shell (else `relay`)                               | P1 (coordinator), seam in Task 6                                                                                                                                                                   |
| second, whether `dms plugins update` follows the default branch or tags             | P2 (answered from the shell's source: the default branch); decision 1                                                                                                                              |
| `hello`, device picker                                                              | Tasks 3, 5, 6, 9; tests `hello goes first …`, `after hello: two usable devices …`; M-1                                                                                                             |
| every control gated on a path or action of the description                          | Task 4 `has`, `blocked`; Task 6 bound controls; Tasks 10–15 rule 1; M-4                                                                                                                            |
| enum values, bounds, step, unit and list limits from the description                | Task 4 tests `choices come from the description …`, `number bounds …`, `a slider scale …`, `preset slots follow the description`; M-4                                                              |
| `pending` values shown                                                              | Task 4 `own value, then pending …`, `a write of another client shows at once through pending`; Task 5 `own value until the reply …`; Task 7 e2e `a value set by another client shows at once`; M-7 |
| live topics only while the popup is open                                            | Task 4 `live topics: none while closed …`; Task 3 `the live list is sent only when it changes …`; Task 8 `liveView`; M-6                                                                           |
| markers for `untested` and `unsupported`                                            | Task 4 `markers …`; Task 9; M-1, M-3                                                                                                                                                               |
| the exit-4 hint                                                                     | Task 3 `no daemon listening is unreachable`; Task 4 `the unreachable hint carries the systemctl line`; Tasks 9, 15; M-2, M-21                                                                      |
| defaults reset built from `default` values (a `set` patch)                          | Task 4 `the defaults reset patch`; Task 13; M-16                                                                                                                                                   |
| each control sends its one request and chains none                                  | Task 5 `a setting control writes exactly one set request …`, `a gimbal arrow writes exactly one gimbal.move …`; Task 7 e2e; card rule 2; M-5, M-19                                                 |
| widget 0.28.0, the last 0.x, refuses obsbot4linux ≥ 1.0                             | already released (0.28.0, #105); Task 1 and R0 add the `plugin.json` string (decision 2)                                                                                                           |
| fake-b's unknown setting does not break the widget                                  | Task 5 `fake-b's unknown group is carried and ignored`; Task 7 e2e                                                                                                                                 |
| its missing groups hide their controls                                              | Task 4 `fake-b has no gimbal and no views …`; Task 7; M-4                                                                                                                                          |
| its own enum values and bounds and its preset count                                 | Task 4 tests; Task 7 e2e; M-4                                                                                                                                                                      |
| the unsupported device shows as in 8.4 step 3 and disappears on unplug as in step 7 | Task 4 `the unsupported hint …`, `selection: …`; Task 5 `unplugging the selected unsupported device`; Task 7 e2e; M-3, M-8                                                                         |
| a value set with the CLI appears in the widget at once (`pending`)                  | Task 7 e2e (a second client); M-7                                                                                                                                                                  |
| a gimbal control sends one request                                                  | Task 5 and Task 7 (the requests the client wrote; the fake's operation log is not on the test socket); review of Task 11; M-5                                                                      |
| widget 0.28.0 released before the 1.0 release candidate                             | done (0.28.0, 0.29.0, 0.29.1 released); R0 before the candidate                                                                                                                                    |

## Camera items this track leaves open

None gates a task of this plan; all are W4 (M2) or track A items whose answers the widget shows as the state reports them:

- CAM-34: what the camera does with a manual move while it tracks (the widget sends only the move).
- CAM-35: the name the camera reports for a preset saved without one (the widget's "(unnamed)" is shown only for an empty name).
- CAM-22 and CAM-2: the `asleep` behaviour of each control (the widget follows the description's `asleep`; today most are `unknown` and stay usable).
- CAM-5: per-leaf `settle_ms` (the widget runs no timers; it shows `pending` as long as the daemon lists it).
- CAM-6: whether `led.blink` and `hand.move_gimbal` survive sleep (the widget shows the state's value or "not known").
- U-6 (CAM-23): `hand.return_after`, whose slider stays hidden until the description has a maximum (product question 1).
- CAM-28: the upgrade rehearsal from the 0.13 desktop state to tool 1.0 and widget 1.0 (M2).

## Self-review

- **Spec coverage.** Every item of spec 10.3 track D and of 12.3 maps to a task (table above). Section 2's rule: card rule 2, Task 5's tests and Task 7. 4.1 rule 4: Tasks 4 and 5, decision 12. Rule 7: Task 3 (`unknown error code`, `unknown event type`, `unknown fields`), Task 4 (`unknown values`, `unknown types`). Reconnect and `hello`: Task 3. 5.10 messages: Task 4 `problemText`. 8.4 steps 3 and 7: Tasks 4, 5, 9, 15. 8.6 markers: Tasks 4, 9. 10.6: Task 1, R0, Task 16 (after the candidate), R1 (after the tool's tag). Spec deviations named: decision 1 (branch), decision 2 ("0.28.0, the last 0.x" is stale).
- **Placeholder scan.** No "TBD" or "handle errors"; every module, test, script, workflow and the four QML components of Task 6 are complete. Tasks 8–15 give exact member code where the cards depend on it (Task 8's interface and members, Task 11's arrow function, Task 13's reset list, Task 16's startup check) and a table per control; the QML layout around them is the 0.29 file being edited. `#N` stands for each task's own issue number, which the build workflow creates.
- **Type consistency.** `CameraLink` member names are the same in Task 6 (Produces), Tasks 8–15 (Consumes) and the code; the camera state is `camState` everywhere (not `state`, which `Item` owns); `ctl.link` is an alias of `cameraLink` in both roots; `Link.set/setPatch/act` return effects that `CameraLink.run` and `NodeLink.run` both handle; `Session` effect names are the same in `session.mjs`, `link.mjs`, `CameraLink.qml` and `node-link.mjs`; `presetRows` rows (`slot`, `used`, `name`, `label`, `pose`) are used with those names in Tasks 12 and 15 (a rename field reads `name`, a title reads `label`); `anyUsable` is `lib/devices.mjs` (Task 4) → `CameraLink.anyUsable` (Task 6) → `applyVisibility()` (Task 8); `stepTarget` returns `{pan, tilt} | null` in Part A, Task 4, its tests, Task 7's e2e test and Task 11's `step()`; the root has no `awake` (part C), and Tasks 8, 9 and 15 map every 0.29 read of it to `live`.
- **Same-wave files.** Wave 3: Task 3 and Task 4 share no file but `CHANGELOG.md`. Wave 5: Tasks 6 and 7 share none but `CHANGELOG.md`. Wave 7: Tasks 9–14 touch disjoint files. Task 15 (wave 8) uses `DevicePicker.qml` (Task 9) and `PresetSlot.qml` (Task 12) and depends on both. `ci.yml` is changed by Tasks 1, 2 and 7 and in R1, never twice in one wave.
- **What a builder sees.** The header holds every name a QML task consumes ("Shared interfaces of the QML tasks": modules, `CameraLink`, bound controls, the `ctl` root with its member code, the card rules); no task refers to another task's steps for code.

## What was proven, and how

- **W0, built.** The W0 plan's code was extracted into a scratch copy of `rebuild` (37471a0) with stage G's real files: `cargo test -p obsbot-proto -p obsbot-device` passed 238 tests; `scripts/regenerate.sh` wrote `descriptions/tiny2lite.json`, `descriptions/fake/fake-a.json`, `descriptions/fake/fake-b.json` and `schema/protocol-2.json`; the fixtures of Task 2 were synced from that tree.
- **Round trip through the W0 types (scratch Rust test, not part of any task).** The hello reply of `--fake fake-a,fake-b,unknown` was composed through `FakeBus`, `Camera::identity`, `Camera::read` and the `obsbot-proto` types; `tests/fixtures/fakes.mjs` holds its facts and states. 18 request lines written by the widget's modules (hello, live, `set` of one leaf on fake-b, `zoom.factor` 1.37, the defaults reset patch on fake-a with manual white balance, an area patch from `Values.changed`, `preset.save` with and without a name on slot 4 of fake-b, a gimbal step on fake-a, `factory_reset` with `confirm`, `view.save`, `gimbal.stop`, and `exposure.time` while automatic) parsed with `Request::from_line` and got the expected answer of `validate_set` / `validate_do` (`ok` for all, `conflict` for the last); 5 fixture messages parsed as `ServerMessage`.
- **Run green:** all module tests (87 after the fix round) in node 26.13.1 and in the Qt 6.12 `qml` tool through `scripts/check-js.sh`, and a node-only test (`structuredClone`) made the `qml` half fail on its own (exit 2); `bash scripts/check-qml.sh` on 47 files (the 32 QML files of `main` at `58d807b`, `CameraLink.qml`, the three bound controls and the 11 modules); `shellcheck scripts/*.sh`; `actionlint` on the three `ci.yml` versions; a mutation of `isOlder` and a broken module each made the gates fail; an alias of a differently named id with a `Connections` on it (Task 8's pattern) in the `qml` tool.
- **Fix round (review of this plan), re-proven in a fresh scratch copy of the widget's `main` at `58d807b` with the fixtures synced from `rebuild` at 3187b84 (`sync-fixtures --check` exit 0):** the extracted code of Tasks 2–7 passed `scripts/check-js.sh` (87 in node and in `qml`), `bash scripts/check-qml.sh` (47 files), `shellcheck` and `node --check` of both e2e files. Each new test failed on the old behaviour: `stepTarget` from a made-up 0/0 base (`expected null, got {"pan":-5,"tilt":0}`), `Link` clearing the pick on a lost connection (the reconnect test), `anyUsable` returning false. The new e2e test "the unit tests' fixtures match the fake daemon's hello" ran against a throw-away mock daemon (node, not committed) that sends `fakes.mjs`'s `HELLO`: pass; with a changed device name or a changed first state value: fail; with a different `firmware`: pass (left out on purpose). The P1 echo server's reply line was measured at 262168 characters. Task 8's member code with the `CameraLink` block (`onAnyUsableChanged`) and `applyVisibility()`, Task 11's `step()`/`poseKnown`/pose line, Task 14's "After (…)" label and Task 15's on-air list expression compiled and linted in stub `Item`s; the P1 probe compiled with Qt 6.12 `qmlcachegen`.
- **Compiled only:** `CameraLink.qml`, `SettingToggle.qml`, `SettingChips.qml`, `SettingSlider.qml`, the P1 probe, Task 16's `StartupCheck.qml`, and Task 8's member code, Task 11's `step()` and Task 13's `controls`/`resetPatch` each wrapped in a stub `Item` (Qt 6.12 `qmlcachegen` and `qmllint`, no warning outside the categories `check-qml.sh` switches off); they cannot run outside the shell. `tests/e2e/*.mjs` passed `node --check` only: there is no B-1 daemon yet.
- **Unproven:** everything inside the shell (P1 answers it for the socket, the environment and the module import; M1 for the rest); the e2e test against a real B-1 daemon (Task 7 runs it); the card rewrites of Tasks 8–15 (not pre-built; specified by member code and tables); GitHub's acceptance of the workflow changes (each task's pull request shows it).

## Execution handoff

Plan complete. The build workflow takes `w1-d-tasks.json` next to this file. Every task but Task 1 has base `v1`, not `main` (decision 1): the workflow must not default to `main`. The coordinator runs R0 and B0 after Task 1 (before Task 2), P1 before Task 6, M1 after wave 8, Task 16 once the tool's release candidate exists, M2 in W4, and R1 on release day.
