// NebuLima Native Desktop Application Library
pub mod colima;
pub mod commands;
pub mod diagnostics;
pub mod docker;
pub mod kubernetes;
pub mod preflight;
pub mod state;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Manager;

/// Standard IPC Error model shared across all Tauri commands
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct IpcError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl IpcError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(
        code: impl Into<String>,
        message: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: Some(detail.into()),
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch attempt should surface the already-running window
            // rather than spawning a competing instance.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_diagnostic_log_path,
            commands::profiles::list_profiles,
            commands::profiles::check_kubectl_available,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::start_profile,
            commands::profiles::stop_profile,
            commands::profiles::delete_profile,
            commands::profiles::set_active_profile,
            commands::containers::list_containers,
            commands::containers::start_container,
            commands::containers::stop_container,
            commands::containers::delete_container,
            commands::containers::subscribe_container_logs,
            commands::containers::unsubscribe_container_logs,
            commands::images::list_images,
            commands::images::inspect_image,
            commands::images::delete_image,
            commands::volumes::list_volumes,
            commands::volumes::prune_volumes,
            commands::volumes::delete_volume,
            commands::kubernetes::get_cluster_state,
            commands::kubernetes::start_cluster,
            commands::kubernetes::stop_cluster,
            commands::kubernetes::reset_cluster,
            commands::kubernetes::delete_cluster,
            commands::kubernetes::list_cluster_pods,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Checked at first launch on the end user's machine, not at build or
            // package-install time (see preflight.rs's doc comment for why). The main window
            // stays hidden (tauri.conf.json's `visible: false`) until this passes.
            let missing = preflight::missing_dependencies();
            if !missing.is_empty() {
                use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                let message = format!(
                    "Missing required dependencies on PATH: {}. Install the missing tool(s) and relaunch NebuLima.",
                    missing.join(", ")
                );
                tracing::error!("{message}");
                handle
                    .dialog()
                    .message(message)
                    .title("NebuLima")
                    .kind(MessageDialogKind::Error)
                    .blocking_show();
                std::process::exit(1);
            }

            let settings_path = handle
                .path()
                .app_config_dir()
                .map(|dir| dir.join("settings.json"))
                .expect("app config dir resolvable");

            let ca_cert_configs_path = handle
                .path()
                .app_config_dir()
                .map(|dir| dir.join("ca_cert_configs.json"))
                .expect("app config dir resolvable");

            let (log_dir, log_guard) =
                diagnostics::logging::init(&handle).expect("diagnostic logging initializes");

            let app_state = Arc::new(state::AppState(tokio::sync::Mutex::new(
                state::AppStateInner::new(settings_path, ca_cert_configs_path, log_dir),
            )));
            app_state
                .0
                .try_lock()
                .expect("no other owner yet during setup")
                .log_guard = Some(log_guard);
            app.manage(app_state.clone());

            let colima_root = docker::socket::home_dir()
                .expect("$HOME resolvable")
                .join(".colima");
            let watcher = colima::discovery::start_profile_watcher(
                handle.clone(),
                colima_root.clone(),
                app_state.clone(),
            )
            .expect("filesystem watcher starts");
            app_state
                .0
                .try_lock()
                .expect("no other owner yet during setup")
                .fs_watcher = Some(watcher);

            let discover_handle = handle.clone();
            let discover_state = app_state.clone();
            let discover_root = colima_root.clone();
            tauri::async_runtime::spawn(async move {
                colima::discovery::reconcile_and_emit(&discover_handle, &discover_root, &discover_state).await;
            });

            // The filesystem watch alone can miss profile state changes that don't touch
            // anything under `~/.colima` (e.g. the VM dying outside the app's own
            // start/stop calls), which is why Colima's live status can drift from what the
            // UI shows. A low-frequency poll keeps `colima list --json` as the ultimate
            // source of truth even when nothing fires the watcher.
            let poll_handle = handle.clone();
            let poll_state = app_state.clone();
            let poll_root = colima_root.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    colima::discovery::reconcile_and_emit(&poll_handle, &poll_root, &poll_state).await;
                }
            });

            // Mirrors the profile poll above: catches a Kubernetes cluster state change
            // (e.g. the embedded k3s cluster becomes unreachable) that the profile-level
            // `kubernetes-cluster-changed` emission in `discovery.rs` can't see on its own,
            // since the owning Colima profile may still be `Running`.
            let kubernetes_poll_handle = handle.clone();
            let kubernetes_poll_state = app_state.clone();
            tauri::async_runtime::spawn(async move {
                commands::kubernetes::run_safety_net_poll(
                    kubernetes_poll_handle,
                    kubernetes_poll_state,
                    std::time::Duration::from_secs(5),
                )
                .await;
            });

            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running NebuLima application");
}
