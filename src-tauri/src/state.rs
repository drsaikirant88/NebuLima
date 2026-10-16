//! Shared application state: settings persistence, discovered profiles, active profile,
//! and the per-socket Docker client/task registry.
use crate::IpcError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinHandle;

// ---------------------------------------------------------------------------
// NebuLimaSettings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NebuLimaSettings {
    pub theme_mode: ThemeMode,
    pub last_active_profile_name: Option<String>,
}

impl Default for NebuLimaSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::Dark,
            last_active_profile_name: None,
        }
    }
}

/// Partial update payload for `update_settings`. `Option<Option<String>>` lets a
/// present-but-null field explicitly clear `lastActiveProfileName`, while an absent field
/// (via `#[serde(default)]`) leaves it untouched.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    #[serde(default)]
    pub theme_mode: Option<ThemeMode>,
    #[serde(default, deserialize_with = "deserialize_present_field")]
    pub last_active_profile_name: Option<Option<String>>,
}

/// Wraps a successfully-deserialized value in `Some(..)` unconditionally, so a field that is
/// present in the JSON body - even if its value is `null` - deserializes to `Some(None)`
/// rather than being collapsed into the outer `None` that `#[serde(default)]` uses for an
/// absent field. Without this, `Option<Option<T>>`'s derived impl can't tell "absent" apart
/// from "present and null".
fn deserialize_present_field<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

impl NebuLimaSettings {
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self).expect("settings always serialize");
        fs::write(path, raw)
    }

    pub fn apply_patch(&mut self, patch: SettingsPatch) {
        if let Some(theme_mode) = patch.theme_mode {
            self.theme_mode = theme_mode;
        }
        if let Some(name) = patch.last_active_profile_name {
            self.last_active_profile_name = name;
        }
    }
}

// ---------------------------------------------------------------------------
// ColimaProfile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerRuntime {
    Docker,
    Containerd,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    pub dns: Vec<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mount {
    pub host_path: String,
    pub guest_path: String,
    pub writable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CaCertMode {
    #[default]
    None,
    File,
    SystemBundle,
}

/// A profile's CA-trust choice (not a Colima concept — Colima has no slot for "trust this
/// CA inside the VM", so this is NebuLima's own sidecar config, persisted separately and
/// re-applied every time the profile actually starts; see `colima/ca_cert.rs`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaCertConfig {
    pub mode: CaCertMode,
    /// Only meaningful when `mode == File`: the host path of the cert to trust.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_path: Option<String>,
}

impl CaCertConfig {
    pub fn load_all(path: &Path) -> HashMap<String, CaCertConfig> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }

    pub fn save_all(configs: &HashMap<String, CaCertConfig>, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(configs).expect("ca cert configs always serialize");
        fs::write(path, raw)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColimaProfile {
    pub name: String,
    pub state: ProfileState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    pub cpu: u32,
    pub memory_gi_b: u32,
    pub disk_gi_b: u32,
    pub runtime: ContainerRuntime,
    pub kubernetes_enabled: bool,
    pub network: NetworkConfig,
    pub mounts: Vec<Mount>,
    #[serde(default)]
    pub ca_cert: CaCertConfig,
    pub docker_socket_path: Option<String>,
    pub is_active: bool,
}

impl ColimaProfile {
    /// Validates `dockerSocketPath` matches `~/.colima/<name>/docker.sock` exactly before
    /// it is ever handed to the `docker/` module.
    pub fn validated_socket_path(&self) -> Result<&str, IpcError> {
        let path = self.docker_socket_path.as_deref().ok_or_else(|| {
            IpcError::new(
                "SOCKET_UNREACHABLE",
                format!(
                    "Profile '{}' has not started yet; no Docker socket is available",
                    self.name
                ),
            )
        })?;
        crate::docker::socket::validate_colima_socket_for_profile(path, &self.name)?;
        Ok(path)
    }
}

// ---------------------------------------------------------------------------
// Kubernetes cluster lifecycle
// ---------------------------------------------------------------------------

/// Mirrors `KubernetesCluster.actionInProgress`'s non-`'none'` values — the `'none'` case is
/// represented by the entry's absence from `AppStateInner::kubernetes_actions_in_flight`
/// rather than as a variant, so "no action in flight" has no serialized/matched state of its
/// own to accidentally fall out of sync with the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClusterAction {
    Starting,
    Stopping,
    Resetting,
    Deleting,
}

