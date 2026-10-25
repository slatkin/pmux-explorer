#!/usr/bin/env bash
# Thin wrapper: delegates to the Rust binary with `--mcp`. Loaded by
# Claude Code as the plugin manifest's inline `mcpServers.pmux` command
# (`${CLAUDE_PLUGIN_ROOT}/mcp.sh`), which starts the MCP channel server
# inside the session.
#
# It copies the `hook.sh` binary lookup: late binding, so the binary is
# resolved fresh on every server start and the user can move or rebuild
# it without regenerating any config.
#
# One deliberate difference from `hook.sh`: when no binary is found,
# this script writes a message to stderr and exits 1. A failed MCP
# server shows up in Claude's `/mcp` panel, while a silent exit 0 would
# look like a server that closed. The last lookup step is a
# `command -v pmux-explorer` check, so "no binary" is detected here and
# not left to a bare `PATH` name that fails later.
PLUGIN_DIR="$(cd "$(dirname "$0")" && pwd -P)"
# Fallback location used when this script is executed from a Claude Code
# plugin install (e.g. `${CLAUDE_PLUGIN_ROOT}/mcp.sh`). The plugin cache
# never contains the binary, so hop over to the tmux plugin directory
# where TPM placed it.
TPM_DIR="$HOME/.tmux/plugins/pmux-explorer"
if [ -x "$PLUGIN_DIR/bin/pmux-explorer" ]; then
  BIN="$PLUGIN_DIR/bin/pmux-explorer"
elif [ -x "$PLUGIN_DIR/target/release/pmux-explorer" ]; then
  BIN="$PLUGIN_DIR/target/release/pmux-explorer"
elif [ -x "$TPM_DIR/bin/pmux-explorer" ]; then
  BIN="$TPM_DIR/bin/pmux-explorer"
elif [ -x "$TPM_DIR/target/release/pmux-explorer" ]; then
  BIN="$TPM_DIR/target/release/pmux-explorer"
elif command -v pmux-explorer &>/dev/null; then
  BIN="pmux-explorer"
else
  echo "pmux-explorer: no binary found for the MCP channel server (looked in $PLUGIN_DIR/bin, $PLUGIN_DIR/target/release, $TPM_DIR/bin, $TPM_DIR/target/release, and on PATH)" >&2
  exit 1
fi
exec "$BIN" --mcp
