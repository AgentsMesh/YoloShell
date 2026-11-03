# ADR 0006: process and bootstrap exec fencing
## Status

Accepted.

## Context

Yolo must clean up a failed PTY startup without signaling an unrelated process
after PID or process-group reuse. A PID, PPID, PGID, or SID alone is not an
identity. macOS `PROC_PIDT_BSDINFOWITHUNIQID` exposes an immutable `unique_id`,
its `parent_unique_id`, and a `pidversion`. XNU also exports
`proc_signal_with_audittoken`, which resolves PID and `pidversion` together,
but macOS provides no retained process handle equivalent to a Linux pidfd.

Yolo also supports a Linux remote runtime. On either platform, close-on-exec
EOF is not proof that `exec` succeeded: a fatal signal before `exec` closes the
same descriptor. Readiness therefore needs a separate kernel observation of the
authenticated bootstrap process crossing `exec`.

Process inventory is also not one atomic kernel snapshot. Reading an anchor,
then its descendants, can combine an old parent with a replacement process or
with a child that was reparented while enumeration was in progress.

The macOS 14.2 desktop is the only client. Remote macOS hosts share that floor;
Linux hosts use platform-specific bootstrap exec proof and signal containment.

## Decision

Persisted process locators remain `(pid, start_time)`. On macOS, destructive authority is an
internal `(pid, unique_id)` live identity. Parentage is the immutable
`unique_id -> parent_unique_id` edge; numeric PPID is diagnostic data, not an ownership edge.
PGID and SID can extend an already proven PTY scope, but neither establishes identity.

macOS discovery enumerates every visible process for Yolo's effective UID with `PROC_UID_ONLY`,
then repeats the complete inventory. Enumeration clears errno before calling libproc, so a zero
result with a reported kernel error cannot be accepted as an empty inventory. Each candidate is
read as metadata, `getsid`, then metadata again; both reads must name the same `(pid, unique_id)`.
PID, start time, `unique_id`, `parent_unique_id`, PGID, and SID must also agree across inventories.

Duplicate PIDs or unique IDs, self-parent edges, truncated metadata, permission or session lookup
failures, and exhausted work budgets invalidate the scan. A candidate-local query failure may omit
only a captured generation that no longer matches. A stable zero descriptor match excludes only
that exact candidate, never its children, and is not a failed scan.

For proofs without a transferable descriptor, stable immutable ancestry survives refreshes so
reparenting cannot erase a proven origin. Conflicting parent evidence can delay anchor retirement
but cannot create a target. Session membership observed while the exact root is live may preserve a
historical scope edge through a short-lived intermediate; that observation is not signal authority.
An ancestry-pruned target still needs a stable exact identity and immutable retained-scope path.

Root-exited session and capability discovery require a typed proof from the direct-child owner using
`waitid(P_PID, ..., WEXITED | WNOWAIT | WNOHANG)`. It is bound to the captured root identity and
consumed before reap; an arbitrary caller or numeric PID cannot enable this authority state.

On macOS, process discovery creates a random value and a reciprocal private pipe. After handoff, manager and watchdog custody retain only the reader; the launcher, exact root, and descendants inherit only the writer.
The pre-exec acknowledgement binds the exact root identity and both nonzero, distinct endpoint handles.
Watchdog and runner each prove the stable exact root holds that writer before the issuer drops its writer under the spawn gate and permits target exec.

Because `dup` and `SCM_RIGHTS` can transfer the writer without an ancestry edge, descriptor-backed
discovery scans the complete stable effective-UID inventory,
including unrelated and pre-root processes. Every member independently proves the exact writer handle; the random environment value only discovers and classifies candidates and cannot prove exclusion or settlement. Parent results never propagate.
Writer presence keeps publication pending. A process that closes the writer before registration explicitly relinquishes membership. Premature EOF and indeterminate release are terminal sticky poison. Clean pipe EOF is accepted only after exact root exit and closes the discovery membership universe: a writer queued in an `SCM_RIGHTS` message or inherited by a descendant still prevents EOF.
After clean EOF, reconciliation never scans ambient process environments. It replays only exact identities matched while a writer was present, keeps them through inventory omission, prunes them only after exact-generation exit, and retains them on identity-query failure.
A post-inventory event and watcher-audit fence restarts writer-present reconciliation before a discovery epoch can publish.
The same EOF does not globally exclude managed PTY descriptors. A PTY descriptor can move between two stable live candidates or remain queued in an `SCM_RIGHTS` message, so every sequential per-process sweep can observe zero holders while custody never ceases.
Released-PTY mode always scans the full same-UID universe, including candidates without environment evidence or prior matches, and retains exact positive identities. Its negative result remains `Pending`; repeated inventories, fingerprints, and sweep counts never manufacture empty or destructive authority.
A future implementation may complete only from a genuine kernel-global PTY exclusion primitive.
Force cleanup moves opaque custody through tracking, freezing, frozen, kill-pending, tracked-gone, and reported-gone; every error returns the same move-only state. Freeze and final signal validation require
two exactly equal typed closure candidates, not merely equal PID sets. The one-way matrix is `ExactPositive <-
Exact`, `IncrementalExactPositive <- Exact | IncrementalExact`, `Ordinary <- Exact | IncrementalExact | Ordinary`,
and `KernelEof <- Exact | IncrementalExact | KernelEof`; weaker evidence never revalidates exact.

