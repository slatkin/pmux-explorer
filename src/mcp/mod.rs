//! The MCP channel server mode: the pieces `--mcp` runs on stdio.
//!
//! A Claude Code channel server inside each parent session, so pmux can
//! push a report pointer into the session without typing into the terminal.
//! [`run_server`] assembles the pieces on three threads (design decision
//! D1): the stdin loop answers the JSON-RPC protocol; the accept loop
//! serves the endpoint socket's reports, statuses and arms; and the
//! thread that started them joins the stdin loop and removes the
//! endpoint socket at EOF. Every stdout line goes through one shared
//! [`LineWriter`], so a channel notification never interleaves with a
//! reply, and every log line goes to stderr — stdout holds only
//! protocol. [`rpc`] parses one stdin line and builds each reply behind
//! that writer; [`path`] derives the endpoint path from the tmux
//! environment (D4); [`endpoint`] binds the socket behind the same-user
//! gate (D5); [`connection`] serves each accepted connection; [`client`]
//! sends the `channel-ready` arm request to an endpoint. The server
//! emits a channel event only for a report — arming is a user decision,
//! so no probe ever prompts the model (design decision D3: the
//! orchestration prompt that wants the channel tells Claude to run
//! `channel-ready`).
//!
//! `--mcp` is a binary mode, so the module surface is crate-only and the
//! submodules are private — the API the rest of the crate uses re-exports
//! here, so those items live behind one path.

use serde_json::Value;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

mod client;
mod connection;
mod endpoint;
mod path;
mod rpc;

pub use client::{ArmError, send_arm};
pub(crate) use connection::reply;
pub use connection::{DEFAULT_READ_TIMEOUT, EndpointState, accept_loop};
pub use endpoint::bind;
pub use path::{EndpointEnv, endpoint_path};

pub(crate) use rpc::{Incoming, LineWriter, PARSE_ERROR, error_reply, parse_line, request_reply};

/// The stderr prefix every server log line carries.
const LOG_PREFIX: &str = "pmux-explorer --mcp:";

/// Runs the MCP channel server mode until stdin reaches EOF.
///
/// Three threads (design decision D1): the stdin loop answers the
/// protocol on the shared writer; the accept loop serves the endpoint
/// socket when one could be bound; this thread joins the stdin loop and,
/// at EOF, removes the socket it owns. A missing tmux environment or a
/// refused bind is logged to stderr, and the server runs with no endpoint
/// and still answers MCP.
pub fn run_server() -> i32 {
    let writer = LineWriter::new(io::stdout());
    let env = EndpointEnv::from_process();
    let uid = env.uid;

    let state = Arc::new(EndpointState::new(
        uid,
        env.tmux_pane.unwrap_or_default().to_string(),
    ));

    // The endpoint: the socket to bind, or the reason there is none.
    let mut owned_socket: Option<PathBuf> = None;
    match endpoint_path(env) {
        Some(path) => match bind(&path, uid) {
            Ok(listener) => {
                let accept_state = Arc::clone(&state);
                let accept_writer = writer.clone();
                owned_socket = Some(path);
                thread::spawn(move || {
                    if let Err(error) =
                        accept_loop(listener, accept_state, accept_writer, DEFAULT_READ_TIMEOUT)
                    {
                        eprintln!("{LOG_PREFIX} the endpoint stopped serving: {error}");
                    }
                });
            }
            Err(error) => eprintln!("{LOG_PREFIX} {error}"),
        },
        None => eprintln!("{LOG_PREFIX} no endpoint: TMUX and TMUX_PANE must name this pane"),
    }

    // The stdin loop owns the protocol until EOF; the accept thread ends
    // with the process, since a blocking accept has no other stop.
    let stdin_writer = writer.clone();
    let stdin_thread = thread::spawn(move || {
        stdin_loop(&stdin_writer, io::stdin().lock());
    });
    stdin_thread.join().expect("the stdin loop must not panic");

    if let Some(socket) = owned_socket
        && let Err(error) = std::fs::remove_file(&socket)
    {
        eprintln!(
            "{LOG_PREFIX} could not remove the endpoint socket {}: {error}",
            socket.display(),
        );
    }
    0
}

