//! First-run / on-demand local runtime provisioning.
//!
//! Probe existing tools, then install only what is missing. Real installs go
//! through [`RealHost`]; tests use [`FakeHost`] so they never hit the network.

use crate::env::{find_rscript, find_rscript_for_app, PythonEnv, REnv};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

pub const AUTO_ITEM_IDS: &[&str] = &["uv", "python", "r", "node", "sci", "pixi", "officecli"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvisionItem {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeProvisionState {
    pub show: bool,
    pub done: bool,
    pub running: bool,
    pub items: Vec<ProvisionItem>,
}

pub type ProgressFn = dyn Fn(&str, &str, u64, Option<u64>) + Send + Sync;

pub trait ProvisionHost: Send + Sync {
    fn find_uv(&self) -> Option<PathBuf>;
    fn find_node(&self) -> Option<PathBuf>;
    fn find_npm(&self) -> Option<PathBuf>;
    fn find_sci(&self) -> Option<PathBuf>;
    fn find_pixi(&self) -> Option<PathBuf>;
    fn find_officecli(&self) -> Option<PathBuf>;
    fn find_rscript(&self, app_data: &Path) -> Option<PathBuf>;
    fn python_ready(&self, app_data: &Path) -> bool;
    fn jsonlite_ok(&self, rscript: &Path) -> bool;
    fn node_major(&self) -> Option<u32>;
    fn has_sci_key(&self) -> bool;

    fn install_uv(&self, cancel: &AtomicBool) -> Result<()>;
    fn ensure_python(&self, app_data: &Path, cancel: &AtomicBool) -> Result<()>;
    fn install_r(&self, app_data: &Path, cancel: &AtomicBool) -> Result<PathBuf>;
    fn install_jsonlite(&self, rscript: &Path, cancel: &AtomicBool) -> Result<()>;
    fn install_node(&self, cancel: &AtomicBool) -> Result<()>;
    fn install_sci(&self, cancel: &AtomicBool) -> Result<()>;
    fn install_pixi(&self, cancel: &AtomicBool) -> Result<()>;
    fn install_officecli(&self, cancel: &AtomicBool) -> Result<()>;
}

pub struct RealHost {
    pub has_sci_key: bool,
}

impl Default for RealHost {
    fn default() -> Self {
        Self { has_sci_key: false }
    }
}

impl ProvisionHost for RealHost {
    fn find_uv(&self) -> Option<PathBuf> {
        PythonEnv::find_uv()
    }
    fn find_node(&self) -> Option<PathBuf> {
        PythonEnv::find_node()
    }
    fn find_npm(&self) -> Option<PathBuf> {
        PythonEnv::find_npm()
    }
    fn find_sci(&self) -> Option<PathBuf> {
        PythonEnv::find_sci()
    }
    fn find_pixi(&self) -> Option<PathBuf> {
        PythonEnv::find_pixi()
    }
    fn find_officecli(&self) -> Option<PathBuf> {
        PythonEnv::find_officecli()
    }
    fn find_rscript(&self, app_data: &Path) -> Option<PathBuf> {
        find_rscript_for_app(app_data)
    }
    fn python_ready(&self, app_data: &Path) -> bool {
        PythonEnv::managed(app_data).python().is_file()
            && PythonEnv::managed(app_data)
                .venv
                .join(".superscience_deps_ok")
                .is_file()
    }
    fn jsonlite_ok(&self, rscript: &Path) -> bool {
        r_has_jsonlite(rscript)
    }
    fn node_major(&self) -> Option<u32> {
        node_major_version(self.find_node()?.as_path())
    }
    fn has_sci_key(&self) -> bool {
        self.has_sci_key
    }

    fn install_uv(&self, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)?;
        run_platform_installer(
            "curl -LsSf https://astral.sh/uv/install.sh | sh",
            "irm https://astral.sh/uv/install.ps1 | iex",
        )
    }
    fn ensure_python(&self, app_data: &Path, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)?;
        PythonEnv::ensure(app_data).map(|_| ())
    }
    fn install_r(&self, app_data: &Path, cancel: &AtomicBool) -> Result<PathBuf> {
        check_cancel(cancel)?;
        if let Some(existing) = find_rscript_for_app(app_data) {
            return Ok(existing);
        }
        install_system_r()?;
        find_rscript()
            .or_else(|| REnv::managed(app_data).rscript())
            .ok_or_else(|| anyhow!("Rscript still missing after install"))
    }
    fn install_jsonlite(&self, rscript: &Path, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)?;
        if r_has_jsonlite(rscript) {
            return Ok(());
        }
        let repos = if prefer_mainland_mirrors() {
            "https://mirrors.tuna.tsinghua.edu.cn/CRAN/"
        } else {
            "https://cloud.r-project.org"
        };
        let expr = format!("install.packages('jsonlite', repos='{repos}')");
        run_cmd(rscript, &["--vanilla", "-e", &expr])
    }
    fn install_node(&self, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)?;
        install_system_node()
    }
    fn install_sci(&self, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)?;
        let npm = PythonEnv::find_npm().ok_or_else(|| anyhow!("npm not found"))?;
        if prefer_mainland_mirrors() {
            let _ = run_cmd(
                &npm,
                &[
                    "config",
                    "set",
                    "registry",
                    "https://registry.npmmirror.com",
                ],
            );
        }
        run_cmd(&npm, &["install", "-g", "scimaster-cli"])
    }
    fn install_pixi(&self, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)?;
        run_platform_installer(
            "curl -fsSL https://pixi.sh/install.sh | bash",
            "irm -useb https://pixi.sh/install.ps1 | iex",
        )
    }
    fn install_officecli(&self, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)?;
        run_platform_installer(
            "curl -fsSL https://d.officecli.ai/install.sh | bash",
            "irm https://d.officecli.ai/install.ps1 | iex",
        )
    }
}

