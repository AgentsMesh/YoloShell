# ADR 0014: Control effect contract for native prompts and interrupts

## Status

Accepted on 2026-10-30 for the native control path. It refines the
prompt-submit/interrupt item of ADR 0013; the hook authority rules of
ADR 0010/0011 and the terminal custody/lease model are unchanged.

## Context

The `yolo agent` live sweeps produced two incidents of the same class:

- Codex `interrupt` returned `Delivered` while a process SIGINT terminated
  the Codex CLI.
- Qwen `interrupt` returned `Delivered` while the provider disabled
  bracketed paste; the Worker's readiness proof then rejected every later
  prompt as `submit_prompt: input_unavailable`, permanently.

Both receipts described a transport write, while the caller read them as an
effect. Readiness was derived from mutable terminal state (foreground process
group plus bracketed paste), and turn completion was inferred by timers and
transcript diffs. The five providers differ exactly where those shortcuts
break:

| Provider | Submit | Interrupt | Readiness fact | Turn-end fact |
| --- | --- | --- | --- | --- |
| Claude Code | atomic paste+Enter | Signal (unverified) | `idle_prompt` notification | ClaudeGeneration authority |
| Codex | atomic | Esc key | turn journal + turn proof | `task_complete`/`turn_aborted` |
| Qwen | atomic | Esc key | `idle_prompt` → TurnEnded | `turn_ended` notification |
| Gemini | paste, settle, Enter | Esc key | `idle_prompt` notification | generic hook generation |
| Grok | atomic | Ctrl-C key | `idle_prompt` notification | generic hook generation |

The common failure is not a missing provider flag. It is that four different
truths were conflated: a transport write, a provider's readiness, a
provider's turn end, and a public receipt.

## Decision

Control has four layers. Each layer has one owner and one truth.

1. **Transport** (mechanism, internal): PTY bytes, paste framing and the
   submit/interrupt encoding. Its truth is only written/unknown. It is never
   a public receipt.
2. **Interaction** (provider policy): how this TUI is prompted or interrupted
   and which terminal-mode preconditions the attempt requires. Its truth is
   accepted, rejected or unverified. Delivery encodings stay provider data;
   the unsafe default is gone.
3. **Observation** (provider authority): readiness and turn end are provider
   facts with provenance and invalidation. A provider is ready when its
   binding says `waiting_for_input`, or when a verified turn-ending record
   proves the turn ended. Terminal mode is a per-operation precondition, not
   a capability.
4. **Effect** (public protocol): receipts describe effects using the existing
   ladder: `Delivered` (transport), `Accepted` (provider accepted the input;
   a turn started), `Confirmed` (the turn ended), `Unknown` (effect not
   observed; never replayed automatically).

### Rules

1. No public receipt may claim an effect that was not observed. The current
   local Worker stops at the transport level and must return `Accepted` only
   after the provider's lifecycle reports the new turn.
2. Capabilities are derived-with-invalidation. Readiness is recomputed from
   the provider binding on every projection pass; a provider state change
   invalidates the previous capability immediately. A terminal-mode flag may
   not become a sticky capability.
3. Interrupt is an explicit provider declaration. The default is
   `Unsupported`, never `Signal`; a new provider must prove its delivery.
4. Readiness facts come from the provider: `waiting_for_input` (notification
   family) or a verified turn-ending record (turn-journal family). PTY bytes,
   timers and free-form transcript text never establish readiness.
5. `ask` waits for the provider turn-end fact and the assistant entry of that
   turn. The 5-second settle window is a transcript-read retry, not a
   completion guess.
6. Conformance asserts effects, not parameters: submit starts a turn;
   interrupt cancels and leaves the provider alive and ready; the turn-end
   fact matches the assistant entry.

## Migration

Landed with this ADR:

- The Worker derives `prompt_input_ready` from the binding state or a
  verified turn-end record; the PTY/mode proof remains only as the submit
  transport precondition.
- `InterruptDelivery` defaults to `Unsupported`; every built-in provider
  declares its delivery explicitly.
- Qwen and Codex deliver their provider key (Esc) instead of SIGINT.
- `NativeControlOutcome` carries `Accepted`/`Confirmed`. After a successful
  write the Worker waits for the next provider binding publication (turn
  started for a prompt, turn ended for an interrupt) and otherwise keeps the
  transport outcome. The control journal still records the transport
  completion; a retry returns the transport outcome.

Next milestones, in order:

1. Publish the provider turn-end fact as a cursor for the notification family
   and make `ask` wait on it plus the turn's assistant entry.
2. Replace parameter-value tests with scripted fake-TUI conformance fixtures
   per provider family.
3. Live-prove Claude's `Signal` delivery; until then it is a declared but
   unverified policy.

## Consequences

- A provider can be ready while its transport is not; the submit attempt then
  fails with an explicit transport rejection instead of a permanent
  capability lockout.
- `Delivered` no longer implies the provider acted. Callers that need the
  effect use `Accepted`/`Confirmed` or observe the snapshot.
- Adding a provider requires a delivery proof and an effect conformance
  fixture, not a default.
- The control journal keeps its transport completion; effect-level outcomes
  are derived per request and are not replayed.
