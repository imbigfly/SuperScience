//! Curated capability discovery tiles shared by the projects home and the
//! fullscreen Capabilities overlay.

use crate::app_support::{compose_icon, save_view_pref};
use crate::bindings::{invoke, invoke_checked};
use crate::dto::ModelProfile;
use crate::i18n::{t, Locale};
use crate::knowledge_settings::{probe_knowledge_ready, KnowledgeSettingsOverlay};
use crate::text::{event_target_checked, event_target_value};
use crate::window_capture_escape;
use leptos::*;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CapabilityGroup {
    Topic,
    Design,
    Implement,
    Writing,
    AiDrawing,
    DataCleaning,
    Efficiency,
    Collab,
}

impl CapabilityGroup {
    pub(crate) fn all() -> &'static [CapabilityGroup] {
        &[
            CapabilityGroup::Topic,
            CapabilityGroup::Design,
            CapabilityGroup::Implement,
            CapabilityGroup::Writing,
            CapabilityGroup::AiDrawing,
            CapabilityGroup::DataCleaning,
            CapabilityGroup::Efficiency,
            CapabilityGroup::Collab,
        ]
    }

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            CapabilityGroup::Topic => "caps.group.topic",
            CapabilityGroup::Design => "caps.group.design",
            CapabilityGroup::Implement => "caps.group.implement",
            CapabilityGroup::Writing => "caps.group.writing",
            CapabilityGroup::AiDrawing => "caps.group.ai_drawing",
            CapabilityGroup::DataCleaning => "caps.group.data_cleaning",
            CapabilityGroup::Efficiency => "caps.group.efficiency",
            CapabilityGroup::Collab => "caps.group.collab",
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            CapabilityGroup::Topic => "topic",
            CapabilityGroup::Design => "design",
            CapabilityGroup::Implement => "implement",
            CapabilityGroup::Writing => "writing",
            CapabilityGroup::AiDrawing => "ai_drawing",
            CapabilityGroup::DataCleaning => "data_cleaning",
            CapabilityGroup::Efficiency => "efficiency",
            CapabilityGroup::Collab => "collab",
        }
    }

    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "topic" => Some(CapabilityGroup::Topic),
            "design" => Some(CapabilityGroup::Design),
            "implement" => Some(CapabilityGroup::Implement),
            "writing" | "paper_writing" => Some(CapabilityGroup::Writing),
            "ai_drawing" => Some(CapabilityGroup::AiDrawing),
            "data_cleaning" => Some(CapabilityGroup::DataCleaning),
            "efficiency" => Some(CapabilityGroup::Efficiency),
            "collab" => Some(CapabilityGroup::Collab),
            // Previous tab ids: land on the closest new stage.
            "assets" => Some(CapabilityGroup::Topic),
            "stats" | "structure" => Some(CapabilityGroup::Implement),
            _ => None,
        }
    }
}

const CAPABILITY_SCENE_PREF_KEY: &str = "superscience-capability-scene";

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct PiiCustomTermView {
    original: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    placeholder: Option<String>,
}

fn keyword_from_line(line: &str) -> String {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return String::new();
    }
    line.split('|').next().unwrap_or("").trim().to_string()
}

