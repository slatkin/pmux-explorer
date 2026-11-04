# Design

## Context

See proposal.md for why. The facts below were observed on 2026-11-04:

- `pi-claude-bridge` 0.9.3 starts Claude Code through the Agent SDK with
  `env: { ...process.env, ... }`, so the child inherits `TMUX_PANE`. Its
  provider path leaves `settingSources` at the default, so user plugins load,
  and the `pmux-explorer@slatkin` plugin hooks (`hooks/hooks.json`) run.
- A live bridge child (`claude`, parent `pi`) had `TMUX_PANE=%81` and
  `PI_CODING_AGENT=true` in `/proc/<pid>/environ`. Pane `%81` showed
  `agent=claude`, a Claude Code session id, and `@pane_subagent_kind=spawned`.
- `cmd_hook` (`src/cli/hook.rs`) already has early `return 0` exits for an
  empty agent or event name, an unknown agent, and an empty `TMUX_PANE`. All
  of them come before the stdin read.

## Goals / Non-Goals

**Goals:**

- One gate that covers every Claude event kind, including the ones that are
  not routed through `transition` (`session-end`, `activity-log`,
  notifications).
- A gate that unit tests can check without changes to the process
  environment.

**Non-Goals:**

- Detecting the bridge itself. The gate keys on "Claude Code under pi", not on
  the bridge, so any wrapper that pi starts is covered.

## Decisions

**D1. The gate is in `cmd_hook`, not in `hook.sh`.** The gate is in the Rust
entry, after the agent name is known and before `tmux_pane()` and the stdin
read. Alternative: `[ -n "$PI_CODING_AGENT" ] && exit 0` in `hook.sh`. Rejected:
`hook.sh` is a late-binding wrapper that must stay free of event logic (its
header says so), it cannot be unit tested, and it would also apply to Codex,
which uses the same wrapper.

**D2. A pure predicate holds the rule.** A function such as
`ignored_under_pi(agent_name: &str, pi_coding_agent: Option<&str>) -> bool`
returns true when `agent_name` is `CLAUDE_AGENT` and the value is present and
not empty. `cmd_hook` passes `std::env::var("PI_CODING_AGENT").ok()`. The tests
call the predicate with fixed values. Alternative: tests that set the
variable. Rejected: `std::env::set_var` is `unsafe` in edition 2024 and races
with tests on other threads.

**D3. Present and not empty, not equal to `true`.** pi writes `true` today.
The bridge README tells hook authors to test `-n "$PI_CODING_AGENT"`, so
"not empty" matches the documented contract and does not depend on the exact
value.

**D4. Only `claude`.** The Claude Code plugin is the only producer that pi's
children load with no action from the user. Codex and OpenCode under pi are a
separate case and are not part of this change.

**D5. The stdin payload is not read.** The gate exits before
`read_stdin_json`, like the existing early exits. Claude Code writes the
payload and does not wait for the hook (see `docs/agent-protocol.md`, "A
producer must not wait for the answer").

## Risks / Trade-offs

- [A Claude Code session that the user starts by hand inside a pi pane is not
  tracked] → Intended. It shares the pi pane, and its events would clobber the
  pi state.
- [A future pi release stops setting `PI_CODING_AGENT`] → The pane shows
  `claude` again, as it does today. Nothing else breaks. The doc paragraph
  names the variable, so the cause is easy to find.
- [AskClaude's inner tool calls stay invisible] → Accepted. The tool is
  opt-in and marked deprecated by the bridge.

## Migration Plan

None. A pane that shows `claude` now is repaired by the next pi event, because
pi's events write `@pane_agent=pi` and the pi session id.
