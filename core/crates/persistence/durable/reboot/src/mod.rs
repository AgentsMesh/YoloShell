#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/adversarial.rs"]
#[doc(hidden)]
pub mod adversarial;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/adversarial_targets.rs"]
mod adversarial_targets;
#[path = "constants.rs"]
mod constants;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/copied_directory.rs"]
mod copied_directory;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/directory_replacement.rs"]
mod directory_replacement;
mod durable_io;
#[cfg(test)]
#[path = "../test_support/durable_test_io.rs"]
mod durable_test_io;
#[path = "guard.rs"]
mod guard;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/guard_reserved_path_tests.rs"]
mod guard_reserved_path_tests;
#[cfg(test)]
#[path = "../test_support/guard_write.rs"]
mod guard_write;
#[path = "install.rs"]
mod install;
#[path = "install_cleanup.rs"]
mod install_cleanup;
#[path = "install_cleanup_state.rs"]
mod install_cleanup_state;
#[path = "install_optional.rs"]
mod install_optional;
#[path = "install_participant.rs"]
mod install_participant;
#[path = "install_pty.rs"]
mod install_pty;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/install_pty_tests.rs"]
mod install_pty_tests;
#[path = "install_publication.rs"]
mod install_publication;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/install_publication_adversarial_tests.rs"]
mod install_publication_adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/install_publication_fixture.rs"]
mod install_publication_fixture;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/install_publication_tests.rs"]
mod install_publication_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/intent_identity_tests.rs"]
mod intent_identity_tests;
#[path = "intent_io.rs"]
mod intent_io;
#[path = "paths.rs"]
mod participant_paths;
#[path = "preparation_authority.rs"]
mod preparation_authority;
#[path = "preparation_authority_validation.rs"]
mod preparation_authority_validation;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../test_support/preparation_capture.rs"]
mod preparation_capture;
#[path = "preparation_recovery.rs"]
mod preparation_recovery;
#[path = "preparation_recovery_io.rs"]
mod preparation_recovery_io;
#[path = "preparation_recovery_participant.rs"]
mod preparation_recovery_participant;
#[path = "preparation_retirement.rs"]
mod preparation_retirement;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/preparation_retirement_tests.rs"]
mod preparation_retirement_tests;
#[path = "preparation_schema.rs"]
mod preparation_schema;
#[path = "preparation_settled.rs"]
mod preparation_settled;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../test_support/prepare.rs"]
mod prepare;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../test_support/prepare_capture.rs"]
mod prepare_capture;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../test_support/prepare_cleanup.rs"]
mod prepare_cleanup;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../test_support/prepare_participant.rs"]
mod prepare_participant;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/prepare_test_hook.rs"]
mod prepare_test_hook;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/prepare_transaction_tests.rs"]
mod prepare_transaction_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/pty_custody_fixture.rs"]
pub(crate) mod pty_custody_fixture;
#[path = "publication_marker.rs"]
mod publication_marker;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/publication_marker_tests.rs"]
mod publication_marker_tests;
#[path = "receipt.rs"]
mod receipt;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../test_support/receipt_prepare.rs"]
mod receipt_prepare;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/remount_handoff.rs"]
mod remount_handoff;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/remount_protocol.rs"]
mod remount_protocol;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/remount_steps.rs"]
mod remount_steps;
#[path = "schema.rs"]
mod schema;
#[path = "session_discovery.rs"]
mod session_discovery;
#[path = "settlement.rs"]
mod settlement;
#[path = "staging.rs"]
mod staging;
#[path = "staging_displaced.rs"]
mod staging_displaced;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "../test_support/staging_prepare.rs"]
mod staging_prepare;
#[cfg(test)]
#[path = "../test_support/boot.rs"]
mod test_boot;
#[path = "validation.rs"]
mod validation;
#[path = "validation_participants.rs"]
mod validation_participants;

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(crate) use guard::SessionDurableMutationGuard;
#[cfg(test)]
pub(crate) use guard_write::DurableMutationFencePhase;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(crate) use prepare::{ParticipantTargets, prepare_and_publish_intent};
#[cfg(test)]
pub(crate) use prepare::{prepare_intent, publish_intent};
#[cfg(test)]
pub(crate) use schema::{OptionalParticipant, RebootMigrationIntent};
pub use settlement::{settle_all_pending, settle_pending_session, settle_pending_session_locked};
