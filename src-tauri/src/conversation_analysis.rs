//! `export_conversation_analysis` — preview or export every used session.

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use superscience_llm::{Message, Role, ToolSchema};
use superscience_store::{SessionSearchResult, Store};
use superscience_tools::{Tool, ToolEnv, ToolResult};

pub const TOOL_NAME: &str = "export_conversation_analysis";
pub const REPORT_FILE_NAME: &str = "对话分析报告.html";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnalysisTurn {
    pub user: String,
    pub conclusion: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnalyzedSession {
    pub id: String,
    pub project_name: String,
    pub title: String,
    pub activity_at: i64,
    pub turns: Vec<AnalysisTurn>,
}

#[derive(Serialize)]
struct PreviewSession {
    id: String,
    project: String,
    title: String,
    turns: usize,
}

pub(crate) struct ExportConversationAnalysisTool {
    store: Store,
    exclude_frame_id: String,
}

impl ExportConversationAnalysisTool {
    pub(crate) fn new(store: Store, exclude_frame_id: String) -> Self {
        Self {
            store,
            exclude_frame_id,
        }
    }
}

#[async_trait]
impl Tool for ExportConversationAnalysisTool {
    fn name(&self) -> &str {
        TOOL_NAME
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            TOOL_NAME,
            "Preview or export every used conversation across all projects. \
Call once without confirmed (or confirmed=false) to list sessions for the user to approve. \
After the user confirms, call again with confirmed=true to write 对话分析报告.html. \
Do not invent sessions. The current analysis chat is excluded.",
            json!({
                "type": "object",
                "properties": {
                    "confirmed": {
                        "type": "boolean",
                        "description": "false or omitted: return a preview only. true: write the HTML report."
                    }
                }
            }),
        )
    }

    fn preview(&self, args: &Value) -> String {
        if args.get("confirmed").and_then(Value::as_bool) == Some(true) {
            "export 对话分析报告.html".into()
        } else {
            "preview all sessions".into()
        }
    }

    async fn run(&self, args: &Value, env: &dyn ToolEnv) -> ToolResult {
        let confirmed = args.get("confirmed").and_then(Value::as_bool).unwrap_or(false);
        match collect_analyzed_sessions(&self.store, &self.exclude_frame_id).await {
            Ok(sessions) => {
                if confirmed {
                    write_report(env, &sessions)
                } else {
                    ToolResult::ok(preview_json(&sessions))
                }
            }
            Err(error) => ToolResult::fail(format!("{TOOL_NAME} error: {error}")),
        }
    }
}

pub(crate) fn add_configured_conversation_analysis_tool(
    agent: &mut superscience_core::Agent,
    store: Store,
    frame_id: &str,
    specialist_id: Option<&str>,
) {
    if specialist_id != Some(crate::specialists::CONVERSATION_ANALYSIS_ID) {
        return;
    }
    agent.add_tool(Box::new(ExportConversationAnalysisTool::new(
        store,
        frame_id.to_string(),
    )));
}

async fn collect_analyzed_sessions(
    store: &Store,
    exclude_frame_id: &str,
) -> Result<Vec<AnalyzedSession>, String> {
    let listed = store
        .list_all_used_sessions()
        .await
        .map_err(|error| error.to_string())?;
    let mut out = Vec::new();
    for item in listed {
        if item.id == exclude_frame_id {
            continue;
        }
        let messages = store
            .load_messages(&item.id)
            .await
            .map_err(|error| error.to_string())?;
        let turns = turns_from_messages(&messages);
        if turns.is_empty() {
            continue;
        }
        out.push(analyzed_from_search(item, turns));
    }
    Ok(out)
}

fn analyzed_from_search(item: SessionSearchResult, turns: Vec<AnalysisTurn>) -> AnalyzedSession {
    AnalyzedSession {
        id: item.id,
        project_name: if item.project_name.trim().is_empty() {
            item.project_id
        } else {
            item.project_name
        },
        title: item.title,
        activity_at: item.activity_at,
        turns,
    }
}

pub(crate) fn turns_from_messages(messages: &[Message]) -> Vec<AnalysisTurn> {
    let mut turns = Vec::new();
    let mut pending_user: Option<String> = None;
    let mut last_assistant: Option<String> = None;
    for message in messages {
        match message.role {
            Role::User if message.tool_name.is_none() => {
                if let Some(user) = pending_user.take() {
                    turns.push(AnalysisTurn {
                        user,
                        conclusion: last_assistant.take().unwrap_or_default(),
                    });
                }
                let text = message.content.as_text();
                if !text.trim().is_empty() {
                    pending_user = Some(text);
                    last_assistant = None;
                }
            }
            Role::Assistant => {
                let text = message.content.as_text();
                if !text.trim().is_empty() {
                    last_assistant = Some(prefer_conclusion(&text).to_string());
                }
            }
            _ => {}
        }
    }
    if let Some(user) = pending_user {
        turns.push(AnalysisTurn {
            user,
            conclusion: last_assistant.unwrap_or_default(),
        });
    }
    turns
}

