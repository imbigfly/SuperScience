#!/usr/bin/env bash
# Fetch the pinned Cua Driver sidecar into src-tauri/binaries/.
#
# The driver is optional: builds without it still work, and the desktop tools
# simply do not appear. CI runs this before `tauri build`; contributors can run
# it once locally.
set -euo pipefail

VERSION="${CUA_DRIVER_VERSION:-0.37.0}"
REPO="${CUA_DRIVER_REPO:-trycua/cua}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST_DIR="$ROOT/src-tauri/binaries"
FORCE=0
[ "${1:-}" = "--force" ] && FORCE=1

case "$(uname -s)" in
  Darwin) os=darwin ;;
  Linux) os=linux ;;
  MINGW*|MSYS*|CYGWIN*) os=windows ;;
  *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

case "$(uname -m)" in
  arm64|aarch64) arch=arm64 ;;
  x86_64|amd64|AMD64) arch=x86_64 ;;
  *) echo "unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

if [ "$os" = darwin ]; then
  # The universal binary covers both Intel and Apple Silicon app builds.
  asset="cua-driver-rs-${VERSION}-darwin-universal-binary.tar.gz"
  exe=cua-driver
elif [ "$os" = linux ]; then
  asset="cua-driver-rs-${VERSION}-linux-${arch}-binary.tar.gz"
  exe=cua-driver
else
  asset="cua-driver-rs-${VERSION}-windows-${arch}-binary.zip"
  exe=cua-driver.exe
fi

mkdir -p "$DEST_DIR"
dest="$DEST_DIR/$exe"

if [ "$FORCE" = 0 ] && [ -x "$dest" ]; then
  if "$dest" --version 2>/dev/null | grep -q "$VERSION"; then
    echo "cua-driver $VERSION already present at $dest"
    exit 0
  fi
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
base="https://github.com/${REPO}/releases/download/cua-driver-rs-v${VERSION}"

echo "fetching $asset"
curl -fL --retry 3 --retry-all-errors --retry-delay 2 --connect-timeout 20 \
  -o "$tmp/$asset" "$base/$asset"
curl -fsSL --retry 3 --retry-all-errors --retry-delay 2 --connect-timeout 20 \
  -o "$tmp/SHA256SUMS" "$base/SHA256SUMS"

expected="$(awk -v f="$asset" '$2 == f || $2 == "*" f { print $1 }' "$tmp/SHA256SUMS" | head -1)"
if [ -z "$expected" ]; then
  echo "no SHA256 entry for $asset" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$asset" | awk '{ print $1 }')"
else
  actual="$(shasum -a 256 "$tmp/$asset" | awk '{ print $1 }')"
fi
if [ "$actual" != "$expected" ]; then
  echo "checksum mismatch for $asset" >&2
  exit 1
fi

case "$asset" in
  *.tar.gz)
    gzip -t "$tmp/$asset"
    tar -xzf "$tmp/$asset" -C "$tmp"
    ;;
  *.zip)
    if command -v unzip >/dev/null 2>&1; then
      unzip -q "$tmp/$asset" -d "$tmp/unpacked"
    else
      mkdir -p "$tmp/unpacked"
      tar -xf "$tmp/$asset" -C "$tmp/unpacked"
    fi
    ;;
esac

found="$(find "$tmp" -type f -name "$exe" | head -1)"
if [ -z "$found" ]; then
  echo "archive did not contain $exe" >&2
  exit 1
fi

cp "$found" "$dest"
chmod 0755 "$dest"
"$dest" --version
echo "installed $dest"
