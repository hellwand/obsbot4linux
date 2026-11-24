#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Stands in for the command line in tests/check_model.rs: answers `set led.brightness=...`
# without sending it, and passes every other call on to the real binary (DROP_LED_REAL_CLI).
# It is a file of the repository and not written by the test: a file that a test writes and
# executes at once fails with "Text file busy" whenever another test thread forks between the
# write and the exec.
case "$*" in *led.brightness=*) echo '{"rev":1}'; exit 0 ;; esac
exec "$DROP_LED_REAL_CLI" "$@"
