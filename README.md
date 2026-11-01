<h1 align="center">pmux-explorer</h1>

<p align="center">One tmux sidebar that tracks every Claude Code, Codex, and OpenCode pane across every session and window. See status, background shells, prompts, Git state, activity, and worktrees without switching windows.</p>

<p align="center">
  <a href="https://slatkin.github.io/pmux-explorer/">Documentation</a> ·
  <a href="https://slatkin.github.io/pmux-explorer/getting-started/installation/">Getting Started</a> ·
  <a href="https://slatkin.github.io/pmux-explorer/features/agent-pane/">Features</a>
</p>

> **This is a fork.** It started from
> [hiroppy/tmux-agent-sidebar](https://github.com/hiroppy/tmux-agent-sidebar),
> which is MIT licensed and credited in [LICENSE](LICENSE). The fork is
> developed on its own and no longer tracks upstream.

## Features

- **Every pane, one view** 
  — tracks pi, Claude Code, Codex, and OpenCode panes across all tmux sessions and windows
- **Live metadata** 
  — prompts, tool calls, response previews, background shell state, wait reasons, task progress, and subagent trees refresh as the agents work
- **Worktrees, included** 
  — spawn a fresh worktree + agent from the sidebar and tear it down — window, worktree, and branch — in one keystroke
- **Desktop notifications** 
  — native alerts when an agent finishes, needs permission, or errors out

OpenCode uses a small local plugin bridge instead of per-event hook config. The bridge targets the OpenCode 2 plugin API, lives at `plugins/opencode/pmux-explorer.js`, and can be symlinked as a single file into `~/.config/opencode/plugins/` so it coexists with any existing plugins.

pi is supported through the pmux extension: pmux ships a pi extension that loads in every pi session and publishes the session's activity to the sidebar.

## Requirements

- tmux 3.0+ (the floating panel — popping the sidebar out of the window — needs tmux 3.8, because it uses `break-pane -W` and `join-pane`)
- [TPM](https://github.com/tmux-plugins/tpm) (or the manual install in [Installation](https://slatkin.github.io/pmux-explorer/getting-started/installation/))
- [GitHub CLI](https://cli.github.com/) (optional — required only for PR numbers in the Git tab)

## Quick Start

### 1. Install the plugin

Using [TPM](https://github.com/tmux-plugins/tpm):

```tmux
set -g @plugin 'slatkin/pmux-explorer'
```

Reload tmux (`tmux source ~/.tmux.conf`), then press `prefix + I`. The install wizard downloads a pre-built binary or builds from source.

### 2. Wire up the agent hooks

- **Claude Code** — register the plugin inside Claude Code:

  ```sh
  /plugin marketplace add ~/.tmux/plugins/pmux-explorer
  /plugin install pmux-explorer@slatkin
  ```

- **Codex** — open a Codex pane, press `prefix + e`, click the yellow `ⓘ` badge, copy the setup snippet, paste it into the Codex pane.
- **OpenCode** — symlink just the plugin file so your existing `~/.config/opencode/plugins/` contents stay untouched:

  ```sh
  mkdir -p ~/.config/opencode/plugins
  ln -sf ~/.tmux/plugins/pmux-explorer/plugins/opencode/pmux-explorer.js \
    ~/.config/opencode/plugins/pmux-explorer.js
  ```

Full walkthroughs: [Claude Code setup](https://slatkin.github.io/pmux-explorer/getting-started/claude-code/) · [Codex setup](https://slatkin.github.io/pmux-explorer/getting-started/codex/) · [OpenCode setup](https://slatkin.github.io/pmux-explorer/getting-started/opencode/)

### 3. Toggle the sidebar

`prefix + e` toggles the sidebar in the current window, `prefix + E` toggles it everywhere.

## Documentation

The [documentation site](https://slatkin.github.io/pmux-explorer/) covers every feature and option:

- [Agent pane breakdown](https://slatkin.github.io/pmux-explorer/features/agent-pane/)
- [Worktree lifecycle](https://slatkin.github.io/pmux-explorer/features/worktree/)
- [Activity log](https://slatkin.github.io/pmux-explorer/features/activity-log/) · [Git tab](https://slatkin.github.io/pmux-explorer/features/git-status/) · [Notifications](https://slatkin.github.io/pmux-explorer/features/notifications/)
- [Agent support matrix](https://slatkin.github.io/pmux-explorer/agents/)
- [Keybindings](https://slatkin.github.io/pmux-explorer/reference/keybindings/) · [tmux options](https://slatkin.github.io/pmux-explorer/reference/tmux-options/) · [Scripting](https://slatkin.github.io/pmux-explorer/reference/scripting/)

## Development

Symlink the plugin directory to your working copy so builds are picked up without copying:

```sh
rm -rf ~/.tmux/plugins/pmux-explorer
ln -s <path-to-this-repo> ~/.tmux/plugins/pmux-explorer
cargo build --release
```

Toggle the sidebar off → on to pick up the new binary.

### Picking up local builds for the Claude Code plugin

If you also installed this as a Claude Code plugin (`/plugin`), its install path
holds a copy of the released binary that hooks resolve before falling back to
`target/release/`. To make local builds flow through Claude Code hooks too,
replace that copy with a symlink to your working copy:

```sh
# Replace the cached plugin install with a symlink to your repo
PLUGIN_CACHE=~/.claude/plugins/cache/<owner>/pmux-explorer/<version>
rm -rf "$PLUGIN_CACHE"
ln -s <path-to-this-repo> "$PLUGIN_CACHE"
```

Also remove the stale release binary at `bin/pmux-explorer` in your repo
if present — both the tmux launcher and `hook.sh` prefer `bin/` over
`target/release/`, so a leftover binary there will mask `cargo build --release`
output:

```sh
rm -f bin/pmux-explorer
```

Note: Claude Code's plugin updater may overwrite the symlink on a future
update; re-run the symlink step if that happens.

## License

[MIT](./LICENSE)
