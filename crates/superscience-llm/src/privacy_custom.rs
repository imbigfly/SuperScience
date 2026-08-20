//! Optional user dictionary for the outbound PII firewall.
//!
//! Delete this module (and its settings/UI wiring) to revert the firewall to
//! built-in regex detectors only. Do not couple this file to skills or UI.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::privacy::PiiVault;

/// Opening mark of a system-minted custom token (`〔词1〕`).
const CUSTOM_TOKEN_PREFIX: &str = "〔词";
/// Closing mark. The pair stops `〔词1〕` from being a prefix of `〔词12〕`.
const CUSTOM_TOKEN_SUFFIX: char = '〕';

pub fn format_custom_token(n: u32) -> String {
    format!("{CUSTOM_TOKEN_PREFIX}{n}{CUSTOM_TOKEN_SUFFIX}")
}

pub fn parse_custom_token_index(token: &str) -> Option<u32> {
    token
        .strip_prefix(CUSTOM_TOKEN_PREFIX)?
        .strip_suffix(CUSTOM_TOKEN_SUFFIX)?
        .parse()
        .ok()
}

pub fn is_delimited_custom_token(token: &str) -> bool {
    parse_custom_token_index(token).is_some()
}

/// Strings that must not be minted: every original, plus any `〔词N〕` already
/// present in the source or inside a keyword.
pub fn occupied_custom_tokens<'a>(
    text: &str,
    originals: impl IntoIterator<Item = &'a str>,
) -> HashSet<String> {
    let mut occupied = HashSet::new();
    for original in originals {
        if !original.is_empty() {
            occupied.insert(original.to_string());
        }
        push_existing_custom_tokens(original, &mut occupied);
    }
    push_existing_custom_tokens(text, &mut occupied);
    occupied
}

fn push_existing_custom_tokens(text: &str, occupied: &mut HashSet<String>) {
    let mut i = 0;
    while i < text.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if let Some(n) = parse_custom_token_at(&text[i..]) {
            let token = format_custom_token(n);
            i += token.len();
            occupied.insert(token);
            continue;
        }
        i += text[i..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(1);
    }
}

fn parse_custom_token_at(rest: &str) -> Option<u32> {
    let rest = rest.strip_prefix(CUSTOM_TOKEN_PREFIX)?;
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let after = rest.get(digits.len()..)?;
    if after.starts_with(CUSTOM_TOKEN_SUFFIX) {
        digits.parse().ok()
    } else {
        None
    }
}

fn mint_custom_token(
    term: &CustomTerm,
    occupied: &HashSet<String>,
    assigned: &HashMap<String, String>,
    vault: &PiiVault,
    next_n: &mut u32,
) -> String {
    if let Some(preferred) = term.placeholder.as_deref() {
        if is_delimited_custom_token(preferred)
            && !occupied.contains(preferred)
            && !vault.token_taken(preferred)
            && !assigned.values().any(|token| token == preferred)
        {
            return preferred.to_string();
        }
    }
    loop {
        *next_n += 1;
        let token = format_custom_token(*next_n);
        if occupied.contains(&token)
            || vault.token_taken(&token)
            || assigned.values().any(|existing| existing == &token)
        {
            continue;
        }
        return token;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CustomCategory {
    Patient,
    Hospital,
    Department,
    Doctor,
    Id,
    #[default]
    Custom,
}

impl CustomCategory {
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "patient" | "患者" | "病人" => Self::Patient,
            "hospital" | "医院" => Self::Hospital,
            "department" | "科室" | "部门" => Self::Department,
            "doctor" | "医生" | "医师" => Self::Doctor,
            "id" | "编号" | "号" => Self::Id,
            _ => Self::Custom,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomTerm {
    pub original: String,
    #[serde(default)]
    pub category: CustomCategory,
    #[serde(default)]
    pub placeholder: Option<String>,
}

impl CustomTerm {
    pub fn new(
        original: impl Into<String>,
        category: CustomCategory,
        placeholder: Option<String>,
    ) -> Option<Self> {
        let original = original.into().trim().to_string();
        if original.is_empty() {
            return None;
        }
        let placeholder = placeholder.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });
        Some(Self {
            original,
            category,
            placeholder,
        })
    }
}

pub fn parse_custom_terms(raw: &str) -> Vec<CustomTerm> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Ok(terms) = serde_json::from_str::<Vec<CustomTerm>>(trimmed) {
        return terms
            .into_iter()
            .filter_map(|term| CustomTerm::new(term.original, term.category, term.placeholder))
            .collect();
    }
    parse_line_terms(trimmed)
}

fn parse_line_terms(raw: &str) -> Vec<CustomTerm> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let mut parts = line.split('|').map(|part| part.trim().to_string());
            let original = parts.next().unwrap_or_default();
            let category = parts
                .next()
                .map(|value| CustomCategory::parse(&value))
                .unwrap_or_default();
            let placeholder = parts.next();
            CustomTerm::new(original, category, placeholder)
        })
        .collect()
}

