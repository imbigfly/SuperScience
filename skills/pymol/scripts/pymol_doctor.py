#!/usr/bin/env python3
"""Read-only PyMOL environment probe.

Run with any Python interpreter, for example:

    python skills/pymol/scripts/pymol_doctor.py

The output is JSON. It never installs anything and never opens a window.

The macOS app bundle counts as an installed PyMOL even when `pymol` is not on
PATH: `/Applications/PyMOL.app` ships its own CLI and Python under
`Contents/bin/`.
"""

from __future__ import annotations

import importlib.util
import json
import os
import plistlib
import shutil
import subprocess
import sys
from pathlib import Path

APP_CANDIDATES = (
    Path("/Applications/PyMOL.app"),
    Path.home() / "Applications" / "PyMOL.app",
)


def run(command: list[str], timeout: int = 15) -> dict:
    try:
        result = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return {
            "ok": result.returncode == 0,
            "returncode": result.returncode,
            "stdout": result.stdout.strip()[:800],
            "stderr": result.stderr.strip()[:800],
        }
    except Exception as error:  # noqa: BLE001 - probe should report, never crash
        return {"ok": False, "error": str(error)}


def macos_app() -> dict:
    """Return the app bundle paths when a PyMOL.app is installed."""
    for app in APP_CANDIDATES:
        if not app.is_dir():
            continue
        info = app / "Contents" / "Info.plist"
        bundle_id = None
        version = None
        if info.is_file():
            try:
                with info.open("rb") as handle:
                    plist = plistlib.load(handle)
                bundle_id = plist.get("CFBundleIdentifier")
                version = plist.get("CFBundleShortVersionString")
            except Exception:  # noqa: BLE001 - a broken plist should not hide the app
                pass
        return {
            "app": str(app),
            "bundle_id": bundle_id,
            "version": version,
            "cli": str(app / "Contents" / "bin" / "pymol"),
            "python": str(app / "Contents" / "bin" / "python"),
            "launch": f"open -a {app}",
        }
    return {}


def pixi_pymol() -> str | None:
    """Return the pixi-installed open-source CLI when it is not on PATH."""
    for candidate in (
        Path.home() / ".pixi" / "bin" / "pymol",
        Path.home() / ".pixi" / "envs" / "pymol-open-source" / "bin" / "pymol",
    ):
        if candidate.is_file():
            return str(candidate)
    return None


def nearby_structures(root: Path, limit: int = 50) -> list[str]:
    """List structure files in the current directory and one level down."""
    suffixes = {".pdb", ".cif", ".mmcif", ".pse"}
    found: list[str] = []
    for base, dirs, files in os.walk(root):
        depth = len(Path(base).relative_to(root).parts)
        if depth >= 2:
            dirs[:] = []
        for name in files:
            if Path(name).suffix.lower() in suffixes:
                found.append(str(Path(base) / name))
                if len(found) >= limit:
                    return sorted(found)
    return sorted(found)


def main() -> int:
    root = Path.cwd()
    app = macos_app()
    pymol = shutil.which("pymol") or pixi_pymol()
    pixi = shutil.which("pixi")
    payload: dict = {
        "cwd": str(root),
        "python": sys.version.split()[0],
        "pymol_app": app or None,
        "pymol_executable": pymol,
        "pymol_importable": importlib.util.find_spec("pymol") is not None,
        "pixi_executable": pixi,
        "structure_files": nearby_structures(root),
    }

    if app and Path(app["cli"]).is_file():
        payload["pymol_app_version"] = run([app["cli"], "--version"])
    elif app:
        payload["pymol_app_version"] = {
            "ok": True,
            "stdout": f"PyMOL {app.get('version') or 'unknown'} ({app['app']})",
        }
    if pymol:
        payload["pymol_version"] = run([pymol, "--version"])
    if pixi:
        payload["pixi_run_pymol"] = run([pixi, "run", "pymol", "--version"], timeout=30)

    available = bool(app) or bool(pymol) or bool(payload["pymol_importable"]) or bool(
        (payload.get("pixi_run_pymol") or {}).get("ok")
    )
    payload["available"] = available
    if app:
        payload["recommended"] = (
            "Launch the installed app and drive it with the desktop tools; do not "
            "install another PyMOL just because `pymol` is missing from PATH."
        )
    elif not available:
        payload["install_hint"] = (
            "PyMOL was not found. Propose, and wait for confirmation before running: "
            "brew install --cask pymol, or "
            "pixi init && pixi add -c conda-forge pymol-open-source && "
            "pixi run pymol -cq <script.py>"
        )

    print(json.dumps(payload, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
