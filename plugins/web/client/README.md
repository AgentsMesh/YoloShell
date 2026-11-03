# Yolo Web Client (trusted-plugin canary)

This is the first trusted plugin canary. It is a user-level Bun process; it has
no sandbox, owns its daemon and serves its own HTTP UI. The canary currently binds
`0.0.0.0` so a trusted LAN can reach it; this is a temporary mode.

```sh
# from a Yolo checkout with the trusted plugin CLI available
yolo plugin link "$PWD/plugins/web/client" --trust
export YOLO_PLUGIN_PORT=4199
export YOLO_HUB_SOCKET=/path/to/hub.sock YOLO_HUB_ID=<hub-id>
yolo plugin startup
cat "$(yolo plugin data-dir web.client)/token"
# open the printed local/LAN URL and paste the token
yolo plugin action web.client status
yolo plugin action web.client stop
```

The server acquires a private `broker.lock` in the plugin data directory before
listening. A second process pointed at the same data directory exits before it
opens the Hub, because the creation journal and local controller projection are
single-writer state. A stale lock from a dead PID is recovered on the next start. See [ADR 0029](../../../docs/adr/0029-single-broker-data-directory.md).

The browser UI is a workbench organized around two work concepts:
**Agents** (an agent session with Chat and Terminal views) and **Shells**
(terminal-only sessions). An agent's terminal is not repeated as a separate
shell row. **Needs you** is an inbox for answerable interactions. TUI layout
tabs and empty panes are not exposed; shared layouts remain internal to the TUI. Browser state has one immutable `AppState` owner in `browser/store.ts`: pure domain reducers under `browser/appReducer.ts`, controllers that dispatch intents and own effects, and the single event composition root in `browser/events.ts`; renderers read selectors and never mutate state. A per-observation `CatalogueIndex` backs agent, terminal and provider lookups, and selection stores an identity rather than a snapshot. The store boundary and selection identity are covered by the plugin unit suite.

The inventory endpoint is an authenticated diagnostic that returns the current
aggregate catalogue with an opaque machine handle for every Hub. The browser
receives a catalogue snapshot followed by per-machine epoch/revision patches; a revision gap triggers a private snapshot resync. The browser keeps live
phases visible (starting,
running, output-blocked, finalizing) and keeps a degraded observation from
erasing a row; exited or lost sessions are not shown. Selecting an agent opens
Chat when its native transcript is readable and otherwise falls back to its
live Terminal; selecting a shell opens Terminal. Chat renders agent text as
safe Markdown (headings, lists, code, tables, emphasis and http(s) links) without
interpreting source HTML. Chat prompts use Hub `agent_prompt` and are enabled by
the observed `submit_prompt` capability and freshness; incapable or stale agents
show a note to use the Terminal instead. An unconfirmed receipt retains the
draft and points at the transcript rather than inviting a blind resend.

**+** beside Agents and Shells opens a creation dialog. Shell creation uses the
public Hub `create`/`finalize` flow; agent creation uses the Hub's closed
`agent_start` launch plan. Each request becomes a durable server-side intent:
`POST /api/shells` or `/api/agents` journals the complete original request
before dispatch, then returns `202` with a job id. The tab observes the project
until it is `done`, rejected, or `unknown`. Retries with the same idempotency
key return the same job; a plugin restart reconciles a shell through
`query_creation` and an agent through the same `start_token`, never by replaying
a new Create. An older Hub that cannot confirm an uncertain shell leaves the job
`unknown`, and the UI tells the user to check Agents and Shells instead of
resubmitting. A starting agent appears under Agents as a pending row
immediately, even before its terminal reaches the catalogue page, and is
replaced when its native binding is observed. Pending identities are persisted
with a bounded grace window so a delayed catalogue cannot erase them. The
creation capability stays server-side and is never sent to the browser.
Journal locations, state transitions and limits are in
[ADR 0027](../../../docs/adr/0027-durable-web-creation.md).

