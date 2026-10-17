//! Integration test for volume list/prune/remove, including the attachment delete warning, run
//! against whatever Docker daemon is reachable at `$DOCKER_HOST`, falling back to Colima's
//! default profile socket (`~/.colima/default/docker.sock`) if present, then
//! `/var/run/docker.sock` — your real local daemon, or CI's runner-provided one. The Colima
//! fallback comes before the generic socket because `docker context`'s notion of "current"
//! is unreliable when both Docker Desktop and Colima are installed (whichever started most
//! recently wins), so this resolves the same way the app's own production code expects
//! (`docker/socket.rs`'s `~/.colima/<profile>/docker.sock` shape) rather than racing contexts.
//! Mirrors containers_test.rs/images_test.rs: the fixture container is built fully offline from
//! the builtin `scratch` pseudo-image, and this connects directly to the daemon via bollard
//! rather than through `docker::containers::connect()`'s colima-path validation.
//!
//! `volumes::prune`'s production call sites prune every unattached volume on the daemon, but
//! this test passes it `PRUNE_LABEL`, restricting *this test's* prune call to only the fixture
//! volumes created below — so this is safe to run against a real, shared Docker daemon, not
//! just an isolated sandbox. This file still deliberately has a single `#[tokio::test]`
//! covering the whole list/is_in_use/remove-while-attached/prune/remove-once-detached scenario
//! sequentially, rather than splitting it across several tests that `cargo test` would run
//! concurrently.
use bollard::Docker;
use nebulima_lib::docker::volumes;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;

const IMAGE_NAME: &str = "nebulima-test-volumes-scratch";
const PRUNE_LABEL: &str = "nebulima-test=volumes-lifecycle";

fn socket_path() -> String {
    if let Ok(host) = std::env::var("DOCKER_HOST") {
        return host.trim_start_matches("unix://").to_string();
    }
    let colima_default = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".colima/default/docker.sock"));
    match colima_default {
        Some(path) if path.exists() => path.to_string_lossy().into_owned(),
        _ => "/var/run/docker.sock".to_string(),
    }
}

async fn connect() -> Docker {
    Docker::connect_with_unix(&socket_path(), 10, bollard::API_DEFAULT_VERSION)
        .expect("connects to the Docker daemon")
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

fn docker(args: &[&str]) -> std::process::Output {
    Command::new("docker")
        .args(args)
        .output()
        .expect("docker command runs")
}

fn remove_fixtures_best_effort(container_name: &str, volume_names: &[&str]) {
    let _ = docker(&["rm", "-f", container_name]);
    for name in volume_names {
        let _ = docker(&["volume", "rm", "-f", name]);
    }
}

/// "All profiles" fan-out — two separate `bollard` connections, tagged as two different
/// profile names, both contribute their rows (correctly tagged) to a single `list_many` call.
/// Both connections point at the same Docker daemon (there is only one in this harness, unlike
/// production where every colima profile has its own daemon), so a single fixture volume is
/// visible — and must be tagged — through both connections; this checks `list_many`'s per-target
/// tagging directly rather than assuming any cross-connection isolation this harness can't provide.
#[tokio::test]
async fn list_many_tags_each_connections_rows_with_its_own_profile_name() {
    let volume = "nebulima-test-volume-list-many";
    let _ = docker(&["volume", "rm", "-f", volume]);
    assert!(docker(&["volume", "create", volume]).status.success());

    let targets = vec![
        ("profile-a".to_string(), Arc::new(connect().await)),
        ("profile-b".to_string(), Arc::new(connect().await)),
    ];
    let all = volumes::list_many(&targets).await;

    let rows: Vec<_> = all.iter().filter(|v| v.name == volume).collect();
    assert_eq!(
        rows.len(),
        2,
        "the fixture volume must be fetched once per connection"
    );
    assert!(rows.iter().any(|v| v.profile_name == "profile-a"));
    assert!(rows.iter().any(|v| v.profile_name == "profile-b"));

    let _ = docker(&["volume", "rm", "-f", volume]);
}

#[tokio::test]
async fn volume_list_attachment_prune_and_remove_lifecycle() {
    build_fixture_image();

    let attached_volume = "nebulima-test-volume-attached";
    let orphan_volume = "nebulima-test-volume-orphan";
    let container_name = "nebulima-test-volume-container";
    remove_fixtures_best_effort(container_name, &[attached_volume, orphan_volume]);

    let create_attached = docker(&["volume", "create", "--label", PRUNE_LABEL, attached_volume]);
    assert!(create_attached.status.success());
    let create_orphan = docker(&["volume", "create", "--label", PRUNE_LABEL, orphan_volume]);
    assert!(create_orphan.status.success());

    let create_container = docker(&[
        "create",
        "--name",
        container_name,
        "-v",
        &format!("{attached_volume}:/data"),
        IMAGE_NAME,
    ]);
    assert!(
        create_container.status.success(),
        "docker create must succeed: {}",
        String::from_utf8_lossy(&create_container.stderr)
    );

    let docker_client = connect().await;

    let listed = volumes::list(&docker_client, "default")
        .await
        .expect("list succeeds");
    assert!(listed.iter().any(|v| v.name == attached_volume));
    assert!(listed.iter().any(|v| v.name == orphan_volume));

    assert!(volumes::is_in_use(&docker_client, attached_volume).await);
    assert!(!volumes::is_in_use(&docker_client, orphan_volume).await);

    let remove_while_attached = volumes::remove(&docker_client, attached_volume).await;
    assert!(
        remove_while_attached.is_err(),
        "removing an attached volume must fail without force"
    );
    assert_eq!(remove_while_attached.unwrap_err().code, "DOCKER_API_ERROR");

    let removed_names = volumes::prune(&docker_client, Some(PRUNE_LABEL))
        .await
        .expect("prune succeeds");
    assert!(removed_names.contains(&orphan_volume.to_string()));
    assert!(!removed_names.contains(&attached_volume.to_string()));

    let after_prune = volumes::list(&docker_client, "default")
        .await
        .expect("list succeeds");
    assert!(!after_prune.iter().any(|v| v.name == orphan_volume));
    assert!(after_prune.iter().any(|v| v.name == attached_volume));

    let _ = docker(&["rm", "-f", container_name]);
    volumes::remove(&docker_client, attached_volume)
        .await
        .expect("remove succeeds once the container is gone");

    let final_list = volumes::list(&docker_client, "default")
        .await
        .expect("list succeeds");
    assert!(!final_list.iter().any(|v| v.name == attached_volume));
}
