# Agent delegation

Delegation lets the main Agent create a bounded set of temporary sub-Agents,
run independent work in parallel, and synthesize their evidence either in the
same turn or after durable background completion. Codex and ACP are optional
executor choices; neither is part of the meaning of a code-capable Agent.

## Quick Actions and Workflow templates

Quick Actions are contextual entry points into reusable Workflow templates.
They do not send a prompt asking the main Agent to invent a plan. Invoking one
instantiates its stored task graph, persists the resolved authorization
snapshot, and starts the same scheduler shown in the Agents panel.

Manage these two layers in Settings:

- **Quick Actions** controls the label, enabled state, selected-text trigger,
  and bound Workflow. **Open Workflow** jumps directly to that graph.
- **Workflows** opens the standalone Workflow Studio. A template stores its
  goal, shared context, approval policy, Subagent tasks, dependencies,
  capabilities, Specialist/executor/model choices, budgets, and output
  schemas. Tasks without dependencies may run in parallel; dependencies define
  the serial stages.

The Studio uses the same task contract and proposal type as runtime delegation.
The right-panel Agents view is intentionally an activity surface rather than a
second workflow editor.
**Literature evidence review**, **Roundtable**, and **Develop computational
method** are read-only built-in Workflows in its library. The Roundtable generator is also available for
custom variants: it expands two or three participants plus a chair into
ordinary parallel opening, cross-review, and synthesis tasks. The generated
nodes remain editable and can be saved as a reusable Workflow, then bound to
one or more Quick Actions.

The Subagent graph is the primary composition surface. It assigns every task
to a topological stage: nodes in the same column are independent and may run
in parallel, while arrows move left to right through dependent stages. Select
a node to edit its complete task contract in the inspector. Drag from an
output handle to a downstream node (or click the handle then click the target)
to create a dependency; use **Add after selected** to insert the next stage
directly after the current node, or **Add parallel node** (toolbar or
double-click empty canvas) for same-stage peers. Click an arrow to select it,
then click again or press Delete to remove the dependency; incoming dependency
chips in the inspector still work too. Nodes can also be added or deleted
directly on the canvas. Every graph interaction updates the ordinary
`depends_on` proposal fields immediately, and cycle-producing connections are
rejected before save.

Workflow Studio replaces the normal Settings chrome with a dedicated
full-window editor. Its stable layout keeps the template library on the left,
save and lifecycle controls in the top bar, the DAG canvas in the center, and
the selected-node inspector on the right. Drag the divider between the canvas
and inspector to allocate space to either side. Workflow-level fields live in
a collapsible configuration strip so they do not permanently reduce the
canvas. The node inspector keeps selected Skills visible as removable chips
and searches the effective Skill catalog on demand instead of rendering the
whole catalog. The canvas includes zoom controls, a reset-to-100% action, a
dotted orientation grid, and a minimap; large graphs remain scrollable without
shrinking the inspector.

The composer `/` picker searches both enabled Skills and Workflow templates.
Selecting a Workflow adds a typed Workflow chip instead of copying prose into
the message. On send, Wisp resolves the template by stable ID, injects its
exact DAG contract, and enables delegation for that native conversation so
the main Agent can execute the graph through `delegate_tasks`. Skill and
Workflow chips can be combined in the same turn; transcript cards retain both
selections after reload. Removed or unavailable templates fail closed instead
of silently falling back to an invented plan.

The first built-in action is **Research literature**. Select text in a
conversation or file preview, then choose the action from the floating
selection toolbar or the right-click menu. Wisp creates a dedicated
conversation and runs three temporary Agents:

1. `supporting_evidence` searches for publications supporting the passage.
2. `challenging_evidence` independently searches for contradictions, boundary
   conditions, failed replications, and methodological critiques.
3. `synthesize` waits for both searches, deduplicates papers, and produces a
   balanced evidence review with caveats and gaps.

