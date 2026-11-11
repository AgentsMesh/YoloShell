use super::super::forwarder::{ProjectionForwarders, ProjectionInput};
use super::super::test_support::ProjectionFixture;

const TEST_CHANNEL_CAPACITY: usize = 2;
const ROUTE_REVISION_INCREMENT: u64 = 1;

#[tokio::test]
async fn snapshot_initial_views_are_seeded_in_route_order_without_queue_races() {
    let mut fixture = ProjectionFixture::new();
    fixture.start();
    let older = fixture.committed.take().unwrap();
    let mut newer = older.clone();
    newer.revision += ROUTE_REVISION_INCREMENT;
    let expected = [older.revision, newer.revision];
    let (sender, mut receiver) = tokio::sync::mpsc::channel(TEST_CHANNEL_CAPACITY);
    let mut forwarders = ProjectionForwarders::new();

    let observed = [older, newer].map(|route| {
        let ProjectionInput::View { route, .. } =
            forwarders.install_seeded(route, sender.clone()).unwrap()
        else {
            unreachable!();
        };
        route.revision
    });

    assert_eq!(observed, expected);
    assert!(receiver.try_recv().is_err());
    forwarders.tasks.abort_all();
}
