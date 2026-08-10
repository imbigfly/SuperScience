//! Specialists (专家): user-definable agent personas — instructions plus a
//! skill/MCP subset and a directly-bound model, selectable per session.
//! Stored as a JSON array under the `specialists` settings key (same pattern
//! as `model_profiles`). Built-ins are materialized on first read so user edits
//! to their model bindings persist like any other row.

use serde::{Deserialize, Serialize};
use superscience_store::Store;
use tauri::State;

pub const SPECIALISTS_KEY: &str = "specialists";
pub const SCIENTIFIC_ILLUSTRATOR_RUBRIC: &str = "\
你是「科学插画专家」。要把用户请求及相关项目/会话上下文变成成品科学图件资源，\
而不仅是绘图建议。\n\n\
绘图前先检查所引用的数据与文件。不得捏造测量值、标签、样本量或科学结论。\
数据驱动图请加载 `figure-style`；多面板图还要加载 `figure-composer`。\n\n\
仅支持两种输出模式。用户对格式或方法的明确选择优先级最高；仅当用户未指定时，\
才依据工具是否可用来决定：\n\
- 直接 SVG 模式：当用户明确要求 SVG、矢量、可编辑图件，或直接生成 SVG 时使用。\
用 `write`、Python 或 R，把真正可发表的图写成描述性的 `figures/*.svg` 文件。\
不要调用 `generate_image`，不要把请求改成 PNG，也不要仅因 `generate_image` \
本身只返回 PNG 就声称不支持 SVG。写好 SVG 后，将该 SVG 栅格化为 PNG 预览，\
用 `view_image` 检查预览，修正 SVG 源中的可见问题，再重新渲染并复检。\
重复此 SVG -> PNG 预览 -> SVG 修正循环，直到图清晰且无裁切。SVG 是主交付物；\
PNG 仅作质检预览。\n\
- PNG 图像模型模式：当用户明确要求 PNG、`gpt-image-2`、`generate_image` \
或图像模型生成时使用。调用 `generate_image`，传入一份完整、自洽的视觉说明，\
并保存描述性的 `figures/*.png`。若 `generate_image` 不可用，说明需要配置\
图像生成模型；不要在用户明确要求 PNG 或图像模型时静默改用 SVG。\n\
- 当用户既未指定格式也未指定方法时：若 `generate_image` 可用，使用 PNG \
图像模型模式；否则使用直接 SVG 模式（含 SVG -> PNG 预览 -> SVG 修正循环），\
并交付 SVG。\n\n\
保持文字可读，使用色盲友好编码，并区分观测数据与概念示意。最后给出简明说明，\
并用项目相对路径的 Markdown 图片链接嵌入已保存的图件。";

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
    /// "" = follow the active model; dangling ids fall back to active too.
    #[serde(default)]
    pub model_id: String,
    /// Reviewer-only backend selection. `None` preserves the legacy behavior:
    /// use `model_id`, falling back to the active HTTP model. Other specialist
    /// personas continue to run inside SuperScience's native agent loop.
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
        name: "审阅专家".into(),
        icon: "review".into(),
        color: "clay".into(),
        description: "追溯会话记录，报告捏造结果、幻觉事实或偏离计划的问题。".into(),
        instructions: crate::review::REVIEWER_RUBRIC.into(),
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
        name: "检索专家".into(),
        icon: "search".into(),
        color: "clay".into(),
        description: "并行检索项目会话，并返回精炼且带出处的证据。".into(),
        instructions: crate::project_reader::READER_RUBRIC.into(),
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
        name: "科学插画专家".into(),
        icon: "image".into(),
        color: "clay".into(),
        description: "根据请求与项目上下文创建可发表的科学图件。".into(),
        instructions: SCIENTIFIC_ILLUSTRATOR_RUBRIC.into(),
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec!["figure-composer".into(), "figure-style".into()]),
        connectors: Some(vec![]),
        builtin: true,
    }
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

/// Load the list, materializing builtins if absent. Builtin name, description,
/// and instructions are always re-pinned to the compiled defaults so updates
/// ship without a settings migration.
pub async fn ensure(store: &Store) -> Vec<Specialist> {
    let mut list = load_raw(store).await;
    match list.iter_mut().find(|s| s.id == "reviewer") {
        Some(r) => {
            let fresh = builtin_reviewer();
            r.builtin = true;
            r.name = fresh.name;
            r.description = fresh.description;
            r.instructions = fresh.instructions;
        }
        None => list.insert(0, builtin_reviewer()),
    }
    match list.iter_mut().find(|s| s.id == "reader") {
        Some(reader) => {
            let fresh = builtin_reader();
            reader.builtin = true;
            reader.name = fresh.name;
            reader.description = fresh.description;
            reader.instructions = fresh.instructions;
            reader.review_backend = None;
            reader.skills = Some(vec![]);
            reader.connectors = Some(vec![]);
        }
        None => list.insert(1.min(list.len()), builtin_reader()),
    }
    match list.iter_mut().find(|s| s.id == "scientific_illustrator") {
        Some(illustrator) => {
            let fresh = builtin_scientific_illustrator();
            illustrator.builtin = true;
            illustrator.name = fresh.name;
            illustrator.description = fresh.description;
            illustrator.instructions = fresh.instructions;
            illustrator.review_backend = None;
            illustrator.skills = Some(vec!["figure-composer".into(), "figure-style".into()]);
            illustrator.connectors = Some(vec![]);
        }
        None => list.insert(2.min(list.len()), builtin_scientific_illustrator()),
    }
    list
}

