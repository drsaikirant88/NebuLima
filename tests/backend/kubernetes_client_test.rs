//! Unit test for `kubernetes/client.rs::resolve_context_for_profile` — the Colima
//! kubeconfig-context naming convention (`colima` for `default`, `colima-<profile>`
//! otherwise), asserted against a fixture `~/.kube/config`-shaped YAML file rather than a
//! live cluster.
use k8s_openapi::api::core::v1::{Pod, PodStatus};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use nebulima_lib::kubernetes::client::PodPhase;
use nebulima_lib::kubernetes::client::{resolve_context_for_profile, to_cluster_pod};
use std::io::Write;
use tempfile::NamedTempFile;

const FIXTURE_KUBECONFIG: &str = r#"
apiVersion: v1
kind: Config
clusters:
  - name: colima
    cluster:
      server: https://127.0.0.1:6443
  - name: colima-work
    cluster:
      server: https://127.0.0.1:6444
contexts:
  - name: colima
    context:
      cluster: colima
      user: colima
  - name: colima-work
    context:
      cluster: colima-work
      user: colima-work
users:
  - name: colima
    user:
      token: dummy
  - name: colima-work
    user:
      token: dummy
current-context: colima
"#;

fn fixture_kubeconfig() -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("tempfile created");
    file.write_all(FIXTURE_KUBECONFIG.as_bytes())
        .expect("fixture written");
    file
}

#[test]
fn picks_the_bare_colima_context_for_the_default_profile() {
    let file = fixture_kubeconfig();
    assert_eq!(
        resolve_context_for_profile("default", file.path()).unwrap(),
        "colima"
    );
}

#[test]
fn picks_the_colima_dash_profile_context_for_a_named_profile() {
    let file = fixture_kubeconfig();
    assert_eq!(
        resolve_context_for_profile("work", file.path()).unwrap(),
        "colima-work"
    );
}

#[test]
fn returns_a_not_found_ipc_error_rather_than_panicking_when_no_context_matches() {
    let file = fixture_kubeconfig();
    let err = resolve_context_for_profile("missing", file.path()).unwrap_err();
    assert_eq!(err.code, "KUBERNETES_CONTEXT_NOT_FOUND");
}

fn fixture_pod(name: &str, namespace: &str, phase: &str) -> Pod {
    Pod {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        status: Some(PodStatus {
            phase: Some(phase.to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

// `list_pods` itself cannot be exercised without a live Kubernetes API server — this
// instead covers `to_cluster_pod`, the pure mapping function it maps every list item
// through, against constructed fixture `Pod` values (same convention as
// `docker::containers::to_container` etc., covered in tests/backend/contract/*). `list_pods`
// applying this mapping over zero items trivially yields `[]`, which is why no separate
// "empty" case is needed for `list_pods` itself.
#[test]
fn to_cluster_pod_maps_name_namespace_and_a_running_phase() {
    let pod = fixture_pod("web-0", "default", "Running");
    let mapped = to_cluster_pod(&pod);
    assert_eq!(mapped.name, "web-0");
    assert_eq!(mapped.namespace, "default");
    assert_eq!(mapped.phase, PodPhase::Running);
}

#[test]
fn to_cluster_pod_maps_every_known_phase_string() {
    assert_eq!(
        to_cluster_pod(&fixture_pod("a", "default", "Pending")).phase,
        PodPhase::Pending
    );
    assert_eq!(
        to_cluster_pod(&fixture_pod("b", "default", "Succeeded")).phase,
        PodPhase::Succeeded
    );
    assert_eq!(
        to_cluster_pod(&fixture_pod("c", "default", "Failed")).phase,
        PodPhase::Failed
    );
}

#[test]
fn to_cluster_pod_defaults_to_unknown_phase_when_status_is_absent() {
    let pod = Pod {
        metadata: ObjectMeta {
            name: Some("no-status".to_string()),
            namespace: Some("default".to_string()),
            ..Default::default()
        },
        status: None,
        ..Default::default()
    };
    assert_eq!(to_cluster_pod(&pod).phase, PodPhase::Unknown);
}
