# TUI interactions

Yolo is a client for persistent Hub Terminals. The shell is the primary
entry point: agents are ordinary programs started by the user inside its PTY.
There is no user-visible workspace collection or workspace directory selector.
Terminal, PTY, AgentRun, provider history and client lifetimes are defined in
[the domain model](terminal-agent-model.md). A client never binds the terminal
named by its own Worker environment (`YOLO_TERMINAL_ID`): running YOLO inside a
YOLO terminal shows a status hint instead of recursively rendering itself.

The content column reserves one Tab strip and one contextual status line. Tabs
belong to the active machine; the selected Tab has a dark background in addition
to its `●` marker and bold cyan label. The machine tree sidebar can remain
visible while the terminal has keyboard focus. See [Tab navigation](tui-tab-navigation.md) for the current interaction model. Healthy
input readiness appears in the Tab marker and contextual status; read-only,
synchronization and offline states are shown only when applicable. Input paused
means explicit confirmation is required, while Resuming input means a fresh grant
is pending. Tab chips use `✎` for writable and `◇` for read-only; paused and
synchronizing input use `‖` and `…`. The status line reserves room for the resume
action before a long terminal title. Errors retain the current identity and
access state. Prefix hints, the picker and help appear when requested.
Provider-owned terminal output remains intact.

## Terminal and Tab Controls

The existing Ctrl+B prefix scopes manager shortcuts:

| Key | Action |
| --- | --- |
| d / q | Detach the client, preserving all shells and agents |
| s | Focus the machine and agent tree |
| b | Show or hide the sidebar |
| g | Open the existing-terminal picker |
| w | Open the tab manager |
| c | Create a new Tab and enter its shell |
| v / - | Split right / down and enter a new shell |
| n / p | Next / previous Tab |
| a | Request available input control without taking it from another client |
| t | Explicitly take input control |
| ? | Show help |
| z | Toggle focused pane zoom |
| { / } | Swap the focused pane with the previous / next pane |
| y | Copy completed local text selection |

Ordinary keys, including q and ?, belong to the terminal program. The Ctrl-B
prefix scopes Yolo controls. Input authority is transient and is acquired through
the Hub; a concurrent observer never silently takes another client's writer.
See [local entry and lifecycle](tui-local-startup.md) and [split-pane geometry](tui-tabs.md).

## Inventory Rows

The sidebar is a tree. Machine roots use the local host name (`This machine`) or
the peer's configured SSH label, with `▾`/`▸`, the number of agent runs and a
reachability glyph (`●` reachable, `○` unreachable, `?` stale or unknown). The
active machine is highlighted. Agent runs are indented one level and keep the
one-line shape: provider-preferred title, optional `!` warning, writer badge and
activity glyph. A braille spinner animates while the Agent is working; `‖` and
`!` mark waiting-for-input and waiting-for-approval states. Directory, lifecycle,
health and the full control description remain available in Agent details.

A healthy connection adds no title row above the tree; the contextual line names
the active machine and catalogue state. A single compact `Disconnected` or
`Refreshing` line appears only when that state needs attention. `Manage machines`
is a footer action under the tree and `m` opens the same manager.

## Inventory Mouse Navigation

Clicking a machine root activates its workspace: the client focuses the
machine's last live terminal (or the first live catalogue terminal for it), and
starts a shell on that machine when none exists. Clicking an agent row activates
its machine and opens the agent's terminal; if no live terminal is available it
opens Agent details instead. Clicking a Terminals picker row selects it for the
existing Enter flow. Wheel input over the tree moves selection by one row.
Activation may acquire write control through the normal terminal path, but the
click itself never injects terminal input. Hit regions use actual rendered list
offsets and item heights. Unpainted trailing rows are not clickable.

`←`/`→` or Space collapse/expand a machine; `Enter` activates the selection;
`n` starts a directory prompt on the selected machine; `/` filters machines and
agents. Tabs, splits and `Ctrl-B n/p` are scoped to the active machine.

The saved viewport binds view, filter, selection, catalogue freshness,
ordered identities and displayed row evidence. A stale viewport consumes the
gesture without selecting another record after sorting, filtering or execution
replacement. Wheel selection dispatches no application command.

## Shell and terminal lifecycle

