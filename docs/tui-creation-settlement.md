# TUI Confirmed Creation Settlement

## Problem

A successful Create leaves a Ready creation binding at the origin until the
client finalizes it. That binding has no expiration. Losing the automatic
Finalize through a full local queue, lifecycle admission limit or disconnection
can leave a usable shell that the Host later refuses to resume.

The TUI previously removed its pending creation before trying to enqueue
Finalize. It retained no settlement record after enqueue failure. A reconnect or
client restart therefore could not complete this obligation.

## Narrow Retry Contract

The TUI retains a bounded set of confirmed creations until a correlated
Finalize completion has been durably acknowledged. The complete original
creation token, origin address and creation capability remain unchanged on
every attempt. The Host already accepts an identical Finalize against a durable
Finalized or Survivor receipt; it rejects an archived or mismatched identity.

This is an explicit Finalize-only retry contract. It does not retry Create,
Resume, terminal input, agent Interrupt, or any other uncertain mutation.
Capacity and disconnected admission failures retry after a bounded delay;
permanent protocol rejections remain visible and retain the journal record.

Each attempt owns a local RequestScope. Failure or NotSent must match that
attempt's scope, deadline and generation before scheduling another attempt.
An exact earlier Completed may settle the same creation and cancel a queued
replacement. Neither outcome grants writer authority or changes an unrelated
pane. An expired attempt can be retired even if its task failed without a
completion callback.

A live writer may resize the terminal while Finalize is still pending. Runtime
geometry records the current canonical size; the creation manifest and immutable
Worker launch retain the original requested geometry. Finalization checks both
contracts separately, together with the unchanged session, execution, worker,
authentication and process identities. A legitimate resize must not leave a
usable shell permanently rejected as a different creation.

## Persistence Boundary

`tui/creation/journal` stores a separate private JSON document; capabilities are
never added to the workspace layout. The document binds the entry Hub and the
retained workspace directory identity. Pending request IDs, creation tokens and
session addresses cannot collide. Reads, writes and acknowledgements validate
the complete record, namespace, ownership lock and current file identity.

The ordering is:

1. Validate Created against the admitted request and origin.
2. Durably retain its exact request and result.
3. Bind the original pane only if its binding intent is still current.
4. Dispatch Finalize over an independently owned lifecycle connection.
5. Durably remove the record only after a matching Completed response.

Restart loads settlement obligations only. It restores no writer lease,
connection generation or pane-binding intent. Closing the original workspace
or pane does not cancel confirmation of an already-created shell.

Persistence errors keep the confirmed result in memory and pause new shell
creation. Recovery must retain the same namespace and ownership lock and accept
only an exact previously observed document/file identity or the exact new
identity captured by the write's commit observer. Arbitrarily reopening a
replacement file is not recovery authority.

Retained failures remain visible while Search, Directory and confirmation forms
show their active prompt. The header preserves the blocked-confirmation count
and reports unsaved shell changes; leaving the form restores the detailed error.
Changing presentation never discards an obligation or grants writer authority.

## Boundaries

The focused regression must include queue saturation, reconnect, old-attempt
failure and success ordering, missing completion callbacks, pane A→B→A rebinding,
and background-workspace failures. Journal tests cover restart, conflicting
identities, entry/directory changes, replacement, exclusive ownership, bounded
records and recoverable write failure before/after publication.

See [creation recovery](tui-creation-recovery.md) for ordering, scope fencing,
result classes and its separate live acceptance. Replaying Create is never a
substitute.
