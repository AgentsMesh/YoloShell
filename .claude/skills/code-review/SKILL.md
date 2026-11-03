---
name: code-review
description: >-
  Reviews the current diff for correctness bugs plus reuse, simplification,
  efficiency, altitude, convention, and Yolo-invariant cleanups at a chosen
  effort level, then reports ranked findings with concrete failure scenarios.
  Use when asked to review a diff, branch, MR, or PR, to find bugs in the
  current changes, or to check a change before merging. Pass --fix to apply the
  findings, --comment to post them as inline MR/PR comments.
argument-hint: "[low|medium|high|xhigh|max] [--fix] [--comment] [<target>]"
---

# Claude Code Adapter

1. Read `shared/SKILL.md` completely before taking action, then the
   `shared/references/` files the level you picked calls for.
2. Parse `$ARGUMENTS` as `[level] [--fix] [--comment] [<target>]` per the shared
   workflow. Resolve bundled references against `shared/`; repository paths
   such as `AGENTS.md` and `docs/` resolve against the checkout root.
3. Phase 0's host user-level convention file is `~/.claude/CLAUDE.md`; pin it
   alongside the repo-root and ancestor-directory files.
4. Fan out finders and verifiers with the **`Agent`** tool — one agent per angle
   in Phase 1, one per `(file, line)` group in Phase 2, one for the Phase 3
   sweep. Run each phase in concurrent batches within host capacity, preserving
   every angle's independent context. If `Agent` is unavailable, take the shared
   workflow's single-pass path and say so in the summary.
5. Report through the **`ReportFindings`** tool when it is available: call it
   once with `{level, findings}`, most-severe first, an empty array if nothing
   survived. Do not also print the findings as text and do not publish an
   artifact — that call is the report. Without the tool, print one line per
   finding: `path/to/file.ext:123 — what's wrong and the concrete failure`, or
   exactly `(none)`.
6. When findings reported through `ReportFindings` are fixed later in this
   session — because `--fix` ran, the user asked, or later work fixed them
   incidentally — call `ReportFindings` again with the same findings, each
   carrying an `outcome` of `fixed`, `no_change_needed`, or `skipped`, before
   any prose summary. The host UI's per-finding status updates only from that
   call; without it the findings stay marked unresolved.
7. `--comment` on a GitHub target: the host inline-comment tool the shared
   workflow defers to is `mcp__github_inline_comment__create_inline_comment`.

Detect the actual forge from the Git remote using the shared workflow; do not
assume GitLab or GitHub.

Note that Claude Code also ships a built-in `/code-review` command with a
cloud-backed `ultra` mode. This skill is the repo-owned equivalent that Codex
shares — prefer it when the review must apply this repo's Yolo invariants, and
keep both in sync by editing `shared/` only.
