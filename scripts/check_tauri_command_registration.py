#!/usr/bin/env python3
"""Fail when the frontend invokes a Tauri command the backend never registers.

A merge that resolves `src-tauri/src/lib.rs` from upstream silently drops
fork-only entries from `invoke_handler`. The command functions still exist and
still carry `#[tauri::command]`, so the build stays green and nothing fails
until a user clicks the button and gets "Command <name> not found".

Run from the repository root:

    python3 scripts/check_tauri_command_registration.py

Exit code 0 means every `invoke`/`invoke_checked` command name the UI uses is
reachable; 1 lists the missing ones.
"""

from __future__ import annotations

import glob
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LIB_RS = REPO / "src-tauri" / "src" / "lib.rs"

INVOKE_RE = re.compile(r'invoke(?:_checked)?\(\s*"([A-Za-z0-9_]+)"')
BACKEND_INVOKE_RE = re.compile(r"invoke\(\s*['\"]([A-Za-z0-9_]+)['\"]")
COMMAND_RE = re.compile(
    r"#\[tauri::command([^\]]*)\]\s*"
    r"(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)"
)
RENAME_RE = re.compile(r'rename\s*=\s*"([^"]+)"')
HANDLER_RE = re.compile(r"tauri::generate_handler!\[(.*?)\n\s*\]\s*;", re.S)
ENTRY_RE = re.compile(r"^\s*([A-Za-z_][\w:]*),\s*$", re.M)


def ui_command_names() -> set[str]:
    names: set[str] = set()
    patterns = ("ui/src/**/*.rs", "ui/src/**/*.js", "ui/*.js")
    for pattern in patterns:
        for path in glob.glob(str(REPO / pattern), recursive=True):
            text = Path(path).read_text(encoding="utf-8", errors="ignore")
            names.update(INVOKE_RE.findall(text))
            names.update(BACKEND_INVOKE_RE.findall(text))
    return names


def command_effective_names() -> dict[str, str]:
    """Map every defined command function to the name Tauri exposes."""
    by_fn: dict[str, str] = {}
    for path in glob.glob(str(REPO / "src-tauri/src/**/*.rs"), recursive=True):
        text = Path(path).read_text(encoding="utf-8", errors="ignore")
        for attrs, fn in COMMAND_RE.findall(text):
            rename = RENAME_RE.search(attrs)
            by_fn[fn] = rename.group(1) if rename else fn
    return by_fn


def registered_fn_names() -> set[str]:
    text = LIB_RS.read_text(encoding="utf-8")
    match = HANDLER_RE.search(text)
    if not match:
        raise SystemExit("could not locate the generate_handler! block in src-tauri/src/lib.rs")
    return {entry.split("::")[-1] for entry in ENTRY_RE.findall(match.group(1))}


def main() -> int:
    by_fn = command_effective_names()
    registered = registered_fn_names()
    servable = {by_fn.get(fn, fn) for fn in registered}
    missing = sorted(ui_command_names() - servable)
    if missing:
        print(f"{len(missing)} command(s) invoked by the UI are not registered:\n")
        for name in missing:
            print(f"  - {name}")
        print("\nRestore them in src-tauri/src/lib.rs inside tauri::generate_handler![...].")
        return 1
    print(f"ok: {len(registered)} registered handlers cover every UI invoke")
    return 0


if __name__ == "__main__":
    sys.exit(main())
