#!/bin/sh
# Completeness checks for the single provider table.
#
# 1. Every topology label referenced by a provider row is exported by
#    topology.bzl (the constants stay explicit but may not drift).
# 2. Every row names an identity and a policy target, and the two compiled-in
#    composition points reference exactly those package entry points.
set -eu

topology=$1
composition=$2
catalog=$3
shift 3
if [ "$#" -eq 0 ]; then
    echo "provider table sources are missing"
    exit 1
fi
table=$(mktemp)
trap 'rm -f "$table"' EXIT
cat "$@" > "$table"
status=0

labels=$(
    awk '/topology_labels = \[/ { inside = 1 } inside { print } inside && /^ *\],/ { inside = 0 }' "$table" \
        | grep -o '"//core/[^"]*"' \
        | tr -d '"' \
        | sort -u
)
if [ -z "$labels" ]; then
    echo "providers.bzl declares no topology labels"
    exit 1
fi
for label in $labels; do
    if ! grep -Fq "\"$label\"" "$topology"; then
        echo "topology.bzl is missing the provider label $label"
        status=1
    fi
done

names=$(awk -F'"' '/^    name = "/ { print $2 }' "$table" | sort -u)
if [ -z "$names" ]; then
    echo "providers.bzl declares no provider rows"
    exit 1
fi
for name in $names; do
    if ! grep -Fq "\"//core/crates/agent/providers/$name/provider:yolo_${name}_identity\"" "$table"; then
        echo "provider row $name does not name its identity target"
        status=1
    fi
    if ! grep -Fq "\"//core/crates/agent/providers/$name/provider:yolo_${name}_provider\"" "$table"; then
        echo "provider row $name does not name its policy target"
        status=1
    fi
    if ! grep -Fq "yolo_${name}_provider::runtime()" "$composition"; then
        echo "runtime composition is missing yolo_${name}_provider::runtime()"
        status=1
    fi
    if ! grep -Fq "yolo_${name}_identity::definition()" "$catalog"; then
        echo "manifest catalog is missing yolo_${name}_identity::definition()"
        status=1
    fi
done

# The table is consumed by Bazel analysis, so every label must resolve; the
# topology check above catches a constant that was never exported.
exit "$status"
