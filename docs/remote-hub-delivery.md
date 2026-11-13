# Remote Hub delivery foundations

Remote installation deploys one `yolo` executable for the target. Its descriptor
binds the image size, SHA-256 digest and supported internal roles into a
canonical build ID. The supported descriptor targets are Linux and macOS on
AArch64 and x86-64; a supported descriptor does
not prove that a release package has been built and tested for that target.

## Publication and recovery

SSH staging uploads only a private `.upload-<UUID>` directory. The native staged
Hub publishes an immutable `releases/<build-id>` directory and atomically selects
it through `current.json`. File and directory synchronization belongs to the
native publication transaction. SFTP alone does not supply those durability
guarantees. Existing releases remain available to already-running processes.

The prepared publication request binds the installation root, operation, stage,
complete descriptor and exact previous selection. A changed root or selection
cannot silently retarget that operation. The caller invokes publication once.
If the response is lost, it verifies a known inspection executable and performs
read-only inspection of the exact operation. Uncertainty never authorizes a
second publication command. Inspection does not create directories, repair
permissions or acquire locks that create files.

Executable verification uses one SFTP handle for the complete expected bytes and
EOF, with owner, mode, path and metadata checks. The expected owner comes from a
bounded numeric UID probe on the authenticated SSH connection. Matching file
contents do not prove inode ownership. Failed or cancelled uploads therefore
retain unproven stages with bounded diagnostic records; those records do not
authorize deletion. A settled cancellation closes its owned SFTP channel while
preserving sibling Hub and terminal channels.

## Startup and enrollment

Remote bootstrap launches Host and Hub from the returned immutable release
paths. Hub state and Host home must remain outside the release tree, including
filesystem aliases. Existing public identity pins are checked before and across
startup locks; an absent or replaced pinned identity is not recreated.

The running Host and Hub roles report the actual executing image's size and
digest. On macOS, capture binds the mapped vnode, open descriptor and launch
path; on Linux it uses the mapped executable. A successful version probe on a
disk file is not sufficient evidence about an existing daemon. A live build or
protocol mismatch rejects enrollment without replacing surviving services or
Workers. Unsupported records are rejected; no automatic migration or alias
endpoint is provided.

Enrollment adds a loopback listener and an explicit consumer export with the
requested rights and reexport depth. It preserves unrelated configuration and
rejects conflicting existing policy. A configuration mutation uses its exact
base revision. After an unknown result, only the exact request ID, resulting
revision, document and applied state can confirm success through read requests.

The SSH transport can open a dedicated `direct-tcpip` channel to the remote
loopback Hub while keeping other channels on the same SSH connection. The inner
Hub connection retains pinned mutual authentication and delegated authorization.
Outer SSH login and public build metadata are not substitutes for Hub authority.

## Integration boundary

These components do not yet constitute the complete remote onboarding product.
The persistent Hub now owns a revisioned SSH profile registry and uses the broker
channel interface through its injected federation dialer. Entry restart reuses
an existing admitted broker. The [SSH profile increment](hub-ssh-profiles.md)
also journals enrollment before submission and adds explicit read-only recovery.
Hub-owned broker launch/adoption orchestration, publication/enrollment operation
management and Mac/TUI onboarding still need integration. Linux delivery also
requires the single-executable release package and catalog integration.

The directed A -> B -> C model remains unchanged: B can own local Agents and relay
C's authorized resources, with stable origin addresses and distinct entry views.
An unavailable relay does not prove remote process death. Persisting the imported
offline catalogue across an entry Hub restart is tracked separately in
the project roadmap.
