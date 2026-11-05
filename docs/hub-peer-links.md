# Hub peer links

Hub federation uses the user's SSH access as its transport. A Hub never opens a
public peer port and never runs its own SSH server. Each Hub publishes a private
Unix socket at `<hub-state>/peer.sock`; a consuming Hub reaches it through a
local SSH profile and the remote SSH account.

```text
A (entry Hub)                                   B (provider Hub)
SSH profile / broker  ── user SSH ────────────▶ system sshd
                      ── direct-streamlocal ──▶ <hub-state>/peer.sock
                                                 │
                                    PeerHello (Hub identity)
                                                 │
                                    one Cap'n Proto RPC link
                                                 │
                                             Federation
```

SSH authenticates the transport and the account. It does not say which Hub is on
the other end and it grants no export rights. The peer identity and the peer
policy remain Hub-owned.

## Adding a peer

A managed remote Hub is configured by remote setup, which writes the local dial
and the remote export policy together. For an independent Hub, add the peer
explicitly:

```sh
yolo hub peer add <hub-id> <public-key> \
  --ssh-profile <local-profile-id> \
  --remote-socket /home/alice/.local/state/yolo/hub/peer.sock
```

The provider side must pin the consumer's Hub identity and configure its export
policy locally; the consumer's SSH login does not do that. `--no-import` pins a
peer without consuming from it. `--export <hub-id>` and `--export-local` add
Observe policy in the other direction.

The remote socket path is the provider Hub's state-derived `peer.sock`. It is
not a TCP port, and a peer entry with no dial cannot originate calls.

## Direction

A link is directed: the dialing Hub is the RPC client and the accepting Hub
serves that one connection. A Hub that both consumes from and serves a peer
holds two links, one in each direction, each opened through its own SSH profile.
Every forwarding hop must have an outbound link to the next hop; the accepted
side never opens reverse traffic.

## Identity handshake

After SSH opens the stream, the client sends one bounded `PeerHello` with its
Hub ID, public key, a random nonce and a signature. The server verifies the
signature and requires the identity to be in its configured peer set before
replying with its own identity, a fresh nonce and a signature over the client
nonce. The client verifies the configured remote identity. This binds the link
to Hub identities without a second account, password or bearer code.

## Policy

`consume` and `exports` remain explicit local configuration. A Hub exports only
the origins its policy allows; receiving a connection never grants rights. An
unknown Hub identity is never trusted on first use.

## Upgrade note

0.1.x configurations with a TCP `listen` field or TCP/loopback dial addresses
are migrated by keeping peer pins, `consume` flags and export policy, but
dropping the old dial because it cannot name a Hub socket. Re-run remote setup,
or add the peer through an SSH profile, to restore the dial. Pairing codes,
`peer invite`, `peer join`, `hub listen`, `MintInvite` and the enrollment server
are removed in 0.2.0.
