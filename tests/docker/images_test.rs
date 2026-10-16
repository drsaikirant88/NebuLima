//! Docker-in-Docker integration test for image list/inspect/remove, including the in-use
//! delete warning, run against the disposable `dind` daemon (tests/docker/docker-compose.yml).
//! Mirrors containers_test.rs: fixture images are built fully offline from the builtin `scratch`
//! pseudo-image (zero registry access needed), and this connects directly to the dind socket via
//! bollard rather than through `docker::containers::connect()`'s colima-path validation.
//!
//! Each test builds its own uniquely-tagged image with a distinguishing `LABEL` so every test
//! gets a distinct content-addressable image ID. `cargo test` runs `#[tokio::test]`s in this
//! binary concurrently by default, so sharing a single image across tests would let one test's
//! container (referencing that shared image) cause unrelated `remove`/`is_in_use` assertions in
//! another, concurrently-running test to fail.
use bollard::Docker;
use nebulima_lib::docker::images;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;

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

/// Builds a `scratch`-based fixture image tagged `tag`, with a `LABEL` derived from `tag` so each
/// test's image is content-addressably distinct even though they all share the same base/CMD.
fn build_fixture_image(tag: &str) -> String {
    let mut child = Command::new("docker")
        .args(["build", "-t", tag, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("docker build spawns");
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(
            format!("FROM scratch\nLABEL nebulima-test=\"{tag}\"\nCMD [\"/nonexistent\"]\n")
                .as_bytes(),
        )
        .expect("writes Dockerfile to stdin");
    let status = child.wait().expect("docker build runs");
    assert!(
        status.success(),
        "docker build must succeed fully offline from `scratch`"
    );

    let output = Command::new("docker")
        .args(["image", "inspect", "--format", "{{.Id}}", tag])
        .output()
        .expect("docker image inspect runs");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn create_fixture_container(name: &str, image_tag: &str) -> String {
    let output = Command::new("docker")
        .args(["create", "--name", name, image_tag])
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

fn remove_fixture_image_best_effort(tag: &str) {
    let _ = Command::new("docker").args(["rmi", "-f", tag]).output();
}

#[tokio::test]
async fn list_includes_the_built_fixture_image() {
    let tag = "nebulima-test-image-list";
    let image_id = build_fixture_image(tag);

    let docker = connect().await;
    let result = images::list(&docker, "default")
        .await
        .expect("list succeeds");
    assert!(result.iter().any(|i| i.id == image_id));

    remove_fixture_image_best_effort(tag);
}

#[tokio::test]
async fn inspect_returns_the_configured_cmd_with_no_layers() {
    let tag = "nebulima-test-image-inspect";
    let image_id = build_fixture_image(tag);

    let docker = connect().await;
    let detail = images::inspect(&docker, &image_id)
        .await
        .expect("inspect succeeds");

    assert_eq!(detail.cmd, Some(vec!["/nonexistent".to_string()]));
    // The daemon's builder (BuildKit, via the buildx plugin installed in this test image)
    // injects a default PATH into every image's config, even a bare `FROM scratch` one with
    // no explicit ENV instruction. The legacy builder does the same thing.
    assert_eq!(
        detail.env,
        vec!["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()]
    );
    assert!(
        detail.layers.is_empty(),
        "a scratch image with only metadata instructions has no filesystem layers"
    );

    remove_fixture_image_best_effort(tag);
}

#[tokio::test]
async fn is_in_use_reflects_whether_a_container_currently_references_the_image() {
    let tag = "nebulima-test-image-inuse";
    let image_id = build_fixture_image(tag);
    let name = "nebulima-test-image-inuse-container";
    remove_fixture_container_best_effort(name);

    let docker = connect().await;
    assert!(!images::is_in_use(&docker, &image_id).await);

    create_fixture_container(name, tag);
    assert!(images::is_in_use(&docker, &image_id).await);

    remove_fixture_container_best_effort(name);
    assert!(!images::is_in_use(&docker, &image_id).await);

    remove_fixture_image_best_effort(tag);
}

/// "All profiles" fan-out — two separate `bollard` connections, tagged as two different
/// profile names, both contribute their rows (correctly tagged) to a single `list_many` call.
/// Both connections point at the same `dind` daemon (there is only one in this harness, unlike
/// production where every colima profile has its own daemon), so a single fixture image is
/// visible — and must be tagged — through both connections; this checks `list_many`'s per-target
/// tagging directly rather than assuming any cross-connection isolation this harness can't provide.
#[tokio::test]
async fn list_many_tags_each_connections_rows_with_its_own_profile_name() {
    let tag = "nebulima-test-image-list-many";
    let image_id = build_fixture_image(tag);

    let targets = vec![
        ("profile-a".to_string(), Arc::new(connect().await)),
        ("profile-b".to_string(), Arc::new(connect().await)),
    ];
    let all = images::list_many(&targets).await;

    let rows: Vec<_> = all.iter().filter(|i| i.id == image_id).collect();
    assert_eq!(
        rows.len(),
        2,
        "the fixture image must be fetched once per connection"
    );
    assert!(rows.iter().any(|i| i.profile_name == "profile-a"));
    assert!(rows.iter().any(|i| i.profile_name == "profile-b"));

    remove_fixture_image_best_effort(tag);
}

#[tokio::test]
async fn remove_fails_while_in_use_and_succeeds_once_the_container_is_gone() {
    let tag = "nebulima-test-image-remove";
    let image_id = build_fixture_image(tag);
    let name = "nebulima-test-image-remove-container";
    remove_fixture_container_best_effort(name);
    create_fixture_container(name, tag);

    let docker = connect().await;
    let result = images::remove(&docker, &image_id).await;
    assert!(
        result.is_err(),
        "removing an image referenced by a container must fail without force"
    );
    assert_eq!(result.unwrap_err().code, "DOCKER_API_ERROR");

    remove_fixture_container_best_effort(name);
    images::remove(&docker, &image_id)
        .await
        .expect("remove succeeds once unreferenced");

    let remaining = images::list(&docker, "default")
        .await
        .expect("list succeeds");
    assert!(!remaining.iter().any(|i| i.id == image_id));
}
