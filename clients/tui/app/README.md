# TUI application composition root

`clients/tui/app` is the single TUI composition root. It owns the event loop,
terminal lifecycle, input routing, rendering, status, and inventory views, and
wires the client crates from `clients/tui/*` plus Hub client access.

Decisions:

- Do not extract another TUI crate now. The app crate is the composition
  boundary; only real cross-app consumers justify a new crate.
- Keep rendering and input modules separate by feature; do not add a generic
  reducer layer speculatively. Split an explicit reducer module only if state
  transitions outgrow the current modules.
- Module wiring stays in `src/module_wiring.rs`; `src/lib.rs` composes modules
  and public re-exports.
