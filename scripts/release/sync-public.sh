#!/bin/sh
# Publish the sanitized source snapshot to the public GitHub repository.
set -eu

repo=${GITHUB_REPOSITORY:-AgentsMesh/YoloShell}
token=${GITHUB_TOKEN:?GITHUB_TOKEN is required to publish the public snapshot}
command -v perl >/dev/null 2>&1 || {
  echo "perl is required to publish the public snapshot" >&2
  exit 1
}
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"
dest=${CI_PROJECT_DIR:-$PWD}/.cache/public-export
rm -rf "$dest"
PUBLIC_REPO_URL="https://github.com/$repo" sh scripts/export-public.sh "$dest"

askpass=$(mktemp "${TMPDIR:-/tmp}/yolo-askpass.XXXXXX")
trap 'rm -f "$askpass"' EXIT INT TERM
cat > "$askpass" <<'EOF'
#!/bin/sh
case "$1" in
  *Username*) echo x-access-token ;;
  *Password*) echo "${GITHUB_TOKEN}" ;;
esac
EOF
chmod 700 "$askpass"
GIT_ASKPASS=$askpass GIT_TERMINAL_PROMPT=0 git -C "$dest" \
  -c credential.helper= push --force "https://github.com/$repo.git" main
echo "public snapshot pushed to https://github.com/$repo"
