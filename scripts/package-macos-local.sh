#!/usr/bin/env bash
# Local macOS packaging with a stable self-signed identity so Keychain ACLs
# survive rebuilds. Not for distribution (no Developer ID / notarization).
#
# Build-cache notes, each learned from a packaging run that took far longer
# than it should:
#
#   * Tauri v2 merges `tauri.<platform>.conf.json` by itself, so this script
#     must NOT pass `--config src-tauri/tauri.macos.conf.json`. Passing it makes
#     the release fingerprint differ from a plain `cargo build --release`, and
#     the two builds then evict each other's artifacts.
#   * `src-tauri/build.rs` declares `rerun-if-env-changed=WISP_CATALOG_OFFLINE`,
#     so any release build that does not export it recompiles src-tauri and all
#     of its dependents. Export the same value in hand-run release builds.
#   * Always pass `--target` even when it equals the host triple. Skipping it
#     puts artifacts in `target/release` while every prior local/CI DMG (and
#     the warm incremental cache) lives under `target/<triple>/release` —
#     the next packaging then cold-rebuilds into an empty second tree.
#   * A dev preview (`cargo tauri dev` / `trunk serve`) holds the same cargo
#     lock on `ui/target` and competes for CPU; overlapping it can make this
#     script many times slower. It is stopped below and restarted at exit.
#     Pass --keep-preview to leave it running.
#
# Speed flags for local iteration (combine freely):
#
#   --reuse-frontend  Skip trunk/wasm; requires an existing ui/dist. Saves ~5min
#                     when only Rust/backend changed. Uses the stable merge file
#                     src-tauri/tauri.pack-reuse-frontend.conf.json (not the
#                     platform conf — see above).
#   --fast            Build with profile.release-local (no LTO, incremental=true,
#                     opt-level=3). Artifacts land under target/<triple>/release-local.
#                     First --fast run is still a full compile into that tree;
#                     later incremental packs are the point.
#
# Typical incremental re-pack after a Rust-only tweak:
#   scripts/package-macos-local.sh --reuse-frontend --fast
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IDENTITY="${APPLE_SIGNING_IDENTITY:-SuperScience Local Codesign}"
TARGET="${TARGET:-aarch64-apple-darwin}"

KEEP_PREVIEW=0
REUSE_FRONTEND=0
FAST=0
for arg in "$@"; do
  case "$arg" in
    --keep-preview) KEEP_PREVIEW=1 ;;
    --reuse-frontend) REUSE_FRONTEND=1 ;;
    --fast) FAST=1 ;;
    -h|--help)
      sed -n '1,40p' "$0"
      exit 0
      ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

cd "$ROOT"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
# Always pass --target so the fingerprint matches prior packaging runs under
# target/<triple>/… (see header). Cross-compiles need it anyway.
TARGET_ARGS=(--target "$TARGET")

if [ "$FAST" = "1" ]; then
  CARGO_PROFILE=release-local
else
  CARGO_PROFILE=release
fi
BUNDLE="$ROOT/target/$TARGET/$CARGO_PROFILE/bundle"

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

# build.rs fetches models.dev at build time; on a slow network that stalls the
# build before falling back. The checked-in snapshot is what releases refresh
# anyway (scripts/refresh_model_catalog.sh), so local packaging always uses it.
export WISP_CATALOG_OFFLINE=1

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

# ───────── dev preview (the personal preview loop on this machine) ─────────
# The loop lives on the Desktop, not in the repo, so use its own stop/start
# scripts when they exist and fall back to killing the known dev processes.
PREVIEW_STOP="$HOME/Desktop/停止APP预览.command"
PREVIEW_START="$HOME/Desktop/启动APP预览.command"
PREVIEW_STOPPED=0

dev_preview_running() {
  pgrep -f "cargo-tauri tauri dev" >/dev/null 2>&1 && return 0
  pgrep -f "trunk serve.*dist-dev" >/dev/null 2>&1 && return 0
  pgrep -f "/target/debug/superscience-tauri" >/dev/null 2>&1 && return 0
  return 1
}

stop_dev_preview() {
  if [ -f "$PREVIEW_STOP" ]; then
    bash "$PREVIEW_STOP" >/dev/null 2>&1 || true
    return 0
  fi
  pkill -f "cargo-tauri tauri dev" 2>/dev/null || true
  pkill -f "trunk serve.*dist-dev" 2>/dev/null || true
  pkill -f "/target/debug/superscience-tauri" 2>/dev/null || true
  return 0
}

