# TUI local startup

On macOS and Linux, the installed `yolo` without connection arguments reuses or
starts the persistent local Hub through the explicit `internal hub --bootstrap`
role. Host, Hub, file helper and TUI processes all use the same executable image;
helper discovery does not search PATH.

macOS resolves the same Foundation Application Support directory as the Mac app,
with service directories Application Support/Yolo/hub-local and
Application Support/Yolo/Runtime. Linux uses `$XDG_STATE_HOME/yolo/hub-local` and
`$XDG_STATE_HOME/yolo/runtime`, or `$HOME/.local/state/yolo/` when XDG_STATE_HOME
is unset or empty. Configured paths must be absolute and normalized; an invalid
state home fails instead of selecting a different Hub. No service directories
are created merely by resolving the profile. The Unix socket and Hub identity are
obtained from the validated bootstrap receipt. Existing corrupt or rebound state
fails; it does not create a replacement identity or switch to another service.

An explicit connection remains `--hub <UUID> --socket <absolute-path>`, or
`--hub <UUID> --endpoint local:<absolute-path>`. Partial connection arguments,
unsupported SSH endpoints and conflicting selections fail before bootstrap,
client-state creation or raw terminal setup. There is no workspace selector.
The Hub owns Terminals; each client owns only its local presentation. See
[terminal, agent and client lifecycles](terminal-agent-model.md) for their separate identities.

Plain `yolo` attaches the retained live Terminal or an existing local live shell.
With no existing shell it creates one in the caller's current directory and
enters the PTY. Run `claude`, `codex`, or any other terminal program there.
Authenticated remote entry uses the remote home directory by default.
`yolo new --cwd /absolute/path --name NAME` explicitly creates another shell.
`yolo attach [TERMINAL]` only attaches an existing Terminal; a missing or ambiguous
selection opens the terminal picker with an explanation and never creates one.

`yolo list [--json] [--all]` prints the Hub terminal catalog; `--all` includes
archived terminals from history and labels them `Archived`. `yolo terminals`
opens the interactive picker. `yolo archive TERMINAL` archives an exited
terminal, and `yolo terminate TERMINAL [--force]` ends a terminal and every
program in it; both take an id, `yolo://...` address, `HUB/TERMINAL` address or
unique label, and `--json` prints the resulting summary. `yolo clients
[--json]` lists attached local terminal clients with their kind: the Hub-bound
client is `primary` and addressed by the Hub id, every additional client has
its own id. `yolo detach [CLIENT_ID]` disconnects one without stopping its
terminals. If more than one client is attached, the command requires an exact
client ID.
Terminal names are labels: `yolo new --name` does not reserve a unique name,
and a selector that matches several terminals opens the picker.
Client discovery and detach inspect the existing Hub identity without starting
new services. Ctrl-B d detaches; Ctrl-B s focuses the Agent runs / Hubs sidebar,
and Ctrl-B g opens the existing-terminal picker. New shells open in a dedicated
Tab; splits create real PTYs inside the current Tab.

Layout and unfinished creation records are private implementation state. A
second TUI claims its own state directory rather than failing a global lock.
Inactive clients with unfinished creation records are recovered without replaying
uncertain requests. `YOLO_TUI_STATE_DIRECTORY` overrides the private local state
base for isolated tests or deployments; it is not a terminal namespace and must
not be shared between different Hub identities.

For an isolated or nondefault macOS or Linux installation, `--local-profile <path>`
selects a private bounded JSON document containing `schema_version` (1),
`state` and `host_home`. Both directory paths must be absolute, normalized and
nonoverlapping. Helpers still come from the same installed package. This selector
conflicts with explicit connection arguments; an invalid selected profile never
falls back to default directories.

Linux service startup also requires the existing process-custody prerequisites,
including delegated cgroup v2 support. Automatic local startup now checks that
delegation before creating a Hub, Host or durable creation intent. A normal login
shell in an un-delegated container receives an actionable error and must connect
to an already-running managed Hub or use a delegated systemd service; it no
longer enters the TUI and later reports `No terminal` after a failed Worker.
The TUI reuses an already-running exact Host/Hub pair; Linux does not relax
service identity or cancellation checks.

The Hub state directory also carries `runtime-contract.json`, binding the state
to the public Hub protocol and state schema. A binary that cannot prove this
contract refuses to open the state before taking mutable service actions. Missing
or mismatched contract data requires an explicit migration or quarantine flow;
the old Hub/runner must be stopped deliberately, while the persistent custody
domain and its PTYs remain protected.

Startup cancellation discards only the owned short-lived bootstrap helper.
Existing startup guards retire uncommitted service contenders; an accepted service
commit is preserved, including after lost acknowledgement. Closing TUI does not
terminate Hub, Host, shell or agent processes. External SIGINT/SIGQUIT enter the
same graceful terminal cleanup as SIGTERM/SIGHUP. Keyboard Ctrl-C remains terminal
input while a pane is attached.

If a launch reports `No such file or directory` from the scoped child, inspect the
private socket namespace root reported by the process. A previous interrupted
startup may leave an empty unbound namespace directory. The safety policy keeps
an unproven namespace fail-closed; after confirming no Yolo Host/Hub process and
confirming the exact directory contains no files or sockets, remove only that
empty directory and retry. Do not remove the namespace root or nonempty/unknown
residues. A fresh private root via `YOLO_SOCKET_ROOT` is a safe diagnostic.

## Validation scope

The real CLI PTY scenario uses an explicit private profile and bootstraps a Hub over
an existing Host. Each launch owns a fresh nonce directory: both stty calls and
the TUI must exit successfully, and the captured terminal settings must match.
External signals require the current launch hint to match a fresh fenced scan
under the exact outer PTY shell, including the actual TUI image and complete argv.
After each exit, the original Worker and shell must still match their live process
identities and complete a fresh random-marker terminal exchange.
A target-shell INT trap records exactly one delivery from the Ctrl-C keypress;
external SIGINT/SIGQUIT terminating the TUI must not append more target deliveries.
A separate owned-bootstrap scenario
starts an empty private profile and verifies exact service reuse. These tests
must run without activating windows and are not acceptance of an unattended
launch into the user's real default Application Support directory.

The existing local-startup cancellation/parent-loss cases passed in the preceding
33-scenario group, including uncommitted Host/Hub retirement and committed PTY
survival. The shared profile unit/integration tests and architecture checks also
passed after the standard macOS path-alias correction. Final full-source
coverage, inactive E2E and current-revision package acceptance remain pending.
There is no deterministic test barrier at the exact prepare-to-UI transition;
the implementation retains one cancellation owner across it, while tests cover
pending bootstrap and an already-rendered TUI on either side of that boundary.
