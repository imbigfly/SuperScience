//! Skill Catalog → explainable, budgeted Workflow Studio draft.

use crate::{active_skill_index, dynamic_workflow, AppState};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;
use wisp_skills::{
    plan_portfolio, render_skill, PortfolioCandidate, PortfolioConfig, PortfolioPlan,
    ResearchIntent, SkillSideEffects,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SkillPortfolioRequest {
    pub(crate) intent: ResearchIntent,
    pub(crate) config: PortfolioConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SkillPortfolioDraft {
    pub(crate) plan: PortfolioPlan,
    pub(crate) proposal: dynamic_workflow::DynamicAgentWorkflowProposal,
}

#[tauri::command]
pub(crate) async fn plan_skill_portfolio(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    request: SkillPortfolioRequest,
) -> Result<SkillPortfolioDraft, String> {
    let project = state.active(window.label());
    let index = active_skill_index(&state.store, &project).await;
    let candidates = index
        .all()
        .iter()
        .filter_map(|skill| {
            let record = index.effective_record(&skill.name)?;
            Some(PortfolioCandidate {
                id: skill.name.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                tags: skill.tags.clone(),
                rendered_instruction: render_skill(skill),
                scope: record.scope.as_str().into(),
                path: record.path.to_string_lossy().into(),
                skill_md_sha256: record.skill_md_sha256.clone()?,
                metadata: skill.wisp.clone(),
            })
        })
        .collect::<Vec<_>>();
    let plan = plan_portfolio(request.intent, &candidates, request.config)?;
    if plan.selected.is_empty() {
        return Err("No effective Skill matched the research intent and budget.".into());
    }
    let proposal = workflow_draft(&plan);
    Ok(SkillPortfolioDraft { plan, proposal })
}

fn workflow_draft(plan: &PortfolioPlan) -> dynamic_workflow::DynamicAgentWorkflowProposal {
    let mut tasks = plan
        .selected
        .iter()
        .enumerate()
        .map(|(index, selection)| dynamic_workflow::DynamicAgentTaskProposal {
            id: format!("skill-{}", index + 1),
            instruction: format!(
                "Apply only the bound Skill to the research request. Return a concise evidence module and mark claims with the Skill source [{}:{}]. Selection rationale: {}",
                selection.scope,
                selection.skill_id,
                selection.reasons.join("; ")
            ),
            depends_on: vec![],
            capabilities: capabilities_for(selection.side_effects),
            skill_ids: vec![selection.skill_id.clone()],
            specialist_id: None,
            output_schema: Some(json!({
                "type": "object",
                "required": ["skill_source", "findings", "limitations"],
                "properties": {
                    "skill_source": {"type": "string"},
                    "findings": {"type": "array", "items": {"type": "string"}},
                    "limitations": {"type": "array", "items": {"type": "string"}}
                }
            })),
            isolated: false,
            model_id: None,
            executor: None,
            budget: Some(dynamic_workflow::AgentBudgetProposal {
                max_tokens: Some(selection.node_budget),
                max_tool_calls: Some(12),
                max_cost_microunits: None,
            }),
        })
        .collect::<Vec<_>>();
    let dependencies = tasks.iter().map(|task| task.id.clone()).collect();
    tasks.push(dynamic_workflow::DynamicAgentTaskProposal {
        id: "synthesis".into(),
        instruction: "Synthesize the dependency results without repeating methodology. Preserve every [scope:skill-id] source marker, reconcile contradictions, and distinguish evidence from inference.".into(),
        depends_on: dependencies,
        capabilities: vec!["reasoning".into()],
        skill_ids: vec![],
        specialist_id: None,
        output_schema: Some(json!({
            "type": "object",
            "required": ["summary", "evidence", "open_questions"],
            "properties": {
                "summary": {"type": "string"},
                "evidence": {"type": "array", "items": {"type": "string"}},
                "open_questions": {"type": "array", "items": {"type": "string"}}
            }
        })),
        isolated: false,
        model_id: None,
        executor: None,
        budget: Some(dynamic_workflow::AgentBudgetProposal {
            max_tokens: Some(plan.synthesis_reserve),
            max_tool_calls: Some(4),
            max_cost_microunits: None,
        }),
    });
    dynamic_workflow::DynamicAgentWorkflowProposal {
        goal: format!("Skill portfolio: {}", plan.intent.request),
        context: format!(
            "Research intent (JSON): {}",
            serde_json::to_string(&plan.intent).unwrap_or_default()
        ),
        approval_policy: if plan.requires_confirmation {
            dynamic_workflow::AgentApprovalPolicy::ReviewAll
        } else {
            dynamic_workflow::AgentApprovalPolicy::AutoSafe
        },
        tasks,
    }
}

fn capabilities_for(side_effects: SkillSideEffects) -> Vec<String> {
    match side_effects {
        SkillSideEffects::ReadOnly => vec!["reasoning".into()],
        SkillSideEffects::Network => vec!["literature_search".into()],
        SkillSideEffects::ProjectWrite => vec!["project_write".into()],
        SkillSideEffects::CodeExecution => vec!["code_run".into()],
        SkillSideEffects::ExternalService => vec!["external_research".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wisp_skills::{PortfolioDeferral, PortfolioSelection, PortfolioTier};

    #[test]
    fn draft_binds_each_skill_and_reserves_synthesis() {
        let plan = PortfolioPlan {
            intent: ResearchIntent {
                request: "design a study".into(),
                ..Default::default()
            },
            tier: PortfolioTier::Standard,
            selected: vec![PortfolioSelection {
                skill_id: "analysis-workflow".into(),
                name: "Analysis".into(),
                scope: "bundled".into(),
                path: "/skill/SKILL.md".into(),
                skill_md_sha256: "abc".into(),
                score: 9,
                reasons: vec!["stage: analysis".into()],
                instruction_tokens: 200,
                node_budget: 1_200,
                side_effects: SkillSideEffects::ReadOnly,
            }],
            deferred: Vec::<PortfolioDeferral>::new(),
            total_token_budget: 3_000,
            child_token_budget: 2_000,
            selected_node_budget: 1_200,
            synthesis_reserve: 1_000,
            max_parallel: 1,
            estimated_batches: 2,
            requires_confirmation: true,
        };
        let draft = workflow_draft(&plan);
        assert_eq!(draft.tasks[0].skill_ids, ["analysis-workflow"]);
        assert_eq!(draft.tasks[1].depends_on, ["skill-1"]);
        assert_eq!(
            draft.tasks[1].budget.as_ref().unwrap().max_tokens,
            Some(1_000)
        );
        assert_eq!(
            draft.approval_policy,
            dynamic_workflow::AgentApprovalPolicy::ReviewAll
        );
    }
}
