# Offline inventory crash acceptance

The entry-crash test first observes the live A/B/C view, then independently reads
the entry's published observation-cache files before sending SIGKILL. The index
must contain B and C, and each indexed record must have the expected entry and
directory identities, a valid origin signature, and a structurally valid catalogue.
The stored agents must match the previously observed execution targets, terminal
bindings and working directories.

The probe pins private directories, uses bounded identity-checked JSON reads,
and rereads the index and shards to reject a mixed publication view. Only missing
index membership, a valid older snapshot, or an observed concurrent file replacement
can continue waiting. Invalid JSON, signatures, indexed-file absence, permissions,
directory substitution and unsupported schema fail the test. Waiting has an
absolute deadline; read-only filesystem work runs on one separate probe thread.

This proves that authenticated observations already published in the filesystem
can be restored after process SIGKILL, without restoring control authority or
inventing remote process death. A live directory reply is not a persistence ACK.
The probe does not prove the writer finished every fsync, physical media durability,
power-loss recovery, or persistence of every latest live message.
