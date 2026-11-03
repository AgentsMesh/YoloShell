---
name: merge
description: >-
  Commit changes, rebase onto the GitHub base branch, push, create or reuse a
  pull request, monitor GitHub Actions checks, and optionally merge it. Use only
  when the user explicitly asks to merge, commit and merge, open a PR, or finish
  the GitHub PR and CI workflow.
---

# Merge — GitHub pull request workflow

One flow: commit → branch/rebase → push → PR → checks → merge.

This skill performs outward-facing GitHub actions. Do not create, push, or merge
a PR unless the user explicitly asked for that outcome.

## Preflight

1. Determine the GitHub repository from the checkout:
   - Prefer a remote whose URL contains `github.com`.
   - Otherwise use `GH_REPO` or an explicit `-R <owner>/<repo>` supplied by the
     user. A GitLab origin is not a GitHub repository; stop and ask rather than
     pushing canonical changes to a guessed public repository.
2. Check the GitHub host authentication:
   - `gh auth status --hostname github.com`
   - If Git operations need credentials, run `gh auth setup-git`.
   - Never print or commit tokens.
3. Resolve the base branch from `git config branch.<name>.gh-merge-base`,
   `gh repo view --json defaultBranchRef`, or `main` in that order. Use `BASE`.
4. Confirm the working tree and current branch with `git status` and
   `git diff`. Treat `.env`, credentials, private keys and runtime state as
   secrets; never stage them.

## Arguments

Parse everything after the skill name:

- plain text → commit message;
- `--squash` → squash merge (default);
- `--merge` → create a merge commit;
- `--rebase` → rebase merge;
- `--no-merge` → commit, push and open the PR, but do not merge;
- `--no-monitor` → skip check monitoring;
- `--draft` → create the PR as a draft.

## Step 1 — Commit

1. Run `git status` and `git diff`; stage only files that belong to the change.
2. Use the supplied commit message if present; otherwise write a message in the
   project's existing style from the actual diff.
3. Commit with `git commit -m "<message>"`.
4. If there is nothing to commit but the branch already has commits ahead of
   the base, continue with those commits.

## Step 2 — Branch and rebase

If the current branch is the base branch:

```bash
git checkout -b <type>/<short-description>
```

Do not rewrite the base branch's history. Then rebase onto the remote base:

```bash
git fetch origin "$BASE"
git rebase "origin/$BASE"
```

Resolve conflicts manually, file by file:

- **Forbidden**: `git rebase -X ours` / `-X theirs` and similar one-sided
  strategies.
- **Forbidden**: `git checkout --ours <file>` / `--theirs <file>` whole-file
  selection.
- **Forbidden**: `sed`, `awk`, `perl -i` or similar bulk edits of conflict
  markers.
- **Forbidden**: deleting one side of a conflict block without understanding
  both intents.
- Required: inspect the full context of each conflict, preserve both semantics
  where possible, choose intentionally when they are mutually exclusive, then
  `git add <file>` and `git rebase --continue`.
- If the intent is unclear, stop and ask the user. If the rebase cannot finish
  cleanly, run `git rebase --abort` and report.

## Step 3 — Push

Push the branch to the GitHub remote, not to the base branch:

```bash
git push origin HEAD -u --force-with-lease
```

`--force-with-lease` is required after a rebase and is safer than `--force`.
Never force push `main`/the base branch.

## Step 4 — Create or reuse the PR

1. Look for an open PR for the current branch:
   `gh pr view --json number,state,isDraft,url`.
2. If one exists, reuse it. If a closed or merged PR exists for the branch,
   report it instead of silently opening a duplicate.
3. Otherwise create the PR against `origin/$BASE`:

```bash
gh pr create \
  --base "$BASE" \
  --title "<concise title>" \
  --body "$(cat <<'EOF'
## Summary
<what changed and why>

## Changes
<areas, modules and tests touched>
EOF
)"
```

Add `--draft` when requested. Print the PR URL.

## Step 5 — Monitor checks

Run the bundled monitor from this skill directory:

```bash
: "${SKILL_DIR:?Set SKILL_DIR to the merge skill directory}"
"$SKILL_DIR/scripts/wait-pr-checks.sh" <PR_NUMBER>
```

Defaults are 30-second polling and a 60-minute timeout. The script prints a
final `VERDICT: GREEN|RED|TIMEOUT` line; trust that line when a shell pipe or
background job swallows the exit code.

- **GREEN** requires a complete, non-empty check list with no pending or failing
  checks. An unreadable or empty list is *unknown*, not green.
- If checks are pending, keep waiting; a timeout means "still running", not
  "failed".
- On failure, inspect the failing check with `gh pr checks <PR>` and
  `gh run view --log-failed`; fix the cause, commit, rebase and push again, then
  re-run the monitor. Do not merge with failing or pending checks.
- If no checks are configured, report that fact; do not assume success.

## Step 6 — Merge

Only after the checks are green and the PR is mergeable:

```bash
: "${SKILL_DIR:?Set SKILL_DIR to the merge skill directory}"
"$SKILL_DIR/scripts/auto-merge-pr.sh" <PR_NUMBER> squash
```

Use `squash` by default; pass `merge` or `rebase` only when requested. The
script uses `gh pr merge --delete-branch`. Do not use `--admin` to bypass branch
protection, required reviews or failing checks. If approval is required, ask
the user to handle it.

After a successful merge, leave the current worktree and branch unchanged. Do
not switch to or pull the base branch unless the user asks.

## Exceptions

- **Not authenticated** → ask the user to run
  `gh auth login --hostname github.com`; never ask for a token in chat.
- **No GitHub remote** → stop and ask for the target repository; do not infer it
  from a GitLab origin.
- **Rebase or merge conflict** → manual resolution per Step 2; ask when intent
  is unclear.
- **Push rejected** → fetch and rebase the remote branch first; use
  `--force-with-lease` only for history you rebased.
- **Required review or approval** → report the blocker; do not bypass it.
- **Check timeout** → re-run the monitor; a timeout is not a failure.

## Safety

- PR creation and merging are outward-facing; only do them on explicit request.
- Never commit secrets, tokens, private keys, `.env` files or runtime state.
- Never push directly to the protected base branch or force push it.
- This repository's public GitHub tree may be a generated snapshot. If the
  checkout has only a GitLab remote, do not treat it as a GitHub PR target;
  ask the user which GitHub repository and branch to use.
