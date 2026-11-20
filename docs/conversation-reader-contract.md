# Native conversation reader

The conversation service projects an authenticated agent's native transcript. It never
changes lifecycle state, native hook authority, input leases, or the provider's files.

## Component boundaries

| Component | Responsibility |
| --- | --- |
| `conversation/protocol` | Public addresses, run associations, entries, parts, page limits and errors |
| `conversation/transcript` | Bounded JSONL framing, prefix hashing and bounded page selection |
| `agent/providers/codex/parser` | Codex native `response_item` and runtime record projection |
| `agent/providers/claude/parser` | Claude native message, tool, compaction and notice projection |
| `conversation/service` | Exact binding/evidence validation, generation tracking and read orchestration |

The protocol depends on shared Hub identities rather than runtime implementations.
Provider adapters produce content only. A `ToolResult` is a separate entry carrying its
`call_id`, so a result can refer to a call on an older page without rewriting that page.

## Origin integration

Keep one `ConversationReader` for the origin's lifetime. `resolve(&AgentBinding,
ConversationBinding)` returns a descriptor. `read(&AgentBinding,
&ConversationReadRequest)` returns a page or `Unchanged`.

The Hub authenticates and authorizes the exact `AgentTarget` and `RuntimeTarget` before
passing its native-authority-validated binding to the reader, and revalidates that
association before publishing a result. The caller must not accept a client-provided
`AgentBinding` as authority. The reader confirms the terminal identity and origin,
native session, cwd, data home and evidence path again.

Both existing provider evidence implementations now expose
`with_transcript_snapshot`. The closure receives the pinned descriptor and the
file length captured at the start, not a path to reopen. The service reads only
that prefix. Native session identity and the opened file's ownership and path
association are checked on each side. If metadata changes during a content read,
the service hashes the same prefix again and compares it with the parsed prefix.
An append after the captured boundary is allowed; prefix changes, truncation and
path replacement cannot publish a mixed snapshot. The original strict custody
checks remain unchanged; the append-aware object check is a separate read boundary.

Public reads currently require an active, authenticated native AgentRun and its
exact Terminal runtime. The stable ConversationAddress does not yet provide
independent history discovery or reading after that agent execution exits.

## Identity and pagination

A conversation ID derives from provider, native session and data-home scope. It is
independent of execution ID, runtime epoch and current agent state. Its address also
names the origin Hub. A descriptor associates that conversation with exact executions.

A history generation persists across verified appends. Replacement, shortening,
changed earlier bytes, or a native clear/rollback boundary produces a new generation.
The reader hashes the old prefix as well as the current snapshot: rewriting an old
record and then appending does not pass as an append. A cursor names its conversation,
generation and an exclusive older offset; a mismatched cursor yields `HistoryChanged`.

Without a cursor, the service returns the newest bounded page in chronological order.
`next_cursor` addresses older entries. Entry IDs are deterministic within a generation
and byte position. Page revisions include the query and association; conditional reads
return `Unchanged` only for the same validated query and evidence. A conditional
request supplies both `since_generation` and `since_revision`; the response must
match both, including after a cache miss.

Checkpoints are memory-only and capped at 64 retained conversation contexts. Each
context has its own read lock and caches one bounded page. The registry lock only
selects or retains a context; scanning another conversation does not hold it.
Active contexts cannot be evicted or recreated concurrently. When all contexts
are busy, a new conversation returns `ReaderBusy` rather than exceeding the bound.
Restart or idle-context eviction invalidates old cursors safely. Stable conversation
addresses survive these events; clients refresh the current page on `HistoryChanged`.
Unchanged metadata permits cached reads. Cache stamps represent the captured prefix,
so data appended during a read remains visible to the next read.

## Explicit limits and current capability boundaries

- Pages default to 64 entries and 256 KiB, capped at 256 entries and 1 MiB.
- `WatchConversation` is one conditional page read with at most 1,000 ms of polling
  wait. It returns a page or unchanged checkpoint; it is not a durable event stream.
- Framing retains at most one 1 MiB JSONL record, plus one projected page.
- Scans read 64 KiB chunks and have a 64 MiB total history limit. Larger files return
  `HistoryLimitExceeded`; an indexed incremental history store is not implemented here.
- Inline text is bounded at 16 KiB with an explicit truncation marker. No full-output
  or image asset is implicitly exposed. Images currently return `ImageUnavailable`;
  publishing verified opaque blob references is a separate capability.
- Incomplete trailing JSON is not published before a newline. Oversized, malformed
  and unsupported records produce explicit partial reasons.
- Codex `response_item` is the canonical message carrier; duplicate `event_msg`
  message echoes are ignored. Provider versions that only record event messages
  require a separate declared adapter capability.
- Codex rollback clears the projected prefix and marks `UnresolvedBranch`; the reader
  does not invent a reconstructed branch or follow guessed parent files. Claude
  sidechains are excluded and marked unresolved. Cross-file history chains are not
  yet resolved.

These limits describe read availability, never agent execution state. An unreadable
conversation does not make an otherwise healthy agent unavailable for terminal use.

## Validation targets

Unit tests are separate from production source, and real filesystem/anchored-evidence
tests live in `service/integration_tests`. The six targets are:

```text
//core/crates/conversation/protocol:unit_tests
//core/crates/conversation/transcript:unit_tests
//core/crates/agent/providers/codex/parser:unit_tests
//core/crates/agent/providers/claude/parser:unit_tests
//core/crates/conversation/service:unit_tests
//core/crates/conversation/service:integration_tests
```

Adversarial cases include malformed and oversized records, split UTF-8, partial tails,
identity and origin mismatch, stale cursors, truncation, replacement, same-inode
rewrite-plus-append, clear boundaries, symlinks, hardlinks, continuous append during
a prefix read, independent read locks and pinned-context eviction refusal. Public
SDK, CLI and Hub integration tests are additional targets. Ubuntu execution and
measured coverage belong in the integrated validation record.
