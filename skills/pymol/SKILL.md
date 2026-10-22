---
name: pymol
description: "Operate PyMOL for molecular structure inspection, publication-quality rendering, and reproducible headless scripts. Use for PDB/mmCIF inputs, cartoon/sticks/surface representations, alignments, measurements, mutagenesis views, ray-traced PNGs, PSE sessions, and batch structure rendering. Prefer the PyMOL Python API or `pymol -cq`; use desktop GUI control only when the user explicitly asks to drive the PyMOL window."
metadata:
  third_party:
    - kind: software
      name: PyMOL
      info_url: https://github.com/schrodinger/pymol-open-source
---

# PyMOL

Use PyMOL for molecular visualization and reproducible scene generation. The
default path is a project-local Python script executed headlessly. GUI
automation is a fallback for interactive inspection only.

## When to use

- Inspect a PDB, mmCIF, or existing `.pse` session.
- Build and render publication figures: cartoon, sticks, surface, mesh, or a
  combination with explicit colors and camera framing.
- Align structures, measure distances/angles/dihedrals, show electron density,
  symmetry mates, ligands, or mutagenesis views.
- Save a reusable `.pse` session plus the `.py` script that produced it.
- Batch-render several structures or views.

Do not use it for read-only in-app previews; the built-in 3Dmol preview is
enough for that. Do not use it for web pages.

## Environment

Probe before writing a script:

```bash
pymol --version
pixi run pymol --version
python -c "import pymol; print(pymol.__version__)"
```

If none works, propose one of these and wait for the user's confirmation before
installing anything:

```bash
# Project-local environment (preferred)
pixi init
pixi add -c conda-forge pymol-open-source
pixi run pymol -cq scripts/render.py

# Or conda/mamba
conda create -n pymol -c conda-forge pymol-open-source
conda run -n pymol pymol -cq scripts/render.py
```

Do not install silently. If PyMOL is missing and the user declines, stop and
report what is needed.

## Inputs

Accept any of:

- A PDB ID (PyMOL can `fetch` it).
- A local `.pdb`, `.cif`, `.mmcif`, or `.pse` path.
- A directory of structures for batch rendering.

Confirm what the user wants to show before choosing representations. Ask for
chains, ligands, residues, or a biological assembly when those choices change
the figure. Do not guess a residue number or claim a binding site without
reading it from the structure.

## Workflow

1. **Inspect.** Load the structure and read back object names, chain IDs,
   residue ranges, ligands, and the number of atoms. Use `cmd.get_names`,
   `cmd.get_chains`, `cmd.count_atoms`, and `cmd.identify`.
2. **Plan.** State the representation, coloring, camera, canvas size, and
   output files in one short plan.
3. **Write a script.** Save it under `scripts/<name>.py`. Keep the script
   self-contained: load input, apply selections and styles, set the camera,
   ray trace, save PNG and PSE.
4. **Run headless.**
   ```bash
   pymol -cq scripts/<name>.py
   ```
5. **Verify.** Confirm the PNG exists and is non-empty, check its dimensions,
   and inspect it with the image viewer. Re-render after fixing clipping,
   labels, or color problems.
6. **Deliver.** Report the PNG, PSE, script, and a short description of the
   selections and settings. Keep the script so the figure can be reproduced.

## Output conventions

- `figures/<name>.png` — ray-traced image.
- `figures/<name>.pse` — saved PyMOL session.
- `scripts/<name>.py` — the exact script used.
- `figures/<name>.md` — optional caption and selection notes.

Use explicit `ray width,height` and `png ..., dpi=..., ray=1` settings. Do not
overwrite an existing figure; add a suffix or ask which name to use.

## Quality rules

- Use a white or explicitly requested background; avoid the default black
  unless the user asks for it.
- Prefer ray tracing for final figures; previews may use a lower quality.
- Keep colors color-blind-safe and consistent across panels.
- Show only the chains, ligands, and residues the user asked for; hide the rest.
- Verify every label and measurement against the loaded structure. Never invent
  residue numbers, distances, angles, or biological conclusions.
- Do not claim a figure is publication-ready until the PNG has been inspected.
- Keep the camera, clipping, and canvas size deterministic in the script.

## GUI fallback

Only when the user explicitly asks to drive the PyMOL window, or when a task is
impossible headlessly:

1. Load `desktop-use`.
2. Call `desktop_health_report` / `desktop_check_permissions` first.
3. Read the window with `desktop_get_window_state`.
4. Act on element tokens, not guessed coordinates.
5. Verify the result after each meaningful change.

Do not use GUI automation for batch rendering, and do not leave the user's
mouse or focus captured after an action.

## Troubleshooting

- `pymol: command not found` — install it with the pixi/conda commands above.
- No display / OpenGL error — use `pymol -cq` and avoid GUI-only commands.
- Ray trace is slow or out of memory — reduce canvas size, disable shadows, or
  render at a lower resolution first.
- mmCIF is not recognized — update PyMOL and load with `cmd.load(path, "obj")`;
  check the format before assuming the file is corrupt.
- A selection is empty — print `cmd.count_atoms(selection)` and inspect chain
  and residue names before changing the representation.
