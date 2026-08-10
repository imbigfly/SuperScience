# Isolated exploration branches

Explorations let you test a scientific direction without changing the project mainline. They are different from ordinary conversation branches: an exploration gets its own persistent workspace and private project records for files, artifacts, runs, decisions, and external resources.

## Start and use an exploration

1. Finish a turn on a native Wisp conversation.
2. Choose **Start exploration** on any completed assistant response that has an immutable project-state revision, name it, and create it.
3. Use the exploration normally. Its banner shows the isolation level and provides **View diff**, **Set as mainline**, **Archive**, and **Discard** actions.
4. Switch between the mainline and sibling explorations from the exploration group under the source conversation in the sidebar.

Wisp records an immutable `ProjectStateRevision` after each successful native mainline turn. A revision freezes the workspace snapshot, conversation archive, Artifact heads, Runs, Decisions, external-resource summary, and stable visual turn index. Workspace blobs are content-addressed, while periodic full manifests and intervening deltas keep revision metadata bounded. This also captures files changed by an external editor before the turn finishes.

Snapshot discovery follows nested `.gitignore` files and project `.wispignore` files using Git ignore syntax, including `!` negation. Use `.wispignore` for files that should remain in the project but never enter exploration history. Wisp also prunes generated metadata and dependency directories such as `.git`, `.pixi`, `.venv`, `node_modules`, `target`, Python caches, and internal `.wisp` artifact/output directories at any depth; source-oriented dot directories such as `.github` are not excluded automatically.

Checkpoint updates are incremental. An unchanged path with the same size and modification time reuses its existing content-addressed blob, while only changed files are read and hashed. Strong file capture is bounded to 32 MiB per file, 64 MiB and 4,096 files per checkpoint; files beyond those bounds remain explicit weak references instead of being copied. Scanning and blob capture are cancellable, so **Stop** can finish a turn even while revision metadata is being recorded.

History created before this feature cannot be reconstructed reliably. Its **Start exploration** action remains visible but disabled; the latest completed turn is still available as a current-state fallback. Context compaction does not change revision turn indices or archived transcripts. ACP conversations cannot be explored yet because ACP v1 has no server-side fork operation.

## Set an exploration as mainline

**Set as mainline** is a fast-forward operation, not a general merge. It is available only while the mainline still exactly matches the exploration checkpoint and the exploration has no active runs or unsupported changed references.

Review the five diff categories before confirming:

- Files
- Artifacts
- Runs
- Decisions
- External effects

If the mainline has advanced, Wisp keeps the exploration available for inspection but refuses promotion. It does not automatically combine conversations or resolve file conflicts.

## Isolation and external effects

Normal project files are copied into the exploration workspace without writable hard links. Ignored paths are absent from the snapshot. Symlinks, devices, sockets, Git metadata, and bounded or large referenced files may be reported as partially isolated. Changing a referenced item blocks fast-forward promotion.

Remote jobs, emails, database writes, MCP/App mutations, and other network-side effects cannot be undone by discarding an exploration. Wisp records these effects and warns before execution, but it does not claim to roll them back.

Archiving preserves a read-only exploration. Discarding removes only its private records and validated app-data workspace; it never deletes the project mainline or sibling explorations.
