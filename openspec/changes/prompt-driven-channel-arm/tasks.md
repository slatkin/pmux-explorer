# Tasks

## 1. Server: no probe, nonceless arm

- [ ] 1.1 Delete `src/mcp/probe.rs` and remove its wiring in
  `src/mcp/mod.rs`: no `Probe` construction or unarmed fallback in
  `run_server`, no probe emit in `stdin_loop` — notifications are simply
  ignored. `EndpointState` holds only `armed`, `uid`, `pane`.
- [ ] 1.2 `connection.rs`: `arm` replies `{"ok":true,"armed":true}` and
  arms unconditionally (unknown fields ignored); the unarmed `report`
  branch replies `unarmed` without repeating a probe; the `nonce` reply
  reason is removed.
- [ ] 1.3 `client.rs`: `send_arm(path, timeout)` sends
  `{"v":1,"kind":"arm"}` with no nonce.

## 2. CLI and test support

- [ ] 2.1 `channel-ready` takes no arguments and always sends the arm
  request; `nonce_argument` is removed. Extra arguments are ignored.
- [ ] 2.2 `test_support.rs`: `probe_with_clock` removed;
  `serve_unarmed_endpoint(path, pane, timeout)` drops the nonce
  parameter.
- [ ] 2.3 Update the unit tests (`mcp/mod.rs`, `connection.rs`,
  `client.rs`, `cli/mod.rs`) and the binary-level test in
  `tests/mcp_tests.rs`: no probe after `initialized`, `channel-ready`
  with no args arms, a stale `nonce` field still arms.

## 3. Docs

- [ ] 3.1 `docs/agent-protocol.md`: replace "The arm probe" and "Probe
  repeats" with the prompt-driven arm description.
- [ ] 3.2 `README.md` and `AGENTS.md`: arming paragraph updated.
- [ ] 3.3 pmux side: `skills/pmux/SKILL.md` channel wake-up section and
  the `channel-endpoint.ts` comment; the user's `pi-orchestrator.md`
  prompt gains the `channel-ready` instruction.
