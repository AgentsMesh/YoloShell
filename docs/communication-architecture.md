# Communication ownership

Yolo uses JSON-RPC 2.0 for public clients and Cap'n Proto for internal services.
Both adapters invoke the same validated domain operations. They do not implement
separate lifecycle, writer, agent or recovery policies.

| Boundary | Representation and owner |
| --- | --- |
| Public clients and plugins to Hub | JSON-RPC 2.0, with negotiated binary terminal blobs |
| Browser clients through the Web gateway | Origin-checked first-frame bearer authentication, then the same JSON-RPC/binary stream over WebSocket |
| Host, Worker and runtime proxies | Cap'n Proto Endpoint RPC and returned event capabilities |
| Hub federation | Cap'n Proto Peer RPC over authenticated SSH channels |
| Native relay control and status | Explicit Cap'n Proto schema through the Rust C ABI |
| Private catalog, readiness and supervisor channels | Bounded Cap'n Proto documents with u32 lengths |
| Internal one-shot CLI replies and private stdin | A bounded Cap'n Proto document; EOF delimits the process result |
| CLI carriage of public setup requests | Base64 Cap'n Proto via `--request-capnp`, never secret argv |
| Local resource handoff | Cap'n Proto identity/acknowledgement plus SCM_RIGHTS |

## Public clients

The TUI, trusted plugins and observation connectors use authenticated local
socket entry points. Trusted plugins use the same-UID local management socket;
browser transport uses the scoped public-client socket. Control calls are bounded
JSON-RPC documents. Terminal
attachments negotiate binary blobs for snapshots and output; data is never encoded
as JSON arrays. See [the public wire contract](public-jsonrpc.md).

The Web client uses the loopback-only Web gateway listener. An exact Origin
allowlist and explicit bearer login precede all Hub connections. Cookies and URL
credentials are not accepted. Each browser connection opens an independently
authenticated SSH channel to `public-client.sock`, using a pinned Hub key and
enrolled client key. Hub rights and revocation remain authoritative. WebSocket
binary messages carry slices of the unchanged public byte stream, including
JSON-RPC lines and terminal blobs. Monotonic expiry, disconnect and gateway
shutdown close the owned scoped connection without terminating a PTY.

The TUI client owns its persistent Unix connections and validates peer and Hub
identity on each. Cancellation retires the exact connection, preventing a
delayed reply from being consumed by a later call. Conversation watches do not
occupy the only input or catalog channel. There is no native AppKit client or
App-to-relay ABI; see [ADR 0023](adr/0023-trusted-only-plugins.md).

## Internal RPC

`transport/capnp/schema` defines structural values and generated capability
interfaces. `value` converts owned domain values; `codec` enforces traversal,
depth, byte and node budgets, preserving native binary Data and rejecting duplicate
map keys before domain deserialization. It embeds no JSON or MessagePack payload.
The structural depth limit is independent of pointer depth in the generated
schema. Read and write budgets account for structs, map entries, list tags and
padded data, with envelope headroom inside the 16 MiB message ceiling.

Private credential stdin uses borrowed schema readers and bounded, scrubbing
owners. The sensitive codec erases partially decoded values on failure, duplicate
map rejection, builder segments and released native buffers. Errors do not include
the malformed credential document.

The RPC executor owns the non-Send capability objects in one bounded LocalSet.
Connections and requests are tasks rather than operating-system threads per Pane.
Domain services run on their owning Tokio runtime. Connection-scoped cancellation
aborts pending domain work; a blocked domain mutex cannot stop unrelated RPC IO.

Endpoint calls serve local runtime operations. Peer discover/execute methods
serve Hub federation. Terminal subscriptions return a capability with next/cancel
operations. Releasing a subscription releases observation resources, not its PTY.
Idle terminal observation has no arbitrary output deadline; federation maintains
its existing heartbeat and cursor validation.

Hub-to-Hub traffic retains SSH authentication, pinned identities, routes,
delegation and origin checks. Cap'n Proto's connection-local references do not
replace Hub, Terminal, execution or runtime identities. Signed peer documents and
delegation claims use canonical Cap'n Proto root bytes with explicit domains,
independent of framing, segment allocation or packed encoding.

