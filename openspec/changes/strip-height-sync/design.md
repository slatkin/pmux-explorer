# Design

## Context

The width lock (`state/width.rs`, `app/width_guard.rs`) already implements the
publish/adopt machine the spec delta asks for on the height axis: a shared
option, a seed-on-enable, a per-tick publish-or-adopt step with an adopt echo
suppressor and a write latch, convergence in hidden windows, and a creation
path that starts new panes at the shared value. Every sizing guard in the repo
deliberately excludes strips (`toggle_width_lock` and `width_guard::enforce`
early-return on `LayoutKind::Strip`; `size_guard::guard_target` returns `None`),
pinned by tests citing the `add-strip-layout` design decisions. `SidebarPaneInfo`
already carries `height` and `window_height`, so the strip axis needs no new
probe data.

## Goals / Non-Goals

**Goals:**
- One lock flag governing both axes; each instance syncs only the axis its fixed layout gave it.
- The width lock's safety semantics carry over exactly: seed-before-flag, floor both ways, adoption cap, echo suppression, latch retries, hidden-window convergence, failed-probe no-ops.

**Non-Goals:**
- No strip band guard (`@sidebar_min_height`/`@sidebar_max_height` options). A squeezed strip with no shared height stays squeezed, same as today.
- No change to column behavior; the width path's spec and code stay as they are.
- No per-window or per-pane height overrides; one server-wide shared height.
- No change to the split-strip internal layout or the tab-divider drag.

## Decisions

**D1 — Reuse `@sidebar_width_lock` as the single lock for both axes.**
A sidebar's layout is fixed for the life of its pane, so no instance is ever
column and strip at once; one flag cannot conflict with itself. Two flags
(`@sidebar_height_lock`) were rejected: they let the axes disagree, double the
toggle story in the UI, and buy nothing. The option's name becomes historical
(meaning: "the sidebar size lock"); `docs/state-management.md` records the
widened meaning and the name stays for compatibility — no migration.

**D2 — The shared height is absolute rows in `@sidebar_docked_height`.**
Percent heights keep working for unlocked creation, exactly as D1 of the width
lock froze percent widths under the lock: a shared value must not re-resolve
against each window's height, or the lock would not converge. A strip enabling
the lock seeds `max(live rows, floor)`; a strip whose probe failed
(`window_height == 0`) enables unseeded, mirroring the width path's
`window_width == 0` rule.

**D3 — The floor is the constant `DEFAULT_MIN_STRIP_HEIGHT = 4` rows, not an option.**
The floor's job is stopping collapse, not readability; the strip-layout spec
already lets filter chips drop whole when rows run out, so a 4-row strip is
cramped but valid. A `@sidebar_min_height` option was rejected for scope — it
can be added later if a user needs a taller floor. The floor binds publishes
and adoptions, and wins over the cap where both cannot hold, mirroring
`width_floor`. A floating strip with no
captured docked height enables the lock unseeded — the width path falls back
to the fixed `PANEL_WIDTH`, but a floating strip has no fixed height constant
to fall back to.

**D4 — The adoption cap is half the window's rows, adopted heights only.**
Direct mirror of the width cap, using `window_height`. The publisher's own
resize stays uncapped through the same ordering the width step uses: the
publish assignment lands before the adopt check reads the fresh shared value.

**D5 — Generalize the guard step instead of duplicating it.**
`width_guard::step` takes the pane's docked size on its axis (width for a
column, height for a strip) and the matching window dimension, and `enforce`
picks `resize-pane -x` or `-y` from the layout. One tracker per instance still
holds because the layout cannot change under a pane, so one echo suppressor
per instance remains correct. `GlobalState` gains `shared_docked_height` beside
`shared_docked_width`, with its own latch; `retry_pending_saves` retries both.

**D6 — Creation mirrors `shared_creation_width`.**
The strip branch of `cmd_toggle` reads the lock flag and `@sidebar_docked_height`
once at creation and starts the pane at the shared rows; unlocked, the existing
`resolve_height_arg` path is untouched. Like every creation read, the options
are read once, never per tick.

**D7 — Strip pop-in under the lock adopts the shared height.**
`state/popout.rs`'s strip dock path targets `@sidebar_docked_height` while the
lock is on and arms `pending_adopt_observation` for the landing, the same
handshake the column pop-in got (the D6 analog). Pop-out capture is unchanged:
the live docked height is still recorded for the unlocked restore.

**D8 — Seed order stays seed-first, flag-second.**
A peer adopting the lock mid-tick must find a shared height already in place,
the same D5 ordering the width seed follows.

## Risks / Trade-offs

- [Vertical space is scarce; a tall shared height adopted into a short window eats the agent pane] → the half-window cap, and the floor wins over the cap only below 4 rows.
- [Users of percent `@sidebar_height` see the percent stop applying while the lock is on] → documented; the width lock set the precedent, and unlocking restores percent behavior.
- [Mixed fleets around a position change: enabling the lock from one axis leaves the other axis's shared value stale or empty] → harmless; instances sync only their own axis, and an empty shared height makes the adopt side idle until a strip publishes or seeds.
- [`w` from a strip previously did nothing; it now resizes strips everywhere] → that is the feature; the lock glyph already shows the server-wide state.
- [`resize-pane -y` can be refused (zoomed window, crowded layout)] → the latch keeps the intent pending and retries on later ticks, the same as the width adopt.

## Migration Plan

None needed: the change adds one option and reuses the existing lock flag.
Rollback is toggling the lock off; strips keep their last heights and percent
creation resumes.
