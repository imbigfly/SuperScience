//! Outbound PII firewall: Detect → Anonymize → Rehydrate.
//!
//! Keeps reversible pseudonyms (EMAIL_1, PHONE_1, …) so the model retains
//! conversational context while real identifiers never leave the host.

use crate::message::{Completion, Content, Message, Part, Role, ToolSchema};
use crate::privacy_custom::{apply_custom_terms, CustomTerm};
use crate::provider::{Provider, Result, StreamSink};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PiiKind {
    Email,
    Phone,
    IdCard,
    Ssn,
}

impl PiiKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Email => "EMAIL",
            Self::Phone => "PHONE",
            Self::IdCard => "ID",
            Self::Ssn => "SSN",
        }
    }
}

#[derive(Debug, Default)]
struct VaultInner {
    /// original → token
    forward: HashMap<String, String>,
    /// token → original
    reverse: HashMap<String, String>,
    counters: HashMap<String, u32>,
}

impl VaultInner {
    fn token_for(&mut self, kind: PiiKind, original: &str) -> String {
        if let Some(existing) = self.forward.get(original) {
            return existing.clone();
        }
        let prefix = kind.prefix();
        let n = self.counters.entry(prefix.to_string()).or_insert(0);
        *n += 1;
        let token = format!("{prefix}_{n}");
        self.forward.insert(original.to_string(), token.clone());
        self.reverse.insert(token.clone(), original.to_string());
        token
    }

    fn custom_token(&mut self, original: &str, prefix: &str, explicit: bool) -> String {
        if let Some(existing) = self.forward.get(original) {
            return existing.clone();
        }
        let token = if explicit {
            let mut candidate = prefix.to_string();
            if self.reverse.contains_key(&candidate) {
                let n = self.counters.entry("custom_clash".into()).or_insert(0);
                *n += 1;
                candidate = format!("{prefix}_{n}");
            }
            candidate
        } else {
            let n = self.counters.entry(prefix.to_string()).or_insert(0);
            *n += 1;
            if prefix.chars().any(|c| !c.is_ascii()) {
                format!("{prefix}{n}")
            } else {
                format!("{prefix}_{n}")
            }
        };
        self.forward.insert(original.to_string(), token.clone());
        self.reverse.insert(token.clone(), original.to_string());
        token
    }
}

#[derive(Clone, Default)]
pub struct PiiVault {
    inner: Arc<Mutex<VaultInner>>,
}

impl PiiVault {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn token_for_custom(&self, original: &str, prefix: &str, explicit: bool) -> String {
        self.inner
            .lock()
            .expect("pii vault lock")
            .custom_token(original, prefix, explicit)
    }

    pub fn token_for_original(&self, original: &str) -> Option<String> {
        self.inner
            .lock()
            .ok()
            .and_then(|guard| guard.forward.get(original).cloned())
    }

    pub fn token_taken(&self, token: &str) -> bool {
        self.inner
            .lock()
            .map(|guard| guard.reverse.contains_key(token))
            .unwrap_or(true)
    }

    pub fn rehydrate(&self, text: &str) -> String {
        let guard = self.inner.lock().expect("pii vault lock");
        if guard.reverse.is_empty() {
            return text.to_string();
        }
        // Single pass on the original text so restoring token A (whose
        // original happens to equal token B) cannot cascade into B's value.
        let mut tokens: Vec<&String> = guard.reverse.keys().collect();
        tokens.sort_by(|a, b| b.len().cmp(&a.len()));

        let mut hits: Vec<(usize, usize, &String)> = Vec::new();
        let mut i = 0;
        while i < text.len() {
            if !text.is_char_boundary(i) {
                i += 1;
                continue;
            }
            let rest = &text[i..];
            if let Some(token) = tokens
                .iter()
                .copied()
                .find(|token| !token.is_empty() && rest.starts_with(token.as_str()))
            {
                hits.push((i, i + token.len(), token));
                i += token.len();
            } else {
                i += rest.chars().next().map(|ch| ch.len_utf8()).unwrap_or(1);
            }
        }

        let mut out = String::with_capacity(text.len());
        let mut cursor = 0;
        for (start, end, token) in hits {
            out.push_str(&text[cursor..start]);
            if let Some(original) = guard.reverse.get(token) {
                out.push_str(original);
            }
            cursor = end;
        }
        out.push_str(&text[cursor..]);
        out
    }
}

