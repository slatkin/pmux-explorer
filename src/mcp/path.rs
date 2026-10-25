//! The channel endpoint path, derived from the tmux environment.
//!
//! Each pane's channel server listens on one Unix socket whose path both
//! sides derive from the environment, so pmux can find a pane's endpoint
//! without a registry: the directory comes from `XDG_RUNTIME_DIR` (or a
//! per-uid directory under `/tmp`), and the file name pins the tmux server
//! (an FNV-1a hash of the socket path, so two tmux servers stay apart) and
//! the pane. The rule is shared with pmux `report-channel`, and both pin
//! the same test vector.

use std::path::PathBuf;

/// The 64-bit FNV-1a offset basis.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// The 64-bit FNV-1a prime, `0x100000001B3`.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The endpoint directory under `XDG_RUNTIME_DIR`, shared with pmux.
const RUNTIME_SUBDIR: &str = "pmux-channel";

/// The endpoint directory prefix under `/tmp`, followed by `-<uid>`.
const TMP_SUBDIR_PREFIX: &str = "/tmp/pmux-channel-";

/// The tmux environment one endpoint path is derived from.
///
/// One value built once at the call site, instead of four positionals
/// threaded through every caller: swapping two same-typed `Option<&str>`
/// fields compiles and silently loses the endpoint, while a named field
/// moves the mistake to a reviewable line. The values are arguments, not
/// process env, so tests set no process env.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointEnv<'a> {
    /// The `TMUX` value.
    pub tmux: Option<&'a str>,
    /// The `TMUX_PANE` value.
    pub tmux_pane: Option<&'a str>,
    /// The `XDG_RUNTIME_DIR` value.
    pub xdg_runtime_dir: Option<&'a str>,
    /// The current user's id, for the `/tmp` fallback directory.
    pub uid: u32,
}

/// The endpoint socket path for one pane, or `None` without tmux.
///
/// The path is `<dir>/<h>-<n>.sock`: `<dir>` is `$XDG_RUNTIME_DIR/pmux-channel`,
/// or `/tmp/pmux-channel-<uid>` when `XDG_RUNTIME_DIR` is unset or empty;
/// `<h>` is the 16-digit lowercase hex FNV-1a 64-bit hash of the tmux socket
/// path (the first comma field of `TMUX`); `<n>` is `TMUX_PANE` without its
/// `%`. `None` means no endpoint: `TMUX` or `TMUX_PANE` missing or empty, a
/// pane id that is not a number, or an empty tmux socket path.
pub fn endpoint_path(env: EndpointEnv<'_>) -> Option<PathBuf> {
    let socket = env.tmux?.split(',').next()?;
    if socket.is_empty() {
        return None;
    }
    let pane = pane_number(env.tmux_pane?)?;
    let dir = match env.xdg_runtime_dir.filter(|dir| !dir.is_empty()) {
        Some(runtime_dir) => PathBuf::from(runtime_dir).join(RUNTIME_SUBDIR),
        None => PathBuf::from(format!("{TMP_SUBDIR_PREFIX}{}", env.uid)),
    };
    Some(dir.join(format!("{:016x}-{pane}.sock", fnv1a64(socket.as_bytes()))))
}

/// The pane number in `TMUX_PANE`: the value without its `%`.
///
/// The value is a tmux pane id, so it is `%` followed by digits; the `%` is
/// optional, and anything else (empty, or not all digits) is no endpoint.
fn pane_number(tmux_pane: &str) -> Option<String> {
    let digits = tmux_pane.strip_prefix('%').unwrap_or(tmux_pane);
    (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.to_string())
}

