use super::*;

#[tokio::test]
async fn observer_recovery_never_escalates_to_takeover() {
    let terminal_id = Uuid::new_v4();
    let occupied = foreign(FIRST_FOREIGN_HOLDER, INITIAL_GENERATION);
    let mut replies = vec![Reply::Disconnect];
    replies.extend(reconnect(terminal_id));
    replies.extend([
        Reply::value(&Some(occupied.clone())),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&Some(occupied.clone())),
        Reply::error(crate::relay::constants::error_code::HELD_BY_OTHER),
        Reply::value(&Some(occupied)),
    ]);
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);

    controller.acquire().await.unwrap();
    controller.recover_transport().await.unwrap();
    controller.maintain().await.unwrap();

    assert_eq!(controller.generation(), None);
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Observe);
    assert!(takeover_generations(&host).is_empty());
}

#[tokio::test]
async fn pending_takeover_survives_an_additional_transport_reconciliation() {
    let terminal_id = Uuid::new_v4();
    let occupied = foreign(FIRST_FOREIGN_HOLDER, INITIAL_GENERATION);
    let mut replies = vec![Reply::value(&Some(occupied.clone())), Reply::Disconnect];
    replies.extend(reconnect(terminal_id));
    replies.extend([
        Reply::value(&Some(occupied)),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&Some(foreign(
            SECOND_FOREIGN_HOLDER,
            SUPERSEDING_GENERATION,
        ))),
        Reply::value(&Some(foreign(SECOND_FOREIGN_HOLDER, LATEST_GENERATION))),
        Reply::Lease(FINAL_GENERATION),
    ]);
    let host = FakeHost::start(replies).await;
    let mut controller = controller(&host, terminal_id);

    controller.takeover().await.unwrap();
    controller.recover_transport().await.unwrap();
    assert_eq!(controller.retry_mode, LeaseAcquireMode::Takeover);
    controller.maintain().await.unwrap();

    assert_eq!(controller.generation(), Some(FINAL_GENERATION));
    assert_eq!(
        takeover_generations(&host),
        [Some(INITIAL_GENERATION), Some(LATEST_GENERATION)]
    );
}
