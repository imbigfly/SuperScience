use crate::dto::{BootstrapStatus, FeedbackAttachment};
use crate::i18n::Locale;
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

/// Non-secret account fields for feedback email diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FeedbackAccountSnapshot {
    pub logged_in: bool,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub group: Option<String>,
}

/// Non-secret preference / runtime fields for feedback diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FeedbackSystemConfig {
    pub locale: String,
    pub theme: String,
    pub notifications_enabled: bool,
    pub update_check_enabled: bool,
    pub max_iter: i64,
    pub auto_compact: bool,
    pub auto_continue: bool,
    pub follow_up_questions: bool,
    pub resume_last_session: bool,
    pub provider: String,
    pub model_id: String,
    pub has_api_key: bool,
}

/// Non-sensitive bootstrap metadata appended to feedback emails so support
/// never has to ask for version/OS, and never receives transcripts, API keys,
/// or absolute paths.
pub(crate) fn issue_report_chat_prompt(
    locale: Locale,
    bootstrap: Option<&BootstrapStatus>,
    model: &str,
    system_config: &FeedbackSystemConfig,
    account: &FeedbackAccountSnapshot,
) -> String {
    let empty = BootstrapStatus {
        skills_loaded: 0,
        python_ok: false,
        local_environment: None,
        mcp_catalog: 0,
        uv_ok: false,
        node_ok: false,
        npm_ok: false,
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

    let mut out = String::new();
    push_section(
        &mut out,
        if locale == Locale::Zh {
            "系统信息"
        } else {
            "System information"
        },
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "SuperScience 版本"
        } else {
            "SuperScience version"
        },
        &bootstrap.app_version,
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "OS / 架构"
        } else {
            "OS / architecture"
        },
        &format!("{} / {}", bootstrap.os, bootstrap.arch),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "模型配置"
        } else {
            "Model profile"
        },
        model,
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "启动耗时"
        } else {
            "Startup timings"
        },
        startup,
    );

    push_section(
        &mut out,
        if locale == Locale::Zh {
            "系统配置"
        } else {
            "System configuration"
        },
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "界面语言"
        } else {
            "Language"
        },
        &system_config.locale,
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "主题"
        } else {
            "Theme"
        },
        &system_config.theme,
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "桌面通知"
        } else {
            "Desktop notifications"
        },
        on_off(locale, system_config.notifications_enabled),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "自动检查更新"
        } else {
            "Update check"
        },
        on_off(locale, system_config.update_check_enabled),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "提供商"
        } else {
            "Provider"
        },
        blank_or(&system_config.provider),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "模型 ID"
        } else {
            "Model ID"
        },
        blank_or(&system_config.model_id),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "API Key"
        } else {
            "API key"
        },
        if system_config.has_api_key {
            if locale == Locale::Zh {
                "已配置"
            } else {
                "configured"
            }
        } else if locale == Locale::Zh {
            "未配置"
        } else {
            "not set"
        },
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "最大迭代"
        } else {
            "Max iterations"
        },
        &system_config.max_iter.to_string(),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "自动压缩上下文"
        } else {
            "Auto-compact"
        },
        on_off(locale, system_config.auto_compact),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "自动续写"
        } else {
            "Auto-continue"
        },
        on_off(locale, system_config.auto_continue),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "后续问题建议"
        } else {
            "Follow-up questions"
        },
        on_off(locale, system_config.follow_up_questions),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "恢复上次会话"
        } else {
            "Resume last session"
        },
        on_off(locale, system_config.resume_last_session),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "Skills / MCP"
        } else {
            "Skills / MCP"
        },
        &format!(
            "{} / {}",
            bootstrap.skills_loaded, bootstrap.mcp_catalog
        ),
    );
    push_kv(
        &mut out,
        if locale == Locale::Zh {
            "运行时就绪"
        } else {
            "Runtimes ready"
        },
        &runtime_summary(locale, bootstrap),
    );

    push_section(
        &mut out,
        if locale == Locale::Zh {
            "当前账户"
        } else {
            "Account"
        },
    );
    if account.logged_in {
        push_kv(
            &mut out,
            if locale == Locale::Zh {
                "登录状态"
            } else {
                "Signed in"
            },
            if locale == Locale::Zh { "已登录" } else { "yes" },
        );
        if let Some(user_id) = account.user_id {
            push_kv(
                &mut out,
                if locale == Locale::Zh {
                    "用户 ID"
                } else {
                    "User ID"
                },
                &user_id.to_string(),
            );
        }
        push_kv(
            &mut out,
            if locale == Locale::Zh {
                "用户名"
            } else {
                "Username"
            },
            blank_or(account.username.as_deref().unwrap_or("")),
        );
        push_kv(
            &mut out,
            if locale == Locale::Zh {
                "显示名"
            } else {
                "Display name"
            },
            blank_or(account.display_name.as_deref().unwrap_or("")),
        );
        push_kv(
            &mut out,
            if locale == Locale::Zh {
                "分组"
            } else {
                "Group"
            },
            blank_or(account.group.as_deref().unwrap_or("")),
        );
    } else {
        push_kv(
            &mut out,
            if locale == Locale::Zh {
                "登录状态"
            } else {
                "Signed in"
            },
            if locale == Locale::Zh {
                "未登录"
            } else {
                "no"
            },
        );
    }

    out
}

