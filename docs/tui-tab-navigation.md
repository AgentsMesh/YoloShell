# Tabs, split panes and the agent sidebar

Yolo's TUI uses a persistent Tab as a terminal page. Each Tab contains a split
tree of panes; each bound pane displays one Hub-owned PTY. A program such as Codex
or Claude Code runs inside that PTY and is discovered through native hooks. An
agent run is neither a Tab nor a PTY.

Navigation separates tabs from panes without exposing a workspace layer.
Shared layout documents supply stable Tab and Pane identities; private client
state retains selection, focus and layout checkpoints. Layout records never grant writer authority or own terminal processes.

## Primary actions

| Action | Key after Ctrl-B | Result |
| --- | --- | --- |
| New Tab | `c` | Preserve every existing Tab; open a shell in a new Tab |
| Split right | `v` or `%` | Add and focus a sibling pane with a new shell |
| Split down | `-` or `"` | Add and focus a lower pane with a new shell |
| Previous / next Tab | `p` / `n` | Restore that Tab's layout and focused pane |
| Focus pane | arrows or `o` | Move within the current split tree |
| Focus sidebar | `s` | Navigate the machine tree |
| Toggle sidebar | `b` | Show or hide it independently of terminal focus |
| Existing terminal picker | `g` | Find retained PTYs, including shells without an agent |
| Manage Tabs | `w` | Select, name, create, or remove a Tab view |
| Close pane / Tab view | `x` / `X` | Remove the view; PTYs and programs keep running |
| Terminal recovery | `R` | Open history with the focused terminal selected |
| Detach | `d` or `q` | Exit this frontend while retaining terminals |
| Literal Ctrl-B | a second Ctrl-B | Forward the control character to the focused PTY |

The Tab strip always shows the catalog's Tab name, including explicit renames.
The focused program's dynamic terminal title remains in its pane and status line;
it never hides the Tab's navigation identity.

The sidebar is a machine tree: machine roots own their Tabs, agent runs are
indented children. `Enter` activates the selected machine or agent (opening the
machine's live terminal, its last live terminal, or a new shell on that machine);
`←`/`→` or Space collapse and expand a machine. `I` shows agent details and `i`
interrupts a supported agent using its existing exact authority. `n` prompts for
a shell directory on the selected machine, and `/` filters the tree. Tabs,
splits, `Ctrl-B n/p` and the last-Tab rule are scoped to the active machine.

The sidebar starts collapsed so a new shell uses the full terminal width.
`Ctrl-B s` explicitly focuses it and `Ctrl-B b` toggles visibility; agent
arrival or exit does not change that choice. When the sidebar is visible the
content column keeps the Tab strip above the panes; hiding it lets the panes
span the width. The sidebar has no Terminals tab.
Agent rows use three lines: title, activity and
directory. Stale/unreachable evidence remains visible; UUIDs, hook provenance and
resume metadata are available in details. The sidebar uses a bounded width;
switching focus between it and a terminal does not resize the PTY. Small windows
show the focused navigation or terminal area instead of unusably narrow panes.

`yolo terminals` opens the independent existing-terminal picker without creating a
shell. It has no layout-tab alias. `yolo`,
`yolo new`, `yolo attach`, `list`, `clients` and `detach` retain their distinct
shell entry, attachment, discovery and client lifecycle roles.

## Creation and navigation invariants

Creation first reserves a dedicated empty destination. The first empty Tab is
reused; an occupied destination is never overwritten. The existing durable shell
creation transaction captures the exact Tab/pane identity before dispatch.
Validation, capacity, journal or queue rejection restores the previous layout.
Late successful creation binds only its still-valid destination, without moving
the user's current focus.

A correlated invalid-directory rejection proves that no shell was created. If
its provisional layout is still unchanged and current, the client restores the
previous view and directory form for editing. A later rename, binding or navigation
prevents that rollback. Unknown delivery retains the durable recovery intent and
never authorizes replaying Create.

Typing while a new shell is starting does not cancel the input-control intent;
those early bytes are not replayed. Explicit navigation still cancels presentation
authority. Opening an agent or retained terminal first locates an existing pane
across Tabs. Only an unbound terminal needs a new Tab.

Selecting a retained terminal closes the picker before accepting terminal input. Tab clicks
are validated against the identities and layout that produced the displayed
frame. Mouse/selection capture receives its release even when the pointer moves
over the Tab strip. Historical agent recovery explicitly preserves the agent's
own terminal target rather than silently switching to the currently focused PTY.

Closing a view never implies terminating a shell. The existing-terminal picker
keeps detached ordinary shells discoverable. Agent exit leaves the shell intact;
confirmed shell exit retains ended output until the user closes that view.

The manager and close-view shortcuts use the same close operation. Closing the
last Tab clears its view while retaining one empty layout entry; the terminal
remains discoverable through the existing-terminal picker.

Help keeps its close and navigation hints visible below a scrollable body.
Use arrows, Page Up/Down or Home/End to browse, Esc/q to return, or Ctrl-B d to
detach without terminating terminals.
