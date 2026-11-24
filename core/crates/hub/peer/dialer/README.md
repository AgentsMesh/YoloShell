# Peer transport factory

`PeerDialer` opens a `PeerIo` for the complete configured `PeerIdentity`,
`PeerAddress`, and one absolute deadline. It has no local signing key and cannot
establish a trusted federation connection. Federation passes the returned I/O
through `peer_link::connect`, which performs the signed `PeerHello` identity
handshake with the remote Hub before any federated call is sent.

`dial_peer` bounds factory execution and validates the address contract.
Dropping the returned future cancels its in-flight factory future. Implementations
must retain cancellation custody for resources they create; the returned `PeerIo`
owns the close action for exactly one transport.

Production injects `SshProfileRuntime`, which resolves the profile id against the
persistent Hub registry and opens a broker `direct-streamlocal` tunnel to the
remote `<hub-state>/peer.sock`. There is no default TCP dialer and no fallback:
a missing profile or failed broker open rejects the link. Embedded or in-process
tests may inject a direct Unix-socket dialer or a rejecting dialer.

`Federation::new_with_dialer` injects a factory. `ManagedHub::open_with_dialer`
retains the same factory across configuration generations, while old generation
tasks and transports are retired before a replacement generation runs. Opening
the transport and the identity handshake share the original deadline. Existing
mutation forwarding and unknown-outcome rules are unchanged.

Profile lookup, SSH connection pooling, enrollment, and external credentials
belong to the composition layer. This component imports no remote broker or
runtime engine and does not infer profile mappings from hostnames.
