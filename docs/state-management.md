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
| `selected_pane_row` | `@sidebar_cursor` | User input (j/k key); tmux write flushed after a short debounce | Cursor position in agent list |
| `repo_filter` | `@sidebar_repo_filter` | User input (repo popup) | Repository filter (All or specific repo) |
| `bottom_tab` | `@sidebar_bottom_tab` | On user tab switch (cycle key / tab-title click); tmux write retried when rejected | Selected bottom tab, shared server-wide; each instance adopts a peer's selection on its next refresh tick. The drawn labels run `Tools│Thought│Tasks│Git` for a pi pane and `Tools│Tasks│Git` otherwise; the stored values are `activity`, `thought`, `tasks` and `git`, and the Activity tab keeps that variant name while drawing as `Tools`. `thought` is drawn only while the focused pane runs pi — a stored Thought falls back to Activity for any other agent, without a write, so the panel returns to Thought when the focus comes back to a pi pane |
| `bottom_percent` | `@sidebar_bottom_height` | User input (rule drag release); tmux write retried when rejected | Bottom panel height as a percent of the sidebar height (0 hides the panel; unset, empty, or an out-of-range value gives 30); shared server-wide, each instance adopts a peer's value on its next refresh tick (the percent's `SaveLatch` instance follows the same pattern) |

Each synced field is guarded so a failed write never loses the user's
choice: the status filter, the bottom tab and the bottom percent each
hold one `SaveLatch<T>` instance (`filter_latch`, `tab_latch`,
`percent_latch` in `GlobalState` — one retry mechanism, written once
and shared by all three). The cursor and repo filter keep plain
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

The three instances differ only at their edges. `filter_latch` and
`tab_latch` parse their single option and adopt nothing while it is
absent. `percent_latch` parses the whole option map instead: an
unset, empty or invalid value resolves to the default
(`DEFAULT_BOTTOM_PERCENT = 30`) and is adopted. The percent is also
the only latch with an extra adoption gate — `apply_all` receives
`split_drag_active` (`AppState::split_drag` is `Some`) and holds the
shared percent unadopted while it stands, so the tick cannot move the
rule out from under the mouse; the ticks after the release adopt the
server value again.

### Per-pane State (keyed by pane ID)

Written by `cli/hook.rs` on agent events, read by `query_sessions()` every **1 second**.

Each pane's runtime data is split into two buckets:

| Source | Update Trigger | Description |
|--------|----------------|-------------|
| tmux pane options | Event-driven + cleanup on agent exit | Agent type, status, cwd, permission mode, prompt, subagents, worktree, etc. |
| `PaneRuntimeState` in `AppState` | Refresh cycle + cleanup on agent exit | `ports`, `command`, `task_progress`, `task_dismissed_total`, `inactive_since`, `task_store_mtime`, `task_store_all_done` |
| `PaneInfo` (query result, `src/tmux/types.rs`) | Every 1s (refresh cycle) | Per-pane snapshot read from tmux in one `list-panes -a` call: agent type, status, cwd, prompt, pane PID, worktree, session id, and `pane_name` — the pane's own title (`pane_title`) when it is a real name: non-empty, different from the host name, and not an agent-set OSC title (`π - …` from pi, `✳ …` from Claude Code); other agents' OSC titles are a known gap. A title carrying an em dash keeps only the text before it (a suffix like a branch or a cwd is decoration), and a title that trims to empty is not a name. `None` otherwise. Rendered after the badge slot (subagent ♟/↻ or permission badge) in the status row |

Pane options written to tmux:

