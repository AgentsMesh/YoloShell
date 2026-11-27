use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

#[tokio::test]
async fn active_release_waits_for_future_exact_alias_closure() {
    let mut fixture = support::MacosHandshakeFixture::new(None);
    let record = fixture.record.clone();
    let (server, release, sent, finish) = fixture.serve_held_active_release();
    fixture
        .host
        .ensure_secondary_custody(&record)
        .await
        .unwrap();
    let lease = fixture
        .host
        .secondary_custody
        .current(&CustodyRuntimeIdentity::from_record(&record))
        .unwrap();

    release.send(()).unwrap();
    sent.await.unwrap();
    fixture.close_worker_aliases();
    finish.send(()).unwrap();
    server.await.unwrap();
    wait_until(&fixture.host, || {
        lease.state() == SecondaryCustodyState::Released
    })
    .await;

    assert!(!lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
}

async fn wait_until(host: &Host, ready: impl Fn() -> bool) {
    tokio::time::timeout(host.worker_ready_timeout, async {
        while !ready() {
            tokio::time::sleep(host.worker_ready_poll_interval).await;
        }
    })
    .await
    .unwrap();
}
