## Purpose

How the sidebar names the reason a pane waits on a human: the shared
raw-token → display-label table that every producer's wait-reason vocabulary
goes through, and which reasons count as action-required for status priority.

## ADDED Requirements

### Requirement: a wait reason is shown through its label

The sidebar SHALL translate a pane's raw wait reason into a readable label
through a single shared table. The label is what the pane row and the desktop
notification body show. A reason the table does not name SHALL be shown
unchanged. The raw reason SHALL stay the stored pane value; only its display
changes.

#### Scenario: a known reason is labelled

- **WHEN** a pane's wait reason is `permission_prompt`
- **THEN** the pane row and the desktop notification body show "permission required"

#### Scenario: an unknown reason passes through

- **WHEN** a pane's wait reason is a token the table does not name
- **THEN** it is shown unchanged, so a producer's new vocabulary still surfaces

### Requirement: pi's prompt kinds have labels

The table SHALL name the prompt kinds pi's producer reports: `select` as
"waiting for selection", `confirm` as "confirmation", `input` as "waiting for
input", `editor` as "editor", and `custom` as "prompt".

#### Scenario: a pi pane waits on an overlay prompt

- **WHEN** a pi pane's wait reason is `custom`
- **THEN** the pane shows "prompt" as the wait reason

#### Scenario: a pi pane waits on a selection or input prompt

- **WHEN** a pi pane's wait reason is `select` or `input`
- **THEN** the pane shows "waiting for selection" or "waiting for input"

### Requirement: a blocking prompt is action-required

A wait reason that names a blocking user prompt SHALL hold the pane status at
`waiting` even while a background shell is live on that pane — the same class
as the permission reasons, because the pane needs the user just as much.

#### Scenario: a pi prompt under a live background shell

- **WHEN** a pi pane has a live background command and the producer reports a
  wait reason that is one of pi's prompt kinds (`select`, `confirm`, `input`,
  `editor`, `custom`)
- **THEN** the pane status becomes `waiting`, not `background`

#### Scenario: a soft reason still demotes

- **WHEN** a pane has a live background command and the reported wait reason is
  not a blocking-prompt or permission class (for example `auth_success`)
- **THEN** the pane status stays `background`
