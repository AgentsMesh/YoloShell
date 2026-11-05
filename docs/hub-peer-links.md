# Hub peer links

Hub federation uses the user's existing SSH access as its transport. A Hub never
opens a public peer port, runs its own SSH server, or asks the user to prepare a
Yolo SSH profile. If `ssh <target>` already works and accepts the target's host
key, `yolo` can join the peer. The login must be non-interactive: `join` runs
`ssh` with `BatchMode=yes`, so a key or SSH agent is required and a password
prompt fails instead of hanging.

## Joining a peer

On the consuming machine, one command:

```sh
yolo hub peer join user@host
```

Both machines must run this release or later: the provider serves the hidden
`peer-stdio` bridge and the public `hub identity` command, and the consumer
implements the SSH dialer. The provider's `yolo` must be on the SSH session's
`PATH` or at `~/.local/bin/yolo`.

The command:

1. runs `yolo hub identity --json` on the provider over the same SSH connection;
2. learns the provider's Hub ID, Ed25519 public key and absolute `peer.sock`
   path over that authenticated channel, and pins the identity locally;
3. writes one peer with the SSH destination and socket;
4. opens the link as one SSH child:

```text
ssh -T user@host \
  'yolo internal hub --peer-stdio --peer-socket /path/to/peer.sock'
```

The SSH child's stdin/stdout carry the `PeerHello` and Cap'n Proto RPC stream.
`peer-stdio` is a hidden bridge that copies bytes to the provider's private peer
socket; it is not a listener and does not parse SSH. `~/.ssh/config`, the SSH
agent and `known_hosts` remain the user's own trust configuration. No key,
profile, broker or port is copied or managed by Yolo.

The provider does not run a command per join. Its Hub accepts the bridge
connection on the same-user `peer.sock` and applies its local sharing policy. A
provider setting can require explicit peer registration; the default trusts
the SSH account boundary.

The provider normally runs its Hub already. If it is stopped, the discovery
command starts it as the SSH account; on Linux that requires the same delegated
cgroup custody as any local start, so the SSH session must be able to launch the
Host. Starting the provider with `yolo start` first avoids that requirement.

Useful companions:

```sh
yolo hub identity --json     # Hub ID, public key, peer socket
yolo hub peer list           # configured peers and their SSH dials
yolo hub peer remove <hub-id>
```

`--no-import` joins without importing the provider's resources. For an explicit
peer, `yolo hub peer add <hub-id> <key> --ssh user@host --remote-socket <path>`
writes the same entry without the identity discovery step.

## Trust model

`peer.sock` lives in a private state directory. Only the provider's user can
reach it, and anyone who can SSH as that user can already run Yolo there. The
SSH account is therefore the admission boundary; a second key exchange or a
per-join approval would add friction without adding authority.

The `PeerHello` handshake still runs on every link. It binds the stream to the
provider's Hub identity discovered during `join` (a changed identity fails
closed) and labels the consumer for routing, audit and policy. It is automatic,
not a user step.

The provider decides what it shares. The default policy grants its own origin to
SSH-authenticated peers with the same rights as local access; an explicit
per-peer export policy overrides it. `ssh_peers.require_registration` in the
Hub configuration restores the strict mode: only pinned peers may connect, and
their individual export policies apply. This is useful for a shared SSH account
or a multi-tenant host.

## Direction

A link is directed: the consumer dials and the provider serves. Two Hubs that
consume from each other hold two links, each opened by its own `join`. Forwarding
still requires an outbound link at every hop.

## Migration

0.2.x peer addresses named a local SSH broker profile. That transport remains
only for managed remote setup; user peer links are SSH destinations. 0.1.x TCP
dial addresses are retired and dropped from configuration. Peer pins, import
flags and export policies survive. Re-add user peer links with
`yolo hub peer join`.
