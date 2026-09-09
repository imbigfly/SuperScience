//! Specialists (专家): user-definable agent personas — instructions plus a
//! skill/MCP subset and a directly-bound model, selectable per session.
//! Stored as a JSON array under the `specialists` settings key (same pattern
//! as `model_profiles`). Built-ins are materialized on first read so user edits
//! to their model bindings persist like any other row.

use serde::{Deserialize, Serialize};
use superscience_store::Store;
use tauri::State;

pub const SPECIALISTS_KEY: &str = "specialists";
/// Per-builtin user overrides for `instructions`. Stored as a separate setting
/// so built-in compiled rubrics are never mutated in place — the user can
/// roll back at any time by clearing their override. Custom (non-builtin)
/// specialists own their `instructions` directly.
pub const SPECIALIST_INSTRUCTION_OVERRIDES_KEY: &str = "specialist_instruction_overrides";
pub const SCIENTIFIC_ILLUSTRATOR_RUBRIC: &str = "\
You are the Scientific Illustrator. Turn the user's request and relevant \
project/session context into a finished scientific figure asset, not merely \
drawing advice.\n\n\
Inspect referenced data and files before drawing. Never invent measurements, \
labels, sample sizes, or scientific conclusions. Load `figure-style` for \
data-backed plots and also `figure-composer` for multi-panel figures.\n\n\
Support exactly two output modes. An explicit user choice of format or method \
has the highest priority; tool availability decides only when the user did not \
choose:\n\
- Direct-SVG mode: use this when the user explicitly asks for SVG, vector, an \
editable figure, or direct SVG generation. Create the actual publication-ready \
figure as a descriptive `figures/*.svg` file using `write`, Python, or R. Do \
not call `generate_image`, do not replace the request with PNG, and do not claim \
SVG is unsupported merely because `generate_image` itself only returns PNG. \
After writing the SVG, rasterize that exact SVG to a PNG preview, inspect the \
preview with `view_image`, fix visible problems in the SVG source, then \
re-render and re-inspect. Repeat this SVG -> PNG preview -> SVG correction loop \
until the figure is legible and unclipped. The SVG is the primary deliverable; \
the PNG is only a QA preview.\n\
- PNG image-model mode: use this when the user explicitly asks for PNG, \
`gpt-image-2`, `grok-imagine-image-2.0`, `generate_image`, or image-model \
generation. Call \
`generate_image` with one complete, self-contained visual brief and save a \
descriptive `figures/*.png` file. If `generate_image` is unavailable, explain \
that an image-generation model must be configured; do not silently substitute \
SVG for an explicit PNG or image-model request.\n\
- When the user specifies neither format nor method, use PNG image-model mode \
if `generate_image` is available. Otherwise use Direct-SVG mode, including its \
SVG -> PNG preview -> SVG correction loop, and deliver the SVG.\n\n\
Keep text legible, use colour-blind-safe encodings, and distinguish observed \
data from conceptual illustration. End with a concise explanation and embed \
the saved figure using a project-relative Markdown image link.";

pub const R_BIOINFORMATICS_FIGURE_ID: &str = "r_bioinformatics_figure";
pub const R_BIOINFORMATICS_FIGURE_NAME: &str = "R bioinformatics figures";
pub const R_BIOINFORMATICS_FIGURE_DESCRIPTION: &str =
    "Draws common bioinformatics plots (volcano, heatmap, PCA/UMAP, survival, and similar) in R by default.";
pub const R_BIOINFORMATICS_FIGURE_RUBRIC: &str = r#"你是「生信图专家」。默认且优先用持久 `r` tool 与 ggplot2（及任务所需的 Bioconductor / CRAN 包）绘制数据驱动的生信图，而不是给空泛建议。

硬性规则：
1. Intake 总共最多五个澄清问题；每次优先只问一个短问题。禁止长问卷。
2. 优先请用户上传数据或给出项目内路径。禁止追问可从文件直接读出的元数据（行列数、schema、体积、列名清单等）——应自己用 `r` 读入后检查。
3. 只补齐阻塞项：图类型（火山图、热图、PCA/UMAP、箱线/小提琴、生存曲线、富集条形/气泡图等，或用户描述的目标）、分组/比较变量、输出偏好。默认交付 `figures/*.pdf`（或用户指定格式）并另存 PNG 预览。
4. 材料齐后立刻：读数 → 写/运行 R → 出图 → 用 `view_image` 质检预览 → 修正后交付。不要用「请确认长计划」拖延；一句「接下来我做 X」即可动手。
5. 无用户明确要求时，不要改用 Python，也不要调用 `generate_image`。
6. 不得捏造统计结果、p 值、样本或标签。缺 R 包时说明安装方式再继续。
7. 可按需 `use_skill` 加载 `nature-figure` / `figure-style`；多面板组合可参考其版式指引，但仍以 R 出图为主。

最后用项目相对路径的 Markdown 图片链接嵌入已保存图件，并简要说明所用脚本与图类型。"#;

pub const HANDWRITING_EXTRACT_ID: &str = "handwriting_extract";
pub const HANDWRITING_EXTRACT_RUBRIC: &str = "\
You are the Handwriting Extract specialist. Turn photos of handwritten lab \
notes, CRF pages, or whiteboard tables into a flagged project CSV. Do not \
invent numbers.\n\n\
Workflow:\n\
1. First reply: ask the user to upload or paste handwritten photos in this \
chat (at most five questions). A folder path is OK only if they name it. Do \
not list/glob/find the project for images. Do not call view_image until this \
chat has attachments or an explicit user-named path. Do not ask row or column \
counts you can read from the images.\n\
2. For each user-provided image, call view_image and extract structured JSON: \
page, image path, headers, and cells with text, confidence, optional \
normalized bbox [x,y,w,h] in 0-1, uncertain, and reason. Align pages to one \
schema and write `data/extracted/<batch>.json`.\n\
3. You MUST call `calibrate_handwriting` on that JSON before presenting \
results. Do not apply your own confidence heuristics as a substitute. The \
tool first applies table rules, then uses the bound calibration model for a \
second look at flagged cells only.\n\
4. Reply with the CSV path, the tool's reliability summary (ok_ratio, \
mean_confidence, uncertain/conflict counts — call this reliability / 可信度 \
评估, never medical accuracy), the QA list, annotated image paths, and \
whether to recapture or hand-edit.\n\n\
Hard rules:\n\
- Do not send the user to data_cleaning until a flagged CSV exists.\n\
- Do not use Tesseract book-OCR scripts from other skills.\n\
- Do not claim medical or CRF gold-standard accuracy. Humans confirm \
uncertain cells.\n\
- Do not mention view_image, calibrate_handwriting, or the outbound text \
firewall in user-facing replies.\n\
- If calibrate_handwriting is unavailable or the calibration model is unset, \
stop and point to the capability card settings.";

