# Design

## Context

See proposal.md - Why for the problem. The facts that shape the approach:

- The per-tick pane reading already carries everything the lock needs:
  `get_sidebar_pane_reading` (`src/tmux/panes.rs`) expands `#{pane_width}
  #{window_width} #{window_height} #{pane_floating_flag}` plus every entry
  of `SHARED_OPTION_FIELDS` in one `display-message` call, and the query
  string and the parse both derive from that list. Two new space-free
  options slot in before the repo filter, which must stay last for its
  `/eot/` terminator parse.
- `GlobalState` (`src/state/global.rs`) owns the shared settings —
  `status_filter`, `selected_pane_row`, `repo_filter`, `bottom_tab` —
  each written with `set -g @sidebar_*` and adopted in `apply_all` under
  a `last_saved_*` guard, with a pending-write latch so a rejected write
  is retried at the top of the next tick instead of being reverted.
- `size_guard::enforce` (`src/app/size_guard.rs`) runs each tick in
  `app::run`, gated on `is_window_active`: it grows a pane squeezed below
  `@sidebar_min_width` (default 20, `0` disables) and never shrinks.
- `popout.rs` captures `docked_width` on pop-out and restores it on
  pop-in; the floating panel is a fixed `PANEL_WIDTH = 40`.
- `cmd_toggle` (`src/cli/toggle.rs`) reads `@sidebar_width` (default `15%`)
  once at pane creation for `split-window -l`; nothing writes it at
  runtime.
- tmux never reports who resized a pane. `pane_width` is all an instance
  gets, so a user's drag and a layout rebalance are indistinguishable —
  the publish rule below is built around that.

## Goals / Non-Goals

**Goals:**
- One docked width per tmux server while the lock is on, converging within
  one refresh tick, including windows the user is not looking at.
- No extra tmux subprocess per tick: the lock state and shared width ride
  the existing `display-message` read; writes happen only on change.
- Reuse the proven shared-setting machinery (`last_saved_*` guards,
  pending-write latches, retry-then-read tick order) rather than a new
  channel.

**Non-Goals:**
- Syncing the floating panel's width — it stays a fixed 40 columns.
- Syncing across tmux servers; the server remains the boundary.
- A push channel (SIGUSR1 broadcast, sockets). The 1s poll satisfies the
  one-tick requirement, and hidden windows converge before they are seen.
- Distinguishing a user resize from a tmux reflow beyond the floor
  heuristic — tmux exposes no such signal.
- Percentage-based syncing; the shared width is absolute columns (a
  percentage of differently-sized windows would still jump).

## Decisions

**D1: Two new options, kept apart from `@sidebar_width`.**
`@sidebar_width_lock` holds the shared on/off state; `@sidebar_docked_width`
holds the synced width in absolute columns. The existing `@sidebar_width`
stays the creation-time default and keeps its percentage support — writing
the runtime width into it would clobber a configured `15%` and blur the
config/runtime split.
*Alternative:* reuse `@sidebar_width` for the synced value. Rejected: it
may legitimately hold `15%`, which is not a column count, and rewriting it
changes what a not-yet-created sidebar would get in ways the user did not
ask for.

**D2: All three options ride `SHARED_OPTION_FIELDS`, before the repo
filter.** The list becomes `[SIDEBAR_FILTER, SIDEBAR_CURSOR,
SIDEBAR_BOTTOM_TAB, SIDEBAR_DOCKED_WIDTH, SIDEBAR_WIDTH_LOCK,
SIDEBAR_MIN_WIDTH, SIDEBAR_REPO_FILTER]`. Column digits and `on`/`off`
contain no space, so the space-split parse stays safe, and the repo
filter keeps its terminator-based last-field parse. `@sidebar_min_width`
rides along because the lock needs the floor every tick in every window —
`size_guard`'s own `show -gv` only runs while the window is active, and
the active-window gate is exactly what the lock drops. Both guards then
read the floor from the tick's options map and `size_guard`'s separate
read goes away. The new middle fields are space-free by contract:
`@sidebar_min_width` is user-settable, but its documented values are
digits (like `on`/`off` for the lock), and a space-bearing value would
shift the positional parse — the same class of foot-gun as writing text
into `@sidebar_width`, accepted and documented.
*Alternative:* a separate `show -g` per tick. Rejected: one extra tmux
process per second for values that already have a channel.

**D3: `width_lock` and `shared_docked_width` live in `GlobalState`.**
`width_lock: bool` (adopted `on`/off) and `shared_docked_width:
Option<u16>` (`None` while no shared width exists) join the struct with
`last_saved_*` guards and `*_save_pending` latches, verbatim copies of the
filter/tab latch shape the spec already pins. The field is named
`shared_docked_width`, not `docked_width`, because `AppState` already
owns a `docked_width` — the pop-out capture (D6) — and two fields with
one name on sibling structs invite cross-wiring.
*Alternative:* keep them on `AppState` next to `sidebar_pane`. Rejected:
the adoption guard would live in one struct and the value in another, and
a writer could bypass the save path by assigning the field — the same
argument that put `bottom_tab` in `GlobalState`.

