# State Management Architecture

## State Scope & Update Frequency

Every piece of state belongs to one of three scopes: **Global** (shared across all sidebar instances via tmux variables), **Per-pane** (keyed by tmux pane ID), or **Local** (single sidebar process only). The table below shows where each field lives, how often it updates, and what triggers the update.

### Global State (synced via tmux global variables)

Stored in `GlobalState`. Written to tmux on change, with the cursor save
debounced briefly so selection changes do not block redraw/input handling;
read from tmux on every refresh tick.

| Field | Tmux Variable | Update Trigger | Description |
|-------|--------------|----------------|-------------|
| `status_filter` | `@sidebar_filter` | User input (left/right key) | Active status filter (All/Busy/Waiting/Idle/Error) |
| `selected_pane_row` / `cursor_pane_id` | `@sidebar_cursor` | User input (j/k key, agent-list click) through `select_row`; a tmux focus change through `adopt_focused_pane`; tmux write flushed after a short debounce | The agent list's one active row: the row it draws on and the pane it names. The option carries the pane id; the row is a cache re-derived from it by `rebuild_row_targets()`, so a filter change, a repo reorder or a closing pane moves the row under the same pane. The cursor follows the tmux focus (the focused pane is the active agent), and that row wears both marks — the `┃` grabber and the selection background — whichever section owns the keyboard |
| `repo_filter` | `@sidebar_repo_filter` | User input (repo popup) | Repository filter (All or specific repo) |
| `bottom_tab` | `@sidebar_bottom_tab` | On user tab switch (cycle key / tab-title click); tmux write retried when rejected | Selected bottom tab, shared server-wide; each instance adopts a peer's selection on its next refresh tick. The drawn tab run shows one Nerd Font glyph per tab — centered on the column's rule, right-aligned on the strip's tab bar — wrench for Activity, thought-bubble for Thought, list for Tasks, git-branch for Git — separated by ` │ ` delimiters; the stored values are `activity`, `thought`, `tasks` and `git`, and the Activity tab keeps that variant name while the Tools glyph draws for it. `thought` is drawn only while the focused pane runs pi — a stored Thought falls back to Activity for any other agent, without a write, so the panel returns to Thought when the focus comes back to a pi pane |
| `bottom_percent` | `@sidebar_bottom_height` | User input (rule drag release); tmux write retried when rejected | Bottom panel height as a percent of the sidebar height (0 hides the panel; unset, empty, or an out-of-range value gives 30); shared server-wide, each instance adopts a peer's value on its next refresh tick (the percent's `SaveLatch` instance follows the same pattern) |
| `tab_percent` | `@sidebar_tab_width` | User input (strip divider drag release); tmux write retried when rejected | Strip tab-panel width, divider included, as a percent of the strip width (0 hides the divider and the tab panel; unset, empty, or an out-of-range value gives 40); shared server-wide, each instance adopts a peer's value on its next refresh tick (the tab percent's `SaveLatch` instance follows the same pattern); every instance parses and adopts it — a column and a strip on the same server share the one option — and only a strip drag writes it |
| `split_tab_a` | `@sidebar_split_tab_a` | On user tab switch in pane A of a split strip (cycle key / tab-title click); tmux write retried when rejected | Pane A's tab in a split strip, shared server-wide among split strips like `bottom_tab`; only `activity` and `git` are selectable. A split selection never writes `@sidebar_bottom_tab`, so a column and a split strip keep their own tabs |
| `split_tab_b` | `@sidebar_split_tab_b` | On user tab switch in pane B of a split strip (cycle key / tab-title click); tmux write retried when rejected | Pane B's tab in a split strip, shared server-wide among split strips; only `tasks` and `thought` are selectable. As with `bottom_tab`, a stored `thought` draws as Tasks while the focused pane is not pi, without a write |
| `split_agents_percent` | `@sidebar_split_agents_width` | User input (split divider 1 drag release); tmux write retried when rejected | Agent-panel width in a split strip as a percent of the strip width (unset, empty, or an out-of-range value gives 33); shared server-wide, each split strip adopts a peer's value on its next refresh tick, held back while a strip drag stands |
| `split_tab_b_percent` | `@sidebar_split_tab_b_width` | User input (split divider 2 drag release); tmux write retried when rejected | Pane B's width with divider 2 as a percent of the strip width (unset, empty, or an out-of-range value gives 33); shared server-wide with the same drag block, and pane A takes the rest of the strip |

