#!/usr/bin/env bash
set -euxo pipefail

TARGET="$1"
PKG_FULL_NAME="$2"

rm -rf "$PKG_FULL_NAME"
mkdir -p "$PKG_FULL_NAME/bin"

bin_ext=""
[[ "$TARGET" == *-windows-* ]] && bin_ext=".exe"

cp "./target/${TARGET}/release/cairo-profiler${bin_ext}" "$PKG_FULL_NAME/bin/"

cp -r ../README.md "$PKG_FULL_NAME/"

if [[ "$TARGET" == *-windows-* ]]; then
  7z a "${PKG_FULL_NAME}.zip" "$PKG_FULL_NAME"
else
  tar czvf "${PKG_FULL_NAME}.tar.gz" "$PKG_FULL_NAME"
fi