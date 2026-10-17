//! Unit test for the socket-path validator: asserts it accepts only
//! `~/.colima/<profile>/docker.sock`-shaped paths and rejects everything else,
//! including path traversal. Exercises the public API in `docker::socket`
//! (src-tauri/src/docker/socket.rs).
use nebulima_lib::docker::socket::{
    validate_colima_socket_for_profile, validate_colima_socket_str,
};

fn home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .expect("$HOME resolvable in test environment")
}

#[test]
fn accepts_only_colima_profile_socket_shaped_paths() {
    let good = home().join(".colima").join("work").join("docker.sock");
    assert_eq!(
        validate_colima_socket_str(good.to_str().unwrap()).unwrap(),
        "work"
    );
}

#[test]
fn rejects_path_traversal_attempts() {
    let traversal = home()
        .join(".colima")
        .join("work")
        .join("..")
        .join("..")
        .join("etc")
        .join("passwd");
    assert!(validate_colima_socket_str(traversal.to_str().unwrap()).is_err());
}

#[test]
fn rejects_the_system_docker_socket() {
    assert!(validate_colima_socket_str("/var/run/docker.sock").is_err());
}

#[test]
fn rejects_arbitrary_absolute_paths() {
    assert!(validate_colima_socket_str("/tmp/evil.sock").is_err());
}

#[test]
fn rejects_relative_paths() {
    assert!(validate_colima_socket_str("relative/.colima/work/docker.sock").is_err());
}

#[test]
fn for_profile_enforces_name_match() {
    let path = home().join(".colima").join("work").join("docker.sock");
    assert!(validate_colima_socket_for_profile(path.to_str().unwrap(), "work").is_ok());
    assert!(validate_colima_socket_for_profile(path.to_str().unwrap(), "other").is_err());
}
