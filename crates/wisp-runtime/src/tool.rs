//! Persistent `python` and `r` tools backed by `RuntimeManager`.

use crate::{
    KernelResp, RuntimeEvent, RuntimeKey, RuntimeManager, LOCAL_CONTEXT_ID, MAX_CODE_BYTES,
};
use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use wisp_llm::ToolSchema;
use wisp_tools::{Tool, ToolEnv, ToolEvent, ToolResult};

/// Normalize path separators for comparison. Only meaningful on Windows,
/// where `\` cannot appear in a filename; on Unix a literal `\` is a legal
/// filename character and replacing it could redirect the path to a
/// *different* existing file (`we\ird.txt` vs `we/ird.txt`), i.e. credit a
/// file the cell never wrote.
fn normalize_separators(path: &str) -> String {
    if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_string()
    }
}

/// Relativize worker-reported absolute writes to the project root.
/// Outside-root paths, the root itself, and empty remainders are dropped;
/// on Windows `\` is normalized to `/`; duplicates are collapsed; the
/// result is sorted.
fn project_relative_writes(root: &Path, reported: &[String]) -> Vec<String> {
    let root = dunce::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut out = Vec::new();
    for raw in reported {
        // Normalize separators before canonicalize/strip so a Windows-style
        // `out\a.txt` still matches on hosts whose temp root is a symlink.
        let normalized = normalize_separators(raw);
        let path = Path::new(&normalized);
        let abs = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let Ok(relative) = abs.strip_prefix(&root) else {
            continue;
        };
        // When canonicalize failed above, `abs` is the raw reported path and
        // the remainder could still climb out of the root (`/root/../etc`).
        // Provenance records feed exports and undo — never let one name a
        // path outside the root.
        if relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            continue;
        }
        let relative = normalize_separators(&relative.to_string_lossy());
        if relative.is_empty() {
            continue;
        }
        out.push(relative);
    }
    out.sort();
    out.dedup();
    out
}

/// Validate paths already reported relative to a host-configured project
/// boundary. Joining and canonicalizing again keeps a compromised or stale
/// worker from naming a symlink target outside the project.
fn validated_project_writes(root: &Path, reported: &[String]) -> Vec<String> {
    let root = dunce::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut out = Vec::new();
    for raw in reported {
        let normalized = normalize_separators(raw);
        let relative = Path::new(&normalized);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            continue;
        }
        let candidate = root.join(relative);
        let Ok(candidate) = dunce::canonicalize(candidate) else {
            continue;
        };
        if candidate.strip_prefix(&root).is_err() {
            continue;
        }
        if !normalized.is_empty() {
            out.push(normalized);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Hand a finished cell's self-reported writes to the running tool
/// environment. Only local kernels report: a remote or WSL worker's absolute
/// paths describe another machine's filesystem. An absent report means the
/// worker could not observe, so the host keeps inferring from its snapshot.
fn report_local_writes(env: &dyn ToolEnv, context_id: &str, response: &KernelResp) {
    if context_id != LOCAL_CONTEXT_ID {
        return;
    }
    let Some(reported) = &response.files_written else {
        return;
    };
    let paths = match response.files_written_base.as_deref() {
        None => project_relative_writes(env.project_root(), reported),
        Some("project") => validated_project_writes(env.project_root(), reported),
        Some(_) => Vec::new(),
    };
    if !paths.is_empty() {
        env.report_written_paths(&paths);
    }
}

pub struct ReplTool {
    manager: RuntimeManager,
    project_id: String,
    scope_key: String,
    session_id: String,
}

pub struct RTool {
    manager: RuntimeManager,
    project_id: String,
    scope_key: String,
    session_id: String,
}

const PYTHON_TOOL_DESCRIPTION: &str = "Execute Python code in a persistent REPL. Variables, imports, and loaded data persist per conversation and execution context; parallel conversations never share interpreter state. Return values of expressions are printed. Paths are interpreted inside the selected context. Use this for analysis, data loading, plotting, and computation when required packages already exist. Do not use this as a package installer; if dependencies are missing, set up a project-local pixi environment or use local-env-setup first.";
const R_TOOL_DESCRIPTION: &str = "Execute R code in a persistent REPL. Variables, libraries, and loaded data persist per conversation and execution context; parallel conversations never share interpreter state. The final visible value is printed. Paths are interpreted inside the selected context. Write plots explicitly with png(), pdf(), ggsave(), or another file device. Rscript and the jsonlite package must already exist in that context; this tool does not install packages.";

impl ReplTool {
    pub fn new(manager: RuntimeManager, project_id: impl Into<String>) -> Self {
        Self {
            manager,
            project_id: project_id.into(),
            scope_key: crate::MAINLINE_RUNTIME_SCOPE.into(),
            session_id: String::new(),
        }
    }

    pub fn new_in_session(
        manager: RuntimeManager,
        project_id: impl Into<String>,
        scope_key: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            manager,
            project_id: project_id.into(),
            scope_key: scope_key.into(),
            session_id: session_id.into(),
        }
    }
}

impl RTool {
    pub fn new(manager: RuntimeManager, project_id: impl Into<String>) -> Self {
        Self {
            manager,
            project_id: project_id.into(),
            scope_key: crate::MAINLINE_RUNTIME_SCOPE.into(),
            session_id: String::new(),
        }
    }

    pub fn new_in_session(
        manager: RuntimeManager,
        project_id: impl Into<String>,
        scope_key: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            manager,
            project_id: project_id.into(),
            scope_key: scope_key.into(),
            session_id: session_id.into(),
        }
    }
}

