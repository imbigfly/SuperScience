#!/usr/bin/env bash
# Local macOS packaging with a stable self-signed identity so Keychain ACLs
# survive rebuilds. Not for distribution (no Developer ID / notarization).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IDENTITY="${APPLE_SIGNING_IDENTITY:-SuperScience Local Codesign}"
TARGET="${TARGET:-aarch64-apple-darwin}"
BUNDLE="$ROOT/target/$TARGET/release/bundle"
APP="$BUNDLE/macos/SuperScience.app"

if ! security find-identity -v -p codesigning | grep -Fq "$IDENTITY"; then
  echo "error: code-signing identity not found: $IDENTITY" >&2
  echo "Create/import a Code Signing cert with that common name, then retry." >&2
  security find-identity -v -p codesigning >&2 || true
  exit 1
fi

cd "$ROOT"
unset NO_COLOR FORCE_COLOR || true

# Build unsigned (or adhoc) first; we re-sign without hardened runtime so a
# self-signed local build can still launch without notarization.
cargo tauri build \
  --config src-tauri/tauri.macos.conf.json \
  --bundles app \
  --target "$TARGET"

VERSION="$(
  python3 - <<'PY'
import json
print(json.load(open("src-tauri/tauri.conf.json"))["version"])
PY
)"
ARCH_SUFFIX="${TARGET%%-*}"
DMG="$BUNDLE/dmg/SuperScience_${VERSION}_${ARCH_SUFFIX}.dmg"

echo "Signing with: $IDENTITY"
# No --options runtime: hardened runtime + self-signed + no notarization often
# prevents launch on recent macOS.
codesign --force --deep --timestamp=none --sign "$IDENTITY" "$APP"
codesign --verify --deep --strict "$APP"
codesign -dv "$APP" 2>&1 | sed -n '1,12p'

STAGE="$(mktemp -d)"
cleanup() { rm -rf "$STAGE"; }
trap cleanup EXIT
cp -R "$APP" "$STAGE/SuperScience.app"
ln -s /Applications "$STAGE/Applications"

mkdir -p "$(dirname "$DMG")"
rm -f "$DMG"
hdiutil create \
  -volname "SuperScience" \
  -srcfolder "$STAGE" \
  -ov -format UDZO \
  -fs HFS+ \
  "$DMG"

echo
echo "Signed local DMG ready:"
echo "  $DMG"
echo "Authority: $IDENTITY"
