# Current contracts before release

Yolo has not shipped. Its product formats have one supported representation at
each boundary. New code must not add readers, aliases, inferred authority or
automatic migrations for retired Yolo versions.

## Supported boundaries

- Socket namespaces use a durable registration and an exact directory
  certificate. Shared ownership and the root directory lifecycle lock protect
  live processes. Temporary namespace markers and their retirement adapters
  are removed.
- PTY custody uses the current descriptor-attested protocol, explicit Worker
  activation and exact guardian retirement evidence. Retired wire versions and
  records cannot enter current state transitions.
- Worker authentication requires a complete hello with an explicit capability
  list. A disconnected runtime observation may still have unknown capabilities;
  uncertainty is not authority.
- Creation receipts require the current schema and the original request
  fingerprint. Startup checkpoints, Hub configuration and TUI journals/layouts
  require their current schemas. Reading a retired record does not upgrade it.
- Desktop Hub state is either freshly initialized for its entry Hub or already
  bound to that Hub. Host identity remapping and Hub-to-Host routing fallbacks
  are removed. Explicit direct transport remains a separate supported route.
- Broker status requires explicit adoption and capability evidence. Missing
  fields must not fabricate readiness.

Unsupported input is rejected before state mutation. Existing records and
terminal output are not deleted merely because they use a retired format.
Schema numbers still identify formats; retaining a version field is not a
promise to read older versions.

## Recovery and deployment

Crash recovery, partial current-format publication, transport reconnect,
process identity fencing and recovery after a machine reboot remain product
features. They recover current state after a fault. Their presence does not
authorize reading retired product formats.

A process from an earlier development build may retain live PTYs or hold
files. Removing source compatibility does not authorize signalling it,
deleting its state or assuming that a new Host can adopt it. Validate a new
binary in an isolated state directory before deployment. An incompatible
development installation requires an explicit cutover plan for its live
terminals; do not use a rolling restart to discover format incompatibility.

## Enforcement

Architecture checks reject reintroduced retired contract symbols, forbid the
removed socket-root publication adapter and check the current namespace
registration/certificate publication boundaries. Rejection tests exercise
retired schemas, missing authority fields and preservation of stored evidence.
These checks complement review; a denylist alone cannot prove the absence of
every possible compatibility adapter.
