use crate::host::tests::*;
use super::*;

const TERMINATION_MATRIX_SOCKET_PREFIX: &str = "termination-matrix";
const TERMINATION_MATRIX_SOCKET_SUFFIX: &str = ".sock";
const REMOTE_TERMINATION_DETAIL: &str = "worker rejected termination";

#[derive(Clone, Copy, Debug)]
enum TerminationReply {
    Success,
    RemoteFailure,
    EmptyFailure,
}

async fn serve_termination_reply(
    listener: UnixListener,
    record: RuntimeRecord,
    reply: TerminationReply,
) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, &record).await;
    let request: RequestEnvelope = stream.request().await.unwrap();
    assert_eq!(request.method, method::TERMINATE_RUNTIME_EXACT);
    let payload: crate::worker::FencedTerminateRequest = request.decode_payload().unwrap();
    assert_eq!(payload.terminal_id, record.terminal_id);
    assert_eq!(payload.runtime_epoch, record.runtime_epoch);
    assert_eq!(payload.expected_status, None);
    let response = match reply {
        TerminationReply::Success => ResponseEnvelope::empty(&request),
        TerminationReply::RemoteFailure => {
            ResponseEnvelope::failure(&request, ErrorBody::conflict(REMOTE_TERMINATION_DETAIL))
        }
        TerminationReply::EmptyFailure => response_without_error(&request),
    };
    let observed = stream.respond(&response).await;
    if matches!(reply, TerminationReply::EmptyFailure) {
        assert!(matches!(
            observed,
            Err(crate::protocol::ProtocolError::EndOfStream)
        ));
    } else {
        observed.unwrap();
    }
}

#[tokio::test]
async fn termination_classifies_missing_archived_and_every_worker_reply() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let missing = host
        .terminate_bound_session(Uuid::new_v4(), false)
        .await
        .unwrap_err();
    assert_eq!(missing.code, crate::protocol::error_code::NOT_FOUND);

    let mut archived = fixture_manifest(directory.path());
    archived.archived = true;
    seed_recovery_manifest(&host, &archived).await;
    host.terminate_bound_session(archived.terminal_id, false)
        .await
        .unwrap();

    for (index, reply) in [
        TerminationReply::Success,
        TerminationReply::RemoteFailure,
        TerminationReply::EmptyFailure,
    ]
    .into_iter()
    .enumerate()
    {
        let manifest = fixture_manifest(directory.path());
        let socket = directory.path().join(format!(
            "{TERMINATION_MATRIX_SOCKET_PREFIX}-{index}{TERMINATION_MATRIX_SOCKET_SUFFIX}"
        ));
        let listener = UnixListener::bind(&socket).unwrap();
        let record = host_runtime_record(&host, manifest.terminal_id, socket);
        persist_host_session(&host, &manifest, &record);
        let server = tokio::spawn(serve_termination_reply(listener, record, reply));

        let result = host
            .terminate_bound_session(manifest.terminal_id, true)
            .await;
        match reply {
            TerminationReply::Success => assert!(result.is_ok()),
            TerminationReply::RemoteFailure => {
                let error = result.unwrap_err();
                assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
                assert_eq!(error.message, REMOTE_TERMINATION_DETAIL);
            }
            TerminationReply::EmptyFailure => {
                let error = result.unwrap_err();
                assert_eq!(
                    error.code,
                    crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
                );
                assert_eq!(
                    error.message,
                    crate::protocol::ProtocolError::ResponseMismatch.to_string()
                );
            }
        }
        server.await.unwrap();
    }
}

#[tokio::test]
async fn exit_wait_accepts_transport_loss_only_after_both_identities_exit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let socket = directory.path().join(format!(
        "{TERMINATION_MATRIX_SOCKET_PREFIX}-exit{TERMINATION_MATRIX_SOCKET_SUFFIX}"
    ));
    let mut worker = ArchiveWorkerProcessFixture::spawn(
        socket.clone(),
        ArchiveWorkerFixtureMode::StatusDisconnectBoth,
    );
    let mut record = host_runtime_record(&host, terminal_id, socket);
    worker.configure(&mut record);
    persist_archive_runtime(&host, &record);

    host.wait_for_session_exit(terminal_id).await.unwrap();
    worker.reap_completed_worker();
}

include!("lifecycle_deferred_termination_tests.rs");
