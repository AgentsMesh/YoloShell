#!/bin/sh
# Exercise the native packager without changing checkout versions or using a real release.
set -eu

fixture=$(mktemp -d "${TEST_TMPDIR:-${TMPDIR:-/tmp}}/yolo-native-package.XXXXXX")
trap 'rm -rf "$fixture"' EXIT
scripts=$fixture/scripts/release
mkdir -p "$scripts" "$fixture/build_system/bazel"
for script in build.sh ensure-bazel.sh set-version.sh variables.sh; do
  cp "scripts/release/$script" "$scripts/"
done

export YOLO_RELEASE_VERSION=1.2.3
export YOLO_BAZEL_OUTPUT_ROOT="$fixture/caller cache"
mkdir -p "$YOLO_BAZEL_OUTPUT_ROOT"
touch "$YOLO_BAZEL_OUTPUT_ROOT/keep"
export BAZEL="$fixture/bazel"
export FIXTURE_BIN="$fixture/configured output"
cat > "$BAZEL" <<'SH'
#!/bin/sh
set -eu
if [ -n "${YOLO_BAZEL_OUTPUT_ROOT:-}" ]; then
  test "$1" = --batch
  test "$2" = "--output_user_root=$YOLO_BAZEL_OUTPUT_ROOT"
  shift 2
fi
case "$1" in
  build) test "$*" = 'build --spawn_strategy=local -c opt //core:yolo' ;;
  info)
    test "$*" = 'info --spawn_strategy=local -c opt bazel-bin'
    printf '%s\n' "$FIXTURE_BIN"
    ;;
  *) exit 1 ;;
esac
SH
chmod 755 "$BAZEL"
relative=core/crates/application/yolo/yolo
mkdir -p "$FIXTURE_BIN/$(dirname "$relative")" "$fixture/bazel-bin/$(dirname "$relative")"
printf '%s\n' current > "$FIXTURE_BIN/$relative"
printf '%s\n' stale > "$fixture/bazel-bin/$relative"
chmod 755 "$FIXTURE_BIN/$relative" "$fixture/bazel-bin/$relative"

cd "$fixture"
triple=aarch64-apple-darwin
archive=dist/yolo-$YOLO_RELEASE_VERSION-$triple.tar.gz
sh "$scripts/build.sh" "$triple"
test "$(tar -tzf "$archive")" = yolo
test "$(tar -xOzf "$archive" yolo)" = current
test -f "$YOLO_BAZEL_OUTPUT_ROOT/keep"

unset YOLO_BAZEL_OUTPUT_ROOT
rm "$fixture/bazel-bin/$relative"
sh "$scripts/build.sh" "$triple"
test "$(tar -xOzf "$archive" yolo)" = current
