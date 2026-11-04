# Hub pairing

Pairing replaces copying a Hub ID, public key, host and port between machines.
One command mints a single-use code; one command consumes it.

```sh
# On the Hub that should be reachable first
yolo hub peer invite --host 192.168.1.10 --port 7400

# On the other Hub
yolo hub peer join 'yolo1_...'
```

## Invitation

`yolo hub peer invite` reads the local configuration and calls the entry-local
`MintInvite` management operation. The Hub signs these claims with its identity
key:

- a random invite ID;
- the issuer `PeerIdentity`;
- the dial host and port the joining Hub should use;
- `not_before_ms` and `expires_at_ms` (1–60 minutes);
- `export_local`: allow the joining Hub to import this Hub's local resources;
- `import_peer`: also import the joining Hub's local resources.

The code is `yolo1_` followed by base64url of a fixed payload and an Ed25519
signature over the canonical claims. `--host` and `--port` default to an applied,
dialable listener address; pass them explicitly when the Hub listens on an
unspecified address. `--no-export`, `--import-peer` and `--ttl-minutes` adjust the
policy and lifetime. A code is a bearer secret; transfer it privately.

## Join

`yolo hub peer join <code>` verifies the signature and expiry locally, then
connects to the advertised address using the existing peer SSH listener with
user `yolo-enroll` and the code as the password. The SSH host key must equal the
invite's issuer key, so the code is only sent to the genuine Hub.

The joining Hub sends one framed request containing its own `PeerIdentity`, an
invite correlation ID and a request ID. The inviting Hub re-verifies the invite,
durably records its single use, adds the joining Hub as a peer under the minted
policy, and replies with its identity, configuration revision and applied state.
Only then does the joining Hub add the inviting Hub as a peer with the advertised
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
