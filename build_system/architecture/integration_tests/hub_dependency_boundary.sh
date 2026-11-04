#!/bin/sh
set -eu

if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "Hub public dependency graph is unavailable" >&2
    exit 1
fi
if [ -s "$1" ]; then
    echo "Hub public models must not inherit runtime, platform or provider controllers" >&2
    cat "$1" >&2
    exit 1
fi
