use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

const STREAM_EOF_DETAIL: &str = "read pre-activation PTY custody rollback stream";
const UNEXPECTED_OPERATION_COMPLETION: &str = "custody operation completed before Ready";

#[tokio::test]
async fn eof_after_exact_pre_activation_release_fail_stops_and_retains_platform() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let _aliases = fixture.take_pre_activation_local_aliases();
    let record = fixture.record.clone();
    let (server, ready, release, _finish) = fixture.serve_pre_activation_release(
        support::PreActivationReleaseIdentity::Exact,
        support::PreActivationReleaseResponse::DropWithoutReading,
    );
    let operation = fixture.host.ensure_secondary_custody(&record);
    tokio::pin!(operation);
    tokio::select! {
        reached = ready => reached.unwrap(),
        result = &mut operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
    }
    let lease = current(&fixture.host, &record);

    release.send(()).unwrap();
    let error = operation.await.unwrap_err();
    server.await.unwrap();

    assert!(matches!(error, WorkerRpcError::Transport(_)));
    assert!(error.to_string().contains(STREAM_EOF_DETAIL));
    assert_eq!(lease.state(), SecondaryCustodyState::RollingBack);
    assert!(lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_some());
    assert!(Arc::ptr_eq(&lease, &current(&fixture.host, &record)));
}

fn current(host: &Host, record: &RuntimeRecord) -> Arc<SecondaryCustodyLease> {
    host.secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(record))
        .unwrap()
}
