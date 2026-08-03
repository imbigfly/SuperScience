//! Bundled demo loader — reads the upstream `seed/manifest_*.json` session
//! recordings and presents each as a pre-baked transcript the UI can open.
//! Full operation history lives in `output_data.items` (UiItem-shaped rows).
//! Figure/data files live in paired `assets_*.tar.gz` archives and are extracted
//! into the workspace when a demo is opened.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::State;

use crate::AppState;
use crate::resource_refs;

/// Bundled demo manifests (`seed/`).
pub fn bundled_dir() -> Option<PathBuf> {
    wisp_paths::seed_dir()
}

#[derive(Serialize, Clone)]
pub struct DemoInfo {
    pub id: String,
    pub title: String,
}

/// One transcript row returned to the UI (same shape as session `UiItem`).
#[derive(Serialize, Clone, Deserialize)]
pub struct DemoUiItem {
    pub role: String,
    pub text: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<resource_refs::UiMessageResource>,
}

#[derive(Serialize, Clone)]
pub struct Demo {
    pub id: String,
    pub title: String,
    pub request: String,
    pub response: String,
    pub thinking: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<DemoUiItem>,
}

#[tauri::command(rename = "list_demos")]
pub(super) fn list_demos_cmd() -> Vec<DemoInfo> {
    list_demos()
}

#[tauri::command(rename = "load_demo")]
pub(super) fn load_demo_cmd(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    id: String,
) -> Result<Demo, String> {
    let ap = state.active(window.label());
    extract_demo_assets(&id, &ap.root)?;
    load_demo(&id).ok_or_else(|| format!("demo '{id}' not found"))
}

fn clean(text: &str) -> String {
    static IMG: OnceLock<Regex> = OnceLock::new();
    static ART: OnceLock<Regex> = OnceLock::new();
    let img = IMG.get_or_init(|| Regex::new(r"!\[([^\]]*)\]\(\{\{artifact:[^}]+\}\}\)").unwrap());
    let art = ART.get_or_init(|| Regex::new(r"\{\{artifact:[^}]+\}\}").unwrap());
    let s = img.replace_all(text, "[$1 (figure)]").to_string();
    art.replace_all(&s, "(artifact)").to_string()
}

fn read_title(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let req = v
        .pointer("/root_frame/input_data/request")
        .and_then(|x| x.as_str())?;
    let first = req.split('.').next().unwrap_or(req).trim();
    Some(first.chars().take(70).collect())
}

/// Enumerate `manifest_*.json` in the bundled seed dir.
pub fn list_demos() -> Vec<DemoInfo> {
    let Some(dir) = bundled_dir() else {
        return vec![];
    };
    let mut out = vec![];
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let stem = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if !stem.starts_with("manifest_") {
                continue;
            }
            let title =
                read_title(&p).unwrap_or_else(|| stem.trim_start_matches("manifest_").to_string());
            out.push(DemoInfo { id: stem, title });
        }
    }
    // Numeric id prefixes (manifest_esr1_01_…) keep the research narrative order.
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn assets_tarball(id: &str) -> Option<PathBuf> {
    let dir = bundled_dir()?;
    let suffix = id.strip_prefix("manifest_")?;
    let path = dir.join(format!("assets_{suffix}.tar.gz"));
    path.is_file().then_some(path)
}

