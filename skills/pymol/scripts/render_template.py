#!/usr/bin/env python3
"""PyMOL rendering template.

Copy this file to `scripts/<name>.py` and edit the CONFIG block. Run it with:

    pymol -cq scripts/<name>.py

or, when PyMOL lives in a project pixi environment:

    pixi run pymol -cq scripts/<name>.py

The template is intentionally explicit: a reproducible figure should name its
input, selection, colors, camera, canvas, and outputs in the script.
"""

from __future__ import annotations

import os

from pymol import cmd

# ---- CONFIG -----------------------------------------------------------------
INPUT = "structures/input.pdb"          # PDB, mmCIF, or .pse
OBJECT = "target"
OUT_PNG = "figures/pymol_scene.png"
OUT_PSE = "figures/pymol_scene.pse"
WIDTH = 1600
HEIGHT = 1200
DPI = 300
BACKGROUND = "white"
# -----------------------------------------------------------------------------

os.makedirs(os.path.dirname(OUT_PNG) or ".", exist_ok=True)
os.makedirs(os.path.dirname(OUT_PSE) or ".", exist_ok=True)

# 1. Load and inspect. Keep the object name stable so selections are readable.
cmd.load(INPUT, OBJECT)
print("objects:", cmd.get_names("objects"))
print("chains:", cmd.get_chains(OBJECT))
print("atoms:", cmd.count_atoms(OBJECT))

# 2. Clean the scene. Remove solvent only when the user did not ask for it.
cmd.remove("solvent")
cmd.hide("everything", "all")

# 3. Representations and colors. Replace these with the user's requested view.
cmd.show("cartoon", OBJECT)
cmd.color("grey80", OBJECT)
# Example ligand highlight (edit the selection after inspecting the structure):
# cmd.show("sticks", "organic")
# cmd.color("orange", "organic")

# 4. Camera and rendering settings.
cmd.bg_color(BACKGROUND)
cmd.set("ray_opaque_background", 1)
cmd.set("antialias", 2)
cmd.set("cartoon_transparency", 0.0)
cmd.orient(OBJECT)

# 5. Ray trace and save. `ray` must run before `png(..., ray=1)` for final art.
cmd.ray(WIDTH, HEIGHT)
cmd.png(OUT_PNG, width=WIDTH, height=HEIGHT, dpi=DPI, ray=1)
cmd.save(OUT_PSE)

print("wrote:", OUT_PNG)
print("wrote:", OUT_PSE)
