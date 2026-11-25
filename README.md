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

| Model              | USB         | Support | Firmware tested |
| ------------------ | ----------- | ------- | --------------- |
| OBSBOT Tiny 2 Lite | `3564:fef9` | tested  | 6.2.8.1         |

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
