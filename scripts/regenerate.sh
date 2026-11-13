#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Regenerates the generated files from the code (design spec 3, 6.2): schema/protocol-2.json,
# descriptions/ and packaging/completions/. Run it after changing obsbot-proto types, a model's
# description or the command-line definition, and commit the result. The stale tests fail
# until then.
set -eu
cd "$(dirname "$0")/.."
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot-proto --test schema
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot-device --test generated
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot4linux --test completions the_completion_files_are_current
git status --short schema descriptions packaging/completions
