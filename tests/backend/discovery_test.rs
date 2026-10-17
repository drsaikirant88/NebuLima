//! Integration test for `colima/discovery.rs`: builds a fake `~/.colima` directory layout
//! under a tempdir and asserts the filesystem scan + live-status reconciliation produce the
//! expected `ColimaProfile` snapshots.
use nebulima_lib::colima::cli::ColimaListEntry;
use nebulima_lib::colima::discovery::{apply_live_status, docker_socket_path, scan_profiles_under};
use nebulima_lib::state::ProfileState;
use std::collections::HashMap;
use std::fs;

fn write_profile(root: &std::path::Path, name: &str, yaml: &str, with_socket: bool) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("colima.yaml"), yaml).unwrap();
    if with_socket {
        fs::write(dir.join("docker.sock"), b"").unwrap();
    }
}

#[test]
fn discovers_multiple_profiles_sorted_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    write_profile(tmp.path(), "zeta", "cpu: 2\nmemory: 2\ndisk: 60\n", false);
    write_profile(tmp.path(), "alpha", "cpu: 4\nmemory: 4\ndisk: 60\n", true);

    let profiles = scan_profiles_under(tmp.path());
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].name, "alpha");
    assert_eq!(profiles[1].name, "zeta");
    assert!(profiles[0].docker_socket_path.is_some());
    assert!(profiles[1].docker_socket_path.is_none());
}

#[test]
fn docker_socket_path_matches_colima_convention() {
    let root = std::path::Path::new("/home/user/.colima");
    let path = docker_socket_path(root, "work");
    assert_eq!(
        path,
        std::path::PathBuf::from("/home/user/.colima/work/docker.sock")
    );
}

#[test]
fn containerd_runtime_is_parsed_from_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    write_profile(
        tmp.path(),
        "k8s",
        "cpu: 2\nmemory: 2\ndisk: 60\nruntime: containerd\n",
        false,
    );

    let profiles = scan_profiles_under(tmp.path());
    assert_eq!(profiles.len(), 1);
    assert_eq!(
        profiles[0].runtime,
        nebulima_lib::state::ContainerRuntime::Containerd
    );
}

#[test]
fn live_status_overrides_default_stopped_state() {
    let tmp = tempfile::tempdir().unwrap();
    write_profile(tmp.path(), "default", "cpu: 2\nmemory: 2\ndisk: 60\n", true);

    let mut profiles = scan_profiles_under(tmp.path());
    assert_eq!(profiles[0].state, ProfileState::Stopped);

    let mut live = HashMap::new();
    live.insert(
        "default".to_string(),
        ColimaListEntry {
            name: "default".to_string(),
            status: "Starting".to_string(),
            address: None,
        },
    );
    apply_live_status(&mut profiles, &live);
    assert_eq!(profiles[0].state, ProfileState::Starting);
}

#[test]
fn profile_absent_from_live_list_keeps_filesystem_default() {
    let tmp = tempfile::tempdir().unwrap();
    write_profile(
        tmp.path(),
        "orphaned",
        "cpu: 2\nmemory: 2\ndisk: 60\n",
        false,
    );

    let mut profiles = scan_profiles_under(tmp.path());
    let live: HashMap<String, ColimaListEntry> = HashMap::new();
    apply_live_status(&mut profiles, &live);
    assert_eq!(profiles[0].state, ProfileState::Stopped);
}
