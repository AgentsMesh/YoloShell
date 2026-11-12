# Persistent Hub SSH profiles

Federation peer addresses explicitly select TCP or SSH. An SSH address contains
a stable profile ID and expected profile authority digest, plus the remote Hub's
canonical loopback address and nonzero port. Missing or changed SSH profiles fail
connection admission; they never fall back to TCP. The configured Hub public key
remains mandatory for the inner mutually authenticated Hub connection.

The persistent Hub owns the profile registry and the federation dialer. Public
`GetSshProfiles` and `ConfigureSshProfiles` commands are local management operations;
peers cannot forward them. Configuration changes bind request ID, base revision
and the complete desired configuration. A successful latest exact replay is
idempotent. An unknown reply requires reading and correlating the current state,
not automatically sending another mutation.

Each registry entry binds the public profile reference to an existing local
broker state directory and exact broker config revision. These values contain no
private keys, passphrases or credential envelopes. The store binds the full Hub
identity, directory identity and exact record file identity. A replacement record
with identical JSON is still a conflict. A storage failure poisons that store
instance rather than silently adopting replacement state.

`--ssh-profiles` may seed a new Hub registry from a private JSON configuration.
Ordinary later updates use the public commands, so an initially empty registry
can be populated without restarting the Hub. The optional seed must agree with
an existing registry. Every real Hub installs the SSH-aware dialer; embedded Hubs
without one continue to reject SSH transport requests explicitly.

## Broker and channel lifetime

A broker configured with `hub_transport: true` cannot carry an old single-Host
artifact or start the old runtime proxy/bootstrap path. Its connection preparation
authenticates SSH only. Mode is part of the immutable broker configuration digest.
Pending startup can prepare SSH before adoption, but cannot open a Hub tunnel.

Preparation returns the exact broker instance, profile authority, configuration
revision, SSH connection UUID and generation. Tunnel open accepts only that
existing connection; stale bindings cannot reconnect or silently acquire a new
generation. The management Unix connection upgrades to a raw byte stream after a
correlated reply. Tunnels have independent capacity and task ownership, so they
do not hold management request slots for their lifetime.

The Hub keeps the same dialer across federation configuration generations. Each
tunnel retains its own close authority. Profile removal or replacement closes
its existing tunnels before confirming the registry change. Observed registry
identity loss closes all its tunnels. Runtime drop closes them synchronously;
watchers use weak references so they do not retain Hub or profile-store locks.
Remote Agent processes and PTY custody remain at the origin.

Existing adopted brokers can survive entry Hub exit. A restarted entry Hub reads
its registry, verifies the existing broker and opens a new inner Hub connection.
Normal tunnel EOF or cancellation closes that tunnel only. Ambiguous SSH channel
open or failed transport cleanup may still close the outer SSH connection under
the existing conservative cleanup policy; sibling-channel survival is not an
unconditional guarantee for those failures.

## Enrollment recovery

Remote enrollment durably records the complete original request, peer identity,
configuration snapshot and exact desired change before its one permitted
Configure submission. Only the fresh in-process submit permit authorizes that
submission. Reading the durable record cannot recreate the permit.

`--recover-remote-bootstrap` only reads the existing journal and running Hub.
It reports Absent, Unknown, Superseded or Ready for the original request. Even a
completed historical receipt must be checked against current build, identity and
exact configuration. A later revocation cannot be undone by replaying an old
bootstrap request. A crash before sending Configure remains Unknown rather than
authorizing a retry. See the [enrollment store contract](../core/crates/hub/enrollment/store/README.md).

## Delivery boundary

This increment connects existing admitted brokers to persistent Hub federation.
Full remote onboarding still needs Hub-owned broker launch/adoption orchestration,
bundle catalog selection, publication/enrollment operation management, and Mac/TUI
forms that compose those operations. Current Mac/TUI peer editors preserve and
protect existing SSH bindings; they do not yet create the entire remote setup.
The complete Linux bundle packaging and independent-machine release acceptance
also remain roadmap items.


## Credential-free host identity discovery

The `transport/ssh/host/probe` crate was retired on 2026-10-26: it had no
production consumer, and live host-key trust is `transport/remote::KnownHosts`.
Its credential-free probe semantics (no authentication, channels, credentials,
trust writes, or profile mutation; authenticated reconnection verifies the key
again) remain the contract for the future Hub-owned trust challenge, which must
be built from live types with a real consumer test.
