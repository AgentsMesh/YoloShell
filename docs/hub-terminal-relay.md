# Native Public Terminal Attachment

`yolo-hub-attach` attaches only through the public Hub protocol. It requires the
entry Hub, origin Hub, session UUID, runtime epoch and pane holder UUID. It never
looks up a PID, resumes a session, recreates a runtime, or follows an execution
replacement implicitly.

The relay owns separate control and output connections. A pending terminal read
is not cancelled by input, resize or lease timers. Output and status writers have
bounded backpressure; control deadlines cannot exceed the local lease lifetime.
Network latency is included when calculating that lifetime. Initial control is
announced only after the initial snapshot is written and the lease is still live.

Observers discard stdin, including terminal-generated replies, and cannot resize
the remote PTY. `--take-control` reads the current lease generation and uses that
exact generation in the takeover request. A conflict does not cause a second
takeover. Lost input replies remain unknown; bytes are not automatically replayed.

EOF, parent status-channel loss and handled termination signals detach. The only
cleanup mutation is release of the last known exact writer grant. A broken
control channel may be replaced once for that conditional release; this does not
retry an interrupted input, acquisition or renewal. Acquisition without a known
reply cannot manufacture a release capability and is left to origin lease expiry.

The local status socket uses the existing native two-field NDJSON shape and never
carries writer capabilities. It must be an inherited connected Unix stream, not a
reserved stdio descriptor. Inbound bytes, descriptor-bearing input, EOF or output
failure cancel the relay. The terminal I/O component owns nonblocking descriptor
duplicates and a separate retained termios descriptor, restoring each on exit.
Possibly aliased stdio is captured as a group before any flags change; its original
flags are restored only after the entire group is retired, regardless of drop order.
Darwin's kernel-owned `PENDIN` transition is not confused with lost terminal mode
restoration.

## Native Routing

`HubTerminalRoute` preserves entry and origin separately and binds exactly one
session/runtime. The Mac route registry publishes per-session updates, so another
session's runtime change does not restart unrelated panes. Restored panes use
explicit observation; initial ownership or user takeover uses explicit control
intent. Removing a public route cannot fall back to a same-named private Host.

The TUI continues to use its existing status capability, process generation,
descriptor custody and retirement mechanisms. No new direct process termination
path or focus-changing test launcher is introduced.

This connects public terminal routes to the TUI surface implementation.
Configured client composition and public catalogue/lifecycle services now
use this path; see [Native workspaces](hub-native-workspaces.md). Automatic local
Hub startup and enrollment still remain. This is not a declaration that every
default installation is migrated or that a release package is ready.
