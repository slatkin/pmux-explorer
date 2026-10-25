//! The MCP channel server mode: the pieces `--mcp` runs on stdio.
//!
//! A Claude Code channel server inside each parent session, so pmux can
//! push a report pointer into the session without typing into the terminal.
//! [`rpc`] parses one stdin line and builds each reply behind the shared
//! line writer (D1); [`path`] derives the endpoint path from the tmux
//! environment (D4), and [`endpoint`] binds the socket behind the
//! same-user gate (D5); [`connection`] serves each accepted connection —
//! the peer-uid gate, the request line, and the report, status and arm
//! handlers; [`probe`] owns the arm probe and its repeat gate (D3), and
//! [`client`] sends the `channel-ready` arm request to an endpoint. The
//! `--mcp` loop that assembles them is a later unit of this change.
//!
//! The visibility echo:
//!`--mcp` is a binary mode, so the module surface is crate-only and the
//! submodules are private — their APIs re-export here, so items live
//! behind one path.

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
    Incoming, LineWriter, METHOD_NOT_FOUND, RpcNotification, RpcRequest, error_reply, parse_line,
    request_reply,
};
