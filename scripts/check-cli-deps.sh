#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Design spec 3: the [dependencies] of the CLI crate never contain obsbot-device or
# obsbot4linuxd (its [dev-dependencies] may). Fails with a message otherwise.
set -eu
cd "$(dirname "$0")/.."
deps=$(cargo metadata --format-version 1 --no-deps --locked |
  jq -r '.packages[] | select(.name == "obsbot4linux") | .dependencies[] | select(.kind == null) | .name')
for forbidden in obsbot-device obsbot4linuxd; do
  if printf '%s\n' "$deps" | grep -qx "$forbidden"; then
    echo "the obsbot4linux CLI depends on $forbidden; only obsbot-proto is allowed (spec section 3)" >&2
    exit 1
  fi
done
echo "CLI dependencies: $(printf '%s' "$deps" | paste -sd' ' -)"