Each authenticated channel uses the Rust library's two-party RPC transport.
Multi-hop Hub routing remains an explicit domain operation with delegation
checks at each hop. This implementation does not claim third-party capability
handoff or a Level 3 distributed object network across Hubs.

## Delivery and process lifetime

Business operation IDs and durable receipts remain independent of RPC question
IDs. The system never interprets an exception, cancellation or reconnect as proof
that a mutation did not execute. It does not automatically replay uncertain input.
Each admitted Call/Execute has one absolute operation deadline. Post-dispatch
timeout, disconnect and missing executor results preserve `OutcomeUnknown` through
private and public adapters. Only explicitly read-only startup probes retry a
lost HELLO or HOST_STATUS response. Retiring a poisoned connection does not retry
the operation that poisoned it.

The TUI orders input on a dedicated bounded channel and renews writer leases on
an independent channel. Confirmed renewal may extend admission of queued input;
it does not extend the deadline of an already dispatched operation. An uncertain
prefix invalidates its remaining input stream. The interruption stays visible
until an explicit user acquisition starts a new stream; navigation, reconnects
and automatic grants cannot silently resume the discarded suffix.

Runtime responses may carry a separate observation receipt. The caller validates
the domain response before confirming it. An RPC proxy passes that receipt through
until its downstream caller observes the response, then awaits confirmation through
every upstream hop. A response requiring confirmation becomes certain only after
that confirmation succeeds. Lost, expired or timed-out confirmations produce an
unknown outcome and never trigger automatic replay. Receipts expire after a bounded
interval; expiry drops retained guards without invoking success callbacks.

This preserves response-drain and idle-shutdown ownership: an unobserved response
cannot commit an idle shutdown, and a peer retaining an unconfirmed capability
cannot pin a session lock or mutation permit indefinitely.

Shutdown drains confirmations already in progress and the Cap'n Proto write queue
before disconnecting, within a fixed shutdown budget. Cancelling a service future
releases its owning domain service immediately; an IO executor cannot retain Host
ownership merely because it has not yet processed connection teardown.

Bootstrap, startup guards and custody use small bounded Cap'n Proto handshakes.
Local FD handoff additionally uses SCM_RIGHTS with exact identity/count checking.
RPC data describes and confirms the handoff; the operating system transfers the
actual resource reference. Cross-machine callers receive remote interfaces, never
a transplanted local descriptor number.

Provider-defined hook stdin, native conversation records, configuration files and
on-disk journal storage are separate formats. Their presence is not a retired IPC
reader. Current wire endpoints do not auto-detect or accept historical formats.

Conversation observation also distinguishes unavailable evidence from a confirmed
binding change. A temporary Worker read failure does not authorize content from
another run, and does not erase a client's already verified history or descriptor.

Terminal snapshot contents and their journal barrier are one published value.
Batch append/reconciliation and the complete terminal projection share the journal
lock, following journal-to-terminal lock order. Output events and periodic
checkpoints follow the complete projection. A snapshot cannot advertise a batch's
tail sequence while its screen contains only a prefix. Healthy, definitely
uncommitted append failures stay with the existing pending-output retry owner;
only uncertain journal writes require reconciliation.

## Build and validation

`build_system/capnp` pins the compiler and Rust generator and declares all schema
outputs to Bazel. Generated source stays in build outputs; handwritten component
files remain below the repository size limit. Tests live in unit/integration
directories and exercise real Cap'n Proto peers, JSON-RPC sockets and private PTYs.
Architecture checks own the new Rust, schema, C header and module-map sources,
validate exact schema-generation macros and static-library ownership, and enforce
the public/internal dependency boundary. Retired byte codecs are rejected in
private IPC owners; persisted formats have explicit, narrow exceptions.

Protocol changes are verified with isolated state before any development-install
cutover. Live Ubuntu PTYs and their custody domain are never used as destructive
fixtures or silently upgraded across incompatible wire contracts.
