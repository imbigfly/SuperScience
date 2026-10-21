#!/usr/bin/env bash
# Sync the newest macOS/Windows installers to this machine.
#
# Primary path: read the `ci-latest` prerelease (published by the
# publish-ci-installers workflow), download its assets through a GitHub mirror,
# and verify SHA256SUMS. Fallback: nightly.link workflow artifacts.
#
# The script is safe to run repeatedly: a release digest marker skips unchanged
# assets, and a directory lock prevents concurrent runs from launchd.
set -euo pipefail

REPO="${CI_SYNC_REPO:-imbigfly/SuperScience}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CI_ARTIFACT_DIR:-$ROOT/dist/final}"
RELEASE_TAG="${CI_RELEASE_TAG:-ci-latest}"
MIRROR="${CI_MIRROR:-https://ghfast.top/}"
FORCE=0
[ "${1:-}" = "--force" ] && FORCE=1

mkdir -p "$DEST"

LOCK="$DEST/.sync.lock"
if ! mkdir "$LOCK" 2>/dev/null; then
  echo "another artifact sync is already running"
  exit 0
fi
trap 'rmdir "$LOCK" 2>/dev/null || true' EXIT

log() { printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*"; }

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

verify_sums() {
  local dir="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$dir" && sha256sum -c SHA256SUMS)
  else
    (cd "$dir" && shasum -a 256 -c SHA256SUMS)
  fi
}

sync_release() {
  local json
  json="$(curl -fsSL --max-time 30 \
    "https://api.github.com/repos/$REPO/releases/tags/$RELEASE_TAG")" || return 1

  local mac win marker parsed
  parsed="$(python3 - "$json" <<'PY'
import json, sys
d = json.loads(sys.argv[1])
assets = {a.get("name", ""): a for a in d.get("assets", [])}
mac = next((n for n in assets if n.endswith("-macos-aarch64.dmg")), "")
win = next((n for n in assets if n.endswith("-windows-x64-setup.exe")), "")
marker = "|".join(sorted(
    f"{n}:{assets[n].get('digest') or assets[n].get('updated_at') or assets[n].get('size')}"
    for n in assets
))
print("\t".join([mac, win, marker]))
PY
)"
  IFS=$'\t' read -r mac win marker <<< "$parsed"

  if [ -z "$mac" ] || [ -z "$win" ]; then
    log "release $RELEASE_TAG has no installer assets"
    return 1
  fi

  local marker_file="$DEST/.$RELEASE_TAG.digest"
  if [ "$FORCE" = 0 ] && [ -f "$marker_file" ] \
     && [ "$(cat "$marker_file")" = "$marker" ] \
     && [ -f "$DEST/$mac" ] && [ -f "$DEST/$win" ]; then
    log "release $RELEASE_TAG unchanged"
    return 0
  fi

  local tmp="$DEST/.incoming"
  rm -rf "$tmp"
  mkdir -p "$tmp"
  local base="https://github.com/$REPO/releases/download/$RELEASE_TAG"
  log "downloading $RELEASE_TAG assets through $MIRROR"
  for name in SHA256SUMS "$mac" "$win"; do
    curl -fL --retry 5 --retry-all-errors --connect-timeout 20 --max-time 3600 \
      -o "$tmp/$name" "${MIRROR}${base}/${name}"
  done
  verify_sums "$tmp"

  rm -f "$DEST"/SuperScience-*-macos-*.dmg "$DEST"/SuperScience-*-windows-*.exe
  mv "$tmp/$mac" "$tmp/$win" "$tmp/SHA256SUMS" "$DEST/"
  rm -rf "$tmp"
  printf '%s\n' "$marker" > "$marker_file"
  log "saved $mac and $win to $DEST"
  return 0
}

sync_nightly_fallback() {
  local workflow="$1" artifact="$2" label="$3"
  local branch="${CI_SYNC_BRANCH:-ci/final-build}"
  local encoded="${branch//\//%2F}"
  local url="https://nightly.link/$REPO/workflows/$workflow/$encoded/$artifact.zip"
  local tmp="$DEST/.$artifact.zip.part"
  log "fallback: downloading $artifact from $url"
  rm -f "$tmp"
  curl -fL --retry 5 --retry-all-errors --connect-timeout 20 --max-time 7200 \
    -o "$tmp" "$url"
  unzip -tq "$tmp" >/dev/null
  local out="$DEST/$label"
  rm -rf "$out"
  mkdir -p "$out"
  unzip -q -o "$tmp" -d "$out"
  rm -f "$tmp"
  log "fallback: saved $artifact to $out"
}

if ! sync_release; then
  log "falling back to workflow artifacts"
  sync_nightly_fallback "build-macos" "darwin-aarch64-dmg" "macos"
  sync_nightly_fallback "build-windows" "windows-x64-nsis" "windows"
fi

log "done"
