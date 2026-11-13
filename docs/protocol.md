# Runtime protocol

Public clients use [JSON-RPC](public-jsonrpc.md). Internal Host/Worker and Hub
federation calls use Cap'n Proto RPC over private Unix sockets or authenticated
SSH channels. Runtime domain envelopes have version 5; transport call identifiers
remain independent of durable business operation identities. See
[communication architecture](communication-architecture.md).

Bootstrap and custody handshakes use bounded Cap'n Proto documents with a
four-byte big-endian length prefix. Descriptor attachments remain local Unix
SCM_RIGHTS operations. No retired MessagePack or JSON control-frame fallback exists.

PTY custody uses protocol 5 and durable schema 4. Offers and armed/retired records
require an explicit `active_running`, `active_terminal_read_only` or
`released_terminal` profile. Missing/null/unknown profiles and retired versions
are rejected; missing data never grants active-running authority.

Every request has a UUID echoed by its response; events identify their session and runtime epoch.
Internal envelopes require an explicit payload (Null for no-argument operations) and reject unknown envelope/error fields.

Required request families:

- hello and capability negotiation
- list, create, get, attach, detach, archive, terminate, and resume session
- acquire, renew, release, and take over writer lease
- terminal input, resize, signal, snapshot, and event subscriptions
- host status and recovery reconciliation

Worker termination accepts only `terminate_runtime_exact`, carrying the retained
runtime epoch. Host recovery uses only `resume_session_exact`. The retired
`resume_session`, `detach_session` and `ack_output` methods are rejected;
detaching releases the event capability and connection. Host writer support is
advertised by the current `writer_lease_v2` capability without its retired alias.

## Desktop Relay Status Capability

Each terminal launch receives one end of a connected anonymous Unix stream
`socketpair`. Both descriptors are first promoted above standard input, output,
and error, then marked close-on-exec. The App keeps the nonblocking reader. Only
at the direct-child spawn boundary does it clear close-on-exec on the writer and
pass its number through `--status-fd`; the parent closes its writer immediately
after the child is created.

The `yolo internal attach` role rejects standard or negative descriptors, verifies that the
inherited endpoint is a connected Unix stream, restores close-on-exec, and uses
it for structured status output. Unexpected App-to-attach input or loss of the
App endpoint terminates the attach relay. No filesystem listener, reconnect,
peer-PID inference, or authorization handshake participates in admission: the
inherited descriptor itself is the direct-child capability. Structured status
events are presentation signals only and never grant Host writer authority.

## Shell Creation Transaction

The durable creation-token transaction is specified in
[shell creation transaction](protocol-creation-transaction.md).

## SSH Channel Mapping

A remote endpoint has one authenticated SSH transport and multiple independent
channels. Each control or terminal-attachment channel executes the configured
absolute `yolo internal host bridge --home <path>` command and then carries
Cap'n Proto RPC. Terminal attachments therefore retain the
same snapshot barrier, output cursor, writer lease, and detach semantics as
local Unix-socket attachments.

Bounded diagnostic commands use separate exec channels. File canonicalization
and reads use SFTP channels and do not consume terminal flow-control windows.
Closing the transport or any of these channels is a detach event, never a
session-termination request.

Runtime onboarding uses additional command and SFTP channels on the same
authenticated transport. Architecture and staged-version probes run as strictly
allowlisted absolute executables. Binary transfer, remote readback verification,
permission changes, content-addressed release rename, and atomic stable-link
selection occur before `bridge --stdio` starts. Deployment messages never carry
a worker termination request, so a client disconnect or failed upgrade cannot
end an existing PTY.

Linux supports remote PTY survival across app, Host, and SSH loss only when the
operator has placed `yolo internal host` under a persistent supervisor outside SSH login
cleanup. With that custody in place, Yolo provides reattachment, command and
file channels, agent observation, and fenced cleanup after Worker loss. A plain
SSH-launched process is reported as lacking verified disconnect custody; SSH
multiplexing and `setsid` alone are not treated as a survival guarantee. The
runtime advertises `safe_process_tree_termination` only when it can open the
boot-bound cgroup-v2 custody record or prove that reboot ended the old runtime.
Same-boot Host fallback first proves the exact Worker identity gone, then freezes
stable bounded membership and signals only exact pidfd identities that hold the
recorded PTY device and capability.

