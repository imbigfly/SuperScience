#!/usr/bin/env bash
# Local macOS packaging with a stable self-signed identity so Keychain ACLs
# survive rebuilds. Not for distribution (no Developer ID / notarization).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IDENTITY="${APPLE_SIGNING_IDENTITY:-SuperScience Local Codesign}"
TARGET="${TARGET:-aarch64-apple-darwin}"
BUNDLE="$ROOT/target/$TARGET/release/bundle"

cd "$ROOT"
PRODUCT_NAME="$(
  python3 - <<'PY'
import json
print(json.load(open("src-tauri/tauri.conf.json"))["productName"])
PY
)"
VERSION="$(
  python3 - <<'PY'
import json
print(json.load(open("src-tauri/tauri.conf.json"))["version"])
PY
)"
APP="$BUNDLE/macos/${PRODUCT_NAME}.app"
ARCH_SUFFIX="${TARGET%%-*}"
DMG="$BUNDLE/dmg/${PRODUCT_NAME}_${VERSION}_${ARCH_SUFFIX}.dmg"

if ! security find-identity -v -p codesigning | grep -Fq "$IDENTITY"; then
  echo "error: code-signing identity not found: $IDENTITY" >&2
  echo "Create/import a Code Signing cert with that common name, then retry." >&2
  security find-identity -v -p codesigning >&2 || true
  exit 1
fi

unset NO_COLOR FORCE_COLOR || true

# Bake the Feishu SMTP client password into the binary so local DMGs can send
# feedback without shipping feedback.local.toml. Prefer an explicit env; else
# read the gitignored local override used for day-to-day development.
if [ -z "${SUPERSCIENCE_FEEDBACK_SMTP_PASSWORD:-}" ] && [ -z "${WISP_FEEDBACK_SMTP_PASSWORD:-}" ]; then
  LOCAL_FEEDBACK="$ROOT/src-tauri/config/feedback.local.toml"
  if [ -f "$LOCAL_FEEDBACK" ]; then
    SUPERSCIENCE_FEEDBACK_SMTP_PASSWORD="$(
      python3 - "$LOCAL_FEEDBACK" <<'PY'
import re, sys
raw = open(sys.argv[1], encoding="utf-8").read()
match = re.search(r'(?m)^\s*password\s*=\s*"([^"]*)"\s*$', raw)
if not match or not match.group(1).strip():
    raise SystemExit(f"error: no smtp.password in {sys.argv[1]}")
print(match.group(1))
PY
    )"
    export SUPERSCIENCE_FEEDBACK_SMTP_PASSWORD
    echo "Using SMTP password from src-tauri/config/feedback.local.toml"
  else
    echo "error: set SUPERSCIENCE_FEEDBACK_SMTP_PASSWORD, or create $LOCAL_FEEDBACK" >&2
    echo "  (copy from feedback.local.toml.example and set smtp.password)" >&2
    exit 1
  fi
fi

# Build unsigned (or adhoc) first; we re-sign without hardened runtime so a
# self-signed local build can still launch without notarization.
cargo tauri build \
  --config src-tauri/tauri.macos.conf.json \
  --bundles app \
  --target "$TARGET"

echo "Signing with: $IDENTITY"
# No --options runtime: hardened runtime + self-signed + no notarization often
# prevents launch on recent macOS.
codesign --force --deep --timestamp=none --sign "$IDENTITY" "$APP"
codesign --verify --deep --strict "$APP"
codesign -dv "$APP" 2>&1 | sed -n '1,12p'

STAGE="$(mktemp -d)"
cleanup() { rm -rf "$STAGE"; }
trap cleanup EXIT
cp -R "$APP" "$STAGE/${PRODUCT_NAME}.app"
ln -s /Applications "$STAGE/Applications"

mkdir -p "$(dirname "$DMG")"
rm -f "$DMG"
hdiutil create \
  -volname "$PRODUCT_NAME" \
  -srcfolder "$STAGE" \
  -ov -format UDZO \
  -fs HFS+ \
  "$DMG"

echo
echo "Signed local DMG ready:"
echo "  $DMG"
echo "Authority: $IDENTITY"
