# Origin Session Metadata

Public session summaries now carry an optional recovery hint, and runtime summaries
can carry optional `safe_termination` support. Missing metadata stays unknown;
false support is not confused with absent support. These fields describe the
origin's current observation and never authorize an operation by themselves.

Recovery hints distinguish ready, recovering, resume, recreate and unavailable.
The local adapter derives them from the existing Host recovery and provider
availability records. Live survivors are preserved. An unavailable origin removes
stale mutation affordances without changing the resource address or inventing a
new runtime. The origin continues to validate runtime selection and process
custody when a control request arrives.

Hub RPC, peer protocol and desktop bridge documents use version 2 for this wire
change. Hub/TUI/desktop helpers need a coordinated upgrade. Provider hook schemas
and the private Host/Worker protocol are unchanged. Existing process custody is
not replaced by any of these public metadata fields.

Native clients can now make truthful recovery/lifecycle decisions without fake
manifest timestamps, private manifest digests, or fabricated recovery history.
Workspace migration and configured application behavior are documented in
[Native Hub workspaces](hub-native-workspaces.md).