/// The stdin loop: one JSON-RPC message per line in, one reply per
/// request out. Notifications are ignored — the server emits a channel
/// event only for a report, never a probe of its own. Every failure — a
/// stdin read error, a failed write — is logged to stderr and the loop
/// continues; only EOF or an unreadable stdin ends it. A malformed line
/// gets the [`PARSE_ERROR`] reply with a null `id`, since a malformed
/// line carries no parsed id, and the server keeps running.
fn stdin_loop<R: io::Read, W: io::Write>(writer: &LineWriter<W>, reader: R) {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return,
            Ok(_) => {}
            Err(error) => {
                eprintln!("{LOG_PREFIX} reading stdin failed: {error}");
                return;
            }
        }
        match parse_line(&line) {
            Incoming::Request(request) => {
                if let Err(error) = writer.write_json(&request_reply(&request)) {
                    eprintln!("{LOG_PREFIX} writing a reply failed: {error}");
                }
            }
            Incoming::Notification(_) => {}
            Incoming::Malformed => {
                let reply = error_reply(&Value::Null, PARSE_ERROR, "Parse error");
                if let Err(error) = writer.write_json(&reply) {
                    eprintln!("{LOG_PREFIX} writing the parse-error reply failed: {error}");
                }
            }
        }
    }
}

/// The current user's id, for the endpoint path and the peer-uid gate.
pub(crate) fn current_uid() -> u32 {
    // SAFETY: `geteuid` has no failure mode and no preconditions.
    unsafe { libc::geteuid() }
}

#[cfg(test)]
mod tests {
    use super::rpc::METHOD_NOT_FOUND;
    use super::*;
    use crate::test_support::Recorder;
    use serde_json::json;

    /// Runs the stdin loop over `input` and returns the stdout it wrote.
    fn stdout_of(input: &str) -> String {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        stdin_loop(&writer, input.as_bytes());
        recorder.stdout()
    }

    // The loop answers requests.

    #[test]
    fn a_request_gets_its_reply() {
        let stdout = stdout_of("{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}\n");
        let reply: Value = serde_json::from_str(stdout.trim_end()).unwrap();
        assert_eq!(reply["id"], 7);
        assert_eq!(reply["result"], json!({}));
    }

    #[test]
    fn an_unknown_method_gets_method_not_found_and_the_server_keeps_running() {
        let stdout = stdout_of(concat!(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n",
        ));
        let lines: Vec<Value> = stdout
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["id"], 1);
        assert_eq!(lines[0]["error"]["code"], METHOD_NOT_FOUND);
        // The unknown method did not end the loop: the next request was
        // still answered.
        assert_eq!(lines[1]["id"], 2);
        assert_eq!(lines[1]["result"], json!({}));
    }

    // The malformed-line decision, deferred from row 1.1.

    #[test]
    fn a_malformed_line_gets_the_parse_error_with_a_null_id() {
        let stdout = stdout_of(concat!(
            "not json\n",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n"
        ));
        let lines: Vec<Value> = stdout
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["id"], Value::Null);
        assert_eq!(lines[0]["error"]["code"], PARSE_ERROR);
        // The malformed line did not end the loop.
        assert_eq!(lines[1]["result"], json!({}));
    }

    // Notifications are ignored: the server never emits a channel event
    // of its own — `initialized` included, where the probe used to fire.

    #[test]
    fn initialized_and_every_other_notification_emit_nothing() {
        let stdout = stdout_of(concat!(
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\"}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}\n",
        ));
        let lines: Vec<Value> = stdout
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["id"], 3);
    }
}
