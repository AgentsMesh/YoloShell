# ADR 0021: Plugin OS hardening and third-party trust status

Status: superseded by [ADR 0023](0023-trusted-only-plugins.md). The
Host-supervised plugin runtime, sandbox, cgroup, seccomp and invocation journal
described below were removed. This document is historical.

## Context

ADR 0020 selected a Host-supervised plugin process with broker-mediated grants
and staged OS hardening: process hygiene, Linux namespaces, then macOS
sandboxing and third-party plugin trust. The platform now ships the first two
stages and an opt-in bwrap sandbox; this ADR records what is implemented, what
is deliberately deferred, and the concrete blockers for the rest so future work
does not re-litigate them from memory.

The product decision for this plan is that plugin code is a compiled-in
first-party module. Native or WASM third-party plugins, marketplaces and remote
distribution were explicitly out of scope.

## Implemented

- One Host-supervised process per invocation, no Hub/Host/Worker socket, no
  client key, scrubbed environment, a private `0700` data directory as `cwd`
  and `YOLO_PLUGIN_DATA`, new session, process-group termination.
- `umask 077`, `RLIMIT_CORE=0`, `RLIMIT_NOFILE=64`, `RLIMIT_FSIZE=16 MiB`,
  clamped to inherited hard limits.
- Opt-in `YOLO_PLUGIN_SANDBOX=bwrap`: `bwrap --unshare-all --die-with-parent
  --new-session` with read-only system/image roots, private tmpfs/proc/dev.
  A configured sandbox without bwrap fails closed as plugin unavailable.
- Opt-in `YOLO_PLUGIN_CGROUP=required`: one `pids.max=64` leaf per invocation
  under the Host's delegated root, parent-side child migration with exact
  membership verification, and `cgroup.kill` on every exit path. Unset or
  `off` keeps the previous behaviour; any other value fails closed when the
  delegated `pids` root is unavailable.
- Opt-in `YOLO_PLUGIN_SANDBOX=sandbox-exec` on macOS: deny-default seatbelt
  profile with `/tmp`-only writes and denied fork/network operations.
- Opt-in `YOLO_PLUGIN_SECCOMP=required`: a bwrap-installed BPF denylist for
  process creation, networking, mounts, namespaces, tracing and kernel
  operations, plus a reviewed post-exec `prctl` owner that denies `execve` and
  `execveat`. Required mode needs bwrap and fails closed without it.
- Durable invocation journaling with Unknown never replayed, and broker calls
  rechecked against the grant every round.

## Blocked or deferred

### seccomp residuals

The reviewed owner is `runtime/engine/process/seccomp`; the process-signal
policy permits only its `install_aarch64` function and the
`PR_SET_NO_NEW_PRIVS`/`PR_SET_SECCOMP` options. Bubblewrap cannot deny
`execve` because it must exec the plugin first, so the plugin adds the second
filter after startup. This leaves a small pre-filter interval for `execve`;
closing it requires a launch-time boundary that can install a filter after
exec without the plugin's cooperation, which is not available in the current
bwrap interface. The repository acceptance suite covers filter installation and the pre-filter launch window.

### cgroup controller limits

The canonical `yolo-host.service` delegates only the `pids` controller
(`Delegate=pids`, `DelegateSubgroup=supervisor`), which the opt-in
`YOLO_PLUGIN_CGROUP=required` mode now uses. Memory/CPU/I/O controllers are
unavailable without changing the running custody service, which must preserve
live PTYs. The plugin leaf is created after spawn and the Host moves its child
into it; cgroup v2 delegation rejects self-migration from a sibling process,
so the child cannot place itself and a plugin that forks before the join can
escape the leaf. That window is acceptable for compiled-in first-party code
but must be closed before third-party plugins are considered. The repository acceptance suite covers the cgroup join window.

### macOS sandbox

Opt-in `YOLO_PLUGIN_SANDBOX=sandbox-exec` now wraps each invocation in
`/usr/bin/sandbox-exec` with a deny-default profile: `process-exec` and
`file-read*` are allowed, writes are limited to the canonical plugin data
directory plus `/tmp`/`/private/tmp`, and
process forking, networking and other operations stay denied. This is a
filesystem/process boundary, not the Linux namespace+seccomp model; it has no
private `/tmp`, no network namespace and no syscall allowlist.

The macOS Rust build blocker was a linker bug: Xcode 27's `ld` emits a
mis-aligned LINKEDIT string pool when `-target` is combined with the Rust
proc-macro link. `--config=macos` now pins the 14.2 deployment target and
passes `-Clink-arg=-Wl,-no_fixup_chains` to both host and exec rustc, which
produces loadable proc-macro dylibs. A serial seatbelt unit test launches a
script through the profile and verifies limits/umask. The spawn-gate
normalizer preserves an existing SIGCHLD handler instead of replacing it with
`SIG_DFL`, so the full macOS host/plugin suite passes serially. The repository acceptance suite covers the seatbelt profile.

### Third-party plugins

Compiled-in first-party modules keep code review, image digests and grant
review in one supply chain. Accepting third-party code needs a separate
decision covering at minimum: distribution and signing, manifest/digest
approval, sandbox capability parity across Linux and macOS, update/rollback,
and a marketplace trust model. None of that is implemented, and no native/WASM
loader should be added opportunistically.

## Consequences

- The current security claim is: process/fd/env isolation, resource limits,
  broker-only effects, optional Linux namespace, pids-cgroup and seccomp
  containment, optional macOS seatbelt confinement, and durable invocation
  evidence. The seccomp filter is a denylist, not a complete allowlist; the
  macOS profile is filesystem/process confinement, not a namespace sandbox.
- Any future seccomp/cgroup work must start with an architecture-policy owner
  and a service-delegation plan, then add a live probe that proves the filter
  or cgroup is active during an invocation.
- Third-party plugin work starts with a new ADR and product decision; it must
  not reuse the compiled-in catalog as an implicit trust boundary.

## References

- [Plugin isolation and trust](0020-plugin-isolation-trust.md)
