# Design

## Context

The Git tab is not a tmux pane. It is a `Rect` that pmux-explorer draws in its own single pane: `draw_bottom_content` (`src/ui/bottom.rs`) is the one entry point for every layout (column, one-pane strip, pane A of a split strip) and calls `git::draw_git_content` for the Git tab. tmux cannot put a second pane into that rect, so lazygit must run on a pseudo-terminal that the sidebar owns, and the sidebar must draw lazygit's screen into the rect.

The current state that the approach uses:

- The event loop (`src/app.rs`) polls crossterm with a timeout of 16 ms or less and drains worker channels with `try_recv` (`git_rx`, `version_rx`). A pty output channel fits the same slot.
- The git poll thread (`src/app/workers.rs`) runs every 2 s only while the Git tab is displayed (`git_poll_active`). It takes the path from `tmux::focused_pane_path`. On a focused-pane change, the main thread also runs one fetch (`refresh_git_for_focused_pane`).
- Each sidebar reads its own pane state once per tick (`SidebarPaneInfo`, `src/tmux/panes.rs`), including `window_active`.
- `@sidebar_bottom_tab` is shared, so every sidebar displays the Git tab when one selects it.
- The sidebar enables mouse capture but not bracketed paste or keyboard enhancement flags (`src/main.rs`). crossterm 0.29 decodes Ctrl-] (byte `0x1d`) as `KeyCode::Char('5')` with `CONTROL` (`event/sys/unix/parse.rs:110`).
- lazygit 0.66.3 has `--use-config-file` (a comma-separated list, later files win), `gui.sidePanelWidth`, `gui.mainPanelSplitMode`, `refresher.refreshInterval` (whole seconds), `refresher.externalChangeCheckInterval` (default 2 s) and `git.autoFetch`. Its layout code (`window_arrangement_helper.go`, `getMidSectionWeights`) gives the diff section weight 0 when `sidePanelWidth` is 1.0, except when `splitMainPanelSideBySide` is true, which `mainPanelSplitMode: vertical` prevents. Probes at 40×15 and 40×24 confirmed the list-only layout.

## Goals / Non-Goals

**Goals:**
- One self-contained lazygit host module behind a seam, so the event loop, the input layer and the renderer each call a small interface, and tests run without a real pty or lazygit.
- No behavior or cost change with `@sidebar_lazygit` `off`: no new thread, no new process, and the native poller as today.

**Non-Goals:**
- A pool of lazygit instances. One instance per sidebar, restarted on a repo change.
- A live switch of `@sidebar_lazygit` without a sidebar restart.
- Tuning lazygit's diff, staging or editor views. They work if the user opens them, but they are not designed for the panel size.
- lazygit's own OSC 8 links and images inside the embedded screen.
- Bracketed paste and the kitty keyboard protocol.

## Decisions

### 1. An embedded pty, not a tmux surface

The host runs lazygit on a pty and draws a screen model into the tab rect.

Alternatives considered: a `display-popup` (lazygit opens outside the tab, which the user did not want), and a sibling tmux pane put where the tab is drawn. The sibling pane breaks the sidebar's single-pane assumptions: `break-pane -W` floats only the sidebar pane, the side switch moves one pane between windows, the size sync resizes one pane, and the agent list must not list the sibling. Each would need a special case.

### 2. `vt100` for the screen and `portable-pty` for the pty, both pinned

`vt100` parses the byte stream into a cell grid with colours, attributes, wide-character flags, cursor state, application cursor mode and the mouse protocol mode and encoding that lazygit asks for. `portable-pty` opens the pty and spawns the child with a working directory and an environment. Both are pinned with `=` like the other dependencies.

Alternatives considered: `tui-term` (a ratatui widget over `vt100`), which couples to a ratatui version and the repo pins `ratatui =0.30.2`, while the cell copy it saves is about 60 lines. `alacritty_terminal` is much larger than this needs. Raw `openpty` and `fork` through `libc` is unsafe in a process that already runs threads. The existing `vte` dependency stays for `capture`.