fn detectors() -> &'static [(PiiKind, Regex)] {
    static DETECTORS: OnceLock<Vec<(PiiKind, Regex)>> = OnceLock::new();
    DETECTORS.get_or_init(|| {
        vec![
            (
                PiiKind::Email,
                Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").expect("email"),
            ),
            (
                PiiKind::Ssn,
                Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("ssn"),
            ),
            (
                PiiKind::IdCard,
                Regex::new(
                    r"\b[1-9]\d{5}(?:18|19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b",
                )
                .expect("id"),
            ),
            // Mainland mobile numbers (11 digits starting with 1).
            (
                PiiKind::Phone,
                Regex::new(r"\b1[3-9]\d{9}\b").expect("phone_cn"),
            ),
            // International-ish phone with separators.
            (
                PiiKind::Phone,
                Regex::new(
                    r"(?:\+\d{1,3}[-.\s]?)?\(?\d{2,4}\)?[-.\s]?\d{3,4}[-.\s]?\d{4}\b",
                )
                .expect("phone_intl"),
            ),
        ]
    })
}

/// True for research identifiers we must not treat as PII.
fn is_science_allowlisted(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    upper.starts_with("GSE")
        || upper.starts_with("GSM")
        || upper.starts_with("PMID")
        || upper.starts_with("DOI:")
        || upper.contains("DOI.ORG/")
}

pub fn anonymize_text(vault: &PiiVault, text: &str) -> String {
    anonymize_text_with_terms(vault, text, &[])
}

