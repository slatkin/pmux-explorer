# Spec Delta

## Purpose

Lets the user opt in to a live lazygit in the Git tab, showing only its list
panels for the focused agent pane's repo, so the user can stage, commit and
manage branches and worktrees without leaving the sidebar.

## ADDED Requirements

### Requirement: the lazygit view is an option, off by default

The global tmux option `@sidebar_lazygit` SHALL select the Git tab's view. The value `on` SHALL select the lazygit view. Any other value, an empty value or no value SHALL select the native view, with no change to its behavior. The conf SHALL seed the option to `off`. The sidebar SHALL read the option once when it starts, so a change takes effect when the sidebar is started again.

#### Scenario: the default is the native view
- **WHEN** the user has not set `@sidebar_lazygit` and opens the Git tab
- **THEN** the tab shows the native view
- **AND** no lazygit process starts

#### Scenario: the option selects lazygit
- **WHEN** `@sidebar_lazygit` is `on` when the sidebar starts and the user opens the Git tab
- **THEN** the tab shows the lazygit view

#### Scenario: a change waits for a restart
- **WHEN** the sidebar started with `@sidebar_lazygit` `off` and the user sets it to `on`
- **THEN** the Git tab keeps the native view until the sidebar is toggled off and on

#### Scenario: an unknown value is off
- **WHEN** `@sidebar_lazygit` is `yes` when the sidebar starts
- **THEN** the Git tab shows the native view

### Requirement: a missing lazygit falls back to the native view

When `@sidebar_lazygit` is `on` and no `lazygit` executable is on `PATH` at startup, the Git tab SHALL show the native view, and the sidebar SHALL show one setup notice that names the missing executable. The tab SHALL NOT be blank.

#### Scenario: lazygit is not installed
- **WHEN** `@sidebar_lazygit` is `on` and `lazygit` is not on `PATH` when the sidebar starts
- **THEN** the Git tab shows the native view
- **AND** the notices popup holds one notice that `lazygit` was not found

### Requirement: lazygit shows only its list panels

The lazygit view SHALL show lazygit's side panels (status, files, branches, commits, stash) across the full width of the area, and SHALL NOT show lazygit's diff panel while a side panel has lazygit's focus, unless a repo-level config file changes the layout.

#### Scenario: the files panel without a diff
- **WHEN** the lazygit view is 40 columns wide and 15 rows high and the repo has changed files
- **THEN** the files panel lists the files across the full width
- **AND** no diff of the selected file is drawn

#### Scenario: a file with staged and unstaged changes
- **WHEN** the selected file has staged and unstaged changes
- **THEN** no diff panel is drawn

### Requirement: a repo-level config wins over the overlay

The view SHALL keep the user's own lazygit config, and the sidebar's settings SHALL override the user-level config file only for the layout (including the panel jump labels), auto-fetch, suspend, the bottom line, the tips, the command log, the startup popups, the update checks and the refresh behaviour. lazygit merges repo-level config files (`.git/lazygit.yml` in the repo and `.lazygit.yml` in any parent directory) after the overlay, so a repo-level file wins for the keys it sets.

#### Scenario: the user's keys stay
- **WHEN** the user's lazygit config binds a custom key
- **THEN** that key does the same thing in the lazygit view

#### Scenario: a repo-level config overrides the overlay
- **WHEN** the repo holds a `.git/lazygit.yml` or a `.lazygit.yml` in a parent directory that sets a key the overlay also sets
- **THEN** the repo-level value wins
- **AND** the rest of the user's config stays

### Requirement: lazygit does not fetch or suspend

The lazygit view SHALL NOT start a background `git fetch`, and lazygit's suspend key SHALL NOT stop the lazygit process, unless a repo-level config file re-enables them.

#### Scenario: no auto-fetch
- **WHEN** the lazygit view runs for two minutes
- **THEN** lazygit runs no `git fetch` that the user did not ask for

#### Scenario: the suspend key does nothing
- **WHEN** the keys are captured and the user presses Ctrl-Z
- **THEN** lazygit keeps drawing and accepts keys

### Requirement: the view refreshes on a poll

The lazygit view SHALL show changes to the working tree within 2 seconds, and changes to refs (commits, branches, checkouts) within 2 seconds, without user input.

#### Scenario: an agent edits a file
- **WHEN** an agent changes a file in the repo the view shows
- **THEN** the files panel shows the change within 2 seconds

#### Scenario: an agent commits
- **WHEN** an agent makes a commit in the repo the view shows
- **THEN** the commits panel shows the new commit within 2 seconds

### Requirement: the header row shows the PR link and the diff stat

The lazygit view SHALL draw one native header row above lazygit. The row SHALL show the current branch's PR number at the left as a link to the PR, when the branch has a PR, and the total `+n/-n` diff stat at the right, when the working tree has changes. lazygit SHALL get the rows under the header row.

#### Scenario: a branch with a PR and changes
- **WHEN** the branch has PR 123 and the working tree has 48 added and 12 deleted lines
- **THEN** the header row shows `#123` at the left as a link to the PR
- **AND** shows `+48/-12` at the right

#### Scenario: no PR and a clean tree
- **WHEN** the branch has no PR and the working tree is clean
- **THEN** the header row is blank
- **AND** lazygit starts on the second row of the tab content

### Requirement: the view follows the focused agent pane's repo

The lazygit view SHALL show the repo of the focused agent pane, the same pane the native Git tab follows. When the focused repo's top-level directory changes and stays changed for 300 ms, the sidebar SHALL stop lazygit and start it again for the new repo. A focus change inside the same top-level directory SHALL NOT restart lazygit.

