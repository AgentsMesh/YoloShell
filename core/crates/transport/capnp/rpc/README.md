# Cap'n Proto RPC execution boundary

The public `Connection`, `RemoteEvents`, `Service`, and `EventSource` interfaces
are `Send`. Cap'n Proto clients, servers, promises, and capability tables stay on
one process-wide Tokio `LocalSet` thread. Connections and outstanding calls have
separate bounded admission; terminal sources have their own capability limit.
No request or terminal starts a new OS thread.

`serve` captures its caller's Tokio runtime. Domain service methods, terminal
event reads, and successful publication callbacks run there, so a domain mutex
or filesystem operation does not block the shared RPC executor. A connection
scope records domain task abort handles and cancels them on disconnect, including
when Cap'n Proto still retains a method promise. Cancellation never retries a
mutation and never establishes that a previously dispatched mutation did not run.

`Endpoint.call` carries structural `Value` fields. `Peer.discover` and
`Peer.execute` are separate generated methods. An attached terminal returns an
`Events` capability with `next` and `cancel`; dropping it detaches the observer.
Idle terminal clients use `next_unbounded`, while signed federation heartbeat
streams use `next(deadline)`. Neither path sends a terminal termination command.

The RPC layer grants no operating-system or Hub authority. Each service must be
created with an authenticated transport context, validate its domain request,
and apply current writer, route, execution, and signature checks before work.

An optional `Reply.on_observed` callback becomes a `Receipt` capability. It runs
once when the client confirms observation. Unconfirmed receipt expiration drops
captured publication tickets and permits without calling the success callback.
`MAX_RECEIPT_WAIT` bounds this lifetime even when a peer retains the capability.
A positive confirmation is required before a receipt-bearing business reply is
reported as certain. A lost acknowledgement or expired confirmation produces
`OutcomeUnknown`, since the callback may commit an operation such as idle
shutdown. The transport never retries it automatically.

Proxies use `request_deferred` and `RemoteReply.into_service_reply`: they retain
upstream receipt custody until their own downstream client confirms observation,
then await positive upstream confirmation before acknowledging downstream.
Dropping an unobserved proxied reply releases the guard without acknowledging it.
Domain adapters should validate decoded response identity before confirming a
deferred receipt; structural decoding alone is not domain validation.

Integration tests exercise true RPC method dispatch, returned event capabilities,
lost confirmation acknowledgements, proxy acknowledgement ordering, retained
receipt expiry, connection cancellation, and domain mutex isolation.
