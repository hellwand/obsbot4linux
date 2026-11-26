#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Stands in for the command line in tests/check_model.rs: `describe` names $NODE_CLI_VIDEO as
# the device's video node (a fake device has none that exists), every other call goes to the
# real binary (NODE_CLI_REAL_CLI) unchanged. A file of the repository, not written by the test
# (see drop-led-cli.sh).
case "$1" in
describe) "$NODE_CLI_REAL_CLI" "$@" | jq -c --arg n "$NODE_CLI_VIDEO" '.video_node = $n' ;;
*) exec "$NODE_CLI_REAL_CLI" "$@" ;;
esac
