# Proposal

## Why

The sidebar shows what a pi agent does (status, prompt, activity, tasks) but not what it thinks. The reasoning of a pi pane is visible today only in a follower pane that the johnny-cant-read extension opens next to that pane. That feature moves into pmux (the pmux change `thinking-feed`) and leaves johnny-cant-read, so the sidebar becomes the place where you read the last thought of the focused pi agent.

## What Changes

- A fourth bottom-panel tab, `Thought`, shows the **last thought** of the focused pane: the newest thinking block of its pi session, growing while the model thinks and kept after the block ends until the next block starts. It is not a stream and keeps no history of earlier blocks.
- The `Thought` label shows on the rule only while the focused pane runs pi. Other agents (Claude, Codex, OpenCode) get no Thought tab.
- When `Thought` is the selected tab but the focused pane is not pi, the panel shows the Activity tab. The shared `@sidebar_bottom_tab` value is not written, so the panel goes back to Thought when the focus returns to a pi pane, and other sidebars are not changed.
- The cycle key skips the hidden `Thought` tab. Tab clicks use the drawn label positions instead of fixed column ranges.
- The sidebar reads the thinking feed that pmux writes for each pi session, found through the pane's `@pane_session_id`. It reads on the existing 1 s refresh, and only while the Thought tab is drawn.
- When a pi pane's process exits and the dead-pane sweep clears its agent state, the sidebar deletes that session's thinking feed file. The sidebar owns feed cleanup — settled with pmux's `thinking-feed` change, which keeps removal a non-goal on its side.
- `docs/agent-protocol.md` gets the thinking feed contract (path rule, record shape, and cleanup rule), shared with pmux.

## Capabilities

### New Capabilities
- `thought-tab`: what the Thought tab shows for a pi pane — where the thinking feed is found, how its records make the last thought, when it is read, what shows when there is no thought, and when a dead pane's feed file is removed.

### Modified Capabilities
- `bottom-panel`: the rule shows a fourth label, `Thought`, only while the focused pane is pi; a click on that label selects it.
- `agent-tasks`: the bottom-tab cycle includes Thought when its label is shown and skips it when it is hidden.
- `shared-view-settings`: a selected but hidden Thought tab displays as Activity without a write to the shared tab value.

## Impact

- Code: `src/state.rs` (`BottomTab` gets a `Thought` variant and the `thought` label), `src/state/tab.rs` (cycle and click hitboxes), `src/ui/bottom.rs` and a new `src/ui/bottom/thought.rs`, a new state submodule for the reader, `src/state/refresh.rs` (the 1 s read), `src/tmux/query.rs` (the dead-pane sweep removes the feed file), and a new feed path helper next to `src/mcp/path.rs`'s runtime-dir rule.
- Docs: `docs/agent-protocol.md`, `docs/state-management.md`, `CLAUDE.md` (module list), README tab list.
- Shared option: `@sidebar_bottom_tab` can now hold `thought`. An older sidebar binary reads that value as Activity (its `from_label` fallback), so mixed versions stay safe.
- Dependency: the pmux change `thinking-feed` writes the feed. Without it, the tab shows its empty notice.
- Not affected: the hook protocol (no new event), the Claude/Codex/OpenCode adapters.
