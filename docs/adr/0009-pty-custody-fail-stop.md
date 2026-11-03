# ADR 0009: PTY custody activation and fail-stop recovery

## Status

Accepted.

## Context

Yolo transfers secondary PTY and process custody from a Worker to its Host so
the Host can preserve or settle an exact runtime when the Worker disappears.
The transfer crosses two independently durable processes. A transport result
cannot reveal whether the peer observed the last frame, and an established
Unix stream ending does not prove that the Worker process died.

After a Worker observes `Prepared`, it may durably arm the offered lease. The
Host must therefore stop treating a later timeout, EOF, protocol error, or
panic as a reversible connection failure. Repeating the exchange in the same
controller generation could create two conflicting accounts of authority.

The same ambiguity exists after activation. An EOF from the established
control stream can mean a current Worker closed one alias, a current Worker is
temporarily unable to communicate, or the Worker exited. Actively settling the
child tree in the first two cases could terminate a live agent that still owns
the PTY.

## Decision

`Prepared` is the irreversible ambiguity boundary. The Host marks the exchange
at risk before attempting to write `Prepared`, because a failed write may still
have reached the Worker. Every timeout, returned error, contained panic, or
cancelled async operation after that point requests an irreversible Host
fail-stop synchronously. The guard is disarmed only after availability is
published. The first exact diagnostic is retained and returned to rejected
mutations and attachments, while normal Host ingress, watchdog, and maintenance
teardown still runs.

The activation sequence is:

1. The Worker sends an exact offer and custody descriptors.
2. The Host validates them and sends `Prepared`.
3. The Worker durably arms the exact offer and sends `CustodyWorkerCommit`.
4. The Host validates the durable commit and sends `Ready`.
5. `Ready` grants the Worker permission to install its controller generation
   and exact custody authority.
6. The Worker durably promotes that authority and sends
   `CustodyWorkerActivated` for the exact lease, offer, and generation.
7. The Host consumes the opaque exact activation proof, marks wire readiness,
   and starts the control monitor.
8. The Host retires every exact predecessor before publishing availability.

Missing, forged, or mismatched activation leaves wire readiness false, leaves
the monitor unstarted, retains platform custody, and fail-stops the Host. The
Host does not send an `Activated` frame; activation proof flows from Worker to
Host.

An established control-stream fault classifies the exact Worker identity:

| Observation | Host action |
| --- | --- |
| `Current` | Fail-stop and retain platform custody. |
| `Indeterminate` | Fail-stop and retain platform custody. |
| `ProvenGone` | Active exact child-tree settlement is permitted. |

A known different boot proves the Worker gone. On the same known boot, an exact
PID and start-time match is current, exact absence is proven gone, and an
observation error is indeterminate. With unknown boot identity, an exact live
match is current, but absence or an observation error remains indeterminate.
Unknown boot identity never proves absence by itself.

Same-live-Worker EOF intentionally does not settle the child tree. There is no
exact reconnect within the same Host controller generation. The Host exits and
a higher controller generation performs recovery after proving the predecessor
Host holder gone.

Successful exact `Release` and `Supersede` exchanges remain normal. Release
still requires Worker-local alias closure, Host settlement proof, platform
descriptor drop, and the exact acknowledgement in that order. A malformed
control, premature release, acknowledgement mismatch, timeout, transport
fault, or monitor panic follows the same Worker classification policy.

## Consequences

The system prefers a bounded Host restart over risking concurrent custody or
terminating a current agent. Availability may pause while a higher generation
takes over, but a current or indeterminate Worker cannot be mistaken for death
solely because its stream ended.

Recovery does not promise continuation of the same PTY after a Worker crash.
The old runtime epoch is settled only under exact authority. Yolo then uses the
agent provider's durable resume mechanism when available, or recreates a
generic shell session when no provider resume contract exists.

Retained platform descriptors can remain live until the Host process exits or
a proven-gone settlement completes. This is intentional: descriptor retention
is safer than manufacturing absence from a timeout or transport symptom.

## Residual Risk

Host fail-stop depends on the service supervisor starting a higher controller
generation. A broken supervisor can reduce availability, but cannot weaken the
custody proof or authorize active settlement.

Process observation can remain indeterminate under platform query failure or
unknown boot identity. Those cases retain custody and stop the Host. Future
platform-global process handles may reduce indeterminate outcomes without
changing the protocol boundary.
