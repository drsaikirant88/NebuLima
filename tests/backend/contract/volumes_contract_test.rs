//! Contract test for volume commands. Exercises the pure mapping function directly — no
//! AppHandle or live Docker daemon is needed to verify this business logic (same
//! convention as images_contract_test).
use bollard::models::{ContainerSummary, MountPoint, Volume as BollardVolume, VolumeUsageData};
use nebulima_lib::commands::session::resolve_target_profiles;
use nebulima_lib::docker::volumes::to_volume;
use nebulima_lib::state::{ColimaProfile, ContainerRuntime, NetworkConfig, ProfileState};

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

fn sample_volume(name: &str, size: i64) -> BollardVolume {
    BollardVolume {
        name: name.to_string(),
        usage_data: Some(VolumeUsageData { size, ref_count: 0 }),
        ..Default::default()
    }
}

fn container_with_mount(
    name: &str,
    mount_type: Option<&str>,
    mount_name: Option<&str>,
) -> ContainerSummary {
    ContainerSummary {
        names: Some(vec![format!("/{name}")]),
        mounts: Some(vec![MountPoint {
            typ: mount_type.map(|t| t.to_string()),
            name: mount_name.map(|n| n.to_string()),
            ..Default::default()
        }]),
        ..Default::default()
    }
}

#[test]
fn to_volume_maps_name_and_size_from_usage_data() {
    let volume = to_volume(sample_volume("data", 42), &[], "default");
    assert_eq!(volume.name, "data");
    assert_eq!(volume.size_bytes, Some(42));
    assert!(volume.attached_container_names.is_empty());
}

#[test]
fn to_volume_reports_null_size_when_usage_data_is_absent() {
    let bollard_volume = BollardVolume {
        name: "data".to_string(),
        ..Default::default()
    };
    let volume = to_volume(bollard_volume, &[], "default");
    assert_eq!(volume.size_bytes, None);
}

#[test]
fn to_volume_lists_containers_that_mount_it_by_name() {
    let containers = vec![container_with_mount("web", Some("volume"), Some("data"))];
    let volume = to_volume(sample_volume("data", 0), &containers, "default");
    assert_eq!(volume.attached_container_names, vec!["web".to_string()]);
}

#[test]
fn to_volume_ignores_containers_mounting_a_different_volume() {
    let containers = vec![container_with_mount("web", Some("volume"), Some("other"))];
    let volume = to_volume(sample_volume("data", 0), &containers, "default");
    assert!(volume.attached_container_names.is_empty());
}

#[test]
fn to_volume_ignores_non_volume_mounts_with_the_same_name() {
    let containers = vec![container_with_mount("web", Some("bind"), Some("data"))];
    let volume = to_volume(sample_volume("data", 0), &containers, "default");
    assert!(volume.attached_container_names.is_empty());
}
