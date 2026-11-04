# Tasks

## 1. The gate

- [x] 1.1 Add the pure predicate (design D2, D3, D4) to `src/cli/hook.rs`: true only for the agent `claude` with a present, non-empty `PI_CODING_AGENT` value. Add a `#[cfg(test)]` module with one test per spec case: `claude` + `"true"` is ignored; `claude` + any other non-empty value is ignored; `claude` + `""` and `claude` + absent are not ignored; `pi`, `codex` and `opencode` + `"true"` are not ignored. Verify with `cargo test ignored_under_pi`.
- [x] 1.2 Call the predicate in `cmd_hook` with `std::env::var("PI_CODING_AGENT").ok()`, after the agent name check and before `tmux_pane()` and the stdin read (design D1, D5), returning 0. Add a short comment that names pi's Claude Code children as the cause. Verify with `cargo test` and `cargo clippy`.
- [x] 1.3 In `docs/agent-protocol.md`, section "The call", after the paragraph "The pane is taken from the environment.", add a paragraph that states the rule: a `claude` event whose environment has a non-empty `PI_CODING_AGENT` is ignored, because it comes from a Claude Code process that pi started in its own pane (for example the claude-bridge provider), and pi's own producer already reports that pane. Verify that the paragraph names the variable and the `claude` agent, and that the doc says nothing different elsewhere (`grep -n PI_CODING_AGENT docs/agent-protocol.md`).

## 2. Integration check

- [x] 2.1 Run `cargo fmt --check`, `cargo clippy` and `cargo test`; all pass. Run `cargo build --release`. Then check the live behavior with tmux: run `TMUX_PANE=<a pi pane id> PI_CODING_AGENT=true ./target/release/pmux-explorer hook claude session-start <<< '{"session_id":"x","cwd":"/tmp"}'` and confirm with `tmux show-options -p -t <pane> @pane_agent` that the value did not change. Record the commands and output in the commit message or the task note.

## Workflow follow-up

- Archive the change after review, which adds the requirement to `openspec/specs/pi-agent-support/spec.md`.
