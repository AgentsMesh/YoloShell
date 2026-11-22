# Trusted plugin protocol

Yolo has one third-party extension type: a trusted plugin running as the user.
There is no sandbox, Hub approval, server-plugin runtime or client/server profile.
The plugin owns its process, daemon, port, PID files, UI and logs. Yolo only
records the plugin, validates its manifest, and runs one startup hook or one
fixed action on request.

## Package and manifest

A plugin is a directory containing `yolo-plugin.json` schema 1 and its own files.
The manifest is bounded to 256 KiB and rejects unknown fields.

```json
{
  "schema_version": 1,
  "id": "acme.dashboard",
  "name": "Acme Dashboard",
  "version": "0.1.0",
  "description": "A trusted Yolo web client",
  "platforms": ["linux", "macos"],
  "startup": {
    "command": ["bun", "scripts/plugin.ts", "start"]
  },
  "actions": [
    {"id": "status", "title": "Status", "command": ["bun", "scripts/plugin.ts", "status"]},
    {"id": "stop", "title": "Stop", "command": ["bun", "scripts/plugin.ts", "stop"]}
  ],
  "web": {"title": "Acme Dashboard"},
  "api": {
    "protocol_version": 1,
    "required_methods": ["hello", "list", "get", "attach"],
    "optional_methods": [],
    "features": []
  }
}
```

- `id` is a dotted lowercase identifier. The `yolo.` prefix is reserved for
  built-in programs and providers; external plugins are rejected.
- `platforms` contains `unix`, `linux` or `macos`. `any` is not valid for a
  plugin process.
- `startup` is optional and at most one hook. `actions` is optional and at most
  64. At least one of them must be present. A manifest with `web` must have a
  startup hook.
- `startup` and action commands are argv arrays. Yolo never invokes a shell,
  joins arguments, or expands variables. The working directory is the plugin
  root. Limits are 64 arguments and 16 KiB of total argv per command.
- The startup and action timeout is 30 seconds. A timeout terminates the
  spawned process group.
- `web` is display metadata only. It contains no command, URL or port. The
  plugin starts and prints its own URL; Yolo never probes a port.
- `api` declares the Hub public JSON-RPC contract the plugin expects. Yolo
  validates its shape and displays it; it is metadata for the plugin's own
  tooling, not Hub authorization or a method gate, because a trusted plugin
  already runs with the user's authority.

## Registry

`yolo plugin link <dir> --trust` validates the manifest, computes a digest of
the manifest and package contents, shows the exact commands and digest, and
requires the explicit `--trust` flag. Linking is in place: Yolo does not copy
source.
`unlink` removes only the registry entry.

`yolo plugin install <id>` installs and enables the selected registry plugin
in one invocation. It uses Git to fetch the newest version by default.
`--version` selects a specific version and `--registry` selects another Git
registry. The installer verifies the pinned commit and manifest, copies the
source into a private managed directory, and records its verified digest.
The official Web plugin is `web.client`. [Default registry](https://github.com/AgentsMesh/YoloShellPlugins)

`--dry-run` verifies the source, prints the commands and digest, and exits
before modifying the installation or running lifecycle hooks. Scripts that
previously invoked a bare install just to preview it should use this flag.
Existing install commands with `--trust` remain compatible.

Installing over a linked id replaces the link after verification. When the
previous manifest declares a `stop` action, the installer stops that plugin and
starts the newly linked version; otherwise it warns that a running plugin must
be restarted manually. Reinstalling the same version reuses the verified
managed source instead of failing.

The local registry is `$XDG_STATE_HOME/yolo/plugins/plugins.json` on Linux and
`~/Library/Application Support/Yolo/plugins/plugins.json` on macOS. Plugin data
and logs live in sibling `plugins-data/` and `plugins-state/` roots. Directories
are private (`0700`); the registry is written atomically under a cross-process
lock. It records the plugin id, source path, digest,
enabled state, platforms, install time and last-startup result. It contains no
secrets.

Every startup/action revalidates the source digest. If the linked directory was
replaced or modified, the operation fails closed until it is relinked.

## Lifecycle and environment

`yolo plugin startup` runs the startup hook of every enabled, platform-matching
plugin once. One failure is logged and does not block other plugins. The hook
must detach and exit within 30 seconds; Yolo does not retain its PID, restart it,
or signal its daemon.

`yolo plugin action <id> <action>` runs one fixed action command. Action
arguments are not supported in schema 1. `yolo plugin status <id>` reports the
last startup result, not liveness.

The plugin inherits the user environment. Yolo overrides:

- `YOLO_HUB_SOCKET`
- `YOLO_HUB_SOCKET_KIND=local_management`
- `YOLO_HUB_ID`
- `YOLO_PLUGIN_ID`
- `YOLO_PLUGIN_ROOT`
- `YOLO_PLUGIN_DATA`
- `YOLO_PLUGIN_ACTION_ID`
- `YOLO_PLUGIN_CONTEXT_JSON` (bounded)

`YOLO_PLUGIN_DATA` is the one writable per-plugin directory. Yolo does not inject
separate config, state, event or binary paths.

Yolo captures startup/action stdout and stderr into a bounded `0600` log. It
redacts the Hub socket path and Hub id. `yolo plugin log` tails only this captured
log; daemon logs belong to the plugin.

The Hub socket is the same-UID **local management** endpoint. It accepts the full
local dispatcher, is authenticated only by UID, and audits the principal as
`"local"`. The plugin has local-admin authority. It must not proxy the socket,
Hub id or any token to a browser. A plugin web server must expose fixed routes,
validate Origin and Host, and use a private token. The reference web client
documents its trusted-LAN mode in
[AgentsMesh/YoloShellWeb](https://github.com/AgentsMesh/YoloShellWeb); that mode
is for trusted networks only and must not expose a generic Hub RPC proxy.

## CLI

`yolo plugin` and `yolo plugin list` show the latest registry version alongside
the local state for each ID. A local development link also appears when its ID
has no published spec. Discovery reads versioned specs. Installation downloads
the plugin source and verifies its commit, manifest and digest.

`yolo plugin list --installed` works offline. Existing scripts can use
`yolo plugin list --json` for the same local records. For catalogue JSON use
`yolo plugin list --registry --json`. Each row contains `id`, `name`,
`registry_version` and `installed`. The latter contains the local record or
`null`. Supply a Git URL after `--registry` to use another catalogue.
An incomplete catalogue produces an error. Text output still shows local
records and any usable specs. Catalogue JSON requires a complete result.

```sh
yolo plugin [list]
yolo plugin list --installed [--json]
yolo plugin list --registry [URL] [--json]
yolo plugin describe <id>
yolo plugin link <dir> [--trust]
yolo plugin install <id> [--version <version>] [--registry <url>] [--dry-run] [--json]
yolo plugin unlink <id>
yolo plugin enable|disable <id>
yolo plugin startup
yolo plugin action <id> <action>
yolo plugin status <id>
yolo plugin log <id>
yolo plugin data-dir <id>
yolo protocol describe
```

There is no `yolo plugins` command, no `plugin server`/`plugin client` scopes,
and no pack/verify/sign/install/approve flow.
