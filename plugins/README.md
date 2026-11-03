# Built-in clients and trusted plugins

The first-party client is the TUI packaged in the Yolo executable. It consumes
the public Hub protocol and owns its local UI state. It does not own PTY custody,
agent identity or Hub routing.

- `catalog`: the bundled TUI descriptor and its Hub API contract.
- `discovery`: the read-only `yolo protocol describe` command.
- `tui`: terminal UI implementation and its private control endpoint.
- `web/gateway`: authenticated scoped browser transport to the public Hub endpoint.
- `web/sdk`: Bun/TypeScript public JSON-RPC client used by external web clients.
- `web/client`: the trusted web-client canary plugin.

External plugins are trusted user-level processes managed by the local plugin
registry (`yolo plugin`). They are not sandboxed, Hub-approved, or supervised by
Yolo. They connect to the same-UID local management socket. Providers remain
compiled-in `ProviderDefinition` declarations and are not plugins.

`//build_system/architecture:plugin_boundary` checks that core components do not
depend on client implementations. The executable composition root is the explicit
exception.
