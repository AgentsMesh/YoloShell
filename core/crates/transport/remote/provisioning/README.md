# Hub provisioning

The broker admits a request only in Hub transport mode, with the complete configured
profile and exact cached broker instance / SSH connection / generation binding.
The service receives that already authenticated SSH connection and an admission
guard. It cannot create credentials, reconnect the pool, or accept arbitrary
commands or file reads from the caller.

Management requests carry `BundleSource { root, descriptor }`; the verified local
reader retains the root, bin directory, manifest and all three executable files.
Manifest, metadata, object identities, byte counts, hashes and target headers must
match before immutable byte snapshots are handed to the installer. Source objects
remain retained and verified through the operation.

Each operation is bound to the full immutable request and profile under a per-ID
coordination lock in the broker's private `hub-provisioning` directory. Records
bind parent/root/entry object identities, and every transition is a file-identity
CAS. The state machine is:

`Prepared -> Publishing -> Published -> Enrolling -> Complete`

The prepared record contains the installer's exact publication request before any
upload or publication. Publishing and Enrolling are durable before their respective
side effects. Only a process-local owner may submit. Cancellation and uncertain
responses leave the last durable phase; ordinary repeated Provision calls perform
read-only recovery and never repeat an upload, publication or bootstrap command.

Explicit Advance may acquire a new enrollment permit only from durable Published:
the publication is read back and matched, then the record is CASed to Enrolling
before one bootstrap. Prepared, Publishing, Enrolling and Complete never qualify.
This preserves a useful continuation after a known publication without interpreting
an uncertain phase as permission to repeat a side effect.

Recover is read-only locally and remotely. It confirms only the original publication
and uses the remote enrollment journal's recovery command for Enrolling/Complete.
Historical completion does not suppress live build/config/pin validation. A newer
configuration that revoked enrollment is returned as Superseded. Unknown retains
its original phase and request.

The dedicated broker RPC uses the shared 300-second absolute operation budget and
observes the local management socket while the operation runs. EOF/cancellation
drops the future before a new phase is polled; the existing SSH channel custody
settles work already in flight. Each SSH operation and each journal mutation is
fenced by the current profile, broker namespace and SSH generation guard.
