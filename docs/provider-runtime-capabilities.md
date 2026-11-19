# Provider runtime capabilities

This document is the operational companion to
[ADR 0013](adr/0013-provider-runtime-capability-ports.md). It defines the tier
floor, the mainstream adaptation matrix and the onboarding contract. A tier is
a guaranteed floor, not a feature count: a provider may offer interrupt without
conversation history, for example.

## Tiers

| Tier | Guarantee | Required evidence |
| --- | --- | --- |
| L0 | Terminal | PTY, attach, input/output, generic presentation; unknown kinds are usable |
| L1 | Identity | exact process detection, binding, cwd/session evidence, exact native resume; unknown stays unresumed |
| L2 | Lifecycle | authenticated hook/journal authority drives lifecycle/activity; install/trust/replay/revocation fenced |
| L3 | History | bounded native transcript read with identity, pagination and cross-Hub delivery |
| L4 | Control | prompt readiness, native submit, interrupt; witness/writer/journal/revalidation |
| L5 | Recovery | exact resume after Worker/Host/reboot, cross-Hub sync, multi-client, conformance + live acceptance |

### Readiness rules

- Stop is not a uniform idle signal. A profile declares `StopIgnored`,
  `StopCandidateWithTurnProof` or `StopFinal`.
- Claude and Qwen ignore Stop. Qwen's `Notification idle_prompt` is the
  prompt-ready fact after completion and interrupt.
- Codex Stop remains a candidate until its structured turn journal proves the
  turn ended.
- PTY bytes, elapsed time and free-form transcript text never establish
  lifecycle or readiness.

## Mainstream adaptation matrix

| Agent | Protocol family | Current surface | Floor | Main gap |
| --- | --- | --- | --- | --- |
| Claude Code | Claude hooks | hooks, JSONL, exact resume | L5 (reference) | register in catalog/ports; keep authority internals |
| Codex | Codex hooks | hooks, rollout turn journal | L5 (reference) | register in catalog/ports; keep occurrence-witness authority |
| **Qwen Code** | Claude-style hooks | hooks, append-only JSONL, `qwen -r <id>`, `QWEN_HOME` | L5 | landed and live-tested |
| Factory Droid | Claude-style hooks | hooks, JSONL, `droid --session-id` | L4–L5 | profile after Qwen; L5 after live proof |
| Cursor CLI | Claude-compatible hooks | hooks, JSONL, interactive-only turn hooks | L2–L4 | exact-ID resume/headless gaps; pin version |
| **Gemini CLI** | Gemini hooks | hooks, JSONL, `gemini --resume <uuid>` | L5 | landed and live-tested |
| **Grok Build** | Grok hooks | Claude-shaped hooks, ACP `updates.jsonl`, `grok -r <uuid>` | L5 | landed and live-tested |
| GitHub Copilot CLI | Copilot camelCase hooks | hooks, SQLite session state | L3–L4 | SQLite/session reader; L5 after control proof |
| OpenCode | Server API/SSE | SQLite, exact session IDs | L3–L5 with server API | no complete native hooks; PTY-only path is L2 |
| Aider | None | Markdown history | L1–L2 partial | no exact session identity or readiness |
| Goose | SQLite/partial hooks | SQLite sessions | L1–L2 partial | limited lifecycle; SQLite reader |

Protocol families are integration shapes, not vendor lineage. Qwen Code is a
Gemini CLI fork but adopted Claude-style hooks; Gemini CLI itself remains a
different hook/transcript family.

## Onboarding contract

A provider supplies:

1. `ProviderDescriptor` with manifest ID, agent kind, install token,
   conversation token, target tier, capabilities, presentation, resume,
   `AuthorityKind`, lifecycle semantics and projection policy.
2. `AgentAdapter` for process match, evidence layout/discovery, binding
   validation and exact resume.
3. Optional hook-ingress profile for the hook-generation or
   occurrence-witness family. Installation is data-driven through the
   provider package's `HookInstallSpec`.
