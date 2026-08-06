use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComposerSendAction {
    Normal,
    BranchNew,
    /// Guide choice: hand the message to the running task, which folds it in
    /// at its next loop iteration (ACP sessions fall back to plain queueing).
    GuideAppend,
    /// Guide choice: stop the running task, roll its unfinished work out of
    /// the model context, and send this message as the replacement.
    InterruptReplace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComposerPickerMode {
    Artifact,
    Session,
    Skill,
}

#[derive(Clone, PartialEq)]
pub(crate) enum ComposerPickerItem {
    Artifact(ArtifactInfo),
    Session(SessionSearchInfo),
    Project {
        id: String,
        name: String,
    },
    Skill(SkillRow),
    Workflow(WorkflowTemplate),
    Context {
        id: String,
        label: String,
    },
    Runtime {
        context_id: String,
        context_label: String,
        language: String,
    },
}

#[derive(Clone)]
pub(crate) enum ComposerReferenceChip {
    Artifact {
        id: String,
        name: String,
    },
    Session {
        id: String,
        title: String,
        project_name: String,
    },
    Project {
        id: String,
        name: String,
    },
    Skill {
        name: String,
    },
    Workflow {
        id: String,
        name: String,
    },
    Context {
        id: String,
        label: String,
    },
    Runtime {
        context_id: String,
        context_label: String,
        language: String,
    },
}

/// A passage attached from a preview. Unlike a plain blockquote, a source-aware
/// quote keeps the workspace path so the agent can act on "change this" instead
/// of treating the selection as an anonymous code sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ComposerQuote {
    pub(crate) text: String,
    pub(crate) source: Option<String>,
}

impl ComposerQuote {
    pub(crate) fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source: None,
        }
    }

    pub(crate) fn from_selection(text: impl Into<String>, source: Option<String>) -> Self {
        Self {
            text: text.into(),
            source,
        }
    }

    pub(crate) fn workspace_source(&self) -> Option<&str> {
        self.source.as_deref().filter(|source| {
            !source.starts_with("artifact:")
                && !source.starts_with("artifact-version:")
                && remote_file_path(source).is_none()
                && matches!(
                    file_kind(source),
                    Some(
                        "code" | "text" | "json" | "markdown" | "csv" | "html" | "fasta" | "smiles"
                    )
                )
        })
    }
}

/// True only when a selection has a real source and that source is the active
/// center preview. Comparing the two `Option`s directly would also make
/// `(None, None)` match, misclassifying an ordinary transcript selection.
pub(crate) fn selection_targets_center_file(
    source: Option<&str>,
    center_file: Option<&str>,
) -> bool {
    matches!((source, center_file), (Some(source), Some(center_file)) if source == center_file)
}

#[derive(Clone, PartialEq)]
pub(crate) enum CommandPaletteItem {
    Project(ProjectSummary),
    Artifact(ArtifactInfo),
    Session(SessionSearchInfo),
    Command(&'static str),
}

#[derive(Clone, PartialEq)]
pub(crate) struct CommandAction {
    pub(crate) id: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) title: String,
    pub(crate) group: String,
    pub(crate) shortcut: &'static str,
}

impl ComposerReferenceChip {
    pub(crate) fn key(&self) -> String {
        match self {
            Self::Artifact { id, .. } => format!("artifact:{id}"),
            Self::Session { id, .. } => format!("session:{id}"),
            Self::Project { id, .. } => format!("project:{id}"),
            Self::Skill { name } => format!("skill:{name}"),
            Self::Workflow { id, .. } => format!("workflow:{id}"),
            Self::Context { id, .. } => format!("context:{id}"),
            Self::Runtime {
                context_id,
                language,
                ..
            } => format!("runtime:{context_id}:{language}"),
        }
    }

    pub(crate) fn label(&self) -> String {
        match self {
            Self::Artifact { name, .. } | Self::Skill { name } | Self::Workflow { name, .. } => {
                name.clone()
            }
            Self::Session {
                title,
                project_name,
                ..
            } => format!("{project_name} / {title}"),
            Self::Project { name, .. } => format!("#project · {name}"),
            Self::Context { label, .. } => label.clone(),
            Self::Runtime {
                context_label,
                language,
                ..
            } => format!("{} · {context_label}", language_display(language)),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Artifact { .. } => "artifact",
            Self::Session { .. } => "session",
            Self::Project { .. } => "project",
            Self::Skill { .. } => "skill",
            Self::Workflow { .. } => "workflow",
            Self::Context { .. } => "context",
            Self::Runtime { .. } => "runtime",
        }
    }

