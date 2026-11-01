# Design

## Context

See `proposal.md` for the motivation. State today:

- About 58 `set_pane_option` / `unset_pane_option` call sites sit in
  `cli/hook/**` and `cli/mod.rs`, plus four in the sidebar's own code
  (`state/refresh.rs` bg-shell sweep and dead-pane wipe, `tmux/query.rs`
  shell-fallback wipe). Each call is one `tmux` process.
- Handlers also read mid-sequence: `@pane_bg_cmd` (Stop), `@pane_subagents`
  (the guard), `@pane_status` and `@pane_started_at` (activity-log).
- `tmux::test_mock` is a `cfg(test)` thread-local store that
  `set_pane_option` and friends intercept. It is the only second adapter that
  exists.
- Only pi has a per-event field list (`PI_EVENTS`). Claude and Codex have
  `HOOK_REGISTRATIONS`, which pairs a trigger with an event kind and lists
  no fields.
- `sanitize_tmux_value` (replaces `\n` and `|`) is applied by some handlers
  and not by others. `@pane_wait_reason` is written raw on several paths.

## Goals / Non-Goals

**Goals:**
- One owner for the rules in `specs/pane-state-writes/spec.md`.
- One tmux process per event for the transition's writes, and one for the
  read.
- A second adapter that is a real implementation of the seam.

**Non-Goals:**
- Writes to `@sidebar_*` options and to the sidebar's own pane options.
- The teardown rituals' shared order and skip lists (issue 15, finding 6).
  Their option wipes move onto the store, and each keeps its own skip list.
- The read side (`list-panes` format and decoder).
- The read-modify-write race on `@pane_subagents` and on the task store.
  Batching does not fix it: two hook processes can still read the same value.
- Non-option effects: the activity log file, the desktop notifications
  themselves, the `println!` of a hook response, the task store. They stay in
  the handlers. (The notification stamps and run id are option writes and ride
  the store; what stays in the handlers is their call site and the send
  decision, not a second write path.)

## Decisions

### D1. A pure `transition`, keyed by plain option names

`transition(&PaneSnapshot, &AgentEvent, &Reported) -> PaneWrites`. `PaneWrites`
is an ordered list of `Set(key, value)` and `Unset(key)`. The keys stay the
existing `&'static str` constants. A typed field enum waits for the schema
table (finding 2), which would generate it; making one here would be thrown
away.

*Alternative:* have each handler call `store.set()` directly. Rejected: the
rules would still live in the callers, which is the problem.

### D2. The snapshot is one read of only the keys the rules need

`PaneSnapshot` holds `status`, `bg_cmd`, `subagents` and `started_at`, read
with one `display-message -p -t <pane>` whose format names those four options.
It uses the same escaping as the `list-panes` read. Handlers that need other
values today (notification stamps, the run id, pending markers, pane-label
lookups) keep reading them directly — through `store.get`, not the snapshot,
because those reads do not feed the transition rules.

*Alternative:* `show-options -p`, which lists every pane option. Rejected: its
output quotes and escapes values, so it needs its own parser.

### D3. One event, one batched `tmux` call

The tmux adapter turns a `PaneWrites` into one `tmux` invocation:
`set -t P -p K V ; set -t P -p -u K2 ; …`.

This was checked on a private tmux server. A reader polled `@a` and `@b` in
one `display-message` while a writer changed both. With two separate `set`
calls the reader saw `@a != @b` in 1483 of 3000 reads. With one `;`-joined
call it saw 0 in 3000, across 3025 batched writes. A command list does not
yield to another client, because `set-option` never waits.

The one read plus one write replace about 20 calls for a `Stop`. A race
between two hook processes remains (both read the same snapshot); it is no
wider than today.

### D4. `PaneStore` has `snapshot`, `get` and `apply`; every option access takes it as an argument

```text
 hook dispatch --builds--> &dyn PaneStore
                               |
   handler: snap = store.snapshot(pane)
            writes = transition(&snap, &event, &reported)
            store.apply(pane, &writes)      // one batch
            ...non-option effects...
```

The store is created once at the `hook` entry and passed down, as `FeedEnv` is.
There is no global. `apply` returns a `Result` that hook callers ignore, as
they ignore tmux errors today, and `apply` on an empty list is a no-op. `get`
reads one option (`show -pv`), for the reads the transition does not own.

Non-transition option access goes through the same store. The dispatch arms
for `cwd-changed` and `task-completed` run the same
`snapshot` → `transition` → `apply` path. `cmd_set_status` applies
`status_writes(status)` — the helper `transition` uses for the
status↔attention coupling — so that rule keeps its one owner. The notification
stamps and run id stay call-site writes (the stamp depends on whether the
desktop notification actually sent), but they go through `store.get` /
`store.apply` like everything else; the pending markers and the pane-label
reads in `context/labels.rs` do the same. With no caller left on the raw
`set/get/unset_pane_option` helpers, `tmux::test_mock` is removed, not merely
shadowed.

Handlers run their non-option effects after `apply`, and an effect that
consults `@pane_subagents` — `mark_task_reset`, the teardown drains, the
`SessionEnd` bail — must see the post-batch state: the Stop batch clears the
stale list first, so the reset marker still lands the way it does today.

