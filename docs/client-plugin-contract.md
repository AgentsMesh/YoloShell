# Bundled client contract

The TUI is the only bundled first-party client. It owns rendering, navigation,
local view state and its event loop. It consumes public Hub resources and
terminal streams; it never receives PTY custody or writer authority from a
manifest.

## Ownership

`clients/tui` contains the TUI implementation. `clients/tui/descriptor` owns its
bundled descriptor and Hub API contract.
`core/crates/application/protocol/cli` owns `yolo protocol describe`.

`core/crates/plugin/api` contains shared API/platform declarations used by the
TUI and trusted plugin manifests. `core/crates/plugin/client/contract` contains
descriptor admission and pure capability negotiation. `core/crates/plugin/client/host`
validates the built-in descriptor and starts the bundled TUI. External trusted
plugins use `core/crates/plugin/trusted` and the local `yolo plugin` registry;
they do not enter this built-in client host.

Provider declarations live in `core/crates/plugin/provider/catalog` and are
compiled into the image. They are not plugins and have no process lifecycle.

`//build_system/architecture:client_boundary` traverses the configured Bazel
dependency graph and rejects production core components that transitively depend
on client implementations or terminal UI libraries. The executable composition
root is the explicit exception.

## Admission

The TUI obtains a live Hello before opening local view state or entering raw
mode. Its API contract declares startup methods and optional feature groups.
Protocol mismatch stops startup; optional features are projected independently.
Capabilities describe protocol support, not authorization.
