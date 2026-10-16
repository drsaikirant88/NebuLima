//! Kubernetes cluster lifecycle IPC commands. `KubernetesCluster` is never persisted — every
//! field is recomputed from the current `ColimaProfile`/`kubernetes_actions_in_flight`/
//! `kubernetes_last_error` state and (when not short-circuited) a live
//! `kubernetes::client::get_node_status` call.
use crate::kubernetes::cli::ColimaKubernetesCli;
use crate::kubernetes::client::{get_node_status, list_pods, ClusterNode, ClusterPod};
use crate::state::{AppState, ClusterAction, ColimaProfile, ProfileState};
use crate::IpcError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KubernetesClusterState {
    Unavailable,
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

/// Mirrors `state::ClusterAction` but adds the `None` variant for the "no action in flight"
/// case — kept separate from `ClusterAction` deliberately, since that type's own doc comment
/// explains why its map key space has no `None` variant of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KubernetesClusterAction {
    None,
    Starting,
    Stopping,
    Resetting,
    Deleting,
}

impl From<ClusterAction> for KubernetesClusterAction {
    fn from(action: ClusterAction) -> Self {
        match action {
            ClusterAction::Starting => Self::Starting,
            ClusterAction::Stopping => Self::Stopping,
            ClusterAction::Resetting => Self::Resetting,
            ClusterAction::Deleting => Self::Deleting,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesCluster {
    pub profile_name: String,
    pub state: KubernetesClusterState,
    pub nodes: Vec<ClusterNode>,
    pub action_in_progress: KubernetesClusterAction,
    pub last_error: Option<String>,
}

/// Payload shape for the `kubernetes-cluster-changed` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesClusterChangedEvent {
    pub profile_name: String,
    pub cluster: KubernetesCluster,
}

/// Builds the `unavailable` snapshot `discovery.rs::transition_and_emit`/`reconcile_and_emit`
/// emits whenever a Kubernetes-enabled profile leaves `Running` — shares the same field
/// defaults `get_cluster_state`'s own short-circuit above uses.
pub fn unavailable_cluster(
    profile_name: String,
    action_in_progress: KubernetesClusterAction,
    last_error: Option<String>,
) -> KubernetesCluster {
    KubernetesCluster {
        profile_name,
        state: KubernetesClusterState::Unavailable,
        nodes: Vec::new(),
        action_in_progress,
        last_error,
    }
}

/// `get_cluster_state` short-circuits to `unavailable` — with no API-server call attempted —
/// whenever the owning profile has Kubernetes disabled or isn't `Running`. Factored out as a
/// pure function so this short-circuit is directly unit-testable without a client stub/mock.
pub fn short_circuit_state(profile: &ColimaProfile) -> Option<KubernetesClusterState> {
    if !profile.kubernetes_enabled || profile.state != ProfileState::Running {
        Some(KubernetesClusterState::Unavailable)
    } else {
        None
    }
}

/// Reachable with at least one ready node ⇒ `running`; reachable but not yet all ready (or no
/// nodes reported yet) ⇒ `starting` (covers `stopped --(start)--> starting --(ready)-->
/// running`).
fn derive_reachable_state(nodes: &[ClusterNode]) -> KubernetesClusterState {
    if !nodes.is_empty() && nodes.iter().all(|node| node.ready) {
        KubernetesClusterState::Running
    } else {
        KubernetesClusterState::Starting
    }
}

#[tauri::command]
pub async fn get_cluster_state(
    state: State<'_, Arc<AppState>>,
    profile: String,
) -> Result<KubernetesCluster, IpcError> {
    compute_cluster_state(state.inner(), profile).await
}

/// Live-queries the current `KubernetesCluster` snapshot for `profile`, built once as a plain
/// `&AppState` function (rather than inline inside the `#[tauri::command]` above) so both the
/// command itself and `run_lifecycle_action`'s "always re-query, never assume" step share one
/// implementation without either needing a Tauri `State<'_, Arc<AppState>>` wrapper, which
/// only the IPC entry point actually has.
pub async fn compute_cluster_state(
    state: &AppState,
    profile: String,
) -> Result<KubernetesCluster, IpcError> {
    let (profile_snapshot, action_in_progress, stored_last_error) = {
        let guard = state.0.lock().await;
        let found = guard
            .profiles
            .iter()
            .find(|p| p.name == profile)
            .cloned()
            .ok_or_else(|| {
                IpcError::new(
                    "PROFILE_NOT_FOUND",
                    format!("No profile named '{profile}' exists"),
                )
            })?;
        let action_in_progress = guard
            .kubernetes_actions_in_flight
            .get(&profile)
            .copied()
            .map(KubernetesClusterAction::from)
            .unwrap_or(KubernetesClusterAction::None);
        let stored_last_error = guard.kubernetes_last_error.get(&profile).cloned();
        (found, action_in_progress, stored_last_error)
    };

    if let Some(cluster_state) = short_circuit_state(&profile_snapshot) {
        return Ok(KubernetesCluster {
            profile_name: profile,
            state: cluster_state,
            nodes: Vec::new(),
            action_in_progress,
            last_error: stored_last_error,
        });
    }

    // Distinguishing a cleanly-stopped-but-momentarily-unreachable cluster from a genuinely
    // erroring one would require matching `kube`'s error variants against real cluster
    // failure modes, so any query failure here is simply reported as `error`.
    match get_node_status(&profile).await {
        Ok(nodes) => Ok(KubernetesCluster {
            state: derive_reachable_state(&nodes),
            profile_name: profile,
            nodes,
            action_in_progress,
            last_error: stored_last_error,
        }),
        Err(err) => Ok(KubernetesCluster {
            profile_name: profile,
            state: KubernetesClusterState::Error,
            nodes: Vec::new(),
            action_in_progress,
            last_error: stored_last_error.or_else(|| Some(err.detail.unwrap_or(err.message))),
        }),
    }
}

/// Pure decision step behind the Kubernetes safety-net poll: emits only when `current.state`
/// differs from the last-emitted value recorded for this profile (or none has been recorded
/// yet), so a cluster that stays in the same state across consecutive polls never re-emits an
/// identical event.
pub fn should_emit_cluster_update(
    last_emitted: &HashMap<String, KubernetesClusterState>,
    current: &KubernetesCluster,
) -> bool {
    last_emitted.get(&current.profile_name) != Some(&current.state)
}

/// Low-frequency safety-net poll catching a Kubernetes cluster state change that
/// `discovery.rs`'s profile-level `kubernetes-cluster-changed` emission can't see on its own —
/// e.g. the embedded k3s cluster becomes unreachable while its owning Colima profile is still
/// `Running`. Only re-queries profiles that are both `kubernetesEnabled` and currently
/// `Running`; emits only when the derived state differs from the last value emitted for that
/// profile. Runs forever — the caller spawns this on a background task.
pub async fn run_safety_net_poll(app: AppHandle, state: Arc<AppState>, interval: Duration) {
    let mut last_emitted: HashMap<String, KubernetesClusterState> = HashMap::new();
    loop {
        tokio::time::sleep(interval).await;

        let candidates: Vec<String> = {
            let guard = state.0.lock().await;
            guard
                .profiles
                .iter()
                .filter(|p| p.kubernetes_enabled && p.state == ProfileState::Running)
                .map(|p| p.name.clone())
                .collect()
        };

        for profile_name in candidates {
            let Ok(cluster) = compute_cluster_state(&state, profile_name.clone()).await else {
                continue;
            };
            if should_emit_cluster_update(&last_emitted, &cluster) {
                last_emitted.insert(profile_name.clone(), cluster.state);
                let _ = app.emit(
                    "kubernetes-cluster-changed",
                    KubernetesClusterChangedEvent {
                        profile_name,
                        cluster,
                    },
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cluster lifecycle actions
// ---------------------------------------------------------------------------

/// Every lifecycle action requires the owning profile to exist and be `Running` before it may
/// be dispatched to `ColimaKubernetesCli`. Factored out as a pure function so it is directly
/// unit-testable.
pub fn ensure_profile_running(profiles: &[ColimaProfile], name: &str) -> Result<(), IpcError> {
    let profile = profiles.iter().find(|p| p.name == name).ok_or_else(|| {
        IpcError::new(
            "PROFILE_NOT_FOUND",
            format!("No profile named '{name}' exists"),
        )
    })?;
    if profile.state != ProfileState::Running {
        return Err(IpcError::new(
            "PROFILE_NOT_RUNNING",
            format!(
                "Profile '{name}' must be running before its Kubernetes cluster can be controlled"
            ),
        ));
    }
    Ok(())
}

/// `reset_cluster`/`delete_cluster` require an explicit `confirmed: true` before anything
/// destructive is dispatched.
pub fn ensure_lifecycle_confirmed(confirmed: bool, profile_name: &str) -> Result<(), IpcError> {
    if !confirmed {
        return Err(IpcError::new(
            "CONFIRMATION_REQUIRED",
            format!(
                "This action on profile '{profile_name}'s Kubernetes cluster requires confirmation"
            ),
        ));
    }
    Ok(())
}

/// Guards against two concurrent lifecycle actions racing against the same cluster. Pure over
/// a plain `&mut HashMap` so the race-guard itself is directly unit-testable without
/// constructing a real `AppState`.
pub fn begin_cluster_action(
    actions_in_flight: &mut HashMap<String, ClusterAction>,
    name: &str,
    action: ClusterAction,
) -> Result<(), IpcError> {
    if actions_in_flight.contains_key(name) {
        return Err(IpcError::new(
            "KUBERNETES_ACTION_IN_PROGRESS",
            format!("A Kubernetes cluster action is already in progress for profile '{name}'"),
        ));
    }
    actions_in_flight.insert(name.to_string(), action);
    Ok(())
}

/// Shared driver behind `start_cluster`/`stop_cluster`/`reset_cluster`/`delete_cluster`:
/// validate the profile is `Running` and no other action is already in flight, mark the
/// action in flight, dispatch `cli_call`, then — regardless of success, failure, or timeout —
/// clear the in-flight marker, persist a failure reason if any, re-query
/// [`compute_cluster_state`], and emit `kubernetes-cluster-changed` with the fresh snapshot.
async fn run_lifecycle_action<F, Fut>(
    app: &AppHandle,
    state: &AppState,
    profile: &str,
    action: ClusterAction,
    failure_code: &'static str,
    cli_call: F,
) -> Result<(), IpcError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), IpcError>>,
{
    {
        let mut guard = state.0.lock().await;
        ensure_profile_running(&guard.profiles, profile)?;
        begin_cluster_action(&mut guard.kubernetes_actions_in_flight, profile, action)?;
    }

    let result = cli_call().await;

    let failure_reason = result
        .as_ref()
        .err()
        .map(|err| err.detail.clone().unwrap_or_else(|| err.message.clone()));

    {
        let mut guard = state.0.lock().await;
        guard.kubernetes_actions_in_flight.remove(profile);
        if let Some(reason) = &failure_reason {
            guard
                .kubernetes_last_error
                .insert(profile.to_string(), reason.clone());
        }
    }

    let cluster = compute_cluster_state(state, profile.to_string()).await?;
    let _ = app.emit(
        "kubernetes-cluster-changed",
        KubernetesClusterChangedEvent {
            profile_name: profile.to_string(),
            cluster,
        },
    );

    // `COLIMA_KUBERNETES_TIMEOUT` is a distinct code for `reset_cluster`/`delete_cluster` —
    // pass it through rather than remapping it to the generic per-action failure code below.
    result.map_err(|err| {
        if err.code == "COLIMA_KUBERNETES_TIMEOUT" {
            err
        } else {
            IpcError::with_detail(
                failure_code,
                format!("Failed to run this Kubernetes cluster action for profile '{profile}'"),
                err.detail.unwrap_or(err.message),
            )
        }
    })
}

#[tauri::command]
pub async fn start_cluster(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
) -> Result<(), IpcError> {
    run_lifecycle_action(
        &app,
        state.inner(),
        &profile,
        ClusterAction::Starting,
        "COLIMA_KUBERNETES_START_FAILED",
        || ColimaKubernetesCli::start(&app, &profile),
    )
    .await
}

#[tauri::command]
pub async fn stop_cluster(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
) -> Result<(), IpcError> {
    run_lifecycle_action(
        &app,
        state.inner(),
        &profile,
        ClusterAction::Stopping,
        "COLIMA_KUBERNETES_STOP_FAILED",
        || ColimaKubernetesCli::stop(&app, &profile),
    )
    .await
}

#[tauri::command]
pub async fn reset_cluster(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    confirmed: bool,
) -> Result<(), IpcError> {
    ensure_lifecycle_confirmed(confirmed, &profile)?;
    run_lifecycle_action(
        &app,
        state.inner(),
        &profile,
        ClusterAction::Resetting,
        "COLIMA_KUBERNETES_RESET_FAILED",
        || ColimaKubernetesCli::reset(&app, &profile),
    )
    .await
}

#[tauri::command]
pub async fn delete_cluster(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    profile: String,
    confirmed: bool,
) -> Result<(), IpcError> {
    ensure_lifecycle_confirmed(confirmed, &profile)?;
    run_lifecycle_action(
        &app,
        state.inner(),
        &profile,
        ClusterAction::Deleting,
        "COLIMA_KUBERNETES_DELETE_FAILED",
        || ColimaKubernetesCli::delete(&app, &profile),
    )
    .await
}

/// Short-circuits to `KUBERNETES_CLUSTER_UNAVAILABLE` whenever the cluster isn't currently
/// `running` — reuses `compute_cluster_state` both for that check and to get
/// `PROFILE_NOT_FOUND` for free, rather than re-deriving the cluster's reachability here.
#[tauri::command]
pub async fn list_cluster_pods(
    state: State<'_, Arc<AppState>>,
    profile: String,
) -> Result<Vec<ClusterPod>, IpcError> {
    let cluster = compute_cluster_state(state.inner(), profile.clone()).await?;
    if cluster.state != KubernetesClusterState::Running {
        return Err(IpcError::new(
            "KUBERNETES_CLUSTER_UNAVAILABLE",
            format!("Profile '{profile}'s Kubernetes cluster is not running"),
        ));
    }
    list_pods(&profile).await
}
