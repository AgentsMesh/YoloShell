# ADR 0026: Stateful web session broker and resource actors

## Status

Accepted (2026-11-02). Supersedes the per-viewer terminal actors, per-viewer
Hub channels and fixed global request lanes described in
[ADR 0024](0024-web-terminal-streaming.md). Keeps ADR 0024's streaming
terminal, history seed, vendored emulator and security decisions.

## Context

Multiple tabs and multiple entry browsers are normal, and one browser may scope
several Hubs. The original web plugin opened a Hub attachment and control
session per browser viewer. That made Hub connection count proportional to
tabs × terminals, duplicated catalogue polling per tab, and let a slow
AgentStart or conversation watch starve inventory when all requests shared a
small set of process-wide lanes. Lease intent was also local to each tab, so
tabs silently competed for the same terminal.

The trust boundary does not change: the plugin is a trusted same-user process;
the browser receives only a shaped protocol, never the Hub socket, Hub identity,
lease grant, creation capability or a generic Hub proxy.

## Decision

### One broker, many clients

One web plugin is the session broker for all tabs using that entry URL. Browser
tabs are `ClientSession` subscribers; they own only view state (selection,
filter, lens, draft). Broker restarts recreate actors from Hub state; no PTY,
lease or terminal lifetime depends on a browser connection.

### Resource ownership

Every shared resource has one upstream owner inside the broker:

- `MachineObserver` per destination Hub: local catalogue, capabilities,
  freshness and reachability.
- `TerminalActor` per `(hub, terminal_id, runtime_epoch)`: one attachment, one
  `TerminalSession`, one shared seed/ring and one local controller selection.
- `ConversationActor` per agent execution: one `watch_conversation` loop and
  one page/revision stream.
- `CreationJob` per creation request: bounded server-owned work with a result
  observable by any authenticated tab.

Tabs never create a second upstream for a resource. They attach as subscribers.

### Hub channel pool

RPC traffic is partitioned by `(destination Hub, latency class)` with bounded
slots and queues:

| Class | Uses | Retry |
| --- | --- | --- |
| query | catalogue/layout reads, capability probes | stale socket once |
| conversation | resolve/read/watch conversation | stale socket once |
| creation | create/finalize/agent_start | never |
| interaction | prompt, respond, lease/control/release | never |
| lifecycle | terminal terminate/archive | never |
| admin | shared-layout updates | never |

Each slot owns its own Hub connection; no class can consume another class's
headroom. A full queue returns a typed capacity error. Reads may reconnect once;
mutations never retry because a lost reply has unknown effect.

### Browser protocol

- `/ws/session`: authenticated JSON stream. Server sends `ready`, catalogue
  snapshots, conversation pages/unavailable/error frames. Client sends
  `catalogue.refresh`, `conversation.watch` and `conversation.unwatch`.
- `/ws/terminal`: authenticated binary output subscription to a
  `TerminalActor`; seed, live bytes, exit and reset/resync.
- `/ws/control`: authenticated input/lease intent to the same actor. The actor
  projects its one Hub lease as `controlled` only to the local controller and
  `observing` to every other tab.
- Mutations that can be slow return a server job (`202`) and are observed by
  that tab; terminal bytes never enter the session JSON stream.
- All sockets use the existing exact Host/Origin, subprotocol, first-message
  token, CSP and no-token-in-URL rules. There is still no generic Hub proxy.

### Terminal semantics

- The actor requests one history seed, falls back to snapshot then legacy
  seedless attach, and retains the seed plus a bounded post-seed ring. A new tab
  receives the shared seed and ring; ring exhaustion triggers one actor-wide
  reseed. Slow subscribers are disconnected and resync on reconnect; they never
  block the PTY.
- One local controller is projected through the actor. A tab with no other
  controller may take over by explicit action; the first keystroke while
  observing is not a takeover when another tab controls. Hidden tabs retain
  control until release, disconnect or an explicit takeover; closing the last
  controller releases the Hub lease.
- Resize is a controller action; observers render the actor's canonical grid.
- An external TUI or another broker remains the Hub authority; actor loss of
  the Hub lease demotes every local tab to observer.

### Catalogue and conversation semantics

- Machine observers query their own destination. New Hubs use local
  `get_platform_snapshot`; older Hubs use paginated `list`/`list_agents`.
  Rows are filtered by origin and transformed to opaque machine handles.
- A failed machine keeps its last rows with `stale`/`unreachable` freshness;
  it never erases another machine's rows and creation on it is disabled.
- Machine handles are HMAC-derived from a plugin-private key, so they survive
  plugin restarts and are stable across tabs.
- Conversation actors keep one cursor per execution and retain the last page
  for late subscribers. HTTP `/api/conversation` remains only for explicit
  older-page pagination.

### Creation jobs

`POST /api/shells` and `POST /api/agents` create a bounded `CreationJob` and
return `202` with a job id. The server owns the Hub mutation and the browser
polls the job (bounded) until `done`, `error` or `unknown`. Jobs are capped and
the job result contains only the opaque terminal address; the Hub creation
capability never enters a job or browser frame. Jobs are durable projections of
a private intent journal, with idempotent browser retries and Hub-backed
reconciliation after a plugin restart; see
[ADR 0027](0027-durable-web-creation.md).

### Limits

Client sessions, terminal actors, subscribers per terminal, output ring bytes,
subscriber buffering, conversation pages, creation jobs and channel queues are
bounded. Slow subscribers are dropped/resynced rather than allowed to consume
unbounded memory. The broker exposes counters suitable for health/metrics.

## Consequences

- Hub connection count scales with active terminal epochs, not tabs × terminals.
- Inventory and conversation are observed once and fanned out to all tabs; a
  slow machine or creation does not block unrelated traffic.
- Terminal control is a server-side ownership decision; tabs no longer guess
  from local lease state.
- The broker is intentionally one process per plugin data directory/user
  entry, enforced by a private `broker.lock`; see
  [ADR 0029](0029-single-broker-data-directory.md). Multiple independent
  brokers keep independent presence; Hub leases still arbitrate the writer.
  Cross-broker presence would require a Hub presence API or a shared broker.
- No Rust change is required for this decision. Old peers are handled by
  capability fallback; richer peer event streams remain a future Hub project.
- The catalogue is fanned out as per-machine epoch/revision patches with full
  snapshot fallback for new or gapped subscribers; see
  [ADR 0028](0028-web-catalogue-patches.md).

## Evidence

Bun tests cover terminal control fan-out, output seeds/ring, machine-view
privacy and the existing protocol/UI suites. Live acceptance
covers two tabs on one terminal, explicit takeover, multi-machine scoping,
peer-shell creation, conversation subscription, creation jobs, plugin restart
reconnect and stable machine handles. The architecture conformance target
passes. Live acceptance evidence is retained with the source revision.
