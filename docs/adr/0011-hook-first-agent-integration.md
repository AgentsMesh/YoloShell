# ADR 0011: Native Agent Launch And Hook-First Registration

## Status

Accepted on 2026-09-09. The native runtime path is implemented as of 2026-09-10;
integration and release acceptance are in progress.
This supersedes launcher-required registration and temporary-settings custody
as requirements for agent state integration. It does not supersede PTY custody,
process identity, durable event delivery or namespace isolation.

## Execution And Ownership

The execution path is Worker -> PTY -> user shell -> native agent.
Agent callbacks execute a separate hook bridge and report to the Worker.
Neither hook callback handling nor configuration generation depends on an agent
launcher, the desktop process, or an agent API client.

The shell retains its normal arguments, PATH, ZDOTDIR, startup files and functions.
Yolo exports session-scoped callback context but does not shadow `codex` or
`claude`, rewrite agent arguments, or require a Begin acknowledgement before the
native executable starts. The existing shell bootstrap gate remains: it owns the
PTY process boundary, not agent command interception.

## Integration Installation

An explicit, user-approved integration operation installs stable native hook
configuration. It preserves user handlers and unrelated preferences, is
idempotent, and writes with retained identity/CAS and a recoverable backup.
It supports the provider data home selected by that host's user. Remote hosts
install their own bridge and configuration; local settings are not copied over
SSH as assumed remote authority.

Configuration generation is a pure operation separate from filesystem mutation.
Generating JSON or merging an in-memory document does not install hooks or
authorize writes to the user's actual settings.

Codex hooks can be loaded from hooks.json, inline config or enabled plugins.
Non-managed hooks must be reviewed and trusted by the user. Yolo does not write
trust receipts, bypass hook trust, defeat managed-only policy, or force disabled
hooks back on. Claude supports settings hooks and plugin hooks, including direct
executable/args callbacks. Unavailable, disabled or untrusted hooks leave the
agent unmanaged/unknown; they do not prevent native CLI startup.

## Registration

The first validated main-agent SessionStart is the registration transaction.
Within the existing generation/mutation gate, the Worker verifies the actual
callback peer, its process ancestry, exact native agent PID/start time, PTY
membership, runtime epoch and provider-supported session evidence. Event JSON
and inherited environment identify candidates; neither alone grants authority.

Invocation identity is scoped by host runtime, Yolo session, runtime epoch,
provider and exact agent process, not by callback helper PID or provider session
ID alone. Concurrent SessionStart callbacks and lost acknowledgements must
resolve to the same committed invocation. A repeated clear/compact/resume event
inside one process does not create another OS invocation. Provider-specific
reducers handle provider session transitions and subagent events separately.

Registration and the first event commit atomically. Events other than
SessionStart cannot create a route. Corrupt records, PID reuse, cross-panel
claims, changed namespace or evidence, stale epochs and revoked credentials
remain rejection cases. No synthetic shim path or launch snapshot may be used
to satisfy the former Begin contract.

The bridge persists events before acknowledging them, retaining ordering,
deduplication and replay across transport loss. Hook callback stdout must not
inject prompts or control provider approvals. No-hook operation is a supported
terminal state, not a reason to terminate the native agent.

## Recovery And Runtime Safety

Live activity authority and durable resume information are different records.
Missing SessionEnd after crash or reboot means the activity is Unknown, not
completed. A previously verified resume identity is not discarded solely for
that reason. Before resume, verify provider evidence, directory, current
process absence and runtime identity. Explicit revocation, source substitution
or ambiguous identity must not be treated as ordinary transport loss.

Worker/Guardian continue to own process lifetime, PTY descriptors, remote
disconnect survival, exact termination and Host replacement. Agent startup no
longer creates temporary settings or a provider-specific watchdog chain.

## Implemented Boundaries

- `hook_installation` owns explicit inspect/install, provider-specific JSON
  merging, exact preview revisions and backups. Ordinary owner-controlled
  provider permissions are preserved. No install occurs during shell startup.
- The desktop Agent Integrations sheet uses the same installer for local hosts
  and approved remote commands on the existing SSH management channel.
- `claude_hook_bridge` and `codex_hooks` parse native callbacks and durably queue
  delivery. The Worker authenticates and registers their first SessionStart.
- Claude native generation records and Codex verified checkpoints preserve
  resume evidence independently of current activity. Recreate revokes the
  previous authorities before clearing the binding projection.
- Command shims, executable launch snapshots, pre-launch Begin ingress and
  launcher-specific watchdogs are removed from startup and packaging.
- Both provider fixtures consume installed native configuration and preserve
  shell-entered arguments. Tests no longer require command interception.
- Dependency checks reject callback/installer dependencies on Host, Worker or
  launchers. Mutation rules permit only the named native generation writers.

Passing focused tests or an internal archive does not establish production
acceptance. Full runtime, Linux, coverage and packaged recovery gates remain
required, followed by the existing external acceptance/signing workflow.

## Sources

Reviewed on 2026-09-09:

- <https://learn.chatgpt.com/docs/hooks>
- <https://code.claude.com/docs/en/hooks>

Current documentation is not an automatic provider-version upgrade. Existing
pinned event contracts remain unchanged until the new integration path receives
its version-specific acceptance evidence.
