use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use wisp_core::{Agent, ContextManager, ExploreTool, GuidanceQueue, MemoryManager, Output};
use wisp_llm::{
    Message, Provider, ProviderConfig, Role, ScriptedCompletion, ScriptedProvider,
    ScriptedProviderSnapshot, ToolSchema,
};
use wisp_skills::SkillIndex;
use wisp_tools::{Approval, Tool, ToolEnv, ToolResult};

const SUITE_SCHEMA: &str = "wisp.agent-eval-suite.v1";
const REPORT_SCHEMA: &str = "wisp.agent-eval-report.v1";
const TRAJECTORY_SCHEMA: &str = "wisp.agent-trajectory.v1";
const BUILTIN_SUITE: &str = include_str!("../eval-suites/offline-v1.yaml");
const DEFAULT_MAX_CONTEXT: usize = 128_000;
const DEFAULT_MAX_ROUNDS: usize = 12;
const DEFAULT_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalMode {
    Offline,
    Live,
}

impl EvalMode {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "offline" => Ok(Self::Offline),
            "live" => Ok(Self::Live),
            _ => bail!("unknown eval mode '{value}'; expected offline or live"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Live => "live",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalOptions {
    pub mode: EvalMode,
    pub suite: Option<PathBuf>,
    pub save: Option<PathBuf>,
    pub compare: Option<PathBuf>,
    pub artifacts: Option<PathBuf>,
    pub cases: Vec<String>,
    pub tags: Vec<String>,
    pub repeat: usize,
    pub timeout_ms: Option<u64>,
    pub parallel: usize,
    pub keep_failed_workspace: bool,
    pub max_tool_calls: Option<u64>,
    pub max_input_tokens: Option<u64>,
    pub max_duration_ms: Option<u64>,
    pub max_cost_microusd: Option<u64>,
    pub input_cost_microusd_per_million: u64,
    pub output_cost_microusd_per_million: u64,
    pub reasoning_cost_microusd_per_million: u64,
    pub max_token_regression_percent: Option<u64>,
    pub max_round_regression: Option<u64>,
    pub min_pass_rate_percent: u64,
    pub allow_regressions: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            mode: EvalMode::Offline,
            suite: None,
            save: None,
            compare: None,
            artifacts: None,
            cases: Vec::new(),
            tags: Vec::new(),
            repeat: 1,
            timeout_ms: None,
            parallel: 1,
            keep_failed_workspace: false,
            max_tool_calls: None,
            max_input_tokens: None,
            max_duration_ms: None,
            max_cost_microusd: None,
            input_cost_microusd_per_million: 0,
            output_cost_microusd_per_million: 0,
            reasoning_cost_microusd_per_million: 0,
            max_token_regression_percent: None,
            max_round_regression: None,
            min_pass_rate_percent: 100,
            allow_regressions: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalSuite {
    schema: String,
    id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    defaults: EvalLimits,
    cases: Vec<EvalCase>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct EvalLimits {
    #[serde(default)]
    max_context_tokens: Option<usize>,
    #[serde(default)]
    max_rounds: Option<usize>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    max_tool_calls: Option<u64>,
    #[serde(default)]
    max_tool_errors: Option<u64>,
    #[serde(default)]
    max_input_tokens: Option<u64>,
    #[serde(default)]
    max_duration_ms: Option<u64>,
    #[serde(default)]
    max_cost_microusd: Option<u64>,
}

impl EvalLimits {
    fn merged(&self, case: &Self, options: &EvalOptions) -> Self {
        Self {
            max_context_tokens: case.max_context_tokens.or(self.max_context_tokens),
            max_rounds: case.max_rounds.or(self.max_rounds),
            timeout_ms: options.timeout_ms.or(case.timeout_ms).or(self.timeout_ms),
            max_tool_calls: options
                .max_tool_calls
                .or(case.max_tool_calls)
                .or(self.max_tool_calls),
            max_tool_errors: case.max_tool_errors.or(self.max_tool_errors),
            max_input_tokens: options
                .max_input_tokens
                .or(case.max_input_tokens)
                .or(self.max_input_tokens),
            max_duration_ms: options
                .max_duration_ms
                .or(case.max_duration_ms)
                .or(self.max_duration_ms),
            max_cost_microusd: options
                .max_cost_microusd
                .or(case.max_cost_microusd)
                .or(self.max_cost_microusd),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalCase {
    id: String,
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    actions: Vec<EvalAction>,
    #[serde(default)]
    files: BTreeMap<String, String>,
    #[serde(default)]
    base64_files: BTreeMap<String, String>,
    #[serde(default)]
    allowed_tools: Vec<String>,
    #[serde(default)]
    script: Vec<ScriptedCompletion>,
    #[serde(default)]
    vision_script: Vec<ScriptedCompletion>,
    #[serde(default)]
    explore_script: Vec<ScriptedCompletion>,
    #[serde(default)]
    fixture_runtimes: bool,
    #[serde(default)]
    fixture_mcp: BTreeMap<String, String>,
    #[serde(default)]
    context_seed: Vec<SeedMessage>,
    #[serde(default)]
    approval: EvalApproval,
    #[serde(default)]
    plan_mode: bool,
    #[serde(default)]
    cancel_after_ms: Option<u64>,
    #[serde(default)]
    auto_compact: Option<bool>,
    #[serde(default)]
    limits: EvalLimits,
    #[serde(default)]
    expect: EvalExpectation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedMessage {
    role: String,
    content: String,
    #[serde(default = "default_seed_repeat")]
    repeat: usize,
}

fn default_seed_repeat() -> usize {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EvalAction {
    Send {
        prompt: String,
        #[serde(default)]
        guidance: Vec<String>,
        #[serde(default)]
        allow_error: bool,
    },
    Resume {
        #[serde(default)]
        allow_error: bool,
    },
    Compact,
    Restart,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct EvalApproval {
    #[serde(default)]
    modes: BTreeMap<String, String>,
    #[serde(default)]
    decisions: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalExpectation {
    #[serde(default = "default_outcome")]
    outcome: String,
    #[serde(default)]
    error_contains: Vec<String>,
    #[serde(default)]
    completion_contains: Vec<String>,
    #[serde(default)]
    expected_files: BTreeMap<String, String>,
    #[serde(default)]
    deleted_files: Vec<String>,
    #[serde(default)]
    required_tools: Vec<String>,
    #[serde(default)]
    forbidden_tools: Vec<String>,
    #[serde(default)]
    tool_order: Vec<String>,
    #[serde(default)]
    tool_args: Vec<ToolArgumentExpectation>,
    #[serde(default)]
    request_contains: Vec<String>,
    #[serde(default)]
    approvals: Option<usize>,
    #[serde(default)]
    remaining_script: Option<usize>,
}

fn default_outcome() -> String {
    "success".into()
}

impl Default for EvalExpectation {
    fn default() -> Self {
        Self {
            outcome: default_outcome(),
            error_contains: Vec::new(),
            completion_contains: Vec::new(),
            expected_files: BTreeMap::new(),
            deleted_files: Vec::new(),
            required_tools: Vec::new(),
            forbidden_tools: Vec::new(),
            tool_order: Vec::new(),
            tool_args: Vec::new(),
            request_contains: Vec::new(),
            approvals: None,
            remaining_script: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolArgumentExpectation {
    name: String,
    #[serde(default)]
    pointer: String,
    equals: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCallRecord {
    call_id: String,
    name: String,
    arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrajectoryEvent {
    schema: String,
    sequence: u64,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    payload: Option<Value>,
}

#[derive(Debug, Clone, Default)]
struct Captured {
    rounds: usize,
    tool_calls: Vec<ToolCallRecord>,
    tool_errors: u64,
    input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
    cached_tokens: u64,
    completion: Option<String>,
    approvals: Vec<bool>,
    events: Vec<TrajectoryEvent>,
}

struct EvalOutput {
    captured: Mutex<Captured>,
    next_sequence: AtomicU64,
    approval_modes: BTreeMap<String, Approval>,
    decisions: Mutex<std::collections::VecDeque<bool>>,
    plan_mode: bool,
}

impl EvalOutput {
    fn new(approval: &EvalApproval, plan_mode: bool) -> Result<Self> {
        let approval_modes = approval
            .modes
            .iter()
            .map(|(tool, mode)| {
                let mode = match mode.as_str() {
                    "allow" => Approval::Allow,
                    "ask" => Approval::Ask,
                    "deny" => Approval::Deny,
                    other => bail!(
                        "approval mode for '{tool}' is '{other}'; expected allow, ask, or deny"
                    ),
                };
                Ok((tool.clone(), mode))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(Self {
            captured: Mutex::new(Captured::default()),
            next_sequence: AtomicU64::new(1),
            approval_modes,
            decisions: Mutex::new(approval.decisions.iter().copied().collect()),
            plan_mode,
        })
    }

    fn push(
        &self,
        kind: &str,
        call_id: Option<String>,
        name: Option<String>,
        ok: Option<bool>,
        payload: Option<Value>,
    ) {
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        self.captured
            .lock()
            .expect("eval capture mutex poisoned")
            .events
            .push(TrajectoryEvent {
                schema: TRAJECTORY_SCHEMA.into(),
                sequence,
                kind: kind.into(),
                call_id,
                name,
                ok,
                payload,
            });
    }

    fn snapshot(&self) -> Captured {
        self.captured
            .lock()
            .expect("eval capture mutex poisoned")
            .clone()
    }
}

impl Output for EvalOutput {
    fn assistant_text(&self, delta: &str) {
        self.push(
            "assistant_delta",
            None,
            None,
            None,
            Some(json!({"delta": delta})),
        );
    }

    fn reasoning(&self, delta: &str) {
        self.push(
            "reasoning_delta",
            None,
            None,
            None,
            Some(json!({"delta": delta})),
        );
    }

    fn tool_call(&self, name: &str, preview: &str) {
        self.push(
            "tool_preview",
            None,
            Some(name.into()),
            None,
            Some(json!({"preview": preview})),
        );
    }

    fn tool_result(&self, name: &str, ok: bool, content: &str, duration_ms: u64) {
        let mut captured = self.captured.lock().expect("eval capture mutex poisoned");
        if !ok {
            captured.tool_errors += 1;
        }
        if name == "attempt_completion" && ok {
            captured.completion = Some(content.to_string());
        }
        drop(captured);
        self.push(
            "tool_execution",
            None,
            Some(name.into()),
            Some(ok),
            Some(json!({"content": content, "duration_ms": duration_ms})),
        );
    }

    fn usage(
        &self,
        round: usize,
        input: u64,
        output: u64,
        reasoning: u64,
        cached: u64,
        ctx_tokens: usize,
        max_context: usize,
        context_usage: wisp_core::ContextUsage,
    ) {
        let mut captured = self.captured.lock().expect("eval capture mutex poisoned");
        captured.rounds += 1;
        captured.input_tokens += input;
        captured.output_tokens += output;
        captured.reasoning_tokens += reasoning;
        captured.cached_tokens += cached;
        drop(captured);
        self.push(
            "usage",
            None,
            None,
            None,
            Some(json!({
                "round": round,
                "input_tokens": input,
                "output_tokens": output,
                "reasoning_tokens": reasoning,
                "cached_tokens": cached,
                "context_tokens": ctx_tokens,
                "max_context_tokens": max_context,
                "context_usage": context_usage,
            })),
        );
    }

    fn compaction_started(&self, strategy: &str) {
        self.push(
            "compaction_started",
            None,
            None,
            None,
            Some(json!({"strategy": strategy})),
        );
    }

    fn compaction(&self, before: usize, after: usize, strategy: &str) {
        self.push(
            "compaction",
            None,
            None,
            None,
            Some(json!({"before_tokens": before, "after_tokens": after, "strategy": strategy})),
        );
    }

    fn confirm(&self, message: &str) -> bool {
        let approved = self
            .decisions
            .lock()
            .expect("eval approval mutex poisoned")
            .pop_front()
            .unwrap_or(false);
        self.captured
            .lock()
            .expect("eval capture mutex poisoned")
            .approvals
            .push(approved);
        self.push(
            "approval",
            None,
            None,
            Some(approved),
            Some(json!({"message": message})),
        );
        approved
    }

    fn approval_mode(&self, tool: &str) -> Approval {
        self.approval_modes
            .get(tool)
            .copied()
            .unwrap_or(Approval::Allow)
    }

    fn restrict_read_paths_to_project(&self) -> bool {
        true
    }

    fn plan_mode(&self) -> bool {
        self.plan_mode
    }

    fn on_message(&self, message: &Message) {
        match message.role {
            Role::Assistant => {
                for call in &message.tool_calls {
                    let arguments = call.args_value();
                    self.captured
                        .lock()
                        .expect("eval capture mutex poisoned")
                        .tool_calls
                        .push(ToolCallRecord {
                            call_id: call.id.clone(),
                            name: call.function.name.clone(),
                            arguments: arguments.clone(),
                        });
                    self.push(
                        "tool_call",
                        Some(call.id.clone()),
                        Some(call.function.name.clone()),
                        None,
                        Some(json!({"arguments": arguments})),
                    );
                }
                if !message.content.as_text().is_empty() {
                    self.push(
                        "assistant_message",
                        None,
                        None,
                        None,
                        Some(json!({"content": message.content, "reasoning": message.reasoning})),
                    );
                }
            }
            Role::Tool => self.push(
                "tool_result",
                message.tool_call_id.clone(),
                message.tool_name.clone(),
                None,
                Some(json!({"content": message.content})),
            ),
            Role::User => self.push(
                "user_message",
                None,
                None,
                None,
                Some(json!({"content": message.content})),
            ),
            Role::System => self.push(
                "system_message",
                None,
                None,
                None,
                Some(json!({"content": message.content})),
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScenarioResult {
    id: String,
    description: String,
    tags: Vec<String>,
    repetition: usize,
    passed: bool,
    failures: Vec<String>,
    duration_ms: u64,
    rounds: usize,
    tool_calls: Vec<ToolCallRecord>,
    tool_errors: u64,
    input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
    cached_tokens: u64,
    cost_microusd: u64,
    completion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    trajectory_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    failed_workspace: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ReportSummary {
    cases: usize,
    attempts: usize,
    passed: usize,
    pass_rate_percent: u64,
    duration_ms: u64,
    rounds: usize,
    tool_calls: u64,
    tool_errors: u64,
    input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
    cached_tokens: u64,
    cost_microusd: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BaselineComparison {
    baseline: String,
    warnings: Vec<String>,
    regressions: Vec<String>,
    improvements: Vec<String>,
    threshold_failures: Vec<String>,
    passed_delta: i64,
    input_tokens_delta: i64,
    rounds_delta: i64,
    duration_ms_delta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalReport {
    schema: String,
    suite_schema: String,
    suite_id: String,
    suite_hash: String,
    wisp_version: String,
    generated_at: String,
    mode: String,
    provider: String,
    model: String,
    repeat: usize,
    summary: ReportSummary,
    scenarios: Vec<ScenarioResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    comparison: Option<BaselineComparison>,
}

#[derive(Clone)]
enum ProviderSource {
    Offline(ScriptedProvider),
    Live(ProviderConfig),
}

impl ProviderSource {
    fn build(&self) -> Box<dyn Provider> {
        match self {
            Self::Offline(provider) => Box::new(provider.clone()),
            Self::Live(config) => wisp_llm::build(config.clone()),
        }
    }

    fn offline_snapshot(&self) -> Option<ScriptedProviderSnapshot> {
        match self {
            Self::Offline(provider) => Some(provider.snapshot()),
            Self::Live(_) => None,
        }
    }
}

struct FixtureMcpTool {
    name: String,
    result: String,
}

#[derive(Default)]
struct EvalRuntimeLauncher;

#[async_trait]
impl wisp_runtime::RuntimeLauncher for EvalRuntimeLauncher {
    async fn launch(
        &self,
        _key: &wisp_runtime::RuntimeKey,
        _cwd: &Path,
    ) -> Result<wisp_runtime::LaunchedRuntime> {
        Ok(wisp_runtime::LaunchedRuntime::new(
            Box::new(EvalRuntimeKernel::default()),
            wisp_runtime::RuntimeMetadata {
                interpreter: Some("headless-fixture".into()),
                version: Some("v1".into()),
                process_id: None,
            },
        ))
    }
}

#[derive(Default)]
struct EvalRuntimeKernel {
    value: String,
}

#[async_trait]
impl wisp_runtime::RuntimeKernel for EvalRuntimeKernel {
    async fn execute(
        &mut self,
        _id: &str,
        code: &str,
        _output: &wisp_runtime::RuntimeOutput,
    ) -> Result<wisp_runtime::KernelResp> {
        let stdout = if let Some(value) = code.strip_prefix("set:") {
            self.value = value.to_string();
            String::new()
        } else if code == "get" {
            self.value.clone()
        } else {
            return Ok(wisp_runtime::KernelResp {
                error: Some(format!("unsupported fixture code '{code}'")),
                ..wisp_runtime::KernelResp::default()
            });
        };
        Ok(wisp_runtime::KernelResp {
            stdout,
            ..wisp_runtime::KernelResp::default()
        })
    }

    async fn inspect(&mut self, _id: &str) -> Result<wisp_runtime::RuntimeObjectList> {
        Ok(wisp_runtime::RuntimeObjectList {
            objects: vec![wisp_runtime::RuntimeObject {
                name: "value".into(),
                type_name: "string".into(),
                summary: self.value.clone(),
                size_bytes: Some(self.value.len() as u64),
            }],
            total_count: 1,
        })
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl Tool for FixtureMcpTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            &self.name,
            "Deterministic offline MCP retrieval fixture for agent evaluation.",
            json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "additionalProperties": true
            }),
        )
    }

    fn defer_schema(&self) -> bool {
        true
    }

    fn read_only(&self) -> bool {
        true
    }

    async fn run(&self, _args: &Value, _env: &dyn ToolEnv) -> ToolResult {
        ToolResult::ok(&self.result)
    }
}

pub async fn run(live_config: Option<ProviderConfig>, options: &EvalOptions) -> Result<()> {
    validate_options(options)?;
    let suite = load_suite(options.suite.as_deref())?;
    validate_suite(&suite, options.mode)?;
    let selected = select_cases(&suite, options)?;
    if selected.is_empty() {
        bail!("no eval cases matched the requested filters");
    }
    if options.mode == EvalMode::Live && live_config.is_none() {
        bail!("live eval requires a configured provider");
    }
    if let Some(dir) = &options.artifacts {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("could not create eval artifacts dir {}", dir.display()))?;
    }

    let suite_hash = sha256_hex(serde_yaml::to_string(&suite)?.as_bytes());
    let semaphore = Arc::new(tokio::sync::Semaphore::new(options.parallel));
    let mut tasks = tokio::task::JoinSet::new();
    for (case_index, case) in selected.into_iter().cloned().enumerate() {
        for repetition in 1..=options.repeat {
            let permit = semaphore.clone().acquire_owned().await?;
            let case = case.clone();
            let defaults = suite.defaults.clone();
            let options = options.clone();
            let live_config = live_config.clone();
            tasks.spawn(async move {
                let _permit = permit;
                let order = case_index * options.repeat + repetition;
                let result = run_case(case, repetition, defaults, live_config, options).await;
                (order, result)
            });
        }
    }

    let mut ordered = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        let (order, result) = joined.context("eval worker panicked")?;
        ordered.push((order, result?));
    }
    ordered.sort_by_key(|(order, _)| *order);
    let scenarios: Vec<_> = ordered.into_iter().map(|(_, result)| result).collect();

    let model = live_config
        .as_ref()
        .map(|config| config.model.clone())
        .unwrap_or_else(|| "scripted-v1".into());
    let provider = live_config
        .as_ref()
        .map(|config| format!("{:?}", config.kind))
        .unwrap_or_else(|| "Scripted".into());
    let mut report = EvalReport {
        schema: REPORT_SCHEMA.into(),
        suite_schema: suite.schema.clone(),
        suite_id: suite.id.clone(),
        suite_hash,
        wisp_version: env!("CARGO_PKG_VERSION").into(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        mode: options.mode.as_str().into(),
        provider,
        model,
        repeat: options.repeat,
        summary: summarize(&scenarios),
        scenarios,
        comparison: None,
    };

    if let Some(path) = &options.compare {
        let bytes = std::fs::read(path)
            .with_context(|| format!("could not read baseline {}", path.display()))?;
        let baseline: EvalReport = serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid eval v1 baseline {}", path.display()))?;
        report.comparison = Some(compare_reports(&report, &baseline, path, options));
    }

    let mut rendered = serde_json::to_string_pretty(&report)?;
    rendered.push('\n');
    if let Some(path) = &options.save {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &rendered)
            .with_context(|| format!("could not save eval report {}", path.display()))?;
    }
    print!("{rendered}");

    let comparison_failed = report.comparison.as_ref().is_some_and(|comparison| {
        !options.allow_regressions
            && (!comparison.regressions.is_empty() || !comparison.threshold_failures.is_empty())
    });
    if report.summary.pass_rate_percent < options.min_pass_rate_percent || comparison_failed {
        bail!(
            "agent eval failed: {}/{} attempts passed ({}%; required {}%)",
            report.summary.passed,
            report.summary.attempts,
            report.summary.pass_rate_percent,
            options.min_pass_rate_percent
        );
    }
    Ok(())
}

fn validate_options(options: &EvalOptions) -> Result<()> {
    if options.repeat == 0 {
        bail!("--repeat must be positive");
    }
    if options.parallel == 0 {
        bail!("--parallel must be positive");
    }
    if options.min_pass_rate_percent > 100 {
        bail!("--min-pass-rate must be between 0 and 100");
    }
    if options.keep_failed_workspace && options.artifacts.is_none() {
        bail!("--keep-failed-workspace requires --artifacts <dir>");
    }
    Ok(())
}

fn load_suite(path: Option<&Path>) -> Result<EvalSuite> {
    let (label, contents) = match path {
        Some(path) => (
            path.display().to_string(),
            std::fs::read_to_string(path)
                .with_context(|| format!("could not read eval suite {}", path.display()))?,
        ),
        None => ("built-in offline-v1".into(), BUILTIN_SUITE.into()),
    };
    if path.is_some_and(|path| path.extension().and_then(|value| value.to_str()) == Some("json")) {
        serde_json::from_str(&contents).with_context(|| format!("invalid JSON eval suite {label}"))
    } else {
        serde_yaml::from_str(&contents).with_context(|| format!("invalid YAML eval suite {label}"))
    }
}

fn validate_suite(suite: &EvalSuite, mode: EvalMode) -> Result<()> {
    if suite.schema != SUITE_SCHEMA {
        bail!(
            "unsupported eval suite schema '{}'; expected {SUITE_SCHEMA}",
            suite.schema
        );
    }
    if suite.id.trim().is_empty() || suite.cases.is_empty() {
        bail!("eval suite requires a non-empty id and at least one case");
    }
    let mut ids = BTreeSet::new();
    for case in &suite.cases {
        if case.id.trim().is_empty() || !ids.insert(case.id.as_str()) {
            bail!("eval case ids must be non-empty and unique: '{}'", case.id);
        }
        if case.prompt.trim().is_empty() && case.actions.is_empty() {
            bail!("eval case '{}' requires prompt or actions", case.id);
        }
        if mode == EvalMode::Offline && case.script.is_empty() {
            bail!("offline eval case '{}' requires a script", case.id);
        }
        for path in case
            .files
            .keys()
            .chain(case.base64_files.keys())
            .chain(case.expect.expected_files.keys())
            .chain(case.expect.deleted_files.iter())
        {
            validate_relative_path(path)
                .with_context(|| format!("invalid path in eval case '{}'", case.id))?;
        }
        if !matches!(case.expect.outcome.as_str(), "success" | "error") {
            bail!("eval case '{}' outcome must be success or error", case.id);
        }
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<()> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        bail!("path must be a non-empty relative path");
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("path may not escape the eval workspace");
    }
    Ok(())
}

fn select_cases<'a>(suite: &'a EvalSuite, options: &EvalOptions) -> Result<Vec<&'a EvalCase>> {
    let known: BTreeSet<_> = suite.cases.iter().map(|case| case.id.as_str()).collect();
    for requested in &options.cases {
        if !known.contains(requested.as_str()) {
            bail!("unknown eval case '{requested}'");
        }
    }
    Ok(suite
        .cases
        .iter()
        .filter(|case| {
            (options.cases.is_empty() || options.cases.iter().any(|id| id == &case.id))
                && (options.tags.is_empty()
                    || options
                        .tags
                        .iter()
                        .all(|tag| case.tags.iter().any(|candidate| candidate == tag)))
        })
        .collect())
}

async fn run_case(
    case: EvalCase,
    repetition: usize,
    defaults: EvalLimits,
    live_config: Option<ProviderConfig>,
    options: EvalOptions,
) -> Result<ScenarioResult> {
    eprintln!(
        "[{} #{}] {} — {}",
        options.mode.as_str(),
        repetition,
        case.id,
        case.description
    );
    let limits = defaults.merged(&case.limits, &options);
    let mut workspace = TempWorkspace::new(&case.id, repetition)?;
    setup_workspace(&case, workspace.path())?;
    let before = snapshot_workspace(workspace.path())?;
    let output = Arc::new(EvalOutput::new(&case.approval, case.plan_mode)?);
    let provider_source = match options.mode {
        EvalMode::Offline => ProviderSource::Offline(ScriptedProvider::new(
            format!("scripted:{}", case.id),
            case.script.clone(),
        )),
        EvalMode::Live => ProviderSource::Live(
            live_config
                .clone()
                .context("live eval provider was not configured")?,
        ),
    };
    let vision_source = match options.mode {
        EvalMode::Offline if !case.vision_script.is_empty() => {
            Some(ProviderSource::Offline(ScriptedProvider::new(
                format!("scripted-vision:{}", case.id),
                case.vision_script.clone(),
            )))
        }
        _ => None,
    };
    let max_context = limits.max_context_tokens.unwrap_or(DEFAULT_MAX_CONTEXT);
    let max_rounds = limits.max_rounds.unwrap_or(DEFAULT_MAX_ROUNDS);
    let mut agent = build_agent(
        &case,
        workspace.path(),
        &provider_source,
        vision_source.as_ref(),
        max_context,
        max_rounds,
    )?;
    if let Some(enabled) = case.auto_compact {
        agent.set_auto_compact(enabled);
    }
    seed_context(&mut agent.ctx, &case.context_seed)?;

    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(delay) = case.cancel_after_ms {
        let cancel = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay)).await;
            cancel.store(true, Ordering::Relaxed);
        });
    }
    let actions = if case.actions.is_empty() {
        vec![EvalAction::Send {
            prompt: case.prompt.clone(),
            guidance: Vec::new(),
            allow_error: false,
        }]
    } else {
        case.actions.clone()
    };
    let started = Instant::now();
    let timeout = Duration::from_millis(limits.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
    let execution = tokio::time::timeout(
        timeout,
        run_actions(
            &case,
            &actions,
            &mut agent,
            &provider_source,
            vision_source.as_ref(),
            max_context,
            max_rounds,
            output.as_ref(),
            cancel.as_ref(),
        ),
    )
    .await;
    let agent_error = match execution {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => {
            cancel.store(true, Ordering::Relaxed);
            Some(format!(
                "scenario timed out after {} ms",
                timeout.as_millis()
            ))
        }
    };
    let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let captured = output.snapshot();
    let after = snapshot_workspace(workspace.path())?;
    let cost_microusd = calculate_cost(&captured, &options);
    let mut failures = verify_case(
        &case,
        &before,
        &after,
        &captured,
        agent_error.as_deref(),
        provider_source.offline_snapshot().as_ref(),
    );
    apply_limits(
        &limits,
        &captured,
        duration_ms,
        cost_microusd,
        &mut failures,
    );

    let trajectory_path = if let Some(artifacts) = &options.artifacts {
        Some(write_trajectory(
            artifacts,
            &case,
            repetition,
            &captured,
            provider_source.offline_snapshot().as_ref(),
            vision_source
                .as_ref()
                .and_then(ProviderSource::offline_snapshot)
                .as_ref(),
        )?)
    } else {
        None
    };
    let failed_workspace = if !failures.is_empty() && options.keep_failed_workspace {
        let artifacts = options
            .artifacts
            .as_ref()
            .expect("validated artifacts requirement");
        let target = artifacts.join("failed-workspaces").join(format!(
            "{}-{}-{}",
            safe_name(&case.id),
            repetition,
            uuid::Uuid::new_v4().simple()
        ));
        Some(workspace.preserve(&target)?.to_string_lossy().into_owned())
    } else {
        None
    };

    let passed = failures.is_empty();
    eprintln!(
        "  {} ({} ms, {} rounds, {} tool calls)",
        if passed { "pass" } else { "FAIL" },
        duration_ms,
        captured.rounds,
        captured.tool_calls.len()
    );
    Ok(ScenarioResult {
        id: case.id,
        description: case.description,
        tags: case.tags,
        repetition,
        passed,
        failures,
        duration_ms,
        rounds: captured.rounds,
        tool_calls: captured.tool_calls,
        tool_errors: captured.tool_errors,
        input_tokens: captured.input_tokens,
        output_tokens: captured.output_tokens,
        reasoning_tokens: captured.reasoning_tokens,
        cached_tokens: captured.cached_tokens,
        cost_microusd,
        completion: captured.completion,
        agent_error,
        trajectory_path: trajectory_path.map(|path| path.to_string_lossy().into_owned()),
        failed_workspace,
    })
}

fn setup_workspace(case: &EvalCase, root: &Path) -> Result<()> {
    for (path, contents) in &case.files {
        write_fixture(root, path, contents.as_bytes())?;
    }
    for (path, encoded) in &case.base64_files {
        let bytes = decode_base64(encoded)
            .with_context(|| format!("invalid base64 fixture '{path}' in case '{}'", case.id))?;
        write_fixture(root, path, &bytes)?;
    }
    Ok(())
}

fn decode_base64(value: &str) -> Result<Vec<u8>> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut inverse = [u8::MAX; 256];
    for (index, byte) in ALPHABET.iter().enumerate() {
        inverse[*byte as usize] = index as u8;
    }
    let clean: Vec<_> = value
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if clean.len() % 4 != 0 {
        bail!("base64 length is not divisible by four");
    }
    let mut out = Vec::new();
    for chunk in clean.chunks(4) {
        let mut numbers = [0u8; 4];
        let mut padding = 0;
        for (index, byte) in chunk.iter().enumerate() {
            if *byte == b'=' {
                padding += 1;
                numbers[index] = 0;
            } else {
                let decoded = inverse[*byte as usize];
                if decoded == u8::MAX {
                    bail!("invalid base64 character");
                }
                numbers[index] = decoded;
            }
        }
        out.push((numbers[0] << 2) | (numbers[1] >> 4));
        if padding < 2 {
            out.push((numbers[1] << 4) | (numbers[2] >> 2));
        }
        if padding == 0 {
            out.push((numbers[2] << 6) | numbers[3]);
        }
    }
    Ok(out)
}

fn build_agent(
    case: &EvalCase,
    root: &Path,
    provider: &ProviderSource,
    vision: Option<&ProviderSource>,
    max_context: usize,
    max_rounds: usize,
) -> Result<Agent> {
    let skill_paths = vec![root.join(".wisp").join("skills")];
    let skills = Arc::new(SkillIndex::load(&skill_paths));
    let memory = Arc::new(MemoryManager::new(root));
    let mut registry = wisp_core::build_registry(skills.clone(), memory, false);
    registry.add(Box::new(wisp_tools::ask_user::AskUserTool));
    for (name, result) in &case.fixture_mcp {
        registry.add(Box::new(FixtureMcpTool {
            name: name.clone(),
            result: result.clone(),
        }));
    }
    if !case.explore_script.is_empty() {
        let provider = Arc::new(ScriptedProvider::new(
            "scripted-explore-v1",
            case.explore_script.clone(),
        ));
        registry.add(Box::new(ExploreTool::new(provider, max_context)));
    }
    if case.fixture_runtimes {
        let manager = wisp_runtime::RuntimeManager::new(Arc::new(EvalRuntimeLauncher));
        let project_id = root.to_string_lossy().into_owned();
        registry.add(Box::new(wisp_runtime::ReplTool::new(
            manager.clone(),
            &project_id,
        )));
        registry.add(Box::new(wisp_runtime::RTool::new(manager, project_id)));
    }
    if !case.allowed_tools.is_empty() {
        registry = registry.filtered(&case.allowed_tools);
    }
    let mut agent = Agent::with_provider(
        provider.build(),
        vision.map(ProviderSource::build),
        registry,
        root.to_path_buf(),
        max_context,
        max_rounds,
    );
    agent.seed_system_prompt(&skills, None);
    Ok(agent)
}

fn seed_context(ctx: &mut ContextManager, seeds: &[SeedMessage]) -> Result<()> {
    for seed in seeds {
        if seed.repeat == 0 {
            bail!("context seed repeat must be at least 1");
        }
        let content = seed.content.repeat(seed.repeat);
        match seed.role.as_str() {
            "user" => ctx.append_user(&content),
            "assistant" => ctx.append_assistant(content, vec![], None),
            "system" => ctx.append_system(&content),
            other => bail!("unsupported context seed role '{other}'"),
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_actions(
    case: &EvalCase,
    actions: &[EvalAction],
    agent: &mut Agent,
    provider: &ProviderSource,
    vision: Option<&ProviderSource>,
    max_context: usize,
    max_rounds: usize,
    output: &EvalOutput,
    cancel: &AtomicBool,
) -> Result<()> {
    for action in actions {
        match action {
            EvalAction::Send {
                prompt,
                guidance,
                allow_error,
            } => {
                let queue: GuidanceQueue = Mutex::new(
                    guidance
                        .iter()
                        .enumerate()
                        .map(|(index, value)| (index as u64 + 1, value.clone()))
                        .collect(),
                );
                let result = agent
                    .run_with_images(
                        prompt,
                        &[],
                        false,
                        output,
                        Some(cancel),
                        (!guidance.is_empty()).then_some(&queue),
                    )
                    .await;
                if let Err(error) = result {
                    output.push(
                        "action_error",
                        None,
                        None,
                        Some(false),
                        Some(json!({"action": "send", "error": error.to_string()})),
                    );
                    if !allow_error {
                        return Err(error);
                    }
                    cancel.store(false, Ordering::Relaxed);
                }
                agent.save();
            }
            EvalAction::Resume { allow_error } => {
                let result = agent.run_resume(output, Some(cancel), None).await;
                if let Err(error) = result {
                    output.push(
                        "action_error",
                        None,
                        None,
                        Some(false),
                        Some(json!({"action": "resume", "error": error.to_string()})),
                    );
                    if !allow_error {
                        return Err(error);
                    }
                    cancel.store(false, Ordering::Relaxed);
                }
                agent.save();
            }
            EvalAction::Compact => {
                output.compaction_started("manual");
                let (before, after, _) = agent.compact().await.map_err(anyhow::Error::msg)?;
                output.compaction(before, after, "manual");
                agent.save();
            }
            EvalAction::Restart => {
                agent.save();
                *agent = build_agent(case, &agent.root, provider, vision, max_context, max_rounds)?;
                if let Some(enabled) = case.auto_compact {
                    agent.set_auto_compact(enabled);
                }
                output.push("restart", None, None, Some(true), None);
            }
        }
    }
    Ok(())
}

fn verify_case(
    case: &EvalCase,
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
    captured: &Captured,
    agent_error: Option<&str>,
    provider: Option<&ScriptedProviderSnapshot>,
) -> Vec<String> {
    let mut failures = Vec::new();
    match case.expect.outcome.as_str() {
        "success" if agent_error.is_some() => failures.push(format!(
            "agent returned an unexpected error: {}",
            agent_error.unwrap_or_default()
        )),
        "error" if agent_error.is_none() => {
            failures.push("agent succeeded but an error was expected".into())
        }
        _ => {}
    }
    if let Some(error) = agent_error {
        for fragment in &case.expect.error_contains {
            if !contains_folded(error, fragment) {
                failures.push(format!("agent error did not contain '{fragment}'"));
            }
        }
    }
    for fragment in &case.expect.completion_contains {
        if !captured
            .completion
            .as_deref()
            .is_some_and(|completion| contains_folded(completion, fragment))
        {
            failures.push(format!("completion did not contain '{fragment}'"));
        }
    }
    for tool in &case.expect.required_tools {
        if !captured.tool_calls.iter().any(|call| &call.name == tool) {
            failures.push(format!("required tool '{tool}' was not called"));
        }
    }
    for tool in &case.expect.forbidden_tools {
        if captured.tool_calls.iter().any(|call| &call.name == tool) {
            failures.push(format!("forbidden tool '{tool}' was called"));
        }
    }
    if !case.expect.tool_order.is_empty() {
        let mut cursor = 0;
        for call in &captured.tool_calls {
            if case.expect.tool_order.get(cursor) == Some(&call.name) {
                cursor += 1;
            }
        }
        if cursor != case.expect.tool_order.len() {
            failures.push(format!(
                "required tool order {:?} was not observed in {:?}",
                case.expect.tool_order,
                captured
                    .tool_calls
                    .iter()
                    .map(|call| call.name.as_str())
                    .collect::<Vec<_>>()
            ));
        }
    }
    for expectation in &case.expect.tool_args {
        let matched = captured
            .tool_calls
            .iter()
            .filter(|call| call.name == expectation.name)
            .any(|call| {
                let actual = if expectation.pointer.is_empty() {
                    Some(&call.arguments)
                } else {
                    call.arguments.pointer(&expectation.pointer)
                };
                actual == Some(&expectation.equals)
            });
        if !matched {
            failures.push(format!(
                "tool '{}' never had {} = {}",
                expectation.name,
                if expectation.pointer.is_empty() {
                    "arguments"
                } else {
                    &expectation.pointer
                },
                expectation.equals
            ));
        }
    }
    if let Some(expected) = case.expect.approvals {
        if captured.approvals.len() != expected {
            failures.push(format!(
                "expected {expected} approval request(s), observed {}",
                captured.approvals.len()
            ));
        }
    }
    if let Some(provider) = provider {
        for fragment in &case.expect.request_contains {
            let found = provider.requests.iter().any(|request| {
                request.messages.iter().any(|message| {
                    contains_folded(&message.content.as_text(), fragment)
                        || message
                            .reasoning
                            .as_deref()
                            .is_some_and(|value| contains_folded(value, fragment))
                })
            });
            if !found {
                failures.push(format!("no provider request contained '{fragment}'"));
            }
        }
        let expected_remaining = case.expect.remaining_script.unwrap_or(0);
        if provider.remaining_completions != expected_remaining {
            failures.push(format!(
                "script has {} completion(s) remaining; expected {expected_remaining}",
                provider.remaining_completions
            ));
        }
    }
    let mut expected = before.clone();
    for path in &case.expect.deleted_files {
        expected.remove(path);
    }
    for (path, contents) in &case.expect.expected_files {
        expected.insert(path.clone(), contents.as_bytes().to_vec());
    }
    if &expected != after {
        let paths: BTreeSet<_> = expected
            .keys()
            .chain(after.keys())
            .filter(|path| expected.get(*path) != after.get(*path))
            .cloned()
            .collect();
        failures.push(format!(
            "workspace differed at: {}",
            paths.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    failures
}

fn contains_folded(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .replace('\\', "/")
        .contains(&needle.to_ascii_lowercase().replace('\\', "/"))
}

fn apply_limits(
    limits: &EvalLimits,
    captured: &Captured,
    duration_ms: u64,
    cost_microusd: u64,
    failures: &mut Vec<String>,
) {
    for (label, actual, maximum) in [
        (
            "rounds",
            captured.rounds as u64,
            limits.max_rounds.map(|value| value as u64),
        ),
        (
            "tool calls",
            captured.tool_calls.len() as u64,
            limits.max_tool_calls,
        ),
        ("tool errors", captured.tool_errors, limits.max_tool_errors),
        (
            "input tokens",
            captured.input_tokens,
            limits.max_input_tokens,
        ),
        ("duration ms", duration_ms, limits.max_duration_ms),
        ("cost microusd", cost_microusd, limits.max_cost_microusd),
    ] {
        if maximum.is_some_and(|maximum| actual > maximum) {
            failures.push(format!(
                "{label} exceeded limit: {actual} > {}",
                maximum.unwrap_or_default()
            ));
        }
    }
}

fn calculate_cost(captured: &Captured, options: &EvalOptions) -> u64 {
    fn component(tokens: u64, rate: u64) -> u128 {
        u128::from(tokens)
            .saturating_mul(u128::from(rate))
            .div_ceil(1_000_000)
    }
    component(
        captured.input_tokens.saturating_sub(captured.cached_tokens),
        options.input_cost_microusd_per_million,
    )
    .saturating_add(component(
        captured.output_tokens,
        options.output_cost_microusd_per_million,
    ))
    .saturating_add(component(
        captured.reasoning_tokens,
        options.reasoning_cost_microusd_per_million,
    ))
    .min(u128::from(u64::MAX)) as u64
}

fn write_trajectory(
    artifacts: &Path,
    case: &EvalCase,
    repetition: usize,
    captured: &Captured,
    provider: Option<&ScriptedProviderSnapshot>,
    vision_provider: Option<&ScriptedProviderSnapshot>,
) -> Result<PathBuf> {
    let dir = artifacts.join("trajectories");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}-{repetition}.jsonl", safe_name(&case.id)));
    let mut lines = String::new();
    lines.push_str(&serde_json::to_string(&json!({
        "schema": TRAJECTORY_SCHEMA,
        "sequence": 0,
        "kind": "metadata",
        "case_id": case.id,
        "repetition": repetition,
        "provider_requests": provider,
        "vision_provider_requests": vision_provider,
    }))?);
    lines.push('\n');
    for event in &captured.events {
        lines.push_str(&serde_json::to_string(event)?);
        lines.push('\n');
    }
    std::fs::write(&path, lines)?;
    Ok(path)
}

fn summarize(results: &[ScenarioResult]) -> ReportSummary {
    let passed = results.iter().filter(|result| result.passed).count();
    ReportSummary {
        cases: results
            .iter()
            .map(|result| result.id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        attempts: results.len(),
        passed,
        pass_rate_percent: if results.is_empty() {
            0
        } else {
            (passed as u64 * 100) / results.len() as u64
        },
        duration_ms: results.iter().map(|result| result.duration_ms).sum(),
        rounds: results.iter().map(|result| result.rounds).sum(),
        tool_calls: results
            .iter()
            .map(|result| result.tool_calls.len() as u64)
            .sum(),
        tool_errors: results.iter().map(|result| result.tool_errors).sum(),
        input_tokens: results.iter().map(|result| result.input_tokens).sum(),
        output_tokens: results.iter().map(|result| result.output_tokens).sum(),
        reasoning_tokens: results.iter().map(|result| result.reasoning_tokens).sum(),
        cached_tokens: results.iter().map(|result| result.cached_tokens).sum(),
        cost_microusd: results.iter().map(|result| result.cost_microusd).sum(),
    }
}

fn compare_reports(
    current: &EvalReport,
    baseline: &EvalReport,
    path: &Path,
    options: &EvalOptions,
) -> BaselineComparison {
    let mut warnings = Vec::new();
    for (label, current_value, baseline_value) in [
        (
            "report schema",
            current.schema.as_str(),
            baseline.schema.as_str(),
        ),
        (
            "suite",
            current.suite_id.as_str(),
            baseline.suite_id.as_str(),
        ),
        (
            "suite hash",
            current.suite_hash.as_str(),
            baseline.suite_hash.as_str(),
        ),
        ("mode", current.mode.as_str(), baseline.mode.as_str()),
        (
            "provider",
            current.provider.as_str(),
            baseline.provider.as_str(),
        ),
        ("model", current.model.as_str(), baseline.model.as_str()),
    ] {
        if current_value != baseline_value {
            warnings.push(format!("{label} differs"));
        }
    }
    let baseline_passes: HashMap<_, _> = baseline
        .scenarios
        .iter()
        .map(|result| ((result.id.clone(), result.repetition), result.passed))
        .collect();
    let current_passes: HashMap<_, _> = current
        .scenarios
        .iter()
        .map(|result| ((result.id.clone(), result.repetition), result.passed))
        .collect();
    let regressions = baseline_passes
        .iter()
        .filter(|(key, passed)| **passed && current_passes.get(*key) == Some(&false))
        .map(|((id, repetition), _)| format!("{id}#{repetition}"))
        .collect();
    let improvements = baseline_passes
        .iter()
        .filter(|(key, passed)| !**passed && current_passes.get(*key) == Some(&true))
        .map(|((id, repetition), _)| format!("{id}#{repetition}"))
        .collect();
    let mut threshold_failures = Vec::new();
    if let Some(percent) = options.max_token_regression_percent {
        let allowed = baseline.summary.input_tokens.saturating_mul(100 + percent) / 100;
        if current.summary.input_tokens > allowed {
            threshold_failures.push(format!(
                "input tokens regressed by more than {percent}%: {} > {allowed}",
                current.summary.input_tokens
            ));
        }
    }
    if let Some(delta) = options.max_round_regression {
        let allowed = baseline.summary.rounds.saturating_add(delta as usize);
        if current.summary.rounds > allowed {
            threshold_failures.push(format!(
                "rounds regressed by more than {delta}: {} > {allowed}",
                current.summary.rounds
            ));
        }
    }
    BaselineComparison {
        baseline: path.to_string_lossy().into_owned(),
        warnings,
        regressions,
        improvements,
        threshold_failures,
        passed_delta: delta(
            current.summary.passed as u64,
            baseline.summary.passed as u64,
        ),
        input_tokens_delta: delta(current.summary.input_tokens, baseline.summary.input_tokens),
        rounds_delta: delta(
            current.summary.rounds as u64,
            baseline.summary.rounds as u64,
        ),
        duration_ms_delta: delta(current.summary.duration_ms, baseline.summary.duration_ms),
    }
}

fn delta(current: u64, baseline: u64) -> i64 {
    i128::from(current)
        .saturating_sub(i128::from(baseline))
        .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

fn write_fixture(root: &Path, relative: &str, contents: &[u8]) -> Result<()> {
    validate_relative_path(relative)?;
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

fn snapshot_workspace(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    fn visit(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(root).expect("entry is under eval root");
            if relative
                .components()
                .next()
                .is_some_and(|part| part.as_os_str() == ".wisp")
            {
                continue;
            }
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                visit(root, &path, files)?;
            } else if file_type.is_file() {
                let key = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                files.insert(key, std::fs::read(path)?);
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}

fn safe_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

static WORKSPACE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TempWorkspace {
    path: PathBuf,
    base: PathBuf,
    preserved: bool,
}

impl TempWorkspace {
    fn new(id: &str, repetition: usize) -> Result<Self> {
        let base = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = WORKSPACE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!(
            "wisp-agent-eval-{}-{nonce}-{sequence}-{}-{repetition}",
            std::process::id(),
            safe_name(id)
        ));
        std::fs::create_dir(&path)
            .with_context(|| format!("could not create eval workspace {}", path.display()))?;
        Ok(Self {
            path,
            base,
            preserved: false,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn preserve(&mut self, target: &Path) -> Result<PathBuf> {
        if target.exists() {
            bail!(
                "refusing to overwrite preserved eval workspace {}",
                target.display()
            );
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&self.path, target).with_context(|| {
            format!(
                "could not preserve failed workspace {} as {}",
                self.path.display(),
                target.display()
            )
        })?;
        self.preserved = true;
        Ok(target.to_path_buf())
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        if self.preserved {
            return;
        }
        let is_eval_dir = self.path.parent() == Some(self.base.as_path())
            && self
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("wisp-agent-eval-"));
        if is_eval_dir {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_suite_is_valid_and_covers_p0_p1_domains() {
        let suite: EvalSuite = serde_yaml::from_str(BUILTIN_SUITE).unwrap();
        validate_suite(&suite, EvalMode::Offline).unwrap();
        let tags: BTreeSet<_> = suite
            .cases
            .iter()
            .flat_map(|case| case.tags.iter().map(String::as_str))
            .collect();
        for required in [
            "filesystem",
            "shell",
            "skills",
            "mcp",
            "approval",
            "resume",
            "session",
            "guidance",
            "cancel",
            "vision",
            "compaction",
            "delegation",
            "python",
            "r",
        ] {
            assert!(
                tags.contains(required),
                "missing built-in coverage tag {required}"
            );
        }
    }

    #[test]
    fn unsafe_fixture_paths_are_rejected() {
        for path in ["", "../escape", "/absolute", "a/../../escape"] {
            assert!(validate_relative_path(path).is_err(), "{path} should fail");
        }
        assert!(validate_relative_path("results/summary.txt").is_ok());
    }

    #[test]
    fn cost_uses_uncached_input_and_declared_rates() {
        let captured = Captured {
            input_tokens: 2_000_000,
            cached_tokens: 1_000_000,
            output_tokens: 500_000,
            reasoning_tokens: 250_000,
            ..Captured::default()
        };
        let options = EvalOptions {
            input_cost_microusd_per_million: 10,
            output_cost_microusd_per_million: 20,
            reasoning_cost_microusd_per_million: 40,
            ..EvalOptions::default()
        };
        assert_eq!(calculate_cost(&captured, &options), 30);
    }

    #[test]
    fn verifier_checks_tool_argument_json_pointer() {
        let case = EvalCase {
            id: "args".into(),
            description: "args".into(),
            tags: vec![],
            prompt: "args".into(),
            actions: vec![],
            files: BTreeMap::new(),
            base64_files: BTreeMap::new(),
            allowed_tools: vec![],
            script: vec![ScriptedCompletion::default()],
            vision_script: vec![],
            explore_script: vec![],
            fixture_runtimes: false,
            fixture_mcp: BTreeMap::new(),
            context_seed: vec![],
            approval: EvalApproval::default(),
            plan_mode: false,
            cancel_after_ms: None,
            auto_compact: None,
            limits: EvalLimits::default(),
            expect: EvalExpectation {
                tool_args: vec![ToolArgumentExpectation {
                    name: "read".into(),
                    pointer: "/path".into(),
                    equals: json!("notes.txt"),
                }],
                remaining_script: Some(0),
                ..EvalExpectation::default()
            },
        };
        let captured = Captured {
            tool_calls: vec![ToolCallRecord {
                call_id: "c1".into(),
                name: "read".into(),
                arguments: json!({"path": "notes.txt"}),
            }],
            ..Captured::default()
        };
        let provider = ScriptedProviderSnapshot {
            model: "fixture".into(),
            requests: vec![],
            remaining_completions: 0,
        };
        assert!(verify_case(
            &case,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &captured,
            None,
            Some(&provider)
        )
        .is_empty());
    }
}