fn push_section(out: &mut String, title: &str) {
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str("## ");
    out.push_str(title);
    out.push('\n');
}

fn push_kv(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(": ");
    out.push_str(value.trim());
    out.push('\n');
}

fn on_off(locale: Locale, enabled: bool) -> &'static str {
    match (locale, enabled) {
        (Locale::Zh, true) => "开",
        (Locale::Zh, false) => "关",
        (_, true) => "on",
        (_, false) => "off",
    }
}

fn blank_or(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "—"
    } else {
        trimmed
    }
}

fn runtime_summary(locale: Locale, bootstrap: &BootstrapStatus) -> String {
    let mark = |ok: bool| -> &'static str {
        if ok {
            if locale == Locale::Zh {
                "✓"
            } else {
                "ok"
            }
        } else if locale == Locale::Zh {
            "×"
        } else {
            "no"
        }
    };
    format!(
        "Python {py} · uv {uv} · Node {node} · npm {npm} · sci {sci} · pixi {pixi} · R {r} · officecli {office} · sci key {sci_key}",
        py = mark(bootstrap.python_ok),
        uv = mark(bootstrap.uv_ok),
        node = mark(bootstrap.node_ok),
        npm = mark(bootstrap.npm_ok),
        sci = mark(bootstrap.sci_ok),
        pixi = mark(bootstrap.pixi_ok),
        r = mark(bootstrap.r_ok),
        office = mark(bootstrap.officecli_ok),
        sci_key = mark(bootstrap.sci_key_ok),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bootstrap(startup: &str) -> BootstrapStatus {
        BootstrapStatus {
            skills_loaded: 1,
            python_ok: true,
            local_environment: None,
            mcp_catalog: 1,
            uv_ok: true,
            node_ok: true,
            npm_ok: true,
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

    fn sample_config() -> FeedbackSystemConfig {
        FeedbackSystemConfig {
            locale: "zh".into(),
            theme: "system".into(),
            notifications_enabled: true,
            update_check_enabled: true,
            max_iter: 40,
            auto_compact: true,
            auto_continue: false,
            follow_up_questions: true,
            resume_last_session: false,
            provider: "openai".into(),
            model_id: "deepseek-chat".into(),
            has_api_key: true,
        }
    }

    #[test]
    fn diagnostics_include_startup_timings_without_paths() {
        let prompt = issue_report_chat_prompt(
            Locale::Zh,
            Some(&bootstrap("total=120ms store=90ms window_ready=600000ms")),
            "deepseek-chat",
            &sample_config(),
            &FeedbackAccountSnapshot {
                logged_in: true,
                user_id: Some(42),
                username: Some("demo".into()),
                display_name: Some("Demo User".into()),
                group: Some("default".into()),
            },
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
