# Changelog

All notable changes to Yolo are recorded here. The project follows semantic
versioning once a stable release exists.

## 0.2.1 — Alpha

### Fixed
- Upgrading from 0.1.x no longer leaves the local Hub refusing its state: an
  older Hub protocol contract is migrated; newer-protocol state is rejected.

## 0.2.0 — Alpha

### Changed

- Hub peer links use SSH instead of a Hub TCP listener and inner SSH server:
  each Hub publishes `<hub-state>/peer.sock`; the broker opens a streamlocal
  channel through an SSH profile. The dialing Hub is the RPC client, and a
  signed `PeerHello` binds the configured identities. SSH grants no export
  rights; `consume`, `exports` and pins remain local policy.
- `yolo hub peer add` takes `--ssh-profile` and `--remote-socket` instead of
  `--host`/`--port`; `yolo hub peer list` reports both.
- Old configurations keep pins, import flags and exports while dropping
  retired TCP/loopback dials. Re-run remote setup or add an SSH dial.

### Removed

- Pairing codes, `yolo hub peer invite`, `yolo hub peer join`, `yolo hub
  listen`, `MintInvite`, the invite ledger, the enrollment server and the
  inner peer SSH transport. 0.1.x TCP peers need an SSH/socket dial.
- The listener-binding record and the `listen` configuration field; the peer
  socket is derived from Hub state and republished for the service lifetime.

## 0.1.7 — Alpha

### Added

- `yolo hub listen <address>` configures and applies the local Hub's peer
  listener. `yolo hub peer invite --host` then defaults the port from it.

### Changed

- `yolo update` now checks the selected release, installs a newer image,
  restarts when needed, and probes admission in one command. `--check`,
  `--admit` and `--no-restart` select the explicit modes; `--install-release`
  remains only as a hidden legacy install-only alias.
- Top-level help lists the auxiliary `agent`, `conversation`, `workspace`,
  `worktree`, `protocol`, `plugin` and `update` commands. These share one clap
  tree, so root connection options are accepted before or after the command;
  unsupported combinations report a command-specific error.
- `yolo update` accepts `--local-profile`.
- `yolo hub peer list` explains its IMPORT/EXPORTS columns and prints guidance
  when no peers are configured.
- Command and argument help now covers `update`, workspace, conversation,
  recipe and plugin commands, including the plugin `--trust` review step.
- `yolo workspace update` uses the revision it just resolved when
  `--expected-revision` is omitted; passing it still rejects a concurrent
  change.
- `yolo worktree create` requires `--trust-repository` at parse time, and its
  name, branch and base options are documented.
- `yolo update --check` prints `update available` or `already current`;
  `--no-restart` reports the `yolo restart` activation step instead of probing
  admission against older running services.
- `agent send`, `ask` and `prompt` help states the wait and atomicity
  difference; `hub peer add` is marked as the advanced pre-shared-key path.

### Fixed

- `yolo hub peer invite` now reports whether `--host`, `--port` or an applied
  listener is missing instead of always asking for both.
- Invite expiry is printed as a remaining duration instead of epoch
  milliseconds.
- Auto-starting local services prints a notice only when no local Hub socket
  exists. Startup failures no longer append the internal child-exit chain.
- Missing profile files and missing local Hub state produce actionable errors;
  `yolo audit` no longer succeeds silently on a never-started profile.

## 0.1.6 — Alpha

### Added

- `yolo status`, `yolo version`, `yolo start`, `yolo stop` and `yolo restart`
  expose the local Host/Hub lifecycle. Status distinguishes the installed image
  from the images the running services reported; `yolo version` complements the
  unchanged one-line `yolo --version` contract.
- `yolo update --restart` and `yolo update --install-release --restart` activate
  the installed image and then run upgrade admission against the restarted
  services.

### Changed

- Upgrade refusal now names `yolo restart` as the activation step.
- `yolo stop` and `yolo restart` resolve exact service PIDs from private socket
  peer credentials; Host shutdown leaves Workers attached to their PTYs.
- `yolo restart` preflights the state contract and Linux delegated Host custody
  before stopping any service.

### Fixed

- Removed the stale `yolo web` dispatch integration test left by the gateway
  extraction (ADR 0035).

## 0.1.5 — Alpha

### Added

- Hubs pair with a single-use, expiring `yolo1_` code. `yolo hub peer invite`
  mints a code signed by the inviting Hub; `yolo hub peer join` verifies the
  inviting Hub's SSH host key, admits the joining identity and configures both
  peer directions without copying Hub IDs, keys, hosts or ports. Codes expire
  in 1–60 minutes, are consumed durably, and grant only the minted export and
  import directions.

### Changed

- Peer-connection diagnostics moved to their own package so Hub federation stays
  within its architecture budget.

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
