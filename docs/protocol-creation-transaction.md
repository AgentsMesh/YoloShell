# Shell creation transaction

Creating a shell for a new tab or split pane uses the same creation transaction.
Before sending the public `create` call, the desktop durably reserves a fresh, non-nil creation token and the intended pane.
Only the caller that commits the `reserved` to `creating` transition dispatches
creation. Recovery of `creating` queries that original operation and never
resends it merely because a response is missing. The token identifies the logical
creation; reusing it with a different request fingerprint fails closed.

A successful create returns a stable session ID and a private creation
capability. The desktop durably records that binding before it changes the
workspace split tree. It then takes exactly one of two paths:

- bind the session to the intended pane with a layout-revision CAS, persist
  the `bound` phase, and retry `finalize_creation` until acknowledged; or
- persist cleanup intent before each side effect, then capability-authorize the
  terminate and archive steps until the orphan is durably archived.

The Host reserves a new token in a short synchronous ledger transaction. Once a
session ID is known, create retries, capability cleanup, finalize, explicit
resume, and automatic reconciliation serialize on that session and re-read the
ledger or receipts inside a short synchronous critical section. No worker,
network, or exit-wait await holds a cross-session creation lock. An unfinalized
creation cannot be explicitly or automatically resumed, so cleanup cannot
terminate one runtime while resume installs another. A finalized creation
cannot later use its capability for cleanup.

Finalization and cleanup completion are recorded as immutable private receipts
indexed independently by creation-token digest and session ID. Both indexes are
published and synchronized before the active creation record is removed.
Startup repairs a one-index publication and removes only active records whose
matching completion receipt is durable. Receipts are protocol bookkeeping;
they are never returned by list/get, inserted into the catalog, or exposed as
user sessions.
