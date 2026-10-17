//! Colima CLI execution abstraction.
//!
//! Every invocation goes through `tauri-plugin-shell`'s scoped `Command::new("colima")` API.
//! No raw shell strings are ever built from user input: each configurable `ColimaProfile`
//! field maps to one fixed, validated flag/value pair taken from the real `colima
//! --help`/`colima start --help` output.
//!
//! Note on `create`/`update`: Colima has no "create without starting" primitive — the only
//! way to materialize or change a profile's persisted `colima.yaml` is `colima start
//! <profile> [flags]` (which also boots the VM). To leave the profile `stopped` afterward
//! (never hand-editing `colima.yaml`), `update_profile` applies the requested flags via
//! `start` and then immediately `stop`s the profile. `create_profile` does the same by
//! default, but the caller may opt out (`keepActive`) and leave the newly configured VM
//! running instead.
use crate::IpcError;
use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColimaListEntry {
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProfileConfigArgs {
    pub cpu: u32,
    pub memory_gi_b: u32,
    pub disk_gi_b: u32,
    pub runtime: String,
    pub kubernetes_enabled: bool,
    pub dns: Vec<String>,
    pub network_address: bool,
    /// `(host_path, guest_path, writable)`
    pub mounts: Vec<(String, String, bool)>,
}

pub struct ColimaCli;

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

/// Builds the fixed `colima start <profile> [flags]` argument vector for the given config.
/// Exposed separately from `run()` so unit tests can assert on argument construction without
/// executing a real process.
pub fn build_start_args(profile: &str, cfg: &ProfileConfigArgs) -> Vec<String> {
    let mut args = vec!["start".to_string(), profile.to_string()];
    args.push("--cpus".into());
    args.push(cfg.cpu.to_string());
    args.push("--memory".into());
    args.push(cfg.memory_gi_b.to_string());
    args.push("--disk".into());
    args.push(cfg.disk_gi_b.to_string());
    args.push("--runtime".into());
    args.push(cfg.runtime.clone());
    args.push(format!("--kubernetes={}", cfg.kubernetes_enabled));
    for dns in &cfg.dns {
        args.push("--dns".into());
        args.push(dns.clone());
    }
    if cfg.network_address {
        args.push("--network-address".into());
    }
    for (host, guest, writable) in &cfg.mounts {
        let suffix = if *writable { ":w" } else { "" };
        args.push("--mount".into());
        args.push(format!("{}:{}{}", host, guest, suffix));
    }
    args.push("--save-config".into());
    args
}

pub fn build_stop_args(profile: &str) -> Vec<String> {
    vec!["stop".to_string(), profile.to_string()]
}

pub fn build_delete_args(profile: &str, wipe_assets: bool) -> Vec<String> {
    let mut args = vec![
        "delete".to_string(),
        profile.to_string(),
        "--force".to_string(),
    ];
    if wipe_assets {
        args.push("-d".to_string());
    }
    args
}

pub fn build_list_args() -> Vec<String> {
    vec!["list".to_string(), "--json".to_string()]
}

impl ColimaCli {
    /// `colima list --json` emits newline-delimited JSON objects, and (per the real CLI,
    /// confirmed locally) exits non-zero with "error retrieving instances" when there are
    /// zero profiles — that specific failure is treated as an empty list, not an error.
    pub async fn list(app: &AppHandle) -> Result<Vec<ColimaListEntry>, IpcError> {
        let args = build_list_args();
        let stdout = match run(app, &args).await {
            Ok(stdout) => stdout,
            Err(err)
                if err
                    .detail
                    .as_deref()
                    .unwrap_or("")
                    .contains("error retrieving instances") =>
            {
                return Ok(Vec::new());
            }
            Err(err) => return Err(err),
        };

        let mut entries = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let entry: ColimaListEntry = serde_json::from_str(line).map_err(|e| {
                IpcError::with_detail(
                    "COLIMA_PARSE_FAILED",
                    "Could not parse `colima list --json` output",
                    e.to_string(),
                )
            })?;
            entries.push(entry);
        }
        Ok(entries)
    }

    /// Applies `cfg` to `profile` via `colima start`, then stops it unless `keep_running` is
    /// set — the create/update primitive described in the module docs above. `update_profile`
    /// always passes `keep_running: false` (editing requires the profile to already be
    /// stopped, so it must end stopped too); `create_profile` passes through the caller's
    /// opt-in "keep active" choice.
    pub async fn apply_config(
        app: &AppHandle,
        profile: &str,
        cfg: &ProfileConfigArgs,
        keep_running: bool,
    ) -> Result<(), IpcError> {
        run(app, &build_start_args(profile, cfg)).await?;
        if keep_running {
            return Ok(());
        }
        run(app, &build_stop_args(profile)).await.map(|_| ())
    }

    pub async fn start(app: &AppHandle, profile: &str) -> Result<(), IpcError> {
        // Starting an already-configured profile: no resource flags needed, Colima
        // reuses the persisted colima.yaml.
        run(app, &["start".to_string(), profile.to_string()])
            .await
            .map(|_| ())
    }

    pub async fn stop(app: &AppHandle, profile: &str) -> Result<(), IpcError> {
        run(app, &build_stop_args(profile)).await.map(|_| ())
    }

    pub async fn delete(app: &AppHandle, profile: &str, wipe_assets: bool) -> Result<(), IpcError> {
        run(app, &build_delete_args(profile, wipe_assets))
            .await
            .map(|_| ())
    }
}
