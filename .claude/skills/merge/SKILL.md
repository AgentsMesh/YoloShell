---
name: merge
description: >-
  GitHub PR workflow: commit, rebase, push, create or reuse a pull request,
  monitor GitHub Actions checks, and merge with squash, merge or rebase. Use
  only when the user explicitly asks to merge, commit and merge, open a PR, or
  finish the GitHub PR and CI workflow.
argument-hint: "[commit message] [--squash|--merge|--rebase] [--no-merge] [--no-monitor] [--draft]"
---

# Claude Code Adapter

1. Read `shared/SKILL.md` completely before acting, then follow its workflow.
   `shared/` is this skill's shared workflow and `scripts/` directory.
2. Parse `$ARGUMENTS` per the shared workflow into a commit message and merge
   options. Default to squash; `--no-merge` only opens the PR, and
   `--no-monitor` skips check monitoring.
3. This is a **GitHub PR** skill: use `gh`, not `glab`. First confirm the
   checkout has a GitHub remote, or that the user explicitly supplied
   `GH_REPO` / `-R owner/repo`. A GitLab origin is not a GitHub repository; the
   public GitHub tree may be a generated snapshot, so stop and ask when unsure.
4. Create, push or merge only when the user explicitly requested it. Do not use
   `--admin` to bypass protection, required reviews or failing checks. Leave the
   current worktree and branch unchanged after a merge.
5. Use the shared scripts for monitoring and merging:

```bash
: "${SKILL_DIR:?Set SKILL_DIR to the merge skill directory}"
"$SKILL_DIR/scripts/wait-pr-checks.sh" <PR_NUMBER>
"$SKILL_DIR/scripts/auto-merge-pr.sh" <PR_NUMBER> squash
```

The scripts print a final `VERDICT: GREEN|RED|TIMEOUT` line; trust that line when
a pipe or background job swallows the exit code. A timeout means checks are still
running, not failed; re-run the monitor rather than merging directly.
6. Resolve rebase/merge conflicts per the shared workflow, file by file. Never
   use `-X ours/theirs`, whole-file `--ours/--theirs`, or sed/awk/perl to rewrite
   conflict markers. Stop and ask when the intent is unclear. Push with
   `--force-with-lease`; never force push main or the base branch.
