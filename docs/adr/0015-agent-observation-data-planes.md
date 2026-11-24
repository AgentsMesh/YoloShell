# ADR 0015: Separate agent observation from the terminal data plane

## Status

Proposed on 2026-10-30. Complements ADR 0014 (control effect contract).

## Context

Control latency grew with the number of live sessions. Measured on canonical
with two live agents and several archived ones: `agent get` took 2.5–4 s,
`agent list` 2.7 s, a Qwen `send` 6.6 s, and a Codex `send` 9.4 s. Every
`ListAgents`, `GetAgent` and `AgentControl` calls
`LocalBackend::observe_agents_locked`, which:

1. fetches the complete Host catalogue (`list_sessions_complete(true)`),
   including archived sessions, with a filesystem read per session;
2. for every view calls `classify_native_observation`, i.e. one worker RPC
   per session that reads PTY-derived control state (foreground process
   group, process scope, brackets), and re-projects the view;
3. rewrites the projection record.

The CLI's `send` performs `List` + `ListAgents` + `GetAgent` + lease +
`AgentControl`; several of these repeat the full sweep. Cost is therefore
O(number of sessions) per command, multiplied by the number of calls per
command, and every sweep contacts workers that have nothing to do with the
requested agent.

Terminal output is already a separate plane: `TerminalView` carries no PTY
bytes and output streams only to attached viewers. The coupling is in
observation, not in bytes: the per-session worker observation reads
PTY-derived state for unviewed agents, and it runs even when the provider
binding has not changed. The Codex worker's control path additionally spends
seconds in its authority/evidence verification, and a lost or expired worker
reply surfaced as an unverified public outcome until the Hub learned to
resolve it from the durable control journal (ADR 0014).

## Decision

Agent observation and the terminal data plane are independent:

- **Control/observation plane**: agent identity, lifecycle, activity,
  readiness, turn end and control witnesses. Small, event-driven and
  per-change. It never carries screen or output bytes.
- **PTY plane**: output journal, screen snapshot and attachment streams.
  Only terminals with a viewer (or an explicit subscription) are read; byte
  delivery is attach-scoped and backpressured.

Readiness is a provider fact (ADR 0014), so observation no longer needs the
PTY probe. The foreground-process/paste-mode check stays inside the Worker at
submit time, where the transport actually uses it.

## Landed on 2026-10-30

- The native observation classifier moved to `yolo_hub_agent_observe`
  (target-scoped, no catalogue dependency).
- `GetAgent` and `AgentControl` pass their target terminal to the refresh;
  other catalogue views no longer reach their Workers. A call-shape test with
  two views proves only the target is observed.
- Host catalogue reads share one snapshot (`yolo_hub_host_catalogue`): two
  seconds of reuse, a complete snapshot also serving archived-free reads, and
  invalidation after every catalogue-mutating command. Tests prove
  coalescing, the archived-free derivation and invalidation.
- An idle binding whose digest, runtime epoch and worker match its stored
  observation reuses that witness for catalogue sweeps, list, get and
  control; only a changed binding or execution reaches the Worker. Tests
  prove the second sweep issues no observation and a changed binding does.
- `GetAgent`/`AgentControl` read exactly one `get_session` view, and the
  agent sweep reads only live (non-archived) sessions. A failed scoped read
  marks only its own session unreachable.
- An archived session is history on both planes. The projection settles it
  unreachable once instead of projecting it from the complete listing while
  the live sweep revokes it; the agent CLI requests the complete catalogue
  only for `--all`, and control commands resolve their selector against live
  sessions only. A steady catalogue therefore never rewrites the projection
  record: `agent list` fell from 207–250 ms to 57–62 ms cold and 16 ms in a
  burst (19 live terminals, 73 archived, image `30eefea7`), with the record
  hash and publication revision constant across repeated reads. With a live
  agent (`5decf7e8`): `send` 0.16–0.23 s (`Accepted`), `interrupt`
  0.44–0.82 s (`Confirmed`), `get` 29–41 ms, `list` 71–126 ms cold.
- Push replaces the per-command catalogue poll. A Worker exposes
  `WATCH_AGENT_EVENTS`: a binding-only stream with no output, screen or
  journal bytes. The Host keeps one subscription per live runtime and bumps
  a revision-stamped `ChangeFeed` on each binding event, catalogue-changing
  request or stream end; writer-lease traffic and terminal input do not
  touch it. A Worker without the stream is simply retried on the next read.
  The Host exposes `WATCH_CATALOGUE`; the Hub keeps one subscription, drops
  its catalogue snapshot and wakes publication on each hint, with a 30 s
  heartbeat as the fallback. A revision gap resynchronises once, and peer
  catalogues carry live terminals (archived sessions are local history).
