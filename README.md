<div align="center">

<img src="docs/assets/logo.svg" alt="Wisp Science logo" width="128" />

# Wisp Science

**The open-source, local-first AI research workbench.**

<a href="https://github.com/xuzhougeng/wisp-science/releases"><img src="https://img.shields.io/github/v/release/xuzhougeng/superscience" alt="Release"></a>
<a href="https://github.com/xuzhougeng/wisp-science/releases"><img src="https://img.shields.io/github/downloads/xuzhougeng/superscience/total" alt="Downloads"></a>
<a href="https://doi.org/10.5281/zenodo.21193742"><img src="https://zenodo.org/badge/1285857639.svg" alt="DOI"></a>
<a href="https://github.com/xuzhougeng/wisp-science/blob/main/LICENSE"><img src="https://img.shields.io/github/license/xuzhougeng/superscience" alt="License"></a>
<a href="https://github.com/xuzhougeng/wisp-science/stargazers"><img src="https://img.shields.io/github/stars/xuzhougeng/superscience?style=social" alt="Stars"></a>
<br>
<a href="https://github.com/xuzhougeng/wisp-science/releases"><img src="https://img.shields.io/badge/Windows-supported-0078D4" alt="Windows supported"></a>
<a href="https://github.com/xuzhougeng/wisp-science/releases"><img src="https://img.shields.io/badge/macOS-supported-000000" alt="macOS supported"></a>
<a href="#build-from-source"><img src="https://img.shields.io/badge/Linux-source%20build-FCC624" alt="Linux source build"></a>

