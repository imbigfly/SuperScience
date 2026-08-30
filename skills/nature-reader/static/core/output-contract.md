# Output contract

Required outputs:

- `reader.html` — the user-facing bilingual report (always generate)
- `source_map.json` for stable source anchors
- `paper.md` for downstream skills and the math validator
- `translation_notes.md` for terminology, uncertainty, and layout notes
- `assets/` for extracted figures, tables, and equation crops when needed

`reader.html` is the primary deliverable shown to the user. Do not hand-write the HTML page. After `source_map.json` and `assets/` exist, run the bundled builder:

```bash
python <skill-dir>/scripts/build_reader_html.py \
  --source-map source_map.json \
  --output reader.html
```

Do not hide missing information. If the source is incomplete, label the output as draft mode.

## Pre-response verification

Before final response, verify:

- `reader.html` exists next to `source_map.json` and was produced by `build_reader_html.py`
- `paper.md` contains `**Original:**` and `**中文:**` block pairs
- every image/table link used in `paper.md` exists under `assets/`
- every figure/table in `assets/` has a corresponding Markdown block and source pointer
- display equations render inside `$$...$$` (or a fenced `math` block), and inline equations render inside `$...$`
- mathematical content is unchanged across the bilingual explanation: only prose is translated, each display equation is shown once, and Chinese text never uses `(I_0)`-style pseudo-math
- no bare LaTeX commands such as `\\frac`, `\\sum`, or `\\begin{...}` appear as ordinary prose
- every display equation has an `E...` anchor and a matching equation entry in `source_map.json`
- every low-confidence or image-only equation points to an existing file under `assets/equations/`
- `source_map.json` parses as JSON and includes source block IDs
- `translation_notes.md` records skipped, uncertain, or draft-mode content

Run these checks before delivery:

```bash
python <skill-dir>/scripts/validate_reader_math.py paper.md --source-map source_map.json
python <skill-dir>/scripts/build_reader_html.py --source-map source_map.json --output reader.html
```

Add `--strict` on the math validator for a published or reusable artifact.

## Tooling guidance

- If the input is a PDF, load the `pdf` skill first for extraction and OCR guidance.
- Do not call `web-artifacts-builder` or `frontend-design` for this skill. Layout is locked by `build_reader_html.py`.
- In the chat reply, point the user at `reader.html` and any draft gaps. Do not paste the full bilingual text into the conversation.
- If the user wants citation-level grounding to original text, keep the source map explicit and do not lose the page or block IDs.