pub const FIGURE_DUPLICATE_AUDIT_ID: &str = "figure_duplicate_audit";
pub const FIGURE_DUPLICATE_AUDIT_RUBRIC: &str = "\
You are the Figure Duplicate Audit specialist. Audit scientific figures for \
duplicated, reused, transformed, or uninformative image panels. Hashes and \
feature matches find candidates; they do not establish misconduct or even \
duplication on their own.\n\n\
Workflow:\n\
1. First reply: confirm one PDF or one directory of manuscript images. Use \
attachments or a path the user named. Do not list, glob, or scan the project \
for inputs. Ask for a page range only when the user did not specify one and \
scanning the whole PDF would materially change scope.\n\
2. Load `figure-duplicate-audit` and follow its prepare / panel-split / \
all-pairs / visual-evidence / coverage / cautious-report protocol. Create a \
new analysis directory; never modify source images or overwrite a prior audit.\n\
3. If the audit scripts cannot import their required packages, load \
`local-env-setup` and install the skill compatibility list. Do not continue \
with a hash-only fallback when the user asked for a strict or exhaustive \
review.\n\
4. Deliver the reviewed panel manifest, candidate table, visual evidence, \
coverage accounting, and a cautious integrity report. Say what is a candidate \
and what is not proven.\n\n\
Hard rules:\n\
- Do not invent a PDF or figure directory.\n\
- Do not call a hash or feature match misconduct.\n\
- Do not skip coverage accounting or silently omit unreadable files.";

pub const CONVERSATION_ANALYSIS_ID: &str = "conversation_analysis";
pub const CONVERSATION_ANALYSIS_NAME: &str = "对话分析";
pub const CONVERSATION_ANALYSIS_DESCRIPTION: &str =
    "分析系统内全部项目的全部会话，提取每个会话中用户发送的所有消息，以及模型在每一轮对话中最终回复的结论性内容，并以对话聊天的形式生成一个 HTML 报告。";
pub const CONVERSATION_ANALYSIS_RUBRIC: &str = r#"# 对话分析 Agent 提示词

## 任务目标
分析系统内全部项目的全部会话，提取每个会话中用户发送的所有消息，以及模型在每一轮对话中最终回复的结论性内容，并以对话聊天的形式生成一个 HTML 报告。

## 输入数据
- 数据源是应用里已保存的全部会话，不限项目
- 排除当前这场对话分析会话、无用户消息的空草稿、以及随手一聊（scratch）会话
- 必须通过 `export_conversation_analysis` 读取；不要让用户粘贴记录，不要用 read/glob/find 扫项目文件

## 工作流程
1. 第一轮先调用 `export_conversation_analysis`（不要传 confirmed，或 confirmed=false），拿到会话清单。
2. 向用户确认：将分析全部项目的全部会话，共多少场、来自哪些项目。未确认前禁止导出报告。
3. 用户明确确认后，再调用 `export_conversation_analysis`，参数 confirmed=true。工具会按下方分析要求写出 `对话分析报告.html`。
4. 不要自己手写 HTML。报告写好后告诉用户文件路径。

## 分析要求
1. **识别会话**：按会话标识（如会话 ID、标题或时间分隔）将对话划分为独立单元。
2. **提取用户消息**：在每个会话中，提取所有由用户发送的文本内容（包括提问、指令、补充说明等）。
3. **提取模型结论**：针对每一轮用户消息，提取模型在该轮对话中**最后一条回复的完整内容**作为“模型结论”。若模型回复包含多条消息，仅保留最后一条；若回复中明确有“总结”“结论”等部分，优先提取该部分。
4. **忽略中间过程**：不展示模型在最终结论之前产生的任何中间思考、追问或分段回复（除非它们是该轮最后一条）。
5. **保持对话顺序**：按照时间顺序，将用户消息与对应的模型结论配对，形成一组对话记录。

## 输出格式
- **文件类型**：单个 HTML 文件，命名为 `对话分析报告.html`
- **展示方式**：以聊天对话风格呈现，用户消息靠右（或标注“用户”），模型结论靠左（或标注“模型”），并用不同背景色区分。
- **会话导航（必须）**：会话很多时，禁止把所有会话纵向堆成一长页让人滚动查找。必须做成「左侧选会话、右侧看对话」：
  - 左侧固定会话列表，每项显示序号、标题或起始时间、轮次数
  - 列表顶部提供搜索框，按标题或时间即时过滤
  - 点击列表项后，右侧只显示该会话；默认显示第 1 个会话
  - 所有会话仍写在同一 HTML 里，不得省略；未选中的会话用 CSS 隐藏
  - 单个文件，只用 HTML + CSS + 少量 JS，不引用外链
- **会话区分**：每个独立会话使用稳定 `id`（如 `session-1`），标题形如“会话 1：<标题或起始时间>”。
- **可读性**：使用简洁的 CSS 样式，保证文字清晰、排版整齐，适合直接查看。
- **完整性**：报告中必须包含所有会话、所有轮次的对话，不得省略。

