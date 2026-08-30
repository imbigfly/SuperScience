#!/usr/bin/env python3
"""Build a fixed-layout bilingual HTML report from nature-reader source_map.json.

Stdlib only. No network, no API keys, no LLM.
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from pathlib import Path
from typing import Any

_SCRIPT_DIR = Path(__file__).resolve().parent
_VENDOR_DIR = _SCRIPT_DIR.parent / "static" / "vendor" / "katex"

DISPLAY_DOLLAR_RE = re.compile(r"(?<!\\)\$\$")
INLINE_DOLLAR_RE = re.compile(r"(?<!\\)(?<!\$)\$(?!\$)")

READER_CSS = """
:root {
  --bg: #f6f4ef;
  --paper: #fffdf8;
  --ink: #1f1b16;
  --muted: #5c564e;
  --line: #d8d0c4;
  --accent: #2f5d50;
  --note: #8a5a18;
}
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; background: var(--bg); color: var(--ink);
  font: 16px/1.55 "Iowan Old Style", "Palatino Linotype", Palatino, "Songti SC", serif; }
.wrap { max-width: 1100px; margin: 0 auto; padding: 24px 20px 64px; }
header.mast { background: var(--paper); border: 1px solid var(--line);
  border-radius: 12px; padding: 20px 24px; margin-bottom: 20px; }
header.mast h1 { margin: 0 0 8px; font-size: 1.55rem; }
.meta, .toc a, .src { color: var(--muted); font-size: 0.92rem; }
.toc { margin: 12px 0 0; padding-left: 1.2em; }
.toc li { margin: 2px 0; }
.pair { display: grid; grid-template-columns: 1fr 1fr; gap: 0;
  background: var(--paper); border: 1px solid var(--line); border-top: none; }
.pair:first-of-type, .card + .pair, header.mast + .pair { border-top: 1px solid var(--line);
  border-radius: 12px 12px 0 0; overflow: hidden; }
.pair:last-of-type { border-radius: 0 0 12px 12px; }
.col { padding: 12px 16px; min-width: 0; }
.col.en { border-right: 1px solid var(--line); }
.col .label { display: block; font-size: 0.72rem; letter-spacing: 0.04em;
  text-transform: uppercase; color: var(--muted); margin-bottom: 4px; }
.heading-row { grid-template-columns: 1fr; }
.heading-row .col { font-weight: 650; font-size: 1.12rem; }
.card { background: var(--paper); border: 1px solid var(--line); margin: 16px 0;
  border-radius: 12px; padding: 16px 18px; }
.card img { max-width: 100%; height: auto; display: block; margin: 8px 0; }
.eq { text-align: center; margin: 10px 0; }
.eq img { margin: 8px auto; }
.low { color: var(--note); font-size: 0.9rem; }
.src a { color: var(--accent); text-decoration: none; }
.src a:hover { text-decoration: underline; }
.notes, .glossary { background: var(--paper); border: 1px solid var(--line);
  border-radius: 12px; padding: 16px 18px; margin-top: 20px; }