The first two nodes run in parallel; the third is their serial dependency.
Every node has a structured output contract. The action requires an enabled
literature Skill or connector and grants only the `literature_search`
capability to the search nodes. The explicit click authorizes this compiled,
read-only graph, and its completion automatically resumes the dedicated
conversation so the final synthesis appears without another message.

Quick Action records are persisted separately from templates. Their label,
enabled state, and menu ordering are user-controlled; the built-in action's
context, template binding, and permissions stay pinned to compiled values.
Built-in Workflow templates are read-only; saving an edited built-in creates a
custom copy.

Selected text and its source are appended to the template's shared context as
explicitly untrusted content. A custom Workflow does not inherit the built-in
approval exception: `review_all` creates a draft in its dedicated
conversation, while `auto_safe` starts only when the resolved graph does not
require confirmation. Write, execute, network, external-service, isolation,
and elevated-budget requests therefore continue through the normal review
boundary.

## Inline temporary Agents

1. Open the composer Agent menu and enable **Delegation** for the current
   conversation. New conversations start with delegation off.
2. Ask the main Agent for an outcome that materially benefits from independent
   or parallel work. The main Agent decides whether delegation is useful and,
   when it is, calls `delegate_tasks` itself.
3. The call describes an overall goal, bounded shared context, and up to eight
   tasks. Each task has its own instruction, dependency IDs, capability IDs,
   optional Specialist, optional JSON output schema, optional isolation
   request, and optional per-task token/tool/cost budget.
4. Wisp resolves every capability through host policy into an exact model,
   executor, tool set, project scope, workspace policy, budget, and timeout.
   The model cannot grant raw tools or permissions to a child.
5. Safe read-only tasks run immediately. A batch that can write, execute code,
   use an external service, request isolation, or exceed normal budgets uses
   the existing approval prompt. Rejecting it starts no child and returns the
   feedback to the main Agent so it can revise the batch.
6. Independent tasks run concurrently up to the batch limit. A dependent task
   starts only after its direct dependencies succeed and receives their
   structured results. An unrelated branch continues after another branch
   fails; only descendants of the failed branch are blocked.
7. Ordered, compact results return as tool output. The main Agent must combine
   them into its final response rather than sending the user elsewhere. If a
   result was truncated, `get_delegated_result` reads that task's full persisted
   result for the same conversation.

Failed and cancelled workflows are resumable. **Retry** keeps the persisted
workflow and every successful task result, reruns only failed/cancelled tasks
and descendants that were blocked, then supplies the retained dependency
results to those descendants. A failed task exposes its current token limit in
the activity card; changing **Retry max tokens** before retrying revises only
that task's authorized budget on the same workflow. The new value is still
checked against capability and host ceilings (64k tokens per task); a rejected
budget names the triggered limit, the requested value, and the ceiling.

Omitting `specialist_id` creates a generic temporary Agent. Selecting a
Specialist reuses its persona, model preference, skills, and connector
restrictions as an immutable snapshot for that run. A Specialist is therefore
an optional preset, not a required fixed team slot. The parent Agent sees only
the currently available Specialist IDs, names, and descriptions; private
instructions are copied into the selected child snapshot, not exposed in the
`delegate_tasks` description. The child prompt is composed from the bounded
worker contract, Specialist identity/instructions, task context and dependency
inputs, then the result contract.

A valid Specialist model preference is used when the task resolves to Native.
An empty or deleted model binding falls back through the normal active-model
selection and the resolved model is persisted. ACP profiles remain executor
choices rather than Specialist model bindings. The built-in Reviewer follows
the same optional selection rule and is never appended to a dynamic plan
automatically.

## Roundtable template

Workflow Studio can generate a structured Roundtable without introducing a
second workflow or chat protocol. Expand **Roundtable template**, choose two or
three discussion seats, and assign each seat an optional Specialist plus a
Native or ACP executor. A Native seat may also select a Wisp model; an ACP
seat's model and reasoning settings remain owned by that ACP Agent profile.
Configure the chair separately, then apply the template.

