//! Profile IPC commands: create, update, start/stop, delete, and switch the active profile.
use crate::colima::ca_cert::trust_ca_cert;
use crate::colima::cli::{ColimaCli, ProfileConfigArgs};
use crate::colima::discovery::{reconcile_and_emit, transition_and_emit};
use crate::docker::containers::has_running_containers;
use crate::docker::socket::home_dir;
use crate::preflight;
use crate::state::{AppState, CaCertConfig, ColimaProfile, ContainerRuntime, ProfileState};
use crate::IpcError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

fn colima_root() -> Result<PathBuf, IpcError> {
    Ok(home_dir()?.join(".colima"))
}

fn runtime_str(runtime: ContainerRuntime) -> String {
    match runtime {
        ContainerRuntime::Docker => "docker".to_string(),
        ContainerRuntime::Containerd => "containerd".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Pure validation helpers — no AppHandle/live colima process needed to test these.
// ---------------------------------------------------------------------------

/// Guards every profile name reaching `colima_cli::build_*_args` before it is placed on a
/// `colima` argv as a bare positional argument, so a name starting with `-` can't be parsed
/// as a flag instead. Colima itself lowercases profile names (they become part of the
/// VM/hostname), so this also normalizes here to keep NebuLima's state in sync with what
/// `colima list` and the on-disk `~/.colima/<name>` directory will actually use.
pub fn normalize_profile_name(name: &str) -> String {
    name.to_lowercase()
}

pub fn validate_profile_name(name: &str) -> Result<(), IpcError> {
    let valid = !name.is_empty()
        && name.len() <= 63
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if !valid {
        return Err(IpcError::new(
            "INVALID_PROFILE_NAME",
            format!(
                "Profile name '{name}' is invalid: it must start with a letter or digit and contain only letters, digits, '-', '_', or '.'"
            ),
        ));
    }
    Ok(())
}

pub fn ensure_name_available(profiles: &[ColimaProfile], name: &str) -> Result<(), IpcError> {
    if profiles.iter().any(|p| p.name == name) {
        return Err(IpcError::new(
            "PROFILE_NAME_CONFLICT",
            format!("A profile named '{name}' already exists"),
        ));
    }
    Ok(())
}

pub fn find_profile<'a>(
    profiles: &'a [ColimaProfile],
    name: &str,
) -> Result<&'a ColimaProfile, IpcError> {
    profiles.iter().find(|p| p.name == name).ok_or_else(|| {
        IpcError::new(
            "PROFILE_NOT_FOUND",
            format!("No profile named '{name}' exists"),
        )
    })
}

pub fn ensure_stopped<'a>(
    profiles: &'a [ColimaProfile],
    name: &str,
) -> Result<&'a ColimaProfile, IpcError> {
    let profile = find_profile(profiles, name)?;
    if profile.state != ProfileState::Stopped {
        return Err(IpcError::new(
            "PROFILE_NOT_STOPPED",
            format!("Profile '{name}' must be stopped before its resources/runtime can be edited"),
        ));
    }
    Ok(profile)
}

/// Deleting a profile that is active or has running containers requires explicit confirmation.
pub fn requires_delete_confirmation(is_active: bool, has_running_containers: bool) -> bool {
    is_active || has_running_containers
}

