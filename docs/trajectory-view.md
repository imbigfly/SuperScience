# Trajectory view

Every conversation has two tabs at the top of the thread area: **Chat** (对话)
and **Trajectory** (轨迹). The trajectory tab is a read-only trace of what the
agent actually did, inspired by the deepseek-harness trajectory UI.

## What it shows

Events are grouped into turns — a turn starts at each of your messages and
covers everything the agent did in response. Each row has a badge and a
one-line summary:

- **USER** — your message that opened the turn.
- **ASSISTANT** — one assistant reply. Click to expand the full text.
- **TOOL** — one tool call, shown as `name {args} → result`. Click to expand
  the full arguments JSON and the complete, untruncated result. Failed calls
  are highlighted in red, and finished calls show their wall-clock duration on
  the right.
- **USAGE** — one model round: input/output/cached tokens for that round.

A search box at the top filters rows by their summary and detail text.

Above each turn, a three-segment bar shows where the turn's wall time went:
**Input** (idle/waiting), **Model** (LLM streaming), and **Tools** (tool
execution), proportional to the recorded durations.

The footer line aggregates the whole session:
turns · steps | LLM time · tool time | output tokens/sec | cache hit rate |
total input/output tokens.

## Where the data comes from

The trajectory is folded from the persisted message log (full tool arguments
and results, which the live chat view truncates for display) and the persisted
UI-event stream (per-round token usage and tool durations). While a turn is
still running, the tab shows lightweight live rows with client-side
timestamps; when the turn finishes, the exact backend snapshot replaces them.

Timestamps come from the `created_at` column on `session_ui_events`; events
persisted before this column existed simply render without timing.