pub(crate) fn prefer_conclusion(text: &str) -> &str {
    const MARKERS: &[&str] = &[
        "## 总结",
        "## 结论",
        "**总结**",
        "**结论**",
        "总结：",
        "结论：",
        "总结:",
        "结论:",
    ];
    let mut best: Option<usize> = None;
    for marker in MARKERS {
        if let Some(idx) = text.rfind(marker) {
            if best.is_none_or(|current| idx > current) {
                best = Some(idx);
            }
        }
    }
    best.map(|idx| text[idx..].trim()).unwrap_or(text.trim())
}

fn preview_json(sessions: &[AnalyzedSession]) -> String {
    let mut projects: BTreeMap<String, usize> = BTreeMap::new();
    let mut listed = Vec::with_capacity(sessions.len());
    for session in sessions {
        *projects.entry(session.project_name.clone()).or_insert(0) += 1;
        listed.push(PreviewSession {
            id: session.id.clone(),
            project: session.project_name.clone(),
            title: session.title.clone(),
            turns: session.turns.len(),
        });
    }
    serde_json::to_string_pretty(&json!({
        "session_count": sessions.len(),
        "project_count": projects.len(),
        "projects": projects,
        "sessions": listed,
        "next": "Ask the user to confirm. After they confirm, call again with confirmed=true."
    }))
    .unwrap_or_else(|_| "{\"session_count\":0}".into())
}

fn write_report(env: &dyn ToolEnv, sessions: &[AnalyzedSession]) -> ToolResult {
    let path = env.project_root().join(REPORT_FILE_NAME);
    if let Err(error) = std::fs::write(&path, render_report(sessions)) {
        return ToolResult::fail(format!(
            "{TOOL_NAME} error: cannot write {}: {error}",
            path.display()
        ));
    }
    ToolResult::ok(format!(
        "Wrote {} ({} sessions). Tell the user the project-relative path {}.",
        path.display(),
        sessions.len(),
        REPORT_FILE_NAME
    ))
}

