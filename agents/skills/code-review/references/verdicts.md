# Verdicts

A verifier gets the diff, the relevant files, and the candidates at one
`(file, line)`. It judges each candidate independently on its own claim and
returns exactly one verdict per candidate, with evidence that quotes or cites
the line.

## The ladder

- **CONFIRMED** — can name the inputs/state that trigger it and the wrong output
  or crash. Quote the line.
- **PLAUSIBLE** — mechanism is real, trigger is uncertain (timing, env, config).
  State what would confirm it.
- **REFUTED** — factually wrong (code doesn't say that) or guarded elsewhere.
  Quote the line that proves it.

Keep CONFIRMED and PLAUSIBLE. Drop REFUTED.

## PLAUSIBLE by default

Do not refute a candidate for being "speculative" or "depends on runtime state"
when the state is realistic: concurrency races, nil/undefined on a
rare-but-reachable path (error handler, cold cache, missing optional field),
falsy-zero treated as missing, off-by-one on a boundary the code does not
exclude, retry storms / partial failures, regex or allowlist that lost an
anchor. **These are PLAUSIBLE.**

**REFUTED only when constructible from the code**: factually wrong (quote the
actual line); provably impossible (type/constant/invariant — show it); already
handled in this diff (cite the guard); or pure style with no observable effect.

This bias exists because refuting on uncertainty is how real bugs get dropped.
A single non-REFUTED vote carries a finding at `high` and above.

## Sweep gap focus

What a first pass tends to miss, and therefore what the sweep hunts:

- moved or extracted code that dropped a guard or an anchor
- second-tier footguns: a dataclass default evaluated once, `hash()`
  non-determinism, a lock scope that shrank, predicate methods with side effects
- setup/teardown asymmetry in tests
- config defaults flipped

## Synthesis ranking

Correctness outranks cleanup, altitude, conventions, and Yolo findings whenever
the report cap forces a cut. Within a group, CONFIRMED outranks PLAUSIBLE.
Findings sharing a root cause merge into one entry that names the other
locations; a merged group is labeled CONFIRMED if any member was.

No silent drops while there is room: every verified finding either appears — as
a primary or as a merge note — or is omitted only because the cap is full. Say
so when the cap cut anything.