### 3. Threading: a reader thread, a parser on the main thread

A reader thread reads the pty master and sends byte chunks over a channel. EOF sends an exit message. The main loop drains the channel with `try_recv` beside `git_rx`, feeds the bytes to the parser, and sets `needs_redraw`. The parser lives only on the main thread, so no lock is needed. Writes to the pty (keys, mouse, resize) are on the main thread. A key write is a few bytes and does not block in practice.

### 4. The config overlay

At startup with the option `on`, the host writes an overlay YAML file into the runtime directory (`$XDG_RUNTIME_DIR/pmux-explorer/`, else `/tmp/pmux-explorer-<uid>/`, mode 0600). The file sets:

```yaml
gui:
  sidePanelWidth: 1.0
  mainPanelSplitMode: vertical
  showBottomLine: false
  showRandomTip: false
  showCommandLog: false
  showPanelJumps: false
disableStartupPopups: true
update:
  method: never
git:
  autoFetch: false
refresher:
  refreshInterval: 2
keybinding:
  universal:
    suspendApp: <disabled>
```

The child gets `--use-config-file <user files>,<overlay>`. The user files are the value of `LG_CONFIG_FILE` when it is set. Otherwise the user file is `config.yml` in the directory that `lazygit --print-config-dir` prints, once at startup, if that file exists. The overlay comes last, so it wins only for the keys it sets, and the user's other settings and keys stay. `refresher.externalChangeCheckInterval` keeps its default of 2 s, which gives the 2 s ref refresh.

Alternative considered: a full generated config with no user file. This loses the user's keys and theme.

### 5. The child's command and environment

The command is `lazygit -p <repo top-level> --use-config-file <list>`, with the working directory at the repo top-level. The environment is the sidebar's, with `TERM=xterm-256color` and `COLORTERM=truecolor`. The pty size is the lazygit area: the tab rect minus the header row.

### 6. The host state machine

```
              option on, binary found, repo resolved,
              tab displayed, window visible
   +------+ ------------------------------------------> +---------+
   | Idle |                                             | Running |<--+
   +------+ <---- 10 s not visible (grace expired) ---- +---------+   |
      ^                                                   |     |     |
      |          repo top-level changed, stable 300 ms    |     |     |
      |          -> kill, spawn for the new repo ---------+-----+     |
      |                                                         |     |
      |                   child exits                           v     |
      |                                                    +--------+ |
      +---------- tab or window hidden ------------------- | Exited | |
                                                           +--------+ |
                                     Enter on the Git tab ------------+
```

The host keeps `wanted_repo`, a pending repo with its first-seen `Instant` (for the 300 ms debounce), and `hidden_since` (for the 10 s grace). The main loop calls `host.tick(now, inputs)` on every iteration, not only on the 1 s refresh, so the debounce has 16 ms resolution. The inputs are: the option, the binary, `git_poll_active()`, `window_active`, `session_attached` and the focused repo top-level. All time is passed in, so tests drive the machine with fixed `Instant`s. A path outside a repo resolves to no repo, and the view shows the not-a-repo notice.

The repo top-level is resolved on the main thread when the focused pane changes, from the same pane path the native tab uses (`tmux::get_pane_path`, then `git::repo_root`). That is one `git rev-parse` call per focus change, which is what `refresh_git_for_focused_pane` already costs.

`#{session_attached}` joins the format of the existing per-tick `display-message` read, so it adds no tmux call.

### 7. Capture and forwarding in the input layer

`handle_event` asks the host before its current dispatch:

