# Tasks

## 1. Label and priority table

- [x] 1.1 Extend the wait-reason label table in `src/ui/text.rs` so pi's
  `ui_prompt` kinds get labels per the `wait-reasons` spec (`select` →
  "waiting for selection", `confirm` → "confirmation", `input` → "waiting for
  input", `editor` → "editor", `custom` → "prompt"); verify the
  `wait_reason_label` unit tests cover all five kinds plus the unknown-reason
  passthrough.
- [x] 1.2 Make the same reasons action-required for status priority per D2 —
  one per-reason entry driving both the label and the priority class (or a
  second match keyed on the same names) in
  `src/cli/hook/handlers/status_priority.rs`; verify tests show each pi kind
  resolving to `waiting` (not `background`) under a live `@pane_bg_cmd`, and
  a soft reason (`auth_success`) still demoting.
- [x] 1.3 Note in `docs/agent-protocol.md` (the `notification` /
  `@pane_wait_reason` wording) that the reason is a raw producer token shown
  through the label table; verify the wording matches the spec's
  pass-through rule.

## 2. Gates

- [x] 2.1 `cargo fmt`, `cargo clippy`, `cargo test` all green and
  `openspec validate wait-reason-labels --strict` clean.