## 示例格式（HTML 片段）
```html
<div class="layout">
  <aside class="nav">
    <input id="q" placeholder="搜索会话…" oninput="filterSessions(this.value)">
    <button type="button" class="nav-item active" data-target="session-1" onclick="showSession('session-1')">会话 1：春天的诗 · 1 轮</button>
    <button type="button" class="nav-item" data-target="session-2" onclick="showSession('session-2')">会话 2：…</button>
  </aside>
  <main class="main">
    <section id="session-1" class="session active">
      <h2>会话 1：春天的诗</h2>
      <div class="chat">
        <div class="user-msg">用户：帮我写一首关于春天的诗</div>
        <div class="model-msg">模型：春风轻拂绿柳梢，桃花朵朵笑迎朝。燕归梁上呢喃语，一片生机满野郊。</div>
      </div>
    </section>
    <section id="session-2" class="session">…</section>
  </main>
</div>
<script>
function showSession(id) {
  document.querySelectorAll('.session').forEach(el => el.classList.toggle('active', el.id === id));
  document.querySelectorAll('.nav-item').forEach(el => el.classList.toggle('active', el.dataset.target === id));
}
function filterSessions(q) {
  const needle = (q || '').trim().toLowerCase();
  document.querySelectorAll('.nav-item').forEach(el => {
    el.hidden = needle !== '' && !el.textContent.toLowerCase().includes(needle);
  });
}
</script>
```"#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Specialist {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub instructions: String,
    /// Compiled rubric for built-in specialists; `None` for user-created ones.
    /// The UI surfaces this as the "factory default" so the user can compare
    /// against their own edits and roll back with one click.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_instructions: Option<String>,
    /// "" = follow the active model; dangling ids fall back to active too.
    #[serde(default)]
    pub model_id: String,
    /// Reviewer-only backend selection. `None` preserves the legacy behavior:
    /// use `model_id`, falling back to the active HTTP model. Other specialist
    /// personas continue to run inside Wisp's native agent loop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_backend: Option<crate::review::ReviewBackendConfig>,
    /// None = inherit the project skill config; Some = whitelist of skill names.
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    /// None = inherit; Some = whitelist of connector slugs / MCP connection ids.
    #[serde(default)]
    pub connectors: Option<Vec<String>>,
    #[serde(default)]
    pub builtin: bool,
}

pub fn builtin_reviewer() -> Specialist {
    Specialist {
        id: "reviewer".into(),
        name: "Reviewer".into(),
        icon: "review".into(),
        color: "clay".into(),
        description:
            "Traces a session transcript and reports fabrication, hallucination, or plan deviation."
                .into(),
        instructions: crate::review::REVIEWER_RUBRIC.into(),
        default_instructions: Some(crate::review::REVIEWER_RUBRIC.into()),
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec![]), // reviewer runs one-shot; skills are irrelevant
        connectors: Some(vec![]),
        builtin: true,
    }
}

pub fn builtin_reader() -> Specialist {
    Specialist {
        id: "reader".into(),
        name: "Reader".into(),
        icon: "search".into(),
        color: "clay".into(),
        description: "Searches project sessions in parallel and returns compact, cited evidence."
            .into(),
        instructions: crate::project_reader::READER_RUBRIC.into(),
        default_instructions: Some(crate::project_reader::READER_RUBRIC.into()),
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec![]),
        connectors: Some(vec![]),
        builtin: true,
    }
}

pub fn builtin_scientific_illustrator() -> Specialist {
    Specialist {
        id: "scientific_illustrator".into(),
        name: "Scientific Illustrator".into(),
        icon: "image".into(),
        color: "clay".into(),
        description:
            "Creates publication-ready scientific figures from the request and project context."
                .into(),
        instructions: SCIENTIFIC_ILLUSTRATOR_RUBRIC.into(),
        default_instructions: Some(SCIENTIFIC_ILLUSTRATOR_RUBRIC.into()),
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec!["figure-composer".into(), "figure-style".into()]),
        connectors: Some(vec![]),
        builtin: true,
    }
}

pub fn builtin_r_bioinformatics_figure() -> Specialist {
    Specialist {
        id: R_BIOINFORMATICS_FIGURE_ID.into(),
        name: R_BIOINFORMATICS_FIGURE_NAME.into(),
        icon: "grid".into(),
        color: "clay".into(),
        description: R_BIOINFORMATICS_FIGURE_DESCRIPTION.into(),
        instructions: R_BIOINFORMATICS_FIGURE_RUBRIC.into(),
        default_instructions: Some(R_BIOINFORMATICS_FIGURE_RUBRIC.into()),
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec!["figure-style".into(), "nature-figure".into()]),
        connectors: Some(vec![]),
        builtin: true,
    }
}

pub fn builtin_handwriting_extract() -> Specialist {
    Specialist {
        id: HANDWRITING_EXTRACT_ID.into(),
        name: "Handwriting Extract".into(),
        icon: "grid".into(),
        color: "clay".into(),
        description: "Reads handwritten lab or CRF photos into a flagged CSV, then calibrates uncertain cells.".into(),
        instructions: HANDWRITING_EXTRACT_RUBRIC.into(),
        default_instructions: Some(HANDWRITING_EXTRACT_RUBRIC.into()),
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec!["handwriting-extract".into()]),
        connectors: Some(vec![]),
        builtin: true,
    }
}

pub fn builtin_figure_duplicate_audit() -> Specialist {
    Specialist {
        id: FIGURE_DUPLICATE_AUDIT_ID.into(),
        name: "Figure Duplicate Audit".into(),
        icon: "copy".into(),
        color: "clay".into(),
        description: "Audit figures for duplicated, reused, or transformed panels.".into(),
        instructions: FIGURE_DUPLICATE_AUDIT_RUBRIC.into(),
        default_instructions: Some(FIGURE_DUPLICATE_AUDIT_RUBRIC.into()),
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec![
            "figure-duplicate-audit".into(),
            "local-env-setup".into(),
        ]),
        connectors: Some(vec![]),
        builtin: true,
    }
}

pub fn builtin_conversation_analysis() -> Specialist {
    Specialist {
        id: CONVERSATION_ANALYSIS_ID.into(),
        name: CONVERSATION_ANALYSIS_NAME.into(),
        icon: "chat".into(),
        color: "clay".into(),
        description: CONVERSATION_ANALYSIS_DESCRIPTION.into(),
        instructions: CONVERSATION_ANALYSIS_RUBRIC.into(),
        default_instructions: Some(CONVERSATION_ANALYSIS_RUBRIC.into()),
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec![]),
        connectors: Some(vec![]),
        builtin: true,
    }
}

fn pin_r_bioinformatics_figure(spec: &mut Specialist) {
    spec.builtin = true;
    spec.default_instructions = Some(R_BIOINFORMATICS_FIGURE_RUBRIC.into());
    spec.review_backend = None;
    spec.skills = Some(vec!["figure-style".into(), "nature-figure".into()]);
    spec.connectors = Some(vec![]);
}

fn pin_handwriting_extract(spec: &mut Specialist) {
    spec.builtin = true;
    spec.default_instructions = Some(HANDWRITING_EXTRACT_RUBRIC.into());
    spec.review_backend = None;
    spec.skills = Some(vec!["handwriting-extract".into()]);
    spec.connectors = Some(vec![]);
    // Chat follows the session model from Settings → Models. Analysis and
    // calibration are separate profile picks, not a second API-key store.
    spec.model_id.clear();
}

