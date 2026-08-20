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


_SCRIPT_DIR = Path(__file__).resolve().parent
_HTML_TEMPLATE = """<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>__TITLE__</title>
  <style>
__CSS__
  </style>
</head>
<body>
  <header>
    <strong>__TITLE__</strong>
    <span class="meta">__STATS__</span>
    <input id="node-search" type="search" placeholder="搜索节点" autocomplete="off" />
    <span class="hint">滚轮缩放 · 单击高亮邻域 · 拖节点可固定 · 拖空白平移 · Esc 清除</span>
    <div class="zoom">
      <button type="button" id="zoom-out" title="缩小">−</button>
      <button type="button" id="zoom-in" title="放大">+</button>
      <button type="button" id="zoom-fit" title="适应窗口">适应</button>
      <button type="button" id="zoom-reset" title="复位">复位</button>
    </div>
  </header>
  <div id="graph-stage">
    <canvas id="graph"></canvas>
  </div>
  <div class="tip" id="tip"></div>
  <script>window.GRAPH_DATA = __DATA__;</script>
  <script>
__JS__
  </script>
</body>
</html>
"""


def _viewer_asset(name: str) -> str:
    return (_SCRIPT_DIR / name).read_text(encoding="utf-8")


def render_html(payload: dict[str, Any]) -> str:
    stats = payload["stats"]
    title = "文本知识图谱"
    return (
        _HTML_TEMPLATE.replace("__TITLE__", html.escape(title))
        .replace(
            "__STATS__",
            f'{stats["nodes"]} 节点 · {stats["edges"]} 边 · {stats["original_edges"]} 原文 · {stats["inferred_edges"]} 推断',
        )
        .replace("__CSS__", _viewer_asset("graph_viewer.css"))
        .replace("__JS__", _viewer_asset("graph_viewer.js"))
        .replace("__DATA__", json.dumps(payload, ensure_ascii=False))
    )


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
    assert "window.GRAPH_DATA =" in html_text
    assert 'id="zoom-in"' in html_text
    assert 'id="zoom-out"' in html_text
    assert 'id="zoom-fit"' in html_text
    assert 'id="zoom-reset"' in html_text
    assert 'id="node-search"' in html_text
    assert "fitNodes" in html_text
    assert "applySearch" in html_text
    assert "inNeighborhood" in html_text
    assert "hitEdge" in html_text
    assert "DRAG_THRESHOLD" in html_text
    assert "graph-stage" in html_text
    assert "70vh" not in html_text
    assert "html, body { height: 100%; }" in html_text
    assert "flex: 1 1 auto" in html_text


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
