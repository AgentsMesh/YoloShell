# Peer transport factory

`PeerDialer` opens a `PeerIo` for the complete configured `PeerIdentity`,
`PeerAddress`, and one absolute deadline. It has no local signing key and cannot
establish a trusted federation connection. Federation always passes the returned
I/O through `peer_transport::connect_io` with its configured pin and private key.

`dial_peer` bounds factory execution and validates the existing address contract.
Dropping the returned future cancels its in-flight factory future. Implementations
must retain cancellation custody for resources they create; returned `PeerIo`
owns the close action for exactly one transport. The default `TcpPeerDialer`
accepts only explicit/default TCP transport and preserves duplicate-socket close
custody through `PeerIo::from_tcp`. SSH bindings require an injected factory;
missing or failed bindings never fall back to TCP.

`Federation::new_with_dialer` injects a factory. `ManagedHub::open_with_dialer`
retains the same factory across configuration generations, while old generation
tasks and transports are retired before a replacement generation runs. Opening
and mutual authentication share the original deadline. Existing mutation
forwarding and unknown-outcome rules are unchanged.

Profile lookup, SSH connection pooling, enrollment, and external credentials
belong to the composition layer. This component imports no remote broker or
runtime engine and does not infer profile mappings from hostnames.
