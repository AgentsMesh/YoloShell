# Public Hub JSON-RPC contract

The local Hub socket and each admitted remote SSH channel carry JSON-RPC 2.0.
The TUI and trusted clients use the same method contract. Hub
federation and Host/Worker custody are separate internal protocols.

Each frame is one UTF-8 JSON object followed by LF. Literal line breaks inside
strings must be JSON escaped. The maximum JSON payload is 16 MiB, excluding LF.
High-volume terminal bytes use the separately bounded attachment frames below.
The reader rejects duplicate object keys at every depth, trailing documents,
invalid UTF-8, oversized frames and truncated frames. JSON nesting is bounded by
the parser. There is no old binary-envelope fallback.

## Requests and routing

```json
{"jsonrpc":"2.0","id":"client-call-1","method":"hello","params":{"destination":"00000000-0000-0000-0000-000000000001"}}
```

The first call on each channel is `hello` addressed to the connected entry Hub.
`result.hello.hub_id` must match the pinned Hub. Its `version` is the application
capability schema version; the JSON-RPC version is always the string `2.0`.
`result.hello.methods` advertises available operations.

Methods use the names in `hub/rpc/protocol::Command`, including `list`, `create`,
`attach`, `lease`, `input`, `list_agents`, `list_hubs`, `list_layouts`,
`update_layout`, `resolve_conversation`, `read_conversation` and
`watch_conversation`. `params` contains `destination` plus that command's fields.
There is no generic `call` method and no nested private request envelope.

The protocol crate's operation table is the single source for method names,
advertised method sets, public rights and mutation semantics. `Hello`, the
machine-readable descriptor, peer rights admission and read-reply admission all
derive from that table; command payload validation and reply correlation remain
per-operation functions because they depend on typed fields.

Routing remains explicit: later calls may target another reachable Hub. The
entry Hub applies admission scope and origin validation before dispatching.
Terminal, agent and layout addresses retain their origin Hub identity.

Request IDs are nonempty strings of at most 128 UTF-8 bytes or integral JSON
numbers. UUID strings are recommended. They are transport correlation only.
Durable creation tokens, operation `request_id`, lease capabilities, runtime
epochs, input IDs and conversation cursors remain in typed business parameters.
Clients preserve these identities when querying an uncertain operation. They
never automatically replay a mutation after a lost or invalid response.

Each channel processes calls sequentially. Use separate persistent, bounded
channels for latency-sensitive control, catalogs, conversation watches and
terminal attachments. A blocked watch must not block input or lease renewal.
Ordered input and lease renewal also need independent channels: an input backlog
must not consume its own lease's renewal window. Renewal preserves writer
identity, but cannot prove delivery of uncertain earlier input or authorize its
automatic replay.
Batch requests are rejected. Client notifications are ignored, produce no
response and execute no operation; receipt-bearing calls require an ID.

## Results and failures

```json
{"jsonrpc":"2.0","id":"client-call-2","result":{"duplicate":false}}
```

The result contains the operation's named body fields, without a second
`result` discriminator. The request method determines its result schema.
Exactly one of `result` and `error` is present. Clients validate correlation,
origin-bearing resources, operation-specific shape and requested target.

Protocol errors use `-32700` (parse), `-32600` (invalid request), `-32601`
(unknown method), `-32602` (invalid parameters) and `-32603` (internal failure).
An unidentified request uses a null response ID. Protocol errors disclose no
decoded credentials or malformed payload details.

Domain failures preserve structured information:

| Code | Error data |
| --- | --- |
| `-32000` | `{"kind":"hub","code":<PublicError>}` |
| `-32001` | `{"kind":"conversation","error":<ConversationError>}` |
| `-32002` | `{"kind":"connector","code":<ConnectorError>}` |

Errors contain a human-readable `message`; clients branch on the code and typed
data. A confirmed `outcome_unknown` remains uncertain even though the transport
successfully delivered that error. Cancellation after sending a mutation, a
truncated response or a mismatched response also leaves the outcome uncertain.
Cancellation retires the channel so a subsequent call cannot consume its reply.

## Terminal streams

`attach` returns `result.stream = "yolo.terminal.binary.v1"` and
`result.attachment`, including its runtime target, optional snapshot, optional
history, replay chunks and next sequence. An attach request may set `seed` to
`"history"` to ask for one raw stream from the first retained output sequence;
the default `"snapshot"` keeps the current screen and input modes only. Snapshot,
history and chunk objects contain `byte_length` in place of `data`. The result is
followed immediately by one binary frame per snapshot, replay chunk or history
blob, in metadata order. A present snapshot or history always has one binary
frame, including a zero-length history blob.

Each binary frame is tag byte `0x01`, a four-byte unsigned big-endian length,
then exactly that many raw bytes. The frame length must equal `byte_length`.
Snapshots and history streams are bounded to 8 MiB; output chunks to 1 MiB; an
initial replay has at most 256 chunks totaling at most 8 MiB. A history result
has `barrier_sequence`, the current runtime `size`, and exactly one blob, and is mutually exclusive
with a snapshot and replay chunks. If the retained journal no longer covers the
session, the Hub answers with a snapshot and marks it `resynchronized`; clients
must handle both. Unknown tags, wrong lengths, truncated blobs and metadata where
a blob is expected retire the channel.

After the initial blobs, the channel carries server notifications:

```json
{"jsonrpc":"2.0","method":"terminal_event","params":{"event":"heartbeat","cursor":{"target":{},"after_sequence":42}}}
```

The example omits the target's fields for brevity. The actual target always
contains the terminal address and runtime epoch. Output, heartbeat, exit,
resynchronization and unreachable events use the validated `TerminalEvent`
schema. An output notification's chunk contains `byte_length` and is immediately
followed by its binary frame. Other events have no binary frame. Bytes may contain
arbitrary LF, NUL and non-UTF-8 values; they are never JSON arrays or text. Clients
validate every sequence and runtime epoch before advancing their cursor.

Closing an attachment detaches its observer; it does not terminate the PTY.
Write, resize, renew and release calls use a separate control channel with the
current writer grant. A stream gap or disconnect never invents an agent exit.
Buffered bytes survive the transition from the initial result to notifications.

## Observation connectors

The dedicated connector socket uses the same framing and error rules with
methods `connector.hello`, `connector.list`, `connector.register`,
`connector.activate`, `connector.publish` and `connector.revoke`.
Its parameters contain `destination`, a durable `operation_id`, and the
connector command fields. The operation ID is independent from the JSON-RPC ID;
register and revoke use it for registry delivery identity.

Enrollment remains owner-authorized. Activation binds the credential,
connector instance and generation. Publications report observations only:
they do not acquire PTY custody, recovery authority or a writer capability.
Disconnecting an activated connector retires its observation attachment.

## Client transport

The TUI connects directly to the authenticated local Unix socket and verifies
the peer UID. It uses independent query/control and conversation-watch
connections, with bounded waiting queues. Each connection validates the exact
entry Hub, application capability schema, JSON-RPC version and request ID.
Semantic reply validation runs before a channel is reused; an invalid body
retires it. Cancellation after dispatch leaves a mutation's outcome unknown and
never replays it. Cancellation before admission issues no operation. There is no
native AppKit client or App-to-relay ABI.
