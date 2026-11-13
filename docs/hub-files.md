# Hub File Access

Status: public file RPC, origin preflight, anchored reader, supervised helper and
shared Rust client are connected. Native Mac/TUI root-management editors are
connected; complete delivery coverage remains required.

## Resource Boundary

File access uses explicit origin-approved roots, not arbitrary caller-supplied
absolute paths or an implicit grant to every session's starting directory.
A shell can start in a home directory or beside runtime credentials; that does
not make the entire directory appropriate for export.

`FileRootAddress` is `(HubId, FileRootId)`. `FileRootTarget` adds a fresh epoch
for the active origin-side read context. Relay paths do not change the address.
An address, epoch or version is a precondition, never an authorization token.

`ReadFileRequest` binds that target, a validated relative path, byte offset,
bounded positive length and optional expected version. A nonzero offset requires
an expected version. Responses must preserve target, path, offset and any pinned
version, return the exact bounded range, and report EOF consistently. File bytes
are redacted from the result's Debug representation.

Paths reject absolute forms, empty components, traversal, controls and excessive
byte/component/depth lengths. Approved root metadata cannot name filesystem root.
Metadata and protocol variants validate on public JSON-RPC and internal Cap'n
Proto deserialization.

## Origin Reader

`hub/file/reader` consumes `hub/file/protocol`; the public contract has no Host,
Worker, provider-controller or private persistence dependency.

The reader opens existing directories without chmod, ACL clearing or creation.
It retains every directory descriptor from filesystem root through the selected
file's parent and uses `openat` with `O_NOFOLLOW` for every component. Regular
file inspection rejects directories, devices, sockets, FIFOs and hard links;
`O_NONBLOCK` prevents opening a FIFO from waiting for another endpoint.

Hub/Host private roots must be supplied as protected namespaces. Empty protection
is rejected. Overlap is checked by descriptor-based object identities rather than
path text, including ancestors and symlink-resolved protected aliases. Descriptor
identities include birth identity through the existing persistence primitives.
Loss of a protected namespace also invalidates a prepared read.

Preparation captures file object identity, length, timestamps, permissions and
ownership. The origin version binds this metadata to the root target and relative
path. Reading checks the retained chain and named/opened file before and after
the bounded read. Replacement, reparenting, truncation and detected metadata
changes reject the result. This is not a cryptographic content snapshot or a
defence against a privileged actor able to forge filesystem metadata.

This synchronous component must not run directly on a Hub async executor.
`spawn_blocking` plus a timeout cannot cancel a stalled filesystem syscall.
The execution layer must supervise a separate helper, bound outstanding work,
and revalidate origin/export authority before publishing bytes. Reader success
alone does not establish that the requesting peer remains authorized.

## Supervised Execution

`file_helper_protocol` defines the private inspection/read exchange.
`file_helper` owns blocking filesystem calls; `file_executor` owns spawning,
framing, deadlines, cancellation and exact child settlement. Helper calls are
one-shot and never replayed. A successful response is accepted only after exact
request validation, EOF without an extra frame, and a confirmed successful exit.

Inspection returns an origin-held object fingerprint for each approved root.
Every subsequent helper reopens and compares that fingerprint before reading,
so replacing a directory between subprocesses cannot silently reuse its approval.

Commands use an inherited private duplex socket on stdin. An independent
owner-lifetime socket on stderr is monitored on a dedicated thread, even while
the request thread blocks. Owner loss, unexpected lifetime data or the 15-second
watchdog retires the process. The executor bounds each operation to 10 seconds,
shares four helper slots across clones and uses `ScopedWaitableChild` for
cancellation/timeout cleanup. There is no PID-only termination fallback.

## Public RPC

The public Hub protocol exposes `ListFileRoots` and `ReadFile`. Hubs and clients
must run the current capability schema. File requests require the independent
`FileRead` export right; observing Agents or controlling a terminal grants no
file operation. Root listing is origin-local rather than an aggregate directory,
and root paths are not inserted into the ordinary Agent catalogue.

The existing revision-fenced Hub configuration now includes `file_roots`, a list
of validated `{root_id, name, path}` records. Roots are inspected by the helper
before the active generation is retired or configuration is committed. A rejected
directory cannot disrupt an existing federation. Each activated file context has
one fresh epoch across its bounded, sorted root pages. The Rust file client rejects
mixed-generation pagination and never publishes partial roots or retries reads.

Every operation revalidates the approved directory object fingerprint inside its
new helper. The origin, each relay and the entry Hub check file authorization again
before releasing a completed reply. Configuration revocation cancels old file
executors; changing file rights does not change the remote Worker/PTY lifetime.

`yolo internal hub --file-helper <absolute-program>` selects the supervised executable for
developer layouts. Installed apps use the bundled `yolo internal file-helper`
role in the signed runtime executable. Public JSON-RPC carries the file methods
directly. Mac configuration decoding preserves
file roots through unrelated edits. Roots can be supplied by startup JSON, public
local configuration transactions or the [native root editors](hub-file-root-management.md).
