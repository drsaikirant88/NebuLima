//! Volume IPC commands: list, bulk-prune, and delete.
use crate::commands::profiles::find_profile;
use crate::commands::session::{ensure_docker_client_for, resolve_target_profiles};
use crate::docker::events::ResourceChangedPayload;
use crate::docker::volumes;
use crate::state::AppState;
use crate::IpcError;
use bollard::Docker;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

async fn emit_volumes_changed(app: &AppHandle, docker: &Docker, profile_name: &str) {
    if let Ok(items) = volumes::list(docker, profile_name).await {
        let payload = ResourceChangedPayload {
            profile_name: profile_name.to_string(),
            items,
        };
        let _ = app.emit("volumes-changed", payload);
    }
}

fn validated_socket_path_for(
    guard: &crate::state::AppStateInner,
    name: &str,
) -> Result<String, IpcError> {
    let profile = find_profile(&guard.profiles, name)?;
    Ok(profile.validated_socket_path()?.to_string())
}

#[tauri::command]
pub async fn list_volumes(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: Option<String>,
) -> Result<Vec<volumes::Volume>, IpcError> {
    let targets_info: Vec<(String, String)> = {
        let guard = state.0.lock().await;
        resolve_target_profiles(&guard.profiles, profile.as_deref())?
            .into_iter()
            .map(|p| Ok((p.name.clone(), p.validated_socket_path()?.to_string())))
            .collect::<Result<Vec<_>, IpcError>>()?
    };

    if let Some(name) = profile {
        let socket_path = &targets_info[0].1;
        let docker = ensure_docker_client_for(&app, &state, &name, socket_path).await?;
        volumes::list(&docker, &name).await
    } else {
        let mut targets = Vec::with_capacity(targets_info.len());
        for (name, socket_path) in targets_info {
            let docker = ensure_docker_client_for(&app, &state, &name, &socket_path).await?;
            targets.push((name, docker));
        }
        Ok(volumes::list_many(&targets).await)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneVolumesResult {
    pub removed_names: Vec<String>,
}

/// Removes every volume with no attached container, across the targeted profile(s) — `None`
/// prunes every currently-running profile (mirrors `list_volumes`'s "all profiles" mode via
/// `resolve_target_profiles`); never requires confirmation since attached volumes are never
/// candidates for pruning.
#[tauri::command]
pub async fn prune_volumes(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: Option<String>,
) -> Result<PruneVolumesResult, IpcError> {
    let targets_info: Vec<(String, String)> = {
        let guard = state.0.lock().await;
        resolve_target_profiles(&guard.profiles, profile.as_deref())?
            .into_iter()
            .map(|p| Ok((p.name.clone(), p.validated_socket_path()?.to_string())))
            .collect::<Result<Vec<_>, IpcError>>()?
    };

    let mut removed_names = Vec::new();
    for (name, socket_path) in targets_info {
        let docker = ensure_docker_client_for(&app, &state, &name, &socket_path).await?;
        removed_names.extend(volumes::prune(&docker).await?);
        emit_volumes_changed(&app, &docker, &name).await;
    }
    Ok(PruneVolumesResult { removed_names })
}

/// Delete requires confirmation only when the volume has an attached container — mirrors
/// `delete_image`'s conditional pattern, unlike `delete_container`'s unconditional one.
#[tauri::command]
pub async fn delete_volume(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    name: String,
    confirmed: bool,
) -> Result<(), IpcError> {
    let socket_path = {
        let guard = state.0.lock().await;
        validated_socket_path_for(&guard, &profile)?
    };
    let docker = ensure_docker_client_for(&app, &state, &profile, &socket_path).await?;

    if !confirmed && volumes::is_in_use(&docker, &name).await {
        return Err(IpcError::new(
            "CONFIRMATION_REQUIRED",
            "This volume is attached to a container; deleting it requires confirmation",
        ));
    }

    volumes::remove(&docker, &name).await?;
    emit_volumes_changed(&app, &docker, &profile).await;
    Ok(())
}
