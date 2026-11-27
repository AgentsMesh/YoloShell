# Persistence Contract

## Durable Layout

```text
YOLO_HOME/
  catalog.sqlite
  creation-capabilities.json
  creation-receipts/
    tokens/<creation-token-digest>
    sessions/<session-id>
  runtime/
  sessions/<session-id>/
    manifest.json
    runtime.json
    worker-launch.json
    reboot-migration.json
    reboot-preparation.json
    resume-transaction.json
    agent-binding.json
    terminal/
      journal-00000000.bin
      snapshot.bin
    events/
      exit.json
      terminate.json
    logs/
```

`creation-capabilities.json` contains only active shell-creation transactions. Records
carry request fingerprints, allocated identities, private capabilities, launch identities,
and Host-side phases. Immutable completion receipts reach both synchronized indexes before
active removal, bounding the ledger without allowing a completed identity to allocate twice.

The active ledger and receipts are transaction metadata, excluded from the catalog,
manifest, `SessionView`, and list/get results. The desktop keeps corresponding workspace
state so a crash can resume binding, finalization, or capability-authorized cleanup.

Recovery-state publication writes the session manifest before the catalog. If the manifest
write fails, the original and desired recovery states are retained in memory and retried
during each reconciliation sweep under the session lifecycle lock. A newer durable state,
an archive, or a missing session supersedes the pending decision, and catalogue-only
failures remain rebuildable from durable manifests. Idle shutdown does not declare the Host
quiescent while a recovery-state write is still pending.

## Runtime Namespaces

Host and Worker Unix sockets are ephemeral and never live under the durable home. On
macOS their default namespace is `/private/tmp/y-<uid-hex>/<home-key>/`; it contains a
private full-digest binding, `host.sock`, and `<session-id>.sock` Worker endpoints. The
128-bit directory key is derived from the effective UID and canonical logical home, while
the binding stores the complete digest and UID so a truncated-key collision fails closed.
The namespace is retained by live Host and Worker processes and removed, with identity
fencing, only after its last endpoint disappears. Instance locks use a separate private
UID-scoped coordination root outside `YOLO_HOME`; retained SQLite aliases use private,
full-digest canonical-catalog-path shards below that root. Socket activation artifacts
stay within their descriptor-rooted namespace instead of sharing a global staging area.

Socket stage names `.b-<key>`, `.p-<key>`, and `.q-<key>` are reserved for Yolo. A
conforming writer must hold the per-target stage lease before creating or replacing one.
Recovery that exclusively locks a canonical lease and validates its full target digest owns
a private, single-link socket in that key's reserved stage slot and may unlink it. This is a
namespace ownership rule. Unsupported contract records are rejected and remain untouched;
the current recovery path never interprets them as cleanup authority.
A two-link probe is reclaimed only when the canonical endpoint has the same inode and target
digest. Unexpected type, UID, mode, link count, target identity, or any additional hardlink
fails closed and preserves the stage, endpoint, and lease.

Every Host and Worker endpoint publication is nested inside a retained namespace binding:
the shared binding lock is acquired before publication and released only after its socket
publication is removed. The active scavenger therefore treats an exclusive binding lock
as the managed liveness fence; it cannot delete a socket owned by a conforming Host or
Worker. Endpoint type, owner, mode, link count, and inode are still revalidated before
unlink. This does not claim protection from a continuously mutating, uncooperative same-UID
process.

The UID socket root has one fixed-size lifecycle sidecar shared by every home key. It is
reboot-ephemeral coordination, not durable product state, and intentionally remains after
the endpoint directory is removed: unlinking a live lock inode would permit split-lock
ABA. A per-path Mach bootstrap lease fences the sidecar on macOS, while `flock` provides
the descriptor lock and Unix fallback. Parent-directory mutations use this lifecycle fence;
endpoint proof and terminal traffic do not.

## Coordination Lifecycle

An instance guard acquires its per-key Mach lease, file `flock`, and pathname identity in
that order. Drop still holds both target fences while it takes the UID lifecycle lock and
directly unlinks only the twice-verified owned inode; a crash therefore leaves either the
canonical stale key or no entry, never shared retirement state. Foreground lock acquisition
never performs garbage collection. The background scavenger takes the lifecycle lock first
and only tries target leases and file locks nonblocking, so it cannot deadlock with Drop.
Each chunk has explicit entry, probe, and deletion budgets.

A coordination scan keeps one directory stream only for the lifetime of its current pass.
No directory cookie or ordinal is persisted or reused after reopening. Between chunks it
compares the root descriptor's device, inode, mtime, and ctime; a change discards the stream
and restarts at the beginning. That revision is only a restart hint and never authorizes a
deletion. Deletion still requires the target Mach lease, target `flock`, descriptor/path
identity fences, and an identity-fenced unlink. A successful unlink also discards the
stream. Unknown names and unproven types, owners, modes, links, ACLs, or identities are
preserved; unexpected I/O and stream failures propagate. Crashes restart from zero, and
eventual convergence requires a sufficiently long quiescent window.

