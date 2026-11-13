# obsbot4linux 1.0: rebuild design

Status: revision 2 (2026-11-12), approved by Thomas on 2026-11-13. Revision 1 (2026-11-01)
was agreed with Thomas in the brainstorming session of that day. Revision 2 applies the
design decisions of 2026-11-12. They followed a review of revision 1 against the 0.13.2
code, an audit of that code, and a second camera model that may arrive soon. Section 15
lists every decision that changed, one row each, so that each can be overruled on its own;
section 15.1 lists the changes made after the approval.

Words used with a fixed meaning:

- **captured**: the bytes are in a recording of the vendor app talking to the camera
  ("recorded from the vendor app"). Cited as "capture N".
- **measured**: written from Linux and read back on the camera, or its effect observed
  ("measured on the camera"). Cited with a date, a public issue or a camera-session item.
- **backed**: a setting whose write and readback, or an action whose frames and effect, are
  captured or measured on that model. A standard UVC control counts as measured once V4L2
  reads back the written value on the camera. "0.13 uses it" is not a level.
- **verified / unverified / unknown**: the status of a statement. Unknown means only a
  device or a capture can settle it.
- **must**, **never**: requirements. Every requirement in this document can be checked by a
  test, a review of the diff, or a camera-session item (section 14).
- **Thomas**: the maintainer, who owns the repository. **The maintainers**: Thomas and the
  people he names in the repository.
- **#N**: an issue or pull request of `github.com/hellwand/obsbot4linux`;
  `obsbot4linux-dux#N`: one of the widget's repository.

## 1. Goal

obsbot4linux 1.0 replaces the 0.13 daemon, protocol, state model and CLI with a clean
structure and no legacy baggage. The camera knowledge earned so far is carried over: frame
codec, command bytes, status decoding, recordings and byte-exact tests. The daemon, the
protocol, the client library and the CLI are written new.

Success criteria (the release gate is section 10.6):

1. Every command and option of 0.13.2 has a row in the parity table (section 9.1). Each row
   is verified on a real Tiny 2 Lite, or is a named parity exception with its reason
   (section 9.2).
2. No setting or action is offered on a guess. A released description holds only backed
   fields. Section 9.3 lists the Tiny 2 Lite fields that are not backed today and the fate
   of each.
3. The CLI, the widget and the desktop app speak one versioned protocol, protocol 2: typed
   envelopes, a generic path → value payload, a JSON schema and one description file per
   model.
4. Several cameras at once, hotplug, and a device identity that survives re-plugging.
5. **Guarantee A.** A new model whose settings and actions fit the existing setting types,
   units and constraint forms (section 4.2) needs no code change in `obsbot-proto` (types
   and client library) or in the CLI. Its description file and the completions are
   regenerated as data in the model's PR (section 6.2). A model that needs a new type, unit
   or constraint form first brings that as an additive contract change (a minor version,
   section 12.5); older clients show such a setting read-only as text (4.1 rule 7).
   Hand-made UIs (widget, desktop app) gate every control on the device description and
   take values, bounds, step, unit and list limits from it, never from constants of their
   own. So they keep working for the settings they know and need an update only to show
   settings that are new to them. 1.0 has no generated UI.
6. A camera with a known OBSBOT vendor id that no model covers is listed as unsupported and
   receives nothing (section 8.4).
7. CI tests the daemon, the client and the CLI without a camera, with two fake models of
   different shape and one unknown device running together (section 5.12).

Out of scope for 1.0: a second camera model (M4), the device lease for the firmware updater
(M5), network cameras, the desktop app (own repository, own spec), a generated UI, and any
command catalogue. Section 13 lists them.

## 2. Principles

- **Clean over compatible.** The user base is very small. Breaking changes come with a
  transition (release notes, minimum versions, migration notes), not by carrying old
  structure. There is no protocol-1 adapter.
- **The camera is the truth.** Byte-exact frame tests and recordings are kept: they encode
  what the camera understands, not our design.
- **The daemon owns the device.** No client touches the camera.
- **One contract.** The socket protocol, the CLI's `--json` output and the schema describe
  the same objects.
- **Camera features only.** The daemon invents nothing. **A request sends exactly what it
  names: the frames of that setting or action, nothing before them and nothing after them.**
  The daemon never writes a setting the user did not ask for, never switches an automatic
  control or a tracking mode off on its own, and never sends a command sequence as a
  workaround (one known exception, the preset list rewind of 8.8, stays until CAM-19). What
  the camera then does on its own is camera behaviour: the state shows it as the camera
  reports it, and where it is not recorded yet it is a camera item (section 14). Two kinds
  of frames belong to a request and are not additions to it: the frames the camera's
  protocol defines for the one setting or action asked for (the two bounds of one range, the
  block a start-pose action writes back), and the end the request itself set: a
  `gimbal.turn` without `keep:true` lasts as long as its connection, so its stop is sent
  when that connection closes, a daemon shutdown included (5.6, 5.9). How the model encodes
  a value in its frame so that it keeps the units of 4.2 (the sign of the tilt, the yaw sign
  of a turn, 4.4) is part of that frame. Reads of the camera's state are not commands.
  Clients follow the same rule: the CLI and the widget never chain a second request to the
  one the user made. Reporting camera state (on-air probe, target flag) is fine.
- **Nothing around the camera.** The daemon keeps no file: no state, no names, no keys, no
  remembered settings, no migration. What the camera cannot do or report is not there.
- **Evidence first.** Only backed fields enter a released description (section 8.5).
- **No probing.** A capability is never discovered by trying ids. A model sends only the
  frames in its send table (section 8.3). A device that no model covers receives nothing. A
  blind query scan once broke the camera's query channel until a replug (measured).
- **Unknown until captures.** Nothing about a model without a unit in hand is stated as a
  device fact. This holds for the Tiny 3 and the Tiny 3 Lite (section 8.9).

## 3. Architecture

Cargo workspace, one repository, one release, four crates:

```
crates/obsbot-proto    protocol 2: envelopes, error codes, description types (Device,
                       Setting, Action), validator, path helpers, schema generation, and
                       the synchronous client. Depends on serde, serde_json, schemars.
crates/obsbot-device   transport, family, models/{tiny2lite, fake}, registry,
                       enumeration. Depends on obsbot-proto for the description types.
crates/obsbot4linuxd   daemon binary; also a library with run(Config) for tests.
crates/obsbot4linux    CLI binary, a pure client.
descriptions/          <model>.json: the description of each real model (generated);
descriptions/fake/     <model>.json: the descriptions of the fake models (generated)
schema/protocol-2.json generated from obsbot-proto
packaging/             arch, deb, rpm, systemd units, completions
docs/                  protocol-2.md; field-ledger.md (Tiny 2 Lite);
                       field-ledger-<model>.md for each further model
scripts/check-model.sh live check of one device against its description (section 9.4)
scripts/regenerate.sh  regenerates schema/, descriptions/ and packaging/completions/
```

Dependency direction: proto ← device ← daemon; proto ← CLI. The `[dependencies]` of the CLI
crate never contain `obsbot-device` or `obsbot4linuxd`; a CI step checks this with
`cargo metadata`. Its `[dev-dependencies]` may contain `obsbot4linuxd`, so that tests run a
fake daemon in-process. All crates take `version`, `rust-version` and
`license = "GPL-3.0-or-later"` from `[workspace.package]`.

The firmware updater is a separate, private project. Whether it joins the workspace is
decided with M5.

### 3.1 Layers inside obsbot-device

1. **transport**: `trait Transport`: extension-unit get, set and length by selector, UVC
   control ioctls, and the descriptor facts read from sysfs (the USB serial string among
   them, for a model whose identity source it is). One implementation talks to USB
   UVC devices. A replay implementation answers from recorded exchanges for tests.
2. **family**: shared building blocks, not a driver: the frame codec (framing, CRC, sequence
   numbers), answer matching, short-command write, status-block read, UVC helpers, the on-air
   probe, and encoders and decoders that are byte-identical across models. A model uses a
   shared encoder only where that model's own evidence covers it.
3. **models/<key>**: one module per model. It implements `trait Camera` (section 8.2). A
   model is code plus const tables (ids, offsets, enum tables, ranges) plus its own flows.
   `models/fake` holds the fake models (section 5.12).

`registry` maps USB (vendor id, product id) pairs to model keys (section 8.1).
`enumeration` reads sysfs and returns the USB facts of every video device (section 5.3).

### 3.2 Carry-over (0.13.2 → 1.0)

| 0.13.2                                       | 1.0                                               | Decision                                        |
| -------------------------------------------- | ------------------------------------------------- | ----------------------------------------------- |
| `frame.rs` (frame, CRC, answer parser, seq.) | `obsbot-device/family`                            | carry over; the clock-seeded sequence stays     |
| device access in `device.rs`, `camera.rs`    | `obsbot-device/transport`                         | carry over, regroup; the extension unit by GUID |
| `commands.rs` byte builders, setting enums   | `models/tiny2lite`; `family` where byte-identical | carry over                                      |
| `status.rs`, `presets.rs` parsers            | `models/tiny2lite`                                | carry over                                      |
| `streaming.rs` (on-air probe)                | `obsbot-device/family`                            | carry over                                      |
| `models.rs` (detection by name strings)      | `obsbot-device/registry` and `enumeration`        | rewrite: USB ids from sysfs                     |
| `camera.rs` sequencing (Tiny 2 Lite flows)   | `models/tiny2lite` behind `trait Camera`          | carry over the logic, rewrite the shape         |
| `camera.rs` `current_pose()`                 | –                                                 | not carried: it reads a remembered or V4L2 pose |
| `daemon.rs`, `proto.rs`, `client.rs`         | `obsbot4linuxd`, `obsbot-proto`                   | rewrite                                         |
| `cli.rs`                                     | `obsbot4linux`                                    | rewrite                                         |
| `state.rs` (remembered values)               | –                                                 | drop: every value is read from the camera       |
| byte-exact tests, recordings                 | `obsbot-device` tests, fixtures (section 8.5)     | carry over                                      |

Carrying the logic excludes what section 2 forbids: the carried gimbal move and rate flows
lose their switch to the static tracking mode (15.1 row 88).

Carried code comes over with the fixes the audit of 0.13.2 asked for. Fixes that need no
camera land on `main` first, in the 0.13 layout, so that the groundwork move carries them
(section 10.2, stage G). Fixes whose acceptance needs the camera are camera-session items
(section 14). Track A closes the list: each audit defect in carried code is fixed in
`obsbot-device` or named in section 14. No known defect is copied into 1.0. The defects
that need no camera:

- the answer poll treats every read error as "not ready", so an unplug shows as "no
  answer" after 1 s instead of failing at once;
- the image setter writes the white-balance temperature and the absolute focus while their
  automatic control is on (1.0: `requires`);
- the on-air state and its applications are read at two moments (1.0: one sample);
- a short preset list counts as a successful read;
- "the current pose" is a remembered or V4L2 pose (1.0: the live pose);
- `preset save` sends the pose before it validates the name (1.0: 4.1 rule 5).

The camera-gated ones are U-8, U-9, U-18, CAM-18, CAM-19 and CAM-20.

### 3.3 Licence

Every crate is `GPL-3.0-or-later`. Files that carry an EUPL-1.2 Article 5 note keep it, and
the note follows their code: every 1.0 file that receives code from such a file carries the
note too, including model modules copied from another model.

## 4. Protocol 2

`docs/protocol-2.md` is the client reference: per-message schemas, a worked sequence with
`pending`, lifecycle diagrams and a multi-device transcript. Its normative part is written
in stage W0 from this section; stage W2 adds the worked examples. The evidence for each
field of a model is in that model's ledger (`docs/field-ledger.md` for the Tiny 2 Lite).

### 4.1 Messages

JSON lines over the Unix socket `$XDG_RUNTIME_DIR/obsbot4linux.sock` (mode 0600; the
environment variable `OBSBOT4LINUX_SOCKET` overrides the path). One JSON object per line,
UTF-8. A request line longer than 64 KiB closes the connection. Lines from the daemon have no
length limit, and clients must not cap them. The daemon accepts at most 64 connections; a
further connection is closed at once.

**Requests** carry `id` (integer or string, echoed verbatim) and exactly one verb:

| Verb    | Shape                                                                                 | Reply `ok`                                                                                     |
| ------- | ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `hello` | `{"id","hello":{"protocol":2,"client":"<name version>","subscribe":bool,"live":[…]}}` | `{"protocol":2,"daemon":"<version>","devices":[<Device>…],"states":{"<id>":{"rev","state"}…}}` |
| `get`   | `{"id","device","get":["<path>"…]\|null}`                                             | `{"rev","state"}`: the full state, or only the paths asked for                                 |
| `set`   | `{"id","device","set":<patch>}`                                                       | `{"rev"}`                                                                                      |
| `do`    | `{"id","device","do":"<action>","with":{…}}`                                          | `{"rev","result"?}`                                                                            |
| `live`  | `{"id","live":["<topic>"…]}`                                                          | `{}`; replaces the connection's live list                                                      |

`subscribe` defaults to `true` and `live` to `[]`. The first message on a connection must be
`hello`. Requests that follow `hello` before its reply are kept and handled after the reply.
A second `hello` is answered `invalid`. A patch is an object of leaf paths; it may
nest them (`{"zone":{"area":{"pan":…}}}`) or use dotted keys (`{"zone.area.pan":…}`).

**Replies:** `{"id":<id>,"ok":{…}}` or `{"id":<id>,"error":{…}}`.

**Events** (only on connections with `subscribe:true`):

- `{"event":"state","device","rev","state"}`: the full state of one device.
- `{"event":"devices","devices":[<Device>…]}`: the full device list. It is sent whenever an
  entry is added, removed or changed (availability, identity hand-over, firmware, video
  node).

When an entry is added (a supported or an unsupported device), the daemon sends the
`devices` event, then one `state` event for that entry. On unplug of a device with an `sn:`
id the daemon sends the `devices` event first, then one `state` event that carries
`available:false` and the last known values; no further state events follow until the
device is back. On re-plug the order is the same: `devices`, then `state`. A `port:` entry
is removed on unplug: the `devices` event no longer lists it, and no state event follows.
This removal is always its own `devices` event, sent before any event that adds an entry
under the same id, also when one scan sees a device-number change (5.3). The events of an
identity hand-over are in section 5.5.

**Errors:** `{"id","error":{"code","message","rev"?,"applied"?,"failed"?,"supported"?}}`.
`message` is always present (English, for logs). `rev` is present whenever the daemon
re-read the device before answering.

| Code                 | When                                                                                                                                            | Extra fields                                |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `protocol`           | the first message is not `hello`, or its protocol is not supported (the connection is closed; `id:null` if there was none)                      | `supported:[2]`                             |
| `invalid`            | malformed request, no verb or two verbs, wrong value type, a read-only path, a second `hello`, a destructive action without `confirm:true`      | –                                           |
| `unknown_device`     | no device with this id                                                                                                                          | –                                           |
| `unsupported`        | well-formed, but the path or action is not in this device's description (a typo included)                                                       | –                                           |
| `unsupported_device` | a `set` or `do` for a device with `support: unsupported` (section 5.4)                                                                          | –                                           |
| `out_of_range`       | the value lies outside the description (bounds, `writable`, string length)                                                                      | –                                           |
| `conflict`           | valid, but the device's current state does not allow it: a `requires` condition not met, an empty preset slot, an unknown view id, no free view | –                                           |
| `unavailable`        | a `set` or `do` for a device that is unplugged                                                                                                  | –                                           |
| `asleep`             | the camera is asleep and the description marks the setting or action `asleep: ignored`                                                          | –                                           |
| `busy`               | 32 requests are already queued for the device (the lease of M5 will use it too)                                                                 | –                                           |
| `camera`             | the camera refused or did not answer within the transport timeouts (section 5.8), the device node stayed locked, or the 15 s deadline passed    | `rev`, `applied:[paths]`, `failed:"<path>"` |
| `internal`           | a daemon bug                                                                                                                                    | –                                           |

**Ordering, `rev` and `pending` (normative):**

1. `rev` is per device entry. It is 1 for the first state of a new entry, only goes up while
   the entry exists, and restarts with the daemon. An `sn:` entry exists until the daemon
   ends. A `port:` entry ends when it is removed; an entry added later under the same
   `port:` id is a new entry and starts at 1 again. Clients forget the rev of an id when a
   `devices` event no longer lists it. Every change of a device's published state (values,
   `pending`, `unread`, `available`) gets a new `rev`. The state of an unsupported entry
   never changes, so its `rev` stays 1 while the entry exists.
2. All requests for a supported, available device go through that device's queue and are
   answered in request order, with these exceptions, which the connection's reader thread
   answers at once: `busy`, `unknown_device`; every request for an unavailable device, and
   every request still queued when its device becomes unavailable (5.6): `get` from its
   published state, `set` and `do` with `unavailable`; every request for an unsupported
   device (`get` from its fixed state, `set` and `do` with
   `unsupported_device`). A stop (an action the model marks as a stop; on the Tiny 2 Lite
   `gimbal.stop` and `zoom.stop`) is taken before every queued request, is accepted even
   when 32 requests are already queued (it is never answered `busy`), and is answered when
   the camera accepted it. Requests for different devices may be answered in any order.
   Requests may be pipelined.
3. For a `set` or `do` that changes state, the daemon sends the state event to every
   subscribed connection that receives it (rule 8), the requester included, **before** the
   reply. The reply's `rev` is the rev of the first state that reflects the request: that
   state shows the written value, or lists the path in `pending`. Several requests may share
   one state and one rev. If nothing changed, the reply carries the current rev.