/// The FNV-1a 64-bit hash of `bytes`, in the standard form.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTOR_SOCKET: &str = "/tmp/tmux-1000/default,1,0";
    const VECTOR_PANE: &str = "%12";
    const VECTOR_RUNTIME_DIR: &str = "/run/user/1000";
    const VECTOR_FILE_NAME: &str = "79301cf9592f327a-12.sock";

    /// The endpoint env the tests derive paths from: uid 1000, matching
    /// the vector's `/tmp/tmux-1000` socket and `/run/user/1000` dir.
    fn env<'a>(
        tmux: Option<&'a str>,
        tmux_pane: Option<&'a str>,
        xdg_runtime_dir: Option<&'a str>,
    ) -> EndpointEnv<'a> {
        EndpointEnv {
            tmux,
            tmux_pane,
            xdg_runtime_dir,
            uid: 1000,
        }
    }

    // Row 2.1: the shared test vector.

    #[test]
    fn the_shared_test_vector_holds() {
        let path = endpoint_path(env(
            Some(VECTOR_SOCKET),
            Some(VECTOR_PANE),
            Some(VECTOR_RUNTIME_DIR),
        ));
        assert_eq!(
            path.as_deref(),
            Some(
                PathBuf::from(VECTOR_RUNTIME_DIR)
                    .join(RUNTIME_SUBDIR)
                    .join(VECTOR_FILE_NAME)
                    .as_path()
            )
        );
        assert_eq!(
            path.unwrap().to_string_lossy(),
            "/run/user/1000/pmux-channel/79301cf9592f327a-12.sock"
        );
    }

    // Row 2.1: the unset runtime dir.

    #[test]
    fn an_unset_runtime_dir_falls_back_to_the_tmp_directory() {
        let path = endpoint_path(env(Some(VECTOR_SOCKET), Some(VECTOR_PANE), None)).unwrap();
        assert_eq!(
            path.to_string_lossy(),
            "/tmp/pmux-channel-1000/79301cf9592f327a-12.sock"
        );
    }

    #[test]
    fn an_empty_runtime_dir_falls_back_to_the_tmp_directory() {
        let path = endpoint_path(env(Some(VECTOR_SOCKET), Some(VECTOR_PANE), Some(""))).unwrap();
        assert_eq!(
            path.to_string_lossy(),
            "/tmp/pmux-channel-1000/79301cf9592f327a-12.sock"
        );
    }

    // Row 2.1: a missing TMUX or TMUX_PANE gives no path.

    #[test]
    fn a_missing_tmux_gives_no_path() {
        assert_eq!(
            endpoint_path(env(None, Some(VECTOR_PANE), Some(VECTOR_RUNTIME_DIR))),
            None
        );
    }

    #[test]
    fn an_empty_tmux_gives_no_path() {
        assert_eq!(
            endpoint_path(env(Some(""), Some(VECTOR_PANE), Some(VECTOR_RUNTIME_DIR))),
            None
        );
    }

    #[test]
    fn a_missing_tmux_pane_gives_no_path() {
        assert_eq!(
            endpoint_path(env(Some(VECTOR_SOCKET), None, Some(VECTOR_RUNTIME_DIR))),
            None
        );
    }

    #[test]
    fn an_empty_tmux_pane_gives_no_path() {
        assert_eq!(
            endpoint_path(env(Some(VECTOR_SOCKET), Some(""), Some(VECTOR_RUNTIME_DIR))),
            None
        );
    }

    #[test]
    fn a_non_numeric_pane_id_gives_no_path() {
        assert_eq!(
            endpoint_path(env(
                Some(VECTOR_SOCKET),
                Some("%x"),
                Some(VECTOR_RUNTIME_DIR)
            )),
            None
        );
    }

    #[test]
    fn the_hash_uses_only_the_first_comma_field() {
        let with_tail = endpoint_path(env(
            Some(VECTOR_SOCKET),
            Some(VECTOR_PANE),
            Some(VECTOR_RUNTIME_DIR),
        ))
        .unwrap();
        let without_tail = endpoint_path(env(
            Some("/tmp/tmux-1000/default"),
            Some(VECTOR_PANE),
            Some(VECTOR_RUNTIME_DIR),
        ))
        .unwrap();
        assert_eq!(with_tail, without_tail);
    }

    #[test]
    fn two_tmux_servers_get_different_file_names() {
        let first = endpoint_path(env(
            Some("/tmp/tmux-1000/default,1,0"),
            Some(VECTOR_PANE),
            Some(VECTOR_RUNTIME_DIR),
        ))
        .unwrap();
        let second = endpoint_path(env(
            Some("/tmp/tmux-1000/other,1,0"),
            Some(VECTOR_PANE),
            Some(VECTOR_RUNTIME_DIR),
        ))
        .unwrap();
        assert_ne!(first, second);
    }
}
