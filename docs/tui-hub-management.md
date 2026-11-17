# Native TUI Hub Management

The Hubs inventory exposes a `Configure Hub` command. Keyboard entry is `m`
while viewing Hubs. Configuration always belongs to the current entry Hub;
selecting an imported Hub or Agent never changes administrative authority.
The manager is a full-width terminal view, separate from Agent PTY surfaces.

## Editing

The view displays the entry Hub ID, public key, configuration revision, and actual
listening address, including the assigned port for a dynamic listener.
Connections, trusted origins, file roots, listener settings, and per-peer export policies
have separate forms. Keyboard and mouse navigation, bounded paste, UTF-8 text
editing, checkboxes, depth controls, scrolling, and wrapped identity fields are
supported. Read-only identity fields cannot be altered while editing a connection.
File root creation, editing and confirmed removal are described in
[Native file root management](hub-file-root-management.md).

Adding a connection requires the peer's Hub ID and pinned public key. Dial host
and port are optional together: an inbound-only peer can still export or import
resources on an independently authorized connection. Import direction is an
explicit checkbox, independent of who dials. Reciprocal identity enrollment and
export authorization must be configured at the other Hub as well; the TUI does
not silently enroll an unknown key.

Trusted origins support downstream origins such as C in A -> B -> C without
turning C into A's direct peer. Exports select an origin, independent permissions,
and a bounded reexport depth. New exports start with Observe only. File-read
permission is independent of terminal input; granting it does not manufacture
an origin capability that has not been implemented.

Each form edits an in-memory draft. Saving a form does not send a Hub mutation.
`Apply Changes` explicitly submits the complete validated draft with the base
revision. `Activate Configuration` is available when the persisted configuration
is inactive. Neither command launches, resumes, or terminates an Agent.

Removing a peer or trusted origin requires confirmation. Dependent exports are
removed only when no remaining peer/trusted-origin pin still authorizes that
origin. An export can be removed without removing its connection. Discarding a
dirty draft on close or reload also requires explicit confirmation.

## Concurrency And Failure

`hub/management/client` owns a separate public connection and one absolute
deadline for each management operation. The TUI adapter runs management,
interactive control, and catalogue work independently. A stalled management
request does not block writer renewal or control traffic.

The UI retains the exact submitted configuration after an uncertain result.
It does not automatically resend Configure or substitute a newer revision.
`Check Change Result` sends only a read request; it confirms success only when
the origin returns the exact applied request receipt and payload. An older,
different, or inactive result remains unconfirmed.

Known conflicts retain the draft but block submission until explicit reload.
Reload rejects a changed entry key, revision rollback, and conflicting data at
the same revision. Stale callbacks from a closed/reopened view cannot complete
the new view. Pointer hit regions are also fenced by the rendered model epoch.

The configuration view does not resize Agent PTYs. Terminal bytes, writer leases,
and terminal input remain in the existing independently controlled paths. Closing
the view does not cancel an already accepted Hub-side configuration transaction.
