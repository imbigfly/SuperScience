# Kernel-reported writes — implementation notes

Tracking notes while implementing `docs/superpowers/plans/2026-08-20-kernel-reported-writes.md` (#937).

## What landed

- `python/kernel_worker.py` registers one `sys.addaudithook` at startup. Per-cell `begin()`/`finish()` collects write-intent `open` paths and `os.rename`/`os.replace` destinations, reports only paths that exist as files, and omits `files_written` entirely once 512 distinct paths are exceeded.
- `python/test_kernel_worker.py` drives a real spawned worker for computed names, C-level saves (numpy/matplotlib present here), append, read-only, failed open, write-then-raise, sqlite3 blind spot, and the cap.
- `KernelResp`/`RawResp`/`read_response` round-trip `files_written` as `Option`. Host `project_relative_writes` canonicalizes, drops outside-root and the root itself, normalizes `\`, sorts, and dedups. Only `LOCAL_CONTEXT_ID` kernels call `ToolEnv::report_written_paths`.
- `ToolEnv::report_written_paths` default no-op. `ToolEnvAdapter` accumulates into an interior mutex and `take_reported_writes` drains it. `agent_loop_inner` drains after every `tools.run` and unions reported paths after unmodified `retain_unambiguous_writes`.
- `union_paths_by_identity` keeps the first spelling for one identity. Engine tests cover the #937 repro, the unreported-path control, spelling, and empty-report no-op.
- CI: Python worker tests on the 3-OS `headless-agent-eval` job after MCP regression. `scripts/probe_write_audit_hook.py` is a standalone probe.

## Post-review fixes (2026-08-20)

- **Bytes path killed the worker:** `open(b'x','wb')` left `bytes` in the
  report and `json.dumps` raised, crashing the process and losing the
  session's kernel. `_note` now `os.fsdecode`s bytes and requires strict
  UTF-8 round-trip; unrepresentable paths are skipped (the host's snapshot
  inference still covers them — reports only ever add).
- **Non-UTF-8 filename broke the protocol:** a surrogate-escaped name
  serialized as a lone `\udcff` escape, which serde_json rejects
  ("lone leading surrogate in hex escape"), losing the whole result frame.
  Covered by the same strict-UTF-8 gate. Both cases have worker tests.
- **Traversal guard:** `project_relative_writes` drops any remainder with a
  non-`Normal` component — when canonicalize fails (path deleted between
  report and host processing), a raw `/root/../etc/x` could otherwise strip
  to `../etc/x` and reach undo.
- **Unix backslash filenames:** separator normalization is now
  Windows-only. On Unix `\` is a legal filename character; replacing it
  could canonicalize to a *different* existing file (`we\ird.txt` vs
  `we/ird.txt`) and credit a file the cell never wrote.
- `union_paths_by_identity` only re-sorts when it inserted something, so an
  all-duplicate report leaves the record byte-identical.
- CI worker-test steps renamed `(Unix)` / `(Windows)`.

**Documented limitation for the PR body** (deliberately not "fixed"): a
reported path that the diff never saw (e.g. `open(p, 'r+')` with nothing
written) is still credited. Same-conversation only, and undo marks it
non-reversible. Intersecting reports with the diff would forfeit real
credit for rewrites the mtime-granularity snapshot cannot see.

## Post-merge fixes (2026-08-21)

- **Reports bypassed the snapshot's skipped directories.** A cell that only
  imported a project-local module reported the `__pycache__` bytecode the
  import wrote, and `project_relative_writes` confines to the project root
  but knows nothing about `SKIP_DIRS`. The path reached the Generated card,
  `execution_log`, session exports, and publication capsules — places the
  snapshot diff could never put it. `provenance::union_reported_writes` now
  applies the snapshot's own rule (skip by directory name at any depth, only
  parent components gate inclusion) before the identity union, and
  `agent_loop_inner` calls it instead of `union_paths_by_identity`.
- **Neither wiring point was covered.** Deleting the `agent.rs` union left all
  160 `wisp-core` tests green, and deleting `env.report_written_paths(&paths)`
  left all 40 `wisp-runtime` tests green: the whole feature could stop working
  silently. The `#937` engine test called `retain_unambiguous_writes` and the
  union directly rather than driving the loop. Added two `agent_loop` tests
  (report folded into the record, snapshot-skipped paths excluded, no leak into
  the next tool call) and lifted the runtime's forwarding branch into
  `report_local_writes` so the local-only gate and the absent-vs-empty
  distinction are testable against a recording `ToolEnv`. All five mutants —
  removed union, removed skip filter, removed report call, ungated context,
  non-draining buffer — now fail a test.

## Deviations

- Windows CI invokes `python` instead of `python3` (sibling step on the same matrix job). `python3` is not on PATH on `windows-latest`; Unix steps keep the planned `python3` command so the worker tests still run on all three OSes.
- Did not `cargo fmt --all` the workspace: check fails on pre-existing drift in `src-tauri/src/lib.rs` and `crates/wisp-mcp/src/client.rs`. Formatting those would violate the empty `src-tauri` diff. Touched crates were formatted with `cargo fmt -p wisp-core -p wisp-runtime -p wisp-tools`.
- `cargo test -p wisp-tauri` failed here: `libsoup-3.0` / WebKit pkg-config missing. Crate-level tests in steps 2–5 are the accepted bar.