The generated proposal uses the ordinary dynamic workflow contract:

1. Every seat produces an independent opening position. These tasks have no
   dependencies and may run in parallel.
2. Every seat then reviews all opening positions, records agreements and
   conflicts, and revises its recommendation.
3. The chair receives all second-round reviews and synthesizes the shared
   conclusions, unresolved disagreements, evidence gaps, risks, and next steps.

The same Specialist, executor, and model assignment is copied into both rounds
for each seat. Enter the overall goal before applying the template; Wisp embeds
that goal into every generated task so detached children receive the actual
discussion topic. Applying preserves the goal, shared context, and approval
policy, and replaces only the task cards. Reapply after changing the goal.
After generation, every task remains editable, including its capabilities,
dependencies, budgets, and output schema.

This is a bounded DAG, not a live multi-model group chat. Temporary children do
not share hidden transcripts or freely message peers; dependency results are
their explicit coordination channel. The normal resolver, approval screen,
executor availability checks, persistence, cancellation, and audit records
still apply.

## Background completion

The composer Agent menu has a per-conversation **Completion** setting. Inline
is the default and preserves the same-turn behavior above. Background returns
a workflow handle as soon as the approved batch is scheduled, allowing the
parent turn and the rest of the app to continue. The main Agent must not poll
that handle.

Workflows started outside a parent model turn, such as Quick Actions, use the
durable background delivery path. The conversation's auto-resume setting still
decides whether their parent is automatically synthesized.

Each background execution reserves a persisted generation before any child
starts. When the workflow reaches succeeded, failed, or cancelled, Wisp stores
one compact result for that generation. Under the same conversation lock used
by normal turns, it then atomically appends one internal result message and
marks the generation delivered. A busy parent finishes its current or already
queued user turn first; this prevents background delivery from racing the
turn's incremental message sequence. Retrying a failed or cancelled workflow
creates a new generation, so the retry can deliver once without redelivering
the earlier result.

When **Background** is selected, enable **Auto-resume parent** to let an idle
parent Agent synthesize newly delivered results without another user message;
the option is hidden for inline completion, where it does not apply. Several
completions that become ready together may be combined into one synthesis
turn, but each generation's resume claim is made only once. If the app stops
after claiming that turn, the
claim is recorded as interrupted instead of being silently replayed on restart.
Without auto-resume, the completion card remains in the owning conversation
and enters the Native parent's context on its next turn. ACP parents receive
the same result as internal context on their next prompt because their own
transcript is maintained by the external Agent.

On startup, queued/running child attempts become explicit failed attempts, and
a background generation reserved before its first child started becomes an
explicit failed workflow. Terminal generations that were persisted just before
a crash are reconstructed from their immutable plan and attempts, then
delivered normally. The compact conversation message may later be removed by
ordinary transcript retention; full task responses and lookup records remain
in workflow attempts.

## Host-managed Run activities

A Workflow may contain a bounded host-managed `run_activity` node in addition
to ordinary Agent nodes. The first supported activity is `method_search`.
Workflow remains an immutable DAG: candidate iteration happens inside one
persisted Run and never expands the approved graph.

Run activities have their own exact authority snapshot: activity version,
ExecutionContext revision, one direct dependency and structured output
pointer, candidate/time/evaluator/cost limits, provider/model profile IDs, and
integrity hash. Specialist, Agent executor, Skill, isolation, model override,
and Agent token/tool budgets are rejected on these nodes. A method-search node
may consume only the required
`method_search_spec_artifact_version_id` field declared by its direct
dependency's output schema.

After its attempt starts, Wisp atomically creates and links the Run and moves
the attempt to `waiting_run`. That state is unfinished for DAG dependencies but
does not occupy one of the root Agent concurrency slots. Descendants remain
blocked until the linked Run succeeds. Run failure, timeout, loss, or
cancellation maps to one terminal attempt exactly once; retry creates a new
attempt and a new Run.

