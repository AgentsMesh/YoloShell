# Offline imported Hub inventory

An entry Hub persists authenticated origin catalogues in its private observation
cache. After restarting without its relay, it can display the last persisted
remote Agent/Session identities and working directories as stale and unreachable.
No delegation chain, route, connection, writer capability or process custody is
restored from the cache. Reconnection requires fresh authenticated authority.

Each origin has a bounded signed record and an index entry bound to the entry
Hub and cache directory. Reads and commits validate retained file identities;
same-byte replacement is a conflict. Unknown files are neither adopted nor
removed, and invalid indexed content is preserved for diagnosis. New origins
write their record before publishing index membership. An interrupted membership
change cannot turn an orphan record into an accepted observation on restart.

The cache is best effort. Live observation and a successful terminal exchange do
not acknowledge a cache write. New snapshots coalesce into one pending slot while
one earlier write finishes. After a crash, different origins can restore their
independently committed snapshots; newer uncommitted observations can be lost.

## Failure isolation

Disk work runs in a dedicated thread, with at most one such worker per Hub in the
process across all configuration generations. Admission happens before any cache
I/O. A stalled worker retains its admission and storage lock; a successor starts
with caching unavailable instead of creating another blocked thread.

Startup loading has a deadline. Only validated, successfully loaded history enters
the directory before peer admission. Live services do not wait indefinitely for
cache I/O. Submission takes a short in-memory lock, never the storage write path.
Retirement discards pending snapshots and revokes later storage commit fences.
A commit already past its fence may settle while retaining its original lock.
Shutdown waits for a bounded interval; Drop does not join a stalled disk thread.
Healthy completion releases storage and worker admission before announcing exit.

Caching failures currently appear in tracing; a client-facing cache-health
surface remains product work.
