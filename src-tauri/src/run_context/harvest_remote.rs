//! Remote output harvest for SSH-direct Runs: enumerate spec-matched files on
//! the server, pull them back with checksum verification, and register them as
//! project artifacts. Selection is the database boundary — only spec-matched
//! outputs are transferred and recorded, never the full remote file inventory.

use super::{
    checked_output, ssh_script_command, transfer_progress, RemoteRun, RemoteRunHandle,
    RunCommandRunner,
};
use crate::harvest::{HarvestedArtifact, OutputResidency, OutputSpec};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A non-bundle glob may match at most this many files before harvest refuses
/// and asks for `bundle: true` or a narrower glob.
pub(super) const HARVEST_MATCH_CAP: usize = 500;
/// Total manifest entries across all specs are capped so massive-small-file
/// runs can never flood SQLite or the UI.
pub(super) const HARVEST_MANIFEST_CAP: usize = 2000;
/// Remote persistent artifact area for oversized outputs that stay on the
/// server. Lives outside the run workdir so workspace cleanup never dangles a
/// registered `ssh://` reference.
pub(super) const REMOTE_PERSIST_ROOT: &str = ".wisp-science/artifacts";

const COLLECT_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const PULL_TIMEOUT: Duration = Duration::from_secs(4 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RemoteOutputKind {
    /// Downloadable file staged under `<workdir>/harvest/files/<rel>`.
    File,
    /// Single tar.gz archive of every file the bundle spec matched.
    Bundle { entries: u64, total_bytes: u64 },
    /// Oversized/remote-residency output moved to the persistent area.
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RemoteOutputEntry {
    pub kind: RemoteOutputKind,
    pub spec_idx: usize,
    pub size: u64,
    pub checksum: String,
    /// Workdir-relative path for files, archive name for bundles, absolute
    /// persistent path for remote references.
    pub path: String,
}

/// Globs are interpolated into a remote `sh` script, so restrict them to a
/// charset that cannot escape into command position.
pub(crate) fn validate_remote_glob(glob: &str) -> Result<(), String> {
    if glob.is_empty() || glob.len() > 512 {
        return Err("output glob must be 1..512 characters".into());
    }
    if glob.starts_with('/') || glob.starts_with('~') {
        return Err(format!("output glob must be workdir-relative: {glob}"));
    }
    if glob.split('/').any(|part| part == "..") {
        return Err(format!("output glob must not contain '..': {glob}"));
    }
    if let Some(bad) = glob
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || "._-/*?[]".contains(*c)))
    {
        return Err(format!(
            "output glob contains an unsupported character '{bad}': {glob}"
        ));
    }
    Ok(())
}

fn spec_limits(spec: &OutputSpec) -> (Option<u64>, Option<u64>) {
    let max_file = match spec.residency {
        OutputResidency::Remote => Some(0),
        OutputResidency::Local => spec.max_file_mb.map(|mb| mb.saturating_mul(1024 * 1024)),
        OutputResidency::Auto => Some(
            spec.max_file_mb
                .map(|mb| mb.saturating_mul(1024 * 1024))
                .unwrap_or(crate::snapshot_store::DEFAULT_SNAPSHOT_LIMIT),
        ),
    };
    let max_total = spec.max_total_mb.map(|mb| mb.saturating_mul(1024 * 1024));
    (max_file, max_total)
}

