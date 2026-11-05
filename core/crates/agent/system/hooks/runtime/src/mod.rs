mod admission;
mod constants;
mod event;
mod identity;
mod lifecycle_metadata;
mod mapping;
mod reducer;
mod revocation;
mod snapshot;
mod state;
mod transition;
mod types;
mod violation;

pub mod sha256_digest {
    pub use yolo_crypto::sha256_digest::*;
}

pub use constants::MAX_HOOK_IDENTIFIER_BYTES;
#[cfg(test)]
pub use constants::MAX_RETAINED_LIFECYCLE_ENTRIES;
#[cfg(any(test, feature = "yolo-test-support"))]
pub use constants::test_values;
pub use constants::{
    AUTHORITY_REVISION_STEP, CURRENT_SCHEMA_VERSION, FIRST_SEQUENCE, INITIAL_GENERATION,
    MAX_REPLAY_WINDOW, ZERO_SEQUENCE, event_names, provider_names,
};
pub use reducer::{ApplyDisposition, ProcessExitDisposition, Reducer};
pub use revocation::{HookEventRevocationProof, RevocationEvidence};
pub use snapshot::{ReducerSnapshot, SnapshotViolation};
pub use types::{
    AttentionState, ChildLifecycle, ClaudeEventMapper, CodexEventMapper, EventKind, EventMapper,
    EventSequence, ExpectedIdentity, Generation, HookEnvelope, HookIdentity, HookMetadata,
    HookViolation, InstrumentationState, NormalizedLifecycle, NotificationKind, ProcessIdentity,
    ProcessState, ProviderEventMapper, HookProviderKind, ProviderVersion, SessionEndReason,
    SessionStartSource, StopCandidate, TerminationCandidate, TurnState,
};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/boundary_tests.rs"]
mod boundary_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/codex_contract_tests.rs"]
mod codex_contract_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/compaction_phase_tests.rs"]
mod compaction_phase_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/inactive_child_tests.rs"]
mod inactive_child_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lifecycle_capacity_tests.rs"]
mod lifecycle_capacity_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lifecycle_transition_tests.rs"]
mod lifecycle_transition_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/permission_candidate_tests.rs"]
mod permission_candidate_tests;
#[cfg(any(test, feature = "yolo-test-support"))]
#[path = "../test_support/reducer.rs"]
mod reducer_test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/replay_window_tests.rs"]
mod replay_window_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/revision_exhaustion_tests.rs"]
mod revision_exhaustion_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_adversarial_tests.rs"]
mod snapshot_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_canonical_adversarial_tests.rs"]
mod snapshot_canonical_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_metadata_tests.rs"]
mod snapshot_metadata_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_revocation_adversarial_tests.rs"]
mod snapshot_revocation_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_revocation_predecessor_tests.rs"]
mod snapshot_revocation_predecessor_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_revocation_proof_tests.rs"]
mod snapshot_revocation_proof_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_stop_candidate_tests.rs"]
mod snapshot_stop_candidate_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_tail_adversarial_tests.rs"]
mod snapshot_tail_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_tests.rs"]
mod snapshot_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/snapshot_wire_adversarial_tests.rs"]
mod snapshot_wire_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/terminal_fence_tests.rs"]
mod terminal_fence_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/terminal_state_tests.rs"]
mod terminal_state_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/fixtures.rs"]
mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
