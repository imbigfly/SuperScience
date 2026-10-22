//! Bundled Cua Driver integration.
//!
//! On macOS the driver runs as an embedded daemon:
//! `cua-driver serve --embedded` is spawned directly by the app, so the daemon
//! inherits the app's Accessibility and Screen Recording grants. A
//! `cua-driver mcp --embedded --socket` proxy exposes that daemon over stdio.
//! On Windows and Linux the MCP server owns its runtime directly.
//!
//! Tools are registered with a `desktop_` prefix. Read-only tools stay silent;
//! tools that mutate desktop state require approval.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use superscience_mcp::{McpClient, McpTool, RemoteTool};
use superscience_tools::Registry;

#[cfg(target_os = "macos")]
const HOST_BUNDLE_ID: &str = "science.superscience";

/// Keeps the embedded daemon alive for the app's lifetime. The daemon owns
/// the TCC responsibility chain; dropping its `Child` would orphan it.
#[cfg(target_os = "macos")]
static DAEMON: std::sync::OnceLock<std::sync::Mutex<Option<std::process::Child>>> =
    std::sync::OnceLock::new();
#[cfg(target_os = "macos")]
static DAEMON_SOCKET: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub(crate) const CONNECTOR_ID: &str = "cua-driver";
pub(crate) const TOOL_PREFIX: &str = "desktop_";

fn driver_file_name() -> &'static str {
    if cfg!(windows) {
        "cua-driver.exe"
    } else {
        "cua-driver"
    }
}