.notes h2, .glossary h2 { margin: 0 0 10px; font-size: 1.1rem; }
.math-inline { font-style: italic; }
.math-display { display: block; margin: 0.6em 0; overflow-x: auto; text-align: center; }
@media (max-width: 760px) {
  .pair { grid-template-columns: 1fr; }
  .col.en { border-right: none; border-bottom: 1px solid var(--line); }
}
"""

RENDER_JS = """
(function () {
  if (!window.katex || !window.katex.renderToString) return;
  document.querySelectorAll("[data-tex]").forEach(function (el) {
    var tex = el.getAttribute("data-tex") || "";
    var display = el.getAttribute("data-display") === "1";
    try {
      el.innerHTML = window.katex.renderToString(tex, {
        displayMode: display,
        throwOnError: false,
        output: "html"
      });
    } catch (err) {
      el.textContent = tex;
    }
  });
})();
"""


def _strip_font_faces(css: str) -> str:
    return re.sub(r"@font-face\{[^{}]*\}", "", css)


def _window_katex(js: str) -> str:
    return re.sub(r"export\{f4 as k\};?\s*$", "window.katex=f4;", js.strip())


def load_vendor() -> tuple[str, str]:
    css_path = _VENDOR_DIR / "katex.css"
    js_path = _VENDOR_DIR / "katex.js"
    if not css_path.is_file() or not js_path.is_file():
        raise SystemExit(f"KaTeX vendor files missing under {_VENDOR_DIR}")
    css = _strip_font_faces(css_path.read_text(encoding="utf-8"))
    js = _window_katex(js_path.read_text(encoding="utf-8"))
    return css, js


def load_source_map(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"Cannot parse source map: {exc}") from exc
    if not isinstance(data, dict):
        raise SystemExit("source_map.json root must be an object")
    return data


def _text(value: Any) -> str:
    return str(value or "").strip()


def escape_text(value: str) -> str:
    return html.escape(value, quote=True)


def render_prose(text: str) -> str:
    """Turn $...$ / $$...$$ into data-tex spans; leave other text escaped."""
    if not text:
        return ""
    parts: list[str] = []
    tokens = list(DISPLAY_DOLLAR_RE.finditer(text))
    cursor = 0
    display_spans: list[tuple[int, int, str]] = []
    for index in range(0, len(tokens) - 1, 2):
        start = tokens[index].start()
        end = tokens[index + 1].end()
        body = text[tokens[index].end() : tokens[index + 1].start()].strip()
        display_spans.append((start, end, body))
    display_iter = iter(display_spans)
    current = next(display_iter, None)
    while cursor < len(text):
        if current and cursor == current[0]:
            parts.append(_math_html(current[2], display=True))
            cursor = current[1]
            current = next(display_iter, None)
            continue
        limit = current[0] if current else len(text)
        chunk = text[cursor:limit]
        parts.append(_render_inline_math(chunk))
        cursor = limit
    return "".join(parts)


def _render_inline_math(text: str) -> str:
    marks = list(INLINE_DOLLAR_RE.finditer(text))
    if len(marks) < 2:
        return escape_text(text).replace("\n", "<br>")
    out: list[str] = []
    cursor = 0
    for index in range(0, len(marks) - 1, 2):
        start = marks[index].start()
        end = marks[index + 1].end()
        out.append(escape_text(text[cursor:start]).replace("\n", "<br>"))
        body = text[marks[index].end() : marks[index + 1].start()]
        out.append(_math_html(body, display=False))
        cursor = end
    out.append(escape_text(text[cursor:]).replace("\n", "<br>"))
    return "".join(out)


def _math_html(tex: str, display: bool) -> str:
    mode = "1" if display else "0"
    klass = "math-display" if display else "math-inline"
    return (
        f'<span class="{klass}" data-tex="{escape_text(tex)}" '
        f'data-display="{mode}">{escape_text(tex)}</span>'
    )


def relative_asset(path: str) -> str:
    cleaned = path.strip().replace("\\", "/")
    if cleaned.startswith(("http://", "https://", "data:")):
        return ""
    return cleaned.lstrip("./")


def figure_lookup(data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    figures: dict[str, dict[str, Any]] = {}
    for item in data.get("figures") or []:
        if isinstance(item, dict) and _text(item.get("id")):
            figures[_text(item["id"])] = item
    for item in data.get("blocks") or []:
        if not isinstance(item, dict):
            continue
        ident = _text(item.get("id"))
        if item.get("type") in {"figure", "table"} and ident:
            figures.setdefault(ident, {}).update(item)
    return figures


def equation_lookup(data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    equations: dict[str, dict[str, Any]] = {}
    for item in data.get("blocks") or []:
        if isinstance(item, dict) and item.get("type") == "equation" and _text(item.get("id")):
            equations[_text(item["id"])] = dict(item)
    for item in data.get("equations") or []:
        if isinstance(item, dict) and _text(item.get("id")):
            ident = _text(item["id"])
            equations[ident] = {**equations.get(ident, {}), **item}
    return equations


def placement_map(figures: dict[str, dict[str, Any]]) -> dict[str, list[str]]:
    placed: dict[str, list[str]] = {}
    for ident, figure in figures.items():
        after = _text(figure.get("placed_after") or figure.get("insert_after"))
        if after:
            placed.setdefault(after, []).append(ident)
    return placed


def ordered_blocks(data: dict[str, Any]) -> list[dict[str, Any]]:
    blocks = [item for item in (data.get("blocks") or []) if isinstance(item, dict)]
    blocks.sort(key=lambda item: (item.get("order") is None, item.get("order") or 0))
    return blocks


def paper_meta(data: dict[str, Any]) -> dict[str, str]:
    paper = data.get("paper") if isinstance(data.get("paper"), dict) else {}
    title = _text(paper.get("title")) or "中英对照阅读报告"
    venue = _text(paper.get("venue"))
    source = _text(paper.get("source_path") or paper.get("source_type"))
    return {"title": title, "venue": venue, "source": source}


def toc_html(data: dict[str, Any], blocks: list[dict[str, Any]]) -> str:
    items: list[str] = []
    pages = data.get("pages") if isinstance(data.get("pages"), list) else []
    if pages:
        for page in pages:
            if not isinstance(page, dict):
                continue
            number = page.get("page")
            if not isinstance(number, int):
                continue
            first = ""
            for ident in page.get("block_ids") or []:
                if isinstance(ident, str) and ident:
                    first = ident
                    break
            href = f"#{escape_text(first)}" if first else f"#page-{number}"
            items.append(f'<li><a href="{href}">p.{number}</a></li>')
    else:
        for block in blocks:
            if block.get("type") != "heading":
                continue
            ident = _text(block.get("id"))
            label = _text(block.get("original_text") or block.get("translation") or ident)
            if ident:
                items.append(f'<li><a href="#{escape_text(ident)}">{escape_text(label)}</a></li>')
    if not items:
        return ""
    return '<ol class="toc">' + "".join(items) + "</ol>"


def source_line(block: dict[str, Any]) -> str:
    ident = _text(block.get("id"))
    page = block.get("page")
    bits = []
    if isinstance(page, int):
        bits.append(f"p.{page}")
    if ident:
        bits.append(ident)
    label = " · ".join(bits) if bits else ident
    if ident:
        return (
            f'<p class="src"><a href="#{escape_text(ident)}">'
            f"{escape_text(label)}</a></p>"
        )
    return f'<p class="src">{escape_text(label)}</p>' if label else ""


def pair_html(block: dict[str, Any]) -> str:
    ident = _text(block.get("id"))
    original = _text(block.get("original_text"))
    translation = _text(block.get("translation"))
    heading = block.get("type") == "heading"
    row_class = "pair heading-row" if heading else "pair"
    anchor = f' id="{escape_text(ident)}"' if ident else ""
    if heading:
        title = original or translation or ident
        return (
            f'<article class="{row_class}"{anchor}>'
            f'<div class="col">{source_line(block)}{render_prose(title)}</div>'
            f"</article>"
        )
    return (
        f'<article class="{row_class}"{anchor}>'
        f'<div class="col en"><span class="label">Original</span>'
        f"{source_line(block)}{render_prose(original)}</div>"
        f'<div class="col zh"><span class="label">中文</span>'
        f"{render_prose(translation)}</div>"
        f"</article>"
    )


def figure_card(ident: str, figure: dict[str, Any]) -> str:
    title = _text(figure.get("alt_text") or figure.get("translation") or ident)
    caption_en = _text(figure.get("original_text") or figure.get("original_caption"))
    caption_zh = _text(figure.get("translation") or figure.get("caption_translation"))
    image = relative_asset(_text(figure.get("image_path")))
    img = f'<img src="{escape_text(image)}" alt="{escape_text(title)}">' if image else ""
    return (
        f'<section class="card" id="{escape_text(ident)}">'
        f"<h3>{escape_text(title)}</h3>"
        f"{source_line(figure)}"
        f"{img}"
        f'<p><span class="label">Original caption</span> {render_prose(caption_en)}</p>'
        f'<p><span class="label">中文图注</span> {render_prose(caption_zh)}</p>'
        f"</section>"
    )


def equation_card(ident: str, equation: dict[str, Any]) -> str:
    latex = _text(equation.get("latex"))
    image = relative_asset(_text(equation.get("image_path")))
    confidence = _text(equation.get("confidence")) or "high"
    number = _text(equation.get("equation_number"))
    bits = [ident]
    if number:
        bits.append(f"Eq. ({number})")
    body = []
    if image and confidence == "low":
        body.append(f'<img src="{escape_text(image)}" alt="{escape_text(ident)}">')
        body.append('<p class="low">低置信度转写 / Low-confidence transcription; 以原图为准。</p>')
    if latex:
        body.append(f'<div class="eq">{_math_html(latex, display=True)}</div>')
    elif image and confidence != "low":
        body.append(f'<img src="{escape_text(image)}" alt="{escape_text(ident)}">')
    return (
        f'<section class="card eq-card" id="{escape_text(ident)}">'
        f"<h3>{escape_text(' · '.join(bits))}</h3>"
        f"{source_line(equation)}"
        f"{''.join(body)}"
        f"</section>"
    )


def notes_html(source_map_path: Path, data: dict[str, Any]) -> str:
    chunks: list[str] = []
    notes_path = source_map_path.parent / "translation_notes.md"
    if notes_path.is_file():
        notes = notes_path.read_text(encoding="utf-8").strip()
        if notes:
            chunks.append(
                '<section class="notes"><h2>阅读提示 / Notes</h2>'
                f"<pre>{escape_text(notes)}</pre></section>"
            )
    glossary = [item for item in (data.get("glossary") or []) if isinstance(item, dict)]
    if glossary:
        rows = []
        for item in glossary:
            term = escape_text(_text(item.get("term")))
            trans = escape_text(_text(item.get("translation")))
            note = escape_text(_text(item.get("note")))
            if term or trans:
                rows.append(f"<li><strong>{term}</strong> — {trans} {note}</li>")
        if rows:
            chunks.append(
                '<section class="glossary"><h2>术语表 / Glossary</h2>'
                f"<ul>{''.join(rows)}</ul></section>"
            )
    return "".join(chunks)


def body_html(data: dict[str, Any], source_map_path: Path) -> str:
    blocks = ordered_blocks(data)
    figures = figure_lookup(data)
    equations = equation_lookup(data)
    placements = placement_map(figures)
    used_figures: set[str] = set()
    used_equations: set[str] = set()
    parts: list[str] = []

    for block in blocks:
        ident = _text(block.get("id"))
        kind = block.get("type")
        if kind in {"figure", "table"} and ident:
            parts.append(figure_card(ident, figures.get(ident, block)))
            used_figures.add(ident)
        elif kind == "equation" or ident in equations:
            parts.append(equation_card(ident, equations.get(ident, block)))
            used_equations.add(ident)
        else:
            parts.append(pair_html(block))
        for figure_id in placements.get(ident, []):
            if figure_id not in used_figures:
                parts.append(figure_card(figure_id, figures[figure_id]))
                used_figures.add(figure_id)

    for ident, figure in figures.items():
        if ident not in used_figures:
            parts.append(figure_card(ident, figure))
    for ident, equation in equations.items():
        if ident not in used_equations:
            parts.append(equation_card(ident, equation))

    parts.append(notes_html(source_map_path, data))
    return "".join(parts)


def render_html(data: dict[str, Any], source_map_path: Path) -> str:
    katex_css, katex_js = load_vendor()
    meta = paper_meta(data)
    blocks = ordered_blocks(data)
    title = escape_text(meta["title"])
    meta_bits = [bit for bit in (meta["venue"], meta["source"]) if bit]
    meta_line = escape_text(" · ".join(meta_bits))
    return f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>
{katex_css}
{READER_CSS}
  </style>
</head>
<body>
  <div class="wrap">
    <header class="mast">
      <h1>{title}</h1>
      <p class="meta">{meta_line}</p>
      {toc_html(data, blocks)}
    </header>
    {body_html(data, source_map_path)}
  </div>
  <script>
{katex_js}
  </script>
  <script>
{RENDER_JS}
  </script>
</body>
</html>
"""


