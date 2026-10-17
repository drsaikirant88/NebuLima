//! Socket-path validator gating every Docker-engine connection. All checks are purely
//! lexical (path-component based) so a request is rejected *before* any filesystem or
//! socket I/O is attempted.
use crate::IpcError;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

/// Resolves the current user's home directory from `$HOME` — sufficient for this app's
/// supported desktop platforms (macOS, Linux) without pulling in an extra dependency.
pub fn home_dir() -> Result<PathBuf, IpcError> {
    std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        IpcError::new(
            "HOME_DIR_UNRESOLVED",
            "Could not resolve the current user's home directory ($HOME is unset)",
        )
    })
}

fn reject(path_str: &str) -> IpcError {
    IpcError::with_detail(
        "INVALID_SOCKET_PATH",
        "Docker socket path must match ~/.colima/<profile>/docker.sock",
        path_str,
    )
}

/// Validates that `path_str` lexically matches `<home>/.colima/<profile>/docker.sock` —
/// absolute, no `..` traversal, exactly one profile segment, filename `docker.sock` — and
/// returns the extracted profile name on success.
pub fn validate_colima_socket_str(path_str: &str) -> Result<String, IpcError> {
    if path_str.is_empty() {
        return Err(reject(path_str));
    }

    let path = Path::new(path_str);

    if !path.is_absolute() {
        return Err(reject(path_str));
    }
    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(reject(path_str));
    }

    let home = home_dir()?;
    let colima_root = home.join(".colima");

    let remainder = path
        .strip_prefix(&colima_root)
        .map_err(|_| reject(path_str))?;
    let mut components = remainder.components();

    let profile_name = match components.next() {
        Some(Component::Normal(segment)) => segment.to_string_lossy().into_owned(),
        _ => return Err(reject(path_str)),
    };
    if profile_name.is_empty() {
        return Err(reject(path_str));
    }

    let socket_segment = components.next();
    if socket_segment != Some(Component::Normal(OsStr::new("docker.sock"))) {
        return Err(reject(path_str));
    }

    // No trailing segments permitted beyond `<profile>/docker.sock`.
    if components.next().is_some() {
        return Err(reject(path_str));
    }

    Ok(profile_name)
}

/// Validates `path_str` matches the pattern *and* belongs to `expected_profile`.
pub fn validate_colima_socket_for_profile(
    path_str: &str,
    expected_profile: &str,
) -> Result<(), IpcError> {
    let profile_name = validate_colima_socket_str(path_str)?;
    if profile_name != expected_profile {
        return Err(reject(path_str));
    }
    Ok(())
}

/// Best-effort defense-in-depth performed immediately before opening the socket connection:
/// resolves symlinks and re-confirms the canonicalized path still lives under the real
/// `~/.colima` directory. Falls back to the lexically-validated path if the socket does not
/// exist yet (so this never becomes a precondition for the "reject before I/O" rule above).
pub fn resolve_for_connection(path_str: &str, expected_profile: &str) -> Result<PathBuf, IpcError> {
    validate_colima_socket_for_profile(path_str, expected_profile)?;
    let path = Path::new(path_str);

    match std::fs::canonicalize(path) {
        Ok(canonical) => {
            let home = home_dir()?;
            let colima_root = std::fs::canonicalize(home.join(".colima"))
                .unwrap_or_else(|_| home.join(".colima"));
            if !canonical.starts_with(&colima_root) {
                return Err(reject(path_str));
            }
            Ok(canonical)
        }
        Err(_) => Ok(path.to_path_buf()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        home_dir().expect("home dir resolvable in test environment")
    }

    #[test]
    fn accepts_well_formed_profile_socket() {
        let path = home().join(".colima").join("default").join("docker.sock");
        let result = validate_colima_socket_str(path.to_str().unwrap());
        assert_eq!(result.unwrap(), "default");
    }

    #[test]
    fn rejects_path_traversal() {
        let path = home()
            .join(".colima")
            .join("..")
            .join("etc")
            .join("docker.sock");
        assert!(validate_colima_socket_str(path.to_str().unwrap()).is_err());
    }

    #[test]
    fn rejects_system_docker_socket() {
        assert!(validate_colima_socket_str("/var/run/docker.sock").is_err());
    }

    #[test]
    fn rejects_relative_path() {
        assert!(validate_colima_socket_str("./.colima/default/docker.sock").is_err());
    }

    #[test]
    fn rejects_wrong_filename() {
        let path = home()
            .join(".colima")
            .join("default")
            .join("not-docker.sock");
        assert!(validate_colima_socket_str(path.to_str().unwrap()).is_err());
    }

    #[test]
    fn rejects_missing_profile_segment() {
        let path = home().join(".colima").join("docker.sock");
        assert!(validate_colima_socket_str(path.to_str().unwrap()).is_err());
    }

    #[test]
    fn rejects_extra_trailing_segments() {
        let path = home()
            .join(".colima")
            .join("default")
            .join("docker.sock")
            .join("extra");
        assert!(validate_colima_socket_str(path.to_str().unwrap()).is_err());
    }

    #[test]
    fn for_profile_rejects_mismatched_profile_name() {
        let path = home().join(".colima").join("default").join("docker.sock");
        assert!(validate_colima_socket_for_profile(path.to_str().unwrap(), "other").is_err());
    }
}
