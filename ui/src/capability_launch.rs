//! Fork-owned home-capability click handling.
//!
//! Upstream `wisp-science` does not have this module. Keep launch policy here so
//! a future merge that rewrites `main.rs` cannot drop the handler body.

use crate::app_support::{
    ensure_right_tab, focus_composer, invoke_new_session, js_error_text,
    replace_visible_transcript, ComposerReferenceChip,
};
use crate::bindings::{force_chat_bottom, invoke, invoke_checked};
use crate::capabilities_home::{
    capability_catalog, title_key_for_capability_action, CapabilityAction, CapabilityPanel,
};
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
const DIRECTOR_RULES_KEY: &str = "caps.prompt.director_rules";
const HANDWRITING_EXTRACT_SKILL: &str = "handwriting-extract";
const CAPABILITY_TITLE_SEP: &str = " · ";
const CAPABILITY_TITLE_MAX_CHARS: usize = 80;

/// Sidebar / recent-session label for a chat started from a capability card.
pub(crate) fn capability_session_title(
    locale: Locale,
    prompt_key: &'static str,
    skill: Option<&'static str>,
    specialist: Option<&'static str>,
) -> Option<String> {
    let exact = CapabilityAction::GuidedChat {
        prompt_key,
        skill,
        specialist,
    };
    let key = title_key_for_capability_action(&exact).or_else(|| {
        capability_catalog()
            .iter()
            .find_map(|tile| match tile.action {
                CapabilityAction::GuidedChat {
                    prompt_key: tile_prompt,
                    skill: tile_skill,
                    specialist: tile_specialist,
                } if tile_prompt == prompt_key
                    && tile_skill == skill
                    && (specialist.is_none() || tile_specialist == specialist) =>
                {
                    Some(tile.title_key)
                }
                CapabilityAction::InstallThenGuided {
                    prompt_key: tile_prompt,
                    skill: tile_skill,
                } if tile_prompt == prompt_key
                    && Some(tile_skill) == skill
                    && specialist.is_none() =>
                {
                    Some(tile.title_key)
                }
                _ => None,
            })
    })?;
    let name = t(locale, key);
    let name = name.trim();
    if name.is_empty() || name == key {
        return None;
    }
    let raw_prompt = t(locale, prompt_key);
    let summary = if raw_prompt.trim().is_empty() || raw_prompt == prompt_key {
        String::new()
    } else {
        collapse_title_text(&raw_prompt)
    };
    Some(format_capability_session_title(name, &summary))
}

fn collapse_title_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_capability_session_title(name: &str, summary: &str) -> String {
    let name = name.trim();
    let summary = summary.trim();
    if summary.is_empty() {
        return name.to_string();
    }
    let summary = summary
        .strip_prefix(name)
        .unwrap_or(summary)
        .trim_start_matches([' ', '·', '-', '—', ':'])
        .trim();
    if summary.is_empty() {
        return name.to_string();
    }
    let prefix = format!("{name}{CAPABILITY_TITLE_SEP}");
    let budget = CAPABILITY_TITLE_MAX_CHARS.saturating_sub(prefix.chars().count());
    if budget == 0 {
        return name.to_string();
    }
    if summary.chars().count() <= budget {
        format!("{prefix}{summary}")
    } else {
        let take = budget.saturating_sub(1);
        let cut: String = summary.chars().take(take).collect();
        format!("{prefix}{cut}…")
    }
}

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
    pub open_runtime_setup: Callback<()>,
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

pub(crate) fn spec_key_for_prompt(prompt_key: &str) -> Option<&'static str> {
    match prompt_key {
        "caps.prompt.python_r" => Some("caps.prompt.python_r.spec"),
        "caps.prompt.bio_db" => Some("caps.prompt.bio_db.spec"),
        "caps.prompt.r_bioinfo_figure" => Some("caps.prompt.r_bioinfo_figure.spec"),
        "caps.prompt.knowledge" => Some("caps.prompt.knowledge.spec"),
        _ => None,
    }
}

