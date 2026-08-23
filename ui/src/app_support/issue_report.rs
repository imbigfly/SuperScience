use crate::dto::BootstrapStatus;
use crate::i18n::{tf, Locale};

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
}
