# Tasks

## 1. The option, the startup read and the missing-binary fallback

- [ ] 1.1 Add the `@sidebar_lazygit` constant in `src/tmux/options.rs` (and its re-export in `src/tmux.rs`), a `lazygit_enabled_from_options` reader in `src/ui/mod.rs` (`on` is on; any other value, empty or absent is off), and the startup read in `init_state` (`src/app/setup.rs`) into a new `AppState` field. Verify with unit tests: absent, `off`, `on` and an unknown value (`yes`).
- [ ] 1.2 Seed `@sidebar_lazygit off` in `pmux-explorer.conf` with the `if -F` pattern, document it in the conf header, and extend the conf/code drift test that pins `@sidebar_position` to pin the new seed. Verify with `cargo test` on the drift test.
- [ ] 1.3 Look up `lazygit` on `PATH` at startup. With the option `on` and no binary, keep the native view and add one setup notice in `refresh_notices` that names the missing executable (design decision 10). Verify with unit tests: option on and no binary gives the notice and the native view, option on with a binary gives no notice, option off gives no notice and no lookup.
- [ ] 1.4 Document the option in `website/src/content/docs/reference/tmux-options.md` and `docs/state-management.md` (the field, read once at startup). Verify that the documented name and default match the code.

## 2. Dependencies and the pty seam

- [ ] 2.1 Add `vt100` and `portable-pty` to `Cargo.toml`, pinned with `=` like the other dependencies (design decision 2). Verify that `cargo build` succeeds and `Cargo.lock` holds the pinned versions.
- [ ] 2.2 Create the lazygit host module with the `PtySpawner` trait (spawn a command with a working directory, an environment and a size, and return a writer, a resizer, a kill handle and an output channel that ends with an exit message) and a fake spawner for tests (design decision 11). Verify with a unit test that the fake records the spawn arguments, the writes and the resizes, and delivers pushed bytes and an exit.
- [ ] 2.3 Implement the production spawner with `portable-pty` and a reader thread that sends byte chunks and the exit message (design decision 3). Verify with a test that spawns `/bin/sh -c 'printf ok'` on a real pty, receives `ok` and then the exit message.

## 3. The key and mouse encoders

- [ ] 3.1 Implement the key encoder from crossterm `KeyEvent` to xterm legacy bytes (design decision 7): characters, C0 control characters, Alt as an ESC prefix, Enter, Backspace, Tab, BackTab, Esc, the arrows in normal and application cursor mode, Home, End, PageUp, PageDown, Delete, Insert and F1–F12, and no bytes for a key that is not covered. Verify with a table-driven unit test that lists each lazygit default binding's key.
- [ ] 3.2 Implement the release-chord check: `Char('5')` with `CONTROL` and `Char(']')` with `CONTROL` are the chord, and nothing else is. Verify with unit tests, including Ctrl-5 typed with the kitty protocol off.
- [ ] 3.3 Implement the mouse encoder: translate an event to area-relative cells and encode it in the mouse protocol mode and encoding that the screen reports (SGR and the default encoding; press, release, drag and scroll), and send nothing when the mode is none or the event is outside the area. Verify with unit tests for each mode and encoding and for the edges of the area.

## 4. Drawing the lazygit view

- [ ] 4.1 Record a lazygit frame fixture: run lazygit with the overlay at 40×14 on a scratch repo through `script` or a private tmux server (`tmux -L`), and save the raw output bytes under `fixtures/lazygit/`. Verify that feeding the fixture to a `vt100` parser gives a screen that holds `Files` and the file names.
- [ ] 4.2 Copy the `vt100` screen into the frame buffer (design decision 8): colours, bold, italic, underline, inverse, wide-character continuation cells, the section fill for default-background cells, and the inverse cursor cell while captured with a visible cursor. Verify with inline `insta` snapshots of the fixture frame, a wide-character line and a captured frame with a visible cursor.
- [ ] 4.3 Draw the header row: the PR number at the left as an OSC 8 link through `HyperlinkOverlay`, the total `+n/-n` at the right through `diff_stat_spans`, blank when there is neither, and the accent background while captured. Verify with inline `insta` snapshots of the four cases (PR and changes, neither, captured, released) and an assertion on the pushed hyperlink overlay.
- [ ] 4.4 Make `draw_bottom_content` choose the lazygit view for the Git tab when the view is active, record the lazygit area rect in the layout state, and draw the not-a-repo and exited notices in the tab content. Verify with inline `insta` snapshots for a column, a one-pane strip and pane A of a split strip, and for both notices.

