//! User feedback: load bundled SMTP settings and send support email.
//!
//! Non-secret settings live in `src-tauri/config/feedback.toml` (embedded at
//! compile time). The SMTP password comes from a local override file
//! `feedback.local.toml` that is gitignored, or from the compile-time env
//! `SUPERSCIENCE_FEEDBACK_SMTP_PASSWORD` / `WISP_FEEDBACK_SMTP_PASSWORD`.
//! Unit tests cover parsing/merging only — they never open a real SMTP
//! connection.

use base64::Engine;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use superscience_dto::FeedbackAttachment;

const BUNDLED_FEEDBACK_CONFIG: &str = include_str!("../config/feedback.toml");
const LOCAL_OVERRIDE_NAME: &str = "feedback.local.toml";

pub const MAX_FEEDBACK_ATTACHMENTS: usize = 8;
pub const MAX_FEEDBACK_ATTACHMENT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_FEEDBACK_ATTACHMENTS_TOTAL_BYTES: usize = 16 * 1024 * 1024;

const BLOCKED_ATTACHMENT_EXTS: &[&str] = &[
    "exe", "bat", "cmd", "com", "scr", "pif", "msi", "dll", "ps1", "vbs", "js",
];

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
    /// Filled from `feedback.local.toml` or a compile-time env; the bundled
    /// file leaves this empty.
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
        apply_feedback_override(&mut config, &raw)
            .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    }
    apply_compile_time_password(&mut config, compile_time_smtp_password());
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

/// Fill an empty SMTP password from a compile-time secret. Local override
/// always wins so developers can rotate without rebuilding.
pub fn apply_compile_time_password(config: &mut FeedbackConfig, compiled: Option<&str>) {
    if !config.smtp.password.trim().is_empty() {
        return;
    }
    if let Some(password) = compiled.map(str::trim).filter(|s| !s.is_empty()) {
        config.smtp.password = password.to_string();
    }
}

fn compile_time_smtp_password() -> Option<&'static str> {
    option_env!("SUPERSCIENCE_FEEDBACK_SMTP_PASSWORD")
        .or_else(|| option_env!("WISP_FEEDBACK_SMTP_PASSWORD"))
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
             to {LOCAL_OVERRIDE_NAME} (gitignored) and set smtp.password, place the same \
             file under the app config directory, or rebuild with \
             SUPERSCIENCE_FEEDBACK_SMTP_PASSWORD / WISP_FEEDBACK_SMTP_PASSWORD."
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

pub fn inferred_subject(message: &str) -> Option<String> {
    let line = message.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    const MAX: usize = 60;
    if line.chars().count() <= MAX {
        Some(line.to_string())
    } else {
        Some(format!("{}…", line.chars().take(MAX).collect::<String>()))
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

pub fn sanitize_filename(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    let leaf = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed).trim();
    let cleaned: String = leaf
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('.').trim();
    if cleaned.is_empty() {
        return Err("attachment name is empty".into());
    }
    Ok(cleaned.to_string())
}

fn attachment_ext(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn reject_blocked_filename(name: &str) -> Result<(), String> {
    let ext = attachment_ext(name);
    if BLOCKED_ATTACHMENT_EXTS.contains(&ext.as_str()) {
        return Err(format!("attachment type `.{ext}` is not allowed"));
    }
    Ok(())
}

fn decode_base64(raw: &str) -> Result<Vec<u8>, String> {
    let payload = raw
        .split_once("base64,")
        .map(|(_, rest)| rest)
        .unwrap_or(raw)
        .trim();
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload))
        .map_err(|error| format!("invalid attachment encoding: {error}"))
}

pub fn prepare_attachments(
    raw: &[FeedbackAttachment],
) -> Result<Vec<(String, String, Vec<u8>)>, String> {
    if raw.len() > MAX_FEEDBACK_ATTACHMENTS {
        return Err(format!(
            "at most {MAX_FEEDBACK_ATTACHMENTS} attachments are allowed"
        ));
    }
    let mut prepared = Vec::with_capacity(raw.len());
    let mut total = 0usize;
    for att in raw {
        let name = sanitize_filename(&att.name)?;
        reject_blocked_filename(&name)?;
        let bytes = decode_base64(&att.data_base64)?;
        if bytes.is_empty() {
            return Err(format!("attachment `{name}` is empty"));
        }
        if bytes.len() > MAX_FEEDBACK_ATTACHMENT_BYTES {
            return Err(format!("attachment `{name}` exceeds 8 MB"));
        }
        total = total.saturating_add(bytes.len());
        if total > MAX_FEEDBACK_ATTACHMENTS_TOTAL_BYTES {
            return Err("attachments exceed 16 MB in total".into());
        }
        let mime = att.mime.trim();
        let mime = if mime.is_empty() {
            "application/octet-stream".into()
        } else {
            mime.to_string()
        };
        prepared.push((name, mime, bytes));
    }
    Ok(prepared)
}

