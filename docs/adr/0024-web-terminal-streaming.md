# ADR 0024: Streaming web terminal and vendored emulator

## Status

Accepted. This ADR refines the Bun exception in
[ADR 0023](0023-trusted-only-plugins.md). The web client and its vendor assets
later moved to `AgentsMesh/YoloShellWeb`
([ADR 0033](0033-web-client-extraction-and-plugin-registry.md)); paths below are
historical. The separate `yolo web` gateway was later removed by
[ADR 0035](0035-remove-yolo-web-gateway.md).

## Context

The trusted web plugin started as a live viewer: it polled one-shot terminal
snapshots, re-rendered a small ANSI subset, and kept lease state in the browser.
That is enough for a canary, but not for a full interactive terminal. Input, scroll,
mouse, paste, alternate screen, scrollback and multi-client ownership require a
real terminal stream and a real emulator.

The Rust core already provides the terminal model: the Worker owns the PTY and a
vt100 parser, the Hub exposes persistent attachments with snapshot/replay and
`terminal_event` notifications, and the TUI demonstrates a persistent surface
with mode-aware key, mouse and bracketed-paste encoding. The web plugin bypassed
these contracts. The change is therefore primarily a web client and SDK
rearchitecture, not a PTY or Hub rewrite.

## Decision

### Streaming client

A browser terminal uses two fixed WebSockets to the plugin server: an
output-only stream and a control stream. The plugin server runs an output actor
per viewer and a control actor per viewer, with independent Hub channels for
attachment, ordered input, lease/control, queries, conversation watches and
prompts. A slow output consumer cannot delay lease renewal or input. The browser
never receives the management socket, Hub identity, lease grant or a generic Hub
proxy. The fixed token/Host/Origin/CSP boundary remains.

The control actor binds the writer lease to the control WebSocket lifetime,
exposes explicit owner/observer state, and releases control on close. The
browser requests control on explicit selection, lens switch, terminal focus or
the first keystroke while observing, buffering that keystroke until the grant;
page restore, initial auto-selection and inventory refresh remain observers so
opening or refreshing a tab never steals a lease. Terminal output is a stream,
not a polling snapshot; the browser emulator writes bytes in order and sends
input, resize and take-control messages over the control socket.

### Initial state

The Worker snapshot seed uses vt100 `state_formatted()`, which restores the
screen and the input modes (application keypad/cursor, bracketed paste, mouse
mode and encoding) for every vt100-based client. A `seed: "history"` attach asks
for one bounded raw history stream from the first retained output sequence, so a
fresh emulator can reconstruct scrollback, alternate-screen transitions and
modes. The Hub concatenates the retained chunks into one blob, uses the current
runtime size from the Worker-backed runtime record, and falls back to the
snapshot when the journal is compacted or the replay budget is exceeded. Resize
events are not journaled, so replay after a resize interprets the whole stream
at the current size; old wrapping may differ. History and snapshot are mutually
exclusive; both are bounded to 8 MiB.

### Vendored emulator

The web plugin commits the following third-party files under
`plugins/web/client/vendor` (the repository moved to `AgentsMesh/YoloShellWeb`
in ADR 0033):

- xterm.js 6.0.0 (`xterm.js`, `xterm.css`, `LICENSE.xterm`)
- xterm-addon-fit 0.11.0 (`addon-fit.js`, `LICENSE.addon-fit`)
- a manifest with versions, licenses, sources and SHA-256 hashes

The files are self-hosted and served from fixed `/vendor/<name>` routes. They
are not installed from a package manager, loaded from a CDN or placed in a
dependency directory.

This was a deliberate, narrow exception to ADR 0023: only the exact allowlisted
files under `plugins/web/client/vendor` were exempt from the handwritten-line
and frontend-brand checks while the client lived in the core repository.
JavaScript tooling and dependency directories remained forbidden everywhere
else. The client and vendor assets moved to `AgentsMesh/YoloShellWeb` in
ADR 0033.

## Consequences

- The web plugin gains a real terminal emulator with modes, mouse, bracketed
  paste, scrollback, selection and resize; hand-written ANSI/input code is
  removed.
- The source policy gains a small vendored-asset class with positive and
  negative tests. It must remain limited to this trusted plugin; it is not a
  general dependency allowance.
- The browser still cannot reach Hub methods. Only terminal bytes, seed data,
  lease state and resize/control messages cross the plugin WebSockets. Because
  the emulator injects style elements for its DOM renderer, the page's CSP keeps
  `script-src 'self'` but allows `style-src 'self' 'unsafe-inline'`; no inline
  scripts or user-authored styles are accepted.
- The history seed is bounded by the worker journal and the attach budget. A
  compacted journal falls back to the snapshot; this boundary is documented.
- The change is rolled out as the approved plan's M1 (streaming xterm) and M2
  (explicit history seed), with Rust, Bun, policy, conformance and live VM
  acceptance evidence.