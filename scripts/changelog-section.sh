#!/bin/sh
# Print the CHANGELOG.md section for a release tag (vX.Y.Z or X.Y.Z, or vX.Y.Z-rc.N for a
# release candidate). Fails unless Cargo.toml has that version, packaging/arch/PKGBUILD has it
# as Arch writes it (without the "-": 1.0.0rc.1) and CHANGELOG.md has a "## X.Y.Z" (or
# "## [X.Y.Z]") heading with content below it. Used by the release workflow and ci.yml.
set -eu
cd "$(dirname "$0")/.."

ver=${1:?usage: changelog-section.sh vX.Y.Z}
ver=${ver#v}
bad() {
  echo "not a X.Y.Z or X.Y.Z-rc.N version: $ver" >&2
  exit 1
}
case $ver in
*[!0-9a-z.-]*) bad ;;
esac
printf '%s\n' "$ver" | grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+(-rc\.[1-9][0-9]*)?' || bad

crate=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
if [ "$crate" != "$ver" ]; then
  echo "Cargo.toml has version $crate, tag is $ver" >&2
  exit 1
fi

pkgver=$(sed -n 's/^pkgver=//p' packaging/arch/PKGBUILD)
archver=$(printf '%s' "$ver" | tr -d -)
if [ "$pkgver" != "$archver" ]; then
  echo "packaging/arch/PKGBUILD has pkgver $pkgver, tag $ver needs $archver" >&2
  exit 1
fi

notes=$(awk -v v="$ver" '
  /^## / { on = ($2 == v || $2 == "[" v "]"); next }
  on
' CHANGELOG.md)
if [ -z "$(printf %s "$notes" | tr -d ' \n')" ]; then
  echo "CHANGELOG.md has no non-empty \"## $ver\" section" >&2
  exit 1
fi
printf '%s\n' "$notes"