/// Visible opener vs the message sent to the model (frame + optional spec + opener).
pub(crate) fn capability_launch_texts(
    locale: Locale,
    prompt_key: &str,
    skill: Option<&str>,
) -> (String, String) {
    let visible = t(locale, prompt_key);
    let spec = spec_key_for_prompt(prompt_key).and_then(|key| {
        let value = t(locale, key);
        if value.is_empty() || value == key {
            None
        } else {
            Some(value)
        }
    });
    if prompt_key == DIRECTOR_KICKOFF_PROMPT {
        let rules = t(locale, DIRECTOR_RULES_KEY);
        let sent = if rules.is_empty() || rules == DIRECTOR_RULES_KEY {
            visible.clone()
        } else {
            format!("{rules}\n\n{visible}")
        };
        return (visible, sent);
    }
    let skill_frame =
        skill.map(|name| tf(locale, "caps.prompt.socratic_frame", &[("skill", name)]));
    let guided_frame = t(locale, "caps.prompt.guided_frame");
    let body = match spec {
        Some(spec) => format!("{spec}\n\n{visible}"),
        None => visible.clone(),
    };
    let sent = guided_capability_message(prompt_key, &body, skill_frame.as_deref(), &guided_frame);
    (visible, sent)
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
            let session_title = capability_session_title(loc, prompt_key, skill, specialist);
            let (visible, sent) = capability_launch_texts(loc, prompt_key, skill);
            let references = skill
                .map(|name| vec![ComposerReferenceArg::Skill { name: name.into() }])
                .unwrap_or_default();
            let turn_model = active_model_label(&ctx.models.get());
            ctx.items.set(vec![
                ChatItem::User(visible),
                ChatItem::Assistant {
                    text: String::new(),
                    model: turn_model,
                    resources: Vec::new(),
                },
            ]);
            force_chat_bottom();
            spawn_local(async move {
                let handwriting_vision = if skill == Some(HANDWRITING_EXTRACT_SKILL) {
                    let v =
                        invoke("get_handwriting_extract_vision_model", JsValue::UNDEFINED).await;
                    serde_wasm_bindgen::from_value::<Option<String>>(v)
                        .ok()
                        .flatten()
                        .filter(|id| !id.trim().is_empty())
                } else {
                    None
                };
                let handwriting_calib = if skill == Some(HANDWRITING_EXTRACT_SKILL) {
                    let v = invoke(
                        "get_handwriting_extract_calibration_model",
                        JsValue::UNDEFINED,
                    )
                    .await;
                    serde_wasm_bindgen::from_value::<Option<String>>(v)
                        .ok()
                        .flatten()
                        .filter(|id| !id.trim().is_empty())
                } else {
                    None
                };
                if skill == Some(HANDWRITING_EXTRACT_SKILL)
                    && (handwriting_vision.is_none() || handwriting_calib.is_none())
                {
                    ctx.status.set(
                        t(
                            ctx.locale.get(),
                            "caps.skill.handwriting_extract.settings.required",
                        )
                        .into(),
                    );
                    ctx.items.set(Vec::new());
                    return;
                }
                let id = match invoke_new_session().await {
                    Ok(id) => id,
                    Err(error) => {
                        ctx.status.set(send_failed(ctx.locale.get(), &error));
                        return;
                    }
                };
                if let Some(title) = session_title {
                    let arg = to_value(&serde_json::json!({
                        "id": id,
                        "title": title,
                    }))
                    .unwrap();
                    let _ = invoke_checked("rename_session", arg).await;
                }
                if let Some(specialist_id) = specialist {
                    let arg = to_value(&serde_json::json!({
                        "frameId": id,
                        "id": specialist_id,
                    }))
                    .unwrap();
                    if let Err(err) = invoke_checked("set_session_specialist", arg).await {
                        let loc = ctx.locale.get();
                        let raw = js_error_text(err);
                        ctx.status.set(tf(
                            loc,
                            "status.send_failed",
                            &[("msg", &localize_backend(loc, &raw))],
                        ));
                        ctx.items.set(Vec::new());
                        return;
                    }
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
                if let Some(model_id) = handwriting_vision {
                    let arg = to_value(&serde_json::json!({
                        "frameId": id,
                        "modelId": model_id,
                    }))
                    .unwrap();
                    if let Err(err) = invoke_checked("set_frame_vision_model", arg).await {
                        let loc = ctx.locale.get();
                        let raw = js_error_text(err);
                        ctx.status.set(tf(
                            loc,
                            "status.send_failed",
                            &[("msg", &localize_backend(loc, &raw))],
                        ));
                        ctx.items.set(Vec::new());
                        return;
                    }
                }
                ctx.running.update(|running| {
                    running.insert(id.clone());
                });
                ctx.refresh_session_history.call(());
                let arg = to_value(&SendMessageArgs {
                    session_id: Some(id.clone()),
                    message: sent,
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
            CapabilityAction::OpenRuntimeSetup => {
                ctx.open_runtime_setup.call(());
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::strip_capability_coach;

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
    fn capability_card_session_title_prefixes_card_name_and_prompt_summary() {
        let zh = capability_session_title(
            Locale::Zh,
            "caps.skill.handwriting_extract.prompt",
            Some("handwriting-extract"),
            Some("handwriting_extract"),
        )
        .expect("handwriting title");
        assert!(zh.starts_with("手写数据提取 · "));
        assert!(!zh.contains("handwriting-extract"));
        assert!(zh.chars().count() <= CAPABILITY_TITLE_MAX_CHARS);

        let en = capability_session_title(
            Locale::En,
            "caps.skill.handwriting_extract.prompt",
            Some("handwriting-extract"),
            None,
        )
        .expect("handwriting title");
        assert!(en.starts_with("Handwritten data extract · "));
        assert!(!en.contains("handwriting-extract"));

        let director =
            capability_session_title(Locale::Zh, DIRECTOR_KICKOFF_PROMPT, None, None).unwrap();
        assert!(director.starts_with(&format!(
            "{}{CAPABILITY_TITLE_SEP}",
            t(Locale::Zh, "caps.tile.director.title")
        )));
        assert_eq!(
            capability_session_title(Locale::Zh, "caps.prompt.missing", None, None),
            None
        );
    }

    #[test]
    fn capability_session_title_format_truncates_and_skips_empty_summary() {
        assert_eq!(format_capability_session_title("选题引导", ""), "选题引导");
        assert_eq!(
            format_capability_session_title("选题引导", "盘点已有数据和资料"),
            "选题引导 · 盘点已有数据和资料"
        );
        let long = "字".repeat(120);
        let titled = format_capability_session_title("选题引导", &long);
        assert!(titled.starts_with("选题引导 · "));
        assert!(titled.ends_with('…'));
        assert_eq!(titled.chars().count(), CAPABILITY_TITLE_MAX_CHARS);
    }

    #[test]
    fn director_kickoff_skips_coaching_frame() {
        assert_eq!(
            guided_capability_message(DIRECTOR_KICKOFF_PROMPT, "intake", Some("frame"), "guided"),
            "intake"
        );
    }

    #[test]
    fn capability_launch_texts_hide_frame_from_visible() {
        let (visible, sent) = capability_launch_texts(
            Locale::Zh,
            "caps.skill.topic_coach.prompt",
            Some("topic-coach"),
        );
        assert!(!visible.contains("能力引导规则"));
        assert!(sent.contains("能力引导规则"));
        assert!(sent.contains(visible.trim()));
        assert_eq!(strip_capability_coach(&sent), visible.trim());

        let (visible, sent) = capability_launch_texts(
            Locale::En,
            "caps.skill.topic_coach.prompt",
            Some("topic-coach"),
        );
        assert!(!visible.contains("Capability coaching"));
        assert!(sent.contains("Capability coaching"));
        assert_eq!(strip_capability_coach(&sent), visible.trim());
    }

    #[test]
    fn director_launch_keeps_rules_off_the_opener() {
        let (visible, sent) = capability_launch_texts(Locale::Zh, DIRECTOR_KICKOFF_PROMPT, None);
        assert!(!visible.contains("硬性规则"));
        assert!(sent.starts_with("硬性规则"));
        assert!(sent.contains(&visible));
        assert_eq!(strip_capability_coach(&sent), visible.trim());

        let (visible, sent) = capability_launch_texts(Locale::En, DIRECTOR_KICKOFF_PROMPT, None);
        assert!(!visible.contains("Hard rules"));
        assert!(sent.starts_with("Hard rules"));
        assert_eq!(strip_capability_coach(&sent), visible.trim());
    }

    #[test]
    fn python_r_spec_is_sent_but_stripped_from_display() {
        let (visible, sent) = capability_launch_texts(Locale::Zh, "caps.prompt.python_r", None);
        assert!(!visible.contains("savefig"));
        assert!(sent.contains("savefig"));
        assert!(sent.contains("能力工具规格"));
        assert_eq!(strip_capability_coach(&sent), visible.trim());

        let (visible, sent) = capability_launch_texts(Locale::En, "caps.prompt.python_r", None);
        assert!(!visible.contains("savefig"));
        assert!(sent.contains("savefig"));
        assert!(sent.contains("Capability tool spec"));
        assert_eq!(strip_capability_coach(&sent), visible.trim());
    }

    #[test]
    fn knowledge_spec_is_sent_but_stripped_from_display() {
        let (visible, sent) = capability_launch_texts(Locale::Zh, "caps.prompt.knowledge", None);
        assert!(!visible.contains("knowledge_search"));
        assert!(sent.contains("knowledge_search"));
        assert!(sent.contains("能力工具规格"));
        assert!(sent.contains("能力引导规则") || sent.contains("Capability coaching"));
        assert_eq!(strip_capability_coach(&sent), visible.trim());

        let (visible, sent) = capability_launch_texts(Locale::En, "caps.prompt.knowledge", None);
        assert!(!visible.contains("knowledge_search"));
        assert!(sent.contains("knowledge_search"));
        assert!(sent.contains("Capability tool spec"));
        assert_eq!(strip_capability_coach(&sent), visible.trim());
    }

    #[test]
    fn orphan_capability_prompt_keys_are_removed() {
        for key in [
            "caps.prompt.literature",
            "caps.prompt.pdf_ppt",
            "caps.prompt.academic_research",
            "caps.prompt.nature_skills",
            "caps.prompt.academic_paper",
            "caps.prompt.scientific_figures",
            "caps.env_setup_prompt",
        ] {
            for locale in [Locale::En, Locale::Zh] {
                assert_eq!(
                    t(locale, key),
                    key,
                    "orphan {key} should not resolve in {:?}",
                    locale
                );
            }
        }
        assert_ne!(
            t(Locale::Zh, "caps.skill.academic_paper.prompt"),
            "caps.skill.academic_paper.prompt"
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
        assert!(!capability_needs_project(
            &CapabilityAction::OpenRuntimeSetup
        ));
        assert!(capability_needs_project(&CapabilityAction::NewChat));
        assert!(!capability_needs_project(
            &CapabilityAction::OpenRuntimeSetup
        ));
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
