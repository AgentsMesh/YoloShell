use super::{recovery_constants::*, recovery_fixture::Fixture};
use crate::{RemoteDirectories, admission, ensure_running, inspect};
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};
use yolo_hub_enrollment_store::Admission;
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;

#[tokio::test]
async fn request_prepared_by_the_startup_owner_cannot_make_a_waiter_start_services_or_replan() {
    for different_payload in [false, true] {
        let mut fixture = Fixture::new(false).await;
        let owner = admission::acquire(&fixture.directories, &fixture.request, deadline())
            .await
            .unwrap();
        let mut fields = fixture.request.fields().clone();
        if different_payload {
            fields.remaining_reexports += 1;
        }
        let request = RemoteBootstrapRequest::new(fields).unwrap();
        assert!(inspect(&fixture.directories, &request).unwrap().is_none());
        let waiting = ensure_running(&fixture.directories, &request, deadline());
        tokio::pin!(waiting);
        assert!(matches!(
            waiting
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        let Admission::Fresh(claim) = owner.claim(fixture.hub.peer_identity()).unwrap() else {
            panic!()
        };
        drop(claim.prepare(fixture.prepared()).unwrap());
        let before = std::fs::read(fixture.file()).unwrap();
        assert!(waiting.await.is_err());
        assert!(!fixture.directories.install_root.exists());
        assert!(!fixture.directories.host_home.exists());
        assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
        assert_eq!(fixture.configured(), 0);
        if different_payload {
            assert!(fixture.shared.lock().unwrap().methods.is_empty());
        }
        fixture.peer.finish().await;
    }
}

#[tokio::test]
async fn cold_admission_has_no_identity_authority_and_missing_pinned_state_is_never_created() {
    let mut fixture = Fixture::new(false).await;
    let directories = RemoteDirectories {
        state: fixture.root.path().join(MOVED_ENTRY),
        host_home: fixture.directories.host_home.clone(),
        install_root: fixture.directories.install_root.clone(),
    };
    assert!(
        admission::acquire(&directories, &fixture.request, deadline())
            .await
            .is_err()
    );
    assert!(!directories.state.exists());
    let mut fields = fixture.request.fields().clone();
    fields.expected_peer = None;
    let request = RemoteBootstrapRequest::new(fields).unwrap();
    let owner = admission::acquire(&directories, &request, deadline())
        .await
        .unwrap();
    owner.verify().unwrap();
    assert!(inspect(&directories, &request).unwrap().is_none());
    assert_eq!(
        yolo_hub_state::HubState::inspect_peer_identity(&directories.state, &directories.host_home)
            .unwrap(),
        None
    );
    assert!(!directories.host_home.exists());
    fixture.peer.finish().await;
}
