# Observed Agent Working Directory

Public Agent snapshots carry an optional `working_directory` string. The value
is the last directory reported through the origin's authenticated Agent binding;
it is separate from `TerminalSummary.cwd`, which remains the Shell's creation
directory. Clients never interpret it as a path on their own machine, a file
root grant, or permission to execute a command.

`AgentWorkingDirectory` requires an absolute Unix path, at most 4,096 UTF-8 bytes,
without control characters. Missing or null metadata means unknown. The directory
shares the containing observation's origin, source, freshness and reachability.
Paths are displayed literally, without executing shell syntax or resolving them
against the client's filesystem.

## Identity and Persistence

The origin projects the validated binding's directory. A changed value advances
the Agent observation cursor, without changing AgentAddress, ExecutionId or the
terminal RuntimeTarget for the same running process. Repeated identical Host
observations do not advance the cursor.

Route loss and projection restart retain the last known directory while marking
the observation stale and unreachable. Reconfirmation comes from the authenticated
Host observation, not from opening the client or finding a matching local path.

The optional field can be absent from retained projection records. Loading such
a record preserves logical/execution identities and treats the directory as
unknown until the Host confirms it. Existing invalid-record, namespace and
single-writer checks remain active.

## Client Behavior and Wire Upgrade

TUI Agent rows and search use the Agent directory. Unknown paths do not borrow
the Shell's creation directory; stale paths have an explicit last-known label.
Session rows and search retain the separate Shell directory.

The Mac catalog associates directory observations with the exact current terminal
target. Active Agent presentation uses the reported directory with current,
last-known or unknown status. A confirmed Agent exit returns the live Shell pane
to its Shell/terminal directory presentation. Catalog refresh failure cannot
leave an old directory marked current.

The Agent snapshot schema remains 1 because the directory is optional in retained
observations. Public clients receive it through JSON-RPC; internal runtime and
peer exchanges use Cap'n Proto. See [communication architecture](communication-architecture.md).
Provider-hook schemas remain owned by their providers.
