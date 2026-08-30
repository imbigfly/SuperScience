use crate::dto::{BootstrapStatus, FeedbackAttachment};
use crate::i18n::{tf, Locale};
use serde::Deserialize;

pub(crate) const MAX_FEEDBACK_ATTACHMENTS: usize = 8;
pub(crate) const MAX_FEEDBACK_ATTACHMENT_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) const MAX_FEEDBACK_ATTACHMENTS_TOTAL_BYTES: u64 = 16 * 1024 * 1024;

const BLOCKED_ATTACHMENT_EXTS: &[&str] = &[
    "exe", "bat", "cmd", "com", "scr", "pif", "msi", "dll", "ps1", "vbs", "js",
];

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct FeedbackDraftFile {
    pub name: String,
    pub mime: String,
    #[serde(default, alias = "dataBase64")]
    pub data_base64: String,
    #[serde(default, alias = "previewUrl")]
    pub preview_url: String,
    #[serde(default)]
    pub size: u64,
}

impl FeedbackDraftFile {
    pub(crate) fn to_payload(&self) -> FeedbackAttachment {
        FeedbackAttachment {
            name: self.name.clone(),
            mime: self.mime.clone(),
            data_base64: self.data_base64.clone(),
        }
    }
}

pub(crate) enum FeedbackAttachError {
    TooMany,
    TooLarge,
    BlockedType,
}

fn attachment_ext(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

pub(crate) fn merge_feedback_attachments(
    current: &[FeedbackDraftFile],
    incoming: Vec<FeedbackDraftFile>,
) -> Result<Vec<FeedbackDraftFile>, FeedbackAttachError> {
    let mut next = current.to_vec();
    let mut total = next.iter().map(|f| f.size.max(0)).sum::<u64>();
    for file in incoming {
        if BLOCKED_ATTACHMENT_EXTS.contains(&attachment_ext(&file.name).as_str()) {
            return Err(FeedbackAttachError::BlockedType);
        }
        if file.size > MAX_FEEDBACK_ATTACHMENT_BYTES {
            return Err(FeedbackAttachError::TooLarge);
        }
        if next.len() >= MAX_FEEDBACK_ATTACHMENTS {
            return Err(FeedbackAttachError::TooMany);
        }
        total = total.saturating_add(file.size);
        if total > MAX_FEEDBACK_ATTACHMENTS_TOTAL_BYTES {
            return Err(FeedbackAttachError::TooLarge);
        }
        next.push(file);
    }
    Ok(next)
}

/// Non-sensitive bootstrap metadata appended to feedback emails so support
/// never has to ask for version/OS, and never receives transcripts, API keys,
/// or absolute paths.
pub(crate) fn issue_report_chat_prompt(
    locale: Locale,
    bootstrap: Option<&BootstrapStatus>,
    model: &str,
) -> String {
    let empty = BootstrapStatus {
        skills_loaded: 0,
        python_ok: false,
        python_initializing: false,
        mcp_catalog: 0,
        uv_ok: false,
        node_ok: false,
        sci_ok: false,
        pixi_ok: false,
        r_ok: false,
        officecli_ok: false,
        sci_key_ok: false,
        app_version: String::new(),
        os: String::new(),
        arch: String::new(),
        workspace: String::new(),
        startup: String::new(),
        errors: vec![],
    };
    let bootstrap = bootstrap.unwrap_or(&empty);
    let startup = bootstrap.startup.trim();
    let startup = if startup.is_empty() {
        if locale == Locale::Zh {
            "未记录"
        } else {
            "not recorded"
        }
    } else {
        startup
    };
    tf(
        locale,
        "issue_report.diagnostics",
        &[
            ("version", &bootstrap.app_version),
            ("os", &bootstrap.os),
            ("arch", &bootstrap.arch),
            ("model", model),
            ("startup", startup),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bootstrap(startup: &str) -> BootstrapStatus {
        BootstrapStatus {
            skills_loaded: 1,
            python_ok: true,
            python_initializing: false,
            mcp_catalog: 1,
            uv_ok: true,
            node_ok: true,
            sci_ok: true,
            pixi_ok: true,
            r_ok: true,
            officecli_ok: true,
            sci_key_ok: true,
            app_version: "0.34.0".into(),
            os: "windows".into(),
            arch: "x86_64".into(),
            workspace: "/mock/root".into(),
            startup: startup.into(),
            errors: vec![],
        }
    }

    #[test]
    fn diagnostics_include_startup_timings_without_paths() {
        let prompt = issue_report_chat_prompt(
            Locale::Zh,
            Some(&bootstrap("total=120ms store=90ms window_ready=600000ms")),
            "deepseek-chat",
        );
        assert!(prompt.contains("total=120ms store=90ms window_ready=600000ms"));
        assert!(prompt.contains("0.34.0"));
        assert!(prompt.contains("windows"));
        assert!(prompt.contains("deepseek-chat"));
        assert!(!prompt.contains("/mock/root"));
        assert!(!prompt.contains("GitHub"));
    }

    #[test]
    fn merge_rejects_blocked_and_oversize() {
        let blocked = merge_feedback_attachments(
            &[],
            vec![FeedbackDraftFile {
                name: "payload.exe".into(),
                mime: "application/octet-stream".into(),
                data_base64: "eA==".into(),
                preview_url: String::new(),
                size: 2,
            }],
        );
        assert!(matches!(blocked, Err(FeedbackAttachError::BlockedType)));

        let too_big = merge_feedback_attachments(
            &[],
            vec![FeedbackDraftFile {
                name: "shot.png".into(),
                mime: "image/png".into(),
                data_base64: "eA==".into(),
                preview_url: String::new(),
                size: MAX_FEEDBACK_ATTACHMENT_BYTES + 1,
            }],
        );
        assert!(matches!(too_big, Err(FeedbackAttachError::TooLarge)));
    }
}
