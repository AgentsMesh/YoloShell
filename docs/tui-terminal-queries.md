# Native TUI Terminal Queries

The native TUI answers standard status, cursor-position and primary device
attribute requests from new live terminal output. Replies use the source
session's existing writer grant, regardless of keyboard focus.

| Query | Reply |
| --- | --- |
| CSI 5 n | CSI 0 n |
| CSI 6 n | CSI row ; column R |
| CSI c or CSI 0 c | CSI ? 1 ; 0 c |

The primary attribute reply declares VT100 without optional hardware features.
Private CPR, secondary DA and other unsupported queries remain unanswered.
CPR uses the actual parser position at the query's final byte, including DEC
origin mode, scrolling margins, saved/restored origin, alternate grid and pending
wrap at the last column. Display scrollback does not change live report position.

## Parsing and Authority

The screen remains in vt100; a maintained vte parser checks whether a query began
within the current reply scope. Both parsers must recognize the same query at the
same final byte. Ignored CSI, extra/subparameters, intermediates and query-looking
text inside a control string do not produce input.

The scope binds exact runtime, observer connection generation, control connection
generation and writer authority. Renewal of the same unexpired writer preserves
a legitimate partial query. Takeover, release, expiry, subscription replacement,
resync, loss of authority or removal from the layout retires it. The screen parser
is never reset merely to clear reply eligibility, preserving partial UTF-8 and
terminal control strings across output chunks.

Snapshots and replay chunks are always quiet. Only the generation-validated live
observation path can produce replies. Input waits for current source geometry;
the focused pane shows Syncing until its screen and confirmed dimensions match
the current viewport. An old cached Control label is not synchronization proof.

Each output event produces at most 16 replies and 96 bytes. The application bounds
pending operations separately. Each queued reply retains its original request ID,
writer grant and expiry plus a cancellable local scope. The actual control-channel
owner checks that scope and deadline immediately before dispatch. A queued stale
reply is discarded without I/O; post-dispatch uncertainty is not a retry permit.
Lost replies, queue pressure and reconnects never trigger delayed replay. A late
failure from a replaced writer cannot remove the current writer.

## Dependency Contracts

Repository-owned Bazel patches apply only to exact pinned dependency versions;
extracted repositories are not edited. vt100 0.16.2 adds the public one-based
report_cursor_position API using its own existing grid state, so the application
does not duplicate cursor/margin/origin semantics.

vte 0.15.0 bounds retained raw OSC payload to 65,536 bytes. Parameter separators
do not consume raw payload space. An oversized OSC keeps its original parsing
state until termination/cancellation, but the entire command is discarded rather
than dispatching truncated title or clipboard data. The no_std bound is the
smaller of its static buffer size and this cap; the default remains 1,024 bytes.
This protects both the eligibility parser and the worker's terminal parser.
