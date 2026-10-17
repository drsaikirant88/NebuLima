//! Contract test for the Profiles IPC surface. Exercises the pure validation helpers
//! backing `commands/profiles.rs` directly, following the `settings_contract_test.rs`
//! pattern — no AppHandle/live colima process required.
use nebulima_lib::commands::profiles::{
    ensure_name_available, ensure_stopped, find_profile, normalize_profile_name,
    requires_delete_confirmation, validate_profile_name, UpdateProfileInput,
};
use nebulima_lib::state::{
    CaCertConfig, CaCertMode, ColimaProfile, ContainerRuntime, NetworkConfig, ProfileState,
};

fn sample_profile(name: &str, state: ProfileState) -> ColimaProfile {
    ColimaProfile {
        name: name.to_string(),
        state,
        error_reason: None,
        cpu: 2,
        memory_gi_b: 2,
        disk_gi_b: 100,
        runtime: ContainerRuntime::Docker,
        kubernetes_enabled: false,
        network: NetworkConfig::default(),
        mounts: vec![],
        ca_cert: nebulima_lib::state::CaCertConfig::default(),
        docker_socket_path: None,
        is_active: false,
    }
}

#[test]
fn ensure_name_available_rejects_existing_name() {
    let profiles = vec![sample_profile("default", ProfileState::Stopped)];
    let err = ensure_name_available(&profiles, "default").unwrap_err();
    assert_eq!(err.code, "PROFILE_NAME_CONFLICT");
}

#[test]
fn ensure_name_available_accepts_new_name() {
    let profiles = vec![sample_profile("default", ProfileState::Stopped)];
    assert!(ensure_name_available(&profiles, "work").is_ok());
}

#[test]
fn find_profile_returns_not_found_for_unknown_name() {
    let profiles = vec![sample_profile("default", ProfileState::Stopped)];
    let err = find_profile(&profiles, "missing").unwrap_err();
    assert_eq!(err.code, "PROFILE_NOT_FOUND");
}

#[test]
fn find_profile_returns_matching_profile() {
    let profiles = vec![sample_profile("default", ProfileState::Stopped)];
    let found = find_profile(&profiles, "default").unwrap();
    assert_eq!(found.name, "default");
}

#[test]
fn ensure_stopped_rejects_running_profile() {
    let profiles = vec![sample_profile("default", ProfileState::Running)];
    let err = ensure_stopped(&profiles, "default").unwrap_err();
    assert_eq!(err.code, "PROFILE_NOT_STOPPED");
}

#[test]
fn ensure_stopped_accepts_stopped_profile() {
    let profiles = vec![sample_profile("default", ProfileState::Stopped)];
    assert!(ensure_stopped(&profiles, "default").is_ok());
}

#[test]
fn requires_delete_confirmation_when_active() {
    assert!(requires_delete_confirmation(true, false));
}

#[test]
fn requires_delete_confirmation_when_running_containers_present() {
    assert!(requires_delete_confirmation(false, true));
}

#[test]
fn does_not_require_delete_confirmation_for_inactive_idle_profile() {
    assert!(!requires_delete_confirmation(false, false));
}

#[test]
fn normalize_profile_name_lowercases_mixed_case_input() {
    assert_eq!(normalize_profile_name("Test"), "test");
    assert_eq!(normalize_profile_name("ALLCAPS"), "allcaps");
    assert_eq!(normalize_profile_name("already-lower"), "already-lower");
}

#[test]
fn validate_profile_name_accepts_ordinary_names() {
    assert!(validate_profile_name("default").is_ok());
    assert!(validate_profile_name("work-2").is_ok());
    assert!(validate_profile_name("k8s_test.local").is_ok());
}

#[test]
fn validate_profile_name_rejects_leading_dash_flag_injection() {
    let err = validate_profile_name("--mount=/:/:w").unwrap_err();
    assert_eq!(err.code, "INVALID_PROFILE_NAME");
}

#[test]
fn validate_profile_name_rejects_empty_and_disallowed_characters() {
    assert!(validate_profile_name("").is_err());
    assert!(validate_profile_name("has space").is_err());
    assert!(validate_profile_name("semi;colon").is_err());
}

#[test]
fn ca_cert_config_default_round_trips_as_none() {
    let config = CaCertConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    assert_eq!(json, r#"{"mode":"none"}"#);
    let parsed: CaCertConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, config);
}

#[test]
fn ca_cert_config_file_mode_round_trips_with_its_path() {
    let config = CaCertConfig {
        mode: CaCertMode::File,
        cert_path: Some("/tmp/corp-ca.pem".to_string()),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: CaCertConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, config);
}

#[test]
fn update_profile_input_without_ca_cert_field_leaves_it_absent() {
    let input: UpdateProfileInput = serde_json::from_str(r#"{"name":"default"}"#).unwrap();
    assert!(input.ca_cert.is_none());
}

#[test]
fn update_profile_input_with_explicit_none_mode_clears_ca_cert() {
    let input: UpdateProfileInput =
        serde_json::from_str(r#"{"name":"default","caCert":{"mode":"none"}}"#).unwrap();
    assert_eq!(input.ca_cert, Some(CaCertConfig::default()));
}

#[test]
fn update_profile_input_with_system_bundle_mode_deserializes() {
    let input: UpdateProfileInput =
        serde_json::from_str(r#"{"name":"default","caCert":{"mode":"systemBundle"}}"#).unwrap();
    assert_eq!(
        input.ca_cert,
        Some(CaCertConfig {
            mode: CaCertMode::SystemBundle,
            cert_path: None
        })
    );
}
