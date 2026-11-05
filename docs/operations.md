# Yolo Operations Guide

This guide covers operating Yolo from its first-party TUI client and its trusted
plugin CLI. See the [single-executable contract](single-executable-architecture.md)
for process roles and [trusted plugins](trusted-plugin-protocol.md) for extension
lifecycle.

## Install and First Launch

Build or install the one production binary:

```bash
bazel build //core:yolo
bazel-bin/core/crates/application/yolo/yolo
```

The first-party client is the TUI; there is no AppKit application, app bundle, or
macOS archive. `yolo` uses a local Hub and Host automatically, or attaches to the
Hub selected through its connection options.

Every new terminal starts the account login shell in the user's home directory.
There is no Codex or Claude Code type to choose. Install and authenticate the
desired CLI in the normal shell environment. Yolo does not replace `claude` or
`codex`, rewrite arguments, or change shell `PATH`, startup files, or functions.
Native commands start normally without integration.

Launch the agent from a Yolo shell. The binding requires authenticated
SessionStart, process, PTY, runtime, and provider evidence. Status comes from
hooks, not terminal output. Disabled, untrusted, restricted, or incomplete hooks
remain unmanaged/Unknown; Yolo does not grant provider hook trust. Resume
requires verified durable identity.

## Local Services and Updates

Use `yolo status` and `yolo version` to distinguish the installed image from the
images the running Host and Hub mapped. `yolo start`, `yolo stop` and
`yolo restart` control the current profile; stopping services preserves live
Workers and PTYs. `yolo update` checks for a newer release, installs it, and
activates it through the same restart path; `--check` is the read-only mode.

See [local service lifecycle](local-service-lifecycle.md) for the command
contract and [upgrade admission](upgrade-admission.md) for the compatibility
checks.

## Tabs, Panes, and Terminals

A Tab contains a split tree of Panes. Each Pane can bind to one Terminal.
Terminal, PTY, AgentRun and provider history have separate lifetimes; see
[the domain model](terminal-agent-model.md). Closing a Pane, Tab or client
detaches its views and leaves live PTYs and processes running.

- `Ctrl-B v` splits right and `Ctrl-B -` splits down; both create a new shell.
- `Ctrl-B z` toggles zoom, `Ctrl-B {` and `Ctrl-B }` swap panes, and dividers
  can be dragged for custom ratios.
- The recursive split tree, ratios, focused pane, tab and pane IDs, and terminal
  bindings persist across launches.
- `yolo` enters the TUI; `yolo new` creates another shell. Start any TUI agent
  directly there.
- `Ctrl-B d` exits the client while preserving its terminals; `yolo attach`
  reconnects.
- `Ctrl-B g` or `yolo terminals` opens the terminal picker; `Ctrl-B s` focuses
  the Agent runs / Hubs sidebar.
- `Ctrl-B w` opens the tab manager; `Ctrl-B c` creates a tab with a shell;
  `Ctrl-B ?` shows help.
- `yolo clients` lists attached clients; `yolo detach [CLIENT_ID]` disconnects
  one from another shell.

One holder controls terminal input at a time. A second pane or client that
opens the same terminal is read-only. Explicit takeover fences the previous
writer before enabling input; it is not a multi-writer mode. A newly created
shell in its original pane should acquire control without that action.

## Hubs and Remote Runtime

The Machines inventory exposes `Manage machines` (`m`). Configuration belongs
to the local Hub: it manages machine SSH/socket dials, trusted origins, file
roots and per-machine export policies. Reciprocal identity pinning and export authorization
must also be configured at the remote machine; the TUI never trusts an unknown key.
Machine links use the user's SSH access to `<hub-state>/peer.sock` and open no
public Hub port. See [Machines](machines.md) and [TUI Hub
management](tui-machine-management.md).

One SSH transport multiplexes runtime, attachment, bounded command, SFTP read,
and deployment channels. Host keys are verified before authentication against
fail-closed known-hosts data. Yolo does not use trust-on-first-use. Unknown,
changed, or revoked host keys are rejected.

