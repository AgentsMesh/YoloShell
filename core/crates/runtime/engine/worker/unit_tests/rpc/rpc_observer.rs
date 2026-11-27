use crate::worker::tests::*;
const OBSERVER_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);

#[tokio::test]
async fn dropping_rpc_observer_releases_capacity_without_ending_the_terminal() {
    let fixture = Fixture::new();
    let before = fixture.state.attachment_clients.available_permits();
    let (mut client, task) = authenticated_worker_connection(&fixture, FIXTURE_CONTROLLER_GENERATION).await;
    attach_worker(&mut client, &fixture).await;
    assert_eq!(fixture.state.attachment_clients.available_permits(), before - 1);
    drop(client);
    task.await.unwrap().unwrap();
    tokio::time::timeout(OBSERVER_RELEASE_TIMEOUT, async {
        while fixture.state.attachment_clients.available_permits() != before { tokio::task::yield_now().await; }
    }).await.unwrap();
    assert!(lock_recover(&fixture.state.exit).is_none());
    assert!(!fixture.state.response_shutdown_is_committed());
}
