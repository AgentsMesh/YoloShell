# Runtime bundle contract

`yolo_distribution_bundle` describes one release of the `yolo` executable,
which launches explicit, independently running process roles. It contains no filesystem operations,
transport, credentials, deployment selector, or build-tool provenance.

Schema 2 has exactly `schema_version`, `source_revision`, `version`, `target`,
`build_id`, and `artifacts`. `target` has `platform` (`linux` or `macos`) and
`architecture` (`aarch64` or `x86_64`). `artifacts` has exactly `runtime`,
with `sha256` and `size_bytes`. Digests and source
revisions use lowercase hexadecimal. Source revisions contain 40 or 64 bytes.
Versions contain 1–64 ASCII alphanumeric, period, or hyphen characters.
Artifact lengths are nonzero and bounded by `MAX_ARTIFACT_BYTES`.

The build ID is the lowercase SHA-256 digest of this canonical byte sequence:

1. The UTF-8 bytes `yolo.runtime.bundle.identity.v2` followed by a NUL byte.
2. The schema version as an unsigned 16-bit big-endian integer.
3. Source revision, version, platform, and architecture, in that order. Each
   string is UTF-8 prefixed by its byte length as an unsigned 64-bit big-endian
   integer.
4. The `runtime` artifact entry: append its role string, digest string, and byte size. Strings
   use the same length prefix; sizes use unsigned 64-bit big-endian integers.

`build_id` itself is excluded. No JSON serialization participates in the hash;
JSON key order and whitespace therefore cannot change the build identity.
Duplicate, missing, and unknown JSON fields are rejected by typed decoding.
Prior schemas and retired artifact names are rejected; no compatibility aliases exist.

The executable path is derived solely from the validated identity:
`releases/<build_id>/bin/yolo`. The caller must validate and own the
installation root and keep all process launches pinned to one release.

`verify_artifact` checks exact length, digest, and supported binary target.
Header detection identifies ELF64 or thin Mach-O64 for supported architectures;
it does not prove that a binary can execute. Release acceptance must separately
run executable identity/version probes on the destination platform.
