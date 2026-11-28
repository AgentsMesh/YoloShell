# Terminal input intent and authority

The TUI keeps the user's desire to control a terminal independently of its
temporary writer capability and output subscription. A transport interruption
must not silently turn an interactive terminal into a permanent observer.

The pure policy is `clients/tui/terminal/control`. It depends on terminal
identity and time, not the local runtime, provider hooks, networking, or keyboard
bytes. `clients/tui/app/src/control` adapts its decisions to scoped Hub
requests. Provider activity, including waiting for approval, does not determine
PTY write permission.

## Independent facts

| Fact | Owner | Lifetime |
| --- | --- | --- |
| Write or observe intent | Terminal control policy | Bound terminal incarnation in this TUI process |
| Writer capability and renewal deadline | Existing writer state | Authenticated grant, until expiry or retirement |
| Output subscription and geometry | Observation/readiness state | Current subscription and snapshot |
| Undispatched keyboard queue | Ordered input Scope | Exact view, connection, runtime and writer |
| Dispatched input completion and interruption | Terminal input Stream | Exact connection, runtime and writer, independent of navigation |
| Acquisition attempt | Control policy and queue scope | Exact request, connection and runtime, with deadline |
| Explicit takeover observation | Separate cancellable queue scope | Latest explicit takeover intent only |

Intent is never restored from saved layout as a writer capability. A newly
launched passive inventory remains passive. A live process's confirmed write
intent survives a control connection restart. First acquisition intent may
survive only while its original view, deadline, connection and exact runtime
still match; navigation or runtime replacement cannot revive it.

## Recovery and cancellation

The periodic reconciler renews existing writers and performs ordinary Acquire
for desired terminals without a valid writer. It coalesces pending attempts,
uses bounded retry intervals and deadlines, and checks exact runtime identity.
Contention retries never use TakeControl. Authorization denial stops automatic
acquisition until a fresh explicit request.

An output-only interruption pauses input readiness and cancels output-query
scopes without revoking an otherwise healthy writer. Output reconnection can
therefore restore usability without acquiring a different lease. If user input
was unconfirmed, its ordered scope is cancelled and the writer is retired;
recovery requires a fresh grant. Neither ordinary keys nor approval Enter are
replayed. The user must press new keys after recovery.

Navigation retires the old view queue but retains completion responsibility for
input already dispatched. Every view of the same writer shares a transport
failure fence. The serial dispatch owner closes that fence before publishing a
failed attempted input's receipt or dispatching a suffix, even if the UI has not
consumed the receipt yet. Known-unsent view cancellation retires only that view.
Navigation and management controls remain readable while a receipt is pending;
the bounded waiting slot is reserved for actual queue pressure. Only an explicit acquisition
with a genuinely new writer grant can replace that uncertain stream. A late old
receipt cannot affect that new grant, another terminal, or a replacement runtime.
Catalog replacement retires the old stream and its matching view scope together.

`Ctrl-B r` records explicit Observe intent and cancels queued acquisitions and
takeovers. A late grant is released unless it is an already admitted duplicate;
a stale renewal cannot replace a writer or clear a newer renewal's pending
marker. `Ctrl-B a` requests ordinary control. `Ctrl-B t` first obtains the
current observed generation in its own cancellable scope, then issues an
explicit fenced takeover. A cancelled observation cannot turn a later ordinary
Acquire into TakeControl.

Policy presentation distinguishes `Restoring input`, `Input control occupied`,
`Input access denied`, and intentional `Read Only`. Contention alone cannot
identify the holder: after reconnect it may be this client's unexpired old lease.
The UI therefore does not claim another client exists without evidence. These labels replace
the old undifferentiated fallback and are not repeated as stale status messages
after control recovers.

## Verification boundaries

Deterministic unit tests cover retry timing, missing replies, old request and
connection generations, replacement runtimes, explicit Observe, delayed
takeover observations, and unconfirmed input cancellation. App tests drive
connection loss through a fresh grant and new input without picker navigation.

Real process tests use private PTYs and restart an isolated Hub while retaining
its Host, Worker and shell. They compare exact captured input and process
identities, including recovery while another client holds the writer and after
that client releases it. Live user terminals are not fault-injection fixtures.
