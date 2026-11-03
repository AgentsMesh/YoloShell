# Enrollment journal

Each remote Hub stores attempts in `enrollments/<request UUID>/record.json` beneath
its existing state directory. The record contains the complete bootstrap request,
the remote public identity, the exact pre-change configuration snapshot, the exact
configuration change (or explicit no-change evidence), and an optional completed
receipt. Disk records also bind the Hub state, journal, and entry directory object
identities. They contain no SSH credentials or private Hub keys.

`claim` holds a per-request coordination lock. An existing entry yields only an
`Observation`; an entry without its record is rejected instead of reinitialized.
A new `Claim::prepare` durably publishes the prepared record with an expected-absent
file CAS before returning `SubmitPermit`. Both claim and permit are process-local,
non-cloneable values and cannot be deserialized from journal contents. Only the
original permit owner may take the first submission path. Dropping it leaves the
prepared record in place, including when the caller crashed before sending anything.

`SubmitPermit::complete` uses the retained file identity for its completion CAS and
revalidates the record and directory bindings before returning. A replaced entry,
even with identical JSON, cannot acquire or complete the old attempt.

`inspect` performs only bounded private-file reads and retained identity checks;
it does not create directories or coordination locks. Its `Observation` must stay
alive across external observations and be verified again before returning a result.
The remote startup layer separately requires the exact currently applied config,
public peer identity, and live service images before interpreting either a prepared
or completed record as ready. A superseding config or revocation is never replayed.
