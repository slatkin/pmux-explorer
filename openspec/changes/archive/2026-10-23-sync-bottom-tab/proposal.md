# Proposal

## Why

Each tmux window runs its own sidebar instance, and each instance keeps its
own bottom tab. Focus a different agent and the tab jumps — to Activity when
a new agent starts, to Git for a shell pane, back to the last choice of the
pane you return to — so the same sidebar shows a different tab in window A
than in window B, and it keeps changing as you click around agents. The user
expects one bottom tab for the whole tmux server: whatever tab is selected
is the tab everywhere, and clicking agents does not move it.

## What Changes

- The bottom tab becomes a shared view setting: one tmux global option
  (`@sidebar_bottom_tab`) holds the selected tab, and every sidebar
  instance reads it on each refresh tick and shows it. Selecting Activity,
  Tasks or Git in one instance applies to every instance on the same tmux
  server within one refresh tick.
- The focus-driven auto-switch is removed. `auto_switch_tab` no longer
  sets Activity for an agent pane, Git for a shell pane, or a saved
  preference on focus change, so clicking a different agent never changes
  the tab.
- The per-pane tab memory (`PaneRuntimeState.tab_pref`) is removed with it:
  there is no per-pane preference to restore once the tab is shared.
- The tab only changes on an explicit user action — the cycle key, a click
  on a tab title, or an adoption of the shared value written by another
  instance.
- Scroll offsets stay per tab and per instance, as today: sharing the
  selected tab shares no scroll position.
- The three auto-switch defaults are replaced by a single startup default:
  an instance that has no shared value yet shows Activity, the tab
  `AppState::new` already starts from.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `shared-view-settings`: the bottom tab joins the shared settings (status
  filter, repo filter, cursor), and the requirement that each instance
  keeps its own bottom tab is replaced by the shared rule.
- `agent-tasks`: the Tasks tab is no longer local to each sidebar
  instance; its "stays local" requirement and scenario are replaced by the
  shared behaviour.

## Impact

- `src/tmux/options.rs`: new `SIDEBAR_BOTTOM_TAB` global option constant.
- `src/tmux/panes.rs`: the shared option added to `SHARED_OPTION_FIELDS`,
  so it rides the existing per-tick `display-message` read with no extra
  tmux process; the repo filter stays last for its space-tolerant parse.
- `src/state/global.rs`: `BottomTab` moves into `GlobalState` with
  `save_bottom_tab` and a `last_saved_bottom_tab` guard in `apply_all`,
  following the status filter's pattern.
- `src/state/tab.rs`: `auto_switch_tab`, `TabDecision`, `save_current_tab`
  and the per-pane restore logic are deleted; `next_bottom_tab` and
  `handle_bottom_tab_click` write through the shared path.
- `src/state/pane_runtime.rs` and `src/state.rs`: `tab_pref` field removed.
- `src/state/refresh.rs`: `refresh_activity_data` stops calling
  `auto_switch_tab`.
- `tests/state_tests.rs`, `tests/bottom_tests.rs`: the auto-switch and
  per-pane tab memory tests are replaced by shared-tab tests.
- `openspec/specs/shared-view-settings/spec.md` and
  `openspec/specs/agent-tasks/spec.md`: requirement changes recorded as
  deltas in this change.
- `docs/state-management.md`: `bottom_tab` moves from Local to Global
  scope, `tab_pref` rows are removed, and invariant 3 is rewritten.
