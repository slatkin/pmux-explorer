# Design

## Decisions

- **D1 — drop the task from the row, keep it in storage.** The fuzzy task is the pmux launch name with hyphens as spaces, and `buildPaneTitle` on the pmux side writes the same launch name into the pane title (`<name> — <task cut>`), which the status row shows in its title slot. With the name already on the status row, the task text duplicated it in a lighter colour. The three `@pane_subagent_*` options keep being written and parsed because the stored identity is producer-contract state described in `docs/agent-protocol.md`; only the row rendering changes.
- **D2 — no name-fallback on the row.** When the pane title never sets (an older pmux build without subagent titles), the row shows only `▷ <parent>` and the pane has no name in the sidebar. Keeping the task as a fallback would reintroduce the duplication this change removes, and the shipped pmux always writes the title; the gap is accepted.
- **D3 — cut machinery unchanged.** The row keeps the single-pass cut with per-piece colours, now over two pieces: the `▷ ` arrow and the parent label. The cut colour when the ellipsis lands inside the parent label is that label's colour, and the `unwrap_or` fallback moves to the last piece's index.

## Risks

- A test that asserted the task rendering exists in `src/ui/panes/row.rs` and `tests/ui_snapshot.rs`; both regenerate with the row parent-only. No other consumer of `SubagentInfo.task` exists in the codebase, so nothing else can drift.
