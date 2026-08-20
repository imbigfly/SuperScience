//! Allowlisted vendor skill packs: pin comparison, safe archive selection, and
//! local frontmatter patches applied after an auto-update install.

use std::path::{Path, PathBuf};

/// How the remote pin is obtained and compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKind {
    /// Full git commit SHA from the default branch.
    Commit,
    /// Semver tag such as `v3.20.1`.
    Semver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinSource {
    DefaultBranch,
    LatestSemverTag,
}

#[derive(Debug, Clone, Copy)]
pub struct VendorPackSpec {
    pub id: &'static str,
    pub owner: &'static str,
    pub repo: &'static str,
    pub pin_kind: PinKind,
    pub pin_source: PinSource,
    /// Pin currently shipped in the signed app bundle (may be empty).
    pub bundled_pin: &'static str,
    pub skills: &'static [&'static str],
    pub vendor_dir: &'static str,
}

const NATURE_SKILLS: &[&str] = &[
    "nature-academic-search",
    "nature-citation",
    "nature-data",
    "nature-downloader",
    "nature-experiment-log",
    "nature-figure",
    "nature-literature-pipeline",
    "nature-paper-card",
    "nature-paper-to-patent",
    "nature-paper2ppt",
    "nature-polishing",
    "nature-proposal-writer",
    "nature-reader",
    "nature-ref-verifier",
    "nature-response",
    "nature-reviewer",
    "nature-shared",
    "nature-statistics",
    "nature-writing",
];

const ACADEMIC_SKILLS: &[&str] = &[
    "academic-paper",
    "academic-paper-reviewer",
    "academic-pipeline",
    "deep-research",
];

/// Shared contracts package produced by [`normalize_academic_shared_layout`].
/// Upstream archives still ship a duplicated `shared/` inside each consumer.
pub const ACADEMIC_SHARED_NAME: &str = "academic-shared";

const ACADEMIC_SHARED_SKILL_MD: &str = r#"---
name: academic-shared
description: Internal shared contracts, protocols, and schemas for academic-paper, academic-paper-reviewer, academic-pipeline, and deep-research. Do not invoke it as a standalone user workflow. Load only the specific shared file requested by another academic skill.
---

# Academic Shared References

Use this package only as a dependency of another installed academic-research skill.

- Load the exact referenced file; do not preload the whole package.
- Treat `contracts/`, `references/`, and the root protocols as shared definitions, not standalone workflows.
- Return to the requesting skill for task logic, output format, and final QA.

Consuming skills reference files as `../academic-shared/...`.
"#;

pub const VENDOR_PACKS: &[VendorPackSpec] = &[
    VendorPackSpec {
        id: "nature-skills",
        owner: "Yuan1z0825",
        repo: "nature-skills",
        pin_kind: PinKind::Commit,
        pin_source: PinSource::DefaultBranch,
        bundled_pin: "c171989db699bd601d4373912b3fb8db96ecc95b",
        skills: NATURE_SKILLS,
        vendor_dir: "nature-skills",
    },
    VendorPackSpec {
        id: "academic-research-skills",
        owner: "Imbad0202",
        repo: "academic-research-skills",
        pin_kind: PinKind::Semver,
        pin_source: PinSource::LatestSemverTag,
        bundled_pin: "v3.20.1",
        skills: ACADEMIC_SKILLS,
        vendor_dir: "academic-research-skills",
    },
    VendorPackSpec {
        id: "humanizer-zh",
        owner: "op7418",
        repo: "Humanizer-zh",
        pin_kind: PinKind::Commit,
        pin_source: PinSource::DefaultBranch,
        bundled_pin: "",
        skills: &["humanizer-zh"],
        vendor_dir: "humanizer-zh",
    },
];

pub fn vendor_pack(id: &str) -> Option<&'static VendorPackSpec> {
    VENDOR_PACKS.iter().find(|pack| pack.id == id)
}

/// Whether a remote pin should be installed over the current overlay / bundled pin.
pub fn needs_remote_install(
    overlay_pin: Option<&str>,
    bundled_pin: &str,
    remote_pin: &str,
) -> bool {
    if remote_pin.is_empty() {
        return false;
    }
    match overlay_pin {
        Some(pin) if pin == remote_pin => false,
        Some(_) => true,
        None if !bundled_pin.is_empty() && bundled_pin == remote_pin => false,
        None => true,
    }
}

