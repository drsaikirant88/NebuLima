//! Dependency pre-flight check: verifies `colima` and `docker` are present and executable
//! on `PATH`. Runs once at app startup (`lib.rs::run`'s `setup` hook), before the main
//! window is shown. This deliberately runs on the end user's machine at first launch rather
//! than on the build/CI machine or as a packaging-format install hook: `.deb`, `.AppImage`,
//! and `.dmg` have no uniform cross-platform install-time hook to run this against, and
//! gating the build itself on these CLIs would force every build/CI machine to install
//! tools the build never actually uses.
use std::env;
use std::path::Path;

const REQUIRED_COMMANDS: [&str; 2] = ["colima", "docker"];

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn is_on_path(command: &str) -> bool {
    env::var_os("PATH").is_some_and(|path_var| {
        env::split_paths(&path_var).any(|dir| {
            let candidate = dir.join(command);
            candidate.is_file() && is_executable(&candidate)
        })
    })
}

/// Names (checked in order) of every required CLI missing from `PATH`; empty when all are
/// present.
pub fn missing_dependencies() -> Vec<&'static str> {
    REQUIRED_COMMANDS
        .iter()
        .copied()
        .filter(|command| !is_on_path(command))
        .collect()
}

/// `kubectl` is only required when a profile has Kubernetes enabled, so unlike
/// `REQUIRED_COMMANDS` it is never checked at startup — only on demand, right before the
/// user is allowed to flip the Kubernetes toggle on in the profile form.
pub fn is_kubectl_available() -> bool {
    is_on_path("kubectl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    fn make_executable(dir: &Path, name: &str) {
        let path = dir.join(name);
        fs::write(&path, "#!/bin/sh\n").expect("write fixture executable");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("set executable bit");
    }

    fn make_non_executable(dir: &Path, name: &str) {
        let path = dir.join(name);
        fs::write(&path, "not executable").expect("write fixture file");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
            .expect("set non-executable permissions");
    }

    // `missing_dependencies` reads the process-wide `PATH` env var, which `cargo test` would
    // otherwise let multiple concurrently-running tests mutate at once. A single test covering
    // every scenario sequentially (same convention `tests/docker/volumes_test.rs` uses for its
    // own global side effect) avoids that race without pulling in a serial-test crate.
    #[test]
    fn missing_dependencies_reflects_path_contents() {
        let dir = tempdir().expect("tempdir");
        let original_path = env::var_os("PATH");

        env::set_var("PATH", dir.path());
        assert_eq!(missing_dependencies(), vec!["colima", "docker"]);

        make_executable(dir.path(), "colima");
        make_executable(dir.path(), "docker");
        assert!(missing_dependencies().is_empty());

        fs::remove_file(dir.path().join("docker")).expect("remove fixture");
        make_non_executable(dir.path(), "docker");
        assert_eq!(missing_dependencies(), vec!["docker"]);

        assert!(!is_kubectl_available());
        make_executable(dir.path(), "kubectl");
        assert!(is_kubectl_available());

        if let Some(path) = original_path {
            env::set_var("PATH", path);
        } else {
            env::remove_var("PATH");
        }
    }
}
