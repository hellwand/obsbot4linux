#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Stands in for the command line in tests/check_model.rs: a camera that answers a write or an
# action with `warning: restart` at once and goes away only later (measured 2026-11-26: the
# Tiny 2 Lite went away 2.9 s after the reply to `factory_reset`). `set audio.enabled=...` and
# `do factory_reset ...` reply at once and reach the fake 1.5 s later; every other call goes
# to the real binary (LATE_RESTART_REAL_CLI). A file of the repository, not written by the
# test (see drop-led-cli.sh).
case "$*" in
*audio.enabled=* | *factory_reset*)
	(
		sleep 1.5
		exec "$LATE_RESTART_REAL_CLI" "$@"
	) </dev/null >/dev/null 2>&1 &
	echo '{"rev":1}'
	exit 0
	;;
esac
exec "$LATE_RESTART_REAL_CLI" "$@"