/// Drop an auto-update overlay when a newer bundled semver has landed in the
/// signed app. Commit pins are refreshed by the GitHub check instead.
pub fn overlay_stale_vs_bundled(overlay_pin: &str, bundled_pin: &str, kind: PinKind) -> bool {
    if overlay_pin.is_empty() || bundled_pin.is_empty() {
        return false;
    }
    match kind {
        PinKind::Semver => parse_semver_pin(overlay_pin)
            .zip(parse_semver_pin(bundled_pin))
            .is_some_and(|(overlay, bundled)| overlay < bundled),
        PinKind::Commit => false,
    }
}

pub fn parse_semver_pin(value: &str) -> Option<semver::Version> {
    let trimmed = value.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() {
        return None;
    }
    semver::Version::parse(trimmed).ok()
}

pub fn validate_skill_dir_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty()
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || Path::new(name).file_name().and_then(|n| n.to_str()) != Some(name)
    {
        return Err(format!("invalid skill directory name '{name}'"));
    }
    Ok(())
}

/// GitHub `archive/*.zip` unpacks into `{repo}-{sha}/`. Prefer that single
/// child directory when present.
pub fn archive_inner_root(unpacked: &Path) -> Result<PathBuf, String> {
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(unpacked)
        .map_err(|error| format!("read archive root {}: {error}", unpacked.display()))?
    {
        let entry = entry.map_err(|error| format!("read archive entry: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "__MACOSX" || name.starts_with('.') {
            continue;
        }
        if entry
            .file_type()
            .map_err(|error| format!("stat archive entry: {error}"))?
            .is_dir()
        {
            dirs.push(entry.path());
        }
    }
    match dirs.as_slice() {
        [only] => Ok(only.clone()),
        _ => Ok(unpacked.to_path_buf()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedSkills {
    pub dirs: Vec<(String, PathBuf)>,
    pub missing: Vec<String>,
}

/// Keep only the skill directories declared by the pack spec.
pub fn collect_declared_skill_dirs(root: &Path, names: &[&str]) -> Result<SelectedSkills, String> {
    let root_canon = root
        .canonicalize()
        .map_err(|error| format!("canonicalize archive root {}: {error}", root.display()))?;
    let mut dirs = Vec::new();
    let mut missing = Vec::new();
    for name in names {
        validate_skill_dir_name(name)?;
        let path = root.join(name);
        if !path.is_dir() {
            missing.push((*name).to_string());
            continue;
        }
        let canon = path
            .canonicalize()
            .map_err(|error| format!("canonicalize skill dir {name}: {error}"))?;
        if !canon.starts_with(&root_canon) {
            return Err(format!("skill path escapes archive: {name}"));
        }
        dirs.push(((*name).to_string(), path));
    }
    if dirs.is_empty() {
        return Err("archive contained none of the expected skill directories".into());
    }
    Ok(SelectedSkills { dirs, missing })
}

/// Upstream `nature-proposal-writer` sometimes ships `name: researchwrite`.
/// SuperScience discovery requires `name ==` directory name.
pub fn ensure_skill_frontmatter_name(skill_md: &Path, expected: &str) -> Result<bool, String> {
    let text = std::fs::read_to_string(skill_md)
        .map_err(|error| format!("read {}: {error}", skill_md.display()))?;
    let Some(rest) = text.strip_prefix("---") else {
        return Ok(false);
    };
    let Some(end) = rest.find("\n---") else {
        return Ok(false);
    };
    let fm = &rest[..end];
    let mut changed = false;
    let mut lines = Vec::new();
    for line in fm.lines() {
        if let Some(value) = line.strip_prefix("name:") {
            if value.trim() != expected {
                lines.push(format!("name: {expected}"));
                changed = true;
                continue;
            }
        }
        lines.push(line.to_string());
    }
    if !changed {
        return Ok(false);
    }
    let patched = format!("---\n{}{}", lines.join("\n"), &rest[end..]);
    std::fs::write(skill_md, patched)
        .map_err(|error| format!("write {}: {error}", skill_md.display()))?;
    Ok(true)
}

/// Rewrite a SKILL.md YAML key `wisp:` to `superscience:` so upstream vendor
/// packs keep planner semantics after install. No-op when `superscience:` is
/// already present. Does not teach the parser an alias.
pub fn rewrite_wisp_frontmatter_key(skill_md: &Path) -> Result<bool, String> {
    if !skill_md.is_file() {
        return Ok(false);
    }
    let text = std::fs::read_to_string(skill_md)
        .map_err(|error| format!("read {}: {error}", skill_md.display()))?;
    let Some(patched) = rewrite_wisp_frontmatter_in_text(&text) else {
        return Ok(false);
    };
    std::fs::write(skill_md, patched)
        .map_err(|error| format!("write {}: {error}", skill_md.display()))?;
    Ok(true)
}

fn rewrite_wisp_frontmatter_in_text(text: &str) -> Option<String> {
    let (delim, rest) = if let Some(rest) = text.strip_prefix("---\r\n") {
        ("---\r\n", rest)
    } else if let Some(rest) = text.strip_prefix("---\n") {
        ("---\n", rest)
    } else {
        return None;
    };
    let end = rest.find("\n---")?;
    let fm = &rest[..end];
    if fm.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed == "superscience:" || trimmed.starts_with("superscience:")
    }) {
        return None;
    }
    let mut changed = false;
    let mut lines = Vec::new();
    for line in fm.split('\n') {
        let ending = if line.ends_with('\r') { "\r" } else { "" };
        let content = line.trim_end_matches('\r');
        let trimmed = content.trim_start();
        if let Some(after) = trimmed.strip_prefix("wisp:") {
            if after.is_empty() || after.starts_with([' ', '\t', '{']) {
                let indent_len = content.len() - trimmed.len();
                lines.push(format!(
                    "{}superscience:{after}{ending}",
                    &content[..indent_len]
                ));
                changed = true;
                continue;
            }
        }
        lines.push(line.to_string());
    }
    if !changed {
        return None;
    }
    Some(format!("{delim}{}{}", lines.join("\n"), &rest[end..]))
}

pub fn apply_pack_local_patches(skill_name: &str, skill_dir: &Path) -> Result<(), String> {
    let _ = rewrite_wisp_frontmatter_key(&skill_dir.join("SKILL.md"))?;
    if skill_name == "nature-proposal-writer" {
        let _ =
            ensure_skill_frontmatter_name(&skill_dir.join("SKILL.md"), "nature-proposal-writer")?;
    }
    Ok(())
}

/// Skill directories that auto-update may write for a pack, including synthetic
/// layouts such as `academic-shared`.
pub fn pack_overlay_skill_names(pack: &VendorPackSpec) -> Vec<&'static str> {
    let mut names = pack.skills.to_vec();
    if pack.id == "academic-research-skills" {
        names.push(ACADEMIC_SHARED_NAME);
    }
    names
}

/// Rewrite path-like `shared/...` refs. Skips matches preceded by a path or
/// identifier character so `academic-shared/` and URL `.../shared/` stay intact.
pub fn rewrite_shared_path_refs(input: &str, replacement: &str) -> String {
    let needle = "shared/";
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(idx) = rest.find(needle) {
        let prev = if idx == 0 {
            None
        } else {
            rest[..idx].chars().next_back()
        };
        let blocked =
            prev.is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'));
        out.push_str(&rest[..idx]);
        if blocked {
            out.push_str(needle);
            rest = &rest[idx + needle.len()..];
            continue;
        }
        out.push_str(replacement);
        rest = &rest[idx + needle.len()..];
    }
    out.push_str(rest);
    out
}

fn should_rewrite_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "md" | "json" | "yaml" | "yml"))
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".schema.json"))
}

