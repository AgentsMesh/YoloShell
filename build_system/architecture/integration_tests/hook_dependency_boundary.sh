#!/bin/sh
set -eu

if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "hook dependency graph is unavailable" >&2
    exit 1
fi
if [ -s "$1" ]; then
    echo "hook components must not depend on agent launchers or runtime controllers" >&2
    cat "$1" >&2
    exit 1
fi
