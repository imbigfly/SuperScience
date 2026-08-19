# Browser Runtime architecture

Wisp 0.3.0 treats the Chrome extension as a **controlled access adapter**.
The desktop process owns sessions, waits, staging, and approval.

```
Agent tools
    -> Browser Runtime (src-tauri/src/browser_bridge)
        -> ws://127.0.0.1:18765  shared / daily Chrome
        -> ws://127.0.0.1:18766  workspace Chrome (dedicated profile)
            -> Manifest V3 extension (browser-extension/)
```

## Sessions

| Session | Browser | Login state | Port |
|---|---|---|---|
| `shared` | User's existing Chrome/Edge profile | Daily cookies and extensions | 18765 |
| `workspace` | System Chrome launched with `%APPDATA%/science.wisp-science/browser-workspace` | Clean until the user signs in there | 18766 |

Both can be connected at once. If they are, tools must pass `session`.

## What the extension does

- Handshake: `extension_version`, `protocol_version=2`, `capabilities[]`
- Conditional wait (URL / selector / text / settle)
- Article scan (`images[]`, `figures[]`, `code_blocks[]`)
- Host-permission asset download into `Downloads/WispBrowserStaging`
- Viewport / full-page / selector capture
- Pause control from the popup (`USER_CONTROLLING`)

The extension never writes project directories and never returns large base64 files as the archive path.

## What the Runtime does

- Multiplexes two WebSocket listeners
- Browser Task Lease (`last_session` + explicit `session`)
- Copies staged files into the project and hashes SHA-256
- Starts/stops the workspace Chrome window
- ChatGPT one-shot send/wait/read on an already-logged-in tab

Playwright is not used. The user's daily Chrome User Data directory is never passed as `--user-data-dir`.