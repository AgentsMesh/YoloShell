# Shared layouts in the TUI

The TUI consumes the public ListLayouts, GetLayout and UpdateLayout operations.
Other clients use the same document contract. A shared document contains
top-level Tabs, each with a Pane split tree and terminal addresses. The TUI
displays the selected Tab. Saving changes to that tree preserves all other Tabs
in the document.

Opening another view of a terminal does not create a new terminal or agent. Public
layouts permit multiple distinct Panes to show the same terminal. Each view has
its own presentation identity and attach state; input still requires the normal
writer grant. Public split proportions range from 0.10 to 0.90 and round-trip
without being silently clamped to different client bounds.

Focus, zoom and Chat drafts remain client-local. Selecting a different Tab does
not copy its draft into another agent execution. Layout persistence never stores
writer grants, process custody, native provider paths or input-delivery authority.

## Revisioned editing

The Hub is the owner of the acknowledged shared document. The TUI keeps a local
candidate and the last acknowledged revision. An update carries a fresh
request identity and the expected revision; the response must match the exact
address, request, next revision and complete candidate content.

A failed or lost update response does not authorize repeating the mutation. The
client reads the document and checks whether that exact update committed. If
another revision won, it preserves the local candidate and reports a conflict.
Reload shared layout discards that candidate in favor of the Hub document. Save
as new layout retains the candidate in a new document, leaving the conflicting
document alone. An unresolved update remains distinct from a confirmed conflict.

The client persists its selected layout, candidate and pending request before
dispatching an update, so application restart can reconcile the same identity.
Private focus or view changes do not increment the shared layout revision.

## Client and resource lifetime

Layout reads and writes are independent of terminal input. Closing a client view
releases its local presentation resources. The Host and Worker continue owning
the terminal, and another client can attach it. Opening an existing shared layout
does not automatically replace empty or unavailable terminal bindings with new
shells.

The public SDK validates directory origin, strict ordering, duplicate identities,
document limits and exact update receipts. The client has no provider filesystem
or PTY owner access through the presentation protocol. Third-party clients can
implement the same interactions using the published Hub contract.

Remote edits are read when opening or explicitly reloading a layout. Refresh
layouts updates the directory. This increment does not add a pushed layout-change
stream or merge concurrent split-tree edits automatically.

See [client resources](client-resource-protocol.md), [client plugin roles](client-plugin-contract.md),
and [Chat view semantics](chat-clients.md). Implementation and test acceptance are
recorded separately; this contract is not a whole-repository release certificate.
