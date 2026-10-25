//! The endpoint connection: the accept loop and the request handlers.
//!
//! Every connection accepted on the endpoint socket carries one request
//! line and one reply line, and then closes. The same-user gate (design
//! decision D5) is the peer uid: a connection from another user is closed
//! with no reply and nothing is emitted. A request line longer than 4 KiB,
//! or one that does not arrive within the read timeout, gets the `invalid`
//! reply, as does a request that is not JSON, has a `v` other than `1`, or
//! has an unknown `kind`.
//!
//! The `report` handler validates the pointer shape and, on an armed
//! server, emits one `notifications/claude/channel` event through the
//! shared stdout writer and answers only after that write is flushed. The
//! `status` handler answers with the arm state and the pane and never
//! emits. The `token` and `bytes` report fields are opaque: the server
//! never validates them, never emits `token`, and copies `bytes` into the
//! event's `meta` as a string as given.

use serde_json::{Map, Value, json};
use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::rpc::LineWriter;

/// The request line limit: a longer line gets the `invalid` reply.
const MAX_REQUEST_LINE_BYTES: usize = 4096;

/// The default wait for a request line before the `invalid` reply.
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(2);

/// The pointer text limit: a longer text is an invalid report.
const MAX_TEXT_BYTES: usize = 1024;

/// The report id length: 32 lowercase hex characters, like the nonce.
const REPORT_ID_LENGTH: usize = 32;

/// The state one endpoint socket serves.
///
/// Shared between the accept loop and the rest of the server: the arm
/// state changes at runtime, the peer uid and the pane are fixed.
#[derive(Debug)]
pub struct EndpointState {
    /// Whether the session answered the arm request with the probe's
    /// nonce; an unarmed server emits no report events.
    pub armed: AtomicBool,
    /// The uid a connection's peer must have — the current user's id in
    /// production, an injected value in tests.
    pub uid: u32,
    /// The pane id (`TMUX_PANE`) the `status` reply reports.
    pub pane: String,
}

/// Accepts endpoint connections until the listener fails.
///
/// Each accepted connection is served in line ([`handle_connection`]); a
/// transient accept failure (an interrupted call, an aborted connection)
/// keeps the loop running, any other failure ends it and is returned for
/// the caller to log.
pub fn accept_loop<W: io::Write>(
    listener: UnixListener,
    state: Arc<EndpointState>,
    writer: LineWriter<W>,
    read_timeout: Duration,
) -> io::Result<()> {
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => handle_connection(&mut stream, &state, &writer, read_timeout),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::ConnectionAborted
                ) =>
            {
                continue;
            }
            Err(error) => return Err(error),
        }
    }
}

/// Serves one accepted connection: the peer-uid gate, the request line,
/// and one reply line.
///
/// A connection whose peer uid differs from `state.uid`, or whose uid
/// cannot be read, is closed with no reply and nothing is emitted. A
/// request line that is oversized, late, or unusable gets the `invalid`
/// reply. The reply is best-effort: a peer that vanished before the reply
/// is not an error, and no state changes on a failed reply.
pub fn handle_connection<W: io::Write>(
    stream: &mut UnixStream,
    state: &EndpointState,
    writer: &LineWriter<W>,
    read_timeout: Duration,
) {
    if peer_uid(stream) != Some(state.uid) {
        return;
    }
    if stream.set_read_timeout(Some(read_timeout)).is_err() {
        return;
    }
    let reply = match read_request_line(stream) {
        Some(line) => socket_request_reply(&line, state, writer),
        None => invalid_reply(),
    };
    let _ = write_reply(stream, &reply);
}

/// The uid of the connected peer, or `None` when the system call fails.
///
/// A failed lookup counts as a foreign peer, so the connection is closed:
/// the same-user gate fails closed.
#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> Option<u32> {
    use std::os::fd::AsRawFd;

    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `getsockopt` writes only `credentials` and `length`, both
    // valid pointers sized for the call, and the fd is open for the call.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut credentials as *mut libc::ucred as *mut libc::c_void,
            &mut length,
        )
    };
    (result == 0).then_some(credentials.uid)
}

/// The uid of the connected peer, or `None` when the system call fails.
///
/// A failed lookup counts as a foreign peer, so the connection is closed:
/// the same-user gate fails closed.
#[cfg(target_os = "macos")]
fn peer_uid(stream: &UnixStream) -> Option<u32> {
    use std::os::fd::AsRawFd;

    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    // SAFETY: `getpeereid` writes only `uid` and `gid`, both valid pointers
    // sized for the call, and the fd is open for the call.
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    (result == 0).then_some(uid)
}

