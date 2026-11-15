# Verified local bundle source

`VerifiedBundle::open` reads only a bounded `manifest.json` and the fixed
`bin/yolo` runtime path beneath the caller's canonical bundle root. Root/bin directory
descriptors and all files remain held. Named-path replacement, symlinks, writable
group/other permissions, unexpected owners/hardlinks, invalid manifest, executable
metadata, binary target, digest or size are rejected.

The returned runtime payload is an immutable byte snapshot. `verify` rechecks the
retained and named object identities and metadata; it must be called throughout
deployment so a path replaced under the same name cannot become the source of a
later operation. No binary bytes appear in the management DTO.
