#!/bin/bash
set -e

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

cp "$(dirname "$0")/../PKGBUILD" "$TMPDIR/"
cd "$TMPDIR"
makepkg -si