Absent delegation, malformed or replaced records, inaccessible controls, freeze
timeout, indeterminate identity, or an unauthenticated PTY holder returns
`destructive_session_unavailable` before unsafe signaling. Numeric PIDs, process
groups, procfs-discovered sets, and `cgroup.kill` cannot authorize a fallback.
Capability-only non-PTY members are retained but remain frozen. Thawing them after
the last PTY master closes could let a stale `/dev/pts/N` path resolve to a later
session, so Yolo reports cleanup as deferred until the frozen leaf is stably empty.
A durable termination tombstone is an intent, not settlement proof; success follows
custody settlement.

After reconnect, the client validates the remote protocol version, runtime
home, boot identity, controller generation, and required capabilities. A
checkpoint with an unchanged runtime epoch reattaches after its acknowledged
sequence. A changed runtime epoch is reported explicitly and must enter the
agent-adapter recovery path rather than being mistaken for the original PTY.

The runtime emits the event families used by the desktop attachment path:

- terminal snapshot and ordered output chunks
- output gap and snapshot resynchronization
- process exit and runtime epoch changes
- writer lease changes and conflicts
- host connection and protocol errors

Output is addressed by `(runtime_epoch, sequence)`. Attach includes the last
acknowledged cursor. The host either replays from that cursor or sends a
snapshot barrier followed by ordered output. Input carries both a writer lease
generation and an input ID. A transport loss after submission makes delivery
unknown, so the relay reconnects but never automatically replays terminal
input. A live worker binds recent input IDs to a payload digest: an exact
duplicate receives the cached result, while reuse with different bytes is
rejected.

Every successful attach result carries a mandatory, non-nil `runtime_epoch`,
including an empty replay with no snapshot or chunks. The Host compares that
epoch with the currently published runtime immediately before forwarding the
result. Local and remote attachment connections then pin the epoch: event
envelopes and terminal output chunks must match it, and any mismatch poisons the
connection before output or writer-lease side effects. Reattachment may continue
only at the original epoch; a replacement PTY enters explicit recovery.

The attachment barrier also fences the worker's live subscription: chunks with
sequence numbers already represented by the initial replay or snapshot are
discarded. This prevents output produced between subscription and response
construction from appearing twice.

Runtime status has an explicit exit state machine. `Starting` precedes bootstrap
release, and `Running` means the child can still accept mutations. Once the child
has been waited, the worker immediately reports `Finalizing`; it keeps the PTY
master open until a reader has confirmed EOF and all staged output is durable.
If that work exceeds its bounded deadline, status becomes `OutputBlocked` while
background recovery continues. `Exited` is published only after the final output
sequence and exit record are durable. `Unreachable` is a Host observation, and
`Archived` is a durable session lifecycle state, not a Worker exit shortcut.

Journal chunks remain durable even when the Worker parser rejects one of them.
On parser recovery the Worker records the rejected chunk's sequence as the
screen baseline, replaces the parser at the same geometry, and publishes an
output gap with resynchronization required. Live subscribers must reattach.
An attachment whose cursor is at or before the baseline receives a fresh
snapshot instead of replay; later cursors may replay only chunks after the
baseline. The rejected chunk is never replayed into a parser state that
predates the reset.

Desktop archive and discard requests carry the catalog row's
`expected_updated_at_ms` and `expected_status`. The Host revalidates catalog
identity and captures the exact runtime while holding the per-session lifecycle
lock. Responsive termination uses the separate `terminate_runtime_exact` Worker
method; the Worker checks runtime epoch and expected status under its mutation
permit before writing a tombstone. The Host reacquires the lifecycle lock after
the reply and rejects success if a successor runtime appeared. Archive repeats
its active-runtime check before publishing the archived manifest. A conditional
discard never terminates a responsive active Worker: it accepts an already
exited runtime or proves that the recorded Worker and child identities are gone.
A running, starting, finalizing, output-blocked, changed, or indeterminate
selection fails with a conflict. The unconditional control command remains an
explicit administrative force operation and is not used by the desktop UI.
Resume treats the durable manifest as authoritative for archive: even if a
catalog publication failed, an archived session cannot be selected, resumed, or
overwritten from a stale catalog row.

Request IDs correlate mutations with their responses; they do not by themselves
make a mutation durable exactly-once. Shell creation additionally uses the
stable token, capability, active ledger, and completion receipts described
above. A live worker keeps a bounded recent-ID deduplication window; this is a
defense against adjacent duplicate requests, not an exactly-once claim across
arbitrary delay or worker loss. Persisted session/runtime records are reconciled
after a host restart. Resume is fenced
against active creation, while ordinary signal, archive, and terminate remain
request-correlated operations rather than claiming exactly-once behavior across
a crash at every instruction boundary.