Kernel EOF is a separate move-only transaction. The output side binds a `KernelEofPermit` to the closure version, PTY master-alias authority identity, and exact EOF observation. Freeze retains its commit binding;
final kill or empty completion consumes a fresh matching `ValidatedKernelEof`. A stale closure, foreign output,
replaced alias, revoked EOF, or missing token cannot reach `SIGKILL` or gone commit. Exact tracked containment
proves only that captured closure, not global PTY absence. EOF proves output closure but does not erase
process-event debt or mint exact process absence; reconciliation remains mandatory.

The macOS output fence resumes the terminal, observes `t_outq` empty, and requests a reader epoch
published only after durable delivery and a later `WouldBlock`; in-flight reads block the fence and new
bytes revoke an earlier epoch. Exact `SIGKILL` then requires two identical empty closure observations.
Normal output completion retires custody. Reported-gone output may enter `MissingEof`, but no deadline
or retry budget proves producers-gone. Abandonment requires `MissingEof` plus proven producers-gone and
output quiescence, `NamespaceOwnershipLoss`, or explicit process-custody proof. Otherwise recovery is
visibly `OutputBlocked`; pending output, an in-flight read, or an EOF ticket blocks settlement. Startup
and fault handoff preserves custody and output. `ECHILD` keeps wait-lost custody and never restores
numeric signaling.

Linux production custody uses a persistent supervisor and a writable, exclusively
delegated cgroup-v2 root. Before `GO`, Yolo migrates the exact bootstrap process into a
bounded session leaf and persists boot-bound root and leaf identities, start time, PTY
device, and capability. A replacement Host can reopen that custody after Worker or SSH
loss only when the runtime epoch and every recorded filesystem identity still match.

Termination freezes the leaf, requires two stable bounded membership observations,
captures an exact pidfd for each member, and signals only members that prove both the
recorded PTY device and capability. It then rescans while frozen. Any surviving non-PTY
member remains frozen and makes cleanup visibly deferred: thawing it after the final
master alias closes could let its stale devpts path address a newly allocated session.
Missing, replaced, timed-out, or otherwise untrusted custody also returns a typed
fail-closed result. Numeric PID, process-group, procfs-set, and `cgroup.kill` fallbacks
are prohibited; neither a deadline nor a durable termination intent can manufacture a
gone proof.

Yolo owns the process-global `SIGCHLD` disposition. Every internal direct-child
spawn is serialized by one process-wide guard that installs `SIG_DFL`, clears
`SA_NOCLDWAIT` and `SA_SIGINFO`, and verifies the installed action. The runtime
keeps that waitable disposition instead of restoring inherited auto-reap state.

Every macOS signal uses an audit token containing the captured PID and
`pidversion`. XNU resolves those words atomically. PID reuse therefore returns
`ESRCH` instead of signaling the replacement. Because `exec` changes
`pidversion` while preserving `unique_id`, Yolo recaptures metadata and retries
with the refreshed token only while the immutable identity still matches. The
signal symbol is resolved dynamically; absence of the symbol, permission
errors, malformed metadata, and exhausted identity churn fail closed. There is
no check-then-`kill` fallback. Foreground-group operations individually signal
stable, exact members after revalidating their group membership.

The long-lived process-tree kqueue watches known PIDs for `NOTE_FORK`,
`NOTE_EXEC`, and `NOTE_EXIT`. These events are lossy invalidation hints only.
They trigger stable inventory reconciliation but never prove ownership. Read or
registration failure degrades to dense inventory polling rather than discarding
captured targets or inventing ancestry.

