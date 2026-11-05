#!/bin/sh
# Publish built release assets to GitHub Releases.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"
eval "$(sh scripts/release/variables.sh)"

token=${GITHUB_TOKEN:-}
[ -n "$token" ] || { echo "GITHUB_TOKEN is required to publish a release" >&2; exit 1; }
repo=${GITHUB_REPOSITORY:-AgentsMesh/YoloShell}
api=${GITHUB_API:-https://api.github.com}
assets=${1:-dist}
sh scripts/release/checksums.sh "$assets"
set -- "$assets"/*.tar.gz
[ -e "$1" ] || { echo "no release archives in $assets" >&2; exit 1; }

temp=$(mktemp -d "${TMPDIR:-/tmp}/yolo-publish.XXXXXX")
trap 'rm -rf "$temp"' EXIT INT TERM
body="$temp/body.md"
payload="$temp/release.json"
response="$temp/response.json"
cat > "$body" <<EOF2
YoloShell $VERSION

Install:

    curl -fsSL https://raw.githubusercontent.com/$repo/main/scripts/install.sh | sh

Update:

    yolo update
    yolo update --check

Nightly channel:

    yolo update --channel nightly

Commit: ${CI_COMMIT_SHA:-local}
EOF2
python3 - "$RELEASE_TAG" "$RELEASE_NAME" "$PRERELEASE" "$body" > "$payload" <<'PY'
import json, sys
tag, name, prerelease, body = sys.argv[1:5]
json.dump({
    "tag_name": tag,
    "target_commitish": "main",
    "name": name,
    "body": open(body).read(),
    "draft": False,
    "prerelease": prerelease == "true",
}, sys.stdout)
PY

curl_https() {
  curl --proto '=https' --proto-redir '=https' --tlsv1.2 --location \
    --fail --silent --show-error "$@"
}
auth_header="Authorization: Bearer $token"
accept_header="Accept: application/vnd.github+json"

status=$(curl_https -o "$response" -w '%{http_code}' \
  -H "$auth_header" -H "$accept_header" "$api/repos/$repo/releases/tags/$RELEASE_TAG" || true)
if [ "$status" = "200" ]; then
  id=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["id"])' "$response")
  curl_https -X DELETE -H "$auth_header" -H "$accept_header" "$api/repos/$repo/releases/$id" >/dev/null
fi

status=$(curl_https -o "$response" -w '%{http_code}' -X POST \
  -H "$auth_header" -H "$accept_header" -H 'Content-Type: application/json' \
  --data @"$payload" "$api/repos/$repo/releases")
if [ "$status" != "201" ]; then
  cat "$response" >&2
  echo "GitHub release creation failed with HTTP $status" >&2
  exit 1
fi
upload=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["upload_url"].split("{")[0])' "$response")
html=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["html_url"])' "$response")

for file in "$assets"/*.tar.gz "$assets/SHA256SUMS"; do
  name=$(basename "$file")
  status=$(curl_https -o "$response" -w '%{http_code}' -X POST \
    -H "$auth_header" -H "$accept_header" -H 'Content-Type: application/octet-stream' \
    --data-binary @"$file" "$upload?name=$name")
  if [ "$status" != "201" ]; then
    cat "$response" >&2
    echo "upload failed for $name with HTTP $status" >&2
    exit 1
  fi
done
echo "published $RELEASE_TAG: $html"
