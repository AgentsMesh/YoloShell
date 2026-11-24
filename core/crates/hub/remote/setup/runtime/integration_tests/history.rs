use super::{broker::Verb, constants::*, fixture::Fixture};
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, SetupAction, SetupCheckpoint};

#[tokio::test]
async fn absent_history_is_entry_bound_without_preparing_or_contacting_a_broker() {
    let fixture = Fixture::new(false, false).await;
    let before = fixture.configuration().await;
    let history = fixture.history().await;
    assert_eq!(history.owner, fixture.state.hub_id());
    assert_eq!(
        history.operation_id,
        fixture.request.request.fields().request_id
    );
    assert_eq!(history.checkpoint, None);
    assert!(!fixture.record().exists());
    assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    assert_eq!(fixture.configuration().await, before);
    fixture.finish().await;
}

#[tokio::test]
async fn prepared_history_survives_a_fresh_runtime_without_broker_replay() {
    let fixture = Fixture::new(false, false).await;
    fixture.seed_prepared().await;
    let bytes = std::fs::read(fixture.record()).unwrap();
    assert_eq!(
        fixture.history().await.checkpoint,
        Some(SetupCheckpoint::Prepared {})
    );
    assert_eq!(
        fixture.fresh_reader_history().await.checkpoint,
        Some(SetupCheckpoint::Prepared {})
    );
    assert_eq!(std::fs::read(fixture.record()).unwrap(), bytes);
    assert!(fixture.broker.state.lock().unwrap().calls.is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn provisioning_history_does_not_wait_for_or_repeat_the_blocked_remote_call() {
    let fixture = Fixture::new(false, true).await;
    fixture.action(SetupAction::Start).await.unwrap();
    fixture.broker.wait_provision().await;
    let bytes = std::fs::read(fixture.record()).unwrap();
    let calls = fixture.broker.state.lock().unwrap().calls.clone();
    assert_eq!(
        fixture.history().await.checkpoint,
        Some(SetupCheckpoint::Provisioning {})
    );
    assert_eq!(fixture.broker.state.lock().unwrap().calls, calls);
    assert_eq!(std::fs::read(fixture.record()).unwrap(), bytes);
    fixture.broker.unblock();
    fixture.settled().await;
    fixture.finish().await;
}

#[tokio::test]
async fn completion_is_historical_after_route_revocation_and_is_read_from_the_journal() {
    let fixture = Fixture::new(false, false).await;
    fixture.action(SetupAction::Start).await.unwrap();
    assert!(matches!(
        fixture.settled().await.outcome,
        RemoteSetupOutcome::Linked { .. }
    ));
    let history = fixture.history().await;
    assert!(matches!(
        history.checkpoint,
        Some(SetupCheckpoint::Complete { .. })
    ));
    let calls = fixture.broker.state.lock().unwrap().calls.clone();
    let bytes = std::fs::read(fixture.record()).unwrap();
    let mut revoked = fixture.configuration().await;
    revoked.state.configuration.peers.clear();
    let revoked = fixture.configure(revoked).await;
    assert_eq!(fixture.history().await, history);
    assert_eq!(fixture.fresh_reader_history().await, history);
    assert_eq!(fixture.configuration().await, revoked);
    assert_eq!(std::fs::read(fixture.record()).unwrap(), bytes);
    assert_eq!(fixture.broker.state.lock().unwrap().calls, calls);
    assert_eq!(fixture.broker.count(Verb::Provision), ONE);
    fixture.finish().await;
}
