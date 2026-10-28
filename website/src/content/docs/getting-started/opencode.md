---
title: OpenCode setup
description: Wire up the OpenCode plugin bridge from the bundled plugin directory.
---

OpenCode uses the bundled plugin bridge, written for the OpenCode 2 plugin
API. Once you make the plugin visible to OpenCode, the sidebar can receive
its events automatically.

## Plugin bridge

### Link the plugin file

Create OpenCode's global plugin directory if it does not already exist, then
symlink the plugin **file** (not the directory) into it. Linking the single
file lets the bridge coexist with any other plugins you have installed:

```sh
mkdir -p ~/.config/opencode/plugins
ln -sf ~/.tmux/plugins/pmux-explorer/plugins/opencode/pmux-explorer.js \
  ~/.config/opencode/plugins/pmux-explorer.js
```

If you keep `pmux-explorer` in a different path, point the symlink at
that copy instead.

Keep the bridge out of any project's `.opencode/plugin|plugins/` directory:
OpenCode loads every source it finds and marks a second copy of the same
plugin id as failed, which raises a `Plugin failed` toast in the TUI. The
global symlink is the single load path.

### Restart OpenCode

OpenCode's plugin supervisor reloads the bridge when the file changes, but a
session that was already running only starts reporting after its next prompt;
a fresh OpenCode session picks the bridge up immediately.
