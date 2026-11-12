# ADR 0018: Agent run lineage

## Status

Accepted on 2026-10-31. Extends the durable `AgentRun` identity in the
platform-elevation plan.

## Context

Orchestration needs to answer "which run requested this one?" so callers can
reconstruct trees, audit fan-out, and later apply team or quota policy. The
question must be answered by durable identity, not by terminal grouping: a run
outlives its execution, and a terminal can be reused by a later run.

The tempting alternatives are wrong:

- inferring parentage from the caller's terminal at query time, which changes
  whenever an execution is replaced and cannot represent parallel children;
- deriving it from labels or names, which are mutable and have no referential
  integrity;
- letting the parent relationship carry authority, which would bypass every
  existing access class and federation attenuation rule.

## Decision

Add optional parentage to the public agent-run contract:

- `AgentStartFields.parent` is an `AgentRunSelector` (name or address). The
  owning Hub resolves it once to an exact `AgentRunAddress` before any
  terminal or run effect. A selector that does not resolve is `NotFound`; a
  parent on another Hub is `OriginMismatch`; a deployment without a run store
  is `Unsupported`.
- The resolved address is stored on the run record and exposed as
  `AgentRunSummary.parent` by `get_agent_run`, `list_agent_runs` and the
  platform snapshot. `update_agent_run` cannot change it.
- `AgentRunFilter::Parent` lists a run's direct children; page order remains
  the run address.
- The start token remains the idempotency key. A retry with the same token
  must resolve to the same parent or returns `Conflict`; it never replays the
  launch.
- Parentage is metadata, not authority. It grants nothing over the parent and
  changes no wait, prompt, stop or terminal precondition.
- Persisted records must keep parents before children. The store validates
  this order on load, which proves referential integrity and acyclicity
  without a separate graph walk. The record schema version stays 1 because
  the field is optional and old records load with no parent.

## Consequences

- Callers can reconstruct orchestration trees from durable run data and can
  list direct children without scanning every run.
- Cross-Hub parentage is rejected today. A federated tree needs the peer
  catalogue work in Phase 4.
- Core bounds are enforced at reservation: at most 128 launched runs per Hub,
  32 launched direct children per parent, and a parent chain of depth 16.
  Rejection is `LimitExceeded` before any effect. Only launched runs consume a
  slot, so concurrent in-flight starts can overshoot by the number of requests
  in flight; configurable and group quotas remain future work.
- `--current` does not auto-parent yet, and launched shells do not receive a
  `YOLO_RUN_ID`; callers pass `--parent`. Observed-run migration (creating a
  run for an execution that was not started through Yolo) remains deferred,
  so such runs have no parent.
