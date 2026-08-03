//! Build script: purge stale bundled seed copies before Tauri re-copies resources.
//!
//! Tauri's map-form `resources` merge into `target/{profile}/seed` and never
//! delete removed files, so renamed demos (and old CRISPR/enzyme seeds) linger
//! beside the binary and show up as extra Example-project sessions.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    purge_stale_seed_dirs();
    tauri_build::build();
}

fn purge_stale_seed_dirs() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = manifest_dir.join("..").join("target");
    if !target.is_dir() {
        return;
    }
    for profile in ["debug", "release"] {
        for rel in ["seed", "_up_/seed"] {
            let dir = target.join(profile).join(rel);
            remove_dir_if_present(&dir);
        }
    }
    println!("cargo:rerun-if-changed=../seed");
}

fn remove_dir_if_present(dir: &Path) {
    if dir.is_dir() {
        let _ = fs::remove_dir_all(dir);
    }
}
