//! Image models and operations against the active profile's Docker engine.
//! `Image`/`ImageLayer`/`ImageInspectDetail` mirror `src/types/ipc.ts` exactly. `inUse` is
//! computed by cross-referencing every container's `ImageID` against this image's ID,
//! rather than trusting `ImageSummary.containers` — bollard's own docs note that count
//! isn't always calculated by the engine.
use crate::docker::containers::rfc3339_from_unix_secs;
use crate::IpcError;
use bollard::models::{ImageConfig, ImageSummary};
use bollard::query_parameters::{ListContainersOptions, ListImagesOptions, RemoveImageOptions};
use bollard::Docker;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub id: String,
    pub repository: Option<String>,
    pub tag: Option<String>,
    pub size_bytes: u64,
    pub created_at: String,
    pub in_use: bool,
    pub profile_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageLayer {
    pub digest: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInspectDetail {
    pub id: String,
    pub env: Vec<String>,
    pub cmd: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub working_dir: Option<String>,
    pub labels: HashMap<String, String>,
    pub exposed_ports: Vec<String>,
    pub layers: Vec<ImageLayer>,
}

/// Splits a Docker `RepoTags` entry (e.g. `alpine:3.19` or `registry.local:5000/foo:bar`) into
/// `(repository, tag)`. The tag separator is the last `:` *after* the last `/`, since a
/// registry host:port segment also contains a colon.
pub fn split_repo_tag(repo_tag: &str) -> (Option<String>, Option<String>) {
    if repo_tag.is_empty() || repo_tag == "<none>:<none>" {
        return (None, None);
    }
    let slash_idx = repo_tag.rfind('/').map(|i| i + 1).unwrap_or(0);
    match repo_tag[slash_idx..].rfind(':') {
        Some(rel) => {
            let colon = slash_idx + rel;
            (
                Some(repo_tag[..colon].to_string()),
                Some(repo_tag[colon + 1..].to_string()),
            )
        }
        None => (Some(repo_tag.to_string()), None),
    }
}

pub fn to_image(summary: ImageSummary, in_use_ids: &HashSet<String>, profile_name: &str) -> Image {
    let (repository, tag) = summary
        .repo_tags
        .first()
        .map(|rt| split_repo_tag(rt))
        .unwrap_or((None, None));

    Image {
        id: summary.id.clone(),
        repository,
        tag,
        size_bytes: summary.size.max(0) as u64,
        created_at: rfc3339_from_unix_secs(summary.created, 0),
        in_use: in_use_ids.contains(&summary.id),
        profile_name: profile_name.to_string(),
    }
}

async fn image_ids_in_use(docker: &Docker) -> HashSet<String> {
    let options = ListContainersOptions {
        all: true,
        ..Default::default()
    };
    docker
        .list_containers(Some(options))
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.image_id)
        .collect()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

pub async fn list(docker: &Docker, profile_name: &str) -> Result<Vec<Image>, IpcError> {
    let in_use_ids = image_ids_in_use(docker).await;
    let options = ListImagesOptions {
        all: false,
        ..Default::default()
    };
    let summaries = docker.list_images(Some(options)).await.map_err(|err| {
        IpcError::with_detail("DOCKER_API_ERROR", "Failed to list images", err.to_string())
    })?;
    Ok(summaries
        .into_iter()
        .map(|s| to_image(s, &in_use_ids, profile_name))
        .collect())
}

/// "All profiles" fan-out — see `docker::containers::list_many`'s doc comment for the
/// per-profile-error-tolerance rationale.
pub async fn list_many(targets: &[(String, Arc<Docker>)]) -> Vec<Image> {
    let futures = targets.iter().map(|(name, docker)| list(docker, name));
    let results = futures_util::future::join_all(futures).await;
    results
        .into_iter()
        .flat_map(|result| result.unwrap_or_default())
        .collect()
}

/// Whether `id` is referenced by any current container (running or stopped), used to
/// decide whether `delete_image` requires confirmation.
pub async fn is_in_use(docker: &Docker, id: &str) -> bool {
    image_ids_in_use(docker).await.contains(id)
}

/// Fetches the full inspect detail for `id`. Per-layer sizes aren't part of
/// `inspect_image`'s response (only content digests, via `RootFS.Layers`) — they're
/// reconstructed on a best-effort basis from `image_history`, matching each layer-creating
/// history entry (`size > 0`) to a digest, base-to-top. History entries that don't create a
/// filesystem layer (e.g. `ENV`/`LABEL`) report `size == 0` and are skipped, so counts should
/// normally line up with the digest list; if they don't, extra digests report `sizeBytes: 0`.
pub async fn inspect(docker: &Docker, id: &str) -> Result<ImageInspectDetail, IpcError> {
    let detail = docker.inspect_image(id).await.map_err(|err| {
        IpcError::with_detail(
            "DOCKER_API_ERROR",
            format!("Failed to inspect image '{id}'"),
            err.to_string(),
        )
    })?;

    let config = detail.config.unwrap_or_else(ImageConfig::default);
    let digests = detail
        .root_fs
        .and_then(|rfs| rfs.layers)
        .unwrap_or_default();

    let mut history = docker.image_history(id).await.unwrap_or_default();
    history.reverse();
    let sizes: Vec<u64> = history
        .into_iter()
        .filter(|h| h.size > 0)
        .map(|h| h.size as u64)
        .collect();

    let layers = digests
        .into_iter()
        .enumerate()
        .map(|(i, digest)| ImageLayer {
            digest,
            size_bytes: sizes.get(i).copied().unwrap_or(0),
        })
        .collect();

    Ok(ImageInspectDetail {
        id: detail.id.unwrap_or_else(|| id.to_string()),
        env: config.env.unwrap_or_default(),
        cmd: config.cmd,
        entrypoint: config.entrypoint,
        working_dir: config.working_dir,
        labels: config.labels.unwrap_or_default(),
        exposed_ports: config.exposed_ports.unwrap_or_default(),
        layers,
    })
}

pub async fn remove(docker: &Docker, id: &str) -> Result<(), IpcError> {
    let options = RemoveImageOptions {
        force: false,
        ..Default::default()
    };
    docker
        .remove_image(id, Some(options), None)
        .await
        .map_err(|err| {
            IpcError::with_detail(
                "DOCKER_API_ERROR",
                format!("Failed to remove image '{id}'"),
                err.to_string(),
            )
        })?;
    Ok(())
}
