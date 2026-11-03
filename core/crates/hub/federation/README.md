# Federation observation persistence

The observation cache admits at most one storage thread per Hub in this process,
across all federation generations. Admission uses the in-memory Hub identity before
any cache I/O begins. A blocked or retiring worker retains admission until it
actually finishes; another generation disables its cache without starting another
thread. Different Hubs have independent admission.
Disk loading finishes before restored observations enter the new directory and
before peer admission starts. Loading has a bounded deadline; errors or a stalled
cache disable persistence for that generation while live services can start.

Live discovery submits complete observation snapshots to a single pending slot.
New submissions replace an older pending snapshot while an active write finishes.
The directory lock and Tokio workers never execute cache disk I/O. Persistence is
best effort: after a crash, recovery returns the latest completed durable snapshot
for each origin, which can lag the most recently observed live snapshot. Seeing a
live observation is not a durable-write acknowledgement. Cross-origin writes have
independent commit points.

Generation shutdown closes submission, discards pending snapshots, and retires
the storage token checked at commit fences. A write that already crossed its
commit fence may finish; the worker retains its coordination lock until it exits.
An explicit async stop waits only for a bounded interval. Drop never joins a disk
thread. Completion is announced only after the store and worker admission are
released, so healthy generations can hand over immediately. If an old worker
remains blocked, a successor starts with caching unavailable. Pending snapshots
are not replayed by a retired worker or handed to a successor implicitly.

The cache stores authenticated catalogue observations only. Restoring them cannot
restore delegation chains, network connections, exports, or executable routes.
Cache unavailability and stalled retirement are reported through tracing.
