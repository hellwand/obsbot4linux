#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Lists the files of every package a release build made: one line per entry, "<package>
# <mode> <path>", sorted, with the version replaced by VERSION and the architecture by ARCH,
# so that two builds can be compared. The release workflow compares the list with
# packaging/contents.txt. Needs bsdtar (libarchive).
# Usage: scripts/package-contents.sh DIR X.Y.Z    (DIR holds the obsbot4linux* packages)
set -eu
dir=${1:?usage: package-contents.sh DIR X.Y.Z}
ver=${2:?usage: package-contents.sh DIR X.Y.Z}

find "$dir" -type f -name 'obsbot4linux*' | while read -r pkg; do
  name=$(basename "$pkg")
  case $name in
  *.deb) bsdtar -xOf "$pkg" 'data.tar*' | bsdtar -tvf - ;;
  *) bsdtar -tvf "$pkg" ;;
  esac | awk -v p="$name" '{ print p, $1, $NF }'
done | sed -e "s/$ver/VERSION/g" -e 's/x86_64\|aarch64\|amd64\|arm64/ARCH/g' | LC_ALL=C sort
