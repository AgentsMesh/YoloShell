# ADR 0010: Provider hooks own agent lifecycle state

## Status

Accepted for Yolo 0.1 Alpha. This decision supersedes transcript-derived agent
state in the generic adapter and the state portion of ADR 0007. ADR 0007 still
governs Claude session-generation identity and transcript fencing.

## Context

Yolo owns a terminal and its PTY, but the bytes rendered by a TUI are not a
protocol. Prompts, spinners, colors, localization, redraws, alternate-screen
output, and provider releases can all change without notice. A transcript is
also not a lifecycle protocol: Codex documents its transcript format as
unstable, and Claude writes its transcript asynchronously.

The Alpha versions expose lifecycle hooks that carry the provider session ID,
cwd, event name, and turn or prompt context as structured JSON. Command hooks
receive that JSON on stdin. They are therefore the best provider-owned fact
source available while Yolo continues to manage the real interactive TUI.

The supported versions are pinned because hook events and fields evolve:

| Provider | Alpha pin | Required state events |
| --- | --- | --- |
| Codex | `codex-cli 0.150.1` | `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, and `Stop` |
| Claude Code | `2.1.241 (Claude Code)` | Codex-equivalent events plus `PermissionDenied`, `PostToolUseFailure`, `PostToolBatch`, `Notification`, `StopFailure`, and `CwdChanged` |

Codex 0.150.1 is the pinned acceptance binary. Its documented release contract
does not contain an `Interrupt` event, so an interrupted turn remains `Unknown`
until a later pinned release exposes and documents a replacement fact. Its
`SessionStart` sources are `startup`, `resume`, `clear`, and `compact`; its
current `SessionEnd` reason is only `other`. Yolo rejects absent or unknown
source and reason values instead of fabricating a default. Claude 2.1.241
contains the events above; later-version fields remain unavailable until a new
acceptance matrix is signed.

Official sources reviewed on 2026-08-28:

- Codex hooks: <https://learn.chatgpt.com/docs/hooks>
- Codex hook configuration: <https://learn.chatgpt.com/docs/config-file/config-advanced#hooks>
- Codex release history: <https://learn.chatgpt.com/docs/changelog>
- Claude hooks: <https://code.claude.com/docs/en/hooks>
- Claude CLI: <https://code.claude.com/docs/en/cli-reference>
- Claude release history: <https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md>

The detailed capability and failure-mode review is recorded in
[`docs/provider-hooks-research.md`](../provider-hooks-research.md).

## Decision

### Authority boundary

Yolo uses this strict ordering:

1. An authenticated, invocation-scoped provider hook may change durable agent
   lifecycle state.
2. Exact process identity and fenced provider evidence may corroborate a hook,
   establish liveness, and validate a provider session ID.
3. PTY bytes may produce a non-durable display hint only.
4. Transcript content may validate identity where a provider contract requires
   it, but may not infer lifecycle state.

Process scanning still recognizes that a shell descendant is probably Codex or
Claude. Recognition alone cannot publish a resumable binding, a provider
session ID, or a state other than `Unknown`.

### Provider contract

Each provider owns instrumentation, event decoding, authentication, reduction,
recovery, and resume construction behind one lifecycle-provider boundary. The
shared layer understands typed facts, never provider event strings or
transcript records.

Every admitted event is bound to the Yolo session, runtime epoch, Worker,
provider kind and version, invocation, exact agent process, hook capability,
provider session ID, and monotonically allocated event sequence. Session and
cwd events also retain fenced path/object identity when the provider supplies a
transcript path.

Provider event names, matcher values, version gates, and failure categories are
centralized typed constants or enums. Reducers match those enums exhaustively;
wire strings and timing values are not repeated in launch, ingress, or UI code.

### Canonical state reduction

The shared durable states are `Unknown`, `Starting`, `Running`,
`WaitingForApproval`, `WaitingForInput`, `Compacting`, `Recovering`, `Failed`,
and `Exited`. Provider reducers apply these facts:

| Hook fact | Canonical transition |
| --- | --- |
| managed process begins | `Starting` |
| accepted `SessionStart` | bind the provider session; provider-supported sources become `WaitingForInput` |
| `UserPromptSubmit`, `PreToolUse`, successful tool continuation | `Running` |
| `PermissionRequest` | record an approval attention fact; do not claim approval is still pending |
| Claude `Notification: permission_prompt` | corroborate `WaitingForApproval`; never override a newer event |
| Claude elicitation or `agent_needs_input` notification | `WaitingForInput` when it belongs to the managed main agent |
| `PreCompact` | `Compacting`, retaining the prior active state |
| `PostCompact` or `SessionStart: compact` | restore the retained active state |
| `Stop` | record a stop candidate; publish `WaitingForInput` only after a provider-specific finalization proof |
| Claude `StopFailure` | `Failed` with a typed provider error category |
| Claude `SessionEnd: clear/resume` | `Recovering` until the next fenced `SessionStart` |
| final `SessionEnd` plus exact process exit | `Exited` |
| exact process exit without a final hook | `Unknown`/crashed; never completed |

Subagent events never replace the main-agent state. A provider may expose
subagent activity separately in the UI.

### Delivery and ordering

Yolo installs one stable Rust hook bridge command per provider. The bridge
durably enqueues the event before attempting Worker delivery. A locked outbox
allocates sequence numbers, so provider-side concurrent hook processes cannot
publish out of order. The Worker accepts an event exactly once by invocation,
sequence, event ID, and generation.

Replays are idempotent. Stale generations are rejected. A sequence gap,
credential mismatch, replaced process identity, invalid session transition, or
outbox corruption revokes state authority and persists `Unknown`. Only a fresh,
authenticated `SessionStart` may establish a new generation after revocation.

Authoritative hooks are synchronous through durable enqueue. Network or Worker
delivery may fail after enqueue because replay owns completion. Provider async
hooks are not the sole authority: Codex may cancel them at session end, Claude
kills them during documented non-interactive teardown, process loss can kill
either helper, and both providers can execute matching hooks concurrently.

`SessionEnd` alone never proves exit. Codex may emit it after an unopened chat
has been idle for 30 minutes, while Claude emits it for `/clear` and `/resume`
inside a live process. Claude also gives SessionEnd hooks a short default
budget, so the bridge only commits the local outbox and never waits for remote
delivery on that path.

### Installation and trust

The shell starts normally. Yolo's shell integration intercepts a user-typed
`codex` or `claude` invocation and prepares provider-specific instrumentation;
the panel creation UI never asks for an agent type.

Claude continues to receive an invocation-scoped `--settings` artifact. Codex
receives stable lifecycle definitions through its supported configuration
layer. Codex requires review and trust for non-managed hooks. Yolo exposes that
one-time trust requirement and waits for the authenticated `SessionStart`
handshake. It does not pass `--dangerously-bypass-hook-trust`, because that
would also bypass trust for unrelated active hooks.

An absolute provider executable, disabled hooks, Claude `--bare` or
`--safe-mode`, Codex `[features].hooks = false`, untrusted hook definitions, or
managed policy can bypass instrumentation. Such a process remains visible as
an unmanaged terminal process with state `Unknown`; it is never silently
promoted from PTY or transcript hints.

### Remote sessions

The hook bridge, Unix ingress socket, and durable outbox live beside the remote
Worker. They do not depend on the desktop SSH connection. SSH loss therefore
does not stop the agent or erase pending lifecycle events; a recovered remote
Worker drains the outbox before publishing current state.

## Verification

Provider contract tests use captured payloads from the exact Alpha binaries.
Adversarial tests must cover concurrent delivery, duplicate IDs, sequence gaps,
old generations, forged session IDs, helper reparenting, process replacement,
hook disable/trust failure, Worker death before acknowledgement, outbox crash
points, `/clear`, resume, compaction, permission waits, API failure, missing
`SessionEnd`, and SSH disconnect during an event.

No test may assert a durable state transition from PTY bytes or transcript
message kinds. E2E acceptance launches each pinned TUI from a shell, proves the
hook handshake, exercises running/approval/input/compaction/failure states, and
then proves native resume without taking window focus.

## Consequences

- Status becomes resilient to TUI rendering changes and localization.
- Codex needs a durable hook path equivalent to the existing Claude generation
  path before it can satisfy Alpha acceptance.
- Hook setup failure is visible and fail-closed instead of looking healthy.
- PTY parsing remains useful for presentation diagnostics, but loses all
  recovery, ownership, and durable-state authority.
