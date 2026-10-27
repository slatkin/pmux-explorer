# Design

## Context

See proposal.md — Why. tmux 3.8 removed `select-pane -P` but gained per-pane options (`set-option -p`), and the window-option list already carries `pane-active-border-style` ("the pane border style for the currently active pane"). Because pane options inherit from window options, writing `pane-active-border-style` on the sidebar's own pane overrides the border colour for that pane only, and only while that pane is active — which is precisely the focus signal. The sidebar already knows its own pane id (`self.tmux_pane`), and the theme already loads `@sidebar_color_*` options through `ColorTheme::from_options`.

## Goals / Non-Goals

**Goals:**
- One tmux write at startup; the running TUI loop stays untouched.
- The signal is visible from any pane in the window, since tmux draws the border.

**Non-Goals:**
- No per-tick or transition-driven border updates: tmux evaluates the option against pane activity itself, so tracking `sidebar_focused` in the sidebar would duplicate tmux's own work.
- No in-TUI rendering changes: the previous fill-based change was reverted and the TUI buffer stays byte-identical.
- No change to other panes: the per-pane write must not touch window-level `pane-border-style` / `pane-active-border-style`, so the user's existing tmux border styling keeps applying everywhere else.

## Decisions

### D1: A startup write of `pane-active-border-style` on the sidebar's own pane

`setup` runs `tmux set-option -p -t <own pane id> pane-active-border-style "fg=<colour>"` once when the TUI session starts. tmux then colours the sidebar's border exactly while the sidebar pane is the active pane of its window — the option is consulted by tmux on every border redraw, so focus transitions are reflected instantly by tmux itself, with zero sidebar involvement and no refresh-tick lag.

Alternatives considered:
- *Per-tick `select-pane`-style transitions*: needs transition state, fires 1 s late on missed SIGUSR1, and burns a tmux round-trip per change — all for what the option already gives.
- *`select-pane -P`*: gone in tmux 3.8; `-P` also set pane content style, which would tint the pane background — the rejected fill again.
- *Status-bar tint*: global restyling, heavy-handed; rejected in favour of the pane border.

### D2: Failure of the write is silently tolerated

The option write is cosmetic; if tmux rejects it (old tmux without per-pane options, exotic socket states) the sidebar still works. `run_tmux` already returns `Option` and existing code discards cosmetic failures the same way.

### D3: The colour rides the existing theme pipeline

`ColorTheme` gains `focus_border`, loaded from `@sidebar_color_focus_border` through `from_options` with the same valid-or-fallback semantics as every other colour. Default `#E69875` (everforest dark-medium orange): loud against dark backgrounds without reading as an error.

### D4: Stale styling is impossible by construction

Pane options are attached to the pane, and the sidebar process *is* the pane process: if it exits, the pane closes and the option goes with it. If tmux keeps the pane (remain-on-exit), the pane is inactive, so `pane-active-border-style` is not consulted — the border falls back to normal styling. No sweep or teardown path is needed.

## Risks / Trade-offs

- [A tmux older than per-pane options ignores `set -p`] → the write fails silently (D2); the user keeps today's behaviour. The plugin already documents a tmux floor; no version probing.
- [The user's own tmux config styles the active border deliberately] → the sidebar pane overrides only itself; every other pane keeps their styling. If they dislike the sidebar's border, `@sidebar_color_focus_border` accepts their value, and the option is theirs to set.
- [The border colour alone may still be missed in a busy window] → it is a dedicated, always-on, high-contrast frame around exactly the pane that owns the keyboard; stronger than tmux's default dim active-border highlight and visible without looking inside the sidebar.