The agent picker uses each machine's Host-owned provider facts from the public
`list_providers` method. Unavailable providers are disabled, a machine with no
available provider cannot start an agent, and a new Hub rejects an unavailable
`agent_start` before creating a terminal. An older Hub without the method keeps
the static fallback. Provisioning, security and the wire shape are in
[ADR 0031](../../../docs/adr/0031-host-owned-provider-discovery.md).

An agent with a pending typed interaction appears under **Needs you** and shows
its prompt and choices above the pane. Answering re-reads the live snapshot,
takes the writer lease, sends the exact `AgentControl` response for that
interaction and releases the lease. Stale, replaced or unsupported interactions
are rejected without a terminal effect.

The terminal is a live xterm.js surface fed by one shared `TerminalActor` per
terminal runtime epoch. The actor owns the only Hub attachment and
`TerminalSession`; every tab attaches as an output/control subscriber. The actor
retains the seed and a bounded post-seed ring, so a late tab receives the same
history/snapshot and recent output without opening another Hub attachment. A
`resync_required` event or ring exhaustion reseeds the shared actor and resets
all subscribers. Slow subscribers are disconnected and resync on reconnect;
they never block the PTY. The initial seed asks for the raw retained history
stream, so scrollback, alternate-screen state and input modes survive reload and
reconnect; when the Worker journal is compacted the Hub falls back to the
`state_formatted()` snapshot. A Hub that fails the history seed without
returning that fallback is retried with the snapshot seed, and a Hub predating
the seed field is retried with the legacy seedless attach.

Control is projected from the actor: one local controller, every other tab
observing. Selecting a row is read-only; the first keystroke while no other tab
controls takes control and buffers the triggering bytes. If another tab
controls, typing remains observation and the tab offers **Take control**; an
explicit takeover demotes the previous tab. Hidden tabs retain control until
release, disconnect or takeover, so switching tabs does not steal or drop a
lease. Closing the last controller releases the actor's Hub lease; an external
TUI or broker remains the Hub authority and demotes all tabs if it takes over.
On binding, a pending agent's terminal selection becomes the Agent selection
while keeping the Terminal lens. Both sockets carry a 15 s heartbeat; a silent
half-open flow, frozen tab or Hub restart reconnects and rebuilds the actor's
private Hub session without a page reload.

Inventory and conversations are pushed over `/ws/session`: one broker-side
machine observer and one conversation watcher feed every tab, instead of each
tab polling. Each machine advances an observation epoch and monotonic revision.
The first frame is a snapshot with those revisions; later frames are patches
containing only changed rows or removals, with a full snapshot for discovery,
epoch changes, unkeyed rows or a client revision gap. Mutations, long-poll
watches, query reads and administrative
operations use a bounded Hub channel pool keyed by destination and latency
class, so a slow AgentStart, prompt, peer or conversation watch cannot starve
the catalogue or another class. Machine handles are HMAC-derived from a
plugin-private key and stay stable across plugin restarts; a stale machine keeps
its last rows with a `stale`/`unreachable` status and cannot be used for
creation until it recovers.
A selected live terminal exposes **End** and a finished one exposes **Archive**
when the machine advertises the public `terminal` command. Both send the exact
terminal id and observed runtime epoch. A successful End leaves a notice with
an Archive action. Ended sessions are not listed as sidebar work: they appear in
**History** together with archived sessions. A finished row offers Archive; an
archived row opens its read-only retained history. **Log** opens a scrollable
plain-text transcript from the live actor's seed/ring, including when the
provider is in an alternate-screen TUI. History is capped at 500
rows and only includes machines that advertise `read_terminal_history`; an older
peer is shown as unavailable instead of offering a broken action. The history
view replays the Host-retained raw stream into a read-only xterm; it never
attaches to a runtime or takes a writer lease. Compaction or the byte budget
marks the stream `truncated`. See
[ADR 0032](../../../docs/adr/0032-archived-terminal-history.md). Older Hubs hide
the lifecycle controls. See
[ADR 0030](../../../docs/adr/0030-web-terminal-lifecycle.md).

