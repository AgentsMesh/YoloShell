# Web client transport

`yolo web` is a runnable browser transport adapter for the public Hub protocol.
It binds a loopback address and accepts WebSocket connections at `/hub` with
subprotocol `yolo.hub.stream.v1`. It does not serve a browser UI or static assets.

## Authority and configuration

Enroll a dedicated Ed25519 client identity through the Hub's existing
`configure_client_access` local administration operation. Select only the origin
grants and rights the browser should have. The Web token identifies callers of
that gateway; it does not widen the enrolled Hub credential. Revoking that
identity immediately closes its accepted browser connections.

Prepare a private profile with this exact structure:

```json
{
  "socket": "/absolute/path/to/public-client.sock",
  "hub_id": "00000000-0000-0000-0000-000000000001",
  "hub_public_key": [32],
  "client": {
    "client_id": "00000000-0000-0000-0000-000000000002",
    "public_key": [32]
  }
}
```

The key arrays above are schematic: each must contain the 32 actual Ed25519
public-key bytes. The matching client private key is an unencrypted OpenSSH key.
The token file contains 32 cryptographically random bytes encoded as 64 lowercase
hexadecimal characters. Profile, key and token must be owned private regular
files in private directories; the existing private-file reader verifies identity,
permissions and size. Do not pass a Hub private key or federation peer key.

```sh
yolo web --profile /private/web/profile.json \
  --client-key /private/web/client-key \
  --token-file /private/web/token \
  --origin http://127.0.0.1:3000 \
  --listen 127.0.0.1:8080
```

`--origin` can be repeated. Origins contain only scheme and authority; wildcard,
`null`, paths, credentials and query fragments are rejected. Non-loopback listen
addresses are rejected. Browser and gateway on different machines require a
secure tunnel; this listener does not claim to supply TLS.

## Wire sequence and lifetime

The WebSocket HTTP upgrade checks Origin before accepting a token. The first
WebSocket message is a text JSON document containing only `token`. It must arrive
within the bounded handshake deadline. Cookies and credentials in URLs are never
used, so cross-site requests have no ambient authentication credential.

After token verification, the gateway opens its own SSH-authenticated connection
to the scoped Hub socket, checks the Hub public key, authenticates the enrolled
client key, opens an independent channel, and performs the public `hello` call.
It sends a text `ready` document containing the live Hello, actual client ID and
session lifetime. A failed handshake or capability check closes the WebSocket.

Subsequent messages are binary slices of the existing Hub stream. They may split
or coalesce JSON LF frames and binary terminal blobs. There is no second command
envelope. Use the existing JSON-RPC request schema and exact-runtime lease rules.
Terminal bytes, including non-UTF-8 data, remain unchanged. Messages are bounded
to 64 KiB; larger public frames must be divided into several WebSocket messages.

Each connection has its own Hub channel. Open separate WebSockets for control,
input, lease renewal, queries, watches and terminal attachments. Closing an
attachment detaches observation and does not kill the terminal. A reconnect gets
a fresh scoped connection and Hello; mutations are never replayed automatically.

The default maximum is 32 connections, with a 30-minute monotonic lifetime.
`--max-connections` and `--session-seconds` can narrow or raise these within the
published bounds. Capacity includes unauthenticated handshakes. Expiry closes
the WebSocket and its exact Hub connection even while output is backpressured.
Ctrl-C closes all gateway-owned connections; it does not stop the Hub or PTYs.

Inbound forwarding has a bounded queue of eight messages (at most 512 KiB),
independent of the browser reader. A blocked Hub writer therefore cannot hide
a browser Close frame. Exceeding that queue retires only that connection; no
queued bytes are silently discarded while the connection remains usable and no
mutation is replayed. A disconnected mutation retains its usual uncertain
outcome and must be queried using its public operation identity.

## Validation

The gateway's integration tests use a real TCP WebSocket listener, real SSH
public-client authentication, the production scoped dispatcher and binary
terminal framing. They cover wrong/missing Origin, rejected bearer credentials,
management-method denial, fragmented JSON-RPC requests, non-UTF-8 terminal blobs,
registry revocation, expiry, disconnect, sibling isolation and capacity recovery.