    pub(crate) fn arg(&self) -> ComposerReferenceArg {
        match self {
            Self::Artifact { id, .. } => ComposerReferenceArg::Artifact { id: id.clone() },
            Self::Session { id, .. } => ComposerReferenceArg::Session { id: id.clone() },
            Self::Project { id, .. } => ComposerReferenceArg::Project { id: id.clone() },
            Self::Skill { name } => ComposerReferenceArg::Skill { name: name.clone() },
            Self::Workflow { id, .. } => ComposerReferenceArg::Workflow { id: id.clone() },
            Self::Context { id, .. } => ComposerReferenceArg::Context { id: id.clone() },
            Self::Runtime {
                context_id,
                language,
                ..
            } => ComposerReferenceArg::Runtime {
                context_id: context_id.clone(),
                language: language.clone(),
            },
        }
    }
}

pub(crate) fn composer_attachment_key(name: &str, idx: usize) -> String {
    format!("att-{idx}-{name}")
}

fn ready_attachment_key(path: &str) -> String {
    format!("path:{path}")
}

/// Attach an already-project-relative (or absolute native) file or directory path as a chip.
/// Returns false when the path was already attached.
pub(crate) fn attach_ready_path(
    attachments: RwSignal<Vec<ComposerAttachment>>,
    path: impl Into<String>,
) -> bool {
    let path = path.into();
    if path.trim().is_empty() {
        return false;
    }
    if attachments.get_untracked().iter().any(|attachment| {
        matches!(attachment, ComposerAttachment::Ready { path: existing, .. } if existing == &path)
    }) {
        return false;
    }
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path.as_str())
        .to_string();
    let key = ready_attachment_key(&path);
    attachments.update(|items| {
        items.push(ComposerAttachment::Ready { key, name, path });
    });
    true
}

pub(crate) fn parse_upload_results(v: JsValue) -> Vec<UploadFileResult> {
    if v.is_null() || v.is_undefined() {
        return vec![];
    }
    serde_wasm_bindgen::from_value(v).unwrap_or_default()
}

pub(crate) fn file_list_len(files: &JsValue) -> usize {
    js_sys::Reflect::get(files, &JsValue::from_str("length"))
        .ok()
        .and_then(|n| n.as_f64())
        .map(|n| n as usize)
        .unwrap_or(0)
}

pub(crate) fn begin_uploads(
    attachments: RwSignal<Vec<ComposerAttachment>>,
    uploading: RwSignal<bool>,
    count: usize,
) {
    if count == 0 {
        return;
    }
    attachments.update(|items| {
        for i in 0..count {
            items.push(ComposerAttachment::Uploading {
                key: format!("up-{}-{i}", js_sys::Date::now()),
                name: String::new(),
            });
        }
    });
    uploading.set(true);
}

pub(crate) fn finish_uploads(
    attachments: RwSignal<Vec<ComposerAttachment>>,
    uploading: RwSignal<bool>,
    results: Vec<UploadFileResult>,
) {
    uploading.set(false);
    attachments.update(|items| {
        merge_finished_uploads(items, results);
    });
}

