//! The endpoint socket: the bind with the ownership rules.
//!
//! The channel server listens on one Unix socket at the endpoint path
//! ([`crate::mcp::path`]). The path lives in a runtime directory, so the
//! bind enforces the same-user gate (design decision D5): the directory is
//! created with mode 0700 and refused when another user owns it, and the
//! socket is created with mode 0600. A socket file left by a crashed
//! server refuses connections, so the bind replaces it; a socket file that
//! accepts a connection has a live owner, and this server does not take
//! over. Every refusal is a [`BindError`] the caller logs to stderr, and
//! the server then runs with no endpoint.

use std::fs;
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

/// The endpoint directory mode: only the owner may enter.
const DIRECTORY_MODE: u32 = 0o700;

/// The endpoint socket mode: only the owner may connect.
const SOCKET_MODE: u32 = 0o600;

/// Why the endpoint bind failed, as the caller logs it to stderr.
#[derive(Debug)]
pub enum BindError {
    /// The endpoint directory exists and another user owns it.
    ForeignDirectory {
        /// The refused directory.
        path: PathBuf,
        /// The uid that owns it.
        owner: u32,
    },
    /// A live server already listens on the endpoint socket.
    LiveOwner {
        /// The socket with the live owner.
        path: PathBuf,
    },
    /// Any other filesystem or socket failure.
    Io(io::Error),
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignDirectory { path, owner } => write!(
                f,
                "endpoint directory {} is owned by uid {owner}, not by the current user",
                path.display(),
            ),
            Self::LiveOwner { path } => {
                write!(f, "another live server listens on {}", path.display())
            }
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for BindError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::ForeignDirectory { .. } | Self::LiveOwner { .. } => None,
        }
    }
}

/// Binds the endpoint socket at `path` for the user `uid`.
///
/// `uid` is the expected owner of the endpoint directory — the current
/// user's id in production, an injected value in tests. The directory is
/// created with mode 0700 when missing and refused when another uid owns
/// it; a socket file that refuses a connection is replaced, and one that
/// accepts a connection is kept.
///
/// # Errors
/// A [`BindError`] naming the refusal: a foreign directory, a live owner,
/// or an underlying filesystem or socket failure.
pub fn bind(path: &Path, uid: u32) -> Result<UnixListener, BindError> {
    let dir = path.parent().ok_or_else(|| {
        BindError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the endpoint path {} has no parent directory",
                path.display()
            ),
        ))
    })?;
    prepare_directory(dir, uid)?;
    bind_socket(path)
}

/// Creates or adopts the endpoint directory, 0700 and owned by `uid`.
fn prepare_directory(dir: &Path, uid: u32) -> Result<(), BindError> {
    match fs::create_dir(dir) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(BindError::Io(error)),
    }
    let metadata = fs::metadata(dir).map_err(BindError::Io)?;
    if metadata.uid() != uid {
        return Err(BindError::ForeignDirectory {
            path: dir.to_path_buf(),
            owner: metadata.uid(),
        });
    }
    // `create_dir` honors the umask, so the mode is set explicitly either way.
    fs::set_permissions(dir, fs::Permissions::from_mode(DIRECTORY_MODE)).map_err(BindError::Io)
}

/// Binds the socket at `path`, replacing a stale file and mode 0600.
fn bind_socket(path: &Path) -> Result<UnixListener, BindError> {
    if path.symlink_metadata().is_ok() {
        // A connect that succeeds has a live owner; a refused connect is a
        // stale file from a crashed server, and the bind replaces it.
        if UnixStream::connect(path).is_ok() {
            return Err(BindError::LiveOwner {
                path: path.to_path_buf(),
            });
        }
        fs::remove_file(path).map_err(BindError::Io)?;
    }
    let listener = UnixListener::bind(path).map_err(BindError::Io)?;
    fs::set_permissions(path, fs::Permissions::from_mode(SOCKET_MODE)).map_err(BindError::Io)?;
    Ok(listener)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// The uid that owns everything the test user creates, for the
    /// owner check; tests inject a different one to force a refusal.
    fn own_uid() -> u32 {
        // SAFETY: `geteuid` has no failure mode and no preconditions.
        unsafe { libc::geteuid() }
    }

    fn endpoint_in(dir: &Path) -> PathBuf {
        dir.join("79301cf9592f327a-12.sock")
    }

    fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    // Row 3.1: the directory 0700 and the socket 0600.

    #[test]
    fn the_bind_creates_the_directory_0700_and_the_socket_0600() {
        let tmp = tempdir().unwrap();
        let path = endpoint_in(tmp.path());
        let listener = bind(&path, own_uid()).unwrap();
        assert_eq!(mode_of(tmp.path()), 0o700);
        assert_eq!(mode_of(&path), 0o600);
        // The bound endpoint accepts a connection.
        UnixStream::connect(&path).unwrap();
        drop(listener);
    }

    // Row 3.1: a stale file is replaced.

    #[test]
    fn a_stale_socket_file_is_replaced() {
        let tmp = tempdir().unwrap();
        let path = endpoint_in(tmp.path());
        drop(UnixListener::bind(&path).unwrap());
        // The file is still there, but refuses a connection.
        assert!(path.symlink_metadata().is_ok());
        assert!(UnixStream::connect(&path).is_err());

        let listener = bind(&path, own_uid()).unwrap();
        assert_eq!(mode_of(&path), 0o600);
        UnixStream::connect(&path).unwrap();
        drop(listener);
    }

    // Row 3.1: a live owner is kept.

    #[test]
    fn a_live_owner_is_kept() {
        let tmp = tempdir().unwrap();
        let path = endpoint_in(tmp.path());
        let owner = bind(&path, own_uid()).unwrap();

        let refused = bind(&path, own_uid()).unwrap_err();
        assert!(matches!(refused, BindError::LiveOwner { .. }));
        // The live owner still listens after the refused second bind.
        UnixStream::connect(&path).unwrap();
        drop(owner);
    }

    // Row 3.1: a foreign directory is refused, through the injected uid.

    #[test]
    fn a_foreign_directory_is_refused() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("foreign");
        fs::create_dir(&dir).unwrap();
        let path = dir.join("79301cf9592f327a-12.sock");

        let refused = bind(&path, own_uid() + 1).unwrap_err();
        let BindError::ForeignDirectory {
            path: refused_dir,
            owner,
        } = refused
        else {
            panic!("expected a foreign-directory refusal, got {refused:?}");
        };
        assert_eq!(refused_dir, dir);
        assert_eq!(owner, own_uid());
        // Nothing was created in the refused directory.
        assert!(path.symlink_metadata().is_err());
    }

    #[test]
    fn a_missing_socket_file_binds_without_a_stale_check() {
        let tmp = tempdir().unwrap();
        let path = endpoint_in(tmp.path());
        assert!(path.symlink_metadata().is_err());
        let listener = bind(&path, own_uid()).unwrap();
        UnixStream::connect(&path).unwrap();
        drop(listener);
    }
}