pub fn anonymize_text_with_terms(vault: &PiiVault, text: &str, terms: &[CustomTerm]) -> String {
    let text = apply_custom_terms(vault, text, terms);
    let mut spans: Vec<(usize, usize, PiiKind, String)> = Vec::new();
    for (kind, re) in detectors() {
        for m in re.find_iter(&text) {
            let value = m.as_str();
            if is_science_allowlisted(value) {
                continue;
            }
            // Prefer longer / earlier matches; skip overlaps.
            if spans
                .iter()
                .any(|(s, e, _, _)| m.start() < *e && m.end() > *s)
            {
                continue;
            }
            spans.push((m.start(), m.end(), *kind, value.to_string()));
        }
    }
    spans.sort_by_key(|(start, _, _, _)| *start);

    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut guard = vault.inner.lock().expect("pii vault lock");
    for (start, end, kind, original) in spans {
        if start < cursor {
            continue;
        }
        out.push_str(&text[cursor..start]);
        out.push_str(&guard.token_for(kind, &original));
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn anonymize_content(vault: &PiiVault, content: &Content, terms: &[CustomTerm]) -> Content {
    match content {
        Content::Text(s) => Content::Text(anonymize_text_with_terms(vault, s, terms)),
        Content::Parts(parts) => Content::Parts(
            parts
                .iter()
                .map(|part| match part {
                    Part::Text { kind, text } => Part::Text {
                        kind: kind.clone(),
                        text: anonymize_text_with_terms(vault, text, terms),
                    },
                    Part::Image { .. } => part.clone(),
                })
                .collect(),
        ),
    }
}

fn anonymize_messages(
    vault: &PiiVault,
    messages: &[Message],
    terms: &[CustomTerm],
) -> Vec<Message> {
    messages
        .iter()
        .map(|m| {
            let mut cloned = m.clone();
            // System / user / assistant / tool text can all leak identifiers.
            cloned.content = anonymize_content(vault, &m.content, terms);
            if m.role == Role::Assistant {
                if let Some(reasoning) = &m.reasoning {
                    cloned.reasoning = Some(anonymize_text_with_terms(vault, reasoning, terms));
                }
            }
            cloned
        })
        .collect()
}

fn rehydrate_completion(vault: &PiiVault, mut completion: Completion) -> Completion {
    completion.content = vault.rehydrate(&completion.content);
    if let Some(reasoning) = completion.reasoning.take() {
        completion.reasoning = Some(vault.rehydrate(&reasoning));
    }
    for call in &mut completion.tool_calls {
        call.function.arguments = vault.rehydrate(&call.function.arguments);
    }
    completion
}

/// Streaming sink that rehydrates pseudonyms as complete tokens arrive.
struct RehydratingSink<'a> {
    inner: &'a mut dyn StreamSink,
    vault: PiiVault,
    text_buf: String,
    reasoning_buf: String,
}

impl<'a> RehydratingSink<'a> {
    fn flush_ready(buf: &mut String, vault: &PiiVault, emit: impl FnOnce(&str)) {
        if buf.is_empty() {
            return;
        }
        let split = hold_index(buf);
        if split == 0 {
            if can_flush_complete_chunk(buf) {
                let out = vault.rehydrate(buf);
                buf.clear();
                if !out.is_empty() {
                    emit(&out);
                }
            } else if let Some(rehydrated) = try_flush_complete_tokens(buf, vault) {
                emit(&rehydrated);
            }
            return;
        }
        let ready = buf[..split].to_string();
        let rest = buf[split..].to_string();
        *buf = rest;
        let out = vault.rehydrate(&ready);
        if !out.is_empty() {
            emit(&out);
        }
    }
}

/// Hold an unclosed `〔…` and a short ASCII suffix that might still grow
/// into `EMAIL_12`.
fn hold_index(buf: &str) -> usize {
    if let Some(pos) = last_unclosed_bracket_start(buf) {
        return pos;
    }
    const HOLD: usize = 16;
    if buf.len() <= HOLD {
        return 0;
    }
    // HOLD bytes may land mid-UTF-8 codepoint (e.g. CJK); floor to a
    // char boundary so streaming Chinese replies do not panic.
    buf.floor_char_boundary(buf.len() - HOLD)
}

fn last_unclosed_bracket_start(buf: &str) -> Option<usize> {
    let mut last_open = None;
    let mut i = 0;
    while i < buf.len() {
        if !buf.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if buf[i..].starts_with('〔') {
            last_open = Some(i);
            i += '〔'.len_utf8();
            continue;
        }
        if buf[i..].starts_with('〕') {
            last_open = None;
            i += '〕'.len_utf8();
            continue;
        }
        i += buf[i..].chars().next().map(|ch| ch.len_utf8()).unwrap_or(1);
    }
    last_open
}

fn can_flush_complete_chunk(buf: &str) -> bool {
    last_unclosed_bracket_start(buf).is_none()
        && !buf.ends_with(|ch: char| ch.is_ascii_alphanumeric() || ch == '_')
}

fn try_flush_complete_tokens(buf: &mut String, vault: &PiiVault) -> Option<String> {
    static TOKEN: OnceLock<Regex> = OnceLock::new();
    let token = TOKEN.get_or_init(|| Regex::new(r"\b(?:EMAIL|PHONE|ID|SSN)_\d+\b").expect("token"));
    if token.is_match(buf) && !buf.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
        let out = vault.rehydrate(buf);
        buf.clear();
        Some(out)
    } else {
        None
    }
}

impl StreamSink for RehydratingSink<'_> {
    fn on_text(&mut self, delta: &str) {
        self.text_buf.push_str(delta);
        let vault = self.vault.clone();
        Self::flush_ready(&mut self.text_buf, &vault, |out| self.inner.on_text(out));
    }

    fn on_reasoning(&mut self, delta: &str) {
        self.reasoning_buf.push_str(delta);
        let vault = self.vault.clone();
        Self::flush_ready(&mut self.reasoning_buf, &vault, |out| {
            self.inner.on_reasoning(out)
        });
    }

    fn on_tool_call(&mut self, index: usize, name: &str, args_delta: &str) {
        // Tool call names stay as-is; arguments may contain tokens.
        let rehydrated = self.vault.rehydrate(args_delta);
        self.inner.on_tool_call(index, name, &rehydrated);
    }

    fn on_usage(&mut self, usage: crate::Usage) {
        self.inner.on_usage(usage);
    }
}

