# ADR 0035: Remove the yolo web scoped browser gateway

Status: Accepted (2026-11-03).

## Context

`yolo web` was a Rust, loopback WebSocket transport over `public-client.sock`.
It authenticated an enrolled client identity, enforced an Origin allowlist and a
bearer token, and forwarded public JSON-RPC and terminal frames. It was not a
plugin and never proxied the local management socket.

After the web client extraction, the trusted web client is a user-level Bun
plugin that connects to the same-UID local management socket and serves its own
UI. It does not use `yolo web`. No other in-tree client uses the gateway. The
gateway therefore had no real consumer, while carrying a separate trust path,
configuration surface and test suite.

The project is pre-release and ADR 0023 explicitly accepts that the plugin and
client surfaces may change without compatibility obligations.

## Decision

1. Remove the `yolo web` command, its `core/crates/transport/web` library and
   its `core/crates/application/web/cli` entrypoint.
2. Remove the gateway documentation and public-JSON-RPC browser transport
   section. Update README, CONTRIBUTING, AGENTS, component layout and
   communication docs to describe only the trusted-plugin path.
3. Keep `public-client.sock` and its scoped client/access mechanisms. They remain
   used by the TUI remote entry and Hub client access; only the browser gateway
   is removed.
4. Commit the web client to the trusted-plugin model: a user-level process with
   local management authority, linked with explicit trust. A future scoped
   browser transport requires a new ADR and a real consumer.

## Consequences

- The core repository no longer ships a browser-facing WebSocket transport. It
  has one fewer public command, trust path and test suite.
- A browser client cannot connect to Yolo without the trusted plugin's local
  management authority. This is the accepted security model; the trusted plugin
  must keep the management socket and token away from the browser.
- Existing scripts using `yolo web` must stop or use the trusted plugin. This is
  a pre-release CLI removal; no compatibility shim is provided.
- `public-client.sock`, client identity enrollment, Hub pinning and scoped
  access remain for the TUI remote entry and future non-browser clients.
- Reintroducing a scoped browser transport is a new architecture decision, not a
  restoration of a dormant directory.
