#!/bin/sh
# Print a shell export for a usable Bazel/Bazelisk command.
set -eu

if [ -n "${BAZEL:-}" ] && command -v "$BAZEL" >/dev/null 2>&1; then
  printf "export BAZEL='%s'\n" "$BAZEL"
  exit 0
fi
if command -v bazel >/dev/null 2>&1 &&
  bazel --version 2>/dev/null | grep -q 'bazel 8.2.1'; then
  printf "export BAZEL='bazel'\n"
  exit 0
fi
if command -v bazelisk >/dev/null 2>&1; then
  printf "export BAZEL='bazelisk'\n"
  exit 0
fi

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) platform=darwin-arm64 ;;
  Linux:x86_64|Linux:amd64) platform=linux-amd64 ;;
  Linux:aarch64|Linux:arm64) platform=linux-arm64 ;;
  *) echo "unsupported Bazelisk platform" >&2; exit 1 ;;
esac
version=v1.25.0
dir=${CI_PROJECT_DIR:-$PWD}/.cache/tools
mkdir -p "$dir"
binary="$dir/bazelisk"
if [ ! -x "$binary" ]; then
  curl --proto '=https' --proto-redir '=https' --tlsv1.2 --location --fail --silent --show-error \
    --output "$binary" \
    "https://github.com/bazelbuild/bazelisk/releases/download/$version/bazelisk-$platform"
  chmod 755 "$binary"
fi
printf "export BAZEL='%s'\n" "$binary"
