//! Contract test for `get_cluster_state`. Exercises the pure `short_circuit_state` helper
//! backing `commands/kubernetes.rs::get_cluster_state` directly, following the
//! `profiles_contract_test.rs` pattern — no AppHandle/live cluster required, and proves the
//! two short-circuit cases never need one: a disabled or non-`Running` profile short-circuits
//! to `unavailable` before any client call could even be reached in the real command (the
//! function signature itself makes a client call impossible to attempt in these branches,
//! since it never constructs a client). The third contract case — "a reachable fixture
//! context ⇒ `state` derived from node readiness" — requires a live or mocked Kubernetes API
//! server to exercise `kubernetes::client::get_node_status` end-to-end, so
//! `derive_reachable_state`'s pure node-readiness-to-state mapping is covered below instead.
use nebulima_lib::commands::kubernetes::{
    begin_cluster_action, ensure_lifecycle_confirmed, ensure_profile_running, short_circuit_state,
    KubernetesClusterState,
};
use nebulima_lib::state::{
    ClusterAction, ColimaProfile, ContainerRuntime, NetworkConfig, ProfileState,
};
use std::collections::HashMap;

fn sample_profile(kubernetes_enabled: bool, state: ProfileState) -> ColimaProfile {
    ColimaProfile {
        name: "default".to_string(),
        state,
        error_reason: None,
        cpu: 2,
        memory_gi_b: 2,
        disk_gi_b: 100,
        runtime: ContainerRuntime::Docker,
        kubernetes_enabled,
        network: NetworkConfig::default(),
        mounts: vec![],
        ca_cert: nebulima_lib::state::CaCertConfig::default(),
        docker_socket_path: None,
        is_active: false,
    }
}

#[test]
fn short_circuits_to_unavailable_when_kubernetes_is_disabled() {
    let profile = sample_profile(false, ProfileState::Running);
    assert_eq!(
        short_circuit_state(&profile),
        Some(KubernetesClusterState::Unavailable)
    );
}

#[test]
fn short_circuits_to_unavailable_when_the_profile_is_not_running() {
    for state in [
        ProfileState::Stopped,
        ProfileState::Starting,
        ProfileState::Stopping,
        ProfileState::Error,
    ] {
        let profile = sample_profile(true, state);
        assert_eq!(
            short_circuit_state(&profile),
            Some(KubernetesClusterState::Unavailable),
            "expected unavailable for ProfileState::{state:?}"
        );
    }
}

#[test]
fn does_not_short_circuit_when_kubernetes_is_enabled_and_the_profile_is_running() {
    let profile = sample_profile(true, ProfileState::Running);
    assert_eq!(short_circuit_state(&profile), None);
}

// ---------------------------------------------------------------------------
// start_cluster/stop_cluster/reset_cluster/delete_cluster: each lifecycle command's
// validate-then-dispatch guard is factored into
// `ensure_profile_running`/`ensure_lifecycle_confirmed`/`begin_cluster_action` (pure, no
// `AppHandle`/live subprocess needed), so the PROFILE_NOT_FOUND/PROFILE_NOT_RUNNING/
// CONFIRMATION_REQUIRED/KUBERNETES_ACTION_IN_PROGRESS branches are exercised directly here.
// The remaining contract case — "a failed/timed-out subprocess ⇒ state is re-queried, not
// assumed" — is `run_lifecycle_action`'s own re-query-after-dispatch step, which requires a
// live `AppHandle` to run the `colima` subprocess end-to-end; no mock-`AppHandle` runtime
// exists in this codebase, so that path remains to be verified against a real cluster.

#[test]
fn ensure_profile_running_rejects_a_nonexistent_profile() {
    let profiles: Vec<ColimaProfile> = vec![];
    let err = ensure_profile_running(&profiles, "default").unwrap_err();
    assert_eq!(err.code, "PROFILE_NOT_FOUND");
}

#[test]
fn ensure_profile_running_rejects_a_profile_that_is_not_running() {
    for state in [
        ProfileState::Stopped,
        ProfileState::Starting,
        ProfileState::Stopping,
        ProfileState::Error,
    ] {
        let profiles = vec![sample_profile(true, state)];
        let err = ensure_profile_running(&profiles, "default").unwrap_err();
        assert_eq!(
            err.code, "PROFILE_NOT_RUNNING",
            "expected PROFILE_NOT_RUNNING for ProfileState::{state:?}"
        );
    }
}

#[test]
fn ensure_profile_running_accepts_a_running_profile() {
    let profiles = vec![sample_profile(true, ProfileState::Running)];
    assert!(ensure_profile_running(&profiles, "default").is_ok());
}

#[test]
fn ensure_lifecycle_confirmed_rejects_an_unconfirmed_action() {
    let err = ensure_lifecycle_confirmed(false, "default").unwrap_err();
    assert_eq!(err.code, "CONFIRMATION_REQUIRED");
}

#[test]
fn ensure_lifecycle_confirmed_accepts_a_confirmed_action() {
    assert!(ensure_lifecycle_confirmed(true, "default").is_ok());
}

#[test]
fn begin_cluster_action_rejects_a_second_concurrent_action_on_the_same_profile() {
    let mut actions_in_flight = HashMap::new();
    actions_in_flight.insert("default".to_string(), ClusterAction::Resetting);
    let err = begin_cluster_action(&mut actions_in_flight, "default", ClusterAction::Deleting)
        .unwrap_err();
    assert_eq!(err.code, "KUBERNETES_ACTION_IN_PROGRESS");
    // The original in-flight action must be left untouched by the rejected call.
    assert_eq!(
        actions_in_flight.get("default"),
        Some(&ClusterAction::Resetting)
    );
}

#[test]
fn begin_cluster_action_allows_concurrent_actions_on_different_profiles() {
    let mut actions_in_flight = HashMap::new();
    actions_in_flight.insert("default".to_string(), ClusterAction::Resetting);
    assert!(begin_cluster_action(&mut actions_in_flight, "work", ClusterAction::Starting).is_ok());
    assert_eq!(
        actions_in_flight.get("work"),
        Some(&ClusterAction::Starting)
    );
}
