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
| `gimbal … --speed S`                                                    | `do gimbal.move … speed=S`                                           | S up to 100, tilt up to 80 with a speed; the tracking stays as it is |
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
| `hand pan\|tilt on\|off`                                                | –                                                                    | not carried over (PX-U-5)                                            |
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
| `focus auto off`                                                        | `set focus.mode=manual`                                              | writes the camera's UVC autofocus off, as 0.13 did                   |
| `focus af continuous\|single\|manual`                                   | `set focus.mode=…`                                                   |                                                                      |
| `focus point X Y`                                                       | –                                                                    | gone (below)                                                         |
| `focus set N`                                                           | `set focus.mode=manual focus.position=N`                             |                                                                      |
| `startup set --pan P --tilt T --zoom Z`                                 | `do power_on.set_pose pose=P,T,Z`                                    |                                                                      |
| `startup set --preset N`                                                | `do power_on.use_preset slot=N`                                      |                                                                      |
| `startup set --current`                                                 | `do power_on.use_current`                                            | the current pose                                                     |
| `startup set --ai M\|none`                                              | `set power_on.start.mode=M\|unset`                                   | M `normal` only (PX-U-18)                                            |
| `startup set <pose option> --ai M`                                      | the `do` of the pose option, then `set power_on.start.mode=M`        | two commands                                                         |
| `startup show`                                                          | `get power_on`                                                       | without the picture settings (PX-U-19)                               |
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
- `hand pan on|off`: the camera did not honour it; the hand tracking kept panning with it off.
- `focus point X Y`: the picture blurred instead of focusing.

Rows that the 1.0 release could not confirm on the camera are named in the release notes,
with the 0.13 command they affect.