impl RehydratingSink<'_> {
    fn finish(&mut self) {
        if !self.text_buf.is_empty() {
            let out = self.vault.rehydrate(&self.text_buf);
            self.text_buf.clear();
            if !out.is_empty() {
                self.inner.on_text(&out);
            }
        }
        if !self.reasoning_buf.is_empty() {
            let out = self.vault.rehydrate(&self.reasoning_buf);
            self.reasoning_buf.clear();
            if !out.is_empty() {
                self.inner.on_reasoning(&out);
            }
        }
    }
}

/// Provider decorator that anonymizes outbound prompts and rehydrates replies.
pub struct PiiFirewallProvider {
    inner: Box<dyn Provider>,
    vault: PiiVault,
    terms: Vec<CustomTerm>,
}

impl PiiFirewallProvider {
    pub fn new(inner: Box<dyn Provider>) -> Self {
        Self::with_terms(inner, Vec::new())
    }

    pub fn with_terms(inner: Box<dyn Provider>, terms: Vec<CustomTerm>) -> Self {
        Self {
            inner,
            vault: PiiVault::new(),
            terms,
        }
    }
}

#[async_trait]
impl Provider for PiiFirewallProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn complete(&self, messages: &[Message], tools: &[ToolSchema]) -> Result<Completion> {
        let sanitized = anonymize_messages(&self.vault, messages, &self.terms);
        let completion = self.inner.complete(&sanitized, tools).await?;
        Ok(rehydrate_completion(&self.vault, completion))
    }

    async fn stream(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        sink: &mut dyn StreamSink,
    ) -> Result<Completion> {
        let sanitized = anonymize_messages(&self.vault, messages, &self.terms);
        let mut wrapped = RehydratingSink {
            inner: sink,
            vault: self.vault.clone(),
            text_buf: String::new(),
            reasoning_buf: String::new(),
        };
        let completion = self.inner.stream(&sanitized, tools, &mut wrapped).await?;
        wrapped.finish();
        Ok(rehydrate_completion(&self.vault, completion))
    }
}

/// Wrap a provider with the outbound PII firewall when `enabled`.
pub fn maybe_wrap(inner: Box<dyn Provider>, enabled: bool) -> Box<dyn Provider> {
    maybe_wrap_with_terms(inner, enabled, Vec::new())
}

