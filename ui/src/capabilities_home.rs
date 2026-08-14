//! Curated capability discovery tiles shared by the projects home and the
//! fullscreen Capabilities overlay.

use crate::app_support::compose_icon;
use crate::bindings::{invoke, invoke_checked};
use crate::i18n::{t, Locale};
use crate::text::event_target_checked;
use crate::window_capture_escape;
use leptos::*;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CapabilityGroup {
    PaperWriting,
    AiDrawing,
    DataCleaning,
    Stats,
    Structure,
    Assets,
    Collab,
}

impl CapabilityGroup {
    pub(crate) fn all() -> &'static [CapabilityGroup] {
        &[
            CapabilityGroup::PaperWriting,
            CapabilityGroup::AiDrawing,
            CapabilityGroup::DataCleaning,
            CapabilityGroup::Stats,
            CapabilityGroup::Structure,
            CapabilityGroup::Assets,
            CapabilityGroup::Collab,
        ]
    }

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            CapabilityGroup::PaperWriting => "caps.group.paper_writing",
            CapabilityGroup::AiDrawing => "caps.group.ai_drawing",
            CapabilityGroup::DataCleaning => "caps.group.data_cleaning",
            CapabilityGroup::Stats => "caps.group.stats",
            CapabilityGroup::Structure => "caps.group.structure",
            CapabilityGroup::Assets => "caps.group.assets",
            CapabilityGroup::Collab => "caps.group.collab",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilityPanel {
    Files,
    Graph,
    Publication,
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
    OpenSettings { section: &'static str },
    OpenPanel(CapabilityPanel),
    EnvSetup,
    OpenDemo,
    /// Switch-primary tile: no click-to-activate chat/settings action.
    None,
}

/// Optional on-card toggle (persisted setting). Switch is the primary control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilityToggle {
    PiiFirewall,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CapabilityTile {
    pub(crate) id: &'static str,
    pub(crate) group: CapabilityGroup,
    pub(crate) title_key: &'static str,
    pub(crate) blurb_key: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) action: CapabilityAction,
    pub(crate) toggle: Option<CapabilityToggle>,
}

