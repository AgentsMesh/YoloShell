# Terminal, agent and client lifecycles

Yolo manages persistent **Terminals**. Users enter a normal shell and run Codex,
Claude Code or another TUI themselves. A Terminal, its current PTY, a running
agent process and a provider's saved conversation have separate identities and
lifetimes. A client observes these resources and may hold permission to send input.

| Concept | Identity and responsibility | What survives an exit |
| --- | --- | --- |
| Terminal | Stable address (`hub_id` + `terminal_id`) on its origin Hub; name, directory, history and recovery metadata | The Terminal record remains after its PTY ends. |
| PTY runtime | A Terminal's `runtime_epoch`, Worker and shell/process tree | A live runtime survives client detach and transport loss. A new shell or machine-restart recovery requires a new epoch. |
| AgentRun | One observed agent execution, fenced by provider, process PID/start time, `execution_id` and PTY runtime | Exiting the agent ends that execution. It does not end an ordinary containing shell. |
| ProviderSessionId | Provider-owned conversation/history identifier and provider-specific resume metadata | Saved history can outlive both AgentRun and PTY. It is not a live-process address or input capability. |
| Client | One attached TUI or desktop view, its panes, controller identity and transient input lease generation | Detach closes the view and releases its input authority; the Terminal and live processes continue. |

A Terminal may host several successive AgentRuns in the same shell. Reopening a
saved provider conversation starts a new AgentRun while reusing its provider
session identifier. A pane binds a Terminal; creating or selecting a pane does
not grant authority over a provider conversation. Hooks provide agent lifecycle
and activity evidence, while PTY/process observation provides runtime liveness.

A verified live AgentRun can have unknown activity. Codex Stop callbacks
precede the provider's final hook decisions; their presence preserves the
execution and recovery binding but does not prove idle input. A Codex permission
callback is also a pre-decision candidate, but it is surfaced as
WaitingForApproval so the user can find the terminal prompt; it grants no prompt
input or typed interaction and a later event clears it. The Hub represents an
unresolved Stop as Running lifecycle with Unknown activity. Semantic prompt
submission requires confirmed input readiness; ordinary leased PTY input remains
independent. See [provider hook evidence](provider-hooks-research.md).

Hooks are installed per provider with `yolo hook install <provider>` for one or
more of `claude`, `codex`, `qwen`, `gemini` and `grok`. `yolo hook status`
shows each provider's configuration file and state. The command reviews the
target file and managed callbacks, asks for confirmation, preserves existing
user hooks and unrelated settings, and keeps a backup. The installed callback
only runs when the managed Worker exports `YOLO_HOOK_EXECUTABLE` and
`YOLO_HOOK_HOME`, so ordinary shells are unaffected. Codex, Gemini CLI and Grok
Build additionally require the user to trust the hooks in the provider's own
settings; Yolo never bypasses hook trust. A provider whose configuration
disables hooks is reported as `disabled` and is not rewritten.

| Event | Meaning | User-facing action |
| --- | --- | --- |
| A normally launched agent finishes or quits | That AgentRun ended; the shell and PTY may still be live. | Continue at the shell prompt, run another command, or detach. Do not show the Terminal as exited solely because the agent ended. |
| The shell exits, or an `exec`-replaced foreground program ends the PTY | The current PTY runtime ended. Its Terminal record and retained output remain. | Show an exited Terminal with no input grant. Offer explicit recovery when available, `Ctrl-B c` / `yolo new` for a new Tab with a shell Terminal, `Ctrl-B g` / `yolo terminals` for the picker, and detach. |
| The recorded Worker and shell identities are confirmed gone, without a complete exit receipt | The old runtime is `Lost`; its normal exit code and output completion are unknown. | Remove it from the default active list, retain its history and show recovery actions. Never synthesize an exit receipt or grant input. |
| The client loses its Hub connection | Current runtime/agent state is unconfirmed. This is not evidence of exit. | Mark the view disconnected, fence input and reconnect to the same Terminal/runtime. Do not issue provider resume automatically. |
| The user detaches or closes the client | Only that client/view ends. | `yolo attach [TERMINAL]` reconnects. Shells and agents continue independently. |
| Worker or machine loss requires recovery | The former live PTY cannot be claimed as the same execution. | Reconcile durable evidence and explicitly recover into a new runtime epoch. Provider history is resumed only through the provider's supported mechanism. |

`q`, `exit`, Ctrl-C and Ctrl-D remain ordinary PTY input while a terminal is
focused. Yolo does not intercept a provider's quit command and reinterpret it as
client detach. `Ctrl-B d` detaches from Yolo. An agent started with `exec` replaces
the shell, so its exit can also end the PTY; the PTY outcome determines the
Terminal's state.

The public picker command is `yolo terminals`. `yolo terminal list` lists
Terminals, `yolo new` creates a Terminal with a shell, and `yolo attach
[TERMINAL]` selects an existing Terminal by ID, address or unique name.
`yolo client list` and `yolo client detach [CLIENT_ID]` address client views,
not AgentRuns or provider history.
The retired `sessions` picker command is rejected; it is not retained as an alias.

The default Terminal and AgentRun lists contain active or unconfirmed resources.
An exited AgentRun leaves that list while its containing shell may continue.
Confirmed ended, lost or archived Terminals are hidden there. `h` includes history
without archiving or deleting anything. Explicit attachment to a completed
Terminal still exposes retained output. Selecting an AgentRun whose original
terminal cannot be attached opens details and a recovery route rather than
silently issuing an attachment or interrupt request.

`Lost` is an observation derived from the recorded boot and exact process
identities. A recognized different boot proves the old execution is gone; in the
same boot both Worker and shell identities must be gone. Unknown identity or
transport failure alone stays `Unreachable`. Reading this observation does not
change the existing recovery policy or disable automatic recovery.

The agent directory retains the latest logical agent association separately
from active execution filtering. A normal exit does not discard its logical
AgentAddress; a later verified execution gets its own ExecutionId. Repeating an
exit observation does not add history entries or change its observation cursor.

The current Hub contract uses `TerminalId`, `TerminalAddress`, and
`TerminalSummary`. A terminal address contains `hub_id` and `terminal_id`; Hub
catalogues expose a `terminals` collection. Its URI is
`yolo://<hub-id>/terminals/<terminal-id>`, and `yolo terminal list --json`
uses this same contract. There are no alternate address fields or resource-name aliases.

`runtime_epoch` identifies one PTY execution. Provider-owned hook `session_id`
fields and provider history directories retain their provider-defined meaning;
they are not Hub terminal addresses. Internal Worker RPC and runtime storage
paths are separate contracts from the public Hub catalogue.
