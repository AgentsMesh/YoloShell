use super::{recovery_constants::*, recovery_fixture::Fixture};
use crate::recover;
use yolo_remote_provisioning_protocol::EnrollmentOutcome;

#[tokio::test]
async fn historical_completion_does_not_hide_a_changed_live_peer_key_or_unapplied_generation() {
    for change_key in [false, true] {
        let mut fixture = Fixture::new(false).await;
        crate::enrollment::enroll(
            &fixture.directories,
            &fixture.connection(),
            &fixture.request,
            deadline(),
        )
        .await
        .unwrap();
        let before = std::fs::read(fixture.file()).unwrap();
        {
            let mut state = fixture.shared.lock().unwrap();
            if change_key {
                use yolo_hub_remote_startup_test_support::{FOREIGN_HUB, FOREIGN_KEY, peer};
                state.snapshot.state.identity = yolo_hub_delegation::PeerIdentity::new(
                    fixture.hub.hub_id(),
                    *peer(FOREIGN_HUB, FOREIGN_KEY).key(),
                )
                .unwrap();
            } else {
                state.snapshot.applied = false;
                state.snapshot.peer_socket = None;
            }
        }
        assert!(matches!(
            recover(&fixture.directories, &fixture.request, deadline())
                .await
                .unwrap()
                .fields()
                .outcome,
            EnrollmentOutcome::Unknown {}
        ));
        assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
        assert_eq!(fixture.configured(), EXPECTED_CONFIGURE_COUNT);
        fixture.peer.finish().await;
    }
}

#[tokio::test]
async fn record_replaced_during_live_readback_cannot_publish_a_ready_result() {
    let mut fixture = Fixture::new(false).await;
    let permit = fixture.prepare();
    fixture.apply(permit.prepared());
    drop(permit);
    let file = fixture.file();
    let backup = fixture.root.path().join(MOVED_ENTRY);
    fixture.shared.lock().unwrap().before_snapshot = Some(Box::new(move || {
        std::fs::rename(&file, &backup).unwrap();
        std::fs::copy(&backup, &file).unwrap();
    }));
    assert!(
        recover(&fixture.directories, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.configured(), 0);
    fixture.peer.finish().await;
}