Notifications require a secure context; the button explains the HTTPS or
localhost requirement instead of silently doing nothing. Browser tabs never
poll the catalogue and never receive the Hub socket, raw Hub id, lease grant or
token. It receives only opaque machine handles for scoping and routing. The
user reads the token from the plugin data file and pastes it into the page.
Fixed routes are `/`, `/app.css`, `/app/*.js`, `/vendor/xterm.js`,
`/vendor/xterm.css`, `/vendor/addon-fit.js`, `/health`, `/api/inventory`
(authenticated diagnostics), `/api/respond`, `/api/conversation` (older-page
pagination), `/api/prompt`, `/api/shells`, `/api/agents`,
`/api/jobs/<uuid>`, `/api/lifecycle`, `/api/archive` (on-demand archived rows),
`/api/archive/history` (bounded raw history), `/api/terminal/log` (bounded live
log), `/ws/session` (catalogue/conversation push),
`/ws/terminal` (output) and `/ws/control` (lease, input, resize,
take/release). All sockets require the exact Host and Origin, the
`yolo.web.terminal.v1` subprotocol, and a first-message token. There is no
generic Hub RPC proxy. `/api/layout` remains a server-side shared-layout read
for future workspace work; the browser currently exposes no layout or tab
surface.

## Vendored terminal emulator

The browser terminal uses a pinned, self-hosted xterm.js 6.0.0 and addon-fit
0.11.0, both MIT. The exact files, sources and SHA-256 hashes are recorded in
`vendor/manifest.json`; the licenses are beside them. The server verifies every
manifest hash at load and fails closed on a mismatch. The files are served from
the fixed `/vendor/` routes and are never loaded from a CDN or installed with a
package manager.

This is the one narrow exception to the repository's TypeScript/Bun policy, as
recorded in [ADR 0024](../../../docs/adr/0024-web-terminal-streaming.md):
only the allowlisted files under `vendor/` are exempt from handwritten-line and
frontend-brand checks. JavaScript tooling and dependency directories remain
forbidden everywhere else. The page keeps `script-src 'self'`; because the
emulator injects style elements, its CSP allows `style-src 'self'
'unsafe-inline'` without accepting inline scripts or user-authored styles.

## Temporary LAN access

The server binds `0.0.0.0` so a trusted LAN can reach it. It accepts loopback
names, the machine's local IPv4 interface addresses, and IPv4 addresses listed
in `YOLO_WEB_ALLOWED_HOSTS`, all with the exact listening port. A present
`Origin` must equal `http://<Host>`. DNS names are rejected.

When Yolo runs in an OrbStack machine, OrbStack forwards the VM's `4199` to the
host's `0.0.0.0:4199`. Start the plugin with the host's LAN IPv4 so browsers on
the host LAN can use it:

```sh
YOLO_WEB_ALLOWED_HOSTS=<host-lan-ip> yolo plugin startup
# comma-separate multiple IPv4 addresses
```

Then open `http://<host-lan-ip>:4199/` from another machine. The bearer token
is still required. LAN traffic is plain HTTP, so use this only on a trusted
network while the temporary mode is enabled. The management socket, Hub
identity and token are never sent to the browser.

## Validation
```sh
bun test
bun build browser/main.ts --target=browser --outdir "${TMPDIR:-/tmp}/yolo-web-browser"
bun build scripts/plugin.ts server/index.ts --target=bun --outdir "${TMPDIR:-/tmp}/yolo-web-build"
```

The browser modules are TypeScript; the server discovers `browser/*.ts` and
transpiles each `/app/<name>.js` asset with `Bun.Transpiler`, so a new module is
served without a second allowlist. There are no npm dependencies or dependency
directories; the emulator files under `vendor/` are committed assets.
