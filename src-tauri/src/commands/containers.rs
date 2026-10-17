//! Container IPC commands: list, start, stop, delete, and log subscriptions.
use crate::commands::profiles::find_profile;
use crate::commands::session::{ensure_docker_client_for, resolve_target_profiles};
use crate::docker::containers;
use crate::docker::events::ResourceChangedPayload;
use crate::state::AppState;
use crate::IpcError;
use bollard::Docker;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

async fn emit_containers_changed(app: &AppHandle, docker: &Docker, profile_name: &str) {
    if let Ok(items) = containers::list(docker, profile_name).await {
        let payload = ResourceChangedPayload {
            profile_name: profile_name.to_string(),
            items,
        };
        let _ = app.emit("containers-changed", payload);
    }
}

/// Resolves `name`'s validated Docker socket path without holding the state lock across the
/// caller's subsequent `.await` points.
fn validated_socket_path_for(
    guard: &crate::state::AppStateInner,
    name: &str,
) -> Result<String, IpcError> {
    let profile = find_profile(&guard.profiles, name)?;
    Ok(profile.validated_socket_path()?.to_string())
}

#[tauri::command]
pub async fn list_containers(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: Option<String>,
) -> Result<Vec<containers::Container>, IpcError> {
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
        containers::list(&docker, &name).await
    } else {
        let mut targets = Vec::with_capacity(targets_info.len());
        for (name, socket_path) in targets_info {
            let docker = ensure_docker_client_for(&app, &state, &name, &socket_path).await?;
            targets.push((name, docker));
        }
        Ok(containers::list_many(&targets).await)
    }
}

#[tauri::command]
pub async fn start_container(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    id: String,
) -> Result<(), IpcError> {
    let socket_path = {
        let guard = state.0.lock().await;
        validated_socket_path_for(&guard, &profile)?
    };
    let docker = ensure_docker_client_for(&app, &state, &profile, &socket_path).await?;
    containers::start(&docker, &id).await?;
    emit_containers_changed(&app, &docker, &profile).await;
    Ok(())
}

#[tauri::command]
pub async fn stop_container(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    id: String,
) -> Result<(), IpcError> {
    let socket_path = {
        let guard = state.0.lock().await;
        validated_socket_path_for(&guard, &profile)?
    };
    let docker = ensure_docker_client_for(&app, &state, &profile, &socket_path).await?;
    containers::stop(&docker, &id).await?;
    emit_containers_changed(&app, &docker, &profile).await;
    Ok(())
}

/// Unlike profiles/images/volumes, container delete confirmation is unconditional — any
/// `confirmed != true` call is rejected regardless of the container's state.
#[tauri::command]
pub async fn delete_container(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    id: String,
    confirmed: bool,
) -> Result<(), IpcError> {
    if !confirmed {
        return Err(IpcError::new(
            "CONFIRMATION_REQUIRED",
            "Deleting a container requires confirmation",
        ));
    }
    let socket_path = {
        let guard = state.0.lock().await;
        validated_socket_path_for(&guard, &profile)?
    };
    let docker = ensure_docker_client_for(&app, &state, &profile, &socket_path).await?;
    containers::remove(&docker, &id).await?;
    emit_containers_changed(&app, &docker, &profile).await;
    Ok(())
}

#[tauri::command]
pub async fn subscribe_container_logs(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    id: String,
) -> Result<(), IpcError> {
    let socket_path = {
        let guard = state.0.lock().await;
        validated_socket_path_for(&guard, &profile)?
    };
    let docker = ensure_docker_client_for(&app, &state, &profile, &socket_path).await?;

    let handle = containers::spawn_log_watcher(app.clone(), docker, id);
    let mut guard = state.0.lock().await;
    if let Some(old) = guard.log_tasks.insert(profile, handle) {
        old.abort();
    }
    Ok(())
}

#[tauri::command]
pub async fn unsubscribe_container_logs(
    state: State<'_, Arc<AppState>>,
    profile: String,
    _id: String,
) -> Result<(), IpcError> {
    let mut guard = state.0.lock().await;
    if let Some(task) = guard.log_tasks.remove(&profile) {
        task.abort();
    }
    Ok(())
}
