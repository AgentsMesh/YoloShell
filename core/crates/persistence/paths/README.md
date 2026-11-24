# Retained paths and durable identity

`ObjectIdentity` and `FileIdentity` remain live observations for descriptor/path
fences, file locks and compare-and-swap operations. Their device fields are not
weakened or normalized when a filesystem is reopened.

`PrivateDirectory::durable_identity()` captures the independent filesystem object
proof from its retained descriptor, with strict pathname verification before and
after capture. Persistent stores compare this proof at open, then retain their
normal live namespace authority. Routine namespace verification does not repeat
the filesystem UUID/export-handle query.

`identity::DurableFileVersion` adds length, modification time and change time to
that object proof. `same_content_version` excludes change time for transitions
which separately prove a rename; `unchanged` includes it. Capturing and verifying
these versions validates current private-file ownership, permissions, link count
and length through `FileIdentity`. The owner still retains its path/ACL evidence
and mutation lock. Successful durable verification produces a freshly captured
live `FileIdentity`; the durable record itself is never a live CAS token.

`NamespaceLease::read_json_with_versions` reads the JSON twice through one opened
file, fences its strict live version, captures durable evidence from that same
descriptor, and rechecks the canonical entry before returning all three values.
It cannot pair old JSON with the identity of a same-value replacement file.

Socket home registration uses the current durable-home format. Admission
verifies the durable home once and retains the registration's strict live file
identity afterward. Retired formats and valid proofs for other directories are
rejected without automatic record repair. Socket directory certificates likewise
persist durable identities for their directory and registration file, allowing a
custom persistent socket root to be reopened after a filesystem remount. Initial
admission verifies both proofs, then caches the immutable certificate. Namespace
and orphan-stage checks compare that certificate under strict retained live
directory/file fences, without recapturing filesystem identity on the hot path.
Creating a certificate remains an xattr create-only operation; existing values
are never repaired or overwritten. Orphan cleanup still removes only proven
empty transport state after participant leases are gone.
