#!/usr/bin/env bash
# Sync the newest macOS/Windows installers to this machine.
#
# Primary path: read the `ci-latest` prerelease (the build workflows publish
# each platform there after a successful package), download the assets through
# a GitHub mirror, and verify each asset's API digest.
# Fallback: nightly.link workflow artifacts.
#
# Repeat runs skip unchanged assets via a digest marker. A directory lock keeps
# launchd from stacking runs.
set -euo pipefail

REPO="${CI_SYNC_REPO:-imbigfly/SuperScience}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CI_ARTIFACT_DIR:-$ROOT/dist/final}"
RELEASE_TAG="${CI_RELEASE_TAG:-ci-latest}"
MIRROR="${CI_MIRROR:-https://ghfast.top/}"
PYTHON="${PYTHON_BIN:-python3}"
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

sync_release() {
  local json
  if ! json="$(curl -fsSL --max-time 30 \
    "https://api.github.com/repos/$REPO/releases/tags/$RELEASE_TAG")"; then
    if compgen -G "$DEST/SuperScience-*-macos-*.dmg" >/dev/null \
       && compgen -G "$DEST/SuperScience-*-windows-*.exe" >/dev/null; then
      log "release API unavailable; keeping existing installers"
      return 0
    fi
    return 1
  fi

  local mac mac_digest win win_digest
  IFS=$'\t' read -r mac mac_digest win win_digest <<< "$("$PYTHON" - "$json" <<'PY'
import json, sys
d = json.loads(sys.argv[1])
assets = {a.get("name", ""): a for a in d.get("assets", [])}
mac = next((n for n in assets if n.endswith("-macos-aarch64.dmg")), "")
win = next((n for n in assets if n.endswith("-windows-x64-setup.exe")), "")
print("\t".join([
    mac,
    assets.get(mac, {}).get("digest", ""),
    win,
    assets.get(win, {}).get("digest", ""),
]))
PY
)"

  if [ -z "$mac" ] || [ -z "$win" ]; then
    log "release $RELEASE_TAG has no installer assets"
    return 1
  fi

  local marker="$mac_digest|$win_digest"
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
  for spec in "$mac:$mac_digest" "$win:$win_digest"; do
    local name="${spec%%:*}"
    local digest="${spec#*:}"
    curl -fL --retry 5 --retry-all-errors --connect-timeout 20 --max-time 3600 \
      -o "$tmp/$name" "${MIRROR}${base}/${name}"
    if [ -n "$digest" ]; then
      local expected="${digest#sha256:}"
      local actual
      actual="$(hash_file "$tmp/$name")"
      if [ "$actual" != "$expected" ]; then
        log "checksum mismatch for $name"
        rm -rf "$tmp"
        return 1
      fi
      log "verified $name"
    fi
  done

  rm -f "$DEST"/SuperScience-*-macos-*.dmg "$DEST"/SuperScience-*-windows-*.exe
  mv "$tmp/$mac" "$tmp/$win" "$DEST/"
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
