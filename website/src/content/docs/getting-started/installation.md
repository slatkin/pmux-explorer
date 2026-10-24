---
title: Installation
description: Install pmux-explorer via TPM or manually.
---

## Requirements

- tmux 3.0+
- [TPM](https://github.com/tmux-plugins/tpm) (for plugin installation)
- [GitHub CLI](https://cli.github.com/) (optional, for displaying PR numbers in the Git tab)
- [Rust](https://rustup.rs/) (only if building from source)

## Option A — TPM (recommended)

Add the plugin to your `tmux.conf`:

```bash
set -g @plugin 'slatkin/pmux-explorer'
```

Reload `tmux.conf`, then press `prefix + I` to install:

```sh
tmux source ~/.tmux.conf
```

On the first run, an install wizard prompts you to download a pre-built binary or build from source.

To update later, press `prefix + U` in TPM's plugin list and select `pmux-explorer`. The install wizard runs again if the bundled binary has changed.

## Option B — Manual

1. Clone the repository:

   ```sh
   git clone https://github.com/slatkin/pmux-explorer.git \
     ~/.tmux/plugins/pmux-explorer
   ```

2. Add the plugin to your `tmux.conf`:

   ```bash
   run-shell ~/.tmux/plugins/pmux-explorer/pmux-explorer.tmux
   ```

3. Install the binary — download a pre-built release, or build from source:

   ```sh
   # macOS (Apple Silicon)
   curl -fSL https://github.com/slatkin/pmux-explorer/releases/latest/download/pmux-explorer-darwin-aarch64 \
     -o ~/.tmux/plugins/pmux-explorer/bin/pmux-explorer
   chmod +x ~/.tmux/plugins/pmux-explorer/bin/pmux-explorer
   ```

   Or build from source:

   ```sh
   cd ~/.tmux/plugins/pmux-explorer
   cargo build --release
   ```

## Reload tmux config

After editing `tmux.conf`, press `prefix + r` (or run `tmux source ~/.tmux.conf`) to reload.

## Next steps

The sidebar receives status updates through agent hooks — continue with the agent you use:

- [Claude Code setup](/pmux-explorer/getting-started/claude-code/)
- [Codex setup](/pmux-explorer/getting-started/codex/)
- [OpenCode setup](/pmux-explorer/getting-started/opencode/)
