# Online Hub Configuration

Hub discovery is entry-relative, not a globally replicated inventory. For
authorized A -> B -> C sharing, A sees A/B/C resources, B sees B/C, and C sees C.
B serves its own resources and relays C without changing C's resource addresses
or acquiring ownership of C's PTYs. Sharing direction is policy, not transport:
a Hub dials every next hop it needs, and an accepted link never carries reverse
requests.

## Public Management Boundary

`yolo-hub` now runs `hub/management/runtime`, wrapping the federation dispatcher.
Mac, TUI, and independent clients can use the same public command contract:

- `GetConfiguration {}` returns the local Hub's public identity, durable revision,
  latest change receipt, configuration, activation status, and published peer
  socket.
- `Configure { change }` carries a non-nil request ID, expected revision, and the
  complete desired `federation_settings::Configuration`.

These methods are advertised only by the managed dispatcher. They administer
the current entry Hub; addressing another Hub is unauthorized. Signed peer calls
cannot invoke them, even with a valid resource delegation. Resource observation
and terminal control remain separately authorized and origin-addressed.

Public clients use JSON-RPC 2.0 and negotiate the current Hub capability schema.
Peer negotiation is a separate internal Cap’n Proto contract. Configuration DTOs
reject unknown fields, invalid identity/revision values, and contradictory
activation/peer-socket state. A Configure success must match its exact request ID,
new revision, and desired configuration and must report `applied: true`.

The public `get_configuration` and `configure` methods transport these commands.
The [TUI](tui-hub-management.md) has a configuration editor using this
entry-local administration boundary. `yolo hub peer add` writes one peer with
its SSH profile, remote socket and export policy; there is no pairing code or
enrollment operation. The local Hub publishes `<hub-state>/peer.sock`; the
provider side pins the consumer identity and configures its export policy
locally or through remote setup. See [Hub peer links](hub-peer-links.md).
`Configure` remains the only general configuration mutation.

## Persistence And Concurrency

`hub/configuration/store` owns the private configuration directory and its
singleton lock. Records bind the exact directory identity and complete local
Hub identity. It retains file identity, checks that identity at each read, and
uses expected-file atomic replacement with a lock/namespace commit fence.
Corruption, replacement, or lost custody permanently disables that store instance.

Revisions start at 1. Compare-and-swap rejects stale revisions and exhaustion
before retiring the current generation. The latest exact duplicate request is a
read-only receipt lookup: it does not rewrite the record, increment revision,
or restart an already applied generation. Changed payloads or preconditions with
the same request ID conflict. Older receipts are not a general operation journal.

`--peer-config` is an explicit initialization seed. Once the Hub has a persisted
configuration, an explicitly supplied conflicting file fails startup rather than
silently replacing durable policy, and the error points at this boundary. Normal
subsequent starts can omit the seed. Change peers on a persisted Hub through
`Configure` (the JSON-RPC method behind `yolo hub peer add`, the TUI and the Mac
configuration editors); a restart seed cannot update them. This protects a running owner from replacement, not against an authorized
user rewriting valid historical state while the service is stopped.

## Activation And Failure

Management requests enter a bounded, serialized actor. Once accepted, an update
continues if the requesting client times out or disconnects. Clients must inspect
the resulting revision/receipt after an uncertain outcome, not automatically
replay a mutation.

An update validates policy first. For activation, the actor fences old
admission, stops and joins peer tasks, drops old peer links, drains public
operations, and releases the old Publication owner. It persists the desired
configuration before constructing the replacement generation. The peer socket
is derived from Hub state and is republished once for the service lifetime, so
an update never rebinds it.

History transfer contains only origin-signed catalogues, never delegated grants,
route proofs, or connection generations. History stays unreachable until new
authenticated discovery establishes authority. Replaced keys cannot inherit
old-key observations. Origin policy epochs still advance durably through
Publication when the configuration digest changes.

`applied` means the new local policy is active, not that every peer is connected.
Storage/activation failure does not claim a successful revocation. When the
configuration was already durable but its generation failed to start, the
service retries activation on its own interval instead of waiting for another
management request; snapshots keep reporting `applied: false` until a
generation is running. Periodic custody verification also retires the service
when configuration storage is lost without an administrative request.

Already-admitted operations and delivered terminal bytes cannot be retracted.
Revocation closes old access paths; it never terminates, resumes, or recreates
an origin process. The executable retains its authenticated Host hold across
generation replacement. Local PTYs remain owned by existing Host/Worker custody.
