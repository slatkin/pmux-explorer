# Shared binary lookup for hook.sh, sourced by it.
#
# The cascade mirrors the install shapes the binary takes: the plugin
# directory's bin/ and target/release/, then the TPM directory's, then
# `command -v`. On success it sets `BIN` (the binary to exec) and
# `LOOKUP_DIR` (the directory the first four probes searched, for error
# messages); on failure it returns 1. The not-found behavior belongs to
# the caller: hook.sh exits 0 silently so an agent session never sees a
# hook failure.
LOOKUP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# Fallback location used when the calling script is executed from a
# Claude Code plugin install (e.g. `${CLAUDE_PLUGIN_ROOT}/hook.sh`). The
# plugin cache never contains the binary, so hop over to the tmux plugin
# directory where TPM placed it.
TPM_DIR="$HOME/.tmux/plugins/pmux-explorer"
if [ -x "$LOOKUP_DIR/bin/pmux-explorer" ]; then
  BIN="$LOOKUP_DIR/bin/pmux-explorer"
elif [ -x "$LOOKUP_DIR/target/release/pmux-explorer" ]; then
  BIN="$LOOKUP_DIR/target/release/pmux-explorer"
elif [ -x "$TPM_DIR/bin/pmux-explorer" ]; then
  BIN="$TPM_DIR/bin/pmux-explorer"
elif [ -x "$TPM_DIR/target/release/pmux-explorer" ]; then
  BIN="$TPM_DIR/target/release/pmux-explorer"
elif command -v pmux-explorer &>/dev/null; then
  BIN="pmux-explorer"
else
  return 1
fi