pub fn build_feedback_message(
    config: &FeedbackConfig,
    message: &str,
    diagnostics: Option<&str>,
    user_subject: Option<&str>,
    attachments: &[FeedbackAttachment],
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
    let subject = {
        let inferred = user_subject
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| inferred_subject(message));
        build_feedback_subject(config, inferred.as_deref())
    };
    let body = build_feedback_body(message, diagnostics);
    if body.trim().is_empty() {
        return Err("feedback message is empty".into());
    }
    let prepared = prepare_attachments(attachments)?;
    let builder = Message::builder().from(from).to(to).subject(subject);
    if prepared.is_empty() {
        return builder
            .multipart(MultiPart::alternative().singlepart(SinglePart::plain(body)))
            .map_err(|error| format!("build feedback email: {error}"));
    }
    let mut mixed = MultiPart::mixed().singlepart(SinglePart::plain(body));
    for (filename, mime, bytes) in prepared {
        let content_type = ContentType::parse(&mime)
            .unwrap_or_else(|_| ContentType::parse("application/octet-stream").expect("octet"));
        mixed = mixed.singlepart(Attachment::new(filename).body(bytes, content_type));
    }
    builder
        .multipart(mixed)
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

async fn send_with_fallback(config: &FeedbackConfig, email: Message) -> Result<(), String> {
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
    attachments: Option<Vec<FeedbackAttachment>>,
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
        attachments.as_deref().unwrap_or(&[]),
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
        assert_eq!(config.feedback.to_email, "huangfei@chaoxiiot.com");
        assert_eq!(config.feedback.from_email, "tcscience@chaoxiiot.com");
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
    fn compile_time_password_fills_empty() {
        let mut config = parse_feedback_config_base(BUNDLED_FEEDBACK_CONFIG).unwrap();
        apply_compile_time_password(&mut config, Some("compiled-secret"));
        validate_feedback_config(&config).unwrap();
        assert_eq!(config.smtp.password, "compiled-secret");
    }

    #[test]
    fn local_override_wins_over_compile_time_password() {
        let mut config = parse_feedback_config_base(BUNDLED_FEEDBACK_CONFIG).unwrap();
        apply_feedback_override(
            &mut config,
            r#"
[smtp]
password = "from-local-file"
"#,
        )
        .unwrap();
        apply_compile_time_password(&mut config, Some("compiled-secret"));
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
            &[],
        )
        .unwrap();
        let rendered = String::from_utf8_lossy(&email.formatted()).into_owned();
        assert!(rendered.contains("The window stayed blank"));
        assert!(rendered.contains("SuperScience version: 0.36.0"));
        assert!(rendered.contains("support@example.com"));
        assert!(rendered.contains("Subject:"));
    }

    #[test]
    fn message_includes_attachment_part() {
        let config = parse_feedback_config(SAMPLE).unwrap();
        let email = build_feedback_message(
            &config,
            "See screenshot",
            None,
            None,
            &[FeedbackAttachment {
                name: "shot.png".into(),
                mime: "image/png".into(),
                data_base64: base64::engine::general_purpose::STANDARD.encode(b"png-bytes"),
            }],
        )
        .unwrap();
        let rendered = String::from_utf8_lossy(&email.formatted()).into_owned();
        assert!(rendered.contains("See screenshot"));
        assert!(rendered.contains("shot.png"));
        assert!(rendered.contains("image/png"));
    }

    #[test]
    fn attachment_limits_and_blocked_types() {
        let too_many: Vec<_> = (0..MAX_FEEDBACK_ATTACHMENTS + 1)
            .map(|i| FeedbackAttachment {
                name: format!("n{i}.txt"),
                mime: "text/plain".into(),
                data_base64: base64::engine::general_purpose::STANDARD.encode(b"x"),
            })
            .collect();
        let err = prepare_attachments(&too_many).unwrap_err();
        assert!(err.contains("at most"));

        let blocked = prepare_attachments(&[FeedbackAttachment {
            name: "payload.exe".into(),
            mime: "application/octet-stream".into(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(b"x"),
        }])
        .unwrap_err();
        assert!(blocked.contains("not allowed"));

        let empty = prepare_attachments(&[FeedbackAttachment {
            name: "notes.txt".into(),
            mime: "text/plain".into(),
            data_base64: String::new(),
        }])
        .unwrap_err();
        assert!(empty.contains("empty") || empty.contains("encoding"));
    }

    #[test]
    fn empty_message_rejected() {
        let config = parse_feedback_config(SAMPLE).unwrap();
        let err = build_feedback_message(&config, "   ", None, None, &[]).unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn inferred_subject_uses_first_line() {
        assert_eq!(
            inferred_subject("Window stayed blank\nmore detail"),
            Some("Window stayed blank".into())
        );
    }

    #[test]
    fn local_override_candidates_include_source_config() {
        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("config")
            .join(LOCAL_OVERRIDE_NAME);
        assert!(local_override_candidates().contains(&expected));
    }

    #[test]
    fn sanitize_filename_strips_paths() {
        assert_eq!(sanitize_filename(r"C:\temp\shot.png").unwrap(), "shot.png");
        assert_eq!(sanitize_filename("/tmp/notes.txt").unwrap(), "notes.txt");
        assert!(sanitize_filename("...").is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn send_live_test_feedback_email() {
        send_feedback_email(
            "【自动测试】这是一封天成科研助手「反馈」功能连通性测试邮件，可忽略。\n\
             发送时间：2026-08-30 22:18（本地）。"
                .into(),
            Some(
                "SuperScience version: 1.7.1\n\
                 OS / architecture: macos / aarch64\n\
                 Model profile: test\n\
                 Startup timings: not recorded"
                    .into(),
            ),
            Some("功能连通性测试".into()),
            None,
        )
        .await
        .expect("live feedback email should send");
    }
}
