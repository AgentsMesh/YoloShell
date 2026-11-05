# Automatic Local Hub Startup

The native application's unconfigured startup connects to a persistent local Hub.
An explicit Hub connection configuration still takes precedence and fails closed
when missing, invalid or bound to a different Hub entry. Startup never silently
switches an existing public Hub connection back to private Host services.

## Startup Boundary

`yolo internal hub --bootstrap --state <state> --host-home <home>
--host-executable <yolo>` ensures the local Host and Hub are serving, then
returns one bounded JSON bootstrap receipt and exits. The receipt contains the
canonical `state_path`, canonical `host_home`, and a `connection` object using the
existing public connection-configuration schema. The single executable uses its
own image for the background Host and Hub roles. No provider launch wrapper or
new hook-ingress endpoint is involved.

`yolo start`, `yolo stop` and `yolo restart` are the supported operator entries
to this boundary. `yolo status` and `yolo version` resolve the same profile
sockets and report the installed image separately from the images the running
Hub and Host reported; status never starts services.

The bootstrap operation uses a private state-directory coordination lock. It
validates a persisted Hub identity without acquiring the running service's
singleton lock. Empty state can be initialized; corrupt, copied, rebound or
symlinked identity records are not regenerated. Concurrent launchers serialize
initialization and reuse an already authenticated, serving Host and Hub.

Host status must pass the existing authenticated local protocol and home binding.
A new Hub returns its identity and socket through a private readiness pipe; the
launcher then verifies the public Hello exchange. Protocol or identity failures
are not treated as permission to replace a service. Each stage shares the overall
startup deadline, and readiness reads have an explicit byte limit.

## Socket Namespace

Hub endpoints use the existing short, private per-user socket namespace, keyed
by the canonical Hub state directory. The retained namespace validates its full
directory binding, keeps concurrent owners alive, and fences replacement before
publication or requests. `hub.sock` and the private Host endpoint are distinct.
Hub identity/key storage and state-directory clone detection are unchanged.

Clients consume the reported socket location; they do not append a filename to
the state path. Each platform validates both requested directories in the owned
role's receipt, and the Rust role verifies their identity/socket mapping before
returning it. The receipt is not itself a `hub-client.json` file;
its `connection` field uses that public configuration format.

The current Yolo executable owns bootstrap and role dispatch. A release with an
unsupported image or protocol is rejected before it can publish a socket; no
legacy socket alias or compatibility endpoint is installed. A Hub restart does
not transfer or terminate Worker/PTY custody, and stable Hub/Terminal/Agent
addresses do not change with this endpoint.

Startup retains an authenticated Host connection before returning from the Host
probe and refreshes it throughout Hub initialization. The Hub acquires its own
authenticated hold before preparing federation state. Losing a singleton race
retires only the launcher's contender, then continues probing the canonical
endpoint within the original deadline. It never respawns contenders indefinitely
or retries authentication/protocol mismatches as transport failures.

## Process Custody

New services use `ScopedWaitableChild`, which reserves retirement capacity before
spawning. Before readiness, cancellation or failure keeps the existing exact-child
retirement behavior. After readiness, retained custody is transferred to the
reserved natural-exit reaper and failure retirement moves to the child-side
startup guard. A bounded private Unix socket carries a versioned commit bound to
the exact child PID. EOF or deadline before commit stops the uncommitted service,
including when the launcher is killed without running Rust destructors. A fully
accepted commit is not undone merely because its acknowledgement is lost.

Both Host and Hub use the same `runtime/engine/startup/guard` component; the Host
does not depend on the public Hub protocol. Bootstrap handles TERM, INT, HUP and
QUIT through controlled cancellation and waits for pending child termination
delivery before exiting. There is no PID-only kill fallback or discarded
waitable-child ownership. Only confirmed services receive a startup commit.

Launcher exit does not terminate confirmed services. Hub exit disconnects clients
without transferring PTY ownership from the Host/Worker runtime. Starting the Hub
again retains its persisted identity and reconnects to the existing Host. This
does not claim that a PTY survives an operating-system reboot or that startup
automatically restores an agent without its provider-specific recovery evidence.

## Native Composition

The release contains one `yolo` executable. Its internal role dispatch serves
Host, Hub, file-helper, attach and TUI entry points. Native preparation runs
asynchronously and is generation-fenced: quitting or replacing startup cannot
install a late client state or window.

The bootstrap receipt binds the entry Hub, local origin, socket and current local
Host-to-Hub binding. Native validation rejects unexpected client-state or
remote-directory defaults. Requested and reported state/Home directories use the
same platform resolution rules, so path aliases do not falsely reject the same
directory. A different directory binding or socket filename is rejected.

Existing client-state and Terminal identities must use the current public schema.
Unsupported records are rejected without migration or rewriting; startup does not
invent remote Hub identities or compatibility origin mappings.
