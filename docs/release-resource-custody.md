# Release resource custody

Release cleanup follows retained object ownership and positive consumer
settlement. A command returning an error is not proof that every process or
container using its directories has ended.

## Temporary objects

`TemporaryDirectory` owns its original parent/root descriptors, registered child
identities, sealing and cleanup. It immediately relinquishes TempDir's pathname
destructor. Cleanup checks retained identities, claims the root through an
exclusive retirement rename, verifies the claimed object and traverses through
directory descriptors. A mismatched claim is restored without replacement; an
occupied original name leaves the claimed object preserved with its exact path.

Snapshots, candidate archives, macOS Bazel, Docker, Linux Bazel and command
scratch directories use this owner. Delivery stages retain their separate
prepared/published policy and share the same resource-use mechanism.
Secure random retirement names use the system entropy call without reopening a
random-device descriptor for every entry.

## Consumers and settlement

`DirectoryResourceOwner` serializes admission and retirement. Active consumers,
unconfirmed releases or poisoned tracking prevent retirement. A retired owner
cannot admit another consumer. Child handles inherit ancestor trackers; nested
owners add their own retirement boundary without retiring the ancestor.

`ProcessCommand` retains its working, home and temporary directories plus declared
tool/artifact resources. Resource use is acquired before spawn and armed when
posix_spawn succeeds. A failure before creation releases safely. After creation,
normal or fallback cleanup releases only with discovery settlement, direct-child
reaping, empty process group and stdout/stderr EOF. An unsettled child preserves
all affected temporary owners. A positively settled timeout or nonzero exit can
still clean up normally; process outcome and settlement are distinct facts.

Docker containers have an additional engine-side owner. The pinned Docker client,
engine identity, exact name, complete container ID, owner label, image, command
and bind mounts constrain removal. The immutable mount specification supplies
both launch arguments and the trusted inspection expectation. The snapshot and
read-only mounted Bazel directory remain retained until exact settlement.

Unknown creation with no acknowledged container ID and no observed name is not
proof of absence: creation may still complete at the engine. Daemon errors,
identity mismatch, unexpected mounts and uncertain cleanup preserve resources.
Known owned IDs are removed individually; no prefix or broad container cleanup
is authorized by this protocol.

## Trust boundary

The producer executes a trusted repository revision and is not a sandbox for
arbitrary programs running as the same OS user. Cleanup detects the tested
replacements before and during retirement, and unique private names prevent
ordinary reuse of the original paths from redirecting cleanup.

macOS unlink operations do not accept an expected inode identity. A hostile
same-UID actor that deliberately discovers and continues modifying retirement
names, or clears their protections, can race the final name-based unlink. Extra
identity checks alone cannot make that operation atomic. The producer therefore
requires temporary namespaces without such hostile concurrent mutation; it does
not claim isolation against that actor. This remains a documented platform trust
limit, not a completed hostile-namespace containment feature.

This limit never excuses cleanup over an unsettled process or container created
by the release operation itself. Those legitimate consumers are tracked and must
produce settlement evidence or force preservation, including error unwinding.
