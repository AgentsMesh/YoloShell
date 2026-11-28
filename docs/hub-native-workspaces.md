# Native Hub Workspaces

The native application selects public Hub services when a private `hub-client.json`
exists in Yolo's application-support directory. `YOLO_HUB_CONFIG_FILE` can select
an explicit configuration file. An explicit missing file, malformed configuration,
or invalid private-file boundary fails closed rather than choosing a private Host.

Unconfigured installations bootstrap a persistent local Hub using the bundled
`yolo internal hub` role, then use the same public services. The local Host home
is preserved; Hub identity lives separately in `hub-local`. There is no default
fallback to private Host services. See [Local startup](hub-local-startup.md) for
custody, readiness and concurrency boundaries. Hub enrollment UI and final release
acceptance remain outstanding.

## Configuration

```json
{
  "schema_version": 1,
  "entry_hub": "11111111-1111-4111-8111-111111111111",
  "socket_path": "/path/to/hub.sock",
  "local_origin": "11111111-1111-4111-8111-111111111111",
  "workspace_path": "/path/to/workspace.json",
  "working_directories": [
    {"hub_id": "11111111-1111-4111-8111-111111111111", "path": "/path/to/project"}
  ]
}
```

The identifiers and paths above are examples, not discoverable defaults. The
entry ID must match the authenticated Hub. `local_origin`, when supplied, must be
the entry and explicitly identifies local working-directory resolution. Remote
origins need an explicit working-directory default; the Mac does not probe its own
filesystem and pretend it describes a remote machine. `workspace_path` is optional.

## Persistence

Workspace schema 4 records its Hub entry. Retired schemas and Host identity
mapping fields are rejected. A new state document is initialized for the selected
entry; an existing document must already be bound to that same Hub. Reading state
does not remap identities, upgrade a schema or reinterpret an unbound Host layout.

Pane/split identities, titles, layout, creation credentials and ledger phases
remain durable current state. A changed revision or entry mismatch prevents the
commit. Rename/durability uncertainty follows the workspace recovery contract.
Hub routing never falls back to private Host services when configuration or a
terminal route is unavailable.

## Public Services

The application uses authenticated persistent JSON-RPC connections for bounded
catalogue and lifecycle calls. The bundled `yolo internal hub-attach` role retains
each terminal attachment and its writer identity. Creation credentials travel
through the authenticated socket, never argv. Hub addresses use `terminal_id` and the
catalogue collection is `terminals`; there are no alternate resource names.

Session recovery hints and optional `safe_termination` metadata come from the
origin. Unknown capability is different from unsupported capability. Mac entries
do not synthesize private recovery history, timestamps, manifest digests or native
agent references. Private lifecycle helpers reject these public entries.

Public lifecycle requests contain an exact session selection. A null expected
runtime means observed absence, not a wildcard. The origin still performs final
authorization and custody checks. Unsupported discard is not implemented by
combining other operations. Uncertain mutations are not automatically replayed.

Catalogue publication is fenced across concurrent refreshes and mutations. A
late catalogue cannot remove a newly published session route. Follow-up directory
failure cannot erase a confirmed creation receipt needed by the durable ledger.

## Versions

Hub, TUI and desktop use the current protocol declarations from their contract
components. Install matching builds together. Yolo does not retain older readers;
see [the unreleased contract policy](unreleased-contract-policy.md).