/// Extract bundled demo files into `dest` (workspace root), flattening the
/// `example_*` folder inside each tarball so transcript filenames resolve.
/// Demos without an assets archive are a no-op.
pub fn extract_demo_assets(id: &str, dest: &Path) -> Result<(), String> {
    let Some(tar_path) = assets_tarball(id) else {
        return Ok(());
    };
    std::fs::create_dir_all(dest).map_err(|e| format!("create demo dest: {e}"))?;
    let file = File::open(&tar_path).map_err(|e| format!("open {}: {e}", tar_path.display()))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    for entry in archive.entries().map_err(|e| format!("read tar: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let path = entry.path().map_err(|e| format!("tar path: {e}"))?;
        let Some(name) = path.file_name() else {
            continue;
        };
        let out = dest.join(name);
        entry
            .unpack(&out)
            .map_err(|e| format!("unpack {}: {e}", out.display()))?;
    }
    Ok(())
}

fn clean_item(mut item: DemoUiItem) -> DemoUiItem {
    item.text = clean(&item.text);
    if let Some(input) = item.input.as_mut() {
        *input = clean(input);
    }
    item
}

/// Load one demo by id (the manifest file stem, e.g. `manifest_esr1_03_rnaseq`).
pub fn load_demo(id: &str) -> Option<Demo> {
    let dir = bundled_dir()?;
    let path = dir.join(format!("{id}.json"));
    let text = std::fs::read_to_string(&path).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let req = v
        .pointer("/root_frame/input_data/request")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let resp = v
        .pointer("/root_frame/output_data/response")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let thinking = v
        .pointer("/root_frame/output_data/thinking")
        .and_then(|x| x.as_str())
        .map(String::from);
    let items: Vec<DemoUiItem> = v
        .pointer("/root_frame/output_data/items")
        .and_then(|x| serde_json::from_value::<Vec<DemoUiItem>>(x.clone()).ok())
        .unwrap_or_default()
        .into_iter()
        .map(clean_item)
        .collect();
    let title = read_title(&path).unwrap_or_else(|| id.trim_start_matches("manifest_").to_string());
    Some(Demo {
        id: id.to_string(),
        title,
        request: clean(&req),
        response: clean(&resp),
        thinking: thinking.map(|t| clean(&t)),
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_esr1_demo_assets() {
        let tmp = std::env::temp_dir().join(format!("wisp-seed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        extract_demo_assets("manifest_esr1_03_rnaseq", &tmp).expect("extract rnaseq assets");
        assert!(tmp.join("GSE153250_counts_matrix.tsv").is_file());
        assert!(tmp.join("GSE153250_sample_groups.txt").is_file());
        assert!(tmp.join("GSE153250_featureCounts_summary.txt").is_file());

        let down = tmp.join("downstream");
        std::fs::create_dir_all(&down).unwrap();
        extract_demo_assets("manifest_esr1_04_downstream", &down)
            .expect("extract downstream assets");
        assert!(down.join("DESeq2_top200.csv").is_file());
        assert!(down.join("research_projects.md").is_file());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn lists_and_loads_bundled_demos() {
        let demos = list_demos();
        assert_eq!(demos.len(), 5, "bundled seed should ship the five ESR1 demos");
        assert_eq!(
            demos.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
            [
                "manifest_esr1_01_datasets",
                "manifest_esr1_02_samples",
                "manifest_esr1_03_rnaseq",
                "manifest_esr1_04_downstream",
                "manifest_esr1_05_hypotheses",
            ]
        );
        for info in &demos {
            let demo = load_demo(&info.id).expect("load demo");
            assert!(!demo.request.is_empty());
            assert!(!demo.request.contains("English reply"));
            assert!(!demo.request.to_ascii_lowercase().contains("guotosky"));
            assert!(!demo.items.is_empty(), "{} should ship transcript items", info.id);
            assert!(
                demo.items.iter().any(|i| i.role == "tool"),
                "{} should include tool operation records",
                info.id
            );
            let blob = serde_json::to_string(&demo).unwrap();
            assert!(!blob.to_ascii_lowercase().contains("guotosky"));
            assert!(!blob.contains("10.10.10."));
            assert!(!blob.contains(":7897"));
            assert!(!blob.contains("{{artifact:"));
        }

        let datasets = load_demo("manifest_esr1_01_datasets").expect("datasets demo");
        assert!(
            datasets.request.contains("MCF7") || datasets.request.contains("ESR1"),
            "datasets demo request should mention ESR1/MCF7"
        );

        let samples = load_demo("manifest_esr1_02_samples").expect("samples demo");
        assert!(
            samples.request.contains("GSE153250"),
            "samples demo request should mention GSE153250"
        );

        let rnaseq = load_demo("manifest_esr1_03_rnaseq").expect("rnaseq demo");
        assert!(
            rnaseq
                .items
                .iter()
                .any(|i| i.tool_name.as_deref() == Some("monitor_run")),
            "rnaseq demo should include SSH/run monitor cards"
        );
        assert!(
            rnaseq.response.contains("GSE153250") || rnaseq.response.contains("siESR1"),
            "rnaseq response should mention the study"
        );

        let downstream = load_demo("manifest_esr1_04_downstream").expect("downstream demo");
        assert!(
            downstream.request.contains("differential")
                || downstream.request.contains("GSEA")
                || downstream.request.contains("Enrichr"),
            "downstream demo request should mention enrichment/DEG"
        );

        let hypotheses = load_demo("manifest_esr1_05_hypotheses").expect("hypotheses demo");
        assert!(
            hypotheses.request.contains("research projects")
                || hypotheses.request.contains("scientific"),
            "hypotheses demo request should ask for research projects"
        );
    }
}
