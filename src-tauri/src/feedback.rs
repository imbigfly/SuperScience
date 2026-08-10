//! User feedback: load bundled SMTP settings and send support email.
//!
//! Non-secret settings live in `src-tauri/config/feedback.toml` (embedded at
//! compile time). The SMTP password comes from a local override file
//! `feedback.local.toml` that is gitignored and never shipped in source.
//! Unit tests cover parsing/merging only — they never open a real SMTP
//! connection.

use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;
use std::path::PathBuf;

const BUNDLED_FEEDBACK_CONFIG: &str = include_str!("../config/feedback.toml");
const LOCAL_OVERRIDE_NAME: &str = "feedback.local.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FeedbackConfig {
    pub feedback: FeedbackSection,
    pub smtp: SmtpSection,
    #[serde(default)]
    pub imap: Option<ImapSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FeedbackSection {
    pub to_email: String,
    pub from_email: String,
    #[serde(default = "default_from_name")]
    pub from_name: String,
    #[serde(default = "default_subject_prefix")]
    pub subject_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SmtpSection {
    pub host: String,
    /// Filled from `feedback.local.toml`; the bundled file leaves this empty.
    #[serde(default)]
    pub password: String,
    pub port_ssl: u16,
    pub port_starttls: u16,
    #[serde(default = "default_prefer_ssl")]
    pub prefer_ssl: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ImapSection {
    pub host: String,
    pub port_ssl: u16,
}

#[derive(Debug, Default, Deserialize)]
struct FeedbackOverride {
    #[serde(default)]
    feedback: Option<FeedbackSectionOverride>,
    #[serde(default)]
    smtp: Option<SmtpSectionOverride>,
    #[serde(default)]
    imap: Option<ImapSection>,
}

#[derive(Debug, Default, Deserialize)]
struct FeedbackSectionOverride {
    to_email: Option<String>,
    from_email: Option<String>,
    from_name: Option<String>,
    subject_prefix: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SmtpSectionOverride {
    host: Option<String>,
    password: Option<String>,
    port_ssl: Option<u16>,
    port_starttls: Option<u16>,
    prefer_ssl: Option<bool>,
}

fn default_from_name() -> String {
    "天成科研助手".into()
}

fn default_subject_prefix() -> String {
    "天成科研助手用户反馈".into()
}

fn default_prefer_ssl() -> bool {
    true
}

pub fn load_feedback_config() -> Result<FeedbackConfig, String> {
    let mut config = parse_feedback_config_base(BUNDLED_FEEDBACK_CONFIG)?;
    if let Some((path, raw)) = read_local_override()? {
        apply_feedback_override(&mut config, &raw).map_err(|error| {
            format!("invalid {}: {error}", path.display())
        })?;
    }
    validate_feedback_config(&config)?;
    Ok(config)
}

pub fn parse_feedback_config(raw: &str) -> Result<FeedbackConfig, String> {
    let config = parse_feedback_config_base(raw)?;
    validate_feedback_config(&config)?;
    Ok(config)
}

pub fn parse_feedback_config_base(raw: &str) -> Result<FeedbackConfig, String> {
    toml::from_str(raw).map_err(|error| format!("invalid feedback.toml: {error}"))
}

pub fn apply_feedback_override(config: &mut FeedbackConfig, raw: &str) -> Result<(), String> {
    let overlay: FeedbackOverride =
        toml::from_str(raw).map_err(|error| format!("invalid feedback override: {error}"))?;
    if let Some(feedback) = overlay.feedback {
        if let Some(value) = feedback.to_email {
            config.feedback.to_email = value;
        }
        if let Some(value) = feedback.from_email {
            config.feedback.from_email = value;
        }
        if let Some(value) = feedback.from_name {
            config.feedback.from_name = value;
        }
        if let Some(value) = feedback.subject_prefix {
            config.feedback.subject_prefix = value;
        }
    }
    if let Some(smtp) = overlay.smtp {
        if let Some(value) = smtp.host {
            config.smtp.host = value;
        }
        if let Some(value) = smtp.password {
            config.smtp.password = value;
        }
        if let Some(value) = smtp.port_ssl {
            config.smtp.port_ssl = value;
        }
        if let Some(value) = smtp.port_starttls {
            config.smtp.port_starttls = value;
        }
        if let Some(value) = smtp.prefer_ssl {
            config.smtp.prefer_ssl = value;
        }
    }
    if let Some(imap) = overlay.imap {
        config.imap = Some(imap);
    }
    Ok(())
}

fn validate_feedback_config(config: &FeedbackConfig) -> Result<(), String> {
    if config.feedback.to_email.trim().is_empty() {
        return Err("feedback.to_email is required".into());
    }
    if config.feedback.from_email.trim().is_empty() {
        return Err("feedback.from_email is required".into());
    }
    if config.smtp.host.trim().is_empty() {
        return Err("smtp.host is required".into());
    }
    if config.smtp.password.is_empty() {
        return Err(format!(
            "smtp.password is required. Copy src-tauri/config/{LOCAL_OVERRIDE_NAME}.example \
             to {LOCAL_OVERRIDE_NAME} (gitignored) and set smtp.password, or place the same \
             file under the app config directory."
        ));
    }
    if config.smtp.port_ssl == 0 || config.smtp.port_starttls == 0 {
        return Err("smtp ports must be non-zero".into());
    }
    Ok(())
}

/// Candidate locations for the gitignored local override.
pub fn local_override_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    paths.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("config")
            .join(LOCAL_OVERRIDE_NAME),
    );
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(
            config_dir
                .join("science.superscience")
                .join(LOCAL_OVERRIDE_NAME),
        );
    }
    if let Some(data_dir) = dirs::data_dir() {
        paths.push(
            data_dir
                .join("science.superscience")
                .join("superscience")
                .join(LOCAL_OVERRIDE_NAME),
        );
    }
    paths
}

