# Tasks

## 1. Gate

- [x] 1.1 Search the user's orchestration prompts and launch aliases for `channel-ready` and `development-channels` (`rg -n "channel-ready|development-channels"` over `~/Dev/agents` and the shell config); verify the result is empty, or record each hit and its owner in the change before step 3
- [x] 1.2 Confirm design decision D3 (no `channel-ready` tombstone) with the user; verify the answer is recorded in `design.md`

## 2. Plugin wiring

- [x] 2.1 Remove the `mcpServers` key from `.claude-plugin/plugin.json`, delete `mcp.sh` and `tests/plugin_manifest_tests.rs`; verify `cargo test` passes and `rg -n "mcp.sh|mcpServers" --glob '!openspec/**'` finds only `bin-lookup.sh` comments
- [x] 2.2 Edit the two comment lines of `bin-lookup.sh` that name `mcp.sh` so they describe `hook.sh` alone; verify `rg -n "mcp" bin-lookup.sh` is empty and `tests/plugin_hooks_tests.rs` passes

## 3. CLI and module

- [x] 3.1 In `src/cli/mod.rs` remove the `--mcp` arm and its comment, the `channel-ready` arm, `cmd_channel_ready`, `run_channel_ready`, the `mcp` import, and the channel-ready tests (`the_channel_ready_dispatch_maps_to_run` included); verify `cargo build` passes
- [x] 3.2 Delete `src/mcp/`, `src/test_support.rs` and `tests/mcp_tests.rs`, and the two `mod` lines in `src/lib.rs`; verify `cargo build`, `cargo test` and `cargo clippy` pass with no unused-dependency or dead-code warning
- [x] 3.3 Add a `cli::run` test that `channel-ready` and `--mcp` are no longer subcommands (`run(&[..])` returns `None`), pinning D3; verify `cargo test cli::` passes

## 4. Docs and specs

- [x] 4.1 Remove the "The channel endpoint" section from `docs/agent-protocol.md` and any sentence elsewhere in the file that points to it; verify `rg -n -i "channel" docs/agent-protocol.md` shows only the unrelated sentence in the introduction
- [x] 4.2 Remove README step "4. Hand Claude a channel (optional)"; verify no link to `#the-channel-endpoint` remains (`rg -n "channel-endpoint|the-channel" README.md AGENTS.md docs website`)
- [x] 4.3 In `AGENTS.md` (`CLAUDE.md` is a symlink to it) remove the `mcp/` entry, `channel-ready` from the subcommand list, and the `--mcp` clause; verify `rg -n -i "mcp|channel-ready" AGENTS.md` is empty
- [ ] 4.4 Archive the change with the spec delta so `openspec/specs/channel-endpoint/` is removed; verify `openspec validate remove-mcp-channel` passes

## 5. Verification

- [x] 5.1 Waived (user, 2026-10-28): the user does not launch Claude with `--dangerously-load-development-channels`, so R1 does not apply to their setup. A probe run is recorded in `design.md` R1. Original task: in a throwaway tmux pane, start `claude --dangerously-load-development-channels plugin:pmux-explorer@slatkin` against the new build, read what the session shows with `tmux capture-pane`, and record in `design.md` R1 what Claude Code does; if it cannot be driven this way, record a waiver with the reason instead
- [x] 5.2 `cargo fmt --check`, `cargo clippy`, `cargo test`, then `cargo build --release`; verify all four exit 0 and `pmux-explorer --version` still prints the version
