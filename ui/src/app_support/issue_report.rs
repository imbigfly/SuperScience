use crate::dto::BootstrapStatus;
use crate::i18n::{tf, Locale};

pub(crate) const GITHUB_ISSUE_NEW: &str = "https://github.com/xuzhougeng/wisp-science/issues/new";

/// User message that kicks off an agent-guided GitHub issue draft. Non-sensitive
/// bootstrap metadata is embedded so the model never has to ask for version/OS
/// or read transcripts, API keys, or absolute paths.
pub(crate) fn issue_report_chat_prompt(
    locale: Locale,
    bootstrap: &BootstrapStatus,
    model: &str,
) -> String {
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
        "issue_report.chat_prompt",
        &[
            ("version", &bootstrap.app_version),
            ("os", &bootstrap.os),
            ("arch", &bootstrap.arch),
            ("model", model),
            ("startup", startup),
            ("repo", "xuzhougeng/wisp-science"),
            ("issue_base", GITHUB_ISSUE_NEW),
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
            npm_ok: true,
            sci_ok: true,
            pixi_ok: true,
            app_version: "0.34.0".into(),
            os: "windows".into(),
            arch: "x86_64".into(),
            workspace: "/mock/root".into(),
            startup: startup.into(),
            errors: vec![],
        }
    }

    #[test]
    fn chat_prompt_includes_startup_timings_without_paths() {
        let prompt = issue_report_chat_prompt(
            Locale::Zh,
            &bootstrap("total=120ms store=90ms window_ready=600000ms"),
            "deepseek-chat",
        );
        assert!(prompt.contains("total=120ms store=90ms window_ready=600000ms"));
        assert!(prompt.contains("0.34.0"));
        assert!(!prompt.contains("/mock/root"));
        assert!(prompt.contains("wisp-science"));
    }
}
