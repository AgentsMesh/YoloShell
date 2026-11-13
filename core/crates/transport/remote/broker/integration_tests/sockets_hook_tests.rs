use super::super::sockets::{BrokerSocketBindStage, BrokerSockets};
use super::super::*;
use std::cell::{Cell, RefCell};

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const MANAGEMENT_SOCKET_HOOK_FAILURE: &str = "injected management socket hook failure";

fn paths() -> (tempfile::TempDir, BrokerPaths) {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    (root, paths)
}

#[tokio::test]
async fn successful_lock_and_namespace_hooks_bind_both_endpoints() {
    let (_root, paths) = paths();
    let lock_called = Cell::new(false);
    let sockets = BrokerSockets::bind_with_lock_hook(&paths, || {
        lock_called.set(true);
        Ok(())
    })
    .await
    .unwrap();
    assert!(lock_called.get());
    drop(sockets);

    let namespace_called = Cell::new(false);
    let sockets = BrokerSockets::bind_with_namespace_hook(&paths, || {
        namespace_called.set(true);
        Ok(())
    })
    .await
    .unwrap();

    assert!(namespace_called.get());
    assert!(paths.runtime_socket().exists());
    assert!(paths.management_socket().exists());
    drop(sockets);
}

#[tokio::test]
async fn management_hook_failure_cleans_both_endpoints_and_releases_the_lock() {
    let (_root, paths) = paths();
    let stages = RefCell::new(Vec::new());

    let error = BrokerSockets::bind_with_socket_hook(&paths, |stage| {
        stages.borrow_mut().push(stage);
        if stage == BrokerSocketBindStage::ManagementPublished {
            anyhow::bail!(MANAGEMENT_SOCKET_HOOK_FAILURE);
        }
        Ok(())
    })
    .await
    .err()
    .unwrap();

    assert!(error.to_string().contains(MANAGEMENT_SOCKET_HOOK_FAILURE));
    assert_eq!(
        stages.into_inner(),
        [
            BrokerSocketBindStage::RuntimePublished,
            BrokerSocketBindStage::ManagementPublished,
        ]
    );
    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());
    drop(BrokerSockets::bind(&paths).await.unwrap());
}
