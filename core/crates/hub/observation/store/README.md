# Offline observation cache

`ObservationStore::open(PrivateDirectory, PeerIdentity, RoutingLimits)` owns a
private directory namespace and a coordination lock for the complete Hub
generation. `history(&self)` returns authenticated observations;
`record_history(&mut self, &ObservationHistory)` upserts origin snapshots.
The owner must drop this store before a successor generation opens the same path.

`open_with_retirement` additionally accepts a cloneable, one-way `Retirement`
token. Retiring it rejects later reads and commit fences without waiting for
filesystem I/O. An operation already past its fence may settle; the old store
keeps its coordination lock until that I/O finishes and the store is dropped.
Retirement never gives a successor permission to bypass the old writer's lock.

The bounded index contains its schema, directory object identity, exact entry
identity and committed origin IDs. Each origin has its own typed, atomically
replaced JSON record containing those bindings and one signed catalogue.
Neither index nor records can represent delegation chains,
connections, routes, writer leases or process custody. Every catalogue signature
and structure is validated, including origin uniqueness and capacity. Expired
catalogues are accepted only as observations; restoring them cannot make a route
or export available. OriginBook merges disk and retired in-memory history before
the new generation accepts peer connections.

Reads retain file identity and contents. Writes compare-and-swap that identity
under the namespace and coordination lock at the commit boundary. A same-byte
file replacement during ownership, renamed directory, copied record in another
directory, incorrect entry identity, malformed record or lost lock is rejected.
Storage failures permanently retire the current instance; invalid caller data
does not. Existing bad indexed data is neither overwritten nor silently initialized.
An absent index initializes only in an empty private cache directory. Identical
valid snapshots cause no write. Absent origins remain as offline history; a newer
signed snapshot can remove sessions or become empty, and a live caller can replace
an origin's old signing identity after its configured pin changes.

Encoding and reading retain the paths crate's 1 MiB limit per origin record,
with total known-origin count bounded by routing policy. Encoding is
counted without allocation before writing; oversized input is never truncated.
A first-origin commit writes its record before adding it to the index. If it
stops between those writes, the old index's committed observations remain readable.
Unindexed origin files and atomic-write residues are never imported as history.
No scan adopts or removes unknown files. A conflicting unindexed origin file
blocks that write; the paths layer retains its existing per-target staging rules.
Cross-origin updates have independent commit points and do not claim a global
transaction; restart can restore different origins' last persisted observations.
The Hub owner must isolate cache failure from live/local services and expose the
cache as unavailable. No recovery of cached observations grants control authority.
