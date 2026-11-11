use super::super::constants::PRIVATE_FILE_MODE;
use super::super::sockets::BrokerSockets;
use super::super::*;
use std::os::unix::fs::PermissionsExt;

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const AUTHORITY_REVOKED: &str = "launch authority was revoked";

async fn authority(paths: &BrokerPaths) -> BrokerLaunchAuthority {
    let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let guard = BrokerReconcileGuard::acquire(paths, deadline)
        .await
        .unwrap();
    authorize_broker_launch(paths, &guard, deadline)
        .await
        .unwrap()
}

#[tokio::test]
async fn idle_retirement_revokes_every_previously_issued_launch() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let superseded = authority(&paths).await;

    retire_broker(
        &paths,
        super::support::HOST_ID,
        tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
    )
    .await
    .unwrap();

    let error = BrokerSockets::bind_authorized(
        &paths,
        Some(superseded),
        None,
        Some(tokio::time::Instant::now() + BROKER_READY_TIMEOUT),
    )
    .await
    .err()
    .unwrap();
    assert!(error.to_string().contains(AUTHORITY_REVOKED));
    assert!(!paths.runtime_socket().exists());
}

#[tokio::test]
async fn authority_record_rejects_public_permissions_without_binding() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let authority = authority(&paths).await;
    std::fs::set_permissions(
        paths.launch_authority(),
        std::fs::Permissions::from_mode(PRIVATE_FILE_MODE | 0o044),
    )
    .unwrap();

    assert!(
        BrokerSockets::bind_authorized(
            &paths,
            Some(authority),
            None,
            Some(tokio::time::Instant::now() + BROKER_READY_TIMEOUT),
        )
        .await
        .is_err()
    );
    assert!(!paths.management_socket().exists());
}

#[tokio::test]
async fn retirement_waits_for_a_superseded_startup_to_release_its_instance_lock() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let issued = authority(&paths).await;
    let startup = AuthorizedBrokerStartup::acquire(paths.clone(), issued).unwrap();
    let retirement_paths = paths.clone();
    let retirement = tokio::spawn(async move {
        retire_broker(
            &retirement_paths,
            super::support::HOST_ID,
            tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
        )
        .await
    });

    while startup.verify().is_ok() {
        tokio::task::yield_now().await;
    }
    assert!(!retirement.is_finished());
    drop(startup);

    retirement.await.unwrap().unwrap();
    let replacement = authority(&paths).await;
    let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    drop(
        AuthorizedBrokerStartup::acquire_for_launch(paths, replacement, deadline)
            .await
            .unwrap(),
    );
}

#[tokio::test]
async fn launched_startup_waits_for_exact_instance_lock_release() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let issued = authority(&paths).await;
    let owner =
        super::super::instance_lock::BrokerInstanceLock::acquire(paths.instance_lock()).unwrap();
    let waiting_paths = paths.clone();
    let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let waiting = tokio::spawn(async move {
        AuthorizedBrokerStartup::acquire_for_launch(waiting_paths, issued, deadline).await
    });

    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    drop(owner);

    let startup = waiting.await.unwrap().unwrap();
    startup.verify().unwrap();
    drop(startup);
}

#[tokio::test]
async fn waiting_startup_cannot_claim_instance_after_authority_revocation() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let issued = authority(&paths).await;
    let owner =
        super::super::instance_lock::BrokerInstanceLock::acquire(paths.instance_lock()).unwrap();
    let waiting_paths = paths.clone();
    let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let waiting = tokio::spawn(async move {
        AuthorizedBrokerStartup::acquire_for_launch(waiting_paths, issued, deadline).await
    });

    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    let _replacement = authority(&paths).await;
    drop(owner);

    let error = waiting.await.unwrap().err().unwrap();
    assert!(error.to_string().contains(AUTHORITY_REVOKED));
    let replacement = authority(&paths).await;
    let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    drop(
        AuthorizedBrokerStartup::acquire_for_launch(paths, replacement, deadline)
            .await
            .unwrap(),
    );
}
