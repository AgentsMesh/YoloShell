#!/bin/sh
# Download the pinned Linux Bazel used by the release containers.
set -eu

arch=${1:?linux architecture is required}
case "$arch" in
  amd64)
    bazel_arch=x86_64
    sha=7ff2b6a675b59a791d007c526977d5262ade8fa52efc8e0d1ff9e18859909fc0
    ;;
  arm64)
    bazel_arch=arm64
    sha=1f50df607751a56ace7775872ffb5dff4c15c25c11a2df30b7b0035ef53a9937
    ;;
  *)
    echo "unsupported Linux architecture: $arch" >&2
    exit 1
    ;;
esac

version=8.2.1
directory=${CI_PROJECT_DIR:-$PWD}/.cache/tools
binary="$directory/bazel-linux-$arch"
mkdir -p "$directory"

if command -v sha256sum >/dev/null 2>&1; then
  hash() { sha256sum "$1" | awk '{print $1}'; }
else
  hash() { shasum -a 256 "$1" | awk '{print $1}'; }
fi

if [ -x "$binary" ] && [ "$(hash "$binary")" = "$sha" ]; then
  printf "export LINUX_BAZEL='%s'\n" "$binary"
  exit 0
fi

curl --proto '=https' --proto-redir '=https' --tlsv1.2 --location \
  --fail --silent --show-error --output "$binary.tmp" \
  "https://github.com/bazelbuild/bazel/releases/download/$version/bazel-$version-linux-$bazel_arch"
if [ "$(hash "$binary.tmp")" != "$sha" ]; then
  rm -f "$binary.tmp"
  echo "Linux Bazel checksum mismatch" >&2
  exit 1
fi
chmod 755 "$binary.tmp"
mv -f "$binary.tmp" "$binary"
printf "export LINUX_BAZEL='%s'\n" "$binary"
