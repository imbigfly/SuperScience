# App updates

The desktop client checks for updates through the TCTOKEN Open API, not GitHub
Releases or the Tauri signed updater.

## Check API

```
GET {base}/api/client/update/check?platform={macos|windows|linux}&version={CARGO_PKG_VERSION}&arch={m|intel|arm64|x64}
```

- Default base: `https://www.tctoken.cn` (same origin as account login).
- Override with `TCTOKEN_API_BASE` to point at staging before the path is live
  in production. No code change is required.
- The request does not need a login token. The client uses an 8–15s timeout.
- Response envelope: `{ success, message, data }`. `success=false` or an HTTP
  error is a failed check. The dialog does **not** fall back to GitHub.
- When the server has no package for the current platform/arch, the API may
  return `client updates config not found`. The UI localizes that to “no
  installer is published for this platform yet.”

Whether an update exists is decided locally from semver: `latest > current`.
If the server still sends `has_update=false`, that vetoes the prompt. A missing
`has_update` field (current schema) is treated as “use semver”. `force_update`
is honored only when that comparison also says the remote version is newer, so
a server typo cannot force a downgrade.

Platform and architecture are mapped as:

| Runtime | Query value |
| --- | --- |
| macOS / Windows / Linux | `macos` / `windows` / `linux` |
| macOS Apple Silicon / Intel | `m` / `intel` |
| Windows or Linux `aarch64` / other | `arm64` / `x64` |

## User flow

1. Startup, Settings, the menu, and the command palette all call the same
   check. Startup is silent: a newer version only shows the sidebar card, never
   a modal.
2. When the user checks manually, the dialog shows release notes. Nothing
   downloads until they choose **Download update**.
3. **Download update** calls the check API again and uses the latest
   `download_url`. A stale URL from an earlier check is not used. If that
   lookup fails and a previous URL is still cached, in-app download falls back
   to the cache. There is no “open download page” action.
4. The client streams that `download_url` to a temporary file (DMG / EXE / other
   installer). If `checksum` is a 64-character SHA-256 hex digest, the file is
   verified before the install step. An empty checksum skips verification and
   is logged.
5. **Open installer** uses the system opener (`tauri-plugin-opener`). macOS
   opens the DMG; Windows opens the installer. The app does not replace its
   own bundle and does not restart itself. The “opening installer” dialog has
   a **Close** button (Escape also dismisses it) so the current app stays
   usable. Quit the old build after the system installer finishes.
6. `force_update=true` hides **Later** and **Don't remind me**. The sidebar
   card remains; the remaining actions are download and open the installer.

`install_supported` is true when a newer version exists and `download_url` is
non-empty. macOS, Windows, and Linux share this path. If `download_url` is
empty, the dialog does not offer download.

## Failure and recovery

- A failed check shows the network or API error only. There is no GitHub
  Releases link.
- An interrupted download or checksum mismatch leaves the installed app
  untouched. Download again from the dialog.
- Active chats and runs do not block opening the installer. You still need to
  quit the old app to finish a replace-in-place install.
- There is no background install and no automatic downgrade.

## Release configuration

CI may still upload `latest.json` as a release artifact. The client no longer
reads it. `src-tauri/tauri.conf.json` does not configure `plugins.updater`.

## Manual smoke test

1. Point `TCTOKEN_API_BASE` at an environment that already serves
   `/api/client/update/check`.
2. Install an older build, check for updates, and confirm the notes and
   download progress.
3. Cancel before downloading; the installed version must stay unchanged.
4. Download, confirm checksum (or the skip-checksum log), then **Open
   installer** and finish with the system UI.
5. With `force_update=true`, confirm **Later** / **Don't remind me** are
   hidden and Escape does not close the dialog.
6. With the API down, confirm the error dialog has no download-page button.
