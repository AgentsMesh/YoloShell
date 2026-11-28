# ADR 0029: One broker per plugin data directory

## Status

Accepted (2026-11-02). Makes the one-broker invariant in
[ADR 0026](0026-web-session-broker.md) enforceable. No Hub or Rust change.

## Context

The web broker owns single-writer state that is not safe to share between
processes:

- the creation intent journal, where a second process could dispatch or
  reconcile the same record;
- the local controller/presence projection, which assumes one authoritative
  decision per terminal inside the entry;
- the instance identity, token and bounded session/actor registries.

The plugin launcher usually guarantees one child, but `server/index.ts` can be
started directly, tools can leave a child alive while writing a new state file,
and a crash can leave a stale identity. Sharing the journal between two
processes would reintroduce duplicate creation and split presence, so the
invariant must fail closed rather than rely on operator discipline.

Cross-broker presence is a different question. Two independently configured
entry URLs can intentionally point at the same Hub. The Hub lease remains the
writer authority, but each broker has its own local controller projection.
Displaying one broker's controller in another broker would require a Hub
presence API or a shared broker; it is not inferred from local state.

## Decision

The broker acquires a private `broker.lock` in `YOLO_PLUGIN_DATA` before
listening:

```text
{pid: pid, owner: uuid, instance: string, at: unix_ms}
```

- The record is written to a same-directory temporary file, fsynced, and
  published with an atomic hard link. An existing link means another attempt
  won the race; the winner's record is complete before the link exists.
- The lock file is mode `0600`; the plugin data directory is expected to be
  `0700`.
- The owner checks the recorded PID. A live PID refuses startup with that PID
  in the message. A dead PID, or an old malformed partial record, is removed and
  acquisition retries a bounded number of times.
- `SIGTERM`/`SIGINT` shutdown releases the lock only when the recorded owner
  matches, so a late shutdown cannot delete a successor's lock.
- A crash leaves the file, but its dead PID makes the next start recover. PID
  reuse can make a stale record look live; the lock is advisory among the
  trusted same-user processes and the operator can remove `broker.lock` after
  verifying no broker is running.

`YOLO_PLUGIN_DATA` absent means an in-memory test/development broker; no lock is
created. The deployed plugin always has a data directory.

## Consequences

- Two broker processes cannot share the creation journal or split local
  presence by accident. The second process exits before opening the Hub or
  listening.
- The lock does not coordinate separate data directories or separate entry
  URLs. Those are separate brokers; Hub leases still arbitrate the terminal
  writer, and cross-broker presence remains a product boundary.
- The lock is not a distributed or security boundary. It runs inside the
  trusted plugin process and protects cooperative local processes.
- Shutdown ordering is: stop accepting sessions, close terminal actors, release
  the lock, exit.
- No Hub protocol, Rust code or service image is required.

## Evidence

Unit tests cover private mode, a second live owner refusal, dead-PID takeover,
fresh malformed-record refusal and stale malformed-record recovery. Live
acceptance started a second `server/index.ts` with the same data directory and
observed exit before listen with `another web broker (pid …) is using this data
directory`; the first broker stayed healthy. Stopping the first broker removed
the lock, and restarting it recreated the lock with mode `0600`. Live acceptance evidence is retained with the source revision.
