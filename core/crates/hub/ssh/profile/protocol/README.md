# Hub SSH profile registry protocol

This crate owns the registry's public data model and lexical validation. It has
no filesystem, runtime, remote broker, or process-launch dependency. The separate
transport-level `SshProfileRef` remains the public profile/revision reference.

`SshBrokerProfile` binds that reference to a normalized absolute broker-state
path and a configuration digest. Paths may name nonexistent files; validation
checks only UTF-8, length, scope, separators, components, and control characters.
Opening the broker state and matching actual credentials belong to the resolver.

`SshProfileConfiguration` has a schema and a bounded list with unique profile IDs
and broker-state paths. `SshProfileState` binds a full Hub identity, nonzero
revision, non-nil latest request ID, and configuration. `SshProfileChange` carries
a non-nil request ID, expected revision, and replacement configuration.

All types have public fields for drafts and explicit `validate` methods. Typed
deserialization also validates them and rejects missing, duplicate, and unknown
fields. No mutable registry or storage identity is manufactured by decoding DTOs.
