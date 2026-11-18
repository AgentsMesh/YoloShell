# Local runtime release store

The store publishes one fully validated runtime bundle under
`releases/<build_id>/manifest.json` and exactly one executable, `releases/<build_id>/bin/yolo`. Uploaded stages
are private directories named `.upload-<canonical UUID>` under the installation
root. Binary files have mode 0700; manifest and selector JSON files have mode 0600.
Directories and files must be owned by the current user, with no ACL grants;
regular files must have one hard link. No symlink is accepted in the release tree.

`ReleaseStore::open` retains the root and releases directory identities.
`open_existing` and `inspect_existing` provide genuinely read-only observation:
they never create directories or lock files or normalize permissions. They retain
the selector identity before reading a release and revalidate it afterward;
concurrent replacement is an observation conflict, even for identical JSON.
`publish` acquires a nonblocking coordination lock, compares the expected
selection, validates the complete staged tree, syncs its files and directories,
publishes that directory with a no-replace rename, syncs both parents, and uses
the retained selector file identity to commit `current.json` atomically.
The selector contains exactly `schema_version`, `revision`, `operation_id`, and
`build_id`. Its revision increases for each new operation. An exact retry of the
currently selected operation/build is idempotent. A stale expected selection
cannot overwrite a successor.

Every artifact is hashed in bounded chunks. Retained descriptors, file birth
identities, timestamps, lengths, ownership, permissions, link counts, ACL state,
and exact directory membership are rechecked before publication and at selector
commit. Existing complete releases may be reused. Existing partial or corrupt
releases are rejected and never repaired in place.

An error may occur after a directory rename or selector publication. Callers
must inspect the exact operation/build with `inspect`, and may inspect a complete
unselected release with `inspect_release`. They must not infer that an error had
no effect. No method removes old releases or failed stages. This keeps evidence
available and avoids deleting executables still used by existing processes.

`InstalledBundle` returns one immutable release directory. Derive the `yolo` executable
for every explicit process role from that directory; selector changes never alter
paths already returned to a caller. Path ownership and executable identity must
still be respected by the subsequent process launcher.
