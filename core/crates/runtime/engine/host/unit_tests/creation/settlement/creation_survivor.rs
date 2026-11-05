use crate::host::tests::*;
use super::*;

const SURVIVOR_ACK_LOSS_NAME: &str = "survivor-ack-loss";
const SURVIVOR_GENERIC_ERROR_NAME: &str = "survivor-generic-error";
const SURVIVOR_RESTART_NAME: &str = "survivor-restart";
const SURVIVOR_DISCARD_NAME: &str = "survivor-discard";

#[tokio::test]
async fn creation_survivor_exact_unavailable_survives_ack_loss_without_second_terminate() {
    let fixture = survivor_creation_fixture(SURVIVOR_ACK_LOSS_NAME).await;
    let listener = bind_private_cleanup_listener(&fixture.runtime.socket_path);
    let requests = Arc::new(std::sync::atomic::AtomicUsize::new(usize::default()));
    let server = tokio::spawn(serve_cleanup_termination(
        listener,
        fixture.runtime.clone(),
        survivor_error(),
        Arc::clone(&requests),
    ));
    let before = durable_session_bytes(&fixture.host, fixture.binding.result.terminal_id);
    let request = cleanup_termination_request(&fixture.binding);

    let lost_ack = fixture
        .host
        .cleanup_terminate_session(request)
        .await
        .unwrap();
    assert_eq!(lost_ack, CleanupTerminationOutcome::Survivor);
    server.await.unwrap();
    let survivor_receipt =
        assert_survivor_completion(&fixture.host, &fixture.binding, &fixture.ledger_path);
    assert_eq!(
        requests.load(std::sync::atomic::Ordering::SeqCst),
        SINGLE_TERMINATION_REQUEST
    );
    let after = durable_session_bytes(&fixture.host, fixture.binding.result.terminal_id);
    assert_ne!(after.0, before.0);
    assert_eq!(after.1, before.1);
    assert_eq!(after.2, before.2);
    assert_uncertain_survivor_recovery(&fixture.host, fixture.binding.result.terminal_id);

    let replay = fixture
        .host
        .cleanup_terminate_session(request)
        .await
        .unwrap();
    assert_eq!(replay, CleanupTerminationOutcome::Survivor);
    fixture
        .host
        .finalize_creation(finalize_survivor_request(&fixture))
        .await
        .unwrap();
    assert_eq!(
        fixture
            .host
            .creation_receipts
            .lookup_binding(&fixture.binding)
            .unwrap(),
        Some(survivor_receipt)
    );
    assert_eq!(
        requests.load(std::sync::atomic::Ordering::SeqCst),
        SINGLE_TERMINATION_REQUEST
    );
}

#[tokio::test]
async fn creation_survivor_generic_error_keeps_binding_and_session_unchanged() {
    let fixture = survivor_creation_fixture(SURVIVOR_GENERIC_ERROR_NAME).await;
    let listener = bind_private_cleanup_listener(&fixture.runtime.socket_path);
    let requests = Arc::new(std::sync::atomic::AtomicUsize::new(usize::default()));
    let server = tokio::spawn(serve_cleanup_termination(
        listener,
        fixture.runtime.clone(),
        ErrorBody::conflict(GENERIC_TERMINATION_DETAIL),
        Arc::clone(&requests),
    ));
    let before = durable_session_bytes(&fixture.host, fixture.binding.result.terminal_id);

    let error = fixture
        .host
        .cleanup_terminate_session(cleanup_termination_request(&fixture.binding))
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    server.await.unwrap();
    assert_transition_ledger(&fixture.ledger_path, &[fixture.binding.clone()]);
    assert!(
        fixture
            .host
            .creation_receipts
            .lookup_binding(&fixture.binding)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        durable_session_bytes(&fixture.host, fixture.binding.result.terminal_id),
        before
    );
    assert_eq!(
        requests.load(std::sync::atomic::Ordering::SeqCst),
        SINGLE_TERMINATION_REQUEST
    );
}

#[tokio::test]
async fn creation_survivor_host_restart_repairs_receipt_before_binding_removal() {
    let fixture = survivor_creation_fixture(SURVIVOR_RESTART_NAME).await;
    fixture
        .host
        .creation_receipts
        .publish(&CreationReceipt::from_binding(
            &fixture.binding,
            CreationCompletion::Survivor,
        ))
        .unwrap();
    assert_transition_ledger(&fixture.ledger_path, &[fixture.binding.clone()]);
    let request = cleanup_termination_request(&fixture.binding);
    let SurvivorCreationFixture {
        directory,
        host,
        binding,
        ledger_path,
        ..
    } = fixture;
    drop(host);

    let reopened = Host::open(HostConfig::new(directory.path())).unwrap();
    let _receipt = assert_survivor_completion(&reopened, &binding, &ledger_path);
    assert_eq!(
        reopened.cleanup_terminate_session(request).await.unwrap(),
        CleanupTerminationOutcome::Survivor
    );
    assert_uncertain_survivor_recovery(&reopened, binding.result.terminal_id);
}

#[tokio::test]
async fn creation_survivor_discard_never_archives_and_archive_conflicts() {
    let fixture = survivor_creation_fixture(SURVIVOR_DISCARD_NAME).await;
    let listener = bind_private_cleanup_listener(&fixture.runtime.socket_path);
    let requests = Arc::new(std::sync::atomic::AtomicUsize::new(usize::default()));
    let server = tokio::spawn(serve_cleanup_termination(
        listener,
        fixture.runtime.clone(),
        survivor_error(),
        Arc::clone(&requests),
    ));

    let view = fixture
        .host
        .cleanup_discard_session(cleanup_discard_request(&fixture.binding))
        .await
        .unwrap();
    server.await.unwrap();
    assert!(!view.manifest.archived);
    let _receipt =
        assert_survivor_completion(&fixture.host, &fixture.binding, &fixture.ledger_path);
    let durable: TerminalManifest = read_json(
        &fixture
            .host
            .paths
            .manifest(fixture.binding.result.terminal_id),
    )
    .unwrap();
    assert!(!durable.archived);
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);

    let archive_error = fixture
        .host
        .cleanup_archive_session(cleanup_archive_request(&fixture.binding))
        .await
        .unwrap_err();
    assert_eq!(archive_error.code, crate::protocol::error_code::CONFLICT);
    let replay = fixture
        .host
        .cleanup_discard_session(cleanup_discard_request(&fixture.binding))
        .await
        .unwrap();
    assert!(!replay.manifest.archived);
    assert_eq!(
        requests.load(std::sync::atomic::Ordering::SeqCst),
        SINGLE_TERMINATION_REQUEST
    );
}
