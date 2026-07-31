# Publication Evidence Capsule — Execution Plan

**Issue:** #576  
**Design:** `docs/superpowers/specs/2026-07-31-publication-evidence-capsule-design.md`

## Delivery rule

Each stage below is one reviewable commit and represents one PR-sized phase.
The stage owns its migrations, compatibility code, tests, docs, and transfer
support. Do not mix work from a later stage into an earlier commit merely to
make the UI look complete.

Unrelated `.codex/` and `.playwright-mcp/` workspace files are never staged.

## Stage D — executable design baseline

**Commit:** `docs: define publication evidence capsule execution plan`

- [x] Reconcile issue #576 with current ArtifactVersion, Run, provenance,
  Session export, project transfer, and Research Graph behavior.
- [x] Define authoritative lineage, immutable evidence, late capture, state
  axes, freeze transaction, visibility policy, manifest, retention, and
  capability levels.
- [x] Map every requested phase to one commit and list its verification gate.

## PR 0 — Artifact and Run lineage hardening

**Commit:** `feat(lineage): bind runs to immutable artifact versions`

### Storage and Artifact identity

- [x] Add the reusable streaming SHA-256 snapshot service.
- [x] Replace message-resource in-memory snapshotting with the shared service.
- [x] Add `artifacts.logical_key` and exact version materialization/capture
  metadata through fresh and idempotent migrations.
- [x] Add an atomic Artifact version save API carrying checksum, size,
  producing Run, environment, logical key, and capture metadata.
- [x] Make manual registration and local Run harvest snapshot small files and
  checksum reference-only large files.
- [x] Add `OutputSpec.logical_key`; fall back to normalized project-relative
  path, never filename alone.

### Exact Run lineage

- [x] Add `external_resources`, `run_inputs`, `run_outputs`,
  `run_code_snapshots`, and `run_environment_snapshots`.
- [x] Add basis/confidence to dependency projections.
- [x] Capture declared local Run inputs before execution.
- [x] Store the exact ArtifactVersion for every harvested output.
- [x] Store a SHA-256 command/code snapshot and canonical SHA-256 environment
  snapshot for every new Run.
- [x] Preserve compatibility `run_artifacts` and Research Graph edges.
- [x] Add all new tables to project transfer, replacement deletion, and
  portable database hashing.

### Gate

- [x] Focused wisp-store migration/lineage tests.
- [x] Harvest and Run-context tests with temporary files and fake runners.
- [x] Message-resource snapshot regression tests, including a streamed file.
- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace`

## PR 1 — Publication domain model

**Commit:** `feat(publication): add immutable revision evidence model`

- [x] Add `publications`, `publication_revisions`, `publication_items`,
  `publication_item_links`, `evidence_bindings`, `evidence_reviews`,
  `evidence_supersessions`, `publication_readiness_reports`,
  `publication_waivers`, and `capsule_builds`.
- [x] Validate all enums and ensure every source belongs to the Publication
  project.
- [x] Implement Publication and Draft revision CRUD.
- [x] Implement ordered item CRUD and semantic item links.
- [x] Bind exact ArtifactVersion and Run sources with separate selection,
  review, reproduction, and visibility axes.
- [x] Implement full materialized revision clone, preserving internal
  relationships with new IDs.
- [x] Add SQLite immutability and exact-source constraints for non-Draft
  revisions.
- [x] Project Publication and evidence relations into Research Graph without
  making graph metadata authoritative.
- [x] Add tables to project transfer, deletion order, and portable hashing.

### Gate

- [x] Store tests cover clone completeness, exact IDs, cross-project
  rejection, supersession locality, and frozen write rejection.
- [x] Transfer roundtrip preserves the complete revision.
- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace`

## PR 2 — atomic Freeze, Readiness, drift, and policy

**Commit:** `feat(publication): freeze evidence with readiness manifest`

- [x] Add Draft → internal Freezing → Frozen transaction protocol.
- [x] Validate existing snapshots and prepare immutable late-capture versions
  without rewriting historical versions.
- [x] Resolve selected evidence through exact Run inputs, outputs, code,
  environment, and external resources.
- [x] Generate structured blockers, warnings, waivers, basis/confidence, and
  capability level.
- [x] Enforce Public/Restricted/Private dependency rules.
- [x] Add secret, absolute path, machine detail, internal network, symlink,
  file-size/type, license, redistribution, and PHI/PII review checks.
- [x] Store canonical schema-v1 manifest and SHA-256 only in the final
  transaction.