/// Remote collection script: classifies matched files per spec, hard-links
/// downloads into `harvest/files`, tars bundle specs, moves oversized files to
/// the persistent area, and prints one manifest line per registered output.
pub(super) fn collect_payload(
    workdir: &str,
    token: &str,
    run_id: &str,
    specs: &[(usize, OutputSpec)],
) -> String {
    let mut script = format!(
        r#"set -eu
umask 077
workdir="$HOME/{workdir}"
[ -f "$workdir/token" ] && [ "$(cat "$workdir/token")" = "{token}" ] || {{ echo 'wisp token mismatch' >&2; exit 73; }}
if command -v sha256sum >/dev/null 2>&1; then wisp_sum() {{ sha256sum "$1" | awk '{{print $1}}'; }}
elif command -v shasum >/dev/null 2>&1; then wisp_sum() {{ shasum -a 256 "$1" | awk '{{print $1}}'; }}
else echo 'remote harvest requires sha256sum or shasum on the server' >&2; exit 69; fi
base="$workdir/inputs"
harvest="$workdir/harvest"
rm -rf "$harvest"
mkdir -p "$harvest/files" "$harvest/bundles"
persist="$HOME/{persist_root}/{run_id}"
cd "$base"
emitted=0
emit_guard() {{
  emitted=$((emitted+1))
  if [ "$emitted" -gt {manifest_cap} ]; then
    printf '__WISP_HARVEST_ERROR__:harvest manifest exceeded {manifest_cap} entries; use bundle:true for many-file outputs\n'
    exit 65
  fi
}}
"#,
        persist_root = REMOTE_PERSIST_ROOT,
        manifest_cap = HARVEST_MANIFEST_CAP,
    );
    for (idx, spec) in specs {
        let glob = &spec.glob;
        if spec.bundle {
            script.push_str(&format!(
                r#"list="$harvest/bundles/list_{idx}"
: > "$list"
count=0
total=0
for f in {glob}; do
  [ -f "$f" ] || continue
  count=$((count+1))
  size=$(wc -c < "$f")
  total=$((total+size))
  printf '%s\n' "$f" >> "$list"
done
if [ "$count" -gt 0 ]; then
  command -v tar >/dev/null 2>&1 || {{ printf '__WISP_HARVEST_ERROR__:bundle outputs require tar on the server\n'; exit 69; }}
  archive="$harvest/bundles/bundle_{idx}.tar.gz"
  tar -czf "$archive" -T "$list"
  rm -f "$list"
  asize=$(wc -c < "$archive")
  asum=$(wisp_sum "$archive")
  emit_guard
  printf '__WISP_HARVEST__:bundle:{idx}:%s:%s:%s:%s:bundle_{idx}.tar.gz\n' "$asize" "$asum" "$count" "$total"
fi
"#,
            ));
            continue;
        }
        let (max_file, max_total) = spec_limits(spec);
        let mut remote_test = Vec::new();
        if let Some(max_file) = max_file {
            remote_test.push(format!("[ \"$size\" -gt {max_file} ]"));
        }
        if let Some(max_total) = max_total {
            remote_test.push(format!("[ \"$total\" -gt {max_total} ]"));
        }
        let remote_test = if remote_test.is_empty() {
            "false".to_string()
        } else {
            remote_test.join(" || ")
        };
        script.push_str(&format!(
            r#"count=0
total=0
for f in {glob}; do
  [ -f "$f" ] || continue
  count=$((count+1))
  if [ "$count" -gt {match_cap} ]; then
    printf '__WISP_HARVEST_ERROR__:output glob matched more than {match_cap} files; set bundle:true or narrow the glob to final products\n'
    exit 65
  fi
  size=$(wc -c < "$f")
  total=$((total+size))
  sum=$(wisp_sum "$f")
  if {remote_test}; then
    dest="$persist/$f"
    mkdir -p "$(dirname "$dest")"
    mv "$f" "$dest"
    emit_guard
    printf '__WISP_HARVEST__:remote:{idx}:%s:%s:%s\n' "$size" "$sum" "$dest"
  else
    mkdir -p "$harvest/files/$(dirname "$f")"
    ln "$f" "$harvest/files/$f" 2>/dev/null || cp "$f" "$harvest/files/$f"
    emit_guard
    printf '__WISP_HARVEST__:file:{idx}:%s:%s:%s\n' "$size" "$sum" "$f"
  fi
done
"#,
            match_cap = HARVEST_MATCH_CAP,
        ));
    }
    script.push_str("printf '__WISP_HARVEST_DONE__\\n'\n");
    script
}

fn safe_relative_path(value: &str) -> Result<&str, String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || value.contains('\\')
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!("harvest manifest returned an unsafe path: {value}"));
    }
    Ok(value)
}

