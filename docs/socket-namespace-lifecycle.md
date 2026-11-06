# Socket namespace lifecycle

Socket addresses are disposable transport state. Session manifests, Hub identity,
and namespace ownership live in the private application state home. Deleting a
temporary regular file cannot invalidate a surviving registered service.

## Authority and lifetime

The immutable `.socket-namespace.json` in the state home binds the full home
digest, user ID, schema, and home directory identity. Every participant retains a
shared lock on that exact file inode. Recovery and final empty-directory removal
require an exclusive lock on the same inode. Replacing a record with identical
JSON does not preserve authority. Each participant also holds a shared kernel
lock on the actual socket directory inode. Admission acquires both leases under
root coordination before exposing the namespace. Losing the durable home cannot
release a live participant's directory lease.

The socket directory carries a create-only `user.yolo.namespace` extended
attribute. It binds the logical namespace to its actual directory inode and
durable registration inode. It survives ordinary-file aging in `/tmp`; copying
it onto another directory does not produce a valid certificate. Missing or corrupt
durable authority for an occupied certified directory is an integrity error, never
permission to initialize replacement authority. The current certificate schema
requires directory leases; earlier certificates are rejected without migration.
There is one ownership model: durable
registration plus directory certificate. Temporary ownership markers are neither
created nor accepted.

## Admission and lookup

`retain_socket_namespace()` admits a participant. It can initialize private state
and recover interrupted initialization. Path getters only validate existing state.
Automatic TUI, Hub, bridge, control, and managed catalog startup retain their
required namespaces before calling validated path getters and keep the guards
through service handoff.

| Observed state | Admission |
| --- | --- |
| No socket directory | Retain/create durable registration, create and certify directory |
| Valid certificate and matching registration | Retain shared ownership and verify again |
| Current certificate, exclusive directory lease, actually empty directory | Remove abandoned transport inode and admit a fresh directory |
| Uncertified directory | Require exclusive registration and prove abandoned transport |
| Live endpoint, publication in progress, unknown entry, unsafe permissions | Preserve state and report refusal |
| Corrupt certificate or missing/replaced registration while occupied | Preserve state and report lost authority |

Root coordination uses an exclusive lock on the root directory inode and a fair
process-local admission turn. It creates no regular lock file. Renaming an owned
root preserves its kernel lock; substituting a directory invalidates its identity.

Recovery of interrupted certification holds root and publication coordination.
Its bounded inventory accepts only recognized, private, single-link socket nodes.
Each endpoint is pinned by a temporary hard link and probed with a nonblocking
connection. Only an immediate `ECONNREFUSED` proves absence; success, pending, and
all other outcomes refuse recovery. Directory, inventory, locks, registration and
absence of a certificate are checked again before the namespace is renamed to
`.unbound-<unique-id>`.

Recovery preserves every original endpoint in that quarantine and creates a fresh
certified directory at the stable address. Quarantines are outside automatic
garbage collection. Sessions and history are never deleted by recovery. Unknown
files, including obsolete ownership records, are preserved without interpretation.

## Release and crash residue

Endpoint owners withdraw only their exact published socket inode. Namespace
release obtains exclusive durable and directory ownership and cleans proven publication
bookkeeping. It holds publication coordination while proving every endpoint stale
with exact-inode probes; a live or unknown entry prevents endpoint removal. Each
unlink rechecks the remaining inventory, durable authority, certificate and socket
identity. Only an actually empty directory is removed. A later owner can recover
preserved stale endpoints through socket publication.

Read-only lookup rechecks the directory certificate after reading durable
registration so a concurrent directory replacement cannot pass validation.
Background scanning visits only certified namespace directories. It can clean
independently locked publication bookkeeping after checking the exact certificate
before and after each bounded scan. To reclaim an abandoned directory, the scanner
also needs an exclusive directory inode lease, a current certificate and an inventory
containing only current publication bookkeeping or nothing. Admission uses this
same bounded cleanup after socket withdrawal has left target lease records. Every
scan chunk rechecks root, certificate and directory identity. Active target leases
remain untouched. It removes proven idle bookkeeping and then removes only an
actually empty directory. This works after a
crash even when the durable home is gone. A live participant, endpoint, unknown
entry or invalid certificate prevents retirement. Scanner authority never permits
endpoint deletion. Each scan chunk holds root coordination, preventing it from
recreating stage locks during final release. Resetting or dropping a scanner does
not grant namespace cleanup authority.

Callers retiring a durable home can use explicit `release()` to distinguish
`Busy` (another shared owner, including a settling PTY guardian), `Preserved`
(unproven cleanup), and `Removed`, and observe errors before retiring the home.
Releasing one participant is not proof that the final owner has left.

Cleanup work is bounded by the existing directory-entry, target and scan-chunk
limits. The publication lease timeout governs waiting for a lease; it is not a
cumulative deadline for filesystem reads, synchronization and verified endpoint
removal. A slow completed filesystem operation does not make proven ownership
unproven. Root coordination and publication lease acquisition retain their own
timeouts, and isolated test-root settlement retains its outer deadline and work
budget. Filesystem calls themselves cannot be interrupted by checking a clock
between iterations. Cleanup adds no unbounded retry loop and grants no authority
to remove live endpoints, unknown entries or substituted objects.

## Regression boundaries

Filesystem integration tests cover certificate survival during ordinary-file
cleanup, interrupted certification, intact quarantine preservation, live listeners,
pending publication, path/record replacement, missing certificates, shared and
exclusive ownership, unknown files, symlinks, hard links, private modes, scanner
behavior, root inode locking and empty-directory release. Pure record validation
tests remain separate from filesystem integration tests.
