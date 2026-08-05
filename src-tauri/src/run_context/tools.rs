use super::{
    RunManager, RunPreflightReport, RunPreflightSpec, RunPreflightStatus, SubmitRunRequest,
};
use superscience_llm::ToolSchema;
use superscience_tools::{Tool, ToolEnv, ToolResult};

pub struct RunInContextTool {
    store: superscience_store::Store,
    manager: RunManager,
    project_id: String,
    frame_id: Option<String>,
}

impl RunInContextTool {
    pub fn new(
        store: superscience_store::Store,
        manager: RunManager,
        project_id: String,
        frame_id: Option<String>,
    ) -> Self {
        Self {
            store,
            manager,
            project_id,
            frame_id,
        }
    }
}

#[async_trait::async_trait]
impl Tool for RunInContextTool {
    fn name(&self) -> &str {
        "run_in_context"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            "run_in_context",
            "Submit a persisted background Run in an execution context (`local`, `ssh:<alias>`, or `wsl:<distro>`). For Python/R work, declare a preflight to check the interpreter, explicit import modules/packages, project-relative files, and syntax before submission. Preflight never installs packages or executes the requested command as a dry run. Set wait_for_completion=true for direct model-free waiting, or submit normally and call monitor_run exactly once with the returned Run id to show an inline live card. Never poll with get_run.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "context_id": { "type": "string", "description": "Execution context id, e.g. local, ssh:gpu, wsl:Ubuntu" },
                    "command": { "type": "string", "description": "Command to execute in that context" },
                    "title": { "type": "string", "description": "Short run title" },
                    "timeout_secs": { "type": "integer", "description": "Job wall timeout. SSH: 1s..7d (default 4h); local/WSL: 1s..300s" },
                    "wait_for_completion": { "type": "boolean", "description": "Suspend this tool until the Run reaches a terminal state, without consuming model tokens or repeatedly calling get_run (default false)" },
                    "preflight": {
                        "type": "object",
                        "description": "Safe declarative Python/R environment checks performed before Run submission",
                        "properties": {
                            "language": { "type": "string", "enum": ["python", "r"] },
                            "packages": {
                                "type": "array",
                                "description": "Explicit Python import module names or R package names; no packages are installed",
                                "items": { "type": "string" },
                                "maxItems": 32
                            },
                            "paths": {
                                "type": "array",
                                "description": "Project-relative files that must exist",
                                "items": { "type": "string" },
                                "maxItems": 32
                            },
                            "syntax_paths": {
                                "type": "array",
                                "description": "Project-relative .py/.R files to parse without executing",
                                "items": { "type": "string" },
                                "maxItems": 32
                            },
                            "allow_warnings": {
                                "type": "boolean",
                                "description": "Proceed after non-fatal preflight warnings only after the user has approved them (default false)"
                            }
                        },
                        "required": ["language"]
                    },
                    "input_paths": {
                        "type": "array",
                        "description": "Optional project-relative files bound as exact Run inputs; SSH also stages them flat into the remote workdir",
                        "items": { "type": "string" }
                    },
                    "output_specs": {
                        "type": "array",
                        "description": "Optional output specs. SSH direct currently accepts explicit ssh:// references only",
                        "items": {
                            "type": "object",
                            "properties": {
                                "glob": { "type": "string" },
                                "kind": { "type": "string" },
                                "residency": { "type": "string", "enum": ["local", "remote", "auto"] },
                                "logical_key": { "type": "string", "description": "Stable logical output identity; defaults to the matched project-relative path" },
                                "max_file_mb": { "type": "integer" },
                                "max_total_mb": { "type": "integer" }
                            },
                            "required": ["glob", "kind", "residency"]
                        }
                    }
                },
                "required": ["context_id", "command"]
            }),
        )
    }

    fn preview(&self, args: &serde_json::Value) -> String {
        let context = args
            .get("context_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
        if args
            .get("wait_for_completion")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            format!("{context}: {command} · wait")
        } else {
            format!("{context}: {command}")
        }
    }

    async fn run(&self, args: &serde_json::Value, env: &dyn ToolEnv) -> ToolResult {
        let request: SubmitRunRequest = match serde_json::from_value(args.clone()) {
            Ok(req) => req,
            Err(e) => return ToolResult::fail(format!("run_in_context args error: {e}")),
        };
        if let Err(error) = crate::ssh_guard::preflight_shell(&request.command) {
            return ToolResult::fail(format!(
                "{error} For server-to-server copies, use `transfer_between_contexts`; use \
                 `configure_ssh_trust` first when the user approves a direct trust edge."
            ));
        }
        if !env.danger_auto_approve() {
            if let Some(danger) = superscience_tools::safety::check_command_safety(&request.command)
            {
                let msg = format!(
                    "Dangerous command detected in run_in_context ({}): {}",
                    danger.label(),
                    request.command
                );
                if !env.confirm(&msg).await {
                    return ToolResult::fail("error: User denied action");
                }
            }
        }
        let mut preflight = match args.get("preflight") {
            Some(value) => match serde_json::from_value::<RunPreflightSpec>(value.clone()) {
                Ok(spec) => Some(spec),
                Err(error) => {
                    return ToolResult::fail(format!(
                        "run_in_context preflight args error: {error}"
                    ))
                }
            },
            None => None,
        };
        if let (Some(spec), Some(input_paths)) = (&mut preflight, request.input_paths.as_ref()) {
            for path in input_paths {
                if !spec.paths.contains(path) && !spec.syntax_paths.contains(path) {
                    spec.paths.push(path.clone());
                }
            }
        }
        let preflight_report = if let Some(spec) = preflight.as_ref() {
            match self
                .manager
                .preflight(&self.store, &request.context_id, env.project_root(), spec)
                .await
            {
                Ok(report) if report.status == RunPreflightStatus::Failed => {
                    return ToolResult::fail(preflight_blocked_result(&report, false))
                }
                Ok(report)
                    if report.status == RunPreflightStatus::Warning && !spec.allow_warnings =>
                {
                    return ToolResult::fail(preflight_blocked_result(&report, true));
                }
                Ok(report) => Some(report),
                Err(error) => {
                    return ToolResult::fail(format!("run_in_context preflight error: {error}"))
                }
            }
        } else {
            None
        };
        let wait_for_completion = args
            .get("wait_for_completion")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let submission = match preflight_report.clone() {
            Some(report) => {
                self.manager
                    .submit_preflighted(
                        self.store.clone(),
                        self.project_id.clone(),
                        self.frame_id.clone(),
                        request,
                        Some(env.project_root().to_path_buf()),
                        report,
                    )
                    .await
            }
            None => {
                self.manager
                    .submit(
                        self.store.clone(),
                        self.project_id.clone(),
                        self.frame_id.clone(),
                        request,
                        Some(env.project_root().to_path_buf()),
                    )
                    .await
            }
        };
        match submission {
            Ok(res) if wait_for_completion => {
                match wait_for_terminal(&self.store, &res.run_id, env).await {
                    Ok((run, detached)) => run_wait_result(run, detached),
                    Err(error) => ToolResult::fail(format!("run_in_context wait error: {error}")),
                }
            }
            Ok(res) => {
                let mut value = serde_json::to_value(res).unwrap_or_default();
                if let Some(report) = preflight_report {
                    value["preflight"] = serde_json::to_value(report).unwrap_or_default();
                }
                ToolResult::ok(value.to_string())
            }
            Err(e) => ToolResult::fail(format!("run_in_context error: {e}")),
        }
    }
}

