use super::constants::*;
use super::schema::*;
use crate::process::BootIdentityRelation::{Different, Same};
use anyhow::{Result, ensure};
use serde::Serialize;

pub(super) fn validate(intent: &RebootMigrationIntent) -> Result<()> {
    ensure!(valid_header(intent), "durable reboot validation: header");
    super::validation_participants::validate_startup(intent)
        .map_err(|error| error.context("durable reboot validation: startup"))?;
    super::validation_participants::validate_claude_custody(intent)
        .map_err(|error| error.context("durable reboot validation: claude custody"))?;
    super::validation_participants::validate_claude_generation(intent)
        .map_err(|error| error.context("durable reboot validation: claude generation"))?;
    super::validation_participants::validate_pty_custody(intent)
        .map_err(|error| error.context("durable reboot validation: pty custody"))?;
    super::validation_participants::validate_runtime(intent)
        .map_err(|error| error.context("durable reboot validation: runtime"))
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn validate_prepared(intent: &RebootMigrationIntent) -> Result<()> {
    validate(intent)?;
    ensure!(
        intent.phase == RebootMigrationPhase::Prepared,
        INTENT_INVALID
    );
    Ok(())
}

pub(super) fn validate_transition<T, F>(
    participant: &RequiredParticipant<T>,
    source_from_target: F,
) -> Result<()>
where
    T: Serialize,
    F: FnOnce(&T) -> Result<T>,
{
    ensure!(
        payload_digest(&participant.target_payload)? == participant.target_sha256,
        INTENT_INVALID
    );
    let source = source_from_target(&participant.target_payload)?;
    ensure!(
        payload_digest(&source)? == participant.source_sha256,
        INTENT_INVALID
    );
    Ok(())
}

pub(super) fn validate_optional<T, F>(
    participant: &OptionalParticipant<T>,
    source_from_target: F,
) -> Result<()>
where
    T: Serialize,
    F: FnOnce(&T) -> Result<T>,
{
    match participant {
        OptionalParticipant::Absent => Ok(()),
        OptionalParticipant::Replace(required) => validate_transition(required, source_from_target),
    }
}

fn valid_header(intent: &RebootMigrationIntent) -> bool {
    intent.schema_version == REBOOT_INTENT_SCHEMA_VERSION
        && !intent.transaction_id.is_nil()
        && !intent.terminal_id.is_nil()
        && !intent.runtime_epoch.is_nil()
        && !intent.worker_id.is_nil()
        && intent.runtime_epoch != intent.worker_id
        && intent.controller_generation != u64::default()
        && known_boot(&intent.source_boot_id)
        && known_boot(&intent.target_boot_id)
        && crate::process::boot_identity_relation(&intent.source_boot_id, &intent.target_boot_id)
            == Different
        && matches!(intent.startup_checkpoint, OptionalParticipant::Replace(_))
}

fn payload_digest<T: Serialize>(value: &T) -> Result<crate::sha256_digest::Sha256Digest> {
    crate::sha256_digest::Sha256Digest::capture(value)
}

fn known_boot(value: &str) -> bool {
    crate::process::boot_identity_relation(value, value) == Same
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/validation_adversarial_tests.rs"]
mod adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/validation_guardian_preparation_tests.rs"]
mod guardian_preparation_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/validation_rollover_fixture.rs"]
pub(super) mod rollover_fixture;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/validation_rollover_proof_tests.rs"]
mod rollover_proof_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/validation_rollover_tests.rs"]
mod rollover_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/validation_test_fixture.rs"]
pub(super) mod test_fixture;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/validation_test_fixture_participants.rs"]
mod test_fixture_participants;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/validation_test_fixture_records.rs"]
mod test_fixture_records;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/validation_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/intent_header_matrix_tests.rs"]
mod header_matrix_tests;
