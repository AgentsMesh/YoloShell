# Machines

A machine runs one Yolo Hub: the local agent hub that owns agents, terminals
and layouts. Importing agents from another machine uses the user's existing SSH
access as its transport. A machine never opens a public peer port, runs its own
SSH server, or asks the user to prepare a Yolo SSH profile. If `ssh <target>`
already works and accepts the target's host key, `yolo` can import from it. The
login must be non-interactive: `yolo machine add` runs `ssh` with
`BatchMode=yes`, so a key or SSH agent is required and a password prompt fails
instead of hanging.

A link is **one-way**: this machine imports the remote machine's agents and
terminals; the remote machine does not import this machine's. To make it
mutual, run `yolo machine add` on both sides and configure each side's exports.

## Adding a machine

On the importing machine, one command:

```sh
yolo machine add user@host
```

Both machines must run this release or later: the remote serves the hidden
`peer-stdio` bridge and the public `machine identity` command, and the local
machine implements the SSH dialer. The remote machine's `yolo` must be on the
SSH session's `PATH` or at `~/.local/bin/yolo`.

The command:

1. runs `yolo machine identity --json` on the remote machine over the same SSH
   connection;
2. learns the remote machine's Hub ID, Ed25519 public key and absolute
   `peer.sock` path over that authenticated channel, and pins the identity
   locally;
3. writes one machine with the SSH destination and socket;
4. opens the link as one SSH child:

```text
ssh -T user@host \
  'yolo internal hub --peer-stdio --peer-socket /path/to/peer.sock'
```

The SSH child's stdin/stdout carry the `PeerHello` and Cap'n Proto RPC stream.
`peer-stdio` is a hidden bridge that copies bytes to the remote machine's
private peer socket; it is not a listener and does not parse SSH.
`~/.ssh/config`, the SSH agent and `known_hosts` remain the user's own trust
configuration. No key, profile, broker or port is copied or managed by Yolo.

The remote machine does not run a command per import. Its Hub accepts the
bridge connection on the same-user `peer.sock` and applies its local sharing
policy. A remote setting can require explicit registration; the default trusts
the SSH account boundary.

The remote machine normally runs its Hub already. If it is stopped, the
discovery command starts it as the SSH account; on Linux that requires the same
delegated cgroup custody as any local start, so the SSH session must be able to
launch the Host. Starting the remote machine with `yolo server start` first avoids
that requirement.

Useful companions:

```sh
yolo machine identity --json   # this machine's Hub ID, public key, peer socket
yolo machine list              # configured machines and their SSH dials
yolo machine remove <hub-id>
```

`yolo machine add --pin-only` discovers and pins the machine identity without
importing its agents. For an explicit machine, `yolo machine pin <hub-id>
<key> --ssh user@host --remote-socket <path>` writes the same entry without
discovery.

On the machine being imported from, `yolo machine consumers` lists the
machines currently importing from it, with their link count and first-connected
time. The TUI `Manage machines` view exposes the same read-only list on its
Consumers page. This is a live inbound view; it does not grant or revoke
access. The remote machine's export policy remains the authority.

## Trust model

`peer.sock` lives in a private state directory. Only the remote machine's user
can reach it, and anyone who can SSH as that user can already run Yolo there.
The SSH account is therefore the admission boundary; a second key exchange or a
per-import approval would add friction without adding authority.

The `PeerHello` handshake still runs on every link. It binds the stream to the
remote machine's Hub identity discovered during `machine add` (a changed
identity fails closed) and labels the consumer for routing, audit and policy.
It is automatic, not a user step.

The remote machine decides what it shares. The default policy grants its own
origin to SSH-authenticated consumers with the same rights as local access; an
explicit per-machine export policy overrides it. `ssh_peers.require_registration`
in the Hub configuration restores the strict mode: only pinned machines may
connect, and their individual export policies apply. This is useful for a
shared SSH account or a multi-tenant host.

## Direction

A link is directed: the importing machine consumes and the remote machine
exports. Two machines that import from each other hold two links, each opened
by its own `machine add`. Forwarding still requires an outbound link at every
hop.

## Migration

0.2.x peer addresses named a local SSH broker profile. That transport remains
only for managed remote setup; user links are SSH destinations. 0.1.x TCP dial
addresses are retired and dropped from configuration. Pins, import flags and
export policies survive. Re-add machines with `yolo machine add`.
