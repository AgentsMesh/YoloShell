# Hub pairing

Pairing replaces copying a Hub ID, public key, host and port between machines.
The inviting Hub mints a single-use code; the joining Hub consumes it with an
address it can reach.

```sh
# On the Hub that will accept the join. With no active listener, this
# provisions 0.0.0.0:7400 before minting the code.
yolo hub peer invite

# On the other Hub. The code names the issuer and policy, not an address.
yolo hub peer join 'yolo2_...' --host 192.168.1.10 --port 7400
```

The joining side owns the address because that is the side that knows the
route: NAT, Tailscale and port forwarding are invisible to the inviting Hub.

## Listener

The inviting Hub must listen so the joining Hub can connect. `peer invite`
provisions the default `0.0.0.0:7400` listener when none is active. Use an
explicit address when the deployment needs a different interface or port:

```sh
yolo hub listen                       # default 0.0.0.0:7400
yolo hub listen 0.0.0.0:7500          # explicit port
yolo hub listen 127.0.0.1:0 --json    # dynamic port; JSON reports the actual one
```

A persisted but inactive listener is never overwritten; the command reports it
and asks you to retry `yolo hub listen`. A Hub without a listener can still
join another Hub; it just cannot accept one. When the listener uses a
non-default port, pass the same port to `peer join`.

## Invitation

`yolo hub peer invite` calls the entry-local `MintInvite` management operation.
The Hub signs these claims with its identity key:

- a random invite ID;
- the issuer `PeerIdentity`;
- `not_before_ms` and `expires_at_ms` (1–60 minutes);
- `export_local`: allow the joining Hub to import this Hub's local resources;
- `import_peer`: also import the joining Hub's local resources.

The code is `yolo2_` followed by base64url of a fixed payload and an Ed25519
signature over the canonical claims. It deliberately contains no dial address.
`--no-export`, `--import-peer` and `--ttl-minutes` adjust the policy and
lifetime. A code is a bearer secret; transfer it privately.

Legacy `yolo1_` codes still mint from older clients and carry an embedded
address. `peer join` uses that address unless `--host` or `--port` overrides it.

## Join

`yolo hub peer join <code> --host <inviting-hub>` verifies the signature and
expiry locally, then connects to the supplied address (port defaults to 7400)
using the existing peer SSH listener with user `yolo-enroll` and the code as
the password. The SSH host key must equal the invite's issuer key, so the code
is only sent to the genuine Hub. `--host` is required for `yolo2_` codes;
`--port` defaults to the legacy code's port or 7400. A wrong address is
reported as an SSH host-key mismatch, distinct from a rejected code or an
enrollment refusal.

The joining Hub sends one framed request containing its own `PeerIdentity`, an
invite correlation ID and a request ID. The inviting Hub re-verifies the invite,
durably records its single use, adds the joining Hub as a peer under the minted
policy, and replies with its identity, configuration revision and applied state.
Only then does the joining Hub add the inviting Hub as a peer with the supplied
dial address, mirroring the policy: `export_local` controls whether it imports,
`import_peer` controls whether it exports its local origin. Normal federation
authentication and discovery take over from there.

## Properties

- Codes expire and are single-use. Consumed invite IDs are kept in
  `<hub-state>/pairing/consumed-invites.json`; a replay is `Unauthorized` even
  across a Hub restart.
- Admission can add exactly one peer with the minted policy. It cannot change
  listeners, file roots, other peers, or export more than the local origin with
  Observe rights and zero re-exports. `Configure` remains the only general
  configuration mutation.
- The enrollment user is advertised only while a managed Hub owns the listener.
  A Hub without management admission rejects password authentication.
- If the inviting Hub persisted the peer but could not activate the new
  generation, the reply reports `applied: false`; the joining side still
  configures its direction and the inviting Hub retries activation autonomously.
- A failed admission consumes the code. Mint another one; codes are cheap.
