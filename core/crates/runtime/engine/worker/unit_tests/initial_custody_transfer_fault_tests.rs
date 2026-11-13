use crate::worker::tests::*;
#[cfg(target_os = "macos")]
mod initial_custody_transfer_fault_tests {
    use super::*;
    use crate::worker::secondary_custody_source::InitialCustodyHandshakeBoundary;
    use std::sync::Arc;

    const FAULT_TEST_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT
        .saturating_add(crate::session_timing::WORKER_AUTH_TIMEOUT);
    const FAULT_TEST_TIMEOUT_DETAIL: &str = "initial custody fault test timed out";
    const FORGED_READY_GENERATION_INCREMENT: u64 = 1;

    #[derive(Clone, Copy)]
    enum InitialHandshakeFailure {
        Injected(InitialCustodyHandshakeBoundary),
        ReadyValidation,
    }

    impl InitialHandshakeFailure {
        fn injected(self) -> Option<InitialCustodyHandshakeBoundary> {
            match self {
                Self::Injected(boundary) => Some(boundary),
                Self::ReadyValidation => None,
            }
        }

        fn durable_is_armed(self) -> bool {
            matches!(
                self,
                Self::Injected(InitialCustodyHandshakeBoundary::CommitWrite)
                    | Self::ReadyValidation
            )
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn descriptor_send_failure_keeps_transfer_ambiguous() {
        assert_ambiguous_failure(InitialHandshakeFailure::Injected(
            InitialCustodyHandshakeBoundary::DescriptorSend,
        ))
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn prepared_read_failure_keeps_transfer_ambiguous() {
        assert_ambiguous_failure(InitialHandshakeFailure::Injected(
            InitialCustodyHandshakeBoundary::PreparedRead,
        ))
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn durable_persist_failure_keeps_transfer_ambiguous() {
        assert_ambiguous_failure(InitialHandshakeFailure::Injected(
            InitialCustodyHandshakeBoundary::DurablePersist,
        ))
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn commit_write_failure_keeps_transfer_ambiguous() {
        assert_ambiguous_failure(InitialHandshakeFailure::Injected(
            InitialCustodyHandshakeBoundary::CommitWrite,
        ))
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ready_validation_failure_keeps_transfer_ambiguous() {
        assert_ambiguous_failure(InitialHandshakeFailure::ReadyValidation).await;
    }

    async fn assert_ambiguous_failure(failure: InitialHandshakeFailure) {
        let fixture = Fixture::new();
        let record = lock_recover(&fixture.state.record).clone();
        let checkpoint = active_checkpoint(&fixture, &record);
        let (durable, durable_identity) = secondary_custody_persistence::publish_pending(
            &fixture.state.namespace,
            &fixture.state.paths,
            &record,
            &checkpoint,
        )
        .unwrap();
        let (listener, _socket_cleanup) = bind_worker_socket(
            &fixture.state.paths,
            fixture.terminal_id(),
            &record.socket_path,
        )
        .await
        .unwrap();
        let transfer = Arc::new(startup_custody_transfer::StartupCustodyTransferFence::new());
        let (source, _pair, _writer) = custody_source_with_transfer_for_test(
            &record,
            Arc::clone(&transfer),
            failure.injected(),
        );
        let source = Arc::new(source);
        let request = initial_fault_request(&record);
        let worker = secondary_custody_handshake::establish_initial(
            &listener,
            &fixture.state.namespace,
            &fixture.state.paths,
            &record,
            &durable,
            durable_identity,
            Arc::clone(&source),
        );
        let host = drive_faulting_host(fixture.state.paths.clone(), request.clone(), failure);
        let (worker, host) =
            tokio::time::timeout(FAULT_TEST_TIMEOUT, async { tokio::join!(worker, host) })
                .await
                .expect(FAULT_TEST_TIMEOUT_DETAIL);
        let offer = host.unwrap();

        assert!(worker.is_err());
        assert!(source.transfer_possible());
        assert_pending_host_release(&transfer);
        assert_durable_boundary(&fixture, &request, &offer, failure);
    }

    include!("../test_support/unit/initial_custody/initial_custody_transfer_fault_support.rs");
}
