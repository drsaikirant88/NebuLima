//! Docker-in-Docker integration test for container list/start/stop/remove, run against
//! the disposable `dind` daemon (tests/docker/docker-compose.yml). Docker Hub pulls are
//! TLS-intercepted and fail inside this sandbox's dind daemon, so the fixture container is
//! built fully offline from the builtin `scratch` pseudo-image (recognized by the legacy
//! builder with zero registry access) rather than a pulled base image. This connects directly
//! to the dind socket via bollard — not through `docker::containers::connect()`, which validates
//! a colima-shaped socket path that `DOCKER_HOST` here deliberately isn't.
use bollard::Docker;
use nebulima_lib::docker::containers;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The "aggregated list refreshes in ≤2s" bar, enforced automatically against this
/// `dind`/fixture backend. Generous relative to production (a handful of local Unix-socket
/// calls), so this only catches a genuine regression (e.g. an accidental serial-await loop),
/// not incidental CI slowness.
const AGGREGATION_TIME_BUDGET: Duration = Duration::from_secs(2);

const IMAGE_NAME: &str = "nebulima-test-scratch";

fn socket_path() -> String {
    std::env::var("DOCKER_HOST")
        .unwrap_or_else(|_| "unix:///var/run/docker.sock".to_string())
        .trim_start_matches("unix://")
        .to_string()
}

async fn connect() -> Docker {
    Docker::connect_with_unix(&socket_path(), 10, bollard::API_DEFAULT_VERSION)
        .expect("connects to the dind daemon")
}

fn build_fixture_image() {
    let mut child = Command::new("docker")
        .args(["build", "-t", IMAGE_NAME, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("docker build spawns");
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(b"FROM scratch\nCMD [\"/nonexistent\"]\n")
        .expect("writes Dockerfile to stdin");
    let status = child.wait().expect("docker build runs");
    assert!(
        status.success(),
        "docker build must succeed fully offline from `scratch`"
    );
}

fn create_fixture_container(name: &str) -> String {
    let output = Command::new("docker")
        .args(["create", "--name", name, IMAGE_NAME])
        .output()
        .expect("docker create runs");
    assert!(
        output.status.success(),
        "docker create must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn remove_fixture_container_best_effort(name: &str) {
    let _ = Command::new("docker").args(["rm", "-f", name]).output();
}

#[tokio::test]
async fn list_includes_the_created_fixture_container() {
    build_fixture_image();
    let name = "nebulima-test-list";
    remove_fixture_container_best_effort(name);
    let id = create_fixture_container(name);

    let docker = connect().await;
    let containers = containers::list(&docker, "default")
        .await
        .expect("list succeeds");
    assert!(containers.iter().any(|c| c.id == id));

    remove_fixture_container_best_effort(name);
}

#[tokio::test]
async fn start_on_a_scratch_container_fails_with_a_docker_api_error() {
    build_fixture_image();
    let name = "nebulima-test-start";
    remove_fixture_container_best_effort(name);
    let id = create_fixture_container(name);

    let docker = connect().await;
    let result = containers::start(&docker, &id).await;
    assert!(
        result.is_err(),
        "starting a container whose entrypoint does not exist must fail"
    );
    assert_eq!(result.unwrap_err().code, "DOCKER_API_ERROR");

    remove_fixture_container_best_effort(name);
}

#[tokio::test]
async fn stop_on_a_non_running_container_either_no_ops_or_reports_a_docker_api_error() {
    build_fixture_image();
    let name = "nebulima-test-stop";
    remove_fixture_container_best_effort(name);
    let id = create_fixture_container(name);

    let docker = connect().await;
    if let Err(err) = containers::stop(&docker, &id).await {
        assert_eq!(err.code, "DOCKER_API_ERROR");
    }

    remove_fixture_container_best_effort(name);
}

/// "All profiles" fan-out — two separate `bollard` connections, tagged as two different
/// profile names, both contribute their rows (correctly tagged) to a single `list_many` call.
/// Both connections point at the same `dind` daemon (there is only one in this harness, unlike
/// production where every colima profile has its own daemon), so a single fixture container is
/// visible — and must be tagged — through both connections; this checks `list_many`'s per-target
/// tagging directly rather than assuming any cross-connection isolation this harness can't provide.
#[tokio::test]
async fn list_many_tags_each_connections_rows_with_its_own_profile_name() {
    build_fixture_image();
    let name = "nebulima-test-list-many";
    remove_fixture_container_best_effort(name);
    let id = create_fixture_container(name);

    let targets = vec![
        ("profile-a".to_string(), Arc::new(connect().await)),
        ("profile-b".to_string(), Arc::new(connect().await)),
    ];
    let started = Instant::now();
    let all = containers::list_many(&targets).await;
    assert!(
        started.elapsed() <= AGGREGATION_TIME_BUDGET,
        "list_many must aggregate within {AGGREGATION_TIME_BUDGET:?}"
    );

    let rows: Vec<_> = all.iter().filter(|c| c.id == id).collect();
    assert_eq!(
        rows.len(),
        2,
        "the fixture container must be fetched once per connection"
    );
    assert!(rows.iter().any(|c| c.profile_name == "profile-a"));
    assert!(rows.iter().any(|c| c.profile_name == "profile-b"));

    remove_fixture_container_best_effort(name);
}

#[tokio::test]
async fn remove_deletes_the_container() {
    build_fixture_image();
    let name = "nebulima-test-remove";
    remove_fixture_container_best_effort(name);
    let id = create_fixture_container(name);

    let docker = connect().await;
    containers::remove(&docker, &id)
        .await
        .expect("remove succeeds");

    let remaining = containers::list(&docker, "default")
        .await
        .expect("list succeeds");
    assert!(!remaining.iter().any(|c| c.id == id));
}
