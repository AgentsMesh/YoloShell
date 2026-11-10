# Provider hook research

Reviewed on 2026-08-28 for the Yolo 0.1 Alpha contract.

Official sources:

- Codex hooks: <https://learn.chatgpt.com/docs/hooks>
- Codex advanced configuration: <https://learn.chatgpt.com/docs/config-file/config-advanced>
- Claude Code hooks: <https://code.claude.com/docs/en/hooks>
- Claude Code CLI: <https://code.claude.com/docs/en/cli-reference>

The online documentation describes the currently published interface, while
Alpha admits only its pinned binaries. A lifecycle fact is supported only when
both the official contract and a real-binary acceptance test confirm it.

## Conclusion

PTY output is a rendering protocol, not an agent lifecycle protocol. Transcript
files are useful for fenced session identity and native resume, but Codex calls
its transcript format unstable and Claude writes transcripts asynchronously.
Neither may publish durable lifecycle state.

Yolo therefore uses four evidence levels, in this order:

1. An invocation-scoped, authenticated provider hook supplies lifecycle facts.
2. Exact process identity supplies liveness and final exit proof.
3. Fenced provider files supply session identity, cwd, and resume evidence.
4. PTY bytes supply presentation-only hints that disappear after restart.

A process scan may label a descendant as probable Codex or Claude Code. It does
not make that process managed. A trusted `SessionStart` handshake is the first
event allowed to create a resumable managed binding.

## Capability matrix

| Lifecycle fact | Codex | Claude Code | Yolo interpretation |
| --- | --- | --- | --- |
| Session start | Yes | Yes | Bind provider session after identity checks |
| User prompt | Yes | Yes | Turn is running |
| Tool start/success | Yes | Yes | Running; track tools by call ID |
| Tool failure | No dedicated event | `PostToolUseFailure` | Typed failure candidate |
| Approval request | Yes | Yes | Attention candidate, not proof a prompt remains open |
| Automatic denial | No dedicated event | `PermissionDenied` | Failure/attention fact |
| Compaction | Pre and post | Pre and post | Retain and restore prior turn state |
| Subagent lifecycle | Start and stop | Start and stop | Child state only; never replace root state |
| Turn stop | Yes | Yes | Stop candidate, not final idle proof |
| Turn failure | No dedicated event | `StopFailure` | Typed failed turn |
| Notification | No lifecycle hook | Yes | Corroborate approval/input attention |
| Cwd change | No dedicated event | Yes | Update cwd through a fenced path transaction |
| MCP elicitation | No dedicated event | Start and result | Waiting-for-input candidate and resolution |
| Session end | Yes | Yes | Logical boundary candidate, never process exit |

Claude Code also exposes setup, prompt expansion, tool batches, task events,
team events, config/instruction/file/directory changes, worktree events, and
message display. These are useful telemetry or policy events but are not all
required to derive the Alpha root-agent state.

## Codex contract

