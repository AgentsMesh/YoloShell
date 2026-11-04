# Private terminal relay control channel

The TUI attach client and its owned terminal relay exchange typed Cap’n Proto
messages from `hub/relay/native/protocol/relay.capnp`. Both sides use the Rust
codec directly. No JSON document, newline reader or alternate legacy wire is
accepted by this codec.

The relay retains the terminal writer grant. This channel carries Agent control
intents and status observations; it never creates a second terminal writer,
acquires authority, launches agents, or retries an operation.

## Framing and bounds

Every frame has a four-byte unsigned network-order payload length followed by a
Cap’n Proto serialized message, including its segment table. A zero length,
oversized length, incomplete segment, trailing bytes, wrong root kind, excessive
traversal or nesting is invalid. Readers validate the length before allocation.
Control payloads are bounded by `MAX_FRAME_BYTES`: 4,096 bytes of envelope
allowance plus the prompt’s 65,536-byte UTF-8 bound. Status payloads use
`MAX_STATUS_BYTES` and the independent bounded holder string.

The root union distinguishes request, response and status. Identities are typed
128-bit values, represented by two UInt64 fields, with non-nil semantic
validation. Cursors remain UInt64 end to end. Status variants cover writer
acquisition/loss, read-only observation and terminal lifecycle events.

## Control and correlation

A request identifies the relay launch, exact Hub-qualified terminal and runtime
epoch, request ID, Agent execution, expected observation cursor and action.
Supported actions are Interrupt and SubmitPrompt. A response echoes that complete
request and returns a receipt, definite not-sent reason, or unknown outcome.

Prompt validation rejects empty/whitespace-only text, over-budget UTF-8, and
terminal control characters other than LF/tab. Escape sequences and raw carriage
returns cannot bypass bracketed-paste framing. Respond remains unavailable until
its provider execution path exists.

Construct requests with `Request::new` and responses with `Response::new`.
After decoding, `response.validate_for(&pending_request)` requires exact launch,
terminal, runtime, Agent execution, cursor, request ID and action correlation.
Reusing a request ID with different prompt text cannot consume an old reply.

Receipt states are accepted, delivered, confirmed, failed and unknown.
Delivered means terminal input delivery, not provider acceptance or agent
completion. The native adapter does not infer confirmed from PTY delivery.
Not-sent reasons require definite evidence before dispatch. After possible
dispatch, timeout, cancellation, disconnect or a malformed response stays
Unknown. Neither Unknown nor Accepted authorizes automatic replay.

The Worker rechecks hook identity, foreground process, bracketed-paste mode,
runtime and writer permission before terminal input. It durably reserves the
intent before one nonblocking write; partial writes stay Unknown. Private parser
diagnostics are discarded and never include prompt contents.
