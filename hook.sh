#!/usr/bin/env bash
# Thin wrapper: delegates to the Rust binary. Called by Claude Code /
# Codex hooks (settings.json).
#
# Why this file exists even though `pmux-explorer setup` can emit
# absolute binary paths:
#
# 1. Late binding. settings.json only needs to know where `hook.sh`
#    lives. The actual binary is resolved fresh on every hook fire (the
#    shared `bin-lookup.sh` cascade), so the user can move or rebuild
#    the binary (bin/ ↔ target/release/, relocate the plugin dir, swap
#    install methods) without having to regenerate their agent config.
#    Without this indirection, any setup-generated path becomes a stale
#    snapshot the moment the binary moves.
#
# 2. Graceful absence. If the binary is missing — during a rebuild,
#    mid-uninstall, or on a fresh clone before `cargo build` — this
#    script exits 0 silently, so the agent session never sees a hook
#    failure. A direct binary invocation would surface "no such file"
#    errors into the user's workflow.
#
# Keep this wrapper small and side-effect-free. Any logic that needs to
# know event semantics belongs in the Rust `hook` subcommand.
. "$(dirname "$0")/bin-lookup.sh" || exit 0
exec "$BIN" hook "$@"
