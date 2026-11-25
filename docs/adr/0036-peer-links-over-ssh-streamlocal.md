# ADR 0036: Peer links over SSH streamlocal

## Status

Accepted. This ADR narrows the transport assumption in
[ADR 0012](0012-federated-hub-protocol.md) for Hub-to-Hub links. It does not
change resource ownership, directed federation, delegation or origin policy.

## Context

The current Hub peer transport runs its own SSH server on a TCP listener. A
pairing code authorizes the first exchange (`yolo-enroll` plus a password), and
the joining side must know a dialable host and port. That creates a public Hub
port even when both Hubs are already reachable through the user's SSH access,
and it adds a second authorization step after SSH has already authenticated the
transport.

The intended model is:

```text
A (entry Hub)                                     B (remote Hub)
SSH profile / broker  ── user SSH ──────────────▶ system sshd
                      ── direct-streamlocal ───▶ <hub-state>/peer.sock
                                                   │
                                      PeerHello (Hub identity)
                                                   │
                                      one Cap'n Proto RPC connection
                                                   │
                                               Federation
```

The outer SSH connection owns reachability and account trust. The Hub owns the
peer identity and the peer policy.

## Decision

### Transport

- A Hub exposes one Unix socket at `<hub-state>/peer.sock`. The state directory
  is private (0700) and the socket is owned by the Hub user (0600); there is no
  TCP peer listener.
- The entry Hub resolves an SSH profile, and the remote broker opens
  `direct-streamlocal` to that socket. The broker never opens a public Hub
  port.
- Peer addresses are SSH-only: a local SSH profile id plus the remote Hub's
  absolute socket path. The profile id is resolved against the current local
  profile store when the link is opened. `PeerAddress::Tcp`, `hub listen
  <ip:port>` and the public listener binding schema are removed.

### Identity

SSH authenticates the transport and the user account. It does not identify a
Hub; a user may operate more than one Hub behind one SSH account.

Each link starts with one lightweight `PeerHello` exchange:

1. The client sends its `PeerIdentity`, a bounded version and a random nonce,
   signed by its Hub key.
2. The server verifies the presented signature, then requires the identity to
   be in its configured peer set before replying with its own identity and a
   nonce signed over the client nonce.
3. The client verifies the configured remote identity and signature.

The handshake is identity binding, not a second authorization and not a pairing
code. There is no `yolo-enroll` user, invite code or enrollment ledger.

### RPC

After `PeerHello`, the same byte stream carries one
`yolo_hub_peer_rpc::Connection`. The Cap'n Proto RPC engine already multiplexes
concurrent requests and event capabilities, so the peer link needs no inner SSH
server, no SSH channels and no custom multiplexer.

A link is **directed**. The dialing Hub is the RPC client and the accepting Hub
serves that one connection. Discovery and forwarded calls share the client; the
accepted side never opens a reverse request. This is the natural shape of
`direct-streamlocal` through a system sshd: the SSH client is the user's
connection, so the Hub has no channel through which to initiate traffic back.
A Hub that both serves and consumes a peer therefore holds two links, one in
each direction, each opened through its own SSH profile.

Forwarding follows the same rule: every hop that sends a call must have an
outbound link to the next hop. A peer entry without a dial can still be a
provider and can still pin policy; it simply cannot originate calls.

Link capacity is the RPC in-flight bound. A connection-level failure retires
the outbound link and its routing proofs.

### Policy and onboarding

SSH authorization never grants export rights. `consume` and `exports` remain
explicit fields of the local peer configuration.

- Managed remote Hubs: Hub-owned remote setup configures both sides, writes the
  local dial with the profile id and the remote `peer.sock` path, and adds the
  remote export policy.
- Independent Hubs: `peer add <hub-id> <key> --ssh-profile <profile>
  --remote-socket <path>` writes the local peer. The provider side must pin the
  consumer Hub identity and configure its export policy locally.
- Unknown peers are never trusted on first use. A peer entry with an expected
  identity and policy is required before the handshake is accepted.

### Amendment for 0.2.3 user peer onboarding

`yolo hub peer join <ssh-target>` replaces profile-based onboarding for
independent Hubs. The dial stores the SSH destination, the SSH child's stdio
carries the peer stream through the hidden `peer-stdio` bridge, and the
provider's SSH account is the admission boundary. `PeerHello` still binds Hub
identities; the provider's local share policy replaces per-join approval
(strict registration remains available). Managed remote setup keeps the
profile-based broker transport.

## Consequences

- Pairing codes, `MintInvite`, `yolo1_`/`yolo2_`, the enrollment server and the
  TCP listener disappear together.
- Configuration migration keeps peer pins, `consume` flags and export policy,
  but drops legacy TCP/loopback dials because they cannot name a Hub socket.
  Re-running remote setup, or adding the peer through an SSH profile, restores
  the dial.
- The broker must allow streamlocal forwarding to the Hub socket; an SSH server
  that forbids streamlocal forwarding cannot carry peer links.
- The public client ingress keeps its existing SSH streamlocal path; peer links
  do not reuse the public client socket or its authorization.
- Live acceptance must cover two Hubs over a real SSH account, concurrent
  discover/execute/event traffic on one link, link loss and recovery, socket
  permissions, and macOS/Linux builds.

## Non-goals

- No generic SSH subsystem, third-party capability handoff or global Hub
  discovery.
- No trust-on-first-use enrollment and no implicit export grants from SSH
  account access.

## Amendment for 0.3.0 machine vocabulary

The user-facing entity is a machine running one Yolo Hub. `yolo machine add`
replaces `yolo hub peer join`, `yolo machine pin` replaces `yolo hub peer add`,
and the TUI inventory is `Machines` rather than `Hubs`. The directed import
model, the SSH admission boundary and the peer wire protocol are unchanged.
