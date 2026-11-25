# Changelog

All notable changes to OBSBot4Linux. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
  0 ok, 1 internal error, 2 invalid input, 3 the device cannot do it now or there is no usable
  camera, 4 the daemon cannot be reached; bash and zsh completions.
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
  `do preset.save slot=N` sends no name: what the camera shows for such a preset is what
  `get presets` shows, and a script that wants the old names passes `name=…` itself.
- No command starts a daemon any more; without the socket unit every command exits 4 and
  prints `systemctl --user enable --now obsbot4linux.socket`.
- Tiny 2 Lite, from measurements on the camera: a `set` with both writes
  `exposure.anti_flicker` before `exposure.mode` (written after manual exposure, the camera
  dropped the anti-flicker change); `exposure.time` takes multiples of 100 µs, the camera's
  step, and refuses other values instead of rounding them down; a `focus.position` that the
  kernel refuses while the continuous autofocus is on fails as the camera's refusal with that
  reason, not as an I/O error, and `focus.mode=manual` switches the UVC continuous autofocus
  off (`continuous` on) so that the kernel takes `focus.position`; `do gimbal.move` with a
  `speed` takes speeds up to 100 °/s and tilts up to 80 (faster or higher, the camera did not
  move at all).
- Tiny 2 Lite, from camera session 1: the settings the camera keeps while it sleeps are marked
  `asleep: accepted`, `zoom.factor` `ignored`; `exposure.anti_flicker` no longer lists `auto`
  and `power_on.start.mode` no longer `lower_body` and `hand` (never read from the camera;
  such a value shows as `raw:<n>`); a pose-taking action refused for the pose it read names
  that pose (#328).

### Removed

- `obsbot4linux daemon`, `--direct`, `detect`, request forwarding in `watch`, and the files in
  `$XDG_STATE_HOME/obsbot4linux/` (see the exceptions below).
- `hand pan on|off` and `focus point X Y` of 0.13 have no 1.0 command: on the camera the first
  did not stop the hand tracking from panning and the second blurred the picture (PX-U-5,
  PX-U-16 below).

### Fixed

- The Arch `PKGBUILD` no longer rewrites itself when built from a checkout, so `makepkg -si`
  leaves the repository clean; a local build is labelled with the release version (#120).

### Upgrading from 0.13

1. Install the 1.0 package (it replaces the 0.13 one) and run `systemctl --user daemon-reload`.
2. End the old daemon: `systemctl --user stop obsbot4linux.service`, or end the
   `obsbot4linux daemon` process that `obsbot4linux watch` started, or log out.
3. Remove drop-ins whose `ExecStart` runs `obsbot4linux daemon`
   (`systemctl --user cat obsbot4linux.service` lists them).
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
| `gimbal … --speed S`                                                    | `do gimbal.move … speed=S`                                           | S up to 100, tilt up to 80 with a speed; the tracking stays as it is                                        |
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
| `hand pan\|tilt on\|off` | `set hand.axes.tilt=…` | `hand pan` is PX-U-5 |
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
| `focus point X Y` | – | PX-U-16 |
| `focus set N` | `set focus.mode=manual focus.position=N` | no visible effect on the Tiny 2 Lite (#99) |
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
| `image focus V` | `set focus.mode=manual focus.position=V` | no visible effect on the Tiny 2 Lite (#99) |
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
- **PX-U-5** `hand pan on|off`: not carried over. The camera kept panning with it off.
- **PX-U-16** `focus point X Y`: not carried over. The picture blurred instead of focusing
  (#99).

## [0.13.2] - 2026-10-31

### Fixed

- The daemon no longer broadcasts a state right after a change that still has the old zone, hand,
  start pose, preset, image, zoom and audio values. It now sends one state after the full read
  that follows the change, so a client's optimistic value is not pulled back for a second (#107).

## [0.13.1] - 2026-10-31

### Fixed

- `wake mode` sets the tracking mode at power-on, not on waking from sleep; `wake zoom` had no
  observable effect on the Tiny 2 Lite, at power-on or on waking. README and help say so; command
  names and JSON keys are unchanged (#102).

## [0.13.0] - 2026-10-31

### Added

- `presets` (`[{slot, name, pose}]`, `null` when the camera did not answer) in `status` and the
  daemon's state events. It is read with the zone and hand data, so a `preset save|rename|delete`
  through the daemon broadcasts the new list at once. Capability `preset_state` (#103).

## [0.12.0] - 2026-10-31

### Added

- `zone view list|save|rename|delete|go` stores, lists and goes to named poses in the camera
  (`0x1344`, `0x1384`, `0x1544`, `0x13c4`, `0x1604`); `status.zone.views` lists them. They are not
  exclusion areas, the camera does not track by them. Capability `zone_views` (#74).
- `hand area --pan MIN MAX --tilt MIN MAX` and `hand area-reset` set the hand tracking area
  (`0x2044`..`0x21c4`) and put it back to the default (`0x25c4`..`0x2684`); `hand gimbal on|off`
  lets the hand tracker move the gimbal or holds it (`0x26c4`). Capabilities `hand_area`,
  `hand_gimbal` (#75).
- `hand pan on|off`, `hand tilt on|off` and `hand return-after <seconds>` (`0x2344`, `0x23c4`,
  `0x2444`); `status.hand` has `pan_enabled`, `tilt_enabled` and `return_after`. Capabilities
  `hand_axes`, `hand_return_after` (#85).

### Fixed

- `zone on|off` sends `0x08c4` only. `0x5084` has no handler in the firmware and the 100 ms pause
  was not needed: the camera followed `0x08c4` alone both ways (#81).
- A reply without a data section (type `0x09`, what the camera answers to the view list while it
  has none) counted as "no answer" and ran into the poll timeout; it now reads as empty.

### Changed

- `zone.return` stays a pose: the leading word of `0x4ec4` is the preset id, always 0, so the
  camera has no "not set" for it (#57).

## [0.11.1] - 2026-10-31

### Added

- `startup set --ai static|normal|upper-body|headless|group|none` sets the AI mode of the initial
  state, alone (the pose stays) or with a pose. `startup show` and `status.state.startup` show it
  as `ai_mode` (`null` when not set). Capability `startup_ai` (#79).

### Fixed

- `startup go` left tracking suspended when the initial state had no AI mode, which is what
  `startup set` always wrote. `startup set` now keeps the mode of the initial state unless
  `--ai` is given, and the README says that `startup go` needs a mode there (#79).

## [0.11.0] - 2026-10-31

### Added

- `speed-mode lazy|slow|normal|fast|very-fast|auto` sets the finer tracking speed (`0x0944`) and
  `speed headroom` the third vertical tracking type (`0x0cc4` = 1). `status.framing` reads the
  speed mode and the options below back. Capabilities `speed_mode`, `speed_headroom` (#64).
- `framing auto-zoom|size|zoom-speed|offset|part` sets the options the AI modes are made of
  (`0x0a44`, `0x0ac4`, `0x0b44`, `0x0bc4` / `0x0c44`, `0x1584`); `status.framing` reads them back.
  Capability `framing` (#84).
- `gimbal rate --pan R --tilt R` turns the gimbal at a speed until `gimbal stop` (`0x6484`, the
  joystick of OBSBOT Center; it does not limit absolute moves). The camera reverses its yaw when
  mirror xor invert is on, which obsbot4linux undoes. No capability `gimbal_rate`: the camera
  has no timeout, so a client that dies leaves it turning. Capability `gimbal_stop` (#78, #80).
- `gimbal --pan P --tilt T --speed S` moves at S degrees per second (`0x6544`), `zoom F --speed S`
  zooms smoothly (`0x6884`) and `zoom stop` / `gimbal stop` stop (`0x68c4`, `0x6704`). Capabilities
  `gimbal_speed`, `zoom_speed` (#80).
- `gimbal center X Y` (`0x0544`), which moved the gimbal only toward the left and downwards in
  tests, see the README. Hidden from `--help`, no capability (#83).

### Changed

- `status.tracking_speed` can now be `headroom`: the camera value 1 of `0x0cc4` used to show as
  `standard`.

## [0.10.0] - 2026-10-31

### Added

- `led blink on|off` makes the status LED blink or burn steadily (short command `18`). The status
  block does not carry it, so `status` cannot show it. `led 0-3` is unchanged (#76).
- `exposure manual --time <1-2500> --gain <1-64>` sets the shutter time and gain together with
  the switch to manual exposure, both range-checked first. The camera uses the shutter step at or
  below the time given. Capability `exposure_values` (#77).
- `audio volume <0-100>` sets the microphone volume (`0x8102`); `status` has `audio_volume`
  (query `0x80c2`). Capability `audio_volume` (#86).
- `wake zoom <1.0-4.0>` sets the zoom after waking up and `wake mode <mode>` the tracking mode the
  camera starts in; both read back in `status` (`wake_up_zoom_percent`, `default_work_mode`).
  `wake` alone still wakes the camera. Capabilities `wake_zoom`, `wake_mode` (#87).
- `focus af continuous|single|manual` sets the camera's own autofocus mode (`0x3442`) and
  `focus point <x> <y>` focuses once on a point (`0x3582`); `status` has `focus` with the mode,
  the manual position and the focus window (query `0x3642`). Capabilities `focus_af_mode`,
  `focus_point` (#88).

### Changed

- `image exposure_time` is refused unless the exposure is manual, since a shutter time sent in
  automatic exposure freezes it; the error points to `exposure manual --time`.

## [0.9.0] - 2026-10-31

### Added

- `firmware` (e.g. `6.2.8.1`, query `0x0408`) and `model_info` (`product`, `platform`, query
  `0x1948`) in `status` and the state events, read once per connect; `obsbot4linux status`
  prints the firmware (through the daemon, from what it already read). The serial number and
  UUID are not read (#63).
- `startup show` prints the start pose (`0x3884`) and the picture settings of the initial state
  (`0x3e84`) (#89). A set group shows the byte as read, a group that is not set is `null`.
- `status` shows `wake_up_zoom_percent`, `sleep_images` and `default_work_mode` (#82).

### Changed

- `status.anti_flicker` is a name now (`off`, `50hz`, `60hz`, `auto`) instead of the V4L2
  number; the camera can report automatic detection, which was passed on as a bare 3 (#82).

### Fixed

- `startup go` no longer turns the next zone tracking return into Static. It sends the short
  command `00` instead of the framed `0x3904`, which arms that flag in the camera (#79). The
  camera ignores the short command while asleep or with the gimbal locked. Found by static
  analysis of the firmware; the zone return itself is not yet checked on the camera.
- The error code of a rejected query ("camera rejected query ...") was read from the wrong place.
  Those answers have a 16-byte header with the code (a signed 16-bit number, e.g. -3) in an
  extension; it is now read from there and shown in decimal (#62).

## [0.8.0] - 2026-10-31

### Changed

- The daemon reads the live gimbal pose (`0x0043`) every second only while a client wants it.
  A `watch` connection asks with the request `["live-pose", "on"]` (and `"off"`); the poll runs
  while at least one connection has it on, a closed connection counts as off, and a new one
  starts off. Without a subscriber `pose` in the state events changes only with the full read
  every 5 s, so fewer queries hold the camera lock. New capability `live_pose_on_demand`; clients
  that never ask no longer get a pose every second (#68).

## [0.7.3] - 2026-10-31

### Fixed

- With zone tracking and "follow new targets" on, the camera switched to Static each time the
  target left the area and the gimbal returned, instead of taking the next person. obsbot4linux
  caused it: the live pose came from the AI query `0x6604`, polled every second, and every
  `0x6604` arms a flag in the camera that turns the next return into Static. The pose now comes
  from the gimbal controller (`0x0043`), which does not set the flag and follows AI tracking just
  the same (#67).

## [0.7.2] - 2026-10-27

### Fixed

- The daemon and a `--direct` call, or two direct calls, could talk to the camera at the same
  time. The vendor unit has one answer buffer and answers are matched by sequence number only,
  so one process could take the other's answer or time out. Every camera handle now holds an
  exclusive lock on the video node; the others wait up to 5 s ("the camera is busy" after that).
  Video applications do not take the lock and are not affected (#45).

## [0.7.1] - 2026-10-27

### Added

- `limits` in the status and the daemon's state events: the pan, tilt and zoom ranges that
  `gimbal`, the poses and `zone area` accept, so clients need not hardcode them (#58).

### Changed

- `zone area` takes the gimbal range, pan -150..150 and tilt -70..90, instead of OBSBOT Center's
  -100..100 and -30..30. Measured on a Tiny 2 Lite: zone tracking keeps to a tilt of 45 (stopped
  at 48.7; 90.7 without zone). Pan beyond 100 is only confirmed by the camera's read-back (#58).

### Fixed

- Tilt below -75 puts the Tiny 2 Lite to sleep (privacy), and woken there it returns to that pose
  and sleeps again. `gimbal` and the poses now stop at tilt -70, OBSBOT's controllable limit,
  instead of -89. The "-90 reads back as +90" noted in 0.7.0 was this sleep. The README shows the
  way out of the loop (#58).

## [0.7.0] - 2026-10-27

### Added

- A daemon that `watch` starts itself appends its errors to `daemon.log` next to `state.json`;
  "cannot reach the daemon" names that file (#42).
- Release artifacts carry a build provenance attestation (#36).

### Changed

- `gimbal`, `preset save`, `startup set` and `zone`/`hand return-here` reject poses outside
  pan -150..150 and tilt -89..90 (the measured mechanical range of the Tiny 2 Lite; -90 reads back
  as +90) and zoom 1.0-4.0, instead of sending them (#32, #39).
- `watch` exits with 1 when the daemon closes the connection, and with 0 when stdin or stdout
  close (#42).
- The daemon exits non-zero when its camera worker stops, so systemd's `Restart=on-failure`
  applies (#42).
- The daemon opens the camera once per tick instead of three to four times (#46).
- Rust 1.88 is the minimum version (`rust-version`), and CI tests with it. CI and release builds
  use a pinned toolchain (#33, #36).
- Releases are only built from commits on `main`, and run fmt and clippy before the build (#35).
- `packaging/arch/PKGBUILD` carries the release version; a release fails when it differs from
  the tag (#43).

### Fixed

- A `watch` client that stalled during a broadcast stayed connected but got no more events; its
  connection is now closed, so it reconnects (#30).
- Two daemons started at the same time could both bind the socket and poll the camera. A lock
  next to the socket now keeps it to one; the socket is created with mode 0600 from the start
  (#31).
- `gimbal --tilt 1000` or `--pan -1000` became "keep this axis" on the wire and still reported
  success (#32).
- `auto-sleep off` with a stored time of 0 s left auto-sleep on; it now sends off with 600 s
  (measured) (#37).
- A selector of the wrong length failed with a raw `ENOBUFS` instead of a clear error (#38).
- `--json status` and `preset list` no longer make the daemon re-read everything and broadcast
  (#40).
- `--direct` writes to `state.json` are no longer overwritten by the daemon's cache, and the file
  is written with a unique, synced temp file (#41).
- With two cameras attached, the same one is picked every time (#45).
- The rpm installs `LICENSE` also with `--excludedocs` (#43).
- README: Rust version, install examples for the current release, a systemd drop-in for a binary
  outside `/usr/bin`, and the device node that is actually used (#33, #34, #44).

## [0.6.0] - 2026-10-25

### Added

- `hand` in `obsbot4linux --json status` and the daemon's state events: the hand tracking area
  (`pan_min`, `pan_max`, `tilt_min`, `tilt_max`) and the `return` position (`pan`, `tilt`, `zoom`,
  `null` while none is set), read from the camera (queries `0x2004`, `0x2504`). `null` when the
  camera does not answer.
- `hand return-here` sets the current pose as the hand tracking return position (`0x24c4`),
  `hand return-go` moves the camera there (`0x2584`) and `hand return-reset` clears it (`0x2544`);
  new capability `hand_return`. `hand left` and `hand right` work as before.

## [0.5.0] - 2026-10-25

### Added

- `zone` in `obsbot4linux --json status` and the daemon's state events: the zone area
  (`pan_min`, `pan_max`, `tilt_min`, `tilt_max`), `follow` and the `return` position
  (`pan`, `tilt`, `zoom`), read from the camera (queries `0x4c44`, `0x4cc4`, `0x4d44`,
  `0x4dc4`, `0x4e44`, `0x4ec4`). `null` when the camera does not answer.
- `zone return-go` moves the camera to the zone return position (`0x4f44`); new capability
  `zone_return`.
- `startup go` moves the camera to the start pose (`0x3904`).

### Changed

- `state.zone_follow` and `state.startup` in the status are read from the camera (`0x4e44`,
  `0x3884`) instead of the state file; the keys are unchanged. They are `null` when the camera
  does not answer, and the state file no longer stores them. The daemon asks for the zone values
  at start, after each command and once a minute.

### Fixed

- `zone return-here` sent `0x3904`, which moves the gimbal to the boot pose and stores nothing
  (#16). It now reads the current pose (`0x6604`, `0x6804`) and sets it as the zone return
  position with `0x4e84`. The old behaviour is `startup go`.

## [0.4.0] - 2026-10-25

### Added

- `target` in `obsbot4linux --json status` and in the daemon's state events: the AI tracking
  target from query `0x01c4`, `{"detected": bool, "box": [..4 floats..]}`, or `null` while
  tracking is Static, no camera is connected or the reply is unusable. `detected` is true
  while the camera has a target (status LED blue) and false while it searches (yellow); `box`
  is then the last one. The daemon asks once a second, only while tracking is on, and sends an
  event when `detected` changes, not for box movement alone. Plain `status` prints
  `target: detected` or `target: searching` while tracking. New capability `target`.

## [0.3.1] - 2026-10-25

### Fixed

- `streaming` missed the most common streamers: the DMS widget's Qt Multimedia preview holds
  the node open without mapping it, and Chrome captures in a sandboxed process whose `/proc`
  entries are unreadable. `streaming` now asks the kernel (`VIDIOC_REQBUFS` with count 0
  fails with `EBUSY` while another handle owns the capture buffers), which sees every
  application. `streaming_apps` stays the best-effort `/proc` list and can be empty while
  `streaming` is true.

## [0.3.0] - 2026-10-25

### Added

- `streaming` (bool) and `streaming_apps` (process names, sorted, deduplicated) in
  `obsbot4linux --json status` and in the daemon's state events: whether an application
  streams from the camera. Detected from `/proc` (video node open and its buffers mapped);
  `false` and `[]` while no camera is connected. The daemon checks every second and sends an
  event when it changes. Only mmap-based capture and processes of the same user are seen.

## [0.2.0] - 2026-10-25

### Added

- `model` and `device` keys in `obsbot4linux --json status` and in the daemon's state
  events (`obsbot4linux watch`): the camera's human name and the V4L2 node to stream from
  (the stable `/dev/v4l/by-id/...-video-index0` link when available). Both are `null`
  while no camera is connected; all existing keys are unchanged.
- `obsbot4linux detect`: exit 0 and the model (`--json`: `{model, device}`) when a supported
  camera is connected, exit 1 without output when not. Enumeration only, no daemon and no
  camera command.
- "Supported cameras" section in the README.

## [0.1.0] - 2026-10-25

### Added

- `obsbot4linux` for the OBSBOT Tiny 2 Lite: every command with `--json`;
  commands tested byte for byte against OBSBOT Center captures.
- `obsbot4linux daemon` owns the camera state and broadcasts every change over
  `$XDG_RUNTIME_DIR/obsbot4linux.sock`; the CLI is a client of it and falls back
  to direct access. The camera is opened per operation, so video calls keep
  streaming.
- `obsbot4linux watch`: JSON lines for the DMS widget, state events out,
  requests in.
- Settings: sleep / wake, AI tracking mode and speed, HDR, exposure (global,
  face, manual over UVC), zoom, field of view, gimbal (Center's pose command),
  status LED, mirror, invert, auto-sleep, sleep background, audio, gestures,
  hand side, zone tracking with area and return position, image controls,
  autofocus, camera presets, start pose, factory reset.
- Live gimbal pose (query `0x6604`) in the status, read every second by the
  daemon.
- Socket-activated systemd user units (`obsbot4linux.socket` / `.service`).
- Release packaging: GitHub Actions run fmt, clippy and tests on every push; a
  `vX.Y.Z` tag builds tarballs, `.deb`, `.rpm` and Arch `.pkg.tar.zst` packages
  (x86_64 and aarch64) with `SHA256SUMS` and creates the GitHub release. The
  PKGBUILD builds either the local checkout or a release tarball.

### Changed

- Renamed from Tiny2Lite4Linux / `t2l` to OBSBot4Linux / `obsbot4linux`, so further OBSBOT
  models can join as modules.

### Fixed

- Selecting global or face exposure no longer freezes the automatic exposure
  (a manual-exposure frame was sent along).
