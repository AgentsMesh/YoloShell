# Changelog

All notable changes to Yolo are recorded here. The project follows semantic versioning once a stable release exists.

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
- `yolo hub peer join <user@host>` onboards a peer over the user's existing SSH
  client, discovering and pinning its identity and socket. Peer links use an SSH
  stdio bridge with no broker profile; the SSH account is the admission boundary
  and its share policy controls exports.

## 0.2.2 — Alpha

### Fixed
- `yolo update` validates the local state contract before stopping services;
  `yolo start` explains an early Hub exit and points to `yolo restart` when the
  running Host belongs to an older image.

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

## 0.1.7 — Alpha

### Added
- `yolo hub listen <address>` configures and applies the local Hub's peer
  listener; `yolo hub peer invite --host` then defaults the port from it.

### Changed
- `yolo update` checks, installs, restarts and probes admission (`--check`,
  `--admit`, `--no-restart`); top-level help lists auxiliary commands and
  accepts root options before or after the command.
- `yolo workspace update` uses the resolved revision unless
  `--expected-revision` conflicts; `yolo worktree create` requires
  `--trust-repository`; agent `send`/`ask`/`prompt` help states their
  wait and atomicity differences.

### Fixed
- Invite expiry prints a remaining duration instead of epoch milliseconds.
- Auto-start prints a notice only when no local Hub socket exists; missing
  profile files or local Hub state produce actionable errors; `yolo audit` no
  longer exits 0 silently.

## 0.1.6 — Alpha

### Added
- `yolo status|version|start|stop|restart` expose the local Host/Hub lifecycle;
  `yolo update --restart` activates the installed image.

### Changed
- `yolo stop`/`restart` resolve exact PIDs; upgrade refusal names `yolo restart`.

## 0.1.5 — Alpha

### Added
- Hubs pair over SSH with a single-use, expiring `yolo1_` code; the provider's account and export policy remain the authority.

## 0.1.4 — Alpha

### Fixed
- `yolo update` resolves the installer symlink before validating the running
  executable, so admission works on the standard macOS installation.

## 0.1.3 — Alpha

### Added
- `yolo hub peer add`/`list` manage federation peers with explicit dial addresses, imports and export policies.

### Fixed
- A YOLO client inside a YOLO terminal stays read-only, and alternate-screen snapshots keep the screen mode.

## 0.1.2 — Alpha

### Added
- `yolo plugin install` replaces an existing link, stops the previous daemon when its manifest declares a `stop` action, and starts the new version.

### Fixed
- Codex Stop candidates resolve with the structured turn journal, so a finished turn that awaits the user reports Needs input instead of Running.
- A missing plugin runtime names the executable in the startup error.

### Changed
- The Web client keeps a fresh creation in Starting until its first prompt and
  lists terminal-native waiting agents in Needs you (web.client 0.1.2).

## 0.1.1 — Alpha

### Added
- `yolo plugin install` resolves a versioned spec from the YoloShellPlugins registry, verifies the pinned source commit and manifest, and installs into a private managed directory after the explicit `--trust` preview.

## 0.1.0 — Alpha

### Added
- Persistent TUI terminal tabs with recursive split panes, detach/reattach, shared layouts and private focus.
- Per-PTY Workers that own terminal custody independently of the client, Host and Hub.
- Agent detection and native resume for Claude Code, Codex, Gemini, Qwen and Grok, with provider-specific capability and authority handling.
- Hub federation, remote runtime entry, public JSON-RPC and a trusted plugin interface.
- A Bun web client with live xterm.js terminals, typed interactions, creation jobs, archive/history and multi-tab writer leases.
- A one-line installer, `yolo update`, digest-verified downloads and immutable-image installation; GitHub releases publish macOS arm64 and Linux x86_64/arm64 assets from GitLab tags.
- Bazel architecture gates, Rust coverage gates, private-PTY acceptance and browser end-to-end tests.

### Known limits

- Alpha support targets macOS Apple Silicon and Linux aarch64/x86_64; Linux remote deployment remains preview.
- Only Claude Code and Codex are accepted agent targets for the current Alpha. Additional provider adapters are experimental.
- Plugins are trusted and unsandboxed by policy. There is no plugin marketplace.
- Plain-HTTP LAN mode is for trusted networks only.