- [x] Implement drift/successor query without mutating a frozen revision.
- [x] Protect frozen versions from ordinary Artifact/Session deletion.

### Gate

- [x] Failure injection proves no partial Frozen revision.
- [x] Workspace mutation leaves frozen bytes and hash unchanged.
- [x] Historical missing-checksum evidence becomes `late_capture`.
- [x] Public readiness reports restricted dependencies as omissions.
- [x] Canonical manifest generation is deterministic.
- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace`

## PR 3 — Publication Workspace UI

**Commit:** `feat(ui): add publication evidence workspace`

- [x] Add backend commands and typed DTOs for Publication workspace operations.
- [x] Add a project-level Publication Workspace entry.
- [x] Add **Use in publication** to Artifact and Run surfaces.
- [x] Add the binding dialog for revision, item, purpose/claim, selection, and
  visibility.
- [x] Render manuscript tree and evidence detail with exact version, lineage
  quality, review/reproduction state, readiness, drift, and supersession.
- [x] Add clone, freeze, waiver, and refresh actions.
- [x] Integrate all nested surfaces with the window Escape stack.
- [x] Add English and Chinese strings and user-facing documentation.

### Gate

- [x] Playwright opens the binding dialog from an Artifact and a Run.
- [x] Immediate Escape closes only the topmost dialog while Workspace remains.
- [x] Frozen UI has no mutation controls and displays exact source IDs.
- [x] Readiness and late-capture warnings are visible.
- [x] `cd ui && cargo check --target wasm32-unknown-unknown`
- [x] `cd ui-tests && npm ci && npx playwright test`
- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace`

## PR 4 — selective deterministic Capsule

**Commit:** `feat(publication): build selective evidence capsules`

- [ ] Build only from a Frozen/Published stored manifest.
- [ ] Emit schema-v1 `capsule.json`, README, REPRODUCE, CITATION, checksums,
  data/access manifest, evidence, provenance, reference results, and
  verification report.
- [ ] Include allowlisted Public immutable bytes only.
- [ ] Emit Restricted/Private dependencies as omissions and access
  instructions.
- [ ] Verify each copied blob against its frozen SHA-256.
- [ ] Normalize entry ordering, archive timestamps, permissions, and paths.
- [ ] Record Capsule Build result separately from the revision manifest.
- [ ] Reject traversal, symlinks, secrets, and live workspace fallbacks.

### Gate

- [ ] Two builds of one revision have the same revision manifest hash and
  normalized archive content.
- [ ] Mutated live files do not affect the capsule.
- [ ] No Restricted/Private bytes appear in a Public build.
- [ ] Corrupt or missing immutable blobs fail closed.
- [ ] Windows-style and macOS/POSIX paths remain portable.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace`

## PR 5 — fine-grained anchors and isolated verification

**Commit:** `feat(publication): verify fine-grained evidence in isolation`

- [ ] Add immutable MessageSpan, ToolCall, CodeCell, ExecutionLog, and
  ExternalResource anchors.
- [ ] Add selection entry points for a message span, tool result, and code
  cell.
- [ ] Add comparator contracts: SHA-256, text, JSON, and numeric tolerance.
- [ ] Add persisted reproduction runs/results and actual environment capture.
- [ ] Materialize a fresh temporary workspace from capsule allowlisted inputs.
- [ ] Execute through the structured runner with a fake-runner test boundary.
- [ ] Compare produced outputs and update only the new reproduction report,
  never the frozen evidence.
- [ ] Promote capability to `reproduced` only when the environment contract and
  every required comparator pass.
- [ ] Surface reproduction details in Publication Workspace.

### Gate

- [ ] Fine-grained anchors remain stable after message/file/tool changes.
- [ ] Verification cannot read an undeclared project file.
- [ ] Exact and tolerant comparisons cover pass/fail cases.
- [ ] Missing environment parity prevents a false `reproduced` claim.
- [ ] Automated tests require no network, SSH, WSL, GPU, scheduler, or API key.
- [ ] `cd ui && cargo check --target wasm32-unknown-unknown`
- [ ] `cd ui-tests && npm ci && npx playwright test`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace`

## Final completion audit

- [ ] Inspect every issue requirement and all 13 design acceptance items
  against authoritative schema, code, tests, transfer artifacts, and rendered
  UI.
- [ ] Run the full Rust, WASM, and Playwright suites.
- [ ] Document manual smoke steps, limitations, and follow-up work.
- [ ] Confirm commit history contains one stage per commit and no unrelated
  workspace files.
