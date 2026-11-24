# Runtime publication protocol

This component contains only validated wire types and lexical path policy. It
does not open files, start processes, or depend on the release store or SSH.

`Selection` owns the single selector schema: `schema_version`, `revision`,
`operation_id`, and `build_id`. The release store reexports this type. Revisions
are nonzero, operation IDs are non-nil, and build IDs are lowercase SHA-256.

`PublicationRequest` carries its schema, canonical installation root, operation ID, stage UUID, complete
bundle descriptor, and expected selector. The expected field is required even
when its value is null. The installation-root field is required. Requests carry
no executable paths, credentials, or commands. The independent CLI root argument
must match the root persisted in the request before any filesystem mutation.

`PublicationReceipt` binds the operation, stage, exact resulting selector, and
immutable release directory. Callers must use `validate(request, root)` after
decoding: syntactically valid receipts from a different request are insufficient.

`InspectionReceipt` contains the installation root and optional verified current
selection, descriptor, and release directory. Its selected field is also required
when null. Decoding checks internal consistency; `validate(root)` binds the
observation to the caller's installation root.

All JSON entry points enforce a 64 KiB limit and typed strict field decoding.
Identical duplicate keys, unknown keys, missing required keys, invalid schemas,
inconsistent identities, and revision overflow are rejected. Lexical path policy
accepts scoped absolute UTF-8 paths without controls or ambiguous components;
remote canonicalization and filesystem ownership remain transport/store duties.
