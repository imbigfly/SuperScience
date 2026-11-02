---
name: pymol
description: "Open and operate the PyMOL desktop app through its own command line using desktop automation (desktop_* tools). Use for PDB/mmCIF/PSE inputs, cartoon/sticks/surface styling, alignments, measurements, disulfide or ligand annotation, ray-traced PNGs, saved PSE sessions, and recorded scripts. Default path is GUI automation; headless `pymol -cq` only when the user explicitly asks for batch/headless work or desktop automation is unavailable."
metadata:
  third_party:
    - kind: software
      name: PyMOL
      info_url: https://github.com/schrodinger/pymol-open-source
---

# PyMOL

Default behavior: **open the PyMOL window and drive it** with the bundled
desktop automation tools (`desktop_*`, Cua Driver). Load structures, style the
scene, render, and save the session from PyMOL's own command line, with the
user watching the window change.

A headless script is a fallback, not the default. Use it only when the user
explicitly asks for headless/batch rendering or when the `desktop_*` tools are
unavailable.

## When to use

- The user asks to open PyMOL, inspect a structure in the PyMOL window, or
  operate PyMOL for them.
- Build and render figures: cartoon, sticks, surface, mesh, colors, labels.
- Annotate disulfides, ligands, binding sites, distances, angles, or
  measurements in the live scene.
- Save a `.pse` session and the script that reproduces it.
- Batch-render many structures (headless fallback).

Do not use it for read-only in-app previews; the built-in 3Dmol preview is
enough for that. Do not use it for web pages.

## Find PyMOL; do not install another one

PyMOL may be installed as a macOS app while `pymol` is **not** on `PATH`.
Check the app first:

```bash
ls -d /Applications/PyMOL.app 2>/dev/null
mdfind "kMDItemCFBundleIdentifier == 'org.pymol.pymol-open-source'" 2>/dev/null
mdfind "kMDItemCFBundleIdentifier == 'com.schrodinger.pymol'" 2>/dev/null
```

- macOS app bundle: `/Applications/PyMOL.app`. The open-source build installed
  here uses bundle id `org.pymol.pymol-open-source`; a legacy Schrödinger
  "Incentive" install uses `com.schrodinger.pymol`. Use
  `desktop_launch_app` with `name: "PyMOL"` when the id is uncertain.
- The open-source app here is backed by a pixi environment:
  `~/.pixi/envs/pymol-open-source/bin/pymol` (version 3.1.0). The legacy app
  carried its own CLI at `Contents/bin/pymol`.
- Also check `desktop_list_apps` for the installed PyMOL entry.

Only when no PyMOL exists anywhere, propose an install and wait for the user's
confirmation before running it:

```bash
# Open-source edition (recommended; no license dialog or watermark)
pixi global install -c conda-forge pymol-open-source

# Project-local environment
pixi init && pixi add -c conda-forge pymol-open-source

# Or conda/mamba
conda create -n pymol -c conda-forge pymol-open-source
```

Do not run `pip install`, `pixi init`, `conda create`, or any long environment
setup as a first move. Do not switch to a different PyMOL installation when the
user already has one.

### Activation dialog

Only the Schrödinger "Incentive PyMOL" build opens an **Activation** window on
launch. Press **Skip Activation** before touching the main window. The
open-source build has no Activation step. If the main window shows
`License File ... For Evaluation Only`, the rendered PNG carries a watermark:
report it and do not call the figure publication-ready.

## Workflow (GUI automation, default)

1. **Check desktop readiness.** The `desktop_*` tools are deferred MCP tools:
   call `search_mcp_tools` with `"desktop_"` first, then call each one through
   `use_mcp_tool` (`tool_name: "desktop_check_permissions"`, etc.). Start with
   `desktop_check_permissions` and `desktop_health_report`. On macOS,
   Accessibility and Screen Recording must be granted to SuperScience. If they
   are missing, stop and ask the user to grant them, then restart the app.
2. **Launch and show the window.** Use `desktop_launch_app` with
   `name: "PyMOL"` (or the open-source `bundle_id:
   "org.pymol.pymol-open-source"`, or `open -a /Applications/PyMOL.app`), then
   `desktop_bring_to_front`. The user should see PyMOL appear.
3. **Dismiss Activation (legacy build only).** If an `Activation` window
   exists, `desktop_get_window_state` it and `desktop_click` `Skip Activation`.
   The open-source build has no such window; do not wait for one.
4. **Read the main window.** `desktop_get_window_state` on the PyMOL window.
   Identify:
   - `AXTextField` labelled `Command Input Area` — where commands are typed.
   - `AXTextArea` at the top — the console/log that echoes commands and errors.
   Never guess pixel coordinates when the accessibility tree exposes these.
5. **Plan once.** State the representation, colors, canvas, camera, and output
   files in one short plan. Confirm only choices that change the figure.
6. **Drive the console.** Prefer `desktop_run_steps` with, per command:
   click the `Command Input Area` field → `desktop_type_text` the command →
   `desktop_press_key` `return`. Batch several commands, then read the console
   `AXTextArea` and check for `Error:` / `Selector-Error`.
   - Background delivery can be refused (`no_focused_text_input`,
     `window_not_key`, `unverifiable`). Re-send the batch with
     `delivery_mode: "foreground"`.
   - `ray` blocks the app briefly. Wait, re-read the window, then re-focus the
     command field before the next batch.