**D4: One width-lock step per tick decides publish vs adopt.**
After `state.refresh()` (which adopts `width_lock`/
`shared_docked_width` into `GlobalState`), a `width_guard`-style step
runs for every docked sidebar — NOT gated on `is_window_active` like
`size_guard`, because the spec requires convergence in hidden windows.
It keeps two pieces of cross-tick state: `last_observed_width:
Option<u16>` and `pending_adopt_observation`, both described below.
With `S` the shared width, `floor` the parsed `@sidebar_min_width`, and
`cap = max(floor, window_width / 2)`:

- `last_observed_width` is `None` until the first docked reading, which
  PRIMES it: a `None` tracker records `W` and can never publish. Only a
  width that changed since a recorded observation counts as a change —
  without priming, a fixed initial value would read the pane's startup
  width as a change on the first tick and publish it (a creation width
  tmux itself clamped would demote `S` server-wide, the same defect the
  adopt flag exists to prevent). It updates on every docked tick,
  locked or not — a tracker that sleeps while unlocked would read a
  stale `W` as a fresh change the tick the lock arrives.
- `target = min(max(S, floor), cap)` — the floor binds both ways: an `S`
  that is somehow below the floor (a seed written while the enabling
  pane was squeezed, a hand-edited option) is adopted at the floor, not
  at `S`. `window_width == 0` means the probe failed, so the step does
  nothing (the same "unknown size = do not act" rule the other guards
  follow).
- **Publish**: `W != last_observed_width` (something resized the pane),
  `W >= floor`, and no `pending_adopt_observation` → the GlobalState
  field is assigned `S = W` FIRST, then written with
  `set -g @sidebar_docked_width W`. Assigning before the adopt check
  matters: the adopt below then computes `target` from the fresh `S`,
  and a just-published width yields `target == W`, never a revert of
  the pane the user just resized.
- **Adopt**: `S` set, `width_save_pending` clear, `W != target`, and
  `W != S` → `resize-pane -x target` and set
  `pending_adopt_observation`. The `W != S` term exempts the
  publisher's own pane from the cap: a deliberate drag past half the
  window publishes and STAYS on the window where it happened, while
  adopting peers clamp — the spec caps the adopted width, not the
  user's hand. The exemption is dead as soon as `S` moves (`W != S`
  again), so it cannot hold a pane at a stale width.
- `last_observed_width = W` at the end of the step.

`pending_adopt_observation` is the echo suppressor, evaluated on every
tick the step runs — locked or not: the first pane-width change observed
after this instance issued a resize is its own adoption landing, so the
flag is consumed and that change is never published — whether the pane
lands exactly on `target`, clamps short of it (a 100-column window
taking 50 of a shared 80), or anywhere else. If no change arrives by
the next tick, the resize was refused and the flag is cleared. Because
the flag is consumed-or-cleared every running tick, one set by an
unlocked pop-in (D6 sets it there too) cannot linger and eat the first
resize after a later enable. A flag keyed to the issued resize beats
comparing `W` to the clamped target: an exact-match rule would publish
a partially-applied landing (target 45, pane lands 40) and demote `S`
for the whole server.

**The adopt side is suppressed while `width_save_pending` is set.** A
rejected publish leaves tmux holding the stale shared value this
instance is trying to replace; the field latch in `apply_all` protects
the GlobalState copy, but the pane is a second copy of the same
information. Adopting the stale `S` during the pending window would
`resize-pane` the sidebar back over the user's resize — the exact
revert the spec forbids — so the adopt check reads the same latch. The
pending write keeps retrying at the top of each tick; once it lands,
`S` equals `W` and the step is idle.

**`size_guard` is unchanged and keeps running, locked or not.** The
locked-plus-unseeded state (a command-line `@sidebar_width_lock on`
with no width, or an enabling write still pending) has no `S` to adopt,
so the lock step alone could never recover a squeeze — skipping
`size_guard` there would regress the shipped floor. When the lock step
and `size_guard` both fire in a squeeze tick they cannot fight:
`size_guard` only grows to the floor and the adopt target never goes
below it, and the step runs after `size_guard`, so its `resize-pane`
to `target` lands last.

*Alternative:* suppress echo by comparing `W` to `clamp(S)` instead of
the outstanding-resize flag. Rejected twice over: it publishes a
clamped landing back over `S`, and it swallows a user's deliberate
resize to exactly the clamped value.