fn context_id(args: &serde_json::Value) -> Result<&str, &'static str> {
    match args.get("context_id") {
        None => Ok(LOCAL_CONTEXT_ID),
        Some(value) => value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or("argument 'context_id' must be a non-empty string"),
    }
}

fn code_arg(args: &serde_json::Value) -> Result<String, String> {
    let code = args
        .get("code")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "missing required argument 'code'".to_string())?;
    if code.len() > MAX_CODE_BYTES {
        return Err(format!(
            "argument 'code' exceeds {MAX_CODE_BYTES} byte limit"
        ));
    }
    Ok(code.to_string())
}

/// Render a kernel response the way the `python`/`r` tools do, so a user-driven
/// run from the UI reads identically to an agent-driven one.
pub fn format_response(resp: &KernelResp) -> String {
    let mut out = String::new();
    if !resp.stdout.is_empty() {
        out.push_str(&resp.stdout);
    }
    if !resp.stderr.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("[stderr] ");
        out.push_str(&resp.stderr);
    }
    if let Some(err) = &resp.error {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("[error] ");
        out.push_str(err);
    }
    if out.is_empty() {
        out = "(no output)".into();
    }
    out
}

async fn run_runtime(
    manager: &RuntimeManager,
    key: RuntimeKey,
    code: String,
    language: &'static str,
    env: &dyn ToolEnv,
) -> ToolResult {
    if key.context_id == LOCAL_CONTEXT_ID || key.context_id.starts_with("wsl:") {
        if let Err(error) = env.preflight_local_execution(&code).await {
            return ToolResult::fail(error).stop_batch();
        }
    }
    let mut execution = match manager.execute(&key, env.project_root(), code).await {
        Ok(execution) => execution,
        Err(error) => return ToolResult::fail(format!("{language} error: {error}")),
    };
    let mut cancel_poll = tokio::time::interval(std::time::Duration::from_millis(50));
    loop {
        tokio::select! {
            event = execution.recv() => match event {
                Some(RuntimeEvent::Stdout(chunk)) => {
                    env.emit(ToolEvent::Stdout { chunk }).await;
                }
                Some(RuntimeEvent::Finished(Ok(response))) => {
                    report_local_writes(env, &key.context_id, &response);
                    let success = response.error.is_none();
                    return ToolResult {
                        success,
                        content: format_response(&response),
                        image: None,
                        control: wisp_tools::ToolControl::Continue,
                    };
                }
                Some(RuntimeEvent::Finished(Err(error))) => {
                    return ToolResult::fail(format!("{language} error: {error}"));
                }
                None => {
                    return ToolResult::fail(format!(
                        "{language} error: runtime ended before returning a result"
                    ));
                }
            },
            _ = cancel_poll.tick() => {
                if env.is_cancelled() {
                    // Dropping this receiver abandons only the caller. The
                    // manager-owned protocol task still drains the cell.
                    return ToolResult::fail(format!("{language} error: interrupted by user"));
                }
            }
        }
    }
}

#[async_trait]
impl Tool for ReplTool {
    fn name(&self) -> &str {
        "python"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            "python",
            PYTHON_TOOL_DESCRIPTION,
            json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Python code to execute (statements or a single expression)" },
                    "context_id": { "type": "string", "description": "Execution context id; defaults to local (for example local, ssh:gpu, or wsl:Ubuntu)" }
                },
                "required": ["code"]
            }),
        )
    }

    fn preview(&self, args: &serde_json::Value) -> String {
        let context = context_id(args).unwrap_or("invalid");
        let code = args
            .get("code")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        format!("[python @ {context}] {code}")
    }

    async fn run(&self, args: &serde_json::Value, env: &dyn ToolEnv) -> ToolResult {
        let code = match code_arg(args) {
            Ok(code) => code,
            Err(error) => return ToolResult::fail(error),
        };
        let context_id = match context_id(args) {
            Ok(context_id) => context_id,
            Err(error) => return ToolResult::fail(error),
        };
        run_runtime(
            &self.manager,
            RuntimeKey::python_in_scope(&self.project_id, &self.scope_key, context_id)
                .with_session(&self.session_id),
            code,
            "python",
            env,
        )
        .await
    }
}

