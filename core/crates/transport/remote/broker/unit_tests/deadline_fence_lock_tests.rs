use super::instance_lock::BrokerInstanceLock;
use super::launch_authority::BrokerLaunchAuthority;
use super::publication_gate::BrokerLaunchPublicationGuard;
use super::sockets::BrokerSockets;
use super::*;
use std::time::Duration;

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const EXACT_WAIT_BUDGET: Duration = Duration::from_millis(7);

#[tokio::test(start_paused = true)]
async fn reconcile_lock_contention_stops_at_the_exact_deadline() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let owner_deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let _owner = BrokerReconcileGuard::acquire(&paths, owner_deadline)
        .await
        .unwrap();

    assert_exact_timeout(BrokerReconcileGuard::acquire, &paths).await;
}

#[tokio::test(start_paused = true)]
async fn publication_lock_contention_stops_at_the_exact_deadline() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let owner_deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let _owner = BrokerLaunchPublicationGuard::acquire(&paths, owner_deadline)
        .await
        .unwrap();

    assert_exact_timeout(BrokerLaunchPublicationGuard::acquire, &paths).await;
}

#[tokio::test(start_paused = true)]
async fn instance_lock_contention_stops_at_the_exact_deadline() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let issue_deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let guard = BrokerReconcileGuard::acquire(&paths, issue_deadline)
        .await
        .unwrap();
    let authority = authorize_broker_launch(&paths, &guard, issue_deadline)
        .await
        .unwrap();
    drop(guard);
    let _owner = BrokerInstanceLock::acquire(paths.instance_lock()).unwrap();
    let started = tokio::time::Instant::now();
    let deadline = started + EXACT_WAIT_BUDGET;

    let result = AuthorizedBrokerStartup::acquire_for_launch(paths, authority, deadline).await;

    assert!(result.is_err());
    assert_eq!(tokio::time::Instant::now(), deadline);
}

#[tokio::test(start_paused = true)]
async fn expired_authority_is_never_published_after_the_final_fence() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let started = tokio::time::Instant::now();
    let deadline = started + EXACT_WAIT_BUDGET;
    let guard = BrokerReconcileGuard::acquire(&paths, deadline)
        .await
        .unwrap();

    let result =
        BrokerLaunchAuthority::issue_with_deadline_hook(&paths, &guard, deadline, || async move {
            tokio::time::advance(EXACT_WAIT_BUDGET).await;
            Ok(())
        })
        .await;

    assert!(result.is_err());
    assert_eq!(tokio::time::Instant::now(), deadline);
    assert!(!paths.launch_authority().exists());
}

#[tokio::test(start_paused = true)]
async fn expiry_after_both_socket_binds_cleans_every_publication() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let deadline = tokio::time::Instant::now() + EXACT_WAIT_BUDGET;
    let guard = BrokerReconcileGuard::acquire(&paths, deadline)
        .await
        .unwrap();
    let authority = authorize_broker_launch(&paths, &guard, deadline)
        .await
        .unwrap();
    drop(guard);
    let startup = AuthorizedBrokerStartup::acquire_for_launch(paths.clone(), authority, deadline)
        .await
        .unwrap();

    let result = BrokerSockets::bind_startup_with_deadline_hook(startup, || async {
        tokio::time::advance(EXACT_WAIT_BUDGET).await;
        Ok(())
    })
    .await;

    assert!(result.is_err());
    assert_eq!(tokio::time::Instant::now(), deadline);
    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());
    assert!(broker_instance_available(&paths).unwrap());
}

#[tokio::test(start_paused = true)]
async fn reconcile_lock_expiry_after_acquisition_releases_the_lock() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let deadline = tokio::time::Instant::now() + EXACT_WAIT_BUDGET;

    let result = BrokerReconcileGuard::acquire_with_deadline_hook(&paths, deadline, expire).await;

    assert_expired(result, deadline);
    let later = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    assert!(BrokerReconcileGuard::acquire(&paths, later).await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn publication_gate_expiry_after_acquisition_releases_the_gate() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let deadline = tokio::time::Instant::now() + EXACT_WAIT_BUDGET;

    let result =
        BrokerLaunchPublicationGuard::acquire_with_deadline_hook(&paths, deadline, expire).await;

    assert_expired(result, deadline);
    let later = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    assert!(
        BrokerLaunchPublicationGuard::acquire(&paths, later)
            .await
            .is_ok()
    );
}

#[tokio::test(start_paused = true)]
async fn instance_lock_expiry_after_acquisition_releases_the_lock() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let issue_deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let guard = BrokerReconcileGuard::acquire(&paths, issue_deadline)
        .await
        .unwrap();
    let authority = authorize_broker_launch(&paths, &guard, issue_deadline)
        .await
        .unwrap();
    drop(guard);
    let deadline = tokio::time::Instant::now() + EXACT_WAIT_BUDGET;

    let result = AuthorizedBrokerStartup::acquire_for_launch_with_deadline_hook(
        paths.clone(),
        authority,
        deadline,
        expire,
    )
    .await;

    assert_expired(result, deadline);
    assert!(broker_instance_available(&paths).unwrap());
}

async fn assert_exact_timeout<'a, F, Fut, T>(acquire: F, paths: &'a BrokerPaths)
where
    F: FnOnce(&'a BrokerPaths, tokio::time::Instant) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let started = tokio::time::Instant::now();
    let deadline = started + EXACT_WAIT_BUDGET;

    let result = acquire(paths, deadline).await;

    assert!(result.is_err());
    assert_eq!(tokio::time::Instant::now(), deadline);
}

async fn expire() -> anyhow::Result<()> {
    tokio::time::advance(EXACT_WAIT_BUDGET).await;
    Ok(())
}

fn assert_expired<T>(result: anyhow::Result<T>, deadline: tokio::time::Instant) {
    assert!(result.is_err());
    assert_eq!(tokio::time::Instant::now(), deadline);
}