// ---------------------------------------------------------------------------
// AppState — shared mutable backend state, managed by Tauri
// ---------------------------------------------------------------------------

pub struct AppStateInner {
    pub settings: NebuLimaSettings,
    pub settings_path: PathBuf,
    /// Per-profile CA-trust choices (`colima/ca_cert.rs`), keyed by profile name — Colima
    /// itself has no slot for this, so it lives in its own sidecar JSON file next to
    /// `settings_path`.
    pub ca_cert_configs: HashMap<String, CaCertConfig>,
    pub ca_cert_configs_path: PathBuf,
    pub log_dir: PathBuf,
    /// Kept alive for the process lifetime; dropping it stops the log writer thread.
    pub log_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    /// Kept alive for the process lifetime; dropping it stops the `~/.colima` filesystem watch.
    pub fs_watcher: Option<notify::RecommendedWatcher>,
    pub profiles: Vec<ColimaProfile>,
    pub active_profile_name: Option<String>,
    pub docker_clients: HashMap<String, Arc<bollard::Docker>>,
    pub stats_tasks: HashMap<String, JoinHandle<()>>,
    pub events_tasks: HashMap<String, JoinHandle<()>>,
    pub log_tasks: HashMap<String, JoinHandle<()>>,
    /// Profile names with a pending Kubernetes cluster lifecycle action, keyed to the
    /// specific action in flight — guards against two concurrent lifecycle actions racing
    /// against the same cluster.
    pub kubernetes_actions_in_flight: HashMap<String, ClusterAction>,
    /// Most recent lifecycle-action failure/timeout message per profile — populated by
    /// `start_cluster`/`stop_cluster`/`reset_cluster`/`delete_cluster` so
    /// `get_cluster_state` can surface it.
    pub kubernetes_last_error: HashMap<String, String>,
}

impl AppStateInner {
    pub fn new(settings_path: PathBuf, ca_cert_configs_path: PathBuf, log_dir: PathBuf) -> Self {
        let settings = NebuLimaSettings::load(&settings_path);
        let ca_cert_configs = CaCertConfig::load_all(&ca_cert_configs_path);
        Self {
            settings,
            settings_path,
            ca_cert_configs,
            ca_cert_configs_path,
            log_dir,
            log_guard: None,
            fs_watcher: None,
            profiles: Vec::new(),
            active_profile_name: None,
            docker_clients: HashMap::new(),
            stats_tasks: HashMap::new(),
            events_tasks: HashMap::new(),
            log_tasks: HashMap::new(),
            kubernetes_actions_in_flight: HashMap::new(),
            kubernetes_last_error: HashMap::new(),
        }
    }

    pub fn find_profile(&self, name: &str) -> Option<&ColimaProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn find_profile_mut(&mut self, name: &str) -> Option<&mut ColimaProfile> {
        self.profiles.iter_mut().find(|p| p.name == name)
    }
}

pub struct AppState(pub tokio::sync::Mutex<AppStateInner>);

/// Tears down every open Docker resource (client handle, running stats/events/log stream
/// tasks) keyed to `previous_profile_name`, so switching the active profile (via
/// `set_active_profile`) never leaves a background task streaming from a socket that is no
/// longer the active one. A no-op when there was no previous profile.
pub fn teardown_profile_resources(state: &mut AppStateInner, previous_profile_name: Option<&str>) {
    let Some(name) = previous_profile_name else {
        return;
    };
    state.docker_clients.remove(name);
    if let Some(task) = state.stats_tasks.remove(name) {
        task.abort();
    }
    if let Some(task) = state.events_tasks.remove(name) {
        task.abort();
    }
    if let Some(task) = state.log_tasks.remove(name) {
        task.abort();
    }
}

