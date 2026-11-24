use super::*;
use yolo_runtime::paths::directory::PrivateDirectory;

const HOME: &str = "home";
const OBSERVER_HOME: &str = "observer-home";
const SOCKET: &str = "host.sock";
const OCCUPIED_HOME: &[u8] = b"occupied";
const UNEXPECTED_START: &str = "Host started while absence evidence was reserved";
const UNEXPECTED_RETRY: &str = "non-contention startup failure was retried";

#[tokio::test]
async fn unoccupied_home_starts_without_waiting() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME);
    let host = open(HostConfig::new(&home)).await.unwrap();
    assert!(Host::observe_absence(&home, None).unwrap().is_none());
    drop(host);
    assert!(Host::observe_absence(&home, None).unwrap().is_some());
}

#[tokio::test]
async fn a_read_only_absence_query_does_not_abort_host_startup() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME);
    let absence = Host::observe_absence(&home, None).unwrap().unwrap();
    let starting = open(HostConfig::new(&home));
    tokio::pin!(starting);
    tokio::select! {
        biased;
        _ = &mut starting => panic!("{UNEXPECTED_START}"),
        _ = tokio::time::sleep(INSTANCE_LOCK_POLL) => {}
    }
    assert!(!home.exists());
    absence.verify().unwrap();
    drop(absence);
    assert!(starting.await.is_ok());
}

#[tokio::test]
async fn endpoint_absence_reservations_are_retryable_too() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let socket = root.path().join(SOCKET);
    let absence = Host::observe_absence(&root.path().join(OBSERVER_HOME), Some(&socket))
        .unwrap()
        .unwrap();
    let mut config = HostConfig::new(root.path().join(HOME));
    config.socket = Some(socket);
    let starting = open(config);
    tokio::pin!(starting);
    tokio::select! {
        biased;
        _ = &mut starting => panic!("{UNEXPECTED_START}"),
        _ = tokio::time::sleep(INSTANCE_LOCK_POLL) => {}
    }
    absence.verify().unwrap();
    drop(absence);
    assert!(starting.await.is_ok());
}

#[tokio::test]
async fn persistent_contention_remains_a_typed_failure_at_the_deadline() {
    let root = tempfile::tempdir().unwrap();
    let config = HostConfig::new(root.path().join(HOME));
    let _owner = Host::open(config.clone()).unwrap();
    let failure = open_until(config, Instant::now()).await.err().unwrap();
    assert!(failure.is::<HostInstanceBusy>());
}

#[tokio::test]
async fn incompatible_home_errors_are_not_retried_or_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOME);
    std::fs::write(&home, OCCUPIED_HOME).unwrap();
    let result = tokio::time::timeout(INSTANCE_LOCK_POLL, open(HostConfig::new(&home)))
        .await
        .expect(UNEXPECTED_RETRY);
    assert!(!result.err().unwrap().is::<HostInstanceBusy>());
    assert_eq!(std::fs::read(&home).unwrap(), OCCUPIED_HOME);
}

#[tokio::test]
async fn cancelling_a_wait_does_not_retain_a_home_lock() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join(HOME);
    let socket = root.path().join(SOCKET);
    let absence = Host::observe_absence(&root.path().join(OBSERVER_HOME), Some(&socket))
        .unwrap()
        .unwrap();
    let mut config = HostConfig::new(&home);
    config.socket = Some(socket);
    {
        let starting = open(config);
        tokio::pin!(starting);
        tokio::select! {
            biased;
            _ = &mut starting => panic!("{UNEXPECTED_START}"),
            _ = tokio::time::sleep(INSTANCE_LOCK_POLL) => {}
        }
    }
    assert!(Host::observe_absence(&home, None).unwrap().is_some());
    absence.verify().unwrap();
}
