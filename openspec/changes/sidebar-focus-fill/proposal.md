# Proposal

## Why

The sidebar looks identical whether or not its tmux pane is the active one. The only focus cue today is the `selection_bg` highlight on the selected pane's status and branch rows — a two-row tint that is easy to miss. There is no at-a-glance signal that keystrokes go to the sidebar, nor which section of the sidebar (agent list vs. bottom panel) owns them.

## What Changes

- When the sidebar pane is the active tmux pane, the whole sidebar is painted with a base focus background colour.
- The section that owns keyboard input is painted with a second, distinct background colour: the top section (filter bar, secondary header, agent list, pet band) while the focus is `Filter` or `Panes`, the bottom panel (rule plus tab content) while the focus is `ActivityLog`.
- When the sidebar pane is not the active tmux pane, no focus fill is painted — the sidebar renders exactly as it does today.
- The existing per-row `selection_bg` on the selected pane's marker rows still overrides the focus fill.
- Two new tmux colour options, `@sidebar_color_focus_bg` and `@sidebar_color_focus_active_bg`, with dark defaults that stay quieter than `selection_bg`.

## Capabilities

### New Capabilities

- `focus-indication`: how the sidebar shows that its tmux pane is active and which of its two sections owns the keyboard — a base fill over the whole sidebar plus a distinct fill on the focused section.

### Modified Capabilities

<!-- none: no existing spec's requirements change. The ActivityLog accent on the
     bottom-panel rule stays as is; the fill adds a signal, it does not replace
     a requirement. -->

## Impact

- `src/ui/mod.rs` (`draw`): two area fills applied before the existing section renders.
- `src/ui/colors.rs` + `src/tmux/options.rs`: two new theme fields and their `@sidebar_color_*` option constants.
- `pmux-explorer.conf`: no seeding needed (colour options resolve at startup with defaults); the `@sidebar_color_*` doc line already covers them.
- Rendering-only change: `sidebar_focused` and the `Focus` enum already exist and are maintained per refresh tick; no new tmux I/O.
- Tests: new render assertions for the fill combinations; existing insta snapshots of unfocused frames are unaffected.
