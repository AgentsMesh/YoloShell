# Native TUI Tab Controls

The Hub owns shared Tab/Pane topology; the TUI persists its local selection and
layout checkpoint. The pure layout component never
creates or terminates processes; application commands coordinate a new Tab or
split pane with a real shell creation transaction. See
[Tab navigation](tui-tab-navigation.md). Terminal bindings keep their
origin-qualified addresses when focus or proportions change.

Normal terminal mode reserves a top Tab strip and one bottom status row. A single pane has no outer
frame or global header; split panes share one-cell internal dividers. Only the
focused pane's adjacent divider segments are highlighted. The focused identity,
input readiness and verified agent activity share the status row. Connection
problems and errors appear there when relevant; help and management views are
explicit actions. The existing-terminal picker is a separate full-area selector;
it closes before terminal input becomes active.

Rendering, mouse coordinates and PTY resize readiness use the same inner-area
calculation, so removing frames does not leave stale terminal dimensions.

## Layout Operations

The existing recursive layout now supports directional focus, resizing the nearest
matching-axis ancestor, restoring balanced proportions, and addressing actual
divider geometry. Pane geometry and divider geometry use the same integer
partition calculation. Ratios remain within the layout's existing bounds.

Developer bindings use the existing Ctrl+B prefix in both terminal and inventory
views: arrow keys choose an adjacent pane; H/L shrink/grow its width; K/J
shrink/grow its height; `=` resets every split to equal halves. Existing `%`, `"`, `o`, `x`
and `d` retain split, cycle, close and detach behavior. Directional focus does not
wrap at the outside edge and ignores zero-area panes.

`z` toggles a focused-pane zoom projection without replacing the owning tree.
`{` and `}` swap the focused pane with the previous/next leaf, wrapping at the
ends. Swaps preserve split shape, ratios, pane IDs, terminal bindings and focused
identity. A zoomed pane uses the available terminal area; hidden dividers cannot
be dragged, and other panes retain their observers and original source sizes.
The resize acknowledgement/snapshot barrier also applies when toggling zoom.

Clicking a pane selects it and requests ordinary input control when needed; it
never takes over another holder. The selecting click and its release do not
pass through to the program. Left-dragging the rendered
divider cell changes that split's ratio relative to its starting position. A click
without movement does not alter the ratio. Layout, window-geometry or view
replacement cancels an old divider drag rather than applying its path elsewhere.

All layout changes use the existing save-before-render client-state path. Custom
ratios and focused pane restore with the same terminal bindings. Pointer capture
and writer capabilities are transient and are never persisted.

## Terminal Mouse Input

The maintained vt100 parser determines the source's current reporting mode.
Encoding supports X10 press, press/release, button motion, any motion, default
single-byte coordinates, UTF-8 coordinates and SGR. Out-of-range coordinates and
unsupported modifiers are explicit encoding errors, never wrapped values.

Mouse reports require a current writer grant and a synchronized source surface
whose dimensions match the displayed pane. Press/drag/release capture is bound to
the original pane, runtime, lease generation, principal and capability. Renewal
does not break capture; takeover, disconnect or replacing the target does. Releases
outside the pane are clamped to the original pane, not sent to another terminal.

Observers never answer source mouse requests. Their wheel scrolling is local
scrollback, as is wheel scrolling when the source has mouse reporting disabled.
Keyboard input returns a writing surface to its live screen. Mouse capture and
bracketed paste modes are enabled/restored with the native terminal screen guard.
A fresh press can restart capture after a release is lost outside the outer
terminal. Outer focus loss and terminal resize cancel transient captures.

## Per-Terminal Runtime Facts

Offline/ended state, observer identity, resynchronization, geometry
synchronization, pending and in-flight resize, renewal intent, last size and the
Codex-open cooldown live in one aggregate keyed by TerminalAddress. Retiring a
runtime removes that aggregate, detaching a pane clears binding-scoped facts
while retaining offline/ended history, and disconnecting clears
connection-scoped facts while retaining screens and catalogue history. New
per-terminal facts belong in this value so no transition can omit their cleanup.
