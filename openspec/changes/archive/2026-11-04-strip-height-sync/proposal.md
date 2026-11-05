# Proposal

## Why

The sidebar's width lock converges a column's width across every window, but a strip has no equivalent: each window's strip keeps whatever height `@sidebar_height` resolved to when it was created. A user who docks the sidebar at the bottom and switches between agents in different windows sees the strip's height jump on every switch, because each window resolved the height independently (and percent heights resolve against per-window sizes).

## What Changes

- Add a strip height lock that mirrors the column width lock: while the shared lock flag is on, every docked strip converges on one shared docked height within one refresh tick, including strips in hidden windows.
- The existing `@sidebar_width_lock` flag becomes the single lock for both axes — a strip's `w` key and its new lock control toggle the same lock every instance follows; a column syncs width, a strip syncs height. The option name stays (no migration; its meaning widens from width to size).
- A strip's control row gains the lock control at column 3 (`☑` locked, `☐` unlocked), after the side switch and the pop-out; a click toggles the lock like `w`.
- Enabling the lock seeds the shared value of the enabling instance's axis and clears the other axis's shared value, so instances on that axis keep their sizes until one of them publishes instead of adopting a value an earlier lock session left behind. An absent shared option reads as "no shared value" on every instance.
- A new shared option `@sidebar_docked_height` holds the shared strip height in absolute rows, written by the publishing strip and adopted by the others.
- A strip created while the lock is on starts at the shared docked height instead of resolving `@sidebar_height`.
- A strip popped back in while the lock is on docks at the shared docked height instead of its pre-pop-out height.
- A floating strip stays outside the lock: it keeps its size, publishes nothing, adopts nothing.
- Adoption is capped at half the window's rows and floored at a built-in minimum strip height (4 rows); the floor binds publishes and adoptions. Percent heights keep working while unlocked; under the lock the shared height is absolute rows only.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `strip-layout`: adds the height-lock behavior for strips — publishing, adopting, creation under the lock, pop-in under the lock, floating outside the lock, and the floor/cap rule. The existing "the strip height is a separate option" requirement keeps governing unlocked creation. The strip filter bar's control row gains the lock control, and enabling the lock from a column clears the shared height.
- `sidebar-panel`: a strip popped back in under the lock takes the shared docked height; the unlocked restore scenario gains its lock precondition.
- `shared-view-settings`: enabling the lock seeds the enabling instance's axis and clears the other axis; the shared-settings list names the docked strip height.

## Impact

- `src/state/width.rs` — `toggle_width_lock` stops early-returning for strips and seeds the shared height when a strip enables the lock.
- `src/app/width_guard.rs` — the step generalizes to the strip axis: publish/adopt on height with `resize-pane -y`, floor and cap in rows.
- `src/state/global.rs` — a shared-height field with its own latch, beside the width field; both ride the same lock flag.
- `src/cli/toggle.rs` — the strip creation branch reads the shared height under the lock, beside `resolve_height_arg`.
- `src/state/popout.rs` — the strip pop-in adopts the shared height under the lock and arms the adopt echo suppressor.
- `src/ui/panes/filter_bar.rs` — `draw_strip_filter_bar` draws the lock glyph at column 3 of the control row.
- `src/state/layout.rs` — `handle_filter_column_click` maps column 3 of the control row to `toggle_width_lock`.
- `src/state/global.rs` (also) — an absent `@sidebar_docked_width`/`@sidebar_docked_height` key adopts as `None`, and a cleared shared value is written with `set -gu`.
- `src/tmux/options.rs` — the `@sidebar_docked_height` option constant.
- `src/tmux/panes.rs` — the shared-height option joins the tick's shared options read.
- `docs/state-management.md` — the new option and the strip-side guard.
- Tests mirror the width-lock suite (publish, adopt, cap, floor, echo suppressor, latch, creation, pop-in) on the height axis.
