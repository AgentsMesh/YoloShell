# ADR 0028: Per-machine web catalogue revisions and patches

## Status

Accepted (2026-11-02). Refines the catalogue section of
[ADR 0026](0026-web-session-broker.md). No Hub protocol or Rust change.

## Context

The first session-broker slice pushed a whole aggregate catalogue snapshot to
every tab whenever any machine changed. With N machines, every 2.5-second poll
of one machine re-serialized every row from every machine to every tab. That
does not scale with machine count or tab count, and it makes a small local
change indistinguishable from a full catalogue replacement at the browser.

The browser already refuses stale snapshots and a slow subscriber is closed
rather than allowed to grow unbounded. What was missing was a per-machine
revision contract that can express a delta, detect a missed delta, and force one
subscriber back to a snapshot without disturbing every other tab.

## Decision

### Observation epoch and revision

Each `MachineObserver` owns an observation `epoch` (a fresh UUID for the
observer lifetime). Every published `MachineView` carries that epoch and a
monotonic `revision`. A new observer, a restarted observer, or a machine
identity change therefore cannot be mistaken for a continuation of the old row
sequence.

### Snapshot base

`CataloguePublisher` keeps the last **published** view per machine, not the last
observed one. It publishes:

- **snapshot** when a machine is first seen, its epoch changes, a row cannot be
  keyed, or a revision would move backwards;
- **patch** when the hub summary or keyed rows change;
- **nothing** when a poll changes only the observer revision.

The patch's `base_revision` is the last published revision. If several quiet
polls were skipped, the next real patch jumps from `base_revision` to the
current revision; every subscriber that followed the stream can still apply it.
A late subscriber receives the same published base, because `current` is built
from published views, not from unpublished observations.

### Wire shape

A full `catalogue` frame contains:

```text
snapshot {server, sequence}
hubs.hubs[] {machine, label, entry, reachable, freshness, epoch, revision,
             capabilities}
terminals.terminals[] and agents.agents[]
```

A `catalogue.patch` frame contains:

```text
snapshot {server, sequence}
machine, epoch, base_revision, revision
hub {machine, label, entry, reachable, freshness, epoch, revision, capabilities}
terminals {upsert: row[], remove: terminal_id[]}
agents {upsert: row[], remove: agent_id[]}
```

`sequence` is the aggregate publisher sequence; `base_revision`/`revision` are
per-machine. Both use opaque machine handles and sanitized rows, never a raw Hub
id.

### Browser application

The browser applies a patch only when:

- the aggregate `sequence` is newer for the same `server`;
- the local machine has the same `epoch`;
- the local machine `revision` equals `base_revision`;
- every upsert row has the required key.

Any gap, epoch change, server change or malformed key rejects the patch. The
browser sends `catalogue.snapshot`; the server answers that one connection with
a forced, newer snapshot sequence. It does not broadcast a full snapshot to
other tabs. Reconnect and slow-subscriber recovery still use a fresh snapshot.

## Consequences

- Catalogue traffic scales with changes, not total machines × tabs.
- Status-only changes (exit/finalize/reachability) are small upserts; archived
  rows are explicit removals.
- A quiet machine costs no catalogue frames even though its observer still
  polls for reachability.
- Patch correctness depends on every row having a stable terminal id or agent
  id. A malformed row forces a snapshot instead of an unsafe delta.
- Multi-tab views remain deterministic because every connection receives the
  same ordered aggregate sequence; only a private gap repair has its own
  sequence.
- No Hub event subscription is introduced. Machines still poll; the broker
  translates poll results into deltas.

## Evidence

Unit tests cover row upserts/removals, summary-only patches, quiet revisions,
epoch changes, unkeyed rows, new-machine snapshots, sequence monotonicity and
browser gap rejection. Live `/ws/session` acceptance observed a snapshot,
a requested resync snapshot, an upsert patch for a new shell, an exit-state
upsert patch, and an archive removal patch; the browser rendered each without a
console error. Live acceptance evidence is retained with the source revision.