- Measured with the same three live agents. Snapshot only (`f0a8d637`):
  `get` 0.22–0.66 s, `list` 0.18–0.38 s, `send` 0.40–0.91 s (`Accepted`),
  `interrupt` 0.84–2.7 s (`Delivered`). With idle observation reuse
  (`a4512459`): `get` 0.04–0.17 s, `list` 0.04–0.07 s, `send` 0.18 s
  (`Accepted`), `interrupt` 0.37 s (`Confirmed`); a Working agent's `get`
  still pays its Worker read (0.32 s). With the push image (`b858570d`)
  and eleven live workers: `list` 0.20 s, `get` 0.22–0.42 s, `send` 0.39 s
  (`Accepted`), `interrupt` 0.63 s; a binding update reached the Hub
  publication revision in 0.13 s instead of the old 2 s poll, and the
  revision stayed constant while idle. Before all of this, `get` 3.1–3.5 s,
  `list` 3.1 s and `send` 5.6 s.

### Sweep isolation (follow-up)

The read path still held the projection lock across the Host catalogue read
and one native observation per active binding. A Worker that paused while a
binding was being established (two simultaneous agent startups) blocked every
concurrent `list`, `agent list`, `get` and `wait` until the client's 5 s
timeout, even though the Host catalogue itself answered in ~50–130 ms.

Catalogue and agent reads now gather evidence before taking the lock:

- The Host catalogue, connector reconciliation and native observations are
  collected first. Native observations run concurrently (16 at a time) under
  one absolute 1 s budget, itself inside a 2 s sweep envelope shared with the
  Host read, so even a read queued behind a maximal sweep stays inside the
  client's 5 s budget; a Worker that misses its budget contributes no
  capability instead of stalling the read, and its row stays listed.
- An observation gate serializes sweeps, so the projection still applies
  observations in observation order. The projection lock itself covers only
  local record work; no Host or Worker call runs while it is held.
- Native control admission still needs the projection across its observation
  and effect, so it keeps that lock but takes the sweep gate first. A slow
  control effect can still delay catalogue sweeps; it cannot deadlock with
  them, and the read path no longer waits on Worker I/O at all.
- `conversation_binding` gathers its session view and witness under the same
  gate and budgets before projecting.

Regression coverage: a Worker that never answers its observation is bounded,
and the agent remains listed without submit authority
(`observation_budget` integration test).

## Work items, in order

1. **Stop sweeping per command.** Landed: `GetAgent` and `AgentControl`
   observe only their target, and an unchanged idle binding reuses its stored
   observation; `ListAgents` reaches a Worker only for a changed or working
   binding.
2. **Push instead of poll.** Landed: the Worker's binding events reach the
   Host over a dedicated output-free stream, and the Host's
   revision-stamped catalogue hints reach the Hub, which refreshes on change
   instead of on every poll.
3. **Cache and scope the catalogue.** Landed: repeated reads inside one
   command share a snapshot; archived sessions are excluded from agent
   observation, `ListAgents` pages its reply, the CLI reads the complete
   catalogue only when `--all` is requested, and control selectors resolve
   against live sessions only.
4. **Keep the PTY plane attach-only.** Landed: catalogue and observation
   paths read no journals or screens; the call-shape tests observe a get and
   a sweep without any attach or snapshot request, and idle bindings issue no
   Worker call at all.
5. **Effect-level conformance.** Call-shape tests cover target scoping,
   snapshot coalescing, idle reuse, changed-binding refresh and an N-idle
   sweep with zero Worker calls (landed); automated latency assertions
   remain a manual measurement.

## Acceptance

- `agent get`, `agent send` and `agent interrupt` latency is independent of
  the number of idle sessions.
- An unchanged idle agent receives no worker observation for a get/control.
- The Codex control path stays within the call deadline, so effect-level
  receipts (`Accepted`/`Confirmed`) are produced rather than resolved from
  the journal.
- No PTY bytes are read or sent for a terminal without a viewer.

## Consequences

- The Hub projection becomes event-driven; a Hub restart performs one full
  sync, then incremental updates.
- Capabilities are only recomputed when the provider binding or its turn
  journal changes; a control operation always refreshes its target.
- Archived or unviewed sessions cost nothing per command.
- Tests that assert the old per-view worker call order must be updated with
  the new call shape.
