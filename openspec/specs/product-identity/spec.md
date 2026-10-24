# product-identity Specification

## Purpose
Define how the product names itself to users and to pmux: its binary, install location, tmux option and storage names, update source, and attribution. This keeps the break from upstream observable and testable.

## Requirements

### Requirement: Product and binary name
The product SHALL be named `pmux-explorer`. Its binary SHALL be `pmux-explorer`, and the CLI help text, usage lines, and notices SHALL use that name. The old name `tmux-agent-sidebar` SHALL NOT appear in user-visible output.

#### Scenario: Version output
- **WHEN** the user runs `pmux-explorer version`
- **THEN** the output is the crate version, in the same format as before the rename

#### Scenario: Usage text
- **WHEN** the user runs `pmux-explorer setup` with two or more arguments
- **THEN** the usage line starts with `pmux-explorer setup`

#### Scenario: Unknown setup target
- **WHEN** the user runs `pmux-explorer setup` with one unknown agent name
- **THEN** the output is the unknown-agent error, and it does not mention the old name

### Requirement: Install layout
The tmux plugin entry script SHALL be `pmux-explorer.tmux` and its conf file SHALL be `pmux-explorer.conf`. The checkout directory SHALL be named `pmux-explorer`. The Claude plugin SHALL register as `pmux-explorer`. The hook wrapper and `setup` output SHALL refer to the binary and `hook.sh` under the plugin directory they run from.

#### Scenario: Setup emits the plugin's hook path
- **WHEN** the user runs `pmux-explorer setup claude` from a checkout named `pmux-explorer`
- **THEN** the emitted hook command points at that checkout's `hook.sh`

#### Scenario: Pi producer contract
- **WHEN** the user runs `pmux-explorer setup pi`
- **THEN** the printed command line reads `pmux-explorer hook pi <event>`

#### Scenario: Release assets
- **WHEN** a release is built
- **THEN** its asset names start with `pmux-explorer-` and the install wizard downloads those names

### Requirement: Tmux option and storage names
The plugin-set tmux option for the binary path SHALL be `@pmux_explorer_bin`. The worktree and spawn options that were `@agent-sidebar-*` SHALL be `@pmux-explorer-*`, with the same suffixes. Per-pane runtime files SHALL be written as `/tmp/pmux-explorer-activity*.log` and `/tmp/pmux-explorer-tasks*.json`. The old option and file names SHALL NOT be read or written.

#### Scenario: Task file path
- **WHEN** `task-set` records a task for pane `%5`
- **THEN** the task file is `/tmp/pmux-explorer-tasks_5.json`

#### Scenario: Worktree spawn markers
- **WHEN** a worktree spawn sets its marker options
- **THEN** the options are `@pmux-explorer-spawned-worktree` and `@pmux-explorer-spawned-branch`

#### Scenario: Old option ignored
- **WHEN** only `@agent-sidebar-default-agent` is set in tmux
- **THEN** the old value has no effect on the default agent

### Requirement: No legacy hook detection
The product SHALL NOT detect, report, or migrate hook entries that carry the old name. A user with old entries removes them by hand.

#### Scenario: Old hook entry present
- **WHEN** `settings.json` contains a hook command that mentions `tmux-agent-sidebar/hook.sh`
- **THEN** the sidebar shows no migration prompt about it

### Requirement: Update source
The update check SHALL query the latest release of `slatkin/pmux-explorer` and no other repository.

#### Scenario: Release lookup
- **WHEN** the update check runs
- **THEN** it requests `repos/slatkin/pmux-explorer/releases/latest`

### Requirement: Attribution and licence
The repository SHALL keep the upstream MIT copyright notice in `LICENSE`. The README SHALL credit `hiroppy/tmux-agent-sidebar` as the origin in one place. No other tracked file outside `openspec/changes/archive/` SHALL contain `hiroppy`, and no tracked file outside the archive SHALL contain an old name form.

#### Scenario: Upstream references
- **WHEN** `git grep hiroppy` runs, excluding `openspec/changes/archive/`
- **THEN** matches appear only in `LICENSE` and the README credit line

#### Scenario: Old name forms
- **WHEN** `git grep` runs for `tmux-agent-sidebar`, `tmux_agent_sidebar`, `TMUX_AGENT_SIDEBAR`, `agent_sidebar`, and `agent-sidebar`, excluding `openspec/changes/archive/` and the README credit line
- **THEN** there are no matches

### Requirement: Unchanged protocol and options
The agent event protocol, its `<event>` names and JSON shape, the `@sidebar_*` options, and the `@pane_*` options SHALL keep their current names and behavior.

#### Scenario: Existing producer payload
- **WHEN** a producer sends a valid payload to `pmux-explorer hook <agent> <event>`
- **THEN** the pane state updates as it did before the rename
