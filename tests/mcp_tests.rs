//! The MCP channel server mode, pinned through the compiled binary: the
//! stdio protocol, the arm probe, the endpoint socket and the EOF teardown
//! are child-process behavior the in-crate unit tests cannot observe.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_pmux-explorer");

/// The tmux env triple one server runs under, with the endpoint under
/// `dir`. The tmux socket path carries the test process's pid, so two
/// concurrent runs never share an endpoint file name.
struct ServerEnv {
    tmux: String,
    pane: &'static str,
    xdg_runtime_dir: PathBuf,
}

impl ServerEnv {
    fn new(dir: &Path) -> Self {
        Self {
            tmux: format!("/tmp/tmux-mcp-test-{}/default,1,0", std::process::id()),
            pane: "%4242",
            xdg_runtime_dir: dir.to_path_buf(),
        }
    }

    /// The env for a child: the tmux triple set, or removed when the test
    /// runs a server outside tmux — a test runner inside tmux must not
    /// leak its own `TMUX` into the child.
    fn apply(&self, command: &mut Command, in_tmux: bool) {
        if in_tmux {
            command.env("TMUX", &self.tmux);
            command.env("TMUX_PANE", self.pane);
        } else {
            command.env_remove("TMUX");
            command.env_remove("TMUX_PANE");
        }
        command.env("XDG_RUNTIME_DIR", &self.xdg_runtime_dir);
    }
}

/// A spawned server with piped stdio, and the handles to read it: the
/// child, the stdout reader thread, the channel every stdout line arrives
/// on, and the stderr the server logged (drained from the start, so a
/// chatty server can never fill the pipe and block).
struct Server {
    child: Child,
    reader: thread::JoinHandle<()>,
    lines: mpsc::Receiver<String>,
    logged: Arc<Mutex<String>>,
}

impl Server {
    fn spawn(env: &ServerEnv, in_tmux: bool) -> Self {
        let mut command = Command::new(BIN);
        command
            .arg("--mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        env.apply(&mut command, in_tmux);
        let mut child = command.spawn().expect("compiled binary must run");
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let logged = Arc::new(Mutex::new(String::new()));
        let stderr_sink = Arc::clone(&logged);
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        stderr_sink.lock().unwrap().push_str(&line);
                        line.clear();
                    }
                }
            }
        });

        let (line_tx, line_rx) = mpsc::channel::<String>();
        let reader = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if line_tx
                            .send(line.trim_end_matches('\n').to_string())
                            .is_err()
                        {
                            break;
                        }
                        line.clear();
                    }
                }
            }
        });
        Self {
            child,
            reader,
            lines: line_rx,
            logged,
        }
    }

    fn stdin(&mut self) -> ChildStdin {
        self.child.stdin.take().unwrap()
    }

    /// The next stdout line, with a generous wait so a hung server fails
    /// the test instead of hanging it.
    fn next_line(&self) -> String {
        self.lines
            .recv_timeout(Duration::from_secs(10))
            .expect("the server should have written a stdout line")
    }

    /// Ends the server: closes stdin, waits for the exit, joins the
    /// reader, and returns the exit status plus every stdout line the
    /// test has not consumed yet.
    fn shutdown(mut self, stdin: Option<ChildStdin>) -> (ExitStatus, Vec<String>) {
        drop(stdin);
        let status = self.child.wait().expect("the server should exit");
        self.reader.join().unwrap();
        let mut lines = Vec::new();
        while let Ok(line) = self.lines.try_recv() {
            lines.push(line);
        }
        (status, lines)
    }
}

/// Writes one JSON value as the next stdin line.
fn send(stdin: &mut ChildStdin, value: &Value) {
    writeln!(stdin, "{value}").unwrap();
    stdin.flush().unwrap();
}

/// Asserts the line parses as a JSON-RPC 2.0 message and returns it.
fn json_rpc(line: &str) -> Value {
    let value: Value = serde_json::from_str(line)
        .unwrap_or_else(|error| panic!("stdout line is not JSON: {line:?}: {error}"));
    assert_eq!(value["jsonrpc"], "2.0", "line: {line:?}");
    value
}

/// Sends the initialize request and returns the reply and its raw line.
fn initialize(server: &Server, stdin: &mut ChildStdin) -> (Value, String) {
    send(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2025-06-18"},
        }),
    );
    let line = server.next_line();
    let reply = json_rpc(&line);
    assert_eq!(reply["id"], 1);
    (reply, line)
}

/// The one endpoint socket the server created under `dir`.
fn socket_file_in(dir: &Path) -> PathBuf {
    let mut sockets: Vec<PathBuf> = std::fs::read_dir(dir.join("pmux-channel"))
        .expect("the endpoint directory should exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sock"))
        .collect();
    assert_eq!(sockets.len(), 1, "exactly one endpoint socket");
    sockets.pop().unwrap()
}

