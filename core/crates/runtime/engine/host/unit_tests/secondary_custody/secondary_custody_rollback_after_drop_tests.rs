use super::super::super::rollback::test_hook::{self, Guard};
use super::*;

const UNCERTAIN_RELEASE_DETAIL: &str = "acknowledgement was cancelled after local release commit";

#[tokio::test]
async fn cancellation_after_platform_drop_before_ack_fail_stops_without_false_release() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let aliases = fixture.take_pre_activation_local_aliases();
    let record = fixture.record.clone();
    let mut boundary = test_hook::install(record.terminal_id);
    let (server, ready, release, finish) = fixture.serve_pre_activation_release(
        support::PreActivationReleaseIdentity::Exact,
        support::PreActivationReleaseResponse::HoldThenObserveNoAcknowledgement,
    );
    let lease = {
        let operation = fixture.host.ensure_secondary_custody(&record);
        tokio::pin!(operation);
        await_ready(ready, &mut operation).await;
        drop(aliases);
        let lease = current(&fixture.host, &record);
        release.send(()).unwrap();
        await_after_drop(&mut boundary, &mut operation).await;
        assert_eq!(lease.state(), SecondaryCustodyState::ReleaseAckPending);
        assert!(!lease.retains_platform());
        assert!(current_optional(&fixture.host, &record).is_none());
        lease
    };

    assert_eq!(lease.state(), SecondaryCustodyState::ReleaseUncertain);
    assert!(!lease.retains_platform());
    assert!(current_optional(&fixture.host, &record).is_none());
    assert!(
        fixture
            .host
            .request_drain
            .fail_stop_detail()
            .unwrap()
            .contains(UNCERTAIN_RELEASE_DETAIL)
    );
    finish.unwrap().send(()).unwrap();
    server.await.unwrap();
}

fn current_optional(host: &Host, record: &RuntimeRecord) -> Option<Arc<SecondaryCustodyLease>> {
    host.secondary_custody
        .current(&crate::pty_custody::CustodyRuntimeIdentity::from_record(
            record,
        ))
}

async fn await_after_drop(
    boundary: &mut Guard,
    operation: &mut (impl std::future::Future<Output = Result<(), WorkerRpcError>> + Unpin),
) {
    let reached = boundary.take_reached();
    tokio::select! {
        reached = reached => reached.unwrap(),
        result = operation => panic!("{UNEXPECTED_OPERATION_COMPLETION}: {result:?}"),
    }
}
