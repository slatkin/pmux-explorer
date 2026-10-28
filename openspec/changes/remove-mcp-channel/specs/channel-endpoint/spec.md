# Spec Delta

## REMOVED Requirements

### Requirement: MCP channel server mode
**Reason**: The channel is superseded by the pmux report inbox and its `inbox --follow` monitor (pmux change `inbox-monitor-wake`, rehearsal passed 2026-10-28). No pmux parent needs the server.
**Migration**: Run `pmux inbox --follow` under Claude's `Monitor` tool. Drop `--dangerously-load-development-channels` from the launch command.

### Requirement: Channel capability and revision
**Reason**: The server is removed with its mode; no `initialize` reply remains to declare a capability.
**Migration**: None.

### Requirement: Channel arming
**Reason**: The server is removed, so there is nothing to arm.
**Migration**: Remove the `channel-ready` instruction from any orchestration prompt.

### Requirement: The channel-ready command
**Reason**: The command's only job was the arm request. Without a server it has no endpoint to reach.
**Migration**: Remove the command from orchestration prompts (design decision D3: it is not kept as a tombstone, and an unknown subcommand starts the TUI).

### Requirement: Endpoint path
**Reason**: No socket is created. The path rule stays in pmux only as far as pmux's own inbox path reuses its hash function.
**Migration**: None in this repository.

### Requirement: Endpoint ownership
**Reason**: No socket, no directory and no ownership rule remain.
**Migration**: A leftover socket file in the runtime directory is harmless and is cleared with the runtime directory.

### Requirement: Endpoint connection
**Reason**: No connection is accepted.
**Migration**: None.

### Requirement: Report request
**Reason**: Reports reach a Claude parent through the pmux inbox instead of a channel event.
**Migration**: pmux falls back from a missing endpoint (`no-endpoint`) to the typed pointer until its own follow-up removes the channel attempt.

### Requirement: Opaque report fields
**Reason**: The `report` request is removed.
**Migration**: None.

### Requirement: Endpoint status
**Reason**: The `status` request is removed with the endpoint.
**Migration**: None.

### Requirement: Claude plugin registration
**Reason**: The plugin declares no server, so `mcpServers.pmux` and `mcp.sh` are removed.
**Migration**: None. The hook registration of the plugin is unchanged.
