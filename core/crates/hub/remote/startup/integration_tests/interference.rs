use super::{recovery_constants::*, recovery_fixture::Fixture};
use crate::{RemoteDirectories, ensure_running, inspect, recover};
use yolo_remote_provisioning_protocol::{EnrollmentOutcome, RemoteBootstrapRequest};

#[tokio::test]
async fn later_revocation_or_successor_revision_never_restores_the_old_authorization() {
    for revoke in [false, true] {
        let mut fixture = Fixture::new(false).await;
        crate::enrollment::enroll(
            &fixture.directories,
            &fixture.connection(),
            &fixture.request,
            deadline(),
        )
        .await
        .unwrap();
        {
            let mut state = fixture.shared.lock().unwrap();
            state.snapshot.state.revision += 1;
            state.snapshot.state.last_change = uuid::Uuid::new_v4();
            if revoke {
                state.snapshot.state.configuration.peers.clear();
            }
        }
        let before = std::fs::read(fixture.file()).unwrap();
        assert!(matches!(
            recover(&fixture.directories, &fixture.request, deadline())
                .await
                .unwrap()
                .fields()
                .outcome,
            EnrollmentOutcome::Superseded {}
        ));
        assert!(
            ensure_running(&fixture.directories, &fixture.request, deadline())
                .await
                .is_err()
        );
        assert_eq!(fixture.configured(), EXPECTED_CONFIGURE_COUNT);
        assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
        fixture.peer.finish().await;
    }
}

#[tokio::test]
async fn request_id_payload_conflict_and_entry_directory_replacement_fail_before_live_requests() {
    let mut fixture = Fixture::new(false).await;
    drop(fixture.prepare());
    let mut fields = fixture.request.fields().clone();
    fields.remaining_reexports += 1;
    let wrong = RemoteBootstrapRequest::new(fields).unwrap();
    assert!(
        recover(&fixture.directories, &wrong, deadline())
            .await
            .is_err()
    );
    assert!(
        ensure_running(&fixture.directories, &wrong, deadline())
            .await
            .is_err()
    );
    assert!(fixture.shared.lock().unwrap().methods.is_empty());
    let entry = fixture.file().parent().unwrap().to_path_buf();
    let moved = fixture.root.path().join(MOVED_ENTRY);
    std::fs::rename(&entry, &moved).unwrap();
    yolo_paths::directory::PrivateDirectory::ensure(&entry).unwrap();
    std::fs::copy(moved.join(RECORD_FILE), fixture.file()).unwrap();
    assert!(
        recover(&fixture.directories, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert!(fixture.shared.lock().unwrap().methods.is_empty());
    fixture.peer.finish().await;
}

#[tokio::test]
async fn readonly_absence_does_not_create_state() {
    let mut fixture = Fixture::new(false).await;
    let absent = RemoteDirectories {
        state: fixture.root.path().join(MOVED_ENTRY),
        host_home: fixture.directories.host_home.clone(),
        install_root: fixture.directories.install_root.clone(),
    };
    assert!(inspect(&absent, &fixture.request).unwrap().is_none());
    assert!(matches!(
        recover(&absent, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        EnrollmentOutcome::Absent {}
    ));
    assert!(!absent.state.exists());
    assert_eq!(fixture.configured(), 0);
    fixture.peer.finish().await;
}
