# obsbot4linux protocol 2

The protocol between the obsbot4linux daemon (`obsbot4linuxd`) and its clients: the CLI, the
widget and the desktop app. This part is normative. The machine-readable form is
`schema/protocol-2.json`; the Rust types are the crate `obsbot-proto`; the golden transcripts in
`crates/obsbot-proto/tests/golden/` show every message. Sections 11 and 12 are not normative:
worked transcripts, lifecycles and the client library in use.

## 1. Transport

- A Unix socket at `$OBSBOT4LINUX_SOCKET`, else `$XDG_RUNTIME_DIR/obsbot4linux.sock`, mode 0600.
- One JSON object per line, UTF-8. A request line longer than 64 KiB closes the connection.
  Lines from the daemon have no length limit; clients must not cap them.
- At most 64 connections; a further connection is closed at once.
- The first message on a connection is `hello`. Requests sent after `hello` and before its
  reply are kept and handled after the reply.
- A client may shut down only its sending side after its last request (a half-close): the
  daemon still handles and answers every request it read, then closes the connection. Only
  a close of the whole connection drops requests not started (design spec 4.1, rule 9).

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
   An action sends exactly what it names, nothing before and nothing after it: the daemon
   sends no `tracking.mode` write with a `gimbal.move`; what the camera then does with its
   tracking shows in the state. A client that wants the tracking off first sets
   `tracking.mode` to `static` in its own request.
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
  camera's ids. A preset's `name` is what the camera reports, empty included: the daemon
  names no preset (`preset.save` without `name` sends no name).

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

Paths of the test models only (fake mode): `beacon.color` (enum) and `beacon.pulse` (bool) of
`fake-b`, a group no client knows.

Actions: `gimbal.move`, `gimbal.stop`, `gimbal.turn`, `zoom.move`, `zoom.stop`, `preset.save`,
`preset.recall`, `preset.rename`, `preset.delete`, `view.save`, `view.go`, `view.rename`,
`view.delete`, `zone.return_here`, `zone.go_return`, `hand.return_here`, `hand.go_return`,
`hand.return_clear`, `hand.area_reset`, `power_on.set_pose`,
`power_on.use_current`, `power_on.use_preset`, `power_on.go`, `sleep`, `wake`,
`factory_reset`; `beacon.flash` of `fake-b`. Their parameters are in each model's description.

## 10. Live topics

| Topic    | Leaves (Tiny 2 Lite)                                                |
| -------- | ------------------------------------------------------------------- |
| `pose`   | `gimbal.pan`, `gimbal.tilt`                                         |
| `target` | `tracking.target.box`                                               |
| `auto`   | `exposure.time`, `exposure.gain`, `image.white_balance.temperature` |

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
half-close ──► every request read is answered ──► the daemon closes
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