## 5. The host state machine

- [ ] 5.1 Write the overlay file into the runtime directory with mode 0600 (design decision 4), and build the user config list from `LG_CONFIG_FILE` or from `lazygit --print-config-dir` plus `config.yml` when that file exists. Verify with unit tests: `LG_CONFIG_FILE` set, a user file present, no user file, and the overlay content.
- [ ] 5.2 Build the child command and environment (design decision 5): `lazygit -p <top-level> --use-config-file <list>`, the working directory, `TERM=xterm-256color`, `COLORTERM=truecolor` and the area size. Verify with a fake-spawner test of the recorded arguments.
- [ ] 5.3 Implement `tick(now, inputs)` with the Idle, Running and Exited states (design decision 6): start when every input allows it, a 300 ms debounce on a repo change, no restart inside one top-level, no repo gives the not-a-repo state, a 10 s grace before stop when the tab or the window is hidden, and a child exit gives Exited with no automatic restart. Verify with fake-spawner tests driven by fixed `Instant`s for each spec scenario under "the view follows the focused agent pane's repo", "lazygit runs only in a visible window" and "an exited lazygit is restarted by the user".
- [ ] 5.4 Add `#{session_attached}` to the sidebar's per-tick `display-message` format and to `SidebarPaneInfo` (`src/tmux/panes.rs`). Verify with the existing reading-parse tests extended for the new field, including a missing field read as not attached.
- [ ] 5.5 Resolve the focused repo top-level on a focused-pane change (`tmux::get_pane_path`, then `git::repo_root`) and pass it to the host. Verify with a unit test through the injectable path and root functions, as `refresh_git_for_focused_pane_with` does.

## 6. Event loop and input wiring

- [ ] 6.1 In `app::run`, drain the host's output channel beside `git_rx`, feed the parser, set `needs_redraw`, and call `host.tick` on every loop iteration. Resize the pty and the parser when the recorded area rect differs from the pty size. Stop the child when the sidebar exits. Verify with unit tests through the fake spawner: bytes in redraw the view, a changed rect sends one resize, and dropping the host kills the child.
- [ ] 6.2 Add capture and forwarding in `handle_event` before the current dispatch (design decision 7): Enter captures or restarts from Exited when the displayed Git tab has the keyboard focus, a left press in the area captures and is forwarded, the release chord and a press outside the area release (the outside press then acts normally), captured keys and area mouse events go to the pty, and a tab change, the grace expiry and a child exit release. Verify with `handle_key_event` tests and mouse tests through the fake spawner for each scenario under "keys go to lazygit only after capture" and "Ctrl-] or a click outside releases the keys", and confirm that the existing column and strip key tests still pass unchanged.
- [ ] 6.3 Document the capture keys (Enter, a click, Ctrl-]) and the lazygit view on the website's Git tab or feature page and in `docs/state-management.md`. Verify that the documented keys match the code.

## 7. The reduced poll

- [ ] 7.1 Add a header fetch in `src/git.rs` (`rev-parse --abbrev-ref HEAD`, `diff --shortstat`, `remote get-url origin`) and use it with `PrCache` in the poll thread and in the focus-change fetch when the lazygit view is active (design decision 9). Keep the native fetch when it is not active. Verify with unit tests through the injectable fetch functions: lazygit mode calls the header fetch and not `fetch_git_data`, native mode is unchanged.

## 8. Integration checks

- [ ] 8.1 Run `cargo fmt --check`, `cargo clippy` and `cargo test`, then `cargo build --release`. Verify that all four pass.
- [ ] 8.2 Live check with `@sidebar_lazygit on` and two windows: the Git tab shows lazygit without a diff panel, an agent's edit and commit show within 2 s, Enter captures and Ctrl-] releases, a focus move to another worktree restarts lazygit once, the hidden window's lazygit stops after 10 s, `q` gives the exited notice and Enter starts lazygit again. Then set the option `off`, restart the sidebar, and confirm that the native tab is unchanged and no lazygit process runs.
