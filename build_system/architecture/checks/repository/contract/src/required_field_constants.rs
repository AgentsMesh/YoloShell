pub(super) const OFFER_OWNER: &str = "core/crates/runtime/engine/pty_custody/src/protocol/mod.rs";
pub(super) const RECORD_OWNER: &str = "core/crates/runtime/engine/pty_custody/src/record/mod.rs";
pub(super) const PROFILE_OWNER: &str = "core/crates/runtime/engine/pty_custody/src/profile.rs";
pub(super) const ENVELOPE_OWNER: &str = "core/crates/wire/src/envelope.rs";
pub(super) const OFFER_TYPE: &str = "CustodyOffer";
pub(super) const STATE_TYPE: &str = "CustodyDurableState";
pub(super) const LEASE_TYPE: &str = "CustodyDurableLease";
pub(super) const PROFILE_TYPE: &str = "CustodyProfile";
pub(super) const STATE_VARIANTS: &[&str] = &["Armed", "Retired"];
pub(super) const ENVELOPE_TYPES: &[&str] =
    &["RequestEnvelope", "ResponseEnvelope", "EventEnvelope"];
pub(super) const PROFILE_FIELD: &str = "profile";
pub(super) const PAYLOAD_FIELD: &str = "payload";
pub(super) const PAYLOAD_TYPE: &str = "Value";
pub(super) const SERDE: &str = "serde";
pub(super) const DERIVE: &str = "derive";
pub(super) const DEFAULT_TRAIT: &str = "Default";
pub(super) const DENY_UNKNOWN: &str = "deny_unknown_fields";
pub(super) const INFERRED_FIELD_OPTIONS: &[&str] = &[
    "default",
    "skip",
    "skip_deserializing",
    "skip_serializing",
    "skip_serializing_if",
];
pub(super) const FIELD_MESSAGE: &str = "custody profiles and runtime envelope payloads require direct field types without optional wrappers, default or skip adapters";
pub(super) const ENVELOPE_MESSAGE: &str = "runtime envelope types must reject unknown fields";
pub(super) const PROFILE_MESSAGE: &str = "CustodyProfile must not implement or derive Default";
pub(super) const FIRST_LINE: usize = 1;
