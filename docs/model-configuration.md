# Model configuration

In General settings, **Suggest follow-up questions** is enabled by default.
After a completed reply, Wisp uses that conversation's current model to offer
three optional next questions. The suggestion panel can be hidden per reply,
or the setting can be turned off to skip the extra model call entirely.

wisp-science calls remote LLM APIs through model profiles. Desktop users
configure these in **Settings -> Models**. Each row is a model profile with its
own display name, provider, API URL, model ID, advanced options, and API key.
For recognized model families the form auto-fills **Max output tokens** and
**Context window** to the vendor's documented ceilings, and saving a max-output
value above the documented ceiling is rejected with an inline error instead of
failing mid-turn with a provider 400.

The composer model picker binds the selected HTTP model to the current
conversation. Switching one populated conversation asks for confirmation and
does not change any other conversation. Empty conversations switch immediately
without a warning. The active profile in Settings remains the default for new
conversations.

Model profiles describe model access and capabilities for the **built-in Wisp
agent**. External coding agents (Codex / Claude via ACP) are configured under
**Settings → Models → ACP Agents** — see [ACP Agents](acp-agents.md). Do not put
an ACP launch command in an HTTP model profile.

For image workflows, mark an API profile as **Supports image input** and
optionally **Use for image analysis**. Image attachments are sent directly to a
visual input model. When the input model is non-visual, Wisp first calls the
assigned vision model and passes its text observations to the input model.
`view_image` and image reads use the assigned vision model in the same way.
Raster image input supports PNG, JPEG, GIF, and WebP. Files up to 5 MiB are
sent unchanged. For larger files, Wisp pauses before the model request and asks
whether to create a temporary JPEG input copy with a longest edge of 2048
pixels. The project file is never modified, and the confirmation warns that
fine details may be lost. Source images above 50 MiB remain rejected.

When switching a populated conversation to a non-visual model, the confirmation
explains that previously sent images will be omitted from future requests to
that model. This substitution happens only while preparing the API request; it
does not delete or rewrite the saved conversation. A new image attached after
the switch is analyzed through the assigned vision model. Without an assigned
vision model, Wisp rejects that new image before starting the main model turn.

Image generation is a separate model role. Create an OpenAI profile with model
ID `gpt-image-2`, then enable **Use for image generation**. The built-in
**Scientific Illustrator** calls OpenAI's Image API and saves a PNG under
`figures/` when that role is assigned and PNG or image-model generation is
requested. An explicit SVG/vector/editable request always uses the specialist's
direct-SVG path, even when `gpt-image-2` is configured: it writes SVG, renders
that exact SVG to a PNG preview, inspects the preview, and iterates on the SVG.
An explicit PNG request requires the configured image-generation model; it is
not silently replaced with SVG. The configured generation tool is also
available in ordinary built-in-agent
conversations, so a direct request for the Scientific Illustrator or
`gpt-image-2` can generate the image without preselecting the specialist. While
the request runs, the conversation shows an image placeholder and replaces it
with the generated PNG. When the user does not specify a format, the specialist
uses the assigned image-generation profile to create PNG if present. Otherwise
it uses the same SVG -> PNG preview -> SVG correction workflow and delivers SVG
under `figures/`. Image-only profiles do not appear in chat, Reviewer,
specialist, delegation, or side-chat model pickers.

An image-generation assignment does not also provide image analysis.
`gpt-image-2` may consume an input image for editing, but its Image API returns
generated pixels rather than the textual observations required by `view_image`
and a non-visual chat model. Configure a chat/Responses profile with **Supports
image input** and **Use for image analysis** for that role; it may use the same
provider credentials, but it remains a separate API capability.

The **Validate** action checks `gpt-image-2` access through OpenAI's model
metadata endpoint. If a compatible gateway does not implement the single-model
route and returns `404` or `405`, Wisp checks its model-list endpoint instead.
It does not send the image-only model to Responses/Chat Completions and does not
generate a billable validation image.

## API providers

| Provider | Use when | Required fields |
| --- | --- | --- |
| OpenAI-compatible | DeepSeek, GLM, local gateways, or any `/chat/completions` compatible endpoint | API URL, Model ID, API key |
| OpenAI (Responses API) | OpenAI reasoning/tool-call models through `/v1/responses` | API URL, Model ID, API key |
| Anthropic | Claude API through `/v1/messages` | API URL, Model ID, API key |

Enter the provider's API base URL. Do not append `/v1`, `/chat/completions`,
`/responses`, or `/v1/messages`; Wisp adds the matching request path for the
selected provider. For OpenAI-compatible services, Wisp tries both
`/chat/completions` and `/v1/chat/completions` when the base URL has no explicit
version or endpoint path. It only falls back when the first route is missing or
returns an obvious non-API response, so authentication and rate-limit failures
are not duplicated.

