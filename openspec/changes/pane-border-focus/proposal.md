# Proposal

## Why

The sidebar's keyboard shortcuts (j/k navigation, tab switching, filter typing) silently swallow keys meant for an agent pane whenever the sidebar's tmux pane is the active one. The only cues today are tmux's default active-pane highlighting, which the user finds too weak — the mistake is noticed only after keystrokes vanish. The previous fill-based attempt (sidebar-focus-fill) was rejected: the fill changed too little and the wrong place, and the user is often looking at an agent pane when the sidebar steals focus.

## What Changes

- When the sidebar pane is the active tmux pane, its pane border is drawn in a loud, dedicated focus colour; when it is not, the border uses tmux's normal styling.
- The signal is tmux-level, so it is visible from any pane in the window — including when the user is looking at an agent pane that is silently not receiving keys.
- One new colour option, `@sidebar_color_focus_border`, accepting a 256-color index or `#RRGGBB` hex, defaulting to a loud everforest orange (`#E69875`).
- Implementation is a single per-pane tmux option write at startup (`set -p pane-active-border-style` on the sidebar's own pane): tmux colours the active pane's border, so the sidebar tracks focus with zero per-tick work, no transition state, and no stale style after a crash (pane options die with the pane).

## Capabilities

### New Capabilities

- `focus-indication`: how the sidebar marks that its tmux pane is the active one, by colouring its own pane border through a per-pane tmux option.

### Modified Capabilities

<!-- none: no existing requirement changes. -->

## Impact

- `src/app/setup.rs` (or the TUI session setup path): one `set -p pane-active-border-style` write for the sidebar's own pane at startup.
- `src/tmux/commands.rs`: a small command builder for the per-pane border-style write.
- `src/ui/colors.rs` + `src/tmux/options.rs`: one new theme colour (`focus_border`) and its `@sidebar_color_*` option constant, loaded through the existing `from_options` path.
- Rendering-only code is untouched: the TUI buffer does not change, so all existing snapshots stay valid.
- tmux-level change: the sidebar pane's border reads differently from other panes' while it is active; user tmux config that sets `pane-active-border-style` at window/global level keeps working for every other pane (the sidebar pane overrides only itself).
