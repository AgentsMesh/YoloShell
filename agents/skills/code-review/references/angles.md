# Finder angles

Each angle is one finder. Give it the pinned review scope from Phase 0 and this
lens text only — a finder reviews **through its assigned lens**, not broadly.

Angle sets by level:

| Level | Correctness | Cleanup |
|---|---|---|
| `medium`, `high` | A, B, C | all six |
| `xhigh`, `max` | A, B, C, D, E | all six |

`low` runs no angles — it is a single hunk-only pass (see `SKILL.md`).

---

## Correctness angles

### Angle A — line-by-line diff scan

Read every hunk in the diff, line by line. Then Read the enclosing function for
each hunk — bugs in unchanged lines of a touched function are in scope (the
change re-exposes or fails to fix them). For every line ask: what input, state,
timing, or platform makes this line wrong? Look for inverted/wrong conditions,
off-by-one, null/undefined deref, missing `await`, falsy-zero checks,
wrong-variable copy-paste, error swallowed in catch, unescaped regex metachars.

### Angle B — removed-behavior auditor

For every line the diff DELETES or replaces, name the invariant or behavior it
enforced, then search the new code for where that invariant is re-established.
If you can't find it, that's a candidate: a removed guard, a dropped error path,
a narrowed validation, a deleted test that was covering a real case.

### Angle C — cross-file tracer

For each function the diff changes, find its callers (Grep for the symbol) and
check whether the change breaks any call site: a new precondition, a changed
return shape, a new exception, a timing/ordering dependency. Also check callees:
does a parallel change in the same change set make a call unsafe?

### Angle D — language-pitfall specialist

Scan for the classic pitfalls of the diff's language/framework — for example:
JS falsy-zero, `==` coercion, closure-captured loop var; Python mutable default
args, late-binding closures; Go nil-map write, range-var capture; Swift implicit
`self` capture in escaping closures, actor-isolation hops that reorder work; SQL
injection; timezone/DST drift; float equality. Flag any instance the diff
introduces.

### Angle E — wrapper/proxy correctness

When the change adds or modifies a type that wraps another (cache, proxy,
decorator, adapter): check that every method routes to the wrapped instance and
not back through a registry/session/global — e.g. a caching provider holding a
`delegate` field that resolves IDs via `session.get(...)` instead of
`delegate.get(...)` will re-enter the cache or recurse. Also check that the
wrapper forwards all the methods the callers actually use.

---

## Cleanup angles

Cleanup, altitude, convention, and Yolo candidates use the same
`file`/`line`/`summary` shape; in `failure_scenario`, state the concrete cost
(what is duplicated, wasted, harder to maintain, or which rule is broken)
instead of a crash. **Correctness bugs always outrank these when the output cap
forces a cut.**

Cover whichever lenses apply — a finder does not need a finding from every
lens; prioritize the highest-cost issues across all of them.

### Reuse

The angles above hunt for bugs; this one and the next two hunt for cleanup in
the changed code. Flag new code that re-implements something the codebase
already has — Grep shared/utility modules and files adjacent to the change, and
name the existing helper to call instead.

### Simplification

Flag unnecessary complexity the diff adds: redundant or derivable state,
copy-paste with slight variation, deep nesting, dead code left behind. Name the
simpler form that does the same job.

### Efficiency

Flag wasted work the diff introduces: redundant computation or repeated I/O,
independent operations run sequentially, blocking work added to startup or hot
paths. Also flag long-lived objects built from closures or captured environments
— they keep the entire enclosing scope alive for the object's lifetime (a memory
leak when that scope holds large values); prefer a class/struct that copies only
the fields it needs. Name the cheaper alternative.

### Altitude

Check that each change is implemented at the right depth, not as a fragile
bandaid. Special cases layered on shared infrastructure are a sign the fix isn't
deep enough — prefer generalizing the underlying mechanism over adding special
cases.

### Conventions (CLAUDE.md / AGENTS.md)

Read every convention file pinned in Phase 0, then check the diff for clear
violations of the rules they state.

Only flag a violation when you can quote the exact rule and the exact line that
breaks it — no style preferences, no vague "spirit of the doc" inferences. In
the finding, name the file path and quote the rule so the report can cite it. If
no convention file applies, return nothing for this angle.

### Yolo invariants

This repo's recurring defects, checked against the actual diff. Each needs the
concrete violating line, not a general concern.

**Ownership**
- Mutable state gained a second writer, or a bug was patched at a call site
  instead of at the owner that holds the invariant. Enumerating write points
  always misses one; name the owner the fix belongs to.
- A capability was reimplemented instead of using its existing owner in
  `core/crates/`, `plugins/`, or `build_system/`. Name the existing mechanism and
  the actual consumers; do not propose speculative abstractions.

**Terminal and agent lifecycle**
- Pane, terminal, agent run, and provider conversation identities or lifetimes
  are conflated. Trace the actual transitions against
  `docs/terminal-agent-model.md` and the relevant protocol types.
- Detach, client failure, Host replacement, or Hub reconnection loses a live
  PTY, its custody, or its authoritative runtime identity. Identify the exact
  transition and owning process; cite `AGENTS.md` when
  judging the installed Linux service lifecycle.
- A delayed event, stale writer, or retry can mutate a different runtime after
  recovery. Trace the capability, epoch, ordering, and idempotency checks.

**Protocols and component boundaries**
- A change bypasses the current internal IPC/Hub or public client contracts;
  compare the exact endpoint with `docs/communication-architecture.md`.
- A change reintroduces retired protocol readers, aliases, or migration paths
  contrary to `docs/unreleased-contract-policy.md`.
- A crate absorbs unrelated responsibilities, or a build/tooling concern moves
  into product business logic. Show the imports and ownership boundary rather
  than judging only directory names or file counts.

**Verification**
- New or modified production code whose tests would still pass with the
  behavioral change reverted — the test guards nothing.
- A new or modified production unit left under the repo's **>95%** coverage
  requirement, or a coverage gap hidden behind a non-standard exclusion.
- Unit and integration tests lose their separate ownership, or a critical
  lifecycle/concurrency change lacks a meaningful adversarial case. Follow the
  applicable repository rules and report the actual verification scope.
- Shared-runtime validation runs outside the documented Ubuntu environment,
  automated TUI tests take window focus, or a deployment stops the persistent
  live custody domain, contrary to `AGENTS.md`.
