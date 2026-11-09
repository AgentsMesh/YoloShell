# Independent Client Channels

The shared `hub/client/channels` component owns separate public protocol clients
for interactive control, lease maintenance, catalogue refresh, bounded lifecycle operations and
creation-result queries. Native TUI terminal observers
remain independent per-session streams. These clients retain the existing
same-UID Unix transport and pinned Hub identity checks; none owns a Host, Worker,
PTY or provider process.

## Control Admission

Each successful control handshake creates a random local connection generation.
The UI stamps a queued operation with the generation it has observed. Missing or
old generations are rejected before command I/O, including read-only commands.
Reconnecting does not replay commands from the failed connection. Typed uncertain
delivery errors remain uncertain; a later successful handshake does not prove
that an earlier operation succeeded.

This generation is not a RuntimeEpoch or writer lease generation. Origin runtime,
execution and lease checks remain mandatory. Control disconnection removes local
write readiness; catalogue success cannot restore it. A new control generation
does not restore old UI writer grants or pending renewals.
The serial input connection owner sends a read-only Hello on that same socket
after 30 seconds without a request, below the server's 120-second idle deadline.
It also keeps that socket alive while waiting for the UI to consume a completed
result; no subsequent mutation bypasses that acknowledgment. A failed probe
retires its exact generation; a stale failure cannot disconnect a newer one.
Catalogue and lease-maintenance owners retire their cached connections after
30 seconds of inactivity and connect before their next request. A healthy probe
on a separate socket is never evidence that an input socket remains usable.

## Lifecycle operations

Create, Finalize, Resume/Recreate, Terminate/Archive and creation cleanup use
independent operation connections, with at most four owned at once. Capacity
exhaustion is explicitly not-sent, without opening a socket or retrying a command.
Input and release stay on the ordered control connection. Renewal uses a separate
persistent maintenance connection with an independent bounded inbox. The input
forwarder has its own bounded owner, so an input transport waiting for an ACK
cannot prevent the router from forwarding a lease heartbeat. Every renewal checks
the control generation before opening and before dispatch, and never replays a
failed mutation.

Queued user input retains its exact request ID and original deadline for receipt
correlation. Its admission uses the current confirmed lease lifetime for the same
writer scope. A timely renewal can extend that admission lifetime; a rejected,
retired or already expired scope cannot be revived. Each dispatched RPC captures
its own finite deadline once, so renewal cannot extend an in-flight attempt.
Unknown prefix delivery cancels all dependent suffixes before dispatch.

The shared command budget uses the existing Host startup, resume and lifecycle
contracts plus their response margin. Ordinary RPC deadlines remain unchanged.
Server execution and federation forwarding use the same command classification;
each forward still shares one absolute deadline across open/send/receive. These
are bounded operation floors, not a propagated global route deadline.

An operation checks generation and local request validity before opening and
again before dispatch. Generation loss during its handshake closes the stream
without sending the mutation. After dispatch, its exact result remains associated
with the original request, including any uncertainty. An operation failure cannot
disconnect a healthy writer connection or publish another Ready generation.
Completed results wait in the bounded owner while fast control is offline, then
are delivered after its next Ready; completed operation streams are already closed.

The TUI retains creation intent across fast reconnect until that independently
owned request settles. A known Created result matches the complete original
request, origin and admission generation. It can bind only its original unchanged
pane and Finalize using the current connection; it never restores a writer.
Uncertain failure retires the pane intent and preserves the original request for
read-only recovery. Exact live evidence that no resource was created permits
removal: local non-dispatch, or an authenticated and correlated InvalidDirectory
rejection from the origin's pre-admission validation. A generic remote error,
OutcomeUnknownResponse(InvalidDirectory), Missing query or restarted intent does
not supply that evidence and never permits replay. Terminal
lifecycle replies trigger a new authoritative catalogue rather than overwriting
newer observations with an unversioned, delayed summary.

The TUI separately settles a confirmed creation's Finalize. Its private journal
retains the complete original request and result before pane binding or dispatch.
Only this exact Finalize may retry: the origin's durable receipt makes it
idempotent, including a lost acknowledgement. Create, Input, Interrupt and Resume
retain their no-automatic-replay contracts. Each Finalize attempt has its own
local request scope; an older failure cannot retire a newer attempt. Restored
settlements do not restore panes, writers or control generations. See
[creation settlement](tui-creation-settlement.md) for persistence and acceptance.

## Creation-result queries

QueryCreation owns four task/socket slots independently of lifecycle work and
the fast control stream. It uses the ordinary five-second RPC budget and any
earlier local validity deadline. Requests stay outside their tasks so a panic or
cancellation still reports the exact failed request. Query failures do not clear
a healthy control connection or change its generation. Completed queries can be
published while control is disconnected or reconnecting.

The TUI persists the original request before its first Create. Restart restores
only query/finalization obligations; it never replays Create or restores writers.
See [creation recovery](tui-creation-recovery.md) for result classification,
durable settlement and the real crash scenario.

All actor result publication still uses bounded backpressure. A consumer that
stops draining the result queue can stall this actor; separate query sockets do
not establish progress against that consumer. Continuous queued demand can also
starve the existing biased reconnect branches. Full-result-queue shutdown and
disconnected-command-flood acceptance remain explicit follow-up work. Results
must not be discarded to bypass either condition.

## Catalogue Fencing

Each refresh has its own request identity. Replacement requests cancel obsolete
reads and retire partially consumed connections. Completed usable connections may
be reused. Failure preserves the previous observation data as stale without
revoking an independently healthy control connection.

Only the latest requested result can update the UI. Create, finalize, lifecycle,
Get and resize completions invalidate older refreshes, including uncertain
completions that may already have changed the origin. An earlier catalogue cannot
overwrite the newly observed runtime or retire a newer pending refresh. Queue
pressure does not undo this invalidation.

The catalogue and control channels do not imply a cross-origin transaction.
Agent-to-terminal association still checks exact public bindings and runtime IDs.

## Owned Shutdown

Catalogue and control futures live inside `run`; lifecycle tasks live in its
bounded JoinSet. Closing catalogue demand cancels in-flight catalogue work.
Closing control demand drains queued requests, including releases, then cancels
remaining lifecycle work. Cancelling the owner drops all owned connections.
The TUI's bounded shutdown grace and origin lease deadlines remain the fallback
when a control operation or release cannot finish.
