# ADR 0032: Archived terminal history

## Status

Accepted, 2026-11-02.

## Context

Archiving a terminal already retains its durable session directory: the
manifest is marked `archived`, the Worker has settled, and the output journal
and last snapshot remain on disk. The public `list` command can include archived
rows, but there was no read path for their retained output. The Web client could
archive finished sessions yet their history disappeared from every surface.

The live test machine accumulated 191 archived and 62 finished sessions. A
history view must read one exact archived runtime without reviving a Worker,
without exposing the Host filesystem, and without loading an unbounded journal
into the Hub or browser.

## Decision

### Public contract

Add an additive, rights-gated public method:

```text
read_terminal_history {
  request: {
    schema_version: 1,
    target: { address: { hub_id, terminal_id }, runtime_epoch },
    max_bytes?: u32
  }
}
```

Access is `TerminalRead` and the mutation class is `ReadOnly`. The reply is one
bounded record:

```text
terminal_history {
  target, data, from_sequence, next_sequence, truncated
}
```

`data` is standard base64 of the retained raw terminal output stream. The
default budget is 2 MiB and the maximum is 4 MiB. The recipient replays the
stream into its terminal emulator; it is not a structured transcript and it is
not interpreted by the Hub. Archived rows are the only accepted targets: live
history remains the `attach` history seed, so the same stream has one owner per
runtime lifecycle.

### Host boundary

`read_terminal_history` is served by a `HistoryDispatcher` inside client-access
authorization, beside the provider projection. It delegates every unrelated
command. The Hub calls the Host over a new `read_retained_history` wire method;
the Hub never reads Worker state directly.

Authenticated peer-origin calls bypass the public client-access chain. The
federation therefore dispatches them through a `yolo_hub_dispatch::
OriginDecorator` that wraps the local backend with the same History and
Provider decorators, rebuilt for every configuration generation. A peer origin
thus serves `read_terminal_history` and `list_providers` under the peer
grant's rights, while local topology disclosure remains backend-only.

The Host-owned `yolo_host_retained_history` crate requires:

- the durable manifest for the terminal is `archived`;
- the runtime record exists and names the requested runtime epoch;
- the journal is opened through the namespace and the journal's public recovery
  API, which verifies the retained runtime epoch and record checksums;
- a compacted journal falls back to the retained snapshot; otherwise the
  retained output is replayed from the beginning of the retained range.

A history read may complete bounded journal recovery (an uncommitted tail or
cursor repair) but never appends output. A missing journal, missing snapshot or
unsettled runtime is reported instead of fabricating history.

### Web surface

The live catalogue still excludes archived rows. The sidebar's **Archived**
action fetches them on demand from the selected machine, or from every observed
reachable machine, with a 500-row cap and a per-machine capability check. The
browser receives opaque machine handles and never a raw Hub id. Selecting a row
opens a read-only xterm and replays the raw stream; no lease, control socket or
Hub attachment is created.

Older Hubs without `read_terminal_history` are excluded from the list with an
explicit unavailable note. They keep their live catalogue and lifecycle
compatibility; archive history is an additive capability, not a fallback.

## Consequences

- Archive history is recoverable without reviving a Worker or changing custody.
- Compaction and byte budgets are visible as `truncated`; the client does not
  imply a complete transcript when the retained range is partial.
- Archived history is not searchable, paginated by the UI, or mergeable across
  machines beyond the 500-row cap.
- An archived session whose journal was removed by external cleanup returns
  `not_found`; a session whose runtime evidence is unsettled cannot be archived
  at all.
- The method is additive and capability-discovered, so an old peer remains
  usable with its existing public methods.
