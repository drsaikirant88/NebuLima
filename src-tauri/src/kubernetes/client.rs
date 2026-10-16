//! Read-only Kubernetes API client for the embedded k3s cluster. Every call here is a typed
//! `GET`/`LIST` against `Node`/`Pod` — never `apply`/`patch`/`create`/`delete` — and is
//! scoped to exactly one kubeconfig context resolved below, never a default/merged/ambient
//! kubeconfig lookup.
use crate::docker::socket::home_dir;
use crate::IpcError;
use k8s_openapi::api::core::v1::{Node, Pod};
use kube::api::ListParams;
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Api, Client, Config};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Bounded wait for any single API-server query so an unreachable/hung cluster cannot block
/// the caller indefinitely.
const CLUSTER_QUERY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterNode {
    pub name: String,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodPhase {
    Pending,
    Running,
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterPod {
    pub name: String,
    pub namespace: String,
    pub phase: PodPhase,
}

fn default_kubeconfig_path() -> Result<PathBuf, IpcError> {
    Ok(home_dir()?.join(".kube").join("config"))
}

/// Resolves the kubeconfig context Colima itself writes for `name`: `colima` for the
/// profile literally named `default`, `colima-<profile>` otherwise.
pub fn resolve_context_for_profile(name: &str, kubeconfig_path: &Path) -> Result<String, IpcError> {
    let kubeconfig = Kubeconfig::read_from(kubeconfig_path).map_err(|e| {
        IpcError::with_detail(
            "KUBECONFIG_READ_FAILED",
            "Failed to read the kubeconfig file",
            e.to_string(),
        )
    })?;

    let candidate = if name == "default" {
        "colima".to_string()
    } else {
        format!("colima-{name}")
    };

    kubeconfig
        .contexts
        .iter()
        .find(|context| context.name == candidate)
        .map(|context| context.name.clone())
        .ok_or_else(|| {
            IpcError::new(
                "KUBERNETES_CONTEXT_NOT_FOUND",
                format!("No kubeconfig context named '{candidate}' was found for profile '{name}'"),
            )
        })
}

async fn client_for_context(kubeconfig_path: &Path, context: &str) -> Result<Client, IpcError> {
    let kubeconfig = Kubeconfig::read_from(kubeconfig_path).map_err(|e| {
        IpcError::with_detail(
            "KUBECONFIG_READ_FAILED",
            "Failed to read the kubeconfig file",
            e.to_string(),
        )
    })?;

    let options = KubeConfigOptions {
        context: Some(context.to_string()),
        ..Default::default()
    };

    let config = Config::from_custom_kubeconfig(kubeconfig, &options)
        .await
        .map_err(|e| {
            IpcError::with_detail(
                "KUBERNETES_CONFIG_FAILED",
                format!("Failed to build a client configuration for context '{context}'"),
                e.to_string(),
            )
        })?;

    Client::try_from(config).map_err(|e| {
        IpcError::with_detail(
            "KUBERNETES_CLIENT_FAILED",
            "Failed to construct a Kubernetes API client",
            e.to_string(),
        )
    })
}

fn node_is_ready(node: &Node) -> bool {
    node.status
        .as_ref()
        .and_then(|status| status.conditions.as_ref())
        .is_some_and(|conditions| {
            conditions
                .iter()
                .any(|c| c.type_ == "Ready" && c.status == "True")
        })
}

/// Lists cluster nodes and their readiness via a read-only `Api::<Node>::list` call against
/// the context resolved for `profile_name`, bounded by `CLUSTER_QUERY_TIMEOUT` so an
/// unreachable API server cannot hang the caller.
pub async fn get_node_status(profile_name: &str) -> Result<Vec<ClusterNode>, IpcError> {
    let kubeconfig_path = default_kubeconfig_path()?;
    let context = resolve_context_for_profile(profile_name, &kubeconfig_path)?;
    let client = client_for_context(&kubeconfig_path, &context).await?;
    let api: Api<Node> = Api::all(client);

    let nodes = tokio::time::timeout(CLUSTER_QUERY_TIMEOUT, api.list(&ListParams::default()))
        .await
        .map_err(|_elapsed| {
            IpcError::with_detail(
                "KUBERNETES_QUERY_TIMEOUT",
                "Timed out querying the Kubernetes API server for node status",
                format!("exceeded {}s", CLUSTER_QUERY_TIMEOUT.as_secs()),
            )
        })?
        .map_err(|e| {
            IpcError::with_detail(
                "KUBERNETES_QUERY_FAILED",
                "Failed to query the Kubernetes API server for node status",
                e.to_string(),
            )
        })?;

    Ok(nodes
        .items
        .iter()
        .map(|node| ClusterNode {
            name: node.metadata.name.clone().unwrap_or_default(),
            ready: node_is_ready(node),
        })
        .collect())
}

fn pod_phase(pod: &Pod) -> PodPhase {
    match pod
        .status
        .as_ref()
        .and_then(|status| status.phase.as_deref())
    {
        Some("Pending") => PodPhase::Pending,
        Some("Running") => PodPhase::Running,
        Some("Succeeded") => PodPhase::Succeeded,
        Some("Failed") => PodPhase::Failed,
        _ => PodPhase::Unknown,
    }
}

/// Pure mapping from the raw API object to our domain type — factored out (same convention
/// as `docker::containers::to_container`/`docker::images::to_image`/`docker::volumes::to_volume`)
/// so it is directly unit-testable against constructed fixture `Pod` values without a live
/// cluster.
pub fn to_cluster_pod(pod: &Pod) -> ClusterPod {
    ClusterPod {
        name: pod.metadata.name.clone().unwrap_or_default(),
        namespace: pod.metadata.namespace.clone().unwrap_or_default(),
        phase: pod_phase(pod),
    }
}

/// Lists pods scheduled on the cluster via a read-only `Api::<Pod>::list` call across every
/// namespace, bounded by `CLUSTER_QUERY_TIMEOUT` so an unreachable API server cannot hang
/// the caller, same shape as `get_node_status` above. Not reachable from
/// `get_cluster_state`; only `list_cluster_pods` calls this.
pub async fn list_pods(profile_name: &str) -> Result<Vec<ClusterPod>, IpcError> {
    let kubeconfig_path = default_kubeconfig_path()?;
    let context = resolve_context_for_profile(profile_name, &kubeconfig_path)?;
    let client = client_for_context(&kubeconfig_path, &context).await?;
    let api: Api<Pod> = Api::all(client);

    let pods = tokio::time::timeout(CLUSTER_QUERY_TIMEOUT, api.list(&ListParams::default()))
        .await
        .map_err(|_elapsed| {
            IpcError::with_detail(
                "KUBERNETES_QUERY_TIMEOUT",
                "Timed out querying the Kubernetes API server for pod status",
                format!("exceeded {}s", CLUSTER_QUERY_TIMEOUT.as_secs()),
            )
        })?
        .map_err(|e| {
            IpcError::with_detail(
                "KUBERNETES_QUERY_FAILED",
                "Failed to query the Kubernetes API server for pod status",
                e.to_string(),
            )
        })?;

    Ok(pods.items.iter().map(to_cluster_pod).collect())
}
