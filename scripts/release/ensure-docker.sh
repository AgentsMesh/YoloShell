#!/bin/sh
# Print a shell export for a reachable Docker CLI.
set -eu

docker=${DOCKER:-}
if [ -z "$docker" ]; then
  for candidate in docker "$HOME/.orbstack/bin/docker" \
    /usr/local/bin/docker /opt/homebrew/bin/docker \
    /Applications/OrbStack.app/Contents/MacOS/xbin/docker; do
    if command -v "$candidate" >/dev/null 2>&1; then
      docker=$candidate
      break
    fi
  done
fi

if [ -z "$docker" ] && [ "$(uname -s):$(uname -m)" = "Darwin:arm64" ]; then
  directory=${CI_PROJECT_DIR:-$PWD}/.cache/tools
  binary="$directory/docker-macos-arm64"
  if [ ! -x "$binary" ]; then
    archive="$directory/docker-macos-arm64.tgz"
    extract="$directory/docker-macos-arm64-extract"
    mkdir -p "$directory"
    curl --proto '=https' --proto-redir '=https' --tlsv1.2 --location \
      --fail --silent --show-error --output "$archive" \
      "https://download.docker.com/mac/static/stable/aarch64/docker-29.4.0.tgz"
    actual=$(shasum -a 256 "$archive" | awk '{print $1}')
    if [ "$actual" != \
      "a3a6a503fad49d3a4490bb7b8e1526e448736bc9a65e629fb4e22398b903853d" ]; then
      rm -f "$archive"
      echo "Docker client checksum mismatch" >&2
      exit 1
    fi
    rm -rf "$extract"
    mkdir -p "$extract"
    tar -xzf "$archive" -C "$extract"
    cp "$extract/docker/docker" "$binary.tmp"
    chmod 755 "$binary.tmp"
    mv -f "$binary.tmp" "$binary"
    rm -rf "$extract" "$archive"
  fi
  docker=$binary
fi

[ -n "$docker" ] || {
  echo "a Docker CLI is required for containerized Linux builds" >&2
  exit 1
}
"$docker" version >/dev/null 2>&1 || {
  echo "Docker daemon is not reachable through $docker" >&2
  exit 1
}
printf "export DOCKER='%s'\n" "$docker"
