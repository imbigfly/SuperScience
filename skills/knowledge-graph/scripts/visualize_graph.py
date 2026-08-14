#!/usr/bin/env python3
"""Render SPO triples as a self-contained interactive HTML graph.

Stdlib only. No network, no API keys, no LLM.
"""

from __future__ import annotations

import argparse
import html
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any


def load_triples(path: Path) -> list[dict[str, Any]]:
    raw = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw, list):
        raise SystemExit("triples JSON must be an array")
    triples: list[dict[str, Any]] = []
    for item in raw:
        if not isinstance(item, dict):
            continue
        subject = str(item.get("subject") or "").strip()
        predicate = str(item.get("predicate") or "").strip()
        obj = str(item.get("object") or "").strip()
        if not subject or not predicate or not obj:
            continue
        triples.append(
            {
                "subject": subject,
                "predicate": predicate,
                "object": obj,
                "inferred": bool(item.get("inferred")),
            }
        )
    return triples


def graph_payload(triples: list[dict[str, Any]]) -> dict[str, Any]:
    degree: Counter[str] = Counter()
    for triple in triples:
        degree[triple["subject"]] += 1
        degree[triple["object"]] += 1
    nodes = [
        {"id": name, "label": name, "degree": count}
        for name, count in sorted(degree.items(), key=lambda item: (-item[1], item[0]))
    ]
    edges = [
        {
            "source": triple["subject"],
            "target": triple["object"],
            "label": triple["predicate"],
            "inferred": triple["inferred"],
        }
        for triple in triples
    ]
    communities: dict[str, int] = {}
    if nodes:
        try:
            communities = _optional_communities(triples)
        except Exception:
            communities = {}
    for node in nodes:
        node["community"] = communities.get(node["id"], 0)
    return {
        "nodes": nodes,
        "edges": edges,
        "stats": {
            "nodes": len(nodes),
            "edges": len(edges),
            "original_edges": sum(1 for edge in edges if not edge["inferred"]),
            "inferred_edges": sum(1 for edge in edges if edge["inferred"]),
        },
    }


def _optional_communities(triples: list[dict[str, Any]]) -> dict[str, int]:
    """Union-find communities; networkx is optional and unused if missing."""
    parent: dict[str, str] = {}

    def find(name: str) -> str:
        parent.setdefault(name, name)
        while parent[name] != name:
            parent[name] = parent[parent[name]]
            name = parent[name]
        return name

    def union(left: str, right: str) -> None:
        a, b = find(left), find(right)
        if a != b:
            parent[b] = a

    for triple in triples:
        union(triple["subject"], triple["object"])
    roots = sorted({find(name) for name in parent})
    index = {root: i for i, root in enumerate(roots)}
    return {name: index[find(name)] for name in parent}


