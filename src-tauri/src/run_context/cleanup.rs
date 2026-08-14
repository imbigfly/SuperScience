//! Post-run server workspace cleanup. Deletes exactly one run workdir after
//! the run is terminal and its declared outputs were harvested (or the user
//! explicitly confirmed data loss), so servers never accumulate garbage.

use super::{checked_output, ssh_script_command, RemoteRunHandle, RunCommandRunner};
use std::time::Duration;

pub(super) const CLEANUP_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DONE_MARKER: &str = "__WISP_CLEANUP__:done";

/// The only path cleanup will ever delete: a HOME-relative workdir that ends
/// with this run's id, with no traversal or expansion tricks. Never trusts a
/// string that could resolve to `~`, `/`, or another run's directory.
pub(super) fn validate_cleanup_workdir(workdir: &str, run_id: &str) -> Result<(), String> {
    if workdir.trim().is_empty() || workdir.len() > 512 {
        return Err("run workdir path is empty or too long".into());
    }
    if let Some(bad) = workdir
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || "._-/".contains(*c)))
    {
        return Err(format!(
            "run workdir contains an unsupported character '{bad}'"
        ));
    }
    if workdir.starts_with('/') {
        return Err("run workdir must be HOME-relative".into());
    }
    let segments: Vec<&str> = workdir.split('/').collect();
    if segments.len() < 2
        || segments
            .iter()
            .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return Err("run workdir must be a nested HOME-relative path".into());
    }
    if segments.last() != Some(&run_id) {
        return Err("run workdir does not belong to this run".into());
    }
    Ok(())
}

fn posix_cleanup_payload(workdir: &str, token: &str) -> String {
    format!(
        r#"set -eu
workdir="$HOME/{workdir}"
if [ ! -d "$workdir" ]; then
  printf '{DONE_MARKER}\n'
  exit 0
fi
[ -f "$workdir/token" ] && [ "$(cat "$workdir/token")" = "{token}" ] || {{ echo 'wisp token mismatch' >&2; exit 73; }}
if [ -f "$workdir/_submitted" ]; then
  handle=$(cat "$workdir/_submitted")
  rest=${{handle#*:}}
  pgid=${{rest%%:*}}
  start=${{handle##*:}}
  current=$(awk '{{print $22}}' "/proc/$pgid/stat" 2>/dev/null || true)
  group=$(awk '{{print $5}}' "/proc/$pgid/stat" 2>/dev/null || true)
  if [ -n "$pgid" ] && [ "$current" = "$start" ] && [ "$group" = "$pgid" ]; then
    kill -KILL "-$pgid" 2>/dev/null || true
    sleep 1
  fi
fi
rm -rf "$workdir"
printf '{DONE_MARKER}\n'
"#
    )
}

fn windows_cleanup_payload(workdir: &str, token: &str) -> String {
    let windows_workdir = workdir.replace('/', "\\");
    format!(
        r#"$ErrorActionPreference = 'Stop'
$workdir = Join-Path $HOME '{windows_workdir}'
if (-not (Test-Path -LiteralPath $workdir)) {{
  Write-Output '{DONE_MARKER}'
  exit 0
}}
$tokenPath = Join-Path $workdir 'token'
if (-not (Test-Path -LiteralPath $tokenPath) -or (Get-Content -LiteralPath $tokenPath -Raw).Trim() -ne '{token}') {{
  Write-Error 'wisp token mismatch'
  exit 73
}}
Remove-Item -LiteralPath $workdir -Recurse -Force
Write-Output '{DONE_MARKER}'
"#
    )
}

fn cleanup_command(
    handle: &RemoteRunHandle,
    workdir: &str,
    token: &str,
) -> Result<super::RunCommand, String> {
    match handle {
        RemoteRunHandle::SshDirect { connection, .. } => ssh_script_command(
            connection,
            "clean up run workspace",
            posix_cleanup_payload(workdir, token),
        ),
        RemoteRunHandle::LocalDetached { transport, .. } => {
            let payload = match transport {
                super::LocalTransport::Posix { .. } => posix_cleanup_payload(workdir, token),
                super::LocalTransport::Windows { .. } => windows_cleanup_payload(workdir, token),
            };
            super::local_detached::transport_script_command(
                handle,
                "clean up run workspace",
                payload,
            )
        }
    }
}

/// Delete the run's workdir on its execution context. The caller has already
/// enforced lifecycle preconditions; this function only guards the path.
pub(super) async fn delete_run_workspace(
    runner: &dyn RunCommandRunner,
    handle: &RemoteRunHandle,
    run_id: &str,
) -> Result<(), String> {
    let (workdir, token) = match handle {
        RemoteRunHandle::SshDirect { workdir, token, .. }
        | RemoteRunHandle::LocalDetached { workdir, token, .. } => {
            (workdir.as_str(), token.as_str())
        }
    };
    validate_cleanup_workdir(workdir, run_id)?;
    let output = checked_output(
        "run workspace cleanup",
        runner
            .run(cleanup_command(handle, workdir, token)?, CLEANUP_TIMEOUT)
            .await,
    )?;
    let normalized = output.stdout.replace("\r\n", "\n");
    if !normalized.lines().any(|line| line == DONE_MARKER) {
        return Err("run workspace cleanup did not confirm deletion".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workdir_validation_rejects_escapes_and_foreign_runs() {
        assert!(validate_cleanup_workdir(".wisp-science/runs/run-1", "run-1").is_ok());
        assert!(validate_cleanup_workdir("scratch/wisp-runs/run-1", "run-1").is_ok());
        for (workdir, run_id) in [
            ("", "run-1"),
            ("/etc", "etc"),
            ("run-1", "run-1"),
            ("~/runs/run-1", "run-1"),
            ("runs/../run-1", "run-1"),
            ("runs/run-2", "run-1"),
            ("runs/$HOME/run-1", "run-1"),
            ("runs/a b/run-1", "run-1"),
        ] {
            assert!(
                validate_cleanup_workdir(workdir, run_id).is_err(),
                "{workdir}"
            );
        }
    }

    #[test]
    fn posix_payload_kills_the_confirmed_group_then_removes_only_the_workdir() {
        let payload = posix_cleanup_payload(".wisp-science/runs/run-1", "tok");
        assert!(payload.contains("workdir=\"$HOME/.wisp-science/runs/run-1\""));
        assert!(payload.contains("wisp token mismatch"));
        assert!(payload.contains("kill -KILL \"-$pgid\""));
        assert!(payload.contains("rm -rf \"$workdir\""));
        assert!(!payload.contains("rm -rf \"$HOME\""));
    }

    #[test]
    fn windows_payload_uses_native_removal_under_home() {
        let payload = windows_cleanup_payload(".wisp-science/runs/run-1", "tok");
        assert!(payload.contains("Join-Path $HOME '.wisp-science\\runs\\run-1'"));
        assert!(payload.contains("Remove-Item -LiteralPath $workdir -Recurse -Force"));
        assert!(payload.contains("wisp token mismatch"));
    }
}
