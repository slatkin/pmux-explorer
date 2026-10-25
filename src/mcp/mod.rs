//! The MCP channel server mode: the pieces `--mcp` runs on stdio.
//!
//! A Claude Code channel server inside each parent session, so pmux can
//! push a report pointer into the session without typing into the terminal.
//! [`run_server`] assembles the pieces on three threads (design decision
//! D1): the stdin loop answers the JSON-RPC protocol and fires the arm
//! probe; the accept loop serves the endpoint socket's reports, statuses
//! and arms; and the thread that started them joins the stdin loop and
//! removes the endpoint socket at EOF. Every stdout line goes through one
//! shared [`LineWriter`], so a channel notification never interleaves with
//! a reply, and every log line goes to stderr — stdout holds only
//! protocol. [`rpc`] parses one stdin line and builds each reply behind
//! that writer; [`path`] derives the endpoint path from the tmux
//! environment (D4); [`endpoint`] binds the socket behind the same-user
//! gate (D5); [`connection`] serves each accepted connection; [`probe`]
//! owns the arm probe and its repeat gate (D3); [`client`] sends the
//! `channel-ready` arm request to an endpoint.
//!
//! The visibility echo:`--mcp` is a binary mode, so the module surface is
//! crate-only and the submodules are private — their APIs re-export here,
//! so items live behind one path.

use serde_json::Value;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

mod client;
mod connection;
mod endpoint;
mod path;
mod probe;
mod rpc;

pub use client::{ArmError, send_arm};
pub use connection::{DEFAULT_READ_TIMEOUT, EndpointState, accept_loop};
pub use endpoint::{BindError, bind};
pub use path::endpoint_path;
pub use probe::{INITIALIZED, MAX_REPEATS, Probe, REPEAT_PERIOD};

pub use rpc::{
    Incoming, LineWriter, METHOD_NOT_FOUND, PARSE_ERROR, RpcNotification, RpcRequest, error_reply,
    parse_line, request_reply,
};

/// The stderr prefix every server log line carries.
const LOG_PREFIX: &str = "pmux-explorer --mcp:";