[English](README.md) · [简体中文](README_zh.md) · [Documentation](#documentation) · [Releases](https://github.com/xuzhougeng/wisp-science/releases)

<img src="docs/assets/app-home.png" alt="Wisp Science desktop app running a bundled RNA-seq analysis demo" width="100%" />

</div>

**Wisp Science** is a desktop AI research assistant and scientific computing
workbench. It connects to OpenAI-compatible and Anthropic models, runs
persistent Python and R environments on local, WSL, SSH, and GPU compute, loads
reusable Agent Skills (`SKILL.md`), and reaches ~80 bioinformatics and
computational biology databases through bundled Model Context Protocol (MCP)
servers — while your data, conversations, and credentials stay on your own
machines.

Built with Rust, Tauri v2, and Leptos, Wisp Science runs as a cross-platform
desktop app or a headless CLI.

> **Our manifesto:** Wisp Science is open source and borderless. We are building
> a scientific workbench that anyone, anywhere can use, study, improve, and
> share.

> **Status:** MVP vertical slice. The agent loop, streaming providers, tools,
> Python/R REPLs, SQLite store, MCP client, and Leptos UI all build and run.
> See [Roadmap](#roadmap-post-mvp) for what is deferred.

## What does WISP stand for?

**WISP = Workspace for Intelligent Scientific Practice**
（中文：面向智能科研实践的工作空间）

- **Workspace** — not a single analysis tool, but a complete research workspace.
- **Intelligent** — AI agents, models, and automation are built in.
- **Scientific** — explicitly built to serve scientific research.
- **Practice** — covers real research practice: literature search, analysis,
  computation, writing, and task management.

## Features

**An agent that does the work, not just chat**

- Streams OpenAI-compatible and Anthropic models, with per-provider model
  profiles and tiered routing from a single trait.
- Reads, writes, searches, and runs shell commands inside a project-rooted
  path sandbox, behind explicit approval gates; an opt-in per-conversation
  **Full Permission** mode auto-approves after a warning.
- Coordinates exact file-tool paths across parallel conversations. Shell,
  Python, and R calls remain concurrent because their file access and child
  process lifetimes cannot be inferred reliably from command text.
- Loads reusable Agent Skills (`SKILL.md`) with progressive disclosure — the
  catalog never floods the prompt.
- Drives external coding agents (Codex, Claude Code, …) over ACP v1, and spins
  up reviewable sub-agent teams with [Controlled Delegation](docs/agent-delegation.md).

**Real compute, from laptop to cluster**

- Persistent Python and R environments per project — variables survive across
  cells, conversations, and app restarts.
- Local, WSL, and SSH/GPU **execution contexts** with one-connection hardware
  and runtime probing; each context keeps its own interpreter paths.
- Structured **Runs** for long jobs: preflight checks, per-second heartbeats,
  and bounded log tails persisted with an environment snapshot.
- Secrets live in the OS keyring, never in SQLite. Free-form `ssh`/`scp` is
  replaced by registered, probed hosts; a failed connection opens a
  connectivity gate instead of silently retrying.

**Built for science**

- ~80 bioinformatics databases (PubMed, GEO, …) through bundled
  [MCP bio-tools servers](#bundled-bio-tools-mcp), discovered on demand via
  `search_mcp_tools` instead of bloating every request.
- Remote MCP services with OAuth (Notion and others), plus installable
  [feature plugins](docs/feature-plugins.md) that package Skills and MCP servers.
- Fully offline previews for Jupyter notebooks, PDF, DOCX/XLSX/PPTX, and
  images — including region cropping straight into the composer.
- A [Publication Workspace](docs/publication-evidence.md) that freezes
  manuscript revisions and exports verifiable, deterministic Evidence Capsules.

**A workbench that remembers**

- Conversations persist to SQLite; restart and the full history is back. One
  click **undoes** a turn's file edits with a preview of what will be restored.
- `@` attaches artifacts, files, execution contexts, and language runtimes;
  `#` reaches saved sessions through a cited, read-only **Reader** specialist;
  `/` applies a skill to the next turn.
- Ctrl+K / Ctrl+P palettes, side chat, conversation folders, a global library
  of cells and figures, and in-app update checks.
- [Encrypted manual sync](docs/project-sync.md) and one-click
  [project transfer](docs/project-transfer.md) keep machines in step — nothing
  ever syncs in the background.

## Get started

### Download

Grab the latest installer from
[GitHub Releases](https://github.com/xuzhougeng/wisp-science/releases):

| Platform | Package | Notes |
|----------|---------|-------|
| Windows  | MSI / NSIS | The installer is unsigned: choose **More info → Run anyway** on SmartScreen. If the window never appears after install, **Quit** from the tray icon and repair the [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/#download-section) (Evergreen Standalone Installer, run as administrator), then reopen Wisp Science. |
| macOS    | `.dmg` (Apple Silicon + Intel) | Unsigned: right-click → **Open** on first launch, or allow it in System Settings → Privacy & Security. |
| Linux    | — | [Build from source](#build-from-source). |

### Build from source

Prerequisites:

- **Rust** (stable, 1.88+) with `wasm32-unknown-unknown`:
  `rustup target add wasm32-unknown-unknown`
- **uv** (Python environment manager): <https://docs.astral.sh/uv/>
- **Trunk**: `cargo install --locked trunk` · **Tauri CLI v2**:
  `cargo install tauri-cli --version "^2"`
- Optional: **R** with the `jsonlite` package for the persistent `r` tool.
  Wisp locates `Rscript` via the interpreter configured in Settings, then
  PATH, then well-known install locations (for example
  `C:\Program Files\R\R-*\bin` on Windows or a conda base environment).
  Wisp never installs R packages automatically.
- Windows needs the **WebView2 Runtime** (present on most Windows 10/11
  systems; the installer acquires it when missing). macOS needs **Xcode
  Command Line Tools** (`xcode-select --install`) and uses the system WebKit.

```bash
cargo tauri dev      # hot-reload: Trunk serves the UI, Tauri opens the window
cargo tauri build    # installers under target/release/bundle (MSI/NSIS, .app/.dmg)
```

For a universal macOS binary (Apple Silicon + Intel):

```bash
rustup target add x86_64-apple-darwin
cargo tauri build --target universal-apple-darwin
```

### Headless CLI

```bash
export WISP_API_KEY=<your provider key>
export WISP_PROVIDER=openai            # openai (default) | openai_responses | anthropic
export WISP_MODEL=deepseek-v4-pro
cargo run -p superscience-cli                  # interactive agent in your terminal
```

Run a single prompt, or stream machine-readable events (one JSON object per
line) for scripting:

```bash
cargo run -p superscience-cli -- run "Summarize the files in this project"
cargo run -p superscience-cli -- run --output jsonl "Summarize the files in this project"
```

The CLI also ships a repeatable agent regression suite (six fixed file tasks,
JSON report, pass/fail plus latency/token deltas against a baseline):

```bash
cargo run -p superscience-cli -- eval --save baseline.json
cargo run -p superscience-cli -- eval --compare baseline.json --save current.json
```

### ACP agents (optional)

Wisp can launch any installed local agent that speaks ACP v1 over stdio —
separate from HTTP model profiles:

1. Install an adapter, e.g. `npm install -g @agentclientprotocol/codex-acp`.
2. **Settings → Models → ACP Agents** → set **Label**, **Command**, and
   **Arguments** → **Save Agent** → **Test Connection**.
3. Select the agent in the chat model picker and send a prompt.

Full setup, Claude example, and troubleshooting:
[docs/acp-agents.md](docs/acp-agents.md).

## Configuration

All optional; sensible defaults are bundled. Desktop stores API keys in the OS
keyring and model profiles in `.superscience/superscience.sqlite` (Settings → Models); see
[Model configuration](docs/model-configuration.md). Custom credentials map a
display name to an environment variable and are injected only into newly
launched local Python and bundled MCP processes — never copied to SSH/WSL
hosts.

**Settings → Storage** lists workspace paths per project. Select a project to
view that workspace's local footprint separately from shared app data.

For project-specific Agent instructions, Wisp reads `AGENTS.md` from the
project root when a new session starts. Instructions entered in **Project
Settings → Agent Context** are stored in `.superscience/SUPERSCIENCE.md` and applied after
`AGENTS.md`, so the explicit Wisp setting takes precedence when both exist.

| Variable             | Purpose                                                       |
|----------------------|---------------------------------------------------------------|
| `WISP_API_KEY`       | Provider API key (CLI). Desktop uses the keyring instead.     |
| `WISP_PROVIDER`      | CLI API provider: `openai` (default), `openai_responses`, or `anthropic` |
| `WISP_API_URL`       | API root; defaults to DeepSeek / OpenAI / Anthropic           |
| `WISP_MODEL`         | Model name                                                    |
| `WISP_MAX_CONTEXT`   | Context budget (default 1,000,000)                            |
| `WISP_MAX_ITER`      | Max agent iterations per turn (default 100; 0 = unlimited)    |
| `WISP_SKILLS_PATH`   | Extra `;`/`:`-separated SKILL.md catalog dirs                 |
| `WISP_KERNEL_WORKER` | Override path to `kernel_worker.py` (bundled by default)      |
| `WISP_MCP_COMMAND`   | Launch an arbitrary stdio MCP server (full command line)      |
| `WISP_MCP_PKG`       | Launch a bundled bio-tools server, e.g. `mcp_pubmed`          |

### Bundled bio-tools MCP

`WISP_MCP_PKG=mcp_pubmed` launches `mcp-servers/bio-tools/run_server.py
mcp_pubmed` inside the uv venv. Install the server's dependencies first:

```bash
uv pip install mcp requests
# plus any server-specific deps (httpx, xmltodict, etc.) the package imports
```

The agent discovers matching tools with `search_mcp_tools` and calls the
selected one through `use_mcp_tool`; the full server catalog is never copied
into every model request.

### Remote MCP (Notion example)

**Settings → Connections → Add connection → Remote URL**, enter
`https://mcp.notion.com/mcp`, set **Authentication** to **OAuth**, and **Test**
or **Save** — either opens Notion's authorization page in your browser. OAuth
tokens stay in the OS keyring; deleting the connection removes its credential.

## Bundled demos

`seed/` ships five pre-baked ESR1 / GSE153250 examples in research order: find
data → inspect sample format → RNA-seq upstream (siESR1 vs siNT counts) →
downstream DEG/ORA/GSEA → scientific hypothesis / research-project design. In
the desktop app, **Open demo** lists them and opens each as a read-only
transcript with full tool/run history — the fastest way to see what Wisp can
do without an API key.

## Documentation

| Topic | Guide |
|-------|-------|
| Model profiles & providers | [docs/model-configuration.md](docs/model-configuration.md) |
| External coding agents (ACP) | [docs/acp-agents.md](docs/acp-agents.md) |
| Multi-agent workflows | [docs/agent-delegation.md](docs/agent-delegation.md) |
| Skills & plugins | [docs/skills.md](docs/skills.md) · [docs/feature-plugins.md](docs/feature-plugins.md) |
| Terminals, remote files, transfers | [docs/terminal-sessions.md](docs/terminal-sessions.md) · [docs/remote-file-browser.md](docs/remote-file-browser.md) · [docs/server-transfers.md](docs/server-transfers.md) |
| Moving & syncing projects | [docs/project-transfer.md](docs/project-transfer.md) · [docs/project-sync.md](docs/project-sync.md) ([中文](docs/project-sync.zh-CN.md)) |
| Publication evidence capsules | [docs/publication-evidence.md](docs/publication-evidence.md) |
| Cross-project library | [docs/global-library.md](docs/global-library.md) |
| IM bots (Feishu / WeChat) | [docs/channels.md](docs/channels.md) |
| Real-browser automation | [docs/real-browser-automation.md](docs/real-browser-automation.md) |
| StickS3 device bridge & desktop pet | [docs/sticks3-device-bridge.md](docs/sticks3-device-bridge.md) · [docs/pet.md](docs/pet.md) |
| App updates | [docs/app-updates.md](docs/app-updates.md) |
| UI design principles | [docs/ui-design-principles.md](docs/ui-design-principles.md) |

## Development

### Repository layout

```
superscience/
├─ crates/
│  ├─ superscience-llm/     Provider trait + OpenAI-compatible + Anthropic + SSE + RoutedProvider
│  ├─ superscience-core/    ContextManager (3-tier compaction), SystemPrompt, agent_loop, memory
│  ├─ superscience-tools/   read/write/edit/search/grep/shell/attempt_completion + Windows safety
│  ├─ superscience-store/   sqlx SQLite (projects/frames/messages/artifacts/settings) + OS keyring
│  ├─ superscience-skills/  SKILL.md discovery + search_skills/use_skill progressive loading
│  ├─ superscience-runtime/ project-scoped Python/R runtime manager + REPL tools
│  ├─ superscience-mcp/     stdio JSON-RPC MCP client + McpTool adapter (bundled bio-tools)
│  ├─ superscience-acp/     ACP v1 stdio client for external coding agents
│  ├─ superscience-sync/    Encrypted snapshot protocol + self-hosted relay server
│  └─ superscience-cli/     `superscience` headless binary
├─ src-tauri/       Tauri v2 desktop shell (commands + agent event stream)
├─ ui/              Leptos CSR frontend (built by Trunk, loaded in WebView2)
├─ python/          kernel_worker.py + mock MCP server (uv-managed)
├─ r/               optional system-R kernel worker (requires jsonlite)
├─ skills/          Bundled SKILL.md catalog for reusable scientific workflows
├─ mcp-servers/     Bundled MCP servers (bio-tools: ~80 DB clients)
└─ seed/            Bundled demo session recordings (ESR1 / GSE153250 ×5)
```

### Testing

- **Rust unit tests** — `cargo test --workspace`
  (covers `superscience-store` SQLite round-trips, the seed demo loader, etc.).
- **MCP client smoke** — `cargo run -p superscience-mcp --example smoke` launches the
  bundled mock MCP server via `uv` and round-trips `tools/list` + `tools/call`.
- **UI E2E (Playwright + Tauri mock)** — `ui-tests/` runs the Leptos UI in a
  headless browser against `trunk serve`, with a mocked `window.__TAURI__` so
  no Rust backend or API key is needed:

  ```bash
  cd ui-tests
  npm install
  npx playwright install chromium   # one-time browser download
  npx playwright test               # serve UI + run the full mocked desktop flow suite
  ```

### Architecture

- **Agent loop** (`superscience-core::agent`): read → think → tool-call → verify,
  streaming tokens to an `Output` sink. Stops on `attempt_completion` or when
  the model returns no tool calls.
- **Context compaction** (`superscience-core::context`): an archive-first pipeline fires
  before each model call at 80% of the context budget — prune tool/media noise,
  then summarize sanitized history, keeping one incremental checkpoint plus an
  8K-token recent tail. Old turns are never silently dropped.
- **Providers** (`superscience-llm`): one trait, two wire formats (OpenAI
  `/chat/completions` and Anthropic `/v1/messages`), both with SSE streaming.
  `RoutedProvider` picks a low/medium/high tier per turn.
- **Tools** (`superscience-tools`): filesystem + shell tools with Windows-aware
  dangerous-command gating and a path sandbox rooted at the project directory.
- **Python/R REPLs** (`superscience-runtime`): one manager-owned process per
  project/context/language keeps its namespace across cells and conversations;
  local, WSL, and SSH contexts share one versioned protocol.
- **MCP** (`superscience-mcp`): a minimal newline-JSON-RPC client launches any stdio
  MCP server; remote schemas stay behind `search_mcp_tools` / `use_mcp_tool`
  until a task needs them.

## Roadmap (post-MVP)

- `FlashThinking` — phase-aware structured thinking-framework injection.
- `loop_engine` — deeper Implementer / Verifier / Updater workflows beyond the
  bounded automatic Reviewer pass shipped today.
- `RoutedProvider` LLM-score tier selection (keyword tier is already wired).

## Acknowledgements

Special thanks to these community members for their feedback, issue reports,
and pull requests (ordered by the number of issues reported):

<p>
  <a href="https://github.com/Yu-Qiao-sjtu"><img src="https://avatars.githubusercontent.com/u/88706761?v=4&amp;s=96" width="64" height="64" alt="@Yu-Qiao-sjtu" title="@Yu-Qiao-sjtu"></a>
  <a href="https://github.com/lfz0924"><img src="https://avatars.githubusercontent.com/u/82395287?v=4&amp;s=96" width="64" height="64" alt="@lfz0924" title="@lfz0924"></a>
  <a href="https://github.com/jarxunlai"><img src="https://avatars.githubusercontent.com/u/199478724?v=4&amp;s=96" width="64" height="64" alt="@jarxunlai" title="@jarxunlai"></a>
  <a href="https://github.com/OrigamiSheep"><img src="https://avatars.githubusercontent.com/u/48906039?v=4&amp;s=96" width="64" height="64" alt="@OrigamiSheep" title="@OrigamiSheep"></a>
  <a href="https://github.com/LeeJyee"><img src="https://avatars.githubusercontent.com/u/166231040?v=4&amp;s=96" width="64" height="64" alt="@LeeJyee" title="@LeeJyee"></a>
  <a href="https://github.com/stardustFFF"><img src="https://avatars.githubusercontent.com/u/306053694?v=4&amp;s=96" width="64" height="64" alt="@stardustFFF" title="@stardustFFF"></a>
  <a href="https://github.com/Doctorluka"><img src="https://avatars.githubusercontent.com/u/101385826?v=4&amp;s=96" width="64" height="64" alt="@Doctorluka" title="@Doctorluka"></a>
  <a href="https://github.com/Charlesyu153"><img src="https://avatars.githubusercontent.com/u/232734740?v=4&amp;s=96" width="64" height="64" alt="@Charlesyu153" title="@Charlesyu153"></a>
  <a href="https://github.com/xiaowen621"><img src="https://avatars.githubusercontent.com/u/241900839?v=4&amp;s=96" width="64" height="64" alt="@xiaowen621" title="@xiaowen621"></a>
  <a href="https://github.com/liaoyuan919"><img src="https://avatars.githubusercontent.com/u/240658511?v=4&amp;s=96" width="64" height="64" alt="@liaoyuan919" title="@liaoyuan919"></a>
  <a href="https://github.com/lhx-JIPS"><img src="https://avatars.githubusercontent.com/u/33241642?v=4&amp;s=96" width="64" height="64" alt="@lhx-JIPS" title="@lhx-JIPS"></a>
  <a href="https://github.com/chenzhiyu48"><img src="https://avatars.githubusercontent.com/u/65606400?v=4&amp;s=96" width="64" height="64" alt="@chenzhiyu48" title="@chenzhiyu48"></a>
  <a href="https://github.com/liuyc414"><img src="https://avatars.githubusercontent.com/u/190511200?v=4&amp;s=96" width="64" height="64" alt="@liuyc414" title="@liuyc414"></a>
  <a href="https://github.com/kevinzzzhang76-dot"><img src="https://avatars.githubusercontent.com/u/251931886?v=4&amp;s=96" width="64" height="64" alt="@kevinzzzhang76-dot" title="@kevinzzzhang76-dot"></a>
  <a href="https://github.com/Shawn-Gua"><img src="https://avatars.githubusercontent.com/u/110019576?v=4&amp;s=96" width="64" height="64" alt="@Shawn-Gua" title="@Shawn-Gua"></a>
  <a href="https://github.com/Hayesss"><img src="https://avatars.githubusercontent.com/u/66942436?v=4&amp;s=96" width="64" height="64" alt="@Hayesss" title="@Hayesss"></a>
  <a href="https://github.com/Az-Fan"><img src="https://avatars.githubusercontent.com/u/189823792?v=4&amp;s=96" width="64" height="64" alt="@Az-Fan" title="@Az-Fan"></a>
  <a href="https://github.com/19951219asd"><img src="https://avatars.githubusercontent.com/u/118892832?v=4&amp;s=96" width="64" height="64" alt="@19951219asd" title="@19951219asd"></a>
  <a href="https://github.com/yeshubiao2017-source"><img src="https://avatars.githubusercontent.com/u/233231577?v=4&amp;s=96" width="64" height="64" alt="@yeshubiao2017-source" title="@yeshubiao2017-source"></a>
  <a href="https://github.com/xuxh95"><img src="https://avatars.githubusercontent.com/u/299415390?v=4&amp;s=96" width="64" height="64" alt="@xuxh95" title="@xuxh95"></a>
  <a href="https://github.com/xiaoshen19930901"><img src="https://avatars.githubusercontent.com/u/24424905?v=4&amp;s=96" width="64" height="64" alt="@xiaoshen19930901" title="@xiaoshen19930901"></a>
  <a href="https://github.com/scsksprings"><img src="https://avatars.githubusercontent.com/u/60927616?v=4&amp;s=96" width="64" height="64" alt="@scsksprings" title="@scsksprings"></a>
  <a href="https://github.com/lpc520"><img src="https://avatars.githubusercontent.com/u/61644087?v=4&amp;s=96" width="64" height="64" alt="@lpc520" title="@lpc520"></a>
  <a href="https://github.com/lijianchunChina"><img src="https://avatars.githubusercontent.com/u/42370856?v=4&amp;s=96" width="64" height="64" alt="@lijianchunChina" title="@lijianchunChina"></a>
  <a href="https://github.com/kjiojio"><img src="https://avatars.githubusercontent.com/u/118580250?v=4&amp;s=96" width="64" height="64" alt="@kjiojio" title="@kjiojio"></a>
  <a href="https://github.com/dmh-git-cop"><img src="https://avatars.githubusercontent.com/u/270353192?v=4&amp;s=96" width="64" height="64" alt="@dmh-git-cop" title="@dmh-git-cop"></a>
  <a href="https://github.com/ZZRSCAR"><img src="https://avatars.githubusercontent.com/u/255126066?v=4&amp;s=96" width="64" height="64" alt="@ZZRSCAR" title="@ZZRSCAR"></a>
  <a href="https://github.com/Toomi0124"><img src="https://avatars.githubusercontent.com/u/300393761?v=4&amp;s=96" width="64" height="64" alt="@Toomi0124" title="@Toomi0124"></a>
  <a href="https://github.com/Lezhao0226"><img src="https://avatars.githubusercontent.com/u/72743280?v=4&amp;s=96" width="64" height="64" alt="@Lezhao0226" title="@Lezhao0226"></a>
  <a href="https://github.com/HSsnano"><img src="https://avatars.githubusercontent.com/u/87816341?v=4&amp;s=96" width="64" height="64" alt="@HSsnano" title="@HSsnano"></a>
  <a href="https://github.com/zwbao"><img src="https://avatars.githubusercontent.com/u/24564677?v=4&amp;s=96" width="64" height="64" alt="@zwbao" title="@zwbao"></a>
  <a href="https://github.com/yuzhenpeng"><img src="https://avatars.githubusercontent.com/u/31943277?v=4&amp;s=96" width="64" height="64" alt="@yuzhenpeng" title="@yuzhenpeng"></a>
  <a href="https://github.com/youxiudongdong-lang"><img src="https://avatars.githubusercontent.com/u/306058340?v=4&amp;s=96" width="64" height="64" alt="@youxiudongdong-lang" title="@youxiudongdong-lang"></a>
  <a href="https://github.com/ying-ge"><img src="https://avatars.githubusercontent.com/u/45988974?v=4&amp;s=96" width="64" height="64" alt="@ying-ge" title="@ying-ge"></a>
  <a href="https://github.com/yemiaoyong"><img src="https://avatars.githubusercontent.com/u/61010663?v=4&amp;s=96" width="64" height="64" alt="@yemiaoyong" title="@yemiaoyong"></a>
  <a href="https://github.com/yejia1988"><img src="https://avatars.githubusercontent.com/u/164177661?v=4&amp;s=96" width="64" height="64" alt="@yejia1988" title="@yejia1988"></a>
  <a href="https://github.com/xingzhuo123"><img src="https://avatars.githubusercontent.com/u/167210517?v=4&amp;s=96" width="64" height="64" alt="@xingzhuo123" title="@xingzhuo123"></a>
  <a href="https://github.com/xiaochuheying19901216"><img src="https://avatars.githubusercontent.com/u/304343377?v=4&amp;s=96" width="64" height="64" alt="@xiaochuheying19901216" title="@xiaochuheying19901216"></a>
  <a href="https://github.com/xiahouzuoying"><img src="https://avatars.githubusercontent.com/u/57342415?v=4&amp;s=96" width="64" height="64" alt="@xiahouzuoying" title="@xiahouzuoying"></a>
  <a href="https://github.com/likemoonriver"><img src="https://avatars.githubusercontent.com/u/157043962?v=4&amp;s=96" width="64" height="64" alt="@likemoonriver" title="@likemoonriver"></a>
  <a href="https://github.com/lijianguoa"><img src="https://avatars.githubusercontent.com/u/52228119?v=4&amp;s=96" width="64" height="64" alt="@lijianguoa" title="@lijianguoa"></a>
  <a href="https://github.com/k1600639239"><img src="https://avatars.githubusercontent.com/u/301947158?v=4&amp;s=96" width="64" height="64" alt="@k1600639239" title="@k1600639239"></a>
  <a href="https://github.com/gongmeiyuan"><img src="https://avatars.githubusercontent.com/u/75189860?v=4&amp;s=96" width="64" height="64" alt="@gongmeiyuan" title="@gongmeiyuan"></a>
  <a href="https://github.com/chhhhai"><img src="https://avatars.githubusercontent.com/u/99796066?v=4&amp;s=96" width="64" height="64" alt="@chhhhai" title="@chhhhai"></a>
  <a href="https://github.com/chenchen199401-cmyk"><img src="https://avatars.githubusercontent.com/u/236738705?v=4&amp;s=96" width="64" height="64" alt="@chenchen199401-cmyk" title="@chenchen199401-cmyk"></a>
  <a href="https://github.com/btzheng"><img src="https://avatars.githubusercontent.com/u/15546828?v=4&amp;s=96" width="64" height="64" alt="@btzheng" title="@btzheng"></a>
  <a href="https://github.com/Winteric123"><img src="https://avatars.githubusercontent.com/u/122366825?v=4&amp;s=96" width="64" height="64" alt="@Winteric123" title="@Winteric123"></a>
  <a href="https://github.com/ShixiangWang"><img src="https://avatars.githubusercontent.com/u/25057508?v=4&amp;s=96" width="64" height="64" alt="@ShixiangWang" title="@ShixiangWang"></a>
  <a href="https://github.com/ScholarlyLuck"><img src="https://avatars.githubusercontent.com/u/267531500?v=4&amp;s=96" width="64" height="64" alt="@ScholarlyLuck" title="@ScholarlyLuck"></a>
  <a href="https://github.com/Junweichengang"><img src="https://avatars.githubusercontent.com/u/41681007?v=4&amp;s=96" width="64" height="64" alt="@Junweichengang" title="@Junweichengang"></a>
  <a href="https://github.com/JarningGau"><img src="https://avatars.githubusercontent.com/u/22016330?v=4&amp;s=96" width="64" height="64" alt="@JarningGau" title="@JarningGau"></a>
  <a href="https://github.com/Cloudy-Zhuang"><img src="https://avatars.githubusercontent.com/u/85553170?v=4&amp;s=96" width="64" height="64" alt="@Cloudy-Zhuang" title="@Cloudy-Zhuang"></a>
  <a href="https://github.com/245429488zc-svg"><img src="https://avatars.githubusercontent.com/u/250579619?v=4&amp;s=96" width="64" height="64" alt="@245429488zc-svg" title="@245429488zc-svg"></a>
  <a href="https://github.com/chewice"><img src="https://avatars.githubusercontent.com/u/244145152?v=4&amp;s=96" width="64" height="64" alt="@chewice" title="@chewice"></a>
  <a href="https://github.com/XuuChen"><img src="https://avatars.githubusercontent.com/u/99383234?v=4&amp;s=96" width="64" height="64" alt="@XuuChen" title="@XuuChen"></a>
</p>

- We first looked at closed scientific-agent products such as Claude Science,
  then chose to build openly after finding them closed and unfriendly to users
  in some regions. Early work learned from their Skills and MCP tool selection;
  the agent architecture, workbench features, and roadmap are developed
  independently by the open-source community.
- Real-browser automation is inspired by
  [GenericAgent's GA Web / TMWebDriver](https://github.com/lsdefine/GenericAgent)
  architecture (MIT, Copyright 2025 lsdefine). Wisp's Rust bridge and Manifest
  V3 extension are an independent implementation; see
  [`browser-extension/NOTICE.md`](browser-extension/NOTICE.md).
- The agent core is based on
  [`w4n9H/mangopi-cli`](https://github.com/w4n9H/mangopi-cli) (Apache-2.0).
- `skills/` and `mcp-servers/bio-tools/` vendored from the upstream
  `superscience` asset bundle (Apache-2.0).
- `skills/bear-*` from [bear-research-skills](https://github.com/fei0810/bear-research-skills)
  (CC BY-NC-SA 4.0); requires `scimaster-cli` for live retrieval.
- `kernels/kernel_worker.py` protocol adapted from the upstream operon kernel
  worker, with POSIX-only `resource`/`/proc`/`SIGINT` machinery dropped for
  Windows.

## License

Except where otherwise noted, Wisp Science is licensed under the
[GNU Affero General Public License v3.0 only](LICENSE). Third-party and vendored
components remain under their respective licenses; upstream notices are
preserved in their directories, and the Apache License 2.0 text is retained in
[`LICENSES/Apache-2.0.txt`](LICENSES/Apache-2.0.txt). Earlier releases remain
available under the license published with those releases.

## Citation

If you use superscience in your research, please cite:

[![DOI](https://zenodo.org/badge/1285857639.svg)](https://doi.org/10.5281/zenodo.21193742)

```bibtex
@software{xu2026wisp,
  author    = {Xu, Zhougeng and hoptop},
  title     = {superscience: A local-first scientific computing agent},
  version   = {v0.33.0},
  year      = {2026},
  publisher = {Zenodo},
  doi       = {10.5281/zenodo.21193742},
  url       = {https://doi.org/10.5281/zenodo.21193742}
}
```

## Star History

<a href="https://star-history.com/#xuzhougeng/superscience&Date">
  <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=xuzhougeng/superscience&type=Date" />
</a>
