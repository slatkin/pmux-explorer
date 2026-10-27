# Proposal

## Why

The `--mcp` channel server asks the model to arm the channel itself:
after `notifications/initialized` it emits a probe event telling Claude
to run `channel-ready --nonce <n>`, and repeats the probe on unarmed
reports. That spends a turn in every flagged session whether or not the
user wants channel delivery that day, and it makes the server — not the
user's orchestration prompt — the owner of the arm decision. Arming
belongs to the prompt: when a Claude session should receive reports on
the channel, the orchestration prompt that launches it says so.

## What Changes

- The server emits no channel event of its own: the probe after
  `notifications/initialized` and the report-triggered repeats are
  removed. The server starts unarmed and stays unarmed until an `arm`
  request arrives.
- The `arm` request carries no nonce: `{"v":1,"kind":"arm"}` arms the
  server for the rest of the process. Extra fields — a `nonce` sent by
  an older caller — are ignored. The `nonce` reply reason is gone.
- `pmux-explorer channel-ready` takes no arguments: it sends the `arm`
  request to the endpoint of its own pane. Arguments an older prompt may
  still pass (`--nonce <n>`) are ignored.
- The nonce machinery goes with the probe: no `/dev/urandom` nonce, no
  repeat gate, no `Probe`. The peer-uid gate stays the only access
  control — any same-uid caller could already observe an armed server;
  arming one is no more privileged.

## Impact

- Affected specs: `channel-endpoint` (Arm probe and Probe repeat
  removed; Arm request, Channel-ready command and Status request
  revised).
- Affected code: `src/mcp/` (`probe.rs` deleted; `mod.rs`,
  `connection.rs`, `client.rs` simplified), `src/cli/mod.rs`
  (`channel-ready`), `src/test_support.rs`, `tests/mcp_tests.rs`.
- Affected docs: `docs/agent-protocol.md`, `README.md`, `AGENTS.md`; on
  the pmux side the `pmux` skill's channel wake-up section now tells the
  orchestrator to arm the channel itself, and the user's Claude
  orchestration prompts carry the `channel-ready` instruction.