pub(super) fn parse_collect_manifest(stdout: &str) -> Result<Vec<RemoteOutputEntry>, String> {
    if let Some(error) = stdout
        .lines()
        .find_map(|line| line.strip_prefix("__WISP_HARVEST_ERROR__:"))
    {
        return Err(error.trim().to_string());
    }
    if !stdout.lines().any(|line| line == "__WISP_HARVEST_DONE__") {
        return Err("remote harvest collection did not complete".into());
    }
    let mut entries = Vec::new();
    for line in stdout.lines() {
        let Some(value) = line.strip_prefix("__WISP_HARVEST__:") else {
            continue;
        };
        let mut parts = value.splitn(2, ':');
        let kind = parts.next().unwrap_or_default();
        let rest = parts
            .next()
            .ok_or_else(|| "harvest manifest line is malformed".to_string())?;
        let entry = match kind {
            "file" | "remote" => {
                let mut fields = rest.splitn(4, ':');
                let spec_idx = parse_field(fields.next(), "spec index")?;
                let size = parse_field(fields.next(), "size")?;
                let checksum = fields
                    .next()
                    .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                    .ok_or_else(|| "harvest manifest checksum is invalid".to_string())?
                    .to_string();
                let path = fields
                    .next()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "harvest manifest path is missing".to_string())?
                    .to_string();
                if kind == "file" {
                    safe_relative_path(&path)?;
                } else if !path.starts_with('/') {
                    return Err(format!(
                        "harvest manifest returned a non-absolute remote path: {path}"
                    ));
                }
                RemoteOutputEntry {
                    kind: if kind == "file" {
                        RemoteOutputKind::File
                    } else {
                        RemoteOutputKind::Remote
                    },
                    spec_idx,
                    size,
                    checksum,
                    path,
                }
            }
            "bundle" => {
                let mut fields = rest.splitn(6, ':');
                let spec_idx = parse_field(fields.next(), "spec index")?;
                let size = parse_field(fields.next(), "size")?;
                let checksum = fields
                    .next()
                    .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                    .ok_or_else(|| "harvest manifest checksum is invalid".to_string())?
                    .to_string();
                let entries_count = parse_field(fields.next(), "bundle entry count")?;
                let total_bytes = parse_field(fields.next(), "bundle total bytes")?;
                let name = fields
                    .next()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "harvest manifest bundle name is missing".to_string())?
                    .to_string();
                safe_relative_path(&name)?;
                RemoteOutputEntry {
                    kind: RemoteOutputKind::Bundle {
                        entries: entries_count,
                        total_bytes,
                    },
                    spec_idx,
                    size,
                    checksum,
                    path: name,
                }
            }
            other => return Err(format!("harvest manifest has unknown entry kind: {other}")),
        };
        entries.push(entry);
        if entries.len() > HARVEST_MANIFEST_CAP {
            return Err(format!(
                "harvest manifest exceeded {HARVEST_MANIFEST_CAP} entries"
            ));
        }
    }
    Ok(entries)
}

fn parse_field<T: std::str::FromStr>(value: Option<&str>, name: &str) -> Result<T, String> {
    value
        .and_then(|v| v.parse::<T>().ok())
        .ok_or_else(|| format!("harvest manifest {name} is invalid"))
}

fn sanitize_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "context".into()
    } else {
        sanitized
    }
}

/// Project-relative landing directory for one Run's pulled outputs.
pub(super) fn landing_dir(harvest_root: &Path, alias: &str, run_id: &str) -> PathBuf {
    harvest_root
        .join("remote")
        .join(sanitize_component(alias))
        .join(sanitize_component(run_id))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut input = std::io::BufReader::new(
        std::fs::File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?,
    );
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            let path = entry.path();
            match entry.metadata() {
                Ok(metadata) if metadata.is_file() => metadata.len(),
                Ok(metadata) if metadata.is_dir() => dir_size(&path),
                _ => 0,
            }
        })
        .sum()
}

fn place_file(source: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if destination.exists() {
        std::fs::remove_file(destination).map_err(|e| e.to_string())?;
    }
    std::fs::rename(source, destination).map_err(|e| e.to_string())
}

#[derive(Debug)]
struct HarvestPlan {
    entries: Vec<RemoteOutputEntry>,
    download_bytes: u64,
    download_files: u64,
}

