# Proposal

## Why

The sidebar is the only view that shows every agent pane in every tmux session at
once, and it knows three agents: Claude Code, Codex and OpenCode. The agent that
runs most on this machine is pi, and the sidebar does not show pi panes at all.
This is not a styling gap. `parse_pane_fields_with_processes` drops any pane
whose `@pane_agent` label it does not recognise, so a pi pane is invisible
rather than unstyled, and no hook output can change that.

Upstream is not going to fix it soon. `hiroppy/tmux-agent-sidebar` has had no
commits from a human for months; the recent history is dependency bumps and
bot-driven pull requests. A pi adapter is a small change, but a small change
that waits on an outside schedule is a change that never lands.

So we fork. `slatkin/tmux-agent-sidebar` becomes the working repository;
`upstream` stays a remote so later upstream work can be pulled in by hand. The
project is MIT, and the fork keeps the licence and the attribution.

The fork is worth more than the adapter. In a fork we may reshape the agent
layer instead of patching it: the event contract becomes a documented producer
protocol, so a producer can be written in any language against a written
specification rather than by reading the Rust adapter.

## What Changes

- pi becomes a first-class agent in the sidebar: `AgentType::Pi` with the label
  `pi`, its own colour (`@sidebar_color_agent_pi`), a `pi` event adapter, the
  pi tool names normalised into the canonical vocabulary, and pi in the
  process-based dead-pane sweep. A pi pane that waits for a human is marked
  the same way a Claude pane is, from pi's `ui_prompt` events.
- The event contract becomes a documented producer protocol. Today the contract
  is implicit in the Claude adapter's field names. After this change
  `docs/agent-protocol.md` states, per event, the JSON fields a producer must
  send and what the sidebar does with them. A producer can then be written
  against the document alone.
- The `setup` command learns `pi`. For a hook-config agent it merges a JSON
  settings file; for a plugin-style agent it prints the producer contract
  instead. OpenCode already needed the second shape; pi needs it too.
- The producer for pi lives in pmux, not in this repository. pmux already ships
  a pi extension that loads in every pi session, so it reaches every pi pane on
  the machine, including the ones pmux did not start. This repository defines
  the contract; pmux implements it, in pmux, under its own change.

## Capabilities

### New Capabilities

- `pi-agent-support`: the sidebar recognises pi panes, renders them with their
  own identity, and accepts pi events through the documented protocol.

### Modified Capabilities

None. This repository has no specs yet; this change adds the first.

## Impact

- `src/tmux/types.rs`: a `PI_AGENT` constant, an `AgentType::Pi` variant, and
  its `from_label` and `as_str` arms.
- `src/ui/colors.rs`: an `agent_pi` colour and the `@sidebar_color_agent_pi`
  option that feeds it.
- `src/adapter/pi.rs`: a new adapter, shaped after the OpenCode adapter, that
  maps the protocol payloads into `AgentEvent` and normalises pi's lowercase
  tool names and argument keys.
- `src/adapter/mod.rs` and `src/event/adapter.rs`: the module and the
  `resolve_adapter` arm.
- `src/tmux/query.rs`: pi in the process-based dead-pane sweep, and the
  permission-mode read, which today applies to Claude alone.
- `src/cli/setup/mod.rs`: the `pi` entry, which prints the producer contract
  instead of merging a settings file.
- `docs/agent-protocol.md`: the protocol, new.
- Tests: the adapter's parse arms and drift test, the `AgentType` label round
  trip, the setup output.
- The producer is not in this change. pmux carries it, under the change
  `2026-10-20-sidebar-events` in the pmux repository.
- Not in this change: the task-progress row, the subagent tree, and anything
  inside pi itself. The reasons are in the design.
