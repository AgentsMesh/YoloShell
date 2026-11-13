# Hub, Agent and Session Inventory

The native TUI now consumes three independent public catalogues. Sessions remain
shell-owned terminals. Agents are hook-derived logical resources with execution-
scoped observations, not sessions created through provider-specific launch
commands. Hubs are known authenticated
resource origins, including origins with no sessions or agents.

## Public Directory

`ListHubs { after }` returns ordered `HubSummary { hub_id, reachable }` records and
an optional continuation cursor. Frames are bounded and reject duplicate IDs,
out-of-order records, cursor regression and invalid continuation boundaries.
Reachability means an authorized Observe route exists; it does not claim that
every runtime on that Hub is healthy.

The entry Hub aggregates its own identity and previously authenticated origins.
Configuration pins alone do not publish a Hub. An origin that loses its route
remains visible as unreachable instead of becoming a new identity on reconnect.
An origin-targeted query returns that origin alone, not its imported directory.
For A -> B -> C, querying A produces A/B/C; querying B through A does not expose
B's unexported downstream inventory.

The `hub/catalog/client` component loads bounded pages using one refresh deadline.
It publishes no partial refresh and performs no automatic retry. An absent
advertised method is represented as unsupported, not as a supported empty list.
Cancelling an in-flight refresh retires its protocol connection.

## Native TUI

Sessions, Agents and Hubs are separate inventory tabs. Agent rows display the
reported lifecycle, activity, source, recovery availability, reachability and
freshness. Search covers stable addresses, providers, associated session names,
working directories and reported states. A Hub scope filters Sessions and Agents
but never hides the other Hub choices.
Agent directories are independent observed origin metadata; see
[Agent working directories](hub-agent-directory.md) for freshness and upgrade rules.
Refresh failure retains the last observations but marks them Stale; current
reachability is then Unknown, not a fabricated remote exit or confirmed outage.

Agent selection requires a current inventory and exact agreement between the
Agent's terminal target and the Session's runtime and Agent binding. A stale
Agent cannot redirect selection into a replacement process. The terminal still
uses the existing origin-authorized writer lease; the inventory owns no process
custody and contains no PTY parsing or provider launch logic.

Shell creation respects the selected Hub scope. Shell terminate/archive/resume
commands remain in the Sessions view; the Agent view does not reinterpret these
as semantic Agent control. Unsupported semantic controls remain unavailable.
Existing durable workspaces continue to store pane layout and terminal bindings;
catalogue tabs, search and temporary Hub scopes are transient view state.

Developer key bindings: Tab/Shift-Tab switches tabs; Enter on a Hub selects its
scope; Escape clears scope and filter; Enter on an Agent opens its bound terminal;
`n` creates a Shell on the selected scope, or on the entry Hub when unscoped.
