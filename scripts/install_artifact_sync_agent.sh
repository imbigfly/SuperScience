#!/usr/bin/env bash
# Install a per-user macOS LaunchAgent that syncs the newest CI installers
# every 30 minutes. The script and downloads live outside ~/Documents because
# launchd is denied TCC access to Documents; Application Support is readable.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LABEL="science.superscience.ci-artifacts"
BASE="$HOME/Library/Application Support/science.superscience"
SCRIPT_DIR="$BASE/scripts"
INSTALL_DIR="$BASE/installers"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
LOG="$HOME/Library/Logs/superscience-ci-artifacts.log"
PYTHON_BIN="$(command -v python3 || true)"
UID_VALUE="$(id -u)"

mkdir -p "$HOME/Library/LaunchAgents" "$HOME/Library/Logs" "$SCRIPT_DIR" "$INSTALL_DIR"
cp "$ROOT/scripts/sync_ci_artifacts.sh" "$SCRIPT_DIR/sync_ci_artifacts.sh"
chmod 0755 "$SCRIPT_DIR/sync_ci_artifacts.sh"

cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$LABEL</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/bash</string>
    <string>$SCRIPT_DIR/sync_ci_artifacts.sh</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>CI_ARTIFACT_DIR</key>
    <string>$INSTALL_DIR</string>
    <key>CI_MIRROR</key>
    <string>https://ghfast.top/</string>
    <key>PYTHON_BIN</key>
    <string>$PYTHON_BIN</string>
    <key>PATH</key>
    <string>/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
  </dict>
  <key>StartInterval</key>
  <integer>1800</integer>
  <key>RunAtLoad</key>
  <false/>
  <key>StandardOutPath</key>
  <string>$LOG</string>
  <key>StandardErrorPath</key>
  <string>$LOG</string>
</dict>
</plist>
EOF

launchctl bootout "gui/$UID_VALUE/$LABEL" 2>/dev/null || true
launchctl bootstrap "gui/$UID_VALUE" "$PLIST"
launchctl enable "gui/$UID_VALUE/$LABEL"

echo "installed $PLIST"
echo "script: $SCRIPT_DIR/sync_ci_artifacts.sh"
echo "downloads: $INSTALL_DIR"
echo "log: $LOG"
launchctl print "gui/$UID_VALUE/$LABEL" | sed -n '1,22p'
