# ADR 0016: Wait for agent states instead of polling

## Status

Accepted on 2026-10-30. Implements the typed long-poll primitive; the
observation-authority tiers in ADR 0017 remain proposed.

## Context

Clients need to wait for an agent to reach a state before acting: "wait until
it is waiting for input", "wait until it needs approval", "wait until it
exits". Without a primitive, callers poll `ListAgents` and pay a catalogue
round trip per attempt, and every client re-implements the same state
matching. `agent ask` already waits, but only for its own turn completion, and
the conversation watch polls at a fixed interval.

A typed wait avoids repeated catalogue polling and duplicated state matching;
the difficult parts are exact target identity and change boundaries.

## Decision

Add one public Hub command, `WaitAgents`:

- **Targets** are exact `AgentTarget`s (agent id + execution id), so a wait is
  pinned to one execution and reports its replacement instead of silently
  matching a new one. Every evaluation reads the projection together with the
  live terminals' primary agents: history keeps an old snapshot, so only the
  current-execution set can prove that a target ended.
- **Conditions** are typed: `all`/`any`/`not` over `activity`, `lifecycle`
  (state only, exit code ignored), `reachability`, `freshness`, plus the
  witness-backed `promptable`, `interruptible` and `respondable` capabilities.
  Lists are any-of. There is no screen text, no free-form token, and no string
  status.
- **`after`** is an optional observation cursor. With it, a condition only
  counts once the target observed a state newer than that cursor; without it,
  an already-true state matches immediately.
- **The wait is bounded**: at most `MAX_AGENT_WAIT_MS` (1 s) per call. The
  client re-issues until its own deadline. The reply carries `matched`,
  `ended` (targets whose pinned execution was replaced or removed) and
  `timed_out`.
- **Runs are selectors, targets stay exact**: a wait may name durable
  `AgentRun`s by address or name instead of executions. The owning Hub
  resolves each run to its terminal's current execution once, deduplicates
  the set, and echoes the evaluated `targets` in the reply so the client can
  correlate a target it never named. A run with no current execution is
  `NotFound`; the caller re-issues by run on its own deadline.
- **The Hub evaluates on its projection.** A wait subscribes to catalogue
  change hints before its first read, re-reads the projection on each hint,
  and rescans once per second as a guarded fallback. The Host feed already
  signals binding changes and worker exit; the rescan only covers a missed
  hint. A waiter never installs a private poll loop.

A wait is evaluated by the Hub that owns the target. The CLI addresses it
the same way as `GetAgent`: the entry Hub forwards the peer-addressed read,
and the owning Hub waits on its local projection. The wait engine rejects a
target that does not belong to the Hub evaluating it.

The CLI exposes `yolo agent wait <selector> --until idle|working|
waiting_for_input|waiting_for_approval|exited|unreachable|stale|promptable|
interruptible|respondable`, repeatable
(any-of via `any`), with `--timeout`.

## Design choices

Adopted:

- Server-side deadline with a bounded long-poll and client re-issue.
- Subscribe-before-evaluate, so a change between "check" and "wait" is not
  lost (we subscribe to the hint channel and re-read the projection).
- Identity pinning: a wait on an execution that is replaced or removed ends
  with an explicit result instead of hanging until the deadline.
- An "only after this point" mode for effect checks, expressed with the
  observation cursor.
- Reporting a timeout distinctly from an unsupported or missing target.

Rejected:

- Screen-text conditions (`pane.wait_output`) and a coarse status enum:
  observation stays typed and never reads PTY bytes (ADR 0015).
- 100 ms probe polling: the Host already pushes binding and catalogue changes.
- One fixed target per wait: the request carries a set, and the result reports
  every matched snapshot and every ended execution.
- String predicates: conditions are a closed AST validated at the protocol
  edge (64 targets, 32 nodes, depth 4).

Transient states that appear and disappear between two evaluations are not
matched yet. If that matters, a bounded per-agent transition ring with a
sequence and a lost-history marker can be added without changing the request
shape; subscribe-before-evaluate makes it unnecessary for ordinary waits.

## Consequences

- No new durable state: a wait is a read of the projection plus a wake source.
- The protocol change is additive; clients discover it through `Hello`.
- `yolo agent wait` returns the matching snapshot, a rejection when the
  execution ended, or a timeout; `agent ask` keeps its specialised turn wait.
- Waits share the catalogue cache and observation sweep with the publisher
  and commands; the extra sweeps are warm reads, and no Host or Worker I/O
  runs under the projection lock.