fn format_pii_keyword_lines(terms: &[PiiCustomTermView]) -> String {
    terms
        .iter()
        .map(|term| term.original.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_pii_token(n: u32) -> String {
    format!("〔词{n}〕")
}

fn parse_pii_keyword_lines(raw: &str) -> Vec<PiiCustomTermView> {
    let mut originals = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for line in raw.lines() {
        let original = keyword_from_line(line);
        if original.is_empty() || !seen.insert(original.clone()) {
            continue;
        }
        originals.push(original);
    }
    let blocked: std::collections::HashSet<String> = originals.iter().cloned().collect();
    let mut next = 0_u32;
    originals
        .into_iter()
        .map(|original| {
            let placeholder = loop {
                next += 1;
                let token = format_pii_token(next);
                if !blocked.contains(&token) {
                    break token;
                }
            };
            PiiCustomTermView {
                original,
                category: "custom".into(),
                placeholder: Some(placeholder),
            }
        })
        .collect()
}

fn preview_pii_placeholders(raw: &str) -> String {
    let map: std::collections::HashMap<_, _> = parse_pii_keyword_lines(raw)
        .into_iter()
        .filter_map(|term| {
            term.placeholder
                .map(|placeholder| (term.original, placeholder))
        })
        .collect();
    raw.lines()
        .map(|line| {
            let original = keyword_from_line(line);
            if original.is_empty() {
                String::new()
            } else {
                map.get(&original).cloned().unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn load_capability_scene_pref() -> Option<CapabilityGroup> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(CAPABILITY_SCENE_PREF_KEY).ok().flatten())
        .and_then(|id| CapabilityGroup::from_id(&id))
}

pub(crate) fn load_capability_scene() -> CapabilityGroup {
    load_capability_scene_pref().unwrap_or(CapabilityGroup::Topic)
}

pub(crate) fn save_capability_scene(group: CapabilityGroup) {
    save_view_pref(CAPABILITY_SCENE_PREF_KEY, group.id());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilityPanel {
    Files,
    Graph,
    Publication,
    /// Kept so the right-tab Agents / Workflow Studio surface can still be opened
    /// programmatically. Home catalog no longer ships a tile for it.
    #[allow(dead_code)]
    Agents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilityAction {
    NewChat,
    /// Start a guided chat. When `skill` is set, the message attaches that Skill
    /// reference and prefixes the shared Socratic coaching frame. When
    /// `specialist` is set, the new session is bound to that specialist before
    /// the first message is sent.
    GuidedChat {
        prompt_key: &'static str,
        skill: Option<&'static str>,
        specialist: Option<&'static str>,
    },
    /// Show a confirm overlay, download a allowlisted skill, then GuidedChat.
    InstallThenGuided {
        prompt_key: &'static str,
        skill: &'static str,
    },
    OpenSettings {
        section: &'static str,
    },
    OpenPanel(CapabilityPanel),
    OpenDemo,
    /// Preview tile: click shows an in-progress notice instead of starting work.
    ComingSoon,
    /// Switch-primary tile: no click-to-activate chat/settings action.
    None,
    /// Open the non-modal local environment setup panel.
    OpenRuntimeSetup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CatalogSkillInstallRequest {
    pub(crate) prompt_key: &'static str,
    pub(crate) skill: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CatalogSkillInstallPhase {
    Confirm,
    Working {
        phase: String,
        received: u64,
        total: Option<u64>,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CatalogSkillInstallState {
    pub(crate) request: CatalogSkillInstallRequest,
    pub(crate) phase: CatalogSkillInstallPhase,
}

impl CatalogSkillInstallState {
    pub(crate) fn confirm(prompt_key: &'static str, skill: &'static str) -> Self {
        Self {
            request: CatalogSkillInstallRequest { prompt_key, skill },
            phase: CatalogSkillInstallPhase::Confirm,
        }
    }

    pub(crate) fn dismissible(&self) -> bool {
        !matches!(self.phase, CatalogSkillInstallPhase::Working { .. })
    }

    pub(crate) fn copy_key(&self, part: &str) -> String {
        format!(
            "caps.install.{}.{}",
            self.request.skill.replace('-', "_"),
            part
        )
    }
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogSkillProgressEvent {
    pub(crate) skill: String,
    pub(crate) received: u64,
    pub(crate) total: Option<u64>,
    pub(crate) phase: String,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogSkillInstallResultView {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) pip_warning: Option<String>,
}

/// Optional on-card toggle (persisted setting). Switch is the primary control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilityToggle {
    PiiFirewall,
}

/// Optional on-card settings entry. Keep per-tile; do not grow a generic framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilityCardSettings {
    HandwritingModels,
    Knowledge,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CapabilityHelp {
    en: &'static str,
    zh: &'static str,
}

impl CapabilityHelp {
    fn get(self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => self.en,
            Locale::Zh => self.zh,
        }
    }
}

const fn help(en: &'static str, zh: &'static str) -> CapabilityHelp {
    CapabilityHelp { en, zh }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CapabilityTile {
    pub(crate) id: &'static str,
    pub(crate) group: CapabilityGroup,
    pub(crate) title_key: &'static str,
    pub(crate) blurb_key: &'static str,
    pub(crate) help: CapabilityHelp,
    pub(crate) icon: &'static str,
    pub(crate) action: CapabilityAction,
    pub(crate) toggle: Option<CapabilityToggle>,
    pub(crate) card_settings: Option<CapabilityCardSettings>,
}

impl CapabilityTile {
    const fn with_settings(mut self, settings: CapabilityCardSettings) -> Self {
        self.card_settings = Some(settings);
        self
    }
}

const fn skill_tile(
    id: &'static str,
    group: CapabilityGroup,
    title_key: &'static str,
    blurb_key: &'static str,
    prompt_key: &'static str,
    skill: &'static str,
    icon: &'static str,
    help: CapabilityHelp,
) -> CapabilityTile {
    CapabilityTile {
        id,
        group,
        title_key,
        blurb_key,
        help,
        icon,
        action: CapabilityAction::GuidedChat {
            prompt_key,
            skill: Some(skill),
            specialist: None,
        },
        toggle: None,
        card_settings: None,
    }
}

const fn specialist_skill_tile(
    id: &'static str,
    group: CapabilityGroup,
    title_key: &'static str,
    blurb_key: &'static str,
    prompt_key: &'static str,
    skill: &'static str,
    specialist: &'static str,
    icon: &'static str,
    help: CapabilityHelp,
) -> CapabilityTile {
    CapabilityTile {
        id,
        group,
        title_key,
        blurb_key,
        help,
        icon,
        action: CapabilityAction::GuidedChat {
            prompt_key,
            skill: Some(skill),
            specialist: Some(specialist),
        },
        toggle: None,
        card_settings: None,
    }
}

const fn specialist_tile(
    id: &'static str,
    group: CapabilityGroup,
    title_key: &'static str,
    blurb_key: &'static str,
    prompt_key: &'static str,
    specialist: &'static str,
    icon: &'static str,
    help: CapabilityHelp,
) -> CapabilityTile {
    CapabilityTile {
        id,
        group,
        title_key,
        blurb_key,
        help,
        icon,
        action: CapabilityAction::GuidedChat {
            prompt_key,
            skill: None,
            specialist: Some(specialist),
        },
        toggle: None,
        card_settings: None,
    }
}

const fn install_then_guided_tile(
    id: &'static str,
    group: CapabilityGroup,
    title_key: &'static str,
    blurb_key: &'static str,
    prompt_key: &'static str,
    skill: &'static str,
    icon: &'static str,
    help: CapabilityHelp,
) -> CapabilityTile {
    CapabilityTile {
        id,
        group,
        title_key,
        blurb_key,
        help,
        icon,
        action: CapabilityAction::InstallThenGuided { prompt_key, skill },
        toggle: None,
        card_settings: None,
    }
}

const fn guided_tile(
    id: &'static str,
    group: CapabilityGroup,
    title_key: &'static str,
    blurb_key: &'static str,
    prompt_key: &'static str,
    icon: &'static str,
    help: CapabilityHelp,
) -> CapabilityTile {
    CapabilityTile {
        id,
        group,
        title_key,
        blurb_key,
        help,
        icon,
        action: CapabilityAction::GuidedChat {
            prompt_key,
            skill: None,
            specialist: None,
        },
        toggle: None,
        card_settings: None,
    }
}

pub(crate) fn capability_catalog() -> &'static [CapabilityTile] {
    static CATALOG: &[CapabilityTile] = &[
        // —— 论文写作与发表；个别卡片按科研阶段归到选题 ——
        skill_tile(
            "academic-paper",
            CapabilityGroup::Writing,
            "caps.skill.academic_paper.title",
            "caps.skill.academic_paper.blurb",
            "caps.skill.academic_paper.prompt",
            "academic-paper",
            "skill",
            help(
                "Give it your topic or a draft. It helps you plan the paper, write sections, revise them, and format the manuscript. Click the card to start a guided chat.",
                "把题目或草稿交给它。它能帮你定提纲、写各节、改稿和排版。点卡片就会开一场带引导的对话。",
            ),
        ),
        skill_tile(
            "academic-pipeline",
            CapabilityGroup::Writing,
            "caps.skill.academic_pipeline.title",
            "caps.skill.academic_pipeline.blurb",
            "caps.skill.academic_pipeline.prompt",
            "academic-pipeline",
            "plan",
            help(
                "Use this when you want the whole path in one place: gather research, write, get a review, then revise. It walks the work end to end instead of stopping at a single draft.",
                "适合从头做到尾：查文献、写稿、审一稿、再改。不是只写一段，而是把整条流水线串起来。",
            ),
        ),
        skill_tile(
            "academic-paper-reviewer",
            CapabilityGroup::Writing,
            "caps.skill.academic_paper_reviewer.title",
            "caps.skill.academic_paper_reviewer.blurb",
            "caps.skill.academic_paper_reviewer.prompt",
            "academic-paper-reviewer",
            "review",
            help(
                "Paste a manuscript. Several reviewer roles read it and tell you what is weak, missing, or overclaimed, like a journal review before you submit.",
                "把稿子丢给它。几个审稿角色会分别挑刺，告诉你哪里虚、哪里缺、哪里说过头，相当于投稿前先挨一遍审。",
            ),
        ),
        skill_tile(
            "deep-research",
            CapabilityGroup::Topic,
            "caps.skill.deep_research.title",
            "caps.skill.deep_research.blurb",
            "caps.skill.deep_research.prompt",
            "deep-research",
            "search",
            help(
                "Tell it a research question. It searches the literature, checks facts, and gives you a cited brief instead of a one-line search dump.",
                "告诉它你要查什么。它会去搜文献、核对事实，给你一份带引用的调研简报，而不是甩一堆链接。",
            ),
        ),
        skill_tile(
            "nature-writing",
            CapabilityGroup::Writing,
            "caps.skill.nature_writing.title",
            "caps.skill.nature_writing.blurb",
            "caps.skill.nature_writing.prompt",
            "nature-writing",
            "edit",
            help(
                "Hand it results or an outline. It drafts or rewrites manuscript sections in a tight, Nature-like style.",
                "把结果或提纲给它。它按顶刊那种紧的写法，起草或重写论文章节。",
            ),
        ),
        skill_tile(
            "nature-polishing",
            CapabilityGroup::Writing,
            "caps.skill.nature_polishing.title",
            "caps.skill.nature_polishing.blurb",
            "caps.skill.nature_polishing.prompt",
            "nature-polishing",
            "edit",
            help(
                "Paste English or Chinese text. It polishes wording and tone so the prose reads closer to a top-journal paper.",
                "把中文或英文段落贴进去。它改措辞和语气，让读起来更像高水平期刊论文。",
            ),
        ),
        skill_tile(
            "humanizer-zh",
            CapabilityGroup::Writing,
            "caps.skill.humanizer_zh.title",
            "caps.skill.humanizer_zh.blurb",
            "caps.skill.humanizer_zh.prompt",
            "humanizer-zh",
            "eye-off",
            help(
                "Paste AI-sounding Chinese or English. It rewrites the obvious machine patterns so the text reads more like a person wrote it.",
                "把明显像 AI 写的文字贴进去。它会改掉那些套话和机械句式，让读起来更像人写的。",
            ),
        ),
        skill_tile(
            "nature-proposal-writer",
            CapabilityGroup::Topic,
            "caps.skill.nature_proposal_writer.title",
            "caps.skill.nature_proposal_writer.blurb",
            "caps.skill.nature_proposal_writer.prompt",
            "nature-proposal-writer",
            "doc",
            help(
                "Give it your idea and any existing notes. It drafts or checks a grant, proposal, or opening report.",
                "把想法和已有材料给它。它能写或检查基金本子、研究计划和开题报告。",
            ),
        ),
        skill_tile(
            "nature-citation",
            CapabilityGroup::Writing,
            "caps.skill.nature_citation.title",
            "caps.skill.nature_citation.blurb",
            "caps.skill.nature_citation.prompt",
            "nature-citation",
            "doc",
            help(
                "Paste a paragraph that needs references. It finds and inserts proper citations instead of leaving empty claims.",
                "把缺参考文献的段落贴进去。它会补上该有的引用，避免空口下结论。",
            ),
        ),
        skill_tile(
            "nature-response",
            CapabilityGroup::Writing,
            "caps.skill.nature_response.title",
            "caps.skill.nature_response.blurb",
            "caps.skill.nature_response.prompt",
            "nature-response",
            "chat",
            help(
                "Give it reviewer comments and your paper. It writes a point-by-point reply letter you can send back to the journal.",
                "把审稿意见和稿子给它。它按条写修回信，方便你直接回给杂志。",
            ),
        ),
        skill_tile(
            "nature-reviewer",
            CapabilityGroup::Writing,
            "caps.skill.nature_reviewer.title",
            "caps.skill.nature_reviewer.blurb",
            "caps.skill.nature_reviewer.prompt",
            "nature-reviewer",
            "review",
            help(
                "Upload a manuscript before submission. It writes a referee-style report so you can fix problems first.",
                "投稿前把稿子给它。它按审稿人的口吻写一份意见，让你先改再投。",
            ),
        ),
        skill_tile(
            "nature-data",
            CapabilityGroup::Writing,
            "caps.skill.nature_data.title",
            "caps.skill.nature_data.blurb",
            "caps.skill.nature_data.prompt",
            "nature-data",
            "folder",
            help(
                "Tell it what data you have and where it will live. It drafts the Data Availability statement and a FAIR-style sharing plan.",
                "告诉它你有哪些数据、打算放哪。它帮你写数据可用性声明，以及怎么按规范共享。",
            ),
        ),
        skill_tile(
            "nature-ref-verifier",
            CapabilityGroup::Writing,
            "caps.skill.nature_ref_verifier.title",
            "caps.skill.nature_ref_verifier.blurb",
            "caps.skill.nature_ref_verifier.prompt",
            "nature-ref-verifier",
            "search",
            help(
                "Give it your reference list. It checks titles, years, authors, and DOIs, and flags entries that do not match.",
                "把参考文献列表给它。它核对题名、年份、作者和 DOI，对不上的会标出来。",
            ),
        ),
        skill_tile(
            "nature-paper-to-patent",
            CapabilityGroup::Writing,
            "caps.skill.nature_paper_to_patent.title",
            "caps.skill.nature_paper_to_patent.blurb",
            "caps.skill.nature_paper_to_patent.prompt",
            "nature-paper-to-patent",
            "doc",
            help(
                "Give it a paper or lab notes. It turns the inventive parts into a Chinese invention-patent draft you can take to an attorney.",
                "把论文或实验笔记给它。它抽出能申请的发明点，写成中国发明专利初稿，方便再交给代理。",
            ),
        ),
        skill_tile(
            "officecli",
            CapabilityGroup::Writing,
            "caps.tile.officecli.title",
            "caps.tile.officecli.blurb",
            "caps.prompt.officecli",
            "officecli",
            "edit",
            help(
                "Ask for a Word, Excel, or PowerPoint file. It creates or edits a real Office document, not just a screenshot of one.",
                "直接跟它说要 Word、Excel 还是 PPT。它生成或改的是真正的 Office 文件，不是一张截图。",
            ),
        ),
        // —— AI画图 ——
        skill_tile(
            "nature-figure",
            CapabilityGroup::AiDrawing,
            "caps.skill.nature_figure.title",
            "caps.skill.nature_figure.blurb",
            "caps.skill.nature_figure.prompt",
            "nature-figure",
            "image",
            help(
                "Give it a table or plot idea. It draws or checks a publication-style figure in Python or R.",
                "把表格或作图想法给它。它用 Python 或 R 画、或检查一张能投稿的图。",
            ),
        ),
        specialist_skill_tile(
            "r-bioinfo-figure",
            CapabilityGroup::AiDrawing,
            "caps.tile.r_bioinfo_figure.title",
            "caps.tile.r_bioinfo_figure.blurb",
            "caps.prompt.r_bioinfo_figure",
            "nature-figure",
            "r_bioinformatics_figure",
            "image",
            help(
                "Give it expression or clinical data. The R specialist makes common bioinformatics plots—volcano, heatmap, PCA, survival, and similar.",
                "把表达量或临床数据给它。生信图专家用 R 画常见图：火山图、热图、PCA、生存曲线等。",
            ),
        ),
        guided_tile(
            "ai-drawing",
            CapabilityGroup::AiDrawing,
            "caps.tile.ai_drawing.title",
            "caps.tile.ai_drawing.blurb",
            "caps.prompt.ai_drawing",
            "image",
            help(
                "Describe the illustration you want. It asks the configured image model to draw a research figure, then you can ask for changes.",
                "用话说清要画什么。它调用已配置的生图模型画科研示意，你还可以让它改。",
            ),
        ),
        // Placeholder cards: visible in AI drawing, click wired later.
        CapabilityTile {
            id: "ai-mechanism-figure",
            group: CapabilityGroup::AiDrawing,
            title_key: "caps.tile.ai_mechanism_figure.title",
            blurb_key: "caps.tile.ai_mechanism_figure.blurb",
            help: help(
                "Describe a biological or chemical mechanism. It draws a journal-style mechanism figure. This card is a preview; click-to-run is not wired yet.",
                "描述一个机制过程，它按顶刊风格画机制图。这张卡还是预告，点下去暂时不会开工。",
            ),
            icon: "image",
            action: CapabilityAction::ComingSoon,
            toggle: None,
            card_settings: None,
        },
        CapabilityTile {
            id: "editable-figure",
            group: CapabilityGroup::AiDrawing,
            title_key: "caps.tile.editable_figure.title",
            blurb_key: "caps.tile.editable_figure.blurb",
            help: help(
                "Give it a mechanism figure. It aims to turn that picture into something you can still edit. This card is a preview; click-to-run is not wired yet.",
                "把机制图给它，目标是变成还能改的格式。这张卡还是预告，点下去暂时不会开工。",
            ),
            icon: "edit",
            action: CapabilityAction::ComingSoon,
            toggle: None,
            card_settings: None,
        },
        CapabilityTile {
            id: "bioinfo-figure-layout",
            group: CapabilityGroup::AiDrawing,
            title_key: "caps.tile.bioinfo_figure_layout.title",
            blurb_key: "caps.tile.bioinfo_figure_layout.blurb",
            help: help(
                "Drop in several bioinformatics plots. It arranges them into one submission-ready panel. This card is a preview; click-to-run is not wired yet.",
                "把好几张生信图丢给它，它排成一张能投稿的拼图。这张卡还是预告，点下去暂时不会开工。",
            ),
            icon: "grid",
            action: CapabilityAction::ComingSoon,
            toggle: None,
            card_settings: None,
        },
        skill_tile(
            "nature-paper2ppt",
            CapabilityGroup::AiDrawing,
            "caps.skill.nature_paper2ppt.title",
            "caps.skill.nature_paper2ppt.blurb",
            "caps.skill.nature_paper2ppt.prompt",
            "nature-paper2ppt",
            "skill",
            help(
                "Give it a paper. It builds a Chinese journal-club PowerPoint that follows the paper's argument, not a wall of screenshots.",
                "把论文给它。它做成中文组会 PPT，顺着论文论点讲，不是把页面截图糊上去。",
            ),
        ),
        install_then_guided_tile(
            "ppt-master",
            CapabilityGroup::AiDrawing,
            "caps.skill.ppt_master.title",
            "caps.skill.ppt_master.blurb",
            "caps.skill.ppt_master.prompt",
            "ppt-master",
            "slides",
            help(
                "Give it a topic or a document. It builds a real, editable PowerPoint with shapes, masters, and charts. This skill is downloaded on first use.",
                "给一个题目或一份材料。它做成能改的原生 PPT（形状、母版、图表）。第一次用会先下载这个技能。",
            ),
        ),
        // —— 数据清洗 ——
        specialist_tile(
            "data-cleaning",
            CapabilityGroup::DataCleaning,
            "caps.tile.data_cleaning.title",
            "caps.tile.data_cleaning.blurb",
            "caps.prompt.data_cleaning",
            "data_cleaning",
            "plan",
            help(
                "Send a spreadsheet or a file path. It looks at the table, cleans messy columns, and gives you a file ready for analysis.",
                "把表格或文件路径发给它。它先看表，再清洗乱列，最后给你一份能直接分析的数据。",
            ),
        ),
        CapabilityTile {
            id: "pii-firewall",
            group: CapabilityGroup::Efficiency,
            title_key: "caps.tile.pii_firewall.title",
            blurb_key: "caps.tile.pii_firewall.blurb",
            help: help(
                "When this switch is on, emails, phones, IDs, and your custom keywords are hidden before a cloud model sees your chat, then put back in the reply you read. The card itself opens the dictionary editor.",
                "开关打开时，发往云端模型前会先藏起邮箱、手机号、身份证号和你自定义的关键词，你看到的回复再还原回来。点卡片会打开词表编辑。",
            ),
            icon: "shield",
            action: CapabilityAction::None,
            toggle: Some(CapabilityToggle::PiiFirewall),
            card_settings: None,
        },
        skill_tile(
            "journal-prescreen",
            CapabilityGroup::Efficiency,
            "caps.skill.journal_prescreen.title",
            "caps.skill.journal_prescreen.blurb",
            "caps.skill.journal_prescreen.prompt",
            "journal-prescreen",
            "review",
            help(
                "Give it a manuscript and a target journal. It checks the author guidelines and marks format, ethics, and length problems with fix suggestions.",
                "把稿子和目标杂志给它。它对照投稿须知，标出格式、伦理、字数等问题，并给出改法。",
            ),
        ),
        specialist_skill_tile(
            "handwriting-extract",
            CapabilityGroup::Efficiency,
            "caps.skill.handwriting_extract.title",
            "caps.skill.handwriting_extract.blurb",
            "caps.skill.handwriting_extract.prompt",
            "handwriting-extract",
            "handwriting_extract",
            "grid",
            help(
                "Upload photos of handwritten lab notes or CRF pages. The handwriting expert reads them with the analysis model, calibrates flagged cells with the calibration model, and marks uncertain spots.",
                "上传手写实验本或 CRF 照片。手写提取专家用图片分析模型识图，再用校准模型核对存疑格子，并标出位置。",
            ),
        )
        .with_settings(CapabilityCardSettings::HandwritingModels),
        skill_tile(
            "topic-coach",
            CapabilityGroup::Efficiency,
            "caps.skill.topic_coach.title",
            "caps.skill.topic_coach.blurb",
            "caps.skill.topic_coach.prompt",
            "topic-coach",
            "plan",
            help(
                "Walk through the data and materials you already have, then score a few journal-fit topic candidates and the next action.",
                "先盘点你已有的数据和资料，再给出几个对准目标期刊的选题候选，并评一下下一步该做什么。",
            ),
        ),
        guided_tile(
            "knowledge",
            CapabilityGroup::Efficiency,
            "caps.tile.knowledge.title",
            "caps.tile.knowledge.blurb",
            "caps.prompt.knowledge",
            "book",
            help(
                "Search your local WeKnora knowledge base and cite excerpts. Use the gear if the connection is not ready.",
                "检索本机 WeKnora 知识库并引用片段。连不上时点右上角齿轮先配好接口。",
            ),
        )
        .with_settings(CapabilityCardSettings::Knowledge),
        CapabilityTile {
            id: "env-setup",
            group: CapabilityGroup::DataCleaning,
            title_key: "caps.tile.env_setup.title",
            blurb_key: "caps.tile.env_setup.blurb",
            help: help(
                "It checks this computer for Python, R, Node, sci, pixi, and officecli, then installs what is missing in the background.",
                "检查这台电脑有没有 Python、R、Node、sci、pixi、officecli，缺的在后台装好。",
            ),
            icon: "gear",
            action: CapabilityAction::OpenRuntimeSetup,
            toggle: None,
            card_settings: None,
        },
        // —— 研究实施与数据分析；知识图谱 / 生物库归选题 ——
        skill_tile(
            "nature-statistics",
            CapabilityGroup::Implement,
            "caps.skill.nature_statistics.title",
            "caps.skill.nature_statistics.blurb",
            "caps.skill.nature_statistics.prompt",
            "nature-statistics",
            "grid",
            help(
                "Paste methods or results. It checks p-values, confidence intervals, and wording so the stats section matches journal habits.",
                "把方法或结果文字贴进去。它检查 p 值、置信区间和表述，避免统计写法不合格。",
            ),
        ),
        skill_tile(
            "knowledge-graph",
            CapabilityGroup::Topic,
            "caps.skill.knowledge_graph.title",
            "caps.skill.knowledge_graph.blurb",
            "caps.skill.knowledge_graph.prompt",
            "knowledge-graph",
            "share",
            help(
                "Give it a paper or notes. It pulls out who-does-what-to-whom and shows that as a graph you can search and drag.",
                "把论文或笔记给它。它抽出谁对谁做了什么，画成一张能搜索、能拖动的关系图。",
            ),
        ),
        guided_tile(
            "stats-analysis",
            CapabilityGroup::Implement,
            "caps.tile.stats_analysis.title",
            "caps.tile.stats_analysis.blurb",
            "caps.prompt.stats_analysis",
            "grid",
            help(
                "Tell it the question and point to the table. It runs tests, regression, or exploratory plots in Python or R and explains the result.",
                "说清问题和数据在哪。它用 Python 或 R 做检验、回归或探索图，并解释结果是什么意思。",
            ),
        ),
        guided_tile(
            "python-r",
            CapabilityGroup::Implement,
            "caps.tile.python_r.title",
            "caps.tile.python_r.blurb",
            "caps.prompt.python_r",
            "plan",
            help(
                "Ask it to analyze a table in a live Python or R session. Variables stay loaded, so you can keep asking follow-ups and get editable SVG plots.",
                "让它在活的 Python 或 R 会话里分析表格。变量会留着，你可以接着问，图是能改的 SVG。",
            ),
        ),
        guided_tile(
            "bio-db",
            CapabilityGroup::Topic,
            "caps.tile.bio_db.title",
            "caps.tile.bio_db.blurb",
            "caps.prompt.bio_db",
            "search",
            help(
                "Name a gene, dataset, or paper. It looks it up in PubMed, GEO, UniProt, PDB, and similar biology databases.",
                "说出基因、数据集或文献。它去 PubMed、GEO、UniProt、PDB 这类库里查。",
            ),
        ),
        CapabilityTile {
            id: "remote-compute",
            group: CapabilityGroup::Implement,
            title_key: "caps.tile.remote_compute.title",
            blurb_key: "caps.tile.remote_compute.blurb",
            help: help(
                "Use this to add an SSH or WSL machine, start a long run, or open a remote terminal. It opens Environment settings.",
                "用来加 SSH/WSL 机器、开长任务，或打开远程终端。点下去会进环境设置。",
            ),
            icon: "server",
            action: CapabilityAction::OpenSettings {
                section: "environments",
            },
            toggle: None,
            card_settings: None,
        },
        // —— 结构预测 / 单细胞（实施）——
        guided_tile(
            "structure",
            CapabilityGroup::Implement,
            "caps.tile.structure.title",
            "caps.tile.structure.blurb",
            "caps.prompt.structure",
            "image",
            help(
                "Give it a sequence or structure task. It can fold proteins, dock ligands, or design sequences with tools like AlphaFold, Boltz, DiffDock, and MPNN.",
                "给它序列或结构任务。它可以折叠蛋白、对接分子、设计序列，常用 AlphaFold、Boltz、DiffDock、MPNN 这类工具。",
            ),
        ),
        guided_tile(
            "single-cell",
            CapabilityGroup::Implement,
            "caps.tile.single_cell.title",
            "caps.tile.single_cell.blurb",
            "caps.prompt.single_cell",
            "grid",
            help(
                "Point it at single-cell data. It helps run scvi/scGPT-style analysis and keep the steps reproducible.",
                "把单细胞数据指给它。它帮你跑 scvi/scGPT 一类分析，并把步骤留下，方便复现。",
            ),
        ),
        // —— 文献检索与阅读（选题）；实验日志归实施 ——
        skill_tile(
            "academic-search-pro",
            CapabilityGroup::Topic,
            "caps.skill.academic_search_pro.title",
            "caps.skill.academic_search_pro.blurb",
            "caps.skill.academic_search_pro.prompt",
            "academic-search-pro",
            "book",
            help(
                "Give keywords, plus year or language if you have them. It searches free libraries such as OpenAlex, PubMed, arXiv, and Crossref, uses lawful web search for Chinese sources, then dedupes and returns a literature matrix or BibTeX. Metadata only: no full-text reading, no citation checks or other-citation audits.",
                "给关键词，可加年份或语种。同时查 OpenAlex、PubMed、arXiv、Crossref 等免费库，中文走合规网页检索；跨库去重后交出文献矩阵或 BibTeX。只给题录，不读全文，也不核引用、不做他引。",
            ),
        ),
        skill_tile(
            "nature-academic-search",
            CapabilityGroup::Topic,
            "caps.skill.nature_academic_search.title",
            "caps.skill.nature_academic_search.blurb",
            "caps.skill.nature_academic_search.prompt",
            "nature-academic-search",
            "search",
            help(
                "Give a topic, a DOI, or a paper to check. Besides searching sources, it can verify references, build a MeSH strategy, count strict other-citations, and show who cited the paper—including high-profile citers. Use this for citation relationships and audits, not a deduped literature table.",
                "给主题、DOI，或一篇要核查的文献。除了多来源检索，还能核参考文献、做 MeSH 策略、查严格他引，以及谁引用了这篇、有没有高影响引用者。要的是引用关系和核验，不是一张去重文献表。",
            ),
        ),
        skill_tile(
            "nature-literature-pipeline",
            CapabilityGroup::Topic,
            "caps.skill.nature_literature_pipeline.title",
            "caps.skill.nature_literature_pipeline.blurb",
            "caps.skill.nature_literature_pipeline.prompt",
            "nature-literature-pipeline",
            "plan",
            help(
                "Give it a field and what “good enough” means. It searches, scores, deep-reads, and files the papers so you get a package, not a pile of links.",
                "告诉它领域和怎样算够用。它会检索、打分、精读并归档，给你一套材料，而不是一堆链接。",
            ),
        ),
        skill_tile(
            "nature-downloader",
            CapabilityGroup::Topic,
            "caps.skill.nature_downloader.title",
            "caps.skill.nature_downloader.blurb",
            "caps.skill.nature_downloader.prompt",
            "nature-downloader",
            "folder",
            help(
                "Give it DOIs or titles. It tries legal routes (open access, your institution, publisher APIs) to fetch the PDF.",
                "给 DOI 或题名。它走合法途径（开放获取、机构权限、出版商接口）去下 PDF。",
            ),
        ),
        skill_tile(
            "nature-paper-card",
            CapabilityGroup::Topic,
            "caps.skill.nature_paper_card.title",
            "caps.skill.nature_paper_card.blurb",
            "caps.skill.nature_paper_card.prompt",
            "nature-paper-card",
            "doc",
            help(
                "Give it one paper, PDF, or DOI. It makes a structured reading card: claims, methods, figures, and limits, tied back to the source.",
                "给一篇论文、PDF 或 DOI。它做成一张深读卡片：主张、方法、图、局限，都能对回原文。",
            ),
        ),
        skill_tile(
            "nature-reader",
            CapabilityGroup::Topic,
            "caps.skill.nature_reader.title",
            "caps.skill.nature_reader.blurb",
            "caps.skill.nature_reader.prompt",
            "nature-reader",
            "doc",
            help(
                "Give it a PDF or DOI. It writes one Chinese–English HTML report you can open: original and translation side by side, figures near the first mention, and clickable source anchors. It does not paste the full paper into chat.",
                "给一份 PDF 或 DOI。它写成一份可打开的中英对照 HTML 报告：原文和译文左右对照，图放在首次提到的附近，来源锚点可点。不会把全文贴进对话。",
            ),
        ),
        skill_tile(
            "nature-experiment-log",
            CapabilityGroup::Implement,
            "caps.skill.nature_experiment_log.title",
            "caps.skill.nature_experiment_log.blurb",
            "caps.skill.nature_experiment_log.prompt",
            "nature-experiment-log",
            "edit",
            help(
                "Send photos, voice notes, or scribbles from the bench. It turns them into a structured Markdown experiment log.",
                "把实验台上的照片、语音或随手记录发来。它整理成结构清楚的 Markdown 实验日志。",
            ),
        ),
        // Workspace surfaces sit on a footer row within their stage tab.
        CapabilityTile {
            id: "research-graph",
            group: CapabilityGroup::Design,
            title_key: "caps.tile.research_graph.title",
            blurb_key: "caps.tile.research_graph.blurb",
            help: help(
                "Opens a map of this project's data, papers, outputs, and decisions, so you can see why a choice was made.",
                "打开这个项目的关系图：数据、文献、产物和决策都在上面，方便回看当时为什么这么做。",
            ),
            icon: "branch",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Graph),
            toggle: None,
            card_settings: None,
        },
        CapabilityTile {
            id: "publication",
            group: CapabilityGroup::Writing,
            title_key: "caps.tile.publication.title",
            blurb_key: "caps.tile.publication.blurb",
            help: help(
                "Opens the publication workspace. Pick the exact evidence for one manuscript version, freeze it, and pack it for checking later.",
                "打开论文证据工作区。给某一稿某一版挑出要用的证据、冻住，以后还能核对。",
            ),
            icon: "doc",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Publication),
            toggle: None,
            card_settings: None,
        },
        CapabilityTile {
            id: "files-library",
            group: CapabilityGroup::Design,
            title_key: "caps.tile.files_library.title",
            blurb_key: "caps.tile.files_library.blurb",
            help: help(
                "Opens project files and the library of saved cells or figures, so you can reopen work without digging through chat.",
                "打开项目文件和收藏库，把以前存下的代码或图再打开，不用翻聊天记录。",
            ),
            icon: "folder",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Files),
            toggle: None,
            card_settings: None,
        },
        CapabilityTile {
            id: "demo",
            group: CapabilityGroup::Design,
            title_key: "caps.tile.demo.title",
            blurb_key: "caps.tile.demo.blurb",
            help: help(
                "Opens the Example project. You can replay bundled research demos without sending a new request or needing an API key.",
                "打开示例项目。可以回看系统自带的研究演示，不用发新请求，也不用 API Key。",
            ),
            icon: "star",
            action: CapabilityAction::OpenDemo,
            toggle: None,
            card_settings: None,
        },
        // —— 协作扩展 ——
        CapabilityTile {
            id: "channels",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.channels.title",
            blurb_key: "caps.tile.channels.blurb",
            help: help(
                "Opens channel settings. After Feishu or WeChat is connected, you can drive the same desktop session from those apps.",
                "打开通道设置。接上飞书或微信后，可以用这些软件驱动同一个桌面会话。",
            ),
            icon: "chat",
            action: CapabilityAction::OpenSettings {
                section: "channels",
            },
            toggle: None,
            card_settings: None,
        },
        guided_tile(
            "browser",
            CapabilityGroup::Collab,
            "caps.tile.browser.title",
            "caps.tile.browser.blurb",
            "caps.prompt.browser",
            "computer",
            help(
                "Starts a chat that can control your real local Chrome through the companion extension—useful for sites that need a logged-in browser.",
                "开一场能操作你本机 Chrome 的对话（需要配套扩展），适合要登录才能用的网站。",
            ),
        ),
        skill_tile(
            "playwright",
            CapabilityGroup::Collab,
            "caps.tile.playwright.title",
            "caps.tile.playwright.blurb",
            "caps.prompt.playwright",
            "playwright",
            "terminal",
            help(
                "Starts a chat that scripts a headless browser (Chromium, Firefox, or WebKit) for tests, screenshots, or scraping.",
                "开一场用脚本驱动无头浏览器的对话，适合做网页测试、截图或抓取。",
            ),
        ),
        CapabilityTile {
            id: "plugins",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.plugins.title",
            blurb_key: "caps.tile.plugins.blurb",
            help: help(
                "Opens plugin settings. You can install extra Skill + MCP packs that add tools this workbench does not ship by default.",
                "打开插件设置。可以安装额外的 Skill + MCP 包，给工作台加它默认没有的工具。",
            ),
            icon: "plus",
            action: CapabilityAction::OpenSettings { section: "plugins" },
            toggle: None,
            card_settings: None,
        },
    ];
    CATALOG
}

pub(crate) fn tiles_for_group(group: CapabilityGroup) -> Vec<&'static CapabilityTile> {
    capability_catalog()
        .iter()
        .filter(|tile| tile.group == group)
        .collect()
}

/// Project workspace surfaces (graph / evidence / files / demo). These sit on a
/// separate row under literature skills so a 5-column grid does not mix them in.
pub(crate) fn is_workspace_surface_tile(tile: &CapabilityTile) -> bool {
    matches!(
        tile.action,
        CapabilityAction::OpenPanel(
            CapabilityPanel::Graph | CapabilityPanel::Publication | CapabilityPanel::Files
        ) | CapabilityAction::OpenDemo
    )
}

fn tiles_for_group_main(group: CapabilityGroup) -> Vec<&'static CapabilityTile> {
    let tiles = tiles_for_group(group);
    let main: Vec<_> = tiles
        .iter()
        .copied()
        .filter(|tile| !is_workspace_surface_tile(tile))
        .collect();
    if main.is_empty() {
        tiles
    } else {
        main
    }
}

fn tiles_for_group_footer(group: CapabilityGroup) -> Vec<&'static CapabilityTile> {
    let tiles = tiles_for_group(group);
    let has_main = tiles.iter().any(|tile| !is_workspace_surface_tile(tile));
    let has_footer = tiles.iter().any(|tile| is_workspace_surface_tile(tile));
    if !has_main || !has_footer {
        return Vec::new();
    }
    tiles
        .into_iter()
        .filter(|tile| is_workspace_surface_tile(tile))
        .collect()
}

fn capability_help_button(
    tile: &'static CapabilityTile,
    locale: RwSignal<Locale>,
    on_help: Callback<&'static CapabilityTile>,
) -> impl IntoView {
    let id = tile.id;
    view! {
        <button type="button" class="cap-tile-help"
            data-testid=format!("cap-tile-help-{id}")
            aria-label=move || t(locale.get(), "caps.help.aria")
            title=move || t(locale.get(), "caps.help.aria")
            on:click=move |ev| {
                ev.prevent_default();
                ev.stop_propagation();
                on_help.call(tile);
            }>
            {compose_icon("circle-alert")}
        </button>
    }
}

fn capability_settings_aria_key(tile: &CapabilityTile) -> &'static str {
    match tile.card_settings {
        Some(CapabilityCardSettings::Knowledge) => "caps.tile.knowledge.settings.aria",
        _ => "caps.skill.handwriting_extract.settings.aria",
    }
}

fn capability_settings_button(
    tile: &'static CapabilityTile,
    locale: RwSignal<Locale>,
    on_settings: Callback<&'static CapabilityTile>,
) -> impl IntoView {
    let id = tile.id;
    let aria_key = capability_settings_aria_key(tile);
    view! {
        <button type="button" class="cap-tile-settings"
            data-testid=format!("cap-tile-settings-{id}")
            aria-label=move || t(locale.get(), aria_key)
            title=move || t(locale.get(), aria_key)
            on:click=move |ev| {
                ev.prevent_default();
                ev.stop_propagation();
                on_settings.call(tile);
            }>
            {compose_icon("gear")}
        </button>
    }
}

fn vision_chat_models(models: &[ModelProfile]) -> Vec<ModelProfile> {
    models
        .iter()
        .filter(|model| model.supports_vision && model.is_chat_model())
        .cloned()
        .collect()
}

fn handwriting_saved_model_id(candidates: &[ModelProfile], id: &str) -> String {
    candidates
        .iter()
        .find(|model| model.id == id)
        .map(|model| model.id.clone())
        .unwrap_or_default()
}

fn handwriting_models_ready(vision_id: &str, calib_id: &str) -> bool {
    !vision_id.trim().is_empty() && !calib_id.trim().is_empty()
}

fn remember_handwriting_picks(
    models: RwSignal<Vec<ModelProfile>>,
    vision_pick: RwSignal<String>,
    calib_pick: RwSignal<String>,
    candidates: Vec<ModelProfile>,
    vision: String,
    calib: String,
) {
    models.set(candidates);
    vision_pick.set(vision);
    calib_pick.set(calib);
}

async fn load_handwriting_model_picks() -> (Vec<ModelProfile>, String, String) {
    let models_v = invoke("list_models", JsValue::UNDEFINED).await;
    let models = serde_wasm_bindgen::from_value::<Vec<ModelProfile>>(models_v).unwrap_or_default();
    let candidates = vision_chat_models(&models);
    let vision_v = invoke("get_handwriting_extract_vision_model", JsValue::UNDEFINED).await;
    let vision = serde_wasm_bindgen::from_value::<Option<String>>(vision_v)
        .ok()
        .flatten()
        .unwrap_or_default();
    let calib_v = invoke(
        "get_handwriting_extract_calibration_model",
        JsValue::UNDEFINED,
    )
    .await;
    let calib = serde_wasm_bindgen::from_value::<Option<String>>(calib_v)
        .ok()
        .flatten()
        .unwrap_or_default();
    let vision_id = handwriting_saved_model_id(&candidates, &vision);
    let calib_id = handwriting_saved_model_id(&candidates, &calib);
    (candidates, vision_id, calib_id)
}

fn capability_action_tile(
    tile: &'static CapabilityTile,
    locale: RwSignal<Locale>,
    on_activate: Callback<CapabilityAction>,
    on_help: Callback<&'static CapabilityTile>,
    on_settings: Option<Callback<&'static CapabilityTile>>,
) -> impl IntoView {
    let action = tile.action;
    let icon = tile.icon;
    let title_key = tile.title_key;
    let blurb_key = tile.blurb_key;
    let id = tile.id;
    let show_settings = tile.card_settings.is_some();
    view! {
        <div class="cap-tile-wrap" role="listitem">
            {on_settings.filter(|_| show_settings).map(|on_settings| {
                view! { {capability_settings_button(tile, locale, on_settings)} }.into_view()
            })}
            {capability_help_button(tile, locale, on_help)}
            <button type="button" class="cap-tile"
                data-testid=format!("cap-tile-{id}")
                on:click=move |_| on_activate.call(action)>
                <span class="cap-tile-icon" aria-hidden="true">{compose_icon(icon)}</span>
                <span class="cap-tile-title">{move || t(locale.get(), title_key)}</span>
                <span class="cap-tile-blurb">{move || t(locale.get(), blurb_key)}</span>
            </button>
        </div>
    }
}

pub(crate) fn env_setup_capability_action() -> CapabilityAction {
    capability_catalog()
        .iter()
        .find(|tile| tile.id == "env-setup")
        .map(|tile| tile.action)
        .expect("env-setup tile")
}

/// Localized title key for a capability-launched session (sidebar + first turn).
pub(crate) fn title_key_for_capability_action(action: &CapabilityAction) -> Option<&'static str> {
    match action {
        CapabilityAction::GuidedChat {
            prompt_key: "caps.prompt.director_kickoff",
            ..
        } => Some("caps.tile.director.title"),
        CapabilityAction::GuidedChat {
            prompt_key,
            skill,
            specialist,
        } => capability_catalog()
            .iter()
            .find(|tile| match tile.action {
                CapabilityAction::GuidedChat {
                    prompt_key: tile_prompt,
                    skill: tile_skill,
                    specialist: tile_specialist,
                } => {
                    tile_prompt == *prompt_key
                        && tile_skill == *skill
                        && tile_specialist == *specialist
                }
                CapabilityAction::InstallThenGuided {
                    prompt_key: tile_prompt,
                    skill: tile_skill,
                } => {
                    tile_prompt == *prompt_key && Some(tile_skill) == *skill && specialist.is_none()
                }
                _ => false,
            })
            .map(|tile| tile.title_key),
        CapabilityAction::InstallThenGuided { .. }
        | CapabilityAction::NewChat
        | CapabilityAction::OpenRuntimeSetup => capability_catalog()
            .iter()
            .find(|tile| tile.action == *action)
            .map(|tile| tile.title_key),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_and_settings_share_the_same_catalog() {
        // Homepage and CapabilitiesOverlay both embed CapabilitySceneTabs,
        // which reads this single catalog. Do not add a second tile list.
        assert_eq!(
            CapabilityGroup::all().len(),
            capability_catalog()
                .iter()
                .map(|tile| tile.group)
                .collect::<std::collections::HashSet<_>>()
                .len()
        );
    }

    #[test]
    fn every_catalog_tile_has_plain_help_copy() {
        for tile in capability_catalog() {
            for locale in [Locale::En, Locale::Zh] {
                let text = tile.help.get(locale);
                assert!(
                    text.chars().count() >= 12,
                    "help copy too short for {} / {}: {text}",
                    tile.id,
                    locale.code()
                );
            }
        }
    }

    #[test]
    fn catalog_openers_do_not_repeat_interview_boilerplate() {
        const BANNED: &[&str] = &[
            "禁止无休止追问",
            "do not interview endlessly",
            "总共最多五个问题",
            "At most five questions",
        ];
        for tile in capability_catalog() {
            let prompt_key = match tile.action {
                CapabilityAction::GuidedChat { prompt_key, .. }
                | CapabilityAction::InstallThenGuided { prompt_key, .. } => prompt_key,
                _ => continue,
            };
            for locale in [Locale::En, Locale::Zh] {
                let text = t(locale, prompt_key);
                assert_ne!(
                    text,
                    prompt_key,
                    "missing opener {} for {}",
                    locale.code(),
                    tile.id
                );
                for phrase in BANNED {
                    assert!(
                        !text.contains(phrase),
                        "{} opener for {} still has {phrase:?}: {text}",
                        locale.code(),
                        tile.id
                    );
                }
            }
        }
    }

    #[test]
    fn literature_search_help_distinguishes_matrix_from_citation_audit() {
        let matrix = capability_catalog()
            .iter()
            .find(|tile| tile.id == "academic-search-pro")
            .expect("academic-search-pro tile");
        let audit = capability_catalog()
            .iter()
            .find(|tile| tile.id == "nature-academic-search")
            .expect("nature-academic-search tile");

        let zh_matrix = matrix.help.get(Locale::Zh);
        let zh_audit = audit.help.get(Locale::Zh);
        assert!(
            zh_matrix.contains("文献矩阵") && zh_matrix.contains("BibTeX"),
            "matrix help should name the table/BibTeX deliverable: {zh_matrix}"
        );
        assert!(
            zh_matrix.contains("不做他引") && zh_matrix.contains("不核引用"),
            "matrix help should say it does not audit citations: {zh_matrix}"
        );
        assert!(
            zh_audit.contains("他引") && zh_audit.contains("高影响引用者"),
            "audit help should name citation-audit work: {zh_audit}"
        );
        assert!(
            zh_audit.contains("不是一张去重文献表"),
            "audit help should contrast with the matrix card: {zh_audit}"
        );

        let en_matrix = matrix.help.get(Locale::En);
        let en_audit = audit.help.get(Locale::En);
        assert!(
            en_matrix.contains("literature matrix") && en_matrix.contains("BibTeX"),
            "matrix help should name the table/BibTeX deliverable: {en_matrix}"
        );
        assert!(
            en_matrix.contains("no citation checks"),
            "matrix help should say it does not audit citations: {en_matrix}"
        );
        assert!(
            en_audit.contains("other-citations") && en_audit.contains("high-profile citers"),
            "audit help should name citation-audit work: {en_audit}"
        );
        assert!(
            en_audit.contains("not a deduped literature table"),
            "audit help should contrast with the matrix card: {en_audit}"
        );
    }

    #[test]
    fn nature_reader_copy_names_html_report() {
        let tile = capability_catalog()
            .iter()
            .find(|tile| tile.id == "nature-reader")
            .expect("nature-reader tile");

        let zh_help = tile.help.get(Locale::Zh);
        let en_help = tile.help.get(Locale::En);
        assert!(
            zh_help.contains("HTML 报告") && zh_help.contains("左右对照"),
            "zh help should name the openable HTML report: {zh_help}"
        );
        assert!(
            !zh_help.contains("阅读稿") && !zh_help.contains("Markdown"),
            "zh help should not call the deliverable a Markdown draft: {zh_help}"
        );
        assert!(
            en_help.contains("HTML report") && en_help.contains("side by side"),
            "en help should name the openable HTML report: {en_help}"
        );
        assert!(
            !en_help.contains("reading draft") && !en_help.contains("Markdown"),
            "en help should not call the deliverable a Markdown draft: {en_help}"
        );

        let zh_blurb = t(Locale::Zh, tile.blurb_key);
        let en_blurb = t(Locale::En, tile.blurb_key);
        assert!(
            zh_blurb.contains("HTML 报告"),
            "zh blurb should name the HTML report: {zh_blurb}"
        );
        assert!(
            en_blurb.contains("HTML report"),
            "en blurb should name the HTML report: {en_blurb}"
        );

        let zh_prompt = t(Locale::Zh, "caps.skill.nature_reader.prompt");
        let en_prompt = t(Locale::En, "caps.skill.nature_reader.prompt");
        assert!(
            zh_prompt.contains("HTML 报告") && zh_prompt.contains("reader.html"),
            "zh prompt should deliver reader.html: {zh_prompt}"
        );
        assert!(
            en_prompt.contains("HTML report") && en_prompt.contains("reader.html"),
            "en prompt should deliver reader.html: {en_prompt}"
        );
    }

    #[test]
    fn capability_actions_resolve_session_title_keys() {
        assert_eq!(
            title_key_for_capability_action(&CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.python_r",
                skill: None,
                specialist: None,
            }),
            Some("caps.tile.python_r.title")
        );
        assert_eq!(
            title_key_for_capability_action(&CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.knowledge",
                skill: None,
                specialist: None,
            }),
            Some("caps.tile.knowledge.title")
        );
        assert_eq!(
            title_key_for_capability_action(&CapabilityAction::GuidedChat {
                prompt_key: "caps.skill.knowledge_graph.prompt",
                skill: Some("knowledge-graph"),
                specialist: None,
            }),
            Some("caps.skill.knowledge_graph.title")
        );
        assert_eq!(
            title_key_for_capability_action(&CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.director_kickoff",
                skill: None,
                specialist: None,
            }),
            Some("caps.tile.director.title")
        );
        assert_eq!(
            title_key_for_capability_action(&CapabilityAction::NewChat),
            None
        );
        assert_eq!(
            title_key_for_capability_action(&CapabilityAction::OpenDemo),
            None
        );
    }

    #[test]
    fn catalog_includes_academic_and_nature_skill_packs() {
        let ids: Vec<_> = capability_catalog().iter().map(|t| t.id).collect();
        for id in [
            "academic-paper",
            "academic-pipeline",
            "academic-paper-reviewer",
            "deep-research",
            "nature-writing",
            "nature-polishing",
            "humanizer-zh",
            "nature-proposal-writer",
            "nature-citation",
            "nature-response",
            "nature-reviewer",
            "nature-data",
            "nature-ref-verifier",
            "nature-paper-to-patent",
            "nature-figure",
            "r-bioinfo-figure",
            "nature-paper2ppt",
            "ppt-master",
            "nature-statistics",
            "knowledge-graph",
            "bio-db",
            "academic-search-pro",
            "nature-academic-search",
            "nature-literature-pipeline",
            "nature-downloader",
            "nature-paper-card",
            "nature-reader",
            "nature-experiment-log",
            "playwright",
            "data-cleaning",
            "pii-firewall",
            "journal-prescreen",
            "handwriting-extract",
            "topic-coach",
            "knowledge",
            "env-setup",
        ] {
            assert!(ids.contains(&id), "missing capability tile {id}");
        }
        let pii = capability_catalog()
            .iter()
            .find(|t| t.id == "pii-firewall")
            .expect("pii tile");
        assert_eq!(pii.group, CapabilityGroup::Efficiency);
        assert_eq!(pii.toggle, Some(CapabilityToggle::PiiFirewall));
        assert_eq!(pii.action, CapabilityAction::None);
        let handwriting = capability_catalog()
            .iter()
            .find(|t| t.id == "handwriting-extract")
            .expect("handwriting tile");
        assert_eq!(
            handwriting.card_settings,
            Some(CapabilityCardSettings::HandwritingModels)
        );
        match handwriting.action {
            CapabilityAction::GuidedChat {
                skill: Some("handwriting-extract"),
                specialist: Some("handwriting_extract"),
                ..
            } => {}
            other => panic!("expected handwriting specialist+skill tile, got {other:?}"),
        }
        let knowledge = capability_catalog()
            .iter()
            .find(|t| t.id == "knowledge")
            .expect("knowledge tile");
        assert_eq!(
            knowledge.card_settings,
            Some(CapabilityCardSettings::Knowledge)
        );
        match knowledge.action {
            CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.knowledge",
                skill: None,
                specialist: None,
            } => {}
            other => panic!("expected knowledge guided chat, got {other:?}"),
        }
        assert_eq!(
            capability_catalog()
                .iter()
                .filter(|tile| tile.card_settings.is_some())
                .map(|tile| tile.id)
                .collect::<Vec<_>>(),
            vec!["handwriting-extract", "knowledge"]
        );
        assert!(!ids.contains(&"nature-shared"));
        assert!(!ids.contains(&"academic-research"));
        assert!(!ids.contains(&"nature-skills"));
        assert!(!ids.contains(&"agents-workflows"));
        assert!(!ids.contains(&"ai-agent"));
    }

    #[test]
    fn skill_tiles_attach_matching_skill_reference() {
        let paper = capability_catalog()
            .iter()
            .find(|t| t.id == "nature-figure")
            .expect("nature-figure tile");
        match paper.action {
            CapabilityAction::GuidedChat {
                skill: Some("nature-figure"),
                specialist: None,
                ..
            } => {}
            other => panic!("expected GuidedChat with nature-figure skill, got {other:?}"),
        }
        let graph = capability_catalog()
            .iter()
            .find(|t| t.id == "knowledge-graph")
            .expect("knowledge-graph tile");
        match graph.action {
            CapabilityAction::GuidedChat {
                skill: Some("knowledge-graph"),
                specialist: None,
                ..
            } => {}
            other => panic!("expected GuidedChat with knowledge-graph skill, got {other:?}"),
        }
        assert_eq!(graph.group, CapabilityGroup::Topic);
        let env = capability_catalog()
            .iter()
            .find(|t| t.id == "env-setup")
            .expect("env-setup tile");
        match env.action {
            CapabilityAction::OpenRuntimeSetup => {}
            other => panic!("expected OpenRuntimeSetup, got {other:?}"),
        }
        assert_eq!(env.group, CapabilityGroup::DataCleaning);
        assert_eq!(env_setup_capability_action(), env.action);
        let cleaner = capability_catalog()
            .iter()
            .find(|t| t.id == "data-cleaning")
            .expect("data-cleaning tile");
        match cleaner.action {
            CapabilityAction::GuidedChat {
                skill: None,
                specialist: Some("data_cleaning"),
                prompt_key: "caps.prompt.data_cleaning",
            } => {}
            other => panic!("expected GuidedChat with data_cleaning specialist, got {other:?}"),
        }
        assert_eq!(cleaner.group, CapabilityGroup::DataCleaning);
        let bio_db = capability_catalog()
            .iter()
            .find(|t| t.id == "bio-db")
            .expect("bio-db tile");
        match bio_db.action {
            CapabilityAction::GuidedChat {
                skill: None,
                specialist: None,
                prompt_key: "caps.prompt.bio_db",
            } => {}
            other => panic!("expected GuidedChat for bio-db without skill, got {other:?}"),
        }
        assert_eq!(bio_db.group, CapabilityGroup::Topic);
        for locale in [Locale::Zh, Locale::En] {
            let featured = t(locale, "conn.featured");
            let prompt = t(locale, "caps.prompt.bio_db");
            assert!(featured.contains("PubMed"), "{featured}");
            assert!(featured.contains("GEO"), "{featured}");
            assert!(prompt.contains("search_mcp_tools"), "{prompt}");
            assert!(prompt.contains("use_mcp_tool"), "{prompt}");
            assert!(prompt.contains("search_articles"), "{prompt}");
            assert!(prompt.contains("geo_search_series"), "{prompt}");
            assert!(prompt.contains("get_uniprot_entries"), "{prompt}");
            assert!(prompt.contains("pdb_search_structures"), "{prompt}");
            assert!(prompt.contains("GEO (Omics Archives)"), "{prompt}");
            assert!(prompt.contains("UniProt (Genes Ontologies)"), "{prompt}");
            assert!(prompt.contains("PDB (Structures Interactions)"), "{prompt}");
            assert!(prompt.contains("search_papers"), "{prompt}");
            assert!(
                prompt.contains("Do not tell me to enable bio-tools or academic-search")
                    || prompt.contains("不要让我去开启名为 bio-tools 或 academic-search"),
                "{prompt}"
            );
        }
        let r_bio = capability_catalog()
            .iter()
            .find(|t| t.id == "r-bioinfo-figure")
            .expect("r-bioinfo-figure tile");
        match r_bio.action {
            CapabilityAction::GuidedChat {
                skill: Some("nature-figure"),
                specialist: Some("r_bioinformatics_figure"),
                ..
            } => {}
            other => panic!(
                "expected GuidedChat with nature-figure + r_bioinformatics_figure, got {other:?}"
            ),
        }
        assert_eq!(
            tiles_for_group(CapabilityGroup::AiDrawing)
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>(),
            vec![
                "nature-figure",
                "r-bioinfo-figure",
                "ai-drawing",
                "ai-mechanism-figure",
                "editable-figure",
                "bioinfo-figure-layout",
                "nature-paper2ppt",
                "ppt-master",
            ]
        );
        let ppt = capability_catalog()
            .iter()
            .find(|t| t.id == "ppt-master")
            .expect("ppt-master tile");
        match ppt.action {
            CapabilityAction::InstallThenGuided {
                skill: "ppt-master",
                prompt_key: "caps.skill.ppt_master.prompt",
            } => {}
            other => panic!("expected InstallThenGuided ppt-master, got {other:?}"),
        }
        let install =
            CatalogSkillInstallState::confirm("caps.skill.ppt_master.prompt", "ppt-master");
        assert_eq!(install.copy_key("title"), "caps.install.ppt_master.title");
        assert!(install.dismissible());
        assert_eq!(
            t(Locale::Zh, "caps.install.ppt_master.title"),
            "安装 PPT Master？"
        );
        assert_eq!(
            t(Locale::En, "caps.install.confirm"),
            "Download and install"
        );
        assert_eq!(
            title_key_for_capability_action(&CapabilityAction::GuidedChat {
                prompt_key: "caps.skill.ppt_master.prompt",
                skill: Some("ppt-master"),
                specialist: None,
            }),
            Some("caps.skill.ppt_master.title")
        );
        for id in [
            "ai-mechanism-figure",
            "editable-figure",
            "bioinfo-figure-layout",
        ] {
            let tile = capability_catalog()
                .iter()
                .find(|t| t.id == id)
                .unwrap_or_else(|| panic!("missing capability tile {id}"));
            assert_eq!(tile.action, CapabilityAction::ComingSoon);
        }
        let asp = capability_catalog()
            .iter()
            .find(|t| t.id == "academic-search-pro")
            .expect("academic-search-pro tile");
        match asp.action {
            CapabilityAction::GuidedChat {
                skill: Some("academic-search-pro"),
                specialist: None,
                prompt_key: "caps.skill.academic_search_pro.prompt",
            } => {}
            other => panic!("expected GuidedChat with academic-search-pro skill, got {other:?}"),
        }
        assert_eq!(asp.group, CapabilityGroup::Topic);
        assert_eq!(asp.icon, "book");
        assert!(tiles_for_group(CapabilityGroup::Writing).len() >= 13);
        assert!(tiles_for_group(CapabilityGroup::Topic).len() >= 10);
        let implement_ids: Vec<_> = tiles_for_group(CapabilityGroup::Implement)
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(
            implement_ids.iter().take(2).copied().collect::<Vec<_>>(),
            vec!["nature-statistics", "stats-analysis"]
        );
        let python_r = capability_catalog()
            .iter()
            .find(|tile| tile.id == "python-r")
            .expect("python-r tile");
        match python_r.action {
            CapabilityAction::GuidedChat {
                skill: None,
                specialist: None,
                prompt_key: "caps.prompt.python_r",
            } => {}
            other => panic!("expected GuidedChat python-r prompt, got {other:?}"),
        }
        for locale in [Locale::Zh, Locale::En] {
            let blurb = t(locale, "caps.tile.python_r.blurb");
            let prompt = t(locale, "caps.prompt.python_r");
            assert!(blurb.to_ascii_lowercase().contains("svg"), "{blurb}");
            assert!(prompt.contains("`python`"), "{prompt}");
            assert!(prompt.contains("`r`"), "{prompt}");
            assert!(prompt.to_ascii_lowercase().contains("svg"), "{prompt}");
            assert!(
                prompt.contains("savefig") && prompt.contains("ggsave"),
                "{prompt}"
            );
        }
    }

    #[test]
    fn workspace_surfaces_sit_on_footer_rows_of_their_stage() {
        assert_eq!(
            tiles_for_group_main(CapabilityGroup::Design)
                .iter()
                .map(|tile| tile.id)
                .collect::<Vec<_>>(),
            vec!["research-graph", "files-library", "demo"]
        );
        assert!(tiles_for_group_footer(CapabilityGroup::Design).is_empty());
        assert_eq!(
            tiles_for_group_footer(CapabilityGroup::Writing)
                .iter()
                .map(|tile| tile.id)
                .collect::<Vec<_>>(),
            vec!["publication"]
        );
        let topic_main: Vec<_> = tiles_for_group_main(CapabilityGroup::Topic)
            .iter()
            .map(|tile| tile.id)
            .collect();
        assert!(topic_main.contains(&"academic-search-pro"));
        assert!(topic_main.contains(&"nature-reader"));
        assert!(!topic_main.iter().any(|id| {
            matches!(
                *id,
                "research-graph" | "publication" | "files-library" | "demo"
            )
        }));
        for group in CapabilityGroup::all() {
            if *group == CapabilityGroup::Writing {
                continue;
            }
            assert!(
                tiles_for_group_footer(*group).is_empty(),
                "unexpected workspace footer tiles in {:?}",
                group
            );
        }
    }

    #[test]
    fn research_stage_tabs_match_classification() {
        let ids = |group| {
            tiles_for_group(group)
                .into_iter()
                .map(|tile| tile.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(CapabilityGroup::Topic),
            vec![
                "deep-research",
                "nature-proposal-writer",
                "knowledge-graph",
                "bio-db",
                "academic-search-pro",
                "nature-academic-search",
                "nature-literature-pipeline",
                "nature-downloader",
                "nature-paper-card",
                "nature-reader",
            ]
        );
        assert_eq!(
            ids(CapabilityGroup::Design),
            vec!["research-graph", "files-library", "demo"]
        );
        assert_eq!(
            ids(CapabilityGroup::Implement),
            vec![
                "nature-statistics",
                "stats-analysis",
                "python-r",
                "remote-compute",
                "structure",
                "single-cell",
                "nature-experiment-log",
            ]
        );
        assert_eq!(
            ids(CapabilityGroup::Writing),
            vec![
                "academic-paper",
                "academic-pipeline",
                "academic-paper-reviewer",
                "nature-writing",
                "nature-polishing",
                "humanizer-zh",
                "nature-citation",
                "nature-response",
                "nature-reviewer",
                "nature-data",
                "nature-ref-verifier",
                "nature-paper-to-patent",
                "officecli",
                "publication",
            ]
        );
        assert_eq!(
            ids(CapabilityGroup::DataCleaning),
            vec!["data-cleaning", "env-setup"]
        );
        assert_eq!(
            ids(CapabilityGroup::Efficiency),
            vec![
                "pii-firewall",
                "journal-prescreen",
                "handwriting-extract",
                "topic-coach",
                "knowledge",
            ]
        );
    }

    #[test]
    fn pii_keyword_lines_assign_stable_placeholders() {
        let parsed =
            parse_pii_keyword_lines("张三\n协和医院\n张三\n# skip\n协和医院 | 医院 | 医院A\n");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].original, "张三");
        assert_eq!(parsed[0].placeholder.as_deref(), Some("〔词1〕"));
        assert_eq!(parsed[1].original, "协和医院");
        assert_eq!(parsed[1].placeholder.as_deref(), Some("〔词2〕"));
        assert_eq!(format_pii_keyword_lines(&parsed), "张三\n协和医院");
        assert_eq!(
            preview_pii_placeholders("张三\n\n协和医院\n张三"),
            "〔词1〕\n\n〔词2〕\n〔词1〕"
        );
    }

    #[test]
    fn pii_keyword_skips_tokens_that_match_an_original() {
        let parsed = parse_pii_keyword_lines("张三\n〔词1〕\n");
        assert_eq!(parsed[0].placeholder.as_deref(), Some("〔词2〕"));
        assert_eq!(parsed[1].original, "〔词1〕");
        assert_eq!(parsed[1].placeholder.as_deref(), Some("〔词3〕"));
        assert_eq!(
            preview_pii_placeholders("张三\n〔词1〕"),
            "〔词2〕\n〔词3〕"
        );
    }

    #[test]
    fn group_ids_round_trip_and_reject_unknown() {
        for group in CapabilityGroup::all() {
            assert_eq!(CapabilityGroup::from_id(group.id()), Some(*group));
        }
        assert_eq!(
            CapabilityGroup::from_id("stats"),
            Some(CapabilityGroup::Implement)
        );
        assert_eq!(
            CapabilityGroup::from_id("paper_writing"),
            Some(CapabilityGroup::Writing)
        );
        assert_eq!(
            CapabilityGroup::from_id("assets"),
            Some(CapabilityGroup::Topic)
        );
        assert_eq!(CapabilityGroup::from_id(""), None);
        assert_eq!(CapabilityGroup::from_id("caps.group.stats"), None);
        assert_eq!(CapabilityGroup::from_id("unknown"), None);
    }

    #[test]
    fn group_tabs_match_product_labels() {
        let keys: Vec<_> = CapabilityGroup::all()
            .iter()
            .map(|g| g.label_key())
            .collect();
        assert_eq!(
            keys,
            vec![
                "caps.group.topic",
                "caps.group.design",
                "caps.group.implement",
                "caps.group.writing",
                "caps.group.ai_drawing",
                "caps.group.data_cleaning",
                "caps.group.efficiency",
                "caps.group.collab",
            ]
        );
        assert_eq!(t(Locale::Zh, "caps.group.topic"), "选题与立项");
        assert_eq!(t(Locale::Zh, "caps.group.efficiency"), "效率工具");
        assert_eq!(t(Locale::En, "caps.group.efficiency"), "Efficiency tools");
        assert_eq!(t(Locale::Zh, "caps.group.design"), "研究设计与规划");
        assert_eq!(t(Locale::Zh, "caps.group.implement"), "研究实施与数据分析");
        assert_eq!(t(Locale::Zh, "caps.group.writing"), "论文写作与发表");
        assert_eq!(t(Locale::En, "caps.group.topic"), "Topic & proposal");
        assert_eq!(t(Locale::En, "caps.group.writing"), "Writing & publication");
    }

    #[test]
    fn efficiency_and_reviewer_copy_resolves_in_both_locales() {
        for id in [
            "pii-firewall",
            "journal-prescreen",
            "handwriting-extract",
            "topic-coach",
            "academic-paper-reviewer",
            "nature-reviewer",
        ] {
            let tile = capability_catalog()
                .iter()
                .find(|tile| tile.id == id)
                .unwrap_or_else(|| panic!("missing tile {id}"));
            for locale in [Locale::En, Locale::Zh] {
                let title = t(locale, tile.title_key);
                assert_ne!(
                    title,
                    tile.title_key,
                    "missing {} title for {id}",
                    locale.code()
                );
                let blurb = t(locale, tile.blurb_key);
                assert_ne!(
                    blurb,
                    tile.blurb_key,
                    "missing {} blurb for {id}",
                    locale.code()
                );
            }
        }
        assert_eq!(
            t(Locale::Zh, "caps.skill.academic_paper_reviewer.title"),
            "国际期刊审稿"
        );
        assert_eq!(
            t(Locale::En, "caps.skill.academic_paper_reviewer.title"),
            "International journal review"
        );
        assert_eq!(
            t(Locale::Zh, "caps.skill.nature_reviewer.title"),
            "Nature审稿"
        );
        assert_eq!(
            t(Locale::En, "caps.skill.nature_reviewer.title"),
            "Nature review"
        );
        assert_eq!(
            t(Locale::Zh, "caps.skill.journal_prescreen.title"),
            "论文预审"
        );
        assert_eq!(
            t(Locale::En, "caps.skill.journal_prescreen.title"),
            "Journal prescreen"
        );
        assert_eq!(
            t(Locale::Zh, "caps.tile.pii_firewall.terms_title"),
            "敏感关键词"
        );
        assert_eq!(
            t(Locale::En, "caps.tile.pii_firewall.terms_title"),
            "Sensitive keywords"
        );
        assert_eq!(
            t(Locale::Zh, "caps.tile.pii_firewall.terms_preview_title"),
            "系统占位词"
        );
        for locale in [Locale::En, Locale::Zh] {
            for key in [
                "caps.skill.handwriting_extract.settings.aria",
                "caps.skill.handwriting_extract.settings.title",
                "caps.skill.handwriting_extract.settings.lead",
                "caps.skill.handwriting_extract.settings.model",
                "caps.skill.handwriting_extract.settings.calibration",
                "caps.skill.handwriting_extract.settings.placeholder",
                "caps.skill.handwriting_extract.settings.empty",
                "caps.skill.handwriting_extract.settings.required",
                "caps.skill.handwriting_extract.settings.save",
                "caps.skill.handwriting_extract.settings.close",
                "caps.tile.knowledge.settings.aria",
                "caps.tile.knowledge.settings.title",
                "caps.tile.knowledge.settings.close",
                "caps.tile.knowledge.settings.required",
                "caps.prompt.knowledge",
                "caps.prompt.knowledge.spec",
            ] {
                assert_ne!(t(locale, key), key, "missing {} {key}", locale.code());
            }
        }
        assert_eq!(
            t(Locale::Zh, "caps.skill.handwriting_extract.settings.title"),
            "手写提取模型"
        );
        assert_eq!(
            t(Locale::En, "caps.skill.handwriting_extract.settings.title"),
            "Handwritten extract models"
        );
        assert_eq!(
            t(Locale::Zh, "caps.tile.knowledge.settings.title"),
            "知识库设置"
        );
        assert_eq!(
            t(Locale::Zh, "caps.prompt.knowledge"),
            "帮我检索本机知识库并引用出处。先问我想查什么，不要空讲功能。"
        );
        assert!(!handwriting_models_ready("", "vl"));
        assert!(!handwriting_models_ready("vl", ""));
        assert!(handwriting_models_ready("vl", "vl"));
    }
}

