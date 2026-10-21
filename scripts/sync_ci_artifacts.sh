#!/usr/bin/env bash
# Sync the newest successful macOS/Windows installer artifacts to this machine.
#
# GitHub Actions artifacts normally require a signed-in session. nightly.link
# exposes the latest artifact of a workflow/branch as a plain ZIP, so this
# script can run from launchd without a token.
#
# The script asks the GitHub API for the latest successful run of each build
# workflow (on any branch), then downloads that run's artifact through
# nightly.link. A per-artifact ETag marker prevents repeat downloads.
set -euo pipefail

REPO="${CI_SYNC_REPO:-imbigfly/SuperScience}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CI_ARTIFACT_DIR:-$ROOT/dist/final}"
FORCE=0
[ "${1:-}" = "--force" ] && FORCE=1

mkdir -p "$DEST"

# One sync at a time; launchd can fire while a manual run is in flight.
LOCK="$DEST/.sync.lock"
if ! mkdir "$LOCK" 2>/dev/null; then
  echo "another artifact sync is already running"
  exit 0
fi
trap 'rmdir "$LOCK" 2>/dev/null || true' EXIT

log() { printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*"; }

latest_run() {
  local workflow="$1"
  curl -fsSL --max-time 20 \
    "https://api.github.com/repos/$REPO/actions/workflows/$workflow/runs?status=success&per_page=1" \
    | python3 -c 'import json,sys
d=json.load(sys.stdin)
runs=d.get("workflow_runs") or []
if runs:
    r=runs[0]
    print(str(r.get("id","")) + "\t" + r.get("head_branch",""))' 2>/dev/null || true
}

sync_one() {
  local workflow="$1" artifact="$2" label="$3"
  local run branch
  IFS=$'\t' read -r run branch <<< "$(latest_run "$workflow.yml")"
  if [ -z "$run" ]; then
    branch="${CI_SYNC_BRANCH:-ci/final-build}"
    run="branch:$branch"
    log "$workflow: no successful run found; falling back to $branch"
  fi

  local encoded="${branch//\//%2F}"
  local url="https://nightly.link/$REPO/workflows/$workflow/$encoded/$artifact.zip"
  local marker="$DEST/.$artifact.run"

  if [ "$FORCE" = 0 ] && [ -f "$marker" ] \
     && [ "$(cat "$marker")" = "$run" ] && [ -d "$DEST/$label" ]; then
    log "$artifact: unchanged (run $run)"
    return 0
  fi

  log "$artifact: downloading run $run from $url"
  local tmp="$DEST/.$artifact.zip.part"
  rm -f "$tmp"
  curl -fL --retry 5 --retry-all-errors --connect-timeout 20 --max-time 7200 \
    -o "$tmp" "$url"
  unzip -tq "$tmp" >/dev/null

  local out="$DEST/$label"
  rm -rf "$out"
  mkdir -p "$out"
  unzip -q -o "$tmp" -d "$out"
  rm -f "$tmp"

  printf '%s\n' "$run" > "$marker"
  log "$artifact: saved to $out"
}

sync_one "build-macos" "darwin-aarch64-dmg" "macos"
sync_one "build-windows" "windows-x64-nsis" "windows"

log "done"