The bootstrap gate combines an authenticated close-on-exec status channel with
a platform kernel observer registered before `GO`. Durable `Pending` precedes
`GO`. A successful launch requires both authenticated EOF and a kernel `exec`
event. A returned exec error is framed on the authenticated channel. Exit
without `exec`, observer loss, and timeout fail closed. The worker confirms this
outcome before input, durable final delivery state, launch retirement, or
`Ready`.

The dedicated macOS bootstrap observer is narrower than the process-tree
watcher: it registers one authenticated child before `GO` and uses one-shot
`EVFILT_PROC` with `NOTE_EXEC | NOTE_EXIT`. In combination with authenticated
EOF, its `NOTE_EXEC` is exec proof. A merged `NOTE_EXEC | NOTE_EXIT` event is
valid, so a command that exits immediately after successful `exec` is not
misclassified.

After ownership and input authority are revoked, finalization may cancel reads only
after typed abandonment proof; cancellation thresholds, retry counts, and wall-clock
budgets never create that proof. The proof selects `Lost` and blocks output before the
finalizer closes the master and persists the final exit. Without it, finalization rechecks
EOF, reader activity, and pending journal output under one lifecycle lock and remains
visibly `OutputBlocked`. Pending output is never discarded to force completion.

On Linux the authenticated hello includes the exact calling TID. A dedicated
tracer thread validates the process identity and `tgkill(tgid, tid, 0)`
membership before and after `PTRACE_SEIZE`, enabling only
`PTRACE_O_TRACEEXEC | PTRACE_O_TRACEEXIT`. The wrapper rechecks that it remains
on the authenticated TID immediately before `exec`. Only
`SIGTRAP/PTRACE_EVENT_EXEC` proves success; an ordinary `SIGTRAP` is reinjected.
An exit event is detached and rejected before final exit, leaving the normal
child status for the PTY owner to reap. Timeout and cancellation use
`PTRACE_INTERRUPT`, drain the resulting stop, and detach within a fixed bound.
Waiting uses the tracer thread's own `__WALL | __WNOTHREAD` domain; this also
handles Linux changing a non-leader exec caller's TID to the process ID.

## Residual Risk

Audit-token signaling closes the PID-reuse interval for individual macOS
signals, but it is a runtime-resolved platform capability. A system without the
symbol cannot terminate a captured process and reports cleanup uncertainty.

The macOS effective-UID inventory is discovery, not a security boundary against a
malicious same-UID process. Credential transitions and kernel visibility can
make the two observations incomplete. Two full enumerations plus metadata and
`getsid` calls for every candidate also scale with the user's process count;
Yolo enforces a fixed process budget and fails closed if it is exceeded.

Yolo never treats EOF alone as exec success. Linux environments that deny
`PTRACE_SEIZE`, including restrictive seccomp or ptrace policies, fail startup
rather than fall back to liveness or pathname sampling. A future platform port
must provide an equivalent kernel exec observation primitive or fail startup
rather than weaken the proof.

Descriptor-backed discovery does not depend on observing every parent edge. Its
complete stable same-UID universe retains an unrelated or pre-root writer recipient,
and `SCM_RIGHTS` transfer extends custody rather than escaping it. Each recipient
must independently prove the exact descriptor; environment evidence cannot create membership. A holder hidden by
credential transition or kernel visibility grants no signal authority, while writer
liveness or indeterminate observation prevents empty completion and keeps recovery
visibly `OutputBlocked` until typed custody proof is available.
For PTY custody, the complete same-UID universe is not an atomic snapshot. Positive exact matches are usable ownership evidence, but sequential negative matches are not absence evidence. Queued or moving `SCM_RIGHTS` custody keeps the released-PTY state `Pending` until a separate global primitive exists.
Code outside Yolo that mutates `SIGCHLD` without using the spawn guard cannot be
serialized with Yolo's child ownership. Such code may steal wait authority;
runtime dependencies must treat the Yolo disposition as process-owned state.

macOS process kqueue has no reliable child-containment mode, and the capability
is discovery rather than a security boundary against a malicious same-UID
process. Cleanup reports uncertainty rather than claiming unproven ownership.

## Consequences

Process churn can retain an ownership anchor rather than risk signaling an
unrelated process. A failed worker stays alive while custody remains unsettled
and surfaces accumulated cleanup faults only after settlement. A future
retained-process or containment API can replace inventory discovery without changing durable session identities.