/// Shared capability discovery surface for the projects home and Settings →
/// Capabilities. Do not fork a second tab/tile UI for either entry point.
#[component]
pub(crate) fn CapabilitySceneTabs(
    locale: RwSignal<Locale>,
    on_activate: Callback<CapabilityAction>,
    #[prop(optional)] initial_group: Option<CapabilityGroup>,
) -> impl IntoView {
    let active = create_rw_signal(initial_group.unwrap_or_else(load_capability_scene));
    view! {
        <section class="cap-scene" data-testid="capability-scene">
            <div class="cap-scene-head">
                <h2 id="capabilities-title">{move || t(locale.get(), "caps.home.title")}</h2>
                <p class="cap-scene-subtitle">{move || t(locale.get(), "caps.home.subtitle")}</p>
            </div>
            <div class="cap-scene-tabs" role="tablist" aria-label=move || t(locale.get(), "caps.home.title")>
                {CapabilityGroup::all().iter().copied().map(|group| {
                    let group_for_click = group;
                    view! {
                        <button type="button" class="cap-scene-tab" role="tab"
                            data-testid=format!("cap-tab-{}", group.label_key())
                            class:active=move || active.get() == group
                            aria-selected=move || (active.get() == group).to_string()
                            on:click=move |_| {
                                save_capability_scene(group_for_click);
                                active.set(group_for_click);
                            }>
                            {move || t(locale.get(), group.label_key())}
                        </button>
                    }
                }).collect_view()}
            </div>
            <CapabilityTileGrid locale=locale group=Signal::derive(move || active.get()) on_activate=on_activate />
        </section>
    }
}