Plain `yolo` enters a shell automatically. The terminal picker also offers `c`
to create a shell, or `n` to edit the directory before pressing Enter. The
directory field starts at `~`, which is valid for both local and remote Hosts.
After the creation is confirmed, the new tab becomes the active terminal and
its PTY is ready for any command, including `claude` and `codex`.

`yolo agent start --provider codex` and managed resume select `--no-daemon`
when the installed Codex supports it. Older versions receive the compatible
`features.daemon_auto_start=false` setting.

With Yolo's Codex hooks installed on the execution machine, type `codex --yolo`
in a newly created Bash or Zsh login shell. For that interactive invocation the
shell selects `--no-daemon` if the native Codex supports it. Other arguments go
directly to Codex, and your own shell alias or function takes precedence.
After a trusted hook binds the session, expand the local machine in the sidebar.

Reattaching preserves the running shell and its startup environment. In a shell
created by an older Worker, or in another shell, you can use
`codex --no-daemon --yolo` explicitly on supported Codex versions.

While a terminal is focused, ordinary keys go to the PTY. `Ctrl-B d` and
`Ctrl-B q` exit the client while leaving the shell and agent running.
`Ctrl-B g` opens the picker; bare `q` exits from navigation views only.
The next `yolo` or `yolo attach` reconnects to the existing Terminal.

The TUI owner can also be detached from another shell without touching the
Terminal or process tree:

```text
yolo client list
yolo client detach
yolo client detach <CLIENT_ID>
```

The command authenticates through a private, same-user Unix socket and a
short-lived owner token. It asks the current TUI to perform its normal bounded
shutdown and preserve the Hub, Host, Worker and PTY. Success is acknowledged only
after final state is saved and the client's storage locks have been released.
Concurrent clients use separate local state; an ambiguous external detach
requires an exact client ID. The control endpoint uses a short socket namespace
so long Application Support paths do not exceed macOS Unix socket limits.

## Terminal Scrolling

Wheel input over the focused terminal is forwarded as mouse input when the
program has enabled a mouse protocol. When an alternate-screen program has not
requested mouse reports, terminal scrollback cannot move; Yolo translates the
wheel into PageUp/PageDown instead. Primary-screen programs continue to use
Yolo's local scrollback.

Codex keeps its main prompt in a scroll region, so that transcript never reaches
terminal scrollback. In a live Codex terminal, wheel-up opens Codex's own
transcript overlay with Ctrl-T and scrolls it three lines per wheel notch;
further wheel input keeps scrolling in three-line steps, and Esc returns to the
prompt. Wheel-down without an open overlay stays at the live tail (or moves
through earlier shell history when present). A rapid wheel burst opens the
overlay once; it does not send a second Ctrl-T before the overlay repaint is
visible. Only the live tail may generate Codex keys: the terminal must still be
bound to an active Codex agent (or show Codex's composer above its model/cwd
footer with no shell prompt after it), and locally scrolled-back content stays
local. Programs that request synchronized output (DEC private mode 2026) are
not painted until they commit the update, so intermediate cursor positions do
not flash. Shift+drag text selection remains local and never forwards the
gesture.

## Terminal Selection and Clipboard

Drag over terminal text and release to copy it to the outer terminal's clipboard.
Use Shift+drag when a program such as an editor requests mouse input. A click
focuses the pane. An observer can copy the visible screen. Ctrl+B y copies the
selection again and resumes live display. Escape also resumes live display.
A local desktop copy uses `pbcopy`, `wl-copy`, or `xclip`. SSH and nested
terminals use OSC 52 so the viewing terminal can update its own clipboard.
OSC 52 also provides the fallback when the desktop clipboard is unavailable.

The selected range expands wide characters, keeps combining text, joins soft
wraps and omits trailing screen padding. The visible rectangle stays frozen
during selection while the terminal continues recording output. A change of
pane, layout, zoom, runtime, screen size or outer-terminal focus cancels the
selection. Only completed user gestures issue clipboard commands.

In a terminal or an agent Chat, press Ctrl+B then `]` to paste copied files or
an image. On macOS Yolo reads the desktop pasteboard. Linux uses `wl-paste` on
Wayland or `xclip` on X11. Ctrl+B then `u` opens a file path field on the
machine running the TUI, including in an SSH session without a desktop.
Yolo uploads the bytes to the selected terminal's machine. The terminal gets
shell-quoted paths. Chat inserts named file links in clipboard order, so the
agent can read them when you send the draft. Enter submits the input.