pub fn prefer_mainland_mirrors() -> bool {
    matches!(
        std::env::var("SUPERSCIENCE_USE_MIRRORS").ok().as_deref(),
        Some("1") | Some("true")
    ) || matches!(
        std::env::var("TZ").ok().as_deref(),
        Some("Asia/Shanghai" | "Asia/Chongqing" | "Asia/Urumqi")
    )
}

pub fn probe_local_runtimes(app_data: &Path, host: &dyn ProvisionHost) -> Vec<ProvisionItem> {
    let mut items = Vec::new();
    push_tool(
        &mut items,
        "uv",
        host.find_uv().is_some(),
        "uv is required to create the managed Python environment",
    );
    push_tool(
        &mut items,
        "python",
        host.python_ready(app_data),
        "Managed Python venv plus MCP/kernel packages",
    );
    let rscript = host.find_rscript(app_data);
    let r_ok = rscript.as_ref().is_some_and(|path| host.jsonlite_ok(path));
    push_tool(
        &mut items,
        "r",
        r_ok,
        "Rscript with the jsonlite package for the persistent r tool",
    );
    let node_ok = host.node_major().is_some_and(|major| major >= 20) && host.find_npm().is_some();
    push_tool(
        &mut items,
        "node",
        node_ok,
        "Node.js >= 20 and npm for literature skills",
    );
    push_tool(
        &mut items,
        "sci",
        host.find_sci().is_some(),
        "scimaster-cli for bear-* literature search",
    );
    push_tool(
        &mut items,
        "pixi",
        host.find_pixi().is_some(),
        "pixi for project-local scientific environments",
    );
    push_tool(
        &mut items,
        "officecli",
        host.find_officecli().is_some(),
        "officecli for Word / Excel / PowerPoint skills",
    );
    items.push(ProvisionItem {
        id: "sci_key".into(),
        status: if host.has_sci_key() {
            "ready".into()
        } else {
            "needs_user".into()
        },
        detail: "SciMaster API key is optional until you run literature search".into(),
    });
    items
}

fn push_tool(items: &mut Vec<ProvisionItem>, id: &str, ready: bool, detail: &str) {
    items.push(ProvisionItem {
        id: id.into(),
        status: if ready {
            "ready".into()
        } else {
            "pending".into()
        },
        detail: detail.into(),
    });
}

pub fn auto_items_satisfied(items: &[ProvisionItem]) -> bool {
    items
        .iter()
        .filter(|item| AUTO_ITEM_IDS.contains(&item.id.as_str()))
        .all(|item| matches!(item.status.as_str(), "ready" | "passed"))
}

pub fn run_provision(
    app_data: &Path,
    host: &dyn ProvisionHost,
    cancel: &AtomicBool,
    on_progress: &ProgressFn,
) -> Result<Vec<ProvisionItem>> {
    let mut items = probe_local_runtimes(app_data, host);
    for id in AUTO_ITEM_IDS {
        if cancel.load(Ordering::Relaxed) {
            return Err(anyhow!("cancelled"));
        }
        let Some(index) = items.iter().position(|item| item.id == *id) else {
            continue;
        };
        if matches!(items[index].status.as_str(), "ready" | "passed") {
            continue;
        }
        items[index].status = "installing".into();
        on_progress(id, "install", 0, None);
        let result = match *id {
            "uv" => host.install_uv(cancel),
            "python" => host.ensure_python(app_data, cancel),
            "r" => host
                .install_r(app_data, cancel)
                .and_then(|rscript| host.install_jsonlite(&rscript, cancel)),
            "node" => host.install_node(cancel),
            "sci" => host.install_sci(cancel),
            "pixi" => host.install_pixi(cancel),
            "officecli" => host.install_officecli(cancel),
            _ => Ok(()),
        };
        match result {
            Ok(()) => {
                items[index].status = "passed".into();
                items[index].detail = format!("{id} ready");
                on_progress(id, "test", 1, Some(1));
            }
            Err(error) if error.to_string() == "cancelled" => return Err(error),
            Err(error) => {
                items[index].status = "failed".into();
                items[index].detail = error.to_string();
                on_progress(id, "error", 0, None);
            }
        }
    }
    Ok(items)
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(anyhow!("cancelled"))
    } else {
        Ok(())
    }
}

