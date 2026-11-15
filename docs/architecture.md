# Runtime Architecture

## Fault Domains

```text
TUI (built-in first-party client)
      |
yolo internal hub-attach
      |
entry Hub -> ... -> origin Hub -> Host supervisor
      |
      +-- detached worker -- PTY -- shell -- Codex
      +-- detached worker -- PTY -- shell -- Claude Code
      +-- detached worker -- PTY -- arbitrary TUI
```

The client, relay, and host are replaceable controllers. A terminal worker solely owns one PTY master and one child process tree. Replacing a controller must detach it without signaling a worker or its child.

The first-party client is the TUI packaged with `yolo`; there is no AppKit or Swift client. One Rust `yolo` executable supplies independent Host, Hub, relay, broker and worker processes on local or remote machines. See the [single-executable contract](single-executable-architecture.md) for component and launch boundaries.

The TUI is presentation only; critical lease, PTY, SSH, provider, and recovery state belongs to Rust. Background terminals retain no view, and layout never restarts a relay. Public clients and trusted plugins use JSON-RPC; the local management socket exposes the full public JSON-RPC dispatcher to same-UID trusted plugins. Internal services and Hub federation use Cap'n Proto, and local resource transfer uses SCM_RIGHTS. See [communication ownership](communication-architecture.md) and [trusted plugins](adr/0023-trusted-only-plugins.md).

## Remote Transport

Rust owns a pooled `russh` connection for each configured remote endpoint. Host keys
are verified before authentication against fail-closed known-hosts data; authentication
uses the user's SSH agent or configured identity files. The same connection opens
independent host-control, terminal-attachment, bounded-command, and SFTP-read channels.

Control and attachment channels execute the configured absolute `yolo` path with
`internal host bridge --stdio`. The bridge attaches to or starts the detached remote host, so losing
an SSH transport drops only its bridge channels. Reconnection opens a new transport,
resumes each attachment from its acknowledged output sequence, and sends no termination.

Command execution uses an explicit program allowlist, argument-safe shell quoting,
timeouts, and output limits. File reads use scoped, bounded SFTP v3 REALPATH/LSTAT/FSTAT
fencing. SFTP v3 has no inode, ctime, nanosecond timestamp, or portable no-follow open,
so this is not an `openat(O_NOFOLLOW)` guarantee against same-UID swaps; see ADR 0005.

Before the first bridge, Rust probes the remote architecture and ensures the configured
`yolo` runtime artifact is installed. The installer accepts only a manifest-verified ELF
or Mach-O artifact for that architecture. It uploads to a `0600` temporary file, reads
it back for SHA-256 and length verification, changes it to `0700`, and renames it to an
immutable content-addressed release. A temporary symlink is atomically renamed onto the
stable path; failures before that rename leave the previous executable selected.

Runtime, command, SFTP read, and deployment operations each have an independent deadline covering channel creation and protocol setup. A stalled channel and its connection can be replaced
even while another channel keeps the SSH transport active. The broker itself
is process-creation free; [ADR 0008](adr/0008-remote-broker-process-containment.md) defines the
supervisor-owned containment boundary and its reparenting limit.

## Identities

- `host_id`: stable local or remote host identity
- `terminal_id`: stable logical Terminal UUID
- `pane_id`: persistent client view identity
- `controller_instance_id`: one relay-process incarnation
- `runtime_epoch`: one PTY lifetime
- `worker_id`: one worker-process incarnation
- `boot_id`: machine boot identity
- `provider_session_id`: identity persisted only in the detected `AgentBinding`
- `output_sequence`: monotonic cursor within a runtime epoch

A PID is never enough to adopt or signal a process. Adoption validates terminal, runtime epoch, worker identity, boot identity, process start time, and a private authentication token over the
worker socket. Persisted process locators use `(pid, start_time)`; macOS destructive authority
uses a live `(pid, unique_id)` identity, and ancestry uses `parent_unique_id`, never numeric PPID.
Exact signaling lets XNU match PID plus `pidversion`; `exec` refreshes that token while retaining
`unique_id`. Missing symbols or metadata errors fail closed without a `kill` fallback.
[ADR 0006](adr/0006-macos-process-signal-fencing.md) defines the complete contract.