/// Replace user terms first (longest original wins), then leave the rest for regex.
///
/// Matches are chosen on the *original* text so a short term such as `医院`
/// cannot eat a longer hospital name. Placeholders are delimited `〔词N〕`
/// tokens that skip any string already present as an original or in `text`.
pub fn apply_custom_terms(vault: &PiiVault, text: &str, terms: &[CustomTerm]) -> String {
    if terms.is_empty() {
        return text.to_string();
    }

    let occupied = occupied_custom_tokens(text, terms.iter().map(|term| term.original.as_str()));
    let mut assigned = HashMap::<String, String>::new();
    let mut next_n = 0_u32;
    for term in terms {
        if assigned.contains_key(&term.original) {
            continue;
        }
        if let Some(existing) = vault.token_for_original(&term.original) {
            assigned.insert(term.original.clone(), existing);
            continue;
        }
        let token = mint_custom_token(term, &occupied, &assigned, vault, &mut next_n);
        assigned.insert(term.original.clone(), token);
    }

    let mut ordered: Vec<&CustomTerm> = terms.iter().collect();
    ordered.sort_by(|a, b| b.original.len().cmp(&a.original.len()));

    let mut hits: Vec<(usize, usize, &CustomTerm)> = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let rest = &text[i..];
        let Some(term) = ordered
            .iter()
            .copied()
            .find(|term| !term.original.is_empty() && rest.starts_with(term.original.as_str()))
        else {
            i += rest.chars().next().map(|ch| ch.len_utf8()).unwrap_or(1);
            continue;
        };
        let end = i + term.original.len();
        hits.push((i, end, term));
        i = end;
    }
    if hits.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, end, term) in hits {
        out.push_str(&text[cursor..start]);
        let token = assigned
            .get(&term.original)
            .cloned()
            .expect("every matched term was assigned a token");
        out.push_str(&vault.token_for_custom(&term.original, &token, true));
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_match_keeps_hospital_name_intact() {
        let vault = PiiVault::new();
        let terms = vec![
            CustomTerm::new("医院", CustomCategory::Hospital, None).unwrap(),
            CustomTerm::new("北京协和医院", CustomCategory::Hospital, None).unwrap(),
        ];
        let out = apply_custom_terms(&vault, "就诊于北京协和医院内分泌科", &terms);
        assert_eq!(out, "就诊于〔词2〕内分泌科");
        assert_eq!(vault.rehydrate(&out), "就诊于北京协和医院内分泌科");
    }

    #[test]
    fn stable_placeholder_for_same_original() {
        let vault = PiiVault::new();
        let terms = vec![CustomTerm::new("张三", CustomCategory::Patient, None).unwrap()];
        let a = apply_custom_terms(&vault, "张三血糖 7.2", &terms);
        let b = apply_custom_terms(&vault, "复查张三", &terms);
        assert_eq!(a, "〔词1〕血糖 7.2");
        assert_eq!(b, "复查〔词1〕");
        assert_eq!(vault.rehydrate(&a), "张三血糖 7.2");
    }

    #[test]
    fn remints_legacy_undelimited_placeholder() {
        let vault = PiiVault::new();
        let terms =
            vec![
                CustomTerm::new("协和医院", CustomCategory::Hospital, Some("医院A".into()))
                    .unwrap(),
            ];
        let out = apply_custom_terms(&vault, "协和医院入院", &terms);
        assert_eq!(out, "〔词1〕入院");
        assert_eq!(vault.rehydrate(&out), "协和医院入院");
    }

    #[test]
    fn delimited_placeholder_survives_bare_ci_and_prefix() {
        let vault = PiiVault::new();
        let terms =
            vec![CustomTerm::new("张三", CustomCategory::Custom, Some("〔词1〕".into())).unwrap()];
        let src = "关键词1见张三，另有词1与词12";
        let out = apply_custom_terms(&vault, src, &terms);
        assert_eq!(out, "关键词1见〔词1〕，另有词1与词12");
        assert_eq!(vault.rehydrate(&out), src);
    }

    #[test]
    fn skips_token_already_in_source_text() {
        let vault = PiiVault::new();
        let terms =
            vec![CustomTerm::new("张三", CustomCategory::Custom, Some("〔词1〕".into())).unwrap()];
        let src = "张三见过〔词1〕";
        let out = apply_custom_terms(&vault, src, &terms);
        assert_eq!(out, "〔词2〕见过〔词1〕");
        assert_eq!(vault.rehydrate(&out), src);
    }

    #[test]
    fn rehydrate_does_not_cascade_when_original_equals_another_token() {
        let vault = PiiVault::new();
        let terms = vec![
            CustomTerm::new("张三", CustomCategory::Custom, Some("〔词2〕".into())).unwrap(),
            CustomTerm::new("〔词1〕", CustomCategory::Custom, Some("〔词3〕".into())).unwrap(),
        ];
        let src = "张三和〔词1〕";
        let out = apply_custom_terms(&vault, src, &terms);
        assert_eq!(out, "〔词2〕和〔词3〕");
        assert_eq!(vault.rehydrate(&out), src);
    }

    #[test]
    fn parses_line_and_json_dictionaries() {
        let lines = parse_custom_terms("张三 | 患者\n协和医院 | 医院 | 医院A\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].category, CustomCategory::Patient);
        assert_eq!(lines[1].placeholder.as_deref(), Some("医院A"));

        let json =
            parse_custom_terms(r#"[{"original":"2024A","category":"id","placeholder":"课题1"}]"#);
        assert_eq!(json[0].original, "2024A");
        assert_eq!(json[0].category, CustomCategory::Id);
    }
}
