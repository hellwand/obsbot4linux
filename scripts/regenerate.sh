#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Regenerates the generated files from the code (design spec 3): schema/protocol-2.json and
# descriptions/. Run it after changing obsbot-proto types or a model's description, and
# commit the result. The stale tests fail until then.
set -eu
cd "$(dirname "$0")/.."
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot-proto --test schema
OBSBOT_REGENERATE=1 cargo test --locked -q -p obsbot-device --test generated
git status --short schema descriptions
