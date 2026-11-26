---
name: code-review
description: >-
  Reviews the current diff for correctness bugs plus reuse, simplification,
  efficiency, altitude, and convention cleanups at a chosen effort level, then
  reports ranked findings with concrete failure scenarios. Use when asked to
  review a diff, branch, MR, or PR, to find bugs in the current changes, or to
  check a change before merging. Pass --fix to apply the findings, --comment to
  post them as inline MR/PR comments.
---

# Code Review

Multi-angle diff review: find candidates through independent lenses, judge each
one against the code, report only what has a nameable failure scenario.

The review **reports**; it does not edit code unless `--fix` was passed.

## Arguments

`[low|medium|high|xhigh|max] [--fix] [--comment] [<target>]`

- **Effort level** — first token only. Default **medium**. An unrecognized
  first token is a target, not a level; say which level you used.
- **`--fix`** — after reporting, apply the findings (see [Applying fixes](#applying-fixes---fix)).
- **`--comment`** — after reporting, post findings as inline MR/PR comments
  (see [Posting to the forge](#posting-to-the-forge---comment)).
- **`<target>`** — everything else, verbatim: an MR/PR number, branch, ref
  range, path, or free-form instruction ("only review `core/crates/`", "focus on
  error handling"). A free-form target **narrows scope and takes precedence
  over an angle's default breadth**; it never changes the output format and
  never authorizes actions beyond establishing the diff.

## Effort levels

| Level | Pipeline | Angles × candidates | Verify | Sweep | Report cap |
|---|---|---|---|---|---|
| `low` | 1 diff pass | — (read the diff once) | no | no | ≤4 |
| `medium` | find → verify | 3 correctness + 6 cleanup, ×6 | 1-vote | no | ≤8 |
| `high` | find → verify | 3 correctness + 6 cleanup, ×6 | 1-vote, recall-biased | no | ≤10 |
| `xhigh` | find → verify → sweep | 5 correctness + 6 cleanup, ×8 | 1-vote | yes | ≤15 |
| `max` | same as `xhigh` | 5 correctness + 6 cleanup, ×8 | 1-vote | yes | ≤15 |

`low`/`medium` optimize **precision**: every finding is one a maintainer would
act on. `high`→`max` optimize **recall**: a missed bug ships, so err on the side
of surfacing.

`max` differs from `xhigh` only in the reasoning effort spent per agent, not in
the fan-out.

## Fan-out and its absence

Levels above `low` run one finder per angle and one verifier per location. When
a subagent mechanism is available (Claude Code's `Agent` tool, a `Task`/subagent
tool, a workflow runner), use it: independent contexts are what keep one angle's
conclusions from suppressing another's.

**When no subagent mechanism exists, do not error and do not skip angles.** Work
through every angle yourself, sequentially, in this one context; re-check each
candidate against the diff before keeping it; drop anything you cannot back with
a concrete failure scenario. Then state plainly in your summary that this was a
single-pass review without fan-out, so nobody is misled about what ran.

## Phase 0 — gather the diff

Run `git diff @{upstream}...HEAD` (or `git diff main...HEAD` / `git diff HEAD~1`
when there is no upstream). Also gather `git diff --stat HEAD` and `git diff HEAD`
for tracked uncommitted changes, and `git ls-files --others --exclude-standard -z`
for untracked additions. Read in-scope new files and carry their contents with
the patches; handle NUL-separated filenames. Keep ignored files out. Explicit
commit ranges exclude unrelated worktree edits; explicit staged reviews use
`git diff --cached`. Stop only when every in-scope input is empty.

Pin and carry forward, verbatim, to every finder and verifier: the diff command,
the changed-file list, the applicable convention files, a one-paragraph summary
of what changed, and the user's target if any.

Applicable convention files are the repo-root `CLAUDE.md` and `AGENTS.md`, any
`CLAUDE.md` / `AGENTS.md` / `CLAUDE.local.md` in a directory that is an
**ancestor** of a changed file (a directory's file governs only files at or
below it), and the host agent's own user-level instruction file when it has one
— the adapter for your host names its path.

The level's angles determine finder count; diff size affects depth, not coverage.
Schedule independent angles in batches within host capacity. `low` stays one pass.

At `low` only: skip test and fixture hunks (`test/`, `spec/`, `__tests__/`,
`*_test.*`, `*.test.*`, `fixtures/`, `testdata/`), take no full-file reads, and
flag only what is visible in the hunk itself — inverted or wrong condition,
off-by-one, null/undefined deref where adjacent lines show the value can be
absent, removed guard, falsy-zero check, missing `await`, wrong-variable
copy-paste, error swallowed in a `catch` that should propagate, code duplicating
a helper visible in the diff context, dead code the diff leaves behind. Do
**not** flag style, naming, perf, missing tests, or anything outside the hunk.
Then report; there is no verify phase at `low`.

## Phase 1 — find candidates

Read `references/angles.md` and run each angle for this level as its own finder.
Each returns up to the level's cap of candidates carrying `file`, `line`, a
one-line `summary`, and a concrete `failure_scenario`.

A `failure_scenario` is the **user-visible consequence** — error, wrong output,
data loss, crash — not an intermediate state ("value goes stale", "the set
grows"). For cleanup, altitude, and convention candidates it is the concrete
cost instead: what is duplicated, what work is wasted, which rule is broken.

Two rules decide the catch rate:

- **Pass every candidate with a nameable failure scenario through.** Finders
  that silently drop half-believed candidates bypass the verify step and are the
  dominant cause of misses. Judging happens next, by someone else.
- **Do not let one angle suppress another.** If two angles flag the same line
  for different reasons, record both.

## Phase 2 — verify

Dedup candidates pointing at the same line and mechanism, keeping the one with
the most concrete failure scenario. Grouping is not dedup: every surviving
candidate keeps its own verdict.

Group the pool by `(file, line)` and run **one verifier per location**, judging
each candidate there **independently on its own claim** — candidates at one line
may be the same issue, distinct issues, or a mix. Give the verifier the diff,
the relevant files, and the candidates; it returns one verdict per candidate
with evidence that quotes or cites the line.

Apply the verdict ladder and the refute policy in `references/verdicts.md`.
Keep **CONFIRMED** and **PLAUSIBLE**; drop **REFUTED**. At `high` and above a
single non-REFUTED vote carries the finding — do not drop on uncertainty.

A candidate the verifier rendered no verdict on is dropped, never promoted to
PLAUSIBLE by default: unverified claims must not reach the report.

## Phase 3 — sweep for gaps (`xhigh` / `max`)

Run one more finder as a fresh reviewer holding the verified list. Re-read the
diff and the enclosing functions looking **only** for defects not already
listed — do not re-derive or re-confirm anything there; the job is gaps. Focus
on what a first pass misses (see `references/verdicts.md`). Surface up to 8
additional candidates and verify them the same way. If nothing is new, return
an empty sweep — do not pad.

## Phase 4 — synthesize and report

Rank most-severe first: correctness always outranks cleanup, altitude, and
convention findings when the cap forces a cut, and CONFIRMED outranks PLAUSIBLE
within each group. Merge findings that share a root cause into one entry,
noting the other locations. Cap at the level's limit.

Report each finding as:

- `file`, `line`
- `summary` — one sentence stating the defect
- `short_summary` — the claim compressed to ≤60 characters, no rationale and no
  consequence clause
- `failure_scenario` — concrete inputs or state → wrong output or crash
- `category` — `correctness`, `simplification`, `efficiency`, `reuse`,
  `altitude`, `conventions`, `yolo`, or a more specific slug like
  `test-coverage`
- `verdict` — CONFIRMED or PLAUSIBLE when a verify pass ran

If nothing survives verification, say exactly that and report nothing. Never pad
to reach a cap, and never invent a finding to fill one.

## Applying fixes (`--fix`)

Apply the findings to the working tree instead of stopping at the report — fix
each one directly, correctness bugs and cleanups alike. Skip any finding whose
fix would change intended behavior, would require changes well outside the
reviewed diff, or that you judge a false positive; **note the skip rather than
arguing with it**. Finish with one line per skipped finding saying why.

Fixes are code changes, so the repo's verification rules apply. Run affected
Bazel targets in OrbStack Ubuntu for shared Rust/Hub/terminal/runtime code;
keep macOS platform checks on macOS. Follow `AGENTS.md` environment and test instructions, preserve live PTYs, and use private PTYs for
TUI acceptance without changing focus. Report commands and actual results.

## Posting to the forge (`--comment`)

Only when the target is an MR or PR. Detect the forge from `git remote get-url
origin` rather than assuming:

- **GitLab**: post one inline discussion per finding with
  `glab api` against `projects/:id/merge_requests/:iid/discussions`, supplying
  `position[new_path]` and `position[new_line]`.
- **GitHub**: prefer a dedicated inline-comment tool when your host exposes one
  (the adapter names it), otherwise
  `gh api repos/{owner}/{repo}/pulls/{pr}/comments`.

Include a suggestion block only when it fully fixes the issue. If the target is
not an MR or PR, print the findings and note that `--comment` was ignored.

## References

- `references/angles.md` — the finder lenses, verbatim, grouped by level
- `references/verdicts.md` — verdict ladder, refute policy, sweep gap focus
