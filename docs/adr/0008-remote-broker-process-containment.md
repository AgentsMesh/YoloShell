# ADR 0008: Remote Broker Process Containment

## Status

Accepted for the macOS 0.1 release.

## Context

The local remote broker pools one SSH transport and serves independent control,
terminal, command, file, and deployment channels. Its process is launched by an
exact supervisor and recorded before admission opens. On recovery, the
supervisor can retain the exact broker root and contain descendants that remain
observable in that live lineage, including a descendant that later starts a new
session.

No portable process API can safely rediscover an arbitrary leaf that double
forked, was reparented before capture, closed inherited evidence, and then
escaped its session. A numeric PID or a later process inventory is observation,
not destructive authority. Treating either as custody would risk signaling an
unrelated process after PID reuse.

## Decision

Production code below `core/src/remote_broker` is process-creation free. It may
create async tasks and threads, but it cannot use standard-library or Tokio
process commands, PTY child spawners, fork/clone/exec families, session or
process-group escape calls, shell process APIs, process wrappers, raw process
syscalls, aliased imports, or dynamic symbol lookup.

Only `remote_cli/start_detached.rs` creates the detached supervisor, and only
`remote_cli/supervisor_process.rs` creates its waitable broker child. Their
prepare, durable-running, commit, and admission protocol binds the child to an
exact supervision record. The child also inherits the record's per-launch
authority marker. Recovery obtains no general process signal API: runtime code
must read a descriptor-anchored record, validate its complete shape and boot,
and verify the live root's inherited marker before it can contain that family.
Forged record bytes therefore cannot mint destructive authority.

Process-tree capture and its TERM/KILL operations remain crate-private. The
only cross-crate operation is the bounded, domain-specific containment method
on an anchored broker supervision store; it accepts no process identity.

The architecture checker enforces this boundary from tokenized Rust source, so
comments and strings cannot satisfy or hide it. Test-only hooks and sources may
create fixture children; they cannot grant production authority.

## Consequences

A broker crash, supervisor crash, or interrupted adoption cannot leave a broker
created helper outside recorded custody. A future local helper is rejected at
review and build time until its identity, durable record, prepare/commit state,
admission fence, termination settlement, and exact reaper ownership are added
to the supervision protocol.

This closes the avoidable daemonization route inside the broker. It does not
claim that arbitrary untrusted code executed elsewhere can be reconstructed
after an already-unobserved reparenting escape. Such uncertainty continues to
fail closed without numeric-PID signaling.
