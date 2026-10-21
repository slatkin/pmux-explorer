# Build and install the tmux-agent-sidebar release binary.
#
#   make build      compile the release binary
#   make install    build, then install it where the sidebar and the hooks find it,
#                   and register the Claude Code plugin that carries the hooks
#   make install-claude-hooks   only the Claude Code plugin step
#   make uninstall  remove the installed binaries and the Claude Code plugin
#   make restart    stop the running sidebars so the next toggle uses the new binary
#   make check      fmt + clippy + test
#
# The installed copy is replaced with a rename, never written in place: a
# running binary cannot be overwritten ("Text file busy").

CARGO ?= cargo
PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
PLUGIN_DIR ?= $(HOME)/.tmux/plugins/tmux-agent-sidebar
CLAUDE ?= claude
CLAUDE_MARKETPLACE ?= slatkin
CLAUDE_PLUGIN ?= tmux-agent-sidebar@$(CLAUDE_MARKETPLACE)
BINARY_NAME := tmux-agent-sidebar
RELEASE_BIN := target/release/$(BINARY_NAME)

.PHONY: all build release install install-claude-hooks uninstall uninstall-claude-hooks restart test fmt fmt-fix clippy check clean

all: build

## Compile the release binary.
build release:
	$(CARGO) build --release

## Install the release binary.
#
# Two destinations, because two different things resolve the binary:
#   * $(BINDIR) is on PATH. The pmux pi extension calls `tmux-agent-sidebar
#     hook pi <event>` for every pi event and finds it there.
#   * $(PLUGIN_DIR)/target/release is what the tmux launcher runs, when the
#     plugin directory is a real directory instead of a symlink to this
#     checkout.
install: build
	@mkdir -p "$(BINDIR)"
	@install -m 0755 "$(RELEASE_BIN)" "$(BINDIR)/.$(BINARY_NAME).new"
	@mv -f "$(BINDIR)/.$(BINARY_NAME).new" "$(BINDIR)/$(BINARY_NAME)"
	@echo "installed $(BINDIR)/$(BINARY_NAME)"
	@set -e; \
	if [ -e "$(PLUGIN_DIR)" ]; then \
		if [ "$$(cd "$(PLUGIN_DIR)" && pwd -P)" = "$$(pwd -P)" ]; then \
			echo "plugin dir is this checkout: target/release is already the binary the launcher uses"; \
		else \
			mkdir -p "$(PLUGIN_DIR)/target/release"; \
			install -m 0755 "$(RELEASE_BIN)" "$(PLUGIN_DIR)/target/release/.$(BINARY_NAME).new"; \
			mv -f "$(PLUGIN_DIR)/target/release/.$(BINARY_NAME).new" "$(PLUGIN_DIR)/target/release/$(BINARY_NAME)"; \
			echo "installed $(PLUGIN_DIR)/target/release/$(BINARY_NAME)"; \
		fi; \
	else \
		echo "no plugin dir at $(PLUGIN_DIR): skipped (the PATH copy is enough for the hooks)"; \
	fi
	@$(MAKE) --no-print-directory install-claude-hooks

## Register this checkout as a Claude Code plugin marketplace and install the
## plugin. The plugin's hooks/hooks.json then wires every Claude hook to
## hook.sh, which finds the binary in $(BINDIR). Safe to run again: an
## existing marketplace is refreshed and an existing plugin is updated.
install-claude-hooks:
	@set -e; \
	if ! command -v "$(CLAUDE)" >/dev/null 2>&1; then \
		echo "no $(CLAUDE) on PATH: skipped the Claude Code hooks"; \
		exit 0; \
	fi; \
	if "$(CLAUDE)" plugin marketplace list 2>/dev/null | grep -q "❯ $(CLAUDE_MARKETPLACE)$$"; then \
		"$(CLAUDE)" plugin marketplace update "$(CLAUDE_MARKETPLACE)"; \
	else \
		"$(CLAUDE)" plugin marketplace add "$(CURDIR)"; \
	fi; \
	if "$(CLAUDE)" plugin list 2>/dev/null | grep -q "❯ $(CLAUDE_PLUGIN)$$"; then \
		"$(CLAUDE)" plugin update "$(CLAUDE_PLUGIN)"; \
	else \
		"$(CLAUDE)" plugin install "$(CLAUDE_PLUGIN)"; \
	fi; \
	echo "Claude Code hooks installed: restart Claude Code to load them"

## Remove the installed copies. The plugin copy is left alone when it is this
## checkout's own target/release.
uninstall:
	@rm -f "$(BINDIR)/$(BINARY_NAME)"
	@echo "removed $(BINDIR)/$(BINARY_NAME)"
	@set -e; \
	if [ -e "$(PLUGIN_DIR)" ] && [ "$$(cd "$(PLUGIN_DIR)" && pwd -P)" != "$$(pwd -P)" ]; then \
		rm -f "$(PLUGIN_DIR)/target/release/$(BINARY_NAME)"; \
		echo "removed $(PLUGIN_DIR)/target/release/$(BINARY_NAME)"; \
	fi
	@$(MAKE) --no-print-directory uninstall-claude-hooks

## Remove the Claude Code plugin and the marketplace that install-claude-hooks
## added. Each is removed only when present, so this is safe to run again.
uninstall-claude-hooks:
	@set -e; \
	if ! command -v "$(CLAUDE)" >/dev/null 2>&1; then \
		echo "no $(CLAUDE) on PATH: skipped the Claude Code hooks"; \
		exit 0; \
	fi; \
	if "$(CLAUDE)" plugin list 2>/dev/null | grep -q "❯ $(CLAUDE_PLUGIN)$$"; then \
		"$(CLAUDE)" plugin uninstall "$(CLAUDE_PLUGIN)"; \
	fi; \
	if "$(CLAUDE)" plugin marketplace list 2>/dev/null | grep -q "❯ $(CLAUDE_MARKETPLACE)$$"; then \
		"$(CLAUDE)" plugin marketplace remove "$(CLAUDE_MARKETPLACE)"; \
	fi; \
	echo "Claude Code hooks removed: restart Claude Code to unload them"

## Stop the running sidebars. Their panes close with the process, so toggle
## the sidebar off and on to bring it back with the new binary.
restart:
	@pkill -f "$(RELEASE_BIN)" 2>/dev/null || true
	@pkill -f "$(BINDIR)/$(BINARY_NAME)" 2>/dev/null || true
	@echo "stopped the running sidebars: toggle the sidebar off and on to start it again"

## Run the test suite.
test:
	$(CARGO) test

## Check the formatting.
fmt:
	$(CARGO) fmt --all -- --check

## Fix the formatting.
fmt-fix:
	$(CARGO) fmt --all

## Lint.
clippy:
	$(CARGO) clippy --workspace --all-targets

## Everything CI runs.
check: fmt clippy test

## Drop the build artifacts.
clean:
	$(CARGO) clean