fn pin_figure_duplicate_audit(spec: &mut Specialist) {
    spec.builtin = true;
    spec.default_instructions = Some(FIGURE_DUPLICATE_AUDIT_RUBRIC.into());
    spec.review_backend = None;
    spec.skills = Some(vec![
        "figure-duplicate-audit".into(),
        "local-env-setup".into(),
    ]);
    spec.connectors = Some(vec![]);
}

fn pin_conversation_analysis(spec: &mut Specialist) {
    spec.builtin = true;
    spec.name = CONVERSATION_ANALYSIS_NAME.into();
    spec.description = CONVERSATION_ANALYSIS_DESCRIPTION.into();
    spec.default_instructions = Some(CONVERSATION_ANALYSIS_RUBRIC.into());
    spec.review_backend = None;
    spec.skills = Some(vec![]);
    spec.connectors = Some(vec![]);
}

async fn load_raw(store: &Store) -> Vec<Specialist> {
    store
        .get_setting(SPECIALISTS_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<Vec<Specialist>>(&s).ok())
        .unwrap_or_default()
}

async fn save_raw(store: &Store, list: &[Specialist]) -> Result<(), String> {
    let json = serde_json::to_string(list).map_err(|e| e.to_string())?;
    store
        .set_setting(SPECIALISTS_KEY, &json)
        .await
        .map_err(|e| e.to_string())
}

async fn load_overrides(store: &Store) -> std::collections::HashMap<String, String> {
    store
        .get_setting(SPECIALIST_INSTRUCTION_OVERRIDES_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<std::collections::HashMap<String, String>>(&s).ok())
        .unwrap_or_default()
}

async fn save_overrides(
    store: &Store,
    overrides: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let json = serde_json::to_string(overrides).map_err(|e| e.to_string())?;
    store
        .set_setting(SPECIALIST_INSTRUCTION_OVERRIDES_KEY, &json)
        .await
        .map_err(|e| e.to_string())
}

/// Load the list, materializing builtins if absent. Builtin `default_instructions`
/// are always re-pinned to their compiled rubrics so improvements ship without
/// a settings migration. Active `instructions` for builtins come from
/// `specialist_instruction_overrides` if the user has edited them, otherwise
/// from `default_instructions`.
pub async fn ensure(store: &Store) -> Vec<Specialist> {
    let mut list = load_raw(store).await;
    let overrides = load_overrides(store).await;
    match list.iter_mut().find(|s| s.id == "reviewer") {
        Some(r) => {
            r.builtin = true;
            r.default_instructions = Some(crate::review::REVIEWER_RUBRIC.into());
        }
        None => list.insert(0, builtin_reviewer()),
    }
    match list.iter_mut().find(|s| s.id == "reader") {
        Some(reader) => {
            reader.builtin = true;
            reader.default_instructions = Some(crate::project_reader::READER_RUBRIC.into());
            reader.review_backend = None;
            reader.skills = Some(vec![]);
            reader.connectors = Some(vec![]);
        }
        None => list.insert(1.min(list.len()), builtin_reader()),
    }
    match list.iter_mut().find(|s| s.id == "scientific_illustrator") {
        Some(illustrator) => {
            illustrator.builtin = true;
            illustrator.default_instructions = Some(SCIENTIFIC_ILLUSTRATOR_RUBRIC.into());
            illustrator.review_backend = None;
            illustrator.skills = Some(vec!["figure-composer".into(), "figure-style".into()]);
            illustrator.connectors = Some(vec![]);
        }
        None => list.insert(2.min(list.len()), builtin_scientific_illustrator()),
    }
    match list.iter_mut().find(|s| s.id == R_BIOINFORMATICS_FIGURE_ID) {
        Some(bio) => pin_r_bioinformatics_figure(bio),
        None => list.insert(3.min(list.len()), builtin_r_bioinformatics_figure()),
    }
    match list.iter_mut().find(|s| s.id == HANDWRITING_EXTRACT_ID) {
        Some(extractor) => pin_handwriting_extract(extractor),
        None => list.insert(4.min(list.len()), builtin_handwriting_extract()),
    }
    match list.iter_mut().find(|s| s.id == FIGURE_DUPLICATE_AUDIT_ID) {
        Some(auditor) => pin_figure_duplicate_audit(auditor),
        None => list.insert(5.min(list.len()), builtin_figure_duplicate_audit()),
    }
    match list.iter_mut().find(|s| s.id == CONVERSATION_ANALYSIS_ID) {
        Some(analyst) => pin_conversation_analysis(analyst),
        None => list.insert(6.min(list.len()), builtin_conversation_analysis()),
    }
    for spec in list.iter_mut() {
        if spec.builtin {
            if let Some(overridden) = overrides.get(&spec.id) {
                spec.instructions = overridden.clone();
            } else if let Some(default) = spec.default_instructions.clone() {
                spec.instructions = default;
            }
        }
    }
    list
}

pub async fn get(store: &Store, id: &str) -> Option<Specialist> {
    ensure(store).await.into_iter().find(|s| s.id == id)
}

/// Bound calibration model for handwriting-extract.
/// Prefers the dedicated settings key; falls back to the specialist row.
pub async fn handwriting_extract_calibration_id(store: &Store) -> Option<String> {
    if let Some(id) = crate::models::handwriting_extract_calibration_id(store).await {
        return Some(id);
    }
    let spec = get(store, HANDWRITING_EXTRACT_ID).await?;
    crate::models::resolve_assigned_vision_id(store, spec.model_id.trim()).await
}

fn fresh_id(existing: &[Specialist]) -> String {
    for n in 1..10_000 {
        let id = format!("sp{n}");
        if !existing.iter().any(|s| s.id == id) {
            return id;
        }
    }
    "sp".into()
}

