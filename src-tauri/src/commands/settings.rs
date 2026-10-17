//! Settings & diagnostics IPC commands.
use crate::state::{AppState, NebuLimaSettings, SettingsPatch};
use crate::IpcError;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, Arc<AppState>>) -> Result<NebuLimaSettings, IpcError> {
    let guard = state.0.lock().await;
    Ok(guard.settings.clone())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, Arc<AppState>>,
    patch: SettingsPatch,
) -> Result<NebuLimaSettings, IpcError> {
    let mut guard = state.0.lock().await;
    guard.settings.apply_patch(patch);
    guard.settings.save(&guard.settings_path).map_err(|e| {
        IpcError::with_detail(
            "SETTINGS_SAVE_FAILED",
            "Could not persist settings to disk",
            e.to_string(),
        )
    })?;
    Ok(guard.settings.clone())
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticLogPath {
    pub path: String,
}

#[tauri::command]
pub async fn get_diagnostic_log_path(
    state: State<'_, Arc<AppState>>,
) -> Result<DiagnosticLogPath, IpcError> {
    let guard = state.0.lock().await;
    Ok(DiagnosticLogPath {
        path: guard.log_dir.to_string_lossy().into_owned(),
    })
}
