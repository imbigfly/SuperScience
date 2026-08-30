//! Unified read-only `knowledge_search` tool. First provider: WeKnora REST.

use crate::knowledge::{
    missing_config_message, missing_kb_ids_message, parse_knowledge_base_ids, runtime_is_ready,
    weknora_search, KnowledgeHit, KnowledgeRuntime, WeKnoraRuntime,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use superscience_llm::ToolSchema;
use superscience_tools::{Tool, ToolEnv, ToolResult};

pub(crate) struct KnowledgeSearchTool {
    runtime: KnowledgeRuntime,
    proxy: Option<String>,
}

impl KnowledgeSearchTool {
    pub(crate) fn new(runtime: KnowledgeRuntime, proxy: Option<String>) -> Self {
        Self { runtime, proxy }
    }
}

fn override_ids(args: &Value) -> Option<Vec<String>> {
    if let Some(ids) = args
        .get("knowledge_base_ids")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|ids| !ids.is_empty())
    {
        return Some(ids);
    }
    args.get("knowledge_base_id")
        .and_then(Value::as_str)
        .map(parse_knowledge_base_ids)
        .filter(|ids| !ids.is_empty())
}

fn format_hits(hits: &[KnowledgeHit]) -> String {
    if hits.is_empty() {
        return "No matching excerpts were returned from the knowledge base.".into();
    }
    json!({ "hits": hits }).to_string()
}

#[async_trait]
impl Tool for KnowledgeSearchTool {
    fn name(&self) -> &str {
        "knowledge_search"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            "knowledge_search",
            "Search the configured research knowledge base and return raw excerpts with scores and source titles. Use this when the user asks about project or lab knowledge stored in the knowledge base. Quote or cite the excerpts; do not treat this as a chat/answer API — write the reply yourself.",
            json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query in the user's language"
                    },
                    "knowledge_base_id": {
                        "type": "string",
                        "description": "Optional single knowledge-base ID that overrides the Settings default"
                    },
                    "knowledge_base_ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional knowledge-base IDs that override the Settings default"
                    }
                },
                "required": ["query"]
            }),
        )
    }

    fn read_only(&self) -> bool {
        true
    }

    fn preview(&self, args: &Value) -> String {
        args.get("query")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    async fn run(&self, args: &Value, _env: &dyn ToolEnv) -> ToolResult {
        if !runtime_is_ready(&self.runtime) {
            return ToolResult::fail(missing_config_message(&self.runtime));
        }
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if query.is_empty() {
            return ToolResult::fail("knowledge_search error: query is required");
        }
        match &self.runtime {
            KnowledgeRuntime::WeKnora(runtime) => {
                search_weknora(runtime, query, args, self.proxy.as_deref()).await
            }
        }
    }
}

async fn search_weknora(
    runtime: &WeKnoraRuntime,
    query: &str,
    args: &Value,
    proxy: Option<&str>,
) -> ToolResult {
    let ids = override_ids(args);
    if ids.as_ref().is_none_or(|value| value.is_empty()) && runtime.knowledge_base_ids.is_empty() {
        return ToolResult::fail(missing_kb_ids_message());
    }
    match weknora_search(runtime, query, ids, proxy).await {
        Ok(hits) => ToolResult::ok(format_hits(&hits)),
        Err(error) => ToolResult::fail(error),
    }
}

pub(crate) fn add_configured_knowledge_search_tool(
    agent: &mut superscience_core::Agent,
    runtime: Option<KnowledgeRuntime>,
    proxy: Option<String>,
) {
    let Some(runtime) = runtime else {
        return;
    };
    if !runtime_is_ready(&runtime) {
        return;
    }
    agent.add_tool(Box::new(KnowledgeSearchTool::new(runtime, proxy)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_ids_prefer_list_then_single() {
        let args = json!({
            "knowledge_base_ids": [" kb-a ", "", "kb-b"],
            "knowledge_base_id": "kb-ignored"
        });
        assert_eq!(
            override_ids(&args),
            Some(vec!["kb-a".into(), "kb-b".into()])
        );
        assert_eq!(
            override_ids(&json!({"knowledge_base_id": "kb-1, kb-2"})),
            Some(vec!["kb-1".into(), "kb-2".into()])
        );
        assert_eq!(override_ids(&json!({})), None);
    }
}
