# Changelog

All notable changes to Yolo are recorded here. The project follows semantic
versioning once a stable release exists.

## 0.4.0 — Alpha

### Changed
- The TUI sidebar is a machine tree with per-machine tabs and panes (tab state migrates to schema v6); activating a machine without a terminal starts a shell there.

## 0.3.1 — Alpha

### Added
- `Manage machines` has a read-only Consumers page listing the machines importing from this Hub, with live link count and connection age.

## 0.3.0 — Alpha

### Changed
- The inventory lists machines, not Hubs: each machine runs one Yolo Hub; selecting a machine scopes its agents and returns to its last terminal.
- `yolo machine add|pin|list|remove|consumers|identity` replaces `hub peer`/`hub source`; `machine consumers` lists importing machines; `--pin-only` replaces `--no-import`. Both machines need 0.3.0 for `machine identity` discovery.

## 0.2.3 — Alpha

### Changed
- `yolo hub peer join <user@host>` onboards a peer over the user's existing
  SSH client, discovering and pinning its identity and socket. Peer links use
  an SSH stdio bridge with no broker profile; the provider's SSH account is the
  admission boundary and its share policy controls exports.

## 0.2.2 — Alpha

### Fixed
- `yolo update` validates the local state contract with the installed image
  before stopping services; `yolo start` explains an early Hub exit and points
  to `yolo restart` when the running Host belongs to an older image.

## 0.2.1 — Alpha

### Fixed
- Upgrading from 0.1.x no longer leaves the local Hub refusing its state: an
  older Hub protocol contract is migrated; newer-protocol state is rejected.

## 0.2.0 — Alpha

### Changed

- Hub peer links use SSH instead of a TCP listener and inner SSH server:
  each Hub publishes `<hub-state>/peer.sock`; a signed `PeerHello` binds the
  configured identities. Pins, imports and exports stay local.
- `yolo hub peer add` uses `--ssh-profile`/`--remote-socket`; `peer list`
  reports both.
- Old configurations keep pins, imports and exports; TCP dials are dropped.

### Removed

- Pairing codes, `yolo hub peer invite`, `yolo hub peer join`, `yolo hub
  listen`, `MintInvite`, the ledger, the enrollment server and the inner peer
  SSH transport. 0.1.x TCP peers need an SSH dial.
- The listener record and `listen` field; the peer socket derives from Hub
  state and is republished for the service lifetime.

## 0.1.7 — Alpha

### Added

- `yolo hub listen <address>` configures and applies the local Hub's peer
  listener. `yolo hub peer invite --host` then defaults the port from it.

### Changed

- `yolo update` checks, installs, restarts and probes admission; `--check`/`--admit`/`--no-restart` select modes, `--install-release` is hidden and `--local-profile` is accepted; `--check` reports availability and `--no-restart` prints the restart step.
- Top-level help lists auxiliary commands and covers update, workspace, conversation, recipe and plugin commands, including plugin `--trust`; one clap tree accepts root options before or after the command.
- `yolo workspace update` uses the resolved revision unless `--expected-revision` conflicts; `yolo worktree create` requires `--trust-repository`.
- `agent send`, `ask` and `prompt` help states the wait and atomicity difference; `hub peer add` is the advanced pre-shared-key path.

### Fixed

- `yolo hub peer invite` names the missing `--host`, `--port` or applied listener.
- Invite expiry is printed as a remaining duration instead of epoch
  milliseconds.
- Auto-start prints a notice only when no local Hub socket exists; startup failures drop the internal child-exit chain.
- Missing profile files or local Hub state produce actionable errors; `yolo audit` no longer exits 0 silently.

## 0.1.6 — Alpha

### Added

- `yolo status`, `yolo version`, `yolo start`, `yolo stop` and `yolo restart`
  expose the local Host/Hub lifecycle; status distinguishes the installed image
  from the running services. `yolo update --restart` activates and admits the
  installed image.

### Changed

- Upgrade refusal now names `yolo restart` as the activation step.
- `yolo stop` and `yolo restart` resolve exact PIDs from private socket
  credentials; Host shutdown leaves Workers attached to their PTYs.
- `yolo restart` preflights state and Linux Host custody before stopping.

### Fixed

- Removed the stale `yolo web` dispatch integration test left by the gateway
  extraction (ADR 0035).

## 0.1.5 — Alpha

### Added

- Hubs pair with a single-use, expiring `yolo1_` code. `yolo hub peer invite`
  mints a signed code; `yolo hub peer join` verifies the inviting Hub's SSH
  host key, admits the joining identity and configures both directions without
  copying Hub IDs, keys or ports. Codes expire in 1–60 minutes, are consumed
  durably, and grant only the minted export and import directions.

### Changed

- Peer-connection diagnostics moved to their own package to keep federation within budget.

## 0.1.4 — Alpha

### Fixed

- `yolo update` resolves the installer symlink before validating the running
  executable, so admission checks work on the standard macOS installation.

## 0.1.3 — Alpha

### Added

- `yolo hub peer add` and `yolo hub peer list` manage federation peers through
  the local entry Hub, with explicit dial addresses, import control and export
  policies.

### Fixed

- A YOLO client running inside a YOLO terminal no longer attaches to its own
  terminal; it shows a status hint and keeps the pane read-only instead of
  recursively rendering itself.
- A terminal snapshot taken on the alternate screen carries the screen mode, so
  reattaching to a full-screen program no longer leaves its content smeared on
  the primary screen.

## 0.1.2 — Alpha

### Added

- `yolo plugin install` replaces an existing link, stops the previous daemon
  when its manifest declares a `stop` action, and starts the new version.
  Reinstalling the same version reuses the verified managed source.

### Fixed

- Codex Stop candidates resolve with the structured turn journal, so a finished
  turn that awaits the user reports Needs input instead of Running.
- A missing plugin runtime names the executable in the startup error.

### Changed

- The Web client keeps a fresh creation in Starting until its first prompt and
  lists terminal-native waiting agents in Needs you (web.client 0.1.2).

## 0.1.1 — Alpha

### Added

- `yolo plugin install` resolves a versioned spec from the YoloShellPlugins
  registry, verifies the pinned source commit and manifest, and installs into a
  private managed directory after the explicit `--trust` preview.

## 0.1.0 — Alpha

### Added

- Persistent TUI terminal tabs with recursive split panes, detach/reattach,
  shared layouts and private focus.
- Per-PTY Workers that own terminal custody independently of the client, Host
  and Hub.
- Agent detection and native resume for Claude Code, Codex, Gemini, Qwen and
  Grok, with provider-specific capability and authority handling.
- Hub federation, remote runtime entry, public JSON-RPC and a trusted plugin
  interface.
- A Bun web client with live xterm.js terminals, Chat transcript projection,
  typed interactions, creation jobs, archive/history and multi-tab writer
  leases.
- GitHub release publishing from GitLab tags and a daily nightly channel,
  with macOS arm64 plus Linux x86_64 and arm64 assets.
- A one-line installer and `yolo update` release checks, digest-verified
  downloads and immutable-image installation.
- Bazel architecture gates, Rust coverage gates, private-PTY acceptance and
  browser end-to-end tests.

### Known limits

- Alpha support targets macOS Apple Silicon and Linux aarch64/x86_64; Linux
  remote deployment remains preview.
- Only Claude Code and Codex are accepted agent targets for the current Alpha.
  Additional provider adapters are experimental.
- Plugins are trusted and unsandboxed by policy. There is no plugin marketplace.
- Plain-HTTP LAN mode is for trusted networks only.
