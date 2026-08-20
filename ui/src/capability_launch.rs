//! Fork-owned home-capability click handling.
//!
//! Upstream `wisp-science` does not have this module. Keep launch policy here so
//! a future merge that rewrites `main.rs` cannot drop the handler body.

use crate::app_support::{
    ensure_right_tab, focus_composer, invoke_new_session, js_error_text,
    replace_visible_transcript, ComposerReferenceChip,
};
use crate::bindings::{force_chat_bottom, invoke, invoke_checked};
use crate::capabilities_home::{CapabilityAction, CapabilityPanel};
use crate::demo_actions::refresh_demo_list;
use crate::dto::*;
use crate::i18n::{localize_backend, send_failed, t, tf, Locale};
use crate::publication::PublicationEvidenceSource;
use crate::research::refresh_research_graph;
use crate::NO_API_KEY_MARK;
use leptos::*;
use serde_wasm_bindgen::to_value;
use std::collections::{HashMap, HashSet};
use wasm_bindgen::JsValue;

const DIRECTOR_KICKOFF_PROMPT: &str = "caps.prompt.director_kickoff";

/// Signals and callbacks needed to start a capability from the home tiles.
#[derive(Clone, Copy)]
pub(crate) struct CapabilityLaunchCtx {
    pub locale: RwSignal<Locale>,
    pub busy: RwSignal<bool>,
    pub show_projects: RwSignal<bool>,
    pub show_capabilities: RwSignal<bool>,
    pub demo_mode: RwSignal<bool>,
    pub items: RwSignal<Vec<ChatItem>>,
    pub running: RwSignal<HashSet<String>>,
    pub status: RwSignal<String>,
    pub active_session: RwSignal<Option<String>>,
    pub session_specialist: RwSignal<Option<Specialist>>,
    pub attachments: RwSignal<Vec<ComposerAttachment>>,
    pub composer_references: RwSignal<Vec<ComposerReferenceChip>>,
    pub sel_artifact: RwSignal<usize>,
    pub right_tab: RwSignal<RightTab>,
    pub show_right: RwSignal<bool>,
    pub open_right_tabs: RwSignal<Vec<RightTab>>,
    pub models: RwSignal<Vec<ModelProfile>>,
    pub needs_api_key: RwSignal<bool>,
    pub transcripts: RwSignal<HashMap<String, Vec<ChatItem>>>,
    pub show_research_graph: RwSignal<bool>,
    pub research_graph: RwSignal<ResearchGraph>,
    pub publication_binding_source: RwSignal<Option<PublicationEvidenceSource>>,
    pub show_publication_workspace: RwSignal<bool>,
    pub project_open_error: RwSignal<Option<String>>,
    pub demos: RwSignal<Vec<DemoInfo>>,
    pub refresh_session_history: Callback<()>,
    pub open_settings: Callback<Option<String>>,
    pub open_project: Callback<(String, Option<String>)>,
}

pub(crate) fn capability_needs_project(action: &CapabilityAction) -> bool {
    matches!(
        action,
        CapabilityAction::NewChat
            | CapabilityAction::GuidedChat { .. }
            | CapabilityAction::InstallThenGuided { .. }
            | CapabilityAction::OpenPanel(CapabilityPanel::Files)
            | CapabilityAction::OpenPanel(CapabilityPanel::Graph)
            | CapabilityAction::OpenPanel(CapabilityPanel::Publication)
            | CapabilityAction::OpenPanel(CapabilityPanel::Agents)
    )
}

pub(crate) fn pick_default_project_id(list: &[ProjectSummary]) -> Option<String> {
    list.iter()
        .find(|project| project.id == "default")
        .or_else(|| list.first())
        .map(|project| project.id.clone())
}

pub(crate) fn guided_capability_message(
    prompt_key: &str,
    body: &str,
    skill_frame: Option<&str>,
    guided_frame: &str,
) -> String {
    if prompt_key == DIRECTOR_KICKOFF_PROMPT {
        body.to_string()
    } else if let Some(frame) = skill_frame {
        format!("{frame}\n\n{body}")
    } else {
        format!("{guided_frame}\n\n{body}")
    }
}

pub(crate) fn install(ctx: CapabilityLaunchCtx) -> Callback<CapabilityAction> {
    let pending = create_rw_signal(None::<CapabilityAction>);
    let start_guided = start_guided_capability_chat(ctx);
    let dispatch = dispatch_capability_action(ctx, start_guided);
    install_pending_flush(ctx, pending, dispatch);
    Callback::new(move |action: CapabilityAction| {
        if capability_needs_project(&action) && ctx.show_projects.get_untracked() {
            pending.set(Some(action));
            spawn_local(async move {
                let v = invoke("list_projects", JsValue::UNDEFINED).await;
                let Ok(list) = serde_wasm_bindgen::from_value::<Vec<ProjectSummary>>(v) else {
                    pending.set(None);
                    ctx.status
                        .set(t(ctx.locale.get(), "caps.need_project").into());
                    return;
                };
                let Some(id) = pick_default_project_id(&list) else {
                    pending.set(None);
                    ctx.status
                        .set(t(ctx.locale.get(), "caps.need_project").into());
                    return;
                };
                ctx.open_project.call((id, None));
            });
            return;
        }
        dispatch.call(action);
    })
}

