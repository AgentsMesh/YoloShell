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

if [ -n "${YOLO_BAZEL_OUTPUT_ROOT:-}" ]; then
  set -- --batch "--output_user_root=$YOLO_BAZEL_OUTPUT_ROOT"
else
  set --
fi
"$BAZEL" "$@" build --spawn_strategy=local -c opt //core:yolo
bin_dir=$("$BAZEL" "$@" info --spawn_strategy=local -c opt bazel-bin)
binary=$bin_dir/core/crates/application/yolo/yolo
[ -x "$binary" ] || { echo "release binary is missing: $binary" >&2; exit 1; }
mkdir -p dist
package=$(mktemp -d "$root/dist/.package.XXXXXX")
trap 'rm -rf "$package"' EXIT
trap 'exit 1' INT TERM
cp "$binary" "$package/yolo"
chmod 755 "$package/yolo"
asset="dist/yolo-$VERSION-$triple.tar.gz"
COPYFILE_DISABLE=1 tar -czf "$package/archive.tar.gz" -C "$package" yolo
mv -f "$package/archive.tar.gz" "$asset"
printf '%s\n' "$asset"
