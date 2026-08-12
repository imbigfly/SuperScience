# Isolated exploration branches

Explorations let you test a scientific direction without changing the project mainline. They are different from ordinary conversation branches: an exploration gets its own persistent workspace and private project records for files, artifacts, runs, decisions, and external resources.

## Start and use an exploration

1. Finish a turn on a native Wisp conversation.
2. Choose **Start exploration** on the latest completed assistant response, name it, and create it.
3. Use the exploration normally. Its banner shows the isolation level and provides **View diff**, **Set as mainline**, **Archive**, and **Discard** actions.
4. Switch between the mainline and sibling explorations from the exploration group under the source conversation in the sidebar.

The first candidate opens an exploration round. Additional candidates created from the same mainline head reuse the same immutable checkpoint, while each receives an independent workspace. While any candidate in the current round is active, Wisp freezes the source conversation and every mainline workspace write in that project; another conversation cannot open a competing round. You can still create and use other conversations for discussion and read-only project inspection. Promote one candidate to finish the round, or archive/discard every candidate to abandon it and resume normal project writes. Promotion automatically archives the other candidates from that round.

Wisp records an immutable `ProjectStateRevision` after each successful native mainline turn. A revision freezes the workspace snapshot, conversation archive, Artifact heads, Runs, Decisions, external-resource summary, and stable visual turn index. Workspace blobs are content-addressed, while periodic full manifests and intervening deltas keep revision metadata bounded. This also captures files changed by an external editor before the turn finishes.

Snapshot discovery follows nested `.gitignore` files and project `.wispignore` files using Git ignore syntax, including `!` negation. Use `.wispignore` for files that should remain in the project but never enter exploration history. Wisp also prunes generated metadata and dependency directories such as `.git`, `.pixi`, `.venv`, `node_modules`, `target`, Python caches, and internal `.wisp` artifact/output directories at any depth; source-oriented dot directories such as `.github` are not excluded automatically.

Checkpoint updates are incremental. An unchanged path with the same size and modification time reuses its existing content-addressed blob, while only changed files are read and hashed. Strong file capture is bounded to 32 MiB per file, 64 MiB and 4,096 files per checkpoint; files beyond those bounds remain explicit weak references instead of being copied. Scanning and blob capture are cancellable, so **Stop** can finish a turn even while revision metadata is being recorded.

Older completed turns keep a disabled **Start exploration** action: exploration rounds intentionally start only at the current head so every candidate remains promotable. Context compaction does not change archived transcripts. ACP conversations cannot be explored yet because ACP v1 has no server-side fork operation.

## Ordinary conversation branches

Ordinary conversation branches use a branch icon in the sidebar and also appear directly below the message checkpoint where they were created. The main conversation remains free to continue while every branch develops its own later context.

Right-click a branch and choose **Merge back** when its focused work is ready. Wisp reads only the branch messages created after its checkpoint and drafts a self-contained summary. The user reviews and edits that draft; the approved text is appended to the current end of main as normal readable conversation context. Mainline turns created after the checkpoint are never compared, truncated, replaced, or included in the branch summary.

Conversation branches are one level deep and merge only once. After merge-back, the branch is frozen as read-only history: it cannot accept new turns, create another branch, rewind, or merge again. On main, the summary is projected as a compact **Merged branch result** card beneath the branch's original checkpoint instead of expanded at the tail. Clicking the card opens the complete Markdown. This is presentation only: the underlying assistant message remains at its real append position as ordinary mainline context that later model turns can read.

Rewind follows that real append position, not the card's visual location. Rewinding main past a merge revokes it and reopens the branch. Rewinding past the branch checkpoint keeps the branch as frozen history, removes its checkpoint attachment, and prevents it from merging into the rewritten mainline.

The summary draft supports **Regenerate** and **Guided generation**. Regenerate creates a fresh draft from only the post-checkpoint branch changes. Guided generation collects user guidance in a separate dialog and creates a new version from three explicit sections: Changes, Current version, and User guidance. A generated version replaces only the pending draft and is never appended to main automatically.

**Delete branch** remains available for an individual branch. The former **Compare branches**, **Make independent**, and destructive family-convergence actions are no longer part of the conversation-branch workflow.

These actions affect conversation history only. They do not merge, restore, or roll back project files, Runs, Artifacts, or external side effects. Use an isolated exploration when those project-level changes must be compared and promoted together.

## Set an exploration as mainline

**Set as mainline** is a fast-forward operation, not a general merge. It is available only while the mainline still exactly matches the exploration checkpoint and the exploration has no active runs or unsupported changed references.

Review the five diff categories before confirming:

- Files
- Artifacts
- Runs
- Decisions
- External effects

Wisp blocks its own mainline writes during the round. External editors and processes are outside that lock, so promotion still rescans the project and refuses to proceed if the mainline no longer matches the checkpoint. It does not automatically combine conversations or resolve file conflicts.

## Isolation and external effects

Normal project files are copied into the exploration workspace without writable hard links. Ignored paths are absent from the snapshot. Symlinks, devices, sockets, Git metadata, and bounded or large referenced files may be reported as partially isolated. Changing a referenced item blocks fast-forward promotion.

Remote jobs, emails, database writes, MCP/App mutations, and other network-side effects cannot be undone by discarding an exploration. Wisp records these effects and warns before execution, but it does not claim to roll them back.

Archiving preserves a read-only exploration. Archiving or discarding every candidate abandons the round and releases the mainline; a stale archived candidate cannot later be restored after the mainline advances. Discarding removes only its private records and validated app-data workspace; it never deletes the project mainline or sibling explorations.
