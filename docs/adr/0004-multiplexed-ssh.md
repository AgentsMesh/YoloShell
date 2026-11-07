# ADR 0004: Multiplexed SSH with a remote PTY owner

## Status

Accepted.

## Context

Yolo needs one authenticated remote connection to carry the runtime control
stream, terminal attachments, short commands, and file access. A terminal agent
must keep running when the client network or desktop process disappears.

SSH Connection Protocol channels are multiplexed over one SSH transport, but a
channel belongs to that transport and is closed with it. The protocol does not
define channel migration to a new TCP connection. OpenSSH ControlMaster and
ControlPersist reuse a living master connection; they do not restore channels
after that connection is lost.

Sources:

- [RFC 4254, SSH Connection Protocol](https://www.rfc-editor.org/rfc/rfc4254)
- [OpenSSH ssh_config: ControlMaster and ControlPersist](https://man.openbsd.org/ssh_config#ControlMaster)
- [OpenSSH protocol specifications](https://www.openssh.com/specs.html)

## Decision

The Rust runtime owns one pooled SSH client per remote host. The pool opens separate
channels on that connection for:

- a long-lived Yolo host control stream;
- each terminal output attachment;
- bounded command execution used for discovery and diagnostics;
- SFTP-backed stat, directory listing, and file reads.

Production uses the native Rust `russh` transport. One authenticated session handle
opens independent exec and SFTP channels; there is no subprocess transport,
environment-variable fallback, or compatibility alias.

Host-key verification happens during key exchange and fails closed against explicit
known-hosts data. Plain, wildcard, negated, hashed, changed, and revoked entries are
handled. Authentication uses the SSH agent or configured identity files. Channel and
reconnect state are isolated from writer-lease state.

The remote Rust worker, not sshd and not the app, owns the PTY. A control
channel starts or reconnects to the remote host. The host starts durable workers
and journals their identities. If the SSH transport disappears, the host stream
may end while workers and their PTYs continue. A replacement connection starts a
new host generation, adopts live workers, and resumes output from acknowledged
sequence numbers.

## Consequences

- Native SSH multiplexing is enough for transport and file access.
- The client bootstraps and upgrades the remote Yolo runtime before opening its
  bridge channel.
- No terminal-multiplexer dependency is required on the remote host.
- Reconnect tests must independently break the SSH transport, control channel,
  and attachment channel while asserting that the worker PID and runtime epoch
  stay unchanged.
- Filesystem commands use bounded channels and never share terminal flow-control
  accounting.

The native pooled connection, runtime bridge channels, bounded command channels,
canonicalized bounded SFTP reads, and runtime deployment orchestration are
implemented in Rust. Deployment validates the artifact manifest SHA-256 and
binary architecture locally, probes the remote architecture, creates only the
exact scoped install directory when needed, uploads through a private temporary
file, verifies remote bytes and permissions, executes the staged binary's version
probe, and atomically selects a content-addressed release. A failed upgrade never
replaces the stable executable. Every channel operation, including channel open,
has its own deadline instead of relying on connection inactivity.

Protocol tests run the real russh client/server and SFTP protocols in memory. A
separate macOS process test runs `russh` against an isolated non-root OpenSSH server,
keeps a terminal attachment open while command and SFTP channels execute, cuts the
underlying TCP transport, and requires the broker to reconnect without changing the
remote host, worker, child, or runtime epoch. The detached host has a new session and
null standard streams; an SSH channel EOF therefore ends only its bridge process.

Some managed servers deliberately kill every process in an SSH login cgroup after
logout. Neither SSH multiplexing nor POSIX session detachment can override that host
policy. Such an endpoint needs an operator-provided persistent user supervisor; Yolo
does not silently depend on `systemd` being installed. AppKit remote host configuration
and artifact selection remain outside this ADR's Rust boundary.
