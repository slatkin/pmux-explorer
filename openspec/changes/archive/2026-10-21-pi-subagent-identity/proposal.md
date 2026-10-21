# Proposal

## Why

A pmux subagent is a pi process that pmux starts in its own pane with
`pi … @<task file>`. pi expands that argument into
`<file name="/abs/path">…</file>`, and the pi producer forwards the expanded
text as the `user-prompt-submit` prompt. The sidebar stores it as is, so the
row under a subagent shows a raw XML line that nobody can read.

That wrapper is also a signal. The `name` attribute is a path that pmux builds
in a fixed shape, `<sessionDir>/artifacts/<parent session id>/(context|pmux-resume)/<name>-<timestamp>.md`.
It tells the sidebar that the pane is a subagent, whether it was spawned or
resumed, which session is its parent, and the display name its parent gave it.
In an orchestration with many child panes, that is the information the user
needs, and the raw XML hides it.

## What Changes

- The pi adapter recognises a `user-prompt-submit` prompt that starts with a
  pmux task tag: a `<file name="…">` whose path has the spawn shape
  (`…/artifacts/<id>/context/<name>-<ts>.md`) or the resume shape
  (`…/artifacts/<id>/pmux-resume/<name>-<ts>.md`). Only the `name` attribute is
  read. The file body is never read and never shown.
- For a matched prompt, the hook records the subagent identity in new pane
  options: the parent session id, the kind (spawned or resumed) and the
  fuzzy task. It does not write the prompt text to `@pane_prompt`.
- The identity persists across later prompts and turns of the same session.
  A later matched prompt replaces it. `session-end` and the dead pi pane sweep
  clear it.
- The fuzzy task is `<name>` without its timestamp suffix, with hyphens shown
  as spaces. The pmux fallback stems `subagent` (spawn) and `resume` (resume)
  mean "no name", and the row then shows no task.
- The pane query reads `pane_title` for every pane. The sidebar resolves the
  parent at each refresh: the pane whose `@pane_session_id` equals the
  recorded parent session id. The parent is shown by that pane's title, or by
  its pane id when the title is empty.
- When no pane carries the parent session id, the row shows `ORPHANED` in red.
- The status row of a subagent pane shows a `subagent` badge, or a `resumed`
  badge for a resumed subagent, in the badge slot that pi leaves empty.
- A new row under the status row shows the parent and the fuzzy task.
- Response previews of a subagent pane stay as they are today.
- The `pi-agent-support` requirement "the gaps stay gaps" changes: pi still
  reports no permission mode and no task progress, but a pmux subagent pane is
  now marked as a subagent with its parent.

Out of scope:
- The pmux skill launch forms (`<scratch>/<name>-<id>.prompt.md`) and the pmux
  fork mode, which sends the task without a `<file>` tag.
- A `<file>` prompt with any other path (for example a person who runs
  `pi @notes.md`). It is stored as today.
- `<skill name="…">` prompts. They are stored as today and do not change the
  identity.
- Nesting the child row under the parent row, and a click that focuses the
  parent.
- Any change to pmux or pi.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pi-agent-support`: adds the pmux subagent identity (recognition of the
  task tag, the stored identity, the parent resolution, the badges and the
  identity row), and modifies "the gaps stay gaps" so that it no longer
  forbids a subagent marking for a pi pane.

## Impact

- `src/adapter/pi.rs`: recognise the task tag in `user-prompt-submit`.
- `src/event.rs`: carry the parsed identity on `AgentEvent::UserPromptSubmit`;
  the other adapters pass none.
- A new pure parser module for the task tag path.
- `src/cli/hook.rs`, `src/cli/hook/handlers/run.rs`: write the identity
  options and skip the prompt write for a matched prompt.
- `src/tmux/options.rs`, `src/cli/hook/context/meta.rs` (`clear_all_meta`),
  `src/tmux/query.rs` (`clear_agent_pane_state`, `pane_format`, the pane line
  parser and a parent resolution pass): new options, the `pane_title` field.
- `src/tmux/types.rs`: `PaneInfo` gets the pane title and the subagent
  identity with its resolved parent.
- `src/ui/panes/row.rs`, `src/ui/panes/row/status.rs` and the row body: the
  badge and the identity row.
- `docs/agent-protocol.md` and `docs/state-management.md`: the new options and
  the changed "cannot report" list.
- Tests: parser unit tests, adapter and handler tests, query tests, and inline
  `insta` snapshots for the new rows.