4. `pending` is part of every state: a map from leaf path to the value written. A path is in
   `pending` while the camera accepted a write but its readback does not show the written
   value yet. It leaves `pending` when a read of its group shows the written value (after
   rounding to the setting's precision), or when the setting's `settle_ms` has passed at a
   read of its group; then the camera's value stands, whatever it is. A new write of the
   same path replaces its pending value and restarts its bound. `pending` is emptied when
   the device becomes unavailable. Clients show their own value from sending a request
   until its reply arrives, and the state (its `pending` values included) after that. They
   run no timers of their own.
5. `set`: the whole patch is validated first, against the description and against the
   current state with the patch applied. A value in `pending` counts as the current value
   there, for `requires` and for the asleep check. If any field fails, nothing is sent.
   The worker then applies the leaves in the order the model defines. On a camera failure midway it
   stops, re-reads, publishes the real state and answers `camera` with `applied` and
   `failed`. There is no rollback: the camera has no transactions. The same holds for `do`:
   all parameters are validated before the first frame is sent, and a failure after a frame
   went out is answered the same way.
6. Motion actions (on the Tiny 2 Lite `gimbal.move`, `gimbal.turn`, `zoom.move`,
   `preset.recall`, `view.go`, `zone.go_return`, `hand.go_return`, `power_on.go`) reply
   when the camera accepted the command. The state shows measured values (pose, zoom
   factor), so it moves through later events.
7. Additive changes (fields, actions, models, enum values, live topics, setting types,
   units) keep `protocol: 2`. A breaking change raises the number. Clients ignore unknown
   fields and unknown event types, show unknown enum values as raw text, show settings of an
   unknown type read-only as text, and treat an unknown error code as `internal`.
8. **Live topics.** The description names a live topic for each volatile leaf (section 4.2).
   A state change that touches only volatile leaves, and that no request caused, is sent
   only to connections whose live list holds one of their topics. Every other connection
   receives the new values with its next event. A change caused by a `set` or `do` is sent
   to every subscribed connection. A topic name no device uses has no effect.
9. **Queue handling.**
   - The worker takes the requests queued for its device as one batch. It runs them in
     order, then re-reads once the union of the groups they touch (section 8.2), publishes
     one state, and answers them in order. A camera failure ends the batch; the requests
     after it stay queued, unless the device became unavailable (5.6).
   - A `get` never reads the camera. It is answered, in its turn, from the published state:
     the paths asked for, plus `available`, `pending` and `unread` in full.
   - A `set` whose patch has exactly one leaf is answered `ok` with the current rev, without
     being sent, when a later queued `set` from the same connection has the same single leaf.
   - A request whose connection closed before the worker started it is dropped. A started
     request runs to the end.
   - Every device request (`get`, `set`, `do`) is answered within 15 s of its arrival
     (`REQUEST_DEADLINE`). A request not started by then is dropped and answered `camera`.
     A started request checks the deadline before each frame and, once it has passed,
     stops as on a camera failure.

**Reconnect (client behaviour):** reconnect with a backoff from 0.5 s, doubling up to 5 s.
Socket activation starts the daemon. Requests die with their connection. After any
reconnect a client drops every pending request and every own value, sends `hello` and
rebuilds from its reply. Revs from two connections are never compared.

**`hello` timing:** the daemon answers `hello` after its first enumeration and the first
read attempt of every supported device that enumeration found (section 5.2), and at the
latest 15 s (`REQUEST_DEADLINE`) after the `hello` arrived. A device whose first read
attempt has not ended by then is in the reply with its leaves `null` and listed in
`unread`; its first read publishes a new state later. The reply and the start of event
delivery are atomic: no state published after the reply's snapshot is missing from the
events that follow.

### 4.2 Devices and descriptions

```
Device = { id, name, model, support, serial, firmware, video_node, available,
           replaced_by?, usb, settings{<leaf path> → Setting},
           actions{<name> → Action}, limits{<list path> → max items} }

usb = { vendor, product, product_name, bcd_device, path, serial_present,
        interfaces[<class>…], extension_units[{unit, guid, controls}…] }

Setting = { type, label, readonly, nullable, readback, asleep, no_effect,
            values[], writable[]          (enum)
            min, max, step, unit, precision  (number, range, pose, point, box)
            max_len, ascii                (string)
            item{<field> → Setting}       (list; read-only)
            optional                      (action parameters only)
            requires[{path, equals}…], default, live, settle_ms, warning }

Action = { label, params{<name> → Setting}, result{<name> → Setting}?, asleep,
           requires[…]?, warning? }
```

Device fields:

- `id`: `sn:<serial>` when the model's identity source gave a valid serial, else
  `port:<usb path>` (section 5.5). A `port:` id is never stored.
- `name`: the model's display name plus the last four characters of the serial
  (`OBSBOT Tiny 2 Lite 0001`); without a serial, the display name plus the USB path; for an
  unsupported device, the USB product name plus the USB path, or `USB <vendor>:<product>`
  plus the USB path when the device has no product string.
- `model`: the model key (`tiny2lite`, `fake-a`, `fake-b`); `unknown` for an unsupported
  device.
- `support`: `tested`, `untested` or `unsupported` (section 8.6). It is device-level only.
  Per-field evidence lives in the model's ledger, not in the protocol.
- `serial`, `firmware`: as the camera reported them, or `null`.
- `video_node`: `/dev/videoN` of the device's video node with sysfs index 0, or `null`. A
  USB device with several video nodes is one entry.
- `usb`: read from sysfs at enumeration, never from the camera. `serial_present` says
  whether the USB descriptor has a serial string; the string itself is never in `usb`.
  `devices --report` prints this object (section 6.2). Formats, as sysfs gives them:
  - `vendor`, `product`, `bcd_device`: strings of four lower-case hex digits (`"3564"`).
  - `product_name`: the product string, or `null` without one.
  - `path`: the USB device name in sysfs (bus, then the port chain: `"3-2.1"`). It stays the
    same across re-plugs into the same port and across daemon restarts, and changes when the
    device is plugged into another port. The `port:` id uses it.
  - `interfaces`: the interface class of each interface, as two lower-case hex digits
    (`"0e"` video, `"01"` audio), in interface order.
  - `extension_units`: per UVC extension unit, `unit` (integer id), `guid` (lower-case,
    `8-4-4-4-12` form) and `controls` (integer, the number of controls the descriptor
    declares).
- `limits`: the maximum item count of a list setting, where the camera has one
  (`{"presets":3}`). A list without an entry has no known maximum.

Setting and Action fields:

- `type`: `bool`, `enum`, `number`, `range` (`{min,max}`), `pose` (`{pan,tilt,zoom}`),
  `point` (`{x,y}`), `box` (`{left,top,right,bottom}`), `string`, `list`. Shapes are always
  objects. For `range`, `pose`, `point` and `box`, `min` and `max` are objects of the same
  shape and bound each component.
- `label`: English, for CLI output.
- `values[]` are all values a state may show; `writable[]` the ones a `set` accepts.
- `unit` is one of `deg`, `deg/s`, `factor`, `us`, `K`, `percent`, `s`, `fraction`, `fps`,
  or absent for a raw camera value. New units are additive.
- `readback`: `false` means the camera accepts the setting but cannot report it.
- `asleep`: `accepted`, `ignored` or `unknown` (section 5.7).
- `no_effect`: the camera accepts and reports the value, and it has no observable effect on
  this model.
- `requires`: conditions on other paths. The check uses the current state with the patch
  applied (a `pending` value counts as current, 4.1 rule 5), so
  `{"exposure":{"mode":"manual","time":20000}}` is accepted while the exposure is
  automatic, and `exposure.time` alone is refused with `conflict`.
- `default`: the camera's own default as it reports it (UVC). Absent when the camera
  reports none or when the reported value lies outside the setting's range.
- `live`: the live topic of a volatile leaf: a value that changes without a request (pose,
  target box, values the camera adjusts itself).
- `settle_ms`: the longest time the camera's readback may lag a write of this leaf
  (rule 4). It is a measured property of the model.
- `warning`: a list of one or both of `restart` (the camera restarts after this write or
  action) and `destructive` (data on the camera is lost). An action whose `warning` holds
  `destructive` requires the parameter `confirm: true`; without it the daemon answers
  `invalid`. Interactive UIs, such as the widget, ask before they send either. The command
  line does not prompt: scripts use it, and a destructive action already needs `confirm=true`.

Rules:

- **Leaves.** Settings are described per leaf path (`zone.area.pan`, `led.brightness`). Only
  leaves are writable.
- **One meaning per path.** A path means the same function, unit and frame on every model.
  Models differ in which paths they have and in `values[]`, `writable[]` and bounds. A new
  function gets a new path or a new enum value.
- **Path dictionary.** `docs/protocol-2.md` lists every path any model uses, with its
  function, type, unit and frame (the units and frames rule below). A model PR that adds a
  path or an enum value adds its line there; the review checks that a reused path has the
  meaning of its line. Naming: lower-case `snake_case` segments joined by `.`, the camera
  function first (`zoom`, `tracking`, `exposure`), then the leaf; no model name in a path.
- **Absent vs null.** A field the model does not have is absent from state and
  description. `null` means one of: the camera reports "not set" (only where the description
  says `nullable`); the leaf was never read (it is in `unread`); or a `readback:false` leaf
  that was not written since the device last became available.
- **`unread`** is part of every state: the leaf paths that were never read since the device
  last became available (`null`; a group not read while asleep included), and those whose
  last read failed or was cut short (they keep their last value). Their group is read again
  at its next tier tick while the camera is awake; at a wake, and for a `touched` group, 5.7
  ("Wake and retries") says when.
- **Undecodable values.** A camera value the model cannot decode is published as the
  string `raw:<n>` (n = the raw number) and logged once per device and value. It is never
  turned into `null` or into a neighbouring value.
- **`readback:false` lifetime.** The state shows the value written since the device last
  became available, `null` before the first write, and `null` again after unplug,
  re-enumeration and `factory_reset`. Nothing is re-sent. Across sleep the value is kept
  until camera item CAM-6 shows otherwise.
- **Units and frames.** Angles in degrees; pan positive = the camera turns to its left; tilt
  positive = up; 0/0 = straight ahead. Picture coordinates 0..1 from the top-left corner.
  Zoom as a factor (1.0 = widest). Gimbal speeds in °/s. Durations in seconds (precision
  0.1). Exposure time in microseconds. Colour temperature in K. Percentages 0–100.
- **Precision.** Each number has a precision in its description (angles 0.1, zoom 0.01,
  fractions 0.01). Values are rounded to it where they are created (decode and echo), so
  state and `--json` never show binary float noise.
- **Identity and serial exposure.** The full id and serial travel on the socket and in the
  CLI's `--json` and `watch` output. Human CLI output, logs and the journal show `…` plus the
  last four characters (`sn:EXAMPLE0000001` → `…0001`), through one formatter. No log line
  prints an identity answer.
- **Lists.** Items of `presets` and `views` carry the camera's own numbers: preset slots
  start at 0, view ids are the camera's ids.

### 4.3 State model of the Tiny 2 Lite

This is the reference model. `descriptions/tiny2lite.json` is its normative form,
generated from the model module. Another model has other paths and values under the rules
of 4.2. ✎ = writable leaf. Ranges, units and asleep behaviour are in the description; the
evidence per field is in `docs/field-ledger.md`. Fields marked U-n are not backed today;
section 9.3 gives their fate.

```
available                                        daemon: enumeration only
pending { <path>: <value> }, unread [<path>…]    daemon
asleep                                           read-only
on_air     { active, apps[] }                    daemon: the on-air probe, read-only
gimbal     { pan, tilt, invert✎ }                pan, tilt: live topic "pose"
zoom       { factor✎, field_of_view✎ }
tracking   { mode✎, profile✎, speed✎, target { detected, box } }
framing    { auto_zoom✎, size✎, zoom_speed✎, offset { x✎, y✎ } (U-4), part✎ }
gestures   { target_lock✎, zoom✎, zoom_ratio✎, dynamic_zoom✎, direction_change✎ }
zone       { enabled✎, follow✎, area { pan✎, tilt✎ }, return }
hand       { side✎, area { pan✎, tilt✎ }, axes { pan✎, tilt✎ } (U-5),
             return_after✎ (U-6), move_gimbal✎, return }
presets    [ { slot, name, pose } ]
views      [ { id, name, pose } ]
exposure   { mode✎, region✎, time✎, gain✎, compensation✎ (U-20), hdr✎,
             anti_flicker✎ (U-25: the value auto) }
image      { brightness✎, contrast✎, saturation✎, hue✎, sharpness✎,
             white_balance { auto✎, temperature✎ }, mirror✎, frame_rate }
focus      { mode✎, region✎, position✎ }
audio      { enabled✎ (U-23), volume✎, noise_reduction✎, auto_gain✎, in_sleep✎ }
led        { brightness✎, blink✎ }
sleep      { auto { enabled✎, seconds✎ }, background✎, background_mirror✎,
             wake_zoom✎ }
power_on   { mode✎, start { source (U-26), pose, mode✎ (U-18) }, settings { … } (U-19) }
```

Per group:

- `on_air`: `active` and `apps` are taken together in one sample. `active` is true for any
  reader of the video node, a client's own preview included; a client that wants "another
  application" accounts for its own stream. `apps` holds process names, best effort. On
  kernels up to 6.16 the probe briefly holds the stream's exclusive privilege, so another
  application that starts its video at that moment can fail with "device busy"; 6.17 and
  newer are not affected.
- `gimbal.invert` reverses the gimbal's pan direction.
- `zoom.factor`: 1.0–4.0. It follows `zoom.field_of_view`, the zoom
  gestures and `preset.recall`.
- `zoom.field_of_view`: values `wide|medium|narrow|custom`, writable `wide|medium|narrow`.
- `tracking.mode`: `static|normal|upper_body|close_up|headless|lower_body|hand|group`. A mode
  change rewrites `framing.*`: `upper_body` and `close_up` set `framing.auto_zoom` and
  `framing.size`; `headless` and `lower_body` set `framing.part`.
- `tracking.profile`: `standard|headroom|sport`. The vendor app labels this "tracking
  speed".
- `tracking.speed`: `lazy|slow|normal|fast|very_fast|auto`.
- `tracking.profile=headroom`, `tracking.speed` and `framing.*` read back as written
  (measured, #98); their effect on the tracking is not observed yet (CAM-33 decides
  `no_effect`).
- `tracking.target`: `detected` (bool) and `box` (`{left,top,right,bottom}` as picture
  fractions, `null` without a target; layout measured, #71). `box` has the live topic
  `target`.
- `framing`: `size` `long_shot|medium_long_shot|medium_shot|medium_close_up|close_up|big_close_up`;
  `zoom_speed` `slow|medium|fast`; `offset.x` and `offset.y` −0.16..0.16 (fraction);
  `part` `normal|head_removal|lower_body`.
- `gestures.zoom_ratio`: 1.0–4.0, step and precision 0.1 (the readback has one decimal).
  `gestures.direction_change` reverses the direction of the dynamic zoom gesture.
- `zone.area.pan` −150..150, `zone.area.tilt` −70..90 (degrees, ranges). `zone.return` is a
  pose and never `null`: the camera has no "not set" for it.
- `hand.side`: `left|right`. `hand.area` defaults to pan −45..45, tilt −30..30.
  `hand.return_after` in seconds. `hand.move_gimbal`: `readback: false`; the camera does not
  store it, and `false` stops the hand tracker's gimbal moves. `hand.return` is a pose or
  `null` (nullable: the camera reports "not set").
- `presets`: at most 3 (`limits.presets`), slots 0–2. `views`: no known maximum. Names are
  at most 32 printable ASCII characters. A preset's `name` is what the camera reports, and
  may be empty (4.4, CAM-35).
- `exposure.mode`: `auto|manual`. `exposure.region`: `face|global`, the metering while the
  exposure is automatic. `exposure.time` in µs; it `requires` `exposure.mode=manual` (a time
  written while automatic freezes the automatic exposure); the camera rounds down to its own
  steps, and the state shows the value it took. `exposure.gain` 1–64. `exposure.time` and
  `exposure.gain` have the live topic `auto`. `exposure.compensation`: the camera's raw
  value 0–18, default 9 (U-20). `exposure.anti_flicker`: values `off|50hz|60hz|auto`,
  writable `off|50hz|60hz` (the kernel driver offers no way to set `auto` on this camera),
  no `default`. The value `auto` was never read from the camera (U-25).
- `image`: brightness, contrast, saturation, hue and sharpness are UVC controls with the
  ranges and defaults the camera reports (constants of the model). The camera has no gamma
  control. `white_balance.temperature` `requires` `white_balance.auto=false` and has the live
  topic `auto`. `frame_rate` is read-only (fps).
- `focus.mode`: `continuous|single|manual`. `focus.region`: `face|global`.
  `focus.position` 0–100, `requires` `focus.mode=manual`, `no_effect` on this model (#99).
- `audio.enabled`: `warning: [restart]`; the camera restarts on every write, both values;
  the restart lasts about 18 s (measured; how long until the device is enumerated again and
  answers is timed in CAM-1). The write is answered once the camera accepted it, and the
  restart is ordinary hotplug (section 5.6). Its readback is the presence of the camera's
  USB audio interface in sysfs (U-23). `audio.volume` 0–100. `audio.noise_reduction`:
  `off|weak|medium|strong`.
- `led.brightness` 0–3. `led.blink`: `readback: false`.
- `sleep.auto.enabled` and `sleep.auto.seconds` are one word on the camera. The camera keeps
  the seconds while auto-sleep is off; writing `seconds` alone does not switch it on. Range
  1–32767 s: the camera cannot store "off" together with 0 s, so 0 is not offered.
  `sleep.background`: values `black|image|own_image`, writable `black|image` (`own_image`
  shows after an upload, which 1.0 does not offer). `sleep.wake_zoom`: 1.0–4.0, `no_effect`
  (#102).
- `power_on.mode`: the tracking mode the camera starts in at power-on (0.13 `wake mode`), all
  eight modes (measured, #102).
- `power_on.start`: the camera's initial state (0.13 `startup`). `source` is `own` or
  `preset_<slot>` (U-26), read-only. `pose` is read-only and set through actions. `mode` is
  the tracking mode of the initial state: values `static|normal|upper_body|close_up|headless|`
  `lower_body|hand|group|unset`, writable `static|normal|upper_body|headless|group|unset`
  (all but `normal` and `unset` are U-18). `unset` is the camera's "not set" and differs
  from `static`.
- `power_on.settings`: read-only leaves of the same block (`tracking_speed`, `autofocus`,
  `auto_exposure`, `exposure_compensation`, `white_balance_mode`, `white_balance_temperature`,
  `contrast`, `saturation`, `sharpness`, `hue`) as raw camera values; `null` where the block
  says "not set" (U-19). Changing the pose or the mode rewrites the block and keeps these
  values.

Live topics of the Tiny 2 Lite: `pose` (`gimbal.pan`, `gimbal.tilt`), `target`
(`tracking.target.box`), `auto` (`exposure.time`, `exposure.gain`,
`image.white_balance.temperature`).

Removed from the 0.13.2 output (no settable function is lost; section 9):

- `capabilities` and `limits`: the description replaces them.
- `status` (the raw camera block) and `state` (the daemon's remembered values).
- `model_info`, `digital_zoom_percent`, `sleep_images`, and the raw V4L2 fields.
- `device` (the device path): now `video_node`. `streaming`: now `on_air`.

Merged duplicates: `focus.auto` (UVC continuous autofocus) is the same camera bit as
`focus.mode=continuous`. `audio.pickup` is `audio.volume` quantised (below 50 near, 50–84
standard, from 85 far); UIs may offer those three as volume presets.

Not offered in 1.0: flipping the image, resetting the start pose, resetting the zone area.
No recording of any of them exists, and 0.13.2 does not offer them (CAM-11, CAM-12).

### 4.4 Actions of the Tiny 2 Lite

```
gimbal.move {pan?, tilt?, speed? (U-9)}     gimbal.stop
gimbal.turn {pan_speed?, tilt_speed?, keep?}
zoom.move {factor, speed (U-12)}            zoom.stop
preset.save {slot, name?, pose?}            preset.recall {slot}
preset.rename {slot, name}                  preset.delete {slot}
view.save {name, pose?} → {id}              view.go {id}
view.rename {id, name}                      view.delete {id}
zone.return_here                            zone.go_return
hand.return_here                            hand.go_return
hand.return_clear                           hand.area_reset (U-7)
focus.point {x, y} (U-16)
power_on.set_pose {pose}                    power_on.use_current
power_on.use_preset {slot}                  power_on.go
sleep                                       wake
factory_reset {confirm}
```

- `gimbal.move`: at least one of `pan` and `tilt`; both missing is `invalid`. A missing axis
  keeps its position (the camera is told to leave it). `speed` in °/s makes the move take as
  long as that speed needs.
- `gimbal.turn`: at least one of `pan_speed` and `tilt_speed` (°/s, up to 178); a missing
  one is 0. It keeps turning until `gimbal.stop` or speed 0. It stops when the issuing
  connection closes, unless `keep:true`; the CLI sends `keep:true`, so a one-shot call
  keeps turning as `gimbal rate` did in 0.13. The daemon sends no stop of its own beyond
  that (section 2): a `keep:true` turn goes on across a daemon shutdown (5.9), and what a
  turn does when the camera falls asleep is the camera's (CAM-7). `pan_speed` has the
  units of 4.2 whatever `image.mirror` and `gimbal.invert` say: the camera reverses the yaw
  of a turn when exactly one of them is on, and the model encodes the speed so that a
  positive `pan_speed` turns the camera to its left, as a positive `pan` of `gimbal.move`
  does (field ledger, measured).
- `gimbal.move` and `gimbal.turn` send only the move (section 2). They leave `tracking.mode`
  alone, as `zoom.move`, `zoom.stop` and `gimbal.stop` do, and they are not refused while the
  tracking is on. What the camera does with a manual move while it tracks is CAM-34; the
  state after the reply shows what the camera reports. A client that wants the tracking off
  first sends `set tracking.mode=static` itself, as its own request.
- `zoom.move`: `speed` is `slow|medium|fast` (`slow` and `fast` are U-12).
- `preset.save`, `view.save` and `power_on.use_current` without `pose`, and
  `zone.return_here` and `hand.return_here`, take the live pose and the current zoom factor
  from the camera. While the camera sleeps its live pose is the parking pose, which lies
  outside the tilt range, so these actions are `asleep: ignored` until CAM-22 shows
  otherwise.
- `preset.save` without `name` sends the preset and no name: no rename frame follows, and
  obsbot4linux invents no name (0.13.2 named it `Preset <slot + 1>`). What the camera then
  reports as the name of an unnamed preset is CAM-35; the state shows it as reported.
  `view.save` needs a name. Names are validated before the first frame is sent.
- `preset.recall`, `preset.rename`, `preset.delete` and `power_on.use_preset` on an empty
  slot, and `view.*` with an unknown id, are refused with `conflict`. So are these actions
  and `preset.save` while their list (`presets` or `views`) is in `unread`; 5.7 ("Wake and
  retries") reads it again.
- `power_on.set_pose`, `power_on.use_current` and `power_on.use_preset` rewrite the initial
  state and keep its tracking mode and `power_on.settings`.
- `sleep` and `wake` reply when the camera accepted the command. The state shows `asleep`
  as the camera reports it; clients wait for that state, never for their own value.
- `factory_reset`: `warning: [destructive, restart]`, needs `confirm:true`. It replies when
  the camera accepted it (5.6). The restart that follows is ordinary hotplug (section 5.3).
  It deletes the presets.
- Every action states its `asleep` behaviour in the description; section 8.8 lists the
  Tiny 2 Lite's.

Not carried over from 0.13: the hidden `gimbal center` (it moved the gimbal only toward the
left and downward in tests). Revision 1's `gimbal.step`, `image.reset` and `device.rename`
are dropped (section 15).

## 5. Daemon (obsbot4linuxd)

The daemon knows devices, groups, tiers, leaves, actions and live topics. It knows no
command id, no offset and no selector of any model: everything model-specific comes through
`trait Camera` (section 8.2).

### 5.1 Threads

Synchronous threads, no async runtime:

- an acceptor;
- per connection one reader thread, which parses requests, answers the requests of 4.1
  rule 2 that need no worker, and routes the rest, and one writer, which drains the
  connection's outbound queue (64 lines; when it is full, the connection is closed and a log
  line names it by its connection number and the `client` of its `hello`);
- a registry thread, which runs the enumeration (5.3);
- one worker per supported device: the only code that touches that device. The workers of
  several devices run in parallel, their first reads included.

A device table (id, description, rev, last state, pending, unread) sits outside the
workers under one lock. `hello` takes that lock to build its snapshot and to register the
connection for events.

### 5.2 Start-up

1. Take the single-instance lock (5.9). Inherit the socket from systemd, or bind it.
2. Run one enumeration pass (5.3).
3. For every supported device: start its worker. The worker runs the first identity
   attempt (5.5), then reads the group that holds `asleep` (on the Tiny 2 Lite the status
   block). If that read reports the camera awake, or the model has no `asleep` leaf, the
   worker reads every other group once; if it reports asleep, only the groups readable
   while asleep. This is the device's first read attempt.
4. Answer `hello` after the first read attempt of every device of step 3, or at the latest
   15 s after the `hello` arrived (4.1, `hello` timing). A device found later enters by a
   `devices` event after its first read attempt has ended. Leaves that were not read are
   `null` and listed in `unread`.

### 5.3 Enumeration and hotplug

- Once a second the registry thread lists `/sys/class/video4linux`. For each node it
  resolves the USB device the node belongs to and reads, from sysfs only: vendor id, product
  id, product string, whether a serial string exists, bcdDevice, the device number on its
  bus, the interface classes, the extension units from the USB descriptors, and the node
  index. It never opens a video node during the scan. The attributes it reads (for example
  `idVendor`, `idProduct`, `product`, `bcdDevice`, `devnum`, `descriptors`, the
  interfaces' `bInterfaceClass` and the node's `index`) are a choice of track A, confirmed
  on the camera by CAM-25.
- One USB device is one entry, whatever number of video nodes it has.
- An enumeration is identified by the USB path together with the device number. When the
  device number under a path changes between two scans, the daemon handles it as an unplug
  followed by a plug-in, even if no scan saw the path missing.
- A USB device whose vendor id is not in the registry's vendor table (section 8.1) is
  ignored and not listed.
- A device with a known vendor id is matched by (vendor id, product id) against the
  registry. A match becomes a supported device; its worker opens the node with index 0. A
  miss becomes an unsupported device (5.4). The product string is never used for matching:
  the camera can rename itself.
- `available` changes only here: true when a device appears, false when its sysfs entry is
  gone. A failed read never sets `available:false` and never drops the identity; it shows in
  `unread` or as a `camera` error. A read that fails because the node is gone is reported at
  once, without waiting for the answer timeout.
- On unplug an `sn:` entry stays listed with `available:false` while the daemon runs; a
  `port:` entry is removed. The re-plugged device takes its old `sn:` entry once its
  identity is read (5.5).
- A camera that re-enumerates (after `factory_reset`, after an `audio.enabled` write, in
  firmware-update mode) is ordinary hotplug: the old node goes, a new one comes. A device
  without a video node (a camera in firmware-update mode, for example) is not listed and
  receives nothing; a camera that enters that mode counts as unplugged.

### 5.4 Unsupported devices

A device with a known vendor id and an unregistered product id gets an entry with
`support: unsupported`, `model: unknown`, id `port:<usb path>`, `serial: null`,
`firmware: null`, its `usb` facts, its `video_node`, `available`, and empty `settings`,
`actions` and `limits`. Its state is `{"available":true,"pending":{},"unread":[]}` with
`rev` 1, and it never changes: on unplug the entry is removed, and a re-plug adds a new
entry (4.1 rule 1). It has no worker. The daemon
sends it nothing: no identity query, no status read, no on-air probe, no extension-unit
request of any kind. It never opens its video node. The connection's reader thread answers
requests for it at once (4.1 rule 2): `get` from the fixed state, `set` and `do` with
`unsupported_device`. Section 8.4 gives the whole walk-through.

### 5.5 Identity

- The identity source is a property of the model (section 8.2): the USB serial string when
  the model has one (read from sysfs through `Transport`, no request), else the model's
  identity read, else none.
- An identity attempt sends the model's identity query while no valid serial has been
  answered, and, where the model has one, its firmware query while no firmware has been
  answered. A series ends at the first attempt after which both have been answered. Where the identity source is the USB serial
  string, the serial is read from sysfs once at plug-in, and an attempt is the firmware
  query alone; a model with neither query sends no attempt. The cap below counts attempts,
  so it covers both queries.
- **First series.** The first attempt is sent at plug-in, whether the camera is asleep or
  awake. Attempts 2 and 3 run with the next two reads of the tier the model names for them
  (on the Tiny 2 Lite the 5 s tier). That tier is not read while the camera is asleep, so
  these attempts wait for the camera to be awake.
- **Wake series.** If the first attempt gave no valid serial and the first read of the
  group that holds `asleep` (5.2) reported the camera asleep, the first series ends after
  that attempt. One further series of three attempts runs from the camera's first wake: at
  the wake and with the next two reads of the named tier. So no more than one attempt is
  ever sent to a sleeping camera.
- After the last attempt the id stays as it is until unplug. The id can change at most once
  per plug-in.
- A serial is valid only if it matches `[A-Za-z0-9]{8,32}`. Without a valid serial the id is
  `port:<usb path>` for this plug-in, with a log line. The pattern is part of the id format
  of protocol 2: widening it is a contract change, not a property of a model.
- **Duplicates.** The duplicate rule is applied before the hand-over. If an available device
  already holds the same `sn:` id, the newcomer gets a `port:` id for this plug-in and a log
  line. The newcomer is the device whose serial was read later; for two serials read in one
  enumeration pass, the one whose serial reaches the device table later (the workers
  read in parallel, so the order of the scan does not decide).
- Identity values (serial, firmware) are kept only once the camera answered them.
- **Hand-over.** When a `port:` id resolves to `sn:<serial>`, the device takes that id. If an
  unavailable entry with that id exists, the device takes that entry. The daemon sends, one
  directly after the other:
  1. a `devices` event that lists the `sn:` entry (available) and the old `port:` entry with
     `available:false` and `replaced_by:"<sn: id>"`;
  2. a `devices` event without the `port:` entry;
  3. a `state` event of the `sn:` entry.

  The new state's `rev` is one more than the larger of the `port:` entry's rev and the
  taken `sn:` entry's last rev, so a client that kept the `sn:` entry never sees its rev go
  down. `pending` and `unread` carry over from the `port:` entry: the device and its worker
  are the same. Requests already queued run on and are answered as usual; their events
  carry the `sn:` id. Requests that arrive for the old id after event 1 are answered
  `unknown_device`. Clients move their selection.

### 5.6 Worker and write path

- Each worker owns one device queue (FIFO, at most 32 requests; more is `busy`, except a
  stop, 4.1 rule 2) and applies the queue rules of 4.1 rule 9.
- **Write path:** validate against the description and the current state (`invalid`,
  `unsupported`, `out_of_range`, `conflict`, `asleep`) → send the request's frames and no
  others (section 2), in the model's apply order →
  re-read the groups the model marks as touched by the written leaves and actions → publish
  (new rev, events) → reply.
- After the re-read, each written leaf with `readback:true` either shows the written value
  or enters `pending` with it (4.1 rule 4). Its group is then read at its own tier; the
  pending check runs at those reads and adds none. A `readback:false` leaf takes the
  written value once the camera accepted the write.
- **Restart.** A write of a leaf, or an action, whose `warning` holds `restart` (on the Tiny
  2 Lite `audio.enabled` and `factory_reset`) is answered `ok` once the camera accepted the
  frame, without a re-read of the touched groups. A written leaf enters `pending` with its
  value; the daemon publishes that state and replies with its rev (4.1 rule 3). An action
  replies with the current rev. The restart that follows is ordinary hotplug (5.3): the
  device becomes unavailable, which empties `pending`, and the re-enumerated device
  publishes what the camera reports. Such a request runs as a batch of its own (4.1
  rule 9): the batch before it ends first, its re-read included. After the camera accepted
  the frame, the worker sends nothing more to the device, no request, no stop and no poll,
  until the device becomes unavailable or `REQUEST_DEADLINE` (15 s) has passed; then it
  works as before. Requests queued meanwhile wait under their own deadline (4.1 rule 9).
- **Unavailable.** When the device becomes unavailable (5.3), its worker ends. A started
  request ends as on a camera failure, but without the re-read of 4.1 rule 5: it is
  answered `camera` with `applied` and `failed`, without `rev` (nothing was re-read). The
  requests still queued are answered as 4.1 rule 2 answers requests for an unavailable
  device; for a `port:` entry, which is removed (4.1, Events), they are answered
  `unknown_device`. A re-plug starts a new worker with an empty queue.
- **Stops.** A stop action jumps the queue (4.1 rule 2). A group read in progress ends at
  the next frame boundary; its leaves are then listed in `unread`, and the group is read
  again as 4.2 (`unread`) says.
- **Continuing motion.** The worker records which connection started an action the model
  marks as continuing (on the Tiny 2 Lite `gimbal.turn`). Any other motion action of that
  device clears the record. When the recorded connection closes without `keep:true`, the
  worker queues the model's stop for it. The worker sends no other stop of its own: not when
  `asleep` turns true, and not for a `keep:true` turn (section 2).

### 5.7 Polling and asleep

- **Poll plan.** The model names its groups. Each group has a tier: 1 s, 5 s, 60 s, or
  `touched` (read at plug-in and after a request that touches it, never periodically).
  Each group says whether it can be read while asleep. A group may name a live topic and a
  faster tier used while at least one connection subscribes that topic. A writable leaf
  with `readback:true` is in a group with a periodic tier (1 s, 5 s or 60 s), so that it can
  leave `pending`; the trait conformance test checks it.
- Intervals count from the end of the previous read of that tier, so a slow read never
  starves the 1 s tier.
- Polls run group by group. Queued requests are served between groups, never inside one.
  A group read stops at its first unanswered query; the group's leaves are listed in
  `unread` and read again as 4.2 says, and the other groups go ahead.
- An unchanged read produces no new rev and no event.
- **Asleep.** If the description has the leaf `asleep`, the worker uses its value. While it
  is true, only the groups readable while asleep are read. A setting or action with
  `asleep: ignored` is refused with `asleep`. One with `accepted` or `unknown` is written; if
  its group cannot be read while asleep, the path stays in `pending` until the first read
  after wake, and its `settle_ms` counts from that read.
- **Wake and retries.** When `asleep` turns false, the worker reads once every group that
  has a leaf in `unread`, `touched` groups included, before the periodic tiers go on; the
  next tick of each such group counts from this read. A `touched` group whose read failed or
  was cut short is read again at the next wake or with the next read of the 60 s tier,
  whichever comes first, until a read succeeds.

### 5.8 Timeouts

| Name                   | Value                                                            | When it runs out                                                                    |
| ---------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `ANSWER_POLL`          | 20 reads, 50 ms apart (about 1 s) per query                      | the query is unanswered; its group's leaves go to `unread`; a request gets `camera` |
| `FRAME_GAP`            | 100 ms between two frames of one command, where a model needs it | –                                                                                   |
| `NODE_LOCK_WAIT`       | 5 s for the per-operation lock on the video node                 | a request gets `camera`; a poll skips the cycle                                     |
| `REQUEST_DEADLINE`     | 15 s from a request's arrival                                    | 4.1 rule 9; `hello` is answered (4.1, `hello` timing)                               |
| `CLIENT_REPLY_TIMEOUT` | 30 s, in the client library                                      | the client reports a lost daemon (CLI exit 4)                                       |
| `ACCEPT_BACKOFF`       | 100 ms after a failed accept                                     | –                                                                                   |

The per-operation lock on the video node is kept from 0.13: a second process that talks to
the same camera (a second user's daemon, the firmware updater) takes turns with the daemon
on single frames. One daemon per camera is assumed. The kernel's own UVC control timeout
bounds each control transfer; its value is camera item CAM-29.

### 5.9 Single instance, shutdown, failures

- **Single instance.** Every daemon, socket-activated or not, takes an exclusive `flock` on
  `<socket path>.lock` first. If the lock is held, it exits non-zero with a message that
  names the lock file. A daemon that binds the socket itself first checks whether something
  already accepts connections on that path and exits with a message if so; then it removes a
  stale socket file and binds under umask 077. A 0.13 daemon that bound the socket itself
  holds the same lock file, so a 1.0 daemon started beside it exits with the lock message. A
  socket-activated 0.13 daemon takes no lock; it runs as `obsbot4linux.service` until the
  upgrade steps of 12.2 stop it.
- **Shutdown.** On SIGTERM or SIGINT the daemon closes its connections, so a continuing
  action started without `keep:true` gets its stop (best effort, 5.6), and exits 0. A
  `keep:true` turn goes on, and a crash leaves the camera as it is; `gimbal.stop` ends a turn
  from any connection, after a restart too.
- **Failures.** A panic in a worker, the registry thread or the acceptor ends the process
  with a non-zero code, and systemd restarts it. A panic in a connection thread closes only
  that connection. Locks are taken poison-tolerant.

### 5.10 Operation

- The unit names stay `obsbot4linux.socket` and `obsbot4linux.service`; `ExecStart` runs
  `obsbot4linuxd`. The socket path does not change.
- The daemon starts only through the systemd socket. Packages do not enable the socket (as
  in 0.13). No CLI command, `watch` included, starts a daemon.
- A client that cannot connect (no socket file, connection refused) exits 4 and prints
  `systemctl --user enable --now obsbot4linux.socket`.
- A reply to `hello` that is not a protocol-2 object (0.13 answers with `ok:false` and a
  string `error`) means an older daemon is running. The client says so ("an obsbot4linux
  daemon older than 1.0 is running: restart it or log out") and exits 4. Clients never stop
  or restart a daemon.
- Hardening in 1.0: `NoNewPrivileges=yes` in the service unit. Other lines come after 1.0.
- A firmware update requires `obsbot4linux.socket` and `obsbot4linux.service` stopped until
  M5 defines the lease.
- The daemon reads and writes no file except its socket and its lock file.

### 5.11 Logs

Logs go to stderr and so to the journal. Serials and `sn:` ids are shortened by the one
formatter of 4.2. No log line prints an identity answer.

- Every device that appears, in the start-up pass included, gets one line: its id, model,
  USB ids, USB path and support level, and for a `port:` id of a supported model the reason
  (no serial answered, an invalid serial, a duplicate).
- A hand-over gets one line with the old and the new id.
- A device that goes gets one line with its id.
- A connection closed for a full outbound queue gets one line (5.1).

### 5.12 Fake mode

- `obsbot4linuxd --fake <model>[,<model>…]` runs the daemon with fake devices instead of the
  sysfs scan. It never touches a real device.
- The fake models are second implementations of `trait Camera`, in memory, in
  `obsbot-device/models/fake`:
  - `fake-a` uses the Tiny 2 Lite's description, for client work; `support: tested`.
  - `fake-b` is deliberately different: other enum values, whole groups missing (no
    `gimbal`, no `views`), a setting and an action under a group no client knows, another
    preset count (5), `readback:false` leaves, leaves with `asleep: ignored` and
    `asleep: unknown`, a `requires`, a readback that lags its write by a set time,
    `support: untested`.
  - `unknown` is a device with an unregistered product id of a known vendor id. It enters
    through the same registry lookup as a real device and so becomes an unsupported entry.
- Fake devices have synthetic serials (`sn:FAKEA0000001`) and synthetic USB facts.
- **Test control** goes through a second socket, `<socket path>.test`, which exists only in
  fake mode. One JSON object per line: `{"plug":"<model>"}`, `{"unplug":"<id>"}`,
  `{"sleep":"<id>"}`, `{"wake":"<id>"}`, `{"fail_next":"<id>"}` (the next camera operation
  on that device fails, an identity read included). It is not part of protocol 2 and not in
  any schema or description.
- CI runs `--fake fake-a,fake-b,unknown` in every daemon, client and CLI test that needs a
  daemon (section 11).

## 6. Client and CLI

### 6.1 Client library (`obsbot-proto`, module `client`)

```rust
let mut c = Client::connect(Hello::new("my-tool 1.0"))?;     // subscribe:false
let devices: &[Device] = c.devices();
let (rev, state) = c.get(&id, Some(&["zoom.factor"]))?;
let rev = c.set(&id, Patch::new().leaf("zoom.factor", 1.5))?;
let (rev, result) = c.act(&id, "preset.recall", json!({"slot": 1}))?;

let (hello, tx, rx) = Client::subscribe(Hello::new("widget 1.0").live(&["pose"]))?;
let req = tx.set(&id, Patch::new().leaf("led.brightness", 2))?;  // returns the request id
for msg in rx { match msg? { Incoming::Reply { id, .. } => …, Incoming::Event(e) => … } }
```

- State, patches and action parameters are path → JSON value. The library offers path
  helpers (`state.at("zoom.factor")`, `Patch::leaf`) and no per-setting methods. It may
  validate a patch against the description with the same validator the daemon uses; the
  daemon stays authoritative.
- `Client::connect` blocks per request. `Client::subscribe` returns the `hello` reply, a
  sender and a receiver; a reader thread delivers replies (with their request ids) and
  events in arrival order. The caller keeps its own device list, starting from the `hello`
  reply, and the `devices` events.
- Errors are typed (`ErrorCode`, with an unknown-code variant). There is no built-in retry:
  on disconnect the receiver ends with an error, and the caller decides whether to
  reconnect (4.1).
- Public enums are `#[non_exhaustive]`, so additive protocol changes do not break Rust
  callers.

### 6.2 CLI (obsbot4linux)

The CLI is generic. It maps 1:1 onto protocol 2, so a new model or setting needs no CLI
change. There are no per-feature subcommands and no shortcuts.

```
obsbot4linux status                               state of one device, grouped
obsbot4linux get <path>…                          leaves or groups
obsbot4linux set <path>=<value> […]               one patch
obsbot4linux do <action> [<param>=<value> …]
obsbot4linux describe [<path|action>]             the device description
obsbot4linux devices [--report]                   device list; --report: USB facts
obsbot4linux watch [--live <topic>…]              hello reply and events as JSON lines
  --device <id | serial suffix | model key>       selects a device (see below)
  --json                                          print the protocol objects
```

- **Device selection.** `status`, `get`, `set`, `do` and `describe` act on one device.
  `--device` takes the full id, a serial suffix of at least four characters (the shortened
  id that human output prints, such as `…0001`, is accepted too), or a model key
  (`unknown` included); it must match exactly one listed device. Without `--device` these
  commands use the only usable device (support `tested` or `untested`, available).
  Unsupported devices never count as usable, but `--device` can name one. `devices` lists
  every device; with `--device` it lists only the matched one. `watch` takes no
  `--device`: it prints the events of every device.
- **Unsupported devices.** `devices`, `devices --report` and `watch` show them;
  `describe --device <id>` prints their entry (empty `settings` and `actions`). `status`,
  `get`, `set` and `do` addressed to one are refused by the CLI before anything is sent
  (exit 3, code `unsupported_device`) with the message: "<name> (USB <vendor>:<product>) is
  not supported by obsbot4linux <version>. `obsbot4linux devices --report` prints what an
  issue report needs: https://github.com/hellwand/obsbot4linux/issues".
- **No usable device.** Without `--device` and without a usable device the CLI exits 3
  (code `no_device`) with "no usable OBSBOT camera", followed by the list of unsupported
  and unavailable devices, if any, and the `devices --report` hint.
- **Value syntax**, one per setting type: bool `on|off|true|false`; enum by name
  (`upper_body`, also `upper-body`); number `1.5`; range `min,max`
  (`zone.area.pan=-20,20`); pose `pan,tilt,zoom`; point `x,y`; string as given (quoted for
  the shell). Only leaves are set. Action parameters use the same syntax
  (`do view.save name=Desk pose=0,5,1.2`). A `key=value` argument never starts with `-`;
  free text after `=` may.
- **Output.** Human output is grouped, with units, serials shortened (4.2), and pending
  values marked `(pending)`. `--json` prints exactly the protocol objects:
  - `status`: `{"device":<Device>,"rev","state"}`;
  - `get`, `set`, `do`: the `ok` payload of the reply;
  - `devices`: `{"devices":[<Device>…]}`, the list of the `hello` reply;
  - `describe`: the device entry (`<Device>`); with a path or an action, that `Setting` or
    `Action` object.

  `describe <path|action>` for a path or action not in the device's description exits 2
  with code `unsupported`.

- **Preset names.** `preset save N` without `--name` sends no name, and the CLI never makes
  one up. For a preset whose `name` is empty, `preset list` and `status` may print a
  placeholder of their own (for example `(unnamed)`) as display only; no client writes such
  a label to the camera, and `--json` prints the name as the camera reported it.

- **Errors.** With `--json`, an error prints `{"error":{"code","message"}}` on stdout;
  help and version stay plain text. An error the daemon sent keeps its code. Errors the CLI
  finds itself use these codes; `usage`, `ambiguous_device`, `no_device` and `daemon` exist
  only in the CLI and are never sent by the daemon:

  | Situation                                                             | Code                 | Exit |
  | --------------------------------------------------------------------- | -------------------- | ---- |
  | bad arguments, a value that does not parse                            | `usage`              | 2    |
  | `--device` matches no listed device                                   | `unknown_device`     | 2    |
  | `--device` matches several devices; several usable devices without it | `ambiguous_device`   | 2    |
  | no usable device                                                      | `no_device`          | 3    |
  | an unsupported device addressed                                       | `unsupported_device` | 3    |
  | no socket, connection refused, no reply within `CLIENT_REPLY_TIMEOUT` | `daemon`             | 4    |
  | the daemon answers `protocol`, or is older than 1.0 (5.10)            | `protocol`           | 4    |

  For `ambiguous_device` the CLI prints the matching devices. For `protocol` it prints
  "the obsbot4linux daemon speaks protocol <n>, this client speaks 2: install the same
  version of both and restart the daemon", or the older-daemon message of 5.10.

- **`devices --report`** prints JSON in both modes, because it is meant to be pasted into
  an issue: `{"devices":[…]}` with, per device, `model`, `support`, `firmware`,
  `video_node` and the `usb` object of 4.2. It prints no id and no serial value: only
  `serial_present`.
- **`watch`** sends `hello` with `subscribe:true` and the given live topics, prints the
  reply as its first line and then every event, one JSON object per line, as received. It
  reads nothing from stdin. It exits 0 on SIGINT or SIGTERM and 4 when the connection
  ends.
- **Exit codes:** 0 ok; 1 `internal`; 2 invalid input (`invalid`, `out_of_range`,
  `unsupported`, `unknown_device`, `usage`, `ambiguous_device`); 3 device not usable
  (`conflict`, `unavailable`, `asleep`, `busy`, `camera`, `no_device`,
  `unsupported_device`); 4 daemon unreachable (`daemon`, `protocol`).
- **One-shot commands** send `hello` with `subscribe:false` and read with `get`. The CLI
  waits for the reply; Ctrl-C closes the connection, which drops a request the daemon has
  not started (4.1 rule 9).
- **`gimbal.turn`**: the CLI always sends `keep:true` with it.
- **One request per command.** Every command sends the one request it maps to and chains no
  other (section 2): `do gimbal.move` and `do gimbal.turn` leave the tracking as it is, and a
  user who wants it off first runs `set tracking.mode=static`.
- **Completions** for bash and zsh cover subcommands, flags, `get`/`describe` paths, `set`
  paths with `=`, enum values after `=`, action names and their parameters. They are
  generated from the CLI definition and the embedded description files
  (`descriptions/*.json` of real models only, never the fakes), committed under
  `packaging/completions/`, and a CLI-crate test fails when they are stale. The CLI embeds
  every `descriptions/*.json` file present at build time (a build-script glob), so a new
  file needs no CLI code. Tab never contacts the daemon, and `--device` values are not
  completed. A new model regenerates the description file and the completions in its PR
  with `scripts/regenerate.sh`; the CLI code does not change. How the scripts are generated
  is a choice of track C.
- **Relay fallback.** If the widget cannot open the Unix socket from the desktop shell that
  hosts it (track D's first check, section 10.3), the CLI gets `relay`: it connects, copies
  stdin lines to the socket and socket lines to stdout unchanged, and sends nothing of its
  own. Owner: track C. It is additive. It is not built unless that check fails.

## 7. Multiple cameras

The daemon handles any number of cameras. Mixed models are not a special case: each device
brings its own description. The CLI needs `--device` only with more than one usable device.
The widget and the desktop app have a device picker. CI runs every daemon, client and CLI
test that needs a daemon with `fake-a`, `fake-b` and `unknown` together.

## 8. Models

### 8.1 Registry

- The registry key is the USB (vendor id, product id) pair, read from sysfs before any I/O.
  A model lists every pair it answers to. A test fails if a pair belongs to two models. The
  product string and any other number the camera reports are notes of the model, never
  keys.
- The **vendor table** lists the OBSBOT vendor ids that make a device visible to the
  daemon. It holds only vendor ids read from a unit in hand (8.7): today `0x3564` (measured
  on the Tiny 2 Lite). A further vendor id is added in a registry PR when a unit with it is
  at hand. A device whose vendor id is not in the table is not listed at all.
- Registered today: `tiny2lite`, `3564:fef9` (measured). Nothing else.

### 8.2 What a model provides (`trait Camera`)

A model is one module in `obsbot-device/models/` plus a registry entry: key, display name,
USB pairs, the GUID of its extension unit, the extension-unit selectors it uses, the tier
its identity attempts follow (5.5), support level, and a constructor that takes a
`Transport`. Its type implements:

```rust
pub trait Camera: Send {
    fn description(&self) -> &Description;              // settings, actions, limits; settle_ms
    fn identity(&mut self) -> Result<Identity>;         // one identity attempt (5.5)
    fn poll_plan(&self) -> &[Group];                    // name, tier, readable asleep, live topic
    fn read(&mut self, group: &str) -> Result<Values>;  // leaf path → value
    fn order(&self, leaves: &[&str]) -> Vec<String>;    // safe apply order of a patch
    fn apply(&mut self, leaf: &str, value: &Value) -> Result<()>;
    fn act(&mut self, action: &str, params: &Value) -> Result<Option<Value>>;
    fn touched(&self, target: Target) -> Vec<String>;   // groups to re-read after a leaf or action
    fn stops(&self) -> &[&str];                         // actions that jump the queue
    fn continuing(&self, action: &str) -> Option<&str>; // the stop of a continuing action
    fn send_table(&self) -> &SendTable;                 // section 8.3
}
```

`touched` names the groups whose values the camera itself changes in response to the leaf or
action, the leaf's own group included. A request sends only what it names (section 2), so no
group is touched because of a frame the client did not ask for.

The signatures are fixed in stage W0 and change only by a contract-change PR. The fake
models implement the same trait, so it is designed against two shapes from the start. The
model owns its frame codec choice: the family codec when the family's codec tests pass on
the model's captured frames (same framing, CRC and sequence numbers), its own codec inside
its module otherwise. A codec moves into `family` only when a second model uses it.

### 8.3 What is sent

- Each model declares its **send table**: every extension-unit frame it may send (selector;
  for framed commands the receiver and command id; for short commands the command) and
  every UVC control it may write. Each entry carries its evidence: a capture key or a
  measurement. For the Tiny 2 Lite, entries that only 0.13.2 sends (the U-rows of 9.3) are
  marked as such until camera session CS-1 either measures them or removes them.
- The transport wrapper checks every outgoing frame against the active model's table in one
  place. Anything else is refused with `internal`, logged and never sent.
- Independently of the table, the wrapper refuses two whole classes: every extension-unit
  selector outside the selectors the model's registry entry declares; and, on the receiver
  that answers identity queries (`0x0d` on the Tiny 2 Lite), every frame that is not one of
  the model's declared identity queries.
- No raw send path exists anywhere: not in the daemon, the CLI, the protocol or the fakes.
  Public code and docs name only what is sent.
- A test drives every setting and action of each model at its boundary values on the replay
  transport and asserts that every frame is in the send table, and that each setting and
  action sends exactly the frames of its ledger row, in that order (section 2).
- **No probing.** Capabilities come from the model's code, never from trying ids. The
  extension unit is found by the GUID its registry entry declares, in the USB descriptor
  (read from sysfs), never by a request.

### 8.4 A camera with a known vendor id and an unknown product id (walk-through 2)

1. The kernel enumerates the camera. Within a second the daemon's scan (5.3) finds a video
   node whose USB device has a vendor id from the vendor table and a product id the registry
   does not know.
2. The daemon creates the unsupported entry of 5.4: id `port:<usb path>`, `model: unknown`,
   `support: unsupported`, the USB facts from sysfs, empty settings and actions. It opens no
   node and sends nothing. It logs one line (5.11).
3. Subscribed clients get a `devices` event with the new entry, then its `state` event. The
   widget lists it in its device picker with the marker `unsupported`, shows no controls
   for it, and shows the hint to run `obsbot4linux devices --report`.
4. `obsbot4linux devices` lists it with the level `unsupported` and exits 0.
   `obsbot4linux devices --report` prints its USB facts (no serial value).
5. If it is the only device, `status`, `get`, `set` and `do` exit 3 with the message of
   6.2. If a supported camera is also connected, the CLI uses that camera without
   `--device`, because unsupported devices never count as usable.
6. A `set` or `do` that reaches the daemon for it is answered `unsupported_device`.
7. On unplug its `port:` entry is removed and a `devices` event is sent. The widget drops
   the entry; if it was selected, the widget selects the only usable device, or none.
8. The user opens an issue at `https://github.com/hellwand/obsbot4linux/issues` with the
   `devices --report` output. Then one of three things happens:
   - the maintainers have a unit: onboarding (8.7) starts at step 1 and ends in a `tested`
     release;
   - the user records the unit as its owner (8.7, "Owners"): onboarding starts at step 1,
     the owner does steps 1–3, the maintainers steps 4–6, and the model is released as
     `untested` (8.6);
   - neither: the device stays unsupported.

CI runs this walk-through with the `unknown` fake (section 11).

### 8.5 Evidence, ledgers, fixtures and public text

- **Ledger.** Each model has a ledger: `docs/field-ledger.md` for the Tiny 2 Lite,
  `docs/field-ledger-<model>.md` for every further model. One row per leaf and action:
  path, write (command and encoding), readback, evidence level with its key, asleep
  behaviour, `readback`, `settle_ms`. Rows refer to spec sections, never to spec line
  numbers.
- **Levels.** `captured` and `measured` are the only levels that let a field into a
  released description. A level belongs to each field, and also to each value of an enum's
  `values[]` and `writable[]` and to each bound of a number or range: a value or bound
  without its own level is not in the description. Fields at the level `daemon` are not
  camera values and need neither: `available`, `pending`, `unread`, and `on_air.*`, which
  the family's on-air probe reads from Linux for every supported model. The model module
  records the levels as constants. A test in stage W3 fails if any field, value or bound of
  any registered model, `tested` or `untested`, has another level.
- **Capture keys.** `capture N` names a recording of the Tiny 2 Lite. A further model's
  recordings are `capture <key>-N` (`capture tiny3-01`), so keys never collide.
- **Fixtures** are single exchanges (request frame and answer), inline in a test or in a
  small text table, each with its capture key. Recording files never enter the repository.
  Identity answers and descriptors that hold a serial string are never fixtures; tests use
  synthetic values. Fixtures come from the raw recordings, or from decoded files made by a
  decoder that keeps zero-value writes.
- **Public text.** The repository is public. Spec, ledgers, code comments, fixtures and
  issues may contain: paths, the command ids and encodings that are sent, ranges, units, a
  model's own USB ids, evidence levels and opaque keys ("capture 11", "measured
  2026-10-31"). They never contain: paths or file names of private notes, names of
  vendor-app methods or firmware symbols, whole command tables, descriptions of factory,
  debug or upgrade commands, lists of ids that must not be sent, recordings or excerpts of
  them, serial numbers or fragments of them, the address of a private server, or third
  parties named as sources. The only statements about how knowledge was obtained are
  "recorded from the vendor app" and "measured on the camera".

### 8.6 Support levels

- **`tested`**: the maintainers verified every row of the model's check on a unit (for the
  Tiny 2 Lite: every parity row, section 9). The description holds only backed fields.
- **`untested`**: built from an owner's captures (8.7 steps 1–6) without step 7 on a unit
  by the maintainers. Release notes list it under "Untested models", never as supported.
  Package descriptions name tested models only. A released description of an untested
  model has no action whose `warning` holds `destructive`.
- **`unsupported`**: a known vendor id with an unregistered product id (5.4).
- The README support table has one row per registered model: display name, USB ids,
  support level, and the firmware versions the maintainers tested (`–` for an untested
  model; `not reported` for a model that reports no firmware).
- The release that adds a model has one release-notes line for it: `<display name> (USB
<vendor>:<product>): tested on firmware <versions>`, or under "Untested models": `<display
name> (USB <vendor>:<product>): built from an owner's recordings, not verified by the
maintainers`.
- The CLI prints the level next to every device that is not `tested`. The daemon logs it
  once per plug-in. The widget and the desktop app show a marker for `untested` and
  `unsupported` devices.

### 8.7 Onboarding a model (walk-through 1)

A model is added only with a unit in hand and with our own captures or an owner's captures.
A unit in hand is a unit connected to a host of the maintainers, or of an owner who records
it under this section. Nothing is built for a model before that.

**Owners.** An owner is a user who has a unit and records it for onboarding. The owner runs
steps 1–3 and sends the recordings and the capture log to a maintainer privately, never as
an issue attachment: identity answers in them carry the serial. The way to send them is
agreed in the tracking issue. Sending them is the owner's consent that public fixtures are
cut from them under 8.5. The maintainers keep the recordings privately, never in a
repository, and delete them when the owner asks; fixtures already published stay. An owner
may run step 7 with a dev build (step 7); rows that pass count as `measured`, cited by date
and the issue that records them, but only step 7 by the maintainers on a unit makes the
model `tested`.

Steps, in this order:

0. **Visibility.** Plug the unit into a Linux host running obsbot4linux 1.0.
   - If `obsbot4linux devices` lists it as unsupported, go to step 1.
   - If it is not listed, read `idVendor` and `idProduct` of the unit from sysfs
     (`/sys/bus/usb/devices/<usb path>/`; `lsusb` shows the same pair). If the vendor id is
     not in the vendor table, add it in a registry PR and start again. If it is in the
     table, the unit has no video node in its current mode (for example firmware-update
     mode, 5.3): record the product id and the mode in the tracking issue, bring the unit
     into its normal camera mode, and start again.
     Nothing is sent to the camera in this step.
1. **Descriptor report.** Run `obsbot4linux devices --report --device port:<usb path>` and
   keep the output: USB ids, product name, bcdDevice, whether a serial string exists,
   interface classes, extension units with GUID and control count, video node. If the unit
   shows other product ids in other modes, record each. Open a tracking issue with the
   report.
2. **Captures.** Record the USB traffic between the vendor app and the unit (for example
   usbmon on a Linux host that passes the camera to a virtual machine running the vendor
   app). Stop `obsbot4linux.socket` and `obsbot4linux.service` first, so that nothing else
   talks to the camera. Keep a capture log: one line per step with time, control, old value
   and new value. Record, in this order:
   1. the connect sequence: recording on, plug in, start the vendor app, 60 s idle (this
      also records the idle poll);
   2. each setting the vendor app shows for this model: every value it offers, and the
      lowest and the highest value of each range, one change per step, ending at the
      original value;
   3. each action the vendor app offers, except a firmware update, which is never recorded
      for onboarding; a factory reset last, and only if the owner accepts that the camera
      loses its stored data;
   4. sleep and wake from the vendor app; then, with the camera asleep, quit the vendor app
      and start it again (its connect sequence against a sleeping camera).

   Nothing is sent from Linux in this step.

3. **Private set and public fixtures.** The recordings and logs stay private with the
   maintainers (see "Owners"). Public fixtures are cut from them under 8.5: single
   exchanges, identity answers replaced by synthetic values, each with its capture key.
4. **Model module** `models/<key>` with its registry entry:
   - USB pairs: every product id from step 1. Display name from the product name. Support
     `untested` until step 8.
   - Extension unit: its GUID from the step 1 report and the selectors seen in the
     captures go into the registry entry (8.2, 8.3).
   - Identity source: the USB serial string if `serial_present`, else the identity query
     the vendor app sends in the connect sequence, else none (`port:` ids).
   - Codec: the family codec if the family's codec tests pass on the captured frames;
     otherwise the model's own codec (8.2).
   - Poll plan: groups and tiers taken from the captured idle poll. No tier reads a group
     more often than the vendor app polls it, the faster tier of a live topic (5.7)
     included. One exception: a group with a writable leaf whose captured readback the idle
     poll never reads takes the 60 s tier, which 5.7 requires. The on-air probe (level
     `daemon`, 8.5) is not part of the vendor app's poll and keeps its 1 s tier.
   - Paths: an existing path where function, unit and frame are the same as on another
     model (4.2); `values[]` and `writable[]` for other option sets; a new path or enum
     value for a new function, with its line in the path dictionary (4.2). This is additive
     and needs no client change (guarantee A).
   - Each field, value and bound gets its level in the model's ledger (8.5). One with
     neither captured nor measured evidence is not in the description.
   - No action whose `warning` holds `destructive` in a released `untested` description
     (8.6); the maintainers' step 7 adds them.
   - `asleep` is `unknown` for every field until step 7 measures it.
   - `settle_ms`: until measured on this model, the largest `settle_ms` of any tested model
     (today the Tiny 2 Lite's 3000 ms), marked "not measured on this model" in the ledger.
5. **Send table** (8.3), each entry with its capture key.
6. **Tests** without the camera: every encoder against the captured request bytes; every
   decoder against captured answers; the status decoder against captured blocks with one
   setting changed per block; the send-table test; the registry test; the trait
   conformance test (10.3 W0); the evidence-level test (8.5). `scripts/regenerate.sh`
   regenerates `descriptions/<key>.json` and the completions, and their stale tests pass.
   CI with the fakes stays green.
7. **Check on the unit.** From step 6 on, a dev build of the model may run on the unit; its
   transport sends only the model's send table (8.3). The check is
   `scripts/check-model.sh --device <id> --asleep` (9.4), run by the maintainers (or by an
   owner, see "Owners"). The maintainers' run adds `--destructive`, on a dev build whose
   description also holds the destructive actions captured in step 2; these rows and the
   restart rows count for `tested` (8.6) like every other row. An owner's run leaves out
   `--destructive`, and the rows it skips keep their captured level. Every passing row
   becomes `measured` with the date; `settle_ms` is set from the measured lags and `asleep`
   from the asleep rows; failing fields, values and bounds leave the description.
8. **Release** as `tested` when the maintainers ran step 7 on a unit, with the firmware
   versions it was tested on, else as `untested`; with the README row and the release-notes
   line of 8.6. The CLI needs no change. The widget and the desktop app show the settings
   they know at once; a setting new to them needs their own update (guarantee A).

M4 is the first model that completes these steps.

### 8.8 The Tiny 2 Lite model (`tiny2lite`)

- USB `3564:fef9`; no USB serial string (measured: two units collide in `/dev/v4l/by-id`).
  The extension unit is found by the GUID of the registry entry; the descriptor recorded
  in capture 01 places it at unit 2. Selectors: 2 (framed commands and their answers), 6
  (short commands and the status block), `0x0c` and `0x0d` (the preset list and its
  entries).
- **Identity:** the serial query `0x18c8` to receiver `0x0d` answers 14 ASCII characters.
  It is recorded from the vendor app, which sends it at connect. A read from Linux, asleep
  and right after plug-in, is camera item CAM-1. Each identity attempt also sends the
  firmware query the ledger lists, while the firmware is missing. Attempts 2 and 3 follow
  the 5 s tier (5.5). No other
  identity query is sent: no field uses its answer.
- **Live pose:** query `0x0043` to receiver 3, plus the zoom-factor query. The camera's
  other pose query arms its switch to static after a zone return (#67) and is not in the
  send table.
- **Poll plan.** 1 s: the status block (asleep, tracking mode and the other status values),
  the on-air probe, and the target while the tracking mode is not `static`. 1 s while a
  connection subscribes `pose`, else
  5 s: the live pose. 5 s: the settings readbacks (tracking, framing, gestures, zoom,
  exposure, image, focus, audio, LED, sleep). 60 s: zone, hand, power-on, views.
  `touched`: presets (read at plug-in and after `preset.save`, `preset.rename` and
  `preset.delete`, not after `preset.recall` or `power_on.use_preset`; the
  preset read writes to the camera to rewind its list, which the vendor app never does,
  CAM-19; this rewind is the one known exception to section 2's "no command sequence as a
  workaround" and stays only until CAM-19 decides). While asleep only the status block is
  read. This plan is carried from 0.13.2; the vendor-app rate rule of 8.7 step 4 applies to
  models onboarded under 8.7.
- **Apply order** inside a patch: a setting's `requires` paths before the setting
  (`exposure.mode=manual` before `exposure.time`; `image.white_balance.auto=false` before
  `image.white_balance.temperature`; `focus.mode=manual` before `focus.position`),
  `exposure.mode` before `exposure.gain`, and `tracking.mode` before `framing.*`. The order
  between `exposure.anti_flicker` and `exposure.mode` is CAM-13.
- **Touched groups:** `tracking.mode` touches tracking and framing; `zoom.field_of_view`
  and the gesture settings touch zoom; `preset.recall` touches zoom and the live pose;
  `gimbal.move`, `gimbal.turn`, `gimbal.stop`, `zoom.stop` and `zoom.move` touch nothing.
  The moves touch the tracking group only if CAM-34 shows that the camera changes the
  tracking mode itself on a manual move; until it is recorded they do not.
- **Asleep:** `zoom.field_of_view` is `ignored` (measured); `power_on.go` is `ignored`
  (measured, #79: no move, no error); `wake` is `accepted` (capture 13). The
  pose-taking actions of 4.4 are `ignored` until CAM-22. The readbacks of `audio.volume`
  and `focus.*` do not answer while asleep (0.13.2 behaviour), so a write to them stays
  pending until the first read after wake. Every other field is `unknown` until CAM-2.
- **`readback:false`:** `hand.move_gimbal`, `led.blink` (CAM-6).
- **`settle_ms`:** 3000 for every writable leaf with a readback. This is one model-wide
  bound taken from the worst measured case (zone tracking follows 1–3 s after the write,
  #81; the target-lock status about 1 s), labelled as such in the ledger. Per-leaf bounds are CAM-5.
- **Stops:** `gimbal.stop`, `zoom.stop`. **Continuing:** `gimbal.turn`, stopped by
  `gimbal.stop`.
- **Support:** `untested` on `rebuild` until stage W3. The release candidate carries
  `support: tested`, the claim that stage W4 then verifies on every parity row. If W4 fails
  a row, the row is fixed or becomes a parity exception, and a new candidate is built.

### 8.9 Tiny 3 and Tiny 3 Lite

- They are two models: two registry entries, two modules. They may share functions where
  captures show identical bytes.
- Nothing is built for them before a unit is at hand. Each follows 8.7 from step 0.
- Every property of either device is **unknown until captures**: USB ids, serial string,
  extension unit, frame layout, status block, connect sequence, settings, values, asleep
  behaviour. Section 14 lists the questions.
- Lite functions are not assumed to carry over, and no "common commands" are assumed: every
  property of either device is unknown until captures. One third-party report links
  repeated identity queries to USB drops on a Tiny 3 (unverified); the cap on identity
  attempts (5.5) holds for every model.
- Until a module exists, a unit of either model is an unsupported device (8.4), as long as
  its vendor id is in the vendor table.

## 9. Parity (1.0 acceptance)

### 9.1 Parity table

Every command and option of 0.13.2 (`src/cli.rs`) maps to a 1.0 command, or is a named
exception (9.2). 1.0 is released only when every row is verified on the real Tiny 2 Lite
(stage W4) or is an exception. U-n refers to 9.3, CAM-n to section 14.

| 0.13.2                                                                  | 1.0                                                                  | Note                                                                                                              |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `--json` (global)                                                       | `--json`                                                             | protocol objects; errors as `{"error":{…}}`                                                                       |
| `--direct` (global)                                                     | –                                                                    | PX-3                                                                                                              |
| `--help`, `--version`                                                   | `--help`, `--version`                                                |                                                                                                                   |
| `daemon`                                                                | `obsbot4linuxd`, started by `obsbot4linux.socket`                    |                                                                                                                   |
| `watch`                                                                 | `watch`                                                              | prints events; request forwarding is PX-4                                                                         |
| `live-pose on\|off`                                                     | `watch --live pose`; a client sends `live:["pose"]`                  |                                                                                                                   |
| `detect`, `detect --json`                                               | `devices`, `devices --json`                                          | PX-5                                                                                                              |
| `status`, `status --json`                                               | `status`, `status --json`                                            | new layout (4.3)                                                                                                  |
| `sleep`                                                                 | `do sleep`                                                           |                                                                                                                   |
| `wake`                                                                  | `do wake`                                                            | replies on acceptance                                                                                             |
| `wake zoom F`                                                           | `set sleep.wake_zoom=F`                                              | `no_effect` (#102)                                                                                                |
| `wake mode M`                                                           | `set power_on.mode=M`                                                |                                                                                                                   |
| `tracking MODE`                                                         | `set tracking.mode=MODE`                                             |                                                                                                                   |
| `speed standard\|sport`                                                 | `set tracking.profile=…`                                             |                                                                                                                   |
| `speed headroom`                                                        | `set tracking.profile=headroom`                                      | effect: CAM-33                                                                                                    |
| `speed-mode MODE`                                                       | `set tracking.speed=MODE`                                            | effect: CAM-33                                                                                                    |
| `framing auto-zoom on\|off`                                             | `set framing.auto_zoom=…`                                            | effect: CAM-33                                                                                                    |
| `framing size S`                                                        | `set framing.size=S`                                                 | effect: CAM-33                                                                                                    |
| `framing zoom-speed S`                                                  | `set framing.zoom_speed=S`                                           | effect: CAM-33                                                                                                    |
| `framing offset [--x X] [--y Y]`                                        | `set [framing.offset.x=X] [framing.offset.y=Y]`                      | U-4                                                                                                               |
| `framing part P`                                                        | `set framing.part=P`                                                 | effect: CAM-33                                                                                                    |
| `hdr on\|off`                                                           | `set exposure.hdr=…`                                                 |                                                                                                                   |
| `exposure global\|face`                                                 | `set exposure.mode=auto exposure.region=…`                           |                                                                                                                   |
| `exposure manual`                                                       | `set exposure.mode=manual`                                           |                                                                                                                   |
| `exposure manual --time T`                                              | `set exposure.mode=manual exposure.time=<T×100>`                     | time now in µs                                                                                                    |
| `exposure manual --gain G`                                              | `set exposure.mode=manual exposure.gain=G`                           |                                                                                                                   |
| `zoom F`                                                                | `set zoom.factor=F`                                                  |                                                                                                                   |
| `zoom F --speed S`                                                      | `do zoom.move factor=F speed=slow\|medium\|fast`                     | 10/30/50; U-12; other S is PX-6                                                                                   |
| `zoom stop`                                                             | `do zoom.stop`                                                       |                                                                                                                   |
| `gimbal [--pan P] [--tilt T]`                                           | `do gimbal.move [pan=P] [tilt=T]`                                    | a missing axis keeps its position; the tracking stays as it is (12.2)                                             |
| `gimbal … --speed S`                                                    | `do gimbal.move … speed=S`                                           | U-9; the tracking stays as it is (12.2)                                                                           |
| `gimbal stop`                                                           | `do gimbal.stop`                                                     |                                                                                                                   |
| `gimbal rate [--pan P] [--tilt T]`                                      | `do gimbal.turn [pan_speed=P] [tilt_speed=T]`                        | the CLI sends `keep:true`; the tracking stays as it is (12.2)                                                     |
| `gimbal center X Y` (hidden)                                            | –                                                                    | PX-1                                                                                                              |
| `fov wide\|medium\|narrow`                                              | `set zoom.field_of_view=…`                                           |                                                                                                                   |
| `led N`                                                                 | `set led.brightness=N`                                               |                                                                                                                   |
| `led blink on\|off`                                                     | `set led.blink=…`                                                    | `readback:false`                                                                                                  |
| `mirror on\|off`                                                        | `set image.mirror=…`                                                 |                                                                                                                   |
| `invert on\|off`                                                        | `set gimbal.invert=…`                                                |                                                                                                                   |
| `auto-sleep off`                                                        | `set sleep.auto.enabled=off`                                         |                                                                                                                   |
| `auto-sleep N` (N ≥ 1)                                                  | `set sleep.auto.enabled=on sleep.auto.seconds=N`                     |                                                                                                                   |
| `auto-sleep 0`                                                          | –                                                                    | PX-7                                                                                                              |
| `sleep-background black\|image`                                         | `set sleep.background=…`                                             |                                                                                                                   |
| `sleep-background-mirror on\|off`                                       | `set sleep.background_mirror=…`                                      |                                                                                                                   |
| `audio noise L`                                                         | `set audio.noise_reduction=L`                                        |                                                                                                                   |
| `audio gain on\|off`                                                    | `set audio.auto_gain=…`                                              |                                                                                                                   |
| `audio distance near\|standard\|far`                                    | `set audio.volume=10\|50\|100`                                       | pickup is the volume in bands (4.3): near < 50, standard 50–84, far ≥ 85; 10, 50 and 100 are a choice inside them |
| `audio volume N`                                                        | `set audio.volume=N`                                                 |                                                                                                                   |
| `audio sleep-mic on\|off`                                               | `set audio.in_sleep=…`                                               |                                                                                                                   |
| `audio mic on\|off`                                                     | `set audio.enabled=…`                                                | the camera restarts on both values; U-23                                                                          |
| `gesture set target-lock\|zoom\|dynamic-zoom\|direction-change on\|off` | `set gestures.<name>=…`                                              | names with `_`                                                                                                    |
| `gesture zoom-ratio R`                                                  | `set gestures.zoom_ratio=R`                                          | precision 0.1                                                                                                     |
| `hand right\|left`                                                      | `set hand.side=…`                                                    |                                                                                                                   |
| `hand return-here`                                                      | `do hand.return_here`                                                | live pose                                                                                                         |
| `hand return-go`                                                        | `do hand.go_return`                                                  |                                                                                                                   |
| `hand return-reset`                                                     | `do hand.return_clear`                                               |                                                                                                                   |
| `hand area --pan MIN MAX --tilt MIN MAX`                                | `set hand.area.pan=MIN,MAX hand.area.tilt=MIN,MAX`                   | U-8                                                                                                               |
| `hand area-reset`                                                       | `do hand.area_reset`                                                 | U-7                                                                                                               |
| `hand pan\|tilt on\|off`                                                | `set hand.axes.pan=…` / `set hand.axes.tilt=…`                       | U-5                                                                                                               |
| `hand return-after S`                                                   | `set hand.return_after=S`                                            | U-6                                                                                                               |
| `hand gimbal on\|off`                                                   | `set hand.move_gimbal=…`                                             | `readback:false`                                                                                                  |
| `zone on\|off`                                                          | `set zone.enabled=…`                                                 |                                                                                                                   |
| `zone follow on\|off`                                                   | `set zone.follow=…`                                                  |                                                                                                                   |
| `zone area --pan MIN MAX --tilt MIN MAX`                                | `set zone.area.pan=MIN,MAX zone.area.tilt=MIN,MAX`                   | U-8                                                                                                               |
| `zone return-here`                                                      | `do zone.return_here`                                                | live pose                                                                                                         |
| `zone return-go`                                                        | `do zone.go_return`                                                  |                                                                                                                   |
| `zone view list`                                                        | `get views`                                                          |                                                                                                                   |
| `zone view save NAME`                                                   | `do view.save name=NAME`                                             | live pose                                                                                                         |
| `zone view save NAME --pan P --tilt T --zoom Z`                         | `do view.save name=NAME pose=P,T,Z`                                  |                                                                                                                   |
| `zone view rename ID NAME`                                              | `do view.rename id=ID name=NAME`                                     |                                                                                                                   |
| `zone view delete ID`                                                   | `do view.delete id=ID`                                               |                                                                                                                   |
| `zone view go ID`                                                       | `do view.go id=ID`                                                   |                                                                                                                   |
| `focus mode face\|global`                                               | `set focus.region=…`                                                 |                                                                                                                   |
| `focus auto on`                                                         | `set focus.mode=continuous`                                          |                                                                                                                   |
| `focus auto off`                                                        | `set focus.mode=manual`, or `single` if CAM-14 shows that            | CAM-14                                                                                                            |
| `focus af continuous\|single\|manual`                                   | `set focus.mode=…`                                                   |                                                                                                                   |
| `focus point X Y`                                                       | `do focus.point x=X y=Y`                                             | U-16                                                                                                              |
| `focus set N`                                                           | `set focus.mode=manual focus.position=N`                             | `no_effect` (#99)                                                                                                 |
| `startup set --pan P --tilt T --zoom Z`                                 | `do power_on.set_pose pose=P,T,Z`                                    |                                                                                                                   |
| `startup set --preset N`                                                | `do power_on.use_preset slot=N`                                      | `power_on.start.source`: U-26                                                                                     |
| `startup set --current`                                                 | `do power_on.use_current`                                            | live pose                                                                                                         |
| `startup set --ai M\|none`                                              | `set power_on.start.mode=M\|unset`                                   | U-18                                                                                                              |
| `startup set <pose option> --ai M`                                      | the `do` of the pose option, then `set power_on.start.mode=M`        |                                                                                                                   |
| `startup show`                                                          | `get power_on`                                                       | U-19                                                                                                              |
| `startup go`                                                            | `do power_on.go`                                                     |                                                                                                                   |
| `image brightness\|contrast\|saturation\|hue\|sharpness V`              | `set image.<name>=V`                                                 |                                                                                                                   |
| `image white_balance_auto 0\|1`                                         | `set image.white_balance.auto=off\|on`                               |                                                                                                                   |
| `image white_balance_temperature V`                                     | `set image.white_balance.auto=off image.white_balance.temperature=V` | `requires`; CAM-15                                                                                                |
| `image gain V`                                                          | `set exposure.gain=V`                                                |                                                                                                                   |
| `image power_line_frequency 0\|1\|2`                                    | `set exposure.anti_flicker=off\|50hz\|60hz`                          |                                                                                                                   |
| `image backlight_compensation V`                                        | `set exposure.compensation=V`                                        | U-20                                                                                                              |
| `image auto_exposure 0\|1\|3`                                           | `set exposure.mode=auto\|manual\|auto`                               | V4L2 1 = manual; 0.13.2 writes 0 for automatic, and 3 maps to `auto` too                                          |
| `image exposure_time V`                                                 | `set exposure.mode=manual exposure.time=<V×100>`                     | µs                                                                                                                |
| `image focus V`                                                         | `set focus.mode=manual focus.position=V`                             | `no_effect` (#99)                                                                                                 |
| `image autofocus 0\|1`                                                  | `set focus.mode=manual\|continuous`                                  | as `focus auto`; CAM-14                                                                                           |
| `image gamma V`                                                         | –                                                                    | PX-2                                                                                                              |
| `preset list`                                                           | `get presets`                                                        |                                                                                                                   |
| `preset save N`                                                         | `do preset.save slot=N`                                              | live pose; no name (12.2, CAM-35)                                                                                 |
| `preset save N --name S`                                                | `do preset.save slot=N name=S`                                       |                                                                                                                   |
| `preset save N --pan P --tilt T --zoom Z`                               | `do preset.save slot=N pose=P,T,Z`                                   |                                                                                                                   |
| `preset rename N S`                                                     | `do preset.rename slot=N name=S`                                     |                                                                                                                   |
| `preset delete N`                                                       | `do preset.delete slot=N`                                            | CAM-18                                                                                                            |
| `preset recall N`                                                       | `do preset.recall slot=N`                                            |                                                                                                                   |
| `factory-reset --yes`                                                   | `do factory_reset confirm=true`                                      | `warning: [destructive, restart]`                                                                                 |
| `factory-reset` without `--yes`                                         | `do factory_reset` is refused with `invalid`                         |                                                                                                                   |

### 9.2 Parity exceptions

- **PX-1** `gimbal center X Y` (hidden in 0.13): not carried over. It moved the gimbal only
  toward the left and downward in tests.
- **PX-2** `image gamma`: the camera has no gamma control (its recorded UVC descriptor does
  not offer one); 0.13's command always failed. No function is lost.
- **PX-3** `--direct`: removed. The CLI never talks to the camera; socket activation starts
  the daemon.
- **PX-4** request forwarding of `watch` from stdin: removed. The widget speaks protocol 2
  on the socket; `relay` is the fallback (6.2).
- **PX-5** `detect` without a daemon and with exit 1: replaced by `devices`, which needs the
  daemon and exits 0 when it lists a device and 3 (`no_device`) when the list is empty. A
  script that needs a usable camera reads `support` from `devices --json`, or runs
  `status`, which exits 3 without one.
- **PX-6** `zoom F --speed S` with any integer S: only `slow|medium|fast` (10, 30, 50), the
  only speeds with a known meaning.
- **PX-7** `auto-sleep 0`: not offered. The camera cannot store "off" together with 0 s, and
  "on with 0 s" was never recorded.
- **PX-U-n**: a row U-n of 9.3 whose check fails becomes the exception PX-U-n, with the
  0.13.2 rows named in its "Fail" column.

### 9.3 Tiny 2 Lite fields without backing today

These fields, values and bounds are 0.13.2 functions whose write and readback (or effect)
are neither captured nor measured today; levels are kept per field, value and bound
(section 8.5). One rule holds for every row: the description written in stage W0 contains
the field, value or bound (it is a 0.13.2 function); the check runs on the camera in
session CS-1 (stage A, camera item CAM-23); what passes is kept with level `measured`;
what fails leaves the description by a contract-change PR before the release candidate and
becomes the named exception. The release candidate contains no row that has not passed. No
field stays on a guess, and none disappears silently. The numbers U-1, U-2, U-3, U-10,
U-11, U-13, U-14, U-15, U-17, U-21, U-22 and U-24 belonged to fields that the field ledger
shows as backed (measured 2026-10-31 or captured); they are not reused.

| U    | Field, value or bound                                                                                                            | Backing today                                                                                                   | Check in CS-1                                                                                                                            | Fail: exception and 0.13.2 rows lost                                                                                   |
| ---- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| U-4  | `framing.offset.x`, `.y`: the bounds −0.16..0.16 and the direction                                                               | values read back (measured, #98); bounds and direction not checked                                              | set both bounds and 0, read back; observe the direction with a person in view                                                            | bounds narrowed to the measured ones; PX-U-4: `framing offset` if no value works                                       |
| U-5  | `hand.axes.pan`, `.tilt`                                                                                                         | no recording; effect not observed                                                                               | set, read back; hand tracking with a person in view                                                                                      | PX-U-5: `hand pan`, `hand tilt`                                                                                        |
| U-6  | `hand.return_after`                                                                                                              | no recorded write                                                                                               | set, read back                                                                                                                           | PX-U-6: `hand return-after`                                                                                            |
| U-7  | `hand.area_reset`                                                                                                                | no recording                                                                                                    | narrow the area, reset, read back the default area                                                                                       | PX-U-7: `hand area-reset`                                                                                              |
| U-8  | `hand.area`, `zone.area` with a new range wholly beyond the stored one                                                           | frame order unmeasured                                                                                          | write such ranges for pan and tilt, read back                                                                                            | the model sends the bounds in the order the check shows; re-checked                                                    |
| U-9  | `gimbal.move` with `speed`: tilt 86–90°, speeds 178–180 °/s, one axis with speed                                                 | speeds 1, 5 and 100 °/s to tilt 80 and pan 145 measured (#98); the rest not measured                            | moves with speed to tilt 86, 88, 90; speed 178 and 180; pan only                                                                         | range narrowed to the measured one; PX-U-9: `gimbal --speed` if no move with speed works                               |
| U-12 | `zoom.move` speeds `slow` (10), `fast` (50)                                                                                      | only 30 captured                                                                                                | a move at each speed, factor reached                                                                                                     | the speed leaves `writable`; PX-U-12: `zoom --speed` with that speed                                                   |
| U-16 | `focus.point`                                                                                                                    | accepted, and the point reads back (measured, #97); the picture blurs instead of focusing (#99)                 | point at an object, observe the focus                                                                                                    | PX-U-16: `focus point`                                                                                                 |
| U-18 | `power_on.start.mode`: writing `static`, `upper_body`, `headless`, `group`; names of read values other than `normal` and `unset` | only `normal` and `unset` recorded from the vendor app                                                          | set each value, read the block, power-cycle, observe the start mode                                                                      | the value leaves `writable`; an unconfirmed read value shows as `raw:<n>`; PX-U-18: `startup set --ai` with that value |
| U-19 | `power_on.settings.*`: values other than the recorded one                                                                        | each leaf captured with one value (capture 08b); the meaning of other values not recorded                       | change each in the vendor app's initial-state dialog (recorded), read the block                                                          | the leaf leaves the description; PX-U-19: that part of `startup show`                                                  |
| U-20 | `exposure.compensation` in EV                                                                                                    | raw range 0–18 and default 9 captured (capture 12); 1/3 EV per step inferred from two clicks                    | compare the vendor app's displayed steps over the whole range                                                                            | stays the raw value (no exception: the raw value is the 0.13 function)                                                 |
| U-23 | `audio.enabled` readback                                                                                                         | the audio interface disappears and returns on toggle (measured); cold plug with the microphone off not measured | cold plug with the microphone off; look for a status-block value that follows the microphone                                             | readback from the status block if one follows; else `readback:false`                                                   |
| U-25 | `exposure.anti_flicker` value `auto` (read-only)                                                                                 | never read from the camera; no recording shows it                                                               | if the vendor app offers automatic anti-flicker: set it there (recorded), then read the status block from Linux; if not, the check fails | the value leaves `values`; the camera's value shows as `raw:3`; no 0.13.2 row is lost                                  |
| U-26 | `power_on.start.source` values `preset_<slot>`                                                                                   | the action is captured (capture 10); no readback after it                                                       | `do power_on.use_preset slot=N` for each slot, read `power_on.start.source`                                                              | the values leave `values`; such a source shows as `raw:<n>`; no 0.13.2 row is lost                                     |

### 9.4 Check script

`scripts/check-model.sh --device <id> [--asleep] [--destructive]` checks one device against
its own description. It replaces 0.13's `check-readback.sh`.

- It requires `--device`. It reads `describe --json` and builds its rows from it: every
  writable enum value, both values of each bool, minimum and maximum of each number and
  range.
- **Setting rows.** Per row: `set`, then wait up to 70 s (the slowest periodic tier, 60 s,
  plus 10 s) for a state that shows the written value. The row passes when the value is
  shown; then the script restores the value. The time from the reply to that state is the
  row's lag, and the results file notes whether the value showed in the state the reply
  published (the touched re-read) or at a later read of the leaf's group. A lag seen at a
  later read is known only to the resolution of that group's tier.
- **`settle_ms` from the lags.** Where every row of a leaf showed its value at the touched
  re-read or at a read of a 1 s group, the ledger sets `settle_ms` to the largest lag,
  rounded up to the next whole second. Where a value showed only at a later read of a 5 s
  or 60 s group, the lag is a tier artefact: the leaf keeps the model-wide bound, and the
  ledger notes "below <tier>". A path can stay pending for up to its group's tier, because
  `pending` is checked only at the group's own reads (5.6).
- **`readback:false` rows** pass only when the person running the script confirms the
  effect (for example, the LED blinks); the script prints what to look for and asks.
  `no_effect` rows are checked by readback only. The script says per row which kind it is.
- **Actions** are a checklist, run one by one after a confirmation: the script prints the
  action, runs it, and prints the state change it saw (paths before and after). The person
  compares that change and the visible effect with the effect column of the model's ledger
  and answers pass or fail. Rows that need a person in view are marked.
- **Asleep rows** (with `--asleep`), for every writable leaf and action whose `asleep` is
  `unknown`: `do sleep`, wait up to 30 s for `asleep:true`, send the write or the action,
  record the reply; `do wake`, wait up to 30 s for `asleep:false`, then wait up to 70 s for
  the written value (for an action, the person confirms the effect). The row's result is
  `accepted` when the value or the effect is there after the wake, and `ignored` when the
  camera refused it or the value or effect is not there. The ledger takes the results.
- Rows whose `warning` holds `restart` or `destructive` run only with `--destructive`, last,
  after presets and views are printed. Such a row passes when the device returns under the
  same id within 60 s with the expected state.
- It snapshots the state first and restores it through a `trap` on EXIT, INT and TERM: the
  snapshot's tracking mode and pose, never hard-coded values. A field whose snapshot value
  is `null` is skipped.
- Output: one line per row and a results file. The ledger update takes `measured` and the
  date from it.
- CI runs it against `fake-a` and `fake-b` with `--assume-pass`, which answers every
  question to the person with pass. In stage W4 the release PR maps its results to every row
  of 9.1.

## 10. Way of working

### 10.1 Branches and rules

- `main` stays releasable. Until stage G is released it stays at 0.13.x and can ship a patch
  at any time.
- Audit fixes that need no camera (tests that pin recorded bytes, docs, CI, logic with unit
  tests) land on `main` first, in the 0.13 layout, so that the groundwork move carries them.
  Fixes whose acceptance needs the camera wait for it (section 14).
- The rewrite happens on a short-lived branch `rebuild`, started from `main` with this spec
  as its first commit. Before its first PR, `ci.yml` runs on `main` and `rebuild` for push
  and pull request (the change lands on `rebuild` itself), and a ruleset for
  `refs/heads/rebuild` requires signed commits and the `required` check and blocks
  deletion. Both are removed after the final merge.
- Each stage or track is one or more PRs into `rebuild`, each with the `required` gate and
  a review by someone other than its author (a maintainer, or a review agent run by one).
- **Contract-change rule.** A change to the `obsbot-proto` types, the description format,
  `trait Camera`, `trait Transport` or the schema goes first as its own small PR into
  `rebuild`. Tracks rebase onto it and never change those files inside a track PR.
- When `main` moves (a 0.13.x or 0.14.x patch), `rebuild` is rebased onto it between PRs,
  with a note in the tracking issue. Nobody pins a `rebuild` commit hash.
- At the end `main` fast-forwards to `rebuild` and `v1.0.0` is tagged. Before tagging,
  `HEAD..origin/main` is empty.

### 10.2 Stage G: groundwork (released as 0.14.0)

- **Content:** the cargo workspace (`[workspace.package]` with version, rust-version and
  licence; `[workspace.dependencies]`); `crates/obsbot-device` with the carried modules at
  their final paths as plain modules (`transport`, `family`, `models/tiny2lite`,
  `registry`); `camera.rs` split into transport and Tiny 2 Lite flows; the 0.13 daemon and
  CLI unchanged in `crates/obsbot4linux` on top of it; byte tests moved with their code; CI
  and packaging for the workspace (package metadata in the binary crate); one mutation-test
  pass over the carried modules, survivors fixed or documented. No behaviour change.
- **Owner files:** all; one owner.
- **Depends on:** the camera-free audit fixes on `main`; CI and the ruleset on `rebuild`.
- **Acceptance without camera:** `cargo test --workspace` passes with every 0.13.2 test
  present; the diff of the move shows renames and module paths only (reviewed); the CLI's
  `--help` output equals 0.13.2's; a packaging run produces the eight artifacts of 12.1
  with the 0.13 binary; the mutation pass is recorded in the PR and kills these mutants: a
  rejected answer handled as stale, a rejected tracking query failing the whole read, and
  each sign flip in the angle conversions. The tests that kill them are camera-free audit
  fixes and land on `main` before stage G (10.1), so stage G itself changes no behaviour.
- **Acceptance with camera (CS-0, CAM-30):** on the real Tiny 2 Lite: `status`, one setting set and
  read back, one gimbal move, `preset list`. Then `main` fast-forwards to the last commit of
  stage G and `v0.14.0` is tagged.

### 10.3 Stages of the rewrite

After stage G is merged into `rebuild` (its release as 0.14.0 may wait for the camera), the
rewrite runs as W0, then W1 (four parallel tracks A–D), then W2, W3 and W4.

**W0: contracts**

- **Content:** the four crates as compiling stubs; the 0.13 daemon, client, protocol, state
  and CLI modules removed from `rebuild` (they stay on `main` for 0.14.x); in
  `obsbot-proto` the envelopes, `ErrorCode`, the description types, the validator and the
  path helpers; `schema/protocol-2.json` with its stale test; in `obsbot-device` the
  signatures of `trait Camera` and `trait Transport`; the fake models `fake-a` and `fake-b`;
  the Tiny 2 Lite description as data (every field of 4.3 and 4.4, U-rows included);
  `descriptions/tiny2lite.json` and `descriptions/fake/*.json`; golden protocol transcripts
  (`hello`, `state` and `devices` events, every error code, an unsupported entry, a
  `replaced_by` hand-over); the normative part of `docs/protocol-2.md`.
- **Owner files:** all; one owner.
- **Depends on:** stage G merged into `rebuild`.
- **Acceptance without camera:** the workspace builds and its tests pass; the schema stale
  test passes; every golden file round-trips byte-equal through the proto types; `fake-a`
  and `fake-b` pass a trait conformance test (every described leaf readable; every writable
  leaf shows a valid written value after the touched re-read, or enters `pending`; every
  writable leaf with a readback in a group with a periodic tier, 5.7); `fake-a`'s
  `gimbal.move` and `gimbal.turn` are one operation each and leave `tracking.mode` alone
  (section 2); the `cargo metadata` check of section 3 passes.
- **Acceptance with camera:** none.

**Track A: device**

- **Content:** the USB transport (extension unit by GUID, named timeouts, per-operation
  node lock, send-table check), the replay transport, the family code (scripted answer-poll
  tests: read error, stale sequence, rejected, empty, no answer), `models/tiny2lite` behind
  the trait (identity, poll plan, read, apply, order, act, touched, stops, continuing, send
  table), `registry` with the vendor table, `enumeration` (sysfs), and `#[ignore]` live
  tests that run only with an explicit environment variable and a connected camera and send
  only the model's send table.
- **Owner files:** `crates/obsbot-device` (except the trait files), `descriptions/tiny2lite.json`,
  `docs/field-ledger.md`.
- **Depends on:** W0.
- **Acceptance without camera:** every encoder against recorded request bytes; every
  decoder against recorded answers; the status decoder against recorded blocks; angle
  conversions as pure functions tested with a recorded answer and one asymmetric case; the
  send-table test; the registry test; the serial-format test; each audit defect in carried
  code fixed or named in section 14; the carried gimbal move and rate flows without their
  switch to the static tracking mode (3.2).
- **Acceptance with camera (CS-1):** the live tests: CAM-1 to CAM-8, CAM-10, CAM-13 to
  CAM-20, CAM-22, CAM-23, CAM-25, CAM-33 to CAM-35 (CAM-21 is recorded if it happens and
  gates nothing).
  Results go into the ledger and arrive as contract-change PRs (U-rows removed,
  `settle_ms`, `asleep` values, `focus auto off` mapping).

**Track B: daemon**

- **Content:** `crates/obsbot4linuxd` as specified in section 5, in two milestones. B-1: the
  daemon core with fake mode (`--fake`, test-control socket), so that tracks C and D can run
  their acceptance against it; it is reached when its last task has landed and is built in
  several pull requests. B-2: the real device path (enumeration, registry lookup,
  workers over `trait Camera` with `tiny2lite`).
- **Owner files:** `crates/obsbot4linuxd`.
- **Depends on:** W0 (B-1); track A's registry, enumeration and `tiny2lite` (B-2).
- **Acceptance without camera:** named integration tests, all with `fake-a`, `fake-b` and
  `unknown` running together: rev order and event-before-reply; pipelined requests; N
  pipelined `set`s on one connection cause at most two re-reads; single-leaf coalescing;
  stops overtake the queue; whole-patch validation, apply order, `applied`/`failed`;
  `requires` gives `conflict`; `pending` appears and clears (lagging readback of `fake-b`,
  and the settle bound); `asleep: ignored` refused, `asleep: unknown` written and pending
  until wake; the 15 s deadline; `busy` at 32; requests of a closed connection dropped;
  `gimbal.turn` stopped on connection close unless `keep`; hotplug add, unplug, re-plug;
  `port:` → `sn:` hand-over with `replaced_by`; the unsupported entry receives no frame (the
  fake transport records none); `hello` waits for the first enumeration; live-topic
  filtering; a slow subscriber is disconnected; the 64 KiB request limit; the 64-connection
  limit; the single-instance lock; the panic policy; `hello` answered by its deadline while
  a first read is slow; the hand-over event sequence and its rev; requests for unsupported
  and unavailable devices answered at once; a `restart` write answered on acceptance, and
  nothing sent after it until the device goes; a device-number change handled as unplug plus
  plug; the queue of an unplugged device answered and a new worker on re-plug; a re-added
  `port:` entry starting at rev 1; the wake read of `unread` groups and the retry of a
  failed `touched` read; an identity series ending at the first complete answer; a `set`
  causes at most one `apply` per leaf of its patch and no `act`, a `do` exactly one `act`
  and no `apply`, and apart from reads nothing else reaches the fake for that request
  (section 2); no stop reaches the fake when it falls asleep during a turn, nor for a
  `keep:true` turn at shutdown (5.6, 5.9).
- **Acceptance with camera:** none in this track (W4 covers the real path).

**Track C: client and CLI**

- **Content:** the client module of `obsbot-proto` (6.1); the CLI (6.2): commands, value
  syntax, human output, `--json`, exit codes, device selection, completions and their stale
  test, the older-daemon message.
- **Owner files:** `crates/obsbot-proto/src/client*`, `crates/obsbot4linux`,
  `packaging/completions/`.
- **Depends on:** W0; B-1 for the tests against a running fake daemon.
- **Acceptance without camera:** value parsing per setting type; rendering snapshots from
  the golden states; one test per error code, the CLI-only codes included, for its exit
  code; the `--json` error shape for a usage error; the `--json` shapes of `devices` and
  `describe`; an unsupported device refused before anything is sent; a non-protocol-2
  `hello` reply gives exit 4 and the restart message; no socket gives exit 4 and the
  `systemctl` line; end-to-end snapshots against an in-process
  `obsbot4linuxd --fake fake-a,fake-b,unknown`; every row of 9.1 turned into its request
  against `fake-a`, the gimbal rows reaching the fake as one `act` each and no `apply`; the
  completions stale test.
- **Acceptance with camera:** none in this track.

**Track D: widget (repository `obsbot4linux-dux`)**

- **Content:** first, a check that the widget can open the Unix socket from the desktop
  shell that hosts it (DankMaterialShell); if it cannot, track C adds `relay` (6.2) and the
  widget uses it. Second, a check whether `dms plugins update` follows the widget
  repository's default branch or its tags; the answer decides how the upper bound of widget
  0.29.2 reaches users. Then the widget on protocol 2: `hello`, device picker, every
  control gated on a path or action of the description, enum values, bounds, step, unit
  and list limits taken from the description, `pending` values shown, live topics only
  while the popup is open, markers for `untested` and `unsupported`, the exit-4 hint, and
  the defaults reset built from `default` values (a `set` patch; which controls it covers is
  the widget's product choice). Each control sends its one request and chains none (section
  2, 12.3). Also widget 0.29.2, the last 0.x release, which refuses
  obsbot4linux ≥ 1.0 (`plugin.json`: `obsbot4linux >= 0.7.3, < 1.0`); 0.28.0 to 0.29.1 are
  released already and carry the refusal. Widget 1.0 is built on a branch `v1` of the
  widget repository until release day, because users receive every commit of its default
  branch at their next plugin update.
- **Owner files:** the widget repository.
- **Depends on:** W0 (golden transcripts); B-1 for live tests.
- **Acceptance without camera:** against `obsbot4linuxd --fake fake-a,fake-b,unknown`:
  `fake-b`'s unknown setting does not break the widget; its missing groups hide their
  controls; its controls offer `fake-b`'s own enum values and bounds and its preset count,
  not the Tiny 2 Lite's; the unsupported device shows as in 8.4 step 3 and disappears on
  unplug as in step 7; a value set with the CLI appears
  in the widget at once (`pending`); a gimbal control sends one request (checked against the
  fake's operation log or by review); widget 0.29.2 is released before the 1.0 release
  candidate.
- **Acceptance with camera:** in W4.

**W2: hook-up**

- **Content:** daemon and `tiny2lite` end to end in a dev build; `scripts/check-model.sh`
  (9.4); `docs/protocol-2.md` completed with worked transcripts; the migration notes drafted
  (12.2).
- **Owner files:** `scripts/`, `docs/`; glue in crates only by contract-change PRs.
- **Depends on:** A, B, C.
- **Acceptance without camera:** the check script passes against `fake-a` and `fake-b` in
  CI; the CLI end-to-end tests pass; descriptions and completions are regenerated and their
  stale tests pass; no U-row that CS-1 failed is left in `descriptions/tiny2lite.json`.
- **Acceptance with camera:** none.

**W3: packaging and release candidate**

- **Content:** two-binary packages (12.1) with units (`ExecStart` = `obsbot4linuxd`,
  `NoNewPrivileges=yes`), completions and the schema; the release workflow publishes a tag
  `v1.0.0-rc.N` as a pre-release; the Tiny 2 Lite set to `support: tested` (8.8); CHANGELOG
  with the migration notes; README support table, and the README without the 0.13 statements
  that the gimbal moves switch the tracking to static first (section 2, 15.1 row 88).
- **Owner files:** `.github/`, `packaging/` (except completions), `CHANGELOG.md`,
  `README.md`, and the support level in the `tiny2lite` registry entry.
- **Depends on:** W2.
- **Acceptance without camera:** a workflow run produces the eight artifacts, and a listing
  of each shows both binaries, the units, the completions and the schema; `makepkg` with its
  `check()` passes without `XDG_RUNTIME_DIR` (the tests use a fake daemon on a temporary
  socket); the evidence-level test of 8.5 passes for `tiny2lite`; the release candidate is
  published.
- **Acceptance with camera:** none (W4 runs on this candidate).

**W4: live parity and release**

- **Content (CS-2, Thomas present):** install the release candidate on the Arch desktop,
  starting from the 0.13 state (socket disabled, the 0.13 daemon a child of the widget's
  `watch`) and following the migration notes; run `check-model.sh` on the Tiny 2 Lite, with
  `--destructive` last; map its results to every row of 9.1; widget 1.0 against the
  candidate; the hardened unit starts, sees the camera, and handles hotplug.
- **Owner files:** release PR (tracking table of 9.1 results), CHANGELOG.
- **Depends on:** W3; D.
- **Acceptance without camera:** none; this stage is the camera session.
- **Acceptance with camera:** CAM-9, CAM-24, CAM-27, CAM-28 (and CAM-26 if a second unit is
  at hand); every row of 9.1 verified or an exception of 9.2. A failing row is fixed or
  becomes an exception, and a new candidate is built. Then the release follows 10.6.

### 10.4 Critical path and camera sessions

Critical path: G → W0 → A → CS-1 → W2 → W3 → W4 (CS-2). B, C and D run beside A; at most
four tracks run in parallel. The camera is needed three times: CS-0 (end of G), CS-1 (track
A) and CS-2 (W4). The Tiny 2 Lite is not available for tests at the moment; stages that need
it wait for it, and everything else goes ahead.

### 10.5 Milestones (proposal)

This is a proposal only. No milestone is edited on GitHub before Thomas has seen this
spec, and the plan is first reconciled with the audit's fix plan.

- **0.14.0, groundwork:** stage G, with #47 and #61 (both in carried code; they land on
  `main` before the move, as 10.1 requires, and ship with 0.14.0 unless a 0.13.x patch
  ships them first).
- **1.0.0, rebuild:** W0 to W4; #73 (target-box events, by the live topic `target`) and #89
  (start pose, start settings and preset names in the state) move into it.
- **0.15.0 (camera research) and 0.16.0 (other apps):** after 1.0, when a camera is at hand.
- A second camera model never blocks 1.0; it is M4.

### 10.6 Release day and "done"

Release day, in this order:

1. Widget 0.29.2 (the last 0.x release, with its upper bound) is out.
2. The release candidate `v1.0.0-rc.N` is published and W4 has passed on it.
3. `main` fast-forwards to `rebuild`; tag `v1.0.0`; the release workflow publishes the
   packages; the maintainers' private mirror of the repository follows the tag.
4. Widget 1.0 is released after the tool, the same day: `v1` is merged into the widget
   repository's default branch by fast-forward. Its `plugin.json` requires
   obsbot4linux ≥ 1.0. The widget's entry in the DankMaterialShell plugin registry is
   updated.
5. The 0.13.2 packages and widget 0.29.2 stay downloadable for rollback. The release notes
   name them. Rollback beyond that is a fix forward.

1.0 is done when:

- every row of 9.1 is verified on the real Tiny 2 Lite or is an exception of 9.2;
- CI is green with `fake-a`, `fake-b` and `unknown` running together;
- the packages are proven by the release candidate (W3);
- widget 1.0 is tested against the release candidate;
- the migration notes are written;
- both walk-throughs (8.4, 8.7) pass on this spec, and walk-through 2 runs in CI with the
  `unknown` fake.

### 10.7 After 1.0

- `main` is the only long-lived branch. Every change is a PR into `main` with the `required`
  gate, signed commits and a review by someone other than its author.
- The contract-change rule of 10.1 stays: a change to the `obsbot-proto` types, the
  description format, `trait Camera`, `trait Transport` or the schema goes first, as its
  own PR. The changes to the seam that M4 shows to be needed (section 13) follow this rule.
- A model PR follows 8.7: module, registry entry, ledger, fixtures, send table, regenerated
  descriptions and completions (`scripts/regenerate.sh`), path dictionary lines, README row.
- Versions follow 12.5. A new model, setting, value, setting type or unit is additive: a
  minor version. A fix is a patch version. A breaking protocol change raises the protocol
  number and the major version, and runs like 1.0: its own branch, a release candidate and
  a live check of every model the maintainers can test.
- A release is a tag `vX.Y.Z` on a commit of `main` whose CI is green; the release workflow
  publishes the packages, and the release notes carry the lines of 8.6. A release that
  changes the packaging, the units or the protocol publishes a release candidate first, as
  in W3.

## 11. Testing

| Level              | What                                                                                                                                          | Where                                 |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| Camera bytes       | encoders and decoders against recorded exchanges on the replay transport; scripted answer-poll tests; angle conversions; send table; registry | `obsbot-device`, CI                   |
| Descriptions       | trait conformance of every model and fake; stale test of `descriptions/`; the evidence-level test (8.5)                                       | `obsbot-device`, CI                   |
| Protocol           | golden transcripts round-trip byte-equal; stale test of `schema/protocol-2.json`                                                              | `obsbot-proto`, CI, `makepkg check()` |
| Daemon             | the named integration tests of track B, with `fake-a`, `fake-b` and `unknown` together                                                        | `obsbot4linuxd`, CI                   |
| Client and CLI     | parsing, rendering snapshots, exit codes, end-to-end against an in-process fake daemon, the 9.1 rows against `fake-a`, completions stale test | `obsbot-proto`, `obsbot4linux`, CI    |
| Live, device crate | `#[ignore]` tests behind an environment variable, sending only the send table                                                                 | `obsbot-device`, CS-1                 |
| Real camera        | `scripts/check-model.sh` on the Tiny 2 Lite; every row of 9.1                                                                                 | W4 (CS-2)                             |
| Mutation           | one pass over the carried modules                                                                                                             | stage G, recorded in its PR           |

- Without `LISTEN_FDS` the daemon binds `OBSBOT4LINUX_SOCKET` or the default path. Tests run
  `obsbot4linuxd::run(Config)` in-process on a temporary socket, in parallel, without
  systemd and without `XDG_RUNTIME_DIR`. The fakes are deterministic.
- Golden files cover protocol 2 and the descriptions. An intentional change updates them in
  the same PR.
- Mutation tests are not a 1.0 gate.

## 12. Release and clients

### 12.1 Packages

- One package `obsbot4linux` with both binaries, the systemd user units, the bash and zsh
  completions (installed to the standard completion directories) and
  `/usr/share/obsbot4linux/protocol-2.json`.
- Formats as today: Arch, deb, rpm and tarball, each for x86_64 and aarch64: eight
  artifacts. aarch64 builds natively. There is no libudev dependency.
- Packages do not enable the socket unit.

### 12.2 Migration notes

The CHANGELOG of 1.0 carries the migration notes:

- the command map (9.1) and the exceptions (9.2);
- the upgrade steps: `systemctl --user daemon-reload`; end the old daemon
  (`systemctl --user stop obsbot4linux.service`, or end the `obsbot4linux daemon` process
  that `obsbot4linux watch` started, or log out); remove drop-ins whose `ExecStart` runs
  `obsbot4linux daemon`; `systemctl --user enable --now obsbot4linux.socket`; update the
  widget together with the tool and reload the shell that hosts it. Setups that never
  enabled the socket (the default for widget users) have no daemon otherwise, because 1.0
  clients no longer start one;
- `$XDG_STATE_HOME/obsbot4linux/` (it held `state.json` and `daemon.log`) is no longer used
  and can be deleted. Nothing is imported, and nothing set on the camera is lost: the camera
  stores it;
- the 0.13 commands `gimbal [--pan P] [--tilt T]` and `gimbal rate` switched the tracking
  to static before they moved. `do gimbal.move` and `do gimbal.turn` send only the move and
  leave the tracking as it is (section 2). A script that relied on the switch runs
  `obsbot4linux set tracking.mode=static` first, as its own step;
- the 0.13 command `preset save N` without a name saved the preset as `Preset <N + 1>`.
  `do preset.save slot=N` sends no name (section 2): what the camera shows for such a preset
  is what the state shows, and a script that wants the old names passes `name=…` itself.

### 12.3 Widget

Track D (section 10.3) defines widget 1.0 and the last 0.x release. Widget 1.0 settles
values by `pending` and `rev`; this replaces its own hold of up to 3 s
(obsbot4linux-dux#89). Its `plugin.json` requires obsbot4linux ≥ 1.0. Each control sends
the one request the user made and chains none (section 2): the gimbal controls send
`gimbal.move` or `gimbal.turn` alone, never a tracking switch before them. A user who wants
the tracking off uses the tracking control first. Saving a preset sends no name unless the
user typed one. A control may show a placeholder of its own for a preset whose `name` is
empty, as pure display; it never writes that label to the camera.

### 12.4 Desktop app

The desktop app (`obsbot4linux-desktop`, own repository, own spec) uses `obsbot-proto` (types
and client). Before `v1.0.0` it builds against a local checkout; from then on it depends on
a git tag ≥ `v1.0.0`. Like the widget, it gates every control on the description, takes
values, bounds, step, unit and list limits from it, and shows the support markers.

### 12.5 Versions

All crates carry the workspace version. It follows SemVer for the `obsbot-proto` library API
and for protocol 2: an additive change raises the minor version; a breaking change raises
the protocol number and the major version.

## 13. Later (not 1.0)

- **M4:** the first captured second model (Tiny 3 or Tiny 3 Lite, whichever arrives),
  through 8.7, plus what it shows the seam lacks (contract changes, 10.7).
- **M5:** the device lease for the firmware updater (`busy` is reserved for it); whether the
  updater joins the workspace, and the packaging of its udev rule.
- A command catalogue, only once two real models show duplicated tables.
- Models without a unit in hand and captures are off the roadmap.
- A generated UI (presentation metadata in the description), only with a consumer.
- Live lists per device (1.0 has one list per connection).
- User-given device names, additive, if two identical cameras ever meet.
- More unit hardening (`ProtectSystem`, `PrivateUsers`, `DeviceAllow`), Arch first, each line
  accepted only on the real unit.
- Functions 1.0 does not offer: flipping the image, resetting the start pose, resetting the
  zone area (each only after CAM-11 or CAM-12 shows that it works), uploading a sleep image.
- Network cameras: a possibility only. An async client, if a client needs it.

## 14. What only the camera can settle

Nothing in this list is guessed in the meantime. Each item names the stage that needs it:
G (CS-0), A (CS-1), W4 (CS-2), or M4 onboarding. A stage whose acceptance needs an item
waits for the camera; work that does not need it goes ahead. Every item is answered
without a frame outside the model's send table. CAM-11 and CAM-12 wait for a recording of
the vendor app; nothing is sent for them before it exists, and they gate nothing. CAM-21
and CAM-29 are recorded only if they happen and gate nothing.

**Tiny 2 Lite**

| CAM    | Question                                                                                                                                                                        | Stage                          | What the answer decides                                        |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ | -------------------------------------------------------------- |
| CAM-1  | Does the serial query answer from Linux with 14 characters: awake, asleep, right after plug-in, after the microphone restart, after a factory reset? How long until it answers? | A                              | identity attempts (5.5) in practice                            |
| CAM-2  | The `asleep` value of each setting and action; are writes to `audio.volume` and `focus.*` while asleep accepted and kept after wake?                                            | A                              | `asleep` fields                                                |
| CAM-3  | Is `zoom.factor` ignored while asleep? Is the camera awake at plug-in? How long from wake (or stream start) until the status reports awake?                                     | A                              | `asleep` of `zoom.factor`; documentation                       |
| CAM-4  | During a speed-limited `zoom.move`, does the zoom readback report the target or the current factor?                                                                             | A                              | text of `docs/protocol-2.md`                                   |
| CAM-5  | `settle_ms` per leaf (measured so far: zone tracking 1–3 s, target lock about 1 s)                                                                                              | A                              | `settle_ms` per leaf                                           |
| CAM-6  | Do `hand.move_gimbal` and `led.blink` survive sleep and wake, re-plug, factory reset?                                                                                           | A                              | the `readback:false` lifetime (wake added if they are dropped) |
| CAM-7  | Does a running `gimbal.turn` stop when the camera falls asleep? Does it resume after wake?                                                                                      | A                              | documentation; clients send `gimbal.stop` themselves           |
| CAM-8  | Is the USB audio interface absent on a cold plug with the microphone off? Does a status-block value follow the microphone?                                                      | A                              | U-23                                                           |
| CAM-9  | `factory_reset` while asleep; does the device return under the same `sn:` id?                                                                                                   | W4                             | `asleep` of `factory_reset`; parity row                        |
| CAM-10 | How long do a cold read of all groups and a read of the 5 s tier take?                                                                                                          | A                              | documented `hello` latency; margin to `CLIENT_REPLY_TIMEOUT`   |
| CAM-11 | Can the image be flipped? Only a recording of the vendor app doing it can show how; nothing is sent before.                                                                     | none in 1.0 (see above)        | `image.flip` (additive, after 1.0)                             |
| CAM-12 | Can the start pose and the zone area be reset? Only a recording of the vendor app doing it can show how; nothing is sent before.                                                | none in 1.0 (see above)        | `power_on.reset`, a zone-area reset (additive, after 1.0)      |
| CAM-13 | Order of `exposure.anti_flicker` and `exposure.mode` writes, in both directions                                                                                                 | A                              | apply order (8.8)                                              |
| CAM-14 | After UVC autofocus off, is `focus.mode` `single` or `manual`?                                                                                                                  | A                              | rows `focus auto off`, `image autofocus 0`                     |
| CAM-15 | Are the white-balance temperature and the absolute focus refused while their automatic control is on?                                                                           | A                              | confirms the `requires` rules (kept either way: a kernel rule) |
| CAM-16 | Hand and zone area: a new range wholly beyond the stored one                                                                                                                    | A                              | U-8                                                            |
| CAM-17 | `gimbal.move` with speed near the tilt limit and the speed ceiling; one axis with speed                                                                                         | A                              | U-9                                                            |
| CAM-18 | `preset.delete`: which preset does its second (clearing) frame address? Does preset 0 change? (needs the vendor app as a second writer)                                         | A                              | the frames of `preset.delete`                                  |
| CAM-19 | Preset read: does the list-rewind write have any effect? Can the vendor app's read path (slot list plus one pose query per slot) replace it?                                    | A                              | the preset read; the `touched` tier stays                      |
| CAM-20 | While the camera boots, does the first field of the status block read 0 until it is ready?                                                                                      | A                              | if yes, a not-ready block counts as no read                    |
| CAM-21 | Can the query channel go silent while the status block still reads? Observed only if it happens; nothing is sent to provoke it.                                                 | A, W4                          | a rule that ends a whole read early                            |
| CAM-22 | What does the live-pose query answer while asleep? Do the go-return actions and the pose-taking actions act while asleep?                                                       | A                              | `asleep` of those actions                                      |
| CAM-23 | Every row of 9.3                                                                                                                                                                | A                              | 9.3                                                            |
| CAM-24 | Every row of 9.1 on the real unit: destructive rows last, tracking rows with a person in view                                                                                   | W4                             | release                                                        |
| CAM-25 | The sysfs attributes of the camera's USB device and video nodes while plugged (read only: ids, descriptors, interface classes, node index)                                      | A                              | `enumeration`                                                  |
| CAM-26 | Two cameras at once with stable USB paths                                                                                                                                       | W4 if a second unit is at hand | otherwise the fakes cover it                                   |
| CAM-27 | The 1.0 user unit with `NoNewPrivileges=yes` on Arch: starts, sees the camera, handles hotplug                                                                                  | W4                             | release                                                        |
| CAM-28 | Upgrade rehearsal from the 0.13 desktop state (socket disabled, daemon a child of the widget's `watch`) to 1.0 and widget 1.0, following 12.2                                   | W4                             | migration notes                                                |
| CAM-29 | The kernel's UVC control timeout with an unresponsive camera. Never provoked; recorded if it happens.                                                                           | any session                    | `ANSWER_POLL` margin                                           |
| CAM-30 | Smoke test of stage G: `status`, one setting set and read back, one gimbal move, `preset list`                                                                                  | G                              | release of 0.14.0                                              |
| CAM-31 | Does the camera keep its USB product id when it renames itself or changes mode (#90)?                                                                                           | after 1.0                      | registry entries                                               |
| CAM-32 | The status block of a real Tiny 2: do Tiny 2 and Tiny 2 Lite share the layout? (an unverified hypothesis today)                                                                 | when a Tiny 2 is at hand       | onboarding of the Tiny 2                                       |
| CAM-33 | Do `tracking.speed`, `tracking.profile=headroom` and `framing.*` change the tracking visibly, with a person in view? (Their write and readback are measured, #98.)              | A                              | `no_effect` of those leaves                                    |
| CAM-34 | With the tracking on (`tracking.mode` not `static`), what do `gimbal.move` and `gimbal.turn` do: move and track on, switch the mode, or nothing? (with a person in view)        | A                              | the text of 4.4; `touched` of the moves (8.8)                  |
| CAM-35 | What does the camera report as the name of a preset saved without a name, and does the name reach the state as empty or as a name of its own? (obsbot4linux sends none)         | A                              | the text of 4.4; the placeholder of clients (6.2, 12.3)        |

**Tiny 3 and Tiny 3 Lite** (stage M4, onboarding steps 0–2 and 7 on the unit in hand;
everything here is unknown until captures)

| CAM     | Question                                                                                                      |
| ------- | ------------------------------------------------------------------------------------------------------------- |
| CAM-T1  | USB vendor and product ids of the unit, in every mode it shows; does one model show more than one product id? |
| CAM-T2  | Is there a USB serial string? The serial format.                                                              |
| CAM-T3  | Extension unit id and GUID, control count, buffer size per selector, video nodes, bcdDevice                   |
| CAM-T4  | Interface classes (audio, serial, MTP); what a serial interface carries                                       |
| CAM-T5  | Frame layout of requests and answers                                                                          |
| CAM-T6  | The vendor app's connect sequence and idle poll (they define the identity source and the poll plan)           |
| CAM-T7  | Status block: its length and the meaning of each field (one setting changed per step)                         |
| CAM-T8  | Tracking readback layout; tracking-mode and tracking-speed values                                             |
| CAM-T9  | The settings and actions the vendor app offers for the model, one recording each                              |
| CAM-T10 | Does the identity query answer, also asleep and right after enumeration?                                      |
| CAM-T11 | Which writes does the camera accept and ignore (readback or visible effect per field)?                        |
| CAM-T12 | The auto-sleep "off" encoding; does a timer write disconnect USB? (one third-party report, unverified)        |
| CAM-T13 | `settle_ms` and `asleep` behaviour per field (step 7)                                                         |

## 15. Changes from revision 1

One row per changed decision. **REVERSAL** marks a change to something Thomas approved on
2026-11-01. "Source" names the decision of 2026-11-12 (a letter and number), "writer" for
a choice made while writing this revision from the review or the audit, or "editor" for a
choice made while applying the reviews of this revision; each row can be overruled on its
own. Decisions A1–A3 (`audio.pickup` merged into `audio.volume`, the hidden
`gimbal center` not carried, `gimbal.turn` with `keep`) were confirmed as revision 1 proposed
them and are not listed.

| #   | Topic                                | Revision 1                                                                         | Revision 2                                                                                                                                                                                                                                                                                                                                                                                           | Why                                                                                        | Evidence (status)                                                                                                                                      | Source                                                           |
| --- | ------------------------------------ | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| 1   | **REVERSAL** Protocol typing         | typed `State`, `Patch`, `Action` with per-setting builders                         | typed envelopes and descriptions; state, patch and parameters are path → JSON value, checked at run time (4.1, 6.1)                                                                                                                                                                                                                                                                                  | typed builders need a proto release for every new setting                                  | the widget hard-codes 53 request sites (verified); new models bring settings the Lite lacks (unknown in detail until captures)                         | C1                                                               |
| 2   | "No client change"                   | literal, for all clients                                                           | guarantee A: no code change to proto, client library or CLI while the model fits the existing types, units and constraint forms; descriptions and completions are regenerated as data; a new type or unit is an additive contract change first; hand-made UIs take values and bounds from the description and update only to show new settings (1, 6.2)                                              | a hand-made UI cannot show a setting it does not know                                      | widget code (verified)                                                                                                                                 | C2; the fit condition editor                                     |
| 3   | **REVERSAL** Crates                  | five: proto, client, device, daemon, CLI                                           | four: the client is a module of `obsbot-proto` (3)                                                                                                                                                                                                                                                                                                                                                   | proto and client are always released together                                              | revision 1 dependency graph (verified)                                                                                                                 | C3                                                               |
| 4   | Schema artefacts                     | `schema/protocol-2.json` only; completions "from the schema"                       | protocol schema plus `descriptions/<model>.json`; the CLI embeds the real models' files for completion (3, 6.2)                                                                                                                                                                                                                                                                                      | path completion needs model data and the CLI must not depend on device code                | revision 1 contradiction (verified)                                                                                                                    | C4                                                               |
| 5   | `--device` completion                | completed live while the daemon runs                                               | not completed; Tab never contacts the daemon (6.2)                                                                                                                                                                                                                                                                                                                                                   | lean; no daemon contact from the shell                                                     | none: design choice                                                                                                                                    | writer                                                           |
| 6   | Device entry                         | `id, name, serial, model, firmware, video_node, available, limits{presets,views}`  | adds `support`, `usb` with the descriptor facts, `limits` per list path (4.2)                                                                                                                                                                                                                                                                                                                        | walk-through 2 and onboarding step 1 need them; `devices --report` prints `usb`            | walk-throughs 8.4, 8.7 (verified on paper)                                                                                                             | C5; `usb` facts writer                                           |
| 7   | Description fields                   | type, ranges, `readback`, `asleep`, `no_effect`                                    | adds `requires`, `warning`, `default`, `live`, `settle_ms`, `optional`; bounds per type; closed unit list (4.2)                                                                                                                                                                                                                                                                                      | dependencies, restarts, camera defaults and lag must be visible to generic clients         | 0.13 refuses exposure time unless manual (verified); the widget resets to UVC defaults (verified)                                                      | C6; `warning` on settings writer                                 |
| 8   | Error codes                          | unknown path = `invalid`; `busy` also during restarts                              | adds `conflict`; a path not in the description = `unsupported`; `busy` = queue full only (4.1)                                                                                                                                                                                                                                                                                                       | a request the current state forbids had no code                                            | 0.13 refuses an empty preset slot (verified)                                                                                                           | C7                                                               |
| 9   | **REVERSAL** `epoch`                 | random `epoch` in the `hello` reply                                                | none; after any reconnect a client rebuilds from `hello` (4.1)                                                                                                                                                                                                                                                                                                                                       | it had no correct use                                                                      | none: design choice (a reconnect always rebuilds from `hello`)                                                                                         | C8                                                               |
| 10  | Live list                            | `live:["pose"]`, pose only                                                         | live topics named in the description (`pose`, `target`, `auto`); volatile-only changes go only to subscribers (4.1 rule 8)                                                                                                                                                                                                                                                                           | auto-adjusted values would send an event every second                                      | 0.13 already suppresses them (verified)                                                                                                                | C9                                                               |
| 11  | Serial in output                     | `--json` shortens the id; `devices --show-serial`                                  | `--json` and `watch` carry the full id; human output and logs shorten; `--show-serial` dropped (4.2, 6.2)                                                                                                                                                                                                                                                                                            | a shortened id cannot be passed back; one contract                                         | revision 1 contradiction (verified)                                                                                                                    | C10; `--show-serial` writer                                      |
| 12  | `--device` matching                  | name first, then id                                                                | full id, a unique serial suffix of ≥ 4 characters, or a unique model key (6.2)                                                                                                                                                                                                                                                                                                                       | names are gone                                                                             | none: follows from row 15                                                                                                                              | C10                                                              |
| 13  | **REVERSAL** Start mode "not set"    | `power_on.start.mode` nullable, `none` = null                                      | the enum value `unset` (4.3)                                                                                                                                                                                                                                                                                                                                                                         | null now means "not read"                                                                  | the widget shows "Not set" for an unread start mode (verified)                                                                                         | C11                                                              |
| 14  | **REVERSAL** `gimbal.step`           | an action for widget parity                                                        | dropped; the widget computes the target and sends `gimbal.move` (4.4)                                                                                                                                                                                                                                                                                                                                | the camera has no relative move; the base pose was undefined                               | the widget steps from its last target (verified)                                                                                                       | C11                                                              |
| 15  | **REVERSAL** Names                   | `device.rename`, `names.json`                                                      | none; the daemon keeps no file (2, 5.10)                                                                                                                                                                                                                                                                                                                                                             | serves two identical cameras nobody has                                                    | none: design choice                                                                                                                                    | C12                                                              |
| 16  | `image.reset`                        | an action for the widget's reset                                                   | dropped; the widget sends a patch of `default` values (4.4, 10.3 D)                                                                                                                                                                                                                                                                                                                                  | no camera command; which controls to reset is a product choice                             | the vendor app resets other controls than the widget (captured, capture 12)                                                                            | writer                                                           |
| 17  | **REVERSAL** Hold timers             | "there are no hold timers"                                                         | `pending` (path → written value) with a per-leaf `settle_ms`, checked at poll ticks (4.1 rule 4)                                                                                                                                                                                                                                                                                                     | without it the old value flickers back after writes; the wait moves into one place         | readbacks lag 1–3 s (measured, #81); target lock about 1 s (measured)                                                                                  | D2, J2                                                           |
| 18  | Write path                           | read back "the affected group"; full read after each command                       | re-read only the touched groups; a batch shares one re-read; single-leaf coalescing (4.1 rule 9, 5.6)                                                                                                                                                                                                                                                                                                | a slider drag queued behind a full read per step                                           | 0.13.2 reads everything (about 1.4 s) after every command (verified)                                                                                   | D1                                                               |
| 19  | Stops                                | no rule; replies strictly in order                                                 | stops jump the queue and end a running group read (4.1 rule 2)                                                                                                                                                                                                                                                                                                                                       | a stop waited behind a long read                                                           | 0.13.2 behaviour (verified)                                                                                                                            | D8                                                               |
| 20  | Deadlines                            | "the transport timeout carried over"                                               | named timeouts; 15 s request deadline; bounded outbound queue; at most 64 connections (5.8, 4.1)                                                                                                                                                                                                                                                                                                     | unbounded waits; a slow subscriber stalled replies                                         | 0.13.2 timeouts (verified)                                                                                                                             | D9; 64 connections writer                                        |
| 21  | Asleep writes                        | `unknown` passed through                                                           | `unknown` written; a path whose readback cannot run asleep stays pending until wake (5.7)                                                                                                                                                                                                                                                                                                            | the write could never be confirmed                                                         | volume and focus readbacks do not answer asleep (0.13.2 behaviour, verified)                                                                           | D3                                                               |
| 22  | Identity                             | 3 tries over about 10 s, also after wake; `port:` entries kept                     | first attempt at plug-in also asleep; attempts 2–3 with the next two reads of a tier the model names; if asleep at plug-in, the first series ends and one series runs from the first wake; serial format check; duplicate → `port:`; `port:` entries removed on unplug; a series ends once serial and firmware are answered, and an attempt sends only the missing queries (5.5)                     | bounded, tied to existing reads; an id changes at most once                                | serial query asleep: unknown (CAM-1)                                                                                                                   | D4, J4; duplicates and `port:` removal writer; series end editor |
| 23  | Unknown devices                      | undefined; "read model and serial" for every device                                | listed `unsupported`, nothing sent, `devices --report` (5.4, 8.4)                                                                                                                                                                                                                                                                                                                                    | any frame to an unknown device is a probe                                                  | a blind query scan broke the query channel once (measured)                                                                                             | D5                                                               |
| 24  | **REVERSAL** Hotplug                 | a udev monitor                                                                     | a sysfs scan once a second, no libudev, no netlink (5.3)                                                                                                                                                                                                                                                                                                                                             | no C dependency; 0.13 already rescans every second                                         | 0.13.2 daemon (verified)                                                                                                                               | D6                                                               |
| 25  | Start-up                             | undefined                                                                          | `hello` answered after the first enumeration; a device announced after its first read (5.2)                                                                                                                                                                                                                                                                                                          | a socket-activated `status` saw no camera                                                  | revision 1 gap (verified)                                                                                                                              | D7                                                               |
| 26  | Single instance                      | lock at `$XDG_RUNTIME_DIR/obsbot4linux.lock`; turn stops on shutdown               | lock at `<socket path>.lock` for every daemon; a clean shutdown stops a turn best effort; a crash leaves the camera (5.9)                                                                                                                                                                                                                                                                            | test daemons collided; the 0.13 daemon uses the same lock name                             | 0.13.2 lock (verified)                                                                                                                                 | D10                                                              |
| 27  | **REVERSAL** Hardening               | `NoNewPrivileges`, `ProtectSystem`, `DeviceAllow`, tested on Arch and Ubuntu 24.04 | `NoNewPrivileges=yes` only; the rest after 1.0 (5.10)                                                                                                                                                                                                                                                                                                                                                | independent of the rebuild and costly to accept                                            | in user units these lines imply `PrivateUsers` (verified, systemd documentation); no run on the real unit yet (verified)                               | D11                                                              |
| 28  | **REVERSAL** Mutation tests          | a stage-5 gate on device and proto                                                 | one pass over the carried code in stage G; no 1.0 gate (10.2, 11)                                                                                                                                                                                                                                                                                                                                    | they test carried code, which moves first                                                  | seven sign-flip mutants survive in the carried code (verified)                                                                                         | D11                                                              |
| 29  | Daemon start                         | "nothing needs re-enabling"                                                        | socket only; packages enable nothing; exit 4 prints the enable line; an older daemon is detected (5.10, 12.2)                                                                                                                                                                                                                                                                                        | default widget setups run the daemon as a child of `watch` with the socket disabled        | the owner's desktop (verified)                                                                                                                         | D12                                                              |
| 30  | 0.13 state files                     | the daemon deletes `state.json`; the guide promised an import                      | the daemon touches no file; the migration notes say the directory can be deleted (12.2)                                                                                                                                                                                                                                                                                                              | "keeps no file at all"; no import exists                                                   | revision 1 contradiction (verified)                                                                                                                    | writer (C12)                                                     |
| 31  | Never-send                           | a deny-list of two ids                                                             | a per-model send table checked in the transport, plus refused classes; no raw path (8.3)                                                                                                                                                                                                                                                                                                             | a deny-list protects only against the ids it names                                         | verified                                                                                                                                               | E1                                                               |
| 32  | Public text                          | text that 8.5 does not allow                                                       | only what 8.5 allows; evidence by level and opaque key                                                                                                                                                                                                                                                                                                                                               | the repository is public                                                                   | verified                                                                                                                                               | E2, J8                                                           |
| 33  | **REVERSAL** Fakes                   | `--fake N`: N identical Tiny 2 Lites with test-only actions                        | `fake-a`, `fake-b` (deliberately different), `unknown`; test control on a second socket (5.12)                                                                                                                                                                                                                                                                                                       | mixed models and unknown devices were never tested                                         | revision 1's fakes cannot exercise mixed models or an unknown device (verified)                                                                        | F1–F3                                                            |
| 34  | **REVERSAL** M4 and the catalogue    | M4 = TOML catalogue plus rudimentary support for further models without captures   | no catalogue; models only with a unit and captures; M4 = the first captured second model (8.7, 13)                                                                                                                                                                                                                                                                                                   | "common commands" cannot be assumed                                                        | none: design choice (a model only with a unit and captures); identity-query loops linked to USB drops on a Tiny 3 (one third-party report, unverified) | B3, B4, B5, J7                                                   |
| 35  | Tiny 3 family                        | –                                                                                  | two models, two modules; nothing built before a unit is at hand; every device fact unknown until captures (8.9)                                                                                                                                                                                                                                                                                      | nothing is known about either device                                                       | none: design choice; every device fact unknown until captures                                                                                          | B5, J1                                                           |
| 36  | Registry key                         | "its USB id"                                                                       | USB (vendor id, product id) pairs, several per model; a vendor table of measured ids only (8.1)                                                                                                                                                                                                                                                                                                      | a model may have several product ids                                                       | none: design choice                                                                                                                                    | B6                                                               |
| 37  | Extension unit                       | unit 2 hard-coded                                                                  | found by GUID in the USB descriptor, read-only (8.3)                                                                                                                                                                                                                                                                                                                                                 | no per-model constant, no request                                                          | the descriptor holds unit and GUID (captured, capture 01)                                                                                              | B7                                                               |
| 38  | Model seam                           | one sentence; the daemon section named Lite ids and tiers                          | three layers in `obsbot-device`; a path-keyed `trait Camera`; the daemon names no id (3.1, 5, 8.2)                                                                                                                                                                                                                                                                                                   | per-model data plus per-model flows; a daemon that knows the Lite cannot run another model | about 83 family-generic and 64 Lite-specific knowledge items in the carried code (verified count)                                                      | B1, B2                                                           |
| 39  | Live pose query                      | the poll tier named the query that arms the zone switch                            | the model uses `0x0043`; the daemon names no query (8.8)                                                                                                                                                                                                                                                                                                                                             | that query arms the switch to static after a zone return                                   | #67 (measured)                                                                                                                                         | B2 (writer)                                                      |
| 40  | **REVERSAL** (partial) Delivery      | a long-lived `rebuild` branch, five serial stages                                  | stage G first (released as 0.14.0 from `main`); W0, four parallel tracks, W2, W3, W4 (10)                                                                                                                                                                                                                                                                                                            | 64 % of the lines are carried; fixes would land twice; packaging first ran on the tag      | line count (verified)                                                                                                                                  | H1, H2, J5                                                       |
| 41  | CI on `rebuild`                      | assumed                                                                            | `ci.yml` and a ruleset for `rebuild` before its first PR (10.1)                                                                                                                                                                                                                                                                                                                                      | PRs into `rebuild` ran no CI                                                               | GitHub settings (verified)                                                                                                                             | J5                                                               |
| 42  | Widget channel                       | undefined; `watch` relayed requests in 0.13                                        | the widget speaks protocol 2 on the socket; `watch` prints events; `relay` as fallback (6.2, 10.3 D)                                                                                                                                                                                                                                                                                                 | one contract; no protocol-1 adapter                                                        | a socket type in the shell's runtime: unknown                                                                                                          | H3                                                               |
| 43  | Release day                          | widget and tool the same day                                                       | widget 0.28.0 with an upper bound first; a release candidate; a "done" list (10.6)                                                                                                                                                                                                                                                                                                                   | widget 0.27 would accept 1.0 and fail at run time                                          | widget start-up check (verified)                                                                                                                       | H4                                                               |
| 44  | Milestones                           | 0.14.0–0.16.0 on `main` as planned on 2026-10-31                                   | a proposal: 0.14.0 = groundwork, #73 and #89 into 1.0, 0.15.0 and 0.16.0 after 1.0 (10.5)                                                                                                                                                                                                                                                                                                            | the groundwork release carries the carried-code fixes                                      | none: planning choice                                                                                                                                  | H5, J6                                                           |
| 45  | Desktop app dependency               | git tag; shell starts after stage 1                                                | a local checkout before `v1.0.0`, a git tag ≥ `v1.0.0` after; SemVer rules (12.4, 12.5)                                                                                                                                                                                                                                                                                                              | no tag exists before 1.0                                                                   | no tag exists before `v1.0.0` (verified)                                                                                                               | H6; checkout writer                                              |
| 46  | Evidence rule                        | "every function of 0.13 stays"                                                     | only backed fields are offered; unbacked Lite fields are checked or become named exceptions (9.3)                                                                                                                                                                                                                                                                                                    | the two rules collided for fields without backing                                          | ledger levels (verified)                                                                                                                               | J3                                                               |
| 47  | Parity script                        | `check-parity.sh`, set → read back → restore for every row                         | `check-model.sh` from the description, `--device`, destructive rows last, restore by trap (9.4)                                                                                                                                                                                                                                                                                                      | not every row can be restored; a second model has no 0.13 rows                             | 0.13's readback script restores incompletely (verified)                                                                                                | writer                                                           |
| 48  | `image.gamma`                        | a path, "incl. gamma"                                                              | removed; PX-2                                                                                                                                                                                                                                                                                                                                                                                        | the camera has no gamma control                                                            | the recorded UVC descriptor (captured)                                                                                                                 | writer                                                           |
| 49  | Exposure compensation                | `image.backlight_compensation`; "a guess"                                          | `exposure.compensation`, the raw value 0–18, default 9; EV only after U-20                                                                                                                                                                                                                                                                                                                           | the vendor app's exposure compensation writes this control; its scale is inferred          | captured, capture 12; the 1/3 EV step unverified                                                                                                       | writer                                                           |
| 50  | Anti-flicker                         | `auto` writable                                                                    | `auto` read-only; no `default` (4.3); the value `auto` itself is U-25                                                                                                                                                                                                                                                                                                                                | the kernel driver offers 0–2 on this camera                                                | kernel driver (verified); the camera reporting `auto`: never recorded (U-25)                                                                           | writer                                                           |
| 51  | Sleep background                     | `black\|image`                                                                     | values add `own_image` (read-only)                                                                                                                                                                                                                                                                                                                                                                   | the camera reports it after an upload                                                      | captured (capture 18)                                                                                                                                  | writer                                                           |
| 52  | `gimbal.move`                        | `pan` and `tilt` required                                                          | either may be missing; the camera keeps that axis (4.4)                                                                                                                                                                                                                                                                                                                                              | a pan-only move pulled a stale tilt                                                        | 0.13.2 behaviour (verified); the keep value (captured)                                                                                                 | writer                                                           |
| 53  | Current pose                         | only `preset.save` and `view.save` named the live pose                             | every pose-taking action uses the live pose and is refused while asleep until CAM-22 (4.4)                                                                                                                                                                                                                                                                                                           | 0.13 used a remembered or V4L2 pose                                                        | 0.13.2 code (verified)                                                                                                                                 | writer                                                           |
| 54  | Target box                           | layout an open point; read every second                                            | layout settled; box changes go to the live topic `target` (4.3)                                                                                                                                                                                                                                                                                                                                      | the layout was checked over the preview                                                    | #71 (measured)                                                                                                                                         | writer, C9                                                       |
| 55  | Preset read                          | in the 60 s tier                                                                   | at plug-in and after the actions that change presets (`touched`, 8.8)                                                                                                                                                                                                                                                                                                                                | the read writes to the camera; the vendor app never does                                   | captured (no such write in any recording)                                                                                                              | writer                                                           |
| 56  | `do wake`                            | replies once the status reports awake                                              | replies on acceptance; the state shows `asleep` (4.4)                                                                                                                                                                                                                                                                                                                                                | an unbounded wait in the worker                                                            | wake time unknown (CAM-3)                                                                                                                              | writer                                                           |
| 57  | Null and unread                      | `null` only for `nullable`                                                         | `unread` leaf paths, `raw:<n>` values, `readback:false` lifetime per availability (4.2)                                                                                                                                                                                                                                                                                                              | "not set" and "not read" were indistinguishable                                            | 0.13 widget shows "Not set" for unread values (verified)                                                                                               | writer                                                           |
| 58  | `audio.enabled`                      | no readback; `busy` during the restart after "off"                                 | readback from the USB audio interface; restart on both values; reply on acceptance without a re-read, the restart is hotplug (4.3, 5.6)                                                                                                                                                                                                                                                              | the restart is hotplug; a sysfs fact is camera state                                       | the interface follows the toggle (measured); restart on both values (measured)                                                                         | writer, C7                                                       |
| 59  | Value details                        | zoom-move speed a raw integer; exposure time "steps in 100 µs"                     | zoom-move speed `slow\|medium\|fast`; no step promise for exposure time; zoom ratio precision 0.1; `sleep.auto.seconds` 1–32767; values rounded at the source                                                                                                                                                                                                                                        | only known values are offered; the readback limits precision                               | 0.13.2 code (verified)                                                                                                                                 | writer                                                           |
| 60  | Write order                          | anti-flicker before manual, citing a 0.13 function                                 | `requires` paths first; tracking mode before framing; the anti-flicker order is CAM-13 (8.8)                                                                                                                                                                                                                                                                                                         | the cited function has no such step                                                        | 0.13.2 code (verified)                                                                                                                                 | writer                                                           |
| 61  | On air                               | `{active, apps[]}`                                                                 | one sample; a client's own preview counts; kernel note (4.3)                                                                                                                                                                                                                                                                                                                                         | 0.13 read the two at different moments                                                     | 0.13.2 code and kernel source (verified)                                                                                                               | writer                                                           |
| 62  | `available`                          | from hotplug                                                                       | only from enumeration; a failed read never flips it (5.3)                                                                                                                                                                                                                                                                                                                                            | 0.13 reported "camera gone" for one failed tick                                            | 0.13.2 daemon (verified)                                                                                                                               | writer                                                           |
| 63  | Poll intervals                       | carried from 0.13                                                                  | counted from the end of the previous read (5.7)                                                                                                                                                                                                                                                                                                                                                      | a slow read starved the 1 s tier                                                           | 0.13.2 daemon (verified)                                                                                                                               | writer                                                           |
| 64  | `view.save` name                     | optional                                                                           | required, as in 0.13 (4.4)                                                                                                                                                                                                                                                                                                                                                                           | 0.13 requires it; the camera's default is not recorded                                     | 0.13.2 code (verified)                                                                                                                                 | writer                                                           |
| 65  | Untested models                      | marked UNTESTED in docs                                                            | a `support` field; no destructive actions; never called supported (8.6)                                                                                                                                                                                                                                                                                                                              | a field was missing                                                                        | none: design choice                                                                                                                                    | C5, writer                                                       |
| 66  | `--json` errors                      | not defined for usage errors                                                       | `{"error":{"code","message"}}` on stdout, exit 2 (6.2)                                                                                                                                                                                                                                                                                                                                               | clap printed text                                                                          | 0.13.2 CLI (verified)                                                                                                                                  | writer                                                           |
| 67  | Licence note                         | names the project the code came from                                               | a general rule: the EUPL note follows the code into every file (3.3)                                                                                                                                                                                                                                                                                                                                 | split and copied code must keep it                                                         | none: licence obligation                                                                                                                               | writer                                                           |
| 68  | Unsupported devices on the socket    | undefined                                                                          | `unsupported_device` for `set` and `do`; the reader thread answers requests for unsupported and unavailable devices at once; the CLI refuses before sending; CLI-only error codes with fixed exits (4.1, 5.4, 6.2)                                                                                                                                                                                   | one code per situation; no worker for a device that receives nothing                       | none: design choice                                                                                                                                    | editor                                                           |
| 69  | `hello` deadline                     | `hello` after the first enumeration                                                | `hello` after the first read attempts, at the latest after 15 s (`REQUEST_DEADLINE`); late devices `unread` (4.1, 5.2)                                                                                                                                                                                                                                                                               | a slow cold read must not make the CLI report a lost daemon                                | cold read time unknown (CAM-10)                                                                                                                        | editor (D7)                                                      |
| 70  | Hand-over events                     | one `devices` event with `replaced_by`                                             | two `devices` events, then a `state` event; rev continues above both entries; `pending` and `unread` carry over (5.5)                                                                                                                                                                                                                                                                                | clients compare revs per id; the old rule left the sequence open                           | none: design choice                                                                                                                                    | editor                                                           |
| 71  | `warning`                            | one value                                                                          | a list; `factory_reset` holds `destructive` and `restart`; every `restart` write replies on acceptance (4.2, 5.6)                                                                                                                                                                                                                                                                                    | the daemon must know which writes restart the camera                                       | factory reset and microphone restart (measured)                                                                                                        | editor (C6, C7)                                                  |
| 72  | Zoom kind                            | `kind`, `optical_max` on `zoom.factor`                                             | dropped; added when a model with optical zoom arrives (4.2)                                                                                                                                                                                                                                                                                                                                          | no 1.0 model uses them                                                                     | none: design choice                                                                                                                                    | editor                                                           |
| 73  | Pending reads                        | –                                                                                  | no extra reads while a path is pending; a writable leaf with a readback is in a periodic group (5.6, 5.7)                                                                                                                                                                                                                                                                                            | decision D2 rules out an extra polling loop                                                | none: design choice                                                                                                                                    | editor (D2)                                                      |
| 74  | Evidence levels                      | per field                                                                          | per field, value and bound; the level test covers untested models too (8.5)                                                                                                                                                                                                                                                                                                                          | the Tiny 2 Lite needed evidence per value (U-12, U-18, U-25, U-26)                         | field ledger (verified)                                                                                                                                | editor                                                           |
| 75  | Owners                               | a model from an owner's captures, no procedure                                     | owners record steps 1–3, send recordings privately, may run step 7 on a dev build; their results count as `measured`; only the maintainers' step 7 makes `tested` (8.4, 8.7)                                                                                                                                                                                                                         | walk-through 2 ended without a route for users who own a unit                              | none: design choice                                                                                                                                    | editor (B4, G)                                                   |
| 76  | Model query                          | sent with identity                                                                 | not sent: no field uses its answer (8.8)                                                                                                                                                                                                                                                                                                                                                             | send only what is used                                                                     | none: design choice                                                                                                                                    | editor                                                           |
| 77  | After 1.0                            | –                                                                                  | process, versions and releases after 1.0 (10.7)                                                                                                                                                                                                                                                                                                                                                      | section 10 covered only the rebuild                                                        | none: design choice                                                                                                                                    | editor                                                           |
| 78  | Paths across models                  | "one meaning per path"                                                             | a path dictionary in `docs/protocol-2.md` with a naming rule (4.2)                                                                                                                                                                                                                                                                                                                                   | reuse of a path could not be checked                                                       | none: design choice                                                                                                                                    | editor                                                           |
| 79  | Re-enumeration                       | sysfs entry gone                                                                   | USB path plus device number; a change is unplug plus plug (5.3)                                                                                                                                                                                                                                                                                                                                      | a re-enumeration within one scan interval was invisible                                    | none: design choice                                                                                                                                    | editor                                                           |
| 80  | Check script                         | set → read back → restore                                                          | adds lag, asleep and confirmation rows; `settle_ms` and `asleep` come from it (9.4)                                                                                                                                                                                                                                                                                                                  | step 7 set these without a procedure                                                       | none: design choice                                                                                                                                    | editor (D2)                                                      |
| 81  | Extension unit per model             | one family selector set and GUID                                                   | GUID and selectors in the registry entry; the class check uses them (8.2, 8.3)                                                                                                                                                                                                                                                                                                                       | a second model may use another unit or other selectors                                     | unknown until captures                                                                                                                                 | editor (B7)                                                      |
| 82  | Worker end                           | –                                                                                  | when a device becomes unavailable its worker ends: a started request gets `camera` without `rev`, queued requests are answered as for an unavailable device (`unknown_device` for a removed `port:` entry), a re-plug starts with an empty queue; a `restart` request runs as its own batch, and after its frame the worker sends nothing, stops included, until the device goes, at most 15 s (5.6) | the fate of the queue at unplug and during a restart was open                              | none: design choice                                                                                                                                    | editor                                                           |
| 83  | `rev` of a re-added `port:` id       | –                                                                                  | `rev` is per entry: a `port:` entry added again starts at 1; its removal is always its own `devices` event; clients forget an id's rev when a `devices` event drops it (4.1)                                                                                                                                                                                                                         | clients that kept the highest rev per id dropped the new entry's states                    | none: design choice                                                                                                                                    | editor (row 70)                                                  |
| 84  | Reads at wake                        | –                                                                                  | at wake every group with an `unread` leaf is read once, `touched` groups included; a failed `touched` read is retried at the next wake or 60 s read; slot and id actions are refused with `conflict` while their list is unread (4.4, 5.7)                                                                                                                                                           | a camera asleep at start-up left `presets` unread until a preset action                    | none: design choice                                                                                                                                    | editor (D3)                                                      |
| 85  | On-air level                         | ledger level `measured`                                                            | level `daemon`: a Linux fact, given to every supported model, outside the vendor-app rate rule (8.5, 8.7)                                                                                                                                                                                                                                                                                            | it cannot be captured, and no check row measures a read-only leaf                          | the probe on the Tiny 2 Lite (measured, 2026-10-25)                                                                                                    | editor                                                           |
| 86  | Destructive actions in onboarding    | –                                                                                  | the maintainers' step 7 adds the captured destructive actions and runs `--destructive`; an owner's run leaves it out; a released `untested` description holds none (8.6, 8.7)                                                                                                                                                                                                                        | a model could become `tested` without its restart and destructive rows                     | none: design choice                                                                                                                                    | editor                                                           |
| 87  | Readbacks the vendor app never polls | –                                                                                  | a writable leaf whose captured readback the idle poll never reads takes the 60 s tier (8.7 step 4)                                                                                                                                                                                                                                                                                                   | 5.7 needs a periodic tier for `pending` to clear                                           | none: design choice                                                                                                                                    | editor (D2)                                                      |

### 15.1 Changes after approval

Revision 2 was approved on 2026-11-13. Each later change has one row; each can be overruled
on its own.

| #   | Date       | Topic                                          | Approved revision 2                                                                                                             | Now                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Why                                                                                                                                                                       | Source                                  |
| --- | ---------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| 88  | 2026-11-13 | An action sends exactly what was asked         | `gimbal.move` and `gimbal.turn` switch `tracking.mode` to `static` before they move, carried over from 0.13                     | they send only the move and leave `tracking.mode` alone; section 2 states the rule for every request and names the two kinds of frames that belong to one; the camera's answer to a manual move while it tracks is CAM-34; clients chain no request; the carried flows lose the switch (2, 3.2, 4.4, 5.6, 6.2, 8.2, 8.3, 8.8, 9.1, 10.3, 12.2, 12.3, 14); new tests: 8.3 the exact frames of each ledger row; 10.3 the W0 fake test, the track B, C and D checks | Thomas asked to keep the daemon free of logic of its own; the carried-over switch was such logic and contradicted section 2, so it is gone                                | Thomas, 2026-11-13                      |
| 89  | 2026-11-13 | Stops the daemon sends on its own              | the daemon stops a running turn when the camera falls asleep, and best effort on a clean shutdown (also for a `keep:true` turn) | the only stop the daemon sends is the end a turn's request set: a turn without `keep:true` stops when its connection closes, a shutdown included; a `keep:true` turn goes on across a shutdown, and across sleep the camera decides (CAM-7) (2, 4.4, 5.6, 5.9, 10.3, 14)                                                                                                                                                                                         | the client that started a turn can still end it, and `gimbal.stop` works from any connection and after a restart, so these stops were logic of our own; 0.13 sent neither | Thomas, 2026-11-13 (the rule of row 88) |
| 90  | 2026-11-13 | Presets saved without a name                   | `preset.save` without `name` names the preset `Preset <slot + 1>` (0.13.2 behaviour)                                            | it sends the preset and no name, no rename frame follows; what the camera reports as the name of an unnamed preset is CAM-35; a client may show its own placeholder for an empty name as display only and never writes it to the camera (2, 4.3, 4.4, 6.2, 9.1, 10.3, 12.2, 12.3, 14); new test: 10.3 the W0 fake test for `preset.save`                                                                                                                         | a name the daemon invents is a write the user did not ask for and contradicts section 2; the camera can show an unnamed preset by itself                                  | the rule of row 88, applied 2026-11-13  |
| 91  | 2026-11-13 | Presets are re-read less often                 | the presets are read at plug-in and after `preset.*` and `power_on.use_preset`                                                  | read at plug-in and after `preset.save`, `preset.rename` and `preset.delete` only; a recall or `power_on.use_preset` does not change the list (8.8)                                                                                                                                                                                                                                                                                                              | a recall adds no preset, and every read of the list is a write of the rewind (section 2 exception, CAM-19)                                                                | the W1 plan review                      |
| 92  | 2026-11-13 | A stop is accepted when the queue is full      | a stop is taken before every queued request; 32 queued requests make the next one `busy`                                        | a stop (`gimbal.stop`, `zoom.stop`) is accepted when 32 requests are queued; every other request is still `busy` (4.1 rule 2, 5.6)                                                                                                                                                                                                                                                                                                                               | a user who floods a device and then presses stop would otherwise get `busy` for the stop                                                                                  | the W1 plan review                      |
| 93  | 2026-11-13 | A pending value counts as current              | a `set` is validated against the current state with the patch applied                                                           | a value in `pending` counts as the current value for `requires` and for the asleep check, also across batches (4.1 rule 5, 4.2); clients that gate controls do the same                                                                                                                                                                                                                                                                                          | `set exposure.mode=manual` and then `set exposure.time=…` must work however the two requests are timed                                                                    | the W1 plan review                      |
| 94  | 2026-11-13 | Duplicate serial: which device is the newcomer | for two serials of one enumeration pass, the one later in the order of the scan                                                 | the one whose serial reaches the device table later (5.5)                                                                                                                                                                                                                                                                                                                                                                                                        | the workers read in parallel, so the order of the scan does not decide and cannot be tested                                                                               | the W1 plan review                      |
| 95  | 2026-11-13 | The command line does not prompt               | "Generic UIs ask before they send either" (4.2)                                                                                 | the command line does not prompt: scripts use it and a destructive action needs `confirm=true`; interactive UIs such as the widget ask (4.2)                                                                                                                                                                                                                                                                                                                     | the sentence read as if the command line had to ask as well                                                                                                               | the W1 plan review                      |
| 96  | 2026-11-13 | `Client::subscribe` returns the hello reply    | `Client::subscribe` returns a sender and a receiver                                                                             | it returns the `hello` reply, a sender and a receiver (6.1)                                                                                                                                                                                                                                                                                                                                                                                                      | the caller needs the device list and the states from the reply, and a refusal or a protocol mismatch is reported by the call itself                                       | the W1 plan review                      |
| 97  | 2026-11-13 | `--device` takes the shortened id              | `--device` takes the full id, a serial suffix of at least four characters, or a model key                                       | the shortened id that human output prints (`…0001`) is accepted as well (6.2)                                                                                                                                                                                                                                                                                                                                                                                    | a person pastes what the command line printed                                                                                                                             | the W1 plan review                      |
| 98  | 2026-11-13 | The last 0.x widget release is 0.29.2          | widget 0.28.0 is the last 0.x release and carries the upper bound                                                               | 0.28.0 to 0.29.1 are released and carry the refusal; 0.29.2 is the last 0.x release and adds the bound `< 1.0` to `plugin.json` (10.3, 10.6)                                                                                                                                                                                                                                                                                                                     | 0.28.0 was released before this change; later 0.x releases exist                                                                                                          | the W1 plan review                      |
| 99  | 2026-11-13 | Widget 1.0 is built on a branch                | widget 1.0 is built in the widget repository, merged on release day                                                             | built on a branch `v1` of the widget repository and merged into the default branch by fast-forward on release day (10.3, 10.6)                                                                                                                                                                                                                                                                                                                                   | `dms plugins update` follows the default branch, so a 1.0 commit there would reach users of 0.13                                                                          | the W1 plan review                      |
| 100 | 2026-11-13 | B-1 is a milestone                             | track B in two PRs, B-1 and B-2                                                                                                 | B-1 is reached when the last task of the daemon core with fake mode has landed; it is built in several pull requests (10.3 B)                                                                                                                                                                                                                                                                                                                                    | the daemon core is too large for one reviewable pull request                                                                                                              | the W1 plan review                      |
