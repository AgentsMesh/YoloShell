#!/usr/bin/env bash
# wait-pr-checks.sh — wait for a GitHub pull request's checks to settle.
#
# Usage: wait-pr-checks.sh <PR_NUMBER> [POLL_INTERVAL] [TIMEOUT]
# Exit codes: 0 green, 1 red/closed, 2 timeout, 3 usage.
#
# `gh pr checks` exits 8 while checks are pending and 1 when a check fails;
# those exit codes are not the verdict. The JSON list is. A missing or empty
# list is never treated as green.
set -uo pipefail

PR="${1:?usage: wait-pr-checks.sh <PR_NUMBER> [POLL_INTERVAL] [TIMEOUT]}"
POLL="${2:-30}"
TIMEOUT="${3:-3600}"
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

repo() {
  if [ -n "${GH_REPO:-}" ]; then
    gh -R "$GH_REPO" "$@"
  else
    gh "$@"
  fi
}

pr_state() {
  repo pr view "$PR" --json state,mergeable,url 2>/dev/null
}

pr_value() {
  pr_state | python3 -c "import json,sys; print(json.load(sys.stdin).get('$1',''))" 2>/dev/null
}

elapsed=0
last=""
echo "⏳ Watching checks for PR #${PR} (poll ${POLL}s, timeout ${TIMEOUT}s)"
while true; do
  state=$(pr_value state)
  case "$state" in
    MERGED)
      echo "🎉 PR #${PR} is already merged."
      echo "VERDICT: GREEN"
      exit 0
      ;;
    CLOSED)
      echo "⚠️  PR #${PR} is closed without merging."
      echo "VERDICT: RED"
      exit 1
      ;;
  esac

  checks_file="$WORK/checks.json"
  repo pr checks "$PR" \
    --json name,state,bucket,link > "$checks_file" 2>/dev/null || true
  result=$(python3 "$SCRIPT_DIR/pr-check-status.py" "$checks_file")
  verdict=${result%%|*}
  summary=${result#*|}
  summary=${summary%%$'\n'*}
  if [ "$summary" != "$last" ]; then
    echo "🔄 $(date '+%H:%M:%S') ${summary} | waited ${elapsed}s"
    last=$summary
  fi

  if [ "$verdict" = "GREEN" ]; then
    echo "✅ All checks passed."
    echo "VERDICT: GREEN"
    exit 0
  fi
  if [ "$verdict" = "RED" ]; then
    echo "❌ Checks failed:"
    printf '%s\n' "$result" | grep '^FAILED' || true
    echo "VERDICT: RED"
    exit 1
  fi
  if [ "$elapsed" -ge "$TIMEOUT" ]; then
    echo "⏰ Timed out after ${TIMEOUT}s; checks are still running."
    echo "VERDICT: TIMEOUT"
    exit 2
  fi
  sleep "$POLL"
  elapsed=$((elapsed + POLL))
done
