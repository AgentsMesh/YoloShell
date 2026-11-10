# Hub listener bindings

This component persists the actual address allocated for an explicitly requested loopback
listener with port zero. It retains the Hub's full public identity, state-root object identity,
file identity and a coordination lock. The management runtime keeps the real listener open
while its address is committed; no probe socket is released and reopened.

The record keeps at most two complete configuration states: the currently committed state
and one prepared successor. Management prepares the successor binding before the existing
configuration compare-and-swap. A restart selects the slot whose entire configuration state
matches durable configuration, covering crashes on either side of that write without altering
the user's Configure request, revision, change identifier or grants.

An explicit disabled listener or nonzero address has no saved allocation in its slot. A later
port-zero request receives a fresh allocation; stale slots never authorize another listener.
An unchanged loopback port-zero request retains its existing address across configuration
changes and process restarts. An occupied saved port, replaced record or replaced root fails
closed. A missing record cannot recreate an already configured loopback port-zero listener.

Configuration storage also retains a required, private `listener_binding_initialized` flag.
It begins false in the same write that creates the configuration. This durable evidence permits
recovery if first startup stops before a binding record exists. A retained real listener and its
durable binding must pass the configuration commit fence before the flag becomes true; the
generation starts afterwards. An existing binding is always reused, including while the flag
is false. Configuration changes preserve the flag. Current private schema records with missing
fields are rejected. Configuration storage separately upgrades the exact old unversioned shape
only for disabled or fixed listeners; a legacy port-zero document cannot impersonate interrupted
first initialization.

The initial unassigned slot permits recovery of an interrupted first allocation; the runtime
cannot announce an applied listener until its actual address and initialization evidence are
durable. Unit-level model validation and filesystem integration tests remain outside `src`.
