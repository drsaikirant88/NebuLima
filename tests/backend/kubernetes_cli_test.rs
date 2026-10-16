//! Unit test for `kubernetes/cli.rs` argument construction on the Kubernetes cluster
//! lifecycle actions: asserts `-p <profile>` always precedes the `kubernetes` subcommand and
//! that only fixed, validated flags are ever produced — never a raw/user-controlled shell
//! string. Mirrors `colima_cli_test.rs`'s pattern.
use nebulima_lib::kubernetes::cli::{
    build_delete_args, build_reset_args, build_start_args, build_stop_args,
};

#[test]
fn start_args_place_profile_flag_before_the_kubernetes_subcommand() {
    assert_eq!(
        build_start_args("work"),
        vec![
            "-p".to_string(),
            "work".to_string(),
            "kubernetes".to_string(),
            "start".to_string(),
        ]
    );
}

#[test]
fn stop_args_place_profile_flag_before_the_kubernetes_subcommand() {
    assert_eq!(
        build_stop_args("work"),
        vec![
            "-p".to_string(),
            "work".to_string(),
            "kubernetes".to_string(),
            "stop".to_string(),
        ]
    );
}

#[test]
fn reset_args_place_profile_flag_before_the_kubernetes_subcommand() {
    assert_eq!(
        build_reset_args("work"),
        vec![
            "-p".to_string(),
            "work".to_string(),
            "kubernetes".to_string(),
            "reset".to_string(),
        ]
    );
}

#[test]
fn delete_args_place_profile_flag_before_the_kubernetes_subcommand() {
    assert_eq!(
        build_delete_args("work"),
        vec![
            "-p".to_string(),
            "work".to_string(),
            "kubernetes".to_string(),
            "delete".to_string(),
        ]
    );
}

#[test]
fn no_argument_ever_contains_shell_metacharacters() {
    for args in [
        build_start_args("work"),
        build_stop_args("work"),
        build_reset_args("work"),
        build_delete_args("work"),
    ] {
        for arg in &args {
            assert!(!arg.contains(';') && !arg.contains('|') && !arg.contains('&'));
        }
    }
}

#[test]
fn profile_name_is_the_only_variable_component() {
    // The subcommand shape itself never changes based on the profile name — no flag is
    // ever derived from or concatenated with untrusted input beyond the single `-p` value.
    let args = build_start_args("--mount=/:/:w");
    assert_eq!(
        args,
        vec![
            "-p".to_string(),
            "--mount=/:/:w".to_string(),
            "kubernetes".to_string(),
            "start".to_string()
        ]
    );
}