fn install_pending_flush(
    ctx: CapabilityLaunchCtx,
    pending: RwSignal<Option<CapabilityAction>>,
    dispatch: Callback<CapabilityAction>,
) {
    create_effect(move |_| {
        if ctx.show_projects.get() {
            return;
        }
        let Some(action) = pending.get_untracked() else {
            return;
        };
        pending.set(None);
        set_timeout(
            move || dispatch.call(action),
            std::time::Duration::from_millis(350),
        );
    });
}

fn start_guided_capability_chat(
    ctx: CapabilityLaunchCtx,
) -> Callback<(&'static str, Option<&'static str>, Option<&'static str>)> {
    Callback::new(
        move |(prompt_key, skill, specialist): (
            &'static str,
            Option<&'static str>,
            Option<&'static str>,
        )| {
            if ctx.busy.get_untracked() {
                return;
            }
            ctx.show_capabilities.set(false);
            ctx.show_projects.set(false);
            ctx.attachments.set(vec![]);
            ctx.composer_references.set(vec![]);
            ctx.sel_artifact.set(0);
            ctx.right_tab.set(RightTab::Artifacts);
            let loc = ctx.locale.get();
            let body: String = t(loc, prompt_key).into();
            let skill_frame =
                skill.map(|name| tf(loc, "caps.prompt.socratic_frame", &[("skill", name)]));
            let guided_frame: String = t(loc, "caps.prompt.guided_frame").into();
            let text =
                guided_capability_message(prompt_key, &body, skill_frame.as_deref(), &guided_frame);
            let references = skill
                .map(|name| vec![ComposerReferenceArg::Skill { name: name.into() }])
                .unwrap_or_default();
            let turn_model = active_model_label(&ctx.models.get());
            ctx.items.set(vec![
                ChatItem::User(text.clone()),
                ChatItem::Assistant {
                    text: String::new(),
                    model: turn_model,
                    resources: Vec::new(),
                },
            ]);
            force_chat_bottom();
            spawn_local(async move {
                let id = match invoke_new_session().await {
                    Ok(id) => id,
                    Err(error) => {
                        ctx.status.set(send_failed(ctx.locale.get(), &error));
                        return;
                    }
                };
                if let Some(specialist_id) = specialist {
                    let arg = to_value(&serde_json::json!({
                        "frameId": id,
                        "id": specialist_id,
                    }))
                    .unwrap();
                    let _ = invoke_checked("set_session_specialist", arg).await;
                }
                ctx.active_session.set(Some(id.clone()));
                if specialist.is_some() {
                    let arg = to_value(&serde_json::json!({ "frameId": id })).unwrap();
                    let v = invoke("get_session_specialist", arg).await;
                    if ctx.active_session.get_untracked().as_deref() == Some(id.as_str()) {
                        ctx.session_specialist.set(
                            serde_wasm_bindgen::from_value::<Option<Specialist>>(v)
                                .ok()
                                .flatten(),
                        );
                    }
                }
                ctx.running.update(|running| {
                    running.insert(id.clone());
                });
                ctx.refresh_session_history.call(());
                let arg = to_value(&SendMessageArgs {
                    session_id: Some(id.clone()),
                    message: text,
                    attachments: vec![],
                    references,
                    resume: false,
                    acp_agent_id: None,
                    guide: None,
                    replace: None,
                })
                .unwrap();
                match invoke_checked("send_message", arg).await {
                    Ok(_) => {
                        ctx.running.update(|running| {
                            running.remove(&id);
                        });
                        ctx.refresh_session_history.call(());
                    }
                    Err(err) => {
                        let loc = ctx.locale.get();
                        let raw = js_error_text(err);
                        if raw.contains(NO_API_KEY_MARK) {
                            ctx.needs_api_key.set(true);
                        }
                        ctx.status.set(tf(
                            loc,
                            "status.send_failed",
                            &[("msg", &localize_backend(loc, &raw))],
                        ));
                        ctx.running.update(|running| {
                            running.remove(&id);
                        });
                    }
                }
            });
        },
    )
}