For `method_search`, creation stops at a second, exact review boundary. The
linked Run is `Draft`, the provider is not called, and no candidate evaluator
runs. The Run detail shows the immutable spec and audit ArtifactVersion IDs,
target symbol, evaluator, baseline/noise summary, reachability result,
guardrails, protected inputs, ExecutionContext, and budgets. **Start search**
revalidates those frozen inputs plus the approved model profile and only then
moves the Run to `Submitted`. This review is distinct from approving the
Workflow plan: plan approval authorizes preparation; Run start authorizes
search against the exact contract that preparation produced.

The method-search Run surface reports bounded progress and candidate lineage,
and provides pause, resume, and cancel controls. A pause takes effect at a
durable candidate boundary. Resume revalidates the exact contract and context;
it never silently substitutes current files. Selected code remains an
ArtifactVersion and is not applied to the project checkout.

Startup recovery keeps `waiting_run` only when its dedicated Run link still
exists in the same project. A missing or mismatched link fails explicitly.
Project import always fails an imported waiting activity instead of guessing
that work on another device can resume; the Run link and prior evidence remain
available for inspection. Cancelling the root requests cancellation of both
active Agent attempts and linked Runs. Graceful desktop shutdown converts a
submitted/running local method search to `Paused` after its last durable
checkpoint and terminates the bounded evaluator with the application. Startup
also converts an interrupted search to `Paused` with a recovery note. Both
paths require an explicit resume.

Method-search v0 is deliberately local and serial: Python only, one declared
function/class target, one local ExecutionContext, 1–50 candidates, immutable
project-local inputs, and a bounded evaluator. It does not provide GPU/remote
scheduling, multi-target repository edits, automatic data downloads, or
automatic application of the selected method.

## Native, ACP, and code execution

Native execution runs the ordinary Wisp Agent loop in a separate child
conversation with only the resolved tools. It supports project reading,
project writing, and bounded Run Manager execution without starting an ACP
client. This is the default eligible executor and is enough for a code task.

Scientific resources are resolved for the owning project and conversation at
draft time, then checked again before execution. Wisp considers the project's
enabled Skills, enabled bundled/custom MCP connections, selected
ExecutionContexts, configured Python/R interpreters, runtime workers, and
vision-capable models. A disabled or missing resource is omitted from both the
editor and `delegate_tasks` schema instead of being advertised optimistically.
Changing this resource set invalidates an already approved authorization
snapshot, so the task must be reviewed against the new authority.

The initial resource mapping is deliberately capability-shaped:

- `literature_search` grants only enabled literature Skills and literature
  connectors.
- `external_research` grants only enabled non-literature MCP connections.
- `visualization` grants configured Python/R tools and figure-oriented Skills.
- `code_run` grants `run_in_context`, `get_run`, and `cancel_run`. A generic
  temporary code task does not inherit every project Skill; a selected
  Specialist may reuse its configured non-literature Skill set.
- `image_inspection` grants local image reading only when the selected Native
  model supports vision.

For every task, its capability grant and its immutable Specialist whitelist
must both allow a Skill or connector. `None` on a selected Specialist keeps
the existing “inherit project settings” behavior; an explicit list narrows it.
The resulting exact resource IDs are installed directly in a Native child or
encoded as private allowlist tokens for that ACP child's filtered Wisp MCP
bridge. They are not inferred from an ACP vendor, command name, or Agent label.

ACP profiles remain available to workflows that explicitly resolve to an ACP
executor. Every configured profile whose command is currently available is
listed separately, and the selected profile ID—not its command, label, model,
or task name—controls routing. A profile may use Codex or another compatible
Agent, but the task is still defined by capabilities and contracts, not by an
ACP or Codex template. Automatic selection continues to prefer Native whenever
Native satisfies the task; choosing ACP is an explicit, approval-visible
override.

