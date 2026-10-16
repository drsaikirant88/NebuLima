//! Volume models and operations against the active profile's Docker engine. `Volume`
//! mirrors `src/types/ipc.ts` exactly. `attachedContainerNames` is computed by
//! cross-referencing every container's `Mounts` entries against this volume's name, the
//! same cross-referencing pattern `docker/images.rs` uses for `inUse` — `bollard`'s volume
//! listing has no equivalent built-in reference count.
use crate::IpcError;
use bollard::models::{ContainerSummary, Volume as BollardVolume};
use bollard::query_parameters::{
    ListContainersOptions, ListVolumesOptions, PruneVolumesOptions, RemoveVolumeOptions,
};
use bollard::Docker;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub name: String,
    pub size_bytes: Option<u64>,
    pub attached_container_names: Vec<String>,
    pub profile_name: String,
}

fn container_display_name(summary: &ContainerSummary) -> String {
    summary
        .names
        .clone()
        .and_then(|names| names.into_iter().next())
        .map(|n| n.trim_start_matches('/').to_string())
        .unwrap_or_default()
}

/// Names of every container whose `Mounts` reference `volume_name` as a named volume
/// (bind mounts and tmpfs are skipped — only `"volume"`-typed entries have a meaningful
/// `Name`; `MountPoint.typ` is a plain `String` in bollard-stubs, not an enum).
fn attached_container_names(volume_name: &str, containers: &[ContainerSummary]) -> Vec<String> {
    containers
        .iter()
        .filter(|c| {
            c.mounts.as_ref().is_some_and(|mounts| {
                mounts.iter().any(|m| {
                    m.typ.as_deref() == Some("volume") && m.name.as_deref() == Some(volume_name)
                })
            })
        })
        .map(container_display_name)
        .collect()
}

pub fn to_volume(
    volume: BollardVolume,
    containers: &[ContainerSummary],
    profile_name: &str,
) -> Volume {
    let attached = attached_container_names(&volume.name, containers);
    Volume {
        size_bytes: volume.usage_data.map(|u| u.size.max(0) as u64),
        attached_container_names: attached,
        name: volume.name,
        profile_name: profile_name.to_string(),
    }
}

async fn list_containers_all(docker: &Docker) -> Vec<ContainerSummary> {
    let options = ListContainersOptions {
        all: true,
        ..Default::default()
    };
    docker
        .list_containers(Some(options))
        .await
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

pub async fn list(docker: &Docker, profile_name: &str) -> Result<Vec<Volume>, IpcError> {
    let containers = list_containers_all(docker).await;
    let response = docker
        .list_volumes(None::<ListVolumesOptions>)
        .await
        .map_err(|err| {
            IpcError::with_detail(
                "DOCKER_API_ERROR",
                "Failed to list volumes",
                err.to_string(),
            )
        })?;
    Ok(response
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| to_volume(v, &containers, profile_name))
        .collect())
}

/// "All profiles" fan-out — see `docker::containers::list_many`'s doc comment for the
/// per-profile-error-tolerance rationale.
pub async fn list_many(targets: &[(String, Arc<Docker>)]) -> Vec<Volume> {
    let futures = targets.iter().map(|(name, docker)| list(docker, name));
    let results = futures_util::future::join_all(futures).await;
    results
        .into_iter()
        .flat_map(|result| result.unwrap_or_default())
        .collect()
}

/// Whether `name` has any attached container, used to decide whether `delete_volume`
/// requires confirmation.
pub async fn is_in_use(docker: &Docker, name: &str) -> bool {
    let containers = list_containers_all(docker).await;
    !attached_container_names(name, &containers).is_empty()
}

/// Removes every unattached volume, returning the names Docker actually deleted. The
/// `all=true` filter is required here — without it, the Docker API (same as the `docker
/// volume prune` CLI without `--all`) only removes *anonymous* volumes, leaving every
/// user-named volume (the only kind this app's UI ever shows) untouched.
pub async fn prune(docker: &Docker) -> Result<Vec<String>, IpcError> {
    let options = PruneVolumesOptions {
        filters: Some(HashMap::from([(
            "all".to_string(),
            vec!["true".to_string()],
        )])),
    };
    let response = docker.prune_volumes(Some(options)).await.map_err(|err| {
        IpcError::with_detail(
            "DOCKER_API_ERROR",
            "Failed to prune volumes",
            err.to_string(),
        )
    })?;
    Ok(response.volumes_deleted.unwrap_or_default())
}

pub async fn remove(docker: &Docker, name: &str) -> Result<(), IpcError> {
    let options = RemoveVolumeOptions { force: false };
    docker
        .remove_volume(name, Some(options))
        .await
        .map_err(|err| {
            IpcError::with_detail(
                "DOCKER_API_ERROR",
                format!("Failed to remove volume '{name}'"),
                err.to_string(),
            )
        })?;
    Ok(())
}
