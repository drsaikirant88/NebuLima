//! Kubernetes cluster lifecycle CLI execution abstraction. Every invocation goes through
//! `tauri-plugin-shell`'s scoped `Command::new("colima")` API — mirroring `colima/cli.rs`'s
//! pattern exactly: fixed argument vectors built from validated inputs only, never a
//! raw/user-controlled shell string.
//!
//! Per the real locally-installed `colima kubernetes --help` (and its `start`/`stop`/`reset`/
//! `delete` subcommand help), the profile is selected via the global `-p <profile>` flag, which
//! must precede the `kubernetes` subcommand itself; none of the four subcommands documents a
//! `--force`/non-interactive flag.
//!
//! `reset`/`delete` have no documented non-interactive flag, which risks an undocumented
//! interactive stdin prompt hanging the subprocess indefinitely. The bounded timeout below
//! is the load-bearing mitigation for that case: on elapse, a distinct
//! `COLIMA_KUBERNETES_TIMEOUT` error is returned instead of hanging the caller or assuming
//! success.
use crate::IpcError;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

/// Bounded wait for `reset`/`delete` — generous enough for a real cluster teardown, short
/// enough that the UI never appears permanently hung.
const LIFECYCLE_ACTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub struct ColimaKubernetesCli;

async fn run(app: &AppHandle, args: &[String]) -> Result<String, IpcError> {
    let output = app
        .shell()
        .command("colima")
        .args(args.to_vec())
        .output()
        .await
        .map_err(|e| {
            IpcError::with_detail(
                "COLIMA_EXEC_FAILED",
                "Failed to execute the colima CLI",
                e.to_string(),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(IpcError::with_detail(
            "COLIMA_COMMAND_FAILED",
            format!("`colima {}` exited with a non-zero status", args.join(" ")),
            stderr,
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Runs `run(app, args)` under `LIFECYCLE_ACTION_TIMEOUT`, mapping an elapsed timeout to a
/// distinct `COLIMA_KUBERNETES_TIMEOUT` error rather than hanging the caller or allowing the
/// timeout to be mistaken for success/failure of the underlying command.
async fn run_with_timeout(app: &AppHandle, args: &[String]) -> Result<String, IpcError> {
    match tokio::time::timeout(LIFECYCLE_ACTION_TIMEOUT, run(app, args)).await {
        Ok(result) => result,
        Err(_elapsed) => Err(IpcError::with_detail(
            "COLIMA_KUBERNETES_TIMEOUT",
            format!(
                "`colima {}` did not complete within {}s",
                args.join(" "),
                LIFECYCLE_ACTION_TIMEOUT.as_secs()
            ),
            "the cluster state must be re-queried rather than assumed",
        )),
    }
}

/// Builds the fixed `-p <profile> kubernetes start` argument vector. Exposed separately from
/// `run()` so unit tests can assert on argument construction without executing a real
/// process.
pub fn build_start_args(profile: &str) -> Vec<String> {
    vec![
        "-p".to_string(),
        profile.to_string(),
        "kubernetes".to_string(),
        "start".to_string(),
    ]
}

pub fn build_stop_args(profile: &str) -> Vec<String> {
    vec![
        "-p".to_string(),
        profile.to_string(),
        "kubernetes".to_string(),
        "stop".to_string(),
    ]
}

pub fn build_reset_args(profile: &str) -> Vec<String> {
    vec![
        "-p".to_string(),
        profile.to_string(),
        "kubernetes".to_string(),
        "reset".to_string(),
    ]
}

pub fn build_delete_args(profile: &str) -> Vec<String> {
    vec![
        "-p".to_string(),
        profile.to_string(),
        "kubernetes".to_string(),
        "delete".to_string(),
    ]
}

impl ColimaKubernetesCli {
    pub async fn start(app: &AppHandle, profile: &str) -> Result<(), IpcError> {
        run(app, &build_start_args(profile)).await.map(|_| ())
    }

    pub async fn stop(app: &AppHandle, profile: &str) -> Result<(), IpcError> {
        run(app, &build_stop_args(profile)).await.map(|_| ())
    }

    /// May block on an undocumented interactive prompt (module docs above) — bounded by
    /// `LIFECYCLE_ACTION_TIMEOUT`.
    pub async fn reset(app: &AppHandle, profile: &str) -> Result<(), IpcError> {
        run_with_timeout(app, &build_reset_args(profile))
            .await
            .map(|_| ())
    }

    /// May block on an undocumented interactive prompt (module docs above) — bounded by
    /// `LIFECYCLE_ACTION_TIMEOUT`.
    pub async fn delete(app: &AppHandle, profile: &str) -> Result<(), IpcError> {
        run_with_timeout(app, &build_delete_args(profile))
            .await
            .map(|_| ())
    }
}
