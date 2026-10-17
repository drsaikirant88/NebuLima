//! Diagnostic logging: rotating daily log files written via `tracing-appender` into the
//! OS-appropriate app-log directory, surfaced to the UI through `get_diagnostic_log_path`
//! so a user can attach them to a bug report without hunting for an OS-specific path.
use crate::IpcError;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

const LOG_FILE_PREFIX: &str = "nebulima.log";

/// Resolves (and creates) the per-OS application log directory via Tauri's path
/// resolver — `~/Library/Logs/<bundle-id>` on macOS, `~/.local/share/<bundle-id>/logs`
/// on Linux.
pub fn resolve_log_dir(app: &AppHandle) -> Result<PathBuf, IpcError> {
    let dir = app.path().app_log_dir().map_err(|e| {
        IpcError::with_detail(
            "LOG_DIR_UNRESOLVED",
            "Could not resolve the application log directory",
            e.to_string(),
        )
    })?;
    std::fs::create_dir_all(&dir).map_err(|e| {
        IpcError::with_detail(
            "LOG_DIR_UNRESOLVED",
            "Could not create the application log directory",
            e.to_string(),
        )
    })?;
    Ok(dir)
}

/// Initializes the global `tracing` subscriber to write daily-rotated logs under the
/// resolved log directory. The returned `WorkerGuard` must be kept alive for the
/// lifetime of the app (stored in `AppStateInner`) — dropping it stops the writer thread.
pub fn init(app: &AppHandle) -> Result<(PathBuf, WorkerGuard), IpcError> {
    let dir = resolve_log_dir(app)?;
    let file_appender = tracing_appender::rolling::daily(&dir, LOG_FILE_PREFIX);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber).map_err(|e| {
        IpcError::with_detail(
            "LOG_INIT_FAILED",
            "Could not initialize diagnostic logging",
            e.to_string(),
        )
    })?;

    Ok((dir, guard))
}
