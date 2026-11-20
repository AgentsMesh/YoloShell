# PTY presentation and notifications

Terminal metadata is observed from committed PTY bytes in the Worker, alongside
the terminal screen parser. It is independent of provider hooks: a title is a
display label, never an AgentRun identity, liveness proof or control grant.
The Terminal's user-assigned name remains unchanged.

The Worker's display title prefers the active AgentRun's provider title:
Claude's `ai-title` transcript records and Codex's `thread_name` session-index
entries. The latest title for the exact provider session is used. When no
provider title is available, the PTY title wins. Provider titles are read from
provider-owned metadata under the bound data home; they are sanitized as
single-line display text and never enter binding identity, recovery authority
or input admission. When the AgentRun exits while its shell continues, the
Worker clears the provider title so the live shell returns to its PTY title;
the completed runtime retains the last title only in its final presentation
and snapshot.

The Host does not resolve or compose provider titles. A Worker image that
predates provider-title ownership reports only its PTY title until that Worker
is restarted; current Workers own the field.

Supported PTY sequences are OSC 0 and OSC 2 for titles, BEL and visual bell for
attention, OSC 9 for notification text, and OSC 777 `notify` for notification
title/body. OSC 9 progress messages are not notifications. Terminal-specific OSC 99 is not
implemented. Existing query, paste, mouse and alternate-screen handling remains
in the terminal protocol components.

The parser accepts split sequences, waits for a complete BEL or ST terminator,
and discards cancelled/overflowed OSC commands. Text is bounded by UTF-8 byte
limits and excludes terminal controls and bidirectional override characters.
Remote metadata uses the same validated contract; receiving a title cannot
execute a terminal control sequence in Yolo's chrome.

Runtime status carries `presentation`, with a nullable title, a notification
counter and the latest nullable notification. These fields are required even
when their values are null or zero. The runtime epoch scopes the counter. Hub
terminal catalogues carry this metadata through federation so an unopened
terminal can still supply a title and attention indication.

TUI terminal rows, focused terminal status, pane borders, Agent rows and the
selected Agent's bottom status use the provider-preferred display title. Missing
or cleared titles fall back to the PTY title, then the Terminal name or provider
name. Agent title lookup requires the
matching runtime and current agent association; a reused terminal cannot label
an earlier AgentRun with the replacement program's title.

Fresh notifications produce an in-app message and a coalesced outer-terminal
bell. Unfocused terminals retain an attention marker until viewed. Initial
catalogues, reconnect baselines, duplicate counters and stale counters do not
ring again. This does not invoke platform notification permission dialogs or
change window focus. The TUI displays metadata in existing sidebar and pane
presentation.

Screen snapshots contain a sanitized title reconstruction, without notification
sequences. The reconstruction carries the same provider-preferred display title
as the live runtime. A completed or unavailable Worker can expose its retained
title only from a snapshot matching its runtime epoch. Notification history is
not durable across Worker loss; saved screen replay does not resend alerts.
