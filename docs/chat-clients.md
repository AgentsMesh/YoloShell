# Chat clients for existing TUI agents

Chat is a client-local view of an existing managed agent. Opening Chat does not
create another Terminal, start another agent, invoke a model API or transfer PTY
custody. The origin Hub owns native conversation binding and content projection.

## TUI interaction

`Ctrl-b m` switches the focused agent pane between Terminal and Chat. Escape
returns from Chat to the terminal. Normal shell panes remain terminals; Chat
requires a current native agent binding with a supported conversation reader.

The Chat composer edits local text. Enter performs **Append & send**; Alt/Option-Enter
inserts a newline (Shift-Enter only where the terminal reports modified Enter).
Page Up requests older content when available; Page Down
returns toward newer content. The input area explains that existing text in the
agent's native terminal editor is included in the submission. Yolo does not erase
that editor. Switch to Terminal to inspect or edit its contents.

Chat uses the existing exact terminal writer grant and public AgentControl
request. Merely viewing a conversation does not grant input authority. Mode
changes retire queued raw-terminal input scopes, so typing captured in one view
cannot arrive in the other view. Output/metadata observation continues while the
terminal view is hidden.

After delivery, the TUI waits for a changed agent observation before another
submission can reuse input readiness. A rejected precondition triggers a fresh
read and writer revalidation; the agent cursor need not change when the rejected
precondition concerned another resource. The TUI refreshes
that observation independently of native conversation reads and displays
**Append & send** only when its own current observation and writer grant permit
it. A refresh never resubmits the retained draft. A newly displayed transcript
message, or another client's refreshed catalogue, cannot authorize this client.

An exited or replaced agent retires its Chat queries and disables submission.
An existing draft remains reachable as a read-only previous-execution view.
Clearing that local draft explicitly permits opening the new agent's Chat; it is
never implicitly copied to the replacement execution.

## Native macOS interaction

The Pane header offers Terminal/Chat. The same TerminalSurface remains attached
when hidden. Focus goes to the selected content, not to a hidden terminal.
Workspace layout reconstruction retains the Pane's Chat model, draft and mode.
Only visible Chat panes issue reads, with at most one continuing watch each.

The native composer uses the same **Append & send** semantics and the same relay
writer/launch/agent checks as terminal controls. Recorded tool activity and
reasoning can be folded. Older history is read through the public cursor API.
Missing images and unsupported native records are explicit.

Drafts belong to an exact agent execution. Switching executions preserves bounded
previous drafts for explicit copy/discard. A full retained-draft list never
overwrites text silently. Temporary catalogue unavailability disables input but
does not turn the current draft into another agent's text.

## Delivery and observation

Submitting records an operation identity and an exact input snapshot. Delivered
means input was delivered to the terminal, not that the agent has executed a turn.
The native transcript supplies independently observed conversation content.
Clients do not correlate native turns merely by matching message text.

Unknown delivery stays unknown across reconnects. It is not automatically
resubmitted. Late replies for another request, runtime, agent execution or relay
launch cannot clear the current draft. A delivered reply cannot erase text edited
after that submission. Changing Chat/Terminal mode does not revive old requests.

A disconnect before the queue owner's result is provisional uncertainty. An exact
later local result proving that the command was never dispatched resolves it to
NotSent and retains the draft for manual submission. Once an actual provider or
transport result reports Accepted or Unknown, this local cancellation path cannot
unlock it.

Conversation queries use separate connections from terminal input. Slow content
reads do not block writer renewal, keyboard input or release. History generation
changes invalidate old pages. Non-overlapping windows are marked/reloaded rather
than joined as if missing messages had been observed.

## Current scope

These clients read Codex/Claude native records through the public Hub contract.
They do not add a provider model API or screen-derived lifecycle authority.
Structured approval is shown only when a supported Interaction contract exists;
unsupported interactions remain available in the original terminal.

The protocol currently reads active authenticated native agent bindings, has a
bounded 64 MiB changed-file scan, and does not provide asset blobs or a retained
conversation directory after execution exit. See
[client resource protocol](client-resource-protocol.md) for exact boundaries.

Validation must include existing native drafts, view switches while input or
queries are pending, changed agent execution, control loss, delayed delivery,
history replacement, reconnect and continued PTY identity. Automated UI tests
run without activating desktop windows; process acceptance uses private PTYs.