4. Optional `ConversationSource`/`TranscriptParser`.
5. Optional `TurnEndProbe`, control/authority and startup-receipt ports.
6. Prompt-submit and interrupt delivery policy: atomic or paste-then-settle
   Enter, and `SIGINT` or a terminal interrupt key. The Worker owns both
   mechanisms; the provider declares only the encoding its composer accepts.
7. Captured payload/transcript fixtures and the conformance suite.

All provider-specific code lives under
`core/crates/agent/providers/<provider>`: the adapter crate at the directory
root, `hooks/`, `parser/` and `provider/`. Mechanism packages under
`agent/{contract,runtime,system}` and `conversation/` contain no provider
leaf. Each provider's onboarding policy lives in its `provider/` package. The
identity target `yolo_<p>_identity` carries the provider ID, kind, tokens,
display name and manifest definition and depends only on the agent and
provider contracts, so client discovery never links adapters. The policy
target `yolo_<p>_provider` composes the descriptor, adapter, environment
keys, projection policy, hook codec, install profile, conversation source and
optional turn-end probe into one `ProviderRuntime`.

Exactly two compiled-in lists name providers:
`core/crates/agent/system/providers/src/builtin.rs` names every policy target
(`yolo_<p>_provider::runtime()`), and
`core/crates/plugin/provider/catalog/src/lib.rs` names every identity target
(`yolo_<p>_identity::definition()`) and builds manifests generically. The
Worker derives its ingress codecs, the Host CLI its install profiles and
projection policies, and the Hub its provider metadata from those lists;
`build_system/bazel/rust/providers.bzl` derives the provider build lists. The
`provider_policy` repository rule rejects provider identity literals outside
provider packages and those two composition lists. A cross-registry test fails
when a descriptor's install token, conversation token, codec or manifest has
no matching policy. Hub projection, the conversation reader, Host startup
receipts, store authority selection and the Worker control source consume that
data or the ports; they do not match provider names. Unknown providers fail
closed at L0/L1.

Providers with a generic hook codec share one `internal host provider-hook
--provider <token>` callback and the durable ingress engine. Install profiles
record the managed command forms they own, so upgrading a provider's callback
rewrites stale managed handlers while preserving user hooks. Qwen Code now
uses this shared engine; its earlier forked ingress was removed.

## Qwen Code pilot facts (0.26.2, 2026-10-28)

Captured against an isolated `QWEN_HOME` and a deterministic local
OpenAI-compatible model server:

- `qwen -r <session-id>` preserves the exact session ID; `SessionStart` reports
  `source=resume`.
- Normal completion: `SessionStart`, `UserPromptSubmit`, `MessageDisplay
  is_final=true`, `Stop`, then `Notification notification_type=idle_prompt`.
- Interrupt (Escape during a slow turn): `UserPromptSubmit`, non-final
  `MessageDisplay`, then `Notification idle_prompt`; no `Stop`.
- `/clear`: `SessionEnd reason=clear` followed by a new session and
  `SessionStart source=clear`.
- `/exit`: `SessionEnd reason=prompt_input_exit`.
- Tool use: `PreToolUse`, `PostToolUse`/`PostToolUseFailure`, `PostToolBatch`;
  transcript records include `functionCall` and `tool_result` with
  `functionResponse`/`toolCallResult`.
- Transcript is append-only JSONL under
  `<QWEN_HOME>/projects/<encoded-project>/chats/<session-id>.jsonl`; records use
  `type=user|assistant|tool_result`, `message.parts[]`, and many ignored
  `type=system` telemetry records.
- Hook payloads carry `session_id`, `transcript_path`, `cwd`,
  `hook_event_name`, `timestamp`, `permission_mode` and event-specific fields.

The full evidence is recorded in
the Qwen provider pilot and Qwen Code L5 live acceptance.

## Gemini CLI facts (0.66.0, 2026-10-29)

Captured against a deterministic local Gemini API server:

