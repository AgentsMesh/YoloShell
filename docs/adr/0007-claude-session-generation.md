# ADR 0007: Claude session generation authority

## Status

Accepted. The original evidence-only recovery restriction is superseded for
Worker-scoped sessions by durable hook authority. ADR 0010 owns the shared
lifecycle-state model; this ADR continues to own Claude generation identity.

## Context

Claude Code can create a new transcript generation after `/clear` without
changing its process identity or launch arguments. Working directory, transcript
freshness, filename order, and modification time cannot prove which panel owns a
new transcript. Two panels can legitimately run in the same directory and write
different transcripts concurrently.

Claude exposes an exact session UUID through `--resume`, `-r`, and
`--session-id` launch arguments. A process file descriptor referring to a valid
transcript is stronger ownership evidence. Neither source proves a later
in-process `/clear` transition when the descriptor association is unavailable.

## Decision

Yolo applies these rules:

1. An associated transcript descriptor may establish a binding.
2. Unassociated discovery is allowed only for an exact, canonical UUID parsed
   from the process arguments, and only for the transcript with that UUID.
3. Working directory and timestamps only scope or describe candidates; they
   never establish ownership.
4. A stable binding is not replaced by newly discovered unassociated evidence.
5. Local transcript reads retain the data-home directory descriptor, resolve
   every component with `openat` and `O_NOFOLLOW`, read through one file
   descriptor, and verify object and namespace identity after the read.
6. Unscoped and evidence-only reads persist non-exited Claude bindings as
   `Unknown`. They cannot authorize automatic recovery.
7. A managed Claude invocation begins a durable generation before `exec`. The
   generation binds the Yolo session, runtime epoch, Worker identity, exact shim
   process identity, invocation token, Claude session identity, cwd, and data
   home.
8. Synchronous lifecycle hooks are admitted only from the exact helper child of
   the bound Claude process. Session start and cwd changes additionally require
   fenced transcript evidence. State hooks are authoritative after process and
   credential admission.
9. `/clear`, resume, fork, stale hooks, Worker restart, and Host restart update or
   recover state only through generation-scoped compare-and-swap transitions.
10. Automatic recovery is enabled only when the durable authority matches both
    the active runtime identity and the persisted binding. Any mismatch remains
    `Unknown` and fail-closed.
11. Interactive argument ambiguity, settings preparation failure, hook disable,
    and Begin rejection stop launch. Only a proven non-TUI mode bypasses
    management; unknown future flags remain managed and pass through unchanged.
12. The injected command-line settings source explicitly sets
    `disableAllHooks` to `false`. Claude's documented precedence places CLI
    settings above local, project, and user settings. Yolo inspects the official
    file-based managed settings directories on macOS and Linux before launch.
    Managed `disableAllHooks` or `allowManagedHooksOnly` values that block the
    artifact hooks reject launch. Malformed values and any dynamic
    `policyHelper` also fail closed because the shim cannot prove their result.
13. Server-delivered policy and macOS MDM preferences cannot be completely
    preflighted from the shim. If either disables or restricts hooks after Begin,
    no fenced SessionStart can publish a binding: the authority has no transcript
    identity and the detector keeps the invocation `Unknown`, without automatic
    recovery.

## Consequences

- Empty or unavailable descriptor inventories do not enable cwd-only binding.
- Concurrent panels cannot claim one another's transcript by write order.
- Managed Claude sessions can recover automatically after Worker or Host restart
  when generation, runtime, process, and binding identities still agree.
- Evidence-only callers cannot accidentally bypass the durable authority path.
- Hook loss, stale generation events, identity replacement, or malformed
  authority state revoke recovery confidence without guessing from timestamps.
- A policy source that Yolo cannot inspect may prevent management, but cannot
  turn a hookless invocation into a recoverable one. This boundary follows the
  official Claude settings precedence documented at
  <https://code.claude.com/docs/en/settings#settings-precedence>.
