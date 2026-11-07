# TUI creation result recovery

A TUI can exit after the origin creates a shell but before the Created reply is
saved locally. Recovery now retains the original request before dispatch and
queries its result; it never resends Create from an uncertain transaction.

## Durable ordering

The client first reserves a bounded local queue slot. It then persists a
`CreationIntent` containing the origin and complete original CreateShellRequest.
Only a successful new journal commit permits that initial Create to enter the
queue. An already-retained exact intent is not another dispatch permit.

A correlated Created reply atomically replaces the intent with a confirmed
finalization record. Only then may the original, still-current in-memory pane
intent bind the shell. Finalize retains its existing idempotent receipt contract.
An expired or failed creation retires pane intent while preserving the original
transaction for read-only recovery.

The current journal schema holds intents and confirmed records under one capacity
and request-ID/token uniqueness contract. Unsupported schemas or missing intent
fields are rejected without upgrade or rewrite. Namespace, directory identity,
ownership lock, file identity and exact own-commit recovery remain enforced.

## Known uncreated outcomes and uncertain delivery

The live client can retain proof that its initial attempt created no resource:
an unused queue permit, an exact locally correlated NotSent, NotConnected,
StaleGeneration, Invalidated or OperationLimit result, or an authenticated
InvalidDirectory rejection issued before origin creation admission. It may then
durably remove that exact intent. If storage fails, a bounded in-memory map keeps
this proof until the same journal can recover and acknowledge removal. The
unchanged provisional layout may be rolled back; a current directory form keeps
its input and displays the actionable error. Later user edits cannot be reverted.

That map is never serialized. A restarted journal, generic remote error,
OutcomeUnknownResponse(InvalidDirectory), malformed reply, unknown delivery or
Missing query cannot establish a no-resource outcome. Those intents
remain available for authenticated origin lookup. Late callbacks cannot remove
an intent whose original live pending owner has already expired or retired.

## Independent query execution

The `tui/creation/recovery` component owns query state and request scopes. It emits
only QueryCreation, preserving the original origin, creation token and request.
The App owns journal transitions and finalization; the recovery component cannot
bind panes, acquire writers, send input, resume an agent or create a shell.

QueryCreation has its own connection/task owner and four capacity slots in
`hub/client/channels`, separate from both fast terminal controls and lifecycle
work. It uses the ordinary five-second RPC budget and any earlier validity
deadline. Query transport failures and task panics report the original request's
failure without disconnecting a healthy fast channel. Closing the owner retires
its tasks and sockets.

Every completion must match the current attempt's scope, generation, deadline,
complete original request and origin; query results also bind token and origin.
Expired, disconnected or replaced attempts cannot overwrite later evidence.

| Result | Client action |
| --- | --- |
| Missing / Pending | Retain the intent and schedule a bounded read-only recheck. |
| Unknown / permanent rejection | Retain the intent, show blocked confirmation, permit an explicit recheck. |
| Ready | Durably promote to a confirmed record, then use existing Finalize settlement. |
| Finalized / Survivor / Archived | Durably settle the matching intent; do not infer current liveness from historical creation evidence. |

Strong results remain in memory until the matching durable transition succeeds.
Storage recovery repeats that exact local write; it does not downgrade evidence
or repeat the original Create. Untouched intents precede retries when query
capacity is limited. Ctrl+B f can explicitly retry blocked checks as well as
proof-preserving storage recovery.

Restart restores transaction obligations only. Recovered Ready results have no
pane-binding intent, writer lease or control generation. Normal catalogue
observation and explicit terminal selection remain responsible for opening views.

## Verification scope

Journal tests cover current-schema rejection, cross-record collisions, atomic
promotion, commit-boundary failure, namespace replacement and exact recovery.
Recovery tests cover all result classes, stale and malformed completions,
capacity, fairness, deadlines and generation changes. Unix channel tests verify
blocked queries cannot stall input/Hello/renew or consume lifecycle capacity.
App integration tests verify durable-before-dispatch, restart-only queries,
known non-dispatch cleanup, storage failure and no recovered pane/writer authority.

The real PTY scenario is
`hub::tui_creation_recovery::lost_created_reply_survives_real_tui_crash_and_restarts_with_query_only`.
Its transparent Unix proxy verifies the durable intent before forwarding the
actual Create, withholds the genuine Created reply, and observes a fenced TUI
crash. Restart must query the original request, finalize the same result, leave
panes/writers unbound and preserve the exact origin runtime with fresh marker IO.
The proxy rejects a second Create and unexpected mutations. Production TUI
coverage profiles are retained after the forced exit.

These scenarios are deterministic test contracts, not production acceptance.
