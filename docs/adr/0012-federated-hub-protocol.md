# ADR 0012: Addressable Hubs and Public Agent Protocols

## Status

Accepted design; implementation in progress. This ADR supersedes the assumption
that the desktop is the only control entry point. It does not supersede local
PTY custody, trusted hooks, process identity, or recovery evidence contracts.

## Resource Ownership

A Hub is a persistent per-user runtime service with its own state directory and
authentication identity. A Session Worker remains the isolated owner of one
PTY. A Hub is not one extra federation service per terminal pane.

Every Hub has a stable HubId bound to enrollment credentials. TerminalAddress is
(HubId, TerminalId); AgentAddress is (HubId, AgentId). RuntimeTarget and AgentTarget
add the exact PTY epoch and Agent execution identity. Provider-native session
references are opaque resume metadata, never global resource addresses. Restoring
an Agent may preserve its logical address but must create a new execution target.

Routing paths and transport connection generations are not resource identities.
Knowing an address, accepting a DTO, or selecting a route never grants authority.
Clients never receive Worker authentication tokens, Guardian descriptors, local
controller generations or an endpoint capable of injecting provider hook events.

## Directed Federation

A -> B means A consumes resources B explicitly exports. It says nothing about
the TCP/SSH dial direction and does not authorize B to consume A's resources.
An outbound connection may carry both directions after separate authorization.

For A -> B -> C with authorized transit, A sees A+B+C, B sees B+C, C sees C.
B serves its own resources and relays C's resources without changing their origin.
Imported resources retain one identity across direct and indirect paths.

The TUI's connected entry Hub determines its view, regardless of the machine
where the client runs. With one exported Agent per Hub and the same authorized
client scope, the A/B/C entry views contain three/two/one Agents. A client of B
does not gain A's resources from A importing B. Additional grants can change
these views; route count never multiplies Agent count.

The remote TUI entry transport is specified separately in
[ADR 0013](0013-tui-remote-entry-transport.md). It uses a separately authenticated
and scoped public client ingress over SSH; federation peer channels cannot carry
public client RPC or confer local management authority. A TUI is not a Hub peer.

Peer catalogues carry the origin's terminals, observed agents, durable agent runs
and workspace summaries. An entry Hub answers each public list as the union of its
local page and the verified catalogues, with the same ordering, filter and cursor
rules as a single Hub; a foreign-origin item in an aggregated list is expected and
does not grant authority. Single-resource reads remain origin-bound, worktree
listing remains local, and catalogue entries are read-only metadata: neither an
origin nor a relay gains an effect channel through them.

Each origin must authorize delegation; each relay can only reduce rights and
remaining reexport depth. Grants and export policies are per recipient and
origin. Independent partial grants cannot be unioned into one stronger route.
Paths have bounded length, cannot revisit a Hub, and cannot claim local ownership.
Receiving a peer snapshot requires a live authenticated peer connection and a
strictly increasing revision. Rejection is atomic. Reconnect invalidates all old
connection callbacks. Missing routes retain explicitly stale, unreachable catalog
entries; they never prove remote process death. Origin controls remain authoritative.

The routing component is not a global distributed topology database or an
authentication implementation. Peer ingress must validate transport identity and
origin grants before installing directory information or admitting any mutation.

## Public Client Protocol

Mac App, native Rust TUI, and third-party clients use the same product protocol.
The TUI is part of this release, not future work. Client protocol, peer protocol,
and authenticated provider/connector ingress are distinct trust boundaries.

Public observations separate lifecycle, activity, freshness, reachability,
observation source, recovery availability, and supported controls. Per-Agent
cursors bind execution, origin stream identity and sequence. Stream replacement
requires a new snapshot; terminal output and directory revisions use their own
cursors. Missing events require explicit resynchronization, not silent replay.

Semantic controls are advertised only when the adapter can implement them.
Observing a permission hook does not imply that automatic approval is supported.
Terminal input remains available through a separately authorized writer lease.
Controls bind the target execution and expected observation, and interactions
add their own identity. The origin checks authorization, writer ownership and
current preconditions at the execution boundary. Request correlation alone does
not promise exactly-once side effects; unknown delivery is never a retry permit.

State-provider ingress is scoped to the provider's registered resources and
cannot create local process custody merely by asserting a PID or an exit state.
Hub filesystem reads are independently authorized and bounded at the origin.

## Native TUI

The Rust TUI manages Hubs, Agents and Shell sessions, supports recursive terminal
panes, focus, durable layout, full-screen attach, detach and explicit takeover.
Layouts bind stable TerminalAddress values without holding writer credentials.
Observers cannot resize the source PTY or answer terminal queries. Only the
current writer controls input, terminal replies and source terminal dimensions.

Use Ratatui/Crossterm for the manager and a maintained VT parser for terminal
surfaces; do not hand-roll a terminal emulator. Shared Rust client components own
connection, authentication, routing, replay and operation results. Terminal state
and process effects do not depend on AppKit or a panel being rendered.

## Delivery Gates

- Runnable Hub service, Mac client and native TUI using public protocol paths.
- A/B/C entry views, reverse connection, explicit export and route deduplication.
- Real PTY sessions and provider hooks, not simulated catalogs as product output.
- Multi-client writer/resize conflict, revocation and stale-execution rejection.
- Relay loss, origin restart, lost replies and replay-gap recovery with no orphan
  processes, duplicate resume or false success.
- TUI PTY E2E and Mac E2E run without taking window focus.
- Separate unit/integration tests, sufficient process/network E2E, every source
  below 200 lines, architecture checks and code-review findings converged.
- Core line/function/region/branch coverage strictly above 95%, measured on the
  full final source revision; no reduced denominator or waived existing failures.

Existing Linux crash-retirement and power-loss gaps remain release blockers.
Federation cannot claim to repair those by routing around a failed local authority.
