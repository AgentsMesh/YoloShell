#!/bin/sh
set -eu
if [ -z "${YOLO_WEB_E2E_CDP:-}" ] || [ -z "${YOLO_WEB_E2E_TERMINAL:-}" ]; then
  echo "browser e2e skipped: set YOLO_WEB_E2E_CDP and YOLO_WEB_E2E_TERMINAL"
  exit 0
fi
exec bun "$(dirname "$0")/browser_e2e.ts"