/// Pure diffing step behind `discovery.rs::reconcile_and_emit`'s teardown trigger: names
/// present in `previous_profiles` with state `Running` whose counterpart in `next_profiles`
/// is either missing (profile directory disappeared) or no longer `Running`.
pub fn profiles_dropped_from_running(
    previous_profiles: &[ColimaProfile],
    next_profiles: &[ColimaProfile],
) -> Vec<String> {
    let still_running: HashSet<&str> = next_profiles
        .iter()
        .filter(|p| p.state == ProfileState::Running)
        .map(|p| p.name.as_str())
        .collect();
    previous_profiles
        .iter()
        .filter(|p| p.state == ProfileState::Running && !still_running.contains(p.name.as_str()))
        .map(|p| p.name.clone())
        .collect()
}

/// Scoped version of [`profiles_dropped_from_running`] for Kubernetes-enabled profiles
/// only — these are exactly the profiles whose dropped-from-`Running` transition must also
/// emit a `kubernetes-cluster-changed` `unavailable` event, on top of the unconditional
/// Docker-resource teardown.
pub fn kubernetes_profiles_dropped_from_running(
    previous_profiles: &[ColimaProfile],
    next_profiles: &[ColimaProfile],
) -> Vec<String> {
    profiles_dropped_from_running(previous_profiles, next_profiles)
        .into_iter()
        .filter(|name| {
            previous_profiles
                .iter()
                .find(|p| &p.name == name)
                .is_some_and(|p| p.kubernetes_enabled)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
            ca_cert: CaCertConfig::default(),
            docker_socket_path: None,
            is_active: false,
        }
    }

    #[tokio::test]
    async fn teardown_is_a_noop_without_a_previous_profile() {
        let mut state = AppStateInner::new(
            PathBuf::from("/tmp/settings.json"),
            PathBuf::from("/tmp/ca_certs.json"),
            PathBuf::from("/tmp/logs"),
        );
        state
            .events_tasks
            .insert("default".to_string(), tokio::spawn(async {}));
        teardown_profile_resources(&mut state, None);
        assert!(state.events_tasks.contains_key("default"));
    }

    #[tokio::test]
    async fn teardown_removes_the_named_profiles_cached_resources() {
        let mut state = AppStateInner::new(
            PathBuf::from("/tmp/settings.json"),
            PathBuf::from("/tmp/ca_certs.json"),
            PathBuf::from("/tmp/logs"),
        );
        state
            .events_tasks
            .insert("default".to_string(), tokio::spawn(async {}));
        state
            .stats_tasks
            .insert("default".to_string(), tokio::spawn(async {}));
        teardown_profile_resources(&mut state, Some("default"));
        assert!(!state.events_tasks.contains_key("default"));
        assert!(!state.stats_tasks.contains_key("default"));
    }

    /// Proxy for `commands::session::ensure_docker_client_for`'s cache-independence
    /// requirement — calling it successively for two different profile names must not evict
    /// either's cached client. The function itself takes a Tauri `AppHandle`/`State`, which
    /// are never constructed in a unit test; this exercises the exact `docker_clients` cache
    /// keying its "check cache, else connect and insert" logic relies on.
    #[test]
    fn docker_clients_cache_is_keyed_by_profile_name_independently() {
        // bollard's `connect_with_unix` checks `Path::exists()` on the socket before building the
        // client, so the path must point at a real (if empty and never dialed) file.
        let dir = tempfile::tempdir().unwrap();
        let sock_a = dir.path().join("nebulima-test-a.sock");
        let sock_b = dir.path().join("nebulima-test-b.sock");
        fs::write(&sock_a, b"").unwrap();
        fs::write(&sock_b, b"").unwrap();

        let mut state = AppStateInner::new(
            PathBuf::from("/tmp/settings.json"),
            PathBuf::from("/tmp/ca_certs.json"),
            PathBuf::from("/tmp/logs"),
        );
        let client_a = Arc::new(
            bollard::Docker::connect_with_unix(
                sock_a.to_str().unwrap(),
                5,
                bollard::API_DEFAULT_VERSION,
            )
            .unwrap(),
        );
        let client_b = Arc::new(
            bollard::Docker::connect_with_unix(
                sock_b.to_str().unwrap(),
                5,
                bollard::API_DEFAULT_VERSION,
            )
            .unwrap(),
        );
        state.docker_clients.insert("a".to_string(), client_a);
        state.docker_clients.insert("b".to_string(), client_b);
        assert!(state.docker_clients.contains_key("a"));
        assert!(state.docker_clients.contains_key("b"));
        assert_eq!(state.docker_clients.len(), 2);
    }

    #[test]
    fn profiles_dropped_from_running_ignores_an_active_profile_switch() {
        // Switching which profile is `isActive` must NOT, by itself, be treated as a drop:
        // both snapshots list the same two profiles, both still `Running`, only `isActive`
        // differs.
        let previous = vec![
            {
                let mut p = sample_profile("a", ProfileState::Running);
                p.is_active = true;
                p
            },
            sample_profile("b", ProfileState::Running),
        ];
        let next = vec![sample_profile("a", ProfileState::Running), {
            let mut p = sample_profile("b", ProfileState::Running);
            p.is_active = true;
            p
        }];
        assert!(profiles_dropped_from_running(&previous, &next).is_empty());
    }

    #[test]
    fn profiles_dropped_from_running_detects_a_stop_transition() {
        let previous = vec![sample_profile("a", ProfileState::Running)];
        let next = vec![sample_profile("a", ProfileState::Stopped)];
        assert_eq!(
            profiles_dropped_from_running(&previous, &next),
            vec!["a".to_string()]
        );
    }

    #[test]
    fn profiles_dropped_from_running_detects_a_removed_profile_directory() {
        let previous = vec![sample_profile("a", ProfileState::Running)];
        let next: Vec<ColimaProfile> = vec![];
        assert_eq!(
            profiles_dropped_from_running(&previous, &next),
            vec!["a".to_string()]
        );
    }

    #[test]
    fn profiles_dropped_from_running_ignores_a_profile_that_was_never_running() {
        let previous = vec![sample_profile("a", ProfileState::Stopped)];
        let next: Vec<ColimaProfile> = vec![];
        assert!(profiles_dropped_from_running(&previous, &next).is_empty());
    }

    #[test]
    fn kubernetes_profiles_dropped_from_running_detects_a_kubernetes_enabled_stop_transition() {
        let mut running = sample_profile("default", ProfileState::Running);
        running.kubernetes_enabled = true;
        let previous = vec![running];
        let next = vec![sample_profile("default", ProfileState::Stopped)];
        assert_eq!(
            kubernetes_profiles_dropped_from_running(&previous, &next),
            vec!["default".to_string()]
        );
    }

    #[test]
    fn kubernetes_profiles_dropped_from_running_ignores_a_non_kubernetes_stop_transition() {
        let previous = vec![sample_profile("default", ProfileState::Running)];
        let next = vec![sample_profile("default", ProfileState::Stopped)];
        assert!(kubernetes_profiles_dropped_from_running(&previous, &next).is_empty());
    }

    #[test]
    fn kubernetes_profiles_dropped_from_running_ignores_an_active_profile_switch() {
        let mut running = sample_profile("default", ProfileState::Running);
        running.kubernetes_enabled = true;
        let previous = vec![running.clone()];
        let mut still_running = running;
        still_running.is_active = true;
        let next = vec![still_running];
        assert!(kubernetes_profiles_dropped_from_running(&previous, &next).is_empty());
    }
}
