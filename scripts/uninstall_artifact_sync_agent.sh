#!/usr/bin/env bash
# Remove the per-user LaunchAgent installed by install_artifact_sync_agent.sh.
set -euo pipefail

LABEL="science.superscience.ci-artifacts"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"

launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
rm -f "$PLIST"
echo "removed $PLIST"
