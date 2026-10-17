//! Contract test for image commands. Exercises the pure mapping functions directly — no
//! AppHandle or live Docker daemon is needed to verify this business logic (same
//! convention as containers_contract_test).
use bollard::models::ImageSummary;
use nebulima_lib::commands::session::resolve_target_profiles;
use nebulima_lib::docker::images::{split_repo_tag, to_image};
use nebulima_lib::state::{ColimaProfile, ContainerRuntime, NetworkConfig, ProfileState};
use std::collections::HashSet;

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
        docker_socket_path: Some(format!("/home/user/.colima/{name}/docker.sock")),
        is_active: false,
    }
}

#[test]
fn resolve_target_profiles_with_no_requested_profile_returns_every_running_profile() {
    let profiles = vec![
        sample_profile("work", ProfileState::Running),
        sample_profile("stopped-one", ProfileState::Stopped),
        sample_profile("default", ProfileState::Running),
    ];
    let resolved = resolve_target_profiles(&profiles, None).unwrap();
    let names: Vec<&str> = resolved.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["work", "default"]);
}

#[test]
fn resolve_target_profiles_rejects_an_unknown_requested_profile() {
    let profiles = vec![sample_profile("default", ProfileState::Running)];
    let err = resolve_target_profiles(&profiles, Some("missing")).unwrap_err();
    assert_eq!(err.code, "PROFILE_NOT_FOUND");
}

#[test]
fn resolve_target_profiles_rejects_a_requested_profile_that_is_not_running() {
    let profiles = vec![sample_profile("default", ProfileState::Stopped)];
    let err = resolve_target_profiles(&profiles, Some("default")).unwrap_err();
    assert_eq!(err.code, "PROFILE_STOPPED");
}

fn sample_summary(id: &str, repo_tags: Vec<String>) -> ImageSummary {
    ImageSummary {
        id: id.to_string(),
        repo_tags,
        created: 1_700_000_000,
        size: 123_456_789,
        ..Default::default()
    }
}

#[test]
fn to_image_maps_repository_and_tag_from_the_first_repo_tag() {
    let summary = sample_summary("sha256:abc", vec!["alpine:3.19".to_string()]);
    let image = to_image(summary, &HashSet::new(), "default");

    assert_eq!(image.id, "sha256:abc");
    assert_eq!(image.repository, Some("alpine".to_string()));
    assert_eq!(image.tag, Some("3.19".to_string()));
    assert_eq!(image.size_bytes, 123_456_789);
    assert_eq!(image.created_at, "2023-11-14T22:13:20.000Z");
}

#[test]
fn to_image_reports_null_repository_and_tag_for_untagged_images() {
    let summary = sample_summary("sha256:dangling", vec![]);
    let image = to_image(summary, &HashSet::new(), "default");

    assert_eq!(image.repository, None);
    assert_eq!(image.tag, None);
}

#[test]
fn to_image_reports_null_repository_and_tag_for_the_legacy_none_sentinel() {
    let summary = sample_summary("sha256:legacy", vec!["<none>:<none>".to_string()]);
    let image = to_image(summary, &HashSet::new(), "default");

    assert_eq!(image.repository, None);
    assert_eq!(image.tag, None);
}

#[test]
fn to_image_sets_in_use_when_a_container_references_its_id() {
    let summary = sample_summary("sha256:abc", vec!["alpine:3.19".to_string()]);
    let mut in_use = HashSet::new();
    in_use.insert("sha256:abc".to_string());

    let image = to_image(summary, &in_use, "default");
    assert!(image.in_use);
}

#[test]
fn to_image_sets_in_use_false_when_no_container_references_its_id() {
    let summary = sample_summary("sha256:abc", vec!["alpine:3.19".to_string()]);
    let image = to_image(summary, &HashSet::new(), "default");
    assert!(!image.in_use);
}

#[test]
fn split_repo_tag_handles_a_registry_host_with_a_port() {
    let (repo, tag) = split_repo_tag("registry.local:5000/team/app:v1.2.3");
    assert_eq!(repo, Some("registry.local:5000/team/app".to_string()));
    assert_eq!(tag, Some("v1.2.3".to_string()));
}

#[test]
fn split_repo_tag_handles_a_repository_with_no_tag_separator() {
    let (repo, tag) = split_repo_tag("my-image");
    assert_eq!(repo, Some("my-image".to_string()));
    assert_eq!(tag, None);
}