Delegated ACP sessions start with no Wisp MCP bridge. Wisp adds only bridge
tools implied by the resolved task permission set; for example, `code_run` can
receive the project-scoped execution-context and Run Manager tools while a
reasoning or file-read task receives no bridge. ACP permission requests are
matched against the same resolved tools, write flag, and project path ceiling,
independent of the ACP vendor. Unknown command, process, MCP, and network
requests are rejected.

Long-lived code is submitted as a persisted Run rather than by increasing the
delegated shell timeout. The child receives the conversation's selected remote
contexts plus the always-available local context, and can query or cancel the
Run by ID. Direct `shell` is never registered for a delegated Native child;
ACP receives the same Run control plane through the filtered bridge.

When a child links a project-local output in its structured summary or
evidence, Wisp snapshots the file as a content-addressed Artifact and returns
its durable ID with the task result. Structured DataAsset and Paper references
remain JSON references in the persisted response and parent delivery; large
or binary payloads are not copied into the conversation. A configured custom
MCP connection is treated as available from its saved configuration, but a
connection failure at execution is still reported by the child because Wisp
does not perform network health checks while drafting.

The same inline delegation surface is exposed through the Wisp MCP bridge as
`wisp_delegate_tasks` and `wisp_get_delegated_result` when the owning
conversation opted in. Because that bridge is non-interactive, a batch that
requires approval is denied instead of silently escalating.

## Bounded nested delegation

Nesting is opt-in per task through the `delegation` capability. A task that was
not resolved with that capability never receives `delegate_tasks`, even if its
prompt asks for it or a stored/raw tool name is forged. An authorized Native
or ACP child receives the same dynamic task protocol with authority narrowed
to its own capability, model, executor, permission, context, budget, and
timeout snapshot.

The default root limit remains one Agent level. Selecting `delegation` raises
that workflow to the hard maximum depth of two: a root child may create one
temporary child batch, and a depth-two child cannot delegate again. Root-wide
limits cover at most eight total tasks, two concurrent active children, and
the aggregate token, tool-call, cost, and wall-clock budgets. Registration and
attempt start reserve these limits atomically before a backend child or ACP
process is created. While an authorized parent waits synchronously for its
children, it yields its concurrency slot and reacquires it before continuing.

Nested task display IDs are namespaced under the parent, such as
`analysis/check_data`, while database workflow, step, and attempt IDs remain
stable. Root cancellation, deadline expiry, and budget exhaustion propagate to
every descendant. Completed nested batches are stored as structured results on
the direct parent response and are included again in the compact root result,
so synthesis does not depend on parsing a child transcript. Lineage and result
lookup survive application restart. Peer-to-peer sibling messaging is not part
of this model; dependencies, persisted artifacts, and parent result rollup are
the coordination paths.

## Persistence and safety

- Wisp persists the resolved v2 plan before execution. Stored steps contain the
  immutable Specialist, requested model/executor preferences, capability
  revisions, resolved permissions/model/executor, contracts, budgets, and
  policy integrity hash used for revalidation. ACP tasks do not store a
  decorative Native model that the ACP process would ignore.
- Background executions persist a generation and completion intent before
  launch. Result insertion, conversation delivery, auto-resume claim, and
  resume outcome are separate durable states; application restart never
  guesses that an unknown external process is still running.
- Before approval, a v2 draft exposes both its editable proposal and the
  resolved authority that will actually run. Each edit checks the draft's
  version, reruns dependency and policy resolution, and replaces the plan
  atomically. Approval makes the snapshot immutable; run and retry reuse that
  exact snapshot instead of asking a planner to recreate it.
- Read-only tasks may share the project workspace. Writable or executable
  tasks without isolation use one mutation lane and cannot edit the same
  checkout concurrently. When Git is installed and the project checkout is
  clean, a task may instead use a unique temporary Git worktree and run in
  parallel with other isolated writers. The approval card shows that Wisp will
  conflict-check and then cherry-pick the task's temporary commit.