pub(crate) fn render_report(sessions: &[AnalyzedSession]) -> String {
    let mut nav = String::new();
    let mut main = String::new();
    for (index, session) in sessions.iter().enumerate() {
        let sid = format!("session-{}", index + 1);
        let active = if index == 0 { " active" } else { "" };
        let heading = format!(
            "会话 {}：{} · {}",
            index + 1,
            session.title,
            session.project_name
        );
        nav.push_str(&format!(
            "<button type=\"button\" class=\"nav-item{active}\" data-target=\"{sid}\" onclick=\"showSession('{sid}')\">{} · {} 轮</button>\n",
            escape_html(&heading),
            session.turns.len(),
        ));
        main.push_str(&format!(
            "<section id=\"{sid}\" class=\"session{active}\"><h2>{}</h2><p class=\"meta\">{}</p><div class=\"chat\">",
            escape_html(&heading),
            escape_html(&format_activity(session.activity_at)),
        ));
        for turn in &session.turns {
            main.push_str(&format!(
                "<div class=\"user-msg\">用户：{}</div>",
                escape_html(&turn.user)
            ));
            if !turn.conclusion.trim().is_empty() {
                main.push_str(&format!(
                    "<div class=\"model-msg\">模型：{}</div>",
                    escape_html(&turn.conclusion)
                ));
            }
        }
        main.push_str("</div></section>\n");
    }
    if sessions.is_empty() {
        main.push_str("<p class=\"empty\">没有可分析的会话。</p>");
    }
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>对话分析报告</title>
<style>
body {{ margin:0; font:14px/1.55 -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; color:#1f2933; background:#f4f6f8; }}
.layout {{ display:flex; min-height:100vh; }}
.nav {{ width:280px; flex:0 0 280px; background:#fff; border-right:1px solid #d9e2ec; padding:16px; overflow:auto; position:sticky; top:0; height:100vh; box-sizing:border-box; }}
.nav h1 {{ font-size:16px; margin:0 0 12px; }}
#q {{ width:100%; box-sizing:border-box; margin-bottom:12px; padding:8px 10px; border:1px solid #bcccdc; border-radius:6px; }}
.nav-item {{ display:block; width:100%; text-align:left; margin:0 0 6px; padding:8px 10px; border:0; border-radius:6px; background:#f0f4f8; cursor:pointer; }}
.nav-item.active {{ background:#334e68; color:#fff; }}
.main {{ flex:1; padding:24px 32px; }}
.session {{ display:none; }}
.session.active {{ display:block; }}
h2 {{ margin:0 0 8px; font-size:20px; }}
.meta {{ color:#627d98; margin:0 0 16px; }}
.chat {{ display:flex; flex-direction:column; gap:10px; max-width:860px; }}
.user-msg,.model-msg {{ max-width:80%; padding:10px 14px; border-radius:12px; white-space:pre-wrap; word-break:break-word; }}
.user-msg {{ align-self:flex-end; background:#d9f2e6; }}
.model-msg {{ align-self:flex-start; background:#fff; border:1px solid #d9e2ec; }}
.empty {{ color:#627d98; }}
</style>
</head>
<body>
<div class="layout">
<aside class="nav">
<h1>对话分析报告</h1>
<input id="q" placeholder="搜索会话…" oninput="filterSessions(this.value)">
{nav}
</aside>
<main class="main">
{main}
</main>
</div>
<script>
function showSession(id) {{
  document.querySelectorAll('.session').forEach(el => el.classList.toggle('active', el.id === id));
  document.querySelectorAll('.nav-item').forEach(el => el.classList.toggle('active', el.dataset.target === id));
}}
function filterSessions(q) {{
  const needle = (q || '').trim().toLowerCase();
  document.querySelectorAll('.nav-item').forEach(el => {{
    el.hidden = needle !== '' && !el.textContent.toLowerCase().includes(needle);
  }});
}}
</script>
</body>
</html>
"#,
        nav = nav,
        main = main
    )
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn format_activity(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let seconds = if ts > 1_000_000_000_000 { ts / 1000 } else { ts };
    format!("最近活动：{seconds}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::path::PathBuf;

    struct TempEnv(PathBuf);

    #[async_trait]
    impl ToolEnv for TempEnv {
        fn project_root(&self) -> &std::path::Path {
            &self.0
        }
        async fn confirm(&self, _message: &str) -> bool {
            true
        }
        async fn emit(&self, _event: superscience_tools::ToolEvent) {}
    }

    #[test]
    fn turns_keep_last_assistant_and_prefer_conclusion() {
        let messages = vec![
            Message::user("帮我写一首关于春天的诗"),
            Message::assistant("先想意象"),
            Message::assistant("春风轻拂绿柳梢。\n\n## 结论\n定稿：一片生机满野郊。"),
            Message::user("再短一点"),
            Message::assistant("燕归梁上呢喃语。"),
        ];
        let turns = turns_from_messages(&messages);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].user, "帮我写一首关于春天的诗");
        assert!(turns[0].conclusion.contains("定稿：一片生机满野郊"));
        assert!(!turns[0].conclusion.starts_with("先想意象"));
        assert_eq!(turns[1].conclusion, "燕归梁上呢喃语。");
    }

    #[test]
    fn render_report_has_session_switcher() {
        let html = render_report(&[AnalyzedSession {
            id: "s1".into(),
            project_name: "课题A".into(),
            title: "春天的诗".into(),
            activity_at: 0,
            turns: vec![AnalysisTurn {
                user: "写一首诗".into(),
                conclusion: "春风轻拂".into(),
            }],
        }]);
        assert!(html.contains("function showSession"));
        assert!(html.contains("搜索会话"));
        assert!(html.contains("用户：写一首诗"));
        assert!(html.contains("模型：春风轻拂"));
        assert!(html.contains("课题A"));
    }

    #[tokio::test]
    async fn preview_excludes_current_frame_and_export_writes_html() {
        let tmp = std::env::temp_dir().join(format!(
            "wisp_conv_{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = Store::open(&tmp).await.unwrap();
        store.create_project("p1", "课题A", "").await.unwrap();
        store.create_project("p2", "课题B", "").await.unwrap();
        store.create_frame("a", "p1", "OPERON", "m").await.unwrap();
        store.create_frame("b", "p2", "OPERON", "m").await.unwrap();
        store.create_frame("cur", "p1", "OPERON", "m").await.unwrap();
        store
            .append_message("a", 1, &Message::user("问A"))
            .await
            .unwrap();
        store
            .append_message("a", 2, &Message::assistant("答A"))
            .await
            .unwrap();
        store
            .append_message("b", 1, &Message::user("问B"))
            .await
            .unwrap();
        store
            .append_message("b", 2, &Message::assistant("答B"))
            .await
            .unwrap();
        store
            .append_message("cur", 1, &Message::user("分析全部会话"))
            .await
            .unwrap();

        let tool = ExportConversationAnalysisTool::new(store, "cur".into());
        let env_dir = std::env::temp_dir().join(format!("wisp_conv_out_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&env_dir).unwrap();
        let env = TempEnv(env_dir.clone());

        let preview = tool.run(&json!({}), &env).await;
        assert!(preview.success, "{}", preview.content);
        assert!(preview.content.contains("\"session_count\": 2"));
        assert!(preview.content.contains("课题A"));
        assert!(preview.content.contains("课题B"));
        assert!(!preview.content.contains("cur"));

        let exported = tool.run(&json!({"confirmed": true}), &env).await;
        assert!(exported.success, "{}", exported.content);
        let report = std::fs::read_to_string(env_dir.join(REPORT_FILE_NAME)).unwrap();
        assert!(report.contains("问A"));
        assert!(report.contains("答B"));
        assert!(!report.contains("分析全部会话"));

        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_dir_all(&env_dir);
    }

    #[test]
    fn tool_name_is_stable() {
        assert_eq!(TOOL_NAME, "export_conversation_analysis");
    }
}
