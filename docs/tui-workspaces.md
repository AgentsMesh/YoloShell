# TUI Client Layout Storage

The TUI keeps private client layout state for tab arrangement, focus and
terminal selection. Hub Workspaces are user-visible through the workspace
picker, create/rename/filter flows and worktree commands; see
[workspaces](hub-native-workspaces.md), [local entry and lifecycle](tui-local-startup.md)
and [controls](tui-interactions.md). The current state schema is required.
Unsupported or foreign state is rejected without rewriting it; no compatibility
layout or alias is imported.

Run `yolo` to open the TUI. On macOS and Linux it can start or reuse the local
Hub automatically. There is no Mac app; the TUI is the only first-party client.
For a repository checkout, use
`bazel run --jobs=1 //core:yolo -- [arguments]`.
The [single-executable contract](single-executable-architecture.md) describes
role dispatch; service roles retain independent processes and are not selected
by executable aliases.

The private client state saves a tab collection with recursive split-pane layouts,
focused panes and inventory/terminal views. Terminal bindings use stable `(HubId, TerminalId)`
addresses, including
resources reached through other Hubs. Missing, unreachable or exited sessions
do not cause automatic creation or resume.

Each entry Hub has an independent private client-state directory. Its exact
location is selected by the local TUI state configuration and is never treated as
a Hub or terminal namespace.

Explicit connection examples are:

```sh
yolo --socket <hub.sock> --hub <HubId>
# Equivalent typed local entry:
yolo --endpoint local:<hub.sock> --hub <HubId>
```

Each TUI client owns its private state. Other protocol clients and TUI instances
remain independent, and state bound to one entry Hub is not silently opened under
another entry.

## Entry Hub and Relay Views

For an authorized resource-import chain A -> B -> C, the entry Hub determines
the catalogue. With one exported Agent on each Hub:

| Connected entry | Visible origins | Distinct Agents |
| --- | --- | ---: |
| A | A, B, C | 3 |
| B | B, C | 2 |
| C | C | 1 |

B manages its own sessions and relays C's exported resources. C's Agent keeps
the same `(C, AgentId)` address at every entry; an additional A -> C route does
not add another Agent. Import direction is independent of which peer initiates
the underlying connection. Reexport requires permission from the origin.

Losing B's route leaves C's known entries stale and unreachable at A; it does
not report C's process as exited or automatically recreate it. Observation,
terminal read, terminal write and lifecycle rights remain independent.

The TUI connects to a local Unix socket using the CLI above. Running it on A, B
or C against that machine's Hub provides the corresponding view. It can also
use a prepared, authenticated remote profile through the direct SSH entry
adapter; that adapter keeps the public client and terminal channels on one SSH
connection while preserving the same entry-scoped view. A bare `ssh://` URI is
rejected because it has no pinned profile or client credential. Linux TUI
release packaging and platform acceptance remain open. The federation contract
is defined in [ADR 0012](adr/0012-federated-hub-protocol.md).

## Tab Layout and Background Terminals

The tab manager stores its tab collection, split trees and focus without creating,
resuming or terminating a Terminal. Background Terminals retain their observers
and current writer leases, including renewals that complete after a layout
change. Only the displayed focused pane requests a PTY resize. Zoom, text
selection, mouse capture and destructive confirmations stay scoped to the client
layout. Existing writer authority stays runtime-only.

Shell creation remembers its original pane, expected binding and
control connection. A delayed success can bind that original pane only while
its expected binding still matches. It cannot overwrite a subsequently selected
Terminal or redirect into another client layout. A removed pane leaves a confirmed
creation available through the Hub catalogue. Ordinary unchanged creation keeps
the new session selected; late completion does not steal a changed UI context.
Definite no-send and failure retire the transient association without retrying a
possibly delivered creation. Fast control reconnect drops writer authority but
retains intent for independently admitted creations until their exact result
arrives; it cannot redirect a late result to another pane. Lifecycle replies only
invalidate the catalogue, so an old reply cannot overwrite newer runtime state.

## Persistence Boundary

The current private client-state document contains the entry Hub, revision and a
validated collection of tab layouts and views. Unsupported, foreign-Hub or
exhausted records are rejected without rewriting them; there is no version-1
upgrade path. The document deliberately excludes writer capabilities, controller identities, runtime
epochs, pending input, creation transactions and destructive confirmations.
Restoration therefore attaches observers without restoring write authority.

Layout/view changes are persisted before their next render. Terminal output and
ordinary redraws do not rewrite unchanged layouts. Files use the existing retained
namespace and exact file-identity atomic-write APIs. A conflicting replacement,
deleted file, namespace change or revision exhaustion makes the current storage
owner unusable instead of overwriting a replacement or generating default state.

Closing the last pane bound to a Terminal in a client releases that client's
writer and stops renewals.
Late acquisition/renewal replies cannot restore an unbound writer, and late
takeover observations cannot start new acquisition after the last pane closes.
It does not terminate the managed shell or Agent. Normal TUI exit also releases
leases; the origin lease deadline remains the fallback when release cannot arrive.
