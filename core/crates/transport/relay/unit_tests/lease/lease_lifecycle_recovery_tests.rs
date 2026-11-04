use super::*;
use crate::relay::session_test_support::{host_status, running_session};

#[tokio::test]
async fn reconcile_reacquires_only_when_the_previous_writer_may_recover() {
    let terminal_id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&Option::<WriterLease>::None),
        Reply::Lease(8),
    ])
    .await;
    let (mut owner, _) =
        make_controller(&host, terminal_id, Some(lease(HOLDER, INSTANCE, 7)), false);
    owner.recover_transport().await.unwrap();
    assert_eq!(owner.generation(), Some(8));

    let terminal_id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&Option::<WriterLease>::None),
    ])
    .await;
    let (mut fenced, _) = make_controller(&host, terminal_id, None, false);
    fenced.recover_transport().await.unwrap();
    assert_eq!(fenced.generation(), None);
}

#[tokio::test]
async fn renewal_transport_failure_attempts_runtime_recovery() {
    let host = FakeHost::start(vec![
        Reply::Disconnect,
        Reply::Disconnect,
        Reply::error(INTERNAL_CODE),
    ])
    .await;
    let (mut controller, _) = make_controller(
        &host,
        Uuid::new_v4(),
        Some(lease(HOLDER, INSTANCE, 2)),
        true,
    );
    assert!(controller.maintain().await.is_err());
    assert_eq!(
        host.methods(),
        vec![
            method::RENEW_WRITER,
            method::HOST_STATUS,
            method::HOST_STATUS
        ]
    );
}
