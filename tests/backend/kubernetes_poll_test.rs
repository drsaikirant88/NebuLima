//! Test for the Kubernetes safety-net poll's emit-decision helper
//! (`commands/kubernetes.rs::should_emit_cluster_update`). The poll loop itself needs a
//! live `AppHandle`/`Client` to exercise end-to-end, so this proves the pure decision it is
//! built on: emit only when the derived state differs from the last-emitted value for that
//! profile, never when it is unchanged across consecutive polls.
use nebulima_lib::commands::kubernetes::{
    should_emit_cluster_update, KubernetesCluster, KubernetesClusterAction, KubernetesClusterState,
};
use std::collections::HashMap;

fn sample_cluster(profile_name: &str, state: KubernetesClusterState) -> KubernetesCluster {
    KubernetesCluster {
        profile_name: profile_name.to_string(),
        state,
        nodes: vec![],
        action_in_progress: KubernetesClusterAction::None,
        last_error: None,
    }
}

#[test]
fn emits_when_no_state_has_been_recorded_yet() {
    let last_emitted = HashMap::new();
    let current = sample_cluster("default", KubernetesClusterState::Running);
    assert!(should_emit_cluster_update(&last_emitted, &current));
}

#[test]
fn does_not_emit_when_state_is_unchanged_across_consecutive_polls() {
    let mut last_emitted = HashMap::new();
    last_emitted.insert("default".to_string(), KubernetesClusterState::Running);
    let current = sample_cluster("default", KubernetesClusterState::Running);
    assert!(!should_emit_cluster_update(&last_emitted, &current));
}

#[test]
fn emits_when_state_differs_from_the_last_emitted_value() {
    let mut last_emitted = HashMap::new();
    last_emitted.insert("default".to_string(), KubernetesClusterState::Running);
    let current = sample_cluster("default", KubernetesClusterState::Error);
    assert!(should_emit_cluster_update(&last_emitted, &current));
}

#[test]
fn tracks_each_profile_independently() {
    let mut last_emitted = HashMap::new();
    last_emitted.insert("default".to_string(), KubernetesClusterState::Running);
    let current = sample_cluster("work", KubernetesClusterState::Running);
    assert!(should_emit_cluster_update(&last_emitted, &current));
}
