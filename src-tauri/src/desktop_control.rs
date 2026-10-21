//! Bundled Cua Driver integration.
//!
//! Cua Driver ships as a sidecar MCP server. The app resolves the bundled
//! binary, launches `cua-driver mcp --direct`, and registers its tools with a
//! `desktop_` prefix. Read-only tools stay silent; tools that mutate desktop
//! state require approval.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use superscience_mcp::{McpClient, McpTool, RemoteTool};
use superscience_tools::Registry;

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

/// Launch the bundled driver's stdio MCP server.
///
/// `--direct` makes macOS attribute Accessibility and Screen Recording grants
/// to the host app (the Tauri process) instead of a separately installed
/// helper bundle. The process inherits no ambient secret environment.
pub(crate) async fn connect(resource_root: Option<&Path>) -> Result<Arc<McpClient>> {
    let path = resolve_driver_path(resource_root)
        .ok_or_else(|| anyhow!("bundled cua-driver binary was not found"))?;
    let mut cmd = tokio::process::Command::new(&path);
    cmd.arg("mcp");
    // `--direct` is the macOS TCC attribution mode: the host app owns the
    // Accessibility/Screen Recording grants instead of a separate helper app.
    #[cfg(target_os = "macos")]
    cmd.arg("--direct");
    cmd.env("CUA_DRIVER_PERMISSION_MODE", "standard");
    superscience_tools::process::hide_console_async(&mut cmd);
    let client = McpClient::launch_with_command(cmd).await?;
    Ok(Arc::new(client))
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
