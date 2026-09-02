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
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec!["figure-composer".into(), "figure-style".into()]),
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
        model_id: String::new(),
        review_backend: None,
        skills: Some(vec!["handwriting-extract".into()]),
        connectors: Some(vec![]),
        builtin: true,
    }
}

fn pin_handwriting_extract(spec: &mut Specialist) {
    spec.builtin = true;
    spec.instructions = HANDWRITING_EXTRACT_RUBRIC.into();
    spec.review_backend = None;
    spec.skills = Some(vec!["handwriting-extract".into()]);
    spec.connectors = Some(vec![]);
    // Chat follows the session model from Settings → Models. Analysis and
    // calibration are separate profile picks, not a second API-key store.
    spec.model_id.clear();
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

/// Load the list, materializing builtins if absent. Builtin instructions are
/// always re-pinned to their compiled rubrics so improvements ship without a
/// settings migration.
pub async fn ensure(store: &Store) -> Vec<Specialist> {
    let mut list = load_raw(store).await;
    match list.iter_mut().find(|s| s.id == "reviewer") {
        Some(r) => {
            r.builtin = true;
            r.instructions = crate::review::REVIEWER_RUBRIC.into();
        }
        None => list.insert(0, builtin_reviewer()),
    }
    match list.iter_mut().find(|s| s.id == "reader") {
        Some(reader) => {
            reader.builtin = true;
            reader.instructions = crate::project_reader::READER_RUBRIC.into();
            reader.review_backend = None;
            reader.skills = Some(vec![]);
            reader.connectors = Some(vec![]);
        }
        None => list.insert(1.min(list.len()), builtin_reader()),
    }
    match list.iter_mut().find(|s| s.id == "scientific_illustrator") {
        Some(illustrator) => {
            illustrator.builtin = true;
            illustrator.instructions = SCIENTIFIC_ILLUSTRATOR_RUBRIC.into();
            illustrator.review_backend = None;
            illustrator.skills = Some(vec!["figure-composer".into(), "figure-style".into()]);
            illustrator.connectors = Some(vec![]);
        }
        None => list.insert(2.min(list.len()), builtin_scientific_illustrator()),
    }
    match list.iter_mut().find(|s| s.id == HANDWRITING_EXTRACT_ID) {
        Some(extractor) => pin_handwriting_extract(extractor),
        None => list.insert(3.min(list.len()), builtin_handwriting_extract()),
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
        } else if spec.id == HANDWRITING_EXTRACT_ID {
            spec.review_backend = None;
            spec.skills = Some(vec!["handwriting-extract".into()]);
            spec.connectors = Some(vec![]);
            spec.model_id.clear();
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
        assert_eq!(list.len(), 4);
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
        let extractor = &list[3];
        assert_eq!(extractor.id, HANDWRITING_EXTRACT_ID);
        assert!(extractor.builtin);
        assert_eq!(extractor.instructions, HANDWRITING_EXTRACT_RUBRIC);
        assert_eq!(
            extractor.skills.as_deref(),
            Some(&["handwriting-extract".to_string()][..])
        );
        assert!(extractor.model_id.is_empty());
        assert!(handwriting_extract_calibration_id(&store).await.is_none());
        // Second read does not duplicate the built-ins.
        assert_eq!(ensure(&store).await.len(), 4);

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
        assert!(remove(&store, HANDWRITING_EXTRACT_ID).await.is_err());
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

        let mut extractor = get(&store, HANDWRITING_EXTRACT_ID).await.unwrap();
        extractor.instructions = "replace rubric".into();
        extractor.model_id = "vl".into();
        extractor.skills = None;
        let list = upsert(&store, extractor).await.unwrap();
        let extractor = list
            .iter()
            .find(|specialist| specialist.id == HANDWRITING_EXTRACT_ID)
            .unwrap();
        assert_eq!(extractor.instructions, HANDWRITING_EXTRACT_RUBRIC);
        assert_eq!(extractor.model_id, "vl");
        assert_eq!(extractor.skills, Some(vec!["handwriting-extract".into()]));
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
