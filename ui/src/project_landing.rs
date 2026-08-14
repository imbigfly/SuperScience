use crate::app_support::{CenterFileTab, ProjectsScreen};
use crate::bindings::invoke;
use crate::capabilities_home::CapabilityAction;
use crate::dto::*;
use crate::i18n::Locale;
use leptos::*;
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub(super) struct ProjectLandingState {
    pub(super) show_projects: RwSignal<bool>,
    pub(super) demo_mode: RwSignal<bool>,
    pub(super) items: RwSignal<Vec<ChatItem>>,
    pub(super) active_session: RwSignal<Option<String>>,
    pub(super) project_open_error: RwSignal<Option<String>>,
    pub(super) demos: RwSignal<Vec<DemoInfo>>,
    pub(super) modal_artifact: RwSignal<Option<(String, String, String)>>,
    pub(super) locale: RwSignal<Locale>,
    pub(super) running: RwSignal<HashSet<String>>,
    pub(super) approval_pending: RwSignal<HashSet<String>>,
    pub(super) sync_actions_available: RwSignal<bool>,
    pub(super) command_palette_open: RwSignal<bool>,
    pub(super) project_transfer: RwSignal<Option<ProjectTransferProgress>>,
    pub(super) privacy_mode_active: RwSignal<bool>,
    pub(super) privacy_hidden_project_ids: RwSignal<HashSet<String>>,
    pub(super) active_demo_id: RwSignal<Option<String>>,
    pub(super) center_files: RwSignal<Vec<CenterFileTab>>,
    pub(super) center_file: RwSignal<Option<String>>,
    pub(super) show_right: RwSignal<bool>,
}
#[component]
pub(super) fn ProjectLanding(
    state: ProjectLandingState,
    open_project: Callback<String>,
    open_project_session: Callback<(String, String)>,
    open_scratch: Callback<()>,
    open_settings: Callback<Option<String>>,
    open_library: Callback<()>,
    on_capability_action: Callback<CapabilityAction>,
    open_project_export: Callback<(String, String)>,
    theme_mode: RwSignal<String>,
    tctoken_session: RwSignal<crate::user_center::TctokenSession>,
    open_user_center: Callback<()>,
) -> impl IntoView {
    let ProjectLandingState {
        show_projects,
        demo_mode,
        items,
        active_session,
        project_open_error,
        demos,
        modal_artifact,
        locale,
        running,
        approval_pending,
        sync_actions_available,
        command_palette_open,
        project_transfer,
        privacy_mode_active,
        privacy_hidden_project_ids,
        active_demo_id,
        center_files,
        center_file,
        show_right,
    } = state;

    move || {
        show_projects.get().then(|| {
            let on_open_demo = Callback::new(move |_: ()| {
                project_open_error.set(None);
                show_projects.set(false);
                demo_mode.set(true);
                active_demo_id.set(None);
                items.set(vec![]);
                active_session.set(None);
                center_files.set(vec![]);
                center_file.set(None);
                show_right.set(false);
                let loc = locale.get_untracked().code().to_string();
                spawn_local(async move {
                    let v = invoke(
                        "list_demos",
                        serde_wasm_bindgen::to_value(&serde_json::json!({ "locale": loc })).unwrap(),
                    )
                    .await;
                    if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<DemoInfo>>(v) {
                        demos.set(list);
                    }
                });
            });
            let on_open_artifact =
                Callback::new(move |(path, name, kind): (String, String, String)| {
                    modal_artifact.set(Some((path, name, kind)));
                });
            let on_open_settings = Callback::new(move |_: ()| open_settings.call(None));
            view! {
                <ProjectsScreen
                    locale=locale
                    running=running
                    approval_pending=approval_pending.read_only()
                    sync_actions_available=sync_actions_available.read_only()
                    open_error=project_open_error
                    on_open=open_project
                    on_open_session=open_project_session
                    on_open_artifact=on_open_artifact
                    on_open_settings=on_open_settings
                    on_open_library=open_library
                    on_open_demo=on_open_demo
                    on_open_scratch=open_scratch
                    on_search=Callback::new(move |_| command_palette_open.set(true))
                    on_capability_action=on_capability_action
                    on_export_project=open_project_export
                    theme_mode=theme_mode
                    project_transfer=project_transfer
                    privacy_mode_active=privacy_mode_active
                    privacy_hidden_project_ids=privacy_hidden_project_ids
                    tctoken_session=tctoken_session
                    on_open_user_center=open_user_center
                />
            }
        })
    }
}