/// Tile grid for a single capability group (used by the fullscreen overlay tabs).
#[component]
pub(crate) fn CapabilityTileGrid(
    locale: RwSignal<Locale>,
    group: Signal<CapabilityGroup>,
    on_activate: Callback<CapabilityAction>,
) -> impl IntoView {
    let pii_firewall_enabled = create_rw_signal(true);
    let pii_firewall_info_open = create_rw_signal(false);
    let pii_terms_text = create_rw_signal(String::new());
    let pii_terms_notice = create_rw_signal(String::new());
    let pii_terms_dirty = create_rw_signal(false);
    let help_tile = create_rw_signal(None::<&'static CapabilityTile>);
    let coming_soon_tile = create_rw_signal(None::<&'static CapabilityTile>);
    let handwriting_vision_open = create_rw_signal(false);
    let handwriting_vision_pick = create_rw_signal(String::new());
    let handwriting_calib_pick = create_rw_signal(String::new());
    let handwriting_vision_models = create_rw_signal(Vec::<ModelProfile>::new());
    let handwriting_vision_notice = create_rw_signal(String::new());
    let handwriting_pending_launch = create_rw_signal(None::<CapabilityAction>);
    let knowledge_settings_open = create_rw_signal(false);
    let knowledge_pending_launch = create_rw_signal(None::<CapabilityAction>);
    let knowledge_notice = create_rw_signal(None::<(bool, String)>);
    let close_handwriting_settings = Callback::new(move |_: ()| {
        handwriting_vision_open.set(false);
        handwriting_pending_launch.set(None);
        handwriting_vision_notice.set(String::new());
    });
    let close_knowledge_settings = Callback::new(move |_: ()| {
        knowledge_settings_open.set(false);
        knowledge_pending_launch.set(None);
        knowledge_notice.set(None);
    });
    let on_knowledge_connected = Callback::new(move |_: ()| {
        let action = knowledge_pending_launch.get_untracked();
        knowledge_settings_open.set(false);
        knowledge_pending_launch.set(None);
        knowledge_notice.set(None);
        if let Some(action) = action {
            on_activate.call(action);
        }
    });
    let on_help = Callback::new(move |tile: &'static CapabilityTile| {
        help_tile.set(Some(tile));
    });
    let on_card_settings =
        Callback::new(
            move |tile: &'static CapabilityTile| match tile.card_settings {
                Some(CapabilityCardSettings::Knowledge) => {
                    knowledge_pending_launch.set(None);
                    knowledge_notice.set(None);
                    knowledge_settings_open.set(true);
                }
                Some(CapabilityCardSettings::HandwritingModels) => {
                    handwriting_pending_launch.set(None);
                    spawn_local(async move {
                        let (candidates, vision, calib) = load_handwriting_model_picks().await;
                        remember_handwriting_picks(
                            handwriting_vision_models,
                            handwriting_vision_pick,
                            handwriting_calib_pick,
                            candidates,
                            vision,
                            calib,
                        );
                        handwriting_vision_notice.set(String::new());
                        handwriting_vision_open.set(true);
                    });
                }
                None => {}
            },
        );
    let gated_activate = Callback::new(move |action: CapabilityAction| {
        if matches!(
            action,
            CapabilityAction::GuidedChat {
                skill: Some("handwriting-extract"),
                ..
            }
        ) {
            spawn_local(async move {
                let (candidates, vision, calib) = load_handwriting_model_picks().await;
                remember_handwriting_picks(
                    handwriting_vision_models,
                    handwriting_vision_pick,
                    handwriting_calib_pick,
                    candidates,
                    vision.clone(),
                    calib.clone(),
                );
                if handwriting_models_ready(&vision, &calib) {
                    handwriting_pending_launch.set(None);
                    on_activate.call(action);
                    return;
                }
                handwriting_vision_notice.set(String::new());
                handwriting_pending_launch.set(Some(action));
                handwriting_vision_open.set(true);
            });
            return;
        }
        if matches!(
            action,
            CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.knowledge",
                ..
            }
        ) {
            spawn_local(async move {
                match probe_knowledge_ready().await {
                    Ok(result) if result.ok => {
                        knowledge_pending_launch.set(None);
                        on_activate.call(action);
                    }
                    Ok(result) => {
                        let text = if result.message.trim().is_empty() {
                            t(
                                locale.get_untracked(),
                                "caps.tile.knowledge.settings.required",
                            )
                            .into()
                        } else {
                            result.message
                        };
                        knowledge_notice.set(Some((false, text)));
                        knowledge_pending_launch.set(Some(action));
                        knowledge_settings_open.set(true);
                    }
                    Err(err) => {
                        knowledge_notice.set(Some((false, err)));
                        knowledge_pending_launch.set(Some(action));
                        knowledge_settings_open.set(true);
                    }
                }
            });
            return;
        }
        on_activate.call(action);
    });
    let persist_pii_terms = move || {
        let terms = parse_pii_keyword_lines(&pii_terms_text.get_untracked());
        pii_terms_notice.set(String::new());
        spawn_local(async move {
            if let Ok(v) = invoke_checked(
                "set_pii_custom_terms",
                to_value(&serde_json::json!({ "terms": terms })).unwrap(),
            )
            .await
            {
                if let Ok(saved) = serde_wasm_bindgen::from_value::<Vec<PiiCustomTermView>>(v) {
                    pii_terms_text.set(format_pii_keyword_lines(&saved));
                    pii_terms_dirty.set(false);
                    pii_terms_notice.set(t(
                        locale.get_untracked(),
                        "caps.tile.pii_firewall.terms_saved",
                    ));
                }
            }
        });
    };
    let close_pii_info = move || {
        if pii_terms_dirty.get_untracked() {
            persist_pii_terms();
        }
        pii_firewall_info_open.set(false);
    };
    create_effect(move |_| {
        spawn_local(async move {
            let v = invoke("get_pii_firewall_enabled", JsValue::UNDEFINED).await;
            if let Ok(on) = serde_wasm_bindgen::from_value::<bool>(v) {
                pii_firewall_enabled.set(on);
            }
        });
    });
    create_effect(move |_| {
        if !pii_firewall_info_open.get() {
            return;
        }
        pii_terms_notice.set(String::new());
        pii_terms_dirty.set(false);
        spawn_local(async move {
            let v = invoke("get_pii_custom_terms", JsValue::UNDEFINED).await;
            if let Ok(terms) = serde_wasm_bindgen::from_value::<Vec<PiiCustomTermView>>(v) {
                pii_terms_text.set(format_pii_keyword_lines(&terms));
                pii_terms_dirty.set(false);
            }
        });
    });
    window_capture_escape(move || {
        if knowledge_settings_open.get_untracked() {
            close_knowledge_settings.call(());
            true
        } else if handwriting_vision_open.get_untracked() {
            close_handwriting_settings.call(());
            true
        } else if help_tile.get_untracked().is_some() {
            help_tile.set(None);
            true
        } else if coming_soon_tile.get_untracked().is_some() {
            coming_soon_tile.set(None);
            true
        } else if pii_firewall_info_open.get_untracked() {
            close_pii_info();
            true
        } else {
            false
        }
    });

    let set_pii_enabled = move |on: bool| {
        pii_firewall_enabled.set(on);
        spawn_local(async move {
            let _ = invoke_checked(
                "set_pii_firewall_enabled",
                to_value(&serde_json::json!({ "enabled": on })).unwrap(),
            )
            .await;
        });
    };
    let save_pii_terms = move |_| persist_pii_terms();

    view! {
        <div class="cap-tile-stack">
        <div class="cap-tile-grid" data-testid="capability-tile-grid" role="list">
            {move || {
                tiles_for_group_main(group.get()).into_iter().map(|tile| {
                    let icon = tile.icon;
                    let title_key = tile.title_key;
                    let blurb_key = tile.blurb_key;
                    let id = tile.id;
                    let toggle = tile.toggle;
                    if let Some(CapabilityToggle::PiiFirewall) = toggle {
                        view! {
                            <div class="cap-tile-wrap" role="listitem">
                            {capability_help_button(tile, locale, on_help)}
                            <div class="cap-tile cap-tile-toggle"
                                data-testid=format!("cap-tile-{id}")
                                tabindex="0"
                                on:click=move |_| pii_firewall_info_open.set(true)
                                on:keydown=move |ev| {
                                    if ev.key() == "Enter" || ev.key() == " " {
                                        ev.prevent_default();
                                        pii_firewall_info_open.set(true);
                                    }
                                }>
                                <div class="cap-tile-toggle-row">
                                    <span class="cap-tile-icon" aria-hidden="true">{compose_icon(icon)}</span>
                                    <label class="toggle cap-tile-switch"
                                        data-testid="cap-tile-pii-firewall-switch"
                                        title=move || t(locale.get(), "caps.tile.pii_firewall.toggle")
                                        on:click=move |ev| ev.stop_propagation()>
                                        <input type="checkbox" role="switch"
                                            data-testid="cap-tile-pii-firewall-input"
                                            prop:checked=move || pii_firewall_enabled.get()
                                            on:click=move |ev| ev.stop_propagation()
                                            on:change=move |ev| {
                                                ev.stop_propagation();
                                                set_pii_enabled(event_target_checked(&ev));
                                            }
                                        />
                                        <span class="toggle-track" aria-hidden="true"></span>
                                        <span class="sr-only">{move || t(locale.get(), "caps.tile.pii_firewall.toggle")}</span>
                                    </label>
                                </div>
                                <span class="cap-tile-title">{move || t(locale.get(), title_key)}</span>
                                <span class="cap-tile-blurb">{move || t(locale.get(), blurb_key)}</span>
                            </div>
                            </div>
                        }.into_view()
                    } else if matches!(tile.action, CapabilityAction::ComingSoon) {
                        let open_soon = Callback::new(move |_| {
                            coming_soon_tile.set(Some(tile));
                        });
                        capability_action_tile(tile, locale, open_soon, on_help, None)
                            .into_view()
                    } else {
                        let settings = tile
                            .card_settings
                            .map(|_| on_card_settings);
                        capability_action_tile(
                            tile,
                            locale,
                            gated_activate,
                            on_help,
                            settings,
                        )
                        .into_view()
                    }
                }).collect_view()
            }}
        </div>
        {move || {
            let footer = tiles_for_group_footer(group.get());
            (!footer.is_empty()).then(|| view! {
                <div class="cap-tile-grid cap-tile-grid-footer" data-testid="capability-tile-grid-footer" role="list">
                    {footer.into_iter().map(|tile| {
                        capability_action_tile(tile, locale, gated_activate, on_help, None)
                    }).collect_view()}
                </div>
            })
        }}
        </div>
        {move || coming_soon_tile.get().map(|tile| {
            let title_key = tile.title_key;
            view! {
                <div class="overlay" data-testid="cap-coming-soon-overlay"
                    on:click=move |_| coming_soon_tile.set(None)>
                    <div class="modal cap-info-modal" role="dialog" aria-modal="true"
                        aria-labelledby="cap-coming-soon-title"
                        on:click=move |ev| ev.stop_propagation()>
                        <div class="ps-head">
                            <h2 id="cap-coming-soon-title">{move || t(locale.get(), title_key)}</h2>
                            <button type="button" class="ps-close"
                                aria-label=move || t(locale.get(), "caps.coming_soon.close")
                                on:click=move |_| coming_soon_tile.set(None)>
                                {compose_icon("close")}
                            </button>
                        </div>
                        <p class="cap-info-lead" data-testid="cap-coming-soon-body">
                            {move || t(locale.get(), "caps.coming_soon.body")}
                        </p>
                        <div class="row">
                            <button type="button" class="primary"
                                data-testid="cap-coming-soon-close"
                                on:click=move |_| coming_soon_tile.set(None)>
                                {move || t(locale.get(), "caps.coming_soon.close")}
                            </button>
                        </div>
                    </div>
                </div>
            }
        })}
        {move || help_tile.get().map(|tile| {
            let title_key = tile.title_key;
            let help = tile.help;
            view! {
                <div class="overlay" data-testid="cap-help-overlay"
                    on:click=move |_| help_tile.set(None)>
                    <div class="modal cap-info-modal" role="dialog" aria-modal="true"
                        aria-labelledby="cap-help-title"
                        on:click=move |ev| ev.stop_propagation()>
                        <div class="ps-head">
                            <h2 id="cap-help-title">{move || t(locale.get(), title_key)}</h2>
                            <button type="button" class="ps-close"
                                aria-label=move || t(locale.get(), "caps.help.close")
                                on:click=move |_| help_tile.set(None)>
                                {compose_icon("close")}
                            </button>
                        </div>
                        <p class="cap-info-lead" data-testid="cap-help-body">
                            {move || help.get(locale.get())}
                        </p>
                        <div class="row">
                            <button type="button" class="primary"
                                data-testid="cap-help-close"
                                on:click=move |_| help_tile.set(None)>
                                {move || t(locale.get(), "caps.help.close")}
                            </button>
                        </div>
                    </div>
                </div>
            }
        })}
        {move || knowledge_settings_open.get().then(|| {
            let require_connection = knowledge_pending_launch.get().is_some();
            view! {
                <KnowledgeSettingsOverlay
                    locale=locale
                    require_connection=require_connection
                    notice=knowledge_notice
                    on_close=close_knowledge_settings
                    on_connected=on_knowledge_connected
                />
            }
        })}
        {move || handwriting_vision_open.get().then(|| {
            let candidates = handwriting_vision_models.get();
            let empty = candidates.is_empty();
            view! {
                <div class="overlay" data-testid="handwriting-vision-overlay"
                    on:click=move |_| close_handwriting_settings.call(())>
                    <div class="modal cap-info-modal" role="dialog" aria-modal="true"
                        aria-labelledby="handwriting-vision-title"
                        on:click=move |ev| ev.stop_propagation()>
                        <div class="ps-head">
                            <h2 id="handwriting-vision-title">
                                {move || t(locale.get(), "caps.skill.handwriting_extract.settings.title")}
                            </h2>
                            <button type="button" class="ps-close"
                                aria-label=move || t(locale.get(), "caps.skill.handwriting_extract.settings.close")
                                on:click=move |_| close_handwriting_settings.call(())>
                                {compose_icon("close")}
                            </button>
                        </div>
                        <p class="cap-info-lead">
                            {move || t(locale.get(), "caps.skill.handwriting_extract.settings.lead")}
                        </p>
                        {if empty {
                            view! {
                                <p class="hint" data-testid="handwriting-vision-empty">
                                    {move || t(locale.get(), "caps.skill.handwriting_extract.settings.empty")}
                                </p>
                            }.into_view()
                        } else {
                            let calib_candidates = candidates.clone();
                            view! {
                                <label class="cap-field">
                                    <span>{move || t(locale.get(), "caps.skill.handwriting_extract.settings.model")}</span>
                                    <select data-testid="handwriting-vision-select"
                                        prop:value=move || handwriting_vision_pick.get()
                                        on:change=move |ev| {
                                            handwriting_vision_pick.set(event_target_value(&ev));
                                        }>
                                        <option value=""
                                            prop:selected=move || handwriting_vision_pick.get().is_empty()>
                                            {move || t(locale.get(), "caps.skill.handwriting_extract.settings.placeholder")}
                                        </option>
                                        {candidates.into_iter().map(|model| {
                                            let id = model.id.clone();
                                            let selected_id = id.clone();
                                            let label = if model.label.trim().is_empty() {
                                                model.model.clone()
                                            } else {
                                                model.label.clone()
                                            };
                                            view! {
                                                <option value=id
                                                    prop:selected=move || handwriting_vision_pick.get() == selected_id>
                                                    {label}
                                                </option>
                                            }
                                        }).collect_view()}
                                    </select>
                                </label>
                                <label class="cap-field">
                                    <span>{move || t(locale.get(), "caps.skill.handwriting_extract.settings.calibration")}</span>
                                    <select data-testid="handwriting-calibration-select"
                                        prop:value=move || handwriting_calib_pick.get()
                                        on:change=move |ev| {
                                            handwriting_calib_pick.set(event_target_value(&ev));
                                        }>
                                        <option value=""
                                            prop:selected=move || handwriting_calib_pick.get().is_empty()>
                                            {move || t(locale.get(), "caps.skill.handwriting_extract.settings.placeholder")}
                                        </option>
                                        {calib_candidates.into_iter().map(|model| {
                                            let id = model.id.clone();
                                            let selected_id = id.clone();
                                            let label = if model.label.trim().is_empty() {
                                                model.model.clone()
                                            } else {
                                                model.label.clone()
                                            };
                                            view! {
                                                <option value=id
                                                    prop:selected=move || handwriting_calib_pick.get() == selected_id>
                                                    {label}
                                                </option>
                                            }
                                        }).collect_view()}
                                    </select>
                                </label>
                            }.into_view()
                        }}
                        {move || {
                            let notice = handwriting_vision_notice.get();
                            (!notice.is_empty()).then(|| view! {
                                <div class="settings-status fail" data-testid="handwriting-vision-notice">{notice}</div>
                            })
                        }}
                        <div class="row">
                            <button type="button"
                                on:click=move |_| close_handwriting_settings.call(())>
                                {move || t(locale.get(), "caps.skill.handwriting_extract.settings.close")}
                            </button>
                            <button type="button" class="primary"
                                data-testid="handwriting-vision-save"
                                disabled=empty
                                on:click=move |_| {
                                    let pick = handwriting_vision_pick.get_untracked();
                                    let calib = handwriting_calib_pick.get_untracked();
                                    if pick.is_empty() || calib.is_empty() {
                                        handwriting_vision_notice.set(
                                            t(locale.get_untracked(), "caps.skill.handwriting_extract.settings.required"),
                                        );
                                        return;
                                    }
                                    spawn_local(async move {
                                        let vision_ok = match invoke_checked(
                                            "set_handwriting_extract_vision_model",
                                            to_value(&serde_json::json!({ "modelId": pick })).unwrap(),
                                        )
                                        .await
                                        {
                                            Ok(value) => {
                                                if let Ok(saved) = serde_wasm_bindgen::from_value::<String>(value) {
                                                    handwriting_vision_pick.set(saved);
                                                }
                                                true
                                            }
                                            Err(_) => false,
                                        };
                                        let calib_ok = if vision_ok {
                                            match invoke_checked(
                                                "set_handwriting_extract_calibration_model",
                                                to_value(&serde_json::json!({ "modelId": calib })).unwrap(),
                                            )
                                            .await
                                            {
                                                Ok(value) => {
                                                    if let Ok(saved) = serde_wasm_bindgen::from_value::<String>(value) {
                                                        handwriting_calib_pick.set(saved);
                                                    }
                                                    true
                                                }
                                                Err(_) => false,
                                            }
                                        } else {
                                            false
                                        };
                                        if vision_ok && calib_ok {
                                            handwriting_vision_notice.set(String::new());
                                            handwriting_vision_open.set(false);
                                            if let Some(action) = handwriting_pending_launch.get_untracked() {
                                                handwriting_pending_launch.set(None);
                                                on_activate.call(action);
                                            }
                                        } else {
                                            handwriting_vision_notice.set(
                                                t(locale.get_untracked(), "caps.skill.handwriting_extract.settings.required"),
                                            );
                                        }
                                    });
                                }>
                                {move || t(locale.get(), "caps.skill.handwriting_extract.settings.save")}
                            </button>
                        </div>
                    </div>
                </div>
            }
        })}
        {move || pii_firewall_info_open.get().then(|| view! {
            <div class="overlay caps-fullscreen-overlay pii-firewall-overlay"
                data-testid="pii-firewall-info-overlay">
                <div class="modal caps-fullscreen pii-firewall-page" role="dialog" aria-modal="true"
                    aria-labelledby="pii-firewall-info-title">
                    <header class="pii-fw-head">
                        <div class="pii-fw-title-block">
                            <span class="pii-fw-mark" aria-hidden="true">{compose_icon("shield")}</span>
                            <div>
                                <h2 id="pii-firewall-info-title">{move || t(locale.get(), "caps.tile.pii_firewall.title")}</h2>
                                <p class="pii-fw-subtitle">{move || t(locale.get(), "caps.tile.pii_firewall.modal_lead")}</p>
                            </div>
                        </div>
                        <button type="button" class="ps-close"
                            aria-label=move || t(locale.get(), "caps.tile.pii_firewall.close")
                            on:click=move |_| close_pii_info()>
                            {compose_icon("close")}
                        </button>
                    </header>
                    <div class="pii-fw-body">
                        <section class="pii-fw-col pii-fw-explain" aria-label=move || t(locale.get(), "caps.tile.pii_firewall.modal_title")>
                            <div class="pii-fw-card">
                                <h3>{move || t(locale.get(), "caps.tile.pii_firewall.modal_what_title")}</h3>
                                <ul class="pii-fw-points">
                                    <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_what_1")}</li>
                                    <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_what_2")}</li>
                                    <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_what_3")}</li>
                                </ul>
                            </div>
                            <div class="pii-fw-card">
                                <h3>{move || t(locale.get(), "caps.tile.pii_firewall.modal_how_title")}</h3>
                                <ol class="pii-fw-steps">
                                    <li>
                                        <span class="pii-fw-step-n" aria-hidden="true">"1"</span>
                                        <span>{move || t(locale.get(), "caps.tile.pii_firewall.modal_how_1")}</span>
                                    </li>
                                    <li>
                                        <span class="pii-fw-step-n" aria-hidden="true">"2"</span>
                                        <span>{move || t(locale.get(), "caps.tile.pii_firewall.modal_how_2")}</span>
                                    </li>
                                    <li>
                                        <span class="pii-fw-step-n" aria-hidden="true">"3"</span>
                                        <span>{move || t(locale.get(), "caps.tile.pii_firewall.modal_how_3")}</span>
                                    </li>
                                </ol>
                            </div>
                            <div class="pii-fw-card pii-fw-example">
                                <h3>{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_title")}</h3>
                                <div class="pii-fw-example-grid">
                                    <div class="pii-fw-example-pane">
                                        <span class="cap-info-tag">{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_you")}</span>
                                        <code>{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_before")}</code>
                                    </div>
                                    <span class="pii-fw-example-arrow" aria-hidden="true">{compose_icon("chevron-right")}</span>
                                    <div class="pii-fw-example-pane muted">
                                        <span class="cap-info-tag muted">{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_model")}</span>
                                        <code>{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_after")}</code>
                                    </div>
                                </div>
                            </div>
                            <p class="pii-fw-note">{move || t(locale.get(), "caps.tile.pii_firewall.modal_note")}</p>
                        </section>
                        <section class="pii-fw-col pii-fw-editor">
                            <div class="pii-fw-pair">
                                <div class="pii-fw-editor-card">
                                    <div class="pii-fw-editor-head">
                                        <h3>{move || t(locale.get(), "caps.tile.pii_firewall.terms_title")}</h3>
                                        <p>{move || t(locale.get(), "caps.tile.pii_firewall.terms_help")}</p>
                                    </div>
                                    <textarea class="pii-fw-terms"
                                        data-testid="pii-firewall-terms"
                                        prop:value=move || pii_terms_text.get()
                                        placeholder=move || t(locale.get(), "caps.tile.pii_firewall.terms_placeholder")
                                        on:input=move |ev| {
                                            pii_terms_text.set(event_target_value(&ev));
                                            pii_terms_dirty.set(true);
                                            pii_terms_notice.set(String::new());
                                        }>
                                    </textarea>
                                </div>
                                <div class="pii-fw-editor-card pii-fw-preview-card">
                                    <div class="pii-fw-editor-head">
                                        <h3>{move || t(locale.get(), "caps.tile.pii_firewall.terms_preview_title")}</h3>
                                        <p>{move || t(locale.get(), "caps.tile.pii_firewall.terms_preview_help")}</p>
                                    </div>
                                    <textarea class="pii-fw-terms pii-fw-preview"
                                        data-testid="pii-firewall-terms-preview"
                                        readonly
                                        prop:value=move || preview_pii_placeholders(&pii_terms_text.get())
                                        placeholder=move || t(locale.get(), "caps.tile.pii_firewall.terms_preview_empty")>
                                    </textarea>
                                </div>
                            </div>
                            <div class="pii-fw-editor-bar">
                                <p class="pii-fw-save-status" data-testid="pii-firewall-terms-notice">
                                    {move || pii_terms_notice.get()}
                                </p>
                                <button type="button" class="primary"
                                    class:pii-fw-save-dirty=move || pii_terms_dirty.get()
                                    data-testid="pii-firewall-terms-save"
                                    on:click=save_pii_terms>
                                    {compose_icon("save")}
                                    {move || t(locale.get(), "caps.tile.pii_firewall.terms_save")}
                                </button>
                            </div>
                        </section>
                    </div>
                    <footer class="pii-fw-foot">
                        <label class="toggle cap-info-toggle">
                            <input type="checkbox" role="switch"
                                data-testid="pii-firewall-info-switch"
                                prop:checked=move || pii_firewall_enabled.get()
                                on:change=move |ev| set_pii_enabled(event_target_checked(&ev))
                            />
                            <span class="toggle-track" aria-hidden="true"></span>
                            <span>{move || t(locale.get(), "caps.tile.pii_firewall.toggle")}</span>
                        </label>
                        <button type="button" class="primary"
                            data-testid="pii-firewall-info-close"
                            on:click=move |_| close_pii_info()>
                            {move || t(locale.get(), "caps.tile.pii_firewall.close")}
                        </button>
                    </footer>
                </div>
            </div>
        })}
    }
}
