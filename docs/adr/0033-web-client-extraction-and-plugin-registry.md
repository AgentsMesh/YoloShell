# ADR 0033: Web client extraction and plugin registry

Status: Accepted (2026-11-03). Amended by
[ADR 0034](0034-mechanisms-clients-plugins.md), which relocates the gateway to
`core/crates/transport/web` and the TUI to `clients/tui`; amended again by
[ADR 0035](0035-remove-yolo-web-gateway.md), which removes the gateway.

## Context

The trusted web client is a Bun/TypeScript program with its own browser UI,
server, tests and release cadence. The core repository is Rust/Bazel. Keeping the
client in `plugins/web/client` made the core carry TypeScript tooling, a vendored
browser emulator and a web-specific source-policy exception, and forced the core
release pipeline to build and test code that ships independently.

`core/crates/transport/web` is different: it is the Rust `yolo web` command, a scoped
WebSocket transport over `public-client.sock`. It is compiled into the `yolo`
binary and is not a trusted plugin. The gateway stays in the core repository.

Plugins currently install by linking a local directory with
`yolo plugin link <dir> --trust`. There is no remote plugin registry or install
command. Versioned distribution metadata needs a separate home that can grow
without changing the core repository.

## Decision

1. Move `plugins/web/client` and `plugins/web/sdk` to the public repository
   `AgentsMesh/YoloShellWeb` with the layout `client/` and `sdk/`. The relative
   SDK imports and plugin manifest stay unchanged.
2. Publish the moved code as a clean public history, not the canonical internal
   history. The source tree is the authority; the core repository keeps its own
   history.
3. Keep `core/crates/transport/web` and `yolo web` in the core repository. They are a
   built-in scoped browser transport, not a plugin, and do not move with the
   client.
4. Remove the web client, SDK, vendored xterm.js assets, web-specific source
   policy exceptions and the core `web` CI job. The core repository becomes
   Rust-only; browser assets and Bun tests are validated in `YoloShellWeb`.
5. Create `AgentsMesh/YoloShellPlugins` as a CocoaPods-Specs-style registry.
   Each version is an immutable `Specs/<id>/<version>.json` containing the exact
   `yolo-plugin.json` manifest and the source repository, path, tag and commit.
   Validation checks identity, version, path, commit and manifest consistency.
6. Keep remote installation out of scope for this decision. The registry is the
   versioned distribution index; `yolo plugin` continues to link local
   directories until an install command and its security review exist.
7. Point the core README, contributor guide, plugin docs, site and ADRs at the
   new repositories. Historical ADR paths remain as design records.

## Consequences

- The core repository no longer builds or tests TypeScript, and its source
  policy has no web vendor exception. The architecture gate becomes simpler and
  the core CI loses the Bun job.
- The web client can version, test and release on its own cadence. Its CI runs
  Bun tests and browser/server builds on Linux and macOS.
- Users who previously linked `plugins/web/client` from a core checkout must
  clone `YoloShellWeb` and link `client/` there. The core README documents the
  new command.
- The plugin registry records manifests and source pins but is not yet consumed
  by an installer. A future install path must verify the source at the pinned
  commit, revalidate the manifest and preserve the explicit trust step.
- The gateway remains the only scoped browser transport in the core repository;
  the trusted client continues to use the same-UID management socket and must
  never expose it to the browser.
