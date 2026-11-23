#!/bin/bash

set -euo pipefail

readonly production_runtime="$1"
readonly adversarial_runtime="$2"
readonly termination_hook="YOLO_TEST_TERMINATION_BARRIER"
readonly reconcile_hook="YOLO_TEST_RECONCILE_BARRIER"
readonly hooks=("${termination_hook}" "${reconcile_hook}")

for hook in "${hooks[@]}"; do
  if LC_ALL=C grep -aFq "${hook}" "${production_runtime}"; then
    echo "error: production yolo contains adversarial hook ${hook}" >&2
    exit 1
  fi
  if ! LC_ALL=C grep -aFq "${hook}" "${adversarial_runtime}"; then
    echo "error: adversarial yolo does not contain hook ${hook}" >&2
    exit 1
  fi
done