- Hooks live in `$GEMINI_CLI_HOME/.gemini/settings.json`; the installed
  profile wires `SessionStart`, `BeforeAgent`, `AfterAgent`, `BeforeTool`,
  `AfterTool`, `Notification` and `SessionEnd`. `AfterAgent` is the turn
  boundary and also fires after an interrupt.
- The CLI relaunches itself with `--max-old-space-size=<n>` before the
  entrypoint, and a global npm install runs a bin shim; detection scans the
  leading options and accepts the entrypoint by name.
- `gemini --resume <uuid>` resumes exactly. On resume the CLI first opens a
  transient transcript, reports it to `SessionStart`, and rewrites the durable
  transcript with `$patch`/`$rewindTo` records.
- The keypress reader converts an Enter arriving within 30 ms of inserted text
  into a newline (`bufferFastReturn`), so native submit settles the paste
  before writing the submit key.
- The CLI installs no `SIGINT` handler; it reads its cancel key from the
  terminal. A native interrupt writes Esc instead of signaling the process.
- Transcripts are `<data home>/tmp/<projectHash>/chats/session-*.jsonl` with
  `user`/`gemini`/`info`/`warning`/`error`/`tool_result` records, `$set`
  metadata patches and duplicate empty `gemini` placeholders before tool
  calls; the parser skips placeholders and treats partial rewrites as
  unsupported content.

The full evidence is recorded in
the Gemini CLI L5 live acceptance.

## Grok Build facts (1.0.70, 2026-10-29)

Captured against a deterministic local OpenAI-compatible model server:

- Hooks are Claude-shaped JSON files in `$GROK_HOME/hooks/*.json` (or
  `~/.grok/hooks`); all files are merged, so the installed profile is one
  `yolo.json` with `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `Stop`,
  `StopCancelled`, `StopFailure`, tool, permission, notification and
  compaction events. Hook commands run through a shell child of the agent.
- Payloads carry both camelCase and snake_case duplicate keys
  (`hookEventName`/`hook_event_name`, `sessionId`/`session_id`,
  `transcriptPath`/`transcript_path`); the codec reads either.
- `SessionStart.source` is `new` for a fresh session and `load` for a resumed
  one; the resumed start carries the transcript path, the fresh start does
  not, so the codec derives `<data>/sessions/<percent-encoded cwd>/<id>/
  updates.jsonl` after admission identifies the live agent.
- A cancelled turn reports `stop_cancelled` followed by
  `turn_completed stop_reason=cancelled`; the codec maps both `stop` (once
  `reason` is `end_turn`) and `stop_cancelled` to the turn boundary. A
  `stop` with `reason=shutdown` follows `SessionEnd` on exit and is ignored.
- The TUI reads its cancel key from the terminal and installs no `SIGINT`
  handler: a native interrupt must write `Ctrl+C` (`0x03`); `SIGINT` kills the
  CLI. The runtime declares `InterruptDelivery::Key(b"\x03")`.
- Sessions live at `<data home>/sessions/<percent-encoded-cwd>/<uuid>/`;
  `updates.jsonl` is the authoritative ACP update stream
  (`user_message_chunk`, `agent_message_chunk`, `agent_thought_chunk`,
  `tool_call`, `tool_call_update`, `turn_completed`, `hook_execution`).
  `grok -r <uuid>` resumes exactly; `-s/--session-id` only names new sessions.
- The launcher on `PATH` is a `grok` symlink; the running binary is
  `~/.grok/downloads/grok-linux-aarch64`, so detection accepts a `grok-`
  executable prefix in addition to the launcher name.
- On resume the CLI authenticates from its own config/credential store, not
  from the launching shell's environment. The resume plan sets `GROK_HOME` and
  `grok -r <uuid>` only; a model configured by a one-off environment variable
  must be moved into the CLI config for resumed turns to authenticate.

The full evidence is recorded in
the Grok Build L5 live acceptance.
