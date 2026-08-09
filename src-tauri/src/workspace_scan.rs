//! Deterministic, symlink-safe workspace traversal shared by project transfer
//! and persistent exploration snapshots.

use std::path::{Component, Path, PathBuf};

pub(crate) const MAX_WORKSPACE_ENTRIES: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceNodeKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceNode {
    pub path: PathBuf,
    pub relative_path: String,
    pub kind: WorkspaceNodeKind,
    pub size_bytes: u64,
    pub mode: Option<u32>,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceScanOptions {
    pub excluded_roots: Vec<PathBuf>,
    pub excluded_relative_prefixes: Vec<String>,
    pub max_entries: usize,
}

impl Default for WorkspaceScanOptions {
    fn default() -> Self {
        Self {
            excluded_roots: Vec::new(),
            excluded_relative_prefixes: Vec::new(),
            max_entries: MAX_WORKSPACE_ENTRIES,
        }
    }
}

pub(crate) fn scan_workspace(
    root: &Path,
    options: &WorkspaceScanOptions,
) -> Result<Vec<WorkspaceNode>, String> {
    fn children(
        directory: &Path,
        visited: &mut usize,
        max_entries: usize,
    ) -> Result<Vec<std::fs::DirEntry>, String> {
        let mut children = Vec::new();
        for child in std::fs::read_dir(directory)
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
        {
            if *visited >= max_entries {
                return Err(format!(
                    "workspace contains more than {max_entries} entries"
                ));
            }
            *visited += 1;
            children.push(
                child.map_err(|error| format!("cannot read {}: {error}", directory.display()))?,
            );
        }
        children.sort_by_key(|entry| entry.file_name());
        Ok(children)
    }

    let root_metadata = std::fs::symlink_metadata(root).map_err(|error| {
        format!(
            "project directory does not exist: {} ({error})",
            root.display()
        )
    })?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(format!(
            "project directory must be a real directory: {}",
            root.display()
        ));
    }

    let mut nodes = Vec::new();
    let mut visited = 0usize;
    let mut pending = children(root, &mut visited, options.max_entries)?;
    pending.reverse();
    while let Some(child) = pending.pop() {
        let path = child.path();
        let relative_path = portable_relative(root, &path)?;
        if options
            .excluded_roots
            .iter()
            .any(|excluded| same_path(&path, excluded))
            || options
                .excluded_relative_prefixes
                .iter()
                .any(|prefix| is_path_at_or_below(&relative_path, prefix))
        {
            continue;
        }

        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        let kind = if metadata.file_type().is_symlink() {
            WorkspaceNodeKind::Symlink
        } else if metadata.is_file() {
            WorkspaceNodeKind::File
        } else if metadata.is_dir() {
            WorkspaceNodeKind::Directory
        } else {
            WorkspaceNodeKind::Other
        };
        nodes.push(WorkspaceNode {
            path: path.clone(),
            relative_path,
            kind,
            size_bytes: metadata.len(),
            mode: file_mode(&metadata),
        });
        if kind == WorkspaceNodeKind::Directory {
            let mut nested = children(&path, &mut visited, options.max_entries)?;
            nested.reverse();
            pending.extend(nested);
        }
    }
    Ok(nodes)
}

fn is_path_at_or_below(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_matches('/');
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('/'))
}

fn portable_relative(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "workspace entry escaped the project root".to_string())?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_str().ok_or_else(|| {
                "project paths must be valid Unicode to move between operating systems".to_string()
            })?),
            _ => return Err("workspace contains a non-portable path".into()),
        }
    }
    Ok(parts.join("/"))
}

#[cfg(unix)]
fn file_mode(metadata: &std::fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(metadata.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn file_mode(_metadata: &std::fs::Metadata) -> Option<u32> {
    None
}

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_is_sorted_and_does_not_follow_symlinks() {
        let root =
            std::env::temp_dir().join(format!("wisp_workspace_scan_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::write(root.join("z.txt"), b"z").unwrap();
        std::fs::write(root.join("nested/a.txt"), b"a").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("nested"), root.join("link")).unwrap();

        let nodes = scan_workspace(&root, &WorkspaceScanOptions::default()).unwrap();
        let paths = nodes
            .iter()
            .map(|node| node.relative_path.as_str())
            .collect::<Vec<_>>();
        #[cfg(unix)]
        assert_eq!(paths, vec!["link", "nested", "nested/a.txt", "z.txt"]);
        #[cfg(not(unix))]
        assert_eq!(paths, vec!["nested", "nested/a.txt", "z.txt"]);
        #[cfg(unix)]
        assert_eq!(nodes[0].kind, WorkspaceNodeKind::Symlink);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn scan_enforces_limits_and_relative_prefix_exclusions() {
        let root = std::env::temp_dir().join(format!(
            "wisp_workspace_scan_limit_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join(".git/objects")).unwrap();
        std::fs::write(root.join(".git/objects/blob"), b"hidden").unwrap();
        std::fs::write(root.join("visible"), b"visible").unwrap();

        let nodes = scan_workspace(
            &root,
            &WorkspaceScanOptions {
                excluded_relative_prefixes: vec![".git".into()],
                ..WorkspaceScanOptions::default()
            },
        )
        .unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].relative_path, "visible");

        let error = scan_workspace(
            &root,
            &WorkspaceScanOptions {
                max_entries: 1,
                ..WorkspaceScanOptions::default()
            },
        )
        .unwrap_err();
        assert!(error.contains("more than 1 entries"));
        let _ = std::fs::remove_dir_all(root);
    }
}
