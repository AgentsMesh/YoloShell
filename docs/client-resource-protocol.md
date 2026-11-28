# Client resources and conversation projections

Yolo clients consume the public Hub protocol. The built-in TUI, native Mac
application and external clients do not obtain a PTY or provider authority by
declaring a plugin manifest. The single `yolo` executable remains the product
entrypoint. Host and Worker process isolation remains independent of UI lifetime.

## Resource ownership

A Terminal is a durable shell resource; RuntimeTarget identifies one PTY execution.
An AgentTarget identifies one agent execution and its origin Hub.
A ConversationAddress identifies a native conversation within
an origin and provider data-home scope; it does not change merely because an agent
resumes in another execution. A ConversationBinding associates a conversation
observation with one exact AgentTarget and RuntimeTarget.

Tab/Pane layout documents belong to their declared Hub. A document contains
tab identities, titles, split trees and terminal bindings. Focus, scrolling,
zoom, pending input and writer leases are client state. Updating or closing a view
does not terminate a Terminal. Shared documents do not restore control authority.

The TUI stores its chosen LayoutAddress and last acknowledged shared projection
alongside private presentation state. `yolo layout list [--json]` lists layout
summaries; `yolo --layout <UUID>` selects an existing document at the entry Hub.
Default client instances retain independent document identities. Explicitly
selecting a missing layout fails instead of creating a substitute.

## Public operations

| Operation | Required right | Result |
|---|---|---|
| ListLayouts | PresentationRead | Bounded, sorted summaries; no split trees |
| GetLayout | PresentationRead | Exact document or absence |
| UpdateLayout | PresentationWrite | Revision CAS result, correlated to the request |
| ResolveConversation | ConversationRead | Native descriptor for an exact execution binding |
| ReadConversation | ConversationRead | Bounded page or matching checkpoint |
| WatchConversation | ConversationRead | Conditional page read with a bounded wait |
| AgentControl: SubmitPrompt | AgentControl and TerminalWrite | Origin-controlled delivery receipt |

Layout summaries contain address, revision, tab count and pane count. GetLayout
returns the full tree. A lost UpdateLayout response is reconciled by reading its
document and last request identity; stale revisions never overwrite a newer edit.
An empty layout is retained and consumes a document slot; layout deletion is not
part of this protocol increment.

Read-response authorization is checked again after asynchronous work at the
federation origin, intermediate Hub and entry Hub. Revocation or expiry while a
conversation watch is pending prevents the content from being returned. FileRead
does not substitute for ConversationRead or PresentationRead.

## Conversation content

The origin verifies native run binding before and after reading. Clients never
provide filesystem paths or scan provider stores. Codex and Claude readers use
their existing anchored evidence handles. Transcript content does not update
hook-derived lifecycle, custody, control authority or recovery eligibility.

Content entries have stable IDs and explicit roles. Text, recorded thinking,
tool calls/results, compaction and notices remain distinct. Tool call IDs link
results across pages. Unsupported records and unavailable images are explicit;
terminal text is not silently promoted to a native conversation record.

Pages declare history generation, content revision, availability, partial reasons
and an older-page cursor. Cursors belong to a conversation and history generation.
Conditional reads carry both since_generation and since_revision. Unchanged
responses must match both. Replacements, truncation and detected rewrites
invalidate old history; native clear/branch records affect projection boundaries.

Reading fixes a prefix length at its start. The opened object and native session
identity remain verified. If metadata changes while reading, that same prefix is
hashed again: append-only tails are allowed, changed prefixes are rejected.
Incomplete final records are marked partial and observed again on a later read.
The original strict native custody verification is unchanged.

Each conversation has an independent read lock and bounded cached context. Busy
contexts are never evicted to create a competing history generation. Metadata-
unchanged pages are cached. Changed files still require bounded validation and
parsing; this is not a persisted incremental transcript index.

The Rust SDK is `hub/conversation/client`. The native SDK is
`YoloWorkspace/Conversation`. Both use the same Resolve/Read/Watch operations.
The CLI consumes the current local Hub by default, or an explicit authenticated
endpoint:

```sh
yolo conversation resolve --agent <agent-id>
yolo conversation --hub <entry-hub> --socket <socket> read --agent <agent-id>
```

`--local-profile <path>` selects a nondefault local profile.

Use `--origin <hub>` for a resource reached through another Hub. Read/watch without
an agent selector take the public request as bounded JSON on stdin, including
older-page cursors or conditional checkpoints. These commands do not bootstrap a
second Host, invoke a model API, or start another agent.

## Input delivery

SubmitPrompt is a new input intent, not a response to an existing Interaction.
The origin checks the exact observed agent execution and actual writer grant.
The Worker checks its native hook witness, current controller/runtime, foreground
process membership, input-ready hook state and bracketed-paste mode.

Prompt text is bounded and excludes terminal control characters except LF and
Tab. Durable intent precedes one nonblocking PTY write. A partial write, interrupted
write or lost proof stays Unknown and cannot authorize replay. Existing operation
receipts resolve repeated requests without submitting again.

Delivered means PTY input delivery, not a new native turn or execution completion.
Input readiness does not prove that a provider's editor is empty or that no native
menu is open. The implementation does not clear native drafts or manufacture a
Confirmed receipt. Interaction responses remain capability-dependent.

## Current limits and development cutover

This increment reads active, authenticated native AgentRun bindings. Independent
conversation discovery after agent exit, asset/blob serving and complete provider
branch reconstruction are not implemented. Unresolved branch evidence is explicit.
Changed-file scanning is capped at 64 MiB. Watch uses bounded conditional page
sampling rather than a durable event stream.

The public Hub schema is 17, Agent schema 2, native control observation schema 3,
desktop control schema 2, and private TUI layout schema 5. Retired development
formats have no compatibility aliases. Validate in isolated state before changing
the installed development services; retain the live PTY custody domain and
existing Worker images until a deliberate contract cutover is possible.
