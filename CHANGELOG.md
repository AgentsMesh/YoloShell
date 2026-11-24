# Changelog

All notable changes to Yolo are recorded here. The project follows semantic versioning once a stable release exists.

## 0.6.8 — Alpha

- Managed systemd upgrades can inspect and atomically replace root-owned unit and drop-in files through the same privileged I/O path.

## 0.6.7 — Alpha

- Candidate state validation uses an independent failing fixture, so Linux upgrades do not rewrite an executable that is still running.

## 0.6.6 — Alpha

- Upgrades now let the installed candidate activate managed systemd profiles, so systemd Host and Hub services use the new deployment backend during `yolo update`.

## 0.6.5 — Alpha

- Machine imports now keep observation separate from Lifecycle control across TUI and Web, including old peers that cannot report route capabilities.
- Managed systemd profiles reconcile effective unit and drop-in images by the socket-owned Host and Hub roles, preserve atomic activation for root-owned units, and reject unmanaged overrides.
- Installers enforce target-user ownership through ordinary-user, sudo and `sudo -H` paths, and trusted plugin startup resolves Bun from managed runtime locations.

## 0.6.4 — Alpha

- Federation exposes route capabilities separately from Observe reachability so clients can disable creation until Lifecycle rights are available.
- Web shell and agent creation preflight the selected machine's mutation route and report an unavailable route before dispatching a mutation.

## 0.6.3 — Alpha

- Federation accepts normal cross-machine clock drift during signed peer discovery, calls and terminal streams.
- Configured machine reachability follows an installed Observe route instead of only an authenticated transport link.
- Web machines distinguish Connecting, Stale and Unreachable states, and observation reads no longer imply mutation routing is ready.

## 0.6.2 — Alpha

## 0.6.1 — Alpha

- Configured machines appear in Hub inventory before their first authenticated catalogue is available.

## 0.6.0 — Alpha

- Drag to copy terminal selections. Paste a desktop file or image with `Ctrl+B ]`, or choose a path with `Ctrl+B u`. The Hub stages the bytes on the machine that owns the terminal.
- Plugins can use `chat_v1` to read managed agents, send prompts with attachments, answer interactions and interrupt a turn. Remote prompts execute at their origin Hub.
- Linux Workers can retire drained terminals when the login environment prevents a complete process scan. Host recovery also finishes pending cleanup of archived attachments while preserving terminal history.

## 0.5.8 — Alpha

- The plugin install command installs and enables the selected registry plugin in one invocation. Use the dry-run option to inspect the verified source, commands and digest without changing the local plugin registry.
- Existing trust install invocations remain accepted for compatibility. The normal help no longer presents that extra confirmation step.
- Registry discovery and local plugin state keep their separate meanings. Offline inventory and catalogue JSON behavior remain available.
## 0.5.7 — Alpha

- `yolo plugin` and `yolo plugin list` show published plugins alongside local install state. A fresh account can discover `web.client` in the official registry and review it before installing.
- `yolo plugin list --installed` works offline. Existing `list --json` scripts retain their local records. Use `list --registry --json` for the catalogue, or supply a Git URL to browse another registry.
- Plugin help highlights listing, installation, startup and logs. Local development and diagnostic commands remain available through `yolo plugin help <command>`.
## 0.5.6 — Alpha

- With trusted Codex hooks installed, new Bash and Zsh login terminals support `codex --yolo` on the first launch and after reattachment. Managed start and resume choose a compatible local-server command.
- Codex accepts large provider executables and resumed user threads from the shared server. Yolo recognizes Claude Code's versioned native executable and foreground shell replacement.
- Agent and conversation reads use independent request deadlines. Reading a published conversation can proceed during catalogue refresh and still verifies the execution before and after reading.
- Federated run and workspace pagination preserves the origin Hub. Workspace membership follows all result pages, and name lookup handles Unicode and cross-Hub ambiguity.
- The TUI displays long replies and narrow-screen agent details, reuses empty tabs, and forwards terminal control keys. SSH preserves empty arguments and accepts keys with non-UTF-8 comments.
- The updater follows release pages and recognizes installed nightlies. Local build probes preserve healthy services during a transient error. Local-action plugins can declare zero required Hub methods.
## 0.5.5 — Alpha

- Managed Codex launches disable the shared app-server so the terminal keeps a stable provider ownership boundary.
- Codex hook callbacks that run in their own process group without a controlling TTY retain admission through the provider process and callback group evidence.
- Host provider capability probes accept closed launch commands with arguments.
## 0.5.4 — Alpha

