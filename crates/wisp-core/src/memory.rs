//! Project-scoped long-term Markdown memory.
//!
//! Notes append to a per-day file under `<root>/.wisp/memory`. Retrieval accepts
//! a small query fan-out, uses Unicode-aware lexical matching (including CJK
//! bigrams/trigrams), and fuses the per-query rankings with reciprocal rank
//! fusion. This keeps retrieval local and deterministic while letting the
//! calling agent plan complementary exact, conceptual, procedural, and temporal
//! searches.

use glob::glob;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const MAX_QUERIES: usize = 4;
const MAX_RESULTS: usize = 20;
const RESULT_CONTENT_CHARS: usize = 2_000;
const RRF_K: f64 = 60.0;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MemorySearchQuery {
    pub text: String,
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MemorySearchResult {
    pub id: String,
    pub file: String,
    pub chunk_index: usize,
    pub content: String,
    pub score: f64,
    pub matched_queries: Vec<String>,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MemorySearchResponse {
    pub queries: Vec<MemorySearchQuery>,
    pub results: Vec<MemorySearchResult>,
    pub truncated: bool,
}

pub struct MemoryManager {
    dir: PathBuf,
}

#[derive(Clone)]
struct MemoryChunk {
    id: String,
    file: String,
    chunk_index: usize,
    content: String,
    lower: String,
    normalized: String,
    recency_bonus: i64,
}

#[derive(Default)]
struct FusedMatch {
    score: f64,
    matched_queries: Vec<String>,
    reasons: Vec<String>,
}

impl MemoryManager {
    pub fn new(root: &Path) -> Self {
        let manager = Self::at(root);
        let _ = std::fs::create_dir_all(manager.dir());
        manager
    }

    /// Point at `<root>/.wisp/memory` without creating it. Use for read-only
    /// browsing of another project's notes; writers still create the directory.
    pub fn at(root: &Path) -> Self {
        Self {
            dir: root.join(".wisp").join("memory"),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn today_path(&self) -> PathBuf {
        self.dir
            .join(format!("{}.md", chrono::Local::now().format("%Y-%m-%d")))
    }

    pub fn append(&self, content: &str) -> std::io::Result<()> {
        use std::io::Write;
        let path = self.today_path();
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        f.write_all(content.trim().as_bytes())?;
        f.write_all(b"\n\n")?;
        Ok(())
    }

    fn split_chunks(text: &str) -> Vec<String> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"\n\s*\n").unwrap());
        re.split(text)
            .map(|chunk| chunk.trim().to_string())
            .filter(|chunk| !chunk.is_empty())
            .collect()
    }

    fn lowercase(text: &str) -> String {
        text.chars().flat_map(char::to_lowercase).collect()
    }

    fn normalized(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut separated = true;
        for ch in Self::lowercase(text).chars() {
            if ch.is_alphanumeric() || is_cjk(ch) {
                out.push(ch);
                separated = false;
            } else if !separated {
                out.push(' ');
                separated = true;
            }
        }
        out.trim().to_string()
    }

    fn query_terms(text: &str) -> Vec<String> {
        let normalized = Self::normalized(text);
        let mut terms = Vec::new();
        let mut seen = HashSet::new();
        for token in normalized.split_whitespace() {
            if seen.insert(token.to_string()) {
                terms.push(token.to_string());
            }
            let chars = token.chars().collect::<Vec<_>>();
            if chars.len() >= 2 && chars.iter().all(|ch| is_cjk(*ch)) {
                for width in [2, 3] {
                    for window in chars.windows(width) {
                        let gram = window.iter().collect::<String>();
                        if seen.insert(gram.clone()) {
                            terms.push(gram);
                        }
                    }
                }
            }
        }
        terms
    }

    fn chunks(&self) -> Vec<MemoryChunk> {
        let pattern = format!("{}/*.md", self.dir.display());
        let mut paths: Vec<PathBuf> = glob(&pattern)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .collect();
        paths.sort_by(|left, right| right.cmp(left));
        let mut chunks = Vec::new();
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let modified = std::fs::metadata(&path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);
            let age_days = (chrono::Utc::now().timestamp() - modified).max(0) / 86_400;
            let recency_bonus = (30 - age_days).max(0);
            let file = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            for (chunk_index, content) in Self::split_chunks(&text).into_iter().enumerate() {
                chunks.push(MemoryChunk {
                    id: format!("{file}#{chunk_index}"),
                    file: file.clone(),
                    chunk_index,
                    lower: Self::lowercase(&content),
                    normalized: Self::normalized(&content),
                    content,
                    recency_bonus,
                });
            }
        }
        chunks
    }

    fn lexical_score(
        chunk: &MemoryChunk,
        query: &MemorySearchQuery,
        prefer_recent: bool,
    ) -> Option<(i64, Vec<String>)> {
        let raw = query.text.trim();
        let lower = Self::lowercase(raw);
        let normalized = Self::normalized(raw);
        let terms = Self::query_terms(raw);
        if normalized.is_empty() || terms.is_empty() {
            return None;
        }

        let mut score = 0i64;
        let mut reasons = Vec::new();
        if lower.len() >= 3 && chunk.lower.contains(&lower) {
            score += 80;
            reasons.push(format!("exact phrase: {raw}"));
        } else if normalized.len() >= 3 && chunk.normalized.contains(&normalized) {
            score += 50;
            reasons.push(format!("normalized phrase: {normalized}"));
        }

        let mut matched = 0usize;
        for term in &terms {
            let occurrences = chunk.normalized.matches(term).count();
            if occurrences == 0 {
                continue;
            }
            matched += 1;
            score += 10 + occurrences.min(3) as i64 * 2;
            if reasons.len() < 6 {
                reasons.push(format!("term: {term}"));
            }
        }
        if matched == 0 {
            return None;
        }
        score += (matched * 30 / terms.len()) as i64;
        score += (chunk.content.len() / 200).min(5) as i64;
        score += if prefer_recent {
            chunk.recency_bonus * 2
        } else {
            chunk.recency_bonus
        };
        Some((score, reasons))
    }

    pub fn search_queries(
        &self,
        queries: &[MemorySearchQuery],
        top_k: usize,
        time_hint: Option<&str>,
    ) -> MemorySearchResponse {
        let queries = queries
            .iter()
            .filter_map(|query| {
                let text = query.text.trim();
                (!text.is_empty()).then(|| MemorySearchQuery {
                    text: text.to_string(),
                    kind: query.kind.trim().to_ascii_lowercase(),
                })
            })
            .take(MAX_QUERIES)
            .collect::<Vec<_>>();
        let chunks = self.chunks();
        let prefer_recent = time_hint.is_some_and(|hint| hint.eq_ignore_ascii_case("recent"));
        let mut fused: HashMap<String, FusedMatch> = HashMap::new();

        for query in &queries {
            let mut ranked = chunks
                .iter()
                .filter_map(|chunk| {
                    Self::lexical_score(chunk, query, prefer_recent)
                        .map(|(score, reasons)| (chunk, score, reasons))
                })
                .collect::<Vec<_>>();
            ranked.sort_by(|left, right| {
                right
                    .1
                    .cmp(&left.1)
                    .then_with(|| left.0.id.cmp(&right.0.id))
            });
            for (rank, (chunk, _, reasons)) in ranked.into_iter().enumerate() {
                let entry = fused.entry(chunk.id.clone()).or_default();
                entry.score += 1.0 / (RRF_K + rank as f64 + 1.0);
                entry.matched_queries.push(query.text.clone());
                for reason in reasons {
                    if !entry.reasons.contains(&reason) {
                        entry.reasons.push(reason);
                    }
                }
            }
        }

        let available = fused.len();
        let limit = top_k.clamp(1, MAX_RESULTS);
        let mut matches = fused.into_iter().collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .1
                .score
                .total_cmp(&left.1.score)
                .then_with(|| left.0.cmp(&right.0))
        });
        let results = matches
            .into_iter()
            .take(limit)
            .filter_map(|(id, fused)| {
                let chunk = chunks.iter().find(|chunk| chunk.id == id)?;
                Some(MemorySearchResult {
                    id,
                    file: chunk.file.clone(),
                    chunk_index: chunk.chunk_index,
                    content: chunk.content.chars().take(RESULT_CONTENT_CHARS).collect(),
                    score: (fused.score * 1_000_000.0).round() / 1_000_000.0,
                    matched_queries: fused.matched_queries,
                    match_reasons: fused.reasons.into_iter().take(10).collect(),
                })
            })
            .collect();
        MemorySearchResponse {
            queries,
            results,
            truncated: available > limit,
        }
    }

    /// Convenience wrapper for single-query non-tool callers.
    pub fn search(&self, query: &str, top_k: usize) -> String {
        serde_json::to_string_pretty(&self.search_queries(
            &[MemorySearchQuery {
                text: query.to_string(),
                kind: "exact".into(),
            }],
            top_k,
            None,
        ))
        .unwrap_or_else(|_| "{\"queries\":[],\"results\":[],\"truncated\":false}".into())
    }
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("wisp-memory-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn at_does_not_create_memory_dir() {
        let root = temp_root("at");
        let memory = MemoryManager::at(&root);
        assert_eq!(memory.dir(), root.join(".wisp").join("memory"));
        assert!(!memory.dir().exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn new_creates_memory_dir() {
        let root = temp_root("new");
        let memory = MemoryManager::new(&root);
        assert!(memory.dir().is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_search_returns_structured_empty_results() {
        let root = temp_root("empty-search");
        let memory = MemoryManager::new(&root);
        let result: MemorySearchResponse =
            serde_json::from_str(&memory.search("preference", 10)).unwrap();
        assert!(result.results.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn chinese_queries_match_without_whitespace() {
        let root = temp_root("cjk");
        let memory = MemoryManager::new(&root);
        memory
            .append("[TURN-MEMORY]\n单细胞分析内存不足时使用 backed 模式分批读取。")
            .unwrap();
        let response = memory.search_queries(
            &[MemorySearchQuery {
                text: "单细胞爆内存".into(),
                kind: "concept".into(),
            }],
            10,
            None,
        );
        assert_eq!(response.results.len(), 1);
        assert!(response.results[0].content.contains("backed"));
        assert!(response.results[0]
            .match_reasons
            .iter()
            .any(|reason| reason.starts_with("term:")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reciprocal_rank_fusion_rewards_multiple_complementary_queries() {
        let root = temp_root("rrf");
        let memory = MemoryManager::new(&root);
        memory
            .append("Scanpy h5ad OOM was fixed with backed mode.\n\nGeneric OOM troubleshooting.")
            .unwrap();
        let response = memory.search_queries(
            &[
                MemorySearchQuery {
                    text: "Scanpy h5ad".into(),
                    kind: "exact".into(),
                },
                MemorySearchQuery {
                    text: "OOM backed mode".into(),
                    kind: "procedural".into(),
                },
            ],
            10,
            None,
        );
        assert_eq!(response.results[0].matched_queries.len(), 2);
        assert!(response.results[0].content.contains("Scanpy"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