/// Resolve the bundled driver without an `AppHandle`.
///
/// Tauri puts `externalBin` payloads next to the main executable and
/// `resources` under the resource directory. Development builds fall back to
/// `src-tauri/binaries/`, where `scripts/fetch_cua_driver.sh` places the file.
pub(crate) fn resolve_driver_path(resource_root: Option<&Path>) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("SUPERSCIENCE_CUA_DRIVER") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return Some(path);
        }
    }

    let name = driver_file_name();
    if let Some(root) = resource_root {
        let candidate = root.join("bin").join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in [
                dir.join(name),
                dir.join("bin").join(name),
                dir.join("resources").join("bin").join(name),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            // macOS app bundles keep resources under Contents/Resources.
            if let Some(contents) = dir.parent() {
                let candidate = contents.join("Resources").join("bin").join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(name);
    dev.is_file().then_some(dev)
}

/// Start (or reuse) the embedded daemon.
///
/// The daemon must be spawned directly by the host app, not through
/// LaunchServices, so macOS keeps it in the app's TCC responsibility chain.
/// `--embedded` disables the standalone driver's responsibility disclaimer,
/// which is what makes the daemon inherit the app's grants.
#[cfg(target_os = "macos")]
fn ensure_daemon(driver: &Path) -> Result<PathBuf> {
    let socket = DAEMON_SOCKET.get_or_init(|| {
        PathBuf::from(format!("/tmp/superscience-cua-{}.sock", std::process::id()))
    });
    let holder = DAEMON.get_or_init(|| std::sync::Mutex::new(None));
    let mut guard = holder
        .lock()
        .map_err(|_| anyhow!("cua-driver daemon lock is poisoned"))?;

    if let Some(child) = guard.as_mut() {
        let alive = child.try_wait().ok().flatten().is_none();
        if alive && socket.exists() {
            return Ok(socket.clone());
        }
    }

    let _ = std::fs::remove_file(socket);
    let log_path =
        std::env::temp_dir().join(format!("superscience-cua-{}.log", std::process::id()));
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();

    let mut cmd = std::process::Command::new(driver);
    cmd.arg("serve")
        .arg("--embedded")
        .arg("--socket")
        .arg(socket)
        .arg("--host-bundle-id")
        .arg(HOST_BUNDLE_ID)
        .arg("--permission-mode")
        .arg("standard")
        .env("CUA_DRIVER_EMBEDDED", "1")
        .env("CUA_DRIVER_HOST_BUNDLE_ID", HOST_BUNDLE_ID)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null());
    if let Some(log) = log {
        cmd.stderr(std::process::Stdio::from(log));
    } else {
        cmd.stderr(std::process::Stdio::null());
    }

    let child = cmd
        .spawn()
        .map_err(|error| anyhow!("spawn cua-driver daemon: {error}"))?;
    *guard = Some(child);

    for _ in 0..100 {
        if socket.exists() {
            return Ok(socket.clone());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(anyhow!(
        "cua-driver daemon did not create {}; see {}",
        socket.display(),
        log_path.display()
    ))
}

/// Launch the bundled driver's MCP endpoint.
pub(crate) async fn connect(resource_root: Option<&Path>) -> Result<Arc<McpClient>> {
    let path = resolve_driver_path(resource_root)
        .ok_or_else(|| anyhow!("bundled cua-driver binary was not found"))?;

    #[cfg(target_os = "macos")]
    {
        // The daemon inherits the app's TCC grants; the stdio MCP process is
        // only a proxy, so it never owns or prompts for permissions.
        let socket = ensure_daemon(&path)?;
        let mut cmd = tokio::process::Command::new(&path);
        cmd.arg("mcp")
            .arg("--embedded")
            .arg("--socket")
            .arg(&socket)
            .env("CUA_DRIVER_EMBEDDED", "1")
            .env("CUA_DRIVER_HOST_BUNDLE_ID", HOST_BUNDLE_ID);
        superscience_tools::process::hide_console_async(&mut cmd);
        let client = McpClient::launch_with_command(cmd).await?;
        Ok(Arc::new(client))
    }

    #[cfg(not(target_os = "macos"))]
    {
        let mut cmd = tokio::process::Command::new(&path);
        cmd.arg("mcp");
        cmd.env("CUA_DRIVER_PERMISSION_MODE", "standard");
        superscience_tools::process::hide_console_async(&mut cmd);
        let client = McpClient::launch_with_command(cmd).await?;
        Ok(Arc::new(client))
    }
}

/// Whether a tool needs an approval card.
///
/// The MCP server's `readOnlyHint` is authoritative: observation tools stay
/// silent, everything unclassified or mutating asks first.
fn requires_approval(tool: &RemoteTool) -> bool {
    !tool.read_only()
}

fn exposed_name(remote: &str) -> String {
    format!("{TOOL_PREFIX}{remote}")
}

/// Register the driver's model-visible tools.
pub(crate) async fn register(
    registry: &mut Registry,
    client: Arc<McpClient>,
) -> Result<Vec<String>, String> {
    let tools = client
        .tools_list()
        .await
        .map_err(|error| format!("cua-driver tools/list: {error}"))?;
    let catalog = Arc::new(tools);
    let mut names = Vec::new();

    for tool in catalog.iter().filter(|tool| tool.visible_to_model()) {
        let name = exposed_name(&tool.name);
        if registry.get(&name).is_some() {
            return Err(format!("desktop tool name collision: {name}"));
        }
        names.push(name.clone());
        let wrapper = if requires_approval(tool) {
            McpTool::with_catalog_named_requiring_approval(
                tool.clone(),
                &name,
                Arc::clone(&client),
                CONNECTOR_ID,
                Arc::clone(&catalog),
            )
        } else {
            McpTool::with_catalog_named(
                tool.clone(),
                &name,
                Arc::clone(&client),
                CONNECTOR_ID,
                Arc::clone(&catalog),
            )
        };
        registry.add(Box::new(wrapper));
    }

    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn remote(name: &str, read_only: bool) -> RemoteTool {
        RemoteTool {
            name: name.into(),
            title: None,
            description: String::new(),
            input_schema: json!({ "type": "object" }),
            output_schema: None,
            meta: None,
            annotations: Some(json!({ "readOnlyHint": read_only })),
        }
    }

    #[test]
    fn prefixes_tool_names() {
        assert_eq!(exposed_name("click"), "desktop_click");
        assert_eq!(exposed_name("get_window_state"), "desktop_get_window_state");
    }

    #[test]
    fn read_only_tools_skip_approval() {
        assert!(!requires_approval(&remote("get_window_state", true)));
        assert!(requires_approval(&remote("click", false)));
        // An absent hint is unclassified, not read-only.
        let mut unclassified = remote("launch", false);
        unclassified.annotations = None;
        assert!(requires_approval(&unclassified));
    }
}
