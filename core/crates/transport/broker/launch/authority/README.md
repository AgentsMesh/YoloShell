# Hub broker launch authority

The Hub owner issues one live authorization channel for one captured `yolo-remote
start-hub --authority-stdio` child. The scope binds the public Hub identity, profile
reference, exact configuration revision, retained configuration file identity and
SHA-256, broker state directory identity, and boot-bound monotonic deadline.

`issue` returns an issuer and an owned stdin socket endpoint. The caller spawns the
fixed broker executable under `ScopedWaitableChild` custody, calls `bind_child`,
then runs `serve`. A child-supplied PID alone cannot bind an issuer. The actual
captured child identity must match every challenge; nonces and ordered request
sequences prevent replay on the channel. HubState signs each acknowledgement,
and the receiver verifies the persistent public identity without opening another
profile store owner or copying a private key.

The owner callback runs before each authorization and signed response. The caller
must retain its profile launch reservation until the exchange completes, so a
profile mutation cannot pass an in-progress launch. EOF, revocation, malformed
frames, changed state, and expired deadlines deny further work.

The receiver asks for a one-use `Commit` authorization bound to the exact ready
broker status. This is permission to hand off, not completion. The existing
broker supervisor must acknowledge adoption before the receiver sends `Finished`
with the same broker instance, configuration, connection UUID and generation and
`adopted=true`. Only that final exchange allows `serve` to return success. A lost
final reply can leave an adopted broker running; callers reconcile its identity
through the existing broker protocol and do not undo an acknowledged adoption.

Optional credentials are the existing IdentityPassphrases envelope, delivered
only as a bounded private byte frame whose digest is signed. The temporary buffers
are zeroized; the envelope is never placed in arguments, environment or journal
records, and invalid credential errors do not retain source text.
