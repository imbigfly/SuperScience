# Headless agent testing

Wisp has two complementary headless interfaces:

- `wisp-science eval` runs repeatable agent conformance and regression suites.
- `wisp-science rpc` runs a long-lived agent over a versioned JSONL stdin/stdout protocol.

Both exercise the production `Agent` loop and tool registry. The default eval
suite uses a deterministic scripted provider, temporary workspaces, and fixture
MCP/subagent boundaries, so it requires no API key, network, SSH host, GPU,
scheduler, Python, or R installation.

## Offline evaluation

Run the built-in suite:

```bash
cargo run -p wisp-cli -- eval \
  --artifacts target/headless-agent-eval \
  --save target/headless-agent-eval/report.json
```

The suite covers reads and exact edits, shell execution, persistent Python and
R runtime cells, approval denial, skills, deferred MCP discovery, read-only
subagent delegation, resume without tool replay, session restart, queued
guidance, cancellation, vision fallback, manual compaction, plan-mode gating,
and project path containment.

Useful selection and stress controls:

```bash
# Select by stable case id or require a tag.
cargo run -p wisp-cli -- eval --case targeted-edit --tag filesystem

# Detect nondeterminism and exercise concurrent workspace isolation.
cargo run -p wisp-cli -- eval --repeat 20 --parallel 4

# Preserve only failed temporary projects for investigation.
cargo run -p wisp-cli -- eval \
  --keep-failed-workspace --artifacts target/headless-agent-eval
```

Every trajectory is JSONL with schema `wisp.agent-trajectory.v1`. It includes
the full provider requests, messages, tool call IDs, parsed tool arguments,
tool results, approvals, compaction events, and usage. The JSON summary uses
`wisp.agent-eval-report.v1`.

### Budgets and baselines

The runner fails when a selected case fails, a budget is exceeded, the pass
rate is below the requested threshold, or a baseline regression crosses a
configured threshold:

```bash
cargo run -p wisp-cli -- eval \
  --max-tool-calls 8 \
  --max-input-tokens 200000 \
  --max-duration-ms 15000 \
  --max-cost-microusd 50000 \
  --input-cost-microusd-per-million 3000000 \
  --output-cost-microusd-per-million 15000000

cargo run -p wisp-cli -- eval --save baseline.json
cargo run -p wisp-cli -- eval \
  --compare baseline.json \
  --max-token-regression-percent 10 \
  --max-round-regression 2 \
  --save current.json
```

Cost rates are explicit integer micro-USD per million tokens; offline defaults
are zero. Cached input is excluded from the billable input estimate.

Use `--mode live` to run the same declarative suite against the configured
provider. Live mode requires the normal `WISP_API_KEY`, `WISP_PROVIDER`, and
`WISP_MODEL` environment variables. Scripted steps are ignored in live mode;
live suites should use tolerant semantic assertions rather than exact prose.

### Custom suites

Pass a YAML or JSON file with `--suite`. The top-level schema is
`wisp.agent-eval-suite.v1`:

```yaml
schema: wisp.agent-eval-suite.v1
id: project-smoke-v1
defaults:
  timeout_ms: 15000
  max_rounds: 8
cases:
  - id: inspect-config
    description: Read the fixture and report its mode.
    tags: [filesystem, smoke]
    prompt: Read config.toml and report mode.
    files:
      config.toml: "mode = \"safe\"\n"
    allowed_tools: [read, attempt_completion]
    script:
      - tool_calls:
          - id: read-1
            name: read
            arguments: {path: config.toml}
      - tool_calls:
          - id: done-1
            name: attempt_completion
            arguments: {result: "mode=safe"}
    expect:
      outcome: success
      completion_contains: [mode=safe]
      required_tools: [read, attempt_completion]
      forbidden_tools: [write, edit, shell]
      tool_order: [read, attempt_completion]
      tool_args:
        - {name: read, pointer: /path, equals: config.toml}
```

Fixture paths must be relative and remain under the temporary project.
`allowed_tools` is an exact capability allowlist. Binary fixtures use
`base64_files`. Multi-turn lifecycle scenarios use `actions` (`send`, `resume`,
`compact`, and `restart`).

## One-shot JSONL

`run --output jsonl` emits `wisp.agent-event.v1`. Every line contains a
monotonic `sequence`, `session_id`, `turn_id`, and event `type`. Tool events
include the provider call ID and full parsed arguments when available. Setup
diagnostics go to stderr, leaving stdout machine-readable.

```bash
cargo run -p wisp-cli -- run --output jsonl "Inspect this project"
```

This mode is intentionally non-interactive: an approval request is emitted and
denied. Use RPC when the controller must answer approvals or cancel a turn.

## Bidirectional RPC

Start a persistent process with the normal provider configuration:

```bash
cargo run -p wisp-cli -- rpc
```

Commands are one JSON object per stdin line. All commands must use
`wisp.agent-rpc.v1` and a caller-defined unique `id`:

```json
{"schema":"wisp.agent-rpc.v1","id":"turn-1","type":"prompt","prompt":"Inspect README.md"}
{"schema":"wisp.agent-rpc.v1","id":"ping-1","type":"ping"}
{"schema":"wisp.agent-rpc.v1","id":"cancel-1","type":"cancel"}
{"schema":"wisp.agent-rpc.v1","id":"approval-1","type":"approval_response","approval_id":"<event approval_id>","approved":false,"feedback":"Do not mutate files"}
{"schema":"wisp.agent-rpc.v1","id":"shutdown-1","type":"shutdown"}
```

The process first emits `ready`. A prompt produces `turn_started`, streaming
message/text/reasoning/tool/usage events, then exactly one `turn_completed`.
While a turn is active the controller may send `ping`, `cancel`,
`approval_response`, or `shutdown`; a second prompt is rejected. Every event
has the schema, sequence, process `session_id`, and relevant `command_id`.

When a tool needs confirmation, Wisp emits:

```json
{"schema":"wisp.agent-rpc.v1","type":"approval_required","approval_id":"...","message":"Run tool 'write'?", "command_id":"turn-1"}
```

The agent remains suspended without blocking the command reader until a
matching `approval_response`, cancellation, shutdown, or stdin closure. Unknown
schemas and malformed input produce `protocol_error` events without terminating
the process.

## CI contract

The offline suite runs on Linux, macOS, and Windows. Tests must never add a
dependency on a real remote host, scheduler, API key, or language runtime.
Platform integrations should use fake command runners and parsing tests; live
provider suites belong in separately credentialed, non-blocking jobs.