fn r_has_jsonlite(rscript: &Path) -> bool {
    let mut cmd = Command::new(rscript);
    cmd.args([
        "--vanilla",
        "-e",
        "cat(requireNamespace('jsonlite', quietly=TRUE))",
    ]);
    superscience_tools::process::hide_console(&mut cmd);
    match cmd.output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).contains("TRUE"),
        _ => false,
    }
}

fn node_major_version(node: &Path) -> Option<u32> {
    let mut cmd = Command::new(node);
    cmd.arg("--version");
    superscience_tools::process::hide_console(&mut cmd);
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .trim_start_matches('v')
        .split('.')
        .next()?
        .parse()
        .ok()
}

fn run_cmd(program: &Path, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    superscience_tools::process::hide_console(&mut cmd);
    let out = cmd.output()?;
    if out.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "{} failed: {}",
            program.display(),
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

fn run_platform_installer(unix: &str, windows: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let _ = unix;
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            windows,
        ]);
        superscience_tools::process::hide_console(&mut cmd);
        let out = cmd.output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(anyhow!(
                "installer failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = windows;
        let mut cmd = Command::new("sh");
        cmd.args(["-lc", unix]);
        superscience_tools::process::hide_console(&mut cmd);
        let out = cmd.output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(anyhow!(
                "installer failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ))
        }
    }
}

fn install_system_r() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        if which::which("brew").is_ok() {
            return run_cmd(Path::new("brew"), &["install", "r"]);
        }
        Err(anyhow!(
            "R is not installed. Install Homebrew and retry, or install R from CRAN."
        ))
    }
    #[cfg(target_os = "windows")]
    {
        if which::which("winget").is_ok() {
            return run_cmd(
                Path::new("winget"),
                &[
                    "install",
                    "--id",
                    "RProject.R",
                    "-e",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ],
            );
        }
        Err(anyhow!(
            "R is not installed. Install it from CRAN, then reopen SuperScience."
        ))
    }
    #[cfg(target_os = "linux")]
    {
        Err(anyhow!(
            "R is not installed. Install r-base with your package manager (for example sudo apt install r-base)."
        ))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(anyhow!("R install is not supported on this platform"))
    }
}

fn install_system_node() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        if which::which("brew").is_ok() {
            return run_cmd(Path::new("brew"), &["install", "node"]);
        }
        Err(anyhow!(
            "Node.js >= 20 is not installed. Install it from https://nodejs.org or with Homebrew."
        ))
    }
    #[cfg(target_os = "windows")]
    {
        if which::which("winget").is_ok() {
            return run_cmd(
                Path::new("winget"),
                &[
                    "install",
                    "--id",
                    "OpenJS.NodeJS.LTS",
                    "-e",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ],
            );
        }
        Err(anyhow!(
            "Node.js >= 20 is not installed. Install the LTS build from https://nodejs.org."
        ))
    }
    #[cfg(target_os = "linux")]
    {
        Err(anyhow!(
            "Node.js >= 20 is not installed. Install it with your package manager or from https://nodejs.org."
        ))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(anyhow!("Node install is not supported on this platform"))
    }
}

/// In-memory host for unit tests. Never shells out or downloads.
#[derive(Default)]
pub struct FakeHost {
    pub present: HashSet<String>,
    pub fail: HashSet<String>,
    pub has_sci_key: bool,
    pub installed: std::sync::Mutex<HashSet<String>>,
}

impl FakeHost {
    pub fn with_present(ids: &[&str]) -> Self {
        Self {
            present: ids.iter().map(|id| (*id).to_string()).collect(),
            ..Self::default()
        }
    }

    fn mark(&self, id: &str) {
        self.installed.lock().unwrap().insert(id.into());
    }

    fn is_present(&self, id: &str) -> bool {
        self.present.contains(id) || self.installed.lock().unwrap().contains(id)
    }
}