- Yolo reads ext4 filesystem UUIDs on Ubuntu 24.04 GA kernels, including Linux 6.8.
- The installer creates private user directories even when the login umask allows group writes.
- Hook installation errors identify the affected configuration file or directory and report its unsafe permission mode.
## 0.5.3 — Alpha

- The TUI can retire the last terminal on a machine in a multi-machine layout. The entry Hub keeps a tab for sharing the layout.
- `yolo terminal screen` reads remote terminals through the Hub that owns the selected terminal, including across multiple hops.
## 0.5.2 — Alpha

- Codex conversations can be read and prompted after a verified turn end.
- Agent runs track terminal exits, recovery into a new runtime and archival. Repeated stops preserve the run revision.
- The machine sidebar shows hook setup guidance when empty and opens a running terminal on the selected machine.
- On Linux, upgrade checks inspect the systemd units that own the selected profile and verify their effective image paths.
## 0.5.1 — Alpha

### Added
- `yolo hook status|install` manages the hooks Yolo uses to observe terminal agents. `install` reviews the managed callbacks, asks for confirmation and preserves user hooks; `--dry-run`, `--yes` and `--json` support automation.
- Codex, Gemini CLI and Grok Build still require trust in the provider's own settings; Yolo never bypasses hook trust.

### Changed
- Provider hook installation no longer exposes the internal revision token or Cap'n Proto output; `yolo internal host hooks` keeps the revision-fenced commands for automation.
## 0.5.0 — Alpha

### Changed
- One root command tree: `yolo --help` lists every command. The shell entry stays `yolo`, `yolo new`, `yolo attach` and `yolo terminals`; other commands are grouped by the noun they operate on: `terminal`, `client`, `layout`, `machine`, `agent`, `server`, `platform`, `conversation`, `workspace`, `worktree`, `protocol`, `plugin`, `update`, `audit` and `recipe`.

### Migration
- `yolo list|screen|archive|terminate` become `yolo terminal ...`.
- `yolo clients|detach` become `yolo client list|detach`.
- `yolo layouts` becomes `yolo layout list`.
- `yolo start|stop|restart|status|version` become `yolo server ...`.
- Old root forms are removed; no aliases are kept.

## 0.4.0 — Alpha

### Changed
- The TUI sidebar is a machine tree with per-machine tabs and panes (tab state
  migrates to schema v6); activating a machine without a terminal starts a
  shell there.

## 0.3.1 — Alpha

### Added
- `Manage machines` has a read-only Consumers page listing the machines
  importing from this Hub, with live link count and connection age.

## 0.3.0 — Alpha

### Changed
- The inventory lists machines, not Hubs: selecting a machine scopes its agents
  and returns to its last terminal.
- `yolo machine add|pin|list|remove|consumers|identity` replaces `hub peer`/
  `hub source`; `--pin-only` replaces `--no-import`. Both machines need 0.3.0
  for `machine identity` discovery.

## 0.2.3 — Alpha

### Changed
- `yolo hub peer join <user@host>` onboards a peer over the user's existing SSH client, discovering and pinning its identity and socket. Peer links use an SSH stdio bridge with no broker profile; the SSH account is the admission boundary and its share policy controls exports.

## 0.2.2 — Alpha

### Fixed
- `yolo update` validates the local state contract before stopping services; `yolo start` explains an early Hub exit and points to `yolo restart` when the running Host belongs to an older image.

## 0.2.1 — Alpha

### Fixed
- Upgrading from 0.1.x migrates the Hub protocol contract; newer-protocol state
  is rejected instead of leaving the local Hub unable to start.

## 0.2.0 — Alpha

### Changed
- Hub peer links use SSH instead of a TCP listener and inner SSH server: each
  Hub publishes `<hub-state>/peer.sock`; a signed `PeerHello` binds the
  configured identities. Pins, imports and exports stay local.
- `yolo hub peer add` uses `--ssh-profile`/`--remote-socket`; `peer list`
  reports both. Old configurations keep pins, imports and exports; TCP dials
  are dropped.
- Pairing codes, `yolo hub peer invite|join`, `yolo hub listen`, `MintInvite`,
  the ledger, the enrollment server and the inner peer SSH transport are
  removed; 0.1.x TCP peers need an SSH dial.

[0.1 release notes](docs/releases/0.1.md)
