use super::super::constants::CODEX_ACK_PROTOCOL;
use super::support::*;
use crate::local_unix::LocalUnixStream;
use std::sync::Arc;

#[tokio::test]
async fn ingest_panic_revokes_watch_before_protocol_rejection() {
    let fixture = Fixture::new();
    let snapshot = std::fs::read(fixture.runtime.snapshot_path()).unwrap();
    let event = fixture.enqueue(fixture.start(FIRST_EVENT_SEQUENCE));
    let runtime = Arc::clone(&fixture.runtime);
    let authority = runtime.subscribe_authority();
    runtime.panic_next_ingest_for_test();

    let (ack, publication) = exchange_uncommitted(Arc::clone(&runtime), &event).await;

    assert!(!ack.accepted);
    assert_eq!(ack.code, CODEX_ACK_PROTOCOL);
    assert!(ack.event_id.is_none());
    assert!(ack.sequence.is_none());
    assert!(publication.is_none());
    assert!(runtime.storage_failed().unwrap());
    assert!(runtime.state().unwrap().is_none());
    assert_eq!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    );
    assert_eq!(std::fs::read(runtime.snapshot_path()).unwrap(), snapshot);
}

#[tokio::test]
async fn cancelled_handoff_latches_before_queued_ingest() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(FIRST_EVENT_SEQUENCE));
    let runtime = Arc::clone(&fixture.runtime);
    let mut authority = runtime.subscribe_authority();
    let gate = runtime.gate_next_handoff_for_test();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    let handler = tokio::spawn(super::super::handler::handle(server, Arc::clone(&runtime)));
    let payload = yolo_capnp_codec::encode(&event).unwrap();
    write_bytes_frame(&mut client, &payload).await;
    let entered = Arc::clone(&gate);
    tokio::time::timeout(
        super::super::constants::CODEX_REQUEST_TIMEOUT,
        tokio::task::spawn_blocking(move || entered.wait_until_entered()),
    )
    .await
    .unwrap()
    .unwrap();

    handler.abort();
    assert!(matches!(handler.await, Err(error) if error.is_cancelled()));
    let retry_runtime = Arc::clone(&runtime);
    let retry_event = event.clone();
    let retry_pid = fixture.process.pid;
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let mut retry = tokio::task::spawn_blocking(move || {
        let _ = started_tx.send(());
        retry_runtime.ingest(retry_event, retry_pid)
    });
    started_rx.await.unwrap();
    assert!(
        tokio::time::timeout(
            super::super::constants::CODEX_SOCKET_PROBE_TIMEOUT,
            &mut retry,
        )
        .await
        .is_err()
    );
    gate.release();
    wait_for_unavailable(&mut authority).await;

    assert!(matches!(
        retry.await.unwrap(),
        Err(super::super::error::CodexHookRejection::Storage(_))
    ));
    assert!(runtime.snapshot_path().exists());
    assert!(matches!(
        runtime.ingest(event, fixture.process.pid),
        Err(super::super::error::CodexHookRejection::Storage(_))
    ));
}

#[test]
fn reconcile_panic_revokes_watch_before_returning() {
    let fixture = Fixture::new();
    let authority = fixture.runtime.subscribe_authority();
    fixture.runtime.panic_next_reconcile_for_test();

    assert!(fixture.runtime.reconcile_pending().is_err());

    assert_eq!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    );
}

#[test]
fn process_exit_panic_revokes_watch_before_returning() {
    let fixture = Fixture::new();
    let authority = fixture.runtime.subscribe_authority();
    fixture.runtime.panic_next_process_exit_for_test();
    let observed =
        crate::agent::hooks::ProcessIdentity::new(fixture.process.pid, fixture.process.start_time);

    assert!(matches!(
        fixture.runtime.observe_process_exit(observed),
        Err(super::super::error::CodexHookRejection::Storage(_))
    ));

    assert_eq!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    );
}

async fn wait_for_unavailable(
    authority: &mut tokio::sync::watch::Receiver<super::super::publication::CodexAuthorityView>,
) {
    tokio::time::timeout(super::super::constants::CODEX_REQUEST_TIMEOUT, async {
        loop {
            if matches!(
                authority.borrow().clone(),
                super::super::publication::CodexAuthorityView::StorageUnavailable
            ) {
                return;
            }
            authority.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
}