#[test]
fn the_full_channel_path_runs_through_the_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let env = ServerEnv::new(tmp.path());
    let mut server = Server::spawn(&env, true);
    let mut stdin = server.stdin();
    let mut seen: Vec<String> = Vec::new();

    // initialize: the reply carries the channel capability and the echoed
    // revision.
    let (reply, line) = initialize(&server, &mut stdin);
    seen.push(line);
    assert_eq!(reply["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(
        reply["result"]["capabilities"]["experimental"]["claude/channel"],
        json!({}),
    );

    // notifications/initialized: the probe, with a fresh 32-hex nonce.
    send(
        &mut stdin,
        &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    );
    let line = server.next_line();
    let probe = json_rpc(&line);
    seen.push(line);
    assert_eq!(probe["method"], "notifications/claude/channel");
    assert_eq!(probe["params"]["meta"]["kind"], "probe");
    let nonce = probe["params"]["meta"]["nonce"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(nonce.len(), 32);
    assert!(
        probe["params"]["content"]
            .as_str()
            .unwrap()
            .contains(&format!("channel-ready --nonce {nonce}")),
        "content: {}",
        probe["params"]["content"],
    );

    // The probe's command, as Claude would run it in its pane: the arm
    // request arms the server, and the exit code is 0.
    let mut ready = Command::new(BIN);
    ready.args(["channel-ready", "--nonce", &nonce]);
    env.apply(&mut ready, true);
    let ready = ready.output().expect("compiled binary must run");
    assert!(
        ready.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ready.stderr),
    );
    assert_eq!(
        serde_json::from_str::<Value>(String::from_utf8_lossy(&ready.stdout).trim_end()).unwrap(),
        json!({"ok": true, "armed": true}),
    );

    // A report through the endpoint socket, as pmux would send it.
    let socket = socket_file_in(tmp.path());
    let report_id = "00112233445566778899aabbccddeeff";
    let text = format!("pmux report {report_id}. the report is at /tmp/x.md:12");
    let mut client = UnixStream::connect(&socket).unwrap();
    writeln!(
        client,
        "{}",
        json!({
            "v": 1,
            "kind": "report",
            "id": report_id,
            "text": text,
            "bytes": "42",
            "token": "feed",
        })
    )
    .unwrap();
    client.flush().unwrap();
    let mut reply = String::new();
    BufReader::new(&mut client)
        .read_line(&mut reply)
        .expect("the server should reply to the report");
    assert_eq!(
        serde_json::from_str::<Value>(reply.trim_end()).unwrap(),
        json!({"ok": true, "emitted": true}),
    );
    drop(client);

    // Exactly one notification reached Claude, with the pointer as it
    // arrived and the opaque bytes as a string.
    let line = server.next_line();
    let event = json_rpc(&line);
    seen.push(line);
    assert_eq!(event["method"], "notifications/claude/channel");
    assert_eq!(event["params"]["content"], text);
    assert_eq!(
        event["params"]["meta"],
        json!({"kind": "report", "report_id": report_id, "bytes": "42"}),
    );

    // EOF: the server exits 0, and the endpoint socket is gone.
    let logged = Arc::clone(&server.logged);
    let (status, extra) = server.shutdown(Some(stdin));
    assert_eq!(status.code(), Some(0), "exit code");
    assert!(extra.is_empty(), "no stdout beyond the protocol: {extra:?}");
    assert!(!socket.exists(), "the endpoint socket should be gone");
    assert_eq!(seen.len(), 3, "three protocol lines: {seen:?}");

    // Stdout held only protocol: every line was a JSON-RPC message, and
    // nothing was logged to stderr on the happy path.
    for line in &seen {
        json_rpc(line);
    }
    assert!(logged.lock().unwrap().is_empty(), "no stderr logs");
}

#[test]
fn a_server_without_tmux_creates_no_socket_and_still_answers() {
    let tmp = tempfile::tempdir().unwrap();
    let env = ServerEnv::new(tmp.path());
    let mut server = Server::spawn(&env, false);
    let mut stdin = server.stdin();

    let (reply, _) = initialize(&server, &mut stdin);
    assert_eq!(reply["result"]["protocolVersion"], "2025-06-18");
    send(
        &mut stdin,
        &json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}),
    );
    let reply = json_rpc(&server.next_line());
    assert_eq!(reply["result"], json!({}));

    let logged = Arc::clone(&server.logged);
    let (status, _) = server.shutdown(Some(stdin));
    assert_eq!(status.code(), Some(0));
    // No socket file was created anywhere under the runtime dir.
    assert!(!tmp.path().join("pmux-channel").exists());
    // Outside tmux the server logs the reason for having no endpoint.
    let logged = logged.lock().unwrap().clone();
    assert!(logged.contains("no endpoint"), "stderr: {logged:?}");
}
