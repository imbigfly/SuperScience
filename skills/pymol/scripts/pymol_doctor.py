#!/usr/bin/env python3
"""Read-only PyMOL environment probe.

Run with any Python interpreter, for example:

    python skills/pymol/scripts/pymol_doctor.py
    pixi run python skills/pymol/scripts/pymol_doctor.py

The output is JSON. It never installs anything and never opens a window.
"""

from __future__ import annotations

import importlib.util
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


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
    pymol = shutil.which("pymol")
    pixi = shutil.which("pixi")
    payload: dict = {
        "cwd": str(root),
        "python": sys.version.split()[0],
        "pymol_executable": pymol,
        "pymol_importable": importlib.util.find_spec("pymol") is not None,
        "pixi_executable": pixi,
        "structure_files": nearby_structures(root),
    }

    if pymol:
        payload["pymol_version"] = run([pymol, "--version"])
    if pixi:
        payload["pixi_run_pymol"] = run([pixi, "run", "pymol", "--version"], timeout=30)

    available = bool(pymol) or bool(payload["pymol_importable"]) or bool(
        (payload.get("pixi_run_pymol") or {}).get("ok")
    )
    payload["available"] = available
    if not available:
        payload["install_hint"] = (
            "PyMOL was not found. Propose, and wait for confirmation before running: "
            "pixi init && pixi add -c conda-forge pymol-open-source && "
            "pixi run pymol -cq <script.py>"
        )

    print(json.dumps(payload, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
