use super::*;
use crate::worker::reader_lifecycle::PendingOutput;

const FIRST_RETRY_ATTEMPT: u32 = 1;
const SECOND_RETRY_ATTEMPT: u32 = 2;

#[derive(Clone, Copy, Debug)]
enum RetainedFaultBoundary {
    PostInstall,
    ParentSync,
    Confirmation,
}

#[test]
fn retained_uncertainty_boundaries_reconcile_exactly_once() {
    for boundary in [
        RetainedFaultBoundary::PostInstall,
        RetainedFaultBoundary::ParentSync,
        RetainedFaultBoundary::Confirmation,
    ] {
        let fixture = Fixture::new();
        let mut events = fixture.state.events.subscribe();
        stage_reader_output(&fixture.state, PRECOMMIT_OUTPUT).unwrap();
        inject_boundary(&fixture, boundary);
        let moved_journal = displace_session_and_retain_journal(&fixture);

        assert_eq!(
            deliver_pending_with_retry(&fixture.state).unwrap(),
            PendingDelivery::NamespaceLost,
            "boundary {boundary:?}"
        );
        assert_confirmed_exactly_once(&fixture, moved_journal, &mut events);
    }
}

#[test]
fn retained_batch_recovery_publishes_distinct_chunks_exactly_once() {
    let fixture = Fixture::new();
    let mut events = fixture.state.events.subscribe();
    stage_reader_output(&fixture.state, PRECOMMIT_OUTPUT).unwrap();
    {
        let mut progress = fixture.state.reader_lifecycle.lock_progress();
        progress.pending.push(PendingOutput {
            data: b"second-batch-output".to_vec(),
            attempt: None,
        });
    }
    let moved_journal = displace_session_and_retain_journal(&fixture);

    assert_eq!(
        deliver_pending_with_retry(&fixture.state).unwrap(),
        PendingDelivery::NamespaceLost
    );
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert_eq!(fixture.state.termination_reason(), None);
    assert_eq!(fixture.state.journal.lock().unwrap().last_sequence(), 2);
    assert_eq!(
        lock_recover(&fixture.state.terminal).screen().contents(),
        format!(
            "{}second-batch-output",
            std::str::from_utf8(PRECOMMIT_OUTPUT).unwrap()
        )
    );
    assert_output_event(&mut events, 1, PRECOMMIT_OUTPUT);
    assert_output_event(&mut events, 2, b"second-batch-output");
    assert!(matches!(
        events.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));

    let mut recovered = Journal::open(moved_journal, fixture.state.runtime_epoch()).unwrap();
    let chunks = recovered.replay_after(u64::default(), usize::MAX).unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].sequence, 1);
    assert_eq!(chunks[0].data, PRECOMMIT_OUTPUT);
    assert_eq!(chunks[1].sequence, 2);
    assert_eq!(chunks[1].data, b"second-batch-output");
}

fn assert_output_event(events: &mut broadcast::Receiver<WorkerEvent>, sequence: u64, data: &[u8]) {
    match events.try_recv().unwrap() {
        WorkerEvent::Output(chunk) => {
            assert_eq!(chunk.sequence, sequence);
            assert_eq!(chunk.data, data);
        }
        event => panic!("unexpected worker event: {event:?}"),
    }
}

#[test]
fn ordinary_and_retained_failures_keep_independent_retry_sequences() {
    let mut retries = PendingCommitRetryBudget::default();
    assert_eq!(retries.record_ordinary_failure(), FIRST_RETRY_ATTEMPT);
    assert_eq!(retries.record_ordinary_failure(), SECOND_RETRY_ATTEMPT);
    assert_eq!(
        retries.record_retained_definitely_uncommitted(),
        FIRST_RETRY_ATTEMPT
    );
    assert_eq!(
        retries.record_retained_commit_uncertain(),
        SECOND_RETRY_ATTEMPT
    );
}

#[test]
fn pending_retained_retry_sequence_survives_delivery_scope_reentry() {
    let fixture = Fixture::new();
    stage_reader_output(&fixture.state, PRECOMMIT_OUTPUT).unwrap();
    {
        let mut progress = fixture.state.reader_lifecycle.lock_progress();
        progress
            .pending_commit_retries
            .record_retained_commit_uncertain();
    }

    let mut progress = fixture.state.reader_lifecycle.lock_progress();
    assert_eq!(
        progress
            .pending_commit_retries
            .record_retained_definitely_uncommitted(),
        SECOND_RETRY_ATTEMPT
    );
}

pub(super) fn assert_confirmed_exactly_once(
    fixture: &Fixture,
    moved_journal: std::path::PathBuf,
    events: &mut broadcast::Receiver<WorkerEvent>,
) {
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert!(!fixture.state.reader_lifecycle.drains_after_namespace_loss());
    assert_eq!(fixture.state.termination_reason(), None);
    assert_eq!(
        fixture.state.journal.lock().unwrap().last_sequence(),
        OUTPUT_TEST_SEQUENCE
    );
    assert_eq!(
        lock_recover(&fixture.state.terminal).screen().contents(),
        std::str::from_utf8(PRECOMMIT_OUTPUT).unwrap()
    );
    assert_one_output_event(events);
    let mut recovered = Journal::open(moved_journal, fixture.state.runtime_epoch()).unwrap();
    let chunks = recovered.replay_after(u64::default(), usize::MAX).unwrap();
    assert_eq!(chunks.len(), OUTPUT_TEST_SEQUENCE as usize);
    assert_eq!(chunks[usize::default()].sequence, OUTPUT_TEST_SEQUENCE);
    assert_eq!(chunks[usize::default()].data, PRECOMMIT_OUTPUT);
}

fn inject_boundary(fixture: &Fixture, boundary: RetainedFaultBoundary) {
    let mut journal = fixture.state.journal.lock().unwrap();
    match boundary {
        RetainedFaultBoundary::PostInstall => journal.inject_retained_post_install_failure(),
        RetainedFaultBoundary::ParentSync => journal.inject_retained_parent_sync_failure(),
        RetainedFaultBoundary::Confirmation => journal.inject_retained_confirmation_failure(),
    }
}

fn assert_one_output_event(events: &mut broadcast::Receiver<WorkerEvent>) {
    match events.try_recv().unwrap() {
        WorkerEvent::Output(chunk) => {
            assert_eq!(chunk.sequence, OUTPUT_TEST_SEQUENCE);
            assert_eq!(chunk.data, PRECOMMIT_OUTPUT);
        }
        event => panic!("unexpected worker event: {event:?}"),
    }
    assert!(matches!(
        events.try_recv(),
        Err(broadcast::error::TryRecvError::Empty)
    ));
}
