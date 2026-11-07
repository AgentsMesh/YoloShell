use super::*;

#[derive(Clone, Copy)]
enum PeerReply {
    Disconnect,
    ShuttingDown,
    Unauthorized,
}

#[tokio::test]
async fn retirement_control_distinguishes_authentication_from_shutdown_failures() {
    for after_authentication in [false, true] {
        for reply in [
            PeerReply::Disconnect,
            PeerReply::ShuttingDown,
            PeerReply::Unauthorized,
        ] {
            assert_control_phase(after_authentication, reply).await;
        }
    }
}

async fn assert_control_phase(after_authentication: bool, reply: PeerReply) {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let authenticated = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let authentication: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        assert_eq!(authentication.method, crate::worker::WORKER_AUTH_METHOD);
        if after_authentication {
            write_frame(
                &mut stream,
                &ResponseEnvelope::success(&authentication, &test_worker_hello(&authenticated))
                    .unwrap(),
            )
            .await
            .unwrap();
            let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
            let shutdown = stream.request().await.unwrap();
            assert_eq!(shutdown.method, WORKER_SHUTDOWN_METHOD);
            let code = match reply {
                PeerReply::Disconnect => return,
                PeerReply::ShuttingDown => crate::protocol::error_code::WORKER_SHUTTING_DOWN,
                PeerReply::Unauthorized => crate::protocol::error_code::UNAUTHORIZED,
            };
            let error = ErrorBody::new(code, authenticated.auth_token, true);
            stream
                .respond(&ResponseEnvelope::failure(&shutdown, error))
                .await
                .unwrap();
            return;
        }
        let code = match reply {
            PeerReply::Disconnect => return,
            PeerReply::ShuttingDown => crate::protocol::error_code::WORKER_SHUTTING_DOWN,
            PeerReply::Unauthorized => crate::protocol::error_code::UNAUTHORIZED,
        };
        let error = ErrorBody::new(code, authenticated.auth_token, true);
        write_frame(
            &mut stream,
            &ResponseEnvelope::failure(&authentication, error),
        )
        .await
        .unwrap();
    });
    let error = host
        .retire_released_worker_classified(&record)
        .await
        .unwrap_err();
    server.await.unwrap();
    let phase = match (after_authentication, reply) {
        (false, _) => WorkerRetirementControlPhase::Authentication,
        (true, PeerReply::Disconnect) => WorkerRetirementControlPhase::ShutdownExchange,
        (true, _) => WorkerRetirementControlPhase::ShutdownResponse,
    };
    let failure = match reply {
        PeerReply::Disconnect if after_authentication => {
            WorkerRetirementControlFailure::OutcomeUnknown
        }
        PeerReply::Disconnect => WorkerRetirementControlFailure::Transport,
        PeerReply::ShuttingDown => WorkerRetirementControlFailure::WorkerShuttingDown,
        PeerReply::Unauthorized => WorkerRetirementControlFailure::RemoteRejected,
    };
    assert_eq!(
        error.observation(),
        WorkerRetirementObservation::Control { phase, failure }
    );
    assert_eq!(
        error.may_settle_after_disconnect(),
        !matches!(reply, PeerReply::Unauthorized)
    );
    let diagnostic =
        host.unresolved_previous_runtime_after_retirement(&record, error.observation());
    assert!(!diagnostic.message.contains(&record.auth_token));
}

#[tokio::test]
async fn invalid_released_evidence_is_classified_before_any_authentication_attempt() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let exit_path = host.paths.exit_record(record.terminal_id);
    let mut exit: ExitRecord = read_json(&exit_path).unwrap();
    exit.runtime_epoch = Uuid::new_v4();
    atomic_write_json(&exit_path, &exit).unwrap();

    let error = host
        .retire_released_worker_classified(&record)
        .await
        .unwrap_err();

    assert_eq!(
        error.observation(),
        WorkerRetirementObservation::Control {
            phase: WorkerRetirementControlPhase::ReleasedEvidenceBeforeAuth,
            failure: WorkerRetirementControlFailure::RemoteRejected,
        }
    );
    assert!(!error.may_settle_after_disconnect());
    assert!(
        tokio::time::timeout(PROCESS_EXIT_POLL_INTERVAL, listener.accept())
            .await
            .is_err()
    );
}

#[test]
fn retirement_failure_classification_never_inspects_error_messages() {
    const UNTRUSTED_DETAIL: &str = "worker_shutting_down secret-auth-token";
    for (rpc, expected) in [
        (
            WorkerRpcError::protocol(UNTRUSTED_DETAIL),
            WorkerRetirementControlFailure::Protocol,
        ),
        (
            WorkerRpcError::identity_mismatch(UNTRUSTED_DETAIL),
            WorkerRetirementControlFailure::IdentityMismatch,
        ),
        (
            WorkerRpcError::remote(ErrorBody::conflict(UNTRUSTED_DETAIL)),
            WorkerRetirementControlFailure::RemoteRejected,
        ),
    ] {
        let error =
            WorkerRetirementError::control(WorkerRetirementControlPhase::Authentication, rpc);
        assert_eq!(
            error.observation(),
            WorkerRetirementObservation::Control {
                phase: WorkerRetirementControlPhase::Authentication,
                failure: expected,
            }
        );
        assert!(!error.may_settle_after_disconnect());
        assert!(!format!("{:?}", error.observation()).contains(UNTRUSTED_DETAIL));
    }
}
