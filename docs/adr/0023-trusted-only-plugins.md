# ADR 0023: Trusted-only plugins and TUI-only first-party client

Status: Accepted (2026-11-01). Supersedes the server-plugin distribution and
manifest-v4 decisions in ADR 0020/0021/0022, and the macOS AppKit client.
Amended by [ADR 0035](0035-remove-yolo-web-gateway.md): the separate `yolo web`
gateway was removed; the trusted-plugin path is the only web-client path.

## Context

The plugin surface had grown into several parallel models: Hub-approved
sandboxed server plugins, manifest roles for client/workflow/provider/server,
built-in client descriptors, a browser gateway with a phantom `yolo.web` plugin
identity, and a separate macOS AppKit client. The product has not shipped, so
there is no compatibility obligation.

The desired extension model is simpler: Yolo owns durable state and exposes one
public JSON-RPC API; a plugin is a trusted user-level program that owns its own
lifecycle and UI. Untrusted, sandboxed extension code is not a product
requirement.

The first-party client is the TUI packaged in the Yolo executable. The macOS
AppKit client, its Swift sources, XCTest/E2E suites, app bundle and release
tooling are removed. macOS remains a supported Yolo/TUI platform.

## Decisions

1. **One plugin type.** There is no server/client/profile taxonomy. Every
   third-party plugin is a trusted process running as the user.
2. **No server-plugin runtime.** Hub plugin store/approval, `.yplugin`, manifest
   v4 roles, Host plugin supervisor/sandbox, native/WASI plugin runners, grants,
   broker, invocation journal, plugin contributions and their Hub methods/events
   are removed. No compatibility reader or migration is provided.
3. **Local plugin contract.** A directory contains `yolo-plugin.json` schema 1
   with identity, platforms, optional one-shot startup, fixed local actions,
   optional web metadata and a Hub API contract. Plugins are linked into a local
   registry; no Hub approval or signature is involved.
4. **Explicit lifecycle.** `yolo plugin startup` runs one hook per enabled,
   platform-matching plugin. The hook must detach and exit within 30 seconds.
   Yolo does not supervise, restart, probe, read PIDs from, or kill a plugin
   daemon. Actions are fixed local commands.
5. **Endpoint trust.** Plugins receive the same-UID local management socket and
   therefore have local-admin authority. The manifest `api` block is client-side
   capability gating, not Hub authorization. A plugin must never expose that
   socket or its token to a browser. Scoped browser access remains `yolo web`
   over `public-client.sock`.
6. **First-party clients.** TUI is the only bundled client. TUI/macOS AppKit
   client descriptors and server-plugin contribution UI are removed. macOS
   core/TUI platform support remains.
7. **Internal providers.** Providers remain compiled-in `ProviderDefinition`
   catalog entries, not plugins, and are not exposed by the plugin registry.
8. **Transport and discovery.** `yolo web` remains a scoped browser transport,
   not a plugin. `yolo protocol describe` remains protocol discovery. The
   `yolo plugins` command is removed.
9. **Bun exception (historical).** At the time of this decision, TypeScript/Bun
   was allowed only under `plugins/web/sdk` and `plugins/web/client`; TypeScript
   obeyed the 199-line limit; JavaScript dependency directories were forbidden
   everywhere. [ADR 0024](0024-web-terminal-streaming.md) defined one narrow,
   vendored-asset exception for the trusted web plugin. The web client and SDK
   moved to `AgentsMesh/YoloShellWeb` in
   [ADR 0033](0033-web-client-extraction-and-plugin-registry.md); the core
   repository now contains no TypeScript.

## Consequences

- A linked plugin has the same authority as the user. Link is an explicit trust
  operation; the trust preview shows commands and the package digest.
- There is no mechanism for sandboxed third-party extension code. Future
  untrusted extensions would require a new, separate design decision.
- Existing server-plugin functionality, server contributions and the macOS app
  disappear rather than remaining as dormant compatibility paths.
- macOS users run the TUI from a terminal; there is no `.app` or AppKit client.
- Providers and the TUI remain compiled into the one Rust executable; the
  browser gateway is the only scoped remote client transport.
- The web client and SDK moved to `AgentsMesh/YoloShellWeb` in
  [ADR 0033](0033-web-client-extraction-and-plugin-registry.md). The Rust
  `yolo web` gateway remains in this repository.