- Native and ACP children both receive the isolated project root. Wisp captures
  a changed-file manifest and binary-capable patch, serializes merge decisions,
  and removes the temporary worktree and branch on success, failure, or
  cancellation. A failed child is never merged. A rejected/conflicting merge
  leaves the main checkout unchanged and stores the patch as an Artifact.
  Child-declared artifacts are copied to durable app storage before worktree
  cleanup, so their paths do not expire with the temporary directory. If a
  declared local artifact cannot be retained, the task fails and its project
  patch is not merged.
- Native Python/R kernels use a request-scoped runtime namespace instead of a
  project-wide kernel while isolated. Worktree finalization waits for Runs
  owned by that child; a failed/cancelled child cancels them. Timeout/drop
  cleanup cancels remaining child Runs and stops its runtime before removing
  the worktree.
- Non-Git and dirty project checkouts do not advertise isolation. Ordinary
  writable tasks still work there through the serialized mutation lane; an
  explicit isolation request fails closed instead of silently weakening its
  workspace guarantee. Initial isolation intentionally does not copy ignored
  files or add overlayfs/APFS/ZFS/ProjFS backends.
- Children receive only their instruction, bounded shared context, applicable
  project instructions, explicit inputs, and direct dependency results. They
  do not receive the full parent transcript.
- Dynamic tasks bind Skill guidance with explicit `skill_ids`, independently
  from capability permissions. Resolution snapshots each effective Skill's
  scope, path, declared version, package origin, and SHA-256. Native and ACP
  children receive only those rendered instructions; a disabled, shadowed, or
  changed Skill fails closed and requires the draft to be regenerated.
- Delegated Agents receive `delegate_tasks` only from an approved `delegation`
  capability and only while root-wide depth, task, concurrency, token, tool,
  cost, cancellation, and time checks still have capacity.
- Output contracts are checked before results reach the parent. Attempts,
  structured results, artifacts, evidence, usage, child conversation IDs, and
  backend session IDs remain auditable in SQLite. Secrets stay in the existing
  credential stores.

## Skill Portfolio Planner

Workflow Studio can generate a draft from the current effective Skill Catalog. The planner
normalizes the research request, applies deterministic lexical and `wisp` metadata scoring, and
selects a compact, standard, or deep complementary portfolio. Its preflight budgets each node for
the rendered Skill instruction plus the request, a fixed sub-Agent loop allowance (system prompt,
tool round trips, and search results), and the node output allowance; it reserves synthesis tokens
before selecting child nodes and defers lower-ranked optional candidates when the remaining child
budget is insufficient.

The confirmation card shows exact Skill sources and reasons, selected and deferred nodes, node and
total budgets, the synthesis reserve, the runtime maximum of two parallel Agents, and estimated DAG
batches. Applying the card opens the generated nodes in Workflow Studio for editing. Network,
execution, write, external-service, multi-Skill, deferred, standard, and deep plans require review;
only a compact, low-cost, read-only plan can remain auto-safe.

The built-in **Data-driven research design** Workflow is the first validation template. It keeps
the general planner domain-neutral while giving the final synthesis a strict eight-part schema:
data observations and robustness; literature consensus, conflicts, and gaps; hypotheses and
alternatives; deductive predictions; discriminating experiments plus rescue/falsification;
failure-driven iteration; translation, feasibility, and risk; and a source-marked evidence–claim
matrix with priorities. Its data and literature nodes run independently before synthesis and each
binds only its declared Skill.

The main Agent can inspect any configured template with the read-only `explain_workflow` tool.
Questions such as “What is Data-driven research design?” return the saved goal, task graph,
dependencies, capabilities, Skill bindings, and output sections. Inspection never starts the
Workflow; execution still requires a separate `delegate_tasks` call or an explicit UI action.

