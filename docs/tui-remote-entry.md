# Direct SSH TUI entry

The TUI can select an explicit remote profile without starting a local Hub or
SSH broker. Resource identity and PTY custody remain at the selected entry and
origin Hubs. A remote entry at A follows the same directed A/B/C visibility
rules as a local client, narrowed by that client's origin grants.

## Connection and authority

One cached outer SSH connection proves the host's exact public key and the
account's private key. A `direct-streamlocal` channel carries an independent
standard SSH connection to the Hub's `public-client.sock`. Its handshake proves
the pinned Hub key and the separate Ed25519 client key. RPC, catalogue, queries
and terminal streams each own inner session channels. Closing one stream keeps
its siblings; closing the entry closes its owned transport, never a source PTY.

The SSH account login does not grant Hub management authority. The client-key
registry maps the verified inner public key to a client ID and per-origin rights.
The profile's client ID is a request declaration; the server still checks it
against the authenticated registry principal before writer operations.
Federation peer usernames, credentials and messages are not accepted by this
ingress. Neither SSH layer accepts server-initiated reverse channels.

Each grant explicitly lists origins and rights: `observe`, `terminal_read`,
`terminal_write`, `lifecycle`, `file_read`, `agent_control`. Rights are independent;
AgentControl additionally requires TerminalWrite and exact origin writer authority.
Observe-only does not permit QueryCreation, because creation-result capabilities
belong to Lifecycle authority. Remote local-management methods are rejected even
when a caller bypasses the Hello advertisement and sends raw frames.

## Enrollment and local administration

Every Hub starts the scoped ingress with an empty client policy by default.
Its private access store is bound to the Hub identity and its retained directory.
`yolo-hub --client-access /absolute/policy.json` can seed a new store; an existing
store only accepts an exactly matching seed. Startup never overwrites another
policy merely because a seed argument was supplied.

Existing local Hub clients can call `GetClientAccess` and `ConfigureClientAccess`
through the same-UID management socket. The replacement contains a request UUID,
the expected current revision and a complete configuration at exactly revision+1.
Public JSON-RPC methods `get_client_access` and `configure_client_access` carry
these operations directly, with the entry Hub in `params.destination`. The
typed result and error contracts are shared by native and external clients.

`AccessConfiguration` contains schema_version, entry_hub, revision and clients.
Each client has an identity (client_id and the Ed25519 public-key bytes), origins
(origin HubId and rights), and expires_at_ms. Client IDs, keys and origin entries
must be unique. Empty clients denies all remote clients. Local runtime state
never stores either client or account private keys.

Only a successful durable replacement publishes the new live registry revision.
All earlier admissions are retired, including connections whose key remains in
the new policy. Changes to files, directories, ownership or the store's exact
identity fail closed. Unknown write outcomes are not replayed automatically;
inspect the current local policy before preparing a new revision.

## Launching the TUI

Create a private profile with these fields:

| Field | Value |
| --- | --- |
| schema_version | 1 |
| host / port / username | Exact SSH endpoint and account. |
| outer_host_key | Full, previously verified OpenSSH public key. |
| remote_socket | Absolute path to the Hub's scoped `public-client.sock`. |
| hub.hub_id / hub.public_key | Enrolled Hub identity and its Ed25519 public-key bytes. |
| client_id | ID associated with the enrolled client public key. |

Launch with explicit, absolute private-file paths:

```sh
yolo --remote-profile /private/profile/entry.json \
  --account-key /private/profile/account-key \
  --client-key /private/profile/hub-client-key
```

The profile and keys are read through the existing private-file boundary with
length limits and exact identity verification. Private key buffers are zeroed
after parsing. Errors and Debug output do not echo keys. These arguments conflict
with local socket, local profile and explicit Hub selection; malformed remote
configuration cannot trigger automatic local startup.

The current adapter supports unencrypted SSH account private keys and an
unencrypted Ed25519 Hub client key. Password, agent and interactive encrypted-key
unlock are not implemented. The OpenSSH server must permit local streamlocal
forwarding for the account. Unknown or changed host/Hub keys fail closed; the
current CLI expects explicitly prepared trust rather than prompting in the TUI.

Bare `--endpoint ssh://...` is still rejected: a URI alone is missing the pinned
Hub identity and scoped client credential. It never becomes a peer tunnel or an
unrestricted forwarded local socket.

## Recovery and verification boundaries

An interrupted request retains its existing public protocol outcome. The adapter
may reconnect for a later explicit channel open but never replays a resource RPC.
TUI journal-before-dispatch and QueryCreation recovery remain unchanged. Restored
workspaces carry view bindings, never writer authority or SSH credentials.

The initial wired component batch passed 25/25 targets. It exercises two real
SSH handshakes, multiplexing, wrong pins/credentials, protocol confusion,
revocation, blocked output and idle attachment retirement. The subsequent real
OpenSSH/production Hub/TUI run passed 8/8 new and regression scenarios. The final
ingress suite passed 14/14 including capacity and failure isolation. Exact logs,
cached Swift evidence and remaining limits are in
the repository acceptance suite.
Full final-source coverage and release packaging remain separate gates.
