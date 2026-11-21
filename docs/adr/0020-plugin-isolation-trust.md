# ADR 0020: Plugin isolation and third-party trust

Status: Superseded by ADR 0023 (trusted-only plugins); historical (2026-10-31). Process hygiene and the opt-in Linux bwrap
sandbox are implemented. Third-party native/WASI trust is superseded by
[ADR 0022](0022-third-party-plugin-distribution.md); the macOS sandbox and
external-runner decisions here remain background.

## Context

A server plugin is a Host-supervised process. The Host starts the Yolo image
with `internal plugin --plugin-id`, clears the environment except `PATH`, sets
`cwd` to its private data directory, creates a new session, exchanges one bounded Cap'n Proto
document over stdin/stdout, and terminates the verified process group after the
invocation (`core/crates/runtime/engine/host/plugin/src/supervisor.rs`). The
plugin has no Hub, Host-management or Worker socket, no client key and no
ambient environment. Every effect goes through the Hub-authorized broker.

That is process and fd isolation, not an OS sandbox. The plugin still runs as
the same UID and can open world-readable files, attempt socket connections,
fork within limits, or consume CPU until the invocation deadline. This ADR
records the threat model, the checks that passed on the current hosts, and the
staged path to a real sandbox.

## Threat model

- **In scope**: a buggy or malicious compiled-in plugin; accidental ambient fd,
  environment or credential leakage; runaway resource use; effect escalation
  outside its grant; process-tree escape on timeout; durable unknown effects.
- **Out of scope for now**: a compromised Hub/Host/Worker binary; a malicious
  local user with the same UID; kernel compromise; remote plugin distribution.
- Third-party native/WASM code is not accepted. Compiled-in first-party code is
  still treated as untrusted at the broker boundary.

## Observed platform capabilities (2026-10-31)

Linux with cgroup v2 and a delegated `pids` controller:

- `unshare --user --map-root-user true` exits 0.
- `unshare --user --map-root-user --mount --pid --net --fork true` exits 0.
- `/usr/bin/bwrap` is installed.
- `/sys/fs/cgroup` is `cgroup2fs` with `cpuset cpu io memory pids`.
- `/proc/sys/kernel/yama/ptrace_scope` is `1` (same-UID ptrace is restricted to
  descendants).
- The Hub state directory is mode `0700`.
- `yolo-host.service` has `Delegate=yes` and `DelegateSubgroup=supervisor`.

macOS `27.0.1` (macOS host):

- `/usr/bin/sandbox-exec` exists; a no-op `(version 1)(allow default)` profile
  exits 0. `sandbox-exec` is deprecated but still functional on this host.
- App Sandbox and XPC service isolation remain available options, but neither
  is wired into the plugin runner.

## Options

1. **Process hygiene only (current)**. Session group, env scrub, a private
   `0700` data directory as `cwd` and `YOLO_PLUGIN_DATA`, no
   ambient fds created by the Host, `RLIMIT_NOFILE=64`, `RLIMIT_CORE=0`,
   `RLIMIT_FSIZE=16 MiB`, `umask 077`, bounded frame exchange, exact
   process-group termination, broker rechecks, invocation journal. Residual
   same-UID risk is explicit.
2. **Linux namespaces (implemented as an opt-in)**. With
   `YOLO_PLUGIN_SANDBOX=bwrap`, the Host launches the plugin through
   `bwrap --unshare-all --die-with-parent --new-session`: `/usr`, `/lib`,
   `/lib64`, `/etc`, `/bin` and the image directory are read-only, `/tmp` is a
   private tmpfs, `/proc` and `/dev` are fresh, and the plugin is PID 1/2 in a
   fresh PID namespace. The plugin still uses stdio as its only broker
   transport. If the variable names an unsupported mode or bwrap is absent,
   the invocation fails as `plugin unavailable` instead of silently launching
   weaker.
3. **macOS sandbox**. Run the plugin under `sandbox-exec` with an explicit
   profile (deny network, deny file writes outside an approved data directory,
   allow reads of the image/runtime) or migrate the runner into an App Sandbox
   XPC service. `sandbox-exec` is deprecated, so the profile is an interim
   boundary; an XPC helper is the durable path.
4. **Third-party code**. Only after (2) and (3) are proven: signed bundle,
   manifest digest approved against the running image, mandatory sandbox, no
   ambient secrets, mandatory grant attenuation, and a separate distribution
   decision. Marketplace and remote distribution stay out of scope.

## Decision

- Keep option 1 as the baseline. This revision adds the resource-limit and
  umask part; `PR_SET_NO_NEW_PRIVS` is deliberately deferred because the
  process-custody policy currently rejects every raw `prctl` call.
- Ship option 2 as the opt-in `YOLO_PLUGIN_SANDBOX=bwrap` mode. The default
  stays the process-hygiene baseline; the Host launcher must forward the
  variable through its reset environment. Seccomp and cgroup placement are
  follow-ups, not requirements for this mode.
- Implement option 3 before accepting any third-party code on macOS.
- Do not accept third-party native/WASM plugins, marketplace distribution or
  remote installation in this ADR. They require their own decision after the
  sandbox and trust-chain design.

## Consequences

- The current residual same-UID risk is documented, not eliminated. A plugin can
  still read files the invoking user can read and attempt network/socket access
  until the sandbox slice lands.
- The bwrap mode depends on `/usr/bin/bwrap` and on the Host launcher
  forwarding `YOLO_PLUGIN_SANDBOX` through `setpriv --reset-env`. The tracked
  service unit does not set it; the canonical deployment sets it in a drop-in.
- Killing the bwrap process group is still the custody operation; bwrap's
  `--die-with-parent` plus PID-namespace teardown contains the inner plugin.
  The Linux test verifies that no descendant survives a timeout.
- Namespace work belongs in the process-spawn/custody layer, not in the broker;
  the broker remains the only effect channel.
- Each plugin gets one Host-owned data directory that survives invocations and
  image updates. bwrap binds it writable after the private `/tmp`; the macOS
  seatbelt profile allows writes only under it (plus `/tmp`). There is no
  aggregate size quota or revoke-time cleanup yet.
- Seccomp, cgroup placement and a macOS sandbox profile remain. The Linux mode
  is not a confidentiality boundary for files under the read-only system roots.
