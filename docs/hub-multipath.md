# Direct and Relayed Hub Paths

The live multipath test exercises A importing C both directly and through B.
It uses production Hub/Federation services over real authenticated SSH channels,
separate Host/Worker processes and a native-hook Agent fixture in a real PTY.
The Hub services themselves run as separate tasks in the test process, not as
three independently launched Hub executables.

## Sequence

1. Start A and C without B. Attach from A and complete a unique PTY input/output
   exchange, proving that the original attachment uses the direct path.
2. Start B with an origin-authorized C export to A. Verify both authenticated
   connections, one C Agent address, and one matching Session/Runtime target.
3. Cut the test proxy's owned A-to-C TCP forwarding streams and reject reconnects.
   The existing direct attachment must terminate without reporting process exit.
4. Wait for the relay-only connection and reachable C inventory. Attach using the
   original output cursor, perform a new input once, and confirm the same runtime.
5. Restore A-to-C. Verify dual-path deduplication, then stop B normally. Attach
   directly with the relayed cursor and complete another independent exchange.

The proxy forwards encrypted bytes only. It does not create catalogue entries,
decode Hub traffic or spoof peer identities. Its forwarding sockets and tasks
have bounded ownership and are retired at the end of the test.

## Assertions

Agent addresses, runtime epoch, Worker ID/PID/start time, shell PID/start time,
and Agent PID/start time remain unchanged. The session is never archived.
Output cursors stay bound to the same RuntimeTarget and do not move backwards.
The origin Agent is not counted twice while both routes are available.

Observation/connection waits are bounded. Mutations are not retried across a
disconnect; each exchange acquires and releases its writer before the next fault.
This does not yet test losing an in-flight mutation response or a held writer
during a path change.
