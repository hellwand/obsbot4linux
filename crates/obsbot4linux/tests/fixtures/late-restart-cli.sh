#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Stands in for the command line in tests/check_model.rs: a camera that answers a write or an
# action with `warning: restart` at once and goes away only later (measured 2026-11-26: the
# Tiny 2 Lite went away 2.9 s after the reply to `factory_reset`). `set audio.enabled=...` and
# `do factory_reset ...` reply at once and reach the fake 3 s later; every other call goes to
# the real binary (LATE_RESTART_REAL_CLI). Each call is logged to LATE_RESTART_LOG in order:
# `reply` when a delayed command is answered, `landed` once it reached the fake, `write` for
# every other `set` or `do`. A file of the repository, not written by the test (see
# drop-led-cli.sh).
case "$*" in
*audio.enabled=* | *factory_reset*)
	echo "reply $*" >>"$LATE_RESTART_LOG"
	(
		sleep 3
		"$LATE_RESTART_REAL_CLI" "$@"
		echo "landed $*" >>"$LATE_RESTART_LOG"
	) </dev/null >/dev/null 2>&1 &
	echo '{"rev":1}'
	exit 0
	;;
esac
case "$1" in set | do) echo "write $*" >>"$LATE_RESTART_LOG" ;; esac
exec "$LATE_RESTART_REAL_CLI" "$@"
