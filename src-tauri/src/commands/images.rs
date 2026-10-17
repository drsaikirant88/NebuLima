//! Image IPC commands: list, inspect, and delete.
use crate::commands::profiles::find_profile;
use crate::commands::session::{ensure_docker_client_for, resolve_target_profiles};
use crate::docker::events::ResourceChangedPayload;
use crate::docker::images;
use crate::state::AppState;
use crate::IpcError;
use bollard::Docker;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

async fn emit_images_changed(app: &AppHandle, docker: &Docker, profile_name: &str) {
    if let Ok(items) = images::list(docker, profile_name).await {
        let payload = ResourceChangedPayload {
            profile_name: profile_name.to_string(),
            items,
        };
        let _ = app.emit("images-changed", payload);
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
pub async fn list_images(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: Option<String>,
) -> Result<Vec<images::Image>, IpcError> {
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
        images::list(&docker, &name).await
    } else {
        let mut targets = Vec::with_capacity(targets_info.len());
        for (name, socket_path) in targets_info {
            let docker = ensure_docker_client_for(&app, &state, &name, &socket_path).await?;
            targets.push((name, docker));
        }
        Ok(images::list_many(&targets).await)
    }
}

#[tauri::command]
pub async fn inspect_image(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    id: String,
) -> Result<images::ImageInspectDetail, IpcError> {
    let socket_path = {
        let guard = state.0.lock().await;
        validated_socket_path_for(&guard, &profile)?
    };
    let docker = ensure_docker_client_for(&app, &state, &profile, &socket_path).await?;
    images::inspect(&docker, &id).await
}

/// Delete requires confirmation only when the image is referenced by a current container —
/// unlike `delete_container` (always unconditional), this mirrors `delete_profile`'s
/// conditional pattern.
#[tauri::command]
pub async fn delete_image(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    id: String,
    confirmed: bool,
) -> Result<(), IpcError> {
    let socket_path = {
        let guard = state.0.lock().await;
        validated_socket_path_for(&guard, &profile)?
    };
    let docker = ensure_docker_client_for(&app, &state, &profile, &socket_path).await?;

    if !confirmed && images::is_in_use(&docker, &id).await {
        return Err(IpcError::new(
            "CONFIRMATION_REQUIRED",
            "This image is used by a container; deleting it requires confirmation",
        ));
    }

    images::remove(&docker, &id).await?;
    emit_images_changed(&app, &docker, &profile).await;
    Ok(())
}
