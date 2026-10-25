//! The MCP channel server mode: the pieces `--mcp` runs on stdio.
//!
//! A Claude Code channel server inside each parent session, so pmux can
//! push a report pointer into the session without typing into the terminal.
//! [`rpc`] parses one stdin line and builds each reply behind the shared
//! line writer (D1); the endpoint path and socket, and the `--mcp` loop
//! that assembles them, are separate modules in this change.
//!
//! The visibility echo:
//!`--mcp` is a binary mode, so the module surface is crate-only and `rpc`
//! is private — its API re-exports here, so items live behind one path.

mod rpc;

pub use rpc::{
    Incoming, LineWriter, METHOD_NOT_FOUND, RpcNotification, RpcRequest, error_reply, parse_line,
    request_reply,
};
