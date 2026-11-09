use crate::worker::tests::*;
const OUTPUT_RECONCILE_FAILURE_COUNT: usize = 3;
const SEALED_STAGING_OUTPUT: &[u8] = b"later-output";
const OUTPUT_EXPECTED_READS: usize = 2;

struct CountingReader {
    reads: Arc<std::sync::atomic::AtomicUsize>,
}

impl Read for CountingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let call = self.reads.fetch_add(1, Ordering::AcqRel);
        if call == 0 {
            let output = b"pending-output";
            buffer[..output.len()].copy_from_slice(output);
            Ok(output.len())
        } else {
            Ok(0)
        }
    }
}

#[test]
fn uncertain_append_is_reconciled_once_without_duplicate_effects() {
    let fixture = Fixture::new();
    let mut events = fixture.state.events.subscribe();
    stage_reader_output(&fixture.state, b"durable-once").unwrap();
    {
        let mut journal = fixture.state.journal.lock().unwrap();
        journal.inject_append_failure(crate::journal::AppendStage::AfterCursorCommit);
        journal.inject_reconcile_failures(OUTPUT_RECONCILE_FAILURE_COUNT);
    }

    assert!(commit_staged_reader_output(&fixture.state).is_err());
    assert!(fixture.state.reader_lifecycle.has_pending());
    assert_eq!(
        lock_recover(&fixture.state.terminal).screen().contents(),
        ""
    );
    assert!(events.try_recv().is_err());
    assert_eq!(
        deliver_pending_with_retry(&fixture.state).unwrap(),
        PendingDelivery::Delivered
    );

    let mut journal = fixture.state.journal.lock().unwrap();
    let chunks = journal.replay_after(0, usize::MAX).unwrap();
    assert_eq!(chunks.len(), OUTPUT_TEST_SEQUENCE as usize);
    assert_eq!(chunks[0].data, b"durable-once");
    assert_eq!(
        lock_recover(&fixture.state.terminal).screen().contents(),
        "durable-once"
    );
    assert!(matches!(events.try_recv(), Ok(WorkerEvent::Output(_))));
    assert!(events.try_recv().is_err());
}

#[test]
fn staging_output_revokes_a_committed_kernel_eof_seal_epoch() {
    let fixture = Fixture::new();
    let mut output = ExclusivePtyOutput::observed(Arc::new(AtomicUsize::default()));
    output.install_sealed_eof_for_activation_test();
    fixture
        .state
        .reader_lifecycle
        .adopt_exclusive_kernel_eof_before(&mut output, || Ok(()))
        .unwrap();
    assert!(
        fixture
            .state
            .reader_lifecycle
            .lock_progress()
            .kernel_eof_seal_epoch
            .is_some()
    );

    stage_reader_output(&fixture.state, SEALED_STAGING_OUTPUT).unwrap();

    let progress = fixture.state.reader_lifecycle.lock_progress();
    assert!(progress.kernel_eof_seal_epoch.is_none());
    assert!(!progress.eof.has_authority());
}

#[path = "output_namespace_loss_tests.rs"]
mod namespace_loss_tests;