fn preflight_blocked_result(report: &RunPreflightReport, requires_confirmation: bool) -> String {
    serde_json::json!({
        "run_submitted": false,
        "preflight": report,
        "requires_confirmation": requires_confirmation,
        "next_action": if requires_confirmation {
            "Show the warning to the user. Only after explicit approval, repeat with preflight.allow_warnings=true."
        } else {
            "Fix the failed preflight checks before submitting the Run."
        }
    })
    .to_string()
}

pub struct GetRunTool {
    store: superscience_store::Store,
    project_id: String,
}

impl GetRunTool {
    pub fn new(store: superscience_store::Store, project_id: String) -> Self {
        Self { store, project_id }
    }
}

#[async_trait::async_trait]
impl Tool for GetRunTool {
    fn name(&self) -> &str {
        "get_run"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            "get_run",
            "Read one immediate status snapshot for a Run. Never call this repeatedly to wait; call monitor_run exactly once for live monitoring until completion.",
            serde_json::json!({
                "type": "object",
                "properties": { "run_id": { "type": "string" } },
                "required": ["run_id"]
            }),
        )
    }

    fn preview(&self, args: &serde_json::Value) -> String {
        args.get("run_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .into()
    }

    async fn run(&self, args: &serde_json::Value, _env: &dyn ToolEnv) -> ToolResult {
        let Some(run_id) = args.get("run_id").and_then(|value| value.as_str()) else {
            return ToolResult::fail("get_run requires run_id");
        };
        match self.store.get_run(run_id).await {
            Ok(Some(run)) if run.project_id == self.project_id => {
                let active = !run.status.is_terminal();
                let mut value = serde_json::to_value(run).unwrap_or_default();
                if active {
                    value["next_action"] = serde_json::Value::String(
                        "Do not call get_run again. Call monitor_run exactly once with this run_id."
                            .into(),
                    );
                }
                ToolResult::ok(value.to_string())
            }
            Ok(Some(_)) => ToolResult::fail("Run does not belong to this project"),
            Ok(None) => ToolResult::fail("Run not found"),
            Err(error) => ToolResult::fail(format!("get_run error: {error}")),
        }
    }
}

