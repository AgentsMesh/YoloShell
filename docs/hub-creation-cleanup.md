# Public Creation Cleanup

The public Hub protocol preserves the existing durable creation ledger. A client
creates a shell with a stable creation token and receives an origin-scoped
`TerminalAddress` plus a private creation capability. Successful workspace binding
is followed by `Finalize`. Failed binding can use `CleanupTerminate`, then
`CleanupArchive`, using that exact creation result.

Cleanup is not ordinary session lifecycle control. It requires the origin's
creation capability and, over federation, delegated `Lifecycle` rights. A wrong
capability, a substituted origin/session, or a finalized creation cannot authorize
cleanup. Public replies do not echo the capability. Capabilities belong in private
input, not process arguments or diagnostics.

Normal terminal lifecycle uses the public `Terminal` command (`yolo terminate`
and `yolo archive`) and never requires a creation capability. Creation cleanup
is the recovery path for a creation whose binding failed.

The internal Host control surface accepts that private input from stdin as
authority UUID text (one UUID per authority field, whitespace separated) or the
legacy Cap'n Proto document. `create` takes the `creation_token`, the
`cleanup-*` commands take the `creation_capability`, and `finalize-creation`
takes the token followed by the capability.

For a ready creation whose workspace binding failed, the Host's
`creation-capabilities.json` records the capability. The supported recovery
sequence is:

```sh
capability=$(jq -r --arg t "$terminal_id" \
  '.bindings[] | select(.terminal_id == $t) | .creation_capability' \
  "$host_home/creation-capabilities.json")
printf '%s\n' "$capability" |
  yolo internal host ctl --home "$host_home" --existing-host \
    cleanup-terminate "$terminal_id" --force --creation-authorization-stdin
printf '%s\n' "$capability" |
  yolo internal host ctl --home "$host_home" --existing-host \
    cleanup-archive "$terminal_id" --creation-authorization-stdin
```

A wrong capability, a malformed text document, an already archived terminal,
and a finalized creation all remain rejected. A finalized terminal that later
loses exact Worker shutdown evidence cannot be archived by this route.

Termination reports `Terminated` or `Survivor` from the existing runtime. A
survivor means safe termination could not be established; the client must retain
the session for recovery and must not archive it as a successful cleanup. The
adapter does not introduce PID termination, relaxed custody, or fallback to an
ordinary terminate/archive operation. Transport uncertainty remains uncertainty;
the bridge does not automatically replay mutations.

An archive receipt must identify the exact requested session and attest that it
is archived. After dispatch, malformed metadata, another session's receipt, or
an unarchived result becomes `OutcomeUnknown`, not a confirmed preflight failure.
Public wire validation enforces the same requirement even for an external Hub.
