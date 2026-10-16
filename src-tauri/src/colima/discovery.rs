//! Profile & socket discovery: scans `~/.colima` for profile directories, parses each
//! `colima.yaml`, detects `docker.sock` presence, and watches for filesystem changes via
//! `notify` so list views never need a manual refresh.
use crate::colima::cli::ColimaCli;
use crate::commands::kubernetes::{
    unavailable_cluster, KubernetesClusterAction, KubernetesClusterChangedEvent,
};
use crate::state::{AppState, ColimaProfile, ContainerRuntime, Mount, NetworkConfig, ProfileState};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

// ---------------------------------------------------------------------------
// colima.yaml schema
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct YamlKubernetes {
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct YamlNetwork {
    #[serde(default)]
    dns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YamlMount {
    location: String,
    #[serde(default)]
    mount_point: Option<String>,
    #[serde(default)]
    writable: bool,
}

fn default_cpu() -> u32 {
    2
}
fn default_memory() -> u32 {
    2
}
fn default_disk() -> u32 {
    100
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColimaYamlConfig {
    #[serde(default = "default_cpu")]
    cpu: u32,
    #[serde(default = "default_memory")]
    memory: u32,
    #[serde(default = "default_disk")]
    disk: u32,
    #[serde(default)]
    runtime: Option<String>,
    #[serde(default)]
    kubernetes: YamlKubernetes,
    #[serde(default)]
    network: YamlNetwork,
    #[serde(default)]
    mounts: Vec<YamlMount>,
}

impl Default for ColimaYamlConfig {
    fn default() -> Self {
        Self {
            cpu: default_cpu(),
            memory: default_memory(),
            disk: default_disk(),
            runtime: None,
            kubernetes: YamlKubernetes::default(),
            network: YamlNetwork::default(),
            mounts: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Pure scanning (no AppHandle needed — directly unit-testable)
// ---------------------------------------------------------------------------

pub fn docker_socket_path(colima_root: &Path, profile: &str) -> PathBuf {
    colima_root.join(profile).join("docker.sock")
}

/// Scans `colima_root` (normally `~/.colima`) for profile directories and parses each
/// one's `colima.yaml` + socket presence into a `ColimaProfile` snapshot. `state` is left
/// as `Stopped` here; [`apply_live_status`] cross-references `colima list --json` to fill
/// in the real lifecycle state.
pub fn scan_profiles_under(colima_root: &Path) -> Vec<ColimaProfile> {
    let entries = match std::fs::read_dir(colima_root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut profiles = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if !name.starts_with('_') => name.to_string(),
            _ => continue,
        };

        let yaml_cfg: ColimaYamlConfig = std::fs::read_to_string(path.join("colima.yaml"))
            .ok()
            .and_then(|raw| serde_yaml::from_str(&raw).ok())
            .unwrap_or_default();

        let socket_path = docker_socket_path(colima_root, &name);
        let has_socket = socket_path.is_file() || socket_path.exists();

        let runtime = match yaml_cfg.runtime.as_deref() {
            Some("containerd") => ContainerRuntime::Containerd,
            _ => ContainerRuntime::Docker,
        };

        let mounts = yaml_cfg
            .mounts
            .into_iter()
            .map(|m| {
                let guest_path = m.mount_point.clone().unwrap_or_else(|| m.location.clone());
                Mount {
                    host_path: m.location,
                    guest_path,
                    writable: m.writable,
                }
            })
            .collect();

        profiles.push(ColimaProfile {
            name: name.clone(),
            state: ProfileState::Stopped,
            error_reason: None,
            cpu: yaml_cfg.cpu,
            memory_gi_b: yaml_cfg.memory,
            disk_gi_b: yaml_cfg.disk,
            runtime,
            kubernetes_enabled: yaml_cfg.kubernetes.enabled,
            network: NetworkConfig {
                dns: yaml_cfg.network.dns,
                ip_address: None,
            },
            mounts,
            ca_cert: crate::state::CaCertConfig::default(),
            docker_socket_path: if has_socket {
                Some(socket_path.to_string_lossy().into_owned())
            } else {
                None
            },
            is_active: false,
        });
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles
}

/// Cross-references `colima list --json` entries (live VM state) onto the filesystem
/// scan's snapshot — the filesystem watch alone cannot tell `starting` from `stopped`.
pub fn apply_live_status(
    profiles: &mut [ColimaProfile],
    live_by_name: &HashMap<String, crate::colima::cli::ColimaListEntry>,
) {
    for profile in profiles.iter_mut() {
        let Some(entry) = live_by_name.get(&profile.name) else {
            continue;
        };
        profile.state = match entry.status.as_str() {
            "Running" => ProfileState::Running,
            "Stopped" => ProfileState::Stopped,
            "Starting" => ProfileState::Starting,
            "Stopping" => ProfileState::Stopping,
            other => {
                profile.error_reason = Some(format!("Colima reported status '{other}'"));
                ProfileState::Error
            }
        };
        if let Some(addr) = entry.address.as_ref().filter(|a| !a.is_empty()) {
            profile.network.ip_address = Some(addr.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Lifecycle state transitions
// ---------------------------------------------------------------------------

/// Applies an explicit lifecycle transition to the named profile in-place. Used by the
/// profile commands to reflect `starting`/`stopping` immediately — ahead of the next
/// filesystem/live-status reconciliation — and to record `error`/`errorReason` on a failed
/// start/stop so it survives until the user retries or dismisses it. `error_reason` is only
/// retained when `new_state == ProfileState::Error`; every other transition clears it.
pub fn set_profile_state(
    profiles: &mut [ColimaProfile],
    name: &str,
    new_state: ProfileState,
    error_reason: Option<String>,
) {
    if let Some(profile) = profiles.iter_mut().find(|p| p.name == name) {
        profile.state = new_state;
        profile.error_reason = if new_state == ProfileState::Error {
            error_reason
        } else {
            None
        };
    }
}

/// Emits `kubernetes-cluster-changed` with `{ state: 'unavailable', ... }` for every profile
/// name in `dropped` — reads `kubernetes_actions_in_flight`/`kubernetes_last_error` off
/// `guard` so `actionInProgress`/`lastError` stay consistent with what `get_cluster_state`
/// would itself report for that profile right now. Call this before dropping the lock guard.
fn emit_kubernetes_unavailable(
    app: &AppHandle,
    guard: &crate::state::AppStateInner,
    dropped: &[String],
) {
    for name in dropped {
        let action_in_progress = guard
            .kubernetes_actions_in_flight
            .get(name)
            .copied()
            .map(KubernetesClusterAction::from)
            .unwrap_or(KubernetesClusterAction::None);
        let last_error = guard.kubernetes_last_error.get(name).cloned();
        let cluster = unavailable_cluster(name.clone(), action_in_progress, last_error);
        let _ = app.emit(
            "kubernetes-cluster-changed",
            KubernetesClusterChangedEvent {
                profile_name: name.clone(),
                cluster,
            },
        );
    }
}

/// Locks `state`, applies [`set_profile_state`], and emits the resulting snapshot via
/// `profiles-changed` — the single path the profile commands use to reflect an optimistic
/// transition or a failure. Also emits `kubernetes-cluster-changed` for a Kubernetes-enabled
/// profile that leaves `Running` as part of this transition.
pub async fn transition_and_emit(
    app: &AppHandle,
    state: &AppState,
    name: &str,
    new_state: ProfileState,
    error_reason: Option<String>,
) {
    let mut guard = state.0.lock().await;
    let previous_snapshot = guard.profiles.clone();
    let was_running = guard.find_profile(name).map(|p| p.state) == Some(ProfileState::Running);
    set_profile_state(&mut guard.profiles, name, new_state, error_reason);
    if was_running && new_state != ProfileState::Running {
        crate::state::teardown_profile_resources(&mut guard, Some(name));
    }
    let kubernetes_dropped =
        crate::state::kubernetes_profiles_dropped_from_running(&previous_snapshot, &guard.profiles);
    emit_kubernetes_unavailable(app, &guard, &kubernetes_dropped);
    let snapshot = guard.profiles.clone();
    drop(guard);
    let _ = app.emit("profiles-changed", snapshot);
}

// ---------------------------------------------------------------------------
// Live discovery (needs an AppHandle to shell out to `colima list --json`)
// ---------------------------------------------------------------------------

pub async fn discover_profiles(app: &AppHandle, colima_root: &Path) -> Vec<ColimaProfile> {
    let mut profiles = scan_profiles_under(colima_root);
    if let Ok(live) = ColimaCli::list(app).await {
        let live_by_name: HashMap<String, _> =
            live.into_iter().map(|e| (e.name.clone(), e)).collect();
        apply_live_status(&mut profiles, &live_by_name);
    }
    profiles
}

/// Re-scans, reconciles `isActive` against the currently active profile, writes the
/// result into `AppState`, and emits `profiles-changed`. Also emits
/// `kubernetes-cluster-changed` for every Kubernetes-enabled profile that leaves `Running`
/// as part of this reconciliation.
pub async fn reconcile_and_emit(app: &AppHandle, colima_root: &Path, state: &AppState) {
    let mut discovered = discover_profiles(app, colima_root).await;
    let mut guard = state.0.lock().await;
    let active_name = guard.active_profile_name.clone();
    for profile in discovered.iter_mut() {
        profile.is_active = active_name.as_deref() == Some(profile.name.as_str());
        if let Some(cfg) = guard.ca_cert_configs.get(&profile.name) {
            profile.ca_cert = cfg.clone();
        }
    }

    // Any profile that was `Running` before this reconciliation but is no longer `Running`
    // afterwards — including one whose directory disappeared entirely — must have its
    // cached Docker client/watchers torn down; `isActive` switching alone must NOT trigger
    // this.
    for name in crate::state::profiles_dropped_from_running(&guard.profiles, &discovered) {
        crate::state::teardown_profile_resources(&mut guard, Some(&name));
    }

    let kubernetes_dropped =
        crate::state::kubernetes_profiles_dropped_from_running(&guard.profiles, &discovered);
    guard.profiles = discovered.clone();
    emit_kubernetes_unavailable(app, &guard, &kubernetes_dropped);
    drop(guard);
    let _ = app.emit("profiles-changed", discovered);
}

/// Starts the `notify`-backed filesystem watch on `colima_root`. The returned watcher must
/// be kept alive by the caller (stored in `AppState`) — dropping it stops the watch.
pub fn start_profile_watcher(
    app: AppHandle,
    colima_root: PathBuf,
    state: Arc<AppState>,
) -> notify::Result<RecommendedWatcher> {
    std::fs::create_dir_all(&colima_root).ok();
    let watch_root = colima_root.clone();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_err() {
            return;
        }
        let app = app.clone();
        let root = watch_root.clone();
        let state = state.clone();
        tauri::async_runtime::spawn(async move {
            reconcile_and_emit(&app, &root, &state).await;
        });
    })?;
    watcher.watch(&colima_root, RecursiveMode::Recursive)?;
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_profile(root: &Path, name: &str, yaml: &str, with_socket: bool) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("colima.yaml"), yaml).unwrap();
        if with_socket {
            fs::write(dir.join("docker.sock"), b"").unwrap();
        }
    }

    #[test]
    fn scans_profile_directories_and_parses_config() {
        let tmp = tempfile::tempdir().unwrap();
        write_profile(
            tmp.path(),
            "default",
            "cpu: 4\nmemory: 8\ndisk: 100\nruntime: docker\nkubernetes:\n  enabled: true\nnetwork:\n  dns:\n    - 1.1.1.1\nmounts:\n  - location: /Users/dev/work\n    mountPoint: /work\n    writable: true\n",
            true,
        );

        let profiles = scan_profiles_under(tmp.path());
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!(p.name, "default");
        assert_eq!(p.cpu, 4);
        assert_eq!(p.memory_gi_b, 8);
        assert_eq!(p.disk_gi_b, 100);
        assert!(p.kubernetes_enabled);
        assert_eq!(p.network.dns, vec!["1.1.1.1".to_string()]);
        assert_eq!(p.mounts.len(), 1);
        assert_eq!(p.mounts[0].guest_path, "/work");
        assert!(p.docker_socket_path.is_some());
        assert_eq!(p.state, ProfileState::Stopped);
    }

    #[test]
    fn ignores_underscore_prefixed_template_directories() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("_templates")).unwrap();
        write_profile(tmp.path(), "work", "cpu: 2\nmemory: 2\ndisk: 60\n", false);

        let profiles = scan_profiles_under(tmp.path());
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "work");
        assert!(profiles[0].docker_socket_path.is_none());
    }

    #[test]
    fn applies_defaults_when_colima_yaml_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("bare")).unwrap();

        let profiles = scan_profiles_under(tmp.path());
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].cpu, 2);
        assert_eq!(profiles[0].memory_gi_b, 2);
        assert_eq!(profiles[0].disk_gi_b, 100);
    }

    #[test]
    fn returns_empty_when_colima_root_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does-not-exist");
        assert!(scan_profiles_under(&missing).is_empty());
    }

    #[test]
    fn apply_live_status_maps_colima_status_strings() {
        use crate::colima::cli::ColimaListEntry;

        let mut profiles = vec![ColimaProfile {
            name: "default".into(),
            state: ProfileState::Stopped,
            error_reason: None,
            cpu: 2,
            memory_gi_b: 2,
            disk_gi_b: 100,
            runtime: ContainerRuntime::Docker,
            kubernetes_enabled: false,
            network: NetworkConfig::default(),
            mounts: vec![],
            ca_cert: crate::state::CaCertConfig::default(),
            docker_socket_path: None,
            is_active: false,
        }];

        let mut live = HashMap::new();
        live.insert(
            "default".to_string(),
            ColimaListEntry {
                name: "default".to_string(),
                status: "Running".to_string(),
                address: Some("192.168.106.2".to_string()),
            },
        );

        apply_live_status(&mut profiles, &live);
        assert_eq!(profiles[0].state, ProfileState::Running);
        assert_eq!(
            profiles[0].network.ip_address,
            Some("192.168.106.2".to_string())
        );
    }

    #[test]
    fn apply_live_status_maps_broken_to_error_with_reason() {
        use crate::colima::cli::ColimaListEntry;

        let mut profiles = vec![ColimaProfile {
            name: "default".into(),
            state: ProfileState::Running,
            error_reason: None,
            cpu: 2,
            memory_gi_b: 2,
            disk_gi_b: 100,
            runtime: ContainerRuntime::Docker,
            kubernetes_enabled: false,
            network: NetworkConfig::default(),
            mounts: vec![],
            ca_cert: crate::state::CaCertConfig::default(),
            docker_socket_path: None,
            is_active: false,
        }];

        let mut live = HashMap::new();
        live.insert(
            "default".to_string(),
            ColimaListEntry {
                name: "default".to_string(),
                status: "Broken".to_string(),
                address: None,
            },
        );

        apply_live_status(&mut profiles, &live);
        assert_eq!(profiles[0].state, ProfileState::Error);
        assert!(profiles[0].error_reason.is_some());
    }

    fn sample_profile(name: &str, state: ProfileState) -> ColimaProfile {
        ColimaProfile {
            name: name.into(),
            state,
            error_reason: None,
            cpu: 2,
            memory_gi_b: 2,
            disk_gi_b: 100,
            runtime: ContainerRuntime::Docker,
            kubernetes_enabled: false,
            network: NetworkConfig::default(),
            mounts: vec![],
            ca_cert: crate::state::CaCertConfig::default(),
            docker_socket_path: None,
            is_active: false,
        }
    }

    #[test]
    fn set_profile_state_transitions_to_starting() {
        let mut profiles = vec![sample_profile("work", ProfileState::Stopped)];
        set_profile_state(&mut profiles, "work", ProfileState::Starting, None);
        assert_eq!(profiles[0].state, ProfileState::Starting);
        assert!(profiles[0].error_reason.is_none());
    }

    #[test]
    fn set_profile_state_records_error_reason_only_for_error_state() {
        let mut profiles = vec![sample_profile("work", ProfileState::Starting)];
        set_profile_state(
            &mut profiles,
            "work",
            ProfileState::Error,
            Some("colima start failed".to_string()),
        );
        assert_eq!(profiles[0].state, ProfileState::Error);
        assert_eq!(
            profiles[0].error_reason,
            Some("colima start failed".to_string())
        );
    }

    #[test]
    fn set_profile_state_clears_error_reason_on_recovery() {
        let mut profiles = vec![sample_profile("work", ProfileState::Error)];
        profiles[0].error_reason = Some("stale failure".to_string());
        set_profile_state(&mut profiles, "work", ProfileState::Running, None);
        assert_eq!(profiles[0].state, ProfileState::Running);
        assert!(profiles[0].error_reason.is_none());
    }

    #[test]
    fn set_profile_state_ignores_unknown_profile_name() {
        let mut profiles = vec![sample_profile("work", ProfileState::Stopped)];
        set_profile_state(&mut profiles, "nonexistent", ProfileState::Running, None);
        assert_eq!(profiles[0].state, ProfileState::Stopped);
    }
}