pub fn maybe_wrap_with_terms(
    inner: Box<dyn Provider>,
    enabled: bool,
    terms: Vec<CustomTerm>,
) -> Box<dyn Provider> {
    if enabled {
        Box::new(PiiFirewallProvider::with_terms(inner, terms))
    } else {
        inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anonymizes_and_rehydrates_roundtrip() {
        let vault = PiiVault::new();
        let src = "Patient John contact jane.doe@example.com or 13812345678, SSN 123-45-6789.";
        let out = anonymize_text(&vault, src);
        assert!(out.contains("EMAIL_1"));
        assert!(out.contains("PHONE_1"));
        assert!(out.contains("SSN_1"));
        assert!(!out.contains("jane.doe@example.com"));
        assert!(!out.contains("13812345678"));
        assert!(!out.contains("123-45-6789"));
        assert_eq!(vault.rehydrate(&out), src);
    }

    #[test]
    fn stable_tokens_across_calls() {
        let vault = PiiVault::new();
        let a = anonymize_text(&vault, "mail a@b.co");
        let b = anonymize_text(&vault, "again a@b.co");
        assert!(a.contains("EMAIL_1"));
        assert!(b.contains("EMAIL_1"));
        assert!(!b.contains("EMAIL_2"));
    }

    #[test]
    fn leaves_geo_accessions_alone() {
        let vault = PiiVault::new();
        let src = "Use GSE153250 and GSM1234567 for the study.";
        assert_eq!(anonymize_text(&vault, src), src);
    }

    #[test]
    fn custom_terms_run_before_regex_and_rehydrate() {
        let vault = PiiVault::new();
        let terms = vec![crate::CustomTerm::new(
            "协和医院",
            crate::CustomCategory::Hospital,
            Some("医院A".into()),
        )
        .unwrap()];
        let src = "联系 协和医院 的 jane@lab.org";
        let out = anonymize_text_with_terms(&vault, src, &terms);
        assert!(out.contains("〔词1〕"), "{out}");
        assert!(out.contains("EMAIL_1"), "{out}");
        assert!(!out.contains("协和医院"), "{out}");
        assert!(!out.contains("jane@lab.org"), "{out}");
        assert_eq!(vault.rehydrate(&out), src);
    }

    struct SharedEcho {
        last: Arc<Mutex<String>>,
    }

    #[async_trait]
    impl Provider for SharedEcho {
        fn name(&self) -> &str {
            "echo"
        }
        fn model(&self) -> &str {
            "echo"
        }
        async fn complete(&self, messages: &[Message], _: &[ToolSchema]) -> Result<Completion> {
            let text = messages
                .last()
                .map(|m| match &m.content {
                    Content::Text(s) => s.clone(),
                    _ => String::new(),
                })
                .unwrap_or_default();
            *self.last.lock().unwrap() = text.clone();
            Ok(Completion {
                content: format!("Saw {text}"),
                reasoning: None,
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: Default::default(),
            })
        }
        async fn stream(
            &self,
            messages: &[Message],
            tools: &[ToolSchema],
            sink: &mut dyn StreamSink,
        ) -> Result<Completion> {
            let c = self.complete(messages, tools).await?;
            sink.on_text(&c.content);
            Ok(c)
        }
    }

    #[tokio::test]
    async fn provider_hides_pii_from_inner() {
        let shared = Arc::new(Mutex::new(String::new()));
        let provider = PiiFirewallProvider::new(Box::new(SharedEcho {
            last: shared.clone(),
        }));
        let out = provider
            .complete(&[Message::user("email me at secret@lab.org")], &[])
            .await
            .unwrap();
        let seen = shared.lock().unwrap().clone();
        assert!(seen.contains("EMAIL_1"));
        assert!(!seen.contains("secret@lab.org"));
        assert!(out.content.contains("secret@lab.org"));
        assert!(!out.content.contains("EMAIL_1"));
    }

    struct CollectSink {
        text: String,
    }

    impl StreamSink for CollectSink {
        fn on_text(&mut self, delta: &str) {
            self.text.push_str(delta);
        }
        fn on_reasoning(&mut self, _: &str) {}
        fn on_tool_call(&mut self, _: usize, _: &str, _: &str) {}
        fn on_usage(&mut self, _: crate::Usage) {}
    }

    #[test]
    fn streaming_rehydrate_survives_cjk_byte_splits() {
        // Regression: HOLD-byte flush used to panic on mid-codepoint splits
        // when streaming Chinese (and other multi-byte) text.
        let vault = PiiVault::new();
        let mut sink = CollectSink {
            text: String::new(),
        };
        let mut re = RehydratingSink {
            inner: &mut sink,
            vault,
            text_buf: String::new(),
            reasoning_buf: String::new(),
        };
        let msg = "字数统计结果是449，路径 manuscript/wc2.txt。";
        for ch in msg.chars() {
            re.on_text(&ch.to_string());
        }
        re.finish();
        assert_eq!(sink.text, msg);
    }

    #[test]
    fn streaming_holds_split_custom_token() {
        let vault = PiiVault::new();
        let terms = vec![crate::CustomTerm::new(
            "张三",
            crate::CustomCategory::Custom,
            Some("〔词1〕".into()),
        )
        .unwrap()];
        assert_eq!(anonymize_text_with_terms(&vault, "张三", &terms), "〔词1〕");
        let mut sink = CollectSink {
            text: String::new(),
        };
        let mut re = RehydratingSink {
            inner: &mut sink,
            vault,
            text_buf: String::new(),
            reasoning_buf: String::new(),
        };
        re.on_text("见过");
        re.on_text("〔");
        re.on_text("词1〕了。");
        re.finish();
        assert_eq!(sink.text, "见过张三了。");
    }
}
