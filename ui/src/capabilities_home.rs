//! Curated capability discovery tiles shared by the projects home and the
//! fullscreen Capabilities overlay.

use crate::app_support::compose_icon;
use crate::i18n::{t, Locale};
use leptos::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CapabilityGroup {
    Start,
    Literature,
    Compute,
    Structure,
    Assets,
    Collab,
}

impl CapabilityGroup {
    pub(crate) fn all() -> &'static [CapabilityGroup] {
        &[
            CapabilityGroup::Start,
            CapabilityGroup::Literature,
            CapabilityGroup::Compute,
            CapabilityGroup::Structure,
            CapabilityGroup::Assets,
            CapabilityGroup::Collab,
        ]
    }

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            CapabilityGroup::Start => "caps.group.start",
            CapabilityGroup::Literature => "caps.group.literature",
            CapabilityGroup::Compute => "caps.group.compute",
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
    GuidedChat { prompt_key: &'static str },
    OpenSettings { section: &'static str },
    OpenPanel(CapabilityPanel),
    EnvSetup,
    OpenDemo,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CapabilityTile {
    pub(crate) id: &'static str,
    pub(crate) group: CapabilityGroup,
    pub(crate) title_key: &'static str,
    pub(crate) blurb_key: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) action: CapabilityAction,
}

pub(crate) fn capability_catalog() -> &'static [CapabilityTile] {
    &[
        CapabilityTile {
            id: "ai-agent",
            group: CapabilityGroup::Start,
            title_key: "caps.tile.ai_agent.title",
            blurb_key: "caps.tile.ai_agent.blurb",
            icon: "chat",
            action: CapabilityAction::NewChat,
        },
        CapabilityTile {
            id: "env-setup",
            group: CapabilityGroup::Start,
            title_key: "caps.tile.env_setup.title",
            blurb_key: "caps.tile.env_setup.blurb",
            icon: "gear",
            action: CapabilityAction::EnvSetup,
        },
        CapabilityTile {
            id: "demo",
            group: CapabilityGroup::Start,
            title_key: "caps.tile.demo.title",
            blurb_key: "caps.tile.demo.blurb",
            icon: "star",
            action: CapabilityAction::OpenDemo,
        },
        CapabilityTile {
            id: "literature",
            group: CapabilityGroup::Literature,
            title_key: "caps.tile.literature.title",
            blurb_key: "caps.tile.literature.blurb",
            icon: "doc",
            action: CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.literature",
            },
        },
        CapabilityTile {
            id: "pdf-ppt",
            group: CapabilityGroup::Literature,
            title_key: "caps.tile.pdf_ppt.title",
            blurb_key: "caps.tile.pdf_ppt.blurb",
            icon: "skill",
            action: CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.pdf_ppt",
            },
        },
        CapabilityTile {
            id: "bio-db",
            group: CapabilityGroup::Compute,
            title_key: "caps.tile.bio_db.title",
            blurb_key: "caps.tile.bio_db.blurb",
            icon: "search",
            action: CapabilityAction::OpenSettings {
                section: "connections",
            },
        },
        CapabilityTile {
            id: "python-r",
            group: CapabilityGroup::Compute,
            title_key: "caps.tile.python_r.title",
            blurb_key: "caps.tile.python_r.blurb",
            icon: "plan",
            action: CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.python_r",
            },
        },
        CapabilityTile {
            id: "remote-compute",
            group: CapabilityGroup::Compute,
            title_key: "caps.tile.remote_compute.title",
            blurb_key: "caps.tile.remote_compute.blurb",
            icon: "server",
            action: CapabilityAction::OpenSettings {
                section: "environments",
            },
        },
        CapabilityTile {
            id: "structure",
            group: CapabilityGroup::Structure,
            title_key: "caps.tile.structure.title",
            blurb_key: "caps.tile.structure.blurb",
            icon: "image",
            action: CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.structure",
            },
        },
        CapabilityTile {
            id: "single-cell",
            group: CapabilityGroup::Structure,
            title_key: "caps.tile.single_cell.title",
            blurb_key: "caps.tile.single_cell.blurb",
            icon: "grid",
            action: CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.single_cell",
            },
        },
        CapabilityTile {
            id: "research-graph",
            group: CapabilityGroup::Assets,
            title_key: "caps.tile.research_graph.title",
            blurb_key: "caps.tile.research_graph.blurb",
            icon: "branch",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Graph),
        },
        CapabilityTile {
            id: "publication",
            group: CapabilityGroup::Assets,
            title_key: "caps.tile.publication.title",
            blurb_key: "caps.tile.publication.blurb",
            icon: "doc",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Publication),
        },
        CapabilityTile {
            id: "files-library",
            group: CapabilityGroup::Assets,
            title_key: "caps.tile.files_library.title",
            blurb_key: "caps.tile.files_library.blurb",
            icon: "folder",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Files),
        },
        CapabilityTile {
            id: "agents-workflows",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.agents.title",
            blurb_key: "caps.tile.agents.blurb",
            icon: "review",
            action: CapabilityAction::OpenPanel(CapabilityPanel::Agents),
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
        },
        CapabilityTile {
            id: "browser",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.browser.title",
            blurb_key: "caps.tile.browser.blurb",
            icon: "computer",
            action: CapabilityAction::GuidedChat {
                prompt_key: "caps.prompt.browser",
            },
        },
        CapabilityTile {
            id: "plugins",
            group: CapabilityGroup::Collab,
            title_key: "caps.tile.plugins.title",
            blurb_key: "caps.tile.plugins.blurb",
            icon: "plus",
            action: CapabilityAction::OpenSettings {
                section: "plugins",
            },
        },
    ]
}

pub(crate) fn tiles_for_group(group: CapabilityGroup) -> Vec<&'static CapabilityTile> {
    capability_catalog()
        .iter()
        .filter(|tile| tile.group == group)
        .collect()
}

/// Scene tabs + tile grid used on the projects home page.
#[component]
pub(crate) fn CapabilitySceneTabs(
    locale: RwSignal<Locale>,
    on_activate: Callback<CapabilityAction>,
    #[prop(optional)] initial_group: Option<CapabilityGroup>,
) -> impl IntoView {
    let active = create_rw_signal(initial_group.unwrap_or(CapabilityGroup::Start));
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
    view! {
        <div class="cap-tile-grid" data-testid="capability-tile-grid" role="list">
            {move || {
                tiles_for_group(group.get()).into_iter().map(|tile| {
                    let action = tile.action;
                    let icon = tile.icon;
                    let title_key = tile.title_key;
                    let blurb_key = tile.blurb_key;
                    let id = tile.id;
                    view! {
                        <button type="button" class="cap-tile" role="listitem"
                            data-testid=format!("cap-tile-{id}")
                            on:click=move |_| on_activate.call(action)>
                            <span class="cap-tile-icon" aria-hidden="true">{compose_icon(icon)}</span>
                            <span class="cap-tile-title">{move || t(locale.get(), title_key)}</span>
                            <span class="cap-tile-blurb">{move || t(locale.get(), blurb_key)}</span>
                        </button>
                    }
                }).collect_view()
            }}
        </div>
    }
}