#[async_trait]
impl Tool for RTool {
    fn name(&self) -> &str {
        "r"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            "r",
            R_TOOL_DESCRIPTION,
            json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "R code to execute (one or more expressions)" },
                    "context_id": { "type": "string", "description": "Execution context id; defaults to local (for example local, ssh:gpu, or wsl:Ubuntu)" }
                },
                "required": ["code"]
            }),
        )
    }

    fn preview(&self, args: &serde_json::Value) -> String {
        let context = context_id(args).unwrap_or("invalid");
        let code = args
            .get("code")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        format!("[r @ {context}] {code}")
    }

    async fn run(&self, args: &serde_json::Value, env: &dyn ToolEnv) -> ToolResult {
        let code = match code_arg(args) {
            Ok(code) => code,
            Err(error) => return ToolResult::fail(error),
        };
        let context_id = match context_id(args) {
            Ok(context_id) => context_id,
            Err(error) => return ToolResult::fail(error),
        };
        run_runtime(
            &self.manager,
            RuntimeKey::r_in_scope(&self.project_id, &self.scope_key, context_id)
                .with_session(&self.session_id),
            code,
            "r",
            env,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::{
        code_arg, context_id, project_relative_writes, report_local_writes,
        validated_project_writes, PYTHON_TOOL_DESCRIPTION, R_TOOL_DESCRIPTION,
    };
    use crate::{KernelResp, LOCAL_CONTEXT_ID, MAX_CODE_BYTES};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use wisp_tools::{ToolEnv, ToolEvent};

    #[test]
    fn python_description_keeps_package_setup_out_of_the_repl() {
        assert!(PYTHON_TOOL_DESCRIPTION.contains("Do not use this as a package installer"));
        assert!(PYTHON_TOOL_DESCRIPTION.contains("project-local pixi"));
        assert!(PYTHON_TOOL_DESCRIPTION.contains("local-env-setup"));
    }

    #[test]
    fn repl_descriptions_promise_per_conversation_state() {
        for description in [PYTHON_TOOL_DESCRIPTION, R_TOOL_DESCRIPTION] {
            assert!(description.contains("persist per conversation"));
            assert!(description.contains("parallel conversations never share interpreter state"));
        }
    }

    #[test]
    fn r_description_requires_existing_runtime_dependencies_and_explicit_plots() {
        assert!(R_TOOL_DESCRIPTION.contains("Rscript"));
        assert!(R_TOOL_DESCRIPTION.contains("jsonlite"));
        assert!(R_TOOL_DESCRIPTION.contains("png()"));
        assert!(R_TOOL_DESCRIPTION.contains("does not install packages"));
    }

    #[test]
    fn context_defaults_to_local_and_rejects_blank_values() {
        assert_eq!(
            context_id(&serde_json::json!({"code": "1"})).unwrap(),
            "local"
        );
        assert!(context_id(&serde_json::json!({"context_id": "  "})).is_err());
        assert_eq!(
            context_id(&serde_json::json!({"context_id": " ssh:gpu "})).unwrap(),
            "ssh:gpu"
        );
    }

    #[test]
    fn code_size_is_rejected_before_runtime_dispatch() {
        let args = serde_json::json!({"code": "x".repeat(MAX_CODE_BYTES + 1)});
        assert!(code_arg(&args).unwrap_err().contains("byte limit"));
    }

    fn unique_tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "wisp_rel_{tag}_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn project_relative_writes_drops_outside_root_and_normalizes() {
        let root = unique_tmp("writes");
        std::fs::create_dir_all(root.join("out")).unwrap();
        std::fs::write(root.join("out/a.txt"), b"a").unwrap();
        std::fs::write(root.join("out/c.txt"), b"c").unwrap();
        let outside = unique_tmp("writes_out");
        std::fs::write(outside.join("cache"), b"x").unwrap();

        let a = root.join("out/a.txt").to_string_lossy().into_owned();
        let c = root.join("out/c.txt").to_string_lossy().into_owned();
        let mut reported = vec![
            a.clone(),
            a.clone(),
            c,
            outside.join("cache").to_string_lossy().into_owned(),
            root.to_string_lossy().into_owned(),
        ];
        // Backslash spellings collapse into the same entries — but only on
        // Windows, where `\` is a separator rather than a filename character.
        if cfg!(windows) {
            reported.push(format!(
                "{}{}out\\a.txt",
                root.display(),
                std::path::MAIN_SEPARATOR
            ));
            reported.push(format!(
                "{}{}out\\c.txt",
                root.display(),
                std::path::MAIN_SEPARATOR
            ));
        }
        let got = project_relative_writes(&root, &reported);
        assert_eq!(got, vec!["out/a.txt".to_string(), "out/c.txt".to_string()]);
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&outside).ok();
    }

    /// On Unix a literal `\` is a legal filename character; the spelling
    /// must be preserved so the record names the file that was written,
    /// not a same-identity sibling under a `we/` directory.
    #[cfg(not(windows))]
    #[test]
    fn project_relative_writes_preserves_backslash_filenames_on_unix() {
        let root = unique_tmp("writes_bs");
        std::fs::write(root.join(r"we\ird.txt"), b"x").unwrap();
        let reported = vec![root.join(r"we\ird.txt").to_string_lossy().into_owned()];
        assert_eq!(
            project_relative_writes(&root, &reported),
            vec![r"we\ird.txt".to_string()]
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn project_relative_writes_rejects_parent_traversal_in_fallback() {
        // A path that does not exist skips canonicalization, so the raw
        // remainder is stripped verbatim; `..` must never survive into the
        // provenance record, where undo would resolve it outside the root.
        let root = unique_tmp("writes_dotdot");
        let escape = format!(
            "{}{}..{}escaped.txt",
            root.display(),
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR
        );
        assert!(project_relative_writes(&root, &[escape]).is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn configured_project_writes_reject_absolute_traversal_and_missing_paths() {
        let root = unique_tmp("configured_writes");
        std::fs::create_dir_all(root.join("out")).unwrap();
        std::fs::write(root.join("out/a.txt"), b"a").unwrap();
        let absolute = root.join("out/a.txt").to_string_lossy().into_owned();
        assert_eq!(
            validated_project_writes(
                &root,
                &[
                    "out/a.txt".into(),
                    "../escape.txt".into(),
                    absolute,
                    "out/missing.txt".into(),
                ],
            ),
            vec!["out/a.txt".to_string()]
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// Captures what the finished-cell branch of `run_runtime` hands to the
    /// agent loop.
    struct RecordingEnv {
        root: PathBuf,
        reported: Mutex<Vec<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl ToolEnv for RecordingEnv {
        fn project_root(&self) -> &Path {
            &self.root
        }
        async fn confirm(&self, _message: &str) -> bool {
            true
        }
        async fn emit(&self, _event: ToolEvent) {}
        fn report_written_paths(&self, paths: &[String]) {
            self.reported.lock().unwrap().push(paths.to_vec());
        }
    }

    fn recording_env(root: PathBuf) -> RecordingEnv {
        RecordingEnv {
            root,
            reported: Mutex::new(Vec::new()),
        }
    }

    fn response_with(files_written: Option<Vec<String>>) -> KernelResp {
        KernelResp {
            files_written,
            ..KernelResp::default()
        }
    }

    #[test]
    fn local_kernel_report_reaches_the_tool_environment() {
        let root = unique_tmp("report_local");
        std::fs::write(root.join("fig_1.png"), b"x").unwrap();
        let env = recording_env(root.clone());
        let reported = vec![root.join("fig_1.png").to_string_lossy().into_owned()];

        report_local_writes(&env, LOCAL_CONTEXT_ID, &response_with(Some(reported)));

        assert_eq!(
            *env.reported.lock().unwrap(),
            vec![vec!["fig_1.png".to_string()]]
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn configured_local_report_reaches_the_tool_environment() {
        let root = unique_tmp("report_configured_local");
        std::fs::write(root.join("fig_1.png"), b"x").unwrap();
        let env = recording_env(root.clone());
        let response = KernelResp {
            files_written: Some(vec!["fig_1.png".into()]),
            files_written_base: Some("project".into()),
            ..KernelResp::default()
        };

        report_local_writes(&env, LOCAL_CONTEXT_ID, &response);

        assert_eq!(
            *env.reported.lock().unwrap(),
            vec![vec!["fig_1.png".to_string()]]
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// A remote or WSL worker's absolute paths describe another filesystem,
    /// and an absent report means "host, keep inferring". Neither may reach
    /// the record.
    #[test]
    fn only_local_kernels_with_an_actual_report_are_forwarded() {
        let root = unique_tmp("report_gates");
        std::fs::write(root.join("fig_1.png"), b"x").unwrap();
        let inside = vec![root.join("fig_1.png").to_string_lossy().into_owned()];
        let env = recording_env(root.clone());

        report_local_writes(&env, "ssh:gpu-box", &response_with(Some(inside.clone())));
        report_local_writes(&env, "wsl:Ubuntu", &response_with(Some(inside)));
        report_local_writes(&env, LOCAL_CONTEXT_ID, &response_with(None));
        report_local_writes(&env, LOCAL_CONTEXT_ID, &response_with(Some(Vec::new())));

        assert!(env.reported.lock().unwrap().is_empty());
        std::fs::remove_dir_all(&root).ok();
    }
}
