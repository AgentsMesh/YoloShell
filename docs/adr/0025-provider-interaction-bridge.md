# ADR 0025: Provider interaction bridge

## Status

Accepted on 2026-11-01. Extends the public `Interaction`/`Respond` schema that
already existed but had no production observation or effect path.

## Context

The public agent protocol already carries `Interaction` (approval, choice,
text), `InteractionReference`, `AgentAction::Respond`, capability and cursor
validation. The backend nevertheless rejected every response as Unsupported,
the projection always set `interaction = None`, and provider hooks dropped the
content of permission notifications. Attention was therefore visible only as
an activity label; no client could answer it.

Hooks also already distinguish `PermissionRequest`, `Elicitation` and
notification attention states. What was missing was a bounded, provider-native
way to carry one pending prompt through observation and return a decision.

## Decision

### Public interaction

`Interaction` gains a bounded `prompt` (4 KiB) for the tool or question that is
waiting. It is additive; snapshots that omit it remain valid. The public agent
schema advances to 4 with 3 accepted for persisted projection migration.

### Observation

`NativeControlObservation` carries one optional `NativeInteraction`: a stable
UUID, kind, bounded choices and prompt. The private control schema advances to
5. The Worker owns at most one pending interaction per PTY, keyed by the exact
terminal, runtime epoch and agent process identity. The Hub projects it only
for a coherent, current, reachable provider-hook observation, sets
`respond_interaction = Available`, and reports approval activity as
`WaitingForApproval`. Stored idle observations are never reused while an
interaction is pending.

### Effect

A public `Respond` is admitted through the same writer, witness, receipt and
uncertainty path as interrupt and prompt. A new private `respond_native_agent`
request resolves the exact pending interaction under the Worker's controller,
lease and witness checks. The receipt records Delivered or NotDelivered; a
stale, replaced or already-answered interaction is never delivered.

### Claude bridge

Claude installs a `PermissionRequest` hook with a 600-second provider timeout.
The hook bridge summarizes the tool name and bounded tool input, sends a
synchronous `InteractionRequested` event over the existing Claude hook socket,
and waits up to 570 seconds for a decision. The event is explicitly rejected by
the durable lifecycle reducer and never enters the outbox; the Worker keeps the
hook connection open while a typed response is pending. A decision is printed
as Claude's native `hookSpecificOutput.decision`; no decision lets Claude's
normal permission flow proceed. Disconnect, replacement or timeout clears the
pending interaction.

Providers without a native, durable decision channel remain
`respond_interaction = Unsupported`. Subagent permission prompts, Elicitation
forms and text interactions are not claimed by this slice. Terminal bytes and
screen text are never used to infer or answer a prompt.

## Consequences

- Approval becomes a typed observation and effect, not a screen-derived state.
- The Hub still holds no approval state: the Worker owns the pending request
  and the Hook connection owns its lifetime.
- A client can render the question and choices from the snapshot; the response
  carries the exact interaction reference and expected cursor.
- Cross-Hub responses use the existing AgentControl + TerminalWrite
  delegation; no new federation right is introduced.
- Attention can wait on the new capability or interaction without a separate
  event stream.
