//! Managed project-scoped language runtimes and agent tool adapters.

pub mod env;
pub mod kernel;
pub mod manager;
pub mod provision;
pub mod tool;

pub use env::{
    bundled_mock_mcp_path, bundled_r_worker_path, bundled_worker_path, conda_prefix_envs,
    direct_rscript, find_rscript, find_rscript_for_app, resolve_bundled_script, PythonEnv, REnv,
};
pub use kernel::{
    KernelClient, KernelReady, KernelResp, KernelWriteScope, MAX_CODE_BYTES, PROTOCOL_VERSION,
};
pub use manager::{
    LaunchedRuntime, RuntimeEvent, RuntimeExecution, RuntimeExecutionOptions, RuntimeInfo,
    RuntimeKernel, RuntimeKey, RuntimeLanguage, RuntimeLauncher, RuntimeManager, RuntimeMetadata,
    RuntimeObject, RuntimeObjectList, RuntimeOutput, RuntimeStatus, LOCAL_CONTEXT_ID,
    MAINLINE_RUNTIME_SCOPE,
};
pub use tool::{
    format_response, format_script_response, read_project_script, ProjectScript, RTool, ReplTool,
    ScriptProvenance,
};
pub use provision::{
    auto_items_satisfied, probe_local_runtimes, run_provision, FakeHost, ProvisionHost,
    ProvisionItem, RealHost, RuntimeProvisionState,
};
