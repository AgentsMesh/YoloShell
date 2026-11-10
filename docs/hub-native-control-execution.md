# Native Agent Control Execution

Status: Worker/Host/private-client/Hub execution is connected, and the first
positive A → B → C SIGINT scenario passes. Native Interrupt is advertised only
with coherent source authority and initialized, verifiable operation history.
This design preserves the terminal/TUI product boundary. It does not assume a
provider API, inject an inferred approval key or interpret terminal output as an
acknowledgement.

## First Supported Effect

The first native adapter delivers SIGINT to the exact native Agent process
selected by authenticated observations. A successful OS delivery can produce
Delivered, not Confirmed cancellation of a turn. Provider behavior may terminate
the TUI, ignore the signal or cancel work. An unrelated foreground group member
receiving a signal does not establish delivery to the selected Agent.

The existing native process layer retains Linux pidfds and macOS audit identities
including pidversion/unique process identity. The exact-member primitive must use
that retained authority. PID-only lookup followed by kill is not permitted.
Captured group membership and current group checks remain admission conditions;
the OS identity-signalling primitive does not atomically assert a foreground PGID.

Approval, choice and text response remain Unsupported until an adapter retains a
real authenticated pending interaction, its identity, permitted response values
and a request-correlated acknowledgement. Current Hook ingestion acknowledgements
and subsequent generic idle/Stop events cannot serve as those acknowledgements.

## Private Observation Witness

Hub AgentProjection allocates public Agent/execution IDs and observation cursors.
Those IDs do not occur in Worker bindings, so the Worker cannot recheck a public
cursor by comparing it to a PID or timestamp. The private observation path must
return a source witness together with the binding that produced that observation.
Projection must retain the witness with that exact public cursor and advance the
cursor when the authoritative witness changes, even if visible state is identical.

The witness binds terminal runtime and worker incarnation, native provider/session
and process identity, plus the provider's accepted source revision. Claude needs
its native scope/invocation, generation authority revision and event sequence.
Codex needs route revision, invocation, generation, authority revision, sequence
and snapshot lineage. Connector observation credentials provide no native witness.

Codex binding publication is asynchronous. Its lifecycle reducer changes under
the source lifecycle mutex before the binding projection acquires the Worker
Agent generation gate. Reading a binding file while holding that gate is therefore
insufficient: final effect admission must enter the current source authority.

## Admission and Commit

The public origin validates exact AgentTarget, cursor, availability and matching
terminal target. Federation requires AgentControl and TerminalWrite together on
one delegated route. The private request carries a selected source witness and
actual writer grant; neither public target equality nor a route is a substitute
for authenticating the writer secret.

The Worker obtains Agent generation, writer mutation and durable session guards
in a consistent order, then enters the provider's current source authority. Use
nonblocking rejection where taking another authority lock could create an inverse
dependency. Host lifecycle guards must not deadlock against a Host waiting for
the Worker. Final validation includes lease expiry, runtime/launch identity,
namespace, source revision and retained effect target. No external callback or
await occurs between the final validation and the single synchronous effect.

## Durable Execution Record

`control_journal` records the operation's exact request ID and canonical digest
under its terminal runtime scope. Reserve writes Unknown durably before returning
an opaque new reservation. Only that new reservation permits the caller to execute
once. An existing Unknown, Delivered or NotDelivered record is a lookup result,
never another execution permit. A different digest for an existing ID is rejected.

The runtime initializer creates the journal before advertising the capability
and durably stores its anchor with runtime authority. Reopen requires that exact
anchor and existing history. Missing history, replaced headers and malformed
records cannot fall back to creation. A live owner detects record replacement.
The immutable anchor does not detect restoration of an older valid mutable record
while closed; there is no external monotonic rollback proof. The journal never
evicts old operation IDs to make room for retries.
Capacity exhaustion rejects further operations until a deliberate new runtime.

The journal verifies retained storage identity and record consistency. It does not
authenticate the caller, source or process, or prove cross-reopen monotonicity.
The Worker must retain its admission guards while reserving and executing. A
syscall success followed by a failed receipt write leaves Unknown. Cancellation
after intent commit, Worker loss, reply loss or a lost source proof must not replay
the effect. Confirmed requires additional correlated provider evidence.