def render_html(payload: dict[str, Any]) -> str:
    data = json.dumps(payload, ensure_ascii=False)
    stats = payload["stats"]
    title = "Knowledge graph"
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{html.escape(title)}</title>
  <style>
    :root {{ color-scheme: light dark; }}
    body {{ margin: 0; font: 14px/1.4 system-ui, sans-serif; background: #f6f5f2; color: #1f1e1c; }}
    header {{ padding: 12px 16px; border-bottom: 1px solid #ddd8ce; display: flex; gap: 16px; flex-wrap: wrap; align-items: baseline; }}
    header strong {{ font-size: 16px; }}
    header span {{ color: #6b6560; }}
    #graph {{ width: 100%; height: calc(100vh - 52px); display: block; }}
    .tip {{ position: fixed; pointer-events: none; background: #1f1e1c; color: #fff; padding: 4px 8px; border-radius: 6px; font-size: 12px; display: none; z-index: 2; }}
  </style>
</head>
<body>
  <header>
    <strong>{html.escape(title)}</strong>
    <span>{stats["nodes"]} nodes · {stats["edges"]} edges · {stats["original_edges"]} original · {stats["inferred_edges"]} inferred</span>
  </header>
  <canvas id="graph"></canvas>
  <div class="tip" id="tip"></div>
  <script>
  const DATA = {data};
  const canvas = document.getElementById("graph");
  const tip = document.getElementById("tip");
  const ctx = canvas.getContext("2d");
  const palette = ["#3b6d9a","#c47b3a","#5a8f5a","#9a4f6b","#6b5b95","#7a6a4a"];
  let nodes = DATA.nodes.map((n, i) => ({{
    ...n,
    x: Math.cos(i) * 180,
    y: Math.sin(i) * 180,
    vx: 0, vy: 0
  }}));
  const byId = Object.fromEntries(nodes.map(n => [n.id, n]));
  const edges = DATA.edges.filter(e => byId[e.source] && byId[e.target]);
  let width = 0, height = 0, dpr = 1, hover = null;

  function resize() {{
    dpr = window.devicePixelRatio || 1;
    width = canvas.clientWidth;
    height = canvas.clientHeight;
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }}
  function radius(node) {{
    return 6 + Math.min(14, Math.sqrt(node.degree || 1) * 3);
  }}
  function step() {{
    for (const e of edges) {{
      const a = byId[e.source], b = byId[e.target];
      const dx = b.x - a.x, dy = b.y - a.y;
      const dist = Math.max(40, Math.hypot(dx, dy));
      const force = (dist - 110) * 0.008;
      const fx = dx / dist * force, fy = dy / dist * force;
      a.vx += fx; a.vy += fy; b.vx -= fx; b.vy -= fy;
    }}
    for (let i = 0; i < nodes.length; i++) {{
      for (let j = i + 1; j < nodes.length; j++) {{
        const a = nodes[i], b = nodes[j];
        let dx = b.x - a.x, dy = b.y - a.y;
        let dist = Math.hypot(dx, dy) || 0.1;
        const force = 420 / (dist * dist);
        const fx = dx / dist * force, fy = dy / dist * force;
        a.vx -= fx; a.vy -= fy; b.vx += fx; b.vy += fy;
      }}
    }}
    for (const n of nodes) {{
      n.vx += (-n.x) * 0.002;
      n.vy += (-n.y) * 0.002;
      n.vx *= 0.86; n.vy *= 0.86;
      n.x += n.vx; n.y += n.vy;
    }}
  }}
  function draw() {{
    ctx.clearRect(0, 0, width, height);
    ctx.save();
    ctx.translate(width / 2, height / 2);
    for (const e of edges) {{
      const a = byId[e.source], b = byId[e.target];
      ctx.beginPath();
      ctx.setLineDash(e.inferred ? [6, 4] : []);
      ctx.strokeStyle = e.inferred ? "#9a8f82" : "#6b6560";
      ctx.lineWidth = 1.2;
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }}
    ctx.setLineDash([]);
    for (const n of nodes) {{
      ctx.beginPath();
      ctx.fillStyle = palette[n.community % palette.length];
      ctx.arc(n.x, n.y, radius(n), 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#1f1e1c";
      ctx.font = "12px system-ui, sans-serif";
      ctx.fillText(n.label, n.x + radius(n) + 4, n.y + 4);
    }}
    ctx.restore();
  }}
  function hit(mx, my) {{
    const x = mx - width / 2, y = my - height / 2;
    for (let i = nodes.length - 1; i >= 0; i--) {{
      const n = nodes[i];
      if (Math.hypot(n.x - x, n.y - y) <= radius(n) + 2) return n;
    }}
    return null;
  }}
  canvas.addEventListener("mousemove", (ev) => {{
    const rect = canvas.getBoundingClientRect();
    hover = hit(ev.clientX - rect.left, ev.clientY - rect.top);
    if (hover) {{
      tip.style.display = "block";
      tip.style.left = ev.clientX + 12 + "px";
      tip.style.top = ev.clientY + 12 + "px";
      tip.textContent = hover.label + " · degree " + hover.degree;
    }} else {{
      tip.style.display = "none";
    }}
  }});
  function tick() {{
    for (let i = 0; i < 2; i++) step();
    draw();
    requestAnimationFrame(tick);
  }}
  window.addEventListener("resize", resize);
  resize();
  tick();
  </script>
</body>
</html>
"""


def write_graph(input_path: Path, output_path: Path) -> dict[str, Any]:
    triples = load_triples(input_path)
    payload = graph_payload(triples)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(render_html(payload), encoding="utf-8")
    return payload


def self_test(tmp: Path) -> None:
    sample = tmp / "triples.json"
    sample.write_text(
        json.dumps(
            [
                {
                    "subject": "Steam engine",
                    "predicate": "enabled",
                    "object": "Factories",
                    "inferred": False,
                },
                {
                    "subject": "Factories",
                    "predicate": "related to",
                    "object": "Urbanization",
                    "inferred": True,
                },
            ],
            ensure_ascii=False,
        ),
        encoding="utf-8",
    )
    out = tmp / "graph.html"
    payload = write_graph(sample, out)
    html_text = out.read_text(encoding="utf-8")
    assert payload["stats"]["nodes"] == 3
    assert payload["stats"]["edges"] == 2
    assert payload["stats"]["inferred_edges"] == 1
    assert "Steam engine" in html_text
    assert "Urbanization" in html_text
    assert "const DATA =" in html_text


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, help="triples JSON array")
    parser.add_argument("--output", type=Path, help="HTML output path")
    parser.add_argument("--self-test", action="store_true", help="run offline fixture checks")
    args = parser.parse_args(argv)
    if args.self_test:
        from tempfile import TemporaryDirectory

        with TemporaryDirectory() as folder:
            self_test(Path(folder))
        print("visualize_graph self-test ok")
        return 0
    if not args.input or not args.output:
        parser.error("--input and --output are required unless --self-test")
    payload = write_graph(args.input, args.output)
    print(json.dumps(payload["stats"], ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
