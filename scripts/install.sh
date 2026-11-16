#!/bin/sh
# Install the latest stable YoloShell release into the user's home directory.
set -eu

repository=${YOLO_RELEASE_REPOSITORY:-AgentsMesh/YoloShell}
install_root=${YOLO_INSTALL_ROOT:-${HOME:?HOME is required}/.local/lib/yolo}
bin_dir=${YOLO_BIN_DIR:-${HOME:?HOME is required}/.local/bin}

if [ -n "${YOLO_TARGET_TRIPLE:-}" ]; then
  triple=$YOLO_TARGET_TRIPLE
else
case "$(uname -s):$(uname -m)" in
  Darwin:arm64) triple=aarch64-apple-darwin ;;
  Linux:x86_64) triple=x86_64-unknown-linux-gnu ;;
  Linux:aarch64|Linux:arm64) triple=aarch64-unknown-linux-gnu ;;
  *) echo "unsupported platform: $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac
fi

if [ -n "${YOLO_VERSION:-}" ]; then
  version=${YOLO_VERSION#v}
  tag=v${version}
else
  latest_url=$(curl --proto '=https' --proto-redir '=https' --tlsv1.2 --location \
    --fail --silent --show-error --output /dev/null --write-out '%{url_effective}' \
    "https://github.com/$repository/releases/latest")
  tag=${latest_url##*/}
  [ -n "$tag" ] || { echo "could not resolve the latest YoloShell release" >&2; exit 1; }
  version=${tag#v}
fi

asset="yolo-${version}-${triple}.tar.gz"
base="https://github.com/$repository/releases/download/$tag"
work=$(mktemp -d "${TMPDIR:-/tmp}/yolo-install.XXXXXX")
trap 'rm -rf "$work"' EXIT INT TERM

echo "downloading yolo $version for $triple"
curl --proto '=https' --proto-redir '=https' --tlsv1.2 --location --fail --silent --show-error \
  --output "$work/$asset" "$base/$asset"
curl --proto '=https' --proto-redir '=https' --tlsv1.2 --location --fail --silent --show-error \
  --output "$work/SHA256SUMS" "$base/SHA256SUMS"

if command -v sha256sum >/dev/null 2>&1; then
  hash_file() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  hash_file() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  echo "sha256sum or shasum is required" >&2; exit 1
fi
actual=$(hash_file "$work/$asset")
expected=$(awk -v name="$asset" '$2 == name || $2 == "*" name {print $1}' "$work/SHA256SUMS" | head -1)
[ -n "$expected" ] || { echo "SHA256SUMS has no entry for $asset" >&2; exit 1; }
[ "$actual" = "$expected" ] || { echo "checksum mismatch for $asset" >&2; exit 1; }

entries=$(tar -tzf "$work/$asset")
case "$entries" in
  yolo|./yolo) ;;
  *) echo "release archive must contain exactly one yolo executable" >&2; exit 1 ;;
esac
tar -xzf "$work/$asset" -C "$work"
digest=$(hash_file "$work/yolo")
mkdir -p "$install_root/$digest" "$bin_dir"
chmod 700 "$install_root/$digest"
cp "$work/yolo" "$install_root/$digest/.yolo.tmp"
chmod 755 "$install_root/$digest/.yolo.tmp"
mv -f "$install_root/$digest/.yolo.tmp" "$install_root/$digest/yolo"
ln -s "$install_root/$digest/yolo" "$bin_dir/.yolo.tmp.$$"
mv -f "$bin_dir/.yolo.tmp.$$" "$bin_dir/yolo"

echo "installed yolo $version at $install_root/$digest/yolo"
echo "symlink: $bin_dir/yolo"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) echo "add $bin_dir to PATH to run yolo from any directory" ;;
esac
