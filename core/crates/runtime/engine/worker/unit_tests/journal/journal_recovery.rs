use crate::worker::tests::*;
#[test]
fn pty_output_recovers_each_append_uncertainty_without_duplicates() {
    let stages = [
        crate::journal::AppendStage::BeforeWrite,
        crate::journal::AppendStage::AfterRecordSync,
        crate::journal::AppendStage::AfterCursorCommit,
    ];
    for stage in stages {
        let fixture = Fixture::new();
        let mut events = fixture.state.events.subscribe();
        fixture
            .state
            .journal
            .lock()
            .unwrap()
            .inject_append_failure(stage);

        stage_reader_output(&fixture.state, b"durable").unwrap();
        assert_eq!(
            deliver_pending_with_retry(&fixture.state).unwrap(),
            PendingDelivery::Delivered
        );

        let mut journal = fixture.state.journal.lock().unwrap();
        assert_eq!(journal.last_sequence(), 1, "stage {stage:?}");
        let chunks = journal.replay_after(0, usize::MAX).unwrap();
        assert_eq!(chunks.len(), 1, "stage {stage:?}");
        assert_eq!(chunks[0].sequence, 1, "stage {stage:?}");
        assert_eq!(chunks[0].data, b"durable", "stage {stage:?}");
        drop(journal);

        match events.try_recv().unwrap() {
            WorkerEvent::Output(chunk) => {
                assert_eq!(chunk.sequence, 1, "stage {stage:?}");
                assert_eq!(chunk.data, b"durable", "stage {stage:?}");
            }
            _ => panic!("unexpected worker event for stage {stage:?}"),
        }
        assert!(matches!(
            events.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        assert_eq!(
            lock_recover(&fixture.state.terminal).screen().contents(),
            "durable",
            "stage {stage:?}"
        );

        commit_reader_output(&fixture.state, b"-next").unwrap();
        assert_eq!(
            fixture.state.journal.lock().unwrap().last_sequence(),
            2,
            "stage {stage:?}"
        );
        assert_eq!(
            lock_recover(&fixture.state.terminal).screen().contents(),
            "durable-next",
            "stage {stage:?}"
        );
    }
}

#[test]
fn pty_output_retries_a_transient_reconcile_failure_with_payload_in_scope() {
    let fixture = Fixture::new();
    {
        let mut journal = fixture.state.journal.lock().unwrap();
        journal.inject_append_failure(crate::journal::AppendStage::AfterRecordSync);
        journal.inject_reconcile_failure();
    }

    stage_reader_output(&fixture.state, b"preserved").unwrap();
    assert_eq!(
        deliver_pending_with_retry(&fixture.state).unwrap(),
        PendingDelivery::Delivered
    );

    let mut journal = fixture.state.journal.lock().unwrap();
    assert_eq!(journal.last_sequence(), 1);
    let chunks = journal.replay_after(0, usize::MAX).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].data, b"preserved");
}