## Process Ownership

`yolo internal attach` runs inside the TUI's local PTY and owns no managed PTY. It streams
terminal bytes, forwards resize and input over persistent framed connections, and maintains
the writer lease. Stdin EOF releases the lease and detaches only.

The relay uses a stable pane holder and a fresh controller instance. A reopened workspace waits
in read-only mode until its predecessor releases or expires. A live incarnation is replaced only
by an explicit takeover fenced to the observed lease generation, even when the pane holder matches.

`yolo internal host serve` owns the durable catalog and starts each worker in a detached process group. A replacement
host increments its controller generation and adopts workers without changing runtime identities.

On macOS, reconciliation takes two complete `PROC_UID_ONLY` inventories, fencing every
session lookup around `getsid`; kqueue only invalidates, and a post-inventory fence drains
new events and watcher audits before publication. Discovery handoff leaves manager and
watchdog custody with only the reader while launcher, exact root, and descendants inherit
the non-`CLOEXEC` writer. That exact writer handle is the sole kernel membership authority; the random environment value is only positive discovery and classification evidence, never negative or settlement authority. Pre-exec acknowledgement binds exact root identity and both reciprocal endpoint handles, then proves the root holds the writer.
Since `dup` and `SCM_RIGHTS` can transfer it, writer-present discovery scans the complete stable same-UID universe, including unrelated and pre-root holders, and independently proves the exact handle for every member. Environment scrub does not hide a writer, while explicitly closing it before registration relinquishes membership.
Pipe EOF after exact root exit closes the process-discovery membership universe because queued `SCM_RIGHTS` copies and inherited descendants keep the writer open. After EOF, Yolo never queries an ambient environment universe; it replays and generation-prunes only exact identities retained from writer-present scans, retaining on identity-query failure. There is no equivalent exclusion primitive for a managed PTY descriptor.
After writer release, PTY scans retain the full same-UID universe, publish exact live holders, and otherwise remain `Pending` rather than authorizing empty cleanup.

Linux production custody uses one boot-bound cgroup-v2 leaf per runtime. The operator provides a persistent
supervisor and writable `YOLO_CGROUP_DELEGATED_ROOT`; Host and Worker management processes stay in a child
cgroup so the delegated parent can enable controllers. Before bootstrap release, Yolo moves the exact shell
root into its leaf and binds start time, PTY `rdev`, capability, mount identity, and inode. Termination freezes
and double-observes bounded membership, captures exact pidfds, and signals only authenticated PTY holders. A
replacement Host can repeat that proof after Worker or SSH loss. Non-PTY members remain frozen and keep cleanup
pending; thawing them after master release could make a stale devpts path address a later session. Unauthenticated
holders block settlement. Missing, stale, malformed, replaced, inaccessible, or timed-out evidence
returns `destructive_session_unavailable`; numeric-PID, process-group, procfs-set, and `cgroup.kill` fallbacks are
forbidden. Reboot proves the old runtime gone and requires a new runtime epoch.

All controller, writer-lease, input, resize, signal, terminate, and shutdown mutations
serialize through one Worker admission gate, with the permit held through each PTY side
effect. Child exit revokes writer authority but retains the PTY master through EOF and
durable tail delivery. Neither wall-clock nor retry budget can manufacture `producers-gone`
or independently select `Lost`. Abandonment requires `MissingEof` plus proven producers-gone
and output quiescence, `NamespaceOwnershipLoss`, or explicit process-custody proof. Otherwise
state remains visibly `OutputBlocked` and recovery continues; pending journal data is never
discarded. Delayed shutdown revalidates its generation before retiring an adopted Worker.

