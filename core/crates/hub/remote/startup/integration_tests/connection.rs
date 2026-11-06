use super::{recovery_constants::*, recovery_fixture::Fixture};
use crate::{inspect, recover};
use yolo_remote_provisioning_protocol::EnrollmentOutcome;
use yolo_hub_rpc_protocol::Method;

#[tokio::test]
async fn recovery_never_combines_build_from_a_closed_connection_with_successor_configuration() {
    let mut fixture = Fixture::new(false).await;
    let permit = fixture.prepare();
    fixture.apply(permit.prepared());
    drop(permit);
    let before = std::fs::read(fixture.file()).unwrap();
    fixture.shared.lock().unwrap().disconnect_after = Some(Method::GetBuild);
    let result = recover(&fixture.directories, &fixture.request, deadline())
        .await
        .unwrap();
    assert!(matches!(
        result.fields().outcome,
        EnrollmentOutcome::Unknown {}
    ));
    {
        let state = fixture.shared.lock().unwrap();
        assert_eq!(state.connections, EXPECTED_CONNECTION_COUNT);
        assert_eq!(state.methods, vec![Method::GetBuild]);
    }
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    fixture.peer.finish().await;
}

#[tokio::test]
async fn prepare_cannot_accept_build_and_config_from_two_different_connections() {
    let mut fixture = Fixture::new(false).await;
    fixture.shared.lock().unwrap().disconnect_after = Some(Method::GetBuild);
    assert!(
        crate::enrollment::enroll(
            &fixture.directories,
            &fixture.connection(),
            &fixture.request,
            deadline()
        )
        .await
        .is_err()
    );
    assert!(
        inspect(&fixture.directories, &fixture.request)
            .unwrap()
            .is_none()
    );
    assert_eq!(fixture.configured(), 0);
    assert_eq!(
        fixture.shared.lock().unwrap().connections,
        EXPECTED_CONNECTION_COUNT
    );
    fixture.peer.finish().await;
}

#[tokio::test]
async fn completion_does_not_reconnect_after_acknowledged_configure_disconnects() {
    let mut fixture = Fixture::new(false).await;
    fixture.shared.lock().unwrap().disconnect_after = Some(Method::Configure);
    assert!(
        crate::enrollment::enroll(
            &fixture.directories,
            &fixture.connection(),
            &fixture.request,
            deadline()
        )
        .await
        .is_err()
    );
    assert!(
        inspect(&fixture.directories, &fixture.request)
            .unwrap()
            .unwrap()
            .completion
            .is_none()
    );
    {
        let state = fixture.shared.lock().unwrap();
        assert_eq!(state.connections, EXPECTED_CONNECTION_COUNT);
        assert_eq!(
            state.methods,
            vec![
                Method::GetBuild,
                Method::GetConfiguration,
                Method::GetBuild,
                Method::Configure
            ]
        );
    }
    assert_eq!(fixture.configured(), EXPECTED_CONFIGURE_COUNT);
    assert!(matches!(
        recover(&fixture.directories, &fixture.request, deadline())
            .await
            .unwrap()
            .fields()
            .outcome,
        EnrollmentOutcome::Ready { .. }
    ));
    assert_eq!(fixture.configured(), EXPECTED_CONFIGURE_COUNT);
    fixture.peer.finish().await;
}

#[tokio::test]
async fn initial_and_completion_evidence_share_the_same_authenticated_socket() {
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
        let state = fixture.shared.lock().unwrap();
        assert_eq!(state.connections, EXPECTED_CONNECTION_COUNT);
        assert_eq!(
            state.methods,
            vec![
                Method::GetBuild,
                Method::GetConfiguration,
                Method::GetBuild,
                Method::Configure,
                Method::GetBuild,
                Method::GetConfiguration,
                Method::GetBuild
            ]
        );
    }
    assert!(
        inspect(&fixture.directories, &fixture.request)
            .unwrap()
            .unwrap()
            .completion
            .is_some()
    );
    fixture.peer.finish().await;
}