fn plan_from_manifest(
    specs: &[(usize, OutputSpec)],
    entries: Vec<RemoteOutputEntry>,
) -> Result<HarvestPlan, String> {
    for (idx, spec) in specs {
        if spec.logical_key.is_some() {
            let matched = entries
                .iter()
                .filter(|entry| entry.spec_idx == *idx)
                .count();
            if matched > 1 {
                return Err(format!(
                    "output logical_key '{}' matched more than one file",
                    spec.logical_key.as_deref().unwrap_or_default()
                ));
            }
        }
    }
    let downloads = entries
        .iter()
        .filter(|entry| !matches!(entry.kind, RemoteOutputKind::Remote));
    let download_bytes = downloads.clone().map(|entry| entry.size).sum();
    let download_files = downloads.count() as u64;
    Ok(HarvestPlan {
        entries,
        download_bytes,
        download_files,
    })
}

/// Harvest a succeeded SSH Run: collect on the server, pull spec-matched
/// outputs through a durable `file_transfer` Run, verify checksums, register
/// artifacts, and mark the parent Run harvested.
///
/// `renew_parent` keeps the still-active parent lifecycle lease alive during
/// long pulls (the automatic path); the manual retry path passes None because
/// the parent is already terminal.
pub(super) async fn harvest_ssh_run(
    store: &wisp_store::Store,
    runner: &dyn RunCommandRunner,
    owner_id: &str,
    remote: &RemoteRun,
    renew_parent: bool,
) -> Result<Vec<HarvestedArtifact>, String> {
    let specs: Vec<(usize, OutputSpec)> = remote
        .output_specs
        .iter()
        .enumerate()
        .filter(|(_, spec)| !spec.glob.contains("://"))
        .map(|(idx, spec)| (idx, spec.clone()))
        .collect();
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    for (_, spec) in &specs {
        validate_remote_glob(&spec.glob)?;
    }
    let frame_id = remote
        .frame_id
        .as_deref()
        .ok_or_else(|| "remote harvest requires a source session".to_string())?;
    let harvest_root = remote
        .harvest_root
        .as_deref()
        .ok_or_else(|| "remote harvest requires the project workspace".to_string())?;
    let RemoteRunHandle::SshDirect {
        connection,
        workdir,
        token,
        ..
    } = &remote.handle
    else {
        return Err("remote harvest requires an SSH-direct Run".into());
    };

    let collect = checked_output(
        "SSH harvest collection",
        runner
            .run(
                ssh_script_command(
                    connection,
                    "collect SSH run outputs",
                    collect_payload(workdir, token, &remote.run_id, &specs),
                )?,
                COLLECT_TIMEOUT,
            )
            .await,
    )?;
    let plan = plan_from_manifest(&specs, parse_collect_manifest(&collect.stdout)?)?;

    let landing = landing_dir(harvest_root, &connection.alias, &remote.run_id);
    if plan.download_files > 0 {
        pull_harvest_dir(
            store,
            runner,
            owner_id,
            remote,
            connection,
            workdir,
            &plan,
            &landing,
            renew_parent,
        )
        .await?;
    }

    let mut harvested = Vec::new();
    for entry in &plan.entries {
        let spec = &remote.output_specs[entry.spec_idx];
        let artifact = match &entry.kind {
            RemoteOutputKind::Remote => {
                let relative = entry
                    .path
                    .split_once(&format!("/{}/{}/", REMOTE_PERSIST_ROOT, remote.run_id))
                    .map(|(_, rel)| rel)
                    .unwrap_or(&entry.path);
                let logical_key = spec
                    .logical_key
                    .clone()
                    .unwrap_or_else(|| format!("path:{relative}"));
                crate::harvest::register_reference_artifact(
                    store,
                    &remote.project_id,
                    frame_id,
                    &remote.run_id,
                    &spec.kind,
                    &format!("ssh://{}{}", connection.alias, entry.path),
                    Some(entry.size),
                    Some(entry.checksum.clone()),
                    &logical_key,
                )
                .await?
            }
            RemoteOutputKind::File => {
                let logical_key = spec
                    .logical_key
                    .clone()
                    .unwrap_or_else(|| format!("path:{}", entry.path));
                crate::harvest::register_local_artifact(
                    store,
                    &remote.project_id,
                    frame_id,
                    &remote.run_id,
                    &spec.kind,
                    harvest_root,
                    &landing.join(&entry.path),
                    &logical_key,
                    &entry.path,
                    entry.size > crate::snapshot_store::DEFAULT_SNAPSHOT_LIMIT,
                )
                .await?
            }
            RemoteOutputKind::Bundle { .. } => {
                let logical_key = spec
                    .logical_key
                    .clone()
                    .unwrap_or_else(|| format!("bundle:{}", spec.glob));
                crate::harvest::register_local_artifact(
                    store,
                    &remote.project_id,
                    frame_id,
                    &remote.run_id,
                    &spec.kind,
                    harvest_root,
                    &landing.join("bundles").join(&entry.path),
                    &logical_key,
                    &spec.glob,
                    entry.size > crate::snapshot_store::DEFAULT_SNAPSHOT_LIMIT,
                )
                .await?
            }
        };
        harvested.push(artifact);
    }
    store
        .mark_run_harvested(&remote.run_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(harvested)
}

/// Download `<workdir>/harvest` into the project landing directory as a
/// persisted `file_transfer` Run with progress, then verify every expected
/// checksum before any file becomes visible at its final path.
#[allow(clippy::too_many_arguments)]
async fn pull_harvest_dir(
    store: &wisp_store::Store,
    runner: &dyn RunCommandRunner,
    owner_id: &str,
    remote: &RemoteRun,
    connection: &crate::ssh_hosts::SshConnection,
    workdir: &str,
    plan: &HarvestPlan,
    landing: &Path,
    renew_parent: bool,
) -> Result<(), String> {
    let transfer_id = uuid::Uuid::new_v4().to_string();
    let started = Instant::now();
    let initial = transfer_progress(
        "download",
        "downloading",
        0,
        plan.download_bytes,
        0,
        plan.download_files,
        None,
        started,
    );
    let mut transfer = wisp_store::RunRecord::new(
        &transfer_id,
        &remote.project_id,
        format!("ssh:{}", connection.alias),
        "Harvest run outputs",
        "file_transfer",
    );
    transfer.frame_id = remote.frame_id.clone();
    transfer.command = Some(format!("harvest {}", remote.run_id));
    transfer.progress_json = serde_json::to_string(&initial).map_err(|e| e.to_string())?;
    store
        .create_run(&transfer)
        .await
        .map_err(|e| e.to_string())?;
    if !store
        .activate_run_lifecycle(
            &transfer_id,
            wisp_store::RunStatus::Running,
            owner_id,
            super::ACTIVE_LEASE_SECS,
        )
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("harvest transfer Run changed state before it could start".into());
    }
    let partial = landing.join(format!(".partial-{transfer_id}"));
    let result = pull_and_verify(
        store,
        runner,
        owner_id,
        remote,
        connection,
        workdir,
        plan,
        landing,
        &partial,
        &transfer_id,
        started,
        renew_parent,
    )
    .await;
    let _ = std::fs::remove_dir_all(&partial);
    let (status, exit_code) = match &result {
        Ok(()) => (wisp_store::RunStatus::Succeeded, Some(0)),
        Err(_) => (wisp_store::RunStatus::Failed, Some(-1)),
    };
    let final_progress = transfer_progress(
        "download",
        if result.is_ok() {
            "downloaded"
        } else {
            "failed"
        },
        if result.is_ok() {
            plan.download_bytes
        } else {
            0
        },
        plan.download_bytes,
        if result.is_ok() {
            plan.download_files
        } else {
            0
        },
        plan.download_files,
        None,
        started,
    );
    let _ = store
        .renew_run_lifecycle(&transfer_id, owner_id, super::ACTIVE_LEASE_SECS)
        .await;
    let _ = store
        .update_run_progress_owned(&transfer_id, owner_id, &final_progress)
        .await;
    if let Err(error) = &result {
        let _ = store
            .update_run_output_owned(&transfer_id, owner_id, None, Some(error))
            .await;
    }
    let _ = store
        .finish_active_run_owned(&transfer_id, owner_id, status, exit_code)
        .await;
    result
}