7. **Annotate from the structure, not from memory.** Read the local PDB/mmCIF
   and its `SSBOND` records (after `fetch`, `save structures/<id>.pdb, <obj>`
   to keep a local copy). Never invent residue numbers.
8. **Render and save.** Set `bg_color`, viewport, `ray`, then export with
   explicit pixel dimensions:
   `png /abs/path.png, width=1600, height=1200, dpi=300, ray=1`.
   `viewport` alone follows the window aspect ratio, so a 1600×1200 request can
   come out as 1600×837; the `png ..., width=..., height=..., ray=1` form is
   what guarantees the requested canvas. Save the session with
   `save ... .pse`. Open `log_open figures/<name>.pml` before the commands if
   the user wants PyMOL's own command log; close it with `log_close`.
9. **Write the clean script.** Save `scripts/<name>.py` containing the exact
   commands that were executed in the GUI (the `.pml` log also records GUI
   noise such as viewport bookkeeping, so deliver the clean `.py` as the
   reproducible artifact).
10. **Verify.** Check the console output, confirm the PNG exists and has the
    requested dimensions, and inspect it with the image viewer. Re-focus and
    fix clipping, labels, colors, or camera, then re-render.
11. **Deliver.** Report the PNG, PSE, script, the executed command list, and a
    short description of selections and settings. Leave PyMOL open on the final
    scene unless the user asks to close it.

## Command reference

One command per console entry. All are PyMOL commands, not shell commands.

| Goal | Command |
| --- | --- |
| Load a PDB ID | `fetch 1crn, crambin, async=0` |
| Keep a local copy | `save structures/1crn.pdb, crambin` |
| Load a file | `load /abs/path/input.pdb, target` |
| Clear everything | `reinitialize` |
| Remove solvent | `remove solvent, target` |
| Cartoon | `hide everything, target` then `show cartoon, target` |
| Color | `color grey80, target` |
| White background | `bg_color white` + `set ray_opaque_background, 1` |
| Canvas | `viewport 1600, 1200` (window aspect; not the final pixel size) |
| Frame | `orient target` |
| Disulfide selection | `select ss_cys, byres (resn CYS and name SG)` |
| Disulfide sticks | `show sticks, ss_cys` |
| Disulfide label | `distance ss1, (resi 3 and name SG), (resi 40 and name SG)` |
| Ray trace | `ray 1600, 1200` |
| Save PNG (exact size) | `png /abs/path/figures/1crn.png, width=1600, height=1200, dpi=300, ray=1` |
| Save session | `save /abs/path/figures/1crn.pse` |
| Record script | `log_open /abs/path/figures/1crn.pml` … `log_close` |

For object/chain introspection that has no PyMOL command, use a `python` block
(`python` … one Python line at a time … `python end`) or read the saved PDB
file with the ordinary file tools. Do not invent command names.

## Output conventions

- `figures/<name>.png` — ray-traced image.
- `figures/<name>.pse` — saved PyMOL session.
- `scripts/<name>.py` — clean re-runnable script matching the executed commands.
- `figures/<name>.pml` — optional raw PyMOL command log.
- `figures/<name>.md` — optional caption and selection notes.

Use explicit `ray width,height` and `png` settings. Do not overwrite an
existing figure; add a suffix or ask which name to use.

## Quality rules

- Use a white or explicitly requested background; avoid the default black
  unless the user asks for it.
- Prefer ray tracing for final figures; previews may use a lower quality.
- Keep colors color-blind-safe and consistent across panels.
- Show only the chains, ligands, and residues the user asked for; hide the rest.
- Verify every label and measurement against the loaded structure. Never invent
  residue numbers, distances, angles, or biological conclusions.
- Do not claim a figure is publication-ready until the PNG has been inspected.
- Keep the camera, clipping, and canvas size deterministic; record them in the
  script.
- Mention the evaluation watermark when present.

## Headless fallback

Only when the user explicitly asks for headless/batch work, or when no
`desktop_*` tool is available:

```bash
pymol -cq scripts/<name>.py
pixi run pymol -cq scripts/<name>.py
```

Use `scripts/render_template.py` as the starting point. Keep the script so the
figure can be regenerated without Desktop.

## Availability

If `search_mcp_tools` returns no `desktop_*` tools, this build did not bundle
Cua Driver. In a source checkout run `scripts/fetch_cua_driver.sh` before
building; in an installed app, desktop automation is unavailable for that
package.

## Troubleshooting

- `pymol: command not found` but `/Applications/PyMOL.app` exists — use the
  app; do not install another PyMOL and do not treat it as missing.
- Activation window blocks input — only the legacy build shows this; press
  `Skip Activation` first.
- Background typing refused — send the same batch with
  `delivery_mode: "foreground"`.
- Command appears in the field but does not run — press `return` again after
  re-focusing the field; `ray` may have held the event loop.
- No display / OpenGL error in headless mode — use the GUI path, or check the
  headless install.
- A selection is empty — print or read the chain and residue names before
  changing the representation.
- mmCIF is not recognized — update PyMOL and load with `load path, obj`.
