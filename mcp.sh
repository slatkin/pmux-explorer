#!/usr/bin/env bash
# Thin wrapper: delegates to the Rust binary with `--mcp`. Loaded by
# Claude Code as the plugin manifest's inline `mcpServers.pmux` command
# (`${CLAUDE_PLUGIN_ROOT}/mcp.sh`), which starts the MCP channel server
# inside the session.
#
# The binary lookup is shared with `hook.sh` (`bin-lookup.sh`): late
# binding, so the binary is resolved fresh on every server start and
# the user can move or rebuild it without regenerating any config.
#
# One deliberate difference from `hook.sh`: when no binary is found,
# this script writes a message to stderr and exits 1. A failed MCP
# server shows up in Claude's `/mcp` panel, while a silent exit 0 would
# look like a server that closed. The last lookup step is a
# `command -v pmux-explorer` check, so "no binary" is detected here and
# not left to a bare `PATH` name that fails later.
. "$(dirname "$0")/bin-lookup.sh" || {
  echo "pmux-explorer: no binary found for the MCP channel server (looked in $LOOKUP_DIR/bin, $LOOKUP_DIR/target/release, $TPM_DIR/bin, $TPM_DIR/target/release, and on PATH)" >&2
  exit 1
}
exec "$BIN" --mcp
