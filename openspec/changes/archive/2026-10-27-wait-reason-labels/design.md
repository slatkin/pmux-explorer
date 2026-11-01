# Design

## Context

`@pane_wait_reason` carries a raw producer token. Two consumers read it today:
`wait_reason_label` (`src/ui/text.rs`) — a match-table shared by the pane row
(`ui/panes/row/body.rs`) and `notification_body` — and
`is_permission_wait_reason` (`src/cli/hook/handlers/status_priority.rs`), the
allowlist that decides whether a wait reason preempts a live background shell.
Both tables know only Claude's vocabulary; pi's `ui_prompt` kinds pass through
raw and demote under a background shell.

## Goals / Non-Goals

**Goals:**
- One vocabulary table that answers both questions about a raw reason: *what
  label does it show* and *does it demand the user*.
- pi's five `ui_prompt` kinds named and classified without touching the
  producer, adapter, or protocol.

**Non-Goals:**
- Forwarding prompt titles or question text into the sidebar (rejected — user
  does not want the question text).
- A producer-side reason vocabulary change; `@pane_wait_reason` stays raw.
- Mapping reasons for producers that do not exist yet beyond the existing
  pass-through.

## Decisions

**D1 — map at the display layer, not the producer.**
The reason is a cross-producer presentation concern; the label table already
exists for exactly this and feeds both the row and the desktop notification.
Alternative considered: have the pmux pi producer send a nicer
`notification_type` — rejected because it fixes only pi, forks the vocabulary
between producer and sidebar, and this change deliberately benefits Claude's
unmapped names too.

**D2 — one per-reason entry drives both label and priority class.**
Extend `wait_reason_label`'s match into a per-reason record `{ label,
action_required }` (or a second match beside it keyed on the same names) so
`is_permission_wait_reason` and the label never drift apart — a reason added
for display should not silently stay demotable. Alternative considered: two
independent lists — rejected; the drift risk is real (a mapped pi reason that
still demotes is exactly the bug this change fixes).

**D3 — `custom` labels as `prompt`, not `question`.**
The kind covers any blocking extension UI, not only ask_user; "prompt" is the
honest generic word, and a future semantic reason (`question` from a bus
contract) can map to "question" distinctly. pi's other kinds get their literal
meanings; `input` shares the existing "waiting for input" wording used by
`idle_prompt`.

**D4 — pi prompt kinds are action-required.**
A blocking prompt needs the user like a permission dialog does, so the five
kinds join the preempt class. This is the only status-path change; attention
and the wait-reason row already behave correctly for pi.

## Risks / Trade-offs

- [A pi `ui_prompt` fired by a non-interactive custom widget (a picker, not a
  question) will read "prompt" and preempt a background shell] → acceptable
  over-claim: every `ui_prompt` kind genuinely blocks on a human; "prompt"
  stays accurate. Narrowing to "question" requires producer semantics the
  `ui_prompt` event does not carry.
- [pi's permission dialog (kind `confirm`) now reads "confirmation" rather
  than "permission required"] → acceptable: it is still clearly
  action-required text; a shared wording like "confirmation required" keeps
  it close to Claude's phrasing if preferred.
