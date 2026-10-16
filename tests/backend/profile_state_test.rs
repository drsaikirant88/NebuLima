//! Lifecycle state-machine test:
//!
//!   stopped --start--> starting --(colima up + socket responds)--> running
//!   running --stop--> stopping --(colima down)--> stopped
//!   starting|stopping --(action fails)--> error
//!   running --(socket unresponsive)--> error
//!   error --(retry succeeds)--> stopped|running
//!
//! Exercises `set_profile_state` (the commands' optimistic transition) and
//! `apply_live_status` (the discovery-driven reconciliation) together, since real usage
//! always combines both: a command sets `starting`/`stopping` immediately, then the next
//! `colima list --json` reconciliation confirms `running`/`stopped` or reports `error`.
use nebulima_lib::colima::cli::ColimaListEntry;
use nebulima_lib::colima::discovery::{apply_live_status, set_profile_state};
use nebulima_lib::state::{ColimaProfile, ContainerRuntime, NetworkConfig, ProfileState};
use std::collections::HashMap;

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

fn live_entry(status: &str) -> HashMap<String, ColimaListEntry> {
    let mut map = HashMap::new();
    map.insert(
        "work".to_string(),
        ColimaListEntry {
            name: "work".to_string(),
            status: status.to_string(),
            address: None,
        },
    );
    map
}

#[test]
fn stopped_to_starting_to_running() {
    let mut profiles = vec![sample_profile("work", ProfileState::Stopped)];

    set_profile_state(&mut profiles, "work", ProfileState::Starting, None);
    assert_eq!(profiles[0].state, ProfileState::Starting);

    apply_live_status(&mut profiles, &live_entry("Running"));
    assert_eq!(profiles[0].state, ProfileState::Running);
    assert!(profiles[0].error_reason.is_none());
}

#[test]
fn running_to_stopping_to_stopped() {
    let mut profiles = vec![sample_profile("work", ProfileState::Running)];

    set_profile_state(&mut profiles, "work", ProfileState::Stopping, None);
    assert_eq!(profiles[0].state, ProfileState::Stopping);

    apply_live_status(&mut profiles, &live_entry("Stopped"));
    assert_eq!(profiles[0].state, ProfileState::Stopped);
}

#[test]
fn starting_action_failure_transitions_to_error_with_reason() {
    let mut profiles = vec![sample_profile("work", ProfileState::Stopped)];

    set_profile_state(&mut profiles, "work", ProfileState::Starting, None);
    set_profile_state(
        &mut profiles,
        "work",
        ProfileState::Error,
        Some("colima start failed: insufficient disk".to_string()),
    );

    assert_eq!(profiles[0].state, ProfileState::Error);
    assert_eq!(
        profiles[0].error_reason,
        Some("colima start failed: insufficient disk".to_string())
    );
}

#[test]
fn stopping_action_failure_transitions_to_error_with_reason() {
    let mut profiles = vec![sample_profile("work", ProfileState::Running)];

    set_profile_state(&mut profiles, "work", ProfileState::Stopping, None);
    set_profile_state(
        &mut profiles,
        "work",
        ProfileState::Error,
        Some("colima stop failed".to_string()),
    );

    assert_eq!(profiles[0].state, ProfileState::Error);
    assert_eq!(
        profiles[0].error_reason,
        Some("colima stop failed".to_string())
    );
}

#[test]
fn running_socket_unresponsive_is_reported_as_error_by_live_status() {
    let mut profiles = vec![sample_profile("work", ProfileState::Running)];

    apply_live_status(&mut profiles, &live_entry("Broken"));

    assert_eq!(profiles[0].state, ProfileState::Error);
    assert!(profiles[0].error_reason.is_some());
}

#[test]
fn error_recovers_to_stopped_on_successful_retry() {
    let mut profiles = vec![sample_profile("work", ProfileState::Error)];
    profiles[0].error_reason = Some("stale failure".to_string());

    set_profile_state(&mut profiles, "work", ProfileState::Stopping, None);
    assert!(profiles[0].error_reason.is_none());

    apply_live_status(&mut profiles, &live_entry("Stopped"));
    assert_eq!(profiles[0].state, ProfileState::Stopped);
    assert!(profiles[0].error_reason.is_none());
}

#[test]
fn error_recovers_to_running_on_successful_retry() {
    let mut profiles = vec![sample_profile("work", ProfileState::Error)];
    profiles[0].error_reason = Some("stale failure".to_string());

    set_profile_state(&mut profiles, "work", ProfileState::Starting, None);
    assert!(profiles[0].error_reason.is_none());

    apply_live_status(&mut profiles, &live_entry("Running"));
    assert_eq!(profiles[0].state, ProfileState::Running);
    assert!(profiles[0].error_reason.is_none());
}
