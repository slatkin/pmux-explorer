## Why

The side the sidebar docks on (`left` or `right`) can only be chosen in the tmux config through `@sidebar_position`. Changing it means editing the config and recreating every sidebar. A user who wants the sidebar on the other side for one session has no way to move it.

## What Changes

- Add a side-switch control in the sidebar's top row, next to the pop-out control. It moves the docked sidebar to the opposite edge of the window.
- Add a keyboard shortcut for the same action: tmux prefix + `Ctrl+O` while the sidebar has focus. Outside the sidebar, the key keeps tmux's default action (`rotate-window`).
- The switch applies to the sidebar in every window at once, and sets the global `@sidebar_position` so new windows follow. The user's config is not changed.
- A window that cannot be moved (zoomed, or its sidebar is floating) is skipped and keeps its side. The sidebar tells the user how many windows moved and how many were skipped.
- While the sidebar is floating, the control is disabled. The floating panel keeps its right-edge anchor.
- The bottom panel's rule draws each tab as a Nerd Font glyph instead of a text label (Tools , Tasks , Git 󰊢, Thought 󰟶) and centers the tab run on the rule.

## Capabilities

### New Capabilities

### Modified Capabilities

- `sidebar-panel`: adds the side-switch control, its shortcut, the all-windows scope, the skip rule and the report.

## Impact

- `src/ui/panes/filter_bar.rs`, `src/state/layout.rs`: second control column in the top row, its drawing and hit test. Inline snapshots of the top row change.
- `src/tmux/commands.rs`, `src/tmux/options.rs`: `join-pane` argument builder and window enumeration.
- New `src/state/` module for the switch action; `src/app/input.rs`: the `M-C-o` key.
- `pmux-explorer.conf`: the `C-o` binding with the focus guard.
- `src/state.rs` (`BottomTab::label`), `src/ui/bottom.rs` (`top_line_border`): tab glyphs and a centered tab run; rule snapshots change.
- `docs/` and the website keybinding and tmux-option pages.