fn merge_finished_uploads(items: &mut Vec<ComposerAttachment>, results: Vec<UploadFileResult>) {
    items.retain(|a| !matches!(a, ComposerAttachment::Uploading { .. }));
    let mut ready_paths = items
        .iter()
        .filter_map(|attachment| match attachment {
            ComposerAttachment::Ready { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    for result in results {
        let name = result
            .info
            .as_ref()
            .map(|i| i.name.clone())
            .or(result.filename.clone())
            .unwrap_or_else(|| "file".into());
        if result.ok {
            if let Some(info) = result.info {
                let path = info.path;
                if !ready_paths.insert(path.clone()) {
                    continue;
                }
                items.push(ComposerAttachment::Ready {
                    key: ready_attachment_key(&path),
                    name,
                    path,
                });
            }
        } else {
            items.push(ComposerAttachment::Error {
                key: composer_attachment_key(&name, items.len()),
                name,
                error: result.error.unwrap_or_else(|| "Upload failed".into()),
            });
        }
    }
}

// Closes the `<details class="settings-add-menu">` a menu button lives in,
// mirroring native `<select>`-style auto-close so the menu doesn't linger
// open after the user picks an option.
pub(crate) fn close_details_ancestor(ev: &web_sys::MouseEvent) {
    let el = ev
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok());
    if let Some(details) = el.and_then(|e| e.closest("details").ok().flatten()) {
        details.remove_attribute("open").ok();
    }
}

pub(crate) fn queue_uploads(
    attachments: RwSignal<Vec<ComposerAttachment>>,
    uploading: RwSignal<bool>,
    files: JsValue,
) {
    let count = file_list_len(&files);
    begin_uploads(attachments, uploading, count);
    spawn_local(async move {
        finish_uploads(
            attachments,
            uploading,
            parse_upload_results(upload_files(files).await),
        );
    });
}

pub(crate) fn upload_from_input(
    attachments: RwSignal<Vec<ComposerAttachment>>,
    uploading: RwSignal<bool>,
    input_id: &'static str,
) {
    uploading.set(true);
    spawn_local(async move {
        let v = upload_input_files(input_id).await;
        finish_uploads(attachments, uploading, parse_upload_results(v));
    });
}

pub(crate) fn upload_from_paste(
    attachments: RwSignal<Vec<ComposerAttachment>>,
    uploading: RwSignal<bool>,
    event: JsValue,
    count: usize,
) {
    begin_uploads(attachments, uploading, count);
    spawn_local(async move {
        let v = upload_pasted_images(event).await;
        finish_uploads(attachments, uploading, parse_upload_results(v));
    });
}

pub(crate) fn attachment_paths(items: &[ComposerAttachment]) -> Vec<String> {
    items
        .iter()
        .filter_map(|a| match a {
            ComposerAttachment::Ready { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect()
}

pub(crate) fn message_with_attachments(text: &str, paths: &[String]) -> String {
    let body = text.trim();
    if paths.is_empty() {
        return body.to_string();
    }
    let files = paths.join(", ");
    if body.is_empty() {
        format!("Uploaded files: {files}")
    } else {
        format!("{body}\n\nUploaded files: {files}")
    }
}

fn prompt_safe_source(source: &str) -> String {
    source
        .replace(['\r', '\n'], " ")
        .replace('`', "\\`")
        .trim()
        .to_string()
}

/// Prefix the message body with quoted-selection snippets as markdown
/// blockquotes. Workspace selections retain their target path and carry a
/// stable action hint, so change requests lead to a real tool edit rather than
/// a suggested replacement code block.
fn message_with_quotes_inner(
    text: &str,
    quotes: &[ComposerQuote],
    include_edit_instruction: bool,
) -> String {
    if quotes.is_empty() {
        return text.trim().to_string();
    }
    let mut out = String::new();
    for quote in quotes {
        if let Some(source) = quote.source.as_deref() {
            let source = prompt_safe_source(source);
            if quote.workspace_source().is_some() {
                out.push_str("Selected excerpt from workspace file `");
            } else {
                out.push_str("Selected excerpt from reference `");
            }
            out.push_str(&source);
            out.push_str("`:\n");
        }
        for line in quote.text.trim().lines() {
            out.push_str("> ");
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(text.trim());
    if include_edit_instruction {
        let mut editable_sources = quotes
            .iter()
            .filter_map(ComposerQuote::workspace_source)
            .map(prompt_safe_source)
            .collect::<Vec<_>>();
        editable_sources.sort();
        editable_sources.dedup();
        if !editable_sources.is_empty() {
            out.push_str("\n\nAI source-edit instruction: If the user requests a change, read the selected workspace file first, modify it directly with the edit tool for a focused in-place change (use write only for a whole-file replacement), and verify the saved result. Do not only return a replacement code block. Target file");
            out.push_str(if editable_sources.len() == 1 {
                ": `"
            } else {
                "s: `"
            });
            out.push_str(&editable_sources.join("`, `"));
            out.push('`');
        }
    }
    out.trim_end().to_string()
}

pub(crate) fn message_with_quotes(text: &str, quotes: &[ComposerQuote]) -> String {
    message_with_quotes_inner(text, quotes, true)
}

/// Side chat is intentionally read-only. It still needs the source label for
/// useful context, but must not inherit the main composer's file-edit command.
pub(crate) fn message_with_read_only_quotes(text: &str, quotes: &[ComposerQuote]) -> String {
    message_with_quotes_inner(text, quotes, false)
}

/// Chip label for a quoted selection: first line, capped at 40 chars.
pub(crate) fn quote_label(text: &str) -> String {
    let line = text.trim().lines().next().unwrap_or_default();
    let mut label: String = line.chars().take(40).collect();
    if line.chars().count() > 40 {
        label.push('…');
    }
    label
}

/// Build the persisted user-facing turn. Reference labels are deliberately
/// kept in the message alongside upload paths: the backend still receives the
/// typed reference ids separately, while a reloaded transcript retains enough
/// information for the UI to rebuild its attachment cards.
pub(crate) fn message_with_composer_context(
    text: &str,
    paths: &[String],
    references: &[ComposerReferenceChip],
    quotes: &[ComposerQuote],
) -> String {
    let mut message = message_with_attachments(&message_with_quotes(text, quotes), paths);
    let mut artifacts = Vec::new();
    let mut sessions = Vec::new();
    let mut projects = Vec::new();
    let mut skills = Vec::new();
    let mut workflows = Vec::new();
    let mut contexts = Vec::new();
    let mut runtimes = Vec::new();
    for reference in references {
        match reference {
            ComposerReferenceChip::Artifact { name, .. } => artifacts.push(name.clone()),
            ComposerReferenceChip::Session {
                title,
                project_name,
                ..
            } => sessions.push(format!("{project_name} / {title}")),
            ComposerReferenceChip::Project { name, .. } => projects.push(name.clone()),
            ComposerReferenceChip::Skill { name } => skills.push(name.clone()),
            ComposerReferenceChip::Workflow { name, .. } => workflows.push(name.clone()),
            ComposerReferenceChip::Context { label, .. } => contexts.push(label.clone()),
            ComposerReferenceChip::Runtime { .. } => runtimes.push(reference.label()),
        }
    }
    for (label, values) in [
        ("Attached artifacts", artifacts),
        ("Attached sessions", sessions),
        ("Project context", projects),
        ("Selected skills", skills),
        ("Selected workflows", workflows),
        ("Target environments", contexts),
        ("Target runtimes", runtimes),
    ] {
        if values.is_empty() {
            continue;
        }
        if !message.is_empty() {
            message.push_str("\n\n");
        }
        message.push_str(label);
        message.push_str(": ");
        message.push_str(&values.join(", "));
    }
    message
}

fn utf16_to_byte_index(text: &str, wanted: usize) -> Option<usize> {
    let mut utf16 = 0;
    for (byte, ch) in text.char_indices() {
        if utf16 == wanted {
            return Some(byte);
        }
        utf16 += ch.len_utf16();
        if utf16 > wanted {
            return None;
        }
    }
    (utf16 == wanted).then_some(text.len())
}

/// Return the active composer token ending at the textarea's UTF-16 caret.
/// ASCII word boundaries suppress email, URL, path, and fraction false positives;
/// CJK text may directly precede a trigger because it normally has no spaces.
pub(crate) fn active_composer_trigger(
    text: &str,
    caret_utf16: usize,
) -> Option<(usize, usize, ComposerPickerMode, String)> {
    let end = utf16_to_byte_index(text, caret_utf16)?;
    let (at, trigger) = text[..end]
        .char_indices()
        .rev()
        .find(|(_, c)| matches!(c, '@' | '#' | '/'))?;
    if let Some(previous) = text[..at].chars().next_back() {
        let embedded_ascii = previous.is_ascii_alphanumeric() || previous == '_';
        let path_or_url = trigger == '/' && matches!(previous, ':' | '/' | '\\' | '.');
        if embedded_ascii || path_or_url {
            return None;
        }
    }
    let query = &text[at + trigger.len_utf8()..end];
    if query.chars().any(char::is_whitespace) {
        return None;
    }
    let mode = match trigger {
        '@' => ComposerPickerMode::Artifact,
        '#' => ComposerPickerMode::Session,
        '/' => ComposerPickerMode::Skill,
        _ => return None,
    };
    Some((at, end, mode, query.to_string()))
}

pub(crate) fn scroll_picker_item(selector: &str, index: usize) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(items) = document.query_selector_all(selector) else {
        return;
    };
    if let Some(item) = items.item(index as u32) {
        item.unchecked_into::<web_sys::Element>().scroll_into_view();
    }
}

#[cfg(test)]
mod mention_tests {
    use super::{active_composer_trigger, ComposerPickerMode};

    #[test]
    fn detects_trigger_at_the_caret() {
        assert!(
            matches!(active_composer_trigger("look at @qc", 11), Some((8, 11, ComposerPickerMode::Artifact, q)) if q == "qc")
        );
        assert!(
            matches!(active_composer_trigger("#old", 4), Some((0, 4, ComposerPickerMode::Session, q)) if q == "old")
        );
        assert!(
            matches!(active_composer_trigger("/boltz", 6), Some((0, 6, ComposerPickerMode::Skill, q)) if q == "boltz")
        );
    }

    #[test]
    fn detects_trigger_before_existing_text_and_after_cjk() {
        let text = "比较#Current已有结果";
        let caret = "比较#Current".encode_utf16().count();
        assert!(
            matches!(active_composer_trigger(text, caret), Some((at, end, ComposerPickerMode::Session, q))
                if at == "比较".len() && end == "比较#Current".len() && q == "Current")
        );

        let text = "🧬@";
        assert!(
            matches!(active_composer_trigger(text, text.encode_utf16().count()), Some((4, 5, ComposerPickerMode::Artifact, q)) if q.is_empty())
        );
        assert_eq!(active_composer_trigger(text, 1), None);
    }

    #[test]
    fn ignores_literal_or_finished_tokens() {
        for text in ["no trigger", "email a@b.com", "https:/", "foo/", "1/2"] {
            assert_eq!(
                active_composer_trigger(text, text.encode_utf16().count()),
                None
            );
        }
        let text = "@qc then more";
        assert_eq!(
            active_composer_trigger(text, text.encode_utf16().count()),
            None
        );
    }
}

#[cfg(test)]
mod upload_attachment_tests {
    use super::{merge_finished_uploads, ready_attachment_key};
    use crate::dto::{ArtifactInfo, ComposerAttachment, UploadFileResult};

    fn ok_result(path: &str) -> UploadFileResult {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_string();
        UploadFileResult {
            ok: true,
            info: Some(ArtifactInfo {
                id: "artifact-1".into(),
                name: name.clone(),
                kind: "file".into(),
                path: path.into(),
                ts: 0,
                project_id: None,
                project_name: None,
                session_id: None,
                session_title: None,
                size_bytes: None,
                origin: None,
            }),
            filename: Some(name),
            error: None,
        }
    }

    #[test]
    fn merge_finished_uploads_dedupes_duplicate_ready_paths() {
        let mut items = vec![ComposerAttachment::Uploading {
            key: "up-1".into(),
            name: String::new(),
        }];

        merge_finished_uploads(
            &mut items,
            vec![
                ok_result("uploads/s41467-026-73270-2_reference.pdf"),
                ok_result("uploads/s41467-026-73270-2_reference.pdf"),
            ],
        );

        assert_eq!(items.len(), 1);
        assert!(matches!(
            &items[0],
            ComposerAttachment::Ready { path, .. }
                if path == "uploads/s41467-026-73270-2_reference.pdf"
        ));
    }

    #[test]
    fn merge_finished_uploads_keeps_existing_ready_path_unique() {
        let path = "uploads/existing.pdf";
        let mut items = vec![ComposerAttachment::Ready {
            key: ready_attachment_key(path),
            name: "existing.pdf".into(),
            path: path.into(),
        }];

        merge_finished_uploads(&mut items, vec![ok_result(path)]);

        assert_eq!(items.len(), 1);
        assert!(matches!(
            &items[0],
            ComposerAttachment::Ready { path, .. } if path == "uploads/existing.pdf"
        ));
    }
}
