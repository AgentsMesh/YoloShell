#!/bin/sh
# Create SHA256SUMS for every release archive in a directory.
set -eu
directory=${1:?asset directory is required}
cd "$directory"
if command -v sha256sum >/dev/null 2>&1; then
  hash() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  hash() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  echo "sha256sum or shasum is required" >&2; exit 1
fi
: > SHA256SUMS
for file in ./*.tar.gz; do
  name=${file#./}
  printf '%s  %s\n' "$(hash "$file")" "$name" >> SHA256SUMS
done
