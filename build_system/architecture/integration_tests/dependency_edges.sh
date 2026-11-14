#!/bin/sh
set -eu

if [ "$#" -ne 3 ] || [ ! -f "$1" ]; then
    echo "Dependency edge fixture result is unavailable" >&2
    exit 1
fi
case "$2" in
    allow) expected= ;;
    reject) expected=$3 ;;
    *) echo "Unknown dependency boundary expectation" >&2; exit 1 ;;
esac
actual=$(cat "$1")
if [ "$actual" != "$expected" ]; then
    printf 'Expected boundary witness: %s\nActual boundary witness: %s\n' "$expected" "$actual" >&2
    exit 1
fi
