#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
vm=${YOLO_WEB_E2E_VM:-}
vm_user=${YOLO_WEB_E2E_VM_USER:-${USER:-stone}}
port=${YOLO_WEB_E2E_MOCK_PORT:-8791}
remote() {
  orb -m "$vm" -u "$vm_user" bash -lc "$1"
}

if [ -n "$vm" ]; then
  remote_home=$(remote 'printf %s "$HOME"')
  plugin_data=${YOLO_WEB_E2E_PLUGIN_DATA:-$remote_home/.local/share/yolo/plugins/web.client}
else
  plugin_data=${YOLO_WEB_E2E_PLUGIN_DATA:-$HOME/.local/share/yolo/plugins/web.client}
fi
mock_log=/tmp/yolo-web-conversation-mock.log
mock_pid=/tmp/yolo-web-conversation-mock.pid

if [ -n "$vm" ]; then
  mock_host=${YOLO_WEB_E2E_MOCK_HOST:-$(remote "hostname -I | awk '{print \$1}'")}
else
  mock_host=${YOLO_WEB_E2E_MOCK_HOST:-127.0.0.1}
fi
mock_url=${YOLO_WEB_E2E_MOCK_URL:-http://$mock_host:$port}
if [ -n "${YOLO_WEB_E2E_BUN:-}" ]; then
  bun_bin=$YOLO_WEB_E2E_BUN
elif [ -n "$vm" ]; then
  bun_bin=$(remote 'command -v bun || printf "%s/.bun/bin/bun" "$HOME"')
else
  bun_bin=bun
fi

: "${YOLO_WEB_E2E_CLAUDE_BIN:=/tmp/claude-install/node_modules/.bin/claude}"
: "${YOLO_WEB_E2E_CLAUDE_HOME:=/tmp/claude-probe-home}"
: "${YOLO_WEB_E2E_CLAUDE_SETTINGS:=/tmp/claude-probe-settings.json}"
export YOLO_WEB_E2E_CLAUDE_BIN YOLO_WEB_E2E_CLAUDE_HOME YOLO_WEB_E2E_CLAUDE_SETTINGS
export YOLO_WEB_E2E_VM YOLO_WEB_E2E_VM_USER

if [ -z "${YOLO_WEB_E2E_TOKEN:-}" ]; then
  if [ -n "$vm" ]; then
    YOLO_WEB_E2E_TOKEN=$(remote "cat '$plugin_data/token'")
  else
    YOLO_WEB_E2E_TOKEN=$(cat "$plugin_data/token")
  fi
fi
: "${YOLO_WEB_E2E_CDP:=$(curl -s http://127.0.0.1:9222/json | python3 -c 'import json,sys; print(next(t["webSocketDebuggerUrl"] for t in json.load(sys.stdin) if t.get("type")=="page" and ":4199" in t.get("url", "")))')}"
export YOLO_WEB_E2E_TOKEN YOLO_WEB_E2E_CDP YOLO_WEB_E2E_MOCK_URL="$mock_url"

if [ -n "$vm" ]; then
  remote "cd '$root' && nohup '$bun_bin' scripts/anthropic_mock_server.ts >'$mock_log' 2>&1 < /dev/null & echo \$! >'$mock_pid'"
else
  (cd "$root" && nohup "$bun_bin" scripts/anthropic_mock_server.ts >"$mock_log" 2>&1 < /dev/null & echo $! >"$mock_pid")
fi
ready=0
for _ in $(seq 1 50); do
  code=$(curl -s -o /dev/null -w "%{http_code}" "$mock_url/state" || true)
  if [ "$code" = "200" ]; then ready=1; break; fi
  sleep 0.2
done
status=0
if [ "$ready" = "1" ]; then
  (cd "$root" && bun test unit_tests/conversationE2e.test.ts) || status=$?
else
  if [ -n "$vm" ]; then remote "cat '$mock_log'" || true; else cat "$mock_log" || true; fi
  status=1
fi
if [ "$ready" = "1" ]; then
  if [ -n "$vm" ]; then
    pid=$(remote "cat '$mock_pid' 2>/dev/null || true")
    if [ -n "${pid:-}" ]; then remote "kill '$pid'" >/dev/null 2>&1 || true; fi
  else
    pid=$(cat "$mock_pid" 2>/dev/null || true)
    if [ -n "${pid:-}" ]; then kill "$pid" >/dev/null 2>&1 || true; fi
  fi
fi
exit "$status"
