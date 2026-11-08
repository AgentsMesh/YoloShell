# ADR 0019: Event subscription delivery without a second transport

## Status

Accepted on 2026-10-31. Extends the sequenced platform event stream from the
platform-elevation plan.

## Context

`list_platform_events` already exposes a bounded, sequenced page with an
epoch and `lost` marker. Clients that want to respond to changes still poll it,
so the Hub cannot wake them and every idle client pays the same round trip.

Two delivery shapes were possible:

1. a connection-mode event stream, mirroring the terminal `Attach` switch;
2. a bounded long-poll over the existing request/reply channel.

A connection-mode stream needs a second public framing mode, client and pool
changes, and a peer union before the entry Hub can forward it. The terminal
attach path also consumes the connection for PTY data, which is a separate
data plane and deliberately not shared with metadata events. The plan defers
peer event streams to Phase 4.

## Decision

Add `subscribe_events`, a bounded long-poll over the existing channel:

- The request carries `after` (epoch + sequence, optional), `limit`
  (1..=256) and `wait_ms` (1..=1000); the result carries the same stream
  identity and page fields as the list result plus `timed_out`.
- The event bus notifies waiters on every append. A subscription reads the
  page first and then waits on the notification with a one-second rescan
  fallback, so an append between the read and the wait cannot be missed.
- A page returns immediately when it has events. A `lost` cursor also
  returns immediately (epoch change, cursor ahead, or evicted events); the
  client reconciles with `get_platform_snapshot` and resumes from `next`.
- An idle deadline returns `timed_out` with no events; the client re-issues
  with the same cursor. The Hub holds no subscription state between calls.
- Events remain derived from confirmed public effects only. The subscription
  carries no PTY bytes, no screen text and no lifecycle authority.

## Consequences

- Clients get push-driven delivery with at most one second of added latency
  and no transport, framing, or connection-pool change.
- An idle follower issues one request per second; this is the same order as
  a polling client, but the Hub answers it from the wake path instead of a
  fresh catalogue read.
- Events are delivered in pages, not one frame per event. A future
  connection-mode stream can reuse the same cursor and `lost` semantics; it
  is not required for the current platform surface.
- The TUI keeps its polling fallback until it adopts the subscription.
- Cross-Hub subscriptions are forwarded to the Hub that owns the stream;
  a union stream across peers remains Phase 4.