def write_reader(source_map: Path, output: Path) -> dict[str, Any]:
    data = load_source_map(source_map)
    output.parent.mkdir(parents=True, exist_ok=True)
    html_text = render_html(data, source_map)
    output.write_text(html_text, encoding="utf-8")
    return {
        "blocks": len(ordered_blocks(data)),
        "figures": len(figure_lookup(data)),
        "equations": len(equation_lookup(data)),
        "output": str(output),
    }


def fixture_map(tmp: Path) -> Path:
    assets = tmp / "assets"
    assets.mkdir(parents=True)
    (assets / "fig1.png").write_bytes(
        bytes.fromhex(
            "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4"
            "890000000a49444154789c63000100000500010d0a2db40000000049454e44ae426082"
        )
    )
    payload = {
        "paper": {
            "title": "Example paper on ferroptosis",
            "venue": "Demo Journal",
            "source_type": "pdf",
            "source_path": "paper.pdf",
        },
        "blocks": [
            {
                "id": "S001",
                "page": 1,
                "type": "heading",
                "order": 1,
                "original_text": "Introduction",
                "translation": "引言",
            },
            {
                "id": "S002",
                "page": 1,
                "type": "paragraph",
                "order": 2,
                "original_text": "The peak intensity is $I_0$ as shown below.",
                "translation": "峰值强度由下方原式中的 $I_0$ 给出。",
            },
            {
                "id": "E001",
                "page": 1,
                "type": "equation",
                "order": 3,
                "confidence": "high",
                "latex": "I_0=\\frac{4E_0}{\\pi w_0^2}",
            },
        ],
        "pages": [{"page": 1, "block_ids": ["S001", "S002", "E001"]}],
        "figures": [
            {
                "id": "F001",
                "page": 1,
                "image_path": "assets/fig1.png",
                "placed_after": "S002",
                "alt_text": "Fig. 1",
                "original_text": "A schematic of the assay.",
                "translation": "实验示意图。",
            }
        ],
        "equations": [
            {
                "id": "E001",
                "page": 1,
                "equation_number": "1",
                "latex": "I_0=\\frac{4E_0}{\\pi w_0^2}",
                "confidence": "high",
            }
        ],
        "glossary": [{"term": "ferroptosis", "translation": "铁死亡", "note": ""}],
    }
    path = tmp / "source_map.json"
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    (tmp / "translation_notes.md").write_text("Draft: figure crop is approximate.\n", encoding="utf-8")
    return path