/// Create (empty id) or update (existing id). Builtin rows can never lose
/// `builtin`, and structural fields (skills/connectors/review_backend) stay
/// pinned to the compiled rubric. The user's `instructions` is allowed
/// through and persisted to `specialist_instruction_overrides` so the
/// factory rubric remains recoverable.
pub async fn upsert(store: &Store, mut spec: Specialist) -> Result<Vec<Specialist>, String> {
    if spec.name.trim().is_empty() {
        return Err("Specialist name is required.".into());
    }
    let mut list = ensure(store).await;
    if spec.id.trim().is_empty() {
        spec.id = fresh_id(&list);
    }
    // Capture the id before `spec` is moved into the row so the override
    // persistence below can still inspect it.
    let upserted_id = spec.id.clone();
    let mut builtin_override: Option<String> = None;
    if let Some(existing) = list.iter_mut().find(|s| s.id == spec.id) {
        if existing.builtin {
            spec.builtin = true;
            // Capture the user's edited instructions so we can persist them to
            // the override map. The stored row keeps the compiled rubric so
            // ensure() can fall back to it if the user resets.
            builtin_override = Some(spec.instructions.clone());
            if let Some(default) = existing.default_instructions.clone() {
                spec.default_instructions = Some(default);
            }
        }
        if spec.id == "reviewer" {
            if let Some(crate::review::ReviewBackendConfig::HttpModel { profile_id }) =
                &spec.review_backend
            {
                // Keep the old field in sync so downgrades and older settings
                // surfaces retain the selected HTTP reviewer.
                spec.model_id = profile_id.clone();
            }
        } else if spec.id == "reader" {
            spec.review_backend = None;
            spec.skills = Some(vec![]);
            spec.connectors = Some(vec![]);
        } else if spec.id == "scientific_illustrator" {
            spec.review_backend = None;
            spec.skills = Some(vec!["figure-composer".into(), "figure-style".into()]);
            spec.connectors = Some(vec![]);
        } else if spec.id == R_BIOINFORMATICS_FIGURE_ID {
            spec.review_backend = None;
            spec.skills = Some(vec!["figure-style".into(), "nature-figure".into()]);
            spec.connectors = Some(vec![]);
        } else if spec.id == HANDWRITING_EXTRACT_ID {
            spec.review_backend = None;
            spec.skills = Some(vec!["handwriting-extract".into()]);
            spec.connectors = Some(vec![]);
            spec.model_id.clear();
        } else if spec.id == FIGURE_DUPLICATE_AUDIT_ID {
            spec.review_backend = None;
            spec.skills = Some(vec![
                "figure-duplicate-audit".into(),
                "local-env-setup".into(),
            ]);
            spec.connectors = Some(vec![]);
        } else if spec.id == CONVERSATION_ANALYSIS_ID {
            spec.name = CONVERSATION_ANALYSIS_NAME.into();
            spec.description = CONVERSATION_ANALYSIS_DESCRIPTION.into();
            spec.review_backend = None;
            spec.skills = Some(vec![]);
            spec.connectors = Some(vec![]);
        }
        *existing = spec;
    } else {
        spec.builtin = false;
        list.push(spec);
    }
    save_raw(store, &list).await?;
    // If the upserted row is a built-in, persist the user-edited instructions
    // to the override map. ensure() then re-applies the override on the next
    // read, so the compiled rubric stays recoverable.
    if let (Some(overridden), Some(target_id)) = (
        builtin_override,
        list.iter()
            .find(|s| s.builtin && s.id == upserted_id)
            .map(|s| s.id.clone()),
    ) {
        let mut overrides = load_overrides(store).await;
        // If the user typed exactly the default rubric back, drop the override
        // so the row reads from default_instructions on next ensure().
        let default = list
            .iter()
            .find(|s| s.id == target_id)
            .and_then(|s| s.default_instructions.clone())
            .unwrap_or_default();
        if overridden.trim() == default.trim() {
            overrides.remove(&target_id);
        } else {
            overrides.insert(target_id, overridden);
        }
        save_overrides(store, &overrides).await?;
    }
    Ok(ensure(store).await)
}

pub async fn remove(store: &Store, id: &str) -> Result<Vec<Specialist>, String> {
    let mut list = ensure(store).await;
    if list.iter().any(|s| s.id == id && s.builtin) {
        return Err("Built-in specialists cannot be removed.".into());
    }
    list.retain(|s| s.id != id);
    save_raw(store, &list).await?;
    Ok(ensure(store).await)
}

#[tauri::command]
pub async fn list_specialists(
    state: State<'_, crate::AppState>,
) -> Result<Vec<Specialist>, String> {
    Ok(ensure(&state.store).await)
}

#[tauri::command]
pub async fn save_specialist_cmd(
    state: State<'_, crate::AppState>,
    spec: Specialist,
) -> Result<Vec<Specialist>, String> {
    upsert(&state.store, spec).await
}

#[tauri::command]
pub async fn remove_specialist(
    state: State<'_, crate::AppState>,
    id: String,
) -> Result<Vec<Specialist>, String> {
    remove(&state.store, &id).await
}

/// Persist a user-edited instruction string for a built-in specialist. The
/// value is stored in the `specialist_instruction_overrides` settings map; the
/// compiled rubric remains recoverable through `reset_specialist_instructions`.
/// Returns the updated specialist list so the UI can re-render in one call.
#[tauri::command]
pub async fn save_specialist_instructions(
    state: State<'_, crate::AppState>,
    id: String,
    instructions: String,
) -> Result<Vec<Specialist>, String> {
    let list = ensure(&state.store).await;
    let spec = list
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("Unknown specialist '{id}'."))?;
    if !spec.builtin {
        return Err("Custom specialists edit instructions directly via save_specialist.".into());
    }
    let mut overrides = load_overrides(&state.store).await;
    let default = spec.default_instructions.clone().unwrap_or_default();
    if instructions.trim() == default.trim() {
        overrides.remove(&id);
    } else {
        overrides.insert(id.clone(), instructions);
    }
    save_overrides(&state.store, &overrides).await?;
    Ok(ensure(&state.store).await)
}

/// Drop the user override for a built-in specialist's instructions. The next
/// read returns the compiled rubric via `default_instructions`.
#[tauri::command]
pub async fn reset_specialist_instructions(
    state: State<'_, crate::AppState>,
    id: String,
) -> Result<Vec<Specialist>, String> {
    let list = ensure(&state.store).await;
    if !list.iter().any(|s| s.id == id && s.builtin) {
        return Err(format!("Unknown or non-builtin specialist '{id}'."));
    }
    let mut overrides = load_overrides(&state.store).await;
    overrides.remove(&id);
    save_overrides(&state.store, &overrides).await?;
    Ok(ensure(&state.store).await)
}

