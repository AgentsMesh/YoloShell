# Runtime bundle artifacts

The release catalog now describes complete runtime bundles. A bundle contains
`manifest.json` with the shared `distribution/bundle::BundleDescriptor`, plus
`bin/yolo-host`, `bin/yolo-hub`, and `bin/yolo-file-helper`. A single Host binary
does not qualify as an installable Hub bundle.

The catalog schema is 2. Its root has `schema_version`, `source_revision`, and
exactly two Linux `runtimes`, for x86_64 and aarch64. Each runtime contains
`image`, `bazel_sha256`, `bundle_root`, and `descriptor`. The source revision,
version, target, build identity, and three artifact sizes/digests live in the
shared descriptor. Catalog paths remain the existing `linux/amd64` and
`linux/arm64` directories; executable paths use the descriptor's closed roles.

The Linux gate builds all three targets in the same frozen source/toolchain
invocation, identifies each configured executable through Bazel cquery, runs
each exact program-name/version probe, and validates ELF architecture and glibc
requirements for every component. All inputs must verify before the architecture
repository is created. The per-bundle descriptor and global catalog are sealed
into the Linux artifact repository used by the release application target.

Release provenance schema is 8. Linux gate evidence binds `bundle_id`, not a
Host-only binary digest. The package contains six Linux executables and their
two descriptors. Bundle inventory and permission policies cover all six. The
Rust `audit-bundle --manifest` command strictly decodes the shared descriptor and
checks each packaged executable; the package audit also binds its build ID to
the sealed global catalog.

Swift catalog reading verifies the same canonical build-ID encoding, all three
files, sizes/digests, Linux binary targets, matching version/source provenance,
and equality with the bundle-local manifest. A complete choice exposes
`RemoteRuntimeBundleProfile`, whose JSON is the Rust `BundleSource` shape:
`{root, descriptor}` with an absolute local path string. Existing Host consumers
use an explicit `hostArtifact()` projection. They do not grant Hub installability.

A developer app without release catalog/source provenance still exposes its
existing local Host artifact, with no complete bundle value. No source revision
is invented for those bytes. A build-produced local bundle needs real provenance
and the same complete directory contract before it can be offered for Hub setup.
