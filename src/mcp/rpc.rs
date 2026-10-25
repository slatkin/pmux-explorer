//! Hand-written JSON-RPC 2.0 for the MCP channel server mode.
//!
//! `--mcp` runs a stdio server where every stdin and stdout line is one
//! JSON-RPC message. The crate has no async runtime, so the protocol layer
//! is hand-written: this module parses one stdin line, builds the reply for
//! a request, and owns the shared line writer that serializes every stdout
//! line behind one mutex — each write is one line plus a flush, so a
//! channel notification never interleaves with a reply.
//!
//! The `initialize` reply pins the conversation to the earlier protocol
//! revisions: the server echoes one of the revisions it knows, falls back
//! to `2025-06-18` for everything else, and never answers `2026-07-28` —
//! a client on that revision will not register the server as a channel.

use serde_json::{Value, json};
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

/// JSON-RPC error code for a request with an unknown method.
pub const METHOD_NOT_FOUND: i64 = -32601;

/// The protocol revisions the server echoes back when the client asks for
/// one of them; everything else gets the default.
const ECHOED_PROTOCOL_REVISIONS: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-06-18"];

/// The `initialize` revision for every request outside
/// [`ECHOED_PROTOCOL_REVISIONS`], including a missing or non-string one.
const DEFAULT_PROTOCOL_REVISION: &str = "2025-06-18";

/// The server name in an `initialize` reply's `serverInfo`.
const SERVER_INFO_NAME: &str = "pmux-explorer";

/// A JSON-RPC request: a line that named a method and carried an id.
#[derive(Debug, Clone, PartialEq)]
pub struct RpcRequest {
    pub method: String,
    pub params: Value,
    pub id: Value,
}

/// A JSON-RPC notification: a line that named a method and carried no id,
/// so no reply is owed.
#[derive(Debug, Clone, PartialEq)]
pub struct RpcNotification {
    pub method: String,
    pub params: Value,
}

/// What one stdin line parsed into.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Request(RpcRequest),
    Notification(RpcNotification),
    /// The line is not a JSON-RPC request or notification: not JSON, not
    /// an object, no string `method`, or an `id` outside the JSON-RPC
    /// string, number and null shapes.
    Malformed,
}

/// Parses one stdin line into a request, a notification or [`Incoming::Malformed`].
///
/// Tolerance is deliberate: the `jsonrpc` field and the structure of
/// `params` are not validated, so a slightly loose client still gets a
/// reply. A line with trailing whitespace (including CRLF endings) parses
/// like the same line without it.
pub fn parse_line(line: &str) -> Incoming {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Incoming::Malformed;
    };
    let Some(object) = message.as_object() else {
        return Incoming::Malformed;
    };
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return Incoming::Malformed;
    };
    let params = object.get("params").cloned().unwrap_or(Value::Null);
    match object.get("id") {
        None => Incoming::Notification(RpcNotification {
            method: method.to_string(),
            params,
        }),
        Some(id) if is_json_rpc_id(id) => Incoming::Request(RpcRequest {
            method: method.to_string(),
            params,
            id: id.clone(),
        }),
        Some(_) => Incoming::Malformed,
    }
}

/// Whether `id` is one of the shapes JSON-RPC allows in an `id` field.
fn is_json_rpc_id(id: &Value) -> bool {
    matches!(id, Value::String(_) | Value::Number(_) | Value::Null)
}

/// Builds the reply for one parsed request.
///
/// The server answers `initialize` and `ping`; every other method gets
/// [`METHOD_NOT_FOUND`] with the request's id echoed back.
pub fn request_reply(request: &RpcRequest) -> Value {
    match request.method.as_str() {
        "initialize" => initialize_reply(&request.params, &request.id),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": request.id,
            "result": {},
        }),
        _ => error_reply(&request.id, METHOD_NOT_FOUND, "Method not found"),
    }
}

/// Builds a JSON-RPC error reply.
pub fn error_reply(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
}

/// Builds the `initialize` reply: only the channel capability, no
/// `instructions`, and the revision rule.
///
/// `protocolVersion` echoes the requested revision when the client asked
/// for one of [`ECHOED_PROTOCOL_REVISIONS`], and is
/// [`DEFAULT_PROTOCOL_REVISION`] otherwise — never `2026-07-28`, a revision
/// a v2 client runtime will not register as a channel. With no
/// `instructions` key the server adds nothing to the model's context
/// outside an event.
fn initialize_reply(params: &Value, id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": negotiated_revision(params),
            "capabilities": {
                "experimental": {
                    "claude/channel": {},
                },
            },
            "serverInfo": {
                "name": SERVER_INFO_NAME,
                "version": crate::VERSION,
            },
        },
    })
}