| `width_lock` | `@sidebar_width_lock` | User input (`w`, the lock glyph a column draws, or the strip's lock control at column 3 of its control row) or a peer's toggle adopted on refresh; tmux write retried when rejected | Shared on/off switch for the sidebar size lock, server-wide — one flag governing both axes; while `on` every docked sidebar converges on `shared_docked_width` (a column) or `shared_docked_height` (a strip) within one refresh tick. Enabling seeds only the enabling instance's own axis and clears the other axis's shared value, so peers on that axis keep their sizes until one of them publishes |
| `shared_docked_width` | `@sidebar_docked_width` | Seeded by the enabling column, then published on an observed docked-width change while locked; tmux write retried when rejected | Shared docked width in columns (`None` until seeded); adopting columns clamp it to half the window and never below the `@sidebar_min_width` floor |
| `shared_docked_height` | `@sidebar_docked_height` | Seeded by the enabling strip, then published on an observed docked-height change while locked; tmux write retried when rejected | Shared docked height in absolute rows (`None` until seeded); adopting strips clamp it to half the window and never below the 4-row strip floor (`DEFAULT_MIN_STRIP_HEIGHT` in `src/state/width.rs`). A strip enabling the lock seeds `max(live rows, floor)`; a floating strip seeds its captured docked height, and enables unseeded when nothing was captured or the probe failed |
| `min_width` | `@sidebar_min_width` | Adopted verbatim every refresh tick — the sidebar never writes it | Parsed width floor (`None` when unset or non-numeric); readers fall back to the default floor |

Each synced field is guarded so a failed write never loses the user's
choice: the status filter, the bottom tab, the bottom percent, the strip
tab percent, the two split tabs, the two split widths, the width lock and the two shared docked sizes each hold
one `SaveLatch<T>` instance (`filter_latch`, `tab_latch`,
`percent_latch`, `tab_percent_latch`, `split_tab_a_latch`,
`split_tab_b_latch`, `split_agents_percent_latch`,
`split_tab_b_percent_latch`, `lock_latch`, `width_latch`,
`height_latch` in `GlobalState` — one retry mechanism, written once and
shared by all eleven). The cursor and repo filter keep plain
`last_saved_cursor` / `last_saved_repo_filter` values instead.

A `SaveLatch` remembers the last value tmux successfully accepted
(`last_saved`) and whether tmux refused the last write
(`save_pending`). While a latch is pending, tmux still holds the
stale value the refused write was meant to replace, so `apply_all`
skips adopting the shared value that tick — adopting it would revert
the user's choice — and it also skips a shared value equal to
`last_saved`, so a local change whose write has not landed yet stays
put. `retry_pending_saves` is the one retry surface: it retries every
pending latch once per refresh tick, before the tick reads the shared
options, so a successful retry is what that tick reads back, while a
retry tmux refuses again leaves the latch set for the next tick. The
retries run together because retrying only some of the latches would
let the rest drift from the shared values.

The instances differ only at their edges. `filter_latch` and
`tab_latch` parse their single option and adopt nothing while it is
absent. `percent_latch` and `tab_percent_latch` parse the whole option
map instead: an unset, empty or invalid value resolves to the default
(`DEFAULT_BOTTOM_PERCENT = 30` and `DEFAULT_TAB_PERCENT = 40`) and is
adopted; the two split-width latches parse the same way with
`DEFAULT_SPLIT_PERCENT = 33`. The two percent latches also carry an extra adoption gate —
`apply_all` receives `split_drag_axis` (the `LayoutKind` of the
in-progress `AppState::split_drag`, if any) and holds that axis's
shared value unadopted while the drag stands — a column drag blocks
only `@sidebar_bottom_height` adoption, a strip drag only
`@sidebar_tab_width` — and, on the same gate, the two split widths —
and the other axis's value adopts as usual — so
the tick cannot move the rule or the divider out from under the mouse;
the ticks after the release adopt the server value again.

The two docked-size latches parse their option the same way, with one
extra rule: a numeric value adopts its columns or rows, a non-numeric
value adopts nothing, and an absent key adopts `None` — so clearing the
option after an earlier value unsets it on every peer instead of leaving
a stale cached value behind.

(`min_width` is read-only, so it needs no guard.) Each docked-size latch
gates the size guard's adopt side on its own axis (`src/app/width_guard.rs`):
while the width latch's `save_pending` flag is set a column must not be
resized back toward the stale shared width tmux still holds, and while
the height latch's flag is set a strip must not be resized back toward
the stale shared height.

### Per-pane State (keyed by pane ID)

Written through the pane-write seam, read by `query_sessions()` every **1 second**.

Every write to an agent pane's `@pane_*` options goes through a `PaneStore`
(`src/pane_store.rs`; the tmux adapter lives in `src/tmux/pane_store.rs`): the
caller reads one `PaneSnapshot` of the four rule options (`@pane_status`,
`@pane_bg_cmd`, `@pane_subagents`, `@pane_started_at`), builds a `PaneWrites`
list with the pure `transition(&PaneSnapshot, &AgentEvent, &Reported)` in
`src/pane_state.rs`, and applies the list as one batch. The store is created
once at the `hook` entry and passed down; the sidebar's sweeps and wipes take
it as an argument the same way. The rules the transition owns:

- **One event, one batch** — a `PaneWrites` list becomes one tmux invocation
  of `;`-joined `set` commands, so the poll reads the pane from before the
  event or from after it, never from between; an empty list applies as a no-op
- **Reported fields** — an option clears only when the event reports its
  field: a reported field with an empty value clears, an unreported field is
  left alone (each adapter declares the set per event)
- **The subagent guard** — while `@pane_subagents` is non-empty, the identity
  writes (agent label, permission mode, session id, cwd, worktree pair, the
  task reset and clear, the worktree-remove teardown, `SessionEnd`) are
  skipped; status, attention and prompt writes are not
- **One-field encoding** — `apply` encodes every value with
  `sanitize_tmux_value`, so a newline, a carriage return or a `|` in a value
  reads back as one field of the pane list

What the transition does not own stays with the handlers, which run their
non-option effects after `apply`: the activity log file, the desktop
notifications and their dedup stamps and run id (the stamp and run-id writes
go through the same store as their own commands), the task store, and the
pending teardown markers. `SessionEnd`, `SubagentStart`, `SubagentStop` and
`WorktreeRemove` keep their guarded plain-list and read-modify-write paths,
expressed as `get` + `apply`.

Each pane's runtime data is split into two buckets:

| Source | Update Trigger | Description |
|--------|----------------|-------------|
| tmux pane options | Event-driven + cleanup on agent exit | Agent type, status, cwd, permission mode, prompt, subagents, worktree, etc. |
| `PaneRuntimeState` in `AppState` | Refresh cycle + cleanup on agent exit | `ports`, `command`, `task_progress`, `task_dismissed_total`, `inactive_since`, `task_store_mtime`, `task_store_all_done` |
| `PaneInfo` (query result, `src/tmux/types.rs`) | Every 1s (refresh cycle) | Per-pane snapshot read from tmux in one `list-panes -a` call: agent type, status, cwd, prompt, pane PID, worktree, session id, and `pane_name` — the pane's own title (`pane_title`) when it is a real name: non-empty, different from the host name, and not an agent-set OSC title (`π - …` from pi, `✳ …` from Claude Code); other agents' OSC titles are a known gap. A title carrying an em dash keeps only the text before it (a suffix like a branch or a cwd is decoration), and a title that trims to empty is not a name. `None` otherwise. Rendered after the badge slot (subagent ♟/↻ or permission badge) in the status row |

Pane options written to tmux (each row's trigger names the transition arms —
`src/pane_state.rs` — or the sweep that writes it):

| Tmux Option | Update Trigger | Description |
|-------------|----------------|-------------|
| `@pane_agent` | SessionStart, UserPromptSubmit, Stop, StopFailure, Notification, PermissionDenied (identity writes, guarded while subagents are active) | Agent type ("claude" / "codex" / "opencode" / "pi") |
| `@pane_status` | SessionStart (`idle`), UserPromptSubmit (`running`), Stop (`idle`/`background`), StopFailure (`error`), Notification/PermissionDenied (`waiting`), ActivityLog (`waiting`/`error` → `running`), the bg-shell sweep (`background` → `idle`), `set-status` | Status ("running" / "background" / "waiting" / "idle" / "error") |
| `@pane_cwd` | SessionStart, CwdChanged | Working directory |
| `@pane_permission_mode` | SessionStart and the other identity events (guarded while subagents are active); plan-mode tools via ActivityLog | Permission mode |
| `@pane_prompt` | UserPromptSubmit, Stop | Latest prompt or response text |
| `@pane_prompt_source` | UserPromptSubmit, Stop | "user" or "response" |
| `@pane_started_at` | UserPromptSubmit; Stop clears it only without a live bg shell; ActivityLog sets it on the first move to `running`; StopFailure and SessionStart clear it | Unix epoch when agent started |
| `@pane_attention` | SessionStart, Stop, StopFailure (clear); Notification, PermissionDenied, TeammateIdle, TaskCompleted (set) | "notification" or "clear" |
| `@pane_wait_reason` | Notification, StopFailure, PermissionDenied, TeammateIdle; SessionStart (resume variants); Stop and UserPromptSubmit clear it | Reason for waiting/error (`permission_denied`, `teammate_idle:<name>`, or error text) |
| `@pane_bg_cmd` | ActivityLog (bg Bash), Refresh sweep (clear), SessionEnd (clear) | Latest sanitized command of a Bash tool started with `run_in_background`. Its presence is the single source of truth for "live bg shell" — Stop routes to `background` while it is set, and the row body renders the command. Persists across UserPromptSubmit so shells spanning turns stay visible; overwritten by the next bg Bash. The refresh loop runs a `ps`-based liveness sweep each tick and clears the marker (plus downgrades `background → idle`) when no process matches the stored command. Only the most recent bg Bash is tracked; older ones are not retained. |
| `@pane_subagents` | SubagentStart/Stop | Comma-separated active subagent list |
| `@pane_worktree_name` | Session-context events that report `worktree` (an empty value clears) | Worktree name (if applicable) |
| `@pane_worktree_branch` | Session-context events that report `worktree` (an empty value clears) | Worktree branch (if applicable) |
| `@pane_session_id` | SessionStart, UserPromptSubmit, Notification, Stop, StopFailure, PermissionDenied, CwdChanged | Agent-reported session id (skipped when subagents are active) |
| `@pane_subagent_kind` | UserPromptSubmit (recognised pmux task prompt, pi only) | "spawned" or "resumed" |
| `@pane_subagent_parent` | UserPromptSubmit (recognised pmux task prompt, pi only) | The parent session id from the prompt path |
| `@pane_subagent_task` | UserPromptSubmit (recognised pmux task prompt, pi only) | The `<name>` part of the prompt path's file name, that is the stem without its `-YYYY-MM-DDTHH-MM-SS` timestamp, with each hyphen turned into a space; the fallback stems `subagent` and `resume` give no task, so it is unset when there is none. Stored, not drawn: the status row's pane name already carries the same launch name |

The three `@pane_subagent_*` options mark a pi pane that pmux started as a
subagent. A later recognised prompt replaces them; a prompt that does not
match leaves them alone. `clear_all_meta()` (session end) and
`clear_agent_pane_state()` (the sweep that clears a pi pane whose agent
process is gone) remove all three with the rest of the pane's metadata — each
wipe path is one `Unset` batch over the shared lifecycle option list, applied
through the store.

In-memory per-pane runtime state. Every field lives inside
`PaneRuntimeState` so the whole record is dropped together when its
pane disappears (`prune_pane_states_to_current_panes`).

| Field | Update Frequency | Description |
|-------|-----------------|-------------|
| `pane_states.map[...].ports` | Every 10s (port scan) | Listening localhost ports detected from the pane process tree |
| `pane_states.map[...].command` | Every 10s (port scan) | Best-effort commandline for the pane process tree, with tmux command fallback in the UI |
| `pane_states.map[...].task_progress` | Every 1s (refresh cycle) | Parsed from activity log — task list per pane |
| `pane_states.map[...].task_dismissed_total` | On task completion | Tracks dismissed completed-task counts |
| `pane_states.map[...].inactive_since` | On status change | Debounce timestamp (3s grace before hiding tasks) |
| `pane_states.map[...].task_progress_log_mtime` | Every 1s (refresh cycle) | mtime of the task-progress log last parsed; skips re-parsing when unchanged |
| `pane_states.map[...].task_store_mtime` | Every 1s (refresh cycle) | mtime of the task store last statted by the close sweep; skips re-reading when unchanged |
| `pane_states.map[...].task_store_all_done` | Every 1s (refresh cycle) | Whether the store read for `task_store_mtime` was non-empty with every item `completed` — the item half of the close rule, cached |

Per-pane file-based state:

| File | Update Trigger | Read Frequency | Description |
|------|---------------|----------------|-------------|
| `/tmp/pmux-explorer-activity_{pane_id}.log` | Each ActivityLog event | Every 1s | Tool usage log (`HH:MM\|tool\|label`), max 200 lines |
| `/tmp/pmux-explorer-tasks{pane_id}.json` | `task-set` / `task-update` / `task-describe` CLI writes; SessionStart and the close sweep remove it | Every 1s (stat), re-read on mtime change | Per-pane Tasks-tab store (`description` + `tasks` array, max 50 items). A store whose items are all `completed` and whose mtime is 90 s old is removed by `refresh_task_close` — focused pane or not — so a finished list returns to the empty state; any store write (add, reopen, describe) restarts the 90-second window |
| `<feed dir>/<session id>.jsonl` — `$XDG_RUNTIME_DIR/pmux-thinking`, or `/tmp/pmux-thinking-<uid>` | The pmux pi extension writes it; the sidebar removes it when it clears the state of a pi pane | Every 1s, only while the Thought tab is drawn, and only from the offset already read | The thinking feed of one pi session (keyed by `@pane_session_id`, not by pane id), reduced to the last thought the Thought tab draws. The first read begins at the newest `start` record; the three teardown paths — `clear_agent_pane_state`, `clear_dead_agent_metadata` and `run_session_end_teardown` — remove it through `thinking_feed::remove_pi_feed` with the session id read before the pane's options are wiped |

### Local State (single sidebar process only)

| Field | Update Frequency | Description |
|-------|-----------------|-------------|
| `repo_groups` | Every 1s | Panes grouped by git repo root (built directly from `tmux::query_sessions()` output, not stored separately as a session list) |
| `focus_state.focused_pane_id` | Every 1s, plus immediately on user-initiated pane jumps | Currently focused agent pane |
| `focus_state.sidebar_focused` | Every 1s | Whether sidebar pane itself has focus |
| `focus_state.focus` | On user input | UI focus: `Filter` / `Panes` / `ActivityLog`; input also triggers an immediate redraw so focus changes appear without waiting for the next poll tick |
| `now` | Every 1s | Current Unix epoch |
| `scrolls.panes` | On user input / render | Agent list scroll position |
| `scrolls.git` | On user input / render | Git status scroll position |
| `scrolls.thought` | On user input / render | Thought tab scroll position; `thought.follow` keeps the view at the last rows while the user has not scrolled up, and a new `start` block resets it |
| `thought.reader` / `thought.last` | Every 1s while the Thought tab is drawn | Incremental reader of the focused pi pane's feed and the last thought its most recent read reduced. A tick that draws another tab reads no feed and leaves the previous thought standing, while no focused pane — or a focused pane with no session id — reads an empty state. The reader's cache is `(session id, offset, mtime)`; `thought.env` (a `FeedEnv`) is captured once at startup |
| `activity.scroll` | On user input / render | Activity log scroll position |
| `activity.entries` | Every 1s | Focused pane's activity entries (max 50) |
| `activity.max_entries` | Once at startup | Max activity log entries to display |
| `activity.log_cache` | Every 1s | `(focused_pane_id, mtime)` of the last-rendered activity log; skips re-reads when unchanged |
| `tasks` | Every 1s (refresh cycle) | The Tasks tab's board (`TasksState`): the board owner's task items, description and `(owner id, mtime)` read cache. The board owner is the focused pane itself, except for a pmux subagent pane with a live parent, which reads its ultimate parent's store (`group::board_owner`) — read-side only, since `task-set`/`task-update`/`task-describe` still write `$TMUX_PANE`'s own store; the cache is keyed on the owner id, so a focus move between a parent and its subagent hits the cache |
| `git` | Every 2s (bg thread) | Branch, diff stats, ahead/behind, PR number |
| `theme` | Once at startup | Color theme from tmux `@sidebar_color_*` variables |
| `popup` | On user input / render | `PopupState` enum: `None` / `Repo { selected, area }` / `Notices { area }`. Enforces "at most one popup open" via the type system |
| `layout` | Every frame (render) | `FrameLayout` sub-struct bundling the ephemeral fields the UI rewrites every frame for click hit-testing: `pane_row_targets`, `line_to_row`, `repo_button_col`, `repo_spawn_targets`, `spawn_remove_targets`, `hyperlink_overlays`, `filter_bar_width` (the width the filter row was rendered with, which the click hit test uses), `strip` (the drawn strip's `StripLayout` — filter, agents, divider, tab-panel and tab-bar rects — that the strip's mouse routing and scroll routing hit-test against) |
| `notices` | Once at startup / on copy | `NoticesState` sub-struct: `button_col`, `missing_hook_groups`, `claude_plugin_status`, `claude_plugin_notice`, `copy_targets`, `copied_at` |
| `timers` | Refresh cycle / on user input | `RefreshTimers` sub-struct gating periodic work: `last_filter_click` (debounce), `last_port_refresh`, `port_scan_initialized` |
| `pending_osc52_copy` | On successful copy / frame flush | OSC 52 clipboard payload queued for terminal forwarding |
| `pet_state` | Every 200ms (animation) | `Idle` / `WalkRight` / `Working` / `WalkLeft` |
| `pet_x` | Every 200ms (animation) | Pet X position |
| `pet_frame` | Every 200ms (animation) | Animation frame counter |
| `pet_bob_timer` | Every 200ms (animation) | Idle bob motion timer |
| `pet_enabled` | Once at startup | Whether the pet is drawn and ticked (from `@sidebar_pet`) |
| `spinner_frame` | Every 200ms (animation) | Spinner animation frame counter |
| `icons` | Once at startup | `StatusIcons` theme (overridable via tmux options) |
| `tmux_pane` | Once at startup | This sidebar's own tmux pane ID |
| `sidebar_pane` | Every 1s (refresh cycle) | This sidebar's own pane info as tmux reported it on the last refresh (`SidebarPaneInfo`): the pane's active flags, the pane's width and height, the window's size, the pane's floating flag, and whether the pane touches its window's top edge. The floating flag is the truth for the docked-vs-floating mode — it is read from the pane, never remembered separately |
| `layout_kind` | Once at startup | Whether this sidebar is a full-height column or a full-width strip (`LayoutKind`), read once from the `PMUX_SIDEBAR_LAYOUT` process environment — never from a pane option; a missing or unknown value gives `Column`. Every layout-specific path branches on it |
| `docked_size` | On pop-out / pop-in (user input) | The pane size captured when the sidebar was popped out, restored by the pop-in (`resize-pane -x` for a column, `-y` for a strip): the width of a column, the height of a strip (`src/state/popout.rs`). `None` while docked and when the pane was already floating when the sidebar started |
| `docked_edge` | On pop-out / pop-in (user input) | The edge a popped-out strip was docked at, captured with the docked size on the way out — a floating pane is no longer at an edge, so the pop-in cannot read it off the pane. `None` for a column and when the pane was already floating; the strip pop-in then falls back to the bare join |
| `width_guard` | Every 1s (refresh cycle) | Cross-tick state of the size-lock guard step (`src/app/width_guard.rs`), run on the axis the instance's fixed layout gave it — a column on width, a strip on height: `last_observed_size` records the last docked pane size on that axis (`None` until the first docked reading primes it, so a startup size cannot publish itself) and `pending_adopt_observation` is the one-shot echo suppressor armed whenever this instance issues a `resize-pane` (`-x` for a column, `-y` for a strip) — an adoption, or the pop-in restore (`src/state/popout.rs`) — so its own landing is not published back as the shared size |
| `version_notice` | Once at startup (bg fetch) | GitHub release update notice, `None` when up-to-date |

---

## Update Cycle Summary

```
┌─────────────────────────────────────────────────────────────┐
│  Every frame (~200ms)                                       │
│  layout.* (rebuilt by ui::draw), spinner + pet animation     │
├─────────────────────────────────────────────────────────────┤
│  Every 1s (refresh cycle)                                   │
│  repo_groups, focus_state.focused_pane_id,                  │
│  sidebar_pane, layout.pane_row_targets, activity.entries,   │
│  pane_states.map[..].task_progress,                         │
│  GlobalState shared options re-read every tick              │
│  (bottom tab adoption included)                             │
├─────────────────────────────────────────────────────────────┤
│  pane_states.map[..].ports, agent liveness cleanup          │
├─────────────────────────────────────────────────────────────┤
│  Once at startup                                             │
│  theme, notices.claude_plugin_*,                            │
│  notices.claude_plugin_notice, notices.missing_hook_groups  │
├─────────────────────────────────────────────────────────────┤
│  Every 2s (git background thread)                           │
│  git (branch, diff, ahead/behind, PR)                       │
├─────────────────────────────────────────────────────────────┤
│  Event-driven (agent hooks)                                 │
│  @pane_* tmux options (one batched apply per event),        │
│  activity log files                                         │
├─────────────────────────────────────────────────────────────┤
│  On user input                                              │
│  focus_state.focus, scrolls.*, activity.scroll,             │
│  GlobalState fields, popup (PopupState enum),               │
│  timers.last_filter_click, pop-out / pop-in (docked_size),  │
│  immediate selection / active-pane redraw                   │
├─────────────────────────────────────────────────────────────┤
│  Every frame (render)                                       │
│  layout.line_to_row, popup.area (Repo/Notices variants),    │
│  notices.button_col, notices.copy_targets,                  │
│  layout.hyperlink_overlays                                  │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow

```
Agent hooks (hook.sh)
  → CLI `hook` subcommand (cli/hook.rs)
    → resolve_adapter() (event.rs) → adapter.parse() → AgentEvent
    → handle_event(): one store snapshot, one transition, one batched
      apply of the PaneWrites through the PaneStore (one tmux command),
      then the non-option effects (activity log files, notifications,
      task store)
                        ↓
TUI main loop (app::run in app.rs; submodules app/{setup,workers,input,render,panel_guard})
  → startup plugin-state reads (cli/plugin_state.rs)
    → installed_plugins.json / ~/.claude/settings.json
    → initializes Claude notices state once
                        ↓
  → refresh() every 1s
    → retry_pending_saves            ← a write tmux rejected is retried before the shared read
    → get_sidebar_pane_reading()     ← reads pane state + @sidebar_* in one display-message
    → global.apply_all()             ← applies the shared options (filter, cursor, bottom tab, repo filter, bottom percent, tab percent, split tabs, split widths, width lock, shared docked width, shared docked height, min width) under the SaveLatch/last_saved_* guards
    → query_sessions() (tmux.rs)     ← reads @pane_* via `tmux list-panes -a`
    → pane_name resolved in the same query (the pane title when it is
      a real name — em dash suffixes trimmed; no extra tmux call)
    → group_panes_by_repo() (group.rs)
    → rebuild_row_targets()          ← applies GlobalState filters
    → refresh_activity_data()        ← reads /tmp activity logs
      → refresh_task_close()         ← stats every pane's task store; removes a store
                                       all-completed for 90 s (any write restarts the window)
      → refresh_thought_feed()       ← reads the focused pi pane's thinking feed, only
                                       while the Thought tab is drawn; new bytes only
    → refresh_task_progress()        ← updates PaneRuntimeState.task_progress
    → refresh_port_data()            ← updates PaneRuntimeState.ports; clears dead pane state
                                       and its thinking feed
    → scan_session_process_snapshot() ← detects dead panes and clears stale tmux metadata
                        ↓
  → git_rx.try_recv()                ← receives GitData from background thread
  → notices popup render/copy state  ← derived from AppState plugin fields
                        ↓
  → ui::draw() renders frame         ← reads all AppState fields;
    status row = agent label + pane_name (ui/panes/row/status.rs);
    claude and pi draw a Nerd Font glyph as their label, codex and
    opencode keep their protocol name
    branch row = git glyph + branch label (ui/panes/row/branch.rs)
    repo header = repo name + right-aligned spawn button, the button only
    when the group has a repo_root (ui/panes/row_collector.rs)
```

---

## Key Types

```rust
/// One entry of a write list: set an option, or unset it. The keys are
/// the `&'static str` option-name constants of `tmux::options`.
enum PaneWrite {
    Set(&'static str, String),
    Unset(&'static str),
}

/// An ordered write list, applied as one batch — one tmux invocation of
/// `;`-joined `set` commands. Later entries win over earlier ones for
/// the same key; an empty list applies as a no-op.
struct PaneWrites {
    entries: Vec<PaneWrite>,
}

/// The four rule options, read with one call. An absent option — or one
/// set to an empty value — is `None`.
struct PaneSnapshot {
    status: Option<String>,
    bg_cmd: Option<String>,
    subagents: Option<String>,
    started_at: Option<String>,
}

/// The store seam every agent pane option access goes through: one
/// snapshot read, one single-option read, one batched apply. Created
/// once at a command's entry and passed down; the tmux adapter
/// (`src/tmux/pane_store.rs`) is production, `MemPaneStore` is tests.
trait PaneStore {
    fn snapshot(&self, pane: &str) -> PaneSnapshot;
    fn get(&self, pane: &str, key: &str) -> String;
    fn apply(&self, pane: &str, writes: &PaneWrites) -> Result<(), String>;
}

/// The write rules for one producer event: pure, owned by
/// `src/pane_state.rs` together with `status_writes` (the
/// status↔attention coupling) and `identity_guarded` (the subagent
/// guard). Values are raw; the store encodes them at `apply` time.
fn transition(
    snapshot: &PaneSnapshot,
    event: &AgentEvent,
    reported: &Reported,
) -> PaneWrites;

enum Focus { Filter, Panes, ActivityLog }
enum StatusFilter { All, Busy, Waiting, Idle, Error }
enum RepoFilter { All, Repo(String) }
enum BottomTab { Activity, Tasks, GitStatus, Thought }
enum PaneStatus { Running, Background, Waiting, Idle, Error, Unknown }
enum AgentType { Claude, Codex, OpenCode, Unknown }
enum PermissionMode { Default, Plan, AcceptEdits, Auto, DontAsk, BypassPermissions, Defer }

/// At-most-one popup state. The enum encodes both which popup is open
/// and its per-popup data so the invariant is checked by the type system.
enum PopupState {
    None,
    Repo { selected: usize, area: Option<Rect> },
    Notices { area: Option<Rect> },
    /// Modal text input shown when the user spawns a new worktree.
    SpawnInput {
        input: String,
        target_repo: String,
        target_repo_root: String,
        agent_idx: usize,
        mode_idx: usize,
        field: SpawnField,
        anchor_y: Option<u16>,
        error: Option<String>,
        area: Option<Rect>,
    },
    /// Confirmation prompt shown when the user removes a sidebar-spawned pane.
    RemoveConfirm {
        pane_id: String,
        branch: String,
        error: Option<String>,
        area: Option<Rect>,
    },
}

struct ScrollState {
    offset: usize,
    total_lines: usize,
    visible_height: usize,
}

struct HyperlinkOverlay {
    x: u16,
    y: u16,
    text: String,
    url: String,
}

struct PaneRuntimeState {
    ports: Vec<u16>,
    command: Option<String>,
    task_progress: Option<TaskProgress>,
    task_dismissed_total: Option<usize>,
    inactive_since: Option<u64>,
    task_progress_log_mtime: Option<SystemTime>,
    task_store_mtime: Option<SystemTime>,
    task_store_all_done: bool,
}

/// Per-pane runtime entries keyed by pane id. The selected bottom tab
/// is not stored here: it is shared server-wide through
/// `@sidebar_bottom_tab` and lives in `GlobalState`, so nothing
/// pane-scoped has to remember it.
struct PaneRuntimeMap {
    map: HashMap<String, PaneRuntimeState>,
}

/// Focus-related fields grouped so UI code can pass them as a single
/// sub-struct rather than juggling three flat fields.
struct FocusState {
    sidebar_focused: bool,
    focus: Focus,
    focused_pane_id: Option<String>,
}

/// Non-activity scrolls (the agent list, the git bottom panel and the
/// Thought tab). Activity's scroll lives inside `ActivityState`
/// because it pairs with the activity entries buffer.
struct ScrollStates {
    panes: ScrollState,
    git: ScrollState,
    thought: ScrollState,
}

/// Activity-log snapshot for the focused pane plus cache metadata so
/// the polling tick can skip redundant file reads.
struct ActivityState {
    entries: Vec<ActivityEntry>,
    scroll: ScrollState,
    max_entries: usize,
    log_cache: Option<(String, SystemTime)>,
}

/// The Thought tab's feed reader state, filled by `refresh_thought_feed`
/// on the refresh tick while the tab is drawn. The reader holds one
/// cache (`session id`, byte offset, mtime) and the reduced last
/// thought; `follow` and `seen_start_ts` drive follow-the-end, and the
/// feed environment is captured once at startup so the path helper
/// takes it as an argument instead of reading the process env.
struct ThoughtState {
    reader: ThinkingFeedReader,
    last: LastThought,
    follow: bool,
    seen_start_ts: Option<String>,
    env: FeedEnv,
    hhmmss: fn(&str) -> Option<String>,
}

/// Frame-scoped render output cached for click hit-testing. Rewritten
/// every frame by the UI layer; consumed by mouse/keyboard handlers
/// before the next render.
struct FrameLayout {
    pane_row_targets: Vec<RowTarget>,
    line_to_row: Vec<Option<usize>>,
    repo_button_col: Option<u16>,
    repo_spawn_targets: Vec<RepoSpawnTarget>,
    spawn_remove_targets: Vec<SpawnRemoveTarget>,
    hyperlink_overlays: Vec<HyperlinkOverlay>,
    filter_bar_width: u16,
    strip: Option<StripLayout>,
}

/// Periodic-refresh bookkeeping.
struct RefreshTimers {
    last_filter_click: Instant,
    last_port_refresh: Instant,
    port_scan_initialized: bool,
}

/// All fields for the ⓘ notices button and its popup.
struct NoticesState {
    button_col: Option<u16>,
    missing_hook_groups: Vec<NoticesMissingHookGroup>,
    claude_plugin_status: ClaudePluginStatus,
    claude_plugin_notice: Option<ClaudePluginNotice>,
    copy_targets: Vec<NoticesCopyTarget>,
    copied_at: Option<(String, Instant)>,
}
```

---

## State Invariants

1. `selected_pane_row` is always < `layout.pane_row_targets.len()` unless the list is empty — clamped in `rebuild_row_targets()` — and `cursor_pane_id` names the pane on that row (or is `None` when the row holds no pane). The two are resolved together by `resolve_cursor_row()`: the named pane's row when it is listed, else the tmux-focused pane's row, else the clamped row; whichever row wins renames the cursor
2. `activity.entries` contains only the focused pane's entries — cleared on focus change
3. The bottom tab is shared server-wide through `@sidebar_bottom_tab` and changes only on an explicit user selection (cycle key / tab-title click) or the per-tick adoption of a peer's selection — never on focus change, and relaunching an agent does not change it. A stored `thought` is displayed as Activity (the `Tools` label) while the focused pane is not pi, again without a write
4. Git fetching respects the `git_tab_active` flag — stops when tab is hidden
5. Task progress has a 3-second debounce — prevents flicker when agent briefly pauses
6. Global state syncs via tmux variables — enables coordination across sidebar instances
7. Scroll positions are independent per panel — agents, activity, git and thought each have their own `ScrollState`
8. `layout.line_to_row` is rebuilt every frame — ensures accurate click routing
9. Pane runtime state is pruned when the pane disappears — prevents stale per-pane ports and task progress from surviving after the agent is gone
10. At most one popup is open at a time — enforced structurally by the `PopupState` enum, not by parallel boolean flags
11. Hook-based cleanup wins when available; pid-based cleanup is a slower fallback that removes panes when the agent process is gone but the hook did not fire
12. A task store with every item `completed` closes 90 seconds after its last write, for every listed pane regardless of focus — adding or reopening an item restarts the window
