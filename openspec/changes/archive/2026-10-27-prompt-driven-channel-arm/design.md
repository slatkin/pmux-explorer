# Design

## Decisions

### D1: The server never prompts the model

The channel server exists to carry report pointers into the session.
Whether the channel is wanted is a per-session user decision, so the
instruction to arm it lives in the user's orchestration prompt (for
example `pi-orchestrator.md`), not in a server-emitted turn. The server
therefore emits `notifications/claude/channel` only for reports — a
`probe` event kind no longer exists.

### D2: The nonce goes with the probe

The nonce existed to prove the arming command came from a model that had
seen the probe. With no probe there is nothing to prove against: the
prompt instructs the model directly, and the peer-uid gate (the socket
lives in a 0700 directory and only same-uid connections get a reply) is
the whole access control. `{"v":1,"kind":"arm"}` therefore arms
unconditionally. Removing the nonce also removes the `/dev/urandom`
read, the repeat gate and its injected clock, and the `Probe` type.

### D3: Wire compatibility is kept loose

An `arm` request that still carries a `nonce` field — from an older
`channel-ready`, or an older prompt's command line — is accepted, since
the handler reads `v` and `kind` only. `channel-ready` ignores its
arguments for the same reason. The `report`, `status`, request/reply and
endpoint-path shapes do not change, so pmux `report-channel` needs no
coordinated release: an unarmed reply still falls back to `send-keys`,
it just arrives without a probe side-effect.
