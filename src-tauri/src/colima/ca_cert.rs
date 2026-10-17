//! CA-certificate trust for a profile's VM. Colima has no concept of "trust this CA inside
//! the VM" — this layers it on top via `colima ssh` once the VM is already running: write the
//! bundle to `/usr/local/share/ca-certificates/host-ca-bundle.crt` as root, then run
//! `update-ca-certificates`.
//!
//! The chosen cert file (or the system-wide bundle `gather_system_ca_bundle` assembles) is
//! piped straight into `colima ssh -- sh -c 'sudo tee ... > /dev/null'`'s stdin rather than
//! copied in via a host-path mount, so there is no dependency on that path being reachable
//! from inside the VM. `CommandChild` has no `Drop` impl, so writing the bytes and then
//! dropping the child (as opposed to calling `.kill()`, which SIGKILLs) just closes the
//! stdin pipe and delivers EOF to `tee`, exactly like a real file would.
//!
//! Unlike `kubernetes/cli.rs`'s `-p <profile> kubernetes <verb>` (global flag before the
//! subcommand), `ssh` takes its profile flag after the subcommand:
//! `colima ssh --profile <profile> -- <cmd>`.
use crate::state::{CaCertConfig, CaCertMode};
use crate::IpcError;
use std::time::Duration;
use tauri::async_runtime::Receiver;
use tauri::AppHandle;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// Bounded wait for the ssh-based trust steps, mirroring `kubernetes/cli.rs`'s
/// `LIFECYCLE_ACTION_TIMEOUT` precedent for a `colima ssh` call that could hang.
const CA_TRUST_TIMEOUT: Duration = Duration::from_secs(30);
const REMOTE_CERT_PATH: &str = "/usr/local/share/ca-certificates/host-ca-bundle.crt";

/// Gathers the host's system CA bundle for the "bundle system certs" mode. Individual
/// keychain/bundle lookups are best-effort (mirrors the original snippet's `|| true`) since
/// not every keychain exists on every machine; only an entirely empty result is an error.
pub async fn gather_system_ca_bundle(app: &AppHandle) -> Result<Vec<u8>, IpcError> {
    let mut bundle = Vec::new();

    match std::env::consts::OS {
        "macos" => {
            for keychain in [
                "/System/Library/Keychains/SystemRootCertificates.keychain",
                "/Library/Keychains/System.keychain",
            ] {
                if let Ok(output) = app
                    .shell()
                    .command("security")
                    .args(["find-certificate", "-a", "-p", keychain])
                    .output()
                    .await
                {
                    bundle.extend_from_slice(&output.stdout);
                }
            }
            if let Ok(home) = std::env::var("HOME") {
                let login_keychain = format!("{home}/Library/Keychains/login.keychain-db");
                if std::path::Path::new(&login_keychain).is_file() {
                    if let Ok(output) = app
                        .shell()
                        .command("security")
                        .args(["find-certificate", "-a", "-p", &login_keychain])
                        .output()
                        .await
                    {
                        bundle.extend_from_slice(&output.stdout);
                    }
                }
            }
        }
        "linux" => {
            for path in [
                "/etc/ssl/certs/ca-certificates.crt",
                "/etc/pki/tls/certs/ca-bundle.crt",
            ] {
                if let Ok(contents) = std::fs::read(path) {
                    bundle = contents;
                    break;
                }
            }
        }
        _ => {}
    }

    if bundle.is_empty() {
        return Err(IpcError::new(
            "CA_BUNDLE_EMPTY",
            "Could not gather any system CA certificates from this host",
        ));
    }
    Ok(bundle)
}

async fn spawn_colima_ssh(
    app: &AppHandle,
    profile: &str,
    remote_args: &[&str],
) -> Result<(Receiver<CommandEvent>, CommandChild, String), IpcError> {
    let mut args = vec![
        "ssh".to_string(),
        "--profile".to_string(),
        profile.to_string(),
        "--".to_string(),
    ];
    args.extend(remote_args.iter().map(|s| s.to_string()));
    let label = args.join(" ");

    let (rx, child) = app
        .shell()
        .command("colima")
        .args(args)
        .spawn()
        .map_err(|e| {
            IpcError::with_detail(
                "COLIMA_EXEC_FAILED",
                "Failed to execute the colima CLI",
                e.to_string(),
            )
        })?;
    Ok((rx, child, label))
}

