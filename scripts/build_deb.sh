#!/usr/bin/env bash

# This script takes as argument a name

set -Eeuo pipefail

readonly SCRIPT_SRC="$(dirname "${BASH_SOURCE[${#BASH_SOURCE[@]} - 1]}")"
readonly SCRIPT_DIR="$(cd "${SCRIPT_SRC}" >/dev/null 2>&1 && pwd)"
readonly SCRIPT_NAME=$(basename "$0")
PACKAGE_DIR="./deb-scratch" #### /!\ This directory will be erased and recreated /!\

# Check cargo and fpm are available.
command -v cargo > /dev/null 2>&1  || { echo "cargo not found. You need to install cargo."; return 1; }

cd ${SCRIPT_DIR}

MIMIRSBRUNN_DIR="$(cd .. >/dev/null 2>&1 && pwd)"

trap 'rm -fr "$tmpdir"' exit

tmpdir=$(mktemp -d -p . -t deb-XXXXXX) || exit 1

echo "Building debian package in $SCRIPT_DIR/$tmpdir"

version=$(cat ../Cargo.toml | grep '^version' | cut -d '=' -f 2 | tr -d \")
version="${version#"${version%%[![:space:]]*}"}" # trim version

pkgdir="$tmpdir/mimirsbrunn_$1-$version"
rootdir="$pkgdir/usr"
mkdir -p "$rootdir"

# build and install mimirsbrunn to a temporary directory
# It is assumed this script is in a folder under the root of the project.
# We use the locked option to make sure the crates in Cargo.lock are used,
# not updated ones.
cargo install --locked --path=${MIMIRSBRUNN_DIR} --root=$rootdir

mkdir -p "$pkgdir/DEBIAN"

cat << EOF > "$pkgdir/DEBIAN/control"
Package: mimirsbrunn
Version: $version
Section: base
Priority: optional
Architecture: amd64
Maintainer: Matthieu Paindavoine <matthieu.paindavoine@kisiodigital.com>
Description: Mimir
EOF

dpkg-deb --build "$pkgdir"

# Trying to guess the name of the debian package we produced.... There is probably a better way.
# It's looking in the current directory for all the deb extensions, sorting them by modified time, taking
# the last one, and then keeping just the name
deb=$(find . -name '*.deb' -printf "%T@ %Tc %p\n" | sort -n | tail -1 | rev | cut -d ' ' -f 1 | rev)

# Finally we put the debian package in a directory (on its own, so it's easier to pickup for uploading)
if [ -d "$PACKAGE_DIR" ]; then
  rm -fr "$PACKAGE_DIR"
fi
mkdir -p "$PACKAGE_DIR"
mv "$deb" "$PACKAGE_DIR"

echo "Generated $PACKAGE_DIR/$deb"