def self_test(tmp: Path) -> None:
    source_map = fixture_map(tmp)
    output = tmp / "reader.html"
    stats = write_reader(source_map, output)
    text = output.read_text(encoding="utf-8")
    assert stats["blocks"] == 3
    assert stats["figures"] == 1
    assert stats["equations"] == 1
    assert 'class="pair' in text
    assert 'id="S001"' in text
    assert 'id="S002"' in text
    assert 'id="E001"' in text
    assert 'id="F001"' in text
    assert 'src="assets/fig1.png"' in text
    assert "window.katex=f4" in text
    assert "data-tex=" in text
    assert not re.search(r"""(?:src|href)=["']https?://""", text, re.I)
    assert "<link " not in text.lower()
    assert "铁死亡" in text
    assert "阅读提示" in text
    assert "@media (max-width: 760px)" in text


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-map", type=Path, help="Path to source_map.json")
    parser.add_argument("--output", type=Path, help="HTML output path")
    parser.add_argument("--self-test", action="store_true", help="run offline fixture checks")
    args = parser.parse_args(argv)
    if args.self_test:
        from tempfile import TemporaryDirectory

        with TemporaryDirectory() as folder:
            self_test(Path(folder))
        print("build_reader_html self-test ok")
        return 0
    if not args.source_map or not args.output:
        parser.error("--source-map and --output are required unless --self-test")
    stats = write_reader(args.source_map, args.output)
    print(json.dumps(stats, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
