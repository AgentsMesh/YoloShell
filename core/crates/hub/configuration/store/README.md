# Hub configuration storage

The public configuration document, revision and last-change identifier are independent from
private storage metadata. Private record schema version 1 requires an explicit
`listener_binding_initialized` boolean. Initial creation persists false; management commits
true only after verifying the retained listener and its durable address. Ordinary configuration
updates preserve it.

Only the current complete record is accepted. Missing schema versions, missing initialization
evidence, unsupported versions, unknown fields and duplicate fields fail without rewriting the
record. Reading configuration never upgrades an older document or infers listener evidence.
