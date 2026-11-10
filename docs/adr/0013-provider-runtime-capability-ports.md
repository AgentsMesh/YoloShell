# ADR 0013: Provider runtime, capability ports and L0–L5

## Status

Accepted on 2026-10-28 for the Qwen Code pilot. Supersedes the implicit
"two adapters plus per-consumer provider matches" structure. It does not change
the hook authority rules in ADR 0010/0011 or the terminal custody model.

## Context

Yolo has a provider-agnostic Terminal/PTY core, Hub snapshot projection,
conversation protocol, native-control journal and a useful `AgentAdapter` seam.
Provider decisions are nevertheless spread across closed enums and kind matches:

- `BuiltinProvider`, the provider catalog, `AgentAdapter`, and the unused
  `AgentProvider` registry describe providers differently.
- Store, Host, Worker and process scanning construct `AgentRegistry::default()`;
  the catalog hardcodes Codex/Claude adapters.
- Hook installation, conversation provider/parser selection, native-control
  validation/source dispatch, Host startup receipts and Hub projection each
  carry their own Codex/Claude checks.
- Adding a provider therefore means editing generic consumers or copying
  detection, evidence, hook delivery, authority, transcript and recovery code.

The problematic invariant is not "there are two providers"; it is that the
generic layers ask "which brand is this?" instead of "which capability is
implemented and verified?".

## Decision

### ProviderRuntime and narrow ports

Each compiled-in provider supplies a `ProviderDescriptor` and a bundle of
optional, object-safe ports:

- `AgentAdapter`: process match, evidence, binding validation, exact resume.
- `HookInstaller` / `HookCodec`: install document profile and wire codec.
- `ConversationSource`: fenced evidence plus `TranscriptParser`.
- `TurnEndProbe`: a provider-owned structured turn-ending record.
- `NativeAuthorityPort` / `StartupReceiptPort`: durable recovery authority.
- `ControlSourcePort`: provider witness/control source; effects stay shared.
- `ProjectionPolicy`: whether lifecycle is hook-authoritative and how source
  maps. Hub projection no longer matches `AgentKind`.

`ProviderRuntime` is a bundle, not a god-trait. Detection, hook reduction,
transcript scanning, the control journal and all effects remain shared.
Provider-specific Stop semantics, admission/outbox schemas, evidence layouts and
install policy stay provider-owned.

### Hook families

Two families cover the observed protocols:

1. **Hook-generation family** (Qwen Code, Factory Droid, future Claude-family
   providers): a provider-neutral durable session event authority with
   `SessionStart`, `Running`/`WaitingForInput`/`WaitingForPermission`,
   `CwdChanged` and `SessionEnd`, plus a bounded replay window, evidence
   identities, revocation and startup-receipt verification.
2. **Occurrence-witness family** (Codex): unchanged outbox, Stop candidate,
   turn-journal proof and authority schema.

Claude's native generation and managed-shim authorities remain behind the same
ports until a separate equivalence migration. No family is forced on a provider
whose semantics differ.

### Capability and tier

`ProviderCapabilities` declares detection, exact resume, lifecycle, conversation,
submit, interrupt and recovery. A tier is a **guaranteed floor** derived from
the ports actually implemented and the conformance suite; capacity may exceed
the floor. Tiers are L0 terminal, L1 identity/resume, L2 lifecycle, L3 history,
L4 control, L5 recovery + cross-Hub acceptance. A manifest cannot grant a tier.

`ProviderRuntime` construction validates the declared floor against present
ports. A built-in false claim fails startup; an unknown kind has no runtime and
fails closed at L0.

### Registration and composition

- `agent/system/providers` is the single compiled-in composition root.
- Process entry points construct one `Arc<ProviderRuntime>` and inject it into
  store, detection, launch, process scanning, conversation, control and Hub
  projection. Generic consumers must not construct a runtime internally.
- Provider IDs, agent kinds and install/conversation tokens are separate
  namespaces. Existing values (`codex`, `claude_code`, install `claude`) remain
  byte-identical. New kinds use `AgentKind::Other`; Qwen is `qwen_code`, install
  token `qwen`, conversation token `qwen_code`.
- No dynamic library loading and no manifest-supplied executable code. A
  third-party manifest is discovery metadata only until its code is compiled and
  signed with the runtime.

### Stop and readiness

Stop semantics are data, not a default:

- `StopIgnored`: no lifecycle change (Claude, Qwen).
- `StopCandidateWithTurnProof`: candidate until a verified provider record
  (Codex).
- `StopFinal`: only when a pinned provider contract proves it.

Qwen emits `Notification idle_prompt` after normal completion and interrupt;
that is the prompt-ready fact. Qwen Stop is ignored. PTY bytes, timers and
free-form transcript text never establish readiness.

### Failure policy

Absent ports or unknown providers fail closed: no binding promotion, no resume,
no conversation, no control, and `Unsupported`/`Unknown` errors rather than a
permissive default. Hook tokens and manifest claims are lookup keys, never
authority.

## Consequences

- Onboarding a provider requires one directory under
  `core/crates/agent/providers` (adapter, hooks, parser, identity and policy
  targets), captured fixtures, one line in the runtime composition and one
  line in the manifest catalog.
- The Hub/TUI/store/worker no longer need a new provider branch.
- Worker/Host injection is a larger one-time refactor; package and line budgets
  require new child packages rather than growing the 42k-line Worker.
- Codex/Claude behavior remains the acceptance baseline; their authority
  internals are wrapped, not rewritten.
- The first L5 pilot is Qwen Code 0.26.2. Gemini, Cursor, Copilot, OpenCode,
  Aider and Goose remain at documented lower tiers until their specific gaps
  (rewritten transcripts, exact resume, SQLite or server control) are closed.

## Sources

- ADR 0010: provider hooks own agent lifecycle state.
- ADR 0011: native agent launch and hook-first registration.
- `docs/provider-hooks-research.md` and `docs/conversation-reader-contract.md`.
- Qwen Code 0.26.2 hook/notification/transcript probes on 2026-10-28.