fn rewrite_text_tree(root: &Path, replacement: &str) -> Result<usize, String> {
    let mut changed = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in
            std::fs::read_dir(&dir).map_err(|error| format!("read {}: {error}", dir.display()))?
        {
            let entry = entry.map_err(|error| format!("read entry: {error}"))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("stat {}: {error}", path.display()))?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if !file_type.is_file() || !should_rewrite_text_file(&path) {
                continue;
            }
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("read {}: {error}", path.display()))?;
            let rewritten = rewrite_shared_path_refs(&text, replacement);
            if rewritten != text {
                std::fs::write(&path, rewritten)
                    .map_err(|error| format!("write {}: {error}", path.display()))?;
                changed += 1;
            }
        }
    }
    Ok(changed)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|error| format!("create {}: {error}", dest.display()))?;
    for entry in
        std::fs::read_dir(src).map_err(|error| format!("read {}: {error}", src.display()))?
    {
        let entry = entry.map_err(|error| format!("read entry: {error}"))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("stat {}: {error}", from.display()))?;
        if file_type.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if file_type.is_file() {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("create {}: {error}", parent.display()))?;
            }
            std::fs::copy(&from, &to)
                .map_err(|error| format!("copy {} → {}: {error}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

/// Collapse duplicated per-skill `shared/` trees from an upstream academic
/// archive into one `academic-shared` package and retarget consumer refs.
pub fn normalize_academic_shared_layout(skills_dir: &Path) -> Result<bool, String> {
    let mut donor: Option<PathBuf> = None;
    for name in ACADEMIC_SKILLS {
        let shared = skills_dir.join(name).join("shared");
        if shared.is_dir() {
            donor = Some(shared);
            break;
        }
    }
    let Some(donor) = donor else {
        return Ok(false);
    };

    let shared_dest = skills_dir.join(ACADEMIC_SHARED_NAME);
    if shared_dest.exists() {
        std::fs::remove_dir_all(&shared_dest)
            .map_err(|error| format!("remove previous {}: {error}", shared_dest.display()))?;
    }
    copy_dir_recursive(&donor, &shared_dest)?;
    let _ = rewrite_text_tree(&shared_dest, "")?;
    std::fs::write(shared_dest.join("SKILL.md"), ACADEMIC_SHARED_SKILL_MD)
        .map_err(|error| format!("write {}/SKILL.md: {error}", ACADEMIC_SHARED_NAME))?;

    for name in ACADEMIC_SKILLS {
        let skill_dir = skills_dir.join(name);
        if !skill_dir.is_dir() {
            continue;
        }
        let mut stack = vec![skill_dir.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir)
                .map_err(|error| format!("read {}: {error}", dir.display()))?
            {
                let entry = entry.map_err(|error| format!("read entry: {error}"))?;
                let path = entry.path();
                if path.file_name().and_then(|n| n.to_str()) == Some("shared") {
                    continue;
                }
                let file_type = entry
                    .file_type()
                    .map_err(|error| format!("stat {}: {error}", path.display()))?;
                if file_type.is_dir() {
                    stack.push(path);
                    continue;
                }
                if !file_type.is_file() || !should_rewrite_text_file(&path) {
                    continue;
                }
                let text = std::fs::read_to_string(&path)
                    .map_err(|error| format!("read {}: {error}", path.display()))?;
                let rewritten = rewrite_shared_path_refs(&text, "../academic-shared/");
                if rewritten != text {
                    std::fs::write(&path, rewritten)
                        .map_err(|error| format!("write {}: {error}", path.display()))?;
                }
            }
        }
        let nested = skill_dir.join("shared");
        if nested.is_dir() {
            std::fs::remove_dir_all(&nested)
                .map_err(|error| format!("remove {}: {error}", nested.display()))?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "superscience-skill-update-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn vendor_pins_match_checked_in_source_txt() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skills/_vendor");
        for pack in VENDOR_PACKS {
            let text = fs::read_to_string(root.join(pack.vendor_dir).join("SOURCE.txt"))
                .unwrap_or_else(|error| panic!("{}: {error}", pack.vendor_dir));
            assert!(
                text.contains(&format!("{}/{}", pack.owner, pack.repo))
                    || text.contains(&format!("github.com/{}/{}", pack.owner, pack.repo)),
                "{} SOURCE.txt missing repo",
                pack.id
            );
            if !pack.bundled_pin.is_empty() {
                assert!(
                    text.contains(pack.bundled_pin),
                    "{} SOURCE.txt missing pin {}",
                    pack.id,
                    pack.bundled_pin
                );
            }
        }
    }

    #[test]
    fn needs_remote_install_skips_matching_pins() {
        assert!(!needs_remote_install(Some("abc"), "bundled", "abc"));
        assert!(needs_remote_install(Some("old"), "bundled", "new"));
        assert!(!needs_remote_install(None, "same", "same"));
        assert!(needs_remote_install(None, "", "new"));
        assert!(!needs_remote_install(None, "same", ""));
    }

    #[test]
    fn semver_overlay_behind_bundled_is_stale() {
        assert!(overlay_stale_vs_bundled(
            "v3.19.0",
            "v3.20.1",
            PinKind::Semver
        ));
        assert!(!overlay_stale_vs_bundled(
            "v3.20.1",
            "v3.20.1",
            PinKind::Semver
        ));
        assert!(!overlay_stale_vs_bundled(
            "c171989db699bd601d4373912b3fb8db96ecc95b",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            PinKind::Commit
        ));
    }

    #[test]
    fn archive_selection_rejects_path_escape_names() {
        assert!(validate_skill_dir_name("../evil").is_err());
        assert!(validate_skill_dir_name("a/b").is_err());
    }

    #[test]
    fn collects_only_declared_skill_dirs_under_wrapper() {
        let root = temp_dir("collect");
        let wrapper = root.join("nature-skills-sha");
        fs::create_dir_all(wrapper.join("nature-writing")).unwrap();
        fs::create_dir_all(wrapper.join("extra-noise")).unwrap();
        fs::write(wrapper.join("nature-writing/SKILL.md"), "x").unwrap();

        let inner = archive_inner_root(&root).unwrap();
        assert_eq!(inner, wrapper);
        let selected =
            collect_declared_skill_dirs(&inner, &["nature-writing", "nature-shared"]).unwrap();
        assert_eq!(selected.dirs.len(), 1);
        assert_eq!(selected.dirs[0].0, "nature-writing");
        assert_eq!(selected.missing, vec!["nature-shared".to_string()]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn patches_researchwrite_frontmatter_name() {
        let root = temp_dir("patch");
        let md = root.join("SKILL.md");
        fs::write(
            &md,
            "---\nname: researchwrite\ndescription: demo\n---\n\n# body\n",
        )
        .unwrap();
        assert!(ensure_skill_frontmatter_name(&md, "nature-proposal-writer").unwrap());
        let text = fs::read_to_string(&md).unwrap();
        assert!(text.contains("name: nature-proposal-writer"));
        assert!(!text.contains("name: researchwrite"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rewrite_shared_refs_skips_academic_shared_and_urls() {
        let input = "`shared/foo.md` and ../academic-shared/foo.md and https://x/shared/foo";
        let out = rewrite_shared_path_refs(input, "../academic-shared/");
        assert_eq!(
            out,
            "`../academic-shared/foo.md` and ../academic-shared/foo.md and https://x/shared/foo"
        );
        assert_eq!(rewrite_shared_path_refs("`shared/foo.md`", ""), "`foo.md`");
    }

    #[test]
    fn rewrite_wisp_frontmatter_key_rewrites_mapping_and_skips_when_product_key_exists() {
        let rewritten = rewrite_wisp_frontmatter_in_text(
            "---\nname: demo\nwisp:\n  schema_version: 1\n---\n# Body\n",
        )
        .unwrap();
        assert!(rewritten.contains("superscience:\n  schema_version: 1"));
        assert!(!rewritten.lines().any(|line| line.trim() == "wisp:"));
        assert!(rewrite_wisp_frontmatter_in_text(
            "---\nname: demo\nsuperscience:\n  schema_version: 1\nwisp:\n  ignored: true\n---\n"
        )
        .is_none());
        assert!(rewrite_wisp_frontmatter_in_text(
            "---\nname: demo\ndescription: mentions wisp: only in prose\n---\n"
        )
        .is_none());
    }

    #[test]
    fn apply_pack_local_patches_rewrites_upstream_wisp_key() {
        let root = temp_dir("wisp-key");
        let md = root.join("SKILL.md");
        fs::write(
            &md,
            "---\nname: demo\nwisp:\n  schema_version: 1\n  side_effects: network\n---\n# body\n",
        )
        .unwrap();
        apply_pack_local_patches("demo", &root).unwrap();
        let text = fs::read_to_string(&md).unwrap();
        assert!(text.contains("superscience:"));
        assert!(!text.lines().any(|line| line.trim() == "wisp:"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn normalize_lifts_duplicated_shared_trees() {
        let root = temp_dir("academic-shared");
        for name in ["academic-paper", "academic-pipeline"] {
            let shared = root.join(name).join("shared");
            fs::create_dir_all(&shared).unwrap();
            fs::write(shared.join("raise_framework.md"), "# raise\n").unwrap();
            fs::write(
                root.join(name).join("SKILL.md"),
                "---\nname: demo\n---\nSee `shared/raise_framework.md` and ../academic-shared/keep.md\n",
            )
            .unwrap();
        }

        assert!(normalize_academic_shared_layout(&root).unwrap());
        assert!(root.join("academic-shared/SKILL.md").is_file());
        assert!(root.join("academic-shared/raise_framework.md").is_file());
        assert!(!root.join("academic-paper/shared").exists());
        assert!(!root.join("academic-pipeline/shared").exists());
        let paper = fs::read_to_string(root.join("academic-paper/SKILL.md")).unwrap();
        assert!(paper.contains("`../academic-shared/raise_framework.md`"));
        assert!(paper.contains("../academic-shared/keep.md"));
        assert!(!paper.contains("`shared/"));
        let _ = fs::remove_dir_all(root);
    }
}
