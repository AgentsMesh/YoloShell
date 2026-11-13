use crate::worker::tests::*;
#[cfg(target_os = "macos")]
mod completed_adoption_tests {
    use super::*;

    const ATTACHMENT_START_SEQUENCE: u64 = 0;
    const COMPLETED_EXIT_CODE: i32 = 0;
    const COMPLETED_OUTPUT: &[u8] = b"completed-runtime-output";
    const CONTROL_INPUT: &[u8] = b"rejected-completed-input";
    const CONTROL_LEASE_HOLDER: &str = "completed-runtime-control";
    const CONTROL_RESIZE_COLUMNS: u16 = 100;
    const CONTROL_RESIZE_ROWS: u16 = 40;
    const CONTROL_SIGNAL: &str = "CONT";
    const EXPECTED_ATTACHMENT_CHUNK_COUNT: usize = 1;
    const FIRST_ATTACHMENT_CHUNK_INDEX: usize = 0;

    #[tokio::test(flavor = "current_thread")]
    async fn finalized_runtime_survives_repeated_released_successor_adoption() {
        let fixture = Fixture::new();
        let output = fixture.state.commit_output(COMPLETED_OUTPUT).unwrap();
        fixture.state.reader_lifecycle.mark_drained();
        finalize_on_thread(
            Arc::clone(&fixture.state),
            Some(COMPLETED_EXIT_CODE),
            None,
            Duration::ZERO,
        )
        .await;
        assert_completed_views(&fixture, &output);
        finalize_predecessor_release(&fixture).await;
        assert!(lock_recover(&fixture.state.master).is_none());

        let first = adopt_completed_runtime(&fixture).await;
        assert_persisted_generation(&fixture, first.generation);
        assert_completed_views(&fixture, &output);
        assert_completed_controls_rejected(&fixture).await;
        assert!(lock_recover(&fixture.state.master).is_none());
        drop(first.peer);
        await_adoption_failure(first.task).await;

        let second = adopt_completed_runtime(&fixture).await;
        assert!(second.generation > first.generation);
        assert_persisted_generation(&fixture, second.generation);
        assert_completed_views(&fixture, &output);
        assert!(lock_recover(&fixture.state.master).is_none());
        drop(second.peer);
        await_adoption_failure(second.task).await;
    }

    fn assert_persisted_generation(fixture: &Fixture, expected: u64) {
        assert_eq!(
            fixture
                .state
                .controller_generation
                .load(Ordering::Acquire),
            expected
        );
        assert_eq!(
            fixture
                .state
                .controller_persisted_generation
                .load(Ordering::Acquire),
            expected
        );
        assert_eq!(
            lock_recover(&fixture.state.record).controller_generation,
            expected
        );
        let persisted: RuntimeRecord = fixture
            .state
            .namespace
            .read_json(
                &fixture
                    .state
                    .paths
                    .runtime_record(fixture.terminal_id()),
            )
            .unwrap();
        assert_eq!(persisted.controller_generation, expected);
    }

    fn assert_completed_views(fixture: &Fixture, output: &OutputChunk) {
        let status = fixture.state.status();
        assert_eq!(status.status, TerminalRuntimeStatus::Exited);
        let exit = status.exit.unwrap();
        assert_eq!(exit.code, Some(COMPLETED_EXIT_CODE));
        assert_eq!(exit.reason, ExitReason::Natural);
        assert_eq!(exit.last_output_sequence, output.sequence);

        let snapshot = fixture.state.current_snapshot_internal().unwrap();
        assert_eq!(snapshot.runtime_epoch, fixture.state.runtime_epoch());
        assert_eq!(snapshot.barrier_sequence, output.sequence);
        assert!(
            snapshot
                .data
                .windows(COMPLETED_OUTPUT.len())
                .any(|window| window == COMPLETED_OUTPUT)
        );

        let attachment = fixture
            .state
            .attach_result_internal(ATTACHMENT_START_SEQUENCE)
            .unwrap();
        assert_eq!(attachment.runtime_epoch, fixture.state.runtime_epoch());
        assert!(attachment.snapshot.is_none());
        assert_eq!(attachment.chunks.len(), EXPECTED_ATTACHMENT_CHUNK_COUNT);
        let replayed = &attachment.chunks[FIRST_ATTACHMENT_CHUNK_INDEX];
        assert_eq!(replayed.runtime_epoch, output.runtime_epoch);
        assert_eq!(replayed.sequence, output.sequence);
        assert_eq!(replayed.data, output.data);
    }

    async fn assert_completed_controls_rejected(fixture: &Fixture) {
        let lease_request = fixture.lease_request(CONTROL_LEASE_HOLDER);
        let grant = lock_recover(&fixture.state.lease)
            .acquire(&lease_request, false, &mut |_| {})
            .unwrap();
        let mut fenced_lease = lease_request.clone();
        fenced_lease.generation = Some(grant.generation);
        let runtime = RuntimeRef::new(fixture.terminal_id(), fixture.state.runtime_epoch());
        let input = InputRequest {
            terminal_id: runtime.terminal_id,
            runtime_epoch: runtime.runtime_epoch,
            generation: grant.generation,
            writer_capability: grant.writer_capability.clone(),
            input_id: Uuid::new_v4(),
            data: CONTROL_INPUT.to_vec(),
        };
        let resize = ResizeRequest {
            terminal_id: runtime.terminal_id,
            runtime_epoch: runtime.runtime_epoch,
            generation: grant.generation,
            writer_capability: grant.writer_capability.clone(),
            cols: CONTROL_RESIZE_COLUMNS,
            rows: CONTROL_RESIZE_ROWS,
        };
        let signal = SignalRequest {
            terminal_id: runtime.terminal_id,
            runtime_epoch: runtime.runtime_epoch,
            generation: grant.generation,
            writer_capability: grant.writer_capability,
            signal: CONTROL_SIGNAL.into(),
        };

        assert_not_running(&fixture.state, method::ACQUIRE_WRITER, &lease_request).await;
        assert_not_running(&fixture.state, method::GET_WRITER_LEASE, &runtime).await;
        assert_not_running(&fixture.state, method::TAKEOVER_WRITER, &fenced_lease).await;
        assert_not_running(&fixture.state, method::RENEW_WRITER, &fenced_lease).await;
        assert_not_running(&fixture.state, method::RELEASE_WRITER, &fenced_lease).await;
        assert_not_running(&fixture.state, method::TERMINAL_INPUT, &input).await;
        assert_not_running(&fixture.state, method::RESIZE_TERMINAL, &resize).await;
        assert_not_running(&fixture.state, method::SIGNAL_SESSION, &signal).await;
    }

    async fn assert_not_running<T: serde::Serialize>(
        state: &Arc<WorkerState>,
        method_name: &str,
        payload: &T,
    ) {
        let error = dispatch_test(state, method_name, payload).await.unwrap_err();
        assert_eq!(error.code, lease_error_code::SESSION_NOT_RUNNING);
        assert!(!error.retryable);
    }
}
