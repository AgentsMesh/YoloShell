use super::reconcile_retained::validate_retained_payload_len;
use super::tests_retained_hooks::{set_confirm_cursor_override, set_confirm_sequence_override};
use super::tests_retained_support::RetainedFixture;
use super::*;

const MAX_RETAINED_PAYLOAD_LENGTH: usize = u32::MAX as usize;
const OVERSIZED_RETAINED_PAYLOAD_LENGTH: usize =
    MAX_RETAINED_PAYLOAD_LENGTH + INITIAL_RETAINED_SEQUENCE as usize;

#[test]
fn retained_payload_length_boundary_is_checked_without_allocating_it() {
    assert!(validate_retained_payload_len(MAX_RETAINED_PAYLOAD_LENGTH).is_ok());
    assert!(matches!(
        validate_retained_payload_len(OVERSIZED_RETAINED_PAYLOAD_LENGTH),
        Err(JournalError::PayloadTooLarge)
    ));
}

#[test]
fn cursor_drift_after_verification_fails_before_state_adoption() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterCursorCommit);
    set_confirm_cursor_override(EMPTY_SEQUENCE);

    assert!(fixture.reconcile().is_err());

    fixture.assert_replacements_unchanged();
}

#[test]
fn scanned_sequence_drift_fails_before_returning_a_chunk() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterCursorCommit);
    set_confirm_sequence_override(u64::MAX);

    assert!(matches!(
        fixture.reconcile(),
        Err(JournalError::Corrupt { sequence: u64::MAX })
    ));

    fixture.assert_replacements_unchanged();
}