The main Agent can also turn an installed Skill into a reusable template with the
`create_workflow` tool. It reads the named Skill, derives capabilities from the Skill's declared
side effects, and registers a single-task Workflow that binds the Skill — the same binding the
delegation runtime expands into full Skill guidance at run time. Optional `params` overrides
(`goal`, `context`, `instruction`, `capabilities`, `approval_policy`, `output_schema`) expose the
Skill as a parameterized Workflow input. The generated template behaves like any user-authored
Workflow: `explain_workflow` shows it, Settings → Workflows edits it, and `delegate_tasks` runs
it. Names must be unique across built-in and user templates.

- Turning Delegation off prevents the main conversation and its MCP bridge from
  listing or invoking delegation tools. It does not erase workflow history or
  implicitly cancel a workflow that is already running.

## Dynamic Agents panel

The right-panel Agents view shows workflow activity owned by the active
conversation. Switching conversations switches the panel context; work in
other conversations keeps running in the background. Workflow definitions are
created and edited only in **Settings → Workflows**; the panel links directly
to that standalone Studio and does not embed a second editor.
Nested workflows appear indented beneath the root workflow with their depth
and namespaced task IDs. They are execution records rather than independent
drafts, so lifecycle controls remain on the root only.
Each dynamic task shows dependencies, requested capabilities, optional
Specialist, resolved model and executor, workspace/tool authority, approval
reasons, status, duration, usage, summary, and whether a full result is
available. **Inspect result** opens a readable view of the persisted summary,
deliverables, changes, evidence, checks, risks, and custom output fields. Child
conversation IDs and other execution-envelope details stay internal; the panel
does not offer a separate child-conversation takeover flow.

Workflow Studio creates arbitrary task graphs instead of assembling a fixed
team. Add bounded tasks, connect them with dependencies, and choose
capabilities, Specialist persona, model, eligible executor, isolation,
budgets, and output schemas there. Turning Delegation off disables approvals,
runs, and retries while leaving supported dynamic history and cancellation
available in the activity panel.

Only schema-version-2 dynamic plans are part of the product surface. Earlier
fixed-plan records are not migrated or deleted, but the Agents panel does not
list them and workflow actions reject them before approval, retry, or execution.

## Manual smoke check

Enable Delegation and ask the main Agent to compare two project files using two
independent temporary Agents. Confirm in the Agents panel that the two root
tasks overlap, their dependent synthesis task waits, and the final chat
response contains one synthesized comparison. Switch **Completion** to
**Background**, repeat the request, and verify the initial tool result is a
running handle followed later by exactly one completion card in the same
conversation. Enable **Auto-resume parent** and verify an idle conversation
adds one synthesized assistant update; start another parent turn and verify a
completion waits behind it. Then create an equivalent graph in
**Settings → Workflows**, attach it from the composer, and confirm the Agents
panel shows the run without showing workflow editing fields. Repeat with a
write capability: Wisp should show the exact resolved authority and start zero
children if approval is denied.
Open a completed task result and confirm its sections and rendered Markdown are
readable without raw JSON or a child-conversation action. Press Escape
immediately after opening it and confirm only the result dialog closes.
Finally, add the **Nested delegation** capability to one root task and let it
create two independent leaf tasks. Confirm that both leaves appear under the
same root card at depth 2, their IDs are prefixed by the parent task, their
structured results appear in the root result, and cancelling the root marks
the parent and both leaves for cancellation.
For the isolation path, start from a clean Git project and create two independent
write tasks with **Use an isolated workspace** enabled. Confirm that approval
shows **Conflict-check, then cherry-pick**, both children overlap, both changes
land as separate commits, and no `wisp-agent/*` worktree branch remains. Then
make both tasks edit the same line and confirm one merge is rejected, the main
file keeps the accepted change, and the rejected patch is available as an
Artifact. Cancel another isolated writer and confirm its partial patch is
preserved without modifying the main checkout.
