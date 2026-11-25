# ADR 0013: TUI Remote Entry Transport

## Status

Accepted transport boundary. Independent client admission, scoped public ingress
and direct SSH entry are implemented; full product acceptance is in progress.
See [remote TUI entry](../tui-remote-entry.md) for the concrete key/profile flow.

## Context

The native TUI speaks the public Hub client protocol. Remote entry must preserve
resource identity, request framing, authority checks and uncertain-write rules
without requiring the client to run or enroll as a federation Hub.

The existing SSH Hub peer tunnel is not a public client endpoint:

1. `remote::russh_backend` opens an outer SSH `direct-tcpip` channel to the
   remote Hub peer listener. `BrokerHubTunnel` carries that byte stream.
2. `peer_transport::connect_io` performs a second SSH handshake over the stream,
   authenticating both enrolled Hub keys.
3. `federation::incoming` reads `PeerRequest::Discover` or signed
   `PeerRequest::Execute` on inner SSH channels and checks delegation.

Sending the public `Command::Hello` directly over `BrokerHubTunnel` is invalid.
Completing peer authentication first does not make its channels public RPC
channels, and federation enrollment must not grant local management authority.

The current public server accepts a same-UID Unix socket and then public RPC
requests beginning with Hello. That socket has no remote client scope admission.
A forwarded connection's Unix peer credentials identify the forwarding process,
not the original client. Hello checks the expected HubId but is not, by itself,
a cryptographic proof of the Hub's identity.

## Separate public client transport

An entry transport selects a Hub without changing resource identity:

- `LocalUnix` connects to the local Hub socket and validates the peer UID.
- The planned SSH adapter identifies the SSH host and account; an entry profile must also
  bind the expected Hub identity, public endpoint and client authorization.

Implement an independent public client ingress. The preferred transport is SSH
Unix socket forwarding to that ingress. Where the SSH server cannot forward a
Unix socket, a fixed Yolo SSH subsystem or narrowly defined bridge may carry the
same public client protocol. Such a bridge must validate the selected endpoint;
it must not interpolate user input into an arbitrary shell command.

The implemented adapter opens one SSH `direct-streamlocal` channel to the scoped
`public-client.sock` ingress. It runs a second standard SSH handshake inside that
stream, using the pinned Hub key and a separate client Ed25519 key. Its dedicated
`yolo-public-client` username and client-key registry do not enroll the TUI as a
peer. Public RPC and terminal streams use independent session channels on that
inner connection. No local Hub or broker is required.

Both variants feed the same `EndpointClient` over `HubEntryStream`. The client
performs public Hello only after authenticating the remote public ingress.
Control and terminal attachments use separate admitted SSH channels on one
authenticated connection. Dropping one stream closes only its channel; the
origin Hub and its Worker remain responsible for PTY custody.

## Identity and authorization

SSH host-key trust, SSH account authentication and Hub client authorization are
separate checks. The entry profile pins the exact SSH endpoint/account and the
expected Hub identity. The public admission exchange must prove possession of
the enrolled Hub key and authenticate a client credential scoped to that Hub;
merely returning its HubId is insufficient. The implementation uses russh's
standard SSH key exchange and signed public-key authentication. It introduces
no custom challenge-signature protocol.

The Hub derives the client principal and scope from verified credentials or
authenticated metadata from a trusted SSH bridge. A caller-supplied username,
requested role or forwarded same-UID connection cannot establish that scope.
Standard Unix socket forwarding does not preserve the SSH user's identity for
the Hub; it therefore requires independent client admission at the scoped socket.

Admission binds the principal, entry Hub, connection generation and granted
scope to each channel. Enforce scope on catalogue visibility, observations,
terminal reads, writer acquisition/input/resize, lifecycle and file operations.
Attached streams must observe revocation as well as request admission. Forwarded
resource operations remain subject to origin delegation and execution checks;
neither client scope nor federation rights may be widened by a relay.

Existing `local_only` commands remain unavailable through this remote entry.
Remote management would require an explicit separately designed management
grant and admission policy. SSH login, peer enrollment or an Observe grant must
never implicitly authorize Configure, provisioning or trust changes.

## SSH admission sequence

1. Resolve the selected entry profile and validate the SSH endpoint/account,
   expected Hub pin and client credential scope.
2. Verify the SSH server key during key exchange against persisted trust.
   Unknown keys require explicit confirmation; changed or revoked keys fail
   closed. A prior probe cannot replace verification of the actual connection.
3. Authenticate the SSH account and open a public client channel to the bound
   scoped endpoint. Validate connection generation and channel purpose.
4. Complete public client admission, including Hub proof and client scope.
   Only then wrap the admitted stream as `HubEntryStream::Remote` and run Hello.
5. On disconnect, release channel custody. Reconnect revalidates trust, Hub pin,
   credentials and current scope. Uncertain mutations are never auto-replayed.

Authentication belongs in shared Rust client components, not terminal rendering.
The UI may collect credential input or trust confirmation, but credentials and
private keys must not enter workspace documents, protocol diagnostics or logs.
File reads remain separate Hub operations with origin-side roots and rights.

## Federation semantics

The connected entry Hub still determines the view. With the same authorized
scope, an SSH TUI entry at A has the same A+B+C catalogue as a local TUI at A.
It does not become a new Hub or alter `(origin HubId, AgentId)` addresses.
B remains both a local manager and a relay for C, and route count never
multiplies Agent rows.

## Implementation status and acceptance

Implemented: independent client identity/grant protocol, durable access store,
revocable admission registry, origin-scoped dispatcher, separate authenticated
public ingress, direct SSH adapter and TUI private-profile/key CLI wiring.
The shared Method authorization map serves peer delegation and client admission;
an empty remote grant never denotes local management authority. Catalogue
filtering retains only authorized origins and never publishes hidden-row cursors.
Writer mutations bind the claimed client ID to the authenticated key's registry ID.

Local-only GetClientAccess/ConfigureClientAccess manage exact policy revisions.
Persistence succeeds before publishing a new registry revision; errors revoke
live admission. Policy changes invalidate prior admissions, including quiet or
blocked streams. Dropping these streams does not terminate source Workers.

Still required for complete delivery: interactive profile/trust/key management,
additional account-auth methods, direct-entry multi-Hub and joint crash acceptance,
full final-source coverage and package validation. The following required cases
remain the acceptance contract; component passes do not imply all are complete.

Unit coverage must include endpoint parsing, host-key transitions, Hub proof,
credential/scope validation, generation mismatches, channel close and uncertain
outcomes. Adversarial tests must reject peer/public protocol confusion, wrong
Hub pins, forged or replayed admission, spoofed SSH usernames, forwarding that
attempts to inherit local management rights and Observe-to-writer escalation.

Loopback E2E must use a real SSH server, real Hub and real TUI over a PTY. Verify
A/B/C entry views, scoped catalogues, terminal input/output, multiplexing,
revocation, entry reconnect and survival of the origin shell after disconnect.
Exercise a broker only if the tested adapter uses one. Tests must run with
`YOLO_E2E_INACTIVE=1` without activating a desktop window.

Bare `ssh://` remains an explicit `UnsupportedEndpoint` result because it supplies
neither Hub pin nor client authorization. The implemented CLI selects a complete
private remote profile and explicit key files. Invalid remote configuration never
falls back to a local socket, a peer channel or unrestricted management access.