pub struct MonitorRunTool {
    store: superscience_store::Store,
    project_id: String,
}

impl MonitorRunTool {
    pub fn new(store: superscience_store::Store, project_id: String) -> Self {
        Self { store, project_id }
    }
}

#[async_trait::async_trait]
impl Tool for MonitorRunTool {
    fn name(&self) -> &str {
        "monitor_run"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            "monitor_run",
            "Monitor one existing long-running Run until it finishes. Call this exactly once instead of repeatedly calling get_run. SuperScience shows a live Run card, suspends the agent without model calls or token use, and resumes it with the terminal result.",
            serde_json::json!({
                "type": "object",
                "properties": { "run_id": { "type": "string" } },
                "required": ["run_id"]
            }),
        )
    }

    fn preview(&self, args: &serde_json::Value) -> String {
        args.get("run_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .into()
    }

    async fn run(&self, args: &serde_json::Value, env: &dyn ToolEnv) -> ToolResult {
        let Some(run_id) = args.get("run_id").and_then(|value| value.as_str()) else {
            return ToolResult::fail("monitor_run requires run_id");
        };
        match self.store.get_run(run_id).await {
            Ok(Some(run)) if run.project_id == self.project_id => {}
            Ok(Some(_)) => return ToolResult::fail("Run does not belong to this project"),
            Ok(None) => return ToolResult::fail("Run not found"),
            Err(error) => return ToolResult::fail(format!("monitor_run error: {error}")),
        }
        match wait_for_terminal(&self.store, run_id, env).await {
            Ok((run, detached)) => run_wait_result(run, detached),
            Err(error) => ToolResult::fail(format!("monitor_run error: {error}")),
        }
    }
}

async fn wait_for_terminal(
    store: &superscience_store::Store,
    run_id: &str,
    env: &dyn ToolEnv,
) -> Result<(superscience_store::RunRecord, bool), String> {
    loop {
        let run = store
            .get_run(run_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Run not found: {run_id}"))?;
        if run.status.is_terminal() {
            return Ok((run, false));
        }
        if env.is_cancelled() {
            return Ok((run, true));
        }
        tokio::time::sleep(if cfg!(test) {
            std::time::Duration::from_millis(10)
        } else {
            std::time::Duration::from_secs(1)
        })
        .await;
    }
}

fn run_wait_result(run: superscience_store::RunRecord, detached: bool) -> ToolResult {
    let succeeded = run.status == superscience_store::RunStatus::Succeeded;
    let mut value = serde_json::to_value(run).unwrap_or_default();
    if detached {
        value["wait_detached"] = serde_json::Value::Bool(true);
    }
    let content = value.to_string();
    if detached || succeeded {
        ToolResult::ok(content)
    } else {
        ToolResult::fail(content)
    }
}

pub struct CancelRunTool {
    store: superscience_store::Store,
    manager: RunManager,
    project_id: String,
}

impl CancelRunTool {
    pub fn new(store: superscience_store::Store, manager: RunManager, project_id: String) -> Self {
        Self {
            store,
            manager,
            project_id,
        }
    }
}

#[async_trait::async_trait]
impl Tool for CancelRunTool {
    fn name(&self) -> &str {
        "cancel_run"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            "cancel_run",
            "Request cancellation of a submitted or running Run. SSH Runs remain `cancelling` until the remote process group confirms termination.",
            serde_json::json!({
                "type": "object",
                "properties": { "run_id": { "type": "string" } },
                "required": ["run_id"]
            }),
        )
    }

    fn preview(&self, args: &serde_json::Value) -> String {
        args.get("run_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .into()
    }

    async fn run(&self, args: &serde_json::Value, _env: &dyn ToolEnv) -> ToolResult {
        let Some(run_id) = args.get("run_id").and_then(|value| value.as_str()) else {
            return ToolResult::fail("cancel_run requires run_id");
        };
        match self.store.get_run(run_id).await {
            Ok(Some(run)) if run.project_id == self.project_id => {}
            Ok(Some(_)) => return ToolResult::fail("Run does not belong to this project"),
            Ok(None) => return ToolResult::fail("Run not found"),
            Err(error) => return ToolResult::fail(format!("cancel_run error: {error}")),
        }
        match self.manager.cancel(&self.store, run_id).await {
            Ok(()) => match self.store.get_run(run_id).await {
                Ok(Some(run)) => ToolResult::ok(serde_json::to_string(&run).unwrap_or_default()),
                Ok(None) => ToolResult::fail("Run disappeared after cancellation request"),
                Err(error) => ToolResult::fail(format!("cancel_run error: {error}")),
            },
            Err(error) => ToolResult::fail(format!("cancel_run error: {error}")),
        }
    }
}