The sidebar's sweeps take the store as an argument, constructed at the sweep's
entry (`apply_session_snapshot`, `query_sessions_with_process_snapshot`) the
way `FeedEnv` is threaded today. The shell-fallback wipe in `tmux/query.rs`
stays inside the parser until finding 6 makes the parser pure.

*Alternative:* a trait object kept in a `thread_local`. Rejected: it hides the
dependency, which is how `test_mock` works today.

### D5. The subagent guard is stated once, with the spec's set

Today the guard skips: permission mode, session id, working directory and
worktree (`sync_pane_location` via `pane_writes_allowed`), the task reset, the
task-store clear on `SessionStart`, the worktree-remove teardown, and
`SessionEnd`. The agent label is written unconditionally today; the spec adds
it to the guarded set. The guard does not skip status, attention, prompt or
the response preview.

`transition` encodes that set as one `identity_guarded` check and applies it
to the listed writes. The overstated comment on `sync_pane_location` in
`context/location.rs` — it claims every pane-scoped write is skipped while
subagents are active — is corrected.

*Alternative:* guard every write. Rejected: it changes behaviour that nobody
asked to change, and I cannot tell from the code whether a child's status
update is wanted.

### D6. The activity-log rule is a transition rule

`activity-log` moves `waiting` or `error` to `running`, as today. It leaves
`idle`, `background` and an absent status alone (new). The `started_at`
default-on-first-run stays tied to the move to `running`. The status priority
helpers in `status_priority.rs` move next to `transition`, so Stop and
Notification statuses come from the same module.

### D7. "Reported" is a property of the event, not of the payload key

The adapter hands `transition` a `Reported` set per event.

- pi: the field list of the matching `PI_EVENTS` entry. `stop` does not list
  `worktree`, so a pi `stop` keeps it. `PI_EVENTS` is unchanged, so `setup pi`
  prints the same contract.
- Claude, Codex, OpenCode: `session_id`, `cwd` and `worktree` for every event
  that carries session context. This keeps today's behaviour for those agents
  with one deliberate divergence, noted below (`cwd`).

A reported field that is absent counts as empty and clears its option. This is
needed because Claude omits `worktree` outside a worktree, and that omission
has to clear stale metadata. Key presence alone cannot tell the two cases
apart. Note `cwd` diverges from today here: an absent or empty `cwd` is
currently ignored, so the stored value survives; under the rule it clears.

An invalid payload becomes `Null` and reports nothing, so it clears nothing.

`docs/agent-protocol.md` is reworded: it no longer says that an omitted field
and an empty field are the same "not reported". It says that the fields listed
for an event are reported (empty means clear) and every other field is left
alone.

*Alternative:* treat key presence as "reported". Rejected: it breaks Claude's
worktree clearing.

*Alternative:* add an explicit `null` for "clear". Rejected: it changes the
protocol for every producer.

### D8. One encoder, applied in `apply`

`apply` encodes every value with one function: `sanitize_tmux_value`, extended
to `\r`. Handlers drop their own calls; the double encoding is harmless
because the function is idempotent. The bg-shell sweep already compares
sanitized process lines with the stored command, so it keeps using the same
function and still matches.

One value cannot survive the encoder as written: the notification stamp is
`ts|fingerprint`, and `|` is exactly what gets replaced. `encode_stamp` /
`parse_stamp` move to `:` as the separator (the timestamp is digits-only, so
splitting on the first `:` is unambiguous even though fingerprints contain
`:`). A `|`-stamp left by an old binary fails to parse, which costs at most
one duplicate desktop notification after upgrade.

`sanitize_log_label` is for the activity log file and is unchanged.

### D9. Sweep and teardown writes move onto the store as plain lists

The bg-shell sweep builds `[Unset(bg_cmd)]`, plus `[Set(status, idle)]` only
when the status is `background`. The three wipe paths each build an
`Unset` list from `AGENT_SESSION_PANE_OPTIONS` with their own skip list and
apply it as one batch. The thinking-feed and activity-log file removals stay
where they are.

## Risks / Trade-offs

- [`activity-log` no longer revives `idle`] → A producer that relied on a tool
  event to start a run without `user-prompt-submit` shows `idle` until its next
  prompt. Claude, Codex and pi all send a prompt event first. The spec states
  the rule, so a producer can see it.
- [A pi pane keeps a worktree label it should have lost] → A pi producer that
  leaves a worktree must say so on an event that reports `worktree`
  (`session-start` or `user-prompt-submit`). The protocol document says this.
- [A hand-written transition table drifts from handler side effects] → Each
  handler keeps its non-option effects; only option writes move. Tests pin the
  option writes per event against the spec scenarios.
- [The snapshot read adds one tmux call to events that wrote almost nothing,
  such as `activity-log`] → The call replaces the one to three reads those
  handlers make today.
- [A large diff in `cli/hook/**`] → Handlers move one at a time. Each lands
  with its option writes expressed through the store.

## Migration Plan

No data migration. Pane options are runtime state in tmux. Deploy is
`cargo build --release`; restart the sidebar. A hook that runs during the
restart uses the new binary on the next event. Rollback is the previous binary.

## Open Questions

- Does the seam land before or after the schema table (finding 2)? The plan
  here works in either order, because keys are constants. Landing the table
  first would let `PaneSnapshot`'s key list be generated.
