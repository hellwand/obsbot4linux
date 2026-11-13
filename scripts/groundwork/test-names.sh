#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Test-name guard of the groundwork stage. Lists the tests of a commit by the last two parts of
# their path (module and name, which a move keeps), sorted. Run it for the base and the head of
# a pull request and compare the two lists: a pure move changes no line.
# Usage: scripts/groundwork/test-names.sh REV > names.txt
set -euo pipefail
rev=${1:?usage: test-names.sh REV}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git archive "$rev" | tar -x -C "$work"
cd "$work"
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$work/target} nice -n 19 \
  cargo test --workspace --locked -q -j 3 -- --list --format terse 2>/dev/null |
  awk -F': ' '$2 == "test" { n = split($1, p, "::"); print (n > 1 ? p[n - 1] "::" : "") p[n] }' |
  LC_ALL=C sort
