# Design

## Context

See proposal.md (Why) and the delta spec for the required behaviour.

The facts that shape the approach:

- The hook writes every prompt to `@pane_prompt`
  (`on_user_prompt_submit`), and `on_stop` replaces it with the response
  preview. After the first turn the `<file>` text is gone, so the sidebar
  cannot parse the prompt at render time. The parse must happen in the hook,
  and the result must live in its own pane options.
- An empty prompt in `on_user_prompt_submit` already means "keep the prompt
  row as it is" (the pi wait-is-over path).
- Handlers never see agent names or raw JSON; adapters own the producer's
  format (`src/adapter/`).
- `build_session_hierarchy` parses one `list-panes -a` line per pane. A
  post-pass over the parsed panes already exists for Codex permission modes
  (`resolve_codex_permission_modes`).
- The pane format is positional. New fields go at the end so the existing
  indices stay valid.
- `@pane_role` marks the sidebar pane, and `@pane_name` is queried but never
  set or rendered. Neither is reused.
- pmux builds the path as
  `<sessionDir>/artifacts/<sessionId>/<folder>/<stem>-<ts>.md`, where
  `<stem>` is `slugifyName(name, { hyphenate: false })` (lowercase
  `[a-z0-9-]`, fallback `subagent`, or `resume` on a resume) and `<ts>` is
  the ISO time cut to 19 characters with `:` and `.` replaced by `-`.
- There is no regex crate in the dependencies.

## Goals / Non-Goals

**Goals:**
- One pure parser for the task tag, unit-tested against the pmux path shapes.
- No new tmux call per refresh: the parent lookup uses fields of the existing
  `list-panes -a` query.
- The parent resolution sees every agent pane, independent of the repository
  filter.

**Non-Goals:**
- No change to how a non-matching prompt is stored.
- No new dependency.
- No nesting of child rows under the parent row.

## Decisions

### D1. The pi adapter parses; the event carries the result

`AgentEvent::UserPromptSubmit` gets a field
`pmux_task: Option<PmuxTask>`. The pi adapter calls the parser on the
`prompt`. On a match it sets `pmux_task` and sets `prompt` to the empty
string. The other adapters set `pmux_task: None`.

The handler writes the identity options when `pmux_task` is `Some`. Because
the prompt is empty, the existing code leaves `@pane_prompt` as it was, and
the status still becomes running.

Alternative: parse in the handler for every agent. It touches fewer
constructors, but it puts a pi-specific format in the shared handler, and the
spec says that other agents are not marked.

### D2. A pure parser module without regex

A new leaf module (for example `src/pmux_task.rs`) holds:

```
enum PmuxTaskKind { Spawned, Resumed }
struct PmuxTask { kind, parent_session_id: String, task: Option<String> }
fn parse_pmux_task(prompt: &str) -> Option<PmuxTask>
```

Steps, with plain string operations:

1. The prompt starts with `<file name="`. Take the text up to the next `"`.
   That is PATH.
2. PATH ends with `.md`. Split it on `/`. The last four segments are
   `artifacts`, `<id>`, `context` or `pmux-resume`, `<stem>.md`. `<id>` is not
   empty.
3. `<stem>` ends with `-` plus 19 characters of the shape
   `DDDD-DD-DDTDD-DD-DD`. The name is the part before it, and it is not
   empty.
4. A name of `subagent` (spawned) or `resume` (resumed) gives `task: None`.
   Otherwise `task` is the name with `-` replaced by a space.

Anything else returns `None`, so the prompt is handled as before.

### D3. Three new pane options

| Option | Value |
|---|---|
| `@pane_subagent_kind` | `spawned` or `resumed` |
| `@pane_subagent_parent` | the parent session id |
| `@pane_subagent_task` | the fuzzy task, unset when there is none |

The handler sets all three on a match (and unsets the task when there is
none), so a later match replaces the earlier identity. A non-matching prompt
does not touch them. Both teardown lists get the three keys:
`clear_all_meta` (session end) and `clear_agent_pane_state` (dead pi pane).

### D4. Four fields appended to the pane format

`pane_format()` appends `@pane_subagent_kind`, `@pane_subagent_parent`,
`@pane_subagent_task` and `pane_title`, after `@pane_bg_cmd`. Both
`MIN_FIELDS` constants grow by four, and `pane_line_field` gets the four
indices.

### D5. Parent resolution as a post-pass

`PaneInfo` gets one field, `subagent: Option<SubagentInfo>`:

```
struct SubagentInfo { kind: PmuxTaskKind, task: Option<String>, parent: SubagentParent }
enum SubagentParent { Pane(String /* title, or pane id */), Orphaned }
```

`build_session_hierarchy` collects, while it parses the lines, a map from
`@pane_session_id` to the parent label (`pane_title`, or the pane id when the
title is empty) for every agent pane. A post-pass next to
`resolve_codex_permission_modes` then sets each subagent's `parent` from the
map, or `Orphaned` when the id is not in the map. The pane parser first
creates the `SubagentInfo` with `Orphaned`.

The map holds only panes that pass `parse_pane_fields_with_processes`. A
parent pane whose agent is gone is cleared there, so its child becomes
orphaned. The resolution runs before any filter, so a parent row that the
repository filter hides still resolves.

The title goes into the map only, not into `PaneInfo`, so the other
`PaneInfo` construction sites get only the `subagent: None` field.

Alternative: resolve in the row renderer. The renderer sees one pane at a
time and only the panes that survive the filter, so it cannot do this.

### D6. Rendering

- Status row: when `pane.subagent` is `Some`, the badge slot shows
  `subagent` (colour `badge_plan`) or `resumed` (colour `badge_auto`). A pi
  pane always has the default permission mode, so the slot is free. The
  width budget code already reserves room for a badge.
- Identity row: a new row function, placed directly after the status row and
  before the branch/ports row. It shows a leading arrow glyph, then the
  parent label in the inactive text colour or `ORPHANED` in `status_error`,
  then two spaces and the task in `text_muted` when there is a task. It is
  cut to the inner width with `truncate_to_width`, so it is always one row.
  It uses the plain context (no active marker), like the other detail rows.

## Risks / Trade-offs

- [pmux changes its path shape or timestamp format] → The parser returns
  `None` and the pane falls back to the present raw prompt display. The
  parser tests name the pmux source of each shape, so a change is easy to
  trace.
- [A program in the parent pane resets `pane_title` with OSC 2, for example
  pi's `π - <name> - <cwd>`] → The row shows whatever the title is at refresh
  time. In the user's orchestration the orchestrator sets the name. Not
  mitigated further.
- [The parent ran `/new` and now has a new session id] → The child shows
  `ORPHANED`, which is correct: that parent session no longer exists.
- [The positional format grows to 32 fields] → The indices are named
  constants and the query tests build full lines; the tests catch an
  off-by-one.
- [Two panes report the same session id] → The map keeps the first. This
  does not happen in practice.

## Migration Plan

None. The new options are absent on existing panes, so those panes render as
before. A subagent started before the upgrade is marked at its next matching
prompt only (a resume).