/// Reads the one request line, up to [`MAX_REQUEST_LINE_BYTES`] bytes.
///
/// `None` means no usable line: a line over the limit, a line that does
/// not arrive within the stream's read timeout, a connection reset, or a
/// connection closed with nothing sent. A line is complete at `\n`, or at
/// EOF when at least one byte arrived.
fn read_request_line(stream: &mut UnixStream) -> Option<String> {
    let mut line: Vec<u8> = Vec::new();
    let mut byte = [0u8];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => {
                return (!line.is_empty()).then(|| String::from_utf8_lossy(&line).into_owned());
            }
            Ok(_) if byte[0] == b'\n' => {
                return Some(String::from_utf8_lossy(&line).into_owned());
            }
            Ok(_) => {
                line.push(byte[0]);
                if line.len() > MAX_REQUEST_LINE_BYTES {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
}

/// Builds the reply for one socket request line.
///
/// The line is one JSON object with `v: 1`; the `report` and `status`
/// kinds are handled, everything else — not JSON, a wrong `v`, a missing
/// or unknown `kind` — gets the `invalid` reply.
fn socket_request_reply<W: io::Write>(
    line: &str,
    state: &EndpointState,
    writer: &LineWriter<W>,
) -> Value {
    let Ok(request) = serde_json::from_str::<Value>(line) else {
        return invalid_reply();
    };
    let Some(object) = request.as_object() else {
        return invalid_reply();
    };
    if object.get("v").and_then(Value::as_i64) != Some(1) {
        return invalid_reply();
    }
    match object.get("kind").and_then(Value::as_str) {
        Some("report") => report_reply(state, object, writer),
        Some("status") => status_reply(state),
        _ => invalid_reply(),
    }
}

/// Builds the reply for one `report` request, emitting the event first.
///
/// The request is valid only when `id` is 32 lowercase hex characters and
/// `text` is one line of at most 1 KiB that starts with
/// `pmux report <id>. `; anything else is the `invalid` reply, with no
/// emission. On an armed server the notification is written and flushed
/// before the reply is built, so the reply only says `emitted` when the
/// write reached stdout; a failed write is `write-failed`.
fn report_reply<W: io::Write>(
    state: &EndpointState,
    object: &Map<String, Value>,
    writer: &LineWriter<W>,
) -> Value {
    let Some(report) = ReportRequest::from_fields(object) else {
        return invalid_reply();
    };
    if !state.armed.load(Ordering::Relaxed) {
        return json!({"ok": false, "reason": "unarmed"});
    }
    match writer.write_json(&channel_notification(&report)) {
        Ok(()) => json!({"ok": true, "emitted": true}),
        Err(_) => json!({"ok": false, "reason": "write-failed"}),
    }
}

/// Builds the reply for one `status` request. It never emits anything.
fn status_reply(state: &EndpointState) -> Value {
    json!({
        "ok": true,
        "armed": state.armed.load(Ordering::Relaxed),
        "pane": state.pane,
    })
}

/// The `invalid` reply, for every unusable request.
fn invalid_reply() -> Value {
    json!({"ok": false, "reason": "invalid"})
}

/// One shape-valid report request: the checked fields, plus `bytes` as a
/// string. `token` is never read.
struct ReportRequest {
    id: String,
    text: String,
    bytes: String,
}

impl ReportRequest {
    /// Extracts the report request from a socket request object, or `None`
    /// when the shape is wrong: `id` must be 32 lowercase hex characters,
    /// `text` one line of at most 1 KiB starting with
    /// `pmux report <id>. `. `token` is opaque and never validated;
    /// `bytes` is copied as a string as given.
    fn from_fields(object: &Map<String, Value>) -> Option<Self> {
        let id = object.get("id")?.as_str()?;
        let text = object.get("text")?.as_str()?;
        if !is_report_id(id) || !is_pointer_text(id, text) {
            return None;
        }
        Some(Self {
            id: id.to_string(),
            text: text.to_string(),
            bytes: bytes_as_string(object.get("bytes")),
        })
    }
}

/// Whether `id` is 32 lowercase hex characters.
fn is_report_id(id: &str) -> bool {
    id.len() == REPORT_ID_LENGTH
        && id
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Whether `text` is one line of at most [`MAX_TEXT_BYTES`] bytes that
/// starts with the pointer for `id`.
fn is_pointer_text(id: &str, text: &str) -> bool {
    text.len() <= MAX_TEXT_BYTES
        && !text.contains('\n')
        && text.starts_with(&format!("pmux report {id}. "))
}

/// `bytes` as a string as given, or `""` when it is absent or null. The
/// field is opaque, so a non-string value is stringified, not rejected.
fn bytes_as_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

/// The channel event for one report: the text as it arrived, and the meta
/// keys the Channels contract allows — no `token`, so it is not emitted.
fn channel_notification(report: &ReportRequest) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/claude/channel",
        "params": {
            "content": report.text,
            "meta": {
                "kind": "report",
                "report_id": report.id,
                "bytes": report.bytes,
            },
        },
    })
}

