# ADR 0030: Web terminal lifecycle over the public Hub command

## Status

Accepted (2026-11-02). Extends the web broker in
[ADR 0026](0026-web-session-broker.md). No Hub protocol, Rust source or service
image change; the public Hub already exposes the typed lifecycle command the TUI
uses.

## Context

The web client could create shells and agents but could not end or archive
them. That forced users to leave the browser for the TUI/CLI after creating a
session, and it left abandoned PTYs and archived history outside the Web
workflow.

The public Hub commands include `Terminal` with a `TerminalCommand`:

```text
request_id: UUID
selection {
  address { hub_id, terminal_id }
  expected_runtime: runtime_epoch | null    // null means "observed no runtime"
}
action {
  operation: "resume" | "recreate"
  operation: "terminate" { force: bool }
  operation: "archive"
}
```

The reply is a `TerminalSummary`. The selection's exact address and runtime
epoch fence the action: a terminal that restarted or changed identity after the
browser observed it produces `target_mismatch` rather than acting on a
lookalike. `archive` keeps history; `terminate` ends the terminal and every
program running in it. The Hub remains the lifecycle authority and the lease
remains the writer authority; these are separate concerns.

## Decision

### Server route and command construction

`POST /api/lifecycle` accepts:

```text
machine: opaque machine handle (optional; defaults to the entry Hub)
terminal_id: UUID
runtime_epoch: UUID | null
action: "terminate" | "archive"
force: bool (terminate only)
```

The server maps the handle back to the raw Hub id, builds a fresh `request_id`,
and sends `terminal { command }` through a dedicated `lifecycle` channel lane.
The lane has one slot and never retries, so a slow End cannot block catalogue,
creation or prompt traffic and a lost reply is never replayed. The reply is
reduced to `{machine, terminal_id, archived, status}`; the raw Hub id and the
Hub summary are not returned.

Timeout is `504` with "check the catalogue before retrying"; `target_mismatch`
is `409`; `not_found` is `404`; `unsupported`/`-32601`, `conflict`,
`unauthorized` and `outcome_unknown` map to actionable errors. The catalogue is
invalidated after success so the observer produces a removal/exit patch.

### Browser surface

The selected terminal header shows **End** only when:

- the machine advertises the `terminal` Hello method (`terminal_lifecycle`
  capability) and is reachable;
- the terminal is not archived and is not already exited/lost;
- a concrete `runtime_epoch` is observable, because End must send an exact
  fence rather than a wildcard.

**Archive** is shown for an exited or lost terminal. Both actions open a modal
confirmation; End is visually destructive. A successful End leaves a
"session ended" notice with the exact machine/terminal/runtime target and an
**Archive** action. Exited and lost unarchived terminals also appear in a
**Finished** section, so a session ended by another client remains reachable
and archivable. Selecting a finished row opens a non-attaching pane (no
terminal socket or xterm) with the same Archive action. The notice clears when
the catalogue no longer contains the unarchived row or after archive succeeds.

An older Hub without the `terminal` method shows no End/Archive controls. The
browser never receives the raw Hub id, a lease grant or a generic lifecycle
proxy.

## Consequences

- Web can now complete the lifecycle of the sessions it creates without
  changing the Hub contract.
- End/Archive act on an exact runtime epoch; a stale browser cannot terminate a
  replacement terminal.
- Terminate is destructive and confirmed; Archive is non-destructive to history
  and confirmed separately. The Web UI does not offer force-terminate yet.
- The dedicated lane prevents lifecycle calls from starving creation or
  interaction; a timed-out lifecycle call is reported as unknown, not retried.
- Exited and lost unarchived terminals leave the live Shells list and appear in
  Finished. Archived history is still excluded from the catalogue and has no
  history viewer yet; the Finished pane is a lifecycle staging area, not a
  transcript browser.
- No Rust change is required. TUI and Web use the same public command and the
  same Hub authority.

## Evidence

Server tests cover exact command construction, null runtime fences, opaque
results, invalid input, timeout, unsupported methods and stale targets. Browser
tests cover capability/epoch gating and live/exited action selection. Live
acceptance exercised a stale epoch (`409`), exact terminate and archive, the
catalogue removal, the browser End → notice → Archive flow, and selecting an
exited row from Finished with no terminal attachment or console errors. Live acceptance evidence is retained with the source revision.