## Persistence

Durable state layout, coordination roots, retained socket namespaces, catalog alias
shards, background maintenance, and creation-ledger recovery are specified in the
[persistence contract](persistence.md). Those resources remain independent from the
replaceable UI, relay, Host, and Broker processes described here.

## Durable Filesystem Boundary

Local durable paths are absolute and lexically normalized. Rust walks every
component with directory descriptors and `O_NOFOLLOW`; managed directories must
be owned by the effective user and are normalized to `0700` only through their
verified descriptors. JSON state is bounded, private, double-read, and installed
with parent-relative exclusive temporary creation, file and directory `fsync`,
atomic rename, and post-install inode and content verification. Catalog
quarantine and SQLite alias cleanup likewise compare pinned identities before
renaming or deleting entries.

SQLite still accepts a pathname at its VFS boundary. Yolo rejects pre-existing
symlinked or unsafe catalog parents and revalidates catalog and alias identities,
but a malicious process running as the same UID can continuously rename an
ancestor between those checks and SQLite's pathname resolution. Protecting
against that actor requires a VFS whose complete pathname resolution remains
rooted at a retained directory descriptor; the current boundary does not claim
that guarantee. macOS `/var` and `/tmp` are the only accepted system aliases and
are mapped to their root-owned `/private` targets before descriptor walking.

Managed Unix sockets use descriptor-verified private directories. Persistent
registration leases and inode-bound directory certificates separate ownership from
temporary transport state; see [socket namespace lifecycle](socket-namespace-lifecycle.md) for admission, recovery, and cleanup authority.

The worker assigns output sequence numbers, appends checksummed output, updates
the terminal model, and then emits ordered events. Cursor gaps receive a full
snapshot barrier instead of an invalid raw tail. Slow subscribers can be
disconnected and resynchronized; they cannot block PTY reads.

The workspace split tree is persisted separately under the user's Application
Support directory. It contains recursive split geometry, focus, pane IDs, and
structured `TerminalIdentity` bindings. It contains no writer generation or
PTY ownership state.

## Recovery States

- `Reattached`: original worker, PTY, and child are alive
- `Resumed`: a new PTY used a validated provider-native resume command
- `Recreated`: a generic shell was intentionally created again
- `Uncertain`: a side effect may have crossed an unacknowledged crash boundary

Host replacement can reattach. Machine reboot cannot: no userspace PTY manager
can keep a process alive across a reboot. Reboot recovery creates a new runtime
epoch and never treats a stale PID as live.

Creation recovery is separately fenced from runtime recovery. After an atomic
token reservation, create retries, finalize, capability cleanup, explicit
resume, and automatic reconciliation use the same per-terminal lock and then a
short ledger transaction. A Terminal with an active, unfinalized creation record
cannot be resumed into a different runtime identity. Slow work for one Terminal
does not block lifecycle operations for unrelated Terminals.

## Provider Boundary

Terminals always start shells. Adapters observe the foreground process tree,
validate fresh provider metadata and cwd, persist an `AgentBinding`, parse
provider-specific state, and build native resume input. They never replace the
TUI with a structured agent API.

Agent kinds are open wire identifiers. Unknown providers retain their raw identifier
and use the generic terminal presentation until a provider-specific adapter is available.

Codex resumes with `codex resume -C <cwd> <id>`. Claude Code resumes with
`claude --resume <id>`. Ambiguous or stale evidence fails closed and cannot bind
an unrelated history file.

## Release Gates

- every tracked handwritten source and document has fewer than 200 lines; generated locks are exempt
- Bazel alone defines production targets; Cargo is dependency metadata; policy literals stay centralized
- default tests cannot activate or focus a TUI window
- Rust lines, functions, regions, and branches each exceed 95 percent
- recovery tests kill the host, relay, worker, transport, and TUI independently
- host or client replacement preserves worker and child identities
- worker or reboot recovery reports a new runtime epoch and explicit state
