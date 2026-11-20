#!/bin/sh
# Build one Linux release asset inside a pinned glibc 2.31 container.
set -eu

triple=${1:?platform triple is required}
case "$triple" in
  x86_64-unknown-linux-gnu)
    arch=amd64
    platform=linux/amd64
    image=docker.io/library/rust@sha256:57a1fe202c77d0a56ea72baa67981509cc75df1d99f904dac875732b188f7e1f
    ;;
  aarch64-unknown-linux-gnu)
    arch=arm64
    platform=linux/arm64
    image=docker.io/library/rust@sha256:1756e77874edd517f809ef6e22f87853e8af1fb1b9e4c345073d61f8afa49507
    ;;
  *)
    echo "unsupported Linux release triple: $triple" >&2
    exit 1
    ;;
esac

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"
eval "$(sh scripts/release/variables.sh)"
sh scripts/release/set-version.sh "$VERSION"
bazel_export=$(sh scripts/release/ensure-linux-bazel.sh "$arch") || exit 1
eval "$bazel_export"
docker_export=$(sh scripts/release/ensure-docker.sh) || exit 1
eval "$docker_export"
[ -n "${LINUX_BAZEL:-}" ] || { echo "Linux Bazel is unavailable" >&2; exit 1; }
[ -n "${DOCKER:-}" ] || { echo "Docker is unavailable" >&2; exit 1; }

cache=${YOLO_RELEASE_CACHE:-$HOME/.cache/yolo-release}
mkdir -p "$cache" dist
rm -f dist/yolo
"$DOCKER" run --rm --platform "$platform" \
  --user "$(id -u):$(id -g)" \
  -e HOME=/tmp/yolo-home -e TMPDIR=/tmp/yolo-tmp -e YOLO_ARCH="$arch" \
  -v "$root:/workspace" \
  -v "$LINUX_BAZEL:/yolo-tools/bazel:ro" \
  -v "$root/dist:/yolo-artifacts" \
  -v "$cache:/yolo-cache" \
  -w /workspace "$image" /bin/bash --noprofile --norc -ceu '
    mkdir -p "$HOME" "$TMPDIR"
    bazel=/yolo-tools/bazel
    flags="--batch --output_user_root=/yolo-cache/bazel-$YOLO_ARCH"
    color="--color=no"
    # The pinned arm64 image ships binutils 2.35; its gold linker crashes with
    # update_erratum_insn on the current image. Select the bundled bfd linker.
    "$bazel" $flags build $color --repository_cache=/yolo-cache/repo \
      --compilation_mode=opt --jobs=4 \
      --@rules_rust//rust/settings:extra_rustc_flag=-Clink-arg=-fuse-ld=bfd \
      --symlink_prefix=/tmp/yolo-links/bazel- //core:yolo
    output=$("$bazel" $flags cquery $color --repository_cache=/yolo-cache/repo \
      --compilation_mode=opt --symlink_prefix=/tmp/yolo-links/bazel- \
      --output=files //core:yolo)
    [ "$(printf "%s\n" "$output" | wc -l)" -eq 1 ] || {
      echo "unexpected Bazel output: $output" >&2
      exit 1
    }
    case "$output" in
      bazel-out/*)
        execroot=$("$bazel" $flags info $color execution_root)
        binary=$execroot/$output
        ;;
      bazel-bin/*) binary="/tmp/yolo-links/$output" ;;
      /*) binary=$output ;;
      *) echo "unexpected Bazel output: $output" >&2; exit 1 ;;
    esac
    test -x "$binary"
    cp "$binary" /yolo-artifacts/yolo
    chmod 755 /yolo-artifacts/yolo
  '

asset="dist/yolo-$VERSION-$triple.tar.gz"
package=$(mktemp -d "${TMPDIR:-/tmp}/yolo-package.XXXXXX")
trap 'rm -rf "$package"' EXIT INT TERM
cp dist/yolo "$package/yolo"
COPYFILE_DISABLE=1 tar -czf "$asset" -C "$package" yolo
rm -f dist/yolo
printf '%s\n' "$asset"