fn dispatch_capability_action(
    ctx: CapabilityLaunchCtx,
    start_guided: Callback<(&'static str, Option<&'static str>, Option<&'static str>)>,
) -> Callback<CapabilityAction> {
    Callback::new(move |action: CapabilityAction| {
        ctx.show_capabilities.set(false);
        match action {
            CapabilityAction::NewChat => {
                ctx.show_projects.set(false);
                ctx.demo_mode.set(false);
                ctx.attachments.set(vec![]);
                ctx.composer_references.set(vec![]);
                ctx.sel_artifact.set(0);
                ctx.right_tab.set(RightTab::Artifacts);
                spawn_local(async move {
                    let id = match invoke_new_session().await {
                        Ok(id) => id,
                        Err(error) => {
                            ctx.status.set(send_failed(ctx.locale.get(), &error));
                            return;
                        }
                    };
                    replace_visible_transcript(
                        ctx.active_session.get_untracked(),
                        None,
                        Vec::new(),
                        ctx.items,
                        ctx.transcripts,
                        ctx.running,
                    );
                    ctx.active_session.set(Some(id));
                    ctx.refresh_session_history.call(());
                    focus_composer();
                });
            }
            CapabilityAction::GuidedChat {
                prompt_key,
                skill,
                specialist,
            } => {
                start_guided.call((prompt_key, skill, specialist));
            }
            CapabilityAction::InstallThenGuided { prompt_key, skill } => {
                start_guided.call((prompt_key, Some(skill), None));
            }
            CapabilityAction::OpenSettings { section } => {
                ctx.open_settings.call(Some(section.to_string()));
            }
            CapabilityAction::OpenPanel(CapabilityPanel::Files) => {
                ctx.show_projects.set(false);
                ensure_right_tab(
                    RightTab::File,
                    ctx.show_right,
                    ctx.open_right_tabs,
                    ctx.right_tab,
                );
            }
            CapabilityAction::OpenPanel(CapabilityPanel::Graph) => {
                ctx.show_projects.set(false);
                ctx.show_research_graph.set(true);
                refresh_research_graph(ctx.research_graph);
            }
            CapabilityAction::OpenPanel(CapabilityPanel::Publication) => {
                ctx.show_projects.set(false);
                ctx.publication_binding_source.set(None);
                ctx.show_publication_workspace.set(true);
            }
            CapabilityAction::OpenPanel(CapabilityPanel::Agents) => {
                ctx.show_projects.set(false);
                ensure_right_tab(
                    RightTab::Agents,
                    ctx.show_right,
                    ctx.open_right_tabs,
                    ctx.right_tab,
                );
            }
            CapabilityAction::OpenDemo => {
                ctx.project_open_error.set(None);
                ctx.show_projects.set(false);
                ctx.demo_mode.set(true);
                ctx.items.set(vec![]);
                ctx.active_session.set(None);
                refresh_demo_list(ctx.demos);
            }
            CapabilityAction::ComingSoon | CapabilityAction::None => {}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(id: &str) -> ProjectSummary {
        ProjectSummary {
            id: id.into(),
            name: id.into(),
            description: String::new(),
            workspace_dir: String::new(),
            session_count: 0,
            artifact_count: 0,
            updated_at: 0,
            running_count: 0,
            needs_you_count: 0,
            sync_configured: false,
            last_synced_at: None,
        }
    }

    #[test]
    fn director_kickoff_skips_coaching_frame() {
        assert_eq!(
            guided_capability_message(DIRECTOR_KICKOFF_PROMPT, "intake", Some("frame"), "guided"),
            "intake"
        );
    }

    #[test]
    fn skill_chat_prefixes_socratic_frame() {
        assert_eq!(
            guided_capability_message(
                "caps.prompt.other",
                "body",
                Some("Ask 5 questions"),
                "guided"
            ),
            "Ask 5 questions\n\nbody"
        );
    }

    #[test]
    fn plain_guided_chat_uses_shared_frame() {
        assert_eq!(
            guided_capability_message("caps.prompt.other", "body", None, "Stay focused"),
            "Stay focused\n\nbody"
        );
    }

    #[test]
    fn settings_and_preview_tiles_do_not_need_a_project() {
        assert!(!capability_needs_project(&CapabilityAction::OpenSettings {
            section: "models"
        }));
        assert!(!capability_needs_project(&CapabilityAction::ComingSoon));
        assert!(!capability_needs_project(&CapabilityAction::OpenDemo));
        assert!(capability_needs_project(&CapabilityAction::NewChat));
        assert!(capability_needs_project(&CapabilityAction::GuidedChat {
            prompt_key: "caps.prompt.other",
            skill: None,
            specialist: None,
        }));
    }

    #[test]
    fn default_project_wins_then_first_row() {
        assert_eq!(
            pick_default_project_id(&[project("alpha"), project("default")]),
            Some("default".into())
        );
        assert_eq!(
            pick_default_project_id(&[project("alpha"), project("beta")]),
            Some("alpha".into())
        );
        assert_eq!(pick_default_project_id(&[]), None);
    }
}
