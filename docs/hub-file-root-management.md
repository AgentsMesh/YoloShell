# TUI File Root Management

The TUI edits the entry Hub's file roots through the existing revision-fenced
configuration protocol. It uses a File Roots entry in Hub Configuration.
Selecting an imported Hub or Agent does not silently switch administrative
authority.

The list shows a root's name and origin-local directory. Add generates a stable
root ID; the ID is visible but read-only. Existing roots can be edited or removed.
Names and paths preserve their UTF-8 bytes, including trailing spaces. The TUI
uses the file protocol's path/name bounds instead of the host-name input limit.
It never opens or normalizes the selected filesystem path itself.

Saving a form changes only its in-memory draft. Apply submits the complete draft
with the observed configuration revision. The origin's existing supervised
preflight rejects invalid or protected roots before changing active configuration.
Removal and discarding a draft require confirmation; no Agent lifecycle action
is issued when managing a root.

## Conflict Boundaries

An edit or removal retains the complete original root record, not only its ID.
Deleted or changed roots cannot be overwritten by an old editor or action.
Swift equality compares name/path UTF-8 bytes, matching the Rust wire model;
Unicode canonical equivalence does not hide a byte-different configuration change.
The Mac removal handler also rechecks its draft after the confirmation completes.

Remote configuration conflicts preserve drafts and require explicit reload.
Uncertain Apply results retain the exact proposed configuration and use read-only
reconciliation; they do not resend the mutation. Pointer hit regions and editor
callbacks retain their existing generation/owner fencing.
