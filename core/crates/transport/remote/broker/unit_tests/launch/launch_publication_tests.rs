use super::super::launch_authority::BrokerLaunchAuthority;
use super::super::sockets::BrokerSockets;
use super::super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const AUTHORITY_REVOKED: &str = "launch authority was revoked";
const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;

async fn issue(paths: &BrokerPaths) -> BrokerLaunchAuthority {
    let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
    let guard = BrokerReconcileGuard::acquire(paths, deadline)
        .await
        .unwrap();
    authorize_broker_launch(paths, &guard, deadline)
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn completed_socket_publication_linearizes_before_later_revocation() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let issued = issue(&paths).await;
    let startup = AuthorizedBrokerStartup::acquire(paths.clone(), issued).unwrap();
    let (entered_send, entered_receive) = std::sync::mpsc::channel();
    let (release_send, release_receive) = std::sync::mpsc::channel();
    let binding = tokio::spawn(async move {
        BrokerSockets::bind_startup_with_publication_hooks(
            startup,
            || Ok(()),
            move || {
                entered_send.send(())?;
                release_receive.recv()?;
                Ok(())
            },
        )
        .await
    });
    tokio::task::spawn_blocking(move || entered_receive.recv().unwrap())
        .await
        .unwrap();

    let revoke_paths = paths.clone();
    let (blocked_send, blocked_receive) = std::sync::mpsc::channel();
    let revoke = tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
        let guard = BrokerReconcileGuard::acquire(&revoke_paths, deadline).await?;
        let mut blocked_send = Some(blocked_send);
        BrokerLaunchAuthority::revoke_observed(
            &revoke_paths,
            &guard,
            deadline,
            move || {
                if let Some(send) = blocked_send.take() {
                    send.send(())?;
                }
                Ok(())
            },
            || Ok(()),
        )
        .await
    });
    tokio::task::spawn_blocking(move || blocked_receive.recv().unwrap())
        .await
        .unwrap();
    assert!(!revoke.is_finished());

    release_send.send(()).unwrap();
    let sockets = binding.await.unwrap().unwrap();
    revoke.await.unwrap().unwrap();
    assert!(paths.runtime_socket().exists());
    assert!(paths.management_socket().exists());
    assert!(
        issued
            .verify(&paths)
            .unwrap_err()
            .to_string()
            .contains(AUTHORITY_REVOKED)
    );
    drop(sockets);
    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn revocation_linearized_first_prevents_stale_socket_publication() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let issued = issue(&paths).await;
    let startup = AuthorizedBrokerStartup::acquire(paths.clone(), issued).unwrap();
    let revoke_paths = paths.clone();
    let (revoked_send, revoked_receive) = std::sync::mpsc::channel();
    let (release_send, release_receive) = std::sync::mpsc::channel();
    let revoke = tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + BROKER_READY_TIMEOUT;
        let guard = BrokerReconcileGuard::acquire(&revoke_paths, deadline).await?;
        BrokerLaunchAuthority::revoke_observed(
            &revoke_paths,
            &guard,
            deadline,
            || Ok(()),
            move || {
                revoked_send.send(())?;
                release_receive.recv()?;
                Ok(())
            },
        )
        .await
    });
    tokio::task::spawn_blocking(move || revoked_receive.recv().unwrap())
        .await
        .unwrap();
    let hook_called = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&hook_called);
    let (blocked_send, blocked_receive) = std::sync::mpsc::channel();
    let binding = tokio::spawn(async move {
        let mut blocked_send = Some(blocked_send);
        BrokerSockets::bind_startup_with_publication_hooks(
            startup,
            move || {
                if let Some(send) = blocked_send.take() {
                    send.send(())?;
                }
                Ok(())
            },
            move || {
                observed.store(true, Ordering::SeqCst);
                Ok(())
            },
        )
        .await
    });
    tokio::task::spawn_blocking(move || blocked_receive.recv().unwrap())
        .await
        .unwrap();

    release_send.send(()).unwrap();
    revoke.await.unwrap().unwrap();
    let error = binding.await.unwrap().err().unwrap();

    assert!(error.to_string().contains(AUTHORITY_REVOKED));
    assert!(!hook_called.load(Ordering::SeqCst));
    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());
}
