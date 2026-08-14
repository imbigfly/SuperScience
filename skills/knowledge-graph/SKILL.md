---
name: knowledge-graph
description: >-
  Build an interactive knowledge graph from unstructured text: papers, notes,
  methods, or pasted excerpts. Use when the user asks for a 知识图谱, knowledge
  graph, entity-relationship graph, SPO triples, subject-predicate-object
  extraction, or to visualize how concepts in a document connect. Do not use
  for the project Research Graph (questions, decisions, runs) or for
  statistical modeling of numeric tables.
license: Apache-2.0
---

# Text knowledge graph

Turn a document into Subject-Predicate-Object triples, then a local interactive
HTML graph. Extraction uses this conversation's model. Visualization is the
offline script [scripts/visualize_graph.py](scripts/visualize_graph.py).

Method adapted from
[robert-mcdermott/ai-knowledge-graph](https://github.com/robert-mcdermott/ai-knowledge-graph)
(Apache-2.0). Do not clone that repo or write a separate `config.toml` / API
key. SuperScience does not expose keyring secrets to shell or Python.

## Inputs

Accept pasted text, or an attached `.txt` / `.md` / paper excerpt. If the user
has not provided source text, ask once for the file or paste. Do not invent a
corpus.

## Workflow

1. **Chunk.** Split on whitespace into windows of about 200 words with 20-word
   overlap. For a long file, `read` or extract in segments; never dump the
   whole document into one model turn.
2. **Extract.** For each chunk, list only triples the text supports:
   `{ "subject", "predicate", "object" }`. Keep entity names short and
   consistent. Do not fabricate people, genes, methods, or causal links.
3. **Standardize.** Merge obvious aliases (e.g. `AI` / `artificial intelligence`)
   to one surface form. Record the mapping in a short note if anything changed.
4. **Optional inference.** If the graph has clearly disconnected communities and
   the user wants a denser view, add a few plausible links and set
   `"inferred": true`. Prefer leaving gaps over guessing.
5. **Write JSON** to `knowledge-graph/triples.json`:

```json
[
  {"subject": "Steam engine", "predicate": "enabled", "object": "Factories", "inferred": false}
]
```

6. **Visualize.** Run the bundled script (stdlib only; no network):

```bash
python <skill-dir>/scripts/visualize_graph.py \
  --input knowledge-graph/triples.json \
  --output knowledge-graph/graph.html
```

Replace `<skill-dir>` with the skill path from `use_skill`. Then open the HTML
artifact in the existing preview.

## Output

Deliver:

- `knowledge-graph/triples.json` — all triples, inferred flagged
- `knowledge-graph/graph.html` — interactive graph (hover nodes/edges; inferred
  edges are dashed)
- a short count: nodes, original edges, inferred edges

If extraction is empty, say so and stop. Do not generate a decorative graph
from made-up triples.