const fn skill_tile(
    id: &'static str,
    group: CapabilityGroup,
    title_key: &'static str,
    blurb_key: &'static str,
    prompt_key: &'static str,
    skill: &'static str,
    icon: &'static str,
) -> CapabilityTile {
    CapabilityTile {
        id,
        group,
        title_key,
        blurb_key,
        icon,
        action: CapabilityAction::GuidedChat {
            prompt_key,
            skill: Some(skill),
            specialist: None,
        },
        toggle: None,
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
) -> CapabilityTile {
    CapabilityTile {
        id,
        group,
        title_key,
        blurb_key,
        icon,
        action: CapabilityAction::GuidedChat {
            prompt_key,
            skill: Some(skill),
            specialist: Some(specialist),
        },
        toggle: None,
    }
}

const fn guided_tile(
    id: &'static str,
    group: CapabilityGroup,
    title_key: &'static str,
    blurb_key: &'static str,
    prompt_key: &'static str,
    icon: &'static str,
) -> CapabilityTile {
    CapabilityTile {
        id,
        group,
        title_key,
        blurb_key,
        icon,
        action: CapabilityAction::GuidedChat {
            prompt_key,
            skill: None,
            specialist: None,
        },
        toggle: None,
    }
}

pub(crate) fn capability_catalog() -> &'static [CapabilityTile] {
    static CATALOG: &[CapabilityTile] = &[
        // —— 论文撰写（academic-research-skills + nature writing family）——
        skill_tile(
            "academic-paper",
            CapabilityGroup::PaperWriting,
            "caps.skill.academic_paper.title",
            "caps.skill.academic_paper.blurb",
            "caps.skill.academic_paper.prompt",
            "academic-paper",
            "skill",
        ),
        skill_tile(
            "academic-pipeline",
            CapabilityGroup::PaperWriting,
            "caps.skill.academic_pipeline.title",
            "caps.skill.academic_pipeline.blurb",
            "caps.skill.academic_pipeline.prompt",
            "academic-pipeline",
            "plan",
        ),
        skill_tile(
            "academic-paper-reviewer",
            CapabilityGroup::PaperWriting,
            "caps.skill.academic_paper_reviewer.title",
            "caps.skill.academic_paper_reviewer.blurb",
            "caps.skill.academic_paper_reviewer.prompt",
            "academic-paper-reviewer",
            "review",
        ),
        skill_tile(
            "deep-research",
            CapabilityGroup::PaperWriting,
            "caps.skill.deep_research.title",
            "caps.skill.deep_research.blurb",
            "caps.skill.deep_research.prompt",
            "deep-research",
            "search",
        ),
        skill_tile(
            "nature-writing",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_writing.title",
            "caps.skill.nature_writing.blurb",
            "caps.skill.nature_writing.prompt",
            "nature-writing",
            "edit",
        ),
        skill_tile(
            "nature-polishing",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_polishing.title",
            "caps.skill.nature_polishing.blurb",
            "caps.skill.nature_polishing.prompt",
            "nature-polishing",
            "edit",
        ),
        skill_tile(
            "humanizer-zh",
            CapabilityGroup::PaperWriting,
            "caps.skill.humanizer_zh.title",
            "caps.skill.humanizer_zh.blurb",
            "caps.skill.humanizer_zh.prompt",
            "humanizer-zh",
            "eye-off",
        ),
        skill_tile(
            "nature-proposal-writer",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_proposal_writer.title",
            "caps.skill.nature_proposal_writer.blurb",
            "caps.skill.nature_proposal_writer.prompt",
            "nature-proposal-writer",
            "doc",
        ),
        skill_tile(
            "nature-citation",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_citation.title",
            "caps.skill.nature_citation.blurb",
            "caps.skill.nature_citation.prompt",
            "nature-citation",
            "doc",
        ),
        skill_tile(
            "nature-response",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_response.title",
            "caps.skill.nature_response.blurb",
            "caps.skill.nature_response.prompt",
            "nature-response",
            "chat",
        ),
        skill_tile(
            "nature-reviewer",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_reviewer.title",
            "caps.skill.nature_reviewer.blurb",
            "caps.skill.nature_reviewer.prompt",
            "nature-reviewer",
            "review",
        ),
        skill_tile(
            "nature-data",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_data.title",
            "caps.skill.nature_data.blurb",
            "caps.skill.nature_data.prompt",
            "nature-data",
            "folder",
        ),
        skill_tile(
            "nature-ref-verifier",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_ref_verifier.title",
            "caps.skill.nature_ref_verifier.blurb",
            "caps.skill.nature_ref_verifier.prompt",
            "nature-ref-verifier",
            "search",
        ),
        skill_tile(
            "nature-paper-to-patent",
            CapabilityGroup::PaperWriting,
            "caps.skill.nature_paper_to_patent.title",
            "caps.skill.nature_paper_to_patent.blurb",
            "caps.skill.nature_paper_to_patent.prompt",
            "nature-paper-to-patent",
            "doc",
        ),
        skill_tile(
            "officecli",
            CapabilityGroup::PaperWriting,
            "caps.tile.officecli.title",
            "caps.tile.officecli.blurb",
            "caps.prompt.officecli",
            "officecli",
            "edit",
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
        ),
        skill_tile(
            "nature-paper2ppt",
            CapabilityGroup::AiDrawing,
            "caps.skill.nature_paper2ppt.title",
            "caps.skill.nature_paper2ppt.blurb",
            "caps.skill.nature_paper2ppt.prompt",
            "nature-paper2ppt",
            "skill",
        ),
        guided_tile(
            "ai-drawing",
            CapabilityGroup::AiDrawing,
            "caps.tile.ai_drawing.title",
            "caps.tile.ai_drawing.blurb",
            "caps.prompt.ai_drawing",
            "image",
        ),
        // Placeholder cards: visible in AI drawing, click wired later.
        CapabilityTile {
            id: "ai-mechanism-figure",
            group: CapabilityGroup::AiDrawing,
            title_key: "caps.tile.ai_mechanism_figure.title",
            blurb_key: "caps.tile.ai_mechanism_figure.blurb",
            icon: "image",
            action: CapabilityAction::None,
            toggle: None,
        },
        CapabilityTile {
            id: "editable-figure",
            group: CapabilityGroup::AiDrawing,
            title_key: "caps.tile.editable_figure.title",
            blurb_key: "caps.tile.editable_figure.blurb",
            icon: "edit",
            action: CapabilityAction::None,
            toggle: None,
        },
        CapabilityTile {
            id: "bioinfo-figure-layout",
            group: CapabilityGroup::AiDrawing,
            title_key: "caps.tile.bioinfo_figure_layout.title",
            blurb_key: "caps.tile.bioinfo_figure_layout.blurb",
            icon: "grid",
            action: CapabilityAction::None,
            toggle: None,
        },
        // —— 数据清洗 ——
        guided_tile(
            "data-cleaning",
            CapabilityGroup::DataCleaning,
            "caps.tile.data_cleaning.title",
            "caps.tile.data_cleaning.blurb",
            "caps.prompt.data_cleaning",
            "plan",
        ),
        CapabilityTile {
            id: "pii-firewall",
            group: CapabilityGroup::DataCleaning,
            title_key: "caps.tile.pii_firewall.title",
            blurb_key: "caps.tile.pii_firewall.blurb",
            icon: "shield",
            action: CapabilityAction::None,
            toggle: Some(CapabilityToggle::PiiFirewall),
        },
        CapabilityTile {
            id: "env-setup",
            group: CapabilityGroup::DataCleaning,
            title_key: "caps.tile.env_setup.title",
            blurb_key: "caps.tile.env_setup.blurb",
            icon: "gear",
            action: CapabilityAction::EnvSetup,
            toggle: None,
        },
        // —— 统计分析 ——
        skill_tile(
            "nature-statistics",
            CapabilityGroup::Stats,
            "caps.skill.nature_statistics.title",
            "caps.skill.nature_statistics.blurb",
            "caps.skill.nature_statistics.prompt",
            "nature-statistics",
            "grid",
        ),
        skill_tile(
            "knowledge-graph",
            CapabilityGroup::Stats,
            "caps.skill.knowledge_graph.title",
            "caps.skill.knowledge_graph.blurb",
            "caps.skill.knowledge_graph.prompt",
            "knowledge-graph",
            "share",
        ),
        guided_tile(
            "stats-analysis",
            CapabilityGroup::Stats,
            "caps.tile.stats_analysis.title",
            "caps.tile.stats_analysis.blurb",
            "caps.prompt.stats_analysis",
            "grid",
        ),
        guided_tile(
            "python-r",
            CapabilityGroup::Stats,
            "caps.tile.python_r.title",
            "caps.tile.python_r.blurb",
            "caps.prompt.python_r",
            "plan",
        ),
        CapabilityTile {
            id: "bio-db",
            group: CapabilityGroup::Stats,
            title_key: "caps.tile.bio_db.title",
            blurb_key: "caps.tile.bio_db.blurb",
            icon: "search",
            action: CapabilityAction::OpenSettings {
                section: "connections",
            },
            toggle: None,
        },
        CapabilityTile {
            id: "remote-compute",
            group: CapabilityGroup::Stats,
            title_key: "caps.tile.remote_compute.title",
            blurb_key: "caps.tile.remote_compute.blurb",
            icon: "server",
            action: CapabilityAction::OpenSettings {
                section: "environments",
            },
            toggle: None,
        },
        // —— 结构与组学 ——
        guided_tile(
            "structure",
            CapabilityGroup::Structure,
            "caps.tile.structure.title",
            "caps.tile.structure.blurb",
            "caps.prompt.structure",
            "image",
        ),
        guided_tile(
            "single-cell",
            CapabilityGroup::Structure,
            "caps.tile.single_cell.title",
            "caps.tile.single_cell.blurb",
            "caps.prompt.single_cell",
            "grid",
        ),
        // —— 研究资产（检索 / 阅读 / 归档）——
        skill_tile(
            "nature-academic-search",
            CapabilityGroup::Assets,
            "caps.skill.nature_academic_search.title",
            "caps.skill.nature_academic_search.blurb",
            "caps.skill.nature_academic_search.prompt",
            "nature-academic-search",
            "search",
        ),
        skill_tile(
            "nature-literature-pipeline",
            CapabilityGroup::Assets,
            "caps.skill.nature_literature_pipeline.title",
            "caps.skill.nature_literature_pipeline.blurb",
            "caps.skill.nature_literature_pipeline.prompt",
            "nature-literature-pipeline",
            "plan",
        ),
        skill_tile(
            "nature-downloader",
            CapabilityGroup::Assets,
            "caps.skill.nature_downloader.title",
            "caps.skill.nature_downloader.blurb",
            "caps.skill.nature_downloader.prompt",
            "nature-downloader",
            "folder",
        ),
        skill_tile(
            "nature-paper-card",
            CapabilityGroup::Assets,
            "caps.skill.nature_paper_card.title",
            "caps.skill.nature_paper_card.blurb",
            "caps.skill.nature_paper_card.prompt",
            "nature-paper-card",
            "doc",
        ),
        skill_tile(
            "nature-reader",
            CapabilityGroup::Assets,
            "caps.skill.nature_reader.title",
            "caps.skill.nature_reader.blurb",
            "caps.skill.nature_reader.prompt",
            "nature-reader",
            "doc",
        ),
        skill_tile(
            "nature-experiment-log",
            CapabilityGroup::Assets,
            "caps.skill.nature_experiment_log.title",
            "caps.skill.nature_experiment_log.blurb",
            "caps.skill.nature_experiment_log.prompt",
            "nature-experiment-log",
            "edit",
        ),
        CapabilityTile {
            id: "research-graph",
            group: CapabilityGroup::Assets,
            title_key: "caps.tile.research_graph.title",
            blurb_key: "caps.tile.research_graph.blurb",
            icon: "branch",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Graph),
            toggle: None,
        },
        CapabilityTile {
            id: "publication",
            group: CapabilityGroup::Assets,
            title_key: "caps.tile.publication.title",
            blurb_key: "caps.tile.publication.blurb",
            icon: "doc",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Publication),
            toggle: None,
        },
        CapabilityTile {
            id: "files-library",
            group: CapabilityGroup::Assets,
            title_key: "caps.tile.files_library.title",
            blurb_key: "caps.tile.files_library.blurb",
            icon: "folder",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Files),
            toggle: None,
        },
        CapabilityTile {
            id: "demo",
            group: CapabilityGroup::Assets,
            title_key: "caps.tile.demo.title",
            blurb_key: "caps.tile.demo.blurb",
            icon: "star",
            action: CapabilityAction::OpenDemo,
            toggle: None,
        },
        // —— 协作扩展 ——
        CapabilityTile {
            id: "ai-agent",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.ai_agent.title",
            blurb_key: "caps.tile.ai_agent.blurb",
            icon: "chat",
            action: CapabilityAction::NewChat,
            toggle: None,
        },
        CapabilityTile {
            id: "agents-workflows",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.agents.title",
            blurb_key: "caps.tile.agents.blurb",
            icon: "review",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Agents),
            toggle: None,
        },
        CapabilityTile {
            id: "channels",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.channels.title",
            blurb_key: "caps.tile.channels.blurb",
            icon: "chat",
            action: CapabilityAction::OpenSettings {
                section: "channels",
            },
            toggle: None,
        },
        guided_tile(
            "browser",
            CapabilityGroup::Collab,
            "caps.tile.browser.title",
            "caps.tile.browser.blurb",
            "caps.prompt.browser",
            "computer",
        ),
        skill_tile(
            "playwright",
            CapabilityGroup::Collab,
            "caps.tile.playwright.title",
            "caps.tile.playwright.blurb",
            "caps.prompt.playwright",
            "playwright",
            "terminal",
        ),
        CapabilityTile {
            id: "plugins",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.plugins.title",
            blurb_key: "caps.tile.plugins.blurb",
            icon: "plus",
            action: CapabilityAction::OpenSettings {
                section: "plugins",
            },
            toggle: None,
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

#[cfg(test)]
mod tests {
    use super::*;

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
            "nature-statistics",
            "knowledge-graph",
            "nature-academic-search",
            "nature-literature-pipeline",
            "nature-downloader",
            "nature-paper-card",
            "nature-reader",
            "nature-experiment-log",
            "playwright",
            "pii-firewall",
        ] {
            assert!(ids.contains(&id), "missing capability tile {id}");
        }
        let pii = capability_catalog()
            .iter()
            .find(|t| t.id == "pii-firewall")
            .expect("pii tile");
        assert_eq!(pii.group, CapabilityGroup::DataCleaning);
        assert_eq!(pii.toggle, Some(CapabilityToggle::PiiFirewall));
        assert_eq!(pii.action, CapabilityAction::None);
        assert!(!ids.contains(&"nature-shared"));
        assert!(!ids.contains(&"academic-research"));
        assert!(!ids.contains(&"nature-skills"));
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
        assert_eq!(graph.group, CapabilityGroup::Stats);
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
                "nature-paper2ppt",
                "ai-drawing",
                "ai-mechanism-figure",
                "editable-figure",
                "bioinfo-figure-layout",
            ]
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
            assert_eq!(tile.action, CapabilityAction::None);
        }
        assert!(tiles_for_group(CapabilityGroup::PaperWriting).len() >= 13);
        assert!(tiles_for_group(CapabilityGroup::Assets).len() >= 6);
        let stats_ids: Vec<_> = tiles_for_group(CapabilityGroup::Stats)
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(
            stats_ids.iter().take(2).copied().collect::<Vec<_>>(),
            vec!["nature-statistics", "knowledge-graph"]
        );
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
                "caps.group.paper_writing",
                "caps.group.ai_drawing",
                "caps.group.data_cleaning",
                "caps.group.stats",
                "caps.group.structure",
                "caps.group.assets",
                "caps.group.collab",
            ]
        );
    }
}

