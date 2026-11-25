# Hub SSH profile registry storage

The store retains one private directory, its birth identity, the owning Hub ID
and public key, a coordination lock, and the exact configuration file identity.
It reexports the neutral `hub/ssh/profile/protocol` DTOs for existing consumers.

Opening without a seed initializes an editable empty registry. `apply` performs
an optimistic revision/request-ID transaction, so profiles can subsequently be
added, replaced, or removed. Only an exact replay of the latest committed request
and configuration is idempotent. Stale revisions and reused request IDs return
`RevisionConflict` without writing; exhausted revisions return `RevisionExhausted`.

Before commit, the store rechecks the retained namespace, lock, file identity,
and previous record at the final publication fence. It updates its retained
record only after an acknowledged atomic write and verifies the result again.
Write errors can mean the change committed; callers must not replay them as new
requests. Storage/identity failures latch the current owner until explicit reopen.

`current`, `configuration`, `verify`, and `resolve` read and validate existing
state without changing profile configuration or its record. Resolving an unknown
or superseded reference does not remove profiles or poison a healthy store.
Neither opening nor updating this registry loads credentials, starts a broker,
opens SSH, enrolls a Hub, or falls back to TCP.

Directory cloning, changing the Hub key, replacing a loaded record with identical
JSON, and replacing/restoring the namespace cannot silently change the authority
of a retained store instance. The caller still owns its configuration-operation
UI, explicit transport binding, and runtime application/reconciliation workflow.
