#!/bin/sh
set -eu

if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo "Broker component dependency graph is unavailable" >&2
    exit 1
fi
if [ -s "$1" ]; then
    echo "Broker components must preserve directed domain boundaries without CLI or Clap dependencies" >&2
    cat "$1" >&2
    exit 1
fi
