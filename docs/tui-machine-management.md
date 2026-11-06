# Native TUI Machine Management

The machine tree exposes a `Manage machines` footer action. Keyboard entry is
`m` while the tree has focus. Configuration always belongs to the current entry Hub;
selecting an imported machine or Agent never changes administrative authority.
The manager is a full-width terminal view, separate from Agent PTY surfaces.

## Editing

The view displays the local Hub ID, public key, configuration revision, and the
local link socket path.
Machines, trusted origins, file roots, and per-machine export policies
have separate forms. Keyboard and mouse navigation, bounded paste, UTF-8 text
editing, checkboxes, depth controls, scrolling, and wrapped identity fields are
supported. Read-only identity fields cannot be altered while editing a machine.
File root creation, editing and confirmed removal are described in
[Native file root management](hub-file-root-management.md).

Adding a machine requires its Hub ID and pinned public key. A dial
names an SSH destination and the remote peer socket path; it is optional: an
pinned machine can still export resources on an independently authorized
link. Import direction is an explicit checkbox and requires a dial. The remote machine's
identity and export authorization must be configured on that machine as well;
the TUI does not silently trust an unknown key.

Trusted origins support downstream origins such as C in A -> B -> C without
turning C into A's direct machine. Exports select an origin, independent permissions,
and a bounded reexport depth. New exports start with Observe only. File-read
permission is independent of terminal input; granting it does not manufacture
an origin capability that has not been implemented.

Each form edits an in-memory draft. Saving a form does not send a Hub mutation.
`Apply Changes` explicitly submits the complete validated draft with the base
revision. `Activate Configuration` is available when the persisted configuration
is inactive. Neither command launches, resumes, or terminates an Agent.

Removing a machine or trusted origin requires confirmation. Dependent exports are
removed only when no remaining machine/trusted-origin pin still authorizes that
origin. An export can be removed without removing its machine. Discarding a
dirty draft on close or reload also requires explicit confirmation.

## Consumers

The `Consumers` page is a read-only live view of the machines currently
importing from this Hub. Each row shows the importing Hub ID, its live link
count and connection age; `Refresh Consumers` issues a new `ListConsumers`
query. An empty page means no inbound link is open right now. The page never
grants or revokes access: admission and export policy remain the authority of
the configuration pages. Refreshing or leaving the page preserves an unsaved
draft, and closing the manager does not close inbound links.

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