/// Drains `rx` until a `Terminated` event (or the receiver closes), bounded by
/// `CA_TRUST_TIMEOUT` — `colima ssh` has no documented non-interactive flag, so an
/// undocumented hang is possible (same risk `kubernetes/cli.rs` already flags for other
/// subcommands).
async fn drain_until_terminated(
    mut rx: Receiver<CommandEvent>,
    label: &str,
) -> Result<(), IpcError> {
    let mut stderr = Vec::new();
    let outcome = tokio::time::timeout(CA_TRUST_TIMEOUT, async {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stderr(bytes) => stderr.extend_from_slice(&bytes),
                CommandEvent::Error(message) => return Err(message),
                CommandEvent::Terminated(payload) if payload.code == Some(0) => return Ok(()),
                CommandEvent::Terminated(payload) => {
                    return Err(format!("exited with code {:?}", payload.code));
                }
                CommandEvent::Stdout(_) => {}
                _ => {}
            }
        }
        Err("process ended without reporting a result".to_string())
    })
    .await;

    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(message)) => Err(IpcError::with_detail(
            "COLIMA_COMMAND_FAILED",
            format!("`colima {label}` failed"),
            format!("{message}\n{}", String::from_utf8_lossy(&stderr)),
        )),
        Err(_elapsed) => Err(IpcError::with_detail(
            "COLIMA_SSH_TIMEOUT",
            format!(
                "`colima {label}` did not complete within {}s",
                CA_TRUST_TIMEOUT.as_secs()
            ),
            "the CA bundle may not have been trusted inside the VM",
        )),
    }
}

async fn run_ssh(app: &AppHandle, profile: &str, remote_args: &[&str]) -> Result<(), IpcError> {
    let (rx, _child, label) = spawn_colima_ssh(app, profile, remote_args).await?;
    drain_until_terminated(rx, &label).await
}

async fn run_ssh_with_stdin(
    app: &AppHandle,
    profile: &str,
    remote_args: &[&str],
    stdin_bytes: &[u8],
) -> Result<(), IpcError> {
    let (rx, mut child, label) = spawn_colima_ssh(app, profile, remote_args).await?;
    child.write(stdin_bytes).map_err(|e| {
        IpcError::with_detail(
            "COLIMA_EXEC_FAILED",
            "Failed to write to the colima ssh stdin pipe",
            e.to_string(),
        )
    })?;
    // No `impl Drop` on `CommandChild` — dropping it here just closes the stdin pipe and
    // delivers EOF to the remote `tee`, without killing it.
    drop(child);
    drain_until_terminated(rx, &label).await
}

/// Applies `config` to `profile`'s already-running VM. A no-op for `CaCertMode::None`.
pub async fn trust_ca_cert(
    app: &AppHandle,
    profile: &str,
    config: &CaCertConfig,
) -> Result<(), IpcError> {
    if config.mode == CaCertMode::None {
        return Ok(());
    }

    tracing::info!(
        profile,
        "[colima] trusting this host's CA bundle inside the VM..."
    );
    let result = trust_ca_cert_inner(app, profile, config).await;
    if let Err(ref err) = result {
        tracing::warn!(
            profile,
            error = %err.message,
            "[colima] failed to trust the host CA bundle inside the VM - docker pulls/builds may fail with TLS errors."
        );
    }
    result
}

async fn trust_ca_cert_inner(
    app: &AppHandle,
    profile: &str,
    config: &CaCertConfig,
) -> Result<(), IpcError> {
    let bundle = match config.mode {
        CaCertMode::None => return Ok(()),
        CaCertMode::File => {
            let path = config.cert_path.as_deref().ok_or_else(|| {
                IpcError::new(
                    "CA_CERT_PATH_MISSING",
                    "No CA certificate file was selected",
                )
            })?;
            std::fs::read(path).map_err(|e| {
                IpcError::with_detail(
                    "CA_CERT_READ_FAILED",
                    format!("Could not read CA certificate file '{path}'"),
                    e.to_string(),
                )
            })?
        }
        CaCertMode::SystemBundle => gather_system_ca_bundle(app).await?,
    };

    let tee_cmd = format!("sudo tee {REMOTE_CERT_PATH} > /dev/null");
    run_ssh_with_stdin(app, profile, &["sh", "-c", &tee_cmd], &bundle).await?;
    run_ssh(app, profile, &["sudo", "update-ca-certificates"]).await
}
