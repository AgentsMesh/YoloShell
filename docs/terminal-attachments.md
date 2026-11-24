# Remote terminal attachments

`stage_attachment` lets a client paste an image or file into a remote terminal
or agent conversation. The origin Hub writes the file on the machine that runs
the target terminal. The returned absolute path can be inserted into the
terminal's input or included in a Chat prompt.

A plugin can request `extensions` with `{ "destination": "<hub-id>" }` and check
the returned `methods` array for `stage_attachment`. Older entry Hubs return
JSON-RPC `-32601` for an unknown method. If an origin cannot be reached
through the configured route, the normal Hub routing error is returned.

## Wire example

Send JSON-RPC requests over an authenticated Hub client. The target is the
terminal address and runtime epoch from the current catalogue.

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "stage_attachment",
  "params": {
    "destination": "<origin-hub-id>",
    "request": {
      "target": {
        "address": { "hub_id": "<origin-hub-id>", "terminal_id": "<terminal-id>" },
        "runtime_epoch": "<runtime-epoch>"
      },
      "upload_id": "<fresh-uuid>",
      "name": "notes.txt",
      "offset": 0,
      "data": "aGVsbG8K",
      "eof": true
    }
  }
}
```

The reply's `attachment` contains the same target and upload ID, `next_offset`
in decoded bytes, and `path`. Intermediate replies have a null path. The
final reply carries the published path and preserves the supplied basename.

Supply the file's basename, including its extension, and stream standard
base64 in chunks of at most 128 KiB decoded. Use the same target, upload ID and
name for all chunks in a transfer. The first offset is zero. A final empty
chunk at the acknowledged offset also closes a transfer. An empty file uses a
single empty final chunk. After a lost response, reconnect and send the same
chunk at the same offset. Overlapping bytes must match and gaps are rejected.

Staging requires the destination's `terminal_write` right. The origin checks
the runtime's identity and liveness. Ordinary terminal input still goes
through the writer lease, and chat uses the agent's prompt capability. Terminal
clients should quote the returned path appropriately for their input surface.
Keep the Hub connection and authority in the trusted plugin process.

The origin keeps files and partial uploads private to the OS user in its
session directory. Successfully archiving the terminal removes them. A Hub
restart preserves staged data and transfer progress. The Rust API
`Client::stage_attachment` streams an `AsyncRead` with an inactivity timeout
for each read and request.