/// The `initialize` revision for the requested one, by the revision rule.
fn negotiated_revision(params: &Value) -> &'static str {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    ECHOED_PROTOCOL_REVISIONS
        .iter()
        .find(|revision| Some(**revision) == requested)
        .copied()
        .unwrap_or(DEFAULT_PROTOCOL_REVISION)
}

/// The shared writer every stdout line passes through.
///
/// All writes hold one mutex (design decision D1), so a channel
/// notification never interleaves with a reply. `W` is generic so the
/// socket handlers and tests can inject a writer, including one that
/// always fails.
#[derive(Debug)]
pub struct LineWriter<W> {
    inner: Arc<Mutex<W>>,
}

impl<W> LineWriter<W> {
    /// Wraps `inner` as the shared writer.
    pub fn new(inner: W) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl<W> Clone for LineWriter<W> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<W: io::Write> LineWriter<W> {
    /// Serializes `message` as one compact JSON line, then flushes, so
    /// the reader on the other end sees the line whole.
    ///
    /// # Errors
    /// When the underlying write or flush fails, and when the shared
    /// lock is poisoned by a panicked writer, as a `BrokenPipe`, so a
    /// report handler can stop instead of hanging the server loop.
    pub fn write_json(&self, message: &Value) -> io::Result<()> {
        let mut inner = self.lock()?;
        writeln!(inner, "{message}")?;
        inner.flush()
    }

    /// Locks the shared writer.
    ///
    /// # Errors
    /// Apoisoning means an earlier write panicked mid-line; the next
    /// write gets a `BrokenPipe` error instead of a panic, so the caller
    /// closes the connection cleanly.
    fn lock(&self) -> io::Result<MutexGuard<'_, W>> {
        self.inner.lock().map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "shared stdout writer poisoned by a panicked write",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request(method: &str, params: Value, id: Value) -> RpcRequest {
        RpcRequest {
            method: method.to_string(),
            params,
            id,
        }
    }

    fn initialize_request(revision: Value) -> RpcRequest {
        request("initialize", json!({"protocolVersion": revision}), json!(1))
    }

    // Row 1.1: parse one line.

    #[test]
    fn parses_a_request() {
        let incoming = parse_line(r#"{"jsonrpc": "2.0", "method": "ping", "id": 7}"#);
        assert_eq!(
            incoming,
            Incoming::Request(request("ping", Value::Null, json!(7)))
        );
    }

    #[test]
    fn parses_a_request_with_params_string_id_and_crlf() {
        let incoming = parse_line(
            "{\"jsonrpc\": \"2.0\", \"method\": \"initialize\", \"id\": \"1\", \
             \"params\": {\"protocolVersion\": \"2025-06-18\"}}\r\n",
        );
        assert_eq!(
            incoming,
            Incoming::Request(request(
                "initialize",
                json!({"protocolVersion": "2025-06-18"}),
                json!("1"),
            ))
        );
    }

    #[test]
    fn a_null_id_is_a_request_not_a_notification() {
        let incoming = parse_line(r#"{"jsonrpc": "2.0", "method": "initialize", "id": null}"#);
        assert_eq!(
            incoming,
            Incoming::Request(request("initialize", Value::Null, Value::Null))
        );
    }

    #[test]
    fn parses_a_notification() {
        let incoming = parse_line(r#"{"jsonrpc": "2.0", "method": "notifications/initialized"}"#);
        assert_eq!(
            incoming,
            Incoming::Notification(RpcNotification {
                method: "notifications/initialized".to_string(),
                params: Value::Null,
            })
        );
    }

    #[test]
    fn a_missing_jsonrpc_field_is_tolerated() {
        let incoming = parse_line(r#"{"method": "notifications/initialized"}"#);
        assert!(matches!(incoming, Incoming::Notification(_)));
    }

    #[test]
    fn parses_a_malformed_line() {
        for line in [
            "",
            "not json at all",
            "null",
            "[1, 2]",
            r#""a string""#,
            r#"{"jsonrpc": "2.0"}"#,
            r#"{"method": 123}"#,
            r#"{"id": 1}"#,
            r#"{"jsonrpc": "2.0", "method": "initialize", "id": {"x": 1}}"#,
        ] {
            assert_eq!(parse_line(line), Incoming::Malformed, "line: {line:?}");
        }
    }

    #[test]
    fn unknown_method_gets_method_not_found() {
        let reply = request_reply(&request("tools/list", Value::Null, json!(7)));
        assert_eq!(reply["error"]["code"], METHOD_NOT_FOUND);
        assert_eq!(reply["id"], json!(7));
        assert!(reply.get("result").is_none());
    }

    #[test]
    fn ping_reply_carries_an_empty_result() {
        let reply = request_reply(&request("ping", Value::Null, json!(2)));
        assert_eq!(reply["result"], json!({}));
        assert!(reply.get("error").is_none());
    }

    // Row 1.2: the initialize reply.

    #[test]
    fn initialize_declares_only_the_channel_capability() {
        let reply = request_reply(&initialize_request(json!("2025-06-18")));
        let capabilities = reply["result"]["capabilities"].as_object().unwrap();
        assert_eq!(capabilities.len(), 1);
        let experimental = capabilities["experimental"].as_object().unwrap();
        assert_eq!(experimental.len(), 1);
        assert_eq!(experimental["claude/channel"], json!({}));
    }

    #[test]
    fn initialize_has_no_instructions() {
        let reply = request_reply(&initialize_request(json!("2025-06-18")));
        assert!(reply["result"].get("instructions").is_none());
    }

    #[test]
    fn the_newest_revision_is_refused() {
        let reply = request_reply(&initialize_request(json!("2026-07-28")));
        assert_eq!(reply["result"]["protocolVersion"], "2025-06-18");
    }

    #[test]
    fn an_older_revision_is_echoed() {
        let reply = request_reply(&initialize_request(json!("2025-03-26")));
        assert_eq!(reply["result"]["protocolVersion"], "2025-03-26");
    }

    #[test]
    fn the_oldest_revision_is_echoed() {
        let reply = request_reply(&initialize_request(json!("2024-11-05")));
        assert_eq!(reply["result"]["protocolVersion"], "2024-11-05");
    }

    #[test]
    fn an_unknown_revision_falls_back_to_the_default() {
        let reply = request_reply(&initialize_request(json!("2024-01-01")));
        assert_eq!(reply["result"]["protocolVersion"], "2025-06-18");
    }

    #[test]
    fn a_missing_revision_falls_back_to_the_default() {
        let reply = request_reply(&request("initialize", json!({}), json!(1)));
        assert_eq!(reply["result"]["protocolVersion"], "2025-06-18");
    }

    // Row 1.1: the shared line writer.

    struct Recorder {
        buf: Vec<u8>,
        flushes: usize,
    }

    impl io::Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    #[test]
    fn each_write_is_one_line_and_one_flush() {
        let writer = LineWriter::new(Recorder {
            buf: Vec::new(),
            flushes: 0,
        });
        writer.write_json(&json!({"a": 1})).unwrap();
        {
            let recorder = writer.inner.lock().unwrap();
            assert_eq!(String::from_utf8_lossy(&recorder.buf), "{\"a\":1}\n");
            assert_eq!(recorder.flushes, 1);
        }
        let clone = writer.clone();
        clone.write_json(&json!({"b": "line\nbreak"})).unwrap();
        let recorder = writer.inner.lock().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&recorder.buf),
            "{\"a\":1}\n{\"b\":\"line\\nbreak\"}\n",
        );
        assert_eq!(recorder.flushes, 2);
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

    #[test]
    fn a_write_failure_is_reported_to_the_caller() {
        let writer = LineWriter::new(Failing);
        let error = writer
            .write_json(&json!({"a": 1}))
            .expect_err("write should fail");
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    struct PanickingOnWrite;

    impl io::Write for PanickingOnWrite {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            panic!("writer panicked mid-write")
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_poisoned_writer_lock_reports_an_error_instead_of_panicking() {
        let writer = LineWriter::new(PanickingOnWrite);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = writer.write_json(&json!({}));
        }));
        let _ignore = caught;
        let error = writer
            .write_json(&json!({}))
            .expect_err("poisoned lock should error");
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }
}
