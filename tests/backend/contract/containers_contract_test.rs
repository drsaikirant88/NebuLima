//! Contract test for container commands. Exercises the pure mapping/calculation functions
//! directly — no AppHandle or live Docker daemon is needed to verify this business logic
//! (same convention as profiles_contract_test).
use bollard::models::{
    ContainerCpuStats, ContainerCpuUsage, ContainerMemoryStats, ContainerStatsResponse,
    ContainerSummary, ContainerSummaryStateEnum, PortSummary, PortSummaryTypeEnum,
};
use nebulima_lib::commands::session::resolve_target_profiles;
use nebulima_lib::docker::containers::{
    compute_latest_stats, to_container, ContainerStatus, PortProtocol,
};
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
fn resolve_target_profiles_with_no_running_profiles_returns_an_empty_list_not_an_error() {
    let profiles = vec![sample_profile("stopped-one", ProfileState::Stopped)];
    let resolved = resolve_target_profiles(&profiles, None).unwrap();
    assert!(resolved.is_empty());
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

#[test]
fn resolve_target_profiles_accepts_a_requested_running_profile() {
    let profiles = vec![sample_profile("default", ProfileState::Running)];
    let resolved = resolve_target_profiles(&profiles, Some("default")).unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].name, "default");
}

fn sample_summary(state: Option<ContainerSummaryStateEnum>) -> ContainerSummary {
    ContainerSummary {
        id: Some("abc123".to_string()),
        names: Some(vec!["/my-container".to_string()]),
        image: Some("alpine:3".to_string()),
        state,
        created: Some(1_000),
        ports: Some(vec![PortSummary {
            ip: None,
            private_port: 80,
            public_port: Some(8080),
            typ: Some(PortSummaryTypeEnum::TCP),
        }]),
        ..Default::default()
    }
}

#[test]
fn to_container_maps_running_state_and_computes_uptime() {
    let summary = sample_summary(Some(ContainerSummaryStateEnum::RUNNING));
    let container = to_container(summary, "default");

    assert_eq!(container.id, "abc123");
    assert_eq!(container.name, "my-container");
    assert_eq!(container.image, "alpine:3");
    assert_eq!(container.status, ContainerStatus::Running);
    assert_eq!(container.profile_name, "default");
    assert!(container.uptime_seconds.is_some());
    assert!(container.uptime_seconds.unwrap() >= 0);
}

#[test]
fn to_container_maps_ports() {
    let summary = sample_summary(Some(ContainerSummaryStateEnum::RUNNING));
    let container = to_container(summary, "default");

    assert_eq!(container.ports.len(), 1);
    assert_eq!(container.ports[0].container_port, 80);
    assert_eq!(container.ports[0].host_port, Some(8080));
    assert_eq!(container.ports[0].protocol, PortProtocol::Tcp);
}

#[test]
fn to_container_reports_no_uptime_when_not_running() {
    let summary = sample_summary(Some(ContainerSummaryStateEnum::EXITED));
    let container = to_container(summary, "default");

    assert_eq!(container.status, ContainerStatus::Exited);
    assert_eq!(container.uptime_seconds, None);
}

#[test]
fn to_container_defaults_unknown_state_to_stopped() {
    let summary = sample_summary(None);
    let container = to_container(summary, "default");

    assert_eq!(container.status, ContainerStatus::Stopped);
}

fn cpu_stats(total_usage: u64, system_usage: u64, online_cpus: u32) -> ContainerCpuStats {
    ContainerCpuStats {
        cpu_usage: Some(ContainerCpuUsage {
            total_usage: Some(total_usage),
            ..Default::default()
        }),
        system_cpu_usage: Some(system_usage),
        online_cpus: Some(online_cpus),
        ..Default::default()
    }
}

#[test]
fn compute_latest_stats_applies_the_standard_docker_cpu_formula() {
    let response = ContainerStatsResponse {
        cpu_stats: Some(cpu_stats(2_000_000_000, 20_000_000_000, 2)),
        precpu_stats: Some(cpu_stats(1_000_000_000, 10_000_000_000, 2)),
        memory_stats: Some(ContainerMemoryStats {
            usage: Some(104_857_600),
            limit: Some(1_073_741_824),
            ..Default::default()
        }),
        ..Default::default()
    };

    let stats = compute_latest_stats(&response, "2026-10-12T00:00:00.000Z".to_string())
        .expect("stats computed");

    // cpu_delta = 1e9, system_delta = 1e10 -> (1e9/1e10) * 2 * 100 = 20.0
    assert!((stats.cpu_percent - 20.0).abs() < 1e-9);
    assert_eq!(stats.memory_bytes, 104_857_600);
    assert_eq!(stats.memory_limit_bytes, 1_073_741_824);
}

#[test]
fn compute_latest_stats_is_zero_when_system_delta_is_zero() {
    let response = ContainerStatsResponse {
        cpu_stats: Some(cpu_stats(2_000_000_000, 10_000_000_000, 2)),
        precpu_stats: Some(cpu_stats(1_000_000_000, 10_000_000_000, 2)),
        memory_stats: Some(ContainerMemoryStats {
            usage: Some(0),
            limit: Some(0),
            ..Default::default()
        }),
        ..Default::default()
    };

    let stats = compute_latest_stats(&response, "2026-10-12T00:00:00.000Z".to_string())
        .expect("stats computed");
    assert_eq!(stats.cpu_percent, 0.0);
}

#[test]
fn compute_latest_stats_returns_none_without_memory_stats() {
    let response = ContainerStatsResponse {
        cpu_stats: Some(cpu_stats(2_000_000_000, 20_000_000_000, 2)),
        precpu_stats: Some(cpu_stats(1_000_000_000, 10_000_000_000, 2)),
        memory_stats: None,
        ..Default::default()
    };

    assert!(compute_latest_stats(&response, "2026-10-12T00:00:00.000Z".to_string()).is_none());
}
