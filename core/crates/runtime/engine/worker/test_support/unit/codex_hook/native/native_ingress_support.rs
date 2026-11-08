pub(super) use super::support::*;

// A native SessionStart has already committed before these ingress fault scenarios begin.
pub(super) const FIRST_EVENT_SEQUENCE: u64 = super::support::SECOND_EVENT_SEQUENCE;
pub(super) const SECOND_EVENT_SEQUENCE: u64 =
    FIRST_EVENT_SEQUENCE + crate::agent::hooks::FIRST_SEQUENCE;
pub(super) const GAP_EVENT_SEQUENCE: u64 =
    SECOND_EVENT_SEQUENCE + crate::agent::hooks::FIRST_SEQUENCE;