/// Writes one reply line and flushes, closing the connection after.
fn write_reply(stream: &mut UnixStream, reply: &Value) -> io::Result<()> {
    writeln!(stream, "{reply}")?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::bind;
    use serde_json::Value as Json;
    use std::sync::Mutex;
    use std::thread;
    use tempfile::tempdir;

    /// The test read timeout: long enough for a same-host write to arrive,
    /// short enough that the timeout case does not wait the real 2 seconds.
    const TEST_READ_TIMEOUT: Duration = Duration::from_millis(100);

    const REPORT_ID: &str = "00112233445566778899aabbccddeeff";

    fn own_uid() -> u32 {
        // SAFETY: `geteuid` has no failure mode and no preconditions.
        unsafe { libc::geteuid() }
    }

    fn state(armed: bool, uid: u32, pane: &str) -> EndpointState {
        EndpointState {
            armed: AtomicBool::new(armed),
            uid,
            pane: pane.to_string(),
        }
    }

    /// A report request line with the given text and extra fields.
    fn report_json(id: &str, text: &str, fields: &[(&str, Json)]) -> String {
        let mut request = json!({"v": 1, "kind": "report", "id": id, "text": text});
        let object = request.as_object_mut().unwrap();
        for (key, value) in fields {
            object.insert((*key).to_string(), value.clone());
        }
        format!("{}\n", request)
    }

    /// Serves one connection on a fresh endpoint and returns the reply line
    /// the client read. `input` is sent before the server reads; with no
    /// input and no half-close, the server waits out the read timeout.
    fn exchange<W: io::Write>(
        input: Option<&[u8]>,
        state: &EndpointState,
        writer: &LineWriter<W>,
    ) -> String {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("79301cf9592f327a-12.sock");
        let listener = bind(&path, own_uid()).unwrap();
        let mut client = UnixStream::connect(&path).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        if let Some(input) = input {
            client.write_all(input).unwrap();
        }
        handle_connection(&mut server, state, writer, TEST_READ_TIMEOUT);
        drop(server);
        let mut reply = String::new();
        match client.read_to_string(&mut reply) {
            Ok(_) => {}
            // Closing a socket with unread sent data can reset the peer;
            // the reply bytes, if any, were still delivered.
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
            Err(error) => panic!("reading the reply failed: {error}"),
        }
        reply
    }

    /// The stdout the recorder saw, as one string, and the flush count.
    fn stdout_of(recorder: &Recorder) -> (String, usize) {
        (
            String::from_utf8_lossy(&recorder.buf.lock().unwrap()).to_string(),
            *recorder.flushes.lock().unwrap(),
        )
    }

    /// A stdout recorder whose state the test inspects through clones of
    /// its shared buffers, since the shared [`LineWriter`] keeps the
    /// writer itself private.
    #[derive(Clone, Default)]
    struct Recorder {
        buf: Arc<Mutex<Vec<u8>>>,
        flushes: Arc<Mutex<usize>>,
    }

    impl io::Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            *self.flushes.lock().unwrap() += 1;
            Ok(())
        }
    }

    struct Failing;

    impl io::Write for Failing {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "gone"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // Row 3.2: the `invalid` cases, one table.

    #[test]
    fn an_unusable_request_line_gets_invalid() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(true, own_uid(), "%12");
        for (name, input) in [
            (
                "an oversized line",
                Some(vec![b'a'; MAX_REQUEST_LINE_BYTES + 1]),
            ),
            ("no line within the timeout", None),
            ("bad JSON", Some(b"not json\n".to_vec())),
            (
                "a wrong v",
                Some(b"{\"v\":2,\"kind\":\"status\"}\n".to_vec()),
            ),
            ("a missing v", Some(b"{\"kind\":\"status\"}\n".to_vec())),
            (
                "a kind that is not a string",
                Some(b"{\"v\":1,\"kind\":2}\n".to_vec()),
            ),
            (
                "an unknown kind",
                Some(b"{\"v\":1,\"kind\":\"other\"}\n".to_vec()),
            ),
        ] {
            let reply = exchange(input.as_deref(), &state, &writer);
            assert_eq!(
                reply, "{\"ok\":false,\"reason\":\"invalid\"}\n",
                "case: {name}",
            );
        }
        // Not one case emitted anything to Claude.
        let (stdout, flushes) = stdout_of(&recorder);
        assert_eq!(stdout, "");
        assert_eq!(flushes, 0);
    }

    // Row 3.2: the same-user gate.

    #[test]
    fn a_foreign_peer_is_closed_with_no_reply() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(true, own_uid() + 1, "%12");
        let request = b"{\"v\":1,\"kind\":\"status\"}\n";
        let reply = exchange(Some(&request[..]), &state, &writer);
        assert_eq!(reply, "");
        let (stdout, flushes) = stdout_of(&recorder);
        assert_eq!(stdout, "");
        assert_eq!(flushes, 0);
    }

    // Row 3.2: the accept loop serves connections.

    #[test]
    fn the_accept_loop_serves_a_connection() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("79301cf9592f327a-12.sock");
        let listener = bind(&path, own_uid()).unwrap();
        let state = Arc::new(state(true, own_uid(), "%12"));
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        // The loop runs until the listener fails, which never happens in
        // this test, so the thread is left detached after the exchange.
        let tmp = std::sync::Mutex::new(Some(tmp));
        thread::spawn(move || {
            let _keep_dir_alive = tmp;
            let _ = accept_loop(listener, state, writer, TEST_READ_TIMEOUT);
        });

        let mut client = UnixStream::connect(&path).unwrap();
        client
            .write_all(b"{\"v\":1,\"kind\":\"status\"}\n")
            .unwrap();
        let mut reply = String::new();
        client.read_to_string(&mut reply).unwrap();
        let reply: Json = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply, json!({"ok": true, "armed": true, "pane": "%12"}),);
    }

    // Row 3.3: the "Report request" scenarios.

    #[test]
    fn an_armed_report_emits_one_notification_then_replies() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(true, own_uid(), "%12");
        let text = format!("pmux report {REPORT_ID}. the report is at /tmp/x.md:12");
        let request = report_json(
            REPORT_ID,
            &text,
            &[("bytes", json!("42")), ("token", json!("feed"))],
        );
        let reply = exchange(Some(request.as_bytes()), &state, &writer);
        let reply: Json = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply, json!({"ok": true, "emitted": true}));

        // One flushed line, written before the reply was built.
        let (stdout, flushes) = stdout_of(&recorder);
        assert_eq!(flushes, 1);
        assert_eq!(stdout.matches('\n').count(), 1);
        let emitted: Json = serde_json::from_str(stdout.trim_end()).unwrap();
        assert_eq!(emitted["method"], "notifications/claude/channel");
        assert_eq!(emitted["params"]["content"], text);
        assert_eq!(
            emitted["params"]["meta"],
            json!({"kind": "report", "report_id": REPORT_ID, "bytes": "42"}),
        );
    }

    #[test]
    fn an_unarmed_report_replies_unarmed_and_emits_nothing() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(false, own_uid(), "%12");
        let text = format!("pmux report {REPORT_ID}. the report is at /tmp/x.md:12");
        let request = report_json(REPORT_ID, &text, &[]);
        let reply = exchange(Some(request.as_bytes()), &state, &writer);
        let reply: Json = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply, json!({"ok": false, "reason": "unarmed"}));
        let (stdout, flushes) = stdout_of(&recorder);
        assert_eq!(stdout, "");
        assert_eq!(flushes, 0);
    }

    #[test]
    fn a_report_with_a_wrong_shape_is_invalid_and_emits_nothing() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(true, own_uid(), "%12");
        let text = format!("pmux report {REPORT_ID}. the report is at /tmp/x.md:12");
        for (name, request) in [
            (
                "a line break in the text",
                report_json(REPORT_ID, &format!("{text}\nmore"), &[]),
            ),
            (
                "a wrong text prefix",
                report_json(REPORT_ID, &format!("see {text}"), &[]),
            ),
            (
                "an oversized text",
                report_json(
                    REPORT_ID,
                    &format!("pmux report {REPORT_ID}. {}", "x".repeat(1025)),
                    &[],
                ),
            ),
            (
                "an id that is too short",
                report_json("abc", "pmux report abc. x", &[]),
            ),
            (
                "an id with uppercase hex",
                report_json(
                    &REPORT_ID.to_uppercase(),
                    &format!("pmux report {}. x", REPORT_ID.to_uppercase()),
                    &[],
                ),
            ),
            (
                "an id that does not match the text",
                report_json(
                    REPORT_ID,
                    &format!("pmux report {} . x", "f".repeat(32)),
                    &[],
                ),
            ),
            (
                "an id that is not a string",
                r#"{"v":1,"kind":"report","id":1,"text":"pmux report 1. x"}"#.to_string(),
            ),
            (
                "a text that is not a string",
                r#"{"v":1,"kind":"report","id":"00112233445566778899aabbccddeeff","text":5}"#
                    .to_string(),
            ),
        ] {
            let reply = exchange(Some(request.as_bytes()), &state, &writer);
            assert_eq!(
                reply, "{\"ok\":false,\"reason\":\"invalid\"}\n",
                "case: {name}",
            );
        }
        let (stdout, flushes) = stdout_of(&recorder);
        assert_eq!(stdout, "");
        assert_eq!(flushes, 0);
    }

    // Row 3.3: the "Opaque report fields" scenarios.

    #[test]
    fn missing_opaque_fields_give_an_empty_bytes_meta() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(true, own_uid(), "%12");
        let text = format!("pmux report {REPORT_ID}. the report is at /tmp/x.md:12");
        let request = report_json(REPORT_ID, &text, &[]);
        let reply = exchange(Some(request.as_bytes()), &state, &writer);
        let reply: Json = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply, json!({"ok": true, "emitted": true}));

        let (stdout, _) = stdout_of(&recorder);
        let emitted: Json = serde_json::from_str(stdout.trim_end()).unwrap();
        let meta = &emitted["params"]["meta"];
        assert_eq!(meta["bytes"], "");
        assert!(meta.get("token").is_none());
    }

    #[test]
    fn the_token_is_never_emitted_and_bytes_is_a_string_as_given() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(true, own_uid(), "%12");
        let text = format!("pmux report {REPORT_ID}. the report is at /tmp/x.md:12");
        let request = report_json(
            REPORT_ID,
            &text,
            &[("token", json!("abc")), ("bytes", json!(42))],
        );
        let reply = exchange(Some(request.as_bytes()), &state, &writer);
        let reply: Json = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply, json!({"ok": true, "emitted": true}));

        let (stdout, _) = stdout_of(&recorder);
        let emitted: Json = serde_json::from_str(stdout.trim_end()).unwrap();
        let meta = &emitted["params"]["meta"];
        assert!(meta.get("token").is_none());
        assert_eq!(meta["bytes"], "42");
    }

    // Row 3.3: the "Status request" scenarios.

    #[test]
    fn a_status_request_reports_the_arm_state_and_pane_with_no_emission() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(false, own_uid(), "%12");
        let request = b"{\"v\":1,\"kind\":\"status\"}\n";
        let reply = exchange(Some(&request[..]), &state, &writer);
        let reply: Json = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply, json!({"ok": true, "armed": false, "pane": "%12"}),);
        let (stdout, flushes) = stdout_of(&recorder);
        assert_eq!(stdout, "");
        assert_eq!(flushes, 0);
    }

    #[test]
    fn a_status_request_on_an_armed_server_reports_armed() {
        let recorder = Recorder::default();
        let writer = LineWriter::new(recorder.clone());
        let state = state(true, own_uid(), "%12");
        let request = b"{\"v\":1,\"kind\":\"status\"}\n";
        let reply = exchange(Some(&request[..]), &state, &writer);
        let reply: Json = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply["armed"], true);
        assert_eq!(reply["pane"], "%12");
    }

    // Row 3.3: the "Write failure" scenario, through an injected writer.

    #[test]
    fn a_failed_stdout_write_replies_write_failed() {
        let writer = LineWriter::new(Failing);
        let state = state(true, own_uid(), "%12");
        let text = format!("pmux report {REPORT_ID}. the report is at /tmp/x.md:12");
        let request = report_json(REPORT_ID, &text, &[]);
        let reply = exchange(Some(request.as_bytes()), &state, &writer);
        let reply: Json = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply, json!({"ok": false, "reason": "write-failed"}));
    }
}
