use super::tests_retained_hooks::set_confirm_sequence_override;
use super::tests_retained_support::{RETAINED_PAYLOAD, RetainedFixture};
use super::*;

const PRE_IO_FAULT_COUNT: usize = 1;

#[test]
fn pre_io_failure_is_definite_at_the_original_cursor() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::BeforeWrite);
    fixture
        .journal
        .inject_retained_reconcile_failures(PRE_IO_FAULT_COUNT);

    assert!(matches!(
        fixture
            .journal
            .reconcile_append_retained_classified(fixture.attempt, RETAINED_PAYLOAD),
        RetainedAppendOutcome::DefinitelyUncommitted(_)
    ));
    let runtime_epoch = fixture.journal.runtime_epoch();
    let mut reopened = Journal::open(&fixture.moved_journal, runtime_epoch).unwrap();
    assert!(
        reopened
            .replay_after(EMPTY_SEQUENCE, usize::MAX)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn pre_io_failure_is_uncertain_after_the_cursor_committed() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterCursorCommit);
    fixture
        .journal
        .inject_retained_reconcile_failures(PRE_IO_FAULT_COUNT);

    assert!(matches!(
        fixture
            .journal
            .reconcile_append_retained_classified(fixture.attempt, RETAINED_PAYLOAD),
        RetainedAppendOutcome::CommitUncertain(_)
    ));
    fixture.reconcile().unwrap().unwrap();
    fixture.assert_exactly_once();
}

#[test]
fn late_confirmation_failure_retries_without_adopting_partial_state() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterCursorCommit);
    set_confirm_sequence_override(u64::MAX);

    assert!(matches!(
        fixture
            .journal
            .reconcile_append_retained_classified(fixture.attempt, RETAINED_PAYLOAD),
        RetainedAppendOutcome::CommitUncertain(JournalError::Corrupt { sequence: u64::MAX })
    ));
    assert_eq!(fixture.journal.last_sequence(), EMPTY_SEQUENCE);
    fixture.reconcile().unwrap().unwrap();
    assert_eq!(fixture.journal.last_sequence(), INITIAL_RETAINED_SEQUENCE);
    fixture.assert_exactly_once();
}
