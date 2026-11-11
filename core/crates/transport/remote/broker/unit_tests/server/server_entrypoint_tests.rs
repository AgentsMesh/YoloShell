use super::super::*;
use super::support::{FakeFactory, HOST_ID, test_config};

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;

fn paths() -> (tempfile::TempDir, BrokerPaths) {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    (root, paths)
}

#[tokio::test]
async fn public_constructor_installs_the_configured_paths_and_pool() {
    let (_root, paths) = paths();
    let broker = RemoteBroker::new(test_config(), paths.clone());

    assert_eq!(broker.paths().state_dir(), paths.state_dir());
    assert_eq!(broker.pool().status().await.host_id, HOST_ID);
}

#[tokio::test]
async fn public_serve_until_honors_an_already_ready_shutdown() {
    let (_root, paths) = paths();
    let broker =
        RemoteBroker::with_factory(test_config(), paths.clone(), FakeFactory::empty(Vec::new()));

    broker.serve_until(std::future::ready(())).await.unwrap();

    assert!(!paths.runtime_socket().exists());
    assert!(!paths.management_socket().exists());
}