/// LLM config for a specialist: its bound profile, or the active-model chain
/// when unbound/dangling (soft fallback — personas are not hard capabilities).
pub async fn specialist_llm(
    store: &Store,
    spec: &Specialist,
) -> (String, String, String, String, u64, String, String) {
    if !spec.model_id.trim().is_empty() {
        if let Some(cfg) = crate::models::profile_llm(store, &spec.model_id).await {
            return cfg;
        }
    }
    let (provider, api_url, model, api_key) = crate::load_settings(store).await;
    let (max_tokens, reasoning_effort, service_tier) =
        crate::models::active_llm_advanced(store).await;
    (
        provider,
        api_url,
        model,
        api_key,
        max_tokens,
        reasoning_effort,
        service_tier,
    )
}

pub async fn specialist_context_window(store: &Store, spec: &Specialist) -> u64 {
    if !spec.model_id.trim().is_empty() {
        if let Some(window) = crate::models::profile_context_window(store, &spec.model_id).await {
            return window;
        }
    }
    crate::models::active_context_window(store).await
}

fn frame_key(frame_id: &str) -> String {
    format!("frame_specialist:{frame_id}")
}

pub async fn set_frame_specialist(store: &Store, frame_id: &str, id: &str) -> Result<(), String> {
    store
        .set_setting(&frame_key(frame_id), id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn session_specialist(store: &Store, frame_id: &str) -> Option<Specialist> {
    let id = store
        .get_setting(&frame_key(frame_id))
        .await
        .ok()
        .flatten()?;
    if id.trim().is_empty() {
        return None;
    }
    get(store, &id).await
}

/// The UI disables the picker once a session has messages; this backend guard
/// enforces the same rule for any other caller.
#[tauri::command]
pub async fn set_session_specialist(
    state: State<'_, crate::AppState>,
    frame_id: String,
    id: String,
) -> Result<(), String> {
    let msgs = state
        .store
        .load_messages(&frame_id)
        .await
        .map_err(|e| format!("{e}"))?;
    if msgs
        .iter()
        .any(|m| m.role != superscience_llm::Role::System)
    {
        return Err("Specialist is locked once the session has messages.".into());
    }
    if !id.is_empty() && get(&state.store, &id).await.is_none() {
        return Err(format!("Unknown specialist '{id}'."));
    }
    set_frame_specialist(&state.store, &frame_id, &id).await
}

#[tauri::command]
pub async fn get_session_specialist(
    state: State<'_, crate::AppState>,
    frame_id: String,
) -> Result<Option<Specialist>, String> {
    Ok(session_specialist(&state.store, &frame_id).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_analysis_rubric_requires_html_report() {
        let rubric = CONVERSATION_ANALYSIS_RUBRIC;
        assert!(rubric.starts_with("# 对话分析 Agent 提示词"));
        assert!(rubric.contains("## 任务目标"));
        assert!(rubric.contains("## 输入数据"));
        assert!(rubric.contains("## 分析要求"));
        assert!(rubric.contains("提取模型结论"));
        assert!(rubric.contains("忽略中间过程"));
        assert!(rubric.contains("对话分析报告.html"));
        assert!(rubric.contains("export_conversation_analysis"));
        assert!(rubric.contains("全部项目"));
        assert!(rubric.contains("未确认前禁止导出报告"));
        assert!(rubric.contains("会话导航（必须）"));
        assert!(rubric.contains("左侧选会话、右侧看对话"));
        assert!(rubric.contains("function showSession"));
        assert!(rubric.contains("<div class=\"user-msg\">用户：帮我写一首关于春天的诗</div>"));
        assert_eq!(CONVERSATION_ANALYSIS_NAME, "对话分析");
        assert_eq!(CONVERSATION_ANALYSIS_DESCRIPTION, "分析系统内全部项目的全部会话，提取每个会话中用户发送的所有消息，以及模型在每一轮对话中最终回复的结论性内容，并以对话聊天的形式生成一个 HTML 报告。");
    }

    #[test]
    fn handwriting_rubric_requires_calibrate_tool() {
        let rubric = HANDWRITING_EXTRACT_RUBRIC;
        assert!(rubric.contains("calibrate_handwriting"));
        assert!(rubric.contains("Do not invent numbers"));
        assert!(rubric.contains("never medical"));
        assert!(!rubric.contains("Warn once"));
        assert!(rubric.contains("Do not mention view_image"));
        assert!(rubric.contains("Do not list/glob/find the project for images"));
        assert!(rubric.contains("Do not call view_image until this"));
    }

    #[test]
    fn illustrator_rubric_gives_explicit_svg_requests_priority() {
        let rubric = SCIENTIFIC_ILLUSTRATOR_RUBRIC;
        let svg_rule = rubric
            .find("when the user explicitly asks for SVG")
            .expect("rubric must define explicit SVG routing");
        let fallback_rule = rubric
            .find("When the user specifies neither format nor method")
            .expect("rubric must define the tool-availability fallback");

        assert!(svg_rule < fallback_rule);
        assert!(rubric.contains("explicit user choice of format or method"));
        assert!(rubric.contains("Do not call `generate_image`"));
        assert!(rubric.contains("rasterize that exact SVG to a PNG preview"));
        assert!(rubric.contains("inspect the preview with `view_image`"));
        assert!(rubric.contains("SVG -> PNG preview -> SVG correction loop"));
        assert!(rubric.contains("The SVG is the primary deliverable"));
        assert!(rubric.contains("explicitly asks for PNG"));
        assert!(rubric.contains("do not silently substitute"));
    }

    async fn test_store() -> (superscience_store::Store, std::path::PathBuf) {
        let tmp = std::env::temp_dir().join(format!("wisp_spec_{}.sqlite", uuid::Uuid::new_v4()));
        (superscience_store::Store::open(&tmp).await.unwrap(), tmp)
    }

    #[tokio::test]
    async fn ensure_materializes_builtin_specialists_once() {
        let (store, tmp) = test_store().await;
        let list = ensure(&store).await;
        assert_eq!(list.len(), 7);
        let r = &list[0];
        assert_eq!(r.id, "reviewer");
        assert!(r.builtin);
        assert_eq!(r.instructions, crate::review::REVIEWER_RUBRIC);
        let reader = &list[1];
        assert_eq!(reader.id, "reader");
        assert!(reader.builtin);
        assert_eq!(reader.instructions, crate::project_reader::READER_RUBRIC);
        let illustrator = &list[2];
        assert_eq!(illustrator.id, "scientific_illustrator");
        assert!(illustrator.builtin);
        assert_eq!(illustrator.instructions, SCIENTIFIC_ILLUSTRATOR_RUBRIC);
        assert_eq!(
            illustrator.skills.as_deref(),
            Some(&["figure-composer".to_string(), "figure-style".to_string()][..])
        );
        let bio = &list[3];
        assert_eq!(bio.id, R_BIOINFORMATICS_FIGURE_ID);
        assert!(bio.builtin);
        assert_eq!(bio.instructions, R_BIOINFORMATICS_FIGURE_RUBRIC);
        assert_eq!(
            bio.skills.as_deref(),
            Some(&["figure-style".to_string(), "nature-figure".to_string()][..])
        );
        let extractor = &list[4];
        assert_eq!(extractor.id, HANDWRITING_EXTRACT_ID);
        assert!(extractor.builtin);
        assert_eq!(extractor.instructions, HANDWRITING_EXTRACT_RUBRIC);
        assert_eq!(
            extractor.skills.as_deref(),
            Some(&["handwriting-extract".to_string()][..])
        );
        assert!(extractor.model_id.is_empty());
        assert!(handwriting_extract_calibration_id(&store).await.is_none());
        let auditor = &list[5];
        assert_eq!(auditor.id, FIGURE_DUPLICATE_AUDIT_ID);
        assert!(auditor.builtin);
        assert_eq!(auditor.instructions, FIGURE_DUPLICATE_AUDIT_RUBRIC);
        assert_eq!(
            auditor.skills.as_deref(),
            Some(
                &[
                    "figure-duplicate-audit".to_string(),
                    "local-env-setup".to_string()
                ][..]
            )
        );
        let analyst = &list[6];
        assert_eq!(analyst.id, CONVERSATION_ANALYSIS_ID);
        assert!(analyst.builtin);
        assert_eq!(analyst.name, CONVERSATION_ANALYSIS_NAME);
        assert_eq!(analyst.description, CONVERSATION_ANALYSIS_DESCRIPTION);
        assert_eq!(analyst.instructions, CONVERSATION_ANALYSIS_RUBRIC);
        assert_eq!(analyst.skills.as_deref(), Some(&[][..]));
        // Second read does not duplicate the built-ins.
        assert_eq!(ensure(&store).await.len(), 7);

        let mut leftover = get(&store, HANDWRITING_EXTRACT_ID).await.unwrap();
        leftover.model_id = "m5".into();
        upsert(&store, leftover).await.unwrap();
        assert!(
            get(&store, HANDWRITING_EXTRACT_ID)
                .await
                .unwrap()
                .model_id
                .is_empty(),
            "handwriting chat must follow Settings → Models, not a leftover specialist binding"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn upsert_roundtrip_and_fresh_id() {
        let (store, tmp) = test_store().await;
        let spec = Specialist {
            id: String::new(),
            name: "Paper hunter".into(),
            icon: "search".into(),
            color: "clay".into(),
            description: "finds papers".into(),
            instructions: "You hunt papers.".into(),
            default_instructions: None,
            model_id: "m1".into(),
            review_backend: None,
            skills: Some(vec!["bear-support".into()]),
            connectors: None,
            builtin: false,
        };
        let list = upsert(&store, spec).await.unwrap();
        let created = list.iter().find(|s| !s.builtin).unwrap();
        assert_eq!(created.id, "sp1");
        assert_eq!(
            created.skills.as_deref(),
            Some(&["bear-support".to_string()][..])
        );
        // Edit by id keeps the id.
        let mut edited = created.clone();
        edited.name = "Paper hunter 2".into();
        let list = upsert(&store, edited).await.unwrap();
        assert_eq!(list.iter().filter(|s| !s.builtin).count(), 1);
        assert_eq!(
            list.iter().find(|s| s.id == "sp1").unwrap().name,
            "Paper hunter 2"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn builtin_specialist_guards() {
        let (store, tmp) = test_store().await;
        ensure(&store).await;
        assert!(remove(&store, "reviewer").await.is_err());
        assert!(remove(&store, "reader").await.is_err());
        assert!(remove(&store, "scientific_illustrator").await.is_err());
        assert!(remove(&store, R_BIOINFORMATICS_FIGURE_ID).await.is_err());
        assert!(remove(&store, HANDWRITING_EXTRACT_ID).await.is_err());
        assert!(remove(&store, FIGURE_DUPLICATE_AUDIT_ID).await.is_err());
        assert!(remove(&store, CONVERSATION_ANALYSIS_ID).await.is_err());
        // Built-in instructions are now user-editable. The override persists
        // separately from the compiled rubric so it can be rolled back.
        let mut r = get(&store, "reviewer").await.unwrap();
        r.instructions = "haha".into();
        r.model_id = "m2".into();
        let list = upsert(&store, r).await.unwrap();
        let r = list.iter().find(|s| s.id == "reviewer").unwrap();
        assert_eq!(r.instructions, "haha");
        assert_eq!(r.model_id, "m2");
        assert_eq!(
            r.default_instructions.as_deref(),
            Some(crate::review::REVIEWER_RUBRIC)
        );

        let mut reader = get(&store, "reader").await.unwrap();
        reader.instructions = "replace rubric".into();
        reader.model_id = "cheap".into();
        reader.skills = None;
        let list = upsert(&store, reader).await.unwrap();
        let reader = list
            .iter()
            .find(|specialist| specialist.id == "reader")
            .unwrap();
        assert_eq!(reader.instructions, "replace rubric");
        assert_eq!(reader.model_id, "cheap");
        assert_eq!(reader.skills, Some(vec![]));
        assert_eq!(
            reader.default_instructions.as_deref(),
            Some(crate::project_reader::READER_RUBRIC),
        );

        let mut illustrator = get(&store, "scientific_illustrator").await.unwrap();
        illustrator.instructions = "replace rubric".into();
        illustrator.skills = None;
        let list = upsert(&store, illustrator).await.unwrap();
        let illustrator = list
            .iter()
            .find(|specialist| specialist.id == "scientific_illustrator")
            .unwrap();
        assert_eq!(illustrator.instructions, "replace rubric");
        assert_eq!(
            illustrator.skills,
            Some(vec!["figure-composer".into(), "figure-style".into()])
        );
        assert_eq!(
            illustrator.default_instructions.as_deref(),
            Some(SCIENTIFIC_ILLUSTRATOR_RUBRIC),
        );

        let mut bio = get(&store, R_BIOINFORMATICS_FIGURE_ID).await.unwrap();
        bio.instructions = "replace rubric".into();
        bio.skills = None;
        let list = upsert(&store, bio).await.unwrap();
        let bio = list
            .iter()
            .find(|specialist| specialist.id == R_BIOINFORMATICS_FIGURE_ID)
            .unwrap();
        assert_eq!(bio.instructions, "replace rubric");
        assert_eq!(
            bio.skills,
            Some(vec!["figure-style".into(), "nature-figure".into()])
        );
        assert_eq!(
            bio.default_instructions.as_deref(),
            Some(R_BIOINFORMATICS_FIGURE_RUBRIC),
        );

        let mut extractor = get(&store, HANDWRITING_EXTRACT_ID).await.unwrap();
        extractor.instructions = "replace rubric".into();
        extractor.model_id = "vl".into();
        extractor.skills = None;
        let list = upsert(&store, extractor).await.unwrap();
        let extractor = list
            .iter()
            .find(|specialist| specialist.id == HANDWRITING_EXTRACT_ID)
            .unwrap();
        assert_eq!(extractor.instructions, "replace rubric");
        assert!(extractor.model_id.is_empty());
        assert_eq!(extractor.skills, Some(vec!["handwriting-extract".into()]));
        assert_eq!(
            extractor.default_instructions.as_deref(),
            Some(HANDWRITING_EXTRACT_RUBRIC),
        );

        let mut auditor = get(&store, FIGURE_DUPLICATE_AUDIT_ID).await.unwrap();
        auditor.instructions = "replace rubric".into();
        auditor.skills = None;
        let list = upsert(&store, auditor).await.unwrap();
        let auditor = list
            .iter()
            .find(|specialist| specialist.id == FIGURE_DUPLICATE_AUDIT_ID)
            .unwrap();
        assert_eq!(auditor.instructions, "replace rubric");
        assert_eq!(
            auditor.skills,
            Some(vec![
                "figure-duplicate-audit".into(),
                "local-env-setup".into()
            ])
        );
        assert_eq!(
            auditor.default_instructions.as_deref(),
            Some(FIGURE_DUPLICATE_AUDIT_RUBRIC),
        );

        let mut analyst = get(&store, CONVERSATION_ANALYSIS_ID).await.unwrap();
        analyst.name = "Conversation Analysis".into();
        analyst.description = "replace description".into();
        analyst.instructions = "replace rubric".into();
        analyst.skills = None;
        let list = upsert(&store, analyst).await.unwrap();
        let analyst = list
            .iter()
            .find(|specialist| specialist.id == CONVERSATION_ANALYSIS_ID)
            .unwrap();
        // Display name and description stay pinned to the factory copy.
        assert_eq!(analyst.name, CONVERSATION_ANALYSIS_NAME);
        assert_eq!(analyst.description, CONVERSATION_ANALYSIS_DESCRIPTION);
        // Instructions are user-editable; default rubric is preserved.
        assert_eq!(analyst.instructions, "replace rubric");
        assert_eq!(
            analyst.default_instructions.as_deref(),
            Some(CONVERSATION_ANALYSIS_RUBRIC),
        );
        assert_eq!(analyst.skills, Some(vec![]));
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn specialist_llm_falls_back_to_active_for_empty_or_dangling() {
        let (store, tmp) = test_store().await;
        // No model profiles configured: active resolution still returns the
        // env/default fallback chain from load_settings.
        let spec = Specialist {
            model_id: "no-such".into(),
            review_backend: None,
            ..builtin_reviewer()
        };
        let (provider, api_url, model, _key, _mt, _re, _st) = specialist_llm(&store, &spec).await;
        assert!(!provider.is_empty());
        assert!(!api_url.is_empty());
        assert!(!model.is_empty());
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn session_specialist_set_get_and_lock() {
        let (store, tmp) = test_store().await;
        ensure(&store).await;
        store.create_project("p1", "proj", "").await.unwrap();
        store.create_frame("f1", "p1", "OPERON", "m").await.unwrap();
        set_frame_specialist(&store, "f1", "reviewer")
            .await
            .unwrap();
        assert_eq!(
            session_specialist(&store, "f1").await.unwrap().id,
            "reviewer"
        );
        // Clearing works.
        set_frame_specialist(&store, "f1", "").await.unwrap();
        assert!(session_specialist(&store, "f1").await.is_none());
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn reviewer_model_binding_feeds_review_config() {
        let (store, tmp) = test_store().await;
        let mut r = get(&store, "reviewer").await.unwrap();
        r.model_id = "does-not-exist".into();
        upsert(&store, r).await.unwrap();
        // Dangling binding falls back to the active chain — never errors.
        let spec = get(&store, "reviewer").await.unwrap();
        let (_p, _u, model, _k, _mt, _re, _st) = specialist_llm(&store, &spec).await;
        assert!(!model.is_empty());
        let _ = std::fs::remove_file(&tmp);
    }

    #[tokio::test]
    async fn reviewer_backend_roundtrips_and_keeps_legacy_http_binding() {
        let (store, tmp) = test_store().await;
        let mut reviewer = get(&store, "reviewer").await.unwrap();
        reviewer.review_backend = Some(crate::review::ReviewBackendConfig::AcpAgent {
            profile_id: "acp-1".into(),
        });
        upsert(&store, reviewer).await.unwrap();
        assert_eq!(
            get(&store, "reviewer").await.unwrap().review_backend,
            Some(crate::review::ReviewBackendConfig::AcpAgent {
                profile_id: "acp-1".into()
            })
        );

        let mut reviewer = get(&store, "reviewer").await.unwrap();
        reviewer.review_backend = Some(crate::review::ReviewBackendConfig::HttpModel {
            profile_id: "http-2".into(),
        });
        upsert(&store, reviewer).await.unwrap();
        let reviewer = get(&store, "reviewer").await.unwrap();
        assert_eq!(reviewer.model_id, "http-2");
        assert_eq!(
            reviewer.review_backend,
            Some(crate::review::ReviewBackendConfig::HttpModel {
                profile_id: "http-2".into()
            })
        );
        let _ = std::fs::remove_file(&tmp);
    }
}
