# TUI interactions

Yolo is a client for persistent Hub Terminals. The shell is the primary
entry point: agents are ordinary programs started by the user inside its PTY.
There is no user-visible workspace collection or workspace directory selector.
Terminal, PTY, AgentRun, provider history and client lifetimes are defined in
[the domain model](terminal-agent-model.md).

The terminal area reserves one Tab strip and one contextual status line. The
selected Tab has a dark background in addition to its `●` marker and bold cyan
label. The Agent runs / Hubs sidebar can remain visible while the terminal has
keyboard focus. See [Tab navigation](tui-tab-navigation.md) for the current interaction model. Healthy
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
| s | Focus the Agent runs / Hubs sidebar |
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

Agent runs are one line: the provider-preferred Agent title, an optional `!`
health warning, the writer badge and a trailing activity glyph. A braille spinner
animates while the Agent is working; `‖` and `!` mark waiting-for-input and
waiting-for-approval states. Directory, lifecycle, health and the full control
description remain available in Agent details.

Hubs use the machine host name when the selected Hub is reached through a local
Unix endpoint, otherwise the compact Hub identity. The status glyph follows the
name: `●` reachable, `○` unreachable and `?` stale or unknown. There is no second
"Reachable" row.

A healthy connection adds no title row above the sidebar: the section tabs name
the view and the rows carry their own identity and status. A single compact
`Disconnected` or `Refreshing` line appears only when that state needs
attention. In Hubs view, `Configure Hub` shares the section row and `m` opens
the same manager.

## Inventory Mouse Navigation

Click an Agent runs or Hubs sidebar tab to change catalogue. Clicking an Agent
runs row selects it and opens its existing terminal; if no terminal is available
it opens the Agent details instead. Clicking a Hubs or Terminals row selects it
for the existing Enter flow. Wheel input over the list moves selection by one
item. Clicking an Agent runs row opens its existing view exactly like Enter:
view activation may acquire write control through the normal terminal path, but
the click itself never injects terminal input. Hit regions use actual rendered
list offsets and item heights. Unpainted trailing rows are not clickable.

The saved viewport binds view, scope, filter, selection, catalogue freshness,
ordered identities and displayed row evidence. A stale viewport consumes the
gesture without selecting another record after sorting, filtering or execution
replacement. Wheel selection dispatches no application command.

## Shell and terminal lifecycle

Plain `yolo` enters a shell automatically. The terminal picker also offers `c`
to create a shell, or `n` to edit the directory before pressing Enter. The
directory field starts at `~`, which is valid for both local and remote Hosts.
After the creation is confirmed, the new tab becomes the active terminal and
its PTY is ready for any command, including `claude` and `codex`.

While a terminal is focused, ordinary keys go to the PTY. `Ctrl-B d` and
`Ctrl-B q` exit the client while leaving the shell and agent running.
`Ctrl-B g` opens the picker; bare `q` exits from navigation views only.
The next `yolo` or `yolo attach` reconnects to the existing Terminal.

The TUI owner can also be detached from another shell without touching the
Terminal or process tree:

```text
yolo clients
yolo detach
yolo detach <CLIENT_ID>
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

Shift+left drag selects terminal text, including for a read-only observer and
when the source program requests mouse reports. It never forwards that gesture
to the source PTY. The selected pane displays a bounded frozen image while its
live surface, observations and query replies continue updating in the background.
Release completes the selection; Ctrl+B y sends one OSC52 clipboard command to
the outer terminal. Escape or ordinary terminal input returns to the live image.
Successful local emission does not promise the outer terminal accepted clipboard
access; its own clipboard policy remains authoritative.

The selected range expands wide characters, keeps combining text, distinguishes
soft wraps from hard newlines and omits trailing screen padding. Captures are
bounded to 65,536 cells and copies to 32,768 UTF-8 bytes; overflow fails the whole
copy. The frozen display rebuilds visible rows with zero scrollback and never
clones the source's entire history.

Only the explicit copy key press can queue the opaque Base64-encoded clipboard
command. Provider OSC52 output, historical output, unfinished selections, repeated
keys and stale runtime/geometry cannot enqueue clipboard commands. Copy commands
are consumed once by the outer screen owner, with no subprocess or retry.

Runtime replacement, pane/layout/mode/zoom changes, scrolling, resizing or focus
loss cancel the selection. Source output alone cannot change the frozen text.