**D5: Enabling the lock writes width first, then the flag — and the
seed is the DOCKED width, floored.** The `w` key and the lock control
both call a `toggle_width_lock` state method that, on enable, writes
`set -g @sidebar_docked_width <seed>` and then
`set -g @sidebar_width_lock on`, in that order so a peer adopting
`lock=on` mid-tick always finds `S` already there. The seed is the
enabling pane's docked width floored at `@sidebar_min_width`
(`max(W, floor)` — a squeezed pane never propagates its squeeze). The
key is reachable while floating (the filter bar renders in the panel),
so the floating case is pinned too: a floating instance seeds the
`docked_width` it captured on the way out — floored the same way — or
`PANEL_WIDTH` when no capture exists (a pane floated from the tmux
command line), never the live 40-column pane width. The seed is written
only on a valid probe: with `sidebar_pane.window_width == 0` the last
reading was the FALLBACK (probe failed, pane possibly gone), so the
enable writes the lock flag alone and leaves the width unseeded rather
than seeding the fallback's 28 columns. A lock enabled from
the tmux command line with no width is left unseeded: the first local
resize publishes — deterministic, and avoids every instance racing to
seed different widths. Unlocking writes `@sidebar_width_lock off`; the
stale `@sidebar_docked_width` value stays behind harmlessly and is
reused if the lock returns.
*Alternative:* every instance seeds on adopting `lock=on` with `S` unset.
Rejected: N instances publish different widths in the same tick and the
server flickers before converging.

**D6: The pop-in restore uses the shared width, clamped like an
adoption.** `apply_pop_in` restores `min(max(S, floor), cap)` when the
lock is on and `S` is set — not raw `S`, since a narrow window would
otherwise dock oversized for a tick before the adopt step clamps it —
else the captured `docked_width` as today. The restore counts as the
instance's own resize, so `apply_pop_in` also sets
`pending_adopt_observation`: the width change the next tick observes is
the restore landing, and without the flag it would echo the restored
width back as a publish. The capture on pop-out still runs
unconditionally — it is cheap, and it is the right answer whenever the
lock is off at pop-in time. While floating, the pane neither publishes
nor adopts: `pane_floating_flag` gates the whole D4 step, so the fixed
40-column panel can never feed itself into the shared width.
*Alternative:* let the adopt step fix the width on the tick after pop-in.
Rejected: one visible frame at the stale width, when the correct value is
already in hand.

**D7: `cmd_toggle` prefers the shared width while locked.**
Before resolving `@sidebar_width`, read `#{@sidebar_width_lock}` and
`#{@sidebar_docked_width}`; locked + a parseable width → use it directly
for `split-window -l`. Otherwise the existing percentage/absolute logic
runs unchanged.
*Alternative:* let the first tick's adopt step correct a new sidebar.
Rejected: every new window would open at the wrong width and visibly jump.

**D8: The lock control is a glyph beside the pop-out control in the
filter bar.** `render_filter_bar` (`src/ui/panes/filter_bar.rs`) already
owns the right-edge control column via `filter_bar_layout`'s
`control_col` (`src/state/layout.rs`), and `handle_filter_click`
hit-tests it — a lock glyph goes one cell left of the pop-out glyph, the
layout's control region widens by that cell, and the same click path
calls `toggle_width_lock`. The `w` key (unused today) calls it too. Two
glyphs — one for locked, one for unlocked — so the state is visible at a
glance. Existing tests pin `control_col = width - 2`; they shift by one
cell with the new glyph.
*Alternative:* a tmux-only option with no in-TUI affordance. Rejected:
the user asked for a toggle; burying it in `tmux set -g` fails that.

## Risks / Trade-offs

- [A deliberate resize below the floor is eaten] → Accepted: any width
  below `@sidebar_min_width` reads as a squeeze — the same interpretation
  `size_guard` makes today — and the pane is grown back toward `target`.
- [`@sidebar_min_width = 0` removes squeeze detection entirely] → With no
  floor, every observed width publishes, so a layout squeeze becomes the
  new shared width. Documented limitation of disabling the floor while
  locked.
- [Two windows resized in the same tick] → Last writer wins on the shared
  option; both converge within a tick. No ping-pong: adoption only fires
  while `W != target`, and a published value equals the pane's own width.
- [A narrower window adopts the cap, diverging from peers] → Inherent to
  absolute columns; the pane reconverges to `S` on the tick after the
  window grows, because `cap` is recomputed every tick.
- [`resize-pane -x` refused or clamped mid-convergence] → `W` stays or
  lands short, `W != target` persists, and the step retries next tick —
  the same bounded retry the other guards accept, and the one-shot flag
  keeps a short landing from ever becoming a publish.
- [A manual resize lands in the same tick as an adoption] → The one-shot
  flag suppresses it once; the user's next resize publishes normally.
- [A publish racing the adopt check in one tick] → Cannot revert: the
  publish path assigns `shared_docked_width = W` BEFORE the adopt check
  reads `S` — a just-published width makes `target` equal `W` on the
  spot, so the pane the user resized is never grown or shrunk back. The
  ordering is load-bearing and pinned in D4's sequence.

## Migration Plan

No data migration. On upgrade both options are unset: every instance runs
unlocked, and turning the lock on for the first time seeds the width then.
An older binary ignores the two unknown options and behaves as today.
Rollback is free; a leftover `@sidebar_docked_width` is ignored while
unlocked.

## Open Questions

None — the spec fixes the shared-ness, floor, cap, floating exclusion and
UI affordance; the remaining choices are resolved in Decisions.