#[allow(clippy::too_many_arguments)]
async fn pull_and_verify(
    store: &wisp_store::Store,
    runner: &dyn RunCommandRunner,
    owner_id: &str,
    remote: &RemoteRun,
    connection: &crate::ssh_hosts::SshConnection,
    workdir: &str,
    plan: &HarvestPlan,
    landing: &Path,
    partial: &Path,
    transfer_id: &str,
    started: Instant,
    renew_parent: bool,
) -> Result<(), String> {
    std::fs::create_dir_all(partial).map_err(|e| e.to_string())?;
    let mut args = connection.scp_option_args()?;
    args.push("-r".into());
    args.push(format!("{}:{workdir}/harvest", connection.target()?));
    args.push(partial.to_string_lossy().into_owned());
    let command = super::RunCommand {
        context_id: format!("ssh:{}", connection.alias),
        program: "scp".into(),
        args,
        script: format!("harvest {}", remote.run_id),
        cwd: None,
        stdin: None,
        envs: crate::ssh_hosts::auth_envs_for_connection(connection)?,
    };
    let pull = runner.run(command, PULL_TIMEOUT);
    tokio::pin!(pull);
    let mut interval = tokio::time::interval(if cfg!(test) {
        Duration::from_millis(10)
    } else {
        Duration::from_secs(1)
    });
    interval.tick().await;
    let output = loop {
        tokio::select! {
            output = &mut pull => break output,
            _ = interval.tick() => {
                if !store
                    .renew_run_lifecycle(transfer_id, owner_id, super::ACTIVE_LEASE_SECS)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    return Err("harvest transfer lifecycle lease expired".into());
                }
                if renew_parent {
                    let _ = store
                        .renew_run_lifecycle(&remote.run_id, owner_id, super::ACTIVE_LEASE_SECS)
                        .await;
                }
                let progress = transfer_progress(
                    "download",
                    "downloading",
                    dir_size(partial),
                    plan.download_bytes,
                    0,
                    plan.download_files,
                    None,
                    started,
                );
                let _ = store
                    .update_run_progress_owned(transfer_id, owner_id, &progress)
                    .await;
            }
        }
    };
    checked_output("SSH harvest download", output)?;
    let staged = partial.join("harvest");
    for entry in &plan.entries {
        let source = match &entry.kind {
            RemoteOutputKind::File => staged.join("files").join(&entry.path),
            RemoteOutputKind::Bundle { .. } => staged.join("bundles").join(&entry.path),
            RemoteOutputKind::Remote => continue,
        };
        let size = std::fs::metadata(&source)
            .map_err(|e| {
                format!(
                    "harvested file missing after download ({}): {e}",
                    entry.path
                )
            })?
            .len();
        if size != entry.size {
            return Err(format!(
                "harvested file size mismatch for {} (expected {}, got {size})",
                entry.path, entry.size
            ));
        }
        let checksum = sha256_file(&source)?;
        if checksum != entry.checksum {
            return Err(format!(
                "harvested file checksum mismatch for {}",
                entry.path
            ));
        }
    }
    for entry in &plan.entries {
        match &entry.kind {
            RemoteOutputKind::File => place_file(
                &staged.join("files").join(&entry.path),
                &landing.join(&entry.path),
            )?,
            RemoteOutputKind::Bundle { .. } => place_file(
                &staged.join("bundles").join(&entry.path),
                &landing.join("bundles").join(&entry.path),
            )?,
            RemoteOutputKind::Remote => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(glob: &str, bundle: bool, residency: OutputResidency) -> OutputSpec {
        OutputSpec {
            glob: glob.into(),
            kind: "data".into(),
            residency,
            logical_key: None,
            max_file_mb: None,
            max_total_mb: None,
            bundle,
        }
    }

    #[test]
    fn glob_validation_blocks_shell_escapes_and_traversal() {
        for ok in [
            "results/*.tsv",
            "Trinity.fasta",
            "out/dir_1/x-?.txt",
            "a[0-9].log",
        ] {
            assert!(validate_remote_glob(ok).is_ok(), "{ok}");
        }
        for bad in [
            "results/$(rm -rf ~)",
            "a;b",
            "a b",
            "/etc/passwd",
            "~/x",
            "../secrets",
            "a/../b",
            "a`b`",
            "a'b",
            "a\"b",
            "",
        ] {
            assert!(validate_remote_glob(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn collect_payload_encodes_caps_bundles_and_persist_relocation() {
        let specs = vec![
            (0, spec("results/*.tsv", false, OutputResidency::Auto)),
            (1, spec("parts/*", true, OutputResidency::Auto)),
            (2, spec("big/*.bam", false, OutputResidency::Remote)),
        ];
        let payload = collect_payload(".wisp-science/runs/r1", "tok", "r1", &specs);
        assert!(payload.contains(&format!("\"$count\" -gt {HARVEST_MATCH_CAP}")));
        assert!(payload.contains("tar -czf"));
        assert!(payload.contains("bundle_1.tar.gz"));
        assert!(payload.contains("mv \"$f\" \"$dest\""));
        assert!(payload.contains(&format!("persist=\"$HOME/{REMOTE_PERSIST_ROOT}/r1\"")));
        assert!(payload.contains("__WISP_HARVEST_DONE__"));
        // Remote residency always relocates: the test is constant-true.
        assert!(payload.contains("[ \"$size\" -gt 0 ]"));
    }

    #[test]
    fn manifest_parses_files_bundles_and_remote_references() {
        let checksum = "ab".repeat(32);
        let stdout = format!(
            "noise\n__WISP_HARVEST__:file:0:10:{checksum}:results/out.tsv\n\
             __WISP_HARVEST__:bundle:1:99:{checksum}:132481:987654:bundle_1.tar.gz\n\
             __WISP_HARVEST__:remote:2:5:{checksum}:/home/u/.wisp-science/artifacts/r/big/x.bam\n\
             __WISP_HARVEST_DONE__\n"
        );
        let entries = parse_collect_manifest(&stdout).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].kind, RemoteOutputKind::File);
        assert_eq!(entries[0].path, "results/out.tsv");
        assert_eq!(
            entries[1].kind,
            RemoteOutputKind::Bundle {
                entries: 132481,
                total_bytes: 987654
            }
        );
        assert_eq!(entries[2].kind, RemoteOutputKind::Remote);
        assert!(entries[2].path.starts_with('/'));
    }

    #[test]
    fn manifest_requires_completion_sentinel_and_surfaces_errors() {
        let checksum = "ab".repeat(32);
        assert!(parse_collect_manifest(&format!(
            "__WISP_HARVEST__:file:0:10:{checksum}:results/out.tsv\n"
        ))
        .unwrap_err()
        .contains("did not complete"));
        assert!(parse_collect_manifest(
            "__WISP_HARVEST_ERROR__:output glob matched more than 500 files; set bundle:true\n"
        )
        .unwrap_err()
        .contains("bundle:true"));
    }

    #[test]
    fn manifest_rejects_unsafe_paths_and_bad_checksums() {
        let checksum = "ab".repeat(32);
        for bad in [
            format!("__WISP_HARVEST__:file:0:10:{checksum}:../evil\n__WISP_HARVEST_DONE__\n"),
            format!("__WISP_HARVEST__:file:0:10:{checksum}:/abs/path\n__WISP_HARVEST_DONE__\n"),
            format!(
                "__WISP_HARVEST__:remote:0:10:{checksum}:relative/path\n__WISP_HARVEST_DONE__\n"
            ),
            "__WISP_HARVEST__:file:0:10:zz:results/x\n__WISP_HARVEST_DONE__\n".to_string(),
        ] {
            assert!(parse_collect_manifest(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn logical_key_specs_reject_multiple_matches() {
        let mut keyed = spec("results/*.tsv", false, OutputResidency::Auto);
        keyed.logical_key = Some("figure:main".into());
        let checksum = "ab".repeat(32);
        let entries = parse_collect_manifest(&format!(
            "__WISP_HARVEST__:file:0:10:{checksum}:results/a.tsv\n\
             __WISP_HARVEST__:file:0:10:{checksum}:results/b.tsv\n\
             __WISP_HARVEST_DONE__\n"
        ))
        .unwrap();
        let error = plan_from_manifest(&[(0, keyed)], entries).unwrap_err();
        assert!(error.contains("more than one file"), "{error}");
    }

    #[test]
    fn landing_dir_sanitizes_alias_components() {
        let dir = landing_dir(Path::new("/proj"), "gpu box/../x", "run-1");
        assert_eq!(dir, Path::new("/proj/remote/gpu_box_.._x/run-1"));
    }
}
