//! Asserts NebuLima never shells out to `kubectl`. Fails the build the moment a `kubectl`
//! subprocess invocation is introduced into either Kubernetes module — `client.rs` talks to
//! the API server directly and `cli.rs` only ever shells out to `colima` (same
//! subprocess-sandboxing rule as every other Colima command).
use std::fs;
use std::path::Path;

fn assert_source_has_no_kubectl_literal(path: &str) {
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    assert!(
        !source.contains("kubectl"),
        "{path} must never shell out to kubectl"
    );
}

#[test]
fn kubernetes_client_never_references_kubectl() {
    assert_source_has_no_kubectl_literal("src/kubernetes/client.rs");
}

#[test]
fn kubernetes_cli_never_references_kubectl() {
    assert_source_has_no_kubectl_literal("src/kubernetes/cli.rs");
}
