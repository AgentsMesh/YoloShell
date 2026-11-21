# ADR 0034: Mechanisms, first-party clients and external plugins

Status: Accepted (2026-11-03). Amended by
[ADR 0035](0035-remove-yolo-web-gateway.md): the web gateway moved here in
decision 5 was subsequently removed.

## Context

After the web client extraction in ADR 0033, `plugins/` contains no external
plugin. It contains the TUI (the first-party client), the TUI descriptor, the
read-only `yolo protocol describe` command and the scoped `yolo web` transport.
All of them are compiled into the `yolo` binary.

The directory name is also an architecture predicate. `plugin_boundary` treats
every label under `plugins/` as a client implementation, so mechanism code placed
there cannot be depended on by core components. The configured-graph gate was
not wired into conformance or CI, and ownership, conformance-data and static
prefix checks used the same literal path. Moving the TUI without updating those
checks would silently remove coverage.

## Decision

1. Use three concepts:
   - **mechanism**: durable runtime and protocol code under `core/`;
   - **first-party client**: the TUI and its descriptor under `clients/tui`;
   - **external trusted plugin**: a user-level process outside this repository,
     with its manifest/registry mechanism under `core/crates/plugin`.
2. Move `plugins/tui` to `clients/tui`. The family includes the terminal UI,
   noninteractive client commands (`audit`, `recipe`) and local bootstrap; it is
   the first-party client family, not a plugin.
3. Move `plugins/catalog` to `clients/tui/descriptor`. It is the TUI's built-in
   descriptor, not a generic multi-client catalog. Keep its separate crate only
   while its real consumers remain.
4. Move `plugins/discovery` to `core/crates/application/protocol/cli`. It is
   read-only protocol introspection, not a client or plugin.
5. Move `plugins/web/gateway` to `core/crates/transport/web` and its CLI to
   `core/crates/application/web/cli`. `yolo web` remains the scoped loopback
   transport over `public-client.sock`; only its location changes.
6. Delete the top-level `plugins/` directory. External plugins live in their own
   repositories: the reference client in `AgentsMesh/YoloShellWeb` and versioned
   specs in `AgentsMesh/YoloShellPlugins`.
7. Rename the boundary gate to `client_boundary` and make it cover `clients/`.
   Activate the configured-graph gate in conformance/CI, add `clients/` to the
   ownership, literal, entrypoint and conformance-data roots, and add negative
   tests. Do not weaken or remove a gate to land the move.
8. Keep `clients/tui/cli` in the client family for this change. It is the
   built-in client's application entrypoint and may use application conveniences;
   a stricter "clients use only public protocol" allowlist is a separate
   decision.

## Consequences

- Directory ownership matches the three concepts. `core/` is mechanisms,
  `clients/` is the first-party client, and external plugins are not in-tree.
- The TUI can be tested and versioned with the `yolo` binary while the plugin
  boundary remains explicit. External plugin trust and the management-socket
  boundary are unchanged.
- The architecture gate is stronger, not weaker: the configured-graph aspect is
  evaluated in CI, `clients/` is included in source/ownership/data checks, and
  stale baselines fail.
- The descriptor now lives at `clients/tui/descriptor` as crate
  `yolo_tui_descriptor`; its TUI-only accessor replaces the multi-client vector.
