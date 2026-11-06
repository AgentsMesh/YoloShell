# One Runtime Executable, Independent Processes

Yolo distributes one Rust executable, `yolo`. Launching it without an internal
role opens the TUI. An explicit `internal <role>` prefix selects a service entry.
The executable basename never selects a role, and retired standalone programs
are not compatibility entrypoints.

## Component Boundaries

| Component | Owns | Must not own |
| --- | --- | --- |
| `application/launch/contract` | Executable identity, explicit role names, opaque argument routing | Environment reads, services, process spawning, business dependencies |
| `application/yolo` | Dispatch to an entry library, diagnostic/exit-code policy | Session state, IPC handlers, persistence, terminal rendering |
| `application/host/cli` | Host command parsing, executor selection and detached-session initialization | TUI or Hub presentation |
| `application/attach/cli` | Local terminal attachment CLI and inherited status-descriptor admission | Hub federation state |
| `application/remote/cli` | Broker argument parsing, typed conversion, dispatch and output | Credentials, reconciliation, child custody, persisted supervision |
| `application/broker/profile/authority` | Profile document integrity and locked start/retirement authority | Process creation and CLI types |
| `application/broker/startup` | Authorized generation reconciliation, adoption and retirement | Raw child handles and CLI parsing |
| `application/broker/supervision` | Detached launch, activation, readiness and supervisor lifecycle | Profile authority and arbitrary record mutation |
| `application/broker/child/custody` | Pending child ownership, compensation, handoff and reaping | CLI options and startup policy |
| `application/broker/supervision/state` | Validated launch description, record transitions, anchored persistence and scavenging | Credential input, process launch and presentation |
| `hub/cli` | Hub service, bootstrap and publication entry composition | Worker implementation |
| `hub/attach/cli` | Federated terminal attachment entry | Native Worker custody |
| `hub/file/helper` | Bounded file-operation worker and its stdio protocol | Arbitrary remote command execution |
| `tui/cli` | TUI argument parsing and connection selection | Hub/server business logic |

Entry libraries expose synchronous `run_from` functions accepting explicit
`OsString` arguments. Each retains its role's existing executor and initialization
ordering. In particular, Host detachment precedes executor threads. The composition
root does not install a universal async runtime around all roles.

Broker startup depends on profile authority and supervision; supervision depends
on custody and supervision state. The state layer has no dependency on the
entry, startup or supervision implementations. `PendingBroker` owns the live
child until adoption or compensation; dropping it retains the cleanup path.
Records expose validated transitions rather than public mutable fields.
CLI conversion preserves the original deadline and describes the credential
source; it neither opens stdin nor grants authority. The startup/supervision
operations read private credentials only after their existing admission checks.

Source, unit tests, integration tests and test support stay in separate directories.
Small entry components use focused modules such as arguments, selection, entry and
dispatch; their service implementations remain in the existing domain crates.
The composition root is restricted to four production source files and 160 lines,
with only entry-library, launch-contract, `anyhow` and `clap` dependencies.

## Runtime Contract

The internal invocation forms are:

```text
yolo internal host <host-command> ...
yolo internal hub ...
yolo internal attach ...
yolo internal remote <broker-command> ...
yolo internal hub-attach ...
yolo internal file-helper
```

Worker, guardian, bootstrap-child and provider-hook commands belong to the Host
entry library. They still execute in independent processes. Separate entry
functions do not transfer leases, session identity, file authority or descriptors;
those continue to cross the existing authenticated and bounded IPC interfaces.

The shared contract in `build_system/bazel/process_role_contract.bzl` generates Rust
role constants. Launchers prepend the exact role and preserve all remaining
arguments, including non-UTF-8 Unix paths. Helpers launch the current executable
image; they do not substitute a sibling based on basename.

Clap help/version keep successful exit status and usage failures keep their usage
status. File-helper runtime failures preserve their silent diagnostic boundary.
Replacing a process role does not replay Create, Input, Interrupt or Resume.

## Packaging and Remote Publication

The standalone installation needs only `yolo`; the first-party TUI is part of that
binary. There is no application bundle or native AppKit entry.

Remote runtime bundles use schema 2 with one `artifacts.runtime` descriptor and
one payload at `bin/yolo`. The bundle identity domain is
`yolo.runtime.bundle.identity.v2`. Retired three-program manifests are rejected;
there are no Host/Hub/FileHelper artifact aliases. Atomic publication, immutable
release identities, retained file handles and read-only recovery stay enforced.

Host, Hub and File Helper still report their independently observed live image
identities. Each must match the single runtime descriptor. A new file on disk
does not prove that already-running processes have changed their mapped image.

Runtime artifacts use the existing 512 MiB live-image bound across publication,
native bootstrap and hook-runtime attestation. Provider executables keep their
256 MiB admission bound. Trusted callers select the image kind from the field or
process being checked; serialized evidence cannot select its own size policy.
Both kinds retain complete content hashes, mapped-image checks and filesystem
identity, owner, link-count and permission validation.

## Architecture and Acceptance

The architecture checker requires exactly one production Rust binary target,
`//core/crates/application/yolo:yolo`. Explicit test-only fixtures and the
adversarial runtime remain separate test artifacts and never enter a release.
The checker constrains entry dependencies and source budgets, preserves the pure
launch contract, and rejects hidden production binaries through unreviewed rules
or macros. Adversarial tests cover aliases, wrappers and mutated declarations.

Acceptance includes role dispatch from one actual image, local and remote shell
creation, provider hooks, independent Worker survival, retained writer authority,
native and TUI recovery, immutable remote publication and exact package inventory.
Passing compilation or a version probe alone does not establish those outcomes
or the repository's strict coverage and production-release gates.
