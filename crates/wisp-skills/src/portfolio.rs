//! Deterministic, explainable Skill selection and budget preflight.

use crate::{SkillSideEffects, WispSkillMetadata};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const RUNTIME_PARALLEL_LIMIT: usize = 2;
/// Extra per-node allowance for the sub-Agent loop itself — system prompt,
/// tool-call round trips, and search results — beyond the rendered Skill body
/// and the reserved output. Measured Skill nodes consume 15k-22k tokens
/// overall, so a budget of instruction + output alone deadlocks them.
const NODE_AGENT_OVERHEAD_TOKENS: u32 = 12_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioTier {
    Compact,
    Standard,
    Deep,
}

impl PortfolioTier {
    fn skill_limit(self) -> usize {
        match self {
            Self::Compact => 2,
            Self::Standard => 4,
            Self::Deep => 8,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchIntent {
    pub request: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub research_stages: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub evidence_types: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioCandidate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub rendered_instruction: String,
    pub scope: String,
    pub path: String,
    pub skill_md_sha256: String,
    pub metadata: Option<WispSkillMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioConfig {
    pub tier: PortfolioTier,
    /// Total token allowance shared by the selected nodes. `0` (the default
    /// stance) means unlimited — per-node budgets are then only estimates
    /// shown in the plan, not enforced limits.
    pub total_token_budget: u32,
    pub synthesis_reserve: u32,
    pub node_output_tokens: u32,
    pub user_parallel_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortfolioSelection {
    pub skill_id: String,
    pub name: String,
    pub scope: String,
    pub path: String,
    pub skill_md_sha256: String,
    pub score: i32,
    pub reasons: Vec<String>,
    pub instruction_tokens: u32,
    pub node_budget: u32,
    pub side_effects: SkillSideEffects,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortfolioDeferral {
    pub skill_id: String,
    pub name: String,
    pub score: i32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortfolioPlan {
    pub intent: ResearchIntent,
    pub tier: PortfolioTier,
    pub selected: Vec<PortfolioSelection>,
    pub deferred: Vec<PortfolioDeferral>,
    pub total_token_budget: u32,
    pub child_token_budget: u32,
    pub selected_node_budget: u32,
    pub synthesis_reserve: u32,
    pub max_parallel: usize,
    pub estimated_batches: usize,
    pub requires_confirmation: bool,
}

pub fn plan_portfolio(
    mut intent: ResearchIntent,
    candidates: &[PortfolioCandidate],
    config: PortfolioConfig,
) -> Result<PortfolioPlan, String> {
    normalize_intent(&mut intent);
    // A total budget of 0 means unlimited: budgets are an advanced tuning
    // knob, so by default the planner must not defer skills over a guessed
    // allowance. Bounded mode still requires room for the synthesis reserve.
    let bounded = config.total_token_budget > 0;
    if bounded && config.total_token_budget <= config.synthesis_reserve {
        return Err("total token budget must exceed the synthesis reserve".into());
    }
    if config.node_output_tokens == 0 {
        return Err("node output token budget must be greater than zero".into());
    }
    let child_token_budget = if bounded {
        config.total_token_budget - config.synthesis_reserve
    } else {
        0
    };
    let request_tokens = estimate_tokens(&intent.request);
    let mut ranked = candidates
        .iter()
        .map(|candidate| {
            let (score, reasons) = score_candidate(&intent, candidate);
            let instruction_tokens =
                estimate_tokens(&candidate.rendered_instruction).saturating_add(request_tokens);
            let node_budget = instruction_tokens
                .saturating_add(config.node_output_tokens)
                .saturating_add(NODE_AGENT_OVERHEAD_TOKENS);
            (candidate, score, reasons, instruction_tokens, node_budget)
        })
        .filter(|(_, score, _, _, _)| *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.id.cmp(&right.0.id))
    });

    let mut selected = Vec::new();
    let mut deferred = Vec::new();
    let mut used = 0u32;
    for (candidate, score, mut reasons, instruction_tokens, node_budget) in ranked {
        let duplicate_role = candidate.metadata.as_ref().is_some_and(|metadata| {
            !metadata.roles.is_empty()
                && selected.iter().any(|item: &PortfolioSelection| {
                    candidates
                        .iter()
                        .find(|other| other.id == item.skill_id)
                        .and_then(|other| other.metadata.as_ref())
                        .is_some_and(|other| other.roles == metadata.roles)
                })
        });
        let adjusted_score = score - if duplicate_role { 2 } else { 0 };
        if selected.len() >= config.tier.skill_limit() {
            deferred.push(deferral(candidate, adjusted_score, "tier_limit"));
        } else if bounded && used.saturating_add(node_budget) > child_token_budget {
            deferred.push(deferral(
                candidate,
                adjusted_score,
                "insufficient_token_budget",
            ));
        } else {
            if duplicate_role {
                reasons.push("duplicate role penalty".into());
            }
            let side_effects = candidate
                .metadata
                .as_ref()
                .map(|metadata| metadata.side_effects)
                .unwrap_or_default();
            used = used.saturating_add(node_budget);
            selected.push(PortfolioSelection {
                skill_id: candidate.id.clone(),
                name: candidate.name.clone(),
                scope: candidate.scope.clone(),
                path: candidate.path.clone(),
                skill_md_sha256: candidate.skill_md_sha256.clone(),
                score: adjusted_score,
                reasons,
                instruction_tokens,
                node_budget,
                side_effects,
            });
        }
    }
    let ready_nodes = selected.len().max(1);
    let max_parallel = config
        .user_parallel_limit
        .max(1)
        .min(RUNTIME_PARALLEL_LIMIT)
        .min(ready_nodes);
    let estimated_batches = selected.len().div_ceil(max_parallel) + 1;
    let has_side_effects = selected
        .iter()
        .any(|item| item.side_effects != SkillSideEffects::ReadOnly);
    let requires_confirmation = selected.len() > 2
        || has_side_effects
        || !deferred.is_empty()
        || config.tier != PortfolioTier::Compact;
    Ok(PortfolioPlan {
        intent,
        tier: config.tier,
        selected,
        deferred,
        total_token_budget: config.total_token_budget,
        child_token_budget,
        selected_node_budget: used,
        synthesis_reserve: config.synthesis_reserve,
        max_parallel,
        estimated_batches,
        requires_confirmation,
    })
}

fn deferral(candidate: &PortfolioCandidate, score: i32, reason: &str) -> PortfolioDeferral {
    PortfolioDeferral {
        skill_id: candidate.id.clone(),
        name: candidate.name.clone(),
        score,
        reason: reason.into(),
    }
}

fn score_candidate(intent: &ResearchIntent, candidate: &PortfolioCandidate) -> (i32, Vec<String>) {
    let mut score = 0;
    let mut reasons = Vec::new();
    if let Some(metadata) = &candidate.metadata {
        score += overlap(
            "domain",
            &intent.domains,
            &metadata.domains,
            6,
            &mut reasons,
        );
        score += overlap(
            "stage",
            &intent.research_stages,
            &metadata.research_stages,
            5,
            &mut reasons,
        );
        score += overlap("role", &intent.roles, &metadata.roles, 4, &mut reasons);
        score += overlap(
            "evidence",
            &intent.evidence_types,
            &metadata.evidence_types,
            4,
            &mut reasons,
        );
        score += overlap(
            "output",
            &intent.outputs,
            &metadata.outputs,
            3,
            &mut reasons,
        );
    }
    let haystack = format!(
        "{} {} {}",
        candidate.name,
        candidate.description,
        candidate.tags.join(" ")
    )
    .to_ascii_lowercase();
    let lexical = intent
        .request
        .split(|ch: char| !ch.is_alphanumeric() && ch != '-')
        .map(str::to_ascii_lowercase)
        .filter(|word| word.chars().count() > 2 && haystack.contains(word))
        .collect::<BTreeSet<_>>();
    if !lexical.is_empty() {
        score += lexical.len().min(4) as i32;
        reasons.push(format!(
            "lexical match: {}",
            lexical.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    (score, reasons)
}

fn overlap(
    label: &str,
    requested: &[String],
    offered: &[String],
    weight: i32,
    reasons: &mut Vec<String>,
) -> i32 {
    let matched = requested
        .iter()
        .filter(|value| offered.contains(value))
        .cloned()
        .collect::<Vec<_>>();
    if !matched.is_empty() {
        reasons.push(format!("{label}: {}", matched.join(", ")));
    }
    matched.len() as i32 * weight
}

fn normalize_intent(intent: &mut ResearchIntent) {
    let request = intent.request.to_ascii_lowercase();
    if intent.domains.is_empty() {
        for (needle, domain) in [
            ("oncology", "oncology"),
            ("cancer", "oncology"),
            ("tumor", "oncology"),
            ("single-cell", "single-cell"),
            ("single cell", "single-cell"),
            ("genom", "genomics"),
            ("transcript", "transcriptomics"),
            ("proteom", "proteomics"),
            ("literature", "scientific-literature"),
        ] {
            if request.contains(needle) {
                intent.domains.push(domain.into());
            }
        }
        if intent.domains.is_empty() {
            intent.domains.push("general".into());
        }
    }
    if intent.research_stages.is_empty() {
        intent.research_stages = vec!["analysis".into(), "hypothesis".into(), "validation".into()];
    }
    if intent.roles.is_empty() {
        intent.roles = vec!["analyst".into(), "critic".into(), "validator".into()];
    }
    if intent.evidence_types.is_empty() {
        intent.evidence_types = if request.contains("omics") || request.contains("组学") {
            vec!["omics".into(), "computational".into()]
        } else {
            vec!["computational".into(), "literature".into()]
        };
    }
    if intent.outputs.is_empty() {
        intent.outputs.push("research-design".into());
    }
    for values in [
        &mut intent.domains,
        &mut intent.research_stages,
        &mut intent.roles,
        &mut intent.evidence_types,
        &mut intent.outputs,
    ] {
        values
            .iter_mut()
            .for_each(|value| *value = value.trim().to_ascii_lowercase());
        values.sort();
        values.dedup();
    }
    intent.request = intent.request.trim().to_string();
}

fn estimate_tokens(text: &str) -> u32 {
    text.chars().count().div_ceil(4).max(1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, role: &str, body_len: usize) -> PortfolioCandidate {
        PortfolioCandidate {
            id: id.into(),
            name: id.into(),
            description: format!("oncology {role}"),
            tags: vec!["oncology".into()],
            rendered_instruction: "x".repeat(body_len),
            scope: "bundled".into(),
            path: format!("/skills/{id}/SKILL.md"),
            skill_md_sha256: format!("hash-{id}"),
            metadata: Some(WispSkillMetadata {
                schema_version: 1,
                domains: vec!["oncology".into()],
                research_stages: vec!["analysis".into()],
                roles: vec![role.into()],
                evidence_types: vec!["omics".into()],
                outputs: vec!["analysis-module".into()],
                side_effects: SkillSideEffects::ReadOnly,
            }),
        }
    }

    #[test]
    fn ranks_deterministically_and_defers_candidates_before_execution() {
        let candidates = (0..8)
            .map(|index| {
                candidate(
                    &format!("skill-{index}"),
                    if index % 2 == 0 { "analyst" } else { "critic" },
                    400,
                )
            })
            .collect::<Vec<_>>();
        let plan = plan_portfolio(
            ResearchIntent {
                request: "oncology omics analysis".into(),
                domains: vec!["oncology".into()],
                research_stages: vec!["analysis".into()],
                roles: vec!["analyst".into(), "critic".into()],
                evidence_types: vec!["omics".into()],
                outputs: vec!["analysis-module".into()],
            },
            &candidates,
            PortfolioConfig {
                tier: PortfolioTier::Deep,
                total_token_budget: 40_000,
                synthesis_reserve: 15_000,
                node_output_tokens: 300,
                user_parallel_limit: 4,
            },
        )
        .unwrap();
        assert_eq!(plan.selected.len(), 2);
        assert_eq!(plan.deferred.len(), 6);
        // Node budgets must cover the Skill body, the sub-Agent loop overhead,
        // and the reserved output, not just the output.
        assert_eq!(
            plan.selected[0].node_budget,
            plan.selected[0].instruction_tokens + 300 + NODE_AGENT_OVERHEAD_TOKENS
        );
        assert!(plan
            .deferred
            .iter()
            .all(|item| item.reason == "insufficient_token_budget"));
        assert_eq!(plan.max_parallel, 2);
        assert_eq!(plan.estimated_batches, 2);
        assert!(plan.requires_confirmation);
    }

    #[test]
    fn zero_total_budget_means_unlimited_and_never_defers_on_tokens() {
        let candidates = (0..4)
            .map(|index| candidate(&format!("skill-{index}"), "analyst", 400))
            .collect::<Vec<_>>();
        let plan = plan_portfolio(
            ResearchIntent {
                request: "oncology omics analysis".into(),
                domains: vec!["oncology".into()],
                research_stages: vec!["analysis".into()],
                roles: vec!["analyst".into()],
                evidence_types: vec!["omics".into()],
                outputs: vec!["analysis-module".into()],
            },
            &candidates,
            PortfolioConfig {
                tier: PortfolioTier::Deep,
                total_token_budget: 0,
                synthesis_reserve: 0,
                node_output_tokens: 300,
                user_parallel_limit: 4,
            },
        )
        .unwrap();
        assert_eq!(plan.child_token_budget, 0);
        assert!(plan
            .deferred
            .iter()
            .all(|item| item.reason != "insufficient_token_budget"));
        assert_eq!(plan.selected.len(), 4);
        // Node budgets stay as estimates for display even when unbounded.
        assert_eq!(
            plan.selected[0].node_budget,
            plan.selected[0].instruction_tokens + 300 + NODE_AGENT_OVERHEAD_TOKENS
        );
    }
}
