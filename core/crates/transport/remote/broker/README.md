# Broker Hub transport

`hub_transport: true` selects SSH transport for a persistent Hub. This configuration
must omit the legacy Host `artifact`. The flag is included in the immutable broker
configuration revision. Legacy runtime bootstrap and runtime socket admission are
disabled in this mode.

`prepare_hub_transport(expected_profile, absolute_deadline)` binds the host ID,
profile authority hash and configuration revision to the current broker instance,
SSH connection UUID and connection generation. It may establish SSH and is allowed
before broker adoption so the detached startup handshake can complete. It never
deploys or probes the legacy Host runtime.

`open_hub_peer(request, absolute_deadline)` requires that exact existing binding,
a loopback endpoint and the intended inner Hub identity. A stale binding is
rejected without connecting or replaying the request. The broker acknowledges the
upgrade on the same verified Unix socket and then forwards bytes on that stream.
The consumer still performs the mutually pinned inner Hub handshake.

Opened tunnels have independent capacity and task custody. They release the
management RPC slot after promotion. Rotation keeps admitted tunnels alive until
their clients close; credential retirement and external shutdown drain them under
the existing bounded shutdown policy. A setup finishing after admission closes
cannot become a proxy.

Closing or dropping a local tunnel closes only its Unix connection and accepted
SSH channel; ordinary local cancellation does not invalidate the pool. Existing
SSH cleanup remains fail closed: an ambiguous channel-open or failed channel-close
settlement can disconnect the outer SSH connection and therefore affect sibling
channels. The broker does not claim isolation from an outer transport failure.

A restarted Hub verifies the existing private broker directory and profile,
prepares the current connection binding, and opens a fresh tunnel. The detached
broker and its SSH connection do not depend on the Hub client process remaining
alive. Enrollment receipts and peer keys remain the Hub coordinator's authority;
the broker does not infer them from endpoints.
