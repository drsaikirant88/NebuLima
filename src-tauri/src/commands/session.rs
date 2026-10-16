//! Shared "resolve the active profile, then ensure a connected (and cached) Docker client
//! plus its events/stats watchers" sequence used by every command module that talks to the
//! active profile's Docker engine (`containers`, `images`, `volumes`).
use crate::docker::{containers, events};
use crate::state::{AppState, ColimaProfile, ProfileState};
use crate::IpcError;
use bollard::Docker;
use std::sync::Arc;
use tauri::{AppHandle, State};

/// Resolves which profile(s) a Containers/Images/Volumes "list" call should query:
/// `Some(name)` requires that profile to exist (else `PROFILE_NOT_FOUND`) and be `Running`
/// (else `PROFILE_STOPPED`); `None` ("all profiles") returns every currently-`Running`
/// profile — which may be an empty list, not an error.
pub fn resolve_target_profiles<'a>(
    profiles: &'a [ColimaProfile],
    requested: Option<&str>,
) -> Result<Vec<&'a ColimaProfile>, IpcError> {
    match requested {
        Some(name) => {
            let profile = crate::commands::profiles::find_profile(profiles, name)?;
            if profile.state != ProfileState::Running {
                return Err(IpcError::new(
                    "PROFILE_STOPPED",
                    format!("Profile '{name}' is not running"),
                ));
            }
            Ok(vec![profile])
        }
        None => Ok(profiles
            .iter()
            .filter(|p| p.state == ProfileState::Running)
            .collect()),
    }
}

/// Lazily connects (and caches) a Docker client for `profile_name`, and lazily spawns its
/// events watcher and stats watcher the first time any command touches this profile. Holding
/// the state lock across the connect `.await` makes "check cache, else connect and spawn"
/// atomic, so two concurrent commands against a not-yet-cached profile can't race into
/// connecting or spawning the watchers twice.
///
/// Usable for any currently-`Running` profile, not just the active one — clients are torn
/// down only when a profile stops/is removed (`state::teardown_profile_resources`), never
/// merely because a different profile became active.
pub async fn ensure_docker_client_for(
    app: &AppHandle,
    state: &State<'_, Arc<AppState>>,
    profile_name: &str,
    socket_path: &str,
) -> Result<Arc<Docker>, IpcError> {
    let mut guard = state.0.lock().await;
    if let Some(client) = guard.docker_clients.get(profile_name) {
        return Ok(client.clone());
    }

    let docker = Arc::new(containers::connect(socket_path, profile_name).await?);
    guard
        .docker_clients
        .insert(profile_name.to_string(), docker.clone());

    let events_handle =
        events::spawn_watcher(app.clone(), docker.clone(), profile_name.to_string());
    guard
        .events_tasks
        .insert(profile_name.to_string(), events_handle);

    let stats_handle =
        containers::spawn_stats_watcher(app.clone(), docker.clone(), profile_name.to_string());
    guard
        .stats_tasks
        .insert(profile_name.to_string(), stats_handle);

    Ok(docker)
}
