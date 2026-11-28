#!/bin/sh
# Emit release variables for a tagged, scheduled or dispatched release.
set -eu

if [ -n "${YOLO_RELEASE_VERSION:-}" ]; then
  version=$YOLO_RELEASE_VERSION
  tag=${YOLO_RELEASE_TAG:-v$version}
  prerelease=${YOLO_RELEASE_PRERELEASE:-false}
  name=${YOLO_RELEASE_NAME:-YoloShell $version}
elif [ -n "${CI_COMMIT_TAG:-}" ]; then
  case "$CI_COMMIT_TAG" in
    v[0-9]*) ;;
    *) echo "release tag must start with v and a version" >&2; exit 1 ;;
  esac
  version=${CI_COMMIT_TAG#v}
  tag=$CI_COMMIT_TAG
  prerelease=false
  name="YoloShell $version"
else
  date=$(date -u +%Y%m%d)
  short=$(git rev-parse --short=7 HEAD)
  version="0.0.0-nightly.$date.$short"
  tag="nightly-$date-$short"
  prerelease=true
  name="YoloShell nightly $date"
fi

printf "VERSION='%s'\n" "$version"
printf "RELEASE_TAG='%s'\n" "$tag"
printf "RELEASE_NAME='%s'\n" "$name"
printf "PRERELEASE='%s'\n" "$prerelease"
