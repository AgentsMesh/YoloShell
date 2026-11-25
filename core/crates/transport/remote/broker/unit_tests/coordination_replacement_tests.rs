use crate::remote_broker::handoff::BrokerReconcileGuard;
use crate::remote_broker::sockets::BrokerSockets;
use crate::remote_broker::{BROKER_READY_TIMEOUT, BrokerPaths};
use std::fs;
use std::os::unix::fs::PermissionsExt;

const COORDINATION_DIRECTORY: &str = "coordination";
const PRIVATE_TEMP_DIRECTORY: &str = "/private/tmp";
const DISPLACED_COORDINATION_DIRECTORY: &str = "coordination-displaced";
const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const SENTINEL_FILE: &str = "replacement-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve-replacement";

#[tokio::test]
async fn process_leases_survive_coordination_root_replacement() {
    let root = tempfile::tempdir_in(PRIVATE_TEMP_DIRECTORY).unwrap();
    let coordination = root.path().join(COORDINATION_DIRECTORY);
    let state = root.path().join(STATE_DIRECTORY);
    let paths = BrokerPaths::new_with_coordination(&state, &coordination).unwrap();
    let guard =
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
            .await
            .unwrap();
    let sockets = BrokerSockets::bind(&paths).await.unwrap();

    let displaced = root.path().join(DISPLACED_COORDINATION_DIRECTORY);
    fs::rename(&coordination, displaced).unwrap();
    let replacement = BrokerPaths::new_with_coordination(&state, &coordination).unwrap();
    let sentinel = coordination.join(SENTINEL_FILE);
    fs::write(&sentinel, SENTINEL_BYTES).unwrap();
    fs::set_permissions(
        &sentinel,
        fs::Permissions::from_mode(super::super::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();

    assert!(
        BrokerReconcileGuard::acquire(&replacement, tokio::time::Instant::now())
            .await
            .is_err()
    );
    assert!(BrokerSockets::bind(&replacement).await.is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), SENTINEL_BYTES);

    drop(sockets);
    drop(guard);
    BrokerReconcileGuard::acquire(
        &replacement,
        tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
    )
    .await
    .unwrap();
    drop(BrokerSockets::bind(&replacement).await.unwrap());
    assert_eq!(fs::read(sentinel).unwrap(), SENTINEL_BYTES);
}