fn read_local_override() -> Result<Option<(PathBuf, String)>, String> {
    for path in local_override_candidates() {
        match std::fs::read_to_string(&path) {
            Ok(raw) => return Ok(Some((path, raw))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "failed to read feedback override {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Ok(None)
}

pub fn build_feedback_subject(config: &FeedbackConfig, user_subject: Option<&str>) -> String {
    let custom = user_subject.map(str::trim).filter(|s| !s.is_empty());
    match custom {
        Some(subject) => format!("{} - {subject}", config.feedback.subject_prefix),
        None => config.feedback.subject_prefix.clone(),
    }
}

pub fn build_feedback_body(message: &str, diagnostics: Option<&str>) -> String {
    let mut body = message.trim().to_string();
    if let Some(diagnostics) = diagnostics.map(str::trim).filter(|s| !s.is_empty()) {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str("---\n");
        body.push_str(diagnostics);
        body.push('\n');
    }
    body
}

pub fn build_feedback_message(
    config: &FeedbackConfig,
    message: &str,
    diagnostics: Option<&str>,
    user_subject: Option<&str>,
) -> Result<Message, String> {
    let from: Mailbox = format!(
        "{} <{}>",
        config.feedback.from_name, config.feedback.from_email
    )
    .parse()
    .map_err(|error| format!("invalid from mailbox: {error}"))?;
    let to: Mailbox = config
        .feedback
        .to_email
        .parse()
        .map_err(|error| format!("invalid to mailbox: {error}"))?;
    let subject = build_feedback_subject(config, user_subject);
    let body = build_feedback_body(message, diagnostics);
    if body.trim().is_empty() {
        return Err("feedback message is empty".into());
    }
    Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .multipart(MultiPart::alternative().singlepart(SinglePart::plain(body)))
        .map_err(|error| format!("build feedback email: {error}"))
}

fn smtp_password(config: &FeedbackConfig) -> String {
    config.smtp.password.trim().to_string()
}

fn smtp_transport(
    config: &FeedbackConfig,
    prefer_ssl: bool,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    let creds = Credentials::new(config.feedback.from_email.clone(), smtp_password(config));
    let tls = TlsParameters::new(config.smtp.host.clone())
        .map_err(|error| format!("smtp tls: {error}"))?;
    let builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp.host)
        .map_err(|error| format!("smtp relay: {error}"))?
        .credentials(creds);
    let transport = if prefer_ssl {
        builder
            .port(config.smtp.port_ssl)
            .tls(Tls::Wrapper(tls))
            .build()
    } else {
        builder
            .port(config.smtp.port_starttls)
            .tls(Tls::Required(tls))
            .build()
    };
    Ok(transport)
}

fn auth_related(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("535")
        || lower.contains("authentication")
        || lower.contains("auth")
        || lower.contains("credential")
}

async fn send_with_fallback(
    config: &FeedbackConfig,
    email: Message,
) -> Result<(), String> {
    let primary = config.smtp.prefer_ssl;
    let first = smtp_transport(config, primary)?;
    match first.send(email.clone()).await {
        Ok(_) => Ok(()),
        Err(first_error) => {
            let first_msg = first_error.to_string();
            let second = smtp_transport(config, !primary)?;
            match second.send(email).await {
                Ok(_) => Ok(()),
                Err(second_error) => {
                    let second_msg = second_error.to_string();
                    if auth_related(&first_msg) || auth_related(&second_msg) {
                        Err(format!(
                            "SMTP authentication failed (tried port {} then {}). \
                             Confirm the IMAP/SMTP password in feedback.local.toml \
                             is the Feishu mailbox client password (not the login password), \
                             and that IMAP/SMTP is enabled for this mailbox. Details: {first_msg} | {second_msg}",
                            if primary {
                                config.smtp.port_ssl
                            } else {
                                config.smtp.port_starttls
                            },
                            if primary {
                                config.smtp.port_starttls
                            } else {
                                config.smtp.port_ssl
                            },
                        ))
                    } else {
                        Err(format!(
                            "failed to send feedback email: {first_msg} | fallback: {second_msg}"
                        ))
                    }
                }
            }
        }
    }
}

/// Send a feedback email using the bundled SMTP configuration plus local secrets.
#[tauri::command]
pub async fn send_feedback_email(
    message: String,
    diagnostics: Option<String>,
    subject: Option<String>,
) -> Result<(), String> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err("Please enter feedback before sending.".into());
    }
    let config = load_feedback_config()?;
    let email = build_feedback_message(
        &config,
        trimmed,
        diagnostics.as_deref(),
        subject.as_deref(),
    )?;
    send_with_fallback(&config, email).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[feedback]
to_email = "support@example.com"
from_email = "support@example.com"
from_name = "Helper"
subject_prefix = "App feedback"

[smtp]
host = "smtp.example.com"
password = "secret"
port_ssl = 465
port_starttls = 587
prefer_ssl = true

[imap]
host = "imap.example.com"
port_ssl = 993
"#;

    #[test]
    fn bundled_config_parses_without_password() {
        let config = parse_feedback_config_base(BUNDLED_FEEDBACK_CONFIG).expect("bundled");
        assert_eq!(config.feedback.to_email, "tcscience@chaoxiiot.com");
        assert_eq!(config.smtp.host, "smtp.feishu.cn");
        assert_eq!(config.smtp.port_ssl, 465);
        assert_eq!(config.smtp.port_starttls, 587);
        assert!(config.smtp.prefer_ssl);
        assert!(config.smtp.password.is_empty());
        let imap = config.imap.as_ref().expect("imap section");
        assert_eq!(imap.host, "imap.feishu.cn");
        assert_eq!(imap.port_ssl, 993);
        let err = validate_feedback_config(&config).unwrap_err();
        assert!(err.contains("feedback.local.toml"));
    }

    #[test]
    fn local_override_supplies_password() {
        let mut config = parse_feedback_config_base(BUNDLED_FEEDBACK_CONFIG).unwrap();
        apply_feedback_override(
            &mut config,
            r#"
[smtp]
password = "from-local-file"
"#,
        )
        .unwrap();
        validate_feedback_config(&config).unwrap();
        assert_eq!(config.smtp.password, "from-local-file");
    }

    #[test]
    fn parse_rejects_missing_password() {
        let raw = SAMPLE.replace("password = \"secret\"\n", "");
        let err = parse_feedback_config(&raw).unwrap_err();
        assert!(err.contains("password"));
    }

    #[test]
    fn message_includes_diagnostics_footer() {
        let config = parse_feedback_config(SAMPLE).unwrap();
        let subject = build_feedback_subject(&config, Some("blank window"));
        assert_eq!(subject, "App feedback - blank window");
        let email = build_feedback_message(
            &config,
            "The window stayed blank",
            Some("SuperScience version: 0.36.0\nOS / architecture: macos / aarch64"),
            Some("blank window"),
        )
        .unwrap();
        let rendered = String::from_utf8_lossy(&email.formatted()).into_owned();
        assert!(rendered.contains("The window stayed blank"));
        assert!(rendered.contains("SuperScience version: 0.36.0"));
        assert!(rendered.contains("support@example.com"));
        assert!(rendered.contains("Subject:"));
    }

    #[test]
    fn empty_message_rejected() {
        let config = parse_feedback_config(SAMPLE).unwrap();
        let err = build_feedback_message(&config, "   ", None, None).unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn local_override_candidates_include_source_config() {
        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("config")
            .join(LOCAL_OVERRIDE_NAME);
        assert!(local_override_candidates().contains(&expected));
    }
}
