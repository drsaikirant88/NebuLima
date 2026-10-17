//! Unit test for `colima/cli.rs` argument construction on create/update/start/stop/delete
//! profile actions: asserts only fixed, validated flags are ever produced — never a
//! raw/user-controlled shell string.
use nebulima_lib::colima::cli::{
    build_delete_args, build_list_args, build_start_args, build_stop_args, ProfileConfigArgs,
};

fn sample_config() -> ProfileConfigArgs {
    ProfileConfigArgs {
        cpu: 4,
        memory_gi_b: 8,
        disk_gi_b: 100,
        runtime: "docker".to_string(),
        kubernetes_enabled: true,
        dns: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
        network_address: true,
        mounts: vec![
            (
                "/Users/dev/workspace".to_string(),
                "/workspace".to_string(),
                true,
            ),
            (
                "/Users/dev/readonly".to_string(),
                "/readonly".to_string(),
                false,
            ),
        ],
    }
}

#[test]
fn start_args_contain_only_fixed_validated_flags() {
    let args = build_start_args("work", &sample_config());

    assert_eq!(args[0], "start");
    assert_eq!(args[1], "work");
    assert!(args.contains(&"--cpus".to_string()));
    assert!(args.contains(&"4".to_string()));
    assert!(args.contains(&"--memory".to_string()));
    assert!(args.contains(&"8".to_string()));
    assert!(args.contains(&"--disk".to_string()));
    assert!(args.contains(&"100".to_string()));
    assert!(args.contains(&"--runtime".to_string()));
    assert!(args.contains(&"docker".to_string()));
    assert!(args.contains(&"--kubernetes=true".to_string()));
    assert!(args.contains(&"--network-address".to_string()));
    assert!(args.contains(&"--save-config".to_string()));

    let dns_positions: Vec<usize> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| a.as_str() == "--dns")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(dns_positions.len(), 2);
    assert_eq!(args[dns_positions[0] + 1], "1.1.1.1");
    assert_eq!(args[dns_positions[1] + 1], "8.8.8.8");

    assert!(args.contains(&"/Users/dev/workspace:/workspace:w".to_string()));
    assert!(args.contains(&"/Users/dev/readonly:/readonly".to_string()));

    // No argument is a free-form shell string containing shell metacharacters.
    for arg in &args {
        assert!(!arg.contains(';') && !arg.contains('|') && !arg.contains('&'));
    }
}

#[test]
fn stop_args_target_exactly_the_named_profile() {
    assert_eq!(
        build_stop_args("work"),
        vec!["stop".to_string(), "work".to_string()]
    );
}

#[test]
fn delete_args_always_force_noninteractive_delete() {
    assert_eq!(
        build_delete_args("work", false),
        vec![
            "delete".to_string(),
            "work".to_string(),
            "--force".to_string()
        ]
    );
}

#[test]
fn delete_args_wipe_assets_adds_dash_d() {
    assert_eq!(
        build_delete_args("work", true),
        vec![
            "delete".to_string(),
            "work".to_string(),
            "--force".to_string(),
            "-d".to_string()
        ]
    );
}

#[test]
fn list_args_always_request_json() {
    assert_eq!(
        build_list_args(),
        vec!["list".to_string(), "--json".to_string()]
    );
}

#[test]
fn disabled_kubernetes_is_explicit_false_not_omitted() {
    let mut cfg = sample_config();
    cfg.kubernetes_enabled = false;
    let args = build_start_args("work", &cfg);
    assert!(args.contains(&"--kubernetes=false".to_string()));
}
