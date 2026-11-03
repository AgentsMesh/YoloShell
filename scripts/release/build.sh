#!/bin/sh
# Build and package one native YoloShell release asset.
set -eu
triple=${1:?platform triple is required}
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$root"
eval "$(sh scripts/release/variables.sh)"
sh scripts/release/set-version.sh "$VERSION"
bazel_export=$(sh scripts/release/ensure-bazel.sh) || exit 1
eval "$bazel_export"
[ -n "${BAZEL:-}" ] || { echo "Bazel is unavailable" >&2; exit 1; }

output_root=${YOLO_BAZEL_OUTPUT_ROOT:-}
if [ -n "$output_root" ]; then
  "$BAZEL" --batch --output_user_root="$output_root" build \
    --spawn_strategy=local -c opt //core:yolo
else
  "$BAZEL" build --spawn_strategy=local -c opt //core:yolo
fi
binary=bazel-bin/core/crates/application/yolo/yolo
[ -x "$binary" ] || { echo "release binary is missing: $binary" >&2; exit 1; }
mkdir -p dist/.package
cp "$binary" dist/.package/yolo
chmod 755 dist/.package/yolo
asset="dist/yolo-$VERSION-$triple.tar.gz"
COPYFILE_DISABLE=1 tar -czf "$asset" -C dist/.package yolo
rm -rf dist/.package
[ -z "$output_root" ] || rm -rf "$output_root"
printf '%s\n' "$asset"
