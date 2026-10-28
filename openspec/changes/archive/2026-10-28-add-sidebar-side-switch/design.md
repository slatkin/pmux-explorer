## Context

See proposal.md for motivation. Today `@sidebar_position` is read only in `cli/toggle.rs` when a sidebar pane is created (`split-window -hfb` for left, `-hf` for right, aimed at the outermost pane). Every window has its own sidebar pane. A sidebar can identify its own pane and reads window facts through `tmux::get_sidebar_pane_reading`. The pop-out control is hit-tested in `state/layout.rs` against the column that `filter_bar_layout` returns.

## Goals / Non-Goals

**Goals:**
- Move live sidebars without restarting the TUI process or losing its state.
- Keep one source of truth for the side: the global `@sidebar_position`.

**Non-Goals:**
- Writing the user's tmux config.
- Moving a floating panel, or adding a configurable key option.
- Per-window sides.

## Decisions

**Move with `join-pane`, not kill and recreate.** For each window, run `join-pane -h -f [-b] -s <sidebar> -t <edge pane>`, where `-b` is used for a move to the left and the target is the outermost workspace pane on the new side (reuse `target_pane_for_position` from `cli/toggle.rs`). The sidebar process keeps running and only gets a resize. Alternative: kill and recreate through `toggle`. That restarts every TUI and loses state, so it is rejected.

**One action in the sidebar that was clicked.** The action runs in the TUI that received the click or key. It lists all panes with one `list-panes -a` call (window id, pane id, role, floating flag, zoomed flag, geometry), finds each sidebar by its `@pane_role`, and moves them one by one. Alternative: a new CLI subcommand called from tmux. It is not needed because the hotkey already reaches the TUI as `M-C-o`, like `M-C-p`.

**Set the option after the loop, from the result.** The new side is derived from the clicked sidebar's current side. `@sidebar_position` is set with `set-option -g` once, even when some windows were skipped, so new windows follow. Skipped windows keep their old side; this can leave a mixed layout until the next switch.

**Skip rule is a zoomed flag, then tmux's answer.** A floating sidebar is skipped before any call (the flag is in the listing), and so is a zoomed window: tmux 3.8's `join-pane` does not refuse a zoomed window — it unzooms it and succeeds — so the sweep carries `#{window_zoomed_flag}` per window and skips it before the call, leaving the window's zoom and side untouched. For all other windows, a failing `join-pane` counts as skipped.

**Disabled while floating.** The glyph is drawn dimmed and the click and key do nothing. The floating geometry stays hard-coded to the right edge (`panel_geometry`).

**Top row layout.** `filter_bar_layout` reserves the pop-out control's two columns and now also the side-switch control's two columns, left of it. Chips that no longer fit are dropped whole, as today. The click test checks both columns before the chip walk.

**Key.** `pmux-explorer.conf` binds `C-o` with the same guard as `C-p`: `if-shell -F '#{@sidebar_pid}' 'send-keys M-C-o' 'rotate-window'`. `input.rs` maps `M-C-o` to the action, and ignores it while a popup is open.

**Icons replace the tab labels.** Each tab draws one Nerd Font glyph from `BottomTab::label()` (Tools , Tasks , Git 󰊢, Thought 󰟶) — the same strings the hitbox walk reads, so a glyph and its hitbox cannot drift apart. The persisted `@sidebar_bottom_tab` values stay lowercase words (`activity`, `tasks`, `git`, `thought`); only the drawing changes.

**The tab run centers on the rule.** `top_line_border` pads `floor((width - run) / 2)` columns on the left and the remainder on the right, so the rule stays exactly `width` long. A run wider than the width gets no pad and truncates against the full width, as today. The hitboxes stay the drawn columns, so centering cannot desync the clicks.

## Risks / Trade-offs

- [A skipped window keeps the old side, so sides can differ across windows] → The report names the skip count. A second switch is not blocked.
- [`join-pane` of the pane that runs the code that calls it] → The process is not killed; tmux only moves the pane. Verify with a test against a real tmux in the task list.
- [The top row is narrow, so a second glyph drops more chips] → Chips drop whole and the layout function is shared by drawing and hit test, so no partial chip is shown.
- [The setting resets to the config value after a tmux restart] → Accepted. Documented in the tmux option reference.
- [Icon glyphs need a Nerd Font terminal] → The sidebar already uses Nerd Font glyphs elsewhere (filter chips, agent icons); the tabs now do the same.