/// Runs the MCP channel server mode until stdin reaches EOF.
///
/// Three threads (design decision D1): the stdin loop answers the
/// protocol on the shared writer; the accept loop serves the endpoint
/// socket when one could be bound; this thread joins the stdin loop and,
/// at EOF, removes the socket it owns. A missing tmux environment or a
/// refused bind is logged to stderr, and the server runs with no endpoint
/// and still answers MCP. A probe that cannot be built is logged, and the
/// server runs unarmed — no probe is ever emitted and no request arms it.
pub fn run_server() -> i32 {
    let writer = LineWriter::new(io::stdout());
    let uid = current_uid();
    let tmux_pane = std::env::var("TMUX_PANE").ok();

    let (probe, probe_fired) = match Probe::for_current_exe() {
        Ok(probe) => (probe, false),
        Err(error) => {
            eprintln!("{LOG_PREFIX} no arm probe ({error}); the server runs unarmed");
            (Probe::unarmed(), true)
        }
    };
    let state = Arc::new(EndpointState::new(
        uid,
        tmux_pane.clone().unwrap_or_default(),
        probe,
    ));

    // The endpoint: the socket to bind, or the reason there is none.
    let tmux = std::env::var("TMUX").ok();
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();
    let mut owned_socket: Option<PathBuf> = None;
    match endpoint_path(
        tmux.as_deref(),
        tmux_pane.as_deref(),
        xdg_runtime_dir.as_deref(),
        uid,
    ) {
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
    let stdin_state = Arc::clone(&state);
    let stdin_writer = writer.clone();
    let stdin_thread = thread::spawn(move || {
        stdin_loop(
            &stdin_state.probe,
            &stdin_writer,
            io::stdin().lock(),
            probe_fired,
        );
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

/// The stdin loop: one JSON-RPC message per line in, one reply per request
/// out, and the arm probe after the first `notifications/initialized`.
///
/// `probe_fired` starts `true` for a server running unarmed
/// ([`Probe::unarmed`]), so no probe is ever emitted; otherwise the first
/// `initialized` notification fires the probe and a duplicate cannot
/// re-fire it. Every failure — a stdin read error, a failed write — is
/// logged to stderr and the loop continues; only EOF or an unreadable
/// stdin ends it. A malformed line gets the [`PARSE_ERROR`] reply with a
/// null `id`, since a malformed line carries no parsed id, and the server
/// keeps running.
fn stdin_loop<R: io::Read, W: io::Write>(
    probe: &Probe,
    writer: &LineWriter<W>,
    reader: R,
    mut probe_fired: bool,
) {
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
            Incoming::Notification(notification) => {
                if probe_fired {
                    continue;
                }
                match probe.emit_after_notification(&notification, writer) {
                    Ok(true) => probe_fired = true,
                    Ok(false) => {}
                    Err(error) => {
                        eprintln!("{LOG_PREFIX} the probe could not be written: {error}");
                    }
                }
            }
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
fn current_uid() -> u32 {
    // SAFETY: `geteuid` has no failure mode and no preconditions.
    unsafe { libc::geteuid() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;
    use std::sync::Mutex;

    /// The probe nonce the test probe carries.
    const NONCE: &str = "00112233445566778899aabbccddeeff";

    /// A stdout recorder the test inspects through clones of its shared
    /// buffer, since the shared [`LineWriter`] keeps the writer private.
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<u8>>>);

    impl io::Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A probe with a fixed nonce and exe whose clock never moves.
    fn probe() -> Probe {
        Probe::new(
            NONCE.to_string(),
            Path::new("/usr/bin/pmux-explorer"),
            Box::new(|| 0),
        )
    }

    /// Runs the stdin loop over `input` and returns the stdout it wrote.
    fn stdout_of(probe: &Probe, input: &str, probe_fired: bool) -> String {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        stdin_loop(probe, &writer, input.as_bytes(), probe_fired);
        String::from_utf8_lossy(&recorder.0.lock().unwrap()).to_string()
    }

    // Row 5.1: the loop answers requests.

    #[test]
    fn a_request_gets_its_reply() {
        let stdout = stdout_of(
            &probe(),
            "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}\n",
            false,
        );
        let reply: Value = serde_json::from_str(stdout.trim_end()).unwrap();
        assert_eq!(reply["id"], 7);
        assert_eq!(reply["result"], json!({}));
    }

    #[test]
    fn an_unknown_method_gets_method_not_found_and_the_server_keeps_running() {
        let stdout = stdout_of(
            &probe(),
            concat!(
                "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n",
                "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n",
            ),
            false,
        );
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

    // Row 5.1: the malformed-line decision, deferred from row 1.1.

    #[test]
    fn a_malformed_line_gets_the_parse_error_with_a_null_id() {
        let stdout = stdout_of(
            &probe(),
            concat!(
                "not json\n",
                "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n"
            ),
            false,
        );
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

    // Row 5.1: the probe fires once, and a duplicate cannot re-fire it.

    #[test]
    fn the_probe_fires_after_initialized_and_not_again() {
        let stdout = stdout_of(
            &probe(),
            concat!(
                "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
                "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
                "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}\n",
            ),
            false,
        );
        let lines: Vec<Value> = stdout
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        // One probe, then the ping reply — the duplicate initialized
        // emitted nothing.
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["method"], "notifications/claude/channel");
        assert_eq!(
            lines[0]["params"]["meta"],
            json!({"kind": "probe", "nonce": NONCE})
        );
        assert_eq!(lines[1]["id"], 3);
    }

    #[test]
    fn another_notification_emits_nothing() {
        let stdout = stdout_of(
            &probe(),
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\"}\n",
            false,
        );
        assert_eq!(stdout, "");
    }

    // Row 5.1: the unarmed fallback never emits a probe.

    #[test]
    fn an_unarmed_server_emits_no_probe_at_all() {
        let stdout = stdout_of(
            &Probe::unarmed(),
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
            true,
        );
        assert_eq!(stdout, "");
    }
}
