#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Move check of the groundwork stage. Prints the lines of Rust code that differ between two
# commits, compared as a multiset: the file a line is in and the order of lines do not count,
# and `use` statements, `mod name;` declarations, `pub(crate)` / `pub(super)`, indentation and
# blank lines are set aside. A pure move prints only the structural lines its task lists.
# Usage: scripts/move-check.sh OLD_REV [NEW_REV]    (NEW_REV defaults to HEAD)
set -euo pipefail
old=${1:?usage: move-check.sh OLD_REV [NEW_REV]}
new=${2:-HEAD}

lines() {
  git ls-tree -r --name-only "$1" | grep '\.rs$' | while read -r f; do
    git show "$1:$f"
  done | awk '
    in_use { if (/;[[:space:]]*$/) in_use = 0; next }
    /^[[:space:]]*(pub(\([a-z]+\))? )?use / { if (!/;[[:space:]]*$/) in_use = 1; next }
    /^[[:space:]]*(pub(\([a-z]+\))? )?mod [a-z0-9_]+;[[:space:]]*$/ { next }
    /^[[:space:]]*$/ { next }
    { sub(/^[[:space:]]+/, ""); sub(/^pub\((crate|super)\) /, ""); print }
  ' | LC_ALL=C sort
}

diff <(lines "$old") <(lines "$new") | grep '^[<>]' || true
