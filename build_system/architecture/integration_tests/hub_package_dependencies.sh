#!/bin/sh
set -eu

if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "Hub package dependency graph is unavailable" >&2
    exit 1
fi
if [ -s "$1" ]; then
    echo "Hub packages must not depend upward on Hub role entries" >&2
    cat "$1" >&2
    exit 1
fi
