#!/bin/sh
# Dispatch a release build to GitHub Actions and wait for its result.
set -eu

tag=${1:?release tag is required}
version=${2:?release version is required}
name=${3:-YoloShell $version}
prerelease=${4:-false}
source_sha=${5:-}
repo=${GITHUB_REPOSITORY:-AgentsMesh/YoloShell}
api=${GITHUB_API:-https://api.github.com}
token=${GITHUB_TOKEN:?GITHUB_TOKEN is required to dispatch a release}
command -v python3 >/dev/null 2>&1 || {
  echo "python3 is required to dispatch a release" >&2
  exit 1
}

work=$(mktemp -d "${TMPDIR:-/tmp}/yolo-dispatch.XXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM
payload="$work/payload.json"
response="$work/response.json"

python3 - "$tag" "$version" "$name" "$prerelease" "$source_sha" > "$payload" <<'PY'
import json, sys
tag, version, name, prerelease, source_sha = sys.argv[1:6]
print(json.dumps({
    "event_type": "yolo-release",
    "client_payload": {
        "tag": tag,
        "version": version,
        "name": name,
        "prerelease": prerelease,
        "source_sha": source_sha,
    },
}))
PY

curl_https() {
  curl --proto '=https' --proto-redir '=https' --tlsv1.2 --location \
    --fail --silent --show-error "$@"
}
auth_header="Authorization: Bearer $token"
accept_header="Accept: application/vnd.github+json"

dispatch_epoch=$(date -u +%s)
status=$(curl_https -o "$response" -w '%{http_code}' -X POST \
  -H "$auth_header" -H "$accept_header" -H 'Content-Type: application/json' \
  --data @"$payload" "$api/repos/$repo/dispatches")
[ "$status" = "204" ] || {
  cat "$response" >&2
  echo "GitHub release dispatch failed with HTTP $status" >&2
  exit 1
}

run_id=
for _ in $(seq 1 120); do
  runs="$work/runs.json"
  curl_https -o "$runs" -H "$auth_header" -H "$accept_header" \
    "$api/repos/$repo/actions/runs?event=repository_dispatch&per_page=20" || true
  run_id=$(python3 - "$runs" "$tag" "$dispatch_epoch" <<'PY'
import datetime, json, sys
try:
    data = json.load(open(sys.argv[1]))
except Exception:
    raise SystemExit
threshold = float(sys.argv[3]) - 60
for run in data.get("workflow_runs", []):
    if run.get("display_title") != "Release " + sys.argv[2]:
        continue
    created = run.get("created_at")
    if not created:
        continue
    moment = datetime.datetime.fromisoformat(created.replace("Z", "+00:00")).timestamp()
    if moment < threshold:
        continue
    print(run["id"])
    break
PY
)
  [ -n "$run_id" ] && break
  sleep 10
done
[ -n "$run_id" ] || {
  echo "GitHub release workflow did not start for $tag" >&2
  exit 1
}

for _ in $(seq 1 240); do
  run="$work/run.json"
  curl_https -o "$run" -H "$auth_header" -H "$accept_header" \
    "$api/repos/$repo/actions/runs/$run_id" || true
  status=$(python3 - "$run" <<'PY'
import json, sys
try:
    print(json.load(open(sys.argv[1])).get("status", ""))
except Exception:
    print("")
PY
)
  conclusion=$(python3 - "$run" <<'PY'
import json, sys
try:
    print(json.load(open(sys.argv[1])).get("conclusion") or "")
except Exception:
    print("")
PY
)
  if [ "$status" = "completed" ]; then
    url=$(python3 - "$run" <<'PY'
import json, sys
try:
    print(json.load(open(sys.argv[1])).get("html_url", ""))
except Exception:
    print("")
PY
)
    echo "GitHub release workflow $conclusion: $url"
    [ "$conclusion" = "success" ]
    exit
  fi
  sleep 30
done
echo "timed out waiting for the GitHub release workflow" >&2
exit 1