- **Captured:** Ctrl-] (`Char('5')` with `CONTROL`, and `Char(']')` with `CONTROL` for terminals that report it that way) releases. Every other key is encoded with xterm legacy encoding and written to the pty: characters, the C0 control characters, Alt as an ESC prefix, Enter `\r`, Backspace `\x7f`, Tab, BackTab `\x1b[Z`, Esc, the arrows (`\x1bO` in application cursor mode, else `\x1b[`), Home, End, PageUp, PageDown, Delete, Insert and F1–F12. A mouse event in the lazygit area is translated to area-relative cells and encoded in the protocol mode and encoding that the screen reports (SGR or the default). A mouse mode of none sends nothing. A left press outside the area releases and then goes through the normal dispatch.
- **Not captured:** Enter, when the displayed Git tab has the keyboard focus (`Focus::ActivityLog` in a column or a one-pane strip, or pane A's focus in a split strip), captures, or starts lazygit again from `Exited`. A left press in the lazygit area captures and is forwarded. The press is the lazygit click. Everything else goes through the normal dispatch, unchanged.

A change of the displayed tab, the grace expiry or a child exit releases the capture. The capture state lives in the host, not in `Focus`, so the existing focus rules and their tests do not change.

### 8. Drawing

The lazygit view draws the header row, then copies the `vt100` screen into the frame buffer cell by cell. Colours map to ratatui colours (indexed and RGB). Bold, italic, underline and inverse map to modifiers. A wide character's continuation cell is skipped. A cell with the default background takes the sidebar's current section fill, so the `focus-indication` fills still cover the tab. While captured and while the screen shows the cursor, the cursor cell is drawn inverse.

The header row reuses `diff_stat_spans` and the PR `HyperlinkOverlay` path from the native header. While captured, the row takes the accent colour as its background.

The view records the lazygit area rect in the layout state each frame. The host compares the rect with the pty size and resizes both the pty and the parser when they differ, so a drag or a window resize follows within one frame.

### 9. The reduced poll

With the option `on`, the poll thread runs a header fetch in place of `fetch_git_data`: `rev-parse --abbrev-ref HEAD` (the `PrCache` key), `diff --shortstat`, `remote get-url origin` and the cached PR number. The thread keeps the 2 s period and the `git_poll_active` gate. The focus-change fetch on the main thread uses the same header fetch. The native file classification and `numstat` calls do not run.

### 10. Startup and the missing binary

`init_state` reads `@sidebar_lazygit` with the same pattern as the other startup-read options and looks up `lazygit` on `PATH`. With the option `on` and no binary, the view stays native, and `refresh_notices` adds one setup notice. The conf seeds `off`, and the conf/code drift test that pins `@sidebar_position` also pins the new seed.

### 11. Testing seam

The host takes a `PtySpawner` trait: spawn a command at a size and return a writer, a resizer, a kill handle and the output channel. Production uses `portable-pty`. Tests use a fake that records writes and sizes and lets the test push bytes and an exit. The key and mouse encoders are pure functions with unit tests. The state machine is tested with injected `Instant`s. The drawing tests feed fixed byte strings into a parser and snapshot the frame with inline `insta` snapshots, as `AGENTS.md` requires for every rendered frame.

## Risks / Trade-offs

- [Key fidelity: a key that the legacy encoder does not cover does nothing in lazygit] → The encoder covers lazygit's default bindings. Its unit tests list each one, and a key that is not covered is dropped, not sent as a wrong byte.
- [lazygit is too small: lazygit shows "not enough space" below its minimum size] → The tab already has a minimum height (`bottom-panel`, "each part keeps a minimum height"). Below lazygit's minimum, lazygit's own message shows, which is accurate.
- [Memory: one lazygit per visible sidebar, about 30–60 MB each] → The visibility gate and the 10 s grace limit it to the visible windows.
- [An editor opened from lazygit (`e`) runs inside the small screen] → It works but is cramped. This is a non-goal and is documented.
- [A wrong config path: lazygit's config directory discovery differs from `--print-config-dir`] → The host asks lazygit itself for the directory. When `LG_CONFIG_FILE` is set, it uses that value.
- [`vt100` misparses a sequence that lazygit uses] → lazygit uses a common xterm subset. The drawing tests include a captured lazygit frame as a fixture.
- [The runtime directory falls back to `/tmp`] → The overlay is written again at each start and holds no secret.

## Migration Plan

None. The option defaults to `off`. To roll back, set `@sidebar_lazygit off` and restart the sidebar.