OpenAI-compatible reasoning streams are normalized into one reasoning channel.
Empty `content` placeholders sent alongside Alibaba/DashScope
`reasoning_content` chunks are ignored, so a continuous thought process remains
one disclosure in the conversation.

If a provider ends a turn after returning only reasoning tokens—without visible
text or a tool call—Wisp reports a resumable error instead of showing the turn
as silently processed. Completed tool results remain in the conversation; use
**Resume** to request the missing final reply without replaying those tools. If
this repeats in a long conversation, send `/compact` before resuming to fold old
turns while preserving an archive of the full history.

**Settings → General → Automatically compact long conversations** is enabled by
default. Following mangopi-cli's model-boundary approach, Wisp checks the
estimated context before every native-agent model call, including later calls
after large tool results and ephemeral host/reviewer injections. At 80% it
archives the complete pre-compact history and targets 60%, leaving headroom so
the next ordinary result does not immediately trigger another rewrite. Older
tool output, reasoning, and images are safely pruned first without shortening
user messages or visible assistant answers; oversized recent tool payloads
become bounded excerpts that point to the archive. If semantic turns must be
removed, Wisp summarizes a sanitized projection of the original history before
deleting them, then retains one incrementally updated summary checkpoint plus
at most two recent turns in an 8K-token tail. Raw images and large tool results
are not replayed to the summary model. The internal summary instruction is
never added to the conversation, and a failed compaction rolls back the rewrite
and stops before Wisp can send the known-oversized main request. Tool
results are also capped to a 16 KiB head/tail excerpt when they enter model
context (the full result is still shown in the tool event), preventing one
read, grep, browser, or MCP response from consuming the whole window. Each
automatic or manual rewrite leaves a persistent **Context automatically
compacted** / **Context compacted** flag in the conversation with the before
and after request-token estimates. Turning the setting off keeps the warning,
manual `/compact`, and overflow recovery dialog available. ACP agents are not
modified because their remote transcripts are owned by the ACP process.

After a native-agent reply, the composer footer shows the estimated percentage
of the active model's context window. The limit tracks the model the session
is currently bound to: switching models or editing a profile's context window
re-bases the gauge immediately, without waiting for the next reply. Open it
for a detail card aligned to the
composer width that splits the same calibrated request estimate into system
prompt, built-in tool definitions, rules, selected Skills, MCP and other
dynamic tools, subagent definitions, and conversation content. These buckets
are mutually exclusive and sum to the value used by automatic compaction.
Select any bucket except Conversation to inspect the exact prompt/rule text or
the tool, Skill, MCP, and subagent definitions included in the latest native
request. Conversation remains a size-only category so the usage card does not
duplicate the chat transcript.
Older native usage rows that only stored a total attribute that window to
Conversation until the next reply refreshes the full breakdown. ACP sessions
expose only the total reported by the remote agent, so Wisp labels that value
as an agent-reported total instead of inventing a breakdown it cannot observe.

## Usage dashboard

**Settings → Usage** shows global input, output, reasoning, and cached-token
totals, a 53-week activity chart with **Daily**, **Weekly**, and **Cumulative**
views, an input-plus-output token share by model, and a ranked list of SKILL
(`use_skill`) and MCP (`mcp:*`) tool calls beneath the model chart. Usage is
grouped by project workspace. Open a workspace to inspect its sessions, which
are loaded 20 at a time with Previous/Next pagination; sub-agent rounds remain
folded into their root session.

New usage rounds persist the model and timestamp used for that request. Older
usage events did not contain those fields, so their dashboard model falls back
to the session's saved model binding and their activity date falls back to the
session's latest activity date.

When the provider explicitly rejects a built-in Wisp-agent request for
exceeding its context window, the conversation opens a recovery dialog instead
of leaving the raw error as a dead end. **Compact and continue** archives the
full history, folds older turns, and resumes after the retained tool results.
**Continue in a new conversation** starts a clean session and attaches a
bounded Reader summary of the old conversation as context. **Pause
conversation** preserves the error and completed work without making another
request. Pressing Escape immediately after the dialog opens is equivalent to
pausing; it closes only this recovery surface.

For OpenAI-compatible and Responses API profiles, Wisp sends its internal
`python` REPL tool as `wisp_python` and maps returned calls back to `python`.
This avoids the reserved `python` function-name collision on Codex models,
including when the request is translated by gateways such as CLIProxyAPI.

API keys are stored in the OS keyring. They are not stored in SQLite.

The desktop app stores model profile metadata in `.wisp/wisp.sqlite`. Existing single-model installs are migrated into a `default` model profile the first time settings are loaded.

## Headless CLI

The `wisp-science` headless CLI uses environment variables and supports API providers:

```powershell
$env:WISP_PROVIDER = "openai"           # openai, openai_responses, or anthropic
$env:WISP_API_URL  = "https://api.deepseek.com"
$env:WISP_MODEL    = "deepseek-v4-pro"
$env:WISP_API_KEY  = "<your provider key>"
cargo run -p wisp-cli
```
