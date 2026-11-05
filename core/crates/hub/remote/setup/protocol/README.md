# Remote setup protocol

The existing `RemoteSetup` operation owns provisioning and route publication.
Its start, inspect, poll and continue calls bind the exact full setup request.

`InspectRemoteSetup { operation_id }` is a separate read-only entry-Hub method.
It returns `RemoteSetupHistory { operation_id, owner, checkpoint }`, projected
from the existing durable setup journal. It accepts no credentials, source paths,
profile configuration or client reconstruction of the original request.

The checkpoint is absent when no operation record exists, or is one of:

- `prepared`: the accepted setup intent was journaled.
- `provisioning`: remote provisioning was admitted; its outcome may be unknown.
- `routing { origin }`: the remote identity was recorded and route publication
  was prepared. Reading this does not apply that pending configuration change.
- `complete { origin }`: completion was journaled in the past. The route, SSH
  profile or remote process may have changed since then.

These are historical checkpoints, not live progress, a connection guarantee,
or permission to replay an uncertain operation. Inspection does not consult or
launch a broker, resume provisioning, reserve a profile, or rewrite journal or
Hub configuration. Removed profiles and revoked routes do not erase history.
Current reachability remains an independently authorized inventory observation.

The operation UUID must be non-nil. Both present and absent results bind to the
requested UUID and entry Hub. Foreign owners, invalid journals and replaced
storage namespaces fail closed. The method is local-only and never delegated
through federation, even with a valid signed resource delegation.

The Rust client exposes `inspect_remote_setup`; the Mac transport provides the
matching typed command and response. This API supplies recovery lookup for a
future onboarding view; it does not implement that view or an operation list.

This method advances the Hub RPC to version 10 and the desktop document to
version 9. Rust clients/TUI, Hubs and the Mac helper must be upgraded together;
unknown Method variants are not silently considered compatible. Older request
and desktop document versions are rejected. The peer envelope remains version
5, following prior entry-only additions: its format is unchanged and this new
method cannot be forwarded even with a valid signed resource delegation. Mixed
RPC versions are also rejected inside peer calls rather than replayed or coerced.

The affected 18-target Rust regression passed in
`.build/hub-history-and-cache-components.log`. History tests cover missing and
active operations, all checkpoints, wrong owners/IDs, namespace replacement,
profile removal and route revocation without reissuing provisioning work.
The complete Swift suite passed 1,550 tests in inactive mode. The existing strict
gate accepted 97.92% lines, 96.78% functions, 96.51% regions and 95.07% branches;
the final output is `.build/hub-history-final-swift-accepted-test.log`.
These results establish the typed API/transport increment, not a completed setup
wizard, operation list, full Core coverage or production package acceptance.