restore_dev_preview() {
  [ "$PREVIEW_STOPPED" = "1" ] || return 0
  if [ -f "$PREVIEW_START" ]; then
    echo "Restarting the dev preview that was running before packaging..."
    bash "$PREVIEW_START" >/dev/null 2>&1 || true
  else
    echo "note: the dev preview was stopped for packaging; start it again when needed." >&2
  fi
  return 0
}

STAGE=""
PACK_T0=$(date +%s)
step_t0=$PACK_T0
log_step() {
  local now label elapsed total
  now=$(date +%s)
  label="$1"
  elapsed=$((now - step_t0))
  total=$((now - PACK_T0))
  printf '[timing] %-28s %4ds  (total %ds)\n' "$label" "$elapsed" "$total"
  step_t0=$now
}

on_exit() {
  if [ -n "$STAGE" ]; then
    rm -rf "$STAGE"
  fi
  restore_dev_preview
}
trap on_exit EXIT

if [ "$KEEP_PREVIEW" = "0" ] && dev_preview_running; then
  echo "Stopping the dev preview so it stops contending for the cargo lock..."
  stop_dev_preview
  PREVIEW_STOPPED=1
  sleep 1
fi
log_step "stop_preview"

CONFIG_ARGS=()
if [ "$REUSE_FRONTEND" = "1" ]; then
  if [ ! -f "$ROOT/ui/dist/index.html" ]; then
    echo "error: --reuse-frontend needs ui/dist/index.html; build the UI once without the flag." >&2
    exit 1
  fi
  # Stable merge file that only clears beforeBuildCommand. Do not pass
  # tauri.macos.conf.json here (fingerprint thrash — see header).
  CONFIG_ARGS=(--config "$ROOT/src-tauri/tauri.pack-reuse-frontend.conf.json")
  echo "Reusing existing frontend at ui/dist (skipping trunk/wasm)"
fi

PROFILE_ARGS=()
if [ "$FAST" = "1" ]; then
  PROFILE_ARGS=(--profile "$CARGO_PROFILE")
  echo "Fast profile: $CARGO_PROFILE (no LTO, incremental)"
fi

echo "Bundle output: $BUNDLE"
echo "Host triple:   $HOST_TRIPLE"
echo "Target:        $TARGET"
echo "Cargo profile: $CARGO_PROFILE"

# Platform conf merges automatically; only pass the reuse-frontend override.
BUILD_CMD=(
  cargo tauri build
  "${TARGET_ARGS[@]}"
)
if [ ${#CONFIG_ARGS[@]} -gt 0 ]; then
  BUILD_CMD+=("${CONFIG_ARGS[@]}")
fi
BUILD_CMD+=(--bundles app)
if [ ${#PROFILE_ARGS[@]} -gt 0 ]; then
  BUILD_CMD+=(-- "${PROFILE_ARGS[@]}")
fi
"${BUILD_CMD[@]}"
log_step "cargo_tauri_build"

if [ ! -d "$APP" ]; then
  echo "error: expected app bundle missing: $APP" >&2
  exit 1
fi

echo "Signing with: $IDENTITY"
# No --options runtime: hardened runtime + self-signed + no notarization often
# prevents launch on recent macOS.
codesign --force --deep --timestamp=none --sign "$IDENTITY" "$APP"
log_step "codesign"
codesign --verify --deep --strict "$APP"
codesign -dv "$APP" 2>&1 | sed -n '1,12p'
log_step "codesign_verify"

STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/${PRODUCT_NAME}.app"
ln -s /Applications "$STAGE/Applications"
log_step "stage_copy"

mkdir -p "$(dirname "$DMG")"
rm -f "$DMG"
hdiutil create \
  -volname "$PRODUCT_NAME" \
  -srcfolder "$STAGE" \
  -ov -format UDZO \
  -fs HFS+ \
  "$DMG"
log_step "hdiutil_udzo"

echo
echo "Signed local DMG ready:"
echo "  $DMG"
echo "Authority: $IDENTITY"
ls -lh "$DMG"
printf '[timing] %-28s %4ds\n' "TOTAL" "$(( $(date +%s) - PACK_T0 ))"