The documented Codex event set is `SessionStart`, `SessionEnd`,
`UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`,
`PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, and `Stop`.
There is no documented `Interrupt` hook in the current release contract.

`SessionStart.source` is exactly `startup`, `resume`, `clear`, or `compact`.
It does not include `fork`. `SessionEnd.reason` is currently always `other`.
Yolo rejects other values instead of silently translating them.

`Stop.stop_hook_active` says whether a Stop hook has already continued the
turn. It does not prove that every concurrent Stop hook accepted completion.
Likewise, `PermissionRequest` runs before all matching hook decisions are
combined, so another hook can auto-allow or deny it. Both events are candidates
until a later provider fact resolves them.

Yolo retains the authenticated process and recovery binding while either Codex
candidate is unresolved. A Stop candidate stays `AgentState::Active` and the
public Hub reports `Lifecycle::Running` with `Activity::Unknown`. A permission
candidate projects to `WaitingForPermission` and is shown as
`Activity::WaitingForApproval` so an approval prompt in the terminal is visible;
it still grants no prompt input and no typed interaction, and a later tool,
stop or session-end event clears it. Another matching hook may resolve the
request differently, so the activity is an attention candidate, not
authorization. Both initial and reentrant Stop callbacks remain candidates. The pinned Codex contract currently supplies no final turn-completion
proof, so after Stop, Chat SubmitPrompt stays unavailable until a later supported
hook establishes a confirmed input state. Users can still type into the ordinary
PTY. Foreground membership, bracketed-paste mode, elapsed time and transcript
content do not settle the hook decision. Subsequent prompt/tool hooks establish
new activity; they do not retroactively prove an earlier Stop completed.

The shared reducer distinguishes `PermissionCandidate` from a confirmed
`Permission` notification. Claude's existing native bridge keeps its own narrower
event contract: Stop is ignored, and accepted input-needed notifications can
confirm WaitingForInput. It does not gain Codex candidate semantics or a new
permission-control capability from the shared enum.

All matching hook sources are additive. Multiple matching command hooks for an
event start concurrently. Async hooks may complete out of order; unfinished
async hooks are cancelled at session end. Reducer order must therefore come
from Yolo's durable sequence allocator, never helper completion time.

Non-managed hooks require review and trust of the exact definition hash.
Project hooks also require project trust. Yolo must never pass
`--dangerously-bypass-hook-trust`, because that bypasses trust for unrelated
hooks too. Before the trusted `SessionStart` arrives, the process remains
visible but unmanaged and `Unknown`.

Codex permits one-off TOML config overrides through `-c`/`--config`. Yolo can
inject stable hook definitions in that invocation layer without editing
`~/.codex/config.toml`. The command definition stays stable so trust is not
invalidated by per-invocation IDs; credentials travel through the managed
launcher environment instead.

Most Codex hooks default to 600 seconds. `SessionEnd` defaults to one second and
is capped at three seconds. It may run for archive/delete, normal close, or a
conversation idle for 30 minutes with no connected client. Its bridge may only
commit a local durable enqueue inside that budget; it must not wait for Worker
or SSH delivery.

## Claude Code contract

Claude Code exposes a broader event surface. State-relevant additions include
`PermissionDenied`, `PostToolUseFailure`, `PostToolBatch`, `Notification`,
`StopFailure`, `CwdChanged`, `Elicitation`, and `ElicitationResult`.

`SessionStart.source` supports `startup`, `resume`, `clear`, `compact`, and
`fork`. `SessionEnd.reason` supports `clear`, `resume`, `logout`,
`prompt_input_exit`, and `other`. Clear and interactive resume are logical
boundaries inside a live process, so they enter recovering state and await a
matching next `SessionStart`.

Claude's `SessionEnd` default budget is 1.5 seconds. A settings hook may raise
the shared budget up to 60 seconds. It has no decision control. The same local
enqueue-only rule used for Codex applies.

Parallel Claude tool calls produce concurrent per-tool hooks, followed by one
`PostToolBatch`. Async command hooks have no enforced timeout after launch;
non-interactive teardown kills unfinished async hooks. Durable authority must
come from synchronous local enqueue, not async completion.

Yolo supplies an invocation-scoped `--settings` artifact. It must merge with
the user's effective settings rather than rewrite user or project files. A
disabled-hook policy, safe/bare launch mode, absolute executable bypass, or
invalid artifact keeps the process unmanaged and `Unknown`.

## Normalized reducer

Provider adapters decode exact wire schemas before entering a shared typed
reducer. Unknown events, unknown enum values, missing required fields, provider
version mismatches, sequence gaps, duplicate conflicts, stale generations, and
identity mismatches fail closed.

The reducer keeps root and subagent state separate. It retains active state
across compaction, tracks tool-call IDs as a set, and treats approval, stop, and
session-end events as candidates where provider concurrency leaves ambiguity.
No timer converts a candidate into an authoritative state.

`Exited` requires both a final session-end candidate and proof that the exact
PID plus process start time is gone. A reused PID, an inaccessible process
observation, or exit without the final hook publishes `Unknown`, not `Exited`.

## Delivery and recovery

Each invocation owns a hook token, invocation ID, generation, provider version,
exact process identity, Unix socket, and durable outbox. The helper durably
enqueues before delivery. ACK includes schema, event ID, and sequence. Duplicate
delivery is idempotent; a conflicting duplicate revokes authority.

ACK, reducer snapshot, public agent binding, and outbox compaction need one
crash-consistent commit boundary. Until a durable reducer snapshot covers a
sequence, the helper must retain every acknowledged event needed for replay.
A bounded in-memory history alone cannot recover a Worker crash.

The socket admits the local Worker user, but UID alone is not origin proof.
Admission also binds the hook capability, exact agent PID/start time, provider
session, generation, and helper lineage. Credentials inherited by an arbitrary
same-user process are insufficient.

For remote sessions, bridge, outbox, socket, and Worker all live on the remote
host. SSH is only a transport for attachment and management channels. Losing
SSH neither kills the agent nor loses hook events; reconnect drains the remote
outbox before publishing current state.

## Acceptance tests

The pinned real CLIs must cover startup, resume, clear, compaction, concurrent
tools, approval, stop continuation, subagents, final exit, hook disable/trust
failure, and native resume. Claude adds fork, tool failure, notifications,
elicitation, cwd changes, and stop failure.

Adversarial tests must cover forged tokens, wrong session/provider/version,
helper reparenting, PID reuse, concurrent enqueue, duplicate conflict, sequence
gap, Worker death before ACK, crash between every durability phase, truncated
outbox/snapshot, missing final hook, and SSH loss during delivery. E2E launches
the TUI from an ordinary shell and must not activate Yolo or take window focus.
