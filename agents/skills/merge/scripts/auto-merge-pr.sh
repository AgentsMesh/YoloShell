#!/usr/bin/env bash
# auto-merge-pr.sh — merge a GitHub pull request after checks pass.
#
# Usage: auto-merge-pr.sh <PR_NUMBER> [squash|merge|rebase]
# Default method: squash. Never uses --admin.
set -euo pipefail

PR="${1:?usage: auto-merge-pr.sh <PR_NUMBER> [squash|merge|rebase]}"
METHOD="${2:-squash}"

repo() {
  if [ -n "${GH_REPO:-}" ]; then
    gh -R "$GH_REPO" "$@"
  else
    gh "$@"
  fi
}

case "$METHOD" in
  squash) FLAG=--squash ;;
  merge) FLAG=--merge ;;
  rebase) FLAG=--rebase ;;
  *)
    echo "unknown merge method: $METHOD (use squash, merge or rebase)" >&2
    exit 2
    ;;
esac

echo "🔀 Merging PR #${PR} (${METHOD})..."
if repo pr merge "$PR" "$FLAG" --delete-branch; then
  echo "✅ PR #${PR} merged."
else
  echo "❌ Merge failed; check required approvals, mergeability and checks." >&2
  exit 1
fi