| Remote target | Runtime artifact | Disconnect recovery | Destructive recovery |
| --- | --- | --- | --- |
| Apple Silicon macOS | Bundled arm64 Mach-O may be used. | Reattaches after transport loss. | Exact macOS process identity fencing. |
| Linux aarch64 or x86_64, glibc 2.31+ | Preview artifacts are built and verified. | Preview until live Linux SSH recovery is release-blocking. | Preview cgroup-v2 custody only. |

On Linux, Yolo requires glibc 2.31 or newer. Run
`yolo internal host serve --home <remote-state>` under a persistent supervisor
outside SSH login cleanup. Set `YOLO_CGROUP_DELEGATED_ROOT` to an absolute,
writable cgroup-v2 parent exposing the `pids` controller and freezer. Keep Host
and Worker processes in a child cgroup; Yolo owns one bounded leaf per runtime
below the unpopulated controller root. Missing or replaced evidence, timeout, or
an unauthenticated holder returns `destructive_session_unavailable` before an
unsafe signal.

## Trusted Plugins

Trusted plugins are user-level processes linked from a local directory. They run
with the user's authority and are not sandboxed. Linking a plugin is the trust
decision:

```bash
yolo plugin link ./my-plugin --trust
yolo plugin startup
yolo plugin list
yolo plugin action my.plugin status
yolo plugin log my.plugin
yolo plugin unlink my.plugin
```

Yolo executes one startup hook and fixed actions; it never supervises plugin
daemons, probes ports, or kills plugin processes. Plugins manage their own
daemon, PID file, port, and UI. A plugin receives `YOLO_HUB_SOCKET`, the
same-UID local management socket, and must not expose that socket or its token to
browsers. See [trusted plugin protocol](trusted-plugin-protocol.md).

## Diagnostics

Check the inline TUI error, Hub availability, and terminal status; retry or
reconnect before touching durable state. A stalled management request does not
block terminal input or catalogue traffic.

Local durable state lives below Yolo's platform data directory:
`~/Library/Application Support/Yolo` on macOS and the XDG data directory on
Linux. Remote profiles and Hub state are private. The per-session `logs`
directory is reserved state, not a complete product log.

Plugin startup and action output is captured to a bounded, redacted log. Use
`yolo plugin log <id>` to tail it; `yolo plugin status <id>` reports only the last
startup result, not liveness. `yolo plugin data-dir <id>` prints the plugin's
writable data directory.

- For an unknown or changed host key, independently verify the server key and
  then update the configured known-hosts file. Never accept a mismatch only to
  make the connection succeed.
- For authentication failure, check the direct endpoint, agent socket, local
  identity permissions, and the selected identity's Keychain passphrase.
- For runtime incompatibility, verify platform, architecture, `--version`
  output, SHA-256, and the absolute remote executable and Remote state paths.
- For an unreachable or uncertain session, restore connectivity and use the
  TUI recovery action. Do not delete sockets, journals, or manifests, and do not
  signal a remembered PID.

## Stopping Sessions and Uninstalling

Stop Session requests graceful termination of the selected active session's
capability-registered process tree after confirmation. Descendants inherit the
discovery writer; a descendant that deliberately closes it before registration
relinquishes Yolo membership and is outside this termination guarantee. Stop All
Background Sessions freezes the complete catalog snapshot and reports every
success, already-exited row, unavailable capability, and failure in snapshot
order. Close Pane and detach remain non-terminating operations.

When the final Host client disconnects, the Host waits for a quiet period and
attempts an automatic idle shutdown. Operators can request the same fenced check
with `yolo internal host ctl --home <path> --existing-host shutdown-if-idle`.
Clients negotiate the `idle_host_shutdown` capability and never treat EOF as
success.

Before uninstalling:

1. Stop every local and remote session normally and archive exited sessions.
2. `yolo plugin action <id> stop` for any plugin that runs a daemon, then
   `yolo plugin unlink <id>` when its files are no longer needed.
3. Quit the TUI and wait for idle Host supervisors to retire automatically. If a
   supervisor remains, use authenticated `shutdown-if-idle`; a refusal identifies
   live or indeterminate state that must be resolved without selecting a PID.
4. Remove the `yolo` binary and, only after the process check, the local state
   directory. Preserve state if any session may need recovery.

Local uninstall does not delete remote state or deployed runtimes. Leave remote
state intact if status is uncertain. A remote administrator may remove it only
after verifying that no Yolo Host, Worker, or child remains.