#### Scenario: focus moves to another worktree
- **WHEN** focus moves from an agent in worktree A to an agent in worktree B and stays for 300 ms
- **THEN** the view shows worktree B

#### Scenario: focus passes through quickly
- **WHEN** focus moves from A to B and back to A within 300 ms
- **THEN** lazygit is not restarted

#### Scenario: focus moves inside one repo
- **WHEN** focus moves between two agents whose working directories are in the same repo top-level
- **THEN** lazygit is not restarted

### Requirement: a path outside a repo starts no lazygit

When the focused agent pane's path is not inside a git repo, the lazygit view SHALL show a notice in the tab content that the path is not a git repo, and SHALL NOT start lazygit.

#### Scenario: an agent outside a repo
- **WHEN** the focused agent pane's working directory is not inside a git repo
- **THEN** the tab shows a not-a-repo notice
- **AND** no lazygit process runs for that path

### Requirement: keys go to lazygit only after capture

While the keys are not captured, the sidebar SHALL handle every key and mouse event as it does with the native view, and lazygit SHALL keep drawing. Enter while the Git tab has the sidebar's keyboard focus, or a left click in the lazygit area, SHALL capture the keys. While captured, every key, and every mouse press, release, drag and scroll in the lazygit area, SHALL go to lazygit, except the release chord. Pasted text arrives as keys and SHALL go to lazygit the same way.

#### Scenario: arrows before capture
- **WHEN** the Git tab has the keyboard focus, the keys are not captured, and the user presses Down
- **THEN** the sidebar handles Down as it does with the native view
- **AND** lazygit gets no key

#### Scenario: Enter captures
- **WHEN** the Git tab has the keyboard focus and the user presses Enter
- **THEN** the keys are captured
- **AND** lazygit does not get that Enter

#### Scenario: a click captures
- **WHEN** the user clicks a file row in the lazygit area while the keys are not captured
- **THEN** the keys are captured
- **AND** lazygit gets the click at the same cell

#### Scenario: captured keys reach lazygit
- **WHEN** the keys are captured and the user presses Shift-Tab, Esc or `q`
- **THEN** lazygit gets the key
- **AND** the sidebar's tab does not change

#### Scenario: mouse events disabled
- **WHEN** the user's config disables lazygit's mouse events and the user clicks in the lazygit area
- **THEN** the keys are captured
- **AND** lazygit gets no mouse event

### Requirement: Ctrl-] or a click outside releases the keys

While the keys are captured, Ctrl-] SHALL release them and SHALL NOT go to lazygit. A left click outside the lazygit area SHALL release them and SHALL then act as a normal sidebar click. A change of the displayed tab or a hidden window SHALL also release them.

#### Scenario: the release chord
- **WHEN** the keys are captured and the user presses Ctrl-]
- **THEN** the keys are released
- **AND** lazygit gets no key

#### Scenario: a click on the agent list
- **WHEN** the keys are captured and the user clicks an agent row
- **THEN** the keys are released
- **AND** the click selects the agent row

#### Scenario: the window hides
- **WHEN** the keys are captured and the user switches to another window
- **THEN** the keys are released

### Requirement: the header row shows the capture state

While the keys are captured, the header row SHALL be drawn in the accent colour as its background, so the capture state shows at any width. While the keys are not captured, the header row SHALL use the tab content's normal background.

#### Scenario: captured
- **WHEN** the keys are captured
- **THEN** every cell of the header row has the accent colour as its background

#### Scenario: released
- **WHEN** the keys are released
- **THEN** the header row has the tab content's normal background

### Requirement: lazygit runs only in a visible window

The sidebar SHALL run lazygit only while the lazygit view is displayed and the sidebar's window is the active window of an attached session. When one of these stops being true, lazygit SHALL keep running for 10 seconds and then stop. When they are true again, lazygit SHALL start within one refresh tick.

#### Scenario: a hidden window
- **WHEN** the user switches to another window and stays there for 10 seconds
- **THEN** the hidden window's sidebar stops its lazygit

#### Scenario: a quick switch back
- **WHEN** the user switches away and back within 10 seconds
- **THEN** the same lazygit keeps running

#### Scenario: the tab shared across windows
- **WHEN** the user selects the Git tab, which every sidebar then displays, and three windows exist in one attached session
- **THEN** only the active window's sidebar runs lazygit

#### Scenario: another tab
- **WHEN** the user selects the Tasks tab for 10 seconds
- **THEN** lazygit stops

### Requirement: an exited lazygit is restarted by the user

When lazygit exits while the sidebar wants it to run, the sidebar SHALL release the keys and show a notice in the tab content that lazygit exited and Enter starts it again. The sidebar SHALL NOT restart it without user input. Enter while the Git tab has the keyboard focus SHALL start it again.

#### Scenario: the user quits lazygit
- **WHEN** the keys are captured and the user presses `q` in lazygit
- **THEN** the keys are released
- **AND** the tab shows the exited notice

#### Scenario: Enter starts it again
- **WHEN** the tab shows the exited notice and the user presses Enter
- **THEN** lazygit starts for the focused repo

### Requirement: the screen follows the area size

The lazygit view SHALL resize lazygit's terminal to the area under the header row whenever that area changes size, including during a split drag and a window resize.

#### Scenario: the user drags the rule
- **WHEN** the user drags the column's rule to make the bottom panel 5 rows taller
- **THEN** lazygit's terminal is 5 rows taller after the frame that draws the new size
