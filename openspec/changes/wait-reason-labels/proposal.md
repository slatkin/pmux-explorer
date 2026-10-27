# Proposal

## Why

The sidebar shows a pane's wait reason verbatim, and only Claude's notification
vocabulary is translated into readable text. pi's `ui_prompt` producer reports
the raw prompt *kind* (`custom`, `select`, `input`, `confirm`, `editor`), so a
pi pane blocked on a user question reads `custom` — a word that carries no
meaning. The same gap applies to any future producer's vocabulary, and to
Claude names that are still unmapped.

A second, related gap: the status resolver treats only Claude's
permission-class reasons as action-required, so a pi pane waiting on a blocking
prompt demotes to `background` whenever a background shell is live, even though
a blocking prompt needs the user just as much as a permission dialog.

## What Changes

- Extend the wait-reason display table with pi's `ui_prompt` kinds so a
  blocking extension prompt reads as something meaningful (`custom` →
  `prompt`, `select` → `waiting for selection`, `confirm` → `confirmation`,
  `input` → `waiting for input`, `editor` → `editor`). The same table feeds
  the pane row and the desktop-notification body, so both improve together.
- Classify pi's blocking-prompt reasons as action-required, so a pi pane that
  waits on a prompt keeps the `waiting` status instead of demoting to
  `background` when a background shell is live. Attention and the wait-reason
  row are already written; this is about the status word.
- Keep the raw reason as the protocol value in `@pane_wait_reason`; only its
  display label and its status-priority class change. No producer, adapter, or
  protocol change — new and unknown reasons still pass through unchanged.

## Capabilities

### New Capabilities

- `wait-reasons`: how the sidebar names why a pane waits — the shared
  raw-reason → display-label table for every producer's vocabulary, and which
  wait reasons count as action-required for status priority.

### Modified Capabilities

<!-- pi-agent-support's "waits for a human is marked" stays accurate: the
     behaviour (waiting status, attention flag, desktop notification) is
     unchanged; this change only names the reason better and keeps the status
     honest under a live background shell. -->

## Impact

- `src/ui/text.rs` — `wait_reason_label` gains the pi `ui_prompt` kind arms.
- `src/cli/hook/handlers/status_priority.rs` — the action-required allowlist
  gains pi's blocking-prompt reasons (or an equivalent agent-aware rule).
- `docs/agent-protocol.md` — the `notification`/`@pane_wait_reason` wording
  notes that the reason is a raw producer token shown through the label table.
- Tests: extend the existing `wait_reason_label` unit tests and the
  `status_priority` tests; a row snapshot if the label appears in a rendered
  fixture.
