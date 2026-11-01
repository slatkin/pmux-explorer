# Design

## Context

See proposal.md — Why. The plumbing already exists: `FocusState::sidebar_focused` mirrors the sidebar pane's `pane_active` (read every refresh tick in `SidebarPaneReading`, applied in `apply_session_snapshot`), and the `after-select-pane`/`after-select-window` tmux hooks signal the process with SIGUSR1 on every pane selection, so focus transitions already force an instant redraw in both directions. `FocusState::focus` (Filter / Panes / ActivityLog) is the keyboard-owner signal. `draw()` in `src/ui/mod.rs` already splits the frame into `chunks[0]` (top: filter bar, secondary header, agent list, pet band) and `chunks[1]` (bottom panel) — exactly the two sections the feature needs.

## Goals / Non-Goals

**Goals:**
- Paint the two focus fills by adding two area fills at the top of `draw()` — nothing else in the render tree changes.
- Both fills are `@sidebar_color_*` options with defaults, like every other colour.

**Non-Goals:**
- No change to how focus or `sidebar_focused` is detected.
- No change to `selection_bg`, the `┃` active-pane marker, or the bottom rule's ActivityLog accent — it stays; the fill adds a signal rather than replacing one.
- No tmux-level pane styling (`select-pane -P`): an untracked pane style survives a crashed sidebar process and is invisible to the render tests.

## Decisions

### D1: Fill whole chunk rects in `draw()`, not per-row

`draw()` fills `chunks[0]` and `chunks[1]` with `Block::default().style(Style::default().bg(colour))` before calling `panes::draw_agents` / `bottom::draw_bottom` / `pet::draw_pet`. Ratatui applies styles as a patch — a span with no explicit `bg` keeps the cell's painted background — so every glyph keeps its own foreground over the fill, and cells nothing draws on still get the fill (blank rows under the last pane included).

Alternatives considered:
- *Per-row `bg` through `RowCtx`*: touches every row helper, misses repo headers, separators, and blank space — and the spec wants the whole section filled.
- *`Paragraph::style` on the list*: same fill only reaches the list widget, not the filter bar, header, pet band, or bottom panel — two more patches for no gain.

### D2: Which section is "active" is a `Focus` decision, not an area test

The keyboard owner is `Focus::ActivityLog` → bottom section; `Filter` or `Panes` → top section. When the bottom panel is hidden (`@sidebar_bottom_height 0`, so `chunks[1]` does not exist), the top is the only place input can land, so the top section gets the active fill regardless of `focus`. This keeps the fill truthful about where keystrokes go.

### D3: The selected pane row keeps `selection_bg`

No special-casing needed: the marker rows of the selected pane already paint spans with an explicit `selection_bg`, which patches over the fill. The two-level hierarchy (fill < selection) is a consequence of patch semantics, not extra code.

### D4: Two new colour options, quiet defaults

`ColorTheme` gains `focus_bg` and `focus_active_bg`, loaded from `@sidebar_color_focus_bg` / `@sidebar_color_focus_active_bg` through the existing `from_tmux` path. Defaults `Indexed(234)` (base) and `Indexed(236)` (active): both sit below `selection_bg` (Indexed 239) so the cursor still pops, and the two fills are distinct on a dark terminal.

### D5: Popups and the flash banner punch through unchanged

Both already render after the list with `Clear`, which resets cells to the terminal default — they keep their current look over any fill.

## Risks / Trade-offs

- [A user sets a fill at or above `selection_bg` prominence and loses the cursor contrast] → defaults keep the ordering; the conf doc line already invites overrides — a one-line note beside the options suffices.
- [The base fill over the pet band makes the pet scene harder to read] → the pet is drawn with fg colours over the fill like everything else; if it reads badly the fix is a user colour choice, not code.
- [Focus-out lag if SIGUSR1 is missed] → the per-tick `pane_active` read corrects any missed signal within one refresh interval (1 s).

## Migration Plan

Pure rendering change with new opt-in colours: default on, no migration, no config steps. Rolling back means reverting the two fills; the theme fields are then dead but harmless.