pub async fn get(store: &Store, id: &str) -> Option<Specialist> {
    ensure(store).await.into_iter().find(|s| s.id == id)
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

/// Create (empty id) or update (existing id). Builtin rows keep their
/// compiled instructions and can never lose `builtin`.
pub async fn upsert(store: &Store, mut spec: Specialist) -> Result<Vec<Specialist>, String> {
    if spec.name.trim().is_empty() {
        return Err("Specialist name is required.".into());
    }
    let mut list = ensure(store).await;
    if spec.id.trim().is_empty() {
        spec.id = fresh_id(&list);
    }
    if let Some(existing) = list.iter_mut().find(|s| s.id == spec.id) {
        if existing.builtin {
            spec.builtin = true;
            spec.instructions = existing.instructions.clone();
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
        }
        *existing = spec;
    } else {
        spec.builtin = false;
        list.push(spec);
    }
    save_raw(store, &list).await?;
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

/// LLM config for a specialist: its bound profile, or the active-model chain
/// when unbound/dangling (soft fallback — personas are not hard capabilities).
pub async fn specialist_llm(
    store: &Store,
    spec: &Specialist,
) -> (String, String, String, String, u64, String) {
    if !spec.model_id.trim().is_empty() {
        if let Some(cfg) = crate::models::profile_llm(store, &spec.model_id).await {
            return cfg;
        }
    }
    let (provider, api_url, model, api_key) = crate::load_settings(store).await;
    let (max_tokens, reasoning_effort) = crate::models::active_llm_advanced(store).await;
    (
        provider,
        api_url,
        model,
        api_key,
        max_tokens,
        reasoning_effort,
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
    fn illustrator_rubric_gives_explicit_svg_requests_priority() {
        let rubric = SCIENTIFIC_ILLUSTRATOR_RUBRIC;
        let svg_rule = rubric
            .find("当用户明确要求 SVG")
            .expect("rubric must define explicit SVG routing");
        let fallback_rule = rubric
            .find("当用户既未指定格式也未指定方法时")
            .expect("rubric must define the tool-availability fallback");

        assert!(svg_rule < fallback_rule);
        assert!(rubric.contains("用户对格式或方法的明确选择优先级最高"));
        assert!(rubric.contains("不要调用 `generate_image`"));
        assert!(rubric.contains("将该 SVG 栅格化为 PNG 预览"));
        assert!(rubric.contains("用 `view_image` 检查预览"));
        assert!(rubric.contains("SVG -> PNG 预览 -> SVG 修正循环"));
        assert!(rubric.contains("SVG 是主交付物"));
        assert!(rubric.contains("明确要求 PNG"));
        assert!(rubric.contains("不要") && rubric.contains("静默改用 SVG"));
    }

    #[test]
    fn builtin_specialists_use_chinese_defaults() {
        let reviewer = builtin_reviewer();
        assert_eq!(reviewer.name, "审阅专家");
        assert!(reviewer.description.contains("追溯会话记录"));
        assert!(reviewer.instructions.contains("审阅专家"));

        let reader = builtin_reader();
        assert_eq!(reader.name, "检索专家");
        assert!(reader.description.contains("并行检索"));
        assert!(reader.instructions.contains("检索专家"));

        let illustrator = builtin_scientific_illustrator();
        assert_eq!(illustrator.name, "科学插画专家");
        assert!(illustrator.description.contains("科学图件"));
        assert!(illustrator.instructions.contains("科学插画专家"));
    }

    async fn test_store() -> (superscience_store::Store, std::path::PathBuf) {
        let tmp =
            std::env::temp_dir().join(format!("superscience_spec_{}.sqlite", uuid::Uuid::new_v4()));
        (superscience_store::Store::open(&tmp).await.unwrap(), tmp)
    }

    #[tokio::test]
    async fn ensure_materializes_builtin_specialists_once() {
        let (store, tmp) = test_store().await;
        let list = ensure(&store).await;
        assert_eq!(list.len(), 3);
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
        // Second read does not duplicate the built-ins.
        assert_eq!(ensure(&store).await.len(), 3);
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
        // Editing the builtin keeps instructions but accepts a model change.
        let mut r = get(&store, "reviewer").await.unwrap();
        r.instructions = "haha".into();
        r.model_id = "m2".into();
        let list = upsert(&store, r).await.unwrap();
        let r = list.iter().find(|s| s.id == "reviewer").unwrap();
        assert_eq!(r.instructions, crate::review::REVIEWER_RUBRIC);
        assert_eq!(r.model_id, "m2");

        let mut reader = get(&store, "reader").await.unwrap();
        reader.instructions = "replace rubric".into();
        reader.model_id = "cheap".into();
        reader.skills = None;
        let list = upsert(&store, reader).await.unwrap();
        let reader = list
            .iter()
            .find(|specialist| specialist.id == "reader")
            .unwrap();
        assert_eq!(reader.instructions, crate::project_reader::READER_RUBRIC);
        assert_eq!(reader.model_id, "cheap");
        assert_eq!(reader.skills, Some(vec![]));

        let mut illustrator = get(&store, "scientific_illustrator").await.unwrap();
        illustrator.instructions = "replace rubric".into();
        illustrator.skills = None;
        let list = upsert(&store, illustrator).await.unwrap();
        let illustrator = list
            .iter()
            .find(|specialist| specialist.id == "scientific_illustrator")
            .unwrap();
        assert_eq!(illustrator.instructions, SCIENTIFIC_ILLUSTRATOR_RUBRIC);
        assert_eq!(
            illustrator.skills,
            Some(vec!["figure-composer".into(), "figure-style".into()])
        );
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
        let (provider, api_url, model, _key, _mt, _re) = specialist_llm(&store, &spec).await;
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
        store
            .create_frame("f1", "p1", "SUPERSCIENCE", "m")
            .await
            .unwrap();
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
        let (_p, _u, model, _k, _mt, _re) = specialist_llm(&store, &spec).await;
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
