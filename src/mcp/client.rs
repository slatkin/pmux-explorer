//! The `channel-ready` client side: one arm request to one endpoint.
//!
//! `pmux-explorer channel-ready --nonce <n>` sends the `arm` request the
//! probe asks for: one line over the endpoint socket of its own pane, one
//! reply line back. The server half — the same-user gate, the request
//! shape, the arm state — lives in [`super::connection`]; this module owns
//! the client half: the request line, the answer window, and the two
//! failure classes the CLI turns into exit codes and stderr messages.

use serde_json::json;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// Why an arm request got no usable reply.
#[derive(Debug)]
pub enum ArmError {
    /// No endpoint socket at the path, or nothing listens on it.
    Unreachable(io::Error),
    /// The endpoint did not send a reply line within the answer window.
    NoAnswer(io::Error),
}

/// Sends one `arm` request line to the endpoint at `path` and returns the
/// reply line without its terminator.
///
/// The request is one JSON line `{"v":1,"kind":"arm","nonce":<nonce>}`;
/// the reply is one line read within `timeout`, after which the wait ends
/// in [`ArmError::NoAnswer`]. A missing socket file, or a connect nothing
/// answers, is [`ArmError::Unreachable`].
///
/// # Errors
/// [`ArmError`], as above.
pub fn send_arm(path: &Path, nonce: &str, timeout: Duration) -> Result<String, ArmError> {
    let mut stream = UnixStream::connect(path).map_err(ArmError::Unreachable)?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(ArmError::NoAnswer)?;
    let request = json!({"v": 1, "kind": "arm", "nonce": nonce});
    writeln!(stream, "{request}").map_err(ArmError::NoAnswer)?;
    stream.flush().map_err(ArmError::NoAnswer)?;
    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    match reader.read_line(&mut line) {
        Ok(0) => Err(ArmError::NoAnswer(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the endpoint closed without a reply",
        ))),
        Ok(_) => Ok(line.trim_end_matches(['\r', '\n']).to_string()),
        Err(error) => Err(ArmError::NoAnswer(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{EndpointState, Probe, accept_loop, bind};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::thread;
    use tempfile::tempdir;

    const NONCE: &str = "00112233445566778899aabbccddeeff";

    fn own_uid() -> u32 {
        // SAFETY: `geteuid` has no failure mode and no preconditions.
        unsafe { libc::geteuid() }
    }

    /// Serves an unarmed endpoint state at a fresh socket path, returning
    /// the path. The accept loop runs detached with the request timeout
    /// tests use elsewhere.
    fn serve_armed_state(dir: &Path) -> PathBuf {
        let path = dir.join("79301cf9592f327a-12.sock");
        let listener = bind(&path, own_uid()).unwrap();
        let probe = Probe::new(
            NONCE.to_string(),
            Path::new("/usr/bin/pmux-explorer"),
            Box::new(|| 0),
        );
        let state = Arc::new(EndpointState::new(own_uid(), "%12".to_string(), probe));
        thread::spawn(move || {
            let _ = accept_loop(
                listener,
                state,
                crate::mcp::LineWriter::new(io::sink()),
                Duration::from_millis(500),
            );
        });
        path
    }

    #[test]
    fn send_arm_gets_the_armed_reply() {
        let tmp = tempdir().unwrap();
        let path = serve_armed_state(tmp.path());
        let reply = send_arm(&path, NONCE, Duration::from_secs(2)).unwrap();
        let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply, serde_json::json!({"ok": true, "armed": true}));
    }

    #[test]
    fn a_missing_endpoint_is_unreachable() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("79301cf9592f327a-12.sock");
        let error = send_arm(&path, NONCE, Duration::from_secs(2)).unwrap_err();
        assert!(matches!(error, ArmError::Unreachable(_)));
    }

    #[test]
    fn an_endpoint_that_closes_without_a_reply_is_no_answer() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("79301cf9592f327a-12.sock");
        let listener = bind(&path, own_uid()).unwrap();
        // A peer that reads one line and drops the connection, replying
        // with nothing.
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            let _ = io::BufRead::read_line(&mut io::BufReader::new(&mut stream), &mut line);
            drop(stream);
        });

        let error = send_arm(&path, NONCE, Duration::from_secs(2)).unwrap_err();
        assert!(matches!(error, ArmError::NoAnswer(_)));
    }

    #[test]
    fn an_endpoint_that_never_answers_hits_the_timeout() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("79301cf9592f327a-12.sock");
        let listener = bind(&path, own_uid()).unwrap();
        // A peer that accepts and answers nothing; the client's short
        // answer window ends the wait instead of the real 2 seconds.
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_secs(2));
            drop(stream);
        });

        let error = send_arm(&path, NONCE, Duration::from_millis(150)).unwrap_err();
        assert!(matches!(error, ArmError::NoAnswer(_)));
    }
}