/// Scene tabs + tile grid used on the projects home page.
#[component]
pub(crate) fn CapabilitySceneTabs(
    locale: RwSignal<Locale>,
    on_activate: Callback<CapabilityAction>,
    #[prop(optional)] initial_group: Option<CapabilityGroup>,
) -> impl IntoView {
    let active = create_rw_signal(initial_group.unwrap_or(CapabilityGroup::PaperWriting));
    view! {
        <section class="cap-scene" data-testid="capability-scene">
            <div class="cap-scene-head">
                <h2>{move || t(locale.get(), "caps.home.title")}</h2>
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
                            on:click=move |_| active.set(group_for_click)>
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
    create_effect(move |_| {
        spawn_local(async move {
            let v = invoke("get_pii_firewall_enabled", JsValue::UNDEFINED).await;
            if let Ok(on) = serde_wasm_bindgen::from_value::<bool>(v) {
                pii_firewall_enabled.set(on);
            }
        });
    });
    window_capture_escape(move || {
        if pii_firewall_info_open.get_untracked() {
            pii_firewall_info_open.set(false);
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

    view! {
        <div class="cap-tile-grid" data-testid="capability-tile-grid" role="list">
            {move || {
                tiles_for_group(group.get()).into_iter().map(|tile| {
                    let action = tile.action;
                    let icon = tile.icon;
                    let title_key = tile.title_key;
                    let blurb_key = tile.blurb_key;
                    let id = tile.id;
                    let toggle = tile.toggle;
                    if let Some(CapabilityToggle::PiiFirewall) = toggle {
                        view! {
                            <div class="cap-tile cap-tile-toggle" role="listitem"
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
                                        title=move || t(locale.get(), "caps.tile.pii_firewall.toggle")
                                        on:click=move |ev| ev.stop_propagation()>
                                        <input type="checkbox" role="switch"
                                            data-testid="cap-tile-pii-firewall-switch"
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
                        }.into_view()
                    } else {
                        view! {
                            <button type="button" class="cap-tile" role="listitem"
                                data-testid=format!("cap-tile-{id}")
                                on:click=move |_| on_activate.call(action)>
                                <span class="cap-tile-icon" aria-hidden="true">{compose_icon(icon)}</span>
                                <span class="cap-tile-title">{move || t(locale.get(), title_key)}</span>
                                <span class="cap-tile-blurb">{move || t(locale.get(), blurb_key)}</span>
                            </button>
                        }.into_view()
                    }
                }).collect_view()
            }}
        </div>
        {move || pii_firewall_info_open.get().then(|| view! {
            <div class="overlay" data-testid="pii-firewall-info-overlay"
                on:click=move |_| pii_firewall_info_open.set(false)>
                <div class="modal cap-info-modal" role="dialog" aria-modal="true"
                    aria-labelledby="pii-firewall-info-title"
                    on:click=move |ev| ev.stop_propagation()>
                    <div class="ps-head">
                        <h2 id="pii-firewall-info-title">{move || t(locale.get(), "caps.tile.pii_firewall.modal_title")}</h2>
                        <button type="button" class="ps-close"
                            aria-label=move || t(locale.get(), "caps.tile.pii_firewall.close")
                            on:click=move |_| pii_firewall_info_open.set(false)>
                            {compose_icon("close")}
                        </button>
                    </div>
                    <p class="cap-info-lead">{move || t(locale.get(), "caps.tile.pii_firewall.modal_lead")}</p>
                    <div class="cap-info-section">
                        <h3>{move || t(locale.get(), "caps.tile.pii_firewall.modal_what_title")}</h3>
                        <ul>
                            <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_what_1")}</li>
                            <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_what_2")}</li>
                            <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_what_3")}</li>
                        </ul>
                    </div>
                    <div class="cap-info-section">
                        <h3>{move || t(locale.get(), "caps.tile.pii_firewall.modal_how_title")}</h3>
                        <ol class="cap-info-steps">
                            <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_how_1")}</li>
                            <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_how_2")}</li>
                            <li>{move || t(locale.get(), "caps.tile.pii_firewall.modal_how_3")}</li>
                        </ol>
                    </div>
                    <div class="cap-info-example">
                        <div class="cap-info-example-label">{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_title")}</div>
                        <div class="cap-info-example-row">
                            <span class="cap-info-tag">{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_you")}</span>
                            <code>{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_before")}</code>
                        </div>
                        <div class="cap-info-example-row">
                            <span class="cap-info-tag muted">{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_model")}</span>
                            <code>{move || t(locale.get(), "caps.tile.pii_firewall.modal_example_after")}</code>
                        </div>
                    </div>
                    <p class="cap-info-note">{move || t(locale.get(), "caps.tile.pii_firewall.modal_note")}</p>
                    <label class="toggle cap-info-toggle">
                        <input type="checkbox" role="switch"
                            data-testid="pii-firewall-info-switch"
                            prop:checked=move || pii_firewall_enabled.get()
                            on:change=move |ev| set_pii_enabled(event_target_checked(&ev))
                        />
                        <span class="toggle-track" aria-hidden="true"></span>
                        <span>{move || t(locale.get(), "caps.tile.pii_firewall.toggle")}</span>
                    </label>
                    <div class="row">
                        <button type="button" class="primary"
                            data-testid="pii-firewall-info-close"
                            on:click=move |_| pii_firewall_info_open.set(false)>
                            {move || t(locale.get(), "caps.tile.pii_firewall.close")}
                        </button>
                    </div>
                </div>
            </div>
        })}
    }
}
