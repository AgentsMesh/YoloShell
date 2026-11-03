#!/bin/sh
# Run the architecture conformance gate in a Linux container.
set -eu

case "$(uname -m)" in
  arm64|aarch64)
    arch=arm64
    platform=linux/arm64
    image=docker.io/library/rust@sha256:1756e77874edd517f809ef6e22f87853e8af1fb1b9e4c345073d61f8afa49507
    ;;
  x86_64|amd64)
    arch=amd64
    platform=linux/amd64
    image=docker.io/library/rust@sha256:57a1fe202c77d0a56ea72baa67981509cc75df1d99f904dac875732b188f7e1f
    ;;
  *)
    echo "unsupported host architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"
bazel_export=$(sh scripts/release/ensure-linux-bazel.sh "$arch") || exit 1
eval "$bazel_export"
docker_export=$(sh scripts/release/ensure-docker.sh) || exit 1
eval "$docker_export"
[ -n "${LINUX_BAZEL:-}" ] || { echo "Linux Bazel is unavailable" >&2; exit 1; }
[ -n "${DOCKER:-}" ] || { echo "Docker is unavailable" >&2; exit 1; }

cache=${YOLO_RELEASE_CACHE:-$HOME/.cache/yolo-release}
mkdir -p "$cache"
"$DOCKER" run --rm --platform "$platform" \
  --user "$(id -u):$(id -g)" \
  -e HOME=/tmp/yolo-home -e TMPDIR=/tmp/yolo-tmp \
  -v "$root:/workspace" \
  -v "$LINUX_BAZEL:/yolo-tools/bazel:ro" \
  -v "$cache:/yolo-cache" \
  -w /workspace "$image" /bin/bash --noprofile --norc -ceu '
    mkdir -p "$HOME" "$TMPDIR"
    /yolo-tools/bazel --batch --output_user_root=/yolo-cache/bazel-verify test \
      --color=no \
      --repository_cache=/yolo-cache/repo --spawn_strategy=local --test_output=errors \
      --cache_test_results=no \
      //build_system/architecture:conformance_test
  '
