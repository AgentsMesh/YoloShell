# ADR 0027: Durable web creation intents

## Status

Accepted (2026-11-02). Refines the creation-job section of
[ADR 0026](0026-web-session-broker.md). Adds no Hub protocol or Rust change.

## Context

Creating a shell or starting an agent is not idempotent by itself. A browser can
lose the HTTP response, a tab can be reloaded, or the plugin can restart after
the Hub accepted a mutation but before the reply was recorded. The first
session-broker slice kept `CreationJob` records in memory, so a restart erased
the only evidence that a mutation might have happened. Retrying then had two
bad outcomes: silently create a duplicate session, or claim a failure that was
not established.

The Hub already exposes the recovery contracts the TUI uses:

- Shell creation has a stable `creation_token`; `query_creation` takes the
  original `CreateShellRequest` and returns durable evidence with outcomes
  `missing`, `pending`, `unknown`, `ready`, `finalized`, `survivor` or
  `archived`.
- `agent_start` uses `start_token` as the durable run identity. Replaying the
  same token and request resumes the recorded run and returns the same terminal.
- `query_creation` never replays `Create`. `missing` and `unknown` are not proof
  that no effect occurred.

## Decision

### Journal before dispatch

Every creation begins with a private on-disk `CreationIntent` under
`YOLO_PLUGIN_DATA/creation-intents/`, written with mode `0600` in a `0700`
directory. The record contains the origin Hub id, kind, normalized retry
fingerprint, the browser idempotency key and the complete original
`CreateShellRequest` or `AgentStartRequest` (including its creation or start
token). It is written and fsynced before any Hub mutation.

The runner checkpoints `dispatched` before sending the mutation. A restart can
therefore distinguish work that was never sent (`prepared`, safe to dispatch)
from work whose fate is unknown (`dispatched` or `unconfirmed`, reconcile
only). The journal is not exposed through the browser; sanitization also strips
any creation capability from job output.

### One intent per retry key

The browser generates one idempotency key for an identical creation form and
keeps it across a lost response. The server maps the key to one intent and one
job id. A duplicate POST with the same key and normalized request returns that
job; reusing a key for a different request is `409`. The browser forgets the key
only after a confirmed success or a definite rejection; `unknown`, timeouts and
network failures retain it.

The server retains settled/failed jobs for 24 hours and caps the journal at 512
records. Unconfirmed evidence is never silently deleted. If the cap is reached
with unresolved intents, new creations are refused rather than evicting
evidence.

### Shell recovery

A fresh intent dispatches `create`, then `finalize` with the server-only
creation capability. An uncertain create or finalize reply moves the intent to
`unconfirmed`; it never sends `create` again. Recovery calls `query_creation`
with the persisted request:

- `ready`: finalize the returned capability, then settle;
- `finalized` or `survivor`: settle from the returned address;
- `archived`: settle with the archived address;
- `missing`, `pending` or `unknown`: retain the intent and retry the query with
  bounded backoff; the public job is `unknown`.

An older Hub that does not implement `query_creation` leaves the job `unknown`.
That is deliberate: the web client cannot prove absence of an effect.

### Agent recovery

An uncertain `agent_start` is retried with the exact persisted request and
`start_token`. The Hub's run store treats the token as the idempotency key, so a
retry resumes the existing run/terminal instead of creating a second one. A
definitive name/validation conflict fails the intent; an uncertain outcome stays
`unconfirmed` for retry.

### Public job state

Jobs are projections of the journal, not the journal:

- `running`: prepared or in flight;
- `unknown`: the Hub has not confirmed the effect; do not submit a new intent;
- `done`: settled result (opaque machine handle and terminal id only);
- `error`: a definite rejection.

Recovery runs once at plugin startup and then continues automatically with
bounded backoff. `GET /api/jobs/:id` rehydrates a job after a plugin restart.

## Consequences

- A plugin crash, lost HTTP reply or reloaded tab cannot duplicate a creation
  that still has its retry key, and cannot erase evidence of an uncertain one.
- Shell recovery depends on the Hub's `query_creation`; old peers degrade to an
  honest `unknown`, never a replay.
- The browser store is per-tab. Closing the tab discards the retry key; the
  server still refuses to auto-recreate, but a user-initiated new request is a
  new intent. The UI must direct the user to check Agents and Shells after an
  unknown outcome.
- Creation journaling assumes one web plugin process for one data directory.
  Multiple concurrent plugin processes sharing the directory are unsupported.
- `query_creation` is a read-only lifecycle operation; no new Rust capability,
  socket or browser route is introduced.

## Evidence

Unit tests cover journal ordering, private permissions, duplicate keys,
normalized retry comparison, lost shell replies, `missing` without replay,
agent token reuse, restart recovery and key-retention policy. Live acceptance
covers two plugin restarts during real shell and agent creation, duplicate POSTs
returning the original job, and exactly one terminal per intent. The
architecture conformance target passes. Live acceptance evidence is retained with the source revision.
