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
if [ "${#work}" -gt 60 ]; then
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
