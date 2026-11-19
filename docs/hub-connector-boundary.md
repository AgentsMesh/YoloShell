# Connector Observation and Effect Boundaries

The Hub now implements local Connector enrollment and observation publication.
Connectors report Agent lifecycle, activity and working directory for one exact
local terminal runtime. Semantic effects and Connector-driven recovery remain
unsupported. A Connector is an observation source, not another Hub, a native Host
binding or the owner of a PTY.

## Components

- `connector_protocol`: bounded enrollment/publication documents, version 1,
  depending only on public identity/Agent/session models and serialization.
- `connector_store`: retained directories, exact member identities, conditional
  durable writes and recovery of interrupted initial registration.
- `connector_registry`: enrollment, source ownership, generations, publication
  watermarks, revocation and stale history.
- `connector_client`: authenticated local connection, pinned Hub, correlated
  replies, cancellation retirement and unknown-outcome classification.
- `connector_service`: separate same-user Unix ingress, scoped authentication,
  scheduling and disconnect/revocation handling.
- `local_backend`: trusted Host admission, native-first Agent/Session catalogue
  association and rejection of unsupported semantic effects.

`yolo-hub` starts the Connector listener alongside its public client and peer
services. `connector_client::socket_for_hub` resolves the companion socket from
the public Hub socket, in the retained short socket namespace. The ready-stdio
document and public Hub/peer protocol versions are unchanged.

## Local Trust and Roles

Both service and client verify the Unix peer UID. A newly greeted same-UID
connection is a local management connection: it can list, register, activate or
revoke enrollments. A successful activation changes that connection permanently
to a publisher, which can only repeat Hello or publish for its exact activation.
Publication cannot register, revoke, switch identities or read credentials.

The same OS user remains an administrator and can open a separate management
connection. The scoped publisher credential isolates protocol roles; it does not
create a security boundary against arbitrary processes running as that user.
Credentials are kept in private state, omitted from list results, and redacted
from diagnostics. They are separate from Worker writer tokens and peer keys.

Keep `AgentProjection::project` restricted to authenticated Host observations.
Connector ingress must not accept SessionView, AgentBinding, reported PID/start
time as authority, hook credentials, recovery plans or a complete caller-written
AgentSnapshot. Reuse retained private directories, singleton ownership, exact file
identity and conditional durable writes rather than an unrestricted JSON store.

## Enrollment and Publication

Initial enrollment belongs to the local origin administrator, not a relayed
publication method. An enrollment identifies its Connector, allowed provider
namespace and exact RuntimeTarget. The origin allocates Connector and Agent IDs,
execution/stream mapping and a revocable publication generation. A Connector
cannot choose another owner's ID. The registry permits at most 1,024 enrollments,
including revoked/superseded history, and one Enabled enrollment per exact terminal
runtime. Enrollment removal and history garbage collection are not implemented.

Activation requires a credential and caller-reported ConnectorInstanceId. This
ID represents the reported Agent incarnation, not the publisher process PID.
Reconnecting with the same instance preserves execution identity; a new instance
creates a new execution under the same logical Agent address. Every activation
replaces the publication generation and stream. A publisher must preserve the
instance ID across observer reconnects and change it when the reported Agent is
replaced; the Hub cannot prove that fact from Connector reports alone. Once that
instance reports Exited, it cannot return to Running; replacement needs a new
instance ID. An idle publisher channel closes after 120 seconds, so a connected
publisher must send periodic requests or authenticate again after disconnection.

Publication carries the Connector ID, generation, monotonic sequence, allocated
AgentTarget, exact terminal target, reported state and directory. Provider metadata
is fixed by registration scope. The origin derives receive time,
cursor, source=Connector, freshness and reachability. Connector health and reported
lifecycle do not establish native process custody or recovery authority. No
interaction or semantic capability can currently be published.

An identical duplicate sequence/payload is an idempotent observation. A changed
duplicate, rollback, gap, retired generation or overflow is rejected. There is no
resynchronization command: reactivation starts a new stream and sequence. Generation
checks fence late callbacks from previous connections; reconnect does not prove
Agent exit. Restarted Hubs restore stale
observations and require fresh authentication before accepting publication.

Enrollment revision, ownership, sequence and exact last publication are persisted
before acknowledgment. A failed or uncertain write retires the registry instance's
write authority until state is reopened and reconciled. Registration writes a
Preparing index record, an independent member, then a compact Ready index entry.
Reopen may recover an interrupted Preparing record; a missing Ready member fails
closed. Revocation uses revision comparison and an exact retry identity.

## Native Ownership

The current Session model has one primary Agent. A Connector cannot claim an
active native binding. If a native binding appears later, it wins; the Connector
observation becomes stale/superseded and pending controls lose their source fence.
Do not merge sources using provider session text or a directory name.

A changed runtime, retired Shell or native takeover permanently supersedes the
enrollment. Temporary Host unavailability retires its live generation and leaves
stale history. A partial single-Session read reconciles only that observed Session.
Storage failures preserve previously validated stale observations, while removing
unverified primary associations. Reported exit never stops the Shell or turns it
into a recovery candidate. Mac and TUI expose the Connector observation source,
including when the provider name is `codex` or `claude`.

Catalogue reads and Connector admissions share an observation gate across the
Host read and registry transition. An older delayed or failed Host response cannot
supersede a newer registration or invalidate a later activation. Lock ordering
is native projection, when present, then observation gate, then registry; no
Connector mutation takes the native projection lock afterward. Cancellation
releases these guards without committing an unobserved transition.

Use one ownership selection for both Session-to-Agent association and the Agent
catalogue. Registration-time inspection of an empty native binding alone cannot
exclude a later native hook, so final effect admission must recheck source ownership.

## Future Semantic Effects

These are requirements for a later effect adapter. Current Connector observations
advertise unsupported controls/recovery, and AgentControl cannot execute them.

The public AgentControl right is independent of publication permission. A control
must bind enrollment, live generation, Agent execution, observation/interaction,
caller rights and exact terminal writer authority where the terminal is affected.
Public target comparison alone is not writer-secret authentication.

A correct origin-owned effect lets the Connector prepare a bounded proposal,
then validates and executes a supported primitive under the Worker's actual
writer mutation gate. The Connector does not receive reusable writer credentials.

A Connector-owned effect requires a durable commit protocol at that effect owner
and an explicit authorization linearization point. The owner must deduplicate
committed operation IDs and serialize admission with generation retirement. If
takeover promises that no old effect can occur afterward, it cannot confirm that
promise until the old endpoint proves its queue retired. Losing that proof leaves
the outcome unknown and semantic control unavailable.

Do not hold a Worker mutex across an unbounded callback and release it on timeout
while the callback can still act. Cancellation, a closed socket, a deadline or a
signed token alone cannot prove that no external effect happened. Delivered means
delivery; Confirmed needs correlated evidence from the actual effect owner.

Origin C signs accepted Connector observations using the existing catalogue path.
A/B consume those resources through existing grants; they never gain publication
authority over C's Connector namespace. For A -> B -> C, B relays C's observations
without replacing C's Agent address. Client entry views follow directed exports
and origin authorization, independently of which endpoint opened the SSH link.