| Tmux Option | Update Trigger | Description |
|-------------|----------------|-------------|
| `@pane_agent` | SessionStart | Agent type ("claude" / "codex" / "opencode") |
| `@pane_status` | Every event | Status ("running" / "background" / "waiting" / "idle" / "error") |
| `@pane_cwd` | SessionStart, CwdChanged | Working directory |
| `@pane_permission_mode` | SessionStart, hook event | Permission mode |
| `@pane_prompt` | UserPromptSubmit, Stop | Latest prompt or response text |
| `@pane_prompt_source` | UserPromptSubmit, Stop | "user" or "response" |
| `@pane_started_at` | UserPromptSubmit | Unix epoch when agent started |
| `@pane_attention` | SessionStart, Stop, StopFailure (clear); Notification, PermissionDenied, TeammateIdle (set) | "notification" or "clear" |
| `@pane_wait_reason` | StopFailure, PermissionDenied, TeammateIdle | Reason for waiting/error (`permission_denied`, `teammate_idle:<name>`, or error text) |
| `@pane_bg_cmd` | ActivityLog (bg Bash), Refresh sweep (clear), SessionEnd (clear) | Latest sanitized command of a Bash tool started with `run_in_background`. Its presence is the single source of truth for "live bg shell" — Stop routes to `background` while it is set, and the row body renders the command. Persists across UserPromptSubmit so shells spanning turns stay visible; overwritten by the next bg Bash. The refresh loop runs a `ps`-based liveness sweep each tick and clears the marker (plus downgrades `background → idle`) when no process matches the stored command. Only the most recent bg Bash is tracked; older ones are not retained. |
| `@pane_subagents` | SubagentStart/Stop | Comma-separated active subagent list |
| `@pane_worktree_name` | SessionStart | Worktree name (if applicable) |
| `@pane_worktree_branch` | SessionStart | Worktree branch (if applicable) |
| `@pane_session_id` | SessionStart, UserPromptSubmit, Notification, Stop, StopFailure, PermissionDenied, CwdChanged | Agent-reported session id (skipped when subagents are active) |
| `@pane_subagent_kind` | UserPromptSubmit (recognised pmux task prompt, pi only) | "spawned" or "resumed" |
| `@pane_subagent_parent` | UserPromptSubmit (recognised pmux task prompt, pi only) | The parent session id from the prompt path |
| `@pane_subagent_task` | UserPromptSubmit (recognised pmux task prompt, pi only) | The `<name>` part of the prompt path's file name, that is the stem without its `-YYYY-MM-DDTHH-MM-SS` timestamp, with each hyphen turned into a space; the fallback stems `subagent` and `resume` give no task, so it is unset when there is none. Stored, not drawn: the status row's pane name already carries the same launch name |

The three `@pane_subagent_*` options mark a pi pane that pmux started as a
subagent. A later recognised prompt replaces them; a prompt that does not
match leaves them alone. `clear_all_meta()` (session end) and
`clear_agent_pane_state()` (the sweep that clears a pi pane whose agent
process is gone) remove all three with the rest of the pane's metadata.

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
| `git` | Every 2s (bg thread) | Branch, diff stats, ahead/behind, PR number |
| `theme` | Once at startup | Color theme from tmux `@sidebar_color_*` variables |
| `popup` | On user input / render | `PopupState` enum: `None` / `Repo { selected, area }` / `Notices { area }`. Enforces "at most one popup open" via the type system |
| `layout` | Every frame (render) | `FrameLayout` sub-struct bundling the ephemeral fields the UI rewrites every frame for click hit-testing: `pane_row_targets`, `line_to_row`, `repo_button_col`, `repo_spawn_targets`, `spawn_remove_targets`, `hyperlink_overlays`, `filter_bar_width` (the width the filter row was rendered with, which the click hit test uses) |
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
| `sidebar_pane` | Every 1s (refresh cycle) | This sidebar's own pane info as tmux reported it on the last refresh (`SidebarPaneInfo`): the pane's active flags, the pane's width, the window's size, and the pane's floating flag. The floating flag is the truth for the docked-vs-floating mode — it is read from the pane, never remembered separately |
| `docked_width` | On pop-out / pop-in (user input) | The pane width captured when the sidebar was popped out, restored by the pop-in with `resize-pane -x` (`src/state/popout.rs`). `None` while docked and when the pane was already floating when the sidebar started |
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
│  @pane_* tmux options, activity log files                   │
├─────────────────────────────────────────────────────────────┤
│  On user input                                              │
│  focus_state.focus, scrolls.*, activity.scroll,             │
│  GlobalState fields, popup (PopupState enum),               │
│  timers.last_filter_click, pop-out / pop-in (docked_width), │
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
    → handle_event() writes @pane_* tmux options + /tmp activity log files
                        ↓
TUI main loop (app::run in app.rs; submodules app/{setup,workers,input,render,panel_guard})
  → startup plugin-state reads (cli/plugin_state.rs)
    → installed_plugins.json / ~/.claude/settings.json
    → initializes Claude notices state once
                        ↓
  → refresh() every 1s
    → retry_pending_saves
                                     ← a write tmux rejected is retried before the shared read
    → get_sidebar_pane_reading()     ← reads pane state + @sidebar_* in one display-message
    → global.apply_all()             ← applies the shared options (filter, cursor, bottom tab, repo filter, bottom percent) under the last_saved_* guards
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
    repo header = git glyph + repo name + right-aligned spawn button
    (ui/panes/row_collector.rs), glyph only when the group has a repo_root
```

---

## Key Types

```rust
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

1. `selected_pane_row` is always < `layout.pane_row_targets.len()` — clamped in `rebuild_row_targets()`
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
