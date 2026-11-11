use super::*;

#[tokio::test]
async fn live_pre_activation_rollback_cannot_manufacture_terminal_settlement_or_retirement() {
    let mut fixture = support::MacosHandshakeFixture::with_live_child(None);
    let aliases = fixture.take_pre_activation_local_aliases();
    let record = fixture.record.clone();
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
        let path = fixture.host.paths.pty_custody(record.terminal_id);
        let before: crate::pty_custody::CustodyDurableRecord =
            crate::paths::read_json(&path).unwrap();
        release.send(()).unwrap();
        await_state_while_pending(
            &mut operation,
            &fixture.host,
            &lease,
            SecondaryCustodyState::RollingBack,
        )
        .await;
        assert!(
            super::super::super::settlement::prove_exact(&lease)
                .unwrap()
                .is_none()
        );
        assert!(
            crate::process::ProcessIdentity::capture(record.child_pid)
                .unwrap()
                .start_time
                == record.child_start_time
        );
        assert!(lease.retains_platform());
        let after: crate::pty_custody::CustodyDurableRecord =
            crate::paths::read_json(&path).unwrap();
        assert!(before == after);
        assert!(after.armed_lease().is_some());
        lease
    };
    assert!(fixture.host.request_drain.fail_stop_detail().is_some());
    assert!(lease.retains_platform());
    finish.unwrap().send(()).unwrap();
    server.await.unwrap();
}