Every admitted process must implement the current Yolo coordination contract. An
unsupported schema, unknown record shape, or process from an earlier contract is rejected
before publication or cleanup. The current process revalidates the canonical pathname and
inode after `flock`; it never treats an older guard as a mutual-exclusion participant.

## Catalog Retention Shards

Each retained catalog namespace holds a shared lease on the `.lease` file in its digest
shard for at least as long as its main, journal, WAL, and SHM VFS handles. Foreground opens
inspect only that exact shard. The retention scanner enters a shard only with a
nonblocking exclusive lease, streams bounded chunks, and preserves unknown or unsafe
content. Noncanonical entries are preserved; no alternate product format is interpreted or
rewritten.

Overlapping Host generations in one process share the same retained catalog capability when
parent device/inode and case-aware artifact name identify one physical slot. A textual path
that resolves to a different physical slot cannot replace that registration until its last
old-generation capability is released. SQLite generation transactions then fence delayed
watchdog writes without reopening the catalog through its public pathname.

An empty retention shard is removed only after dropping the scanner's first lease and
reacquiring locks in `UID lifecycle -> exclusive shard lease` order. The scanner then
reopens the shard through the retained parent descriptor, compares its identity, proves
that `.lease` is its sole entry, unlinks the owned lease, and removes the directory while
the lifecycle lock is held. Identity changes and nonempty or unsafe shapes are preserved;
resource and I/O failures such as `EMFILE` and `EIO` reach the maintenance backoff.

## Socket Recovery

The current 128-bit key is inspected before general recovery work. A matching complete
digest is reused. A different valid digest can move only after its binding lock upgrades to
exclusive and its directory and binding identities are revalidated. Its quarantine name is
`.retired-<complete-home-digest>`, so an occupied entry can block only that digest. If that
exact target is empty, it is removed under the lifecycle fence and the move is retried. If
it has a valid unlocked binding, one bounded cleanup job runs outside the fence before a
single retry. Busy, unsafe, invalid, or suffix-mismatched targets fail closed.

Valid retired names must exactly match the complete digest stored in their binding.
Unproven active bindings use the disjoint `.retired-invalid-<active-key>` namespace and
never enter the valid namespace. Any other name is preserved and is never interpreted as a
retirement record. Empty active directories and missing bindings remain in place because
neither state proves that liveness has ended. An invalid binding moves only while its
retained inode can be upgraded to an exclusive lock and its pathname identity still
matches.

Endpoint proof and deletion occur after releasing the UID-global lifecycle lock. A retired
cleanup validates its binding, then streams a complete proof up to the shared maximum
managed-session limit while retaining endpoint identities in memory. Unknown content or
overflow causes zero unlink and leaves the complete retired directory intact. A proven
set is deleted once with per-entry inode revalidation, keeping supported cleanup O(n). The
cleanup briefly reacquires the lifecycle lock only to remove its still-empty, identity-
matching directory; a missing, reused, unsafe, or nonempty entry is preserved as a
successful handoff. Retired binding acquisition is nonblocking, so concurrent maintenance
skips an entry whose cleanup owns the exclusive binding lock. The background scanner keeps
one live enumeration pass across bounded chunks and restarts that pass after a detected
root mutation; it never persists a directory cookie across process or stream lifetimes.

## Durable Reboot Recovery

Durable reboot mutation holds both the session lifecycle lock and the session directory
lock. The fixed-slot preparation authority is published before any stage or receipt helper
and binds the session directory identity, transaction, participant plan, source identities,
and source and target digests. Recovery uses only that authority; it never scans names or
deletes objects by prefix or shape.

The authority moves through `Active`, `Settled`, and `Retired`. Every participant mutation
and every authority transition revalidates the exact authority inode, session directory,
intent absence or identity, and participant identities. A Settled target is accepted only
with the same device, inode, birth generation, length, and modification timestamp; a
replacement inode with identical JSON is not accepted.

This contract supports cooperative Yolo writers. It does not protect against a malicious
or continuously mutating same-UID process that ignores the lifecycle and directory locks.
macOS has no portable unlink-by-handle primitive, so the final `verify(pathname) ->
unlinkat(pathname)` interval is protected by that cooperative lock contract. Yolo does not
claim that pathname race is eliminated by a kernel atomicity guarantee.

## Maintenance Ownership

One UID-global maintenance owner runs all registered scanners with independent cadence and
bounded error backoff. Host starts maintenance only after socket ingress is ready; Broker
starts it only after both endpoints bind. Shutdown first closes admission and requests worker
cancellation, then releases the replaceable socket publication, namespace retention, and
instance guards before awaiting scanner completion. Thread handles move into an independent
blocking reaper before that await, so a stalled scan cannot block controller replacement and
canceling the async shutdown cannot fall back to a synchronous join or detach an untracked
thread. Scanner startup and runtime failures are retried in that background fault domain and
cannot block an already published Host or Broker.
