# Tasks

## 1. Feed path and reducer

- [x] 1.1 Add the thinking feed path helper (design D5): `<dir>/<session id>.jsonl` with the `$XDG_RUNTIME_DIR/pmux-thinking` / `/tmp/pmux-thinking-<uid>` rule, env values as arguments, and rejection of an empty, `.`, `..` or `/`-holding session id; verify with unit tests for both directories, each rejected id, and the path vector the pmux `thinking-feed` change pins
- [x] 1.2 Add the last-thought reducer (design D3): `start` clears, `delta` appends with the 16000/8000 tail trim while open, `end` with text replaces, `end` without text closes, invalid lines ignored, text made terminal-safe at ingest; verify with unit tests for every scenario in `specs/thought-tab` "the records make the last thought", "the last thought is bounded" and the escape-sequence scenario
- [x] 1.3 Add the incremental reader (design D4): per-feed `(session id, offset, mtime)` cache, backward 64 KiB scan to the newest `start` on a new session or a shorter file, partial trailing line held back; verify with unit tests on temp files for the first read of a large feed (only the newest block parsed), an append between reads, a partial line completed on the next read, and a truncated/replaced file

## 2. Thought tab state and refresh

- [ ] 2.1 Add `BottomTab::Thought` with the label `thought` in `as_str`/`from_label`; verify with unit tests that `thought` parses to Thought and an unknown value still parses to Activity
- [ ] 2.2 Add the shown-tabs list and the displayed-tab method (design D1, D2): Thought is shown only when the focused pane's agent is pi, and a stored Thought displays as Activity otherwise; switch drawing, `scroll_bottom`, the git poller flag and the cycle to the displayed tab; verify with unit tests for the `specs/shared-view-settings` scenarios, including that a focus change performs no `@sidebar_bottom_tab` write
- [ ] 2.3 Make the cycle step through the shown tabs from the displayed tab; verify with the `specs/agent-tasks` cycle scenarios (three tabs for a non-pi pane, four for a pi pane, Tasks after the fallback)
- [ ] 2.4 Replace the fixed column ranges in `handle_bottom_tab_click` with label spans computed from the shown tabs and the drawn label strings; verify with unit tests for every `specs/bottom-panel` click scenario, including a click on the fill where a hidden `Thought` label would be
- [ ] 2.5 Read the feed in the 1 s refresh only while the displayed tab is Thought, through the reader of 1.3 and the focused pane's `session_id`; verify with a unit test that another displayed tab reads no feed and that a new delta shows on the next refresh
- [ ] 2.6 Delete the feed of a dead pi pane in the stale-agent sweep (design D7): `clear_agent_pane_state` resolves the feed path of the pane's `@pane_session_id` through the D5 helper and removes the file before the pane's options are wiped, ignoring a missing file and a rejected id; verify with unit tests that a dead pi pane's feed is removed with its state, that a missing feed file does not stop the clearing, and that an unsafe session id removes nothing

## 3. Rendering

- [ ] 3.1 Draw `Thought` as the fourth rule label when shown, with the displayed tab in the accent color; verify with `insta` inline snapshots of the rule for a pi pane at 40 columns, a non-pi pane at 30 columns, and the 20-column truncation (update the existing `top_line_border` test)
- [ ] 3.2 Add `src/ui/bottom/thought.rs` (design D6): the muted `HH:mm:ss` start line, the wrapped thought, follow-the-end scrolling that a scroll up suspends and a new `start` resets, and the `No thinking yet` notice; verify with `insta` inline snapshots for a short thought, a tall thought showing its last rows, a scrolled-up view, and the empty notice

## 4. Documentation

- [ ] 4.1 Add a "The thinking feed" section to `docs/agent-protocol.md`: the path rule, file and directory modes, the record shape, the last-thought reading, and the cleanup rule (the sidebar removes the file when the pane's agent state is cleared); verify it matches the pmux `thinking-feed` spec word for word on the path and the record fields
- [ ] 4.2 Update `docs/state-management.md` (the `bottom_tab` row, the `BottomTab` enum, the reader state, the feed file's removal in the teardown sweep), `CLAUDE.md` (module list entries for the reader and `ui/bottom/thought.rs`), and the README tab list; verify by reading each changed section against the code

## 5. Integration

- [ ] 5.1 Run `cargo fmt`, `cargo clippy`, `cargo test` and `cargo build --release`; verify all pass, then, with the pmux `thinking-feed` change installed, focus a pi pane in tmux and verify the Thought label shows, the thought appears within a second of the model thinking, focusing a Claude pane hides the label and shows Activity, and exiting the pi process removes its feed file
