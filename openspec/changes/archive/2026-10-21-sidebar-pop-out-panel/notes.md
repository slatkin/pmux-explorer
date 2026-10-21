# Live verification notes

Observed on 2026-10-21 against a live tmux 3.8 with the debug binary built
from the implementation HEAD (`d30c195`). The run used a dedicated tmux
session (`sidebar-popout-check`), 120x40 as requested; tmux created it with
`base-index 1`, so the test window is index 1 (`@16`), 120x40 (39 pane rows
plus the status line). The workspace pane `%148` was marked with
`@pane_agent=claude` and `@pane_status=running`; the sidebar pane `%150` was
created with `split-window -h -l 25` running the worktree binary. (A
sidebar auto-spawned by the local plugin config in the test window was
killed before the run so only the pane under test was present.)

## 5.1 — the round trip

Starting geometry (docked):

```text
%148  94x39  left=0    float=0   (workspace)
%150  25x39  left=95   float=0   (sidebar)
```

After `p` and one refresh (floating):

```text
%148  120x39  left=0    float=0   (workspace takes the full width)
%150   38x38  left=81   float=1   (sidebar, floating)
```

`pane_width`/`pane_height`/`pane_left`/`pane_top` for a floating pane report
the content area: 38x38 of content at (81,1), with tmux's border drawn
outside it. The outer box is therefore 40x40 spanning columns 80..119 and
rows 0..39 — 40 columns wide, right edge flush with the 120-column window
(120 - 40 = 80, as `panel_geometry` computes), as tall as the window. The
content capture is 38 columns wide, matching the design's measured
transcript (`-x 40` gives 38 columns of content).

After the second `p` (docked again):

```text
%148  94x39  left=0    float=0
%150  25x39  left=95   float=0
```

The sidebar is a tiled pane again and came back at exactly the 25 columns it
had before the pop-out; the workspace pane is back to its 94-column share.

Pane captures before and after: the filter row is identical
(`≡5 ●2 ◐0 ○3 ✕0` both times, `⤢` in both docked captures), the selected
entry (`/skill:code-review PR 3`, first in the list) and the two repo rows
are unchanged apart from live data arriving from other sessions during the
run (new subagent rows appeared, one repo entry scrolled off — outside the
test), and the bottom tab reads `╭ Activity │ Git ─…` in both captures with
the same two tabs in the same order. The only expected difference between
the docked and floating captures besides line wrapping is the control glyph,
which is `⤢` docked and `⤡` floating. No scroll had been applied, so the
list starts at the top in both captures.

## 5.2 — the zoomed window

With `resize-pane -Z` on the sidebar pane (zoomed to 120x39) and a `p`:

```text
float=0 zoomed? size=120x39
```

and the captured text contains the flash banner:

```text
Pop-out refused: can't float a pane while window is zoomed
```

The sidebar stayed docked and reported the refusal, as the spec requires.

After `resize-pane -Z` again (un-zoomed), the sidebar still works: a `p`
floated it (`%150 38x38 left=81 float=1`) and a second `p` docked it back
(`%150 25x39 left=95 float=0`).

## 5.3 — the gates

Run in a plain clone of the branch at `d30c195`
(`/tmp/sidebar-popout-gate`), because three `group::tests` tests fail in a
worktree for reasons unrelated to this change:

| command | result |
| --- | --- |
| `cargo fmt --all -- --check` | pass, no output |
| `cargo test` | 1373 passed, 0 failed |
| `cargo clippy --workspace --all-targets` | 0 errors, 5 warnings |
| `cargo fmt --all` | no changes |

The clippy warnings: the four pre-existing ones the change inherited
(`src/tmux/query.rs:214` and `:215` collapsible `if`s, `src/group.rs:145`
`sort_by_key`, `src/desktop_notification.rs:331` unneeded `return`) plus one
test-only warning, `tests/state_tests.rs:1158` "variable does not need to be
mutable" on the `let mut click` closure introduced by `60a77a2`. Plain
`cargo clippy` (what CI runs) reports only the four pre-existing warnings;
the fifth appears only with `--all-targets`.

## 5.4 — the resize behaviour (added after the code review)

Observed on 2026-10-21 against a live tmux 3.8, in the same dedicated
detached session (`sidebar-popout-check`, 120x40) as the checks above,
against a debug build of the branch at `89c94c1`.

Docked, before the pop-out:

```text
%150  25x39  left=95   float=0
```

After `p` and one refresh (floating):

```text
%150  38x38  left=81   float=1
```

38 columns of content — 40 columns of outer box, tmux draws the border
outside the content area as in 5.1 — at left=81, flush with the right edge
of the 120-column window, since 120 - 40 = 80.

The window was then resized to 100 columns:

```text
%150  38x38  left=61   float=1
```

The panel stayed 38 wide and moved to left=61 — flush with the new right
edge, since 100 - 40 = 60.

After a second `p` (docked again):

```text
%150  25x39  left=95   float=0
```

The pane is a tiled pane again at the 25 docked columns it had before the
pop-out.

The top row of the floating sidebar rendered the five chips with the
control glyph at the right edge:

```text
 ≡0 ●0 ◐0 ○0 ✕0         ⤢
```
