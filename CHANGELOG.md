# Changelog

All notable changes to Yolo are recorded here. The project follows semantic
versioning once a stable release exists.

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
