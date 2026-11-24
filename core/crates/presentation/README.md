# Client presentation boundary

`layout` owns reusable split trees, geometry and client-local pane selection.
It has no process, provider or persistence dependency. Its `Layout` includes
one client's focused pane; that value is never part of shared topology.

`protocol` owns shared layout documents addressed by `(HubId, layout_id)`.
A document contains Tab identities, display titles, split trees and Pane to
TerminalAddress bindings. Focus, scrolling, writer leases and PTY size are not
shared layout facts. A terminal may appear in several independent views.

`LayoutUpdate` replaces one bounded document using `expected_revision` and
`request_id`. Expected revision zero creates an absent document. Every accepted
update advances the revision and records its request id. A stale request is a
conflict. After a lost reply, read the document and inspect `last_request_id`;
never assume a failed connection means the mutation did not execute.

`store` owns durable CAS and retained directory/file authority. Its public
operations are `open(PrivateDirectory, HubId)`, `list()`, `get(LayoutAddress)`
and `update(LayoutUpdate)`. The Hub supplies authentication, authorization and
dispatch before invoking it. The store never calls terminal lifecycle code.
Closing the last Tab produces an empty document and leaves every Terminal
and AgentRun alive.

Protocol bounds and canonical non-nil identities apply on deserialization.
Tab and Pane identities are unique across a document, including split leaves
in different Tabs. Persisted corruption, foreign Hub ownership, duplicate
writers, directory replacement and file replacement fail closed.

The TUI uses `layout` directly. Its private presentation catalog exports through
`shared_tabs()` and imports through `from_shared()`, preserving surviving
client-local focus and view mode. An empty document has a local placeholder;
that placeholder does not create a terminal or shared resource by itself.