// ---------------------------------------------------------------------------
// Command input shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInput {
    #[serde(default)]
    pub dns: Vec<String>,
    #[serde(default)]
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountInput {
    pub host_path: String,
    pub guest_path: String,
    pub writable: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProfileInput {
    pub name: String,
    pub cpu: u32,
    pub memory_gi_b: u32,
    pub disk_gi_b: u32,
    pub runtime: ContainerRuntime,
    pub kubernetes_enabled: bool,
    #[serde(default)]
    pub network: NetworkInput,
    #[serde(default)]
    pub mounts: Vec<MountInput>,
    #[serde(default)]
    pub ca_cert: Option<CaCertConfig>,
    /// When `false` (the default), `create_profile` stops the VM once its configuration has
    /// been applied; `true` leaves it running.
    #[serde(default)]
    pub keep_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileInput {
    pub name: String,
    #[serde(default)]
    pub cpu: Option<u32>,
    #[serde(default)]
    pub memory_gi_b: Option<u32>,
    #[serde(default)]
    pub disk_gi_b: Option<u32>,
    #[serde(default)]
    pub runtime: Option<ContainerRuntime>,
    #[serde(default)]
    pub kubernetes_enabled: Option<bool>,
    #[serde(default)]
    pub network: Option<NetworkInput>,
    #[serde(default)]
    pub mounts: Option<Vec<MountInput>>,
    #[serde(default)]
    pub ca_cert: Option<CaCertConfig>,
}

fn config_args_for_create(input: &CreateProfileInput) -> ProfileConfigArgs {
    ProfileConfigArgs {
        cpu: input.cpu,
        memory_gi_b: input.memory_gi_b,
        disk_gi_b: input.disk_gi_b,
        runtime: runtime_str(input.runtime),
        kubernetes_enabled: input.kubernetes_enabled,
        dns: input.network.dns.clone(),
        network_address: input.network.ip_address.is_some(),
        mounts: input
            .mounts
            .iter()
            .map(|m| (m.host_path.clone(), m.guest_path.clone(), m.writable))
            .collect(),
    }
}

/// Merges `patch` onto `existing`'s current configuration — only the fields present in the
/// patch override the profile's current values.
fn config_args_for_update(
    existing: &ColimaProfile,
    patch: &UpdateProfileInput,
) -> ProfileConfigArgs {
    let dns = patch
        .network
        .as_ref()
        .map(|n| n.dns.clone())
        .unwrap_or_else(|| existing.network.dns.clone());
    let ip_address = patch
        .network
        .as_ref()
        .map(|n| n.ip_address.clone())
        .unwrap_or_else(|| existing.network.ip_address.clone());
    let mounts = patch
        .mounts
        .as_ref()
        .map(|mounts| {
            mounts
                .iter()
                .map(|m| (m.host_path.clone(), m.guest_path.clone(), m.writable))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            existing
                .mounts
                .iter()
                .map(|m| (m.host_path.clone(), m.guest_path.clone(), m.writable))
                .collect()
        });

    ProfileConfigArgs {
        cpu: patch.cpu.unwrap_or(existing.cpu),
        memory_gi_b: patch.memory_gi_b.unwrap_or(existing.memory_gi_b),
        disk_gi_b: patch.disk_gi_b.unwrap_or(existing.disk_gi_b),
        runtime: runtime_str(patch.runtime.unwrap_or(existing.runtime)),
        kubernetes_enabled: patch
            .kubernetes_enabled
            .unwrap_or(existing.kubernetes_enabled),
        dns,
        network_address: ip_address.is_some(),
        mounts,
    }
}

/// Persists `config` as `name`'s CA-trust choice. Called from `create_profile`/
/// `update_profile` only when the caller explicitly sent a `caCert` field — absent means
/// "leave whatever is already stored alone", mirroring the rest of `UpdateProfileInput`'s
/// patch semantics.
async fn persist_ca_cert_config(
    state: &AppState,
    name: &str,
    config: CaCertConfig,
) -> Result<(), IpcError> {
    let mut guard = state.0.lock().await;
    guard.ca_cert_configs.insert(name.to_string(), config);
    let configs = guard.ca_cert_configs.clone();
    let path = guard.ca_cert_configs_path.clone();
    drop(guard);
    CaCertConfig::save_all(&configs, &path).map_err(|e| {
        IpcError::with_detail(
            "CA_CERT_CONFIG_SAVE_FAILED",
            "Could not persist the CA certificate configuration to disk",
            e.to_string(),
        )
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertTrustWarningEvent {
    pub profile_name: String,
    pub message: String,
    pub severity: &'static str,
}

/// Applies `profile`'s stored CA-trust config (if any) after it has just started, emitting
/// `cert-trust-warning` either way (success or failure) so the frontend can toast it — a
/// failure here does not fail `start_profile` itself; the profile stays `Running`.
async fn apply_ca_cert_trust(app: &AppHandle, state: &AppState, name: &str) {
    let config = {
        let guard = state.0.lock().await;
        guard.ca_cert_configs.get(name).cloned().unwrap_or_default()
    };
    if config.mode == crate::state::CaCertMode::None {
        return;
    }

    let event = match trust_ca_cert(app, name, &config).await {
        Ok(()) => CertTrustWarningEvent {
            profile_name: name.to_string(),
            message: "Trusted the host's CA bundle inside the VM.".to_string(),
            severity: "success",
        },
        Err(err) => CertTrustWarningEvent {
            profile_name: name.to_string(),
            message: format!(
                "Failed to trust the host CA bundle inside the VM - docker pulls/builds may fail with TLS errors: {}",
                err.message
            ),
            severity: "error",
        },
    };
    let _ = app.emit("cert-trust-warning", event);
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_profiles(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ColimaProfile>, IpcError> {
    let guard = state.0.lock().await;
    Ok(guard.profiles.clone())
}

/// Checked on demand right before the profile form lets the user flip Kubernetes on —
/// unlike `colima`/`docker`, `kubectl` is only needed once a profile actually enables it,
/// so it is never part of the app-wide startup pre-flight check.
#[tauri::command]
pub fn check_kubectl_available() -> bool {
    preflight::is_kubectl_available()
}

#[tauri::command]
pub async fn create_profile(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    mut input: CreateProfileInput,
) -> Result<ColimaProfile, IpcError> {
    input.name = normalize_profile_name(&input.name);
    validate_profile_name(&input.name)?;
    {
        let guard = state.0.lock().await;
        ensure_name_available(&guard.profiles, &input.name)?;
    }

    let config = config_args_for_create(&input);
    ColimaCli::apply_config(&app, &input.name, &config, input.keep_active).await?;

    if let Some(ca_cert) = input.ca_cert.clone() {
        persist_ca_cert_config(state.inner(), &input.name, ca_cert).await?;
    }

    let root = colima_root()?;
    reconcile_and_emit(&app, &root, state.inner()).await;

    let guard = state.0.lock().await;
    find_profile(&guard.profiles, &input.name).cloned()
}

#[tauri::command]
pub async fn update_profile(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: UpdateProfileInput,
) -> Result<ColimaProfile, IpcError> {
    let config = {
        let guard = state.0.lock().await;
        let existing = ensure_stopped(&guard.profiles, &input.name)?;
        config_args_for_update(existing, &input)
    };

    ColimaCli::apply_config(&app, &input.name, &config, false).await?;

    if let Some(ca_cert) = input.ca_cert.clone() {
        persist_ca_cert_config(state.inner(), &input.name, ca_cert).await?;
    }

    let root = colima_root()?;
    reconcile_and_emit(&app, &root, state.inner()).await;

    let guard = state.0.lock().await;
    find_profile(&guard.profiles, &input.name).cloned()
}

#[tauri::command]
pub async fn start_profile(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    name: String,
) -> Result<(), IpcError> {
    {
        let guard = state.0.lock().await;
        find_profile(&guard.profiles, &name)?;
    }

    transition_and_emit(&app, state.inner(), &name, ProfileState::Starting, None).await;

    match ColimaCli::start(&app, &name).await {
        Ok(()) => {
            let root = colima_root()?;
            reconcile_and_emit(&app, &root, state.inner()).await;
            apply_ca_cert_trust(&app, state.inner(), &name).await;
            Ok(())
        }
        Err(err) => {
            let reason = err.detail.clone().unwrap_or_else(|| err.message.clone());
            transition_and_emit(
                &app,
                state.inner(),
                &name,
                ProfileState::Error,
                Some(reason.clone()),
            )
            .await;
            Err(IpcError::with_detail(
                "COLIMA_START_FAILED",
                format!("Failed to start profile '{name}'"),
                reason,
            ))
        }
    }
}

#[tauri::command]
pub async fn stop_profile(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    name: String,
) -> Result<(), IpcError> {
    {
        let guard = state.0.lock().await;
        find_profile(&guard.profiles, &name)?;
    }

    transition_and_emit(&app, state.inner(), &name, ProfileState::Stopping, None).await;

    match ColimaCli::stop(&app, &name).await {
        Ok(()) => {
            let root = colima_root()?;
            reconcile_and_emit(&app, &root, state.inner()).await;
            Ok(())
        }
        Err(err) => {
            let reason = err.detail.clone().unwrap_or_else(|| err.message.clone());
            transition_and_emit(
                &app,
                state.inner(),
                &name,
                ProfileState::Error,
                Some(reason.clone()),
            )
            .await;
            Err(IpcError::with_detail(
                "COLIMA_STOP_FAILED",
                format!("Failed to stop profile '{name}'"),
                reason,
            ))
        }
    }
}

#[tauri::command]
pub async fn delete_profile(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    name: String,
    confirmed: bool,
    wipe_assets: bool,
) -> Result<(), IpcError> {
    let (socket_path, is_active) = {
        let guard = state.0.lock().await;
        let profile = find_profile(&guard.profiles, &name)?;
        (profile.docker_socket_path.clone(), profile.is_active)
    };

    if !confirmed {
        let running = match &socket_path {
            Some(path) => has_running_containers(path, &name).await,
            None => false,
        };
        if requires_delete_confirmation(is_active, running) {
            return Err(IpcError::new(
                "CONFIRMATION_REQUIRED",
                format!("Deleting profile '{name}' requires confirmation: it is active or has running containers"),
            ));
        }
    }

    ColimaCli::delete(&app, &name, wipe_assets).await?;

    {
        let mut guard = state.0.lock().await;
        if guard.active_profile_name.as_deref() == Some(name.as_str()) {
            guard.active_profile_name = None;
        }
        if guard.ca_cert_configs.remove(&name).is_some() {
            let configs = guard.ca_cert_configs.clone();
            let path = guard.ca_cert_configs_path.clone();
            // Best-effort cleanup: the delete itself already succeeded, so a stale sidecar
            // entry is logged rather than surfaced as a failed delete.
            if let Err(e) = CaCertConfig::save_all(&configs, &path) {
                tracing::warn!(name, error = %e, "failed to persist CA cert config cleanup after profile delete");
            }
        }
    }

    let root = colima_root()?;
    reconcile_and_emit(&app, &root, state.inner()).await;
    Ok(())
}

#[tauri::command]
pub async fn set_active_profile(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    name: String,
) -> Result<ColimaProfile, IpcError> {
    {
        let mut guard = state.0.lock().await;
        find_profile(&guard.profiles, &name)?;
        guard.active_profile_name = Some(name.clone());
        for profile in guard.profiles.iter_mut() {
            profile.is_active = profile.name == name;
        }
        let snapshot = guard.profiles.clone();
        drop(guard);
        let _ = tauri::Emitter::emit(&app, "profiles-changed", snapshot);
    }

    let guard = state.0.lock().await;
    find_profile(&guard.profiles, &name).cloned()
}
