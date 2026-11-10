#!/bin/sh
# Write the release version into the Bazel marketing-version constant.
set -eu
version=${1:?version is required}
case "$version" in
  *[!A-Za-z0-9._-]*|'') echo "invalid release version: $version" >&2; exit 1 ;;
esac
printf 'YOLO_MARKETING_VERSION = "%s"\n' "$version" > build_system/bazel/version.bzl
