# Tasks

Every task ends with a verification. The gates at the end are the repository's
own: `cargo fmt --all -- --check`, `cargo clippy` and `cargo test`.

## 1. The agent type and its colour

- [x] 1.1 In `src/tmux/types.rs`, add `pub const PI_AGENT: &str = "pi";`, add a `Pi` variant to `AgentType`, and add its arms to `from_label`, `as_str` and `label`. Keep the `Unknown` variant last and keep its `#[allow(dead_code)]`. Verify: `cargo test` passes, and a new test in `src/tmux/types.rs` asserts that `AgentType::from_label("pi")` is `Some(AgentType::Pi)` and that `AgentType::Pi.as_str()` is `"pi"`.
- [x] 1.2 In `src/ui/colors.rs`, add an `agent_pi` field to the theme, read it from the `@sidebar_color_agent_pi` option beside the other agent colours, give it a default in `ColorTheme::default()`, and add the `AgentType::Pi` arm to `agent_color`. Verify: `cargo test` passes, and a new test asserts that the theme's pi colour is the default when the option is unset and the option's colour when it is set.
- [x] 1.3 In `src/tmux/options.rs`, add the `SIDEBAR_COLOR_AGENT_PI` option key and export it from `src/tmux.rs`. Verify: `cargo test` passes and `cargo clippy` reports no warning.

## 2. The pi adapter

- [x] 2.1 Add `src/adapter/pi.rs`, shaped after `src/adapter/opencode.rs`. It SHALL map `session-start`, `session-end`, `user-prompt-submit`, `notification`, `stop`, `stop-failure` and `activity-log` onto the internal events, reading the same field names the Claude adapter reads. The `notification` mapping SHALL set the waiting status and the attention flag, the way the Claude adapter's does. Verify: `cargo test` passes, and a new test asserts that a `notification` event with a `wait_reason` becomes `AgentEvent::Notification` with that reason and `meta_only` false.
- [x] 2.2 In `src/adapter/pi.rs`, add `normalize_tool_name`, mapping `read` to `Read`, `edit` to `Edit`, `write` to `Write`, `bash` to `Bash`, `powershell` to `PowerShell`, `grep` to `Grep`, `find` and `ls` to `Glob`, and leaving every other name as it is. Add `normalize_tool_input`, adding the key `file_path` beside `path` for `Read`, `Edit` and `Write`. Verify: a new test asserts that a `read` event with `tool_input` `{"path": "/a/b/c.rs"}` produces the tool name `Read` and the label `c.rs`.
- [x] 2.3 Add `pub mod pi;` to `src/adapter/mod.rs` and the `pi` arm to `resolve_adapter` in `src/event/adapter.rs`. Verify: `cargo test` passes, and a new test asserts that `resolve_adapter("pi")` returns an adapter that parses `user-prompt-submit` into `AgentEvent::UserPromptSubmit` with the agent `pi`.
- [x] 2.4 Add the drift test that the OpenCode and Claude adapters have, for the pi adapter's `HOOK_REGISTRATIONS` table. pi has no hook configuration file, so the table SHALL be empty, and the drift test SHALL assert that the table is empty and that every `parse` arm is reachable from the protocol's event names. Verify: `cargo test` passes.

## 3. The pane query path

- [x] 3.1 In `src/tmux/query.rs`, add `AgentType::Pi` to the dead-pane sweep, so that a pi pane whose process is gone is cleared. The sweep matches the process name against the agent label, and the pi process is called `pi`. Verify: a new test asserts that a pi pane with no live pi process is dropped, and that one with a live pi process is kept.
- [x] 3.2 Leave the permission-mode read as it is: the badge stays a Claude concept, and the pi adapter reports the default mode. Verify: `cargo test` passes and the new test in 3.1 asserts that a pi pane carries the default permission mode.

## 4. The setup command and the protocol document

- [x] 4.1 In `src/cli/setup/mod.rs`, add a `pi` entry that prints the producer contract: the path of the binary, the event names, the JSON fields of each event, and the name of the extension that produces them. It SHALL change no configuration file. Verify: a new test runs the setup for `pi` against a fake home directory and asserts that the output names `user-prompt-submit` and that no file under the fake home changed.
- [x] 4.2 Write `docs/agent-protocol.md`: for every event of the protocol, the JSON fields, whether a field is required, and what the sidebar does with it. Start from the field names the Claude adapter reads, and mark the fields that only some agents send. Verify: the document names all seven events of the pi adapter, and the setup output of 4.1 matches the document's field list.
- [x] 4.3 In `website/src/content/docs`, add a page for the pi agent, and link it from the agent index. The page states what a pi pane shows and, in as many words, what it does not show: no attention badge, no permission mode, no task progress, no subagent tree. Verify: the site builds with `npm run build` in `website/`, or with the repository's own site command if the package scripts differ.

- [x] 4.4 The fork installs itself, not the upstream. `install-wizard.sh` downloads the release binary from `slatkin/tmux-agent-sidebar`, the Claude Code marketplace and plugin metadata name the fork, and the README's TPM and plugin install lines point at the fork. The upstream's MIT licence and copyright stay in `LICENSE`, and the README says in as many words that this is a fork. Verify: `rg -n "hiroppy/tmux-agent-sidebar" install-wizard.sh README.md` finds no plugin path, and `rg -n "hiroppy" LICENSE` finds the copyright.

## 5. Gates

- [x] 5.1 Run `cargo fmt --all`, then `cargo fmt --all -- --check`, `cargo clippy --all-targets` and `cargo test`. Verify: the formatter is clean, clippy reports the same four warnings as the pristine checkout (in `desktop_notification.rs`, `group.rs` and `query.rs`) and no new one, and the test suite passes. Three `group::` tests fail inside a git worktree, because they compare the git root with the manifest directory; they pass in the main checkout.
- [x] 5.2 Run the complexity check on the changed Rust files: `rust-code-analysis-cli -p src/adapter/pi.rs -p src/tmux/types.rs -p src/tmux/query.rs -p src/ui/colors.rs -p src/cli/setup/mod.rs -m -O json > rca.json`, rank it with `python3 .agents/skills/rust-complexity/scripts/rank_complexity.py rca.json`, and delete `rca.json`. Verify: no changed function has a cyclomatic sum over 25, and the cognitive column of the worst function is under 25.
- [x] 5.3 Build the release binary and check it against a live pi pane: `cargo build --release`, then run the binary with `hook pi user-prompt-submit` and a JSON payload on stdin in a pane, and confirm that the sidebar shows the pi row with the prompt. Verify: the pane appears in the sidebar with the `pi` colour and the prompt text, and the same works for `activity-log` with a pi tool name.
