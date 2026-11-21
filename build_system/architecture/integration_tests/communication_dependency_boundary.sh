#!/bin/sh
set -eu
if test -s "$1"; then
    echo "Public JSON-RPC, internal Cap'n Proto and persistence codecs must remain separate" >&2
    cat "$1" >&2
    exit 1
fi
