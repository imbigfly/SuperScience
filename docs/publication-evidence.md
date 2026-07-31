# Publication Evidence Workspace

The Publication Workspace selects the small set of project evidence that
supports a manuscript. It is separate from project backup and Session export.

Open **Publication** from the project sidebar. A Publication contains ordered
manuscript items (Section, Claim, Figure, Table, Methods, and Supplement) and
one or more revisions. Draft revisions can be edited. Frozen and Published
revisions are read-only; use **Clone revision** to continue work without
changing historical evidence.

Registered Artifacts and persisted Runs expose **Use in publication**. The
binding dialog records:

- the exact target revision and manuscript item;
- the evidence purpose and optional supported Claim;
- Candidate, Selected, or Rejected selection state;
- Public, Restricted, or Private visibility.

An Artifact selection is resolved to its current exact `ArtifactVersion` when
the binding is saved. The binding never follows `Artifact.latest_version_id`
afterward. The evidence panel shows the exact source ID, review and
reproduction state, lineage quality, version/checksum details, drift, and
revision-local supersession.

## Freezing

**Freeze** runs dependency and safety checks before making a revision
immutable. Select the intended Capsule visibility and explicitly confirm PHI /
PII and redistribution review where applicable. The readiness panel reports
blockers, warnings, omissions, documented waivers, and the resulting capability
level:

- `archived`
- `traceable`
- `re_executable`
- `reproduced`

Historical live files without trustworthy creation-time checksums are captured
as a new `late_capture` version and reported as
`historical_content_unverified`. They are not rewritten to look like original
Run output.

Public policy never includes Restricted or Private dependency bytes. Those
dependencies remain manifest references or omissions with access instructions.
Frozen evidence and its manifest hash are retained independently of ordinary
Artifact, Session, undo, and garbage-collection operations.

## Building a Capsule

Frozen and Published revisions expose **Build Capsule**. The save produces a
deterministic ZIP derived only from the stored frozen manifest and exact
content-addressed snapshots. It never falls back to a current workspace file.
Every copied blob is streamed through SHA-256 verification before the archive
is published.

The schema-v1 archive contains:

- `capsule.json`, `checksums.sha256`, `README.md`, `REPRODUCE.md`, and
  `CITATION.cff`;
- the exact frozen manifest and selected evidence under `evidence/`;
- Run, input, output, code, and environment lineage under `provenance/`;
- access instructions and reference-only dependencies under `data/`;
- immutable reference results and the verification report.

Entry names, ordering, timestamps, and permissions are normalized. Rebuilding
the same revision from the same immutable blobs produces the same archive
bytes and SHA-256. Public Capsules copy only Public allowlisted bytes;
Restricted and Private dependencies remain metadata and access instructions.
Traversal paths, symlinks, credential-like content, missing snapshots, and
checksum mismatches fail closed. Each attempt is recorded separately from the
frozen revision, including its revision-manifest hash and archive hash.

## Current scope

The first release accepts exact ArtifactVersion and Run evidence. Message
spans, tool calls, code cells, isolated reruns, and result comparators are later
phases. Until a clean rerun passes, the feature is a Publication Evidence /
Traceability Capsule and does not claim full reproducibility.