impl ProvisionHost for FakeHost {
    fn find_uv(&self) -> Option<PathBuf> {
        self.is_present("uv").then(|| PathBuf::from("/fake/uv"))
    }
    fn find_node(&self) -> Option<PathBuf> {
        self.is_present("node").then(|| PathBuf::from("/fake/node"))
    }
    fn find_npm(&self) -> Option<PathBuf> {
        self.is_present("node").then(|| PathBuf::from("/fake/npm"))
    }
    fn find_sci(&self) -> Option<PathBuf> {
        self.is_present("sci").then(|| PathBuf::from("/fake/sci"))
    }
    fn find_pixi(&self) -> Option<PathBuf> {
        self.is_present("pixi").then(|| PathBuf::from("/fake/pixi"))
    }
    fn find_officecli(&self) -> Option<PathBuf> {
        self.is_present("officecli")
            .then(|| PathBuf::from("/fake/officecli"))
    }
    fn find_rscript(&self, _app_data: &Path) -> Option<PathBuf> {
        self.is_present("r").then(|| PathBuf::from("/fake/Rscript"))
    }
    fn python_ready(&self, _app_data: &Path) -> bool {
        self.is_present("python")
    }
    fn jsonlite_ok(&self, _rscript: &Path) -> bool {
        self.is_present("r")
    }
    fn node_major(&self) -> Option<u32> {
        self.is_present("node").then_some(20)
    }
    fn has_sci_key(&self) -> bool {
        self.has_sci_key
    }

    fn install_uv(&self, cancel: &AtomicBool) -> Result<()> {
        self.finish("uv", cancel)
    }
    fn ensure_python(&self, _app_data: &Path, cancel: &AtomicBool) -> Result<()> {
        self.finish("python", cancel)
    }
    fn install_r(&self, _app_data: &Path, cancel: &AtomicBool) -> Result<PathBuf> {
        self.finish("r", cancel)?;
        Ok(PathBuf::from("/fake/Rscript"))
    }
    fn install_jsonlite(&self, _rscript: &Path, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)
    }
    fn install_node(&self, cancel: &AtomicBool) -> Result<()> {
        self.finish("node", cancel)
    }
    fn install_sci(&self, cancel: &AtomicBool) -> Result<()> {
        self.finish("sci", cancel)
    }
    fn install_pixi(&self, cancel: &AtomicBool) -> Result<()> {
        self.finish("pixi", cancel)
    }
    fn install_officecli(&self, cancel: &AtomicBool) -> Result<()> {
        self.finish("officecli", cancel)
    }
}

impl FakeHost {
    fn finish(&self, id: &str, cancel: &AtomicBool) -> Result<()> {
        check_cancel(cancel)?;
        if self.fail.contains(id) {
            return Err(anyhow!("{id} install failed"));
        }
        self.mark(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn probe_marks_missing_tools_pending_and_sci_key_needs_user() {
        let host = FakeHost::with_present(&["uv", "python"]);
        let items = probe_local_runtimes(Path::new("/tmp"), &host);
        let status = |id: &str| {
            items
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.status.as_str())
                .unwrap()
        };
        assert_eq!(status("uv"), "ready");
        assert_eq!(status("python"), "ready");
        assert_eq!(status("r"), "pending");
        assert_eq!(status("node"), "pending");
        assert_eq!(status("sci_key"), "needs_user");
        assert!(!auto_items_satisfied(&items));
    }

    #[test]
    fn fake_host_installs_pending_items_without_network() {
        let host = FakeHost::default();
        let cancel = AtomicBool::new(false);
        let items =
            run_provision(Path::new("/tmp"), &host, &cancel, &|_id, _phase, _, _| {}).unwrap();
        assert!(auto_items_satisfied(&items));
        assert_eq!(
            items
                .iter()
                .find(|item| item.id == "sci_key")
                .unwrap()
                .status,
            "needs_user"
        );
    }

    #[test]
    fn cancel_stops_before_remaining_items() {
        let host = FakeHost::default();
        let cancel = AtomicBool::new(true);
        let error = run_provision(Path::new("/tmp"), &host, &cancel, &|_, _, _, _| {}).unwrap_err();
        assert_eq!(error.to_string(), "cancelled");
    }

    #[test]
    fn failed_item_does_not_satisfy_auto_set() {
        let host = FakeHost {
            fail: ["r".into()].into_iter().collect(),
            ..FakeHost::default()
        };
        let cancel = AtomicBool::new(false);
        let items = run_provision(Path::new("/tmp"), &host, &cancel, &|_, _, _, _| {}).unwrap();
        assert!(!auto_items_satisfied(&items));
        assert_eq!(
            items.iter().find(|item| item.id == "r").unwrap().status,
            "failed"
        );
    }
}
