# Origin creation queries

`QueryCreation` observes the original creation transaction at its owning Hub.
It is a read-only protocol operation, including when forwarded through other
Hubs. It does not replay Create, settle a launch, repair receipts, reconcile a
session, grant a writer, or start a background retry loop.

## Identity contract

The logical creation identity is origin + creation token + Host request semantics.
The Host fingerprints its existing `CreateSession` serialization before resolving
cwd: creation_token, name, original cwd, cols and rows. Create and Query share
the same public-to-Host conversion.

Public `CreateShellRequest.request_id` and public schema_version are not stored
in the Host fingerprint. A different public request_id with the same token and
Host fields therefore refers to the same creation. The query does not prove the
original public request byte-for-byte. Each query has a fresh outer RPC request
ID; Missing is not cached as an idempotent mutation result.

The response's outer token must match the queried token, and every returned
address must belong to the authenticated response origin. Peer replies also bind
the entire signed call fingerprint. The origin checks the durable token digest,
request fingerprint, result and phase/completion before returning authority.

## Outcomes

| Outcome | Meaning |
|---|---|
| Missing | No matching evidence in this read. A delayed Create can still arrive. Never delete an intent or automatically Create from this result. |
| Pending | Reserved, Launching, SpawnFailed or Terminated activity exists, but no confirmed result is exposed. Query does not advance it. |
| Ready | The original committed creation result, not proof that its session is currently running or resumable. |
| Finalized / Survivor | The historical creation result and completion, not current session liveness. |
| Archived | The original address, without a creation capability. |
| Unknown | Stored evidence cannot establish the requested Host semantics. |

All outcomes describe the creation transaction. Current session state requires
a separate observation. A worker that started before Ready was persisted can
remain Pending; Query does not automatically resolve that earlier crash window.

## Receipt schema and read-only storage

Current receipts copy the binding's existing request fingerprint. They never
derive it from a mutable session manifest or from the resolved working directory.
Unsupported receipts are rejected without upgrade or rewrite. Queries do not
complete a missing receipt index.

The Host uses its retained namespace and a bounded creation-lock wait. It checks
the persisted ledger against its retained state and reads receipts through the
existing identity-fenced readers. Conflicting indexes, fingerprints, phases,
poisoned state or changed storage fail closed. Missing a retained entry or failing
a read does not become a successful Missing response.

The existing recovery/open/publish paths may write or clean files and are not
called by Query. A known, consistent single receipt index can be observed while
the absent companion index remains absent. Without a binding or token-index
record, Query does not scan all sessions to invent a result.

## Authorization and version domains

Query has `mutates = false` but requires Lifecycle authority, because its result
can reveal a creation capability. Observe-only delegation cannot issue it.
Intermediaries route the origin request and signed reply; catalogue caches are
not creation evidence. Errors, unsupported peers and route loss never trigger a
Create fallback.

Public clients use JSON-RPC 2.0 with the current Hub capability schema. The Mac
client performs its own authenticated Hello and validates that schema directly.
There is no separate desktop document schema. The app and Hub ship together;
a mismatched capability schema is unsupported.

The typed Query command is available through the public protocol. Clients remain
responsible for their prewritten intents and explicit recovery workflow; exposing
the query does not authorize automatic Create replay after an uncertain result.

## Validation scope

The component tests cover phase and receipt matrices, fingerprint disagreements,
Finalize idempotency, partial indexes without repair, namespace/ledger replacement,
bounded lock failure, Missing followed by later reservation, exact adapter
mapping, capability redaction, malformed replies, Lifecycle delegation and
origin/call correlation. Query snapshots compare bytes, identities, permissions,
mtime and ctime while excluding read-induced atime changes.

This protocol increment is applied, and its protocol/client/backend components
passed in the first unfiltered 30-target batch. After review, the affected Host
regression passed all 226 tests, including the Finalize fingerprint conflict
test. The combined live regression and complete package acceptance
remain separate gates. The component and Host regression suites cover the protocol increment. This is not acceptance of the full TUI creation-crash workflow or a
release.
