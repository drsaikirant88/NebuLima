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

/// Runs `shell -lc 'printf %s "$PATH"'` and returns its trimmed stdout, or `None` if the
/// shell couldn't be spawned, exited non-zero, or printed nothing. Split out from
/// `fix_path_env` so tests can point it at a fixture script instead of the real login shell.
fn resolve_shell_path(shell: &str) -> Option<String> {
    let output = std::process::Command::new(shell)
        .arg("-lc")
        .arg("printf %s \"$PATH\"")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let resolved = String::from_utf8(output.stdout).ok()?;
    let trimmed = resolved.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Replaces the process's inherited `PATH` with the one reported by the user's login
/// shell. GUI-launched apps (Finder/Dock/Spotlight) are spawned by `launchd` with a
/// bare-minimum `PATH` that never sources `.zprofile`/`.bash_profile`/etc., so
/// Homebrew-installed tools like `colima`/`docker` are invisible to this process even
/// though every terminal on the same machine finds them fine. Terminal-launched runs
/// (`npm run tauri dev`) already inherit a full shell `PATH` and are unaffected either
/// way. Call this before `missing_dependencies` and before any `colima`/`docker` CLI
/// invocation; `tauri-plugin-shell` resolves bare command names against this same
/// process-wide `PATH`, so this one fix covers both.
#[cfg(not(windows))]
pub fn fix_path_env() {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    match resolve_shell_path(&shell) {
        Some(resolved) => env::set_var("PATH", resolved),
        None => tracing::warn!(
            "Could not resolve login shell PATH via {shell}; keeping inherited PATH"
        ),
    }
}

#[cfg(windows)]
pub fn fix_path_env() {}

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
    // every scenario sequentially (same convention `tests/backend/volumes_test.rs` uses for its
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

    // `resolve_shell_path` never touches the process-wide `PATH` env var itself (unlike
    // `fix_path_env`, which isn't unit tested here), so these can run concurrently with
    // every other test in this file.
    fn make_fake_shell(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("fake_shell");
        fs::write(&path, format!("#!/bin/sh\n{body}")).expect("write fixture shell");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("set executable bit");
        path
    }

    #[test]
    fn resolve_shell_path_returns_trimmed_stdout_on_success() {
        let dir = tempdir().expect("tempdir");
        let shell = make_fake_shell(dir.path(), "printf '%s' '/fake/bin:/fake/sbin'\n");
        assert_eq!(
            resolve_shell_path(shell.to_str().expect("utf8 path")),
            Some("/fake/bin:/fake/sbin".to_string())
        );
    }

    #[test]
    fn resolve_shell_path_returns_none_on_nonzero_exit() {
        let dir = tempdir().expect("tempdir");
        let shell = make_fake_shell(dir.path(), "exit 1\n");
        assert_eq!(resolve_shell_path(shell.to_str().expect("utf8 path")), None);
    }

    #[test]
    fn resolve_shell_path_returns_none_on_empty_output() {
        let dir = tempdir().expect("tempdir");
        let shell = make_fake_shell(dir.path(), "");
        assert_eq!(resolve_shell_path(shell.to_str().expect("utf8 path")), None);
    }

    #[test]
    fn resolve_shell_path_returns_none_when_shell_missing() {
        assert_eq!(resolve_shell_path("/nonexistent/shell/binary"), None);
    }
}
