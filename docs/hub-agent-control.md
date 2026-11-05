# Public Agent Control Requests

The shared Hub protocol carries `AgentControl { request, grant }` and returns
an `AgentControl { receipt }` reply. Native Claude/Codex observations can advertise
Interrupt when the Worker retains coherent hook authority and durable operation
history. Interrupt sends SIGINT only to the selected retained process identity.
OS delivery produces Delivered, never Confirmed cancellation. A Claude
`PermissionRequest` also advertises `respond_interaction` while its exact
interaction is pending; approval choices are `allow` and `deny`. Elicitation,
text responses and providers without a native decision channel remain
Unsupported.

## Request, Reply and Routing

The request contains the exact AgentTarget, expected observation cursor, semantic
action, request ID and terminal writer grant. Its Agent origin, grant origin and
RPC destination must agree. At the origin, the grant's full RuntimeTarget must
match the current Agent observation's terminal target.

A reply must match the request ID and complete AgentTarget, including execution
ID. A correctly signed reply for a different request or execution is invalid.
The RPC operation is classified as a mutation, so a lost, malformed or mismatched
reply has an unknown delivery outcome and cannot authorize automatic replay.
A definitive Failed receipt remains terminal; it cannot advance to Delivered or
Confirmed through the public receipt API.

Federated routing requires both AgentControl and TerminalWrite on the same valid
delegated route. Observe, Lifecycle, FileRead or TerminalWrite alone is insufficient.
Partial permissions from different routes cannot be combined. Existing grants
do not gain the new right, and both Mac and TUI export editors expose it as an
independent checkbox that defaults off.

## Current Native Behavior

| Condition | Result |
| --- | --- |
| Interrupt with current native witness, runtime and authenticated writer | Delivered if the exact process accepts OS signal delivery |
| Respond to the current Claude permission interaction with the exact reference | Delivered if the pending hook accepted the choice; NotDelivered otherwise |
| Respond after the interaction was replaced, answered or timed out | Failed: PreconditionsChanged; no hook effect |
| Repeated request with identical digest | Original durable receipt; no second signal or decision |
| Unsupported action or observation-only source | Failed: Unsupported |
| Changed execution, cursor, freshness, reachability or terminal target | Failed: PreconditionsChanged |
| Lost delivery/receipt proof or unavailable private control result | Unknown; no automatic replay |
| Unknown Agent address | NotFound |
| Agent observations unavailable in the backend | Method unadvertised; direct dispatch Unsupported |
| Different Agent/grant/destination origins | OriginMismatch before dispatch |
| No route with both required rights | Unreachable under the existing routing disclosure policy |

Unsupported controls do not send terminal bytes or signals, resize a PTY, or
renew a writer lease. A synthetic Available capability cannot create effect
authority. An approval response requires the current interaction reference and
snapshot cursor; it never falls back to keystrokes or screen parsing.

The Worker validates the writer secret, controller generation, lease expiry,
execution, source ownership and observation witness together under its mutation
guards. An intent is persisted as Unknown before the single signal attempt.
Proof loss after that write cannot authorize another attempt. Receipt lookup
does not require a fresh writer lease and does not send or renew anything.

Public calls use JSON-RPC with the current Hub capability schema; peer and
Host/Worker calls use Cap'n Proto. Agent observation/request schemas remain 1.
See [communication architecture](communication-architecture.md) for the current
boundaries. Observation, receipt lookup and Interrupt methods are capability
gated. Protocol support is stable after control-history initialization; each
observation separately checks current history/source availability. Retired
Worker protocols are rejected rather than adapted.
